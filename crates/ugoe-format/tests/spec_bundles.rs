//! 公開仕様のファイルの例 (`spec/examples/bundles/`) を、参照実装の読み込みで検査する。
//! 例の形式は `spec/examples/README.md#file-examples`、コンテナの仕様は `docs/spec/file-formats.md`。
//!
//! 例のファイルは仕様だけから作った別の書き出しで作られている。このテストの側で例を合わせて直してはならない。
//! 参照実装と食い違う例は、理由を書いて [`KNOWN_MISMATCHES`] に入れる。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use ugoe_format::{
    FeatureDecl, FormatError, ReadReport, ReadWarning, TextureData, TextureFormat,
    expression_from_json, model_from_json, module_from_json, motion_from_json, physics_from_json,
    read_bundle_with_report, read_module_with_report,
};
use ugoe_model::Model;
use ugoe_model::module::Module;

/// 参照実装と食い違うと分かっている例: (`file`, 理由)。
/// ここに挙げた例は、食い違いが残っていることも検査する (直ったら外す)。
const KNOWN_MISMATCHES: &[(&str, &str)] = &[];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Examples {
    #[allow(dead_code)]
    summary: String,
    files: Vec<FileExample>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileExample {
    file: String,
    #[allow(dead_code)]
    summary: String,
    /// 参考の情報 (既定値のフィールドを書くかどうか)。読んだ結果は同じなので照合しない。
    #[allow(dead_code)]
    style: String,
    chunks: Vec<ExpectedChunk>,
    payload_order: Option<Vec<usize>>,
    features: Option<Vec<FeatureDecl>>,
    result: Outcome,
    error: Option<String>,
    unknown_required_features: Option<Vec<String>>,
    warnings: Option<ExpectedWarnings>,
    model: Option<String>,
    physics: Option<String>,
    #[serde(default)]
    motions: Vec<String>,
    #[serde(default)]
    expressions: Vec<String>,
    module: Option<String>,
    meta: Option<Value>,
    textures: Vec<ExpectedImage>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Ok,
    Error,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedChunk {
    tag: String,
    #[serde(default)]
    compressed: bool,
    /// 知らないチャンクのペイロード (UTF-8)。
    payload: Option<String>,
    /// 長さ 0 のペイロードの `offset`。
    offset: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedWarnings {
    unknown_features: Vec<String>,
}

/// テクスチャ 1 枚と、その法線マップ (同じ形)。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    normal_map: Option<Box<ExpectedImage>>,
}

/// `FEAT` チャンクのペイロード (`spec/bundle.md` の Feature declarations)。
#[derive(Deserialize)]
struct FeatureTable {
    #[serde(default)]
    features: Vec<FeatureDecl>,
}

/// チャンク表の 1 項目 (読み込みのコードを使わずに読む)。
struct RawChunk {
    tag: [u8; 4],
    flags: u32,
    offset: usize,
    length: usize,
}

impl RawChunk {
    fn tag(&self) -> String {
        String::from_utf8_lossy(&self.tag).into_owned()
    }

    fn payload<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.offset..self.offset + self.length]
    }

    /// 圧縮されていれば展開したペイロード。
    fn decoded_payload(&self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        let raw = self.payload(bytes);
        if self.flags & 1 == 0 {
            return Ok(raw.to_vec());
        }
        let mut out = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(raw)
            .map_err(|e| e.to_string())?
            .read_to_end(&mut out)
            .map_err(|e| e.to_string())?;
        Ok(out)
    }
}

fn bundles_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/examples/bundles")
}

