//! ファイル形式の読み書き。仕様: `docs/spec/file-formats.md`。
//!
//! - `.ugoe` : ランタイムバンドル (ヘッダ + チャンク表 + チャンク)。バイナリ。[`bundle`] を参照。
//! - `.ugoemod` : 着せ替えのモジュール。バンドルと同じコンテナ。[`module_file`] を参照。
//! - `*.motion.json` / `*.expression.json` / `*.physics.json` : JSON。
//! - テクスチャの PNG と KTX2 のデコード (上限つき): [`image`]。PNG とブロック圧縮の KTX2 は feature `png`。
//!
//! 読み込みは **常に** スキーマバージョンを検査し、未知の新しい版は明示的なエラーにする。
//!
//! `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない
//! (`docs/portability.md#ランタイム層が守る制約-移植性のため`)。`std` が無くても、バンドル・モジュール・JSON の読み書きと
//! zstd の展開、読み込みの上限 ([`ReadLimits`]) は同じに働く。`std` を要るのは feature `png` (PNG と BCn のデコード) だけ。

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

extern crate alloc;
// feature `png` の PNG のデコーダが `std::io` を使う (テストは std の prelude で動く)
#[cfg(all(feature = "std", not(test)))]
extern crate std;

pub mod bundle;
pub mod features;
pub mod image;
pub mod module_file;
mod msgpack;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ugoe_model::Model;
use ugoe_model::expression::Expression;
use ugoe_model::module::Module;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, describe_errors};

pub use bundle::{
    Bundle, ChunkTag, ReadLimit, ReadLimits, ReadReport, ReadWarning, TextureData, TextureFormat,
    WriteOptions, read_bundle, read_bundle_with, read_bundle_with_report, write_bundle,
    write_bundle_with,
};
pub use features::{FEATURES, Feature, FeatureDecl};
pub use module_file::{
    ModuleFile, read_module, read_module_with, read_module_with_report, write_module,
    write_module_with,
};
pub use msgpack::{MsgPackDecodeError, MsgPackEncodeError};

/// `.ugoe` ファイル先頭のマジックバイト。
pub const MAGIC: [u8; 4] = *b"UGOE";

