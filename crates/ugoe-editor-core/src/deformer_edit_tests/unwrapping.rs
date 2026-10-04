//! デフォーマを外す (`UnwrapDeformer`) のテスト。

use super::*;

#[test]
fn unwrap_moves_children_to_the_parent_and_keeps_look() {
    // Hand は既定値 (Flip = 0) で固定なので、Arm の角度に関わらず一致する。
    let model = fixtures::rotation_chain();
    let after = applied(&model, unwrap("Hand"));
    assert!(after.deformers.iter().all(|d| d.id.as_str() != "Hand"));
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Arm"));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 30.0)]],
    );
}

#[test]
fn unwrap_root_deformer_moves_nested_children_to_root() {
    let model = fixtures::rotation_chain();
    let after = applied(&model, unwrap("Arm"));
    assert!(after.deformers.iter().all(|d| d.id.as_str() != "Arm"));
    assert_eq!(deformer(&after, "Hand").parent, None);
    assert_eq!(deformer(&after, "Cloth").parent, None);
    assert_same_look(&model, &after, &[REST, &[("Flip", 1.0)]]);
}

#[test]
fn unwrap_warp_inside_warp_keeps_rest_look() {
    let model = fixtures::rotation_under_warp();
    let after = applied(&model, unwrap("Cheek"));
    assert_eq!(mesh_parent(&after, "Blush").as_deref(), Some("Body"));
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn unwrap_clears_mirror_pairs_and_rejects_unknown_ids() {
    let mut model = fixtures::rotation_chain();
    model.deformers[0].mirror = Some("Hand".into());
    model.deformers[1].mirror = Some("Arm".into());
    let after = applied(&model, unwrap("Hand"));
    assert_eq!(deformer(&after, "Arm").mirror, None);
    assert!(matches!(
        rejected(&model, unwrap("Nope")),
        EditError::NotFound { .. }
    ));
}

#[test]
fn unwrap_forgets_the_deformer_as_a_mirror_source() {
    // Arm の格子のミラー元が Hand。Hand を外すと、削除 (`remove_deformer`) と同じくミラー元の記録を外す。
    let mut model = fixtures::rotation_chain();
    if let DeformerKind::Rotation(r) = &mut model.deformers[0].kind {
        r.keyforms.mirror_source = Some(MirrorSource {
            id: "Hand".into(),
            axis: None,
            tolerance: None,
        });
    }
    let after = applied(&model, unwrap("Hand"));
    match &deformer(&after, "Arm").kind {
        DeformerKind::Rotation(r) => assert_eq!(r.keyforms.mirror_source, None),
        _ => panic!("Arm is a rotation deformer"),
    }
}
