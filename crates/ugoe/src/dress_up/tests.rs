use std::sync::Arc;

use ugoe_model::fixtures::{dress_module, dress_up_base, long_hair_module};
use ugoe_model::part::PartKeyform;
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{KeyformGrid, Part, standard};

use super::*;

const DT: f32 = 1.0 / 60.0;

fn param(puppet: &Puppet, id: &str) -> f32 {
    let p = puppet.instance().parameters();
    p.get(p.index_of(id).unwrap())
}

fn set_param(puppet: &mut Puppet, id: &str, value: f32) {
    let p = puppet.instance_mut().parameters_mut();
    let i = p.index_of(id).unwrap();
    p.set(i, value);
}

fn names(puppet: &Puppet) -> Vec<&str> {
    puppet.modules().iter().map(|m| m.name.as_str()).collect()
}

fn mesh_ids(puppet: &Puppet) -> Vec<&str> {
    let model = puppet.instance().compiled().model();
    model.meshes.iter().map(|m| m.id.as_str()).collect()
}

fn module_texture(module: &str, index: u32) -> TextureSource {
    TextureSource::Module {
        module: module.into(),
        index,
    }
}

#[test]
fn attach_and_detach_report_texture_changes() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    assert!(puppet.modules().is_empty());
    assert_eq!(puppet.texture_sources(), [TextureSource::Base(0)]);

    let change = puppet.attach_module(Arc::new(long_hair_module())).unwrap();
    assert_eq!(
        change,
        DressUpChange {
            textures: vec![1],
            texture_count: 2,
        }
    );
    // 差し替えの直後に評価しているので、描画できる状態になっている。
    assert_eq!(puppet.instance().drawables().len(), 5);
    assert!(mesh_ids(&puppet).contains(&"LongHair/Front"));

    let change = puppet.attach_module(Arc::new(dress_module())).unwrap();
    assert_eq!(change.textures, [2]);
    assert_eq!(
        puppet.texture_sources(),
        [
            TextureSource::Base(0),
            module_texture("LongHair", 0),
            module_texture("Dress", 0),
        ]
    );

    // 前のモジュールを外すと、後ろのモジュールのテクスチャの番号がずれる。
    let change = puppet.detach_module("LongHair").unwrap();
    assert_eq!(
        change,
        DressUpChange {
            textures: vec![1],
            texture_count: 2,
        }
    );
    assert_eq!(names(&puppet), ["Dress"]);
    assert_eq!(
        puppet.texture_sources(),
        [TextureSource::Base(0), module_texture("Dress", 0)]
    );

    let change = puppet.detach_module("Dress").unwrap();
    assert_eq!(change.textures, Vec::<u32>::new());
    assert_eq!(change.texture_count, 1);
    assert_eq!(puppet.instance().compiled().model(), &dress_up_base());
    assert_eq!(puppet.base_model(), &dress_up_base());
    assert!(matches!(
        puppet.detach_module("Dress"),
        Err(DressUpError::NotAttached(name)) if name == "Dress"
    ));
}

#[test]
fn attaching_to_an_occupied_slot_replaces_the_module() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    puppet.attach_module(Arc::new(long_hair_module())).unwrap();
    puppet.attach_module(Arc::new(dress_module())).unwrap();
    let mut other = long_hair_module();
    other.name = "OtherHair".into();
    let change = puppet.attach_module(Arc::new(other)).unwrap();
    // 同じ位置に入るので、後ろのモジュールのテクスチャの番号は変わらない。
    assert_eq!(names(&puppet), ["OtherHair", "Dress"]);
    assert_eq!(change.textures, [1]);
    assert!(mesh_ids(&puppet).contains(&"OtherHair/Front"));
    assert!(!mesh_ids(&puppet).contains(&"LongHair/Front"));

    // 同じ名前の別のモジュール (新しい版など) に差し替えても、画像が違いうるので番号を知らせる。
    let change = puppet.attach_module(Arc::new(dress_module())).unwrap();
    assert_eq!(names(&puppet), ["OtherHair", "Dress"]);
    assert_eq!(change.textures, [2]);
    // 同じモジュールを付け直しただけなら、設定し直すものはない。
    let dress = Arc::clone(&puppet.modules()[1]);
    let change = puppet.attach_module(dress).unwrap();
    assert_eq!(change.textures, Vec::<u32>::new());
}

