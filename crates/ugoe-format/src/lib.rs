//! ファイル形式の読み書き。仕様: `docs/spec/file-formats.md`。
//!
//! - `.ugoe` : ランタイムバンドル (ヘッダ + チャンク表 + チャンク)。バイナリ。[`bundle`] を参照。
//! - `*.motion.json` / `*.expression.json` / `*.physics.json` : JSON。
//!
//! 読み込みは **常に** スキーマバージョンを検査し、未知の新しい版は明示的なエラーにする。

pub mod bundle;
pub mod features;

use ugoe_model::Model;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::ValidationError;

pub use bundle::{
    Bundle, ChunkTag, ReadReport, ReadWarning, TextureData, TextureFormat, WriteOptions,
    read_bundle, read_bundle_with_report, write_bundle, write_bundle_with,
};
pub use features::{FEATURES, Feature, FeatureDecl};

/// `.ugoe` ファイル先頭のマジックバイト。
pub const MAGIC: [u8; 4] = *b"UGOE";

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not a ugoe file")]
    BadMagic,
    #[error("unsupported schema version {found} (supported up to {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("unsupported container version {found} (supported {supported})")]
    UnsupportedContainerVersion { found: u32, supported: u32 },
    #[error("unsupported header flags {0:#x}")]
    UnsupportedFlags(u32),
    #[error("chunk {tag}: unsupported flags {flags:#x}")]
    UnsupportedChunkFlags { tag: ChunkTag, flags: u32 },
    #[error("unexpected end of input")]
    Truncated,
    #[error("chunk {tag}: offset or length out of bounds")]
    ChunkOutOfBounds { tag: ChunkTag },
    #[error("chunk {tag}: offset {offset} is not aligned to 16 bytes")]
    MisalignedChunk { tag: ChunkTag, offset: u64 },
    #[error("chunk {tag} overlaps the header, the chunk table or another chunk")]
    OverlappingChunks { tag: ChunkTag },
    #[error("required chunk {0} is missing")]
    MissingChunk(ChunkTag),
    #[error("chunk {0} must appear at most once")]
    DuplicateChunk(ChunkTag),
    #[error("texture count {actual} does not match model textures {expected}")]
    TextureCountMismatch { expected: usize, actual: usize },
    #[error("texture #{index}: not a PNG or KTX2 image")]
    UnknownTextureFormat { index: usize },
    #[error("texture #{index}: bytes do not match the declared format")]
    TextureFormatMismatch { index: usize },
    #[error("chunk decompression failed: {0}")]
    Decompress(String),
    #[error("chunk {tag}: decompressed size exceeds the limit")]
    DecompressedTooLarge { tag: ChunkTag },
    #[error("bundle is too large")]
    TooLarge,
    #[error("model failed validation with {} error(s)", .0.len())]
    Invalid(Vec<ValidationError>),
    /// 物理の設定が `validate_physics` の検査に通らない。
    #[error("physics settings failed validation with {} error(s)", .0.len())]
    InvalidPhysics(Vec<ValidationError>),
    /// 知らない必須の機能を宣言している (`docs/spec/file-formats.md#機能の宣言`)。
    #[error("unsupported required features: {}", .0.join(", "))]
    UnsupportedFeatures(Vec<String>),
    /// 未知の値 (新しい版で増えた enum の値) は書き出せない。値の場所の一覧。
    #[error("cannot write unknown values: {}", .0.join(", "))]
    UnknownValues(Vec<String>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
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
pub fn model_from_json(json: &str) -> Result<Model, FormatError> {
    let model: Model = serde_json::from_str(json)?;
    check_schema_version(&model)?;
    Ok(model)
}

/// 物理の設定 (`*.physics.json`) を JSON 表現から読み込む。参照先の検査
/// (`ugoe_model::validate::validate_physics`) は、組み合わせるモデルが決まってから呼び出し側で行う。
pub fn physics_from_json(json: &str) -> Result<PhysicsSettings, FormatError> {
    Ok(serde_json::from_str(json)?)
}

/// 物理の設定を JSON 表現で書き出す。
pub fn physics_to_json(physics: &PhysicsSettings) -> Result<String, FormatError> {
    Ok(serde_json::to_string_pretty(physics)?)
}

/// モーション (`*.motion.json`) を JSON 表現から読み込む。
pub fn motion_from_json(json: &str) -> Result<Motion, FormatError> {
    Ok(serde_json::from_str(json)?)
}

/// 表情 (`*.expression.json`) を JSON 表現から読み込む。
pub fn expression_from_json(json: &str) -> Result<Expression, FormatError> {
    Ok(serde_json::from_str(json)?)
}

/// スキーマのバージョンが読める範囲か検査する (`docs/spec/file-formats.md` のバージョニング)。
fn check_schema_version(model: &Model) -> Result<(), FormatError> {
    if model.schema_version > ugoe_model::SCHEMA_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: model.schema_version,
            supported: ugoe_model::SCHEMA_VERSION,
        });
    }
    Ok(())
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
    fn physics_json_roundtrip() {
        let physics = ugoe_model::fixtures::physics_hair_settings();
        let back = physics_from_json(&physics_to_json(&physics).unwrap()).unwrap();
        assert_eq!(physics, back);
    }
}
