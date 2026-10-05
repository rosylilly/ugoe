//! 見た目を保つ付け替えとミラーのテスト (T0607 × T0614・T0618)。

use super::*;

fn reparent_to_root(id: &str) -> Command {
    Command::ReparentPreservingLook {
        element: crate::ChildRef::Mesh(id.into()),
        new_parent: None,
    }
}

/// `keyform_inference` の `Face` (64..192 の正方形のワープ) の UV → キャンバス座標。
fn face_to_canvas(p: Vec2) -> Vec2 {
    Vec2::splat(64.0) + p * 128.0
}

#[track_caller]
fn assert_positions(actual: &MeshKeyform, expected: impl IntoIterator<Item = Vec2>) {
    for (p, q) in actual.positions.iter().zip(expected) {
        assert!(p.abs_diff_eq(q, 1e-3), "{p} != {q}");
    }
}

#[test]
fn reparenting_a_mirror_target_keeps_its_cells_and_source() {
    // 右目 (ミラー先) だけをワープの外へ出す。検証を通り、ミラー元の記録と由来はそのまま、
    // ミラーのセルは見た目を保って写し直される (ミラー元が変わっていないので、ミラーし直さない)。
    let model = mirrored_model();
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("EyeR")).unwrap();
    assert_eq!(report.warnings, vec![]);
    let after = applied(&model, reparent_to_root("EyeR"));
    let (before, right) = (
        &mesh(&model, "EyeR").keyforms,
        &mesh(&after, "EyeR").keyforms,
    );
    assert_eq!(right.mirror_source, before.mirror_source);
    assert_eq!(right.origins, before.origins);
    for (b, a) in before.keyforms.iter().zip(&right.keyforms) {
        assert_positions(a, b.positions.iter().map(|&p| face_to_canvas(p)));
    }

    // 片側だけ付け替えると対の親が異なる。T0615 の時点ではミラー元の座標系で反転した値をそのまま
    // 書いたのでミラーし直せず警告を返したが、T0618 からはキャンバス座標を経由して反転するので、
    // ミラー元を編集すると警告なしにミラーし直される。結果は、付け替えずに編集したものを
    // キャンバス座標へ写したものと同じ (`Face` はどのセルでも同じ形なので、姿勢によらない)。
    let left = base(&after);
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(-30.0, 0.0),
        keyform: shifted(&left, -0.2, 0.0),
    };
    let mut doc = Document::new(after.clone());
    let report = doc.apply_with_report(edit.clone()).unwrap();
    assert_eq!(report.warnings, vec![]);
    let mut unmoved = Document::new(model.clone());
    unmoved.apply(edit.clone()).unwrap();
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0)] {
        assert_eq!(right_origin(doc.model(), x, y), Mirror);
        assert_positions(
            right_cell(doc.model(), x, y),
            right_cell(unmoved.model(), x, y)
                .positions
                .iter()
                .map(|&p| face_to_canvas(p)),
        );
    }

    // 左目も付け替えて座標系をそろえると、ミラー元の編集はまたミラーし直される。
    // 結果は、付け替えずに編集したものをキャンバス座標へ写したものと同じ。
    let both = applied(&after, reparent_to_root("EyeL"));
    let left = base(&both);
    let mut doc = Document::new(both);
    let report = doc
        .apply_with_report(Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: xy(-30.0, 0.0),
            keyform: MeshKeyform {
                positions: base(&model)
                    .positions
                    .iter()
                    .map(|&p| face_to_canvas(p + Vec2::new(-0.2, 0.0)))
                    .collect(),
                ..left
            },
        })
        .unwrap();
    assert_eq!(report.warnings, vec![]);
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0)] {
        assert_eq!(right_origin(doc.model(), x, y), Mirror);
        assert_positions(
            right_cell(doc.model(), x, y),
            right_cell(unmoved.model(), x, y)
                .positions
                .iter()
                .map(|&p| face_to_canvas(p)),
        );
    }
}

