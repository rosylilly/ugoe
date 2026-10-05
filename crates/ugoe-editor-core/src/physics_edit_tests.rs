//! 物理の編集 (`physics_edit.rs`) のテスト。入力は `fixtures::dynamic_hair` と
//! `fixtures::dynamic_hair_settings` (髪の房の回転デフォーマの鎖・衝突体・頂点への出力を持つ)。

use ugoe_core::Vec2;
use ugoe_model::fixtures;
use ugoe_model::physics::{
    ColliderShape, PhysicsBlend, PhysicsCollider, PhysicsInput, PhysicsSettings, PhysicsSource,
    PhysicsTarget, VertexOwner,
};
use ugoe_model::validate::ValidationError;

use crate::{Command, Document, EditError};

fn doc() -> Document {
    Document::new(fixtures::dynamic_hair()).with_physics(Some(fixtures::dynamic_hair_settings()))
}

fn physics(doc: &Document) -> &PhysicsSettings {
    doc.physics().unwrap()
}

fn close(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-3
}

fn circle(id: &str, attach: Option<&str>, center: Vec2) -> PhysicsCollider {
    PhysicsCollider {
        id: id.into(),
        name: id.into(),
        attach: attach.map(Into::into),
        shape: ColliderShape::Circle {
            center,
            radius: 5.0,
        },
    }
}

