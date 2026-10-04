//! デフォーマの作成・見た目を保つ付け替え・セルの編集のテスト。式は `docs/editor.md#デフォーマの編集`。
//!
//! 見た目の比較は、ランタイムで評価したメッシュのキャンバス座標を、適用の前後でいくつかの
//! パラメータ値について比べる。

use ugoe_core::Vec2;
use ugoe_model::deformer::{DeformerKind, RotationKeyform};
use ugoe_model::validate::validate;
use ugoe_model::{Deformer, KeyformOrigin, MirrorSource, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::{ChildRef, Command, Document, EditError};

const TOL: f32 = 2e-3;

// ---- 補助 ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
fn applied(model: &Model, command: Command) -> Model {
    assert!(validate(model).is_empty(), "fixture must be valid");
    let mut doc = Document::new(model.clone());
    doc.apply(command).unwrap();
    let after = doc.model().clone();
    assert_eq!(validate(&after), vec![]);
    assert!(doc.undo());
    assert_eq!(doc.model(), model);
    assert!(doc.redo());
    assert_eq!(doc.model(), &after);
    after
}

/// 適用に失敗し、モデルと履歴が変わらないことを確かめて、エラーを返す。
#[track_caller]
fn rejected(model: &Model, command: Command) -> EditError {
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// パラメータ値 (`(ID, 値)`、ほかは既定値) で評価した全メッシュのキャンバス座標。
fn evaluate(model: &Model, values: &[(&str, f32)]) -> Vec<Vec<Vec2>> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for &(id, value) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, value);
    }
    instance.update();
    instance
        .drawables()
        .iter()
        .map(|d| d.positions.clone())
        .collect()
}

/// 各パラメータ値で、前後の見た目 (全メッシュの頂点) が許容誤差内で一致することを確かめる。
#[track_caller]
fn assert_same_look(before: &Model, after: &Model, poses: &[&[(&str, f32)]]) {
    for pose in poses {
        let (a, b) = (evaluate(before, pose), evaluate(after, pose));
        assert_eq!(a.len(), b.len());
        for (i, (ma, mb)) in a.iter().zip(&b).enumerate() {
            for (p, q) in ma.iter().zip(mb) {
                assert!(
                    p.abs_diff_eq(*q, TOL),
                    "pose {pose:?}, mesh {}: {p} != {q}",
                    before.meshes[i].id
                );
            }
        }
    }
}

const REST: &[(&str, f32)] = &[];

fn deformer<'a>(model: &'a Model, id: &str) -> &'a Deformer {
    model
        .deformers
        .iter()
        .find(|d| d.id.as_str() == id)
        .unwrap()
}

fn mesh_parent(model: &Model, id: &str) -> Option<String> {
    model
        .meshes
        .iter()
        .find(|m| m.id.as_str() == id)
        .unwrap()
        .parent
        .as_ref()
        .map(ToString::to_string)
}

fn mesh(id: &str) -> ChildRef {
    ChildRef::Mesh(id.into())
}

fn def(id: &str) -> ChildRef {
    ChildRef::Deformer(id.into())
}

fn warp(children: Vec<ChildRef>, rows: u16, cols: u16) -> Command {
    Command::CreateWarpDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children,
        parts: Vec::new(),
        rows,
        cols,
        parent: None,
    }
}

fn rotation(children: Vec<ChildRef>, origin: Option<Vec2>) -> Command {
    Command::CreateRotationDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children,
        parts: Vec::new(),
        origin,
        parent: None,
    }
}

fn reparent(element: ChildRef, new_parent: Option<&str>) -> Command {
    Command::ReparentPreservingLook {
        element,
        new_parent: new_parent.map(Into::into),
    }
}

fn set_default(model: &mut Model, id: &str, value: f32) {
    let p = model
        .parameters
        .iter_mut()
        .find(|p| p.id.as_str() == id)
        .unwrap();
    p.default = value;
}

fn warp_points(model: &Model, id: &str) -> Vec<Vec2> {
    match &deformer(model, id).kind {
        DeformerKind::Warp(w) => w.keyforms.keyforms[0].points.clone(),
        _ => panic!("not a warp"),
    }
}

fn rotation_keyforms(model: &Model, id: &str) -> Vec<RotationKeyform> {
    match &deformer(model, id).kind {
        DeformerKind::Rotation(r) => r.keyforms.keyforms.clone(),
        _ => panic!("not a rotation deformer"),
    }
}

// ---- CreateWarpDeformer ----

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
        d.part = if d.id.as_str() == "Hand" {
            "Fingers"
        } else {
            "Limb"
        }
        .into();
    }
    for m in &mut model.meshes {
        m.part = if m.id.as_str() == "Palm" {
            "Fingers"
        } else {
            "Limb"
        }
        .into();
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
        .find(|m| m.id.as_str() == "Palm")
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

// ---- ReparentPreservingLook ----

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

// ---- 拒否 ----

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

// ---- MoveRotationOrigin ----

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

// ---- UnwrapDeformer ----

fn unwrap(id: &str) -> Command {
    Command::UnwrapDeformer { id: id.into() }
}

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
