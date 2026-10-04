use ugoe_model::{DeformerId, ParamId};

use crate::{KeyformTarget, WorldError};

/// 推定のエラー。エラーのときは格子を変えない。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InferError {
    /// 格子の軸のパラメータがモデルにない。
    #[error("axis parameter {parameter} does not exist in the model")]
    UnknownParameter { parameter: ParamId },
    /// 軸のキーにパラメータの `default` と等しいもの (中立のキー) がない。
    #[error("axis {parameter} has no neutral key (default {default} is not among its keys)")]
    MissingNeutralKey { parameter: ParamId, default: f32 },
    /// 同じパラメータの軸が格子に 2 本以上ある。
    #[error("parameter {parameter} is bound to more than one axis")]
    DuplicateAxis { parameter: ParamId },
    /// 軸のキーに有限でない値 (NaN・無限大) がある。
    #[error("axis {parameter} has a non-finite key {key}")]
    NonFiniteKey { parameter: ParamId, key: f32 },
    /// 軸のキーが狭義の昇順でない (降順の箇所や、重複したキーがある)。
    #[error("keys of axis {parameter} are not strictly ascending: {keys:?}")]
    KeysNotAscending { parameter: ParamId, keys: Vec<f32> },
    /// 軸が多すぎる (64 本まで)。
    #[error("too many axes ({count}, at most 64)")]
    TooManyAxes { count: usize },
    /// セルの数 (軸のキー数の積) が `usize` に収まらない。
    #[error("grid is too large (the product of the axis key counts overflows)")]
    GridTooLarge,
    /// `keyforms` の数が軸のキー数の積と合わない。
    #[error("grid has {found} keyforms, expected {expected}")]
    GridSizeMismatch { expected: usize, found: usize },
    /// `origins` が空でなく、長さが `keyforms` と合わない。
    #[error("grid has {found} origins, expected {expected}")]
    OriginsLengthMismatch { expected: usize, found: usize },
    /// 基準のセル (すべての軸が中立のセル) が推定の元にならない (`authored` でも、別の方法で
    /// 作ったセルでもなく、この推定で書き換えるセル)。
    #[error("base cell {cell} must be authored (or derived by another method)")]
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
    /// 親が異なる対のミラーで、点を親の座標系とキャンバス座標の間で写せない
    /// (`docs/spec/keyform-inference.md#親が異なる対`)。
    #[error("cannot map through the parent deformers: {0}")]
    ParentTransform(WorldError),
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

