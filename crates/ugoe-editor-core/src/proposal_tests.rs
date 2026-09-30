//! 提案 (`crate::proposal`) のテスト。入力は `ugoe_model::fixtures::single_quad`。

use ugoe_model::fixtures;

use crate::{ChildRef, Command, Document, KeyformTarget, ProposalError, ProposalOrigin, Proposals};

fn parameter(id: &str) -> ugoe_model::Parameter {
    let mut p = fixtures::single_quad().parameters.remove(0);
    p.id = id.into();
    p.name = id.into();
    (p.min, p.default, p.max) = (-1.0, 0.0, 1.0);
    p
}

/// 0: パラメータ `Y` を足す、1: `Quad` をワープ `W` で包む、2: `W` に `Y` のキーを足す (0 と 1 を使う)、
/// 3: `Quad` の名前を変える (どれも使わない)。
fn commands() -> Vec<Command> {
    vec![
        Command::AddParameter {
            parameter: parameter("Y"),
        },
        Command::CreateWarpDeformer {
            id: Some("W".into()),
            name: "W".into(),
            children: vec![ChildRef::Mesh("Quad".into())],
            parts: Vec::new(),
            rows: 2,
            cols: 2,
            parent: None,
        },
        Command::AddKeyformKey {
            target: KeyformTarget::Deformer("W".into()),
            parameter: "Y".into(),
            key: 1.0,
        },
        Command::RenameMesh {
            id: "Quad".into(),
            name: "Renamed".into(),
        },
    ]
}

fn setup() -> (Document, Proposals, u32) {
    let doc = Document::new(fixtures::single_quad());
    let mut proposals = Proposals::new();
    let view = proposals.add(&doc, "rig", "because", commands()).unwrap();
    (doc, proposals, view.id)
}

fn has_deformer(doc: &Document, id: &str) -> bool {
    doc.model().deformers.iter().any(|d| d.id.as_str() == id)
}

fn mesh_name(doc: &Document) -> &str {
    &doc.model().meshes[0].name
}

#[test]
fn summarizes_commands_and_their_dependencies() {
    let (doc, proposals, _) = setup();
    let views = proposals.views(&doc);
    assert_eq!(views.len(), 1);
    let view = &views[0];
    assert_eq!(
        (view.title.as_str(), view.rationale.as_str()),
        ("rig", "because")
    );
    assert!(!view.stale);
    let summary: Vec<(&str, Option<&str>, &[usize])> = view
        .commands
        .iter()
        .map(|c| (c.op.as_str(), c.subject.as_deref(), c.requires.as_slice()))
        .collect();
    assert_eq!(
        summary,
        [
            ("add_parameter", Some("Y"), &[][..]),
            ("create_warp_deformer", Some("W"), &[][..]),
            ("add_keyform_key", Some("deformer `W`"), &[0, 1][..]),
            ("rename_mesh", Some("Quad"), &[][..]),
        ]
    );
}

#[test]
fn invalid_proposals_are_not_kept() {
    let doc = Document::new(fixtures::single_quad());
    let mut proposals = Proposals::new();
    let err = proposals
        .add(&doc, "bad", "", commands()[2..].to_vec())
        .unwrap_err();
    assert!(matches!(err, ProposalError::Rejected(_)), "{err}");
    assert!(proposals.is_empty());
    assert!(matches!(
        proposals.add(&doc, "empty", "", Vec::new()),
        Err(ProposalError::NothingSelected)
    ));
}

#[test]
fn preview_leaves_the_document_unchanged() {
    let (doc, proposals, id) = setup();
    let (preview, _) = proposals.preview(&doc, id, &[]).unwrap();
    assert!(has_deformer(&preview, "W"));
    assert_eq!(mesh_name(&preview), "Renamed");
    assert!(!has_deformer(&doc, "W"));
    assert_eq!(mesh_name(&doc), "Quad");
    assert!(!doc.is_modified());
    // プレビューしても提案は残る。
    assert_eq!(proposals.views(&doc).len(), 1);
}

#[test]
fn adopting_is_one_undo_step() {
    let (mut doc, mut proposals, id) = setup();
    proposals.adopt(&mut doc, id, &[]).unwrap();
    assert!(has_deformer(&doc, "W"));
    assert_eq!(mesh_name(&doc), "Renamed");
    assert!(proposals.views(&doc).is_empty());
    assert!(doc.undo());
    assert!(!has_deformer(&doc, "W"));
    assert_eq!(mesh_name(&doc), "Quad");
    assert!(doc.model().parameters.iter().all(|p| p.id.as_str() != "Y"));
    assert!(!doc.undo());
    // Undo で提案を作った状態に戻ると、採用した提案が再び出て、選び直して採用し直せる。
    let views = proposals.views(&doc);
    assert_eq!(views.len(), 1);
    assert!(!views[0].stale);
    proposals.adopt(&mut doc, id, &[3]).unwrap();
    assert_eq!(mesh_name(&doc), "Quad");
    // 採用の後に別の編集をすると、Undo で採用の直後に戻っても出ない。
    doc.apply(Command::RenameMesh {
        id: "Quad".into(),
        name: "Edited".into(),
    })
    .unwrap();
    assert!(doc.undo());
    assert!(proposals.views(&doc).is_empty());
}

#[test]
fn excluded_commands_are_left_out() {
    let (mut doc, mut proposals, id) = setup();
    let (preview, _) = proposals.preview(&doc, id, &[3]).unwrap();
    assert_eq!(mesh_name(&preview), "Quad");
    proposals.adopt(&mut doc, id, &[3]).unwrap();
    assert!(has_deformer(&doc, "W"));
    assert_eq!(mesh_name(&doc), "Quad");
}

