//! `Document::apply` と各コマンドのテスト。
//!
//! 入力は `ugoe_model::fixtures` のモデルを使う。新しい要素の値も、構造体リテラルではなく
//! フィクスチャの要素を複製して作る (モデルの型にフィールドが増えても直す箇所を減らすため)。

use ugoe_model::deformer::DeformerKind;
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{
    BlendMode, Deformer, HitArea, KeyformBinding, KeyformGrid, MaskRef, Mesh, Model, Parameter,
    ParameterGroup, Part, fixtures,
};

use crate::{CellError, Command, Document, EditError, KeyformCell};

// ---- 要素を作る補助 ----

fn new_parameter(id: &str, min: f32, default: f32, max: f32) -> Parameter {
    let mut p = fixtures::single_quad().parameters.remove(0);
    p.id = id.into();
    p.name = id.into();
    (p.min, p.default, p.max) = (min, default, max);
    p
}

fn new_part(id: &str, parent: Option<&str>) -> Part {
    let mut p = fixtures::single_quad().parts.remove(0);
    p.id = id.into();
    p.name = id.into();
    p.parent = parent.map(Into::into);
    p
}

/// パラメータ `X` で左右に動く四角形 (`single_quad` の `Quad`)。パーツ `Root`、親なし。
fn new_mesh(id: &str) -> Mesh {
    let mut m = fixtures::single_quad().meshes.remove(0);
    m.id = id.into();
    m.name = id.into();
    m
}

/// 1×1 セルの定数ワープ (`rotation_chain` の `Cloth`)。パーツ `Root`。
fn new_warp(id: &str, parent: Option<&str>) -> Deformer {
    let mut d = rotation_chain_deformer("Cloth");
    d.id = id.into();
    d.name = id.into();
    d.parent = parent.map(Into::into);
    d
}

fn rotation_chain_deformer(id: &str) -> Deformer {
    fixtures::rotation_chain()
        .deformers
        .into_iter()
        .find(|d| d.id.as_str() == id)
        .unwrap()
}

pub(crate) fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

/// 1 軸の格子。
fn grid1<T: Clone>(parameter: &str, keys: &[f32], keyforms: Vec<T>) -> KeyformGrid<T> {
    let mut grid = KeyformGrid::constant(keyforms[0].clone());
    grid.bindings = vec![binding(parameter, keys)];
    grid.keyforms = keyforms;
    grid
}