#[test]
fn failed_attach_changes_nothing() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    puppet.attach_module(Arc::new(long_hair_module())).unwrap();
    let compiled = Arc::clone(puppet.instance().compiled());
    let mut hat = dress_module();
    hat.slot = "Hat".into();
    assert!(matches!(
        puppet.attach_module(Arc::new(hat)),
        Err(DressUpError::Compose(ComposeError::UnknownSlot { .. }))
    ));
    assert!(Arc::ptr_eq(puppet.instance().compiled(), &compiled));
    assert_eq!(names(&puppet), ["LongHair"]);
}

#[test]
fn parameters_carry_over_and_keep_their_base_values() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    set_param(&mut puppet, standard::ANGLE_X, 12.0);
    set_param(&mut puppet, "BodySway", 0.7);
    puppet.update(DT);
    puppet.attach_module(Arc::new(dress_module())).unwrap();
    assert_eq!(param(&puppet, standard::ANGLE_X), 12.0);
    // 基準値も引き継ぐので、次の `update` でも値は保たれる。
    puppet.update(DT);
    assert_eq!(param(&puppet, standard::ANGLE_X), 12.0);
    assert_eq!(param(&puppet, "BodySway"), 0.7);
    let skirt = puppet
        .instance()
        .compiled()
        .mesh_index("Dress/Skirt")
        .unwrap();
    let x = puppet.instance().drawables()[skirt].positions[0].x;
    assert!((x - (64.0 + 22.4)).abs() < 1e-3, "{x}");
}

#[test]
fn module_physics_is_added_and_removed() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    assert!(puppet.physics().is_none());
    // 本体に物理が無くても、モジュールのリグは既定の設定に加わる。
    puppet.attach_module(Arc::new(long_hair_module())).unwrap();
    let physics = puppet.physics().unwrap();
    assert_eq!(physics.settings().rigs[0].id, "LongHair/Sway");
    assert_eq!(physics.settings().gravity, ugoe_core::Vec2::new(0.0, 1.0));
    assert!(puppet.base_physics().is_none());
    set_param(&mut puppet, standard::ANGLE_X, 20.0);
    for _ in 0..10 {
        puppet.update(DT);
    }
    puppet.detach_module("LongHair").unwrap();
    assert!(puppet.physics().is_none());
}

#[test]
fn base_physics_is_kept_and_collides_through_the_slot() {
    let mut base = dress_up_base();
    base.slots[0].colliders = vec!["Chest".into()];
    let settings = PhysicsSettings {
        colliders: vec![ugoe_model::physics::PhysicsCollider {
            id: "Chest".into(),
            name: "Chest".into(),
            attach: Some("Torso".into()),
            shape: ugoe_model::physics::ColliderShape::Circle {
                center: ugoe_core::Vec2::new(0.5, 0.0),
                radius: 8.0,
            },
        }],
        ..empty_physics()
    };
    let mut puppet = Puppet::new(base).unwrap().with_physics(settings.clone());
    assert_eq!(puppet.base_physics(), Some(&settings));
    let mut hair = long_hair_module();
    hair.physics.rigs[0].colliders.push("Chest".into());
    puppet.attach_module(Arc::new(hair)).unwrap();
    let composed = puppet.physics().unwrap().settings();
    assert_eq!(composed.rigs[0].colliders, ["LongHair/Collar", "Chest"]);
    assert_eq!(puppet.base_physics(), Some(&settings));
    puppet.update(DT);

    puppet.set_modules(Vec::new()).unwrap();
    assert_eq!(puppet.physics().unwrap().settings(), &settings);
}

/// 本体 (スロット `Hair` が衝突体 `Chest` を公開する) と、`Chest` を持つ物理の設定。
fn chest_base() -> (ugoe_model::Model, PhysicsSettings) {
    let mut base = dress_up_base();
    base.slots[0].colliders = vec!["Chest".into()];
    let settings = PhysicsSettings {
        colliders: vec![ugoe_model::physics::PhysicsCollider {
            id: "Chest".into(),
            name: "Chest".into(),
            attach: Some("Torso".into()),
            shape: ugoe_model::physics::ColliderShape::Circle {
                center: ugoe_core::Vec2::new(0.5, 0.0),
                radius: 8.0,
            },
        }],
        ..empty_physics()
    };
    (base, settings)
}

/// `Chest` に当たる `LongHair`。
fn chest_hair() -> Arc<Module> {
    let mut hair = long_hair_module();
    hair.physics.rigs[0].colliders.push("Chest".into());
    Arc::new(hair)
}

