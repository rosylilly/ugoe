use serde::{Deserialize, Serialize};
use ugoe_core::Id;

pub type ParamId = Id<Parameter>;

/// モデルを動かす入力スカラー値 (例: 顔の角度 X, 目の開閉)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    /// 中継用 (内部) のパラメータか。物理のリグの間で値を受け渡すためのもので
    /// (`docs/spec/physics.md#リグの順序と中継`)、トラッキングは書き込まず、
    /// ビューアなどのアプリの一覧では既定で隠す (`docs/spec/model.md#パラメータ`)。省略時は `false`。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub internal: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    #[default]
    Normal,
    BlendShape,
    /// 未知の値 (新しい版のファイル)。通常のパラメータとして扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParameterGroup {
    pub id: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `internal` は省略すると `false` で、`false` なら書き出さない。
    #[test]
    fn internal_defaults_to_false_and_is_skipped() {
        let json = r#"{"id":"P","name":"P","min":0,"max":1,"default":0}"#;
        let mut p: Parameter = serde_json::from_str(json).unwrap();
        assert!(!p.internal);
        let value = serde_json::to_value(&p).unwrap();
        assert!(value.get("internal").is_none());

        p.internal = true;
        let value = serde_json::to_value(&p).unwrap();
        assert_eq!(value["internal"], true);
        assert_eq!(serde_json::from_value::<Parameter>(value).unwrap(), p);
    }
}