pub(crate) fn cell(entries: &[(&str, f32)]) -> KeyformCell {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

fn mesh_ids(model: &Model) -> Vec<&str> {
    model.meshes.iter().map(|m| m.id.as_str()).collect()
}

// ---- 検査の補助 ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
pub(crate) fn applied(model: &Model, command: Command) -> Model {
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
pub(crate) fn rejected(model: &Model, command: Command) -> EditError {
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// 検証エラーで拒否され、その中に `pred` を満たすものがあること。
#[track_caller]
fn rejected_invalid(model: &Model, command: Command, pred: impl Fn(&ValidationError) -> bool) {
    match rejected(model, command) {
        EditError::Invalid(errors) => assert!(errors.iter().any(pred), "{errors:?}"),
        other => panic!("expected validation error, got {other:?}"),
    }
}

fn not_found(kind: &'static str, id: &str) -> EditError {
    EditError::NotFound {
        kind,
        id: id.into(),
    }
}

// ---- パラメータ ----

#[test]
fn add_parameter() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddParameter {
            parameter: new_parameter("Y", -1.0, 0.0, 1.0),
        },
    );
    assert_eq!(after.parameters.len(), 2);
    assert_eq!(
        rejected(
            &model,
            Command::AddParameter {
                parameter: new_parameter("X", -1.0, 0.0, 1.0),
            },
        ),
        EditError::DuplicateId {
            kind: "parameter",
            id: "X".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddParameter {
            parameter: new_parameter("Y", 1.0, 0.0, -1.0),
        },
        |e| matches!(e, ValidationError::ParameterRange { .. }),
    );
}

#[test]
fn remove_parameter_collapses_axis_at_default_key() {
    let model = fixtures::single_quad();
    let after = applied(&model, Command::RemoveParameter { id: "X".into() });
    assert!(after.parameters.is_empty());
    let quad = &after.meshes[0].keyforms;
    assert!(quad.bindings.is_empty());
    // 既定値 0 のキー (中央) の断面が残る。
    assert_eq!(
        quad.keyforms,
        vec![model.meshes[0].keyforms.keyforms[1].clone()]
    );
}

#[test]
fn remove_parameter_collapses_axis_at_nearest_key() {
    let mut model = fixtures::rotation_chain();
    // `Arm` の軸 `Angle` のキーは -30 と 30。既定値 10 はキーでないので、近い 30 の断面が残る。
    model.parameters[0].default = 10.0;
    let after = applied(&model, Command::RemoveParameter { id: "Angle".into() });
    let DeformerKind::Rotation(arm) = &after.deformers[0].kind else {
        panic!("Arm is a rotation deformer");
    };
    let DeformerKind::Rotation(before) = &model.deformers[0].kind else {
        panic!("Arm is a rotation deformer");
    };
    assert!(arm.keyforms.bindings.is_empty());
    assert_eq!(arm.keyforms.keyforms, vec![before.keyforms.keyforms[1]]);
}

#[test]
fn remove_parameter_collapses_glue_and_part_grids() {
    let model = fixtures::glued_quads();
    let after = applied(&model, Command::RemoveParameter { id: "Glue".into() });
    assert!(after.glues[0].keyforms.bindings.is_empty());
    assert_eq!(after.glues[0].keyforms.keyforms[0].intensity, 1.0);

    let model = fixtures::part_opacity_hierarchy();
    let mut doc = Document::new(model.clone());
    for p in &model.parameters {
        doc.apply(Command::RemoveParameter { id: p.id.clone() })
            .unwrap();
    }
    assert!(
        doc.model()
            .parts
            .iter()
            .all(|p| p.keyforms.bindings.is_empty() && p.keyforms.keyforms.len() == 1)
    );
}

#[test]
fn remove_parameter_collapses_origins_and_inference_axes() {
    use ugoe_model::KeyformInference;
    use ugoe_model::KeyformOrigin::{Authored, Derived};
    let model = fixtures::keyform_inference();
    let face = |model: &Model| match &model.deformers[0].kind {
        DeformerKind::Warp(warp) => warp.keyforms.clone(),
        _ => panic!("Face is a warp deformer"),
    };
    let after = applied(
        &model,
        Command::RemoveParameter {
            id: "AngleX".into(),
        },
    );
    let grid = face(&after);
    assert_eq!(grid.bindings.len(), 1);
    // `AngleX = 0` の断面 (線形インデックス 1, 4, 7) が残る。中央だけが `authored`。
    assert_eq!(grid.origins, vec![Derived, Authored, Derived]);
    let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference else {
        panic!("inference must stay depth rotation");
    };
    assert_eq!(rotation.yaw, None);
    assert_eq!(rotation.pitch, Some("AngleY".into()));

    let after = applied(
        &after,
        Command::RemoveParameter {
            id: "AngleY".into(),
        },
    );
    let grid = face(&after);
    assert!(grid.bindings.is_empty());
    assert_eq!(grid.origins, vec![Authored]);
    let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference else {
        panic!("inference must stay depth rotation");
    };
    assert_eq!(
        (rotation.yaw.as_ref(), rotation.pitch.as_ref()),
        (None, None)
    );
}

#[test]
fn remove_parameter_drops_mirror_and_effect_references() {
    let mut model = fixtures::pose_and_effects();
    let right = model
        .parameters
        .iter_mut()
        .find(|p| p.id.as_str() == "EyeROpen")
        .unwrap();
    right.mirror = Some("EyeLOpen".into());
    let after = applied(
        &model,
        Command::Batch {
            label: "remove".into(),
            commands: vec![
                Command::RemoveParameter {
                    id: "EyeLOpen".into(),
                },
                Command::RemoveParameter {
                    id: "Breath".into(),
                },
                Command::RemoveParameter {
                    id: "MouthOpenY".into(),
                },
            ],
        },
    );
    let right = after
        .parameters
        .iter()
        .find(|p| p.id.as_str() == "EyeROpen")
        .unwrap();
    assert_eq!(right.mirror, None);
    assert_eq!(after.effects.eye_blink.parameters, vec!["EyeROpen".into()]);
    assert!(
        after
            .effects
            .breath
            .parameters
            .iter()
            .all(|b| b.parameter.as_str() != "Breath")
    );
    assert_eq!(after.effects.breath.parameters.len(), 4);
    assert!(after.effects.lip_sync.parameters.is_empty());

    assert_eq!(
        rejected(&model, Command::RemoveParameter { id: "Nope".into() }),
        not_found("parameter", "Nope")
    );
}

#[test]
fn set_parameter_range() {
    let model = fixtures::single_quad();
    let cmd: Command = serde_json::from_str(
        r#"{"op":"set_parameter_range","id":"X","min":-2,"default":0,"max":2}"#,
    )
    .unwrap();
    let after = applied(&model, cmd);
    assert_eq!(after.parameters[0].max, 2.0);
    assert_eq!(
        rejected(
            &model,
            Command::SetParameterRange {
                id: "X".into(),
                min: 1.0,
                default: 0.0,
                max: 2.0,
            },
        ),
        EditError::InvalidRange {
            min: 1.0,
            default: 0.0,
            max: 2.0,
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::SetParameterRange {
                id: "Nope".into(),
                min: 0.0,
                default: 0.0,
                max: 0.0,
            },
        ),
        not_found("parameter", "Nope")
    );
}

#[test]
fn rename_parameter() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenameParameter {
            id: "X".into(),
            name: "Horizontal".into(),
        },
    );
    assert_eq!(after.parameters[0].name, "Horizontal");
    assert_eq!(after.parameters[0].id.as_str(), "X");
    assert_eq!(
        rejected(
            &model,
            Command::RenameParameter {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("parameter", "Nope")
    );
}

#[test]
fn set_parameter_group() {
    let mut model = fixtures::single_quad();
    model.parameter_groups.push(ParameterGroup {
        id: "Face".into(),
        name: "Face".into(),
    });
    let after = applied(
        &model,
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Face".into()),
        },
    );
    assert_eq!(after.parameters[0].group.as_deref(), Some("Face"));
    let cleared = applied(
        &after,
        Command::SetParameterGroup {
            id: "X".into(),
            group: None,
        },
    );
    assert_eq!(cleared.parameters[0].group, None);
    rejected_invalid(
        &model,
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Body".into()),
        },
        |e| matches!(e, ValidationError::UnknownReference { field: "group", .. }),
    );
}

