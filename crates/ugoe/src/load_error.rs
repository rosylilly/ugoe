//! モデルの読み込み (ファイルを読む・コンパイルする) の失敗の種類 (`code`) と補足の値 (T0457)。
//!
//! wasm の `LoadError` (`crates/ugoe-wasm/README.md#読み込みの失敗-t0434`) と C ABI の `ugoe_error_kind`
//! (`crates/ugoe-ffi/README.md#モデルの読み込みの失敗-t0457`) は、ここの値を使う。
//! テクスチャのデコードやレンダラへの登録の失敗は組み込み先ごとのものなので、ここでは決めない。

use std::fmt;

use ugoe_format::FormatError;
use ugoe_model::validate::ValidationError;
use ugoe_runtime::CompileError;

use crate::limits::ReadLimitExceeded;
use crate::{DressUpError, FieldValue};

/// モデルの読み込みの失敗。`.ugoe` バンドルやモデルの JSON を読めない ([`FormatError`]) か、
/// 読んだモデルを評価できない ([`CompileError`])。
///
/// 組み込み先は [`LoadError::code`] と [`LoadError::fields`] で種類と補足の値を読む (`Display` の文は安定した値として扱わない)。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// ファイルを読めない (壊れている・版が新しい・知らない必須の機能・検査を通らない・読み込みの上限を超えたなど)。
    #[error(transparent)]
    Format(#[from] FormatError),
    /// 読んだモデルをコンパイルできない。
    #[error("model does not compile: {0}")]
    Compile(#[from] CompileError),
}

/// 読み込みの失敗の補足の値 ([`LoadError::fields`])。wasm は数と文字列、C ABI は 10 進の文字列にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadErrorValue {
    /// 番号・数・大きさ (版・検査のエラーの数・上限のバイト数)。
    Number(u64),
    /// 文 (上限の名前・チャンクのタグ・機能の ID の列・検査のエラーの種類の列)。
    Text(String),
}

impl fmt::Display for LoadErrorValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Text(s) => f.write_str(s),
        }
    }
}

impl LoadError {
    /// ファイルを読めないときの種類 (着せ替えの [`DressUpError::INVALID_FILE`] と同じ値)。
    pub const INVALID_FILE: &'static str = DressUpError::INVALID_FILE;