/// 付けている間に `set_base_physics` で差し替えた本体の設定は、合成し直して使い、付け外しの後も保たれる
/// (`set_physics` で渡した設定は次の付け外しで失われる)。
#[test]
fn base_physics_replaced_while_dressed_survives_dress_up() {
    let (base, settings) = chest_base();
    let mut puppet = Puppet::new(base).unwrap().with_physics(settings.clone());
    puppet.attach_module(chest_hair()).unwrap();
    set_param(&mut puppet, standard::ANGLE_X, 20.0);

    let windy = PhysicsSettings {
        wind: ugoe_core::Vec2::new(0.3, 0.0),
        ..settings.clone()
    };
    puppet.set_base_physics(Some(windy.clone())).unwrap();
    assert_eq!(puppet.base_physics(), Some(&windy));
    let composed = puppet.physics().unwrap().settings();
    assert_eq!(composed.wind, windy.wind);
    assert_eq!(composed.rigs[0].colliders, ["LongHair/Collar", "Chest"]);
    // モデルは変わらないので、パラメータもそのまま。
    assert_eq!(param(&puppet, standard::ANGLE_X), 20.0);
    puppet.update(DT);

    puppet.attach_module(Arc::new(dress_module())).unwrap();
    assert_eq!(puppet.physics().unwrap().settings().wind, windy.wind);
    puppet.set_modules(Vec::new()).unwrap();
    assert_eq!(puppet.physics().unwrap().settings(), &windy);

    // set_physics は今の設定だけを差し替える。
    puppet.attach_module(chest_hair()).unwrap();
    let mut calm = puppet.physics().unwrap().settings().clone();
    calm.wind = ugoe_core::Vec2::ZERO;
    puppet.set_physics(Some(calm)).unwrap();
    puppet.attach_module(Arc::new(dress_module())).unwrap();
    assert_eq!(puppet.physics().unwrap().settings().wind, windy.wind);
}

/// 本体の物理の有無を切り替えると、スロットが公開する衝突体を指すかどうかも変わる (モデルを差し替える)。
#[test]
fn base_physics_toggles_exposed_colliders_while_dressed() {
    let (base, settings) = chest_base();
    let mut puppet = Puppet::new(base).unwrap();
    puppet.attach_module(chest_hair()).unwrap();
    let rig_colliders = |puppet: &Puppet| {
        puppet.physics().unwrap().settings().rigs[0]
            .colliders
            .clone()
    };
    let slot_colliders = |puppet: &Puppet| {
        puppet.instance().compiled().model().slots[0]
            .colliders
            .clone()
    };
    assert_eq!(rig_colliders(&puppet), ["LongHair/Collar"]);
    assert!(slot_colliders(&puppet).is_empty());
    set_param(&mut puppet, standard::ANGLE_X, 20.0);

    puppet.set_base_physics(Some(settings.clone())).unwrap();
    assert_eq!(rig_colliders(&puppet), ["LongHair/Collar", "Chest"]);
    assert_eq!(slot_colliders(&puppet), ["Chest"]);
    assert_eq!(param(&puppet, standard::ANGLE_X), 20.0);
    assert_eq!(names(&puppet), ["LongHair"]);
    puppet.update(DT);

    puppet.set_base_physics(None).unwrap();
    assert!(puppet.base_physics().is_none());
    assert_eq!(rig_colliders(&puppet), ["LongHair/Collar"]);
    assert!(slot_colliders(&puppet).is_empty());
    puppet.detach_module("LongHair").unwrap();
    assert!(puppet.physics().is_none());
}

/// 本体のモデルに対する検査を通らない設定では、何も変えない。モジュールを付けたことがなければ `set_physics` と同じ。
#[test]
fn invalid_base_physics_changes_nothing() {
    let (base, settings) = chest_base();
    let mut puppet = Puppet::new(base).unwrap();
    // スロットが公開する `Chest` が無い。
    let err = puppet.set_base_physics(Some(empty_physics())).unwrap_err();
    assert_eq!(err.code(), "invalid_physics");
    assert!(puppet.physics().is_none());
    puppet.set_base_physics(Some(settings.clone())).unwrap();
    assert_eq!(puppet.physics().unwrap().settings(), &settings);
    assert_eq!(puppet.base_physics(), Some(&settings));

    puppet.attach_module(chest_hair()).unwrap();
    let before = puppet.physics().unwrap().settings().clone();
    assert!(puppet.set_base_physics(Some(empty_physics())).is_err());
    assert_eq!(puppet.physics().unwrap().settings(), &before);
    assert_eq!(puppet.base_physics(), Some(&settings));
}

