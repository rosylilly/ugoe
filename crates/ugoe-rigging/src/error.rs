use ugoe_model::{DeformerId, ParamId};

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
    /// 対象の要素がその推定に使える格子を持たない。
    #[error("target {0:?} has no keyform grid that supports this inference")]
    UnsupportedTarget(KeyformTarget),
    /// 要素の親デフォーマがモデルにない。
    #[error("parent deformer {deformer} does not exist in the model")]
    UnknownParent { deformer: DeformerId },
    /// 要素の親デフォーマの種類を扱えない (前方互換で読み飛ばした種類など)。
    #[error("parent deformer {deformer} has an unsupported kind")]
    UnsupportedParent { deformer: DeformerId },
    /// デフォーマの親をたどると循環する。
    #[error("deformer hierarchy has a cycle through {deformer}")]
    ParentCycle { deformer: DeformerId },
    /// ミラーの元 (`from`) と先 (`to`) が `mirror` で対になっていない (`docs/spec/keyform-inference.md#対応付け`)。
    #[error("{to:?} is not the mirror of {from:?}")]
    NotMirrorPair {
        from: KeyformTarget,
        to: KeyformTarget,
    },
    /// ミラーの元と先の種類や格子の大きさ (ワープの行・列) が合わない。
    #[error("{to:?} cannot be mirrored from {from:?} (different kind or warp size)")]
    IncompatibleMirror {
        from: KeyformTarget,
        to: KeyformTarget,
    },
    /// 格子の軸に、ミラーで対応する軸が相手の格子にない (または 1 対 1 に対応しない)。
    #[error("axis {parameter} has no one-to-one mirrored axis in the other grid")]
    MirrorAxisMismatch { parameter: ParamId },
    /// ミラーで対応するキー (自分自身と対にしたパラメータでは符号を反転したキー) が元の格子にない。
    #[error("source axis {parameter} has no key {key} required by the mirror")]
    MissingMirrorKey { parameter: ParamId, key: f32 },
    /// 反転した元の頂点が許容距離の中にない、先の頂点がある。`vertices` は先の頂点の添字 (昇順)。
    #[error("target vertices {vertices:?} have no mirrored source vertex within {tolerance}")]
    UnmatchedVertices {
        vertices: Vec<usize>,
        tolerance: f32,
    },
    /// 格子の推定の設定が深度からの回転 (`depth_rotation`) でない。
    #[error("target {0:?} is not configured for depth rotation")]
    NotDepthRotation(KeyformTarget),
    /// 対象が深度 (`depth`) を持たない。
    #[error("target {0:?} has no depth")]
    MissingDepth(KeyformTarget),
    /// 深度の数が頂点 (制御点) の数と合わない。
    #[error("depth has {found} values, expected {expected}")]
    DepthLengthMismatch { expected: usize, found: usize },
    /// `yaw` / `pitch` のパラメータが格子の軸にない。
    #[error("rotation parameter {parameter} is not an axis of the grid")]
    MissingRotationAxis { parameter: ParamId },
    /// 対象の親の座標系がアフィンでない (親がワープデフォーマ) ので、深度から回転できない。
    #[error("target {0:?} is under a warp deformer; depth rotation needs an affine parent space")]
    NonAffineParent(KeyformTarget),
}
