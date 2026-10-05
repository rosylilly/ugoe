//! デフォーマの作成 (`CreateWarpDeformer`・`CreateRotationDeformer`) のテスト。

use super::*;

#[test]
fn create_warp_around_root_mesh_keeps_look_at_every_pose() {
    let model = fixtures::single_quad();
    let after = applied(&model, warp(vec![mesh("Quad")], 2, 3));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("X", -1.0)], &[("X", 0.3)], &[("X", 1.0)]],
    );
    assert_eq!(mesh_parent(&after, "Quad").as_deref(), Some("New"));
    let d = deformer(&after, "New");
    assert_eq!(d.parent, None);
    assert_eq!(d.part.as_str(), "Root");
    // 基準の姿 (64..192 の正方形) に長い辺の 5% の余白を付けた矩形を 3×2 セルに等分する。
    let points = warp_points(&after, "New");
    assert_eq!(points.len(), 4 * 3);
    assert!(points[0].abs_diff_eq(Vec2::new(57.6, 57.6), 1e-3));
    assert!(points[11].abs_diff_eq(Vec2::new(198.4, 198.4), 1e-3));
    assert!(points[1].abs_diff_eq(Vec2::new(57.6 + 140.8 / 3.0, 57.6), 1e-3));
    // 子の座標は UV (0..1) になる。
    let quad = &after.meshes[0].keyforms.keyforms[1].positions;
    assert!(
        quad.iter()
            .all(|p| p.cmpge(Vec2::ZERO).all() && p.cmple(Vec2::ONE).all())
    );
}

#[test]
fn create_warp_inside_warp_keeps_look_where_parent_is_affine() {
    // Inner と Outer の親は Warp (X = 0 で正方形、X = 1 でせん断、X = -1 で台形)。
    let model = fixtures::warp_grid();
    let after = applied(&model, warp(vec![mesh("Inner"), mesh("Outer")], 2, 2));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Warp"
    );
    // 親のワープがアフィンな姿 (0 ≤ X ≤ 1) では厳密に一致する。
    assert_same_look(&model, &after, &[REST, &[("X", 0.4)], &[("X", 1.0)]]);
    // 台形では、新しいワープは親の写像を制御点でしか標本化しないので、子の見た目は少しずれる
    // (ワープの中のワープに固有の近似。docs/editor.md#デフォーマの編集)。
    let (a, b) = (
        evaluate(&model, &[("X", -1.0)]),
        evaluate(&after, &[("X", -1.0)]),
    );
    let error = a
        .iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(p, q)| p.distance(*q))
        .fold(0.0, f32::max);
    assert!(error > TOL && error < 16.0, "{error}");
}

#[test]
fn create_warp_around_rotation_deformer_keeps_look_at_every_pose() {
    // Hand (回転、親 Arm) をワープで包む。Hand は「ワープの子の回転デフォーマ」になる。
    let model = fixtures::rotation_chain();
    let after = applied(&model, warp(vec![def("Hand")], 1, 1));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Arm"
    );
    assert_eq!(
        deformer(&after, "Hand").parent.as_ref().unwrap().as_str(),
        "New"
    );
    assert_same_look(
        &model,
        &after,
        &[
            REST,
            &[("Angle", -30.0)],
            &[("Angle", 12.0)],
            &[("Angle", 30.0), ("Flip", 1.0)],
        ],
    );
}

#[test]
fn create_warp_with_children_under_different_parents_uses_common_ancestor() {
    // Palm (親 Hand) と Sleeve (親 Cloth) の共通の祖先は Arm。
    let model = fixtures::rotation_chain();
    let after = applied(&model, warp(vec![mesh("Palm"), mesh("Sleeve")], 2, 2));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Arm"
    );
    // 親 (Hand・Cloth) は既定値で固定なので、Arm だけが動くポーズでは厳密に保たれる。
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 20.0)]],
    );
}

#[test]
fn create_warp_keeps_children_given_with_their_parent_deformer() {
    // Palm の親 Hand も子に指定した。Palm は Hand ごと移るので、Hand の下に残る
    // (パーツを包むときと同じ。Palm を新しいワープへ直接付け替えると Hand の反転に追従しなくなる)。
    let model = fixtures::rotation_chain();
    let after = applied(&model, warp(vec![def("Hand"), mesh("Palm")], 2, 2));
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Hand"));
    assert_eq!(
        deformer(&after, "Hand").parent.as_ref().unwrap().as_str(),
        "New"
    );
    assert_same_look(&model, &after, &[REST, &[("Flip", 1.0)]]);
}

