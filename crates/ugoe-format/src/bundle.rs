//! `.ugoe` バンドル (チャンク形式のバイナリコンテナ) の読み書き。
//! 仕様: `docs/spec/file-formats.md` の「バンドル (`.ugoe`)」、判断の経緯: ADR 0008。
//!
//! - ヘッダ・チャンク表はリトルエンディアン。チャンクは 16 バイト境界に揃える。
//! - `MODL` `PHYS` `MOTN` `EXPR` のペイロードは MessagePack (フィールド名付き)。
//! - `TEXR` は PNG / KTX2 のバイト列そのまま。`META` は JSON。
//! - `FEAT` は使っている機能の宣言 (MessagePack)。[`crate::features`] を参照。
//! - 読み込みは入力を信用しない。範囲外・重なり・切り詰めはすべてエラーにし、パニックしない。

use std::borrow::Cow;
use std::fmt;
use std::io::Read;

use serde::Serialize;
use serde::de::DeserializeOwned;
use ugoe_model::Model;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;

use crate::features::{self, FeatureDecl, FeatureTable};
use crate::{FormatError, MAGIC, check_schema_version};

/// コンテナ形式のバージョン (ヘッダの `version`)。スキーマのバージョンとは別。
pub const CONTAINER_VERSION: u32 = 0;

/// ヘッダのバイト数。
pub const HEADER_LEN: usize = 16;

/// チャンク表 1 項目のバイト数。
pub const CHUNK_ENTRY_LEN: usize = 24;

/// チャンクの開始位置の境界 (バイト)。
pub const CHUNK_ALIGN: usize = 16;

/// チャンクフラグ bit0: ペイロードが zstd で圧縮されている。
pub const CHUNK_FLAG_ZSTD: u32 = 1;

/// 圧縮チャンクを展開したときの上限 (バイト)。圧縮爆弾で無制限にメモリを使わないため。
pub const MAX_DECOMPRESSED_CHUNK_LEN: usize = 256 * 1024 * 1024;

/// MessagePack の入れ子の深さの上限。モデルの型の入れ子より十分大きく、スタックを使い切らない値。
const MAX_DEPTH: usize = 128;

/// チャンクのタグ (4 バイトの ASCII)。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkTag(pub [u8; 4]);

impl ChunkTag {
    /// `Model`。1 つだけ。
    pub const MODL: Self = Self(*b"MODL");
    /// `PhysicsSettings`。0〜1 個。
    pub const PHYS: Self = Self(*b"PHYS");
    /// `Motion`。複数可。
    pub const MOTN: Self = Self(*b"MOTN");
    /// `Expression`。複数可。
    pub const EXPR: Self = Self(*b"EXPR");
    /// テクスチャ 1 枚。`Model.textures` と同じ順。
    pub const TEXR: Self = Self(*b"TEXR");
    /// 自由なメタデータ (JSON)。0〜1 個。
    pub const META: Self = Self(*b"META");
    /// 使っている機能の宣言。0〜1 個 (宣言がなければ書かない)。
    pub const FEAT: Self = Self(*b"FEAT");
}

impl fmt::Display for ChunkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in &self.0 {
            if b.is_ascii_graphic() {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for ChunkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkTag({self})")
    }
}

/// テクスチャ画像の形式。`TEXR` チャンクの先頭のシグネチャで判別する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Png,
    Ktx2,
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const KTX2_SIGNATURE: [u8; 12] = [
    0xab, b'K', b'T', b'X', b' ', b'2', b'0', 0xbb, 0x0d, 0x0a, 0x1a, 0x0a,
];

impl TextureFormat {
    /// バイト列の先頭のシグネチャから形式を判別する。
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&PNG_SIGNATURE) {
            Some(Self::Png)
        } else if bytes.starts_with(&KTX2_SIGNATURE) {
            Some(Self::Ktx2)
        } else {
            None
        }
    }
}

/// テクスチャ 1 枚分の画像ファイルのバイト列 (デコードはしない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureData {
    pub format: TextureFormat,
    /// PNG または KTX2 ファイルの中身そのもの。
    pub bytes: Vec<u8>,
}

/// `.ugoe` バンドル 1 つ分の中身。
#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    /// チャンク表の順に並ぶ。名前は `Motion::name`。
    pub motions: Vec<Motion>,
    /// チャンク表の順に並ぶ。名前は `Expression::name`。
    pub expressions: Vec<Expression>,
    /// `model.textures` と同じ数・同じ順。
    pub textures: Vec<TextureData>,
    /// 作者やライセンスなどの自由なメタデータ。
    pub meta: Option<serde_json::Value>,
}

impl Bundle {
    /// モデルだけのバンドル (テクスチャなどは後から足す)。
    pub fn new(model: Model) -> Self {
        Self {
            model,
            physics: None,
            motions: Vec::new(),
            expressions: Vec::new(),
            textures: Vec::new(),
            meta: None,
        }
    }
}

/// 書き出しの設定。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WriteOptions {
    /// `TEXR` 以外のチャンクを zstd で圧縮する。テクスチャは既に圧縮済みなので対象外。
    pub compress: bool,
}

/// バンドルを既定の設定 (無圧縮) でバイト列に書き出す。
pub fn write_bundle(bundle: &Bundle) -> Result<Vec<u8>, FormatError> {
    write_bundle_with(bundle, WriteOptions::default())
}