#[test]
fn set_parameter_mirror() {
    let model = fixtures::pose_and_effects();
    let after = applied(
        &model,
        Command::SetParameterMirror {
            id: "EyeLOpen".into(),
            mirror: Some("EyeROpen".into()),
        },
    );
    let left = after
        .parameters
        .iter()
        .find(|p| p.id.as_str() == "EyeLOpen")
        .unwrap();
    assert_eq!(left.mirror, Some("EyeROpen".into()));
    // 自分自身との対 (符号の反転) も許す。
    let after = applied(
        &model,
        Command::SetParameterMirror {
            id: "AngleX".into(),
            mirror: Some("AngleX".into()),
        },
    );
    let angle = after
        .parameters
        .iter()
        .find(|p| p.id.as_str() == "AngleX")
        .unwrap();
    assert_eq!(angle.mirror, Some("AngleX".into()));
    rejected_invalid(
        &model,
        Command::SetParameterMirror {
            id: "EyeLOpen".into(),
            mirror: Some("Nope".into()),
        },
        |e| {
            matches!(
                e,
                ValidationError::UnknownReference {
                    field: "mirror",
                    ..
                }
            )
        },
    );
}

// ---- パーツ ----

#[test]
fn add_part() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddPart {
            part: new_part("Head", Some("Root")),
        },
    );
    assert_eq!(after.parts[1].id.as_str(), "Head");
    assert_eq!(
        rejected(
            &model,
            Command::AddPart {
                part: new_part("Root", None),
            },
        ),
        EditError::DuplicateId {
            kind: "part",
            id: "Root".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddPart {
            part: new_part("Head", Some("Nope")),
        },
        |e| {
            matches!(
                e,
                ValidationError::UnknownReference {
                    field: "parent",
                    ..
                }
            )
        },
    );
}

#[test]
fn remove_part_rejects_parts_with_children() {
    let model = fixtures::pose_and_effects();
    assert_eq!(
        rejected(&model, Command::RemovePart { id: "Root".into() }),
        EditError::HasChildren {
            kind: "part",
            id: "Root".into(),
            children: vec![
                "part `ArmA`".into(),
                "part `SleeveA`".into(),
                "part `ArmB`".into(),
                "mesh `Quad`".into(),
            ],
        }
    );
    let model = fixtures::rotation_chain();
    let err = rejected(&model, Command::RemovePart { id: "Root".into() });
    assert!(
        matches!(&err, EditError::HasChildren { children, .. } if children[0] == "deformer `Arm`"),
        "{err:?}"
    );
    assert_eq!(
        rejected(&model, Command::RemovePart { id: "Nope".into() }),
        not_found("part", "Nope")
    );
}

#[test]
fn remove_part_drops_pose_references() {
    let model = fixtures::pose_and_effects();
    let after = applied(&model, Command::RemovePart { id: "ArmA".into() });
    assert_eq!(after.parts.len(), 3);
    let entries = &after.pose.groups[0].entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].part.as_str(), "ArmB");

    let after = applied(
        &model,
        Command::RemovePart {
            id: "SleeveA".into(),
        },
    );
    assert!(after.pose.groups[0].entries[0].links.is_empty());

    // グループが空になったらグループごと消える。
    let after = applied(
        &model,
        Command::Batch {
            label: "remove arms".into(),
            commands: vec![
                Command::RemovePart { id: "ArmA".into() },
                Command::RemovePart { id: "ArmB".into() },
            ],
        },
    );
    assert!(after.pose.groups.is_empty());
}

