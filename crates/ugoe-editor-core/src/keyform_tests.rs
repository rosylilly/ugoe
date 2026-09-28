//! キーフォームの編集 (`AddKeyformKey`・`RemoveKeyformKey`・`SetKeyformCell`) と
//! 座標の変換 (`crate::space`) のテスト。

use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::{KeyformOrigin, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::space::{self, ParameterValues, SpaceError};
use crate::{CellError, Command, Document, EditError, Keyform, KeyformCell, KeyformTarget};

fn mesh(id: &str) -> KeyformTarget {
    KeyformTarget::Mesh(id.into())
}

fn deformer(id: &str) -> KeyformTarget {
    KeyformTarget::Deformer(id.into())
}

fn add_key(target: KeyformTarget, parameter: &str, key: f32) -> Command {
    Command::AddKeyformKey {
        target,
        parameter: parameter.into(),
        key,
    }
}

fn remove_key(target: KeyformTarget, parameter: &str, key: f32) -> Command {
    Command::RemoveKeyformKey {
        target,
        parameter: parameter.into(),
        key,
    }
}

fn cell(entries: &[(&str, f32)]) -> KeyformCell {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

/// パラメータ値の組 `values` ごとの、すべてのメッシュの評価結果 (頂点位置・不透明度・描画順)。
fn evaluate(model: &Model, values: &[Vec<(&str, f32)>]) -> Vec<Vec<(Vec<Vec2>, f32, f32)>> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    values
        .iter()
        .map(|set| {
            instance.parameters_mut().reset();
            for &(id, v) in set {
                let index = instance.parameters().index_of(id).unwrap();
                instance.parameters_mut().set(index, v);
            }
            instance.update();
            instance
                .drawables()
                .iter()
                .map(|d| (d.positions.clone(), d.opacity, d.draw_order))
                .collect()
        })
        .collect()
}

/// パラメータ `id` の値域を `steps` 等分した値 (両端を含む)。
fn sweep(model: &Model, id: &str, steps: usize) -> Vec<f32> {
    let p = model
        .parameters
        .iter()
        .find(|p| p.id.as_str() == id)
        .unwrap();
    (0..=steps)
        .map(|i| p.min + (p.max - p.min) * i as f32 / steps as f32)
        .collect()
}

fn assert_same_look(before: &Model, after: &Model, values: &[Vec<(&str, f32)>]) {
    let a = evaluate(before, values);
    let b = evaluate(after, values);
    for ((set, a), b) in values.iter().zip(&a).zip(&b) {
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            for (p, q) in x.0.iter().zip(&y.0) {
                assert!(
                    (*p - *q).length() < 1e-3,
                    "mesh {i} at {set:?}: {p:?} != {q:?}"
                );
            }
            assert!((x.1 - y.1).abs() < 1e-5, "opacity of mesh {i} at {set:?}");
            assert!(
                (x.2 - y.2).abs() < 1e-3,
                "draw order of mesh {i} at {set:?}"
            );
        }
    }
}

fn one_axis(model: &Model, id: &'static str) -> Vec<Vec<(&'static str, f32)>> {
    sweep(model, id, 40)
        .into_iter()
        .map(|v| vec![(id, v)])
        .collect()
}

fn warp_keys(doc: &Document, id: &str) -> (Vec<f32>, usize, Vec<KeyformOrigin>) {
    let d = doc.model().deformers.iter().find(|d| d.id.as_str() == id);
    match &d.unwrap().kind {
        DeformerKind::Warp(w) => (
            w.keyforms.bindings[0].keys.clone(),
            w.keyforms.keyforms.len(),
            w.keyforms.origins.clone(),
        ),
        _ => panic!("not a warp"),
    }
}

// ---- AddKeyformKey ----