/// バンドルをバイト列に書き出す。
///
/// テクスチャの数が `model.textures` と一致しない場合や、`TextureData::format` と
/// 中身のシグネチャが食い違う場合はエラーにする (読み込めないファイルを作らないため)。
/// 未知の値 (`Unknown`) を含む場合は [`FormatError::UnknownValues`] で拒否する
/// (元の値を失っているので、別の値として黙って書き出さないため)。
///
/// 使っている機能は [`features::detect`] で自動で宣言し、宣言があれば `FEAT` チャンクに書く。
pub fn write_bundle_with(bundle: &Bundle, options: WriteOptions) -> Result<Vec<u8>, FormatError> {
    write_with_features(bundle, options, &features::detect(bundle))
}

/// 機能の宣言 `declared` を指定して書き出す (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn write_with_features(
    bundle: &Bundle,
    options: WriteOptions,
    declared: &[FeatureDecl],
) -> Result<Vec<u8>, FormatError> {
    check_texture_count(&bundle.model, bundle.textures.len())?;
    let unknown = unknown_values(bundle);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }

    let mut chunks: Vec<(ChunkTag, u32, Cow<'_, [u8]>)> = Vec::new();
    let mut push_encoded = |tag: ChunkTag, raw: Vec<u8>| {
        if options.compress {
            let compressed = ruzstd::encoding::compress_to_vec(
                raw.as_slice(),
                ruzstd::encoding::CompressionLevel::Fastest,
            );
            chunks.push((tag, CHUNK_FLAG_ZSTD, Cow::Owned(compressed)));
        } else {
            chunks.push((tag, 0, Cow::Owned(raw)));
        }
    };

    if !declared.is_empty() {
        let table = FeatureTable {
            features: declared.to_vec(),
        };
        push_encoded(ChunkTag::FEAT, encode(&table)?);
    }
    push_encoded(ChunkTag::MODL, encode(&bundle.model)?);
    if let Some(physics) = &bundle.physics {
        push_encoded(ChunkTag::PHYS, encode(physics)?);
    }
    for motion in &bundle.motions {
        push_encoded(ChunkTag::MOTN, encode(motion)?);
    }
    for expression in &bundle.expressions {
        push_encoded(ChunkTag::EXPR, encode(expression)?);
    }
    if let Some(meta) = &bundle.meta {
        push_encoded(ChunkTag::META, serde_json::to_vec(meta)?);
    }
    for (index, texture) in bundle.textures.iter().enumerate() {
        if TextureFormat::detect(&texture.bytes) != Some(texture.format) {
            return Err(FormatError::TextureFormatMismatch { index });
        }
        chunks.push((ChunkTag::TEXR, 0, Cow::Borrowed(texture.bytes.as_slice())));
    }

    let chunk_count = u32::try_from(chunks.len()).map_err(|_| FormatError::TooLarge)?;
    let table_end = HEADER_LEN + CHUNK_ENTRY_LEN * chunks.len();

    // 先にチャンクの配置を決め、ヘッダ・チャンク表・本体の順に書く。
    let mut placements = Vec::with_capacity(chunks.len());
    let mut cursor = table_end;
    for (_, _, payload) in &chunks {
        let offset = align_up(cursor).ok_or(FormatError::TooLarge)?;
        placements.push(offset);
        cursor = offset
            .checked_add(payload.len())
            .ok_or(FormatError::TooLarge)?;
    }

    let mut out = Vec::with_capacity(cursor);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&chunk_count.to_le_bytes());
    for ((tag, flags, payload), &offset) in chunks.iter().zip(&placements) {
        out.extend_from_slice(&tag.0);
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&(offset as u64).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    }
    for ((_, _, payload), &offset) in chunks.iter().zip(&placements) {
        out.resize(offset, 0);
        out.extend_from_slice(payload);
    }
    Ok(out)
}

/// 読み込みの警告 ([`read_bundle_with_report`])。読み込みは成功しているが、表示が劣化しうる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadWarning {
    /// 知らない任意の機能。仕様の定めるフォールバックを適用して読んだ。
    UnknownOptionalFeature(String),
    /// 未知の値 (`Unknown` として読んだ enum の値)。その種類のフォールバックを適用する。
    /// `path` はチャンク (`MODL` や `MOTN#0`) とその中の場所。
    UnknownValue(String),
}

impl fmt::Display for ReadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOptionalFeature(id) => write!(f, "unknown optional feature {id}"),
            Self::UnknownValue(path) => write!(f, "unknown value at {path}"),
        }
    }
}

/// 読み込みの報告。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadReport {
    /// `FEAT` チャンクで宣言されていた機能 (知っているものも含む、宣言のまま)。
    pub features: Vec<FeatureDecl>,
    /// 警告 (知らない任意の機能と未知の値)。
    pub warnings: Vec<ReadWarning>,
}

/// バイト列からバンドルを読み込む。警告は捨てる ([`read_bundle_with_report`] を参照)。
pub fn read_bundle(bytes: &[u8]) -> Result<Bundle, FormatError> {
    read_bundle_with_report(bytes).map(|(bundle, _)| bundle)
}

/// バイト列からバンドルを読み込み、警告を含む報告も返す。
///
/// コンテナの整合性 (マジック・版・範囲・境界・重なり・必須チャンク) を検査したうえで、
/// 機能の宣言 (`FEAT`) を調べ、モデルのスキーマバージョンを検査し、最後に
/// `ugoe_model::validate::validate` を通す。
/// 検証に失敗したモデルは下流 (変形・描画) でパニックしうるため、ここで弾く。
///
/// - 知らない必須の機能があれば [`FormatError::UnsupportedFeatures`] で拒否する
///   (ペイロードのデコードより先に調べるので、知らない形のデータでもこのエラーになる)。
/// - 知らない任意の機能と未知の値は、[`ReadWarning`] として報告して読み込みを続ける。
/// - 未知のタグのチャンクは読み飛ばす。
pub fn read_bundle_with_report(bytes: &[u8]) -> Result<(Bundle, ReadReport), FormatError> {
    read_with_known(bytes, |id| features::find(id).is_some())
}