#[test]
fn create_warp_under_warp_parent_with_rotation_child_keeps_rest_look() {
    // Head (Body の子の回転デフォーマ) を包む。新しいワープは Body の UV 空間に置かれる。
    let model = fixtures::rotation_under_warp();
    let after = applied(&model, warp(vec![def("Head"), def("Cheek")], 2, 2));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Body"
    );
    assert_same_look(&model, &after, &[REST]);
    // 新しいワープの格子は Body の UV 空間の矩形 (Body は既定値で回っていない)。
    let points = warp_points(&after, "New");
    assert!(points.iter().all(|p| p.cmpge(Vec2::splat(-0.1)).all()));
    assert!(points.iter().all(|p| p.cmple(Vec2::splat(1.1)).all()));
}

#[test]
fn create_warp_with_explicit_root_parent() {
    // Sleeve (Cloth の子) をキャンバス直下のワープに移す。基準の姿では見た目が変わらない。
    let model = fixtures::rotation_chain();
    let command = Command::CreateWarpDeformer {
        id: None,
        name: "Sleeve warp".into(),
        children: vec![mesh("Sleeve")],
        parts: Vec::new(),
        rows: 1,
        cols: 1,
        parent: Some("Arm".into()),
    };
    let after = applied(&model, command);
    let created = after.deformers.last().unwrap();
    assert_eq!(created.id.as_str(), "Sleeve_warp");
    assert_eq!(created.parent.as_ref().unwrap().as_str(), "Arm");
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn create_warp_part_is_common_ancestor_part() {
    let mut model = fixtures::single_quad();
    // Quad2 を子パーツ Child に置く。Quad (Root) と Quad2 (Child) の共通の祖先は Root。
    let mut child = model.parts[0].clone();
    child.id = "Child".into();
    child.parent = Some("Root".into());
    model.parts.push(child);
    let mut quad2 = model.meshes[0].clone();
    quad2.id = "Quad2".into();
    quad2.part = "Child".into();
    model.meshes.push(quad2);
    let after = applied(&model, warp(vec![mesh("Quad2")], 1, 1));
    assert_eq!(deformer(&after, "New").part.as_str(), "Child");
    let after = applied(&model, warp(vec![mesh("Quad2"), mesh("Quad")], 1, 1));
    assert_eq!(deformer(&after, "New").part.as_str(), "Root");
}

// ---- CreateRotationDeformer ----

/// `rotation_chain` に、`Root` の子のパーツ `Limb` (名前 `腕`) と、その子のパーツ `Fingers` を足す。
/// `Arm`・`Cloth`・`Sleeve` を `Limb` に、`Hand`・`Palm` を `Fingers` に移す。
fn chain_with_parts() -> Model {
    let mut model = fixtures::rotation_chain();
    let mut limb = model.parts[0].clone();
    limb.id = "Limb".into();
    limb.name = "腕".into();
    limb.parent = Some("Root".into());
    let mut fingers = limb.clone();
    fingers.id = "Fingers".into();
    fingers.name = "Fingers".into();
    fingers.parent = Some("Limb".into());
    model.parts.extend([limb, fingers]);
    for d in &mut model.deformers {
        d.part = if d.id == "Hand" { "Fingers" } else { "Limb" }.into();
    }
    for m in &mut model.meshes {
        m.part = if m.id == "Palm" { "Fingers" } else { "Limb" }.into();
    }
    model
}

fn wrap_parts(parts: &[&str], name: &str) -> Command {
    Command::CreateWarpDeformer {
        id: None,
        name: name.into(),
        children: Vec::new(),
        parts: parts.iter().map(|&p| p.into()).collect(),
        rows: 2,
        cols: 2,
        parent: None,
    }
}

#[test]
fn create_warp_around_part_wraps_its_outermost_elements() {
    let model = chain_with_parts();
    let after = applied(&model, wrap_parts(&["Limb"], ""));
    // 名前はパーツの名前 (日本語) から、ID はパーツの ID から作る。
    let new = after.deformers.last().unwrap();
    assert_eq!(new.id.as_str(), "WarpLimb");
    assert_eq!(new.name, "腕のワープ");
    assert_eq!(new.part.as_str(), "Limb");
    // いちばん外側の `Arm` だけを包み、その下の `Hand`・`Cloth`・メッシュはそのまま。
    assert_eq!(deformer(&after, "Arm").parent.as_ref(), Some(&new.id));
    assert_eq!(
        deformer(&after, "Hand").parent.as_ref().unwrap().as_str(),
        "Arm"
    );
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Hand"));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", 30.0)], &[("Angle", -45.0), ("Flip", 1.0)]],
    );
}

