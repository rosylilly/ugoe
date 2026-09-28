//! ファイル形式の読み書き。仕様: `docs/spec/file-formats.md`。
//!
//! - `.ugoe` : ランタイムバンドル (ヘッダ + モデル + テクスチャ)。バイナリ。
//! - `*.motion.json` / `*.expression.json` / `*.physics.json` : JSON。
//!
//! 読み込みは **常に** スキーマバージョンを検査し、未知の新しい版は明示的なエラーにする。

use ugoe_model::Model;

/// `.ugoe` ファイル先頭のマジックバイト。
pub const MAGIC: [u8; 4] = *b"UGOE";

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not a ugoe file")]
    BadMagic,
    #[error("unsupported schema version {found} (supported up to {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// モデルを JSON 表現で書き出す (デバッグ・差分確認・AI 入出力用)。
pub fn model_to_json(model: &Model) -> Result<String, FormatError> {
    Ok(serde_json::to_string_pretty(model)?)
}

/// JSON 表現からモデルを読み込む。
pub fn model_from_json(json: &str) -> Result<Model, FormatError> {
    let model: Model = serde_json::from_str(json)?;
    if model.schema_version > ugoe_model::SCHEMA_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: model.schema_version,
            supported: ugoe_model::SCHEMA_VERSION,
        });
    }
    Ok(model)
}

// TODO(T0105): `.ugoe` バイナリコンテナ (MAGIC + ヘッダ + チャンク) の読み書き。

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