/// 読み書きのエラー。読み込みは壊れた入力でパニックせず、このエラーを返す。
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// 先頭のマジックバイトが [`MAGIC`] でない (ugoe のファイルではない)。
    #[error("not a ugoe file")]
    BadMagic,
    /// スキーマバージョンがこのクレートの読める版より新しい。
    #[error("unsupported schema version {found} (supported up to {supported})")]
    UnsupportedVersion {
        /// ファイルに書かれた版。
        found: u32,
        /// 読める最も新しい版 ([`ugoe_model::SCHEMA_VERSION`])。
        supported: u32,
    },
    /// ヘッダのコンテナの版 (ヘッダ・チャンク表の形の版) が知らないもの。
    #[error("unsupported container version {found} (supported {supported})")]
    UnsupportedContainerVersion {
        /// ファイルに書かれた版。
        found: u32,
        /// 読める版。
        supported: u32,
    },
    /// ヘッダのフラグに知らないビットが立っている。中身はフラグの値。
    #[error("unsupported header flags {0:#x}")]
    UnsupportedFlags(u32),
    /// チャンク表の項目のフラグに知らないビットが立っている。
    #[error("chunk {tag}: unsupported flags {flags:#x}")]
    UnsupportedChunkFlags {
        /// チャンクの種類。
        tag: ChunkTag,
        /// フラグの値。
        flags: u32,
    },
    /// ヘッダかチャンク表が途中で切れている。
    #[error("unexpected end of input")]
    Truncated,
    /// チャンク表の範囲 (オフセットと長さ) がファイルの外を指す。
    #[error("chunk {tag}: offset or length out of bounds")]
    ChunkOutOfBounds {
        /// チャンクの種類。
        tag: ChunkTag,
    },
    /// チャンクのオフセットが 16 バイト境界に揃っていない。
    #[error("chunk {tag}: offset {offset} is not aligned to 16 bytes")]
    MisalignedChunk {
        /// チャンクの種類。
        tag: ChunkTag,
        /// ファイルの先頭からのオフセット (バイト)。
        offset: u64,
    },
    /// チャンクの範囲が、ヘッダ・チャンク表・ほかのチャンクと重なる。
    #[error("chunk {tag} overlaps the header, the chunk table or another chunk")]
    OverlappingChunks {
        /// 重なりが見つかったチャンクの種類。
        tag: ChunkTag,
    },
    /// このファイルの種類には置けないチャンク (`.ugoe` の `MODU`、`.ugoemod` の `MODL` など)。
    /// バンドルとモジュールのファイルを取り違えたときに返す。
    #[error("chunk {0} is not allowed in this kind of file")]
    UnexpectedChunk(ChunkTag),
    /// 必須のチャンク (`MODL` など) が無い。
    #[error("required chunk {0} is missing")]
    MissingChunk(ChunkTag),
    /// 1 つまでしか置けないチャンクが 2 つ以上ある。
    #[error("chunk {0} must appear at most once")]
    DuplicateChunk(ChunkTag),
    /// `TEXR` チャンクの数が、モデルの `textures` の数と一致しない。
    #[error("texture count {actual} does not match model textures {expected}")]
    TextureCountMismatch {
        /// モデルの `textures` の数。
        expected: usize,
        /// `TEXR` チャンクの数。
        actual: usize,
    },
    /// テクスチャのバイト列の先頭が PNG・KTX2 のどちらのシグネチャでもない。
    #[error("texture #{index}: not a PNG or KTX2 image")]
    UnknownTextureFormat {
        /// テクスチャの番号 (`TEXR` チャンクの順番)。
        index: usize,
    },
    /// 書き出すテクスチャのバイト列が、[`TextureData::format`](bundle::TextureData::format) の形式と食い違う。
    #[error("texture #{index}: bytes do not match the declared format")]
    TextureFormatMismatch {
        /// テクスチャの番号。
        index: usize,
    },
    /// `NRML` の数が、`normal_map` を持つテクスチャの数と一致しない。
    #[error("normal map count {actual} does not match textures with a normal map {expected}")]
    NormalMapCountMismatch {
        /// `normal_map` を持つテクスチャの数。
        expected: usize,
        /// `NRML` チャンクの数。
        actual: usize,
    },
    /// 法線マップのバイト列の先頭が PNG・KTX2 のどちらのシグネチャでもない。
    #[error("normal map #{index}: not a PNG or KTX2 image")]
    UnknownNormalMapFormat {
        /// `NRML` チャンクの順番。
        index: usize,
    },
    /// 書き出す法線マップのバイト列が、宣言した形式と食い違う。
    #[error("normal map #{index}: bytes do not match the declared format")]
    NormalMapFormatMismatch {
        /// 法線マップの順番 (`NRML` チャンクの順番)。
        index: usize,
    },
    /// 圧縮されたチャンクを展開できない。中身は展開のライブラリのメッセージ。
    #[error("chunk decompression failed: {0}")]
    Decompress(String),
    /// 読み込みの上限 ([`ReadLimits`]) を超えたので、チャンク `tag` の展開を途中でやめた。
    /// `limit` はどの上限か、`max` はその上限の値 (バイト)。ファイルが壊れているとは限らない
    /// (上限を上げれば読めることがある。`docs/spec/file-formats.md#読み込みの上限`)。
    #[error("chunk {tag}: {limit} exceeds the read limit of {max} bytes")]
    ReadLimitExceeded {
        /// 展開をやめたチャンクの種類。
        tag: ChunkTag,
        /// 超えた上限の種類。
        limit: ReadLimit,
        /// その上限の値 (バイト)。
        max: u64,
    },
    /// 書き出すバンドルが大きすぎて、チャンクの数やオフセットがヘッダ・チャンク表の整数に収まらない。
    #[error("bundle is too large")]
    TooLarge,
    /// モデルが `validate` の検査に通らない。文の形は `docs/spec/model.md#検証エラーの文`。
    #[error("{}", describe_errors("model", .0))]
    Invalid(Vec<ValidationError>),
    /// 着せ替えのモジュールが `validate_module` の検査に通らない。
    #[error("{}", describe_errors("module", .0))]
    InvalidModule(Vec<ValidationError>),
    /// 物理の設定が `validate_physics` の検査に通らない。
    #[error("{}", describe_errors("physics settings", .0))]
    InvalidPhysics(Vec<ValidationError>),
    /// モーション (`MOTN` か `*.motion.json`) が `validate_motion` の検査に通らない。`name` はモーションの名前。
    #[error("{}", describe_errors(&format!("motion `{name}`"), .errors))]
    InvalidMotion {
        /// モーションの名前。
        name: String,
        /// 見つかったエラーのすべて。
        errors: Vec<ValidationError>,
    },
    /// 表情 (`EXPR` か `*.expression.json`) が `validate_expression` の検査に通らない。`name` は表情の名前。
    #[error("{}", describe_errors(&format!("expression `{name}`"), .errors))]
    InvalidExpression {
        /// 表情の名前。
        name: String,
        /// 見つかったエラーのすべて。
        errors: Vec<ValidationError>,
    },
    /// 知らない必須の機能を宣言している (`docs/spec/file-formats.md#機能の宣言`)。
    #[error("unsupported required features: {}", .0.join(", "))]
    UnsupportedFeatures(Vec<String>),
    /// 未知の値 (新しい版で増えた enum の値) は書き出せない。値の場所の一覧。
    #[error("cannot write unknown values: {}", .0.join(", "))]
    UnknownValues(Vec<String>),
    /// JSON の読み書きに失敗した (JSON の構文・型が合わないなど)。
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// チャンクの MessagePack への書き出しに失敗した。
    #[error(transparent)]
    MsgPackEncode(#[from] MsgPackEncodeError),
    /// チャンクの MessagePack を読めない (形が合わないなど)。
    #[error(transparent)]
    MsgPackDecode(#[from] MsgPackDecodeError),
}

impl FormatError {
    /// 検証エラーの列を持つ種類 (`Invalid`・`InvalidModule`・`InvalidPhysics`・`InvalidMotion`・`InvalidExpression`)
    /// なら、その列。ほかの種類は `None`。
    ///
    /// 読む側 (CLI・エディタ・ビューア・MCP) は種類ごとに展開せずにこれを使う (種類が増えても取りこぼさない)。
    /// モーションと表情の名前は `Display` の文に入る。
    pub fn validation_errors(&self) -> Option<&[ValidationError]> {
        match self {
            Self::Invalid(errors)
            | Self::InvalidModule(errors)
            | Self::InvalidPhysics(errors)
            | Self::InvalidMotion { errors, .. }
            | Self::InvalidExpression { errors, .. } => Some(errors),
            _ => None,
        }
    }
}

/// モデルを JSON 表現で書き出す (デバッグ・差分確認・AI 入出力用)。
///
/// 未知の値 (`Unknown`) を含むモデルは [`FormatError::UnknownValues`] で拒否する。
pub fn model_to_json(model: &Model) -> Result<String, FormatError> {
    let unknown = ugoe_model::unknown::in_model(model);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }
    Ok(serde_json::to_string_pretty(model)?)
}

