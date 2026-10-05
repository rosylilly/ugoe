//! 分類の直し (`classification.rs`・`Command::SetLayerCorrection`) のテスト。

use ugoe_model::fixtures;

use crate::classification::MAX_LABEL_LEN;
use crate::message::Coded;
use crate::{
    Command, Document, EditError, LayerCorrection, LayerCorrections, LayerSide, Project,
    ProjectMeta,
};

fn path(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn correction(label: &str, side: LayerSide) -> LayerCorrection {
    LayerCorrection {
        label: label.to_owned(),
        side,
    }
}

fn set(path: Vec<String>, correction: Option<LayerCorrection>) -> Command {
    Command::SetLayerCorrection { path, correction }
}

#[test]
fn setting_a_correction_is_one_undo_step() {
    let mut doc = Document::new(fixtures::single_quad());
    let eye = path(&["Face", "Eye L"]);
    let model = doc.model().clone();

    doc.apply(set(
        eye.clone(),
        Some(correction("eye_white", LayerSide::Left)),
    ))
    .unwrap();
    assert_eq!(
        doc.layer_corrections().get(&eye),
        Some(&correction("eye_white", LayerSide::Left))
    );
    assert!(doc.is_modified());
    assert_eq!(doc.model(), &model, "the model does not change");

    // 直し直しと消すのも 1 回ずつの Undo。
    doc.apply(set(eye.clone(), Some(correction("iris", LayerSide::Right))))
        .unwrap();
    doc.apply(set(eye.clone(), None)).unwrap();
    assert!(doc.layer_corrections().is_empty());

    assert!(doc.undo());
    assert_eq!(
        doc.layer_corrections().get(&eye),
        Some(&correction("iris", LayerSide::Right))
    );
    assert!(doc.undo());
    assert_eq!(
        doc.layer_corrections().get(&eye),
        Some(&correction("eye_white", LayerSide::Left))
    );
    assert!(doc.undo());
    assert!(doc.layer_corrections().is_empty());
    assert!(!doc.is_modified());
    assert!(doc.redo());
    assert_eq!(doc.layer_corrections().len(), 1);
}

#[test]
fn the_same_correction_adds_no_undo_step() {
    let mut doc = Document::new(fixtures::single_quad());
    let eye = path(&["Eye L"]);
    doc.apply(set(eye.clone(), Some(correction("eye", LayerSide::Left))))
        .unwrap();
    let state = doc.state_id();
    doc.apply(set(eye, Some(correction("eye", LayerSide::Left))))
        .unwrap();
    // ない直しを消すのも何もしない。
    doc.apply(set(path(&["Other"]), None)).unwrap();
    assert_eq!(doc.state_id(), state);
    assert_eq!(doc.undo_count(), 1);
}

#[test]
fn edits_keep_the_corrections() {
    let mut doc = Document::new(fixtures::single_quad());
    let eye = path(&["Eye L"]);
    doc.apply(set(eye.clone(), Some(correction("eye", LayerSide::Left))))
        .unwrap();
    doc.apply(Command::SetLighting { enabled: true }).unwrap();
    assert_eq!(doc.layer_corrections().len(), 1);
    // モデルの編集の Undo は直しを戻さない。
    assert!(doc.undo());
    assert_eq!(doc.layer_corrections().len(), 1);
    assert!(doc.detached().layer_corrections().get(&eye).is_some());
}

#[test]
fn malformed_corrections_are_rejected() {
    let mut doc = Document::new(fixtures::single_quad());
    let cases = [
        set(Vec::new(), Some(correction("eye", LayerSide::Left))),
        set(path(&["Eye"]), Some(correction("", LayerSide::Left))),
        set(path(&["Eye"]), Some(correction("Eye", LayerSide::Left))),
        set(
            path(&["Eye"]),
            Some(correction("eye white", LayerSide::Left)),
        ),
        set(
            path(&["Eye"]),
            Some(correction(&"a".repeat(MAX_LABEL_LEN + 1), LayerSide::Left)),
        ),
        set(vec!["x".to_owned(); 65], None),
        set(vec!["x".repeat(1025)], None),
    ];
    for command in cases {
        let error = doc.apply(command.clone()).unwrap_err();
        assert!(
            matches!(error, EditError::InvalidLayerCorrection { .. }),
            "{command:?}: {error:?}"
        );
        assert_eq!(error.code(), "edit.invalid_layer_correction");
    }
    assert!(doc.layer_corrections().is_empty());
    assert!(!doc.can_undo());
}

#[test]
fn corrections_are_not_allowed_in_a_batch() {
    let mut doc = Document::new(fixtures::single_quad());
    let error = doc
        .apply(Command::Batch {
            label: "batch".into(),
            commands: vec![set(
                path(&["Eye"]),
                Some(correction("eye", LayerSide::Left)),
            )],
        })
        .unwrap_err();
    assert!(matches!(error, EditError::InvalidLayerCorrection { .. }));
    assert!(doc.layer_corrections().is_empty());
}

#[test]
fn corrections_round_trip_through_the_project_file() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("character.ugoeproj");
    let project = Project::empty("character");
    let mut doc = project.document();
    doc.apply(set(
        path(&["Face", "Eye L"]),
        Some(correction("eye_white", LayerSide::Left)),
    ))
    .unwrap();
    doc.apply(set(
        path(&["Mouth"]),
        Some(correction("mouth", LayerSide::Center)),
    ))
    .unwrap();
    Project::from_document(project.meta.clone(), &doc)
        .save(&dir)
        .unwrap();

    let text = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        json["editor"]["classification"],
        serde_json::json!({ "corrections": [
            { "path": ["Face", "Eye L"], "label": "eye_white", "side": "left" },
            { "path": ["Mouth"], "label": "mouth", "side": "center" },
        ] })
    );

    let loaded = Project::load(&dir).unwrap();
    let reopened = loaded.document();
    assert_eq!(reopened.layer_corrections(), doc.layer_corrections());
    assert!(!reopened.is_modified());

    // 直しをすべて消して保存すると、項目も消える。
    let mut doc = reopened;
    doc.apply(set(path(&["Face", "Eye L"]), None)).unwrap();
    doc.apply(set(path(&["Mouth"]), None)).unwrap();
    Project::from_document(loaded.meta.clone(), &doc)
        .save(&dir)
        .unwrap();
    let text = std::fs::read_to_string(dir.join("project.json")).unwrap();
    assert!(!text.contains("classification"), "{text}");
}

#[test]
fn malformed_stored_corrections_are_dropped() {
    let mut meta = ProjectMeta::new("x");
    meta.editor.insert(
        "classification".into(),
        serde_json::json!({ "corrections": [
            { "path": ["Eye"], "label": "eye", "side": "left" },
            { "path": ["Eye"], "label": "iris", "side": "right" },
            { "path": [], "label": "eye", "side": "left" },
            { "path": ["Hair"], "label": "Hair!", "side": "left" },
            { "path": ["Nose"], "label": "nose", "side": "up" },
            { "path": "Mouth", "label": "mouth", "side": "center" },
            42,
        ] }),
    );
    let corrections = meta.layer_corrections();
    assert_eq!(corrections.len(), 1);
    assert_eq!(
        corrections.get(&path(&["Eye"])),
        Some(&correction("eye", LayerSide::Left))
    );

    for value in [
        serde_json::json!(1),
        serde_json::json!({ "corrections": {} }),
    ] {
        meta.editor.insert("classification".into(), value);
        assert_eq!(meta.layer_corrections(), LayerCorrections::default());
    }
}