/// 知っている機能の判定 `known` を指定して読む (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn read_with_known(
    bytes: &[u8],
    known: impl Fn(&str) -> bool,
) -> Result<(Bundle, ReadReport), FormatError> {
    let entries = read_table(bytes)?;

    // 機能の宣言は、ほかのチャンクをデコードする前に調べる。
    let mut feat = entries.iter().filter(|e| e.tag == ChunkTag::FEAT);
    let declared = match (feat.next(), feat.next()) {
        (None, _) => Vec::new(),
        (Some(entry), None) => decode::<FeatureTable>(&entry.payload(bytes)?)?.features,
        (Some(_), Some(entry)) => return Err(FormatError::DuplicateChunk(entry.tag)),
    };
    let mut warnings: Vec<ReadWarning> = features::check(&declared, known)
        .map_err(FormatError::UnsupportedFeatures)?
        .into_iter()
        .map(ReadWarning::UnknownOptionalFeature)
        .collect();

    let mut model = None;
    let mut physics = None;
    let mut meta = None;
    let mut motions = Vec::new();
    let mut expressions = Vec::new();
    let mut textures = Vec::new();

    for entry in &entries {
        match entry.tag {
            ChunkTag::MODL => {
                if model.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                let decoded: Model = decode(&entry.payload(bytes)?)?;
                check_schema_version(&decoded)?;
                model = Some(decoded);
            }
            ChunkTag::PHYS => {
                if physics.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                physics = Some(decode(&entry.payload(bytes)?)?);
            }
            ChunkTag::META => {
                if meta.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                meta = Some(serde_json::from_slice(&entry.payload(bytes)?)?);
            }
            ChunkTag::MOTN => motions.push(decode(&entry.payload(bytes)?)?),
            ChunkTag::EXPR => expressions.push(decode(&entry.payload(bytes)?)?),
            ChunkTag::TEXR => {
                let payload = entry.payload(bytes)?.into_owned();
                let format =
                    TextureFormat::detect(&payload).ok_or(FormatError::UnknownTextureFormat {
                        index: textures.len(),
                    })?;
                textures.push(TextureData {
                    format,
                    bytes: payload,
                });
            }
            // 機能の宣言は先に読んだ。前方互換のため、未知のタグは読み飛ばす。
            _ => {}
        }
    }

    let model = model.ok_or(FormatError::MissingChunk(ChunkTag::MODL))?;
    check_texture_count(&model, textures.len())?;
    let errors = ugoe_model::validate::validate(&model);
    if !errors.is_empty() {
        return Err(FormatError::Invalid(errors));
    }

    let bundle = Bundle {
        model,
        physics,
        motions,
        expressions,
        textures,
        meta,
    };
    warnings.extend(
        unknown_values(&bundle)
            .into_iter()
            .map(ReadWarning::UnknownValue),
    );
    Ok((
        bundle,
        ReadReport {
            features: declared,
            warnings,
        },
    ))
}

/// バンドルの中の未知の値の場所 (チャンクの名前付き) をすべて返す。
fn unknown_values(bundle: &Bundle) -> Vec<String> {
    use ugoe_model::unknown;

    let mut found: Vec<String> = unknown::in_model(&bundle.model)
        .into_iter()
        .map(|path| format!("{}: {path}", ChunkTag::MODL))
        .collect();
    if let Some(physics) = &bundle.physics {
        let paths = unknown::in_physics(physics);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}: {p}", ChunkTag::PHYS)),
        );
    }
    for (index, motion) in bundle.motions.iter().enumerate() {
        let paths = unknown::in_motion(motion);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}#{index}: {p}", ChunkTag::MOTN)),
        );
    }
    for (index, expression) in bundle.expressions.iter().enumerate() {
        let paths = unknown::in_expression(expression);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}#{index}: {p}", ChunkTag::EXPR)),
        );
    }
    found
}

/// 検査済みのチャンク表の 1 項目。
#[derive(Debug)]
struct Entry {
    tag: ChunkTag,
    flags: u32,
    start: usize,
    end: usize,
}

impl Entry {
    /// ペイロードを取り出す (圧縮されていれば展開する)。範囲は `read_table` で検査済み。
    fn payload<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, [u8]>, FormatError> {
        let raw = bytes
            .get(self.start..self.end)
            .ok_or(FormatError::ChunkOutOfBounds { tag: self.tag })?;
        if self.flags & CHUNK_FLAG_ZSTD == 0 {
            return Ok(Cow::Borrowed(raw));
        }
        let decoder = ruzstd::decoding::StreamingDecoder::new(raw)
            .map_err(|e| FormatError::Decompress(e.to_string()))?;
        let mut out = Vec::new();
        decoder
            .take(MAX_DECOMPRESSED_CHUNK_LEN as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|e| FormatError::Decompress(e.to_string()))?;
        if out.len() > MAX_DECOMPRESSED_CHUNK_LEN {
            return Err(FormatError::DecompressedTooLarge { tag: self.tag });
        }
        Ok(Cow::Owned(out))
    }
}

