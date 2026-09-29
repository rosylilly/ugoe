//! 物理の警告 (`physics_check.rs`) のテスト。入力は `fixtures::dynamic_hair` と `fixtures::dynamic_hair_settings`。

use ugoe_core::Vec2;
use ugoe_model::fixtures;
use ugoe_model::physics::{ColliderShape, PhysicsSettings};

use crate::physics_check::{RestPoseCollision, rest_pose_collisions};

/// 肩のカプセルを、真下に垂れた髪 (x = 148、y = 100..190) を横切る線分 (118,140)–(168,160) に動かした設定。
fn crossing_settings() -> PhysicsSettings {
    let mut settings = fixtures::dynamic_hair_settings();
    settings.colliders[0].shape = ColliderShape::Capsule {
        a: Vec2::new(-10.0, -60.0),
        b: Vec2::new(40.0, -40.0),
        radius: 8.0,
    };
    settings
}

#[test]
fn fixture_has_no_rest_pose_collision() {
    let found = rest_pose_collisions(
        &fixtures::dynamic_hair(),
        &fixtures::dynamic_hair_settings(),
    )
    .unwrap();
    assert_eq!(found, vec![]);
}

#[test]
fn collider_across_the_rest_pose_is_reported_per_rig_and_collider() {
    let found = rest_pose_collisions(&fixtures::dynamic_hair(), &crossing_settings()).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    let RestPoseCollision {
        rig,
        collider,
        particles,
    } = &found[0];
    assert_eq!((rig.as_str(), collider.as_str()), ("Hair", "Shoulder"));
    // 肩は粒子 1 (y = 130) と 2 (y = 160) の間を横切る。
    assert!(particles.contains(&2), "{particles:?}");
    assert!(particles.windows(2).all(|w| w[0] < w[1]), "{particles:?}");
}

#[test]
fn a_second_collider_gets_its_own_entry() {
    let mut settings = crossing_settings();
    let mut circle = settings.colliders[0].clone();
    circle.id = "Chest".into();
    circle.shape = ColliderShape::Circle {
        center: Vec2::new(15.0, -40.0),
        radius: 6.0,
    };
    settings.colliders.push(circle);
    settings.rigs[0].colliders.push("Chest".into());
    let found = rest_pose_collisions(&fixtures::dynamic_hair(), &settings).unwrap();
    let colliders: Vec<_> = found.iter().map(|c| c.collider.as_str()).collect();
    assert_eq!(colliders, ["Shoulder", "Chest"], "{found:?}");
}

#[test]
fn parameter_only_physics_is_not_checked() {
    let found = rest_pose_collisions(
        &fixtures::physics_hair(),
        &fixtures::physics_hair_settings(),
    )
    .unwrap();
    assert_eq!(found, vec![]);
}