#[test]
fn rename_part() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenamePart {
            id: "Root".into(),
            name: "Body".into(),
        },
    );
    assert_eq!(after.parts[0].name, "Body");
    assert_eq!(
        rejected(
            &model,
            Command::RenamePart {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("part", "Nope")
    );
}

#[test]
fn set_part_parent() {
    let model = fixtures::pose_and_effects();
    let after = applied(
        &model,
        Command::SetPartParent {
            id: "SleeveA".into(),
            parent: Some("ArmA".into()),
        },
    );
    assert_eq!(after.parts[2].parent, Some("ArmA".into()));
    rejected_invalid(
        &model,
        Command::SetPartParent {
            id: "Root".into(),
            parent: Some("ArmA".into()),
        },
        |e| matches!(e, ValidationError::ParentCycle { kind: "part", .. }),
    );
    rejected_invalid(
        &model,
        Command::SetPartParent {
            id: "ArmA".into(),
            parent: Some("Nope".into()),
        },
        |e| matches!(e, ValidationError::UnknownReference { .. }),
    );
}

#[test]
fn set_part_keyforms() {
    let model = fixtures::single_quad();
    let kf = model.parts[0].keyforms.keyforms[0];
    let hidden = ugoe_model::part::PartKeyform { opacity: 0.0 };
    let after = applied(
        &model,
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: grid1("X", &[0.0, 1.0], vec![kf, hidden]),
        },
    );
    assert_eq!(after.parts[0].keyforms.keyforms.len(), 2);
    rejected_invalid(
        &model,
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: grid1("X", &[0.0, 1.0], vec![kf]),
        },
        |e| matches!(e, ValidationError::KeyformCount { .. }),
    );
}

// ---- デフォーマ ----

#[test]
fn add_deformer() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::AddDeformer {
            deformer: new_warp("Skirt", Some("Arm")),
        },
    );
    assert_eq!(after.deformers.len(), 4);
    assert_eq!(
        rejected(
            &model,
            Command::AddDeformer {
                deformer: new_warp("Arm", None),
            },
        ),
        EditError::DuplicateId {
            kind: "deformer",
            id: "Arm".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddDeformer {
            deformer: new_warp("Skirt", Some("Nope")),
        },
        |e| matches!(e, ValidationError::UnknownReference { .. }),
    );
}

#[test]
fn remove_deformer() {
    let model = fixtures::rotation_chain();
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Arm".into() }),
        EditError::HasChildren {
            kind: "deformer",
            id: "Arm".into(),
            children: vec!["deformer `Hand`".into(), "deformer `Cloth`".into()],
        }
    );
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Hand".into() }),
        EditError::HasChildren {
            kind: "deformer",
            id: "Hand".into(),
            children: vec!["mesh `Palm`".into()],
        }
    );
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Nope".into() }),
        not_found("deformer", "Nope")
    );
    let after = applied(
        &model,
        Command::Batch {
            label: "remove hand".into(),
            commands: vec![
                Command::RemoveMesh { id: "Palm".into() },
                Command::RemoveDeformer { id: "Hand".into() },
            ],
        },
    );
    assert_eq!(after.deformers.len(), 2);
}

#[test]
fn rename_deformer() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::RenameDeformer {
            id: "Arm".into(),
            name: "Upper arm".into(),
        },
    );
    assert_eq!(after.deformers[0].name, "Upper arm");
    assert_eq!(
        rejected(
            &model,
            Command::RenameDeformer {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("deformer", "Nope")
    );
}

#[test]
fn set_deformer_parent_and_part() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::SetDeformerParent {
            id: "Cloth".into(),
            parent: None,
        },
    );
    assert_eq!(after.deformers[2].parent, None);
    rejected_invalid(
        &model,
        Command::SetDeformerParent {
            id: "Arm".into(),
            parent: Some("Hand".into()),
        },
        |e| {
            matches!(
                e,
                ValidationError::ParentCycle {
                    kind: "deformer",
                    ..
                }
            )
        },
    );

    let mut with_part = model.clone();
    with_part.parts.push(new_part("Body", Some("Root")));
    let after = applied(
        &with_part,
        Command::SetDeformerPart {
            id: "Arm".into(),
            part: "Body".into(),
        },
    );
    assert_eq!(after.deformers[0].part.as_str(), "Body");
    rejected_invalid(
        &model,
        Command::SetDeformerPart {
            id: "Arm".into(),
            part: "Nope".into(),
        },
        |e| matches!(e, ValidationError::UnknownReference { field: "part", .. }),
    );
}

