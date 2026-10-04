//! `Batch` と、適用の前後の検証・履歴のテスト。

use super::*;

#[test]
fn batch_is_validated_as_a_whole() {
    let model = fixtures::single_quad();
    // 途中 (パーツより先にメッシュを追加) は不正でも、最後に整合していればよい。
    let mut mesh = new_mesh("Hat");
    mesh.part = "Head".into();
    let after = applied(
        &model,
        Command::Batch {
            label: "add hat".into(),
            commands: vec![
                Command::AddMesh { mesh },
                Command::AddPart {
                    part: new_part("Head", Some("Root")),
                },
            ],
        },
    );
    assert_eq!(mesh_ids(&after), ["Quad", "Hat"]);
}

#[test]
fn failing_batch_rolls_back_everything() {
    let model = fixtures::single_quad();
    let err = rejected(
        &model,
        Command::Batch {
            label: "partial".into(),
            commands: vec![
                Command::RenameMesh {
                    id: "Quad".into(),
                    name: "Renamed".into(),
                },
                Command::RemoveMesh { id: "Nope".into() },
            ],
        },
    );
    assert_eq!(err, not_found("mesh", "Nope"));
}

#[test]
fn invalid_document_accepts_commands_that_add_no_new_errors() {
    // 読み込んだ時点で不正なモデルでも、新たなエラーを増やさない編集はできる。
    let mut model = fixtures::single_quad();
    model.meshes[0].texture = 9;
    let mut doc = Document::new(model.clone());
    doc.apply(Command::RenameMesh {
        id: "Quad".into(),
        name: "Still broken".into(),
    })
    .unwrap();
    // 既存のエラーを直す編集もできる。
    doc.apply(Command::SetMeshProperties {
        id: "Quad".into(),
        part: None,
        texture: Some(0),
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
        lit: None,
    })
    .unwrap();
    assert!(validate(doc.model()).is_empty());
}

#[test]
fn new_command_clears_redo() {
    let model = fixtures::single_quad();
    let mut doc = Document::new(model);
    let rename = |name: &str| Command::RenameMesh {
        id: "Quad".into(),
        name: name.into(),
    };
    doc.apply(rename("A")).unwrap();
    assert!(doc.undo());
    doc.apply(rename("B")).unwrap();
    assert!(!doc.redo());
    assert_eq!(doc.model().meshes[0].name, "B");
}
