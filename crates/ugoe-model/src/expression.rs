use serde::{Deserialize, Serialize};

use crate::ParamId;

/// 表情: モーションの上に重ねるパラメータ差分。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Expression {
    /// 表情の名前。ホストはこの名前で表情を止める。
    pub name: String,
    /// フェードインの時間 (秒)。0 以下ならフェードなし。
    #[serde(default)]
    pub fade_in: f32,
    /// 止めてからフェードアウトする時間 (秒)。0 以下ならフェードなし。
    #[serde(default)]
    pub fade_out: f32,
    /// 重ねるパラメータの値。
    pub parameters: Vec<ExpressionParameter>,
}

/// 表情が 1 つのパラメータに重ねる値。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ExpressionParameter {
    /// 対象のパラメータ。
    pub parameter: ParamId,
    /// `op` で現在の値と合成する値。
    pub value: f32,
    /// 現在の値との合成の方法。
    pub op: ExpressionOp,
}

/// 表情の値の合成の方法。`v` は現在の値、`w` はフェードの重み (`docs/spec/motion.md#表情`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ExpressionOp {
    /// 加算: `v + value * w`。
    Add,
    /// 乗算: `v * lerp(1, value, w)`。
    Multiply,
    /// 上書き: `lerp(v, value, w)`。
    Overwrite,
    /// 未知の値 (新しい版のファイル)。その項目を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}