#[test]
fn add_key_between_keys_keeps_the_look() {
    let model = fixtures::warp_grid();
    let mut doc = Document::new(model.clone());
    doc.apply(add_key(deformer("Warp"), "X", 0.5)).unwrap();
    doc.apply(add_key(deformer("Warp"), "X", -0.25)).unwrap();
    let (keys, len, origins) = warp_keys(&doc, "Warp");
    assert_eq!(keys, vec![-1.0, -0.25, 0.0, 0.5, 1.0]);
    assert_eq!(len, 5);
    // 格子に `derived` のセルがないので、新しいセルは `authored` (`origins` は空のまま)。
    assert!(origins.is_empty());
    assert_same_look(&model, doc.model(), &one_axis(&model, "X"));
}

#[test]
fn add_key_outside_keys_copies_the_end_cell() {
    // `Arm` のキーは -30 と 30。範囲の外のキーは端のセルと同じ値になる (外挿しない)。
    let mut model = fixtures::rotation_chain();
    let arm = model
        .deformers
        .iter_mut()
        .find(|d| d.id.as_str() == "Arm")
        .unwrap();
    if let DeformerKind::Rotation(r) = &mut arm.kind {
        r.keyforms.bindings[0].keys = vec![-10.0, 10.0];
    }
    let mut doc = Document::new(model.clone());
    doc.apply(add_key(deformer("Arm"), "Angle", 20.0)).unwrap();
    doc.apply(add_key(deformer("Arm"), "Angle", -30.0)).unwrap();
    doc.apply(add_key(deformer("Arm"), "Angle", 0.0)).unwrap();
    assert_same_look(&model, doc.model(), &one_axis(&model, "Angle"));
}

#[test]
fn add_key_on_new_axis_keeps_cells() {
    let model = fixtures::rotation_chain();
    let mut doc = Document::new(model.clone());
    doc.apply(add_key(mesh("Palm"), "Angle", 0.0)).unwrap();
    doc.apply(add_key(mesh("Palm"), "Angle", 30.0)).unwrap();
    let palm = &doc.model().meshes[0];
    assert_eq!(palm.keyforms.bindings.len(), 1);
    assert_eq!(palm.keyforms.bindings[0].keys, vec![0.0, 30.0]);
    assert_eq!(palm.keyforms.keyforms.len(), 2);
    let values: Vec<_> = sweep(&model, "Angle", 12)
        .into_iter()
        .flat_map(|a| [vec![("Angle", a)], vec![("Angle", a), ("Flip", 1.0)]])
        .collect();
    assert_same_look(&model, doc.model(), &values);
}

#[test]
fn add_key_on_two_axis_grid_keeps_the_look_and_marks_derived() {
    // `Face` は AngleX × AngleY の 3×3 で、中央以外は `derived`。
    // 格子の中身を少し崩して、補間が本当に効くようにする。
    let mut modified = fixtures::keyform_inference();
    if let DeformerKind::Warp(w) = &mut modified.deformers[0].kind {
        for (i, k) in w.keyforms.keyforms.iter_mut().enumerate() {
            for (j, p) in k.points.iter_mut().enumerate() {
                *p += Vec2::new(i as f32 * 3.0, (i * j) as f32);
            }
            k.opacity = 1.0 - i as f32 * 0.1;
        }
    }
    let mut doc = Document::new(modified.clone());
    doc.apply(add_key(deformer("Face"), "AngleY", 10.0))
        .unwrap();
    let DeformerKind::Warp(w) = &doc.model().deformers[0].kind else {
        panic!()
    };
    assert_eq!(w.keyforms.bindings[1].keys, vec![-30.0, 0.0, 10.0, 30.0]);
    assert_eq!(w.keyforms.keyforms.len(), 12);
    assert_eq!(w.keyforms.origins.len(), 12);
    // 軸 0 (AngleX) が最も速い: AngleY = 10 の行はインデックス 6..9。
    for (i, origin) in w.keyforms.origins.iter().enumerate() {
        let expected = if i == 4 {
            KeyformOrigin::Authored
        } else {
            KeyformOrigin::Derived
        };
        assert_eq!(*origin, expected, "cell {i}");
    }
    let mut values = Vec::new();
    for x in sweep(&modified, "AngleX", 12) {
        for y in sweep(&modified, "AngleY", 12) {
            values.push(vec![("AngleX", x), ("AngleY", y)]);
        }
    }
    assert_same_look(&modified, doc.model(), &values);
}

