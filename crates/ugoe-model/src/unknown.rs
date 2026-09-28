//! 未知の値 (拡張可能な enum の `Unknown`) の検出。
//! 仕様: `docs/spec/file-formats.md#未知の値の読み込み`。
//!
//! 新しい版のファイルを読むと、知らないタグは `Unknown` になる。`Unknown` は書き出せないので、
//! 読み込み時の警告と、書き出し前の検査に使う。返すのは値の場所 (`meshes[Face].blend` など)。

use crate::Model;
use crate::avatar::{
    AvatarPermission, AvatarProfile, CommercialUsage, CreditNotation, Modification, TrackingSource,
};
use crate::deformer::DeformerKind;
use crate::expression::{Expression, ExpressionOp};
use crate::keyform::{KeyformGrid, KeyformInference, KeyformOrigin};
use crate::mesh::{BlendMode, MaskMode};
use crate::motion::{CurveTarget, Motion, Segment};
use crate::parameter::ParameterKind;
use crate::physics::{ColliderShape, PhysicsSettings, PhysicsSource, PhysicsTarget};

/// モデルの中の未知の値の場所をすべて返す。
pub fn in_model(model: &Model) -> Vec<String> {
    let mut found = Vec::new();
    for parameter in &model.parameters {
        if parameter.kind == ParameterKind::Unknown {
            found.push(format!("parameters[{}].kind", parameter.id));
        }
    }
    for part in &model.parts {
        check_grid(&format!("parts[{}]", part.id), &part.keyforms, &mut found);
    }
    for deformer in &model.deformers {
        let path = format!("deformers[{}]", deformer.id);
        match &deformer.kind {
            DeformerKind::Warp(warp) => check_grid(&path, &warp.keyforms, &mut found),
            DeformerKind::Rotation(rotation) => check_grid(&path, &rotation.keyforms, &mut found),
            DeformerKind::Unknown => found.push(format!("{path}.kind")),
        }
    }
    for mesh in &model.meshes {
        let path = format!("meshes[{}]", mesh.id);
        if mesh.blend == BlendMode::Unknown {
            found.push(format!("{path}.blend"));
        }
        if mesh.mask_mode == MaskMode::Unknown {
            found.push(format!("{path}.mask_mode"));
        }
        check_grid(&path, &mesh.keyforms, &mut found);
    }
    for glue in &model.glues {
        check_grid(&format!("glues[{}]", glue.id), &glue.keyforms, &mut found);
    }
    if let Some(avatar) = &model.avatar {
        check_avatar(avatar, &mut found);
    }
    found
}

/// アバタープロファイルの許諾とトラッキングの入力の種類。
fn check_avatar(avatar: &AvatarProfile, found: &mut Vec<String>) {
    let meta = &avatar.meta;
    let fields = [
        (
            "avatar_permission",
            meta.avatar_permission == AvatarPermission::Unknown,
        ),
        (
            "commercial_usage",
            meta.commercial_usage == CommercialUsage::Unknown,
        ),
        (
            "credit_notation",
            meta.credit_notation == CreditNotation::Unknown,
        ),
        ("modification", meta.modification == Modification::Unknown),
    ];
    for (field, is_unknown) in fields {
        if is_unknown {
            found.push(format!("avatar.meta.{field}"));
        }
    }
    for (index, entry) in avatar.tracking.iter().enumerate() {
        if entry.source == TrackingSource::Unknown {
            found.push(format!("avatar.tracking[{index}].source"));
        }
    }
}

/// 物理の設定の中の未知の値の場所をすべて返す。
pub fn in_physics(physics: &PhysicsSettings) -> Vec<String> {
    let mut found = Vec::new();
    for rig in &physics.rigs {
        for (index, input) in rig.inputs.iter().enumerate() {
            if input.source == PhysicsSource::Unknown {
                found.push(format!("rigs[{}].inputs[{index}].source", rig.id));
            }
        }
        for (index, output) in rig.outputs.iter().enumerate() {
            if output.target == PhysicsTarget::Unknown {
                found.push(format!("rigs[{}].outputs[{index}].target", rig.id));
            }
            if output.source == PhysicsSource::Unknown {
                found.push(format!("rigs[{}].outputs[{index}].source", rig.id));
            }
        }
    }
    for collider in &physics.colliders {
        if collider.shape == ColliderShape::Unknown {
            found.push(format!("colliders[{}].shape", collider.id));
        }
    }
    found
}

