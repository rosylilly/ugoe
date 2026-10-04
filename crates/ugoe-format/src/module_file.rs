//! 着せ替えのモジュール (`.ugoemod`) の読み書き。
//! 仕様: `docs/spec/file-formats.md` の「モジュール (`.ugoemod`)」と `docs/spec/modules.md`。
//!
//! - コンテナはバンドル (`.ugoe`) と同じ ([`crate::bundle`])。`MODL` の代わりに `MODU` を 1 つ持つ。
//! - `TEXR` と `NRML` はバンドルと同じく、`Module::textures` の順 (法線マップは `normal_map` を持つものの順) に並べる。
//! - `MODL` `PHYS` `MOTN` `EXPR` はバンドル用のチャンクなので置けない ([`FormatError::UnexpectedChunk`])。
//!   モジュールの物理は `MODU` の中 (`Module::physics`) に入る。
//! - 読み込みは入力を信用しない。範囲外・重なり・切り詰めはすべてエラーにし、パニックしない。

use ugoe_model::module::Module;

use crate::bundle::{
    ChunkTag, ChunkWriter, Container, Payloads, ReadLimits, ReadReport, ReadWarning, TextureData,
    WriteOptions, check_normal_map_count, check_schema_version_first, check_texture_count, decode,
    encode, normal_map_of, read_container, read_normal_map, read_texture,
};
use crate::features::{self, FeatureDecl};
use crate::{FormatError, check_schema_version};

/// `.ugoemod` ファイル 1 つ分の中身: 着せ替えのモジュールと、その画像。
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleFile {
    /// 着せ替えのモジュール (`MODU` チャンク)。
    pub module: Module,
    /// `module.textures` と同じ数・同じ順。
    pub textures: Vec<TextureData>,
    /// 法線マップ (`NRML` チャンク)。`module.textures` のうち `normal_map` が `Some` のものと
    /// 同じ数・同じ順。テクスチャの番号からは [`ModuleFile::normal_map`] で引く。
    pub normal_maps: Vec<TextureData>,
    /// 作者やライセンスなどの自由なメタデータ。
    pub meta: Option<serde_json::Value>,
}

impl ModuleFile {
    /// モジュールだけのファイル (テクスチャなどは後から足す)。
    pub fn new(module: Module) -> Self {
        Self {
            module,
            textures: Vec::new(),
            normal_maps: Vec::new(),
            meta: None,
        }
    }

    /// `module.textures` の `texture` 番目のテクスチャの法線マップ。なければ `None`。
    pub fn normal_map(&self, texture: usize) -> Option<&TextureData> {
        normal_map_of(&self.module.textures, &self.normal_maps, texture)
    }
}

/// モジュールを既定の設定 (無圧縮) でバイト列に書き出す。
pub fn write_module(file: &ModuleFile) -> Result<Vec<u8>, FormatError> {
    write_module_with(file, WriteOptions::default())
}

/// モジュールをバイト列に書き出す。チャンクは `FEAT` `MODU` `META` `TEXR`… `NRML`… の順。
///
/// テクスチャと法線マップの数が `module.textures` と一致しない場合や、`TextureData::format` と
/// 中身のシグネチャが食い違う場合はエラーにする (読み込めないファイルを作らないため)。
/// 未知の値 (`Unknown`) を含む場合は [`FormatError::UnknownValues`] で拒否する。
///
/// 使っている機能は [`features::detect_module`] で自動で宣言し、宣言があれば `FEAT` チャンクに書く。
pub fn write_module_with(file: &ModuleFile, options: WriteOptions) -> Result<Vec<u8>, FormatError> {
    write_with_features(file, options, &features::detect_module(file))
}

/// 機能の宣言 `declared` を指定して書き出す (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn write_with_features(
    file: &ModuleFile,
    options: WriteOptions,
    declared: &[FeatureDecl],
) -> Result<Vec<u8>, FormatError> {
    check_texture_count(&file.module.textures, file.textures.len())?;
    check_normal_map_count(&file.module.textures, file.normal_maps.len())?;
    let unknown = unknown_values(&file.module);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }

    let mut writer = ChunkWriter::new(options, declared)?;
    writer.push_encoded(ChunkTag::MODU, encode(&file.module)?);
    if let Some(meta) = &file.meta {
        writer.push_encoded(ChunkTag::META, serde_json::to_vec(meta)?);
    }
    writer.push_images(&file.textures, &file.normal_maps)?;
    writer.finish()
}