#[test]
fn add_key_marks_new_cells_derived_only_if_grid_has_derived_cells() {
    let mut model = fixtures::warp_grid();
    if let DeformerKind::Warp(w) = &mut model.deformers[0].kind {
        // 方法を記録した由来 (`additive` など) も推定で作ったセルに数える。
        w.keyforms.origins = vec![
            KeyformOrigin::Additive,
            KeyformOrigin::Authored,
            KeyformOrigin::Authored,
        ];
    }
    let mut doc = Document::new(model);
    doc.apply(add_key(deformer("Warp"), "X", 0.5)).unwrap();
    use KeyformOrigin::{Additive, Authored, Derived};
    assert_eq!(
        warp_keys(&doc, "Warp").2,
        vec![Additive, Authored, Derived, Authored]
    );
}

#[test]
fn add_key_rejects_invalid_requests() {
    let mut doc = Document::new(fixtures::warp_grid());
    let before = doc.model().clone();
    assert_eq!(
        doc.apply(add_key(deformer("Warp"), "X", 0.0)),
        Err(EditError::InvalidCell {
            kind: "deformer",
            id: "Warp".into(),
            error: CellError::DuplicateKey {
                parameter: "X".into(),
                value: 0.0
            },
        })
    );
    assert!(matches!(
        doc.apply(add_key(deformer("Warp"), "X", f32::NAN)),
        Err(EditError::InvalidCell {
            error: CellError::NonFiniteKey { .. },
            ..
        })
    ));
    assert_eq!(
        doc.apply(add_key(deformer("Warp"), "Nope", 0.0)),
        Err(EditError::NotFound {
            kind: "parameter",
            id: "Nope".into()
        })
    );
    assert_eq!(
        doc.apply(add_key(mesh("Nope"), "X", 0.5)),
        Err(EditError::NotFound {
            kind: "mesh",
            id: "Nope".into()
        })
    );
    assert_eq!(doc.model(), &before);
}

#[test]
fn add_key_is_undoable() {
    let model = fixtures::warp_grid();
    let mut doc = Document::new(model.clone());
    doc.apply(add_key(deformer("Warp"), "X", 0.5)).unwrap();
    assert!(doc.undo());
    assert_eq!(doc.model(), &model);
    assert!(doc.redo());
    assert_eq!(warp_keys(&doc, "Warp").0, vec![-1.0, 0.0, 0.5, 1.0]);
}

// ---- RemoveKeyformKey ----

#[test]
fn remove_key_drops_the_slice() {
    let model = fixtures::warp_grid();
    let mut doc = Document::new(model.clone());
    doc.apply(remove_key(deformer("Warp"), "X", 0.0)).unwrap();
    let (keys, len, _) = warp_keys(&doc, "Warp");
    assert_eq!((keys, len), (vec![-1.0, 1.0], 2));
    let (DeformerKind::Warp(before), DeformerKind::Warp(after)) =
        (&model.deformers[0].kind, &doc.model().deformers[0].kind)
    else {
        panic!()
    };
    assert_eq!(after.keyforms.keyforms[0], before.keyforms.keyforms[0]);
    assert_eq!(after.keyforms.keyforms[1], before.keyforms.keyforms[2]);
    assert!(doc.undo());
    assert_eq!(doc.model(), &model);
}

