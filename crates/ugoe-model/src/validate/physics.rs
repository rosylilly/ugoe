//! 物理の設定の検査。検査項目は `docs/spec/model.md#物理の検証`。

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use super::{
    Refs, ValidationError, check_finite, check_non_negative, check_vec2, collect_ids,
    warp_point_count,
};
use crate::deformer::DeformerKind;
use crate::physics::{
    ColliderShape, NormalizationRange, PhysicsSettings, PhysicsTarget, VertexOwner,
};
use crate::{DeformerId, Model};

/// 物理の設定を、参照先のモデルと合わせて検査する。検出したエラーを全て返す。
///
/// モデル自体の検査 ([`super::validate`]) は含まない。
pub fn validate_physics(physics: &PhysicsSettings, model: &Model) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // モデル側の重複 ID は `validate` が報告するので、ここでは表を作るだけにする。
    let mut ignored = Vec::new();
    let params = collect_ids(
        "parameter",
        model.parameters.iter().map(|p| &p.id),
        &mut ignored,
    );
    let deformers = collect_ids(
        "deformer",
        model.deformers.iter().map(|d| &d.id),
        &mut ignored,
    );
    let meshes = collect_ids("mesh", model.meshes.iter().map(|m| &m.id), &mut ignored);

    collect_ids(
        "physics rig",
        physics.rigs.iter().map(|r| &r.id),
        &mut errors,
    );
    let colliders = collect_ids(
        "physics collider",
        physics.colliders.iter().map(|c| &c.id),
        &mut errors,
    );

    check_vec2("gravity", physics.gravity, &mut errors);
    check_vec2("wind", physics.wind, &mut errors);
    // 0 以下はランタイムが既定の周波数に置き換えるので許す (`docs/spec/physics.md#時間の進め方`)。
    check_finite("fps", physics.fps, &mut errors);

    for rig in &physics.rigs {
        let refs = Refs {
            kind: "physics rig",
            id: rig.id.as_str(),
        };
        let path = format!("rigs[{}]", rig.id);
        for (index, input) in rig.inputs.iter().enumerate() {
            refs.check(
                &params,
                "inputs",
                "parameter",
                &input.parameter,
                &mut errors,
            );
            check_finite(
                &format!("{path}.inputs[{index}].weight"),
                input.weight,
                &mut errors,
            );
        }
        // 出力の `particle` が 0 や範囲外なら、ランタイムがその出力を無視する (エラーにしない)。
        for (index, output) in rig.outputs.iter().enumerate() {
            let at = format!("{path}.outputs[{index}]");
            check_finite(&format!("{at}.scale"), output.scale, &mut errors);
            check_finite(&format!("{at}.weight"), output.weight, &mut errors);
            match &output.target {
                PhysicsTarget::Parameter { id } => {
                    refs.check(&params, "outputs", "parameter", id, &mut errors);
                }
                PhysicsTarget::DeformerAngle { deformer }
                | PhysicsTarget::DeformerOffset { deformer } => {
                    let kind = refs
                        .check(&deformers, "outputs", "deformer", deformer, &mut errors)
                        .map(|d| &model.deformers[d].kind);
                    if let Some(DeformerKind::Warp(_)) = kind {
                        errors.push(kind_mismatch(&refs, deformer, "rotation"));
                    }
                }
                PhysicsTarget::Vertices { owner, weights } => {
                    let expected = match owner {
                        VertexOwner::Mesh(mesh) => refs
                            .check(&meshes, "outputs", "mesh", mesh, &mut errors)
                            .map(|m| model.meshes[m].uvs.len()),
                        VertexOwner::Warp(deformer) => {
                            let found = refs.check(
                                &deformers,
                                "outputs",
                                "deformer",
                                deformer,
                                &mut errors,
                            );
                            match found.map(|d| &model.deformers[d].kind) {
                                Some(DeformerKind::Warp(warp)) => {
                                    Some(warp_point_count(warp.rows, warp.cols))
                                }
                                Some(DeformerKind::Rotation(_)) => {
                                    errors.push(kind_mismatch(&refs, deformer, "warp"));
                                    None
                                }
                                // 未知の種類と存在しないデフォーマは、重みの数を検査しない。
                                Some(DeformerKind::Unknown) | None => None,
                            }
                        }
                        // 未知の対象の出力は無視する (`docs/spec/file-formats.md#未知の値の読み込み`)。
                        VertexOwner::Unknown => continue,
                    };
                    if let Some(expected) = expected.filter(|&e| e != weights.len()) {
                        errors.push(ValidationError::WeightCount {
                            rig: rig.id.to_string(),
                            output: index,
                            expected,
                            actual: weights.len(),
                        });
                    }
                    // 範囲外の重みは最初の 1 つだけ報告する。
                    if let Some((w, &weight)) = weights
                        .iter()
                        .enumerate()
                        .find(|(_, w)| !(0.0..=1.0).contains(*w))
                    {
                        errors.push(ValidationError::InvalidValue {
                            field: format!("{path}.outputs[{index}].weights[{w}]"),
                            value: weight,
                            expected: "in 0..=1",
                        });
                    }
                }
                // 未知の出力先は無視する (`docs/spec/file-formats.md#未知の値の読み込み`)。
                PhysicsTarget::Unknown => {}
            }
        }
        // 粒子が 0 個のリグはエラーにしない (出力がすべて範囲外になり、何もしない)。
        for (index, particle) in rig.particles.iter().enumerate() {
            let at = format!("{path}.particles[{index}]");
            check_vec2(&format!("{at}.position"), particle.position, &mut errors);
            // 1 を超えると揺れが増幅して発散する (`docs/spec/physics.md#係数の意味`)。
            if !(0.0..=1.0).contains(&particle.mobility) {
                errors.push(ValidationError::InvalidValue {
                    field: format!("{at}.mobility"),
                    value: particle.mobility,
                    expected: "in 0..=1",
                });
            }
            let values = [
                ("delay", particle.delay),
                ("acceleration", particle.acceleration),
                ("radius", particle.radius),
                ("collision_radius", particle.collision_radius),
            ];
            for (name, value) in values {
                check_non_negative(&format!("{at}.{name}"), value, &mut errors);
            }
        }
        let ranges = [
            ("position", rig.normalization.position),
            ("angle", rig.normalization.angle),
        ];
        for (name, range) in ranges {
            check_normalization(&format!("{path}.normalization.{name}"), range, &mut errors);
        }
        for collider in &rig.colliders {
            refs.check(
                &colliders,
                "colliders",
                "physics collider",
                collider,
                &mut errors,
            );
        }
        if let Some(frame) = &rig.frame {
            refs.check(&deformers, "frame", "deformer", frame, &mut errors);
        }
    }

    for collider in &physics.colliders {
        let refs = Refs {
            kind: "physics collider",
            id: collider.id.as_str(),
        };
        if let Some(attach) = &collider.attach {
            refs.check(&deformers, "attach", "deformer", attach, &mut errors);
        }
        let path = format!("colliders[{}].shape", collider.id);
        let (points, radius) = match &collider.shape {
            ColliderShape::Circle { center, radius } => (vec![("center", *center)], *radius),
            ColliderShape::Capsule { a, b, radius } => (vec![("a", *a), ("b", *b)], *radius),
            ColliderShape::Unknown => continue,
        };
        for (name, point) in points {
            check_finite(&format!("{path}.{name}.x"), point.x, &mut errors);
            check_finite(&format!("{path}.{name}.y"), point.y, &mut errors);
        }
        check_non_negative(&format!("{path}.radius"), radius, &mut errors);
    }

    // スロットが公開する衝突体 (`docs/spec/modules.md#スロット`)。
    for slot in &model.slots {
        let refs = Refs {
            kind: "slot",
            id: slot.id.as_str(),
        };
        for collider in &slot.colliders {
            refs.check(
                &colliders,
                "colliders",
                "physics collider",
                collider,
                &mut errors,
            );
        }
    }

    errors
}