#[test]
fn set_deformer_keyforms() {
    let model = fixtures::rotation_chain();
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let rest = cloth.keyforms.keyforms[0].clone();
    let mut moved = rest.clone();
    moved.points[0].x += 1.0;
    let warp = grid1("Flip", &[0.0, 1.0], vec![rest.clone(), moved]);
    let after = applied(
        &model,
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: warp.clone(),
        },
    );
    let DeformerKind::Warp(cloth) = &after.deformers[2].kind else {
        panic!("Cloth is a warp deformer");
    };
    assert_eq!(cloth.keyforms, warp);
    assert_eq!(
        rejected(
            &model,
            Command::SetWarpKeyforms {
                id: "Arm".into(),
                keyforms: warp,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Arm".into(),
            expected: "warp",
        }
    );
    let mut short = rest;
    short.points.pop();
    rejected_invalid(
        &model,
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: KeyformGrid::constant(short),
        },
        |e| matches!(e, ValidationError::WarpPointCount { .. }),
    );

    let DeformerKind::Rotation(hand) = rotation_chain_deformer("Hand").kind else {
        panic!("Hand is a rotation deformer");
    };
    let rotation = KeyformGrid::constant(hand.keyforms.keyforms[1]);
    let after = applied(
        &model,
        Command::SetRotationKeyforms {
            id: "Hand".into(),
            keyforms: rotation.clone(),
        },
    );
    let DeformerKind::Rotation(hand) = &after.deformers[1].kind else {
        panic!("Hand is a rotation deformer");
    };
    assert_eq!(hand.keyforms, rotation);
    assert_eq!(
        rejected(
            &model,
            Command::SetRotationKeyforms {
                id: "Cloth".into(),
                keyforms: rotation,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Cloth".into(),
            expected: "rotation",
        }
    );
}

#[test]
fn unknown_deformer_kinds_are_not_edited() {
    // 新しい版のファイルから読んだ、未知の種類のデフォーマ (`docs/spec/file-formats.md#未知の値の読み込み`)。
    let mut model = fixtures::rotation_chain();
    model.deformers[1].kind = DeformerKind::Unknown;
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let DeformerKind::Rotation(hand) = rotation_chain_deformer("Hand").kind else {
        panic!("Hand is a rotation deformer");
    };
    assert_eq!(
        rejected(
            &model,
            Command::SetWarpKeyforms {
                id: "Hand".into(),
                keyforms: cloth.keyforms,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Hand".into(),
            expected: "warp",
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::SetRotationKeyforms {
                id: "Hand".into(),
                keyforms: hand.keyforms,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Hand".into(),
            expected: "rotation",
        }
    );
    // パラメータの削除は、キーフォームを持たない未知のデフォーマには触れない。
    let after = applied(&model, Command::RemoveParameter { id: "Flip".into() });
    assert_eq!(after.deformers[1].kind, DeformerKind::Unknown);
}

// ---- メッシュ ----

#[test]
fn add_mesh() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddMesh {
            mesh: new_mesh("Quad2"),
        },
    );
    assert_eq!(mesh_ids(&after), ["Quad", "Quad2"]);
    assert_eq!(
        rejected(
            &model,
            Command::AddMesh {
                mesh: new_mesh("Quad"),
            },
        ),
        EditError::DuplicateId {
            kind: "mesh",
            id: "Quad".into(),
        }
    );
    let mut bad = new_mesh("Quad2");
    bad.texture = 5;
    rejected_invalid(&model, Command::AddMesh { mesh: bad }, |e| {
        matches!(e, ValidationError::TextureOutOfRange { .. })
    });
}

#[test]
fn remove_mesh_drops_mask_references() {
    let model = fixtures::nested_masks();
    let after = applied(&model, Command::RemoveMesh { id: "Iris".into() });
    assert_eq!(mesh_ids(&after), ["Pupil", "Sclera"]);
    assert!(after.meshes[0].masks.is_empty());
    assert_eq!(
        rejected(&model, Command::RemoveMesh { id: "Nope".into() }),
        not_found("mesh", "Nope")
    );
}

#[test]
fn remove_mesh_and_deformer_drop_mirror_references() {
    let mut model = fixtures::glued_quads();
    model.meshes[0].mirror = Some("Right".into());
    model.meshes[1].mirror = Some("Left".into());
    model.glues.clear();
    let after = applied(&model, Command::RemoveMesh { id: "Left".into() });
    assert_eq!(after.meshes[0].mirror, None);

    let mut model = fixtures::rotation_chain();
    model.deformers[1].mirror = Some("Cloth".into());
    model.deformers[2].mirror = Some("Hand".into());
    let after = applied(
        &model,
        Command::Batch {
            label: "remove hand".into(),
            commands: vec![
                Command::RemoveMesh { id: "Palm".into() },
                Command::RemoveDeformer { id: "Hand".into() },
            ],
        },
    );
    assert_eq!(after.deformers[1].id.as_str(), "Cloth");
    assert_eq!(after.deformers[1].mirror, None);
}

#[test]
fn remove_mesh_drops_glues_and_hit_areas() {
    let mut model = fixtures::glued_quads();
    model.hit_areas.push(HitArea {
        id: "Touch".into(),
        mesh: "Left".into(),
    });
    let after = applied(&model, Command::RemoveMesh { id: "Left".into() });
    assert!(after.glues.is_empty());
    assert!(after.hit_areas.is_empty());
}

