use ugoe_model::ParamId;

use crate::KeyformTarget;

/// 推定のエラー。エラーのときは格子を変えない。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InferError {
    /// 格子の軸のパラメータがモデルにない。
    #[error("axis parameter {parameter} does not exist in the model")]
    UnknownParameter { parameter: ParamId },
    /// 軸のキーにパラメータの `default` と等しいもの (中立のキー) がない。
    #[error("axis {parameter} has no neutral key (default {default} is not among its keys)")]
    MissingNeutralKey { parameter: ParamId, default: f32 },
    /// 軸が多すぎる (64 本まで)。
    #[error("too many axes ({count}, at most 64)")]
    TooManyAxes { count: usize },
    /// `keyforms` の数が軸のキー数の積と合わない。
    #[error("grid has {found} keyforms, expected {expected}")]
    GridSizeMismatch { expected: usize, found: usize },
    /// `origins` が空でなく、長さが `keyforms` と合わない。
    #[error("grid has {found} origins, expected {expected}")]
    OriginsLengthMismatch { expected: usize, found: usize },
    /// 基準のセル (すべての軸が中立のセル) が `authored` でない。
    #[error("base cell {cell} must be authored")]
    BaseNotAuthored { cell: usize },
    /// `authored` のセルの頂点 (制御点) の数が基準のセルと合わない。
    #[error("cell {cell} has {found} vertices, expected {expected} (as the base cell)")]
    ShapeMismatch {
        cell: usize,
        expected: usize,
        found: usize,
    },
    /// 対象の要素がモデルにない。
    #[error("target {0:?} does not exist in the model")]
    UnknownTarget(KeyformTarget),
    /// 対象の要素が加算合成できる格子を持たない。
    #[error("target {0:?} has no keyform grid that supports additive inference")]
    UnsupportedTarget(KeyformTarget),
}
