//! 本体の衝突体の参照と、本体の物理が無いときの開き方のテスト (T0660・T0661)。

use super::*;

/// 本体に物理が無いのに、モジュールのリグがスロットの公開する本体の衝突体を参照していれば、本体に物理を足すよう
/// 案内するエラーにする。本体の物理に衝突体があれば開ける (T0660)。
#[test]
fn exposed_colliders_need_the_base_physics() {
    let mut base = base();
    let hair = base
        .model
        .slots
        .iter_mut()
        .find(|s| s.id == "Hair")
        .unwrap();
    hair.colliders = vec!["Shoulders".into()];
    let mut module = long_hair_module();
    module.physics.rigs[0].colliders = vec!["Shoulders".into()];
    let rig = module.physics.rigs[0].id.to_string();
    let context = |base: &ModuleBase| {
        ModuleContext::new(
            base.clone(),
            &module.slot,
            module.name.clone(),
            module.requires.clone(),
        )
        .unwrap()
    };

    let without = context(&base);
    assert_eq!(without.unavailable_colliders(), ["Shoulders"]);
    // 確かめてから外すと開ける (外したものを返す。モジュール自身の衝突体への参照は残す。T0661)。
    let mut dropped = module.clone();
    dropped.physics.rigs[0].colliders = vec!["Shoulders".into(), "Own".into()];
    dropped
        .physics
        .colliders
        .push(ugoe_model::physics::PhysicsCollider {
            id: "Own".into(),
            name: "Own".into(),
            attach: None,
            shape: ugoe_model::physics::ColliderShape::Circle {
                center: Vec2::new(0.0, 0.0),
                radius: 4.0,
            },
        });
    assert_eq!(
        without.drop_unavailable_colliders(&mut dropped),
        [ColliderReference {
            rig: rig.as_str().into(),
            collider: "Shoulders".into(),
        }]
    );
    assert_eq!(dropped.physics.rigs[0].colliders, ["Own"]);
    without.workspace(&dropped, &images(&dropped)).unwrap();
    assert!(without.drop_unavailable_colliders(&mut dropped).is_empty());
    let error = without.workspace(&module, &images(&module)).unwrap_err();
    assert_eq!(
        error,
        ModuleError::ColliderNeedsBasePhysics {
            rig,
            collider: "Shoulders".into(),
            slot: "Hair".into(),
        }
    );
    assert!(
        error.to_string().contains("the base model has no physics"),
        "{error}"
    );
    // スロットが公開していない衝突体は、今までどおり合成のエラー。
    module.physics.rigs[0].colliders = vec!["Elsewhere".into()];
    assert!(matches!(
        without.workspace(&module, &images(&module)),
        Err(ModuleError::Compose(
            ugoe_runtime::ComposeError::UnknownCollider { .. }
        ))
    ));

    module.physics.rigs[0].colliders = vec!["Shoulders".into()];
    let mut physics = crate::physics_edit::default_settings();
    physics
        .colliders
        .push(ugoe_model::physics::PhysicsCollider {
            id: "Shoulders".into(),
            name: "Shoulders".into(),
            attach: None,
            shape: ugoe_model::physics::ColliderShape::Circle {
                center: Vec2::new(0.0, 0.0),
                radius: 10.0,
            },
        });
    base.physics = Some(physics);
    let with = context(&base);
    assert!(with.unavailable_colliders().is_empty());
    let workspace = with.workspace(&module, &images(&module)).unwrap();
    let physics = workspace.physics.unwrap();
    assert!(physics.rigs.iter().any(|r| r.colliders == ["Shoulders"]));
}

/// 物理の無い本体で開けないモジュールのプロジェクトを、衝突体の参照を外して開く (T0661)。外さずには開けず、
/// 外して開いたものを保存すると `module.json` から参照が消える。
#[test]
fn module_project_opens_without_unavailable_colliders() {
    let temp = tempfile::tempdir().unwrap();
    let mut base = base();
    let hair = base
        .model
        .slots
        .iter_mut()
        .find(|s| s.id == "Hair")
        .unwrap();
    hair.colliders = vec!["Shoulders".into()];
    let base_path = temp.path().join("avatar.ugoeproj");
    Project::new(ProjectMeta::new("avatar"), base.model, base.textures)
        .save(&base_path)
        .unwrap();
    let dir = temp.path().join("hair.ugoeproj");
    Project::new_module("LongHair", &base_path, &"Hair".into())
        .unwrap()
        .save(&dir)
        .unwrap();
    let mut module = long_hair_module();
    module.physics.rigs[0].colliders = vec!["Shoulders".into()];
    let rig = module.physics.rigs[0].id.to_string();
    std::fs::write(
        dir.join("module.json"),
        ugoe_format::module_to_json(&module).unwrap(),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("textures")).unwrap();
    std::fs::write(
        dir.join("textures/long_hair.png"),
        solid_png(1, 1, [1, 2, 3, 255]),
    )
    .unwrap();

    assert!(matches!(
        Project::load(&dir),
        Err(ProjectError::Module(
            ModuleError::ColliderNeedsBasePhysics { .. }
        ))
    ));
    let (project, dropped) =
        Project::load_module_without_unavailable_colliders(&dir, None, None).unwrap();
    assert_eq!(
        dropped,
        [ColliderReference {
            rig: rig.as_str().into(),
            collider: "Shoulders".into(),
        }]
    );
    project.save(&dir).unwrap();
    let saved =
        ugoe_format::module_from_json(&std::fs::read_to_string(dir.join("module.json")).unwrap())
            .unwrap();
    let saved_rig = saved
        .physics
        .rigs
        .iter()
        .find(|r| r.id == rig.as_str())
        .unwrap();
    assert!(saved_rig.colliders.is_empty());
    Project::load(&dir).unwrap();
}

/// 外した参照は ID を型付きで持つが、IPC へ渡す JSON は文字列の ID のまま (T1600)。
#[test]
fn collider_reference_serializes_ids_as_strings() {
    let reference = ColliderReference {
        rig: "Sway".into(),
        collider: "Shoulders".into(),
    };
    assert_eq!(
        serde_json::to_value(&reference).unwrap(),
        serde_json::json!({ "rig": "Sway", "collider": "Shoulders" })
    );
}