fn read_text(path: &str) -> String {
    let path = bundles_dir().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn reference_reader_meets_every_file_example() {
    let examples: Examples = serde_json::from_str(&read_text("bundles.json"))
        .unwrap_or_else(|e| panic!("bundles.json: {e}"));
    assert!(!examples.files.is_empty());
    let mut failures = Vec::new();
    for example in &examples.files {
        let mut found = Vec::new();
        check_file(example, &mut found);
        let known = KNOWN_MISMATCHES
            .iter()
            .find(|(file, _)| *file == example.file);
        match (known, found.is_empty()) {
            (None, _) => {
                failures.extend(found.into_iter().map(|f| format!("{}: {f}", example.file)))
            }
            (Some(_), false) => {}
            (Some((file, _)), true) => failures.push(format!(
                "{file}: listed in KNOWN_MISMATCHES but meets its example; remove it"
            )),
        }
    }
    for (file, _) in KNOWN_MISMATCHES {
        if !examples.files.iter().any(|e| e.file == *file) {
            failures.push(format!(
                "{file}: listed in KNOWN_MISMATCHES but not in bundles.json"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn check_file(example: &FileExample, failures: &mut Vec<String>) {
    let path = bundles_dir().join(&example.file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let table = match chunk_table(&bytes) {
        Ok(table) => table,
        Err(e) => return failures.push(format!("chunk table: {e}")),
    };
    check_chunk_table(example, &bytes, &table, failures);
    check_features(example, &bytes, &table, failures);

    let is_module = example.file.ends_with(".ugoemod");
    match example.result {
        Outcome::Ok => {
            if is_module {
                match read_module_with_report(&bytes) {
                    Ok((file, report)) => {
                        check_body(&file.module, example.module.as_deref(), "module", failures);
                        check_images(example, &file.textures, |i| file.normal_map(i), failures);
                        check_ok_common(example, file.meta.as_ref(), &report, failures);
                    }
                    Err(e) => failures.push(format!("read_module: {e}")),
                }
            } else {
                match read_bundle_with_report(&bytes) {
                    Ok((bundle, report)) => {
                        check_body(&bundle.model, example.model.as_deref(), "model", failures);
                        check_bundle_extras(example, &bundle, failures);
                        check_images(
                            example,
                            &bundle.textures,
                            |i| bundle.normal_map(i),
                            failures,
                        );
                        check_ok_common(example, bundle.meta.as_ref(), &report, failures);
                    }
                    Err(e) => failures.push(format!("read_bundle: {e}")),
                }
            }
        }
        Outcome::Error => {
            let result = if is_module {
                read_module_with_report(&bytes).map(|_| ())
            } else {
                read_bundle_with_report(&bytes).map(|_| ())
            };
            match result {
                Ok(()) => failures.push(format!(
                    "read succeeded; expected error {:?}",
                    example.error
                )),
                Err(e) => check_error(example, &e, &bytes, failures),
            }
            // 拒否されるファイルでも、本体のチャンクは例の JSON の文書のデータを持つ。
            let tag = if is_module { *b"MODU" } else { *b"MODL" };
            match table.iter().find(|c| c.tag == tag) {
                Some(chunk) if is_module => match decode_chunk::<Module>(chunk, &bytes) {
                    Ok(module) => {
                        check_body(&module, example.module.as_deref(), "module", failures)
                    }
                    Err(e) => failures.push(format!("MODU: {e}")),
                },
                Some(chunk) => match decode_chunk::<Model>(chunk, &bytes) {
                    Ok(model) => check_body(&model, example.model.as_deref(), "model", failures),
                    Err(e) => failures.push(format!("MODL: {e}")),
                },
                None => failures.push("no body chunk".to_owned()),
            }
        }
    }
}

// --- コンテナ ------------------------------------------------------------------------------------

/// ヘッダとチャンク表を読む (`spec/bundle.md` の Header と Chunk table)。
fn chunk_table(bytes: &[u8]) -> Result<Vec<RawChunk>, String> {
    let u32_at = |at: usize| -> Result<u32, String> {
        let b = bytes.get(at..at + 4).ok_or("truncated")?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u64_at = |at: usize| -> Result<usize, String> {
        let b = bytes.get(at..at + 8).ok_or("truncated")?;
        let mut raw = [0; 8];
        raw.copy_from_slice(b);
        usize::try_from(u64::from_le_bytes(raw)).map_err(|e| e.to_string())
    };
    if bytes.get(..4) != Some(b"UGOE".as_slice()) {
        return Err("bad magic".to_owned());
    }
    let count = u32_at(12)? as usize;
    (0..count)
        .map(|i| {
            let at = 16 + i * 24;
            let tag = bytes.get(at..at + 4).ok_or("truncated")?;
            let chunk = RawChunk {
                tag: [tag[0], tag[1], tag[2], tag[3]],
                flags: u32_at(at + 4)?,
                offset: u64_at(at + 8)?,
                length: u64_at(at + 16)?,
            };
            if chunk.offset + chunk.length > bytes.len() {
                return Err(format!("chunk {i} out of bounds"));
            }
            Ok(chunk)
        })
        .collect()
}

fn check_chunk_table(
    example: &FileExample,
    bytes: &[u8],
    table: &[RawChunk],
    failures: &mut Vec<String>,
) {
    let expected: Vec<(String, bool)> = example
        .chunks
        .iter()
        .map(|c| (c.tag.clone(), c.compressed))
        .collect();
    let actual: Vec<(String, bool)> = table.iter().map(|c| (c.tag(), c.flags & 1 != 0)).collect();
    if actual != expected {
        failures.push(format!(
            "chunks (tag, compressed): {actual:?} != {expected:?}"
        ));
        return;
    }
    for (i, (chunk, expected)) in table.iter().zip(&example.chunks).enumerate() {
        if let Some(payload) = &expected.payload
            && chunk.payload(bytes) != payload.as_bytes()
        {
            failures.push(format!(
                "chunk {i} {}: payload {:?} != {payload:?}",
                expected.tag,
                String::from_utf8_lossy(chunk.payload(bytes))
            ));
        }
        if let Some(offset) = expected.offset
            && (chunk.offset, chunk.length) != (offset, 0)
        {
            failures.push(format!(
                "chunk {i} {}: (offset, length) ({}, {}) != ({offset}, 0)",
                expected.tag, chunk.offset, chunk.length
            ));
        }
    }
    // 空のペイロードは位置を問わないので、並びの照合から除く。
    let mut order: Vec<usize> = (0..table.len()).collect();
    order.sort_by_key(|&i| table[i].offset);
    let expected_order = example
        .payload_order
        .clone()
        .unwrap_or_else(|| (0..table.len()).collect());
    let non_empty = |order: &[usize]| -> Vec<usize> {
        order
            .iter()
            .copied()
            .filter(|&i| table[i].length > 0)
            .collect()
    };
    if non_empty(&order) != non_empty(&expected_order) {
        failures.push(format!("payload order {order:?} != {expected_order:?}"));
    }
}

fn check_features(
    example: &FileExample,
    bytes: &[u8],
    table: &[RawChunk],
    failures: &mut Vec<String>,
) {
    let feat = table.iter().find(|c| &c.tag == b"FEAT");
    match (feat, &example.features) {
        (None, None) => {}
        (Some(chunk), Some(expected)) => match decode_chunk::<FeatureTable>(chunk, bytes) {
            Ok(table) if table.features == *expected => {}
            Ok(table) => failures.push(format!("FEAT: {:?} != {expected:?}", table.features)),
            Err(e) => failures.push(format!("FEAT: {e}")),
        },
        (Some(_), None) => failures.push("FEAT present but no features expected".to_owned()),
        (None, Some(_)) => failures.push("FEAT missing".to_owned()),
    }
}

fn decode_chunk<T: DeserializeOwned>(chunk: &RawChunk, bytes: &[u8]) -> Result<T, String> {
    let payload = chunk.decoded_payload(bytes)?;
    rmp_serde::from_slice(&payload).map_err(|e| e.to_string())
}

// --- 読んだ中身 ----------------------------------------------------------------------------------

/// 本体 (`Model` / `Module`) が、例の JSON の文書を読んだものと同じか。
fn check_body<T>(actual: &T, document: Option<&str>, what: &str, failures: &mut Vec<String>)
where
    T: PartialEq + serde::Serialize + FromJson,
{
    let Some(document) = document else {
        return failures.push(format!("{what}: no document in the example"));
    };
    let expected = T::from_json(&read_text(document)).unwrap_or_else(|e| panic!("{document}: {e}"));
    if *actual != expected {
        failures.push(format!(
            "{what} differs from {document}: {}",
            first_difference(actual, &expected)
        ));
    }
}

/// JSON の文書から読む型。
trait FromJson: Sized {
    fn from_json(json: &str) -> Result<Self, FormatError>;
}

impl FromJson for Model {
    fn from_json(json: &str) -> Result<Self, FormatError> {
        model_from_json(json)
    }
}

impl FromJson for Module {
    fn from_json(json: &str) -> Result<Self, FormatError> {
        module_from_json(json)
    }
}

/// 最初に食い違う場所 (JSON にしたときのパス)。
fn first_difference<T: serde::Serialize>(actual: &T, expected: &T) -> String {
    fn walk(path: String, a: &Value, e: &Value) -> Option<String> {
        match (a, e) {
            (Value::Object(a), Value::Object(e)) => {
                let keys: std::collections::BTreeSet<&String> = a.keys().chain(e.keys()).collect();
                keys.into_iter().find_map(|k| {
                    let (a, e) = (
                        a.get(k).unwrap_or(&Value::Null),
                        e.get(k).unwrap_or(&Value::Null),
                    );
                    walk(format!("{path}.{k}"), a, e)
                })
            }
            (Value::Array(a), Value::Array(e)) if a.len() == e.len() => a
                .iter()
                .zip(e)
                .enumerate()
                .find_map(|(i, (a, e))| walk(format!("{path}[{i}]"), a, e)),
            _ if a == e => None,
            _ => Some(format!("{path}: {a} != {e}")),
        }
    }
    let (a, e) = (
        serde_json::to_value(actual).unwrap_or(Value::Null),
        serde_json::to_value(expected).unwrap_or(Value::Null),
    );
    walk(String::new(), &a, &e).unwrap_or_else(|| "(no difference in JSON)".to_owned())
}

fn check_bundle_extras(
    example: &FileExample,
    bundle: &ugoe_format::Bundle,
    failures: &mut Vec<String>,
) {
    let physics = example
        .physics
        .as_deref()
        .map(|p| physics_from_json(&read_text(p)).unwrap_or_else(|e| panic!("{p}: {e}")));
    if bundle.physics != physics {
        failures.push(format!(
            "physics differs from {:?}: {}",
            example.physics,
            first_difference(&bundle.physics, &physics)
        ));
    }
    let motions: Vec<_> = example
        .motions
        .iter()
        .map(|p| motion_from_json(&read_text(p)).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect();
    if bundle.motions != motions {
        failures.push(format!(
            "motions differ from {:?}: {}",
            example.motions,
            first_difference(&bundle.motions, &motions)
        ));
    }
    let expressions: Vec<_> = example
        .expressions
        .iter()
        .map(|p| expression_from_json(&read_text(p)).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect();
    if bundle.expressions != expressions {
        failures.push(format!(
            "expressions differ from {:?}: {}",
            example.expressions,
            first_difference(&bundle.expressions, &expressions)
        ));
    }
}

/// メタデータと警告 (読めたファイルで共通)。
fn check_ok_common(
    example: &FileExample,
    meta: Option<&Value>,
    report: &ReadReport,
    failures: &mut Vec<String>,
) {
    if meta != example.meta.as_ref() {
        failures.push(format!("meta {meta:?} != {:?}", example.meta));
    }
    let expected: Vec<ReadWarning> = example
        .warnings
        .iter()
        .flat_map(|w| &w.unknown_features)
        .map(|id| ReadWarning::UnknownOptionalFeature(id.clone()))
        .collect();
    if report.warnings != expected {
        failures.push(format!("warnings {:?} != {expected:?}", report.warnings));
    }
    let features = example.features.clone().unwrap_or_default();
    if report.features != features {
        failures.push(format!(
            "reported features {:?} != {features:?}",
            report.features
        ));
    }
}

fn check_images<'a>(
    example: &FileExample,
    textures: &'a [TextureData],
    normal_map: impl Fn(usize) -> Option<&'a TextureData>,
    failures: &mut Vec<String>,
) {
    if textures.len() != example.textures.len() {
        return failures.push(format!(
            "texture count {} != {}",
            textures.len(),
            example.textures.len()
        ));
    }
    for (i, (texture, expected)) in textures.iter().zip(&example.textures).enumerate() {
        check_image(texture, expected, &format!("texture {i}"), failures);
        match (normal_map(i), &expected.normal_map) {
            (None, None) => {}
            (Some(actual), Some(expected)) => {
                check_image(
                    actual,
                    expected,
                    &format!("texture {i} normal map"),
                    failures,
                );
            }
            (actual, _) => failures.push(format!(
                "texture {i}: has normal map {} != expected {}",
                actual.is_some(),
                expected.normal_map.is_some()
            )),
        }
    }
}

fn check_image(
    actual: &TextureData,
    expected: &ExpectedImage,
    label: &str,
    failures: &mut Vec<String>,
) {
    if actual.format != TextureFormat::Png {
        return failures.push(format!("{label}: {:?}, not PNG", actual.format));
    }
    match decode_png(&actual.bytes) {
        Ok((width, height, rgba)) => {
            if (width, height) != (expected.width, expected.height) {
                failures.push(format!(
                    "{label}: size {width}x{height} != {}x{}",
                    expected.width, expected.height
                ));
            } else if rgba != expected.rgba {
                failures.push(format!("{label}: rgba {rgba:?} != {:?}", expected.rgba));
            }
        }
        Err(e) => failures.push(format!("{label}: {e}")),
    }
}

/// PNG を 8 bit の RGBA (乗算済みでない) に展開する。
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("PNG image is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".to_owned()),
    };
    Ok((info.width, info.height, rgba))
}

// --- 拒否 ----------------------------------------------------------------------------------------

/// 参照実装のエラーを、`spec/bundle.md` の Reading の手順の名前 (`spec/examples/README.md#file-examples`) にする。
fn reading_step(error: &FormatError, bytes: &[u8]) -> &'static str {
    use FormatError as E;
    match error {
        E::BadMagic | E::UnsupportedContainerVersion { .. } | E::UnsupportedFlags(_) => "header",
        E::Truncated if bytes.len() < 16 => "header",
        E::Truncated
        | E::UnsupportedChunkFlags { .. }
        | E::ChunkOutOfBounds { .. }
        | E::MisalignedChunk { .. }
        | E::OverlappingChunks { .. } => "chunk_table",
        E::UnsupportedFeatures(_) => "feature_declarations",
        E::DuplicateChunk(tag) if tag.0 == *b"FEAT" => "feature_declarations",
        E::UnsupportedVersion { .. } => "schema_version",
        E::UnexpectedChunk(_)
        | E::DuplicateChunk(_)
        | E::Decompress(_)
        | E::ReadLimitExceeded { .. }
        | E::TooLarge
        | E::Json(_)
        | E::MsgPackDecode(_) => "chunks",
        E::MissingChunk(_)
        | E::TextureCountMismatch { .. }
        | E::UnknownTextureFormat { .. }
        | E::TextureFormatMismatch { .. }
        | E::NormalMapCountMismatch { .. }
        | E::UnknownNormalMapFormat { .. }
        | E::NormalMapFormatMismatch { .. } => "counts",
        E::Invalid(_) | E::InvalidModule(_) | E::InvalidPhysics(_) => "validation",
        E::UnknownValues(_) | E::MsgPackEncode(_) => "(not a reading error)",
    }
}

fn check_error(
    example: &FileExample,
    error: &FormatError,
    bytes: &[u8],
    failures: &mut Vec<String>,
) {
    let step = reading_step(error, bytes);
    if Some(step) != example.error.as_deref() {
        failures.push(format!("error {step} ({error}) != {:?}", example.error));
        return;
    }
    if let FormatError::UnsupportedFeatures(ids) = error {
        let expected = example
            .unknown_required_features
            .clone()
            .unwrap_or_default();
        if *ids != expected {
            failures.push(format!("unknown required features {ids:?} != {expected:?}"));
        }
    }
}