/// JSON 表現からモデルを読み込む。未知の値は `Unknown` として読む
/// (場所は `ugoe_model::unknown::in_model` で調べられる)。
///
/// スキーマのバージョンだけを検査する。[`read_bundle`] と違い、`ugoe_model::validate::validate` は
/// 呼び出し側で行う。
pub fn model_from_json(json: &str) -> Result<Model, FormatError> {
    check_json_schema_version_first(json)?;
    let model: Model = serde_json::from_str(json)?;
    check_schema_version(model.schema_version)?;
    Ok(model)
}

/// 着せ替えのモジュールを JSON 表現で書き出す。
///
/// 未知の値 (`Unknown`) を含むモジュールは [`FormatError::UnknownValues`] で拒否する。
pub fn module_to_json(module: &Module) -> Result<String, FormatError> {
    let unknown = ugoe_model::unknown::in_module(module);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }
    Ok(serde_json::to_string_pretty(module)?)
}

/// JSON 表現から着せ替えのモジュールを読み込む。スキーマのバージョンだけを検査する
/// (検証は `ugoe_model::validate::validate_module` を呼び出し側で行う)。
pub fn module_from_json(json: &str) -> Result<Module, FormatError> {
    check_json_schema_version_first(json)?;
    let module: Module = serde_json::from_str(json)?;
    check_schema_version(module.schema_version)?;
    Ok(module)
}

/// 物理の設定 (`*.physics.json`) を JSON 表現から読み込む。参照先の検査
/// (`ugoe_model::validate::validate_physics`) は、組み合わせるモデルが決まってから呼び出し側で行う。
pub fn physics_from_json(json: &str) -> Result<PhysicsSettings, FormatError> {
    Ok(serde_json::from_str(json)?)
}

