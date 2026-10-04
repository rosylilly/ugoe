//! 見た目を保つ付け替え (`ReparentPreservingLook`) のテスト。

use super::*;

#[test]
fn reparent_mesh_between_rotations_keeps_look_at_every_pose() {
    // Palm: Hand → Arm。Hand は既定値で固定 (Flip = 0) なので、Arm の角度に関わらず一致する。
    let model = fixtures::rotation_chain();
    let after = applied(&model, reparent(mesh("Palm"), Some("Arm")));
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Arm"));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 30.0)]],
    );
}

#[test]
fn reparent_mesh_into_unknown_deformer_uses_its_known_ancestor() {
    // 未知の種類のデフォーマは恒等変換なので、その座標系は祖先の既知のデフォーマ (Arm) のもの。
    let mut model = fixtures::rotation_chain();
    model.deformers.push(Deformer {
        id: "Future".into(),
        name: "Future".into(),
        part: deformer(&model, "Arm").part.clone(),
        parent: Some("Arm".into()),
        kind: DeformerKind::Unknown,
        mirror: None,
        mirror_axis: None,
    });
    let after = applied(&model, reparent(mesh("Palm"), Some("Future")));
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Future"));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 30.0)]],
    );
}

#[test]
fn reparent_mesh_out_of_warp_keeps_rest_look() {
    let model = fixtures::rotation_chain();
    let after = applied(&model, reparent(mesh("Sleeve"), None));
    assert_eq!(mesh_parent(&after, "Sleeve"), None);
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn reparent_mesh_into_warp_keeps_rest_look() {
    // キャンバス座標から、変形した (X = -1 が既定値の台形の) ワープの UV へ逆写像する。
    let mut model = fixtures::warp_grid();
    set_default(&mut model, "X", -0.6);
    let mut free = model.meshes[0].clone();
    free.id = "Free".into();
    free.parent = None;
    for kf in &mut free.keyforms.keyforms {
        for p in &mut kf.positions {
            *p = *p * 100.0 + Vec2::new(40.0, 50.0);
        }
    }
    model.meshes.push(free);
    let after = applied(&model, reparent(mesh("Free"), Some("Warp")));
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn reparent_warp_in_warp_to_root_keeps_rest_look() {
    let model = fixtures::rotation_under_warp();
    let after = applied(&model, reparent(def("Cheek"), None));
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn reparent_rotation_under_warp_to_rotation_keeps_rest_look() {
    // Head (Body の子の回転) を、回した回転デフォーマの下へ。
    let mut model = fixtures::rotation_under_warp();
    set_default(&mut model, "Turn", 0.3);
    let doc_model = applied(
        &model,
        rotation(vec![def("Cheek")], Some(Vec2::new(90.0, 60.0))),
    );
    let after = applied(&doc_model, reparent(def("Head"), Some("New")));
    assert_same_look(&doc_model, &after, &[REST]);
    let after = applied(&doc_model, reparent(def("Head"), None));
    assert_same_look(&doc_model, &after, &[REST]);
}

#[test]
fn reparent_rotation_to_rotated_parent_adjusts_angle() {
    // Arm を 20 度回した状態を既定にし、Hand をキャンバス直下へ。Hand の角度は 20 度増える。
    let mut model = fixtures::rotation_chain();
    set_default(&mut model, "Angle", 20.0);
    let after = applied(&model, reparent(def("Hand"), None));
    assert_same_look(&model, &after, &[REST, &[("Angle", 20.0), ("Flip", 1.0)]]);
    let k = rotation_keyforms(&after, "Hand");
    assert!((k[0].angle - 20.0).abs() < 1e-3, "{}", k[0].angle);
    // 反転したキーフォームも回転量は同じ向き (反転は変えない)。
    assert!(!k[0].reflect_x && k[1].reflect_x);
    assert!((k[1].angle - 20.0).abs() < 1e-3, "{}", k[1].angle);
}

#[test]
fn reparent_rotation_across_reflected_parent_keeps_look() {
    // Palm を回転 New で包み (親 Hand)、Hand が反転している (Flip = 1) 状態を既定にして、New を Arm へ。
    // 古い親と新しい親の線形部分の比が反転を含むので、New の反転と角度の向きが変わる。
    let mut model = fixtures::rotation_chain();
    let with_new = applied(&model, rotation(vec![mesh("Palm")], None));
    model = with_new;
    set_default(&mut model, "Flip", 1.0);
    set_default(&mut model, "Angle", 10.0);
    let before = rotation_keyforms(&model, "New")[0];
    let after = applied(&model, reparent(def("New"), Some("Arm")));
    assert_same_look(&model, &after, &[REST]);
    let k = rotation_keyforms(&after, "New")[0];
    assert_ne!(k.reflect_x, before.reflect_x);
    assert!((k.scale - 0.5).abs() < 1e-4);
}

#[test]
fn reparent_to_same_parent_is_noop() {
    let model = fixtures::rotation_chain();
    let after = applied(&model, reparent(mesh("Palm"), Some("Hand")));
    assert_eq!(after, model);
}

#[test]
fn reparent_marks_derived_cells_for_rederivation() {
    // 付け替えは格子のすべてのセルを変えるので、`touched` に積まれる (authored のセルは authored のまま)。
    let model = fixtures::single_quad();
    let after = applied(&model, reparent(mesh("Quad"), None));
    assert_eq!(after, model);
    let mut model = fixtures::rotation_chain();
    model.meshes[0].keyforms.origins = vec![KeyformOrigin::Authored];
    let after = applied(&model, reparent(mesh("Palm"), None));
    assert_eq!(
        after.meshes[0].keyforms.origins,
        vec![KeyformOrigin::Authored]
    );
}
