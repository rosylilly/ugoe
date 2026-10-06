//! 着せ替えのスロットとモジュールの検査。検査項目は `docs/spec/model.md#検証` と
//! `docs/spec/modules.md#検証`。

use std::collections::{HashMap, HashSet};

use ugoe_core::Vec2;

use super::{Refs, ValidationError, collect_ids, validate, validate_physics};
use crate::parameter::ParameterKind;
use crate::physics::PhysicsSettings;
use crate::{DeformerId, MeshId, Model, Module, ParamId, Parameter, PartId};

/// `Model::slots` の検査: ID の一意性、参照先の存在、描画順の範囲。
pub(super) fn check_slots(
    model: &Model,
    params: &HashMap<&ParamId, usize>,
    parts: &HashMap<&PartId, usize>,
    deformers: &HashMap<&DeformerId, usize>,
    meshes: &HashMap<&MeshId, usize>,
    errors: &mut Vec<ValidationError>,
) {
    collect_ids("slot", model.slots.iter().map(|s| &s.id), errors);
    // スロットの `pose_parts` は、ポーズの要素 (リンクは含めない) のパーツでなければならない。
    let pose_entries: HashMap<&PartId, usize> = model
        .pose
        .groups
        .iter()
        .flat_map(|g| &g.entries)
        .enumerate()
        .map(|(i, e)| (&e.part, i))
        .collect();
    for slot in &model.slots {
        let id = slot.id.to_string();
        let refs = Refs {
            kind: "slot",
            id: &id,
        };
        if let Some(anchor) = &slot.anchor {
            refs.check(deformers, "anchor", "deformer", anchor, errors);
        }
        for part in &slot.replaces {
            refs.check(parts, "replaces", "part", part, errors);
        }
        for param in &slot.parameters {
            refs.check(params, "parameters", "parameter", param, errors);
        }
        for mesh in &slot.mask_sources {
            refs.check(meshes, "mask_sources", "mesh", mesh, errors);
        }
        if let Some(part) = &slot.part {
            refs.check(parts, "part", "part", part, errors);
        }
        for part in &slot.pose_parts {
            refs.check(&pose_entries, "pose_parts", "pose entry", part, errors);
        }
        let range = slot.draw_order;
        if !range.min.is_finite() {
            errors.push(ValidationError::InvalidValue {
                field: format!("slots[{id}].draw_order.min"),
                value: range.min,
                expected: "finite",
            });
        }
        if !(range.max.is_finite() && range.max >= range.min) {
            errors.push(ValidationError::InvalidValue {
                field: format!("slots[{id}].draw_order.max"),
                value: range.max,
                expected: "finite and >= draw_order.min",
            });
        }
    }
}