/// モーションの中の未知の値の場所をすべて返す。
pub fn in_motion(motion: &Motion) -> Vec<String> {
    let mut found = Vec::new();
    for (index, curve) in motion.curves.iter().enumerate() {
        if curve.target == CurveTarget::Unknown {
            found.push(format!("curves[{index}].target"));
        }
        for (s, segment) in curve.segments.iter().enumerate() {
            if *segment == Segment::Unknown {
                found.push(format!("curves[{index}].segments[{s}]"));
            }
        }
    }
    found
}

/// 表情の中の未知の値の場所をすべて返す。
pub fn in_expression(expression: &Expression) -> Vec<String> {
    expression
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, p)| p.op == ExpressionOp::Unknown)
        .map(|(index, _)| format!("parameters[{index}].op"))
        .collect()
}

fn check_grid<T>(path: &str, grid: &KeyformGrid<T>, found: &mut Vec<String>) {
    if let Some(index) = grid
        .origins
        .iter()
        .position(|o| *o == KeyformOrigin::Unknown)
    {
        found.push(format!("{path}.keyforms.origins[{index}]"));
    }
    if let Some(KeyformInference::Unknown) = grid.inference {
        found.push(format!("{path}.keyforms.inference"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    use serde_json::json;

    #[test]
    fn baseline_fixture_has_no_unknown_values() {
        assert!(in_model(&fixtures::single_quad()).is_empty());
    }

    /// 新しい版で増えた値を含むモデルの JSON (未知のタグと、そのタグの未知の中身)。
    fn future_model_json() -> serde_json::Value {
        let mut value = serde_json::to_value(fixtures::single_quad()).unwrap();
        let mesh = &mut value["meshes"][0];
        mesh["blend"] = json!("overlay");
        mesh["mask_mode"] = json!("xor");
        mesh["keyforms"]["origins"] = json!(["authored", "predicted", "derived"]);
        mesh["keyforms"]["inference"] = json!({ "method": "neural", "weights": [1, 2] });
        value["parameters"][0]["kind"] = json!("vector");
        value["deformers"] = json!([{
            "id": "Bend",
            "name": "Bend",
            "part": "Root",
            "parent": null,
            "kind": { "type": "bezier", "handles": [[0.0, 1.0]], "extra": { "a": 1 } },
        }]);
        value
    }

    #[test]
    fn unknown_tags_in_json_become_unknown() {
        let model: Model = serde_json::from_value(future_model_json()).unwrap();
        let mesh = &model.meshes[0];
        assert_eq!(mesh.blend, BlendMode::Unknown);
        assert_eq!(mesh.blend.effective(), BlendMode::Normal);
        assert_eq!(mesh.mask_mode.effective(), MaskMode::Union);
        assert_eq!(mesh.keyforms.origins[1], KeyformOrigin::Unknown);
        assert_eq!(mesh.keyforms.origin(1), KeyformOrigin::Authored);
        assert_eq!(mesh.keyforms.inference, Some(KeyformInference::Unknown));
        assert_eq!(model.parameters[0].kind.effective(), ParameterKind::Normal);
        assert_eq!(model.deformers[0].kind, DeformerKind::Unknown);
        assert_eq!(
            in_model(&model),
            [
                "parameters[X].kind",
                "deformers[Bend].kind",
                "meshes[Quad].blend",
                "meshes[Quad].mask_mode",
                "meshes[Quad].keyforms.origins[1]",
                "meshes[Quad].keyforms.inference",
            ]
        );
        // 未知の値は検証エラーにしない。
        assert!(crate::validate::validate(&model).is_empty());
    }

    #[test]
    fn unknown_avatar_values_are_found() {
        let mut value = serde_json::to_value(fixtures::avatar_profile()).unwrap();
        value["avatar"]["meta"]["commercial_usage"] = json!("government");
        value["avatar"]["tracking"] = json!([
            { "source": "blend_shape", "input": "jawForward", "parameter": "X" },
            { "source": "hand", "input": "thumb", "parameter": "X" },
        ]);
        let model: Model = serde_json::from_value(value).unwrap();
        assert_eq!(
            in_model(&model),
            ["avatar.meta.commercial_usage", "avatar.tracking[1].source"]
        );
    }

    #[test]
    fn unknown_values_cannot_be_serialized() {
        let mut model = fixtures::single_quad();
        model.meshes[0].blend = BlendMode::Unknown;
        assert!(serde_json::to_string(&model).is_err());
        let target = CurveTarget::Unknown;
        assert!(serde_json::to_string(&target).is_err());
    }

    #[test]
    fn unknown_curve_targets_segments_and_ops() {
        let motion: Motion = serde_json::from_value(json!({
            "name": "m",
            "duration": 1.0,
            "curves": [
                {
                    "target": { "kind": "camera_zoom", "id": { "camera": 2 } },
                    "segments": [{ "type": "start", "p": { "t": 0.0, "v": 0.0 } }],
                },
                {
                    // 知っている種類は、キーの順序によらず読める。
                    "target": { "id": "X", "kind": "parameter" },
                    "segments": [
                        { "type": "start", "p": { "t": 0.0, "v": 0.0 } },
                        { "type": "hermite", "to": { "t": 1.0, "v": 1.0 }, "tangents": [0, 1] },
                    ],
                },
                {
                    "target": { "kind": "future_without_id" },
                    "segments": [],
                },
            ],
        }))
        .unwrap();
        assert_eq!(motion.curves[0].target, CurveTarget::Unknown);
        assert_eq!(motion.curves[1].target, CurveTarget::Parameter("X".into()));
        assert_eq!(motion.curves[1].segments[1], Segment::Unknown);
        assert_eq!(motion.curves[2].target, CurveTarget::Unknown);
        assert_eq!(
            in_motion(&motion),
            [
                "curves[0].target",
                "curves[1].segments[1]",
                "curves[2].target"
            ]
        );

        // 知っている種類で `id` が文字列でなければ、これまでどおりエラーにする。
        for target in [
            json!({ "kind": "parameter" }),
            json!({ "kind": "parameter", "id": 1 }),
        ] {
            assert!(serde_json::from_value::<CurveTarget>(target).is_err());
        }

        let expression: Expression = serde_json::from_value(json!({
            "name": "e",
            "parameters": [{ "parameter": "X", "value": 1.0, "op": "screen" }],
        }))
        .unwrap();
        assert_eq!(expression.parameters[0].op, ExpressionOp::Unknown);
        assert_eq!(in_expression(&expression), ["parameters[0].op"]);
    }

    #[test]
    fn unknown_physics_sources() {
        let physics: PhysicsSettings = serde_json::from_value(json!({
            "gravity": [0.0, -1.0],
            "wind": [0.0, 0.0],
            "fps": 60.0,
            "rigs": [{
                "id": "Hair",
                "name": "Hair",
                "inputs": [{ "parameter": "X", "source": "z", "weight": 100.0 }],
                "outputs": [{
                    "target": { "type": "spring" },
                    "particle": 1,
                    "source": "angle",
                    "scale": 1.0,
                    "weight": 100.0,
                }],
                "particles": [],
                "normalization": {
                    "position": { "min": -1.0, "default": 0.0, "max": 1.0 },
                    "angle": { "min": -1.0, "default": 0.0, "max": 1.0 },
                },
            }],
            "colliders": [{
                "id": "Body",
                "name": "Body",
                "shape": { "type": "polygon", "points": [] },
            }],
        }))
        .unwrap();
        assert_eq!(
            in_physics(&physics),
            [
                "rigs[Hair].inputs[0].source",
                "rigs[Hair].outputs[0].target",
                "colliders[Body].shape",
            ]
        );
    }
}