#[test]
fn create_warp_around_sub_part_skips_elements_under_outside_deformers() {
    // `Fingers` の `Hand` は `Arm` (パーツの外) の下にあるが、パーツの中ではいちばん外側。
    let mut model = chain_with_parts();
    // `Palm` を `Limb` に戻すと、`Fingers` の中は `Hand` だけになる。
    model
        .meshes
        .iter_mut()
        .find(|m| m.id == "Palm")
        .unwrap()
        .part = "Limb".into();
    let after = applied(
        &model,
        Command::CreateRotationDeformer {
            id: None,
            name: String::new(),
            children: Vec::new(),
            parts: vec!["Fingers".into()],
            origin: None,
            parent: None,
        },
    );
    let new = after.deformers.last().unwrap();
    assert_eq!(
        (new.id.as_str(), new.name.as_str()),
        ("RotationFingers", "Fingers rotation")
    );
    assert_eq!(new.parent.as_ref().unwrap().as_str(), "Arm");
    assert_eq!(deformer(&after, "Hand").parent.as_ref(), Some(&new.id));
    assert_eq!(mesh_parent(&after, "Palm").as_deref(), Some("Hand"));
    assert_same_look(&model, &after, &[REST, &[("Angle", 30.0)]]);
}

#[test]
fn create_warp_with_parts_and_children_deduplicates() {
    let model = chain_with_parts();
    // 子に `Arm` を明示し、パーツでも `Arm` が選ばれる。
    let command = Command::CreateWarpDeformer {
        id: Some("Both".into()),
        name: "Both".into(),
        children: vec![def("Arm")],
        parts: vec!["Limb".into()],
        rows: 2,
        cols: 2,
        parent: None,
    };
    let after = applied(&model, command);
    assert_eq!(
        deformer(&after, "Arm").parent.as_ref().unwrap().as_str(),
        "Both"
    );
}

#[test]
fn create_names_deformers_from_the_first_child() {
    let model = fixtures::single_quad();
    // 名前が空なら子の名前 (ASCII) + 種類、ID は種類 + 子の ID。
    let after = applied(
        &model,
        Command::CreateWarpDeformer {
            id: None,
            name: " ".into(),
            children: vec![mesh("Quad")],
            parts: Vec::new(),
            rows: 1,
            cols: 1,
            parent: None,
        },
    );
    let new = after.deformers.last().unwrap();
    assert_eq!(
        (new.id.as_str(), new.name.as_str()),
        ("WarpQuad", "Quad warp")
    );
    // 英数字を含まない名前は名前に使い、ID は子から作る。
    let mut japanese = model.clone();
    japanese.meshes[0].name = "顔".into();
    let after = applied(
        &japanese,
        Command::CreateRotationDeformer {
            id: None,
            name: "頭の回転".into(),
            children: vec![mesh("Quad")],
            parts: Vec::new(),
            origin: None,
            parent: None,
        },
    );
    let new = after.deformers.last().unwrap();
    assert_eq!(
        (new.id.as_str(), new.name.as_str()),
        ("RotationQuad", "頭の回転")
    );
    // 英数字を含む名前からは、これまでどおり名前から ID を作る。
    let after = applied(&japanese, warp_named("Face warp"));
    assert_eq!(after.deformers.last().unwrap().id.as_str(), "Face_warp");
    // 空の名前で日本語の子なら `顔のワープ`。ID が重なれば番号を付ける。
    let once = applied(&japanese, warp_named(""));
    let new = once.deformers.last().unwrap();
    assert_eq!(
        (new.id.as_str(), new.name.as_str()),
        ("WarpQuad", "顔のワープ")
    );
    // 作ったデフォーマを包むときは、種類の接頭辞を重ねない (`RotationQuad`)。
    let wrapped = applied(
        &once,
        Command::CreateRotationDeformer {
            id: None,
            name: String::new(),
            children: vec![def("WarpQuad")],
            parts: Vec::new(),
            origin: None,
            parent: None,
        },
    );
    let new = wrapped.deformers.last().unwrap();
    assert_eq!(
        (new.id.as_str(), new.name.as_str()),
        ("RotationQuad", "顔のワープの回転")
    );
    let twice = applied(&once, warp_named(""));
    // 2 回目の子は `Quad` (いまは `WarpQuad` の下) で、ID は `WarpQuad_2`。
    assert_eq!(twice.deformers.last().unwrap().id.as_str(), "WarpQuad_2");
}