#[test]
fn rename_mesh() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenameMesh {
            id: "Quad".into(),
            name: "Square".into(),
        },
    );
    assert_eq!(after.meshes[0].name, "Square");
    assert_eq!(
        rejected(
            &model,
            Command::RenameMesh {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("mesh", "Nope")
    );
}

#[test]
fn set_mesh_properties_changes_only_given_fields() {
    let model = fixtures::nested_masks();
    let cmd: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Iris","blend":"additive"}"#)
            .unwrap();
    let after = applied(&model, cmd);
    let mut expected = model.meshes[1].clone();
    expected.blend = BlendMode::Additive;
    assert_eq!(after.meshes[1], expected);

    let mut with_part = model.clone();
    with_part.parts.push(new_part("Eye", Some("Root")));
    let after = applied(
        &with_part,
        Command::SetMeshProperties {
            id: "Pupil".into(),
            part: Some("Eye".into()),
            texture: Some(0),
            blend: Some(BlendMode::Multiply),
            culling: Some(true),
            masks: Some(vec![MaskRef::inverted("Sclera")]),
            mask_mode: Some(ugoe_model::MaskMode::Intersection),
            mask_only: Some(true),
        },
    );
    let pupil = &after.meshes[0];
    assert_eq!(pupil.part.as_str(), "Eye");
    assert_eq!(pupil.blend, BlendMode::Multiply);
    assert!(pupil.culling && pupil.mask_only);
    assert_eq!(pupil.masks, vec![MaskRef::inverted("Sclera")]);
    assert_eq!(pupil.mask_mode, ugoe_model::MaskMode::Intersection);
}

#[test]
fn set_mesh_properties_rejects_invalid_results() {
    let model = fixtures::nested_masks();
    let props = |masks, texture| Command::SetMeshProperties {
        id: "Sclera".into(),
        part: None,
        texture,
        blend: None,
        culling: None,
        masks,
        mask_mode: None,
        mask_only: None,
    };
    rejected_invalid(
        &model,
        props(Some(vec![MaskRef::normal("Pupil")]), None),
        |e| matches!(e, ValidationError::MaskCycle { .. }),
    );
    rejected_invalid(&model, props(None, Some(1)), |e| {
        matches!(e, ValidationError::TextureOutOfRange { .. })
    });
    assert_eq!(
        rejected(
            &model,
            Command::SetMeshProperties {
                id: "Nope".into(),
                part: None,
                texture: None,
                blend: None,
                culling: None,
                masks: None,
                mask_mode: None,
                mask_only: None,
            },
        ),
        not_found("mesh", "Nope")
    );
}

#[test]
fn set_mesh_parent() {
    let model = fixtures::warp_grid();
    let after = applied(
        &model,
        Command::SetMeshParent {
            id: "Inner".into(),
            parent: None,
        },
    );
    assert_eq!(after.meshes[0].parent, None);
    rejected_invalid(
        &model,
        Command::SetMeshParent {
            id: "Inner".into(),
            parent: Some("Nope".into()),
        },
        |e| {
            matches!(
                e,
                ValidationError::UnknownReference {
                    field: "parent",
                    ..
                }
            )
        },
    );
}

#[test]
fn set_mesh_keyforms() {
    let model = fixtures::single_quad();
    let rest = model.meshes[0].keyforms.keyforms[1].clone();
    let after = applied(
        &model,
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: KeyformGrid::constant(rest.clone()),
        },
    );
    assert_eq!(after.meshes[0].keyforms.keyforms, vec![rest.clone()]);
    let mut bad = rest;
    bad.positions.pop();
    rejected_invalid(
        &model,
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: KeyformGrid::constant(bad),
        },
        |e| matches!(e, ValidationError::VertexCount { .. }),
    );
}

#[test]
fn set_mesh_keyform_marks_cell_authored() {
    use ugoe_model::KeyformOrigin::{Authored, Derived};
    let mut model = fixtures::single_quad();
    // 基準のセル (X = 0) は推定の元なので `authored` のまま。
    model.meshes[0].keyforms.origins = vec![Derived, Authored, Derived];
    let keyform = model.meshes[0].keyforms.keyforms[2].clone();
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform,
        },
    );
    assert_eq!(
        after.meshes[0].keyforms.origins,
        vec![Derived, Authored, Authored]
    );
}

#[test]
fn set_mesh_keyform_cell() {
    let model = fixtures::single_quad();
    let mut keyform = model.meshes[0].keyforms.keyforms[2].clone();
    keyform.opacity = 0.5;
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: keyform.clone(),
        },
    );
    let mut expected = model.meshes[0].keyforms.clone();
    expected.keyforms[2] = keyform.clone();
    assert_eq!(after.meshes[0].keyforms, expected);

    assert_eq!(
        rejected(
            &model,
            Command::SetMeshKeyform {
                id: "Quad".into(),
                cell: cell(&[("X", 0.5)]),
                keyform: keyform.clone(),
            },
        ),
        EditError::InvalidCell {
            kind: "mesh",
            id: "Quad".into(),
            error: CellError::UnknownKey {
                parameter: "X".into(),
                value: 0.5,
            },
        }
    );
    let mut bad = keyform;
    bad.positions.pop();
    rejected_invalid(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: bad,
        },
        |e| matches!(e, ValidationError::VertexCount { .. }),
    );
}