/// ヘッダとチャンク表を読み、各チャンクの範囲を検査する。
fn read_table(bytes: &[u8]) -> Result<Vec<Entry>, FormatError> {
    let header = bytes.get(..HEADER_LEN).ok_or(FormatError::Truncated)?;
    if header[0..4] != MAGIC {
        return Err(FormatError::BadMagic);
    }
    let version = read_u32(header, 4)?;
    if version != CONTAINER_VERSION {
        return Err(FormatError::UnsupportedContainerVersion {
            found: version,
            supported: CONTAINER_VERSION,
        });
    }
    let header_flags = read_u32(header, 8)?;
    if header_flags != 0 {
        return Err(FormatError::UnsupportedFlags(header_flags));
    }
    let chunk_count = read_u32(header, 12)? as usize;

    // 表が入力に収まることを先に確かめる。これで表の大きさ (= 確保量) は入力長で抑えられる。
    let table_end = chunk_count
        .checked_mul(CHUNK_ENTRY_LEN)
        .and_then(|len| len.checked_add(HEADER_LEN))
        .ok_or(FormatError::Truncated)?;
    let table = bytes
        .get(HEADER_LEN..table_end)
        .ok_or(FormatError::Truncated)?;

    let mut entries = Vec::with_capacity(chunk_count);
    for raw in table.as_chunks::<CHUNK_ENTRY_LEN>().0 {
        let tag = ChunkTag([raw[0], raw[1], raw[2], raw[3]]);
        let flags = read_u32(raw, 4)?;
        let offset = read_u64(raw, 8)?;
        let length = read_u64(raw, 16)?;
        if flags & !CHUNK_FLAG_ZSTD != 0 {
            return Err(FormatError::UnsupportedChunkFlags { tag, flags });
        }
        let start = usize::try_from(offset).map_err(|_| FormatError::ChunkOutOfBounds { tag })?;
        let end = usize::try_from(length)
            .ok()
            .and_then(|len| start.checked_add(len))
            .filter(|&end| end <= bytes.len())
            .ok_or(FormatError::ChunkOutOfBounds { tag })?;
        if start % CHUNK_ALIGN != 0 {
            return Err(FormatError::MisalignedChunk { tag, offset });
        }
        if start < table_end {
            return Err(FormatError::OverlappingChunks { tag });
        }
        entries.push(Entry {
            tag,
            flags,
            start,
            end,
        });
    }

    // チャンク同士の重なりを検査する (表の順序は保ったまま、位置順に並べた写しで調べる)。
    let mut ranges: Vec<(usize, usize, ChunkTag)> =
        entries.iter().map(|e| (e.start, e.end, e.tag)).collect();
    ranges.sort_unstable_by_key(|&(start, end, _)| (start, end));
    for pair in ranges.windows(2) {
        if let [(_, prev_end, _), (start, _, tag)] = pair
            && start < prev_end
        {
            return Err(FormatError::OverlappingChunks { tag: *tag });
        }
    }

    Ok(entries)
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, FormatError> {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(FormatError::Truncated)
}

fn read_u64(bytes: &[u8], at: usize) -> Result<u64, FormatError> {
    bytes
        .get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(FormatError::Truncated)
}

fn align_up(value: usize) -> Option<usize> {
    value
        .checked_add(CHUNK_ALIGN - 1)
        .map(|v| v / CHUNK_ALIGN * CHUNK_ALIGN)
}

fn check_texture_count(model: &Model, actual: usize) -> Result<(), FormatError> {
    if model.textures.len() == actual {
        Ok(())
    } else {
        Err(FormatError::TextureCountMismatch {
            expected: model.textures.len(),
            actual,
        })
    }
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, FormatError> {
    Ok(rmp_serde::to_vec_named(value)?)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, FormatError> {
    let mut de = rmp_serde::Deserializer::from_read_ref(bytes);
    de.set_max_depth(MAX_DEPTH);
    Ok(T::deserialize(&mut de)?)
}

#[cfg(test)]
mod tests {
    use ugoe_core::Vec2;
    use ugoe_model::deformer::{Deformer, DeformerKind, RotationDeformer, RotationKeyform};
    use ugoe_model::expression::{ExpressionOp, ExpressionParameter};
    use ugoe_model::motion::{Curve, CurveTarget, MotionEvent, Point, Segment};
    use ugoe_model::physics::{
        NormalizationRange, PhysicsInput, PhysicsNormalization, PhysicsOutput, PhysicsParticle,
        PhysicsRig, PhysicsSource, PhysicsTarget,
    };
    use ugoe_model::{DepthRotation, KeyformGrid, KeyformInference, KeyformOrigin, TextureRef};

    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(b"not really a png body");
        bytes
    }

    fn ktx2() -> Vec<u8> {
        let mut bytes = KTX2_SIGNATURE.to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        bytes
    }

    fn physics() -> PhysicsSettings {
        let range = NormalizationRange {
            min: -10.0,
            default: 0.0,
            max: 10.0,
        };
        PhysicsSettings {
            gravity: Vec2::new(0.0, -1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: vec![PhysicsRig {
                id: "Hair".into(),
                name: "Hair".into(),
                inputs: vec![PhysicsInput {
                    parameter: "X".into(),
                    source: PhysicsSource::X,
                    weight: 100.0,
                    reflect: false,
                }],
                outputs: vec![PhysicsOutput {
                    target: PhysicsTarget::Parameter { id: "X".into() },
                    particle: 1,
                    source: PhysicsSource::Angle,
                    scale: 1.5,
                    weight: 100.0,
                    reflect: true,
                }],
                particles: vec![
                    PhysicsParticle {
                        position: Vec2::ZERO,
                        mobility: 1.0,
                        delay: 1.0,
                        acceleration: 1.0,
                        radius: 0.0,
                        collision_radius: 0.0,
                    },
                    PhysicsParticle {
                        position: Vec2::new(0.0, 10.0),
                        mobility: 0.9,
                        delay: 0.8,
                        acceleration: 1.5,
                        radius: 10.0,
                        collision_radius: 0.0,
                    },
                ],
                normalization: PhysicsNormalization {
                    position: range,
                    angle: range,
                },
                colliders: vec![],
                frame: None,
            }],
            colliders: vec![],
        }
    }

    fn motion() -> Motion {
        let p = |t, v| Point { t, v };
        Motion {
            name: "idle".into(),
            duration: 2.0,
            looping: true,
            fade_in: 0.5,
            fade_out: 0.25,
            curves: vec![
                Curve {
                    target: CurveTarget::Parameter("X".into()),
                    segments: vec![
                        Segment::Start { p: p(0.0, 0.0) },
                        Segment::Linear { to: p(0.5, 1.0) },
                        Segment::Bezier {
                            c1: p(0.7, 1.0),
                            c2: p(0.8, -1.0),
                            to: p(1.0, -1.0),
                        },
                        Segment::Stepped { to: p(1.5, 0.0) },
                        Segment::InverseStepped { to: p(2.0, 1.0) },
                    ],
                    fade_in: Some(0.1),
                    fade_out: None,
                },
                Curve {
                    target: CurveTarget::PartOpacity("Root".into()),
                    segments: vec![Segment::Start { p: p(0.0, 1.0) }],
                    fade_in: None,
                    fade_out: None,
                },
                Curve {
                    target: CurveTarget::EffectGroup("EyeBlink".into()),
                    segments: vec![Segment::Start { p: p(0.0, 1.0) }],
                    fade_in: None,
                    fade_out: Some(0.2),
                },
            ],
            events: vec![MotionEvent {
                t: 1.0,
                value: "wave".into(),
            }],
        }
    }

    fn expression() -> Expression {
        Expression {
            name: "smile".into(),
            fade_in: 0.3,
            fade_out: 0.0,
            parameters: vec![ExpressionParameter {
                parameter: "X".into(),
                value: 0.5,
                op: ExpressionOp::Multiply,
            }],
        }
    }

    /// モデル・物理・モーション・表情・テクスチャ (PNG と KTX2)・メタデータをすべて持つバンドル。
    fn full_bundle() -> Bundle {
        let mut model = ugoe_model::fixtures::single_quad();
        // 内部タグ付き enum (`DeformerKind`) の往復も確かめるため、デフォーマを 1 つ足す。
        model.deformers.push(Deformer {
            id: "Rot".into(),
            name: "Rot".into(),
            part: "Root".into(),
            parent: None,
            kind: DeformerKind::Rotation(RotationDeformer {
                keyforms: KeyformGrid::constant(RotationKeyform {
                    origin: Vec2::new(128.0, 128.0),
                    angle: 15.0,
                    scale: 1.0,
                    reflect_x: false,
                    reflect_y: true,
                    opacity: 1.0,
                }),
            }),
            mirror: Some("Rot".into()),
            mirror_axis: Some(0.0),
        });
        // オーサリング用のデータ (セルの由来・推定の設定・深度) も往復させる。
        let quad = &mut model.meshes[0];
        quad.keyforms.origins = vec![
            KeyformOrigin::Derived,
            KeyformOrigin::Authored,
            KeyformOrigin::Authored,
        ];
        quad.keyforms.inference = Some(KeyformInference::DepthRotation(DepthRotation {
            angle_scale: 0.5,
            pivot: Vec2::new(0.0, 32.0),
            pivot_depth: -16.0,
            yaw: Some("X".into()),
            pitch: None,
        }));
        quad.depth = Some(vec![0.0, 1.0, 2.0, 3.0]);
        model.textures.push(TextureRef {
            name: "compressed".into(),
            width: 4,
            height: 4,
        });
        Bundle {
            model,
            physics: Some(physics()),
            motions: vec![
                motion(),
                Motion {
                    name: "empty".into(),
                    duration: 0.0,
                    looping: false,
                    fade_in: 0.0,
                    fade_out: 0.0,
                    curves: vec![],
                    events: vec![],
                },
            ],
            expressions: vec![expression()],
            textures: vec![
                TextureData {
                    format: TextureFormat::Png,
                    bytes: png(),
                },
                TextureData {
                    format: TextureFormat::Ktx2,
                    bytes: ktx2(),
                },
            ],
            meta: Some(serde_json::json!({ "author": "ugoe", "license": "CC0" })),
        }
    }

    fn model_only() -> Bundle {
        let mut bundle = Bundle::new(ugoe_model::fixtures::single_quad());
        bundle.textures.push(TextureData {
            format: TextureFormat::Png,
            bytes: png(),
        });
        bundle
    }

    /// 書き出しと同じ配置規則で、任意のチャンク列からコンテナを組み立てる (壊れた入力を作る用)。
    fn assemble(chunks: &[(ChunkTag, u32, Vec<u8>)]) -> Vec<u8> {
        let table_end = HEADER_LEN + CHUNK_ENTRY_LEN * chunks.len();
        let mut offsets = Vec::new();
        let mut cursor = table_end;
        for (_, _, payload) in chunks {
            let offset = align_up(cursor).unwrap();
            offsets.push(offset);
            cursor = offset + payload.len();
        }
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
        for ((tag, flags, payload), offset) in chunks.iter().zip(&offsets) {
            out.extend_from_slice(&tag.0);
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&(*offset as u64).to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        }
        for ((_, _, payload), offset) in chunks.iter().zip(&offsets) {
            out.resize(*offset, 0);
            out.extend_from_slice(payload);
        }
        out
    }

    fn model_chunk(model: &Model) -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::MODL, 0, encode(model).unwrap())
    }

    fn texture_chunk() -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::TEXR, 0, png())
    }

    /// チャンク表 `index` 番目の項目の、`field` バイト目から始まる u64 を書き換える。
    fn patch_u64(bytes: &mut [u8], index: usize, field: usize, value: u64) {
        let at = HEADER_LEN + CHUNK_ENTRY_LEN * index + field;
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn roundtrip_full_bundle() {
        let bundle = full_bundle();
        let bytes = write_bundle(&bundle).unwrap();
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
    }

    #[test]
    fn roundtrip_compressed_bundle() {
        let bundle = full_bundle();
        let bytes = write_bundle_with(&bundle, WriteOptions { compress: true }).unwrap();
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
        // テクスチャ以外のチャンクには圧縮フラグが立つ。
        let entries = read_table(&bytes).unwrap();
        for entry in entries {
            let expected = if entry.tag == ChunkTag::TEXR {
                0
            } else {
                CHUNK_FLAG_ZSTD
            };
            assert_eq!(entry.flags, expected, "{}", entry.tag);
        }
    }

    #[test]
    fn roundtrip_model_only() {
        let bundle = model_only();
        assert_eq!(
            read_bundle(&write_bundle(&bundle).unwrap()).unwrap(),
            bundle
        );
    }

    /// アバタープロファイル (基準の機能 `avatar.profile`) は `MODL` に入り、機能として宣言しない。
    #[test]
    fn roundtrip_avatar_profile() {
        let mut bundle = model_only();
        bundle.model = ugoe_model::fixtures::avatar_profile();
        assert!(features::detect(&bundle).is_empty());
        for compress in [false, true] {
            let bytes = write_bundle_with(&bundle, WriteOptions { compress }).unwrap();
            let (read, report) = read_bundle_with_report(&bytes).unwrap();
            assert_eq!(read, bundle);
            assert!(report.features.is_empty());
        }
        let json = crate::model_to_json(&bundle.model).unwrap();
        assert_eq!(crate::model_from_json(&json).unwrap(), bundle.model);
    }

    #[test]
    fn written_layout_follows_spec() {
        let bytes = write_bundle(&full_bundle()).unwrap();
        assert_eq!(&bytes[0..4], b"UGOE");
        assert_eq!(read_u32(&bytes, 4).unwrap(), CONTAINER_VERSION);
        assert_eq!(read_u32(&bytes, 8).unwrap(), 0);
        // MODL + PHYS + MOTN×2 + EXPR + META + TEXR×2
        assert_eq!(read_u32(&bytes, 12).unwrap(), 8);
        let entries = read_table(&bytes).unwrap();
        assert_eq!(entries[0].tag, ChunkTag::MODL);
        for entry in &entries {
            assert_eq!(entry.start % CHUNK_ALIGN, 0);
        }
    }

    #[test]
    fn unknown_chunks_are_skipped() {
        let model = ugoe_model::fixtures::single_quad();
        let bytes = assemble(&[
            (ChunkTag(*b"XTRA"), 0, vec![0xff; 5]),
            model_chunk(&model),
            // 未知のタグのチャンクは展開もしない (圧縮フラグ付きの壊れた中身でも読み飛ばす)。
            (ChunkTag(*b"ZZZZ"), CHUNK_FLAG_ZSTD, b"garbage".to_vec()),
            texture_chunk(),
        ]);
        let bundle = read_bundle(&bytes).unwrap();
        assert_eq!(bundle, model_only());
    }

    fn assert_err(bytes: &[u8], check: impl Fn(&FormatError) -> bool) {
        match read_bundle(bytes) {
            Ok(_) => panic!("expected an error"),
            Err(e) => assert!(check(&e), "unexpected error: {e:?}"),
        }
    }

    #[test]
    fn rejects_empty_and_truncated_input() {
        assert_err(&[], |e| matches!(e, FormatError::Truncated));
        let bytes = write_bundle(&full_bundle()).unwrap();
        // 末尾を 1 バイトでも欠いたら読めない (最後のチャンクが範囲外になる)。
        for len in 0..bytes.len() {
            assert!(
                read_bundle(&bytes[..len]).is_err(),
                "prefix {len} was accepted"
            );
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[0] = b'X';
        assert_err(&bytes, |e| matches!(e, FormatError::BadMagic));
    }

    #[test]
    fn rejects_unknown_container_version_and_flags() {
        let good = write_bundle(&model_only()).unwrap();
        let mut bytes = good.clone();
        bytes[4] = 1;
        assert_err(&bytes, |e| {
            matches!(e, FormatError::UnsupportedContainerVersion { found: 1, .. })
        });
        let mut bytes = good;
        bytes[8] = 1;
        assert_err(&bytes, |e| matches!(e, FormatError::UnsupportedFlags(1)));
    }

    #[test]
    fn rejects_unknown_chunk_flags() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[HEADER_LEN + 4] = 0b10;
        assert_err(&bytes, |e| {
            matches!(e, FormatError::UnsupportedChunkFlags { flags: 2, .. })
        });
    }

    #[test]
    fn rejects_huge_chunk_count() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_err(&bytes, |e| matches!(e, FormatError::Truncated));
    }

    #[test]
    fn rejects_out_of_range_offsets_and_lengths() {
        let good = write_bundle(&model_only()).unwrap();
        for (field, value) in [(8, u64::MAX), (8, 1 << 40), (16, u64::MAX), (16, 1 << 40)] {
            let mut bytes = good.clone();
            patch_u64(&mut bytes, 0, field, value);
            assert_err(
                &bytes,
                |e| matches!(e, FormatError::ChunkOutOfBounds { tag } if *tag == ChunkTag::MODL),
            );
        }
    }

    #[test]
    fn rejects_misaligned_chunk() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        let entries = read_table(&bytes).unwrap();
        patch_u64(&mut bytes, 1, 8, entries[1].start as u64 + 1);
        patch_u64(
            &mut bytes,
            1,
            16,
            (entries[1].end - entries[1].start - 1) as u64,
        );
        assert_err(&bytes, |e| matches!(e, FormatError::MisalignedChunk { .. }));
    }

    #[test]
    fn rejects_overlapping_chunks() {
        let good = write_bundle(&model_only()).unwrap();
        let entries = read_table(&good).unwrap();

        // 2 つのチャンクが同じ位置を指す。
        let mut bytes = good.clone();
        patch_u64(&mut bytes, 1, 8, entries[0].start as u64);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::OverlappingChunks { .. })
        });

        // チャンクがヘッダ・チャンク表に重なる。
        let mut bytes = good;
        patch_u64(&mut bytes, 0, 8, 0);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::OverlappingChunks { .. })
        });
    }

    #[test]
    fn rejects_missing_and_duplicate_chunks() {
        let model = ugoe_model::fixtures::single_quad();
        assert_err(&assemble(&[texture_chunk()]), |e| {
            matches!(e, FormatError::MissingChunk(ChunkTag::MODL))
        });
        assert_err(
            &assemble(&[model_chunk(&model), model_chunk(&model), texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::MODL)),
        );
        let phys = (ChunkTag::PHYS, 0, encode(&physics()).unwrap());
        assert_err(
            &assemble(&[model_chunk(&model), phys.clone(), phys, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::PHYS)),
        );
    }

    #[test]
    fn rejects_texture_problems() {
        let model = ugoe_model::fixtures::single_quad();
        assert_err(&assemble(&[model_chunk(&model)]), |e| {
            matches!(
                e,
                FormatError::TextureCountMismatch {
                    expected: 1,
                    actual: 0
                }
            )
        });
        assert_err(
            &assemble(&[model_chunk(&model), (ChunkTag::TEXR, 0, b"GIF89a".to_vec())]),
            |e| matches!(e, FormatError::UnknownTextureFormat { index: 0 }),
        );
    }

    #[test]
    fn rejects_broken_payloads() {
        let model = ugoe_model::fixtures::single_quad();
        // MessagePack として壊れている。
        assert_err(
            &assemble(&[(ChunkTag::MODL, 0, vec![0xc1, 0x00]), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        // 途中で切れた MessagePack。
        let mut short = encode(&model).unwrap();
        short.truncate(short.len() / 2);
        assert_err(
            &assemble(&[(ChunkTag::MODL, 0, short), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        // zstd として壊れている。
        assert_err(
            &assemble(&[
                (ChunkTag::MODL, CHUNK_FLAG_ZSTD, b"not zstd".to_vec()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::Decompress(_)),
        );
        // META が JSON でない。
        assert_err(
            &assemble(&[
                model_chunk(&model),
                (ChunkTag::META, 0, b"{".to_vec()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::Json(_)),
        );
    }

    #[test]
    fn rejects_newer_schema_version() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.schema_version = ugoe_model::SCHEMA_VERSION + 1;
        assert_err(&assemble(&[model_chunk(&model), texture_chunk()]), |e| {
            matches!(e, FormatError::UnsupportedVersion { .. })
        });
    }

    #[test]
    fn rejects_invalid_model() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.meshes[0].indices.push(0);
        assert_err(
            &assemble(&[model_chunk(&model), texture_chunk()]),
            |e| matches!(e, FormatError::Invalid(errors) if !errors.is_empty()),
        );
    }

    #[test]
    fn single_byte_corruption_never_panics() {
        let good = write_bundle_with(&full_bundle(), WriteOptions { compress: true }).unwrap();
        for i in 0..good.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut bytes = good.clone();
                bytes[i] ^= flip;
                let _ = read_bundle(&bytes);
            }
        }
    }

    fn test_features() -> Vec<FeatureDecl> {
        vec![
            FeatureDecl::optional("test.optional"),
            FeatureDecl::required("test.required"),
        ]
    }

    #[test]
    fn baseline_bundles_have_no_feat_chunk() {
        let bytes = write_bundle(&full_bundle()).unwrap();
        let entries = read_table(&bytes).unwrap();
        assert!(entries.iter().all(|e| e.tag != ChunkTag::FEAT));
        let (_, report) = read_bundle_with_report(&bytes).unwrap();
        assert_eq!(report, ReadReport::default());
    }

    #[test]
    fn roundtrip_feature_declarations() {
        // 試験用の機能 ID を知っているランタイムとして読む。
        let known = |id: &str| id.starts_with("test.");
        for compress in [false, true] {
            let bundle = full_bundle();
            let bytes =
                write_with_features(&bundle, WriteOptions { compress }, &test_features()).unwrap();
            assert_eq!(read_table(&bytes).unwrap()[0].tag, ChunkTag::FEAT);
            let (read, report) = read_with_known(&bytes, known).unwrap();
            assert_eq!(read, bundle);
            assert_eq!(report.features, test_features());
            assert!(report.warnings.is_empty());
        }
    }

    #[test]
    fn unknown_required_feature_is_rejected_with_ids() {
        let bytes =
            write_with_features(&model_only(), WriteOptions::default(), &test_features()).unwrap();
        assert_err(
            &bytes,
            |e| matches!(e, FormatError::UnsupportedFeatures(ids) if ids == &["test.required"]),
        );
        // 知らない形のモデル (新しい版) でも、デコードより先に機能の宣言で拒否する。
        let feat = FeatureTable {
            features: vec![
                FeatureDecl::required("test.a"),
                FeatureDecl::required("test.b"),
            ],
        };
        let bytes = assemble(&[
            (ChunkTag::MODL, 0, b"future payload".to_vec()),
            (ChunkTag::FEAT, 0, encode(&feat).unwrap()),
        ]);
        assert_err(
            &bytes,
            |e| matches!(e, FormatError::UnsupportedFeatures(ids) if ids == &["test.a", "test.b"]),
        );
    }

    #[test]
    fn unknown_optional_feature_loads_with_warnings() {
        let bundle = model_only();
        let declared = [
            FeatureDecl::optional("test.optional"),
            // 知っている機能 (基準の機能) の宣言は警告にしない。
            FeatureDecl::optional("mask.intersection"),
        ];
        let bytes = write_with_features(&bundle, WriteOptions::default(), &declared).unwrap();
        let (read, report) = read_bundle_with_report(&bytes).unwrap();
        assert_eq!(read, bundle);
        assert_eq!(report.features, declared);
        assert_eq!(
            report.warnings,
            [ReadWarning::UnknownOptionalFeature("test.optional".into())]
        );
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
    }

    #[test]
    fn rejects_duplicate_feat_chunks() {
        let model = ugoe_model::fixtures::single_quad();
        let feat = (ChunkTag::FEAT, 0, encode(&FeatureTable::default()).unwrap());
        assert_err(
            &assemble(&[feat.clone(), model_chunk(&model), feat, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::FEAT)),
        );
    }

    /// 新しい版で増えた値 (未知のタグとその中身) を含むバンドルを、MessagePack で組み立てる。
    fn future_bundle_bytes() -> Vec<u8> {
        use serde_json::json;

        let mut model = serde_json::to_value(ugoe_model::fixtures::single_quad()).unwrap();
        model["meshes"][0]["blend"] = json!("overlay");
        model["meshes"][0]["mask_mode"] = json!("xor");
        model["deformers"] = json!([{
            "id": "Bend",
            "name": "Bend",
            "part": "Root",
            "parent": null,
            "kind": { "type": "bezier", "handles": [[0.0, 1.0]], "extra": { "a": 1 } },
        }]);
        model["meshes"][0]["parent"] = json!("Bend");
        // 構造体の未知のフィールドも無視する。
        model["meshes"][0]["future_field"] = json!({ "nested": [1, 2, 3] });

        let mut physics = serde_json::to_value(physics()).unwrap();
        physics["rigs"][0]["outputs"][0]["source"] = json!("velocity");
        let mut motion = serde_json::to_value(motion()).unwrap();
        motion["curves"][1]["target"] = json!({ "kind": "camera", "id": { "zoom": 2 } });
        motion["curves"][0]["segments"][2] =
            json!({ "type": "hermite", "to": { "t": 1.0, "v": 0.0 } });
        let mut expression = serde_json::to_value(expression()).unwrap();
        expression["parameters"][0]["op"] = json!("screen");

        let msgpack = |value: &serde_json::Value| rmp_serde::to_vec_named(value).unwrap();
        assemble(&[
            (ChunkTag::MODL, 0, msgpack(&model)),
            (ChunkTag::PHYS, 0, msgpack(&physics)),
            (ChunkTag::MOTN, 0, msgpack(&motion)),
            (ChunkTag::EXPR, 0, msgpack(&expression)),
            texture_chunk(),
        ])
    }

    #[test]
    fn unknown_enum_tags_in_msgpack_load_with_warnings() {
        let (bundle, report) = read_bundle_with_report(&future_bundle_bytes()).unwrap();
        let mesh = &bundle.model.meshes[0];
        assert_eq!(mesh.blend, ugoe_model::BlendMode::Unknown);
        assert_eq!(mesh.mask_mode, ugoe_model::MaskMode::Unknown);
        assert_eq!(bundle.model.deformers[0].kind, DeformerKind::Unknown);
        assert_eq!(bundle.motions[0].curves[1].target, CurveTarget::Unknown);
        assert_eq!(bundle.motions[0].curves[0].segments[2], Segment::Unknown);
        let warnings: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
        assert_eq!(
            warnings,
            [
                "unknown value at MODL: deformers[Bend].kind",
                "unknown value at MODL: meshes[Quad].blend",
                "unknown value at MODL: meshes[Quad].mask_mode",
                "unknown value at PHYS: rigs[Hair].outputs[0].source",
                "unknown value at MOTN#0: curves[0].segments[2]",
                "unknown value at MOTN#0: curves[1].target",
                "unknown value at EXPR#0: parameters[0].op",
            ]
        );
    }

    #[test]
    fn unknown_values_are_not_written_back() {
        let (bundle, _) = read_bundle_with_report(&future_bundle_bytes()).unwrap();
        match write_bundle(&bundle) {
            Err(FormatError::UnknownValues(paths)) => assert_eq!(paths.len(), 7),
            other => panic!("unexpected result: {other:?}"),
        }
        assert!(matches!(
            crate::model_to_json(&bundle.model),
            Err(FormatError::UnknownValues(_))
        ));
    }

    #[test]
    fn write_rejects_inconsistent_textures() {
        let mut bundle = model_only();
        bundle.textures[0].format = TextureFormat::Ktx2;
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::TextureFormatMismatch { index: 0 })
        ));
        bundle.textures.clear();
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::TextureCountMismatch { .. })
        ));
    }
}