/// バイト列からモジュールを既定の上限 ([`ReadLimits::default`]) で読み込む。警告は捨てる ([`read_module_with_report`] を参照)。
pub fn read_module(bytes: &[u8]) -> Result<ModuleFile, FormatError> {
    read_module_with_report(bytes).map(|(file, _)| file)
}

/// バイト列からモジュールを既定の上限 ([`ReadLimits::default`]) で読み込み、警告を含む報告も返す。
///
/// コンテナと機能の宣言の検査はバンドル ([`crate::read_bundle_with_report`]) と同じ。
/// そのうえで、スキーマバージョンを (バンドルと同じく `MODU` をデコードする前に) 検査し、`ugoe_model::validate::validate_module` を通す
/// (エラーは [`FormatError::InvalidModule`])。本体との整合 (スロット・パラメータ・マスクの参照先) は
/// 合成 (`ugoe_runtime::compose`) が検査する。
///
/// - バンドル用のチャンク (`MODL` `PHYS` `MOTN` `EXPR`) があれば [`FormatError::UnexpectedChunk`]。
/// - 未知の値の警告の場所は `MODU: meshes[Hair].blend` や `MODU: physics.rigs[Sway].outputs[0].source`。
/// - 未知のタグのチャンクは読み飛ばす。
pub fn read_module_with_report(bytes: &[u8]) -> Result<(ModuleFile, ReadReport), FormatError> {
    read_module_with(bytes, ReadLimits::default())
}

/// 読み込みの上限 `limits` を指定してモジュールを読み込み、警告を含む報告も返す
/// (上限の扱いはバンドルの [`crate::read_bundle_with`] と同じ)。
pub fn read_module_with(
    bytes: &[u8],
    limits: ReadLimits,
) -> Result<(ModuleFile, ReadReport), FormatError> {
    read_with_known(bytes, limits, |id| features::find(id).is_some())
}

/// 知っている機能の判定 `known` を指定して読む (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn read_with_known(
    bytes: &[u8],
    limits: ReadLimits,
    known: impl Fn(&str) -> bool,
) -> Result<(ModuleFile, ReadReport), FormatError> {
    let mut payloads = Payloads::new(bytes, limits);
    let Container {
        entries,
        declared,
        mut warnings,
    } = read_container(&mut payloads, known)?;
    let mut body = check_schema_version_first(&entries, &mut payloads, ChunkTag::MODU)?;

    let mut module = None;
    let mut meta = None;
    let mut textures = Vec::new();
    let mut normal_maps = Vec::new();

    for entry in &entries {
        match entry.tag {
            ChunkTag::MODU => {
                if module.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                let payload = match body.take() {
                    Some(payload) => payload,
                    None => payloads.get(entry)?,
                };
                let decoded: Module = decode(&payload)?;
                check_schema_version(decoded.schema_version)?;
                module = Some(decoded);
            }
            ChunkTag::META => {
                if meta.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                meta = Some(serde_json::from_slice(&payloads.get(entry)?)?);
            }
            ChunkTag::TEXR => textures.push(read_texture(entry, &mut payloads, textures.len())?),
            ChunkTag::NRML => {
                normal_maps.push(read_normal_map(entry, &mut payloads, normal_maps.len())?);
            }
            // バンドルのファイルを取り違えたとき、黙って読み飛ばさずに知らせる。
            ChunkTag::MODL | ChunkTag::PHYS | ChunkTag::MOTN | ChunkTag::EXPR => {
                return Err(FormatError::UnexpectedChunk(entry.tag));
            }
            // 機能の宣言は先に読んだ。前方互換のため、未知のタグは読み飛ばす。
            _ => {}
        }
    }

    let module = module.ok_or(FormatError::MissingChunk(ChunkTag::MODU))?;
    check_texture_count(&module.textures, textures.len())?;
    check_normal_map_count(&module.textures, normal_maps.len())?;
    let errors = ugoe_model::validate::validate_module(&module);
    if !errors.is_empty() {
        return Err(FormatError::InvalidModule(errors));
    }

    warnings.extend(
        unknown_values(&module)
            .into_iter()
            .map(ReadWarning::UnknownValue),
    );
    Ok((
        ModuleFile {
            module,
            textures,
            normal_maps,
            meta,
        },
        ReadReport {
            features: declared,
            warnings,
        },
    ))
}

