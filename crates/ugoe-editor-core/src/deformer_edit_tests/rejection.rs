//! 作成と付け替えを拒否する場合のテスト。

use super::*;

#[test]
fn create_rejects_invalid_requests() {
    let model = fixtures::rotation_chain();
    let is_edit = |e: EditError| matches!(e, EditError::DeformerEdit { .. });
    assert!(is_edit(rejected(&model, warp(vec![], 1, 1))));
    assert!(is_edit(rejected(
        &model,
        warp(vec![mesh("Palm"), mesh("Palm")], 1, 1)
    )));
    assert!(is_edit(rejected(&model, warp(vec![mesh("Palm")], 0, 1))));
    assert!(is_edit(rejected(&model, warp(vec![mesh("Palm")], 1, 65))));
    assert_eq!(
        rejected(&model, warp(vec![mesh("Nope")], 1, 1)),
        EditError::NotFound {
            kind: "mesh",
            id: "Nope".into()
        }
    );
    let mut dup = warp(vec![mesh("Palm")], 1, 1);
    if let Command::CreateWarpDeformer { id, .. } = &mut dup {
        *id = Some("Arm".into());
    }
    assert!(matches!(
        rejected(&model, dup),
        EditError::DuplicateId { .. }
    ));
    // 親が子自身・子の子孫なら循環する。
    for parent in ["Arm", "Hand", "Cloth"] {
        let command = Command::CreateRotationDeformer {
            id: None,
            name: "R".into(),
            children: vec![def("Arm")],
            parts: Vec::new(),
            origin: None,
            parent: Some(parent.into()),
        };
        assert!(is_edit(rejected(&model, command)), "{parent}");
    }
    assert!(is_edit(rejected(
        &model,
        rotation(vec![mesh("Palm")], Some(Vec2::new(f32::NAN, 0.0)))
    )));
}

#[test]
fn reparent_rejects_cycles_and_unknown_parents() {
    let model = fixtures::rotation_chain();
    let is_edit = |e: EditError| matches!(e, EditError::DeformerEdit { .. });
    assert!(is_edit(rejected(&model, reparent(def("Arm"), Some("Arm")))));
    assert!(is_edit(rejected(
        &model,
        reparent(def("Arm"), Some("Hand"))
    )));
    assert!(matches!(
        rejected(&model, reparent(mesh("Palm"), Some("Nope"))),
        EditError::NotFound { .. }
    ));
    assert!(matches!(
        rejected(&model, reparent(def("Nope"), None)),
        EditError::NotFound { .. }
    ));
}

#[test]
fn reparent_rejects_degenerate_warp() {
    // 制御点がすべて同じ点のワープには逆写像がない。
    let mut model = fixtures::rotation_under_warp();
    if let DeformerKind::Warp(w) = &mut model.deformers[2].kind {
        for p in &mut w.keyforms.keyforms[0].points {
            *p = Vec2::splat(0.5);
        }
    }
    let err = rejected(&model, reparent(mesh("Face"), Some("Cheek")));
    assert!(matches!(err, EditError::DeformerEdit { .. }), "{err}");
}