#[test]
fn commands_use_snake_case_ops() {
    let json = serde_json::to_value(Command::CreatePhysicsRig {
        id: None,
        name: String::new(),
        deformers: vec!["Hair1".into()],
    })
    .unwrap();
    assert_eq!(json["op"], "create_physics_rig");
    let back: Command = serde_json::from_str(r#"{"op":"set_physics_frame","rig":"Hair"}"#).unwrap();
    assert_eq!(
        back,
        Command::SetPhysicsFrame {
            rig: "Hair".into(),
            frame: None
        }
    );
}

#[test]
fn set_world_creates_settings_and_undo_removes_them() {
    let mut doc = Document::new(fixtures::dynamic_hair());
    assert!(doc.physics().is_none());
    doc.apply(Command::SetPhysicsWorld {
        gravity: Vec2::new(0.0, 2.0),
        wind: Vec2::new(0.5, 0.0),
        fps: 30.0,
    })
    .unwrap();
    let p = physics(&doc);
    assert_eq!(
        (p.gravity, p.wind, p.fps),
        (Vec2::new(0.0, 2.0), Vec2::new(0.5, 0.0), 30.0)
    );
    assert!(p.rigs.is_empty());
    assert!(doc.is_modified());
    assert!(doc.undo());
    assert!(doc.physics().is_none());
    assert!(!doc.is_modified());
    assert!(doc.redo());
    assert_eq!(physics(&doc).fps, 30.0);
}

#[test]
fn set_world_rejects_bad_values() {
    let mut doc = doc();
    for (gravity, fps) in [
        (Vec2::new(f32::NAN, 1.0), 60.0),
        (Vec2::Y, 0.0),
        (Vec2::Y, f32::INFINITY),
    ] {
        let err = doc
            .apply(Command::SetPhysicsWorld {
                gravity,
                wind: Vec2::ZERO,
                fps,
            })
            .unwrap_err();
        assert!(matches!(err, EditError::InvalidPhysics { .. }), "{err}");
    }
    assert_eq!(physics(&doc), &fixtures::dynamic_hair_settings());
}

#[test]
fn create_rig_from_deformer_chain() {
    let mut doc = Document::new(fixtures::dynamic_hair());
    doc.apply(Command::CreatePhysicsRig {
        id: None,
        name: String::new(),
        deformers: vec!["Hair1".into(), "Hair2".into(), "Hair3".into()],
    })
    .unwrap();
    let p = physics(&doc);
    assert_eq!((p.gravity, p.fps), (Vec2::Y, 60.0));
    let rig = &p.rigs[0];
    // 名前は最初のデフォーマの名前、ID は名前から。
    assert_eq!(rig.name, "Hair1");
    assert_eq!(rig.id.as_str(), "Hair1");
    // 原点は (148,100)・(148,130)・(148,160)。先端は Hair3 から真下に、Strand (Hair3 の局所 (-3,0)〜(3,30))
    // の最も深い点まで。
    let positions: Vec<Vec2> = rig.particles.iter().map(|p| p.position).collect();
    assert_eq!(positions.len(), 4);
    assert!(close(positions[0], Vec2::ZERO));
    assert!(close(positions[1], Vec2::new(0.0, 30.0)));
    assert!(close(positions[2], Vec2::new(0.0, 60.0)));
    assert!(
        close(positions[3], Vec2::new(0.0, 90.0)),
        "{:?}",
        positions[3]
    );
    assert_eq!(rig.particles[1].radius, 30.0);
    assert!((rig.particles[3].radius - 30.0).abs() < 1e-3);
    let targets: Vec<(PhysicsTarget, u32)> = rig
        .outputs
        .iter()
        .map(|o| (o.target.clone(), o.particle))
        .collect();
    let expected: Vec<(PhysicsTarget, u32)> = [1u32, 2, 3]
        .iter()
        .map(|&k| {
            let deformer = format!("Hair{k}");
            (
                PhysicsTarget::DeformerAngle {
                    deformer: deformer.as_str().into(),
                },
                k,
            )
        })
        .collect();
    assert_eq!(targets, expected);

    // 同じ名前で作ると ID に番号が付く。
    doc.apply(Command::CreatePhysicsRig {
        id: None,
        name: String::new(),
        deformers: vec!["Hair1".into()],
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[1].id.as_str(), "Hair1_2");
    // 1 つのデフォーマでも、先端は子孫の点。
    let single = &physics(&doc).rigs[1];
    assert_eq!(single.particles.len(), 2);
    assert!((single.particles[1].position.y - 90.0).abs() < 1e-3);
}

#[test]
fn create_rig_rejects_bad_chains() {
    let mut doc = doc();
    let create = |deformers: Vec<&str>| Command::CreatePhysicsRig {
        id: Some("New".into()),
        name: "New".into(),
        deformers: deformers.into_iter().map(Into::into).collect(),
    };
    assert!(matches!(
        doc.apply(create(vec![])),
        Err(EditError::InvalidPhysics { .. })
    ));
    assert!(matches!(
        doc.apply(create(vec!["Cloth"])),
        Err(EditError::DeformerKindMismatch { .. })
    ));
    assert!(matches!(
        doc.apply(create(vec!["Missing"])),
        Err(EditError::NotFound { .. })
    ));
    assert!(matches!(
        doc.apply(create(vec!["Hair1", "Hair1"])),
        Err(EditError::InvalidPhysics { .. })
    ));
    // 既存の ID。
    let mut dup = create(vec!["Hair1"]);
    if let Command::CreatePhysicsRig { id, .. } = &mut dup {
        *id = Some("Hair".into());
    }
    assert!(matches!(doc.apply(dup), Err(EditError::DuplicateId { .. })));
    assert!(!doc.is_modified());
}

#[test]
fn add_rename_and_remove_rigs() {
    let mut doc = doc();
    let mut rig = physics(&doc).rigs[0].clone();
    rig.id = "Hair_copy".into();
    doc.apply(Command::AddPhysicsRig { rig: rig.clone() })
        .unwrap();
    assert_eq!(physics(&doc).rigs.len(), 3);
    assert!(matches!(
        doc.apply(Command::AddPhysicsRig { rig }),
        Err(EditError::DuplicateId { .. })
    ));
    doc.apply(Command::RenamePhysicsRig {
        id: "Hair_copy".into(),
        name: "Copy".into(),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[2].name, "Copy");
    doc.apply(Command::RemovePhysicsRig {
        id: "Hair_copy".into(),
    })
    .unwrap();
    assert_eq!(physics(&doc), &fixtures::dynamic_hair_settings());
    assert!(matches!(
        doc.apply(Command::RemovePhysicsRig { id: "Nope".into() }),
        Err(EditError::NotFound { .. })
    ));
    // 物理のないドキュメントでは、リグの編集は見つからない。
    let mut empty = Document::new(fixtures::dynamic_hair());
    assert!(matches!(
        empty.apply(Command::RenamePhysicsRig {
            id: "Hair".into(),
            name: "x".into()
        }),
        Err(EditError::NotFound { .. })
    ));
}

#[test]
fn set_inputs_validates_references() {
    let mut doc = doc();
    let input = |parameter: &str| PhysicsInput {
        parameter: parameter.into(),
        source: PhysicsSource::Angle,
        weight: 50.0,
        reflect: false,
    };
    doc.apply(Command::SetPhysicsInputs {
        rig: "Hair".into(),
        inputs: vec![input("AngleZ")],
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].inputs, vec![input("AngleZ")]);
    let err = doc
        .apply(Command::SetPhysicsInputs {
            rig: "Hair".into(),
            inputs: vec![input("Missing")],
        })
        .unwrap_err();
    assert!(
        matches!(&err, EditError::Invalid(e) if matches!(e[..], [ValidationError::UnknownReference { .. }])),
        "{err}"
    );
    let mut nan = input("AngleZ");
    nan.weight = f32::NAN;
    assert!(matches!(
        doc.apply(Command::SetPhysicsInputs {
            rig: "Hair".into(),
            inputs: vec![nan],
        }),
        Err(EditError::InvalidPhysics { .. })
    ));
    assert_eq!(physics(&doc).rigs[0].inputs, vec![input("AngleZ")]);
}

#[test]
fn set_outputs_validates_targets() {
    let mut doc = doc();
    let mut outputs = physics(&doc).rigs[0].outputs.clone();
    outputs[0].target = PhysicsTarget::DeformerAngle {
        deformer: "Cloth".into(),
    };
    let err = doc
        .apply(Command::SetPhysicsOutputs {
            rig: "Hair".into(),
            outputs: outputs.clone(),
        })
        .unwrap_err();
    assert!(matches!(err, EditError::Invalid(_)), "{err}");
    outputs[0].target = PhysicsTarget::Vertices {
        owner: VertexOwner::Mesh("Strand".into()),
        weights: vec![1.0; 3],
    };
    let err = doc
        .apply(Command::SetPhysicsOutputs {
            rig: "Hair".into(),
            outputs: outputs.clone(),
        })
        .unwrap_err();
    assert!(
        matches!(&err, EditError::Invalid(e) if matches!(e[..], [ValidationError::WeightCount { .. }])),
        "{err}"
    );
    outputs[0].target = PhysicsTarget::Vertices {
        owner: VertexOwner::Mesh("Strand".into()),
        weights: vec![1.0; 4],
    };
    doc.apply(Command::SetPhysicsOutputs {
        rig: "Hair".into(),
        outputs: outputs.clone(),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].outputs, outputs);
    outputs[0].particle = 0;
    assert!(matches!(
        doc.apply(Command::SetPhysicsOutputs {
            rig: "Hair".into(),
            outputs,
        }),
        Err(EditError::InvalidPhysics { .. })
    ));
}

#[test]
fn set_particles_checks_coefficients() {
    let mut doc = doc();
    let mut particles = physics(&doc).rigs[0].particles.clone();
    particles[1].mobility = 0.5;
    particles[2].collision_radius = 4.0;
    doc.apply(Command::SetPhysicsParticles {
        rig: "Hair".into(),
        particles: particles.clone(),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].particles, particles);

    let bad = |edit: fn(&mut ugoe_model::physics::PhysicsParticle)| {
        let mut p = particles.clone();
        edit(&mut p[1]);
        Command::SetPhysicsParticles {
            rig: "Hair".into(),
            particles: p,
        }
    };
    for command in [
        bad(|p| p.mobility = 1.5),
        bad(|p| p.delay = -1.0),
        bad(|p| p.radius = -1.0),
        bad(|p| p.collision_radius = -0.5),
        bad(|p| p.acceleration = f32::NAN),
        bad(|p| p.acceleration = -1.0),
        bad(|p| p.position.x = f32::INFINITY),
        Command::SetPhysicsParticles {
            rig: "Hair".into(),
            particles: vec![],
        },
    ] {
        let err = doc.apply(command).unwrap_err();
        assert!(matches!(err, EditError::InvalidPhysics { .. }), "{err}");
    }
    assert_eq!(physics(&doc).rigs[0].particles, particles);
}

#[test]
fn set_normalization_requires_ordered_range() {
    let mut doc = doc();
    let mut n = physics(&doc).rigs[0].normalization;
    n.angle.max = 20.0;
    doc.apply(Command::SetPhysicsNormalization {
        rig: "Hair".into(),
        normalization: n,
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].normalization.angle.max, 20.0);
    n.position.min = 5.0;
    assert!(matches!(
        doc.apply(Command::SetPhysicsNormalization {
            rig: "Hair".into(),
            normalization: n,
        }),
        Err(EditError::InvalidPhysics { .. })
    ));
}

#[test]
fn frame_must_be_a_deformer() {
    let mut doc = doc();
    doc.apply(Command::SetPhysicsFrame {
        rig: "Hair".into(),
        frame: Some("Head".into()),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].frame, Some("Head".into()));
    assert!(matches!(
        doc.apply(Command::SetPhysicsFrame {
            rig: "Hair".into(),
            frame: Some("Missing".into()),
        }),
        Err(EditError::Invalid(_))
    ));
    doc.apply(Command::SetPhysicsFrame {
        rig: "Hair".into(),
        frame: None,
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].frame, None);
}

#[test]
fn collider_add_edit_remove() {
    let mut doc = doc();
    doc.apply(Command::AddPhysicsCollider {
        collider: circle("HeadBall", Some("Head"), Vec2::new(0.0, -10.0)),
    })
    .unwrap();
    assert!(matches!(
        doc.apply(Command::AddPhysicsCollider {
            collider: circle("HeadBall", None, Vec2::ZERO),
        }),
        Err(EditError::DuplicateId { .. })
    ));
    doc.apply(Command::SetPhysicsRigColliders {
        rig: "Hair".into(),
        colliders: vec!["Shoulder".into(), "HeadBall".into()],
    })
    .unwrap();
    // 存在しない衝突体・重複は拒否する。
    assert!(matches!(
        doc.apply(Command::SetPhysicsRigColliders {
            rig: "Hair".into(),
            colliders: vec!["Nope".into()],
        }),
        Err(EditError::Invalid(_))
    ));
    assert!(matches!(
        doc.apply(Command::SetPhysicsRigColliders {
            rig: "Hair".into(),
            colliders: vec!["Shoulder".into(), "Shoulder".into()],
        }),
        Err(EditError::InvalidPhysics { .. })
    ));
    // 形状と取り付け先の変更 (ドラッグでの移動)。
    let moved = circle("HeadBall", None, Vec2::new(10.0, 20.0));
    doc.apply(Command::SetPhysicsCollider {
        collider: moved.clone(),
    })
    .unwrap();
    assert_eq!(physics(&doc).colliders[1], moved);
    let mut negative = moved.clone();
    negative.shape = ColliderShape::Circle {
        center: Vec2::ZERO,
        radius: -1.0,
    };
    assert!(matches!(
        doc.apply(Command::SetPhysicsCollider { collider: negative }),
        Err(EditError::Invalid(_))
    ));
    let mut attach_missing = moved;
    attach_missing.attach = Some("Missing".into());
    assert!(matches!(
        doc.apply(Command::SetPhysicsCollider {
            collider: attach_missing
        }),
        Err(EditError::Invalid(_))
    ));
    // 削除はリグの参照からも外す。
    doc.apply(Command::RemovePhysicsCollider {
        id: "HeadBall".into(),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[0].colliders, vec!["Shoulder"]);
    assert_eq!(physics(&doc).colliders.len(), 1);
    // Undo で戻る。
    assert!(doc.undo());
    assert_eq!(physics(&doc).colliders.len(), 2);
    assert_eq!(physics(&doc).rigs[0].colliders.len(), 2);
}

/// 未知の対象 (`VertexOwner::Unknown`) への出力は、モデルの要素を消しても外さない
/// (何を指すか分からないので、未知の出力先と同じく残す)。
#[test]
fn removing_model_elements_keeps_unknown_vertex_owners() {
    let mut settings = fixtures::dynamic_hair_settings();
    let unknown = PhysicsTarget::Vertices {
        owner: VertexOwner::Unknown,
        weights: vec![1.0],
    };
    settings.rigs[1].outputs[0].target = unknown.clone();
    let mut doc = Document::new(fixtures::dynamic_hair()).with_physics(Some(settings));
    doc.apply(Command::RemoveMesh {
        id: "Ribbon".into(),
    })
    .unwrap();
    doc.apply(Command::UnwrapDeformer { id: "Hair2".into() })
        .unwrap();
    let targets: Vec<_> = physics(&doc).rigs[1]
        .outputs
        .iter()
        .map(|o| o.target.clone())
        .collect();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0], unknown);
    assert!(matches!(
        &targets[1],
        PhysicsTarget::Vertices { owner: VertexOwner::Warp(w), .. } if w == "Cloth"
    ));
}

#[test]
fn removing_model_elements_forgets_physics_references() {
    let mut doc = doc();
    // AngleX は Skirt の入力。消すと入力が外れる。
    doc.apply(Command::RemoveParameter {
        id: "AngleX".into(),
    })
    .unwrap();
    assert!(physics(&doc).rigs[1].inputs.is_empty());
    // Ribbon を消すと、その頂点への出力が外れる (Cloth への 2 つは残る)。
    doc.apply(Command::RemoveMesh {
        id: "Ribbon".into(),
    })
    .unwrap();
    assert_eq!(physics(&doc).rigs[1].outputs.len(), 2);
    // Hair2 を外すと、その角度への出力が外れる。
    doc.apply(Command::UnwrapDeformer { id: "Hair2".into() })
        .unwrap();
    let hair: Vec<_> = physics(&doc).rigs[0]
        .outputs
        .iter()
        .map(|o| o.target.clone())
        .collect();
    assert_eq!(
        hair,
        ["Hair1", "Hair3"].map(|d| PhysicsTarget::DeformerAngle { deformer: d.into() })
    );
    assert!(doc.undo());
    assert_eq!(physics(&doc).rigs[0].outputs.len(), 3);
}

#[test]
fn removing_the_attached_deformer_keeps_the_collider_in_place() {
    let mut doc = doc();
    doc.apply(Command::AddPhysicsCollider {
        collider: circle("Tip", Some("Hair3"), Vec2::new(1.0, 2.0)),
    })
    .unwrap();
    doc.apply(Command::SetPhysicsFrame {
        rig: "Hair".into(),
        frame: Some("Hair3".into()),
    })
    .unwrap();
    doc.apply(Command::Batch {
        label: "remove the tip".into(),
        commands: vec![
            Command::RemoveMesh {
                id: "Strand".into(),
            },
            Command::RemoveDeformer { id: "Hair3".into() },
        ],
    })
    .unwrap();
    let p = physics(&doc);
    let tip = p.colliders.iter().find(|c| c.id == "Tip").unwrap();
    assert_eq!(tip.attach, None);
    // Hair3 の原点はキャンバスの (148,160)、角度 0。
    let ColliderShape::Circle { center, .. } = tip.shape else {
        panic!("circle expected")
    };
    assert!(close(center, Vec2::new(149.0, 162.0)), "{center:?}");
    assert_eq!(p.rigs[0].frame, None);
    assert_eq!(p.rigs[0].outputs.len(), 2);
}

#[test]
fn vertex_edits_keep_weights_aligned() {
    let mut doc = doc();
    let ribbon_weights = |doc: &Document| match &physics(doc).rigs[1].outputs[2].target {
        PhysicsTarget::Vertices { weights, .. } => weights.clone(),
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(ribbon_weights(&doc), vec![0.0, 0.0, 1.0, 1.0]);
    doc.apply(Command::RemoveMeshVertices {
        id: "Ribbon".into(),
        indices: vec![0],
    })
    .unwrap();
    assert_eq!(ribbon_weights(&doc), vec![0.0, 1.0, 1.0]);
    doc.apply(Command::AddMeshVertex {
        id: "Ribbon".into(),
        position: Vec2::new(83.0, 205.0),
    })
    .unwrap();
    assert_eq!(ribbon_weights(&doc), vec![0.0, 1.0, 1.0, 0.0]);
}

#[test]
fn model_edits_that_break_physics_are_rejected() {
    let mut doc = doc();
    // Strand の頂点数を変える自動メッシュはないので、ワープの制御点の数を変える代わりに
    // 頂点への出力を持つメッシュを別のメッシュに置き換える: 削除して同じ ID で頂点数の違うメッシュを足す。
    let mut mesh = doc
        .model()
        .meshes
        .iter()
        .find(|m| m.id == "Ribbon")
        .unwrap()
        .clone();
    mesh.uvs.pop();
    for kf in mesh.keyforms.all_keyforms_mut() {
        kf.positions.pop();
    }
    mesh.indices = vec![0, 1, 2];
    // 同じ Batch の中で消して足すと、消した要素ではない (後で同じ ID がある) ので参照は残り、
    // 重みの数が合わなくなる。
    let err = doc
        .apply(Command::Batch {
            label: "replace".into(),
            commands: vec![
                Command::RemoveMesh {
                    id: "Ribbon".into(),
                },
                Command::AddMesh { mesh },
            ],
        })
        .unwrap_err();
    assert!(
        matches!(&err, EditError::Invalid(e) if e.iter().any(|e| matches!(e, ValidationError::WeightCount { .. }))),
        "{err}"
    );
    assert_eq!(physics(&doc), &fixtures::dynamic_hair_settings());
}

// ---- リグの順序と中継パラメータ (T0312) ----

fn rig_ids(doc: &Document) -> Vec<&str> {
    physics(doc).rigs.iter().map(|r| r.id.as_str()).collect()
}

fn reorder(id: &str, before: Option<&str>) -> Command {
    Command::ReorderPhysicsRig {
        id: id.into(),
        before: before.map(Into::into),
    }
}

#[test]
fn reorder_rig_moves_before_and_undo_restores() {
    let mut doc = doc();
    assert_eq!(rig_ids(&doc), ["Hair", "Skirt"]);
    doc.apply(reorder("Skirt", Some("Hair"))).unwrap();
    assert_eq!(rig_ids(&doc), ["Skirt", "Hair"]);
    doc.apply(reorder("Skirt", None)).unwrap();
    assert_eq!(rig_ids(&doc), ["Hair", "Skirt"]);
    assert!(doc.undo());
    assert_eq!(rig_ids(&doc), ["Skirt", "Hair"]);
    assert!(doc.undo());
    assert_eq!(physics(&doc), &fixtures::dynamic_hair_settings());
    assert!(doc.redo());
    assert_eq!(rig_ids(&doc), ["Skirt", "Hair"]);
}

#[test]
fn reorder_rig_rejects_unknown_and_self() {
    let mut doc = doc();
    assert!(matches!(
        doc.apply(reorder("Nope", None)),
        Err(EditError::NotFound { .. })
    ));
    assert!(matches!(
        doc.apply(reorder("Hair", Some("Nope"))),
        Err(EditError::NotFound { .. })
    ));
    assert!(matches!(
        doc.apply(reorder("Hair", Some("Hair"))),
        Err(EditError::SelfReference { .. })
    ));
    assert!(matches!(
        Document::new(fixtures::dynamic_hair()).apply(reorder("Hair", None)),
        Err(EditError::NotFound { .. })
    ));
    assert_eq!(physics(&doc), &fixtures::dynamic_hair_settings());
}

#[test]
fn set_parameter_internal_and_undo() {
    let mut doc = Document::new(fixtures::physics_relay())
        .with_physics(Some(fixtures::physics_relay_settings()));
    let internal = |doc: &Document, id: &str| {
        doc.model()
            .parameters
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .internal
    };
    assert!(internal(&doc, fixtures::RELAY_FOLLOW));
    doc.apply(Command::SetParameterInternal {
        id: fixtures::RELAY_FOLLOW.into(),
        internal: false,
    })
    .unwrap();
    assert!(!internal(&doc, fixtures::RELAY_FOLLOW));
    assert!(doc.undo());
    assert!(internal(&doc, fixtures::RELAY_FOLLOW));
    assert!(matches!(
        doc.apply(Command::SetParameterInternal {
            id: "Nope".into(),
            internal: true,
        }),
        Err(EditError::NotFound { .. })
    ));
}

/// 出力ごとの合成の方法を切り替えて、元に戻す (`docs/spec/physics.md#合成の方法`)。
#[test]
fn set_output_blend_and_undo() {
    let mut doc = Document::new(fixtures::physics_additive())
        .with_physics(Some(fixtures::physics_additive_settings()));
    let blend = |doc: &Document| physics(doc).rigs[0].outputs[0].blend;
    assert_eq!(blend(&doc), PhysicsBlend::Add);
    let mut outputs = physics(&doc).rigs[0].outputs.clone();
    outputs[0].blend = PhysicsBlend::Overwrite;
    doc.apply(Command::SetPhysicsOutputs {
        rig: "BodySway".into(),
        outputs: outputs.clone(),
    })
    .unwrap();
    assert_eq!(blend(&doc), PhysicsBlend::Overwrite);
    assert!(doc.undo());
    assert_eq!(blend(&doc), PhysicsBlend::Add);
    assert!(doc.redo());
    assert_eq!(blend(&doc), PhysicsBlend::Overwrite);

    // 未知の値は保存できないので受け付けない。
    outputs[0].blend = PhysicsBlend::Unknown;
    assert!(matches!(
        doc.apply(Command::SetPhysicsOutputs {
            rig: "BodySway".into(),
            outputs,
        }),
        Err(EditError::InvalidPhysics { .. })
    ));
    assert_eq!(blend(&doc), PhysicsBlend::Overwrite);
}