/// 物理の設定を JSON 表現で書き出す。
///
/// 未知の値 (`Unknown`) を含む物理の設定は [`FormatError::UnknownValues`] で拒否する。
pub fn physics_to_json(physics: &PhysicsSettings) -> Result<String, FormatError> {
    let unknown = ugoe_model::unknown::in_physics(physics);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }
    Ok(serde_json::to_string_pretty(physics)?)
}

/// モーション (`*.motion.json`) を JSON 表現から読み込む。
///
/// 数値を `ugoe_model::validate::validate_motion` で検査し、通らなければ [`FormatError::InvalidMotion`]
/// (`docs/spec/model.md#モーションと表情の検証`)。
pub fn motion_from_json(json: &str) -> Result<Motion, FormatError> {
    let motion: Motion = serde_json::from_str(json)?;
    check_motion(&motion)?;
    Ok(motion)
}

/// 表情 (`*.expression.json`) を JSON 表現から読み込む。
///
/// 数値を `ugoe_model::validate::validate_expression` で検査し、通らなければ [`FormatError::InvalidExpression`]。
pub fn expression_from_json(json: &str) -> Result<Expression, FormatError> {
    let expression: Expression = serde_json::from_str(json)?;
    check_expression(&expression)?;
    Ok(expression)
}

/// モーションの数値を `ugoe_model::validate::validate_motion` で検査する。通らなければ [`FormatError::InvalidMotion`]
/// (`docs/spec/model.md#モーションと表情の検証`)。
///
/// JSON とバンドルの読み込みが使う検査と同じもの。エディタのプロジェクト (`ugoe-editor-core`) の読み込みも
/// これを使い、書き出したバンドルを読み直すと拒まれる値を同じ理由で拒む。
pub fn check_motion(motion: &Motion) -> Result<(), FormatError> {
    let errors = ugoe_model::validate::validate_motion(motion);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(FormatError::InvalidMotion {
            name: motion.name.clone(),
            errors,
        })
    }
}

/// 表情の数値を `ugoe_model::validate::validate_expression` で検査する。通らなければ
/// [`FormatError::InvalidExpression`] (`docs/spec/model.md#モーションと表情の検証`)。
///
/// JSON とバンドルの読み込みが使う検査と同じもの ([`check_motion`] を参照)。
pub fn check_expression(expression: &Expression) -> Result<(), FormatError> {
    let errors = ugoe_model::validate::validate_expression(expression);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(FormatError::InvalidExpression {
            name: expression.name.clone(),
            errors,
        })
    }
}

/// スキーマのバージョンが読める範囲か検査する (`docs/spec/file-formats.md` のバージョニング)。
fn check_schema_version(version: u32) -> Result<(), FormatError> {
    if version > ugoe_model::SCHEMA_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: version,
            supported: ugoe_model::SCHEMA_VERSION,
        });
    }
    Ok(())
}