/// 使う要素を作るコマンドを外すと、採用もプレビューもせず、ドキュメントと提案はそのまま。
#[test]
fn excluding_a_requirement_is_rejected() {
    let (mut doc, mut proposals, id) = setup();
    let err = proposals.adopt(&mut doc, id, &[0]).unwrap_err();
    assert!(err.to_string().contains("`Y` not found"), "{err}");
    assert!(proposals.preview(&doc, id, &[1]).is_err());
    assert!(!doc.is_modified());
    assert_eq!(proposals.views(&doc).len(), 1);
    // 依存ごと外せば採用できる。
    proposals.adopt(&mut doc, id, &[0, 1, 2]).unwrap();
    assert_eq!(mesh_name(&doc), "Renamed");
}

#[test]
fn selection_errors() {
    let (mut doc, mut proposals, id) = setup();
    assert!(matches!(
        proposals.preview(&doc, id, &[0, 1, 2, 3]),
        Err(ProposalError::NothingSelected)
    ));
    assert!(matches!(
        proposals.preview(&doc, id, &[4]),
        Err(ProposalError::InvalidIndex { index: 4 })
    ));
    assert!(matches!(
        proposals.adopt(&mut doc, id + 1, &[]),
        Err(ProposalError::NotFound(_))
    ));
    proposals.reject(id).unwrap();
    assert!(proposals.is_empty());
    assert!(matches!(
        proposals.reject(id),
        Err(ProposalError::NotFound(_))
    ));
}

/// 提案の後にドキュメントが変わると使えない。Undo で同じ状態に戻れば、また使える。
#[test]
fn proposals_go_stale_when_the_document_changes() {
    let (mut doc, mut proposals, id) = setup();
    doc.apply(Command::RenameMesh {
        id: "Quad".into(),
        name: "Edited".into(),
    })
    .unwrap();
    assert!(proposals.views(&doc)[0].stale);
    assert!(matches!(
        proposals.preview(&doc, id, &[]),
        Err(ProposalError::Stale)
    ));
    assert!(matches!(
        proposals.adopt(&mut doc, id, &[]),
        Err(ProposalError::Stale)
    ));
    assert!(doc.undo());
    assert!(!proposals.views(&doc)[0].stale);
    proposals.adopt(&mut doc, id, &[]).unwrap();
    assert_eq!(mesh_name(&doc), "Renamed");
}

/// 一部だけの採用も 1 回の Undo で戻る。
#[test]
fn partial_adoption_keeps_the_rest_as_one_step() {
    let (mut doc, mut proposals, id) = setup();
    proposals.adopt(&mut doc, id, &[3]).unwrap();
    assert!(doc.undo());
    assert!(!has_deformer(&doc, "W"));
    assert!(!doc.is_modified());
}

#[test]
fn detached_copies_have_no_history() {
    let (doc, ..) = setup();
    let mut copy = doc.detached();
    assert!(!copy.undo());
    assert!(copy.is_modified());
    assert_eq!(copy.state_id(), doc.state_id());
}

/// 同じ要素への後の編集は、前の編集に依存する (前の結果に頼ることがあるため)。
#[test]
fn later_edits_of_the_same_element_depend_on_earlier_ones() {
    let doc = Document::new(fixtures::single_quad());
    let mut proposals = Proposals::new();
    let rename = |name: &str| Command::RenameMesh {
        id: "Quad".into(),
        name: name.into(),
    };
    let view = proposals
        .add(&doc, "names", "", vec![rename("A"), rename("B")])
        .unwrap();
    let requires: Vec<&[usize]> = view
        .commands
        .iter()
        .map(|c| c.requires.as_slice())
        .collect();
    assert_eq!(requires, [&[][..], &[0][..]]);
}

/// 提案を作ったもの (エディタにつないだエージェント) を記すと一覧に出る。無い提案には記せない。
#[test]
fn records_where_a_proposal_came_from() {
    let (doc, mut proposals, id) = setup();
    assert_eq!(proposals.views(&doc)[0].source, None);
    assert!(proposals.set_source(id, "claude-code"));
    assert_eq!(
        proposals.views(&doc)[0].source.as_deref(),
        Some("claude-code")
    );
    assert!(!proposals.set_source(id + 1, "other"));
}

/// 作り直した提案は元の提案を置き換えずに並べ、元・注文・反映できなかった注文を一覧に出す。
#[test]
fn a_revision_is_listed_beside_the_original() {
    let (doc, mut proposals, id) = setup();
    let first = &proposals.views(&doc)[0];
    assert_eq!((first.revises, first.instructions.as_deref()), (None, None));
    assert!(first.unapplied.is_empty());
    let revision = proposals
        .add(&doc, "revised", "why", commands())
        .unwrap()
        .id;
    let origin = ProposalOrigin {
        revises: Some(id),
        instructions: Some("目は細く".into()),
        unapplied: vec!["hair sway".into()],
    };
    assert!(proposals.set_origin(revision, origin.clone()));
    let views = proposals.views(&doc);
    assert_eq!(views.len(), 2);
    assert_eq!(views[0].id, id);
    assert_eq!(views[1].revises, Some(id));
    assert_eq!(views[1].instructions.as_deref(), Some("目は細く"));
    assert_eq!(views[1].unapplied, ["hair sway"]);
    assert!(!proposals.set_origin(revision + 1, origin));
}