#[test]
fn remove_key_on_two_axes_keeps_origins_aligned() {
    let mut doc = Document::new(fixtures::keyform_inference());
    doc.apply(remove_key(deformer("Face"), "AngleX", -30.0))
        .unwrap();
    let DeformerKind::Warp(w) = &doc.model().deformers[0].kind else {
        panic!()
    };
    assert_eq!(w.keyforms.keyforms.len(), 6);
    // 残るのは AngleX = 0, 30 の列。中央 (AngleX = 0, AngleY = 0) はインデックス 2。
    let authored: Vec<_> = (0..6)
        .filter(|&i| w.keyforms.origin(i) == KeyformOrigin::Authored)
        .collect();
    assert_eq!(authored, vec![2]);
}

#[test]
fn removing_the_last_key_removes_the_axis() {
    let mut doc = Document::new(fixtures::warp_grid());
    doc.apply(remove_key(deformer("Warp"), "X", -1.0)).unwrap();
    doc.apply(remove_key(deformer("Warp"), "X", 1.0)).unwrap();
    doc.apply(remove_key(deformer("Warp"), "X", 0.0)).unwrap();
    let DeformerKind::Warp(w) = &doc.model().deformers[0].kind else {
        panic!()
    };
    assert!(w.keyforms.bindings.is_empty());
    assert_eq!(w.keyforms.keyforms.len(), 1);
    let DeformerKind::Warp(original) = &fixtures::warp_grid().deformers[0].kind else {
        panic!()
    };
    assert_eq!(w.keyforms.keyforms[0], original.keyforms.keyforms[1]);
}

#[test]
fn remove_key_rejects_unknown_axis_and_key() {
    let mut doc = Document::new(fixtures::warp_grid());
    assert!(matches!(
        doc.apply(remove_key(deformer("Warp"), "X", 0.5)),
        Err(EditError::InvalidCell {
            error: CellError::UnknownKey { .. },
            ..
        })
    ));
    assert!(matches!(
        doc.apply(remove_key(mesh("Inner"), "X", 0.0)),
        Err(EditError::InvalidCell {
            error: CellError::UnknownAxis(_),
            ..
        })
    ));
}

// ---- SetKeyformCell ----

#[test]
fn set_keyform_cell_writes_the_cell_and_marks_it_authored() {
    let mut doc = Document::new(fixtures::keyform_inference());
    let DeformerKind::Warp(w) = &doc.model().deformers[0].kind else {
        panic!()
    };
    let mut keyform = w.keyforms.keyforms[8].clone();
    keyform.points[0] = Vec2::new(1.0, 2.0);
    doc.apply(Command::SetKeyformCell {
        target: deformer("Face"),
        cell: cell(&[("AngleX", 30.0), ("AngleY", 30.0)]),
        keyform: Keyform::Warp(keyform.clone()),
    })
    .unwrap();
    let DeformerKind::Warp(w) = &doc.model().deformers[0].kind else {
        panic!()
    };
    assert_eq!(w.keyforms.keyforms[8], keyform);
    assert_eq!(w.keyforms.origin(8), KeyformOrigin::Authored);
    // 他の推定のセルは自動で作り直される (方法が記録される)。
    assert!(w.keyforms.origin(7).is_derived());
}

#[test]
fn set_keyform_cell_works_for_every_kind() {
    let mut doc = Document::new(fixtures::rotation_chain());
    let model = doc.model().clone();
    let DeformerKind::Rotation(r) = &model.deformers[0].kind else {
        panic!()
    };
    let mut arm = r.keyforms.keyforms[1];
    arm.origin = Vec2::new(100.0, 100.0);
    arm.angle = 45.0;
    let mut palm = model.meshes[0].keyforms.keyforms[0].clone();
    palm.positions[2] = Vec2::new(20.0, 20.0);
    let mut root = model.parts[0].keyforms.keyforms[0];
    root.opacity = 0.5;
    for (target, cell, keyform) in [
        (
            deformer("Arm"),
            cell(&[("Angle", 30.0)]),
            Keyform::Rotation(arm),
        ),
        (mesh("Palm"), cell(&[]), Keyform::Mesh(palm.clone())),
        (
            KeyformTarget::Part("Root".into()),
            cell(&[]),
            Keyform::Part(root),
        ),
    ] {
        doc.apply(Command::SetKeyformCell {
            target,
            cell,
            keyform,
        })
        .unwrap();
    }
    let m = doc.model();
    let DeformerKind::Rotation(r) = &m.deformers[0].kind else {
        panic!()
    };
    assert_eq!(r.keyforms.keyforms[1], arm);
    assert_eq!(m.meshes[0].keyforms.keyforms[0], palm);
    assert_eq!(m.parts[0].keyforms.keyforms[0], root);
}