/// 正規化の範囲: 3 つとも有限値で、`min <= default <= max`。
fn check_normalization(path: &str, range: NormalizationRange, errors: &mut Vec<ValidationError>) {
    let values = [
        ("min", range.min),
        ("default", range.default),
        ("max", range.max),
    ];
    let mut finite = true;
    for (name, value) in values {
        finite &= value.is_finite();
        check_finite(&format!("{path}.{name}"), value, errors);
    }
    // 有限でない値は上で報告したので、順序は有限のときだけ見る。
    if finite && !(range.min <= range.default && range.default <= range.max) {
        errors.push(ValidationError::InvalidValue {
            field: format!("{path}.default"),
            value: range.default,
            expected: "between min and max",
        });
    }
}

fn kind_mismatch(
    refs: &Refs<'_>,
    deformer: &DeformerId,
    expected: &'static str,
) -> ValidationError {
    ValidationError::DeformerKindMismatch {
        kind: refs.kind,
        id: refs.id.to_owned(),
        field: "outputs",
        deformer: deformer.to_string(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::physics::{
        NormalizationRange, PhysicsBlend, PhysicsCollider, PhysicsInput, PhysicsNormalization,
        PhysicsOutput, PhysicsParticle, PhysicsRig, PhysicsSource,
    };
    use ugoe_core::Vec2;

    /// ワープ・回転デフォーマとメッシュを持つモデルと、その要素を参照する物理。
    fn setup() -> (Model, PhysicsSettings) {
        let model = fixtures::rotation_under_warp();
        let find = |warp: bool| {
            model
                .deformers
                .iter()
                .find(|d| matches!(d.kind, DeformerKind::Warp(_)) == warp)
                .unwrap()
        };
        let warp = find(true);
        let DeformerKind::Warp(grid) = &warp.kind else {
            unreachable!()
        };
        let points = (usize::from(grid.rows) + 1) * (usize::from(grid.cols) + 1);
        let rotation = find(false).id.clone();
        let mesh = &model.meshes[0];
        let output = |target| PhysicsOutput {
            target,
            particle: 1,
            source: PhysicsSource::Angle,
            scale: 1.0,
            weight: 100.0,
            reflect: false,
            blend: PhysicsBlend::Overwrite,
        };
        let range = NormalizationRange {
            min: -1.0,
            default: 0.0,
            max: 1.0,
        };
        let physics = PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: vec![PhysicsRig {
                id: "Hair".into(),
                name: "Hair".into(),
                inputs: vec![PhysicsInput {
                    parameter: model.parameters[0].id.clone(),
                    source: PhysicsSource::X,
                    weight: 100.0,
                    reflect: false,
                }],
                outputs: vec![
                    output(PhysicsTarget::Parameter {
                        id: model.parameters[0].id.clone(),
                    }),
                    output(PhysicsTarget::DeformerAngle {
                        deformer: rotation.clone(),
                    }),
                    output(PhysicsTarget::DeformerOffset {
                        deformer: rotation.clone(),
                    }),
                    output(PhysicsTarget::Vertices {
                        owner: VertexOwner::Mesh(mesh.id.clone()),
                        weights: vec![0.5; mesh.uvs.len()],
                    }),
                    output(PhysicsTarget::Vertices {
                        owner: VertexOwner::Warp(warp.id.clone()),
                        weights: vec![1.0; points],
                    }),
                ],
                particles: vec![PhysicsParticle {
                    position: Vec2::ZERO,
                    mobility: 1.0,
                    delay: 1.0,
                    acceleration: 1.0,
                    radius: 0.0,
                    collision_radius: 2.0,
                }],
                normalization: PhysicsNormalization {
                    position: range,
                    angle: range,
                },
                colliders: vec!["Body".into()],
                frame: Some(rotation.clone()),
            }],
            colliders: vec![PhysicsCollider {
                id: "Body".into(),
                name: "Body".into(),
                attach: Some(warp.id.clone()),
                shape: ColliderShape::Capsule {
                    a: Vec2::new(0.0, 0.0),
                    b: Vec2::new(0.0, 1.0),
                    radius: 5.0,
                },
            }],
        };
        (model, physics)
    }

    #[test]
    fn valid_dynamic_physics_passes() {
        let (model, physics) = setup();
        assert_eq!(validate_physics(&physics, &model), []);
    }

    #[test]
    fn hair_fixture_is_valid() {
        let physics = fixtures::physics_hair_settings();
        assert_eq!(validate_physics(&physics, &fixtures::physics_hair()), []);
    }

    #[test]
    fn reports_unknown_references() {
        let (model, mut physics) = setup();
        let rig = &mut physics.rigs[0];
        rig.outputs[0].target = PhysicsTarget::Parameter { id: "Nope".into() };
        rig.outputs[1].target = PhysicsTarget::DeformerAngle {
            deformer: "Nope".into(),
        };
        rig.outputs[3].target = PhysicsTarget::Vertices {
            owner: VertexOwner::Mesh("Nope".into()),
            weights: vec![],
        };
        rig.colliders.push("Nope".into());
        rig.frame = Some("Nope".into());
        physics.colliders[0].attach = Some("Nope".into());
        let fields: Vec<_> = validate_physics(&physics, &model)
            .into_iter()
            .map(|e| match e {
                ValidationError::UnknownReference {
                    kind,
                    field,
                    target_kind,
                    ..
                } => (kind, field, target_kind),
                e => panic!("unexpected {e}"),
            })
            .collect();
        assert_eq!(
            fields,
            [
                ("physics rig", "outputs", "parameter"),
                ("physics rig", "outputs", "deformer"),
                ("physics rig", "outputs", "mesh"),
                ("physics rig", "colliders", "physics collider"),
                ("physics rig", "frame", "deformer"),
                ("physics collider", "attach", "deformer"),
            ]
        );
    }

    #[test]
    fn reports_wrong_deformer_kinds_and_weights() {
        let (model, mut physics) = setup();
        let rig = &mut physics.rigs[0];
        let PhysicsTarget::Vertices {
            owner: VertexOwner::Warp(warp),
            ..
        } = rig.outputs[4].target.clone()
        else {
            unreachable!()
        };
        let PhysicsTarget::DeformerAngle { deformer: rotation } = rig.outputs[1].target.clone()
        else {
            unreachable!()
        };
        rig.outputs[2].target = PhysicsTarget::DeformerOffset {
            deformer: warp.clone(),
        };
        rig.outputs[4].target = PhysicsTarget::Vertices {
            owner: VertexOwner::Warp(rotation.clone()),
            weights: vec![],
        };
        if let PhysicsTarget::Vertices { weights, .. } = &mut rig.outputs[3].target {
            weights.push(1.5);
        }
        rig.particles[0].collision_radius = -1.0;
        physics.colliders[0].shape = ColliderShape::Circle {
            center: Vec2::new(f32::INFINITY, 0.0),
            radius: -1.0,
        };
        let errors = validate_physics(&physics, &model);
        let vertices = model.meshes[0].uvs.len();
        assert_eq!(
            errors,
            [
                ValidationError::DeformerKindMismatch {
                    kind: "physics rig",
                    id: "Hair".into(),
                    field: "outputs",
                    deformer: warp.to_string(),
                    expected: "rotation",
                },
                ValidationError::WeightCount {
                    rig: "Hair".into(),
                    output: 3,
                    expected: vertices,
                    actual: vertices + 1,
                },
                ValidationError::InvalidValue {
                    field: format!("rigs[Hair].outputs[3].weights[{vertices}]"),
                    value: 1.5,
                    expected: "in 0..=1",
                },
                ValidationError::DeformerKindMismatch {
                    kind: "physics rig",
                    id: "Hair".into(),
                    field: "outputs",
                    deformer: rotation.to_string(),
                    expected: "warp",
                },
                ValidationError::InvalidValue {
                    field: "rigs[Hair].particles[0].collision_radius".into(),
                    value: -1.0,
                    expected: "finite and >= 0",
                },
                ValidationError::InvalidValue {
                    field: "colliders[Body].shape.center.x".into(),
                    value: f32::INFINITY,
                    expected: "finite",
                },
                ValidationError::InvalidValue {
                    field: "colliders[Body].shape.radius".into(),
                    value: -1.0,
                    expected: "finite and >= 0",
                },
            ]
        );
    }

    // docs/spec/model.md#物理の検証: 数値は有限値。係数と長さは 0 以上、`mobility` は 0〜1、
    // 正規化の範囲は min <= default <= max。
    #[test]
    fn reports_invalid_numbers() {
        let inf = f32::INFINITY;
        let (model, mut physics) = setup();
        physics.gravity.y = inf;
        physics.wind.x = -inf;
        physics.fps = inf;
        let rig = &mut physics.rigs[0];
        rig.inputs[0].weight = inf;
        rig.outputs[0].scale = inf;
        rig.outputs[2].weight = -inf;
        let particle = &mut rig.particles[0];
        particle.position.x = inf;
        particle.mobility = 1.5;
        particle.delay = -1.0;
        particle.acceleration = -1.0;
        particle.radius = inf;
        rig.normalization.position.max = inf;
        rig.normalization.angle.default = 2.0;
        let errors = validate_physics(&physics, &model);
        let invalid = |field: &str, value, expected| ValidationError::InvalidValue {
            field: field.into(),
            value,
            expected,
        };
        assert_eq!(
            errors,
            [
                invalid("gravity.y", inf, "finite"),
                invalid("wind.x", -inf, "finite"),
                invalid("fps", inf, "finite"),
                invalid("rigs[Hair].inputs[0].weight", inf, "finite"),
                invalid("rigs[Hair].outputs[0].scale", inf, "finite"),
                invalid("rigs[Hair].outputs[2].weight", -inf, "finite"),
                invalid("rigs[Hair].particles[0].position.x", inf, "finite"),
                invalid("rigs[Hair].particles[0].mobility", 1.5, "in 0..=1"),
                invalid("rigs[Hair].particles[0].delay", -1.0, "finite and >= 0"),
                invalid(
                    "rigs[Hair].particles[0].acceleration",
                    -1.0,
                    "finite and >= 0"
                ),
                invalid("rigs[Hair].particles[0].radius", inf, "finite and >= 0"),
                invalid("rigs[Hair].normalization.position.max", inf, "finite"),
                invalid(
                    "rigs[Hair].normalization.angle.default",
                    2.0,
                    "between min and max"
                ),
            ]
        );

        // NaN の `mobility` も範囲の外。
        let (model, mut physics) = setup();
        physics.rigs[0].particles[0].mobility = f32::NAN;
        let errors = validate_physics(&physics, &model);
        assert!(
            matches!(
                errors.as_slice(),
                [ValidationError::InvalidValue { field, .. }]
                    if field == "rigs[Hair].particles[0].mobility"
            ),
            "{errors:?}"
        );
    }

    // ランタイムのフォールバックに任せる値はエラーにしない: 0 以下の `fps` (既定の 60)、
    // 0 や範囲外の出力の `particle` (その出力を無視)、粒子が 0 個のリグ、範囲の外の出力の `weight` (クランプ)。
    #[test]
    fn runtime_fallbacks_are_not_errors() {
        let (model, mut physics) = setup();
        physics.fps = 0.0;
        let rig = &mut physics.rigs[0];
        rig.outputs[0].particle = 0;
        rig.outputs[1].particle = 99;
        rig.outputs[2].weight = 250.0;
        let mut empty = rig.clone();
        empty.id = "Empty".into();
        empty.particles.clear();
        physics.rigs.push(empty);
        assert_eq!(validate_physics(&physics, &model), []);
        physics.fps = -30.0;
        assert_eq!(validate_physics(&physics, &model), []);
    }

    #[test]
    fn reports_duplicate_rig_and_collider_ids() {
        let (model, mut physics) = setup();
        physics.rigs.push(physics.rigs[0].clone());
        physics.colliders.push(physics.colliders[0].clone());
        assert_eq!(
            validate_physics(&physics, &model),
            [
                ValidationError::DuplicateId {
                    kind: "physics rig",
                    id: "Hair".into(),
                },
                ValidationError::DuplicateId {
                    kind: "physics collider",
                    id: "Body".into(),
                },
            ]
        );
    }

    #[test]
    fn unknown_targets_and_shapes_are_not_errors() {
        let (model, mut physics) = setup();
        physics.rigs[0].outputs[1].target = PhysicsTarget::Unknown;
        physics.colliders[0].shape = ColliderShape::Unknown;
        assert_eq!(validate_physics(&physics, &model), []);
    }
}