/// モジュールを単体で検査する。検出したエラーを全て返す。
///
/// モジュールの要素を 1 つのモデルとみなして [`validate`] と [`validate_physics`] を行う
/// (参照するパラメータは存在するものとみなす)。本体との整合 (スロット・パラメータ・
/// マスクの参照先) は、合成 (`ugoe_runtime::compose`) が検査する。
pub fn validate_module(module: &Module) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !Module::is_valid_name(&module.name) {
        errors.push(ValidationError::InvalidModuleName {
            name: module.name.clone(),
        });
    }
    // 種類が未知のパラメータは、通常の格子とブレンドシェイプ格子のどちらの軸にも置ける。
    let parameters = module
        .referenced_parameters()
        .into_iter()
        .map(|id| Parameter {
            name: id.to_string(),
            id,
            min: 0.0,
            max: 0.0,
            default: 0.0,
            kind: ParameterKind::Unknown,
            mirror: None,
            group: None,
            internal: false,
        })
        .collect();
    let model = module.stand_in_model(parameters);
    errors.extend(validate(&model));
    if !module.physics.is_empty() {
        // 本体の衝突体 (スロットの `colliders`) への参照は合成時に検査する。同じ ID があれば
        // モジュールの衝突体を指すので、モジュールにない ID だけを除く。
        let own: HashSet<&str> = module
            .physics
            .colliders
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        let rigs = module
            .physics
            .rigs
            .iter()
            .map(|rig| {
                let mut rig = rig.clone();
                rig.colliders.retain(|c| own.contains(c.as_str()));
                rig
            })
            .collect();
        let physics = PhysicsSettings {
            gravity: Vec2::ZERO,
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs,
            colliders: module.physics.colliders.clone(),
        };
        errors.extend(validate_physics(&physics, &model));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MaskRef;
    use crate::fixtures::{
        dress_module, dress_up_base, jacket_module, long_hair_module, posed_dress_up_base,
        sleeves_module,
    };

    #[test]
    fn fixtures_are_valid() {
        assert_eq!(validate(&dress_up_base()), vec![]);
        assert_eq!(validate_module(&long_hair_module()), vec![]);
        assert_eq!(validate_module(&dress_module()), vec![]);
        assert_eq!(validate_module(&jacket_module()), vec![]);
        assert_eq!(validate(&posed_dress_up_base()), vec![]);
        assert_eq!(validate_module(&sleeves_module()), vec![]);
    }

    #[test]
    fn slot_references_and_draw_order_are_checked() {
        let mut model = dress_up_base();
        let hair = &mut model.slots[0];
        hair.anchor = Some("Nope".into());
        hair.replaces.push("NoPart".into());
        hair.parameters.push("NoParam".into());
        hair.mask_sources.push("NoMesh".into());
        hair.draw_order.max = 100.0;
        model.slots[1].draw_order.min = f32::NAN;
        model.slots.push(model.slots[1].clone());
        let unknown = |field, target_kind, target: &str| ValidationError::UnknownReference {
            kind: "slot",
            id: "Hair".into(),
            field,
            target_kind,
            target: target.into(),
        };
        let errors = validate(&model);
        let min_is_nan = |id: &str| {
            errors.iter().any(|e| {
                matches!(e, ValidationError::InvalidValue { field, .. }
                    if *field == format!("slots[{id}].draw_order.min"))
            })
        };
        assert!(min_is_nan("Outfit"));
        let expected = [
            ValidationError::DuplicateId {
                kind: "slot",
                id: "Outfit".into(),
            },
            unknown("anchor", "deformer", "Nope"),
            unknown("replaces", "part", "NoPart"),
            unknown("parameters", "parameter", "NoParam"),
            unknown("mask_sources", "mesh", "NoMesh"),
            ValidationError::InvalidValue {
                field: "slots[Hair].draw_order.max".into(),
                value: 100.0,
                expected: "finite and >= draw_order.min",
            },
        ];
        for error in &expected {
            assert!(errors.contains(error), "{error:?} not in {errors:?}");
        }
    }

    #[test]
    fn slot_part_pose_parts_and_colliders_are_checked() {
        use crate::physics::{ColliderShape, PhysicsCollider};
        use crate::pose::{PoseEntry, PoseGroup};
        let mut model = dress_up_base();
        model.pose.groups.push(PoseGroup {
            entries: vec![PoseEntry {
                part: "Hair".into(),
                links: vec!["Body".into()],
            }],
        });
        let slot = &mut model.slots[1];
        slot.part = Some("NoPart".into());
        // リンクのパーツや、ポーズにないパーツは `pose_parts` にできない。
        slot.pose_parts = vec!["Hair".into(), "Body".into(), "Root".into()];
        slot.colliders = vec!["Chest".into(), "NoCollider".into()];
        let unknown = |field, target_kind, target: &str| ValidationError::UnknownReference {
            kind: "slot",
            id: "Outfit".into(),
            field,
            target_kind,
            target: target.into(),
        };
        assert_eq!(
            validate(&model),
            vec![
                unknown("part", "part", "NoPart"),
                unknown("pose_parts", "pose entry", "Body"),
                unknown("pose_parts", "pose entry", "Root"),
            ]
        );
        let physics = PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: Vec::new(),
            colliders: vec![PhysicsCollider {
                id: "Chest".into(),
                name: "Chest".into(),
                attach: Some("Torso".into()),
                shape: ColliderShape::Circle {
                    center: Vec2::new(0.5, 0.5),
                    radius: 8.0,
                },
            }],
        };
        assert_eq!(
            validate_physics(&physics, &model),
            vec![unknown("colliders", "physics collider", "NoCollider")]
        );
    }

    #[test]
    fn module_internal_references_are_checked() {
        let mut module = long_hair_module();
        module.name = "Long/Hair".into();
        module.meshes[0].parent = Some("Missing".into());
        // 本体のメッシュへのマスクは合成時に検査するので、ここではエラーにしない。
        module.meshes[1].masks.push(MaskRef::normal("SomeBaseMesh"));
        // 本体の衝突体 (スロットの `colliders`) への参照も合成時に検査する。
        module.physics.rigs[0].colliders.push("BaseCollider".into());
        module.pose.groups.push(crate::pose::PoseGroup {
            entries: vec![crate::pose::PoseEntry {
                part: "NoPart".into(),
                links: vec![],
            }],
        });
        let errors = validate_module(&module);
        assert_eq!(
            errors,
            vec![
                ValidationError::InvalidModuleName {
                    name: "Long/Hair".into()
                },
                ValidationError::UnknownReference {
                    kind: "mesh",
                    id: "Back".into(),
                    field: "parent",
                    target_kind: "deformer",
                    target: "Missing".into(),
                },
                ValidationError::UnknownReference {
                    kind: "pose group",
                    id: "0".into(),
                    field: "part",
                    target_kind: "part",
                    target: "NoPart".into(),
                },
            ]
        );
        assert!(!Module::is_valid_name(""));
        assert!(!Module::is_valid_name("#x"));
        assert!(Module::is_valid_name("夏服"));
    }

    #[test]
    fn referenced_parameters_include_physics() {
        let ids: Vec<String> = long_hair_module()
            .referenced_parameters()
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        assert_eq!(ids, ["AngleX", "HairFront"]);
    }

    #[test]
    fn slots_and_modules_round_trip_through_json() {
        let model = dress_up_base();
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(
            json["slots"][1],
            serde_json::json!({
                "id": "Outfit",
                "anchor": "Torso",
                "draw_order": { "min": 300.0, "max": 400.0 },
                "parameters": ["BodySway"],
                "part": "Body",
            })
        );
        assert_eq!(serde_json::from_value::<Model>(json).unwrap(), model);
        // スロットがなければ書き出さない。
        let json = serde_json::to_value(crate::fixtures::single_quad()).unwrap();
        assert!(json.get("slots").is_none());

        for module in [long_hair_module(), dress_module()] {
            let json = serde_json::to_value(&module).unwrap();
            // 空のポーズは書き出さない。
            assert!(json.get("pose").is_none());
            assert_eq!(serde_json::from_value::<Module>(json).unwrap(), module);
        }
        // 要素・条件・物理は省略できる。
        let module: Module = serde_json::from_value(serde_json::json!({
            "schema_version": 0,
            "name": "Empty",
            "slot": "Hair",
        }))
        .unwrap();
        assert!(module.meshes.is_empty() && module.physics.is_empty() && module.pose.is_empty());
        assert_eq!(validate_module(&module), vec![]);
    }

    #[test]
    fn unknown_values_in_modules_are_found() {
        let mut module = long_hair_module();
        assert!(crate::unknown::in_module(&module).is_empty());
        module.meshes[0].blend = crate::BlendMode::Unknown;
        module.physics.colliders[0].shape = crate::physics::ColliderShape::Unknown;
        assert_eq!(
            crate::unknown::in_module(&module),
            ["meshes[Back].blend", "physics.colliders[Collar].shape"]
        );
    }
}
