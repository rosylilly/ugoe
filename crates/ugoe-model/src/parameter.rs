use serde::{Deserialize, Serialize};
use ugoe_core::Id;

pub type ParamId = Id<Parameter>;

/// モデルを動かす入力スカラー値 (例: 顔の角度 X, 目の開閉)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub id: ParamId,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// ブレンドシェイプ用パラメータか (加算変形。`docs/spec/model.md`)。
    #[serde(default)]
    pub kind: ParameterKind,
    /// 左右ペア (例: EyeLOpen ↔ EyeROpen)。エディタのミラー機能・AI 自動設定で使う。
    #[serde(default)]
    pub mirror: Option<ParamId>,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    #[default]
    Normal,
    BlendShape,
    /// 未知の値 (新しい版のファイル)。通常のパラメータとして扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    Unknown,
}

impl ParameterKind {
    /// フォールバックを適用した種類 (`Unknown` は通常のパラメータ)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Normal,
            kind => kind,
        }
    }
}

/// エディタ表示用のパラメータグループ。ランタイムの挙動には影響しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterGroup {
    pub id: String,
    pub name: String,
}