fn warp_named(name: &str) -> Command {
    Command::CreateWarpDeformer {
        id: None,
        name: name.into(),
        children: vec![mesh("Quad")],
        parts: Vec::new(),
        rows: 1,
        cols: 1,
        parent: None,
    }
}

#[test]
fn create_rejects_unknown_or_empty_parts() {
    let model = chain_with_parts();
    assert_eq!(
        rejected(&model, wrap_parts(&["Nope"], "")),
        EditError::NotFound {
            kind: "part",
            id: "Nope".into()
        }
    );
    let mut empty = model.clone();
    let mut part = empty.parts[1].clone();
    part.id = "Empty".into();
    empty.parts.push(part);
    assert!(matches!(
        rejected(&empty, wrap_parts(&["Empty"], "")),
        EditError::DeformerEdit { reason } if reason.contains("no meshes or deformers")
    ));
}

#[test]
fn create_rotation_around_root_mesh_keeps_look_at_every_pose() {
    let model = fixtures::single_quad();
    let after = applied(&model, rotation(vec![mesh("Quad")], None));
    assert_same_look(&model, &after, &[REST, &[("X", -1.0)], &[("X", 0.6)]]);
    let k = rotation_keyforms(&after, "New");
    // 原点は基準の姿の外接矩形の中心。
    assert!(k[0].origin.abs_diff_eq(Vec2::new(128.0, 128.0), 1e-4));
    assert_eq!((k[0].angle, k[0].scale), (0.0, 1.0));
    // 子の座標は原点からのピクセル単位。
    let quad = &after.meshes[0].keyforms.keyforms[1].positions;
    assert!(quad[0].abs_diff_eq(Vec2::new(-64.0, -64.0), 1e-4));
}

#[test]
fn create_rotation_inside_rotation_chain_keeps_look_at_every_pose() {
    // Palm (親 Hand、拡大率 0.5、Flip で反転) を、指定した原点の回転デフォーマで包む。
    let model = fixtures::rotation_chain();
    let origin = Some(Vec2::new(200.0, 140.0));
    let after = applied(&model, rotation(vec![mesh("Palm")], origin));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Hand"
    );
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 30.0), ("Flip", 1.0)]],
    );
}

#[test]
fn create_rotation_under_warp_keeps_rest_look() {
    // Cheek (Body の子のワープ) を回転デフォーマで包む。回転デフォーマはワープの伸縮を子に伝えないので、
    // 基準の姿でだけ一致する。
    let model = fixtures::rotation_under_warp();
    let after = applied(&model, rotation(vec![def("Cheek")], None));
    assert_eq!(
        deformer(&after, "New").parent.as_ref().unwrap().as_str(),
        "Body"
    );
    assert_same_look(&model, &after, &[REST]);
}

#[test]
fn create_rotation_around_nested_rotation_keeps_look() {
    // Hand (回転) を回転で包む。Hand の角度・拡大率・反転は新しい親に対して直る。
    let mut model = fixtures::rotation_chain();
    set_default(&mut model, "Angle", 20.0);
    let after = applied(&model, rotation(vec![def("Hand")], None));
    assert_same_look(
        &model,
        &after,
        &[REST, &[("Angle", -30.0)], &[("Angle", 30.0), ("Flip", 1.0)]],
    );
}
