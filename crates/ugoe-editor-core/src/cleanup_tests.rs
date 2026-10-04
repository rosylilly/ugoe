//! 削除の後始末 (`crate::cleanup`) のテスト。規則は `docs/editor.md#削除の連鎖と拒否`。

use ugoe_model::deformer::DeformerKind;
use ugoe_model::validate::validate;
use ugoe_model::{KeyformOrigin, MirrorSource, Model, fixtures};

use crate::{Command, Document};

fn mirror_source(id: &str) -> Option<MirrorSource> {
    Some(MirrorSource {
        id: id.into(),
        axis: None,
        tolerance: None,
    })
}

/// 適用に成功し、結果が検証を通り、Undo で元に戻ることを確かめて、適用後のモデルを返す。
#[track_caller]
fn applied(model: &Model, command: Command) -> Model {
    let mut doc = Document::new(model.clone());
    doc.apply(command).unwrap();
    let after = doc.model().clone();
    assert_eq!(validate(&after), vec![]);
    assert!(doc.undo());
    assert_eq!(doc.model(), model);
    after
}

// ブレンドシェイプ格子のミラー元も、要素を消すと外す (セルは値を保って `authored` になる)。
#[test]
fn removing_an_element_forgets_it_as_a_blend_shape_mirror_source() {
    let mut model = fixtures::blend_shapes();
    let shape = &mut model.meshes[0].keyforms.blend_shapes[0];
    shape.mirror_source = mirror_source("Cheek");
    shape.origins = vec![KeyformOrigin::Authored, KeyformOrigin::Mirror];
    let values = shape.keyforms.clone();
    assert!(validate(&model).is_empty());

    let after = applied(&model, Command::RemoveMesh { id: "Cheek".into() });
    let shape = &after.meshes[0].keyforms.blend_shapes[0];
    assert_eq!(shape.mirror_source, None);
    assert!(shape.origins.iter().all(|o| *o == KeyformOrigin::Authored));
    assert_eq!(shape.keyforms, values);
}

// デフォーマで包んだ子を外す (`unwrap_deformer`) ときも、ブレンドシェイプ格子のミラー元から外す。
#[test]
fn unwrapping_a_deformer_forgets_it_as_a_blend_shape_mirror_source() {
    let mut model = fixtures::blend_shapes();
    let DeformerKind::Warp(face) = &mut model.deformers[0].kind else {
        panic!("Face is a warp");
    };
    face.keyforms.blend_shapes[0].mirror_source = mirror_source("Jaw");
    assert!(validate(&model).is_empty());

    let after = applied(&model, Command::UnwrapDeformer { id: "Jaw".into() });
    let DeformerKind::Warp(face) = &after.deformers[0].kind else {
        panic!("Face is a warp");
    };
    assert_eq!(face.keyforms.blend_shapes[0].mirror_source, None);
}

// 外すのは適用で消えた要素への参照だけ。読み込んだ時点で壊れていた参照は変えない。
#[test]
fn references_broken_before_the_edit_are_kept() {
    let mut model = fixtures::blend_shapes();
    model.meshes[1].keyforms.mirror_source = mirror_source("Ghost");
    model.meshes[1].mirror = Some("Ghost".into());
    assert!(!validate(&model).is_empty());

    let mut doc = Document::new(model.clone());
    doc.apply(Command::RemoveMesh { id: "Chin".into() })
        .unwrap();
    let cheek = &doc.model().meshes[1];
    assert_eq!(cheek.keyforms.mirror_source, mirror_source("Ghost"));
    assert_eq!(cheek.mirror, Some("Ghost".into()));
}

// 後始末はコマンド全体の前後を比べる。`batch` の中で消して同じ ID で足し直した要素は消えていないので、
// その要素への参照は残る。
#[test]
fn an_element_removed_and_added_back_in_a_batch_keeps_its_references() {
    let mut model = fixtures::blend_shapes();
    model.meshes[0].masks = vec![ugoe_model::mesh::MaskRef::normal("Cheek")];
    let cheek = model.meshes[1].clone();
    let after = applied(
        &model,
        Command::Batch {
            label: "replace".into(),
            commands: vec![
                Command::RemoveMesh { id: "Cheek".into() },
                Command::AddMesh { mesh: cheek },
            ],
        },
    );
    assert_eq!(after.meshes[0].masks, model.meshes[0].masks);

    let after = applied(&model, Command::RemoveMesh { id: "Cheek".into() });
    assert!(after.meshes[0].masks.is_empty());
}