/// JSON のオブジェクトから `schema_version` だけを先に読んで検査する。
///
/// 形の変わった新しい版の JSON が、デコードのエラーでなく [`FormatError::UnsupportedVersion`] になるようにするため
/// (バンドルの [`bundle::peek_schema_version`] と同じ。`docs/spec/file-formats.md#バージョニング`)。
/// 先読みできなければここではエラーにせず、全体のデコードに任せる。
fn check_json_schema_version_first(json: &str) -> Result<(), FormatError> {
    #[derive(serde::Deserialize)]
    struct Versioned {
        schema_version: Option<u32>,
    }
    match serde_json::from_str::<Versioned>(json) {
        Ok(Versioned {
            schema_version: Some(version),
        }) => check_schema_version(version),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let model = ugoe_model::fixtures::single_quad();
        let back = model_from_json(&model_to_json(&model).unwrap()).unwrap();
        assert_eq!(model, back);
    }

    #[test]
    fn motion_and_expression_json_are_validated() {
        let motion = |duration: &str, v: &str| {
            motion_from_json(&format!(
                r#"{{ "name": "nod", "duration": {duration}, "curves": [ {{
                    "target": {{ "kind": "parameter", "id": "AngleY" }},
                    "segments": [ {{ "type": "start", "p": {{ "t": 0, "v": {v} }} }} ] }} ] }}"#
            ))
        };
        assert!(motion("1.0", "0").is_ok());
        // `1e39` は `f32` では無限大になる。
        assert!(matches!(
            motion("1.0", "1e39"),
            Err(FormatError::InvalidMotion { name, errors }) if name == "nod" && errors.len() == 1
        ));
        assert!(matches!(
            motion("-1.0", "0"),
            Err(FormatError::InvalidMotion { errors, .. }) if errors.len() == 1
        ));

        let expression = |value: &str| {
            expression_from_json(&format!(
                r#"{{ "name": "smile", "parameters": [ {{ "parameter": "X", "value": {value}, "op": "add" }} ] }}"#
            ))
        };
        assert!(expression("0.5").is_ok());
        assert!(matches!(
            expression("-1e39"),
            Err(FormatError::InvalidExpression { name, errors }) if name == "smile" && errors.len() == 1
        ));
    }

    // T0015: 検証エラーの列を持つ種類はすべて `validation_errors` で列を返し、文はすべてのエラーを並べる。
    #[test]
    fn validation_errors_cover_every_list_variant() {
        let error = |field: &str| ValidationError::InvalidValue {
            field: field.into(),
            value: -1.0,
            expected: ">= 0",
        };
        let errors = vec![error("a"), error("b")];
        let cases = [
            (FormatError::Invalid(errors.clone()), "model"),
            (FormatError::InvalidModule(errors.clone()), "module"),
            (
                FormatError::InvalidPhysics(errors.clone()),
                "physics settings",
            ),
            (
                FormatError::InvalidMotion {
                    name: "nod".into(),
                    errors: errors.clone(),
                },
                "motion `nod`",
            ),
            (
                FormatError::InvalidExpression {
                    name: "smile".into(),
                    errors: errors.clone(),
                },
                "expression `smile`",
            ),
        ];
        for (e, subject) in cases {
            assert_eq!(e.validation_errors(), Some(errors.as_slice()), "{e}");
            assert_eq!(
                e.to_string(),
                format!(
                    "{subject} failed validation with 2 error(s): a = -1 must be >= 0; b = -1 must be >= 0"
                )
            );
        }
        assert_eq!(FormatError::TooLarge.validation_errors(), None);
    }

    #[test]
    fn module_json_roundtrip() {
        let module = ugoe_model::fixtures::long_hair_module();
        let back = module_from_json(&module_to_json(&module).unwrap()).unwrap();
        assert_eq!(module, back);

        let mut newer = module;
        newer.schema_version = ugoe_model::SCHEMA_VERSION + 1;
        let json = serde_json::to_string(&newer).unwrap();
        assert!(matches!(
            module_from_json(&json),
            Err(FormatError::UnsupportedVersion { .. })
        ));
    }

    /// 形の変わった新しい版の JSON も、デコードのエラーでなく版のエラーになる (バンドルと同じ)。
    #[test]
    fn newer_schema_json_is_rejected_before_decoding() {
        let newer = serde_json::json!({
            "nodes": [{ "id": "Root" }],
            "schema_version": ugoe_model::SCHEMA_VERSION + 1,
        })
        .to_string();
        for result in [
            model_from_json(&newer).map(drop),
            module_from_json(&newer).map(drop),
        ] {
            assert!(
                matches!(result, Err(FormatError::UnsupportedVersion { found, .. }) if found == ugoe_model::SCHEMA_VERSION + 1),
                "{result:?}"
            );
        }
        // 同じ版なら今までどおりデコードのエラー。
        let same = serde_json::json!({ "schema_version": ugoe_model::SCHEMA_VERSION }).to_string();
        assert!(matches!(model_from_json(&same), Err(FormatError::Json(_))));
    }

    #[test]
    fn physics_json_roundtrip() {
        let physics = ugoe_model::fixtures::physics_hair_settings();
        let back = physics_from_json(&physics_to_json(&physics).unwrap()).unwrap();
        assert_eq!(physics, back);
    }

    /// 未知の値を含む物理は、モデルと同じく場所を示して書き出しを拒否する。
    #[test]
    fn physics_with_unknown_values_is_not_written() {
        let mut physics = ugoe_model::fixtures::physics_hair_settings();
        physics.rigs[0].outputs[0].source = ugoe_model::physics::PhysicsSource::Unknown;
        match physics_to_json(&physics) {
            Err(FormatError::UnknownValues(paths)) => assert_eq!(paths.len(), 1),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