#[test]
fn set_keyform_cell_rejects_mismatches() {
    let mut doc = Document::new(fixtures::warp_grid());
    let before = doc.model().clone();
    let inner = before.meshes[0].keyforms.keyforms[0].clone();
    assert_eq!(
        doc.apply(Command::SetKeyformCell {
            target: deformer("Warp"),
            cell: cell(&[("X", 0.0)]),
            keyform: Keyform::Mesh(inner.clone()),
        }),
        Err(EditError::KeyformKindMismatch {
            kind: "deformer",
            id: "Warp".into(),
            expected: "warp",
            actual: "mesh",
        })
    );
    let mut fewer = inner.clone();
    fewer.positions.pop();
    assert_eq!(
        doc.apply(Command::SetKeyformCell {
            target: mesh("Inner"),
            cell: cell(&[]),
            keyform: Keyform::Mesh(fewer),
        }),
        Err(EditError::InvalidCell {
            kind: "mesh",
            id: "Inner".into(),
            error: CellError::ShapeMismatch {
                expected: 4,
                actual: 3
            },
        })
    );
    assert!(matches!(
        doc.apply(Command::SetKeyformCell {
            target: mesh("Inner"),
            cell: cell(&[("X", 0.0)]),
            keyform: Keyform::Mesh(inner),
        }),
        Err(EditError::InvalidCell {
            error: CellError::UnknownAxis(_),
            ..
        })
    ));
    assert_eq!(doc.model(), &before);
}

#[test]
fn keyform_commands_have_stable_json() {
    let json = r#"[
        {"op": "add_keyform_key", "target": {"kind": "mesh", "id": "Quad"}, "parameter": "X", "key": 0.5},
        {"op": "remove_keyform_key", "target": {"kind": "deformer", "id": "Warp"}, "parameter": "X", "key": 0},
        {"op": "set_keyform_cell", "target": {"kind": "part", "id": "Root"}, "cell": {},
         "keyform": {"kind": "part", "opacity": 0.5}}
    ]"#;
    let commands: Vec<Command> = serde_json::from_str(json).unwrap();
    assert_eq!(commands[0], add_key(mesh("Quad"), "X", 0.5));
    assert_eq!(commands[1], remove_key(deformer("Warp"), "X", 0.0));
    let Command::SetKeyformCell { keyform, .. } = &commands[2] else {
        panic!()
    };
    assert_eq!(
        keyform,
        &Keyform::Part(ugoe_model::part::PartKeyform { opacity: 0.5 })
    );
    // 書き出しと読み直しで同じになる。
    let back: Vec<Command> =
        serde_json::from_str(&serde_json::to_string(&commands).unwrap()).unwrap();
    assert_eq!(back, commands);
}

// ---- SetMeshDrawOrder ----