#[test]
fn reorder_mesh() {
    let model = fixtures::nested_masks();
    assert_eq!(mesh_ids(&model), ["Pupil", "Iris", "Sclera"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Sclera".into(),
            before: Some("Pupil".into()),
        },
    );
    assert_eq!(mesh_ids(&after), ["Sclera", "Pupil", "Iris"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Pupil".into(),
            before: None,
        },
    );
    assert_eq!(mesh_ids(&after), ["Iris", "Sclera", "Pupil"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Pupil".into(),
            before: Some("Sclera".into()),
        },
    );
    assert_eq!(mesh_ids(&after), ["Iris", "Pupil", "Sclera"]);

    assert_eq!(
        rejected(
            &model,
            Command::ReorderMesh {
                id: "Iris".into(),
                before: Some("Iris".into()),
            },
        ),
        EditError::SelfReference {
            kind: "mesh",
            id: "Iris".into(),
            field: "before",
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::ReorderMesh {
                id: "Iris".into(),
                before: Some("Nope".into()),
            },
        ),
        not_found("mesh", "Nope")
    );
}

// ---- Batch と検証 ----

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

// ---- JSON ----

/// バリアント名 (`op` の値)。バリアントを追加すると、ここと `json_samples` の更新をコンパイラが求める。
fn op_name(command: &Command) -> &'static str {
    match command {
        Command::AddParameter { .. } => "add_parameter",
        Command::RemoveParameter { .. } => "remove_parameter",
        Command::SetParameterRange { .. } => "set_parameter_range",
        Command::Batch { .. } => "batch",
        Command::RenameParameter { .. } => "rename_parameter",
        Command::SetParameterGroup { .. } => "set_parameter_group",
        Command::SetParameterMirror { .. } => "set_parameter_mirror",
        Command::AddPart { .. } => "add_part",
        Command::RemovePart { .. } => "remove_part",
        Command::RenamePart { .. } => "rename_part",
        Command::SetPartParent { .. } => "set_part_parent",
        Command::SetPartKeyforms { .. } => "set_part_keyforms",
        Command::AddDeformer { .. } => "add_deformer",
        Command::RemoveDeformer { .. } => "remove_deformer",
        Command::RenameDeformer { .. } => "rename_deformer",
        Command::SetDeformerParent { .. } => "set_deformer_parent",
        Command::SetDeformerPart { .. } => "set_deformer_part",
        Command::SetWarpKeyforms { .. } => "set_warp_keyforms",
        Command::SetRotationKeyforms { .. } => "set_rotation_keyforms",
        Command::AddMesh { .. } => "add_mesh",
        Command::RemoveMesh { .. } => "remove_mesh",
        Command::RenameMesh { .. } => "rename_mesh",
        Command::SetMeshProperties { .. } => "set_mesh_properties",
        Command::SetMeshParent { .. } => "set_mesh_parent",
        Command::SetMeshKeyforms { .. } => "set_mesh_keyforms",
        Command::SetMeshKeyform { .. } => "set_mesh_keyform",
        Command::ReorderMesh { .. } => "reorder_mesh",
        Command::AddTexture { .. } => "add_texture",
        Command::SetTextureImage { .. } => "set_texture_image",
        Command::RemoveTexture { .. } => "remove_texture",
        Command::SetCanvas { .. } => "set_canvas",
        Command::RegenerateMesh { .. } => "regenerate_mesh",
        Command::InferKeyforms { .. } => "infer_keyforms",
        Command::MirrorKeyforms { .. } => "mirror_keyforms",
        Command::MarkKeyformAuthored { .. } => "mark_keyform_authored",
        Command::MarkKeyformDerived { .. } => "mark_keyform_derived",
        Command::MoveMeshVertices { .. } => "move_mesh_vertices",
        Command::AddMeshVertex { .. } => "add_mesh_vertex",
        Command::RemoveMeshVertices { .. } => "remove_mesh_vertices",
        Command::RetriangulateMesh { .. } => "retriangulate_mesh",
    }
}

const OP_COUNT: usize = 40;

