use serde::{Deserialize, Serialize};

use crate::ParamId;

/// 表情: モーションの上に重ねるパラメータ差分。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expression {
    pub name: String,
    #[serde(default)]
    pub fade_in: f32,
    #[serde(default)]
    pub fade_out: f32,
    pub parameters: Vec<ExpressionParameter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionParameter {
    pub parameter: ParamId,
    pub value: f32,
    pub op: ExpressionOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionOp {
    Add,
    Multiply,
    Overwrite,
}