#[test]
fn set_mesh_draw_order_writes_every_cell_and_keeps_origins() {
    // `part_opacity_hierarchy` の `D` は描画順を `Fade` で 500 → 100 に動かす。
    let mut model = fixtures::part_opacity_hierarchy();
    let d = model
        .meshes
        .iter_mut()
        .find(|m| m.id.as_str() == "D")
        .unwrap();
    d.keyforms.origins = vec![KeyformOrigin::Authored, KeyformOrigin::Additive];
    let mut doc = Document::new(model.clone());
    doc.apply(Command::SetMeshDrawOrder {
        id: "D".into(),
        draw_order: 250.0,
    })
    .unwrap();
    let d = doc
        .model()
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "D")
        .unwrap();
    assert!(d.keyforms.keyforms.iter().all(|k| k.draw_order == 250.0));
    assert_eq!(
        d.keyforms.origins,
        vec![KeyformOrigin::Authored, KeyformOrigin::Additive]
    );
    // 他のフィールドは変わらない。
    let before = model.meshes.iter().find(|m| m.id.as_str() == "D").unwrap();
    assert_eq!(
        d.keyforms.keyforms[1].opacity,
        before.keyforms.keyforms[1].opacity
    );

    assert_eq!(
        doc.apply(Command::SetMeshDrawOrder {
            id: "D".into(),
            draw_order: f32::INFINITY,
        }),
        Err(EditError::InvalidDrawOrder {
            id: "D".into(),
            draw_order: f32::INFINITY,
        })
    );
    assert!(doc.undo());
    assert_eq!(doc.model(), &model);
}

// ---- 座標の変換 ----

fn values(entries: &[(&str, f32)]) -> ParameterValues {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

fn assert_close(a: &[Vec2], b: &[Vec2], eps: f32) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!((*p - *q).length() < eps, "{p:?} != {q:?}");
    }
}

#[test]
fn local_to_canvas_matches_runtime_evaluation() {
    // `Inner` の頂点を親 (`Warp`) の座標系から写すと、ランタイムの評価と一致する。
    let model = fixtures::warp_grid();
    for x in [-1.0, -0.3, 0.0, 1.0] {
        let local = model.meshes[0].keyforms.keyforms[0].positions.clone();
        let canvas =
            space::local_to_canvas(&model, &mesh("Inner"), &values(&[("X", x)]), &local).unwrap();
        let expected = &evaluate(&model, &[vec![("X", x)]])[0][0].0;
        assert_close(&canvas, expected, 1e-3);
    }
}

#[test]
fn canvas_to_local_inverts_warp_and_rotation_parents() {
    let cases = [
        (fixtures::warp_grid(), mesh("Inner"), values(&[("X", -1.0)])),
        (fixtures::warp_grid(), mesh("Outer"), values(&[("X", 0.4)])),
        (
            fixtures::rotation_chain(),
            mesh("Palm"),
            values(&[("Angle", 20.0), ("Flip", 1.0)]),
        ),
        (
            fixtures::rotation_chain(),
            mesh("Sleeve"),
            values(&[("Angle", -10.0)]),
        ),
        (
            fixtures::rotation_under_warp(),
            deformer("Head"),
            values(&[("Turn", 0.5)]),
        ),
        (fixtures::single_quad(), mesh("Quad"), values(&[("X", 0.5)])),
    ];
    for (model, target, values) in cases {
        let local = [
            Vec2::new(0.3, 0.6),
            Vec2::new(-0.2, 1.4),
            Vec2::new(5.0, 7.0),
        ];
        let canvas = space::local_to_canvas(&model, &target, &values, &local).unwrap();
        // 初期値はずらしておく。
        let guesses: Vec<Vec2> = local.iter().map(|p| *p + Vec2::new(0.1, -0.1)).collect();
        let back = space::canvas_to_local(&model, &target, &values, &canvas, &guesses).unwrap();
        assert_close(&back, &local, 1e-3);
    }
}

#[test]
fn space_rejects_elements_without_coordinates() {
    let model = fixtures::single_quad();
    assert_eq!(
        space::local_to_canvas(
            &model,
            &KeyformTarget::Part("Root".into()),
            &values(&[]),
            &[]
        ),
        Err(SpaceError::NoSpace {
            kind: "part",
            id: "Root".into()
        })
    );
    assert!(matches!(
        space::local_to_canvas(&model, &mesh("Nope"), &values(&[]), &[]),
        Err(SpaceError::NotFound { .. })
    ));
}