fn json_samples() -> Vec<Command> {
    let quad = fixtures::single_quad();
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let DeformerKind::Rotation(arm) = rotation_chain_deformer("Arm").kind else {
        panic!("Arm is a rotation deformer");
    };
    let mesh_keyforms = quad.meshes[0].keyforms.clone();
    vec![
        Command::AddParameter {
            parameter: new_parameter("Y", -1.0, 0.0, 1.0),
        },
        Command::RemoveParameter { id: "X".into() },
        Command::SetParameterRange {
            id: "X".into(),
            min: -2.0,
            default: 0.5,
            max: 2.0,
        },
        Command::Batch {
            label: "batch".into(),
            commands: vec![Command::RemoveParameter { id: "X".into() }],
        },
        Command::RenameParameter {
            id: "X".into(),
            name: "Horizontal".into(),
        },
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Face".into()),
        },
        Command::SetParameterMirror {
            id: "X".into(),
            mirror: None,
        },
        Command::AddPart {
            part: new_part("Head", Some("Root")),
        },
        Command::RemovePart { id: "Head".into() },
        Command::RenamePart {
            id: "Root".into(),
            name: "Body".into(),
        },
        Command::SetPartParent {
            id: "Head".into(),
            parent: Some("Root".into()),
        },
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: quad.parts[0].keyforms.clone(),
        },
        Command::AddDeformer {
            deformer: rotation_chain_deformer("Hand"),
        },
        Command::RemoveDeformer { id: "Hand".into() },
        Command::RenameDeformer {
            id: "Hand".into(),
            name: "Wrist".into(),
        },
        Command::SetDeformerParent {
            id: "Hand".into(),
            parent: None,
        },
        Command::SetDeformerPart {
            id: "Hand".into(),
            part: "Root".into(),
        },
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: cloth.keyforms,
        },
        Command::SetRotationKeyforms {
            id: "Arm".into(),
            keyforms: arm.keyforms,
        },
        Command::AddMesh {
            mesh: new_mesh("Quad2"),
        },
        Command::RemoveMesh { id: "Quad".into() },
        Command::RenameMesh {
            id: "Quad".into(),
            name: "Square".into(),
        },
        Command::SetMeshProperties {
            id: "Quad".into(),
            part: None,
            texture: Some(0),
            blend: Some(BlendMode::Additive),
            culling: None,
            masks: Some(vec![MaskRef::inverted("Other")]),
            mask_mode: None,
            mask_only: Some(false),
        },
        Command::SetMeshParent {
            id: "Quad".into(),
            parent: Some("Warp".into()),
        },
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: mesh_keyforms.keyforms[2].clone(),
        },
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: mesh_keyforms,
        },
        Command::ReorderMesh {
            id: "Quad".into(),
            before: Some("Other".into()),
        },
        Command::AddTexture {
            name: "hair".into(),
            png: crate::texture::solid_png(1, 1, [0; 4]),
        },
        Command::SetTextureImage {
            name: "white".into(),
            png: crate::texture::solid_png(1, 1, [255; 4]),
        },
        Command::RemoveTexture {
            name: "white".into(),
        },
        Command::SetCanvas {
            width: 512.0,
            height: 256.0,
            origin: ugoe_core::Vec2::new(256.0, 200.0),
        },
        Command::RegenerateMesh {
            id: "Quad".into(),
            options: crate::AutoMeshOptions {
                edge_spacing: 12.0,
                ..crate::AutoMeshOptions::default()
            },
        },
        Command::InferKeyforms {
            target: crate::KeyformTarget::Deformer("Face".into()),
            method: crate::InferMethod::DepthRotation,
        },
        Command::MirrorKeyforms {
            source: crate::KeyformTarget::Mesh("EyeL".into()),
            target: crate::KeyformTarget::Mesh("EyeR".into()),
            axis: Some(0.5),
            tolerance: Some(0.01),
            overwrite_authored: true,
        },
        Command::MarkKeyformAuthored {
            target: crate::KeyformTarget::Part("Root".into()),
            cells: vec![cell(&[("X", 1.0)])],
        },
        Command::MarkKeyformDerived {
            target: crate::KeyformTarget::Glue("Glue".into()),
            cells: vec![KeyformCell::new()],
        },
        Command::MoveMeshVertices {
            id: "Quad".into(),
            moves: vec![crate::VertexMove {
                index: 2,
                delta: ugoe_core::Vec2::new(1.0, -2.0),
            }],
        },
        Command::AddMeshVertex {
            id: "Quad".into(),
            position: ugoe_core::Vec2::new(100.0, 120.0),
        },
        Command::RemoveMeshVertices {
            id: "Quad".into(),
            indices: vec![1, 3],
        },
        Command::RetriangulateMesh { id: "Quad".into() },
    ]
}

#[test]
fn every_variant_roundtrips_json() {
    let samples = json_samples();
    let mut ops: Vec<_> = samples.iter().map(op_name).collect();
    ops.sort_unstable();
    ops.dedup();
    assert_eq!(ops.len(), OP_COUNT, "json_samples must cover every variant");
    for command in samples {
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(json["op"], op_name(&command));
        let back: Command = serde_json::from_value(json).unwrap();
        assert_eq!(back, command);
    }
}

#[test]
fn json_uses_ids_for_keyform_cells() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"set_mesh_keyform","id":"Quad","cell":{"X":1},
            "keyform":{"positions":[[0,0],[1,0],[1,1],[0,1]],"opacity":1,"draw_order":500}}"#,
    )
    .unwrap();
    let Command::SetMeshKeyform { cell: c, .. } = &cmd else {
        panic!("unexpected command {cmd:?}");
    };
    assert_eq!(c, &cell(&[("X", 1.0)]));
}
