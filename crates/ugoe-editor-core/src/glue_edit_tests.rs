//! グルーの編集 (`CreateGlue`・`SetGlue`・`RemoveGlue`) のテスト。規則は `docs/editor.md#グルーの編集`。

use ugoe_model::mesh::{Glue, GlueKeyform, GluePair};
use ugoe_model::validate::validate;
use ugoe_model::{KeyformGrid, Model, fixtures};

use crate::{Command, Document, EditError};

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

#[track_caller]
fn rejected(model: &Model, command: Command) -> EditError {
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// `glued_quads` のグルーを外したもの (`Left` と `Right` は 10 ピクセル離れている)。
fn unglued() -> Model {
    let mut model = fixtures::glued_quads();
    model.glues.clear();
    model
}

fn create(distance: f32) -> Command {
    Command::CreateGlue {
        id: None,
        mesh_a: "Left".into(),
        mesh_b: "Right".into(),
        distance,
    }
}

fn pair(a: u16, b: u16, weight: f32) -> GluePair {
    GluePair { a, b, weight }
}

#[test]
fn create_pairs_vertices_within_the_distance() {
    let after = applied(&unglued(), create(15.0));
    let glue = &after.glues[0];
    assert_eq!(glue.id.as_str(), "Glue_Left_Right");
    // Right の頂点 0 (110,0) と 3 (110,100) が、Left の頂点 1 (100,0) と 2 (100,100) につながる。
    assert_eq!(glue.pairs, vec![pair(1, 0, 0.5), pair(2, 3, 0.5)]);
    assert_eq!(
        glue.keyforms,
        KeyformGrid::constant(GlueKeyform { intensity: 1.0 })
    );

    // 同じ両端でもう 1 つ作ると、ID は重ならない。
    let twice = applied(&after, create(15.0));
    assert_eq!(twice.glues[1].id.as_str(), "Glue_Left_Right_2");
}

#[test]
fn create_rejects_bad_input() {
    let model = unglued();
    // 10 ピクセルより近い頂点は無い。
    assert!(matches!(
        rejected(&model, create(5.0)),
        EditError::InvalidGlue { .. }
    ));
    let same = Command::CreateGlue {
        id: None,
        mesh_a: "Left".into(),
        mesh_b: "Left".into(),
        distance: 15.0,
    };
    assert!(matches!(
        rejected(&model, same),
        EditError::InvalidGlue { .. }
    ));
    let missing = Command::CreateGlue {
        id: None,
        mesh_a: "Left".into(),
        mesh_b: "Nope".into(),
        distance: 15.0,
    };
    assert!(matches!(
        rejected(&model, missing),
        EditError::NotFound { kind: "mesh", .. }
    ));
    assert!(matches!(
        rejected(&model, create(f32::NAN)),
        EditError::InvalidGlue { .. }
    ));
    let taken = Command::CreateGlue {
        id: Some("Seam".into()),
        mesh_a: "Left".into(),
        mesh_b: "Right".into(),
        distance: 15.0,
    };
    assert!(matches!(
        rejected(&fixtures::glued_quads(), taken),
        EditError::DuplicateId { kind: "glue", .. }
    ));
}

#[test]
fn create_names_module_meshes_without_the_prefix() {
    let mut model = unglued();
    for mesh in &mut model.meshes {
        mesh.id = ugoe_model::MeshId::new(format!("Hat/{}", mesh.id));
    }
    let after = applied(
        &model,
        Command::CreateGlue {
            id: None,
            mesh_a: "Hat/Left".into(),
            mesh_b: "Hat/Right".into(),
            distance: 15.0,
        },
    );
    assert_eq!(after.glues[0].id.as_str(), "Glue_Left_Right");
}

#[test]
fn set_replaces_the_glue_and_can_pair_again() {
    let model = fixtures::glued_quads();
    let mut pull = model.glues[1].clone();
    pull.pairs[0].weight = 0.25;
    pull.keyforms = KeyformGrid::constant(GlueKeyform { intensity: 0.75 });
    let after = applied(
        &model,
        Command::SetGlue {
            glue: pull.clone(),
            distance: None,
        },
    );
    assert_eq!(after.glues[1], pull);
    assert_eq!(after.glues[0], model.glues[0]);

    // 距離を渡すと組を作り直す (重みは先頭の組の重み)。
    let after = applied(
        &model,
        Command::SetGlue {
            glue: pull,
            distance: Some(15.0),
        },
    );
    assert_eq!(
        after.glues[1].pairs,
        vec![pair(1, 0, 0.25), pair(2, 3, 0.25)]
    );
    assert_eq!(after.glues[1].keyforms.keyforms[0].intensity, 0.75);
}

#[test]
fn set_rejects_unknown_glues_and_bad_pairs() {
    let model = fixtures::glued_quads();
    let unknown = Glue {
        id: "Nope".into(),
        ..model.glues[0].clone()
    };
    assert!(matches!(
        rejected(
            &model,
            Command::SetGlue {
                glue: unknown,
                distance: None
            }
        ),
        EditError::NotFound { kind: "glue", .. }
    ));
    let mut out_of_range = model.glues[0].clone();
    out_of_range.pairs.push(pair(9, 0, 0.5));
    assert!(matches!(
        rejected(
            &model,
            Command::SetGlue {
                glue: out_of_range,
                distance: None
            }
        ),
        EditError::Invalid(_)
    ));
    let mut infinite = model.glues[0].clone();
    infinite.pairs[0].weight = f32::INFINITY;
    assert!(matches!(
        rejected(
            &model,
            Command::SetGlue {
                glue: infinite,
                distance: None
            }
        ),
        EditError::InvalidGlue { .. }
    ));
}

#[test]
fn remove_deletes_the_glue() {
    let model = fixtures::glued_quads();
    let after = applied(&model, Command::RemoveGlue { id: "Seam".into() });
    assert_eq!(after.glues.len(), 1);
    assert_eq!(after.glues[0].id.as_str(), "Pull");
    assert!(matches!(
        rejected(&model, Command::RemoveGlue { id: "Nope".into() }),
        EditError::NotFound { kind: "glue", .. }
    ));
}
