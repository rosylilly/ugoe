//! `ugoe validate` の物理の警告 (`ugoe_editor_core::physics_check`)。

use ugoe_editor_core::physics_check::{RestPoseCollision, rest_pose_collisions};
use ugoe_model::Model;
use ugoe_model::physics::PhysicsSettings;

/// 物理の警告の文 (1 件 1 行)。警告はエラーではないので、検証の結果 (`ok`) は変えない。
pub fn warnings(model: &Model, physics: &PhysicsSettings) -> anyhow::Result<Vec<String>> {
    Ok(rest_pose_collisions(model, physics)?
        .iter()
        .map(rest_pose_message)
        .collect())
}

/// 静止姿勢で衝突体に当たっている鎖 (`docs/spec/physics.md#静止姿勢で衝突体に入っている場合`)。
fn rest_pose_message(c: &RestPoseCollision) -> String {
    let particles = c
        .particles
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "physics rig `{}` hits collider `{}` in its rest pose (segments ending at particle {particles}); \
         it will start bent along the collider instead of hanging freely",
        c.rig, c.collider
    )
}

#[cfg(test)]
mod tests {
    use ugoe_core::Vec2;
    use ugoe_model::fixtures;
    use ugoe_model::physics::ColliderShape;

    use super::*;

    #[test]
    fn rest_pose_collisions_become_warnings() {
        let model = fixtures::dynamic_hair();
        assert!(
            warnings(&model, &fixtures::dynamic_hair_settings())
                .unwrap()
                .is_empty()
        );
        // 肩を、真下に垂れた髪を横切る位置へ動かす。
        let mut settings = fixtures::dynamic_hair_settings();
        settings.colliders[0].shape = ColliderShape::Capsule {
            a: Vec2::new(-10.0, -60.0),
            b: Vec2::new(40.0, -40.0),
            radius: 8.0,
        };
        let found = warnings(&model, &settings).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("physics rig `Hair` hits collider `Shoulder` in its rest pose"),
            "{}",
            found[0]
        );
    }
}