/// 物理を渡さずにモデルだけで作った Puppet でも、スロットが衝突体を公開するスロットにモジュールを付けられる。
/// 公開した本体の衝突体は何も指さず、リグの参照は外れる (T0655)。
#[test]
fn exposed_colliders_are_ignored_without_base_physics() {
    let mut base = dress_up_base();
    base.slots[0].colliders = vec!["Chest".into()];
    let mut puppet = Puppet::new(base).unwrap();
    let mut hair = long_hair_module();
    hair.physics.rigs[0].colliders.push("Chest".into());
    puppet.attach_module(Arc::new(hair)).unwrap();
    let composed = puppet.physics().unwrap().settings();
    assert_eq!(composed.rigs[0].colliders, ["LongHair/Collar"]);
    assert!(puppet.base_physics().is_none());
    // 本体として覚えたモデルのスロットは変えない (後から物理を持つ本体として使える)。
    assert_eq!(puppet.base_model().slots[0].colliders, ["Chest"]);
    set_param(&mut puppet, standard::ANGLE_X, 20.0);
    for _ in 0..10 {
        puppet.update(DT);
    }
    puppet.detach_module("LongHair").unwrap();
    assert!(puppet.physics().is_none());
}

/// 腕の差分 `ArmA` / `ArmB` のポーズを持つ本体と、それに袖を追従させる `Dress`。
fn pose_base_and_module() -> (ugoe_model::Model, Module) {
    let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
    let entry = |part: &str, links: &[&str]| PoseEntry {
        part: part.into(),
        links: links.iter().map(|&l| l.into()).collect(),
    };
    let mut base = dress_up_base();
    for id in ["ArmA", "ArmB"] {
        base.parts.push(Part {
            id: id.into(),
            name: id.into(),
            parent: Some("Body".into()),
            keyforms: opaque(),
        });
    }
    base.pose.groups = vec![PoseGroup {
        entries: vec![entry("ArmA", &[]), entry("ArmB", &[])],
    }];
    base.slots[1].pose_parts = vec!["ArmA".into(), "ArmB".into()];
    let mut dress = dress_module();
    for id in ["SleeveA", "SleeveB"] {
        dress.parts.push(Part {
            id: id.into(),
            name: id.into(),
            parent: Some("Dress".into()),
            keyforms: opaque(),
        });
    }
    dress.pose.links = vec![entry("ArmA", &["SleeveA"]), entry("ArmB", &["SleeveB"])];
    (base, dress)
}

#[test]
fn pose_state_carries_over_and_module_parts_follow() {
    let (base, dress) = pose_base_and_module();
    let mut puppet = Puppet::new(base).unwrap();
    puppet.set_effects(ugoe_model::Effects::default());
    assert!(puppet.pose_mut().show(&"ArmB".into()));
    puppet.update(1.0);
    let opacity = |puppet: &Puppet, id: &str| {
        let i = puppet.instance().compiled().part_index(id).unwrap();
        puppet.instance().part_opacity_override(i)
    };
    assert_eq!(opacity(&puppet, "ArmB"), (1.0, 1.0));

    puppet.attach_module(Arc::new(dress)).unwrap();
    // 表示中の要素は `ArmB` のまま (先頭の要素に戻ってフェードし直さない)。
    assert_eq!(opacity(&puppet, "ArmB"), (1.0, 1.0));
    assert_eq!(opacity(&puppet, "ArmA"), (0.0, 1.0));
    assert_eq!(opacity(&puppet, "Dress/SleeveB"), (1.0, 1.0));
    assert_eq!(opacity(&puppet, "Dress/SleeveA"), (0.0, 1.0));
    puppet.update(DT);
    assert_eq!(opacity(&puppet, "Dress/SleeveB"), (1.0, 1.0));
    assert_eq!(opacity(&puppet, "Dress/SleeveA"), (0.0, 1.0));
}

#[test]
fn errors_have_stable_codes_and_fields() {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    let error = puppet.detach_module("LongHair").unwrap_err();
    assert_eq!(error.code(), "not_attached");
    assert_eq!(error.fields(), [("module", "LongHair")]);

    puppet.attach_module(Arc::new(long_hair_module())).unwrap();
    let mut other = long_hair_module();
    other.name = "OtherHair".into();
    let mut modules = puppet.modules().to_vec();
    modules.push(Arc::new(other));
    let error = puppet.set_modules(modules).unwrap_err();
    assert_eq!(error.code(), "slot_occupied");
    assert_eq!(
        error.fields(),
        [
            ("slot", long_hair_module().slot.as_str()),
            ("module", "OtherHair"),
            ("attached", "LongHair"),
        ]
    );
}
