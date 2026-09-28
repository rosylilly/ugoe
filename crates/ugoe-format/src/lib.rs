//! ファイル形式の読み書き。仕様: `docs/spec/file-formats.md`。
//!
//! - `.ugoe` : ランタイムバンドル (ヘッダ + チャンク表 + チャンク)。バイナリ。[`bundle`] を参照。
//! - `*.motion.json` / `*.expression.json` / `*.physics.json` : JSON。
//!
//! 読み込みは **常に** スキーマバージョンを検査し、未知の新しい版は明示的なエラーにする。

pub mod bundle;

use ugoe_model::Model;
use ugoe_model::validate::ValidationError;

pub use bundle::{
    Bundle, ChunkTag, TextureData, TextureFormat, WriteOptions, read_bundle, write_bundle,
    write_bundle_with,
};

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
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
}

/// モデルを JSON 表現で書き出す (デバッグ・差分確認・AI 入出力用)。
pub fn model_to_json(model: &Model) -> Result<String, FormatError> {
    Ok(serde_json::to_string_pretty(model)?)
}

/// JSON 表現からモデルを読み込む。
pub fn model_from_json(json: &str) -> Result<Model, FormatError> {
    let model: Model = serde_json::from_str(json)?;
    check_schema_version(&model)?;
    Ok(model)
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
}