    /// 機械が読む種類。値は組み込み先が照らし合わせるので変えない。
    ///
    /// - `invalid_file`: ファイルを読めない (ugoe のファイルでない・壊れている・JSON の誤りなど。細かい理由は文にある)。
    /// - `unsupported_version`: ファイルの版が読める版より新しい。
    /// - `unsupported_features`: 知らない必須の機能を宣言している。
    /// - `invalid_model`: モデル・物理・モーション・表情の検査を通らない (読むときの検査と、コンパイルの前の検査)。
    /// - `compile`: 評価できない構造。
    /// - `read_limit_exceeded`: 読み込みの上限を超えた ([`ReadLimitExceeded::CODE`])。
    pub fn code(&self) -> &'static str {
        self.info().0
    }

    /// 補足の値 (キーと値)。キーは種類ごとに決まる:
    ///
    /// - `unsupported_version`: `found` (ファイルの版)・`supported` (読める版)。
    /// - `unsupported_features`: `features` (知らない必須の機能の ID を `", "` でつないだもの)。
    /// - `invalid_model`: `errors` (検査のエラーの数)・`error_codes` (検査のエラーの種類 (`ValidationError::code`) を出た順に、
    ///   重なりを除いて `", "` でつないだもの。着せ替えの `invalid_physics` と同じ。[`crate::validation_fields`]。T0469)。
    /// - `read_limit_exceeded`: `limit` (`"chunk_decompressed"` | `"total_decompressed"`)・`chunk` (タグ)・`max` (バイト数)。
    ///   値は [`ReadLimitExceeded::fields`] と同じ。
    /// - ほか: なし。
    pub fn fields(&self) -> Vec<(&'static str, LoadErrorValue)> {
        self.info().1
    }

    /// 読み込みの上限を超えた失敗か (`read_limit_exceeded`)。ファイルが壊れているとは限らない (上限を上げれば読めることがある)。
    pub fn exceeds_limit(&self) -> bool {
        matches!(self, Self::Format(e) if ReadLimitExceeded::from_error(e).is_some())
    }

    fn info(&self) -> (&'static str, Vec<(&'static str, LoadErrorValue)>) {
        use LoadErrorValue::{Number, Text};
        let invalid = |errors: &[ValidationError]| {
            let fields = crate::validation_fields(errors)
                .into_iter()
                .map(|(key, value)| {
                    let value = match value {
                        FieldValue::Index(n) => Number(n as u64),
                        value => Text(value.to_string()),
                    };
                    (key, value)
                })
                .collect();
            ("invalid_model", fields)
        };
        match self {
            Self::Compile(CompileError::Invalid(errors)) => invalid(errors),
            Self::Compile(_) => ("compile", Vec::new()),
            Self::Format(e) => {
                if let Some(exceeded) = ReadLimitExceeded::from_error(e) {
                    return (
                        ReadLimitExceeded::CODE,
                        vec![
                            ("limit", Text(exceeded.limit.as_str().to_owned())),
                            ("chunk", Text(exceeded.chunk.to_string())),
                            ("max", Number(exceeded.max)),
                        ],
                    );
                }
                if let Some(errors) = e.validation_errors() {
                    return invalid(errors);
                }
                match e {
                    &FormatError::UnsupportedVersion { found, supported }
                    | &FormatError::UnsupportedContainerVersion { found, supported } => (
                        "unsupported_version",
                        vec![
                            ("found", Number(found.into())),
                            ("supported", Number(supported.into())),
                        ],
                    ),
                    FormatError::UnsupportedFeatures(features) => (
                        "unsupported_features",
                        vec![("features", Text(features.join(", ")))],
                    ),
                    // 壊れたファイル (マジック・チャンク表・圧縮・JSON・MessagePack・テクスチャの数など)。
                    _ => (Self::INVALID_FILE, Vec::new()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe_format::{ChunkTag, ReadLimit};

    use super::*;

    fn validation() -> Vec<ValidationError> {
        vec![ValidationError::InvalidModuleName {
            name: String::new(),
        }]
    }

    #[test]
    fn every_kind_has_a_code_and_fields() {
        use LoadErrorValue::{Number, Text};
        let text = |s: &str| Text(s.to_owned());
        type Fields = Vec<(&'static str, LoadErrorValue)>;
        let cases: Vec<(LoadError, &str, Fields)> = vec![
            (FormatError::BadMagic.into(), "invalid_file", vec![]),
            (
                FormatError::UnsupportedVersion {
                    found: 9,
                    supported: 1,
                }
                .into(),
                "unsupported_version",
                vec![("found", Number(9)), ("supported", Number(1))],
            ),
            (
                FormatError::UnsupportedContainerVersion {
                    found: 3,
                    supported: 2,
                }
                .into(),
                "unsupported_version",
                vec![("found", Number(3)), ("supported", Number(2))],
            ),
            (
                FormatError::UnsupportedFeatures(vec!["x.a".into(), "x.b".into()]).into(),
                "unsupported_features",
                vec![("features", text("x.a, x.b"))],
            ),
            (
                FormatError::InvalidPhysics(validation()).into(),
                "invalid_model",
                vec![
                    ("errors", Number(1)),
                    ("error_codes", text("invalid_module_name")),
                ],
            ),
            (
                FormatError::ReadLimitExceeded {
                    tag: ChunkTag::MODL,
                    limit: ReadLimit::TotalDecompressed,
                    max: u64::MAX,
                }
                .into(),
                "read_limit_exceeded",
                vec![
                    ("limit", text("total_decompressed")),
                    ("chunk", text("MODL")),
                    ("max", Number(u64::MAX)),
                ],
            ),
            (
                CompileError::Invalid(validation()).into(),
                "invalid_model",
                vec![
                    ("errors", Number(1)),
                    ("error_codes", text("invalid_module_name")),
                ],
            ),
            (
                CompileError::UnknownParameter("p".into()).into(),
                "compile",
                vec![],
            ),
            (CompileError::Unsupported("x").into(), "compile", vec![]),
        ];
        for (error, code, fields) in cases {
            assert_eq!(error.code(), code, "{error}");
            assert_eq!(error.fields(), fields, "{error}");
            assert_eq!(error.exceeds_limit(), code == "read_limit_exceeded");
        }
    }

    /// 上限の補足の値は C ABI が前から返していた [`ReadLimitExceeded::fields`] と同じ文字列になる。
    #[test]
    fn read_limit_fields_match_the_limits_module() {
        let format = FormatError::ReadLimitExceeded {
            tag: ChunkTag::MODL,
            limit: ReadLimit::ChunkDecompressed,
            max: 7,
        };
        let expected = ReadLimitExceeded::from_error(&format).unwrap().fields();
        let actual: Vec<_> = LoadError::from(format)
            .fields()
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        assert_eq!(actual, expected);
    }

    /// `invalid_model` の種類の一覧は、出た順に重なりを除いて `, ` でつなぐ (着せ替えの `invalid_physics` と同じ。T0469)。
    #[test]
    fn invalid_model_lists_the_validation_codes() {
        let duplicate = ValidationError::DuplicateId {
            kind: "mesh",
            id: "m".into(),
        };
        let errors = [validation(), vec![duplicate], validation()].concat();
        let expected = vec![
            ("errors", LoadErrorValue::Number(3)),
            (
                "error_codes",
                LoadErrorValue::Text("invalid_module_name, duplicate_id".into()),
            ),
        ];
        let compile = LoadError::from(CompileError::Invalid(errors.clone()));
        assert_eq!(compile.fields(), expected);
        let format = LoadError::from(FormatError::Invalid(errors));
        assert_eq!(format.code(), "invalid_model");
        assert_eq!(format.fields(), expected);
    }

    #[test]
    fn compile_errors_keep_their_text() {
        let error = LoadError::from(CompileError::Unsupported("x"));
        assert_eq!(
            error.to_string(),
            "model does not compile: unsupported feature: x"
        );
    }
}
