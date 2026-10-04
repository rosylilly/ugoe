//! 検証エラーの機械が読む種類 (`docs/spec/model.md#検証エラーのコード`)。

use super::ValidationError;

impl ValidationError {
    /// 機械が読む種類 (`"duplicate_id"`・`"keyform_count"` など。バリアントの名前の `snake_case`)。
    ///
    /// 補足の値はバリアントのフィールドそのもの。文 (`Display`) は人が読むためのもので安定した値として扱わないが、
    /// この値は組み込み先 (エディタの翻訳など) が照らし合わせるので **変えない**。新しいバリアントには新しい値を付ける。
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateId { .. } => "duplicate_id",
            Self::KeyformCount { .. } => "keyform_count",
            Self::KeysNotAscending { .. } => "keys_not_ascending",
            Self::EmptyKeys { .. } => "empty_keys",
            Self::DuplicateBinding { .. } => "duplicate_binding",
            Self::IndicesNotTriangles { .. } => "indices_not_triangles",
            Self::IndexOutOfRange { .. } => "index_out_of_range",
            Self::VertexCount { .. } => "vertex_count",
            Self::WarpPointCount { .. } => "warp_point_count",
            Self::EmptyWarpGrid { .. } => "empty_warp_grid",
            Self::UnknownReference { .. } => "unknown_reference",
            Self::TextureOutOfRange { .. } => "texture_out_of_range",
            Self::GlueVertexOutOfRange { .. } => "glue_vertex_out_of_range",
            Self::ParentCycle { .. } => "parent_cycle",
            Self::MaskCycle { .. } => "mask_cycle",
            Self::ParameterRange { .. } => "parameter_range",
            Self::DuplicatePosePart { .. } => "duplicate_pose_part",
            Self::InvalidModuleName { .. } => "invalid_module_name",
            Self::OriginCount { .. } => "origin_count",
            Self::UnboundInferenceParameter { .. } => "unbound_inference_parameter",
            Self::AsymmetricMirror { .. } => "asymmetric_mirror",
            Self::DepthCount { .. } => "depth_count",
            Self::DeformerKindMismatch { .. } => "deformer_kind_mismatch",
            Self::WeightCount { .. } => "weight_count",
            Self::ParameterKindMismatch { .. } => "parameter_kind_mismatch",
            Self::InvalidBlendShape { .. } => "invalid_blend_shape",
            Self::InvalidValue { .. } => "invalid_value",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_fixed() {
        let s = String::new;
        let cases = [
            (
                ValidationError::DuplicateId {
                    kind: "part",
                    id: s(),
                },
                "duplicate_id",
            ),
            (
                ValidationError::KeyformCount {
                    kind: "mesh",
                    id: s(),
                    expected: 2,
                    actual: 1,
                },
                "keyform_count",
            ),
            (
                ValidationError::KeysNotAscending {
                    kind: "mesh",
                    id: s(),
                    parameter: s(),
                    keys: vec![1.0, 0.0],
                },
                "keys_not_ascending",
            ),
            (
                ValidationError::EmptyKeys {
                    kind: "mesh",
                    id: s(),
                    parameter: s(),
                },
                "empty_keys",
            ),
            (
                ValidationError::DuplicateBinding {
                    kind: "mesh",
                    id: s(),
                    parameter: s(),
                },
                "duplicate_binding",
            ),
            (
                ValidationError::IndicesNotTriangles { id: s(), len: 4 },
                "indices_not_triangles",
            ),
            (
                ValidationError::IndexOutOfRange {
                    id: s(),
                    index: 9,
                    vertices: 3,
                },
                "index_out_of_range",
            ),
            (
                ValidationError::VertexCount {
                    id: s(),
                    expected: 3,
                    actual: 2,
                },
                "vertex_count",
            ),
            (
                ValidationError::WarpPointCount {
                    id: s(),
                    expected: 4,
                    actual: 3,
                },
                "warp_point_count",
            ),
            (
                ValidationError::EmptyWarpGrid {
                    id: s(),
                    rows: 0,
                    cols: 1,
                },
                "empty_warp_grid",
            ),
            (
                ValidationError::UnknownReference {
                    kind: "mesh",
                    id: s(),
                    field: "part",
                    target_kind: "part",
                    target: s(),
                },
                "unknown_reference",
            ),
            (
                ValidationError::TextureOutOfRange {
                    id: s(),
                    texture: 1,
                    textures: 1,
                },
                "texture_out_of_range",
            ),
            (
                ValidationError::GlueVertexOutOfRange {
                    id: s(),
                    mesh: s(),
                    index: 9,
                    vertices: 3,
                },
                "glue_vertex_out_of_range",
            ),
            (
                ValidationError::ParentCycle {
                    kind: "part",
                    cycle: vec![],
                },
                "parent_cycle",
            ),
            (ValidationError::MaskCycle { cycle: vec![] }, "mask_cycle"),
            (
                ValidationError::ParameterRange {
                    id: s(),
                    min: 1.0,
                    default: 0.0,
                    max: 2.0,
                },
                "parameter_range",
            ),
            (
                ValidationError::DuplicatePosePart { part: s() },
                "duplicate_pose_part",
            ),
            (
                ValidationError::InvalidModuleName { name: s() },
                "invalid_module_name",
            ),
            (
                ValidationError::OriginCount {
                    kind: "mesh",
                    id: s(),
                    expected: 2,
                    actual: 1,
                },
                "origin_count",
            ),
            (
                ValidationError::UnboundInferenceParameter {
                    kind: "mesh",
                    id: s(),
                    parameter: s(),
                },
                "unbound_inference_parameter",
            ),
            (
                ValidationError::AsymmetricMirror {
                    kind: "mesh",
                    id: s(),
                    mirror: s(),
                    back: None,
                },
                "asymmetric_mirror",
            ),
            (
                ValidationError::DepthCount {
                    kind: "mesh",
                    id: s(),
                    expected: 3,
                    actual: 2,
                },
                "depth_count",
            ),
            (
                ValidationError::DeformerKindMismatch {
                    kind: "mesh",
                    id: s(),
                    field: "deformer",
                    deformer: s(),
                    expected: "warp",
                },
                "deformer_kind_mismatch",
            ),
            (
                ValidationError::WeightCount {
                    rig: s(),
                    output: 0,
                    expected: 3,
                    actual: 2,
                },
                "weight_count",
            ),
            (
                ValidationError::ParameterKindMismatch {
                    kind: "mesh",
                    id: s(),
                    parameter: s(),
                    expected: "blend_shape",
                },
                "parameter_kind_mismatch",
            ),
            (
                ValidationError::InvalidBlendShape {
                    kind: "mesh",
                    id: s(),
                    index: 0,
                    reason: "has no keyforms",
                },
                "invalid_blend_shape",
            ),
            (
                ValidationError::InvalidValue {
                    field: s(),
                    value: f32::NAN,
                    expected: "finite",
                },
                "invalid_value",
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code, "{error:?}");
        }
    }
}