#[test]
fn reparenting_a_self_mirrored_element_remirrors_in_the_new_space() {
    // 左右対称な 1 つのメッシュ (`Nose`、ミラー元が自分自身) の AngleX = 30 をミラーで作り、
    // ワープの外へ出す。付け替えは格子を変えるので、ミラーのセルは新しい座標系でミラーし直され、
    // 写し直した値と一致する (対称軸の既定値はワープの 0.5 からキャンバスの原点 128 に変わるが、
    // どちらも同じ縦の直線)。
    let mut model = fixtures::keyform_inference();
    // AngleX を自分自身と対にする (AngleX = 30 のセルは AngleX = -30 のセルの反転)。
    let angle_x = model.parameters.iter_mut().find(|p| p.id == "AngleX");
    angle_x.unwrap().mirror = Some("AngleX".into());
    let nose = mesh_mut(&mut model, "Nose");
    let base = nose.keyforms.keyforms[0].clone();
    nose.keyforms = KeyformGrid::new(
        vec![binding("AngleX", &KEYS)],
        vec![shifted(&base, -0.05, 0.0), base.clone(), base],
    );
    nose.keyforms.origins = vec![Authored, Authored, Derived];
    let mut doc = Document::new(model);
    doc.apply(Command::MirrorKeyforms {
        source: target("Nose"),
        target: target("Nose"),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    })
    .unwrap();
    let model = doc.model().clone();
    let before = &mesh(&model, "Nose").keyforms;
    assert_eq!(before.origin(2), Mirror);
    assert_shift(&before.keyforms[2], &before.keyforms[1], 0.05, 0.0);

    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("Nose")).unwrap();
    assert_eq!(report.warnings, vec![]);
    let after = applied(&model, reparent_to_root("Nose"));
    let nose = &mesh(&after, "Nose").keyforms;
    assert_eq!(nose.mirror_source, before.mirror_source);
    assert_eq!(nose.origins, before.origins);
    for (b, a) in before.keyforms.iter().zip(&nose.keyforms) {
        assert_positions(a, b.positions.iter().map(|&p| face_to_canvas(p)));
    }
}

// ---- 親が異なる左右の対 (T0618) ----

/// `mirrored_model` の右目のキャンバス座標 (既定値の姿勢)。
fn right_canvas(model: &Model, x: f32, y: f32) -> Vec<Vec2> {
    crate::space::local_to_canvas(
        model,
        &target("EyeR"),
        &Default::default(),
        &right_cell(model, x, y).positions,
    )
    .unwrap()
}

#[test]
fn wrapping_one_side_in_a_new_warp_keeps_remirroring() {
    // 右目だけを新しいワープで包む (親が `Face` と `New` に分かれる)。対は対応したままで警告なし。
    let model = mirrored_model();
    let wrap = Command::CreateWarpDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children: vec![crate::ChildRef::Mesh("EyeR".into())],
        parts: Vec::new(),
        rows: 2,
        cols: 2,
        parent: None,
    };
    let mut doc = Document::new(model.clone());
    assert_eq!(doc.apply_with_report(wrap).unwrap().warnings, vec![]);
    assert_eq!(mesh(doc.model(), "EyeR").parent, Some("New".into()));

    // ミラー元を編集すると、キャンバス座標を経由してミラーし直される。見た目は、付け替えずに
    // 編集したときと同じ。
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 0.0),
        keyform: shifted(&base(&model), 0.07, -0.02),
    };
    assert_eq!(
        doc.apply_with_report(edit.clone()).unwrap().warnings,
        vec![]
    );
    let unmoved = applied(&model, edit);
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0), (30.0, 30.0)] {
        let expected: Vec<Vec2> = right_cell(&unmoved, x, y)
            .positions
            .iter()
            .map(|&p| face_to_canvas(p))
            .collect();
        for (p, q) in right_canvas(doc.model(), x, y).iter().zip(&expected) {
            assert!(p.abs_diff_eq(*q, 1e-2), "({x}, {y}): {p} != {q}");
        }
    }
}