/// モジュールの中の未知の値の場所 (チャンクの名前付き) をすべて返す。
fn unknown_values(module: &Module) -> Vec<String> {
    ugoe_model::unknown::in_module(module)
        .into_iter()
        .map(|path| format!("{}: {path}", ChunkTag::MODU))
        .collect()
}

#[cfg(test)]
mod tests {
    use ugoe_model::TextureRef;
    use ugoe_model::fixtures::{dress_module, long_hair_module};

    use super::*;
    use crate::bundle::tests::{assemble, ktx2, png};
    use crate::bundle::{CHUNK_FLAG_ZSTD, TextureFormat, read_table};
    use crate::{Bundle, read_bundle, write_bundle};

    fn png_data() -> TextureData {
        TextureData {
            format: TextureFormat::Png,
            bytes: png(),
        }
    }

    /// 物理・テクスチャ (PNG と KTX2)・法線マップ・メタデータをすべて持つモジュール。
    fn full_module() -> ModuleFile {
        let mut module = long_hair_module();
        module.textures.push(TextureRef {
            name: "long_hair_tips".into(),
            width: 4,
            height: 4,
            normal_map: Some("long_hair_tips_normal".into()),
        });
        ModuleFile {
            module,
            textures: vec![
                png_data(),
                TextureData {
                    format: TextureFormat::Ktx2,
                    bytes: ktx2(),
                },
            ],
            normal_maps: vec![png_data()],
            meta: Some(serde_json::json!({ "author": "ugoe", "license": "CC0" })),
        }
    }

    fn module_only() -> ModuleFile {
        let mut file = ModuleFile::new(dress_module());
        file.textures.push(png_data());
        file
    }

