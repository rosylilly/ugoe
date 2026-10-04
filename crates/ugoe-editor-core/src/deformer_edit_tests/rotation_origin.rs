//! 回転デフォーマの原点の移動 (`MoveRotationOrigin`) のテスト。

use super::*;

fn move_origin(id: &str, delta: Vec2) -> Command {
    Command::MoveRotationOrigin {
        id: id.into(),
        delta,
    }
}

#[test]
fn move_rotation_origin_shifts_every_cell_and_keeps_rest_look() {
    let model = fixtures::rotation_chain();
    let delta = Vec2::new(10.0, -6.0);
    let after = applied(&model, move_origin("Arm", delta));
    for (a, b) in rotation_keyforms(&model, "Arm")
        .iter()
        .zip(rotation_keyforms(&after, "Arm"))
    {
        assert!(b.origin.abs_diff_eq(a.origin + delta, 1e-5));
        assert_eq!((a.angle, a.scale), (b.angle, b.scale));
    }
    assert_same_look(&model, &after, &[REST, &[("Flip", 1.0)]]);
}

#[test]
fn move_rotation_origin_rotates_children_about_the_new_pivot() {
    // Arm は既定値 (Angle = 0) で角度 0・拡大率 1。Angle = 30 では、前は古い原点、後は新しい原点のまわりに
    // 同じ角度だけ回るので、`後 = 前 + (I - R) · delta`。
    let model = fixtures::rotation_chain();
    let delta = Vec2::new(10.0, -6.0);
    let after = applied(&model, move_origin("Arm", delta));
    let pose: &[(&str, f32)] = &[("Angle", 30.0)];
    let rot = ugoe_core::Mat2::from_angle(30f32.to_radians());
    let shift = delta - rot * delta;
    for (ma, mb) in evaluate(&model, pose).iter().zip(evaluate(&after, pose)) {
        for (p, q) in ma.iter().zip(&mb) {
            assert!(q.abs_diff_eq(*p + shift, TOL), "{p} + {shift} != {q}");
        }
    }
}

#[test]
fn move_rotation_origin_under_warp_keeps_rest_look() {
    let model = fixtures::rotation_under_warp();
    let after = applied(&model, move_origin("Head", Vec2::new(0.1, 0.05)));
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn move_rotation_origin_rejects_invalid_requests() {
    let model = fixtures::rotation_chain();
    assert_eq!(
        rejected(&model, move_origin("Cloth", Vec2::ONE)),
        EditError::DeformerKindMismatch {
            id: "Cloth".into(),
            expected: "rotation"
        }
    );
    assert!(matches!(
        rejected(&model, move_origin("Nope", Vec2::ONE)),
        EditError::NotFound { .. }
    ));
    assert!(matches!(
        rejected(&model, move_origin("Arm", Vec2::new(f32::NAN, 0.0))),
        EditError::DeformerEdit { .. }
    ));
}