/// `mirrored_model` の `Face` を x に −20 ずらし、目の `mirror_axis` を外したモデル。
/// 左右の目は `Face` の UV 空間の u = 0.5 (キャンバスの x = 108) で対称で、キャンバスの原点
/// (x = 128) では対称でない。
fn off_center_model() -> Model {
    let mut model = mirrored_model();
    for id in ["EyeL", "EyeR"] {
        mesh_mut(&mut model, id).mirror_axis = None;
    }
    let face = model.deformers.iter_mut().find(|d| d.id == "Face").unwrap();
    if let ugoe_model::deformer::DeformerKind::Warp(w) = &mut face.kind {
        for kf in &mut w.keyforms.keyforms {
            kf.points.iter_mut().for_each(|p| p.x -= 20.0);
        }
    }
    model
}

#[test]
fn reparenting_breaks_an_off_center_pair_and_warns() {
    // 親が同じなら対称軸の既定値は親の UV 空間の 0.5。右目を外に出すと親が異なり、既定値は
    // キャンバスの原点になるので対応しなくなる。付け替えは見た目を保って行い、警告を返す。
    let model = off_center_model();
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("EyeR")).unwrap();
    assert_eq!(
        report
            .warnings
            .iter()
            .map(|w| (&w.target, &w.source))
            .collect::<Vec<_>>(),
        vec![(&target("EyeR"), &target("EyeL"))]
    );
    assert!(
        report.warnings[0]
            .message
            .contains("no mirrored source vertex"),
        "{}",
        report.warnings[0].message
    );
    // `apply` は同じ結果になる (警告を捨てるだけ)。
    assert_eq!(&applied(&model, reparent_to_root("EyeR")), doc.model());

    // 元の対称軸を `Face` の UV 空間で指定すると (キャンバスへ写すと x = 108)、また対応する。
    let report = doc
        .apply_with_report(Command::SetMirrorPair {
            target: target("EyeL"),
            mirror: Some("EyeR".into()),
            axis: Some(0.5),
        })
        .unwrap();
    assert_eq!(report.warnings, vec![]);
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 0.0),
        keyform: shifted(&base(&model), 0.07, 0.0),
    };
    assert_eq!(doc.apply_with_report(edit).unwrap().warnings, vec![]);
    // 右目の AngleX = 30 は、左目の AngleX = 30 (u +0.07) をキャンバスの x = 108 で反転したもの
    // (このモデルの AngleX は対にしていない)。
    let left: Vec<Vec2> = shifted(&base(&model), 0.07, 0.0)
        .positions
        .iter()
        .map(|&p| Vec2::new(216.0 - (44.0 + p.x * 128.0), 64.0 + p.y * 128.0))
        .collect();
    let mut right = right_canvas(doc.model(), 30.0, 0.0);
    right.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
    let mut left = left;
    left.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
    for (p, q) in right.iter().zip(&left) {
        assert!(p.abs_diff_eq(*q, 1e-2), "{p} != {q}");
    }
}

#[test]
fn reparenting_the_source_warns_once() {
    // ミラー元を付け替えると、依存する格子をミラーし直す (失敗すれば警告)。付け替えの検査は
    // 同じ組の警告を重ねない。
    let model = off_center_model();
    let (after, warnings) = warnings_of(&model, reparent_to_root("EyeL"));
    assert_eq!(
        warnings
            .iter()
            .map(|w| (&w.target, &w.source))
            .collect::<Vec<_>>(),
        vec![(&target("EyeR"), &target("EyeL"))]
    );
    assert_eq!(mesh(&after, "EyeR").keyforms, mesh(&model, "EyeR").keyforms);
}

#[test]
fn reparenting_an_unpaired_or_already_broken_pair_does_not_warn() {
    // 対のない要素 (`Nose`) の付け替えは検査しない。
    let model = off_center_model();
    let (_, warnings) = warnings_of(&model, reparent_to_root("Nose"));
    assert_eq!(warnings, vec![]);
    // 付け替えの前からミラーできない対は、付け替えでは警告しない (壊れたときに警告済み)。
    let mut broken = model.clone();
    mesh_mut(&mut broken, "EyeR").keyforms.keyforms[4]
        .positions
        .iter_mut()
        .for_each(|p| p.y += 0.2);
    let (_, warnings) = warnings_of(&broken, reparent_to_root("EyeR"));
    assert_eq!(warnings, vec![]);
}