    fn module_chunk(module: &Module) -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::MODU, 0, encode(module).unwrap())
    }

    fn texture_chunk() -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::TEXR, 0, png())
    }

    fn assert_err(bytes: &[u8], check: impl Fn(&FormatError) -> bool) {
        match read_module(bytes) {
            Ok(_) => panic!("expected an error"),
            Err(e) => assert!(check(&e), "unexpected error: {e:?}"),
        }
    }

    #[test]
    fn roundtrip_full_module() {
        let file = full_module();
        assert!(!file.module.physics.is_empty());
        for compress in [false, true] {
            let bytes = write_module_with(&file, WriteOptions { compress }).unwrap();
            let (read, report) = read_module_with_report(&bytes).unwrap();
            assert_eq!(read, file);
            assert_eq!(report, ReadReport::default());
        }
    }

    #[test]
    fn roundtrip_module_only() {
        let file = module_only();
        assert_eq!(read_module(&write_module(&file).unwrap()).unwrap(), file);
    }

    #[test]
    fn written_layout_follows_spec() {
        for compress in [false, true] {
            let bytes = write_module_with(&full_module(), WriteOptions { compress }).unwrap();
            assert_eq!(&bytes[0..4], b"UGOE");
            // 着せ替えは基準の機能なので `FEAT` は書かない。
            let tags: Vec<ChunkTag> = read_table(&bytes).unwrap().iter().map(|e| e.tag).collect();
            assert_eq!(
                tags,
                [
                    ChunkTag::MODU,
                    ChunkTag::META,
                    ChunkTag::TEXR,
                    ChunkTag::TEXR,
                    ChunkTag::NRML
                ]
            );
            for entry in read_table(&bytes).unwrap() {
                let image = entry.tag == ChunkTag::TEXR || entry.tag == ChunkTag::NRML;
                let expected = if compress && !image {
                    CHUNK_FLAG_ZSTD
                } else {
                    0
                };
                assert_eq!(entry.flags, expected, "{}", entry.tag);
            }
        }
    }

    #[test]
    fn normal_maps_follow_textures_with_normal_map() {
        let file = full_module();
        assert_eq!(file.normal_map(0), None);
        assert_eq!(file.normal_map(1), Some(&file.normal_maps[0]));
        assert_eq!(file.normal_map(2), None);
    }

    /// バンドルとモジュールのファイルを取り違えると、どちらも読み込みを拒否する。
    #[test]
    fn bundles_and_modules_are_not_confused() {
        let module = write_module(&module_only()).unwrap();
        assert!(matches!(
            read_bundle(&module),
            Err(FormatError::UnexpectedChunk(ChunkTag::MODU))
        ));
        let mut bundle = Bundle::new(ugoe_model::fixtures::single_quad());
        bundle.textures.push(png_data());
        let bundle = write_bundle(&bundle).unwrap();
        assert_err(&bundle, |e| {
            matches!(e, FormatError::UnexpectedChunk(ChunkTag::MODL))
        });
        for tag in [ChunkTag::PHYS, ChunkTag::MOTN, ChunkTag::EXPR] {
            let bytes = assemble(&[
                module_chunk(&dress_module()),
                (tag, 0, vec![0x80]),
                texture_chunk(),
            ]);
            assert_err(
                &bytes,
                |e| matches!(e, FormatError::UnexpectedChunk(t) if *t == tag),
            );
        }
    }

    #[test]
    fn unknown_chunks_are_skipped() {
        let bytes = assemble(&[
            (ChunkTag(*b"XTRA"), 0, vec![0xff; 5]),
            module_chunk(&dress_module()),
            (ChunkTag(*b"ZZZZ"), CHUNK_FLAG_ZSTD, b"garbage".to_vec()),
            texture_chunk(),
        ]);
        assert_eq!(read_module(&bytes).unwrap(), module_only());
    }

    #[test]
    fn rejects_missing_and_duplicate_chunks() {
        let module = dress_module();
        assert_err(&assemble(&[texture_chunk()]), |e| {
            matches!(e, FormatError::MissingChunk(ChunkTag::MODU))
        });
        assert_err(
            &assemble(&[
                module_chunk(&module),
                module_chunk(&module),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::MODU)),
        );
        let meta = (ChunkTag::META, 0, b"{}".to_vec());
        assert_err(
            &assemble(&[module_chunk(&module), meta.clone(), meta, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::META)),
        );
    }

    #[test]
    fn rejects_texture_and_normal_map_problems() {
        let module = dress_module();
        assert_err(&assemble(&[module_chunk(&module)]), |e| {
            matches!(
                e,
                FormatError::TextureCountMismatch {
                    expected: 1,
                    actual: 0
                }
            )
        });
        assert_err(
            &assemble(&[
                module_chunk(&module),
                (ChunkTag::TEXR, 0, b"GIF89a".to_vec()),
            ]),
            |e| matches!(e, FormatError::UnknownTextureFormat { index: 0 }),
        );
        assert_err(
            &assemble(&[
                module_chunk(&module),
                texture_chunk(),
                (ChunkTag::NRML, 0, png()),
            ]),
            |e| {
                matches!(
                    e,
                    FormatError::NormalMapCountMismatch {
                        expected: 0,
                        actual: 1
                    }
                )
            },
        );
        let mut lit = dress_module();
        lit.textures[0].normal_map = Some("dress_normal".into());
        assert_err(
            &assemble(&[
                module_chunk(&lit),
                texture_chunk(),
                (ChunkTag::NRML, 0, b"GIF89a".to_vec()),
            ]),
            |e| matches!(e, FormatError::UnknownNormalMapFormat { index: 0 }),
        );
    }

    #[test]
    fn write_rejects_inconsistent_images() {
        let mut file = full_module();
        file.textures[1].format = TextureFormat::Png;
        assert!(matches!(
            write_module(&file),
            Err(FormatError::TextureFormatMismatch { index: 1 })
        ));
        let mut file = full_module();
        file.normal_maps[0].format = TextureFormat::Ktx2;
        assert!(matches!(
            write_module(&file),
            Err(FormatError::NormalMapFormatMismatch { index: 0 })
        ));
        let mut file = full_module();
        file.normal_maps.clear();
        assert!(matches!(
            write_module(&file),
            Err(FormatError::NormalMapCountMismatch {
                expected: 1,
                actual: 0
            })
        ));
        file.textures.pop();
        assert!(matches!(
            write_module(&file),
            Err(FormatError::TextureCountMismatch {
                expected: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn rejects_newer_schema_version() {
        let mut module = dress_module();
        module.schema_version = ugoe_model::SCHEMA_VERSION + 1;
        assert_err(&assemble(&[module_chunk(&module), texture_chunk()]), |e| {
            matches!(e, FormatError::UnsupportedVersion { .. })
        });
    }

    #[test]
    fn rejects_invalid_module() {
        let mut module = dress_module();
        module.name = "Bad/Name".into();
        assert_err(
            &assemble(&[module_chunk(&module), texture_chunk()]),
            |e| matches!(e, FormatError::InvalidModule(errors) if !errors.is_empty()),
        );
        // 物理の参照先の誤りも `validate_module` が見つける。
        let mut file = full_module();
        // モジュールにない衝突体は本体の衝突体への参照として合成時に検査するので、`frame` の誤りで試す。
        file.module.physics.rigs[0].frame = Some("Missing".into());
        assert_err(
            &write_module(&file).unwrap(),
            |e| matches!(e, FormatError::InvalidModule(errors) if !errors.is_empty()),
        );
    }

    #[test]
    fn rejects_broken_payloads() {
        assert_err(
            &assemble(&[(ChunkTag::MODU, 0, vec![0xc1, 0x00]), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        let mut short = encode(&dress_module()).unwrap();
        short.truncate(short.len() / 2);
        assert_err(
            &assemble(&[(ChunkTag::MODU, 0, short), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        assert_err(
            &assemble(&[
                (ChunkTag::MODU, CHUNK_FLAG_ZSTD, b"not zstd".to_vec()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::Decompress(_)),
        );
    }

    #[test]
    fn rejects_truncated_input() {
        let bytes = write_module(&full_module()).unwrap();
        for len in 0..bytes.len() {
            assert!(
                read_module(&bytes[..len]).is_err(),
                "prefix {len} was accepted"
            );
        }
    }

    #[test]
    fn single_byte_corruption_never_panics() {
        let good = write_module_with(&full_module(), WriteOptions { compress: true }).unwrap();
        for i in 0..good.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut bytes = good.clone();
                bytes[i] ^= flip;
                let _ = read_module(&bytes);
            }
        }
    }

    /// 読み込みの上限はモジュールにもバンドルと同じく効く。
    #[test]
    fn read_limits_apply_to_modules() {
        let file = full_module();
        let bytes = write_module_with(&file, WriteOptions { compress: true }).unwrap();
        assert_eq!(
            read_module_with(&bytes, ReadLimits::default()).unwrap().0,
            file
        );
        let limits = ReadLimits::default().with_max_total_decompressed(1);
        assert!(matches!(
            read_module_with(&bytes, limits),
            Err(FormatError::ReadLimitExceeded {
                limit: crate::ReadLimit::TotalDecompressed,
                max: 1,
                ..
            })
        ));
    }

    #[test]
    fn feature_declarations_are_checked() {
        let declared = [
            FeatureDecl::optional("test.optional"),
            FeatureDecl::required("test.required"),
        ];
        let bytes =
            write_with_features(&module_only(), WriteOptions::default(), &declared).unwrap();
        assert_err(
            &bytes,
            |e| matches!(e, FormatError::UnsupportedFeatures(ids) if ids == &["test.required"]),
        );
        let (read, report) =
            read_with_known(&bytes, ReadLimits::default(), |id| id == "test.required").unwrap();
        assert_eq!(read, module_only());
        assert_eq!(report.features, declared);
        assert_eq!(
            report.warnings,
            [ReadWarning::UnknownOptionalFeature("test.optional".into())]
        );
    }

    /// 新しい版で増えた値を含むモジュールは警告付きで読め、書き戻せない。
    #[test]
    fn unknown_values_load_with_warnings_and_are_not_written_back() {
        use serde_json::json;

        let mut module = serde_json::to_value(long_hair_module()).unwrap();
        module["meshes"][0]["blend"] = json!("overlay");
        module["physics"]["rigs"][0]["outputs"][0]["source"] = json!("velocity");
        module["future_field"] = json!({ "nested": [1, 2, 3] });
        let bytes = assemble(&[
            (ChunkTag::MODU, 0, rmp_serde::to_vec_named(&module).unwrap()),
            texture_chunk(),
        ]);
        let (file, report) = read_module_with_report(&bytes).unwrap();
        let warnings: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
        let mesh = &file.module.meshes[0].id;
        assert_eq!(
            warnings,
            [
                format!("unknown value at MODU: meshes[{mesh}].blend"),
                "unknown value at MODU: physics.rigs[Sway].outputs[0].source".to_owned(),
            ]
        );
        match write_module(&file) {
            Err(FormatError::UnknownValues(paths)) => assert_eq!(paths.len(), 2),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