impl InferError {
    /// 機械が読む種類 (`"missing_neutral_key"` など。バリアントの名前の `snake_case`)。
    /// 補足の値はバリアントのフィールドそのもの。値は組み込み先 (エディタの翻訳など) が照らし合わせるので **変えない**。
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownParameter { .. } => "unknown_parameter",
            Self::MissingNeutralKey { .. } => "missing_neutral_key",
            Self::DuplicateAxis { .. } => "duplicate_axis",
            Self::NonFiniteKey { .. } => "non_finite_key",
            Self::KeysNotAscending { .. } => "keys_not_ascending",
            Self::TooManyAxes { .. } => "too_many_axes",
            Self::GridTooLarge => "grid_too_large",
            Self::GridSizeMismatch { .. } => "grid_size_mismatch",
            Self::OriginsLengthMismatch { .. } => "origins_length_mismatch",
            Self::BaseNotAuthored { .. } => "base_not_authored",
            Self::ShapeMismatch { .. } => "shape_mismatch",
            Self::UnknownTarget(_) => "unknown_target",
            Self::UnsupportedTarget(_) => "unsupported_target",
            Self::UnknownParent { .. } => "unknown_parent",
            Self::UnsupportedParent { .. } => "unsupported_parent",
            Self::ParentCycle { .. } => "parent_cycle",
            Self::NotMirrorPair { .. } => "not_mirror_pair",
            Self::IncompatibleMirror { .. } => "incompatible_mirror",
            Self::MirrorAxisMismatch { .. } => "mirror_axis_mismatch",
            Self::MissingMirrorKey { .. } => "missing_mirror_key",
            Self::UnmatchedVertices { .. } => "unmatched_vertices",
            Self::ParentTransform(_) => "parent_transform",
            Self::NotDepthRotation(_) => "not_depth_rotation",
            Self::MissingDepth(_) => "missing_depth",
            Self::DepthLengthMismatch { .. } => "depth_length_mismatch",
            Self::MissingRotationAxis { .. } => "missing_rotation_axis",
            Self::NonAffineParent(_) => "non_affine_parent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_fixed() {
        let parameter = ParamId::new("ParamAngleX");
        let deformer = DeformerId::new("Face");
        let target = || KeyformTarget::Mesh("EyeL".into());
        let cases = [
            (
                InferError::UnknownParameter {
                    parameter: parameter.clone(),
                },
                "unknown_parameter",
            ),
            (
                InferError::MissingNeutralKey {
                    parameter: parameter.clone(),
                    default: 0.0,
                },
                "missing_neutral_key",
            ),
            (
                InferError::DuplicateAxis {
                    parameter: parameter.clone(),
                },
                "duplicate_axis",
            ),
            (
                InferError::NonFiniteKey {
                    parameter: parameter.clone(),
                    key: f32::NAN,
                },
                "non_finite_key",
            ),
            (
                InferError::KeysNotAscending {
                    parameter: parameter.clone(),
                    keys: vec![1.0, 0.0],
                },
                "keys_not_ascending",
            ),
            (InferError::TooManyAxes { count: 65 }, "too_many_axes"),
            (InferError::GridTooLarge, "grid_too_large"),
            (
                InferError::GridSizeMismatch {
                    expected: 2,
                    found: 1,
                },
                "grid_size_mismatch",
            ),
            (
                InferError::OriginsLengthMismatch {
                    expected: 2,
                    found: 1,
                },
                "origins_length_mismatch",
            ),
            (InferError::BaseNotAuthored { cell: 0 }, "base_not_authored"),
            (
                InferError::ShapeMismatch {
                    cell: 1,
                    expected: 4,
                    found: 3,
                },
                "shape_mismatch",
            ),
            (InferError::UnknownTarget(target()), "unknown_target"),
            (
                InferError::UnsupportedTarget(target()),
                "unsupported_target",
            ),
            (
                InferError::UnknownParent {
                    deformer: deformer.clone(),
                },
                "unknown_parent",
            ),
            (
                InferError::UnsupportedParent {
                    deformer: deformer.clone(),
                },
                "unsupported_parent",
            ),
            (
                InferError::ParentCycle {
                    deformer: deformer.clone(),
                },
                "parent_cycle",
            ),
            (
                InferError::NotMirrorPair {
                    from: target(),
                    to: target(),
                },
                "not_mirror_pair",
            ),
            (
                InferError::IncompatibleMirror {
                    from: target(),
                    to: target(),
                },
                "incompatible_mirror",
            ),
            (
                InferError::MirrorAxisMismatch {
                    parameter: parameter.clone(),
                },
                "mirror_axis_mismatch",
            ),
            (
                InferError::MissingMirrorKey {
                    parameter: parameter.clone(),
                    key: 1.0,
                },
                "missing_mirror_key",
            ),
            (
                InferError::UnmatchedVertices {
                    vertices: vec![0],
                    tolerance: 1.0,
                },
                "unmatched_vertices",
            ),
            (
                InferError::ParentTransform(WorldError::Singular { index: 0 }),
                "parent_transform",
            ),
            (InferError::NotDepthRotation(target()), "not_depth_rotation"),
            (InferError::MissingDepth(target()), "missing_depth"),
            (
                InferError::DepthLengthMismatch {
                    expected: 4,
                    found: 3,
                },
                "depth_length_mismatch",
            ),
            (
                InferError::MissingRotationAxis { parameter },
                "missing_rotation_axis",
            ),
            (InferError::NonAffineParent(target()), "non_affine_parent"),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code, "{error:?}");
        }
    }
}
