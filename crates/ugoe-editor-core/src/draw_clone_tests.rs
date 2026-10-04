//! 描画用の複製 (`CreateDrawClone`・`SyncDrawClone`) のテスト。規則は `docs/editor.md#描画用の複製`。

use ugoe::Puppet;
use ugoe_core::Vec2;
use ugoe_model::validate::validate;
use ugoe_model::{Mesh, MeshId, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance, ParamIndex};

use crate::draw_clone::{clones_of, glue_id, is_in_sync, original_of};
use crate::{Command, Document, EditError, VertexMove};

/// 複製の頂点と元の頂点の許容誤差 (ピクセル)。グルーの `pb + (pa - pb) * 1` は丸めで `pa` と
/// 1 ulp 程度ずれうる (`docs/editor.md#描画用の複製`)。
const TOL: f32 = 1e-3;

// ---- 補助 (tests.rs と同じ規則) ----

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

fn create(source: &str) -> Command {
    Command::CreateDrawClone {
        source: source.into(),
        id: None,
        name: String::new(),
    }
}

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model.meshes.iter().find(|m| m.id.as_str() == id).unwrap()
}

fn index(model: &Model, id: &str) -> usize {
    model
        .meshes
        .iter()
        .position(|m| m.id.as_str() == id)
        .unwrap()
}

#[track_caller]
fn assert_same_positions(a: &[Vec2], b: &[Vec2], context: &str) {
    assert_eq!(a.len(), b.len(), "{context}");
    for (p, q) in a.iter().zip(b) {
        assert!(p.abs_diff_eq(*q, TOL), "{context}: {p} != {q}");
    }
}

/// パラメータを 1 つずつ最小・最大・中間に動かした姿で、複製が元と同じ位置にあることを確かめる。
#[track_caller]
fn assert_follows_parameters(model: &Model, original: &str, clone: &str) {
    let (a, b) = (index(model, original), index(model, clone));
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    let mut samples = vec![None];
    for (i, p) in model.parameters.iter().enumerate() {
        for v in [p.min, p.max, (p.min + p.max) * 0.5 + 0.1 * (p.max - p.min)] {
            samples.push(Some((i, v)));
        }
    }
    for sample in samples {
        instance.parameters_mut().reset();
        if let Some((i, v)) = sample {
            instance.parameters_mut().set(ParamIndex(i as u32), v);
        }
        instance.update();
        let d = instance.drawables();
        assert_same_positions(&d[b].positions, &d[a].positions, &format!("{sample:?}"));
    }
}

// ---- 作成 ----

#[test]
fn create_copies_topology_and_glues_every_vertex() {
    let model = fixtures::rotation_chain();
    let after = applied(&model, create("Sleeve"));
    let original = mesh(&after, "Sleeve");
    let clone = mesh(&after, "Sleeve_Clone");
    assert_eq!(clone.name, format!("{}の複製", original.name));
    assert_eq!(clone.uvs, original.uvs);
    assert_eq!(clone.indices, original.indices);
    assert_eq!(clone.texture, original.texture);
    assert_eq!(clone.parent, original.parent);
    assert_eq!(clone.part, original.part);
    assert_eq!(clone.masks, original.masks);
    // キーフォームは基準の形 1 つ。
    assert!(clone.keyforms.bindings.is_empty());
    assert_eq!(clone.keyforms.keyforms.len(), 1);
    assert_eq!(
        clone.keyforms.keyforms[0].draw_order,
        original.keyforms.keyforms[0].draw_order
    );
    // 元の直後に置く。
    assert_eq!(index(&after, "Sleeve_Clone"), index(&after, "Sleeve") + 1);
    // グルーは末尾で、全頂点を weight 0・強さ 1 で張る。
    let glue = after.glues.last().unwrap();
    assert_eq!(glue.id.as_str(), "DrawClone_Sleeve_Clone");
    assert_eq!(glue.mesh_a.as_str(), "Sleeve");
    assert_eq!(glue.mesh_b.as_str(), "Sleeve_Clone");
    assert_eq!(glue.pairs.len(), original.uvs.len());
    assert!(
        glue.pairs
            .iter()
            .enumerate()
            .all(|(i, p)| usize::from(p.a) == i && p.a == p.b && p.weight == 0.0)
    );
    assert_eq!(glue.keyforms.keyforms[0].intensity, 1.0);

    let clone_id = MeshId::new("Sleeve_Clone");
    assert_eq!(original_of(&after, &clone_id).unwrap().as_str(), "Sleeve");
    assert_eq!(clones_of(&after, &"Sleeve".into()), vec![&clone_id]);
    assert!(is_in_sync(&after, &clone_id));
}

#[test]
fn create_uses_given_id_and_name_and_numbers_default_ids() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::CreateDrawClone {
            source: "Palm".into(),
            id: Some("PalmFront".into()),
            name: "手のひら (前)".into(),
        },
    );
    assert_eq!(mesh(&after, "PalmFront").name, "手のひら (前)");
    assert!(
        after
            .glues
            .iter()
            .any(|g| g.id.as_str() == "DrawClone_PalmFront")
    );

    let twice = applied(&applied(&model, create("Palm")), create("Palm"));
    assert!(twice.meshes.iter().any(|m| m.id.as_str() == "Palm_Clone"));
    assert!(twice.meshes.iter().any(|m| m.id.as_str() == "Palm_Clone_2"));
    assert_eq!(clones_of(&twice, &"Palm".into()).len(), 2);
}

#[test]
fn cloning_a_clone_clones_the_original() {
    let model = applied(&fixtures::rotation_chain(), create("Palm"));
    let after = applied(&model, create("Palm_Clone"));
    let glue = after.glues.last().unwrap();
    assert_eq!(glue.mesh_a.as_str(), "Palm");
    assert_eq!(glue.mesh_b.as_str(), "Palm_Clone_2");
}

#[test]
fn create_rejects_unknown_source_and_duplicate_id() {
    let model = fixtures::rotation_chain();
    assert!(matches!(
        rejected(&model, create("Nope")),
        EditError::NotFound { kind: "mesh", .. }
    ));
    assert!(matches!(
        rejected(
            &model,
            Command::CreateDrawClone {
                source: "Palm".into(),
                id: Some("Sleeve".into()),
                name: String::new(),
            }
        ),
        EditError::DuplicateId { kind: "mesh", .. }
    ));
}

// ---- ランタイムでの位置 ----

#[test]
fn clone_renders_at_original_positions_under_parameters() {
    for (model, meshes) in [
        (fixtures::rotation_chain(), &["Palm", "Sleeve"][..]),
        (fixtures::rotation_under_warp(), &["Face", "Blush"][..]),
        (fixtures::keyform_inference(), &[][..]),
        (fixtures::blend_shapes(), &[][..]),
    ] {
        let names: Vec<String> = if meshes.is_empty() {
            model.meshes.iter().map(|m| m.id.to_string()).collect()
        } else {
            meshes.iter().map(|s| (*s).to_owned()).collect()
        };
        for name in names {
            let after = applied(&model, create(&name));
            assert_follows_parameters(&after, &name, &format!("{name}_Clone"));
        }
    }
}

/// 元が他のメッシュとグルーでつながっていても、複製はグルーの後の位置に一致する
/// (複製のグルーは `glues` の末尾に置く)。
#[test]
fn clone_follows_a_glued_original() {
    let model = applied(&fixtures::glued_quads(), create("Left"));
    assert_follows_parameters(&model, "Left", "Left_Clone");
    // 既定値 (Glue = 1) では継ぎ目の頂点が引き寄せられている。
    let instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    let d = instance.drawables();
    let (left, clone) = (index(&model, "Left"), index(&model, "Left_Clone"));
    let rest = &mesh(&model, "Left_Clone").keyforms.keyforms[0].positions;
    assert!(!d[left].positions[1].abs_diff_eq(rest[1], TOL));
    assert_same_positions(&d[clone].positions, &d[left].positions, "rest");
}

/// 動的物理の加算量 (デフォーマの角度・メッシュの頂点) はグルーの前に加わるので、
/// 複製にもそのまま伝わる (`docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`)。
#[test]
fn clone_follows_dynamic_physics_offsets() {
    let mut model = fixtures::dynamic_hair();
    for id in ["Ribbon", "Strand"] {
        model = applied(&model, create(id));
    }
    let mut puppet = Puppet::new(model.clone())
        .unwrap()
        .with_physics(fixtures::dynamic_hair_settings());
    let angle_x = puppet.instance().parameters().index_of("AngleX").unwrap();
    let rest = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    let mut moved = [false; 2];
    for frame in 0..90 {
        let value = if frame < 30 { 30.0 } else { -30.0 };
        puppet.instance_mut().parameters_mut().set(angle_x, value);
        puppet.update(1.0 / 60.0);
        let d = puppet.instance().drawables();
        for (k, id) in ["Ribbon", "Strand"].into_iter().enumerate() {
            let (a, b) = (index(&model, id), index(&model, &format!("{id}_Clone")));
            assert_same_positions(&d[b].positions, &d[a].positions, &format!("{id} {frame}"));
            // 物理で元の頂点が既定の姿から動いたことも確かめる (テストが空振りしないように)。
            if !d[a].positions[3].abs_diff_eq(rest.drawables()[a].positions[3], 0.5) {
                moved[k] = true;
            }
        }
    }
    assert_eq!(moved, [true, true]);
    // `Ribbon` は頂点の加算量を受ける。
    let offsets = puppet.instance().dynamic_offsets().unwrap();
    let ribbon = index(&model, "Ribbon");
    assert!(
        offsets
            .mesh_positions(ribbon)
            .iter()
            .any(|o| o.length() > 0.01)
    );
}

// ---- 元の形の編集への追従 ----

#[track_caller]
fn assert_synced(model: &Model, original: &str, clone: &str) {
    assert!(is_in_sync(model, &clone.into()), "{clone} drifted");
    let (o, c) = (mesh(model, original), mesh(model, clone));
    assert_eq!(c.uvs, o.uvs);
    assert_eq!(c.indices, o.indices);
    assert_follows_parameters(model, original, clone);
}

#[test]
fn clone_follows_vertex_edits_of_the_original() {
    let model = applied(&fixtures::rotation_chain(), create("Palm"));
    // 追加 (内部の点と輪郭の外の点)。`Palm` はキャンバスで (192,128)〜(200,136)。
    let added = applied(
        &model,
        Command::AddMeshVertex {
            id: "Palm".into(),
            position: Vec2::new(196.0, 132.0),
        },
    );
    assert_eq!(mesh(&added, "Palm_Clone").uvs.len(), 5);
    assert_synced(&added, "Palm", "Palm_Clone");

    let removed = applied(
        &added,
        Command::RemoveMeshVertices {
            id: "Palm".into(),
            indices: vec![0],
        },
    );
    assert_eq!(mesh(&removed, "Palm_Clone").uvs.len(), 4);
    assert_synced(&removed, "Palm", "Palm_Clone");

    let moved = applied(
        &removed,
        Command::MoveMeshVertices {
            id: "Palm".into(),
            moves: vec![VertexMove {
                index: 0,
                delta: Vec2::new(1.0, 0.5),
            }],
        },
    );
    assert_ne!(mesh(&moved, "Palm").uvs, mesh(&removed, "Palm").uvs);
    assert_synced(&moved, "Palm", "Palm_Clone");

    let retriangulated = applied(&moved, Command::RetriangulateMesh { id: "Palm".into() });
    assert_synced(&retriangulated, "Palm", "Palm_Clone");
}

#[test]
fn clone_follows_texture_and_parent_of_the_original() {
    let model = applied(&fixtures::rotation_chain(), create("Sleeve"));
    let reparented = applied(
        &model,
        Command::ReparentPreservingLook {
            element: crate::ChildRef::Mesh("Sleeve".into()),
            new_parent: Some("Arm".into()),
        },
    );
    assert_eq!(
        mesh(&reparented, "Sleeve_Clone")
            .parent
            .as_ref()
            .unwrap()
            .as_str(),
        "Arm"
    );
    assert_synced(&reparented, "Sleeve", "Sleeve_Clone");
}

#[test]
fn shape_edits_of_a_clone_are_rejected() {
    let model = applied(&fixtures::rotation_chain(), create("Palm"));
    for command in [
        Command::AddMeshVertex {
            id: "Palm_Clone".into(),
            position: Vec2::new(196.0, 132.0),
        },
        Command::RemoveMeshVertices {
            id: "Palm_Clone".into(),
            indices: vec![0],
        },
        Command::MoveMeshVertices {
            id: "Palm_Clone".into(),
            moves: vec![],
        },
        Command::RetriangulateMesh {
            id: "Palm_Clone".into(),
        },
        Command::RegenerateMesh {
            id: "Palm_Clone".into(),
            options: Default::default(),
        },
    ] {
        assert_eq!(
            rejected(&model, command),
            EditError::DrawCloneShape {
                id: "Palm_Clone".into(),
                original: "Palm".into(),
            }
        );
    }
}

/// 複製には描画順・マスク・不透明度を持たせられ、それは元の形の編集で変わらない。
#[test]
fn clone_keeps_its_own_draw_order_masks_and_opacity() {
    let mut model = applied(&fixtures::rotation_chain(), create("Palm"));
    for command in [
        Command::SetMeshDrawOrder {
            id: "Palm_Clone".into(),
            draw_order: 900.0,
        },
        Command::SetMeshProperties {
            id: "Palm_Clone".into(),
            part: None,
            texture: None,
            blend: None,
            culling: None,
            masks: Some(vec![ugoe_model::MaskRef::inverted("Sleeve")]),
            mask_mode: None,
            mask_only: None,
            lit: None,
        },
    ] {
        model = applied(&model, command);
    }
    let mut keyform = mesh(&model, "Palm_Clone").keyforms.keyforms[0].clone();
    keyform.opacity = 0.5;
    model = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Palm_Clone".into(),
            cell: Default::default(),
            keyform,
        },
    );
    let after = applied(
        &model,
        Command::AddMeshVertex {
            id: "Palm".into(),
            position: Vec2::new(196.0, 132.0),
        },
    );
    let clone = mesh(&after, "Palm_Clone");
    assert_eq!(clone.keyforms.keyforms[0].draw_order, 900.0);
    assert_eq!(clone.keyforms.keyforms[0].opacity, 0.5);
    assert_eq!(clone.masks, vec![ugoe_model::MaskRef::inverted("Sleeve")]);
    assert_synced(&after, "Palm", "Palm_Clone");
}

// ---- ずれの検出と合わせ直し ----

#[test]
fn sync_repairs_a_drifted_clone() {
    let mut model = applied(&fixtures::glued_quads(), create("Left"));
    // 読み込んだデータなどで食い違った状態を作る: 複製の UV・グルーのペア・強さ・順序。
    let clone = model
        .meshes
        .iter_mut()
        .find(|m| m.id.as_str() == "Left_Clone")
        .unwrap();
    clone.uvs[0] = Vec2::new(0.5, 0.5);
    let glue = model.glues.pop().unwrap();
    let mut glue = glue.clone();
    glue.pairs.truncate(2);
    glue.keyforms.keyforms[0].intensity = 0.5;
    model.glues.insert(0, glue);
    assert_eq!(validate(&model), vec![]);
    assert!(!is_in_sync(&model, &"Left_Clone".into()));

    // 元を指定しても複製を指定しても同じ。
    for id in ["Left", "Left_Clone"] {
        let after = applied(&model, Command::SyncDrawClone { id: id.into() });
        assert_eq!(
            after.glues.last().unwrap().id,
            glue_id(&"Left_Clone".into())
        );
        assert_synced(&after, "Left", "Left_Clone");
    }
}

#[test]
fn sync_rejects_meshes_without_clones() {
    let model = fixtures::rotation_chain();
    assert_eq!(
        rejected(&model, Command::SyncDrawClone { id: "Palm".into() }),
        EditError::NotDrawClone { id: "Palm".into() }
    );
    assert!(matches!(
        rejected(&model, Command::SyncDrawClone { id: "Nope".into() }),
        EditError::NotFound { kind: "mesh", .. }
    ));
}

#[test]
fn removing_the_original_leaves_a_plain_mesh() {
    let model = applied(&fixtures::rotation_chain(), create("Palm"));
    let after = applied(&model, Command::RemoveMesh { id: "Palm".into() });
    assert!(after.glues.is_empty());
    assert!(original_of(&after, &"Palm_Clone".into()).is_none());
    // 複製は基準の形のまま残る。
    assert_eq!(
        mesh(&after, "Palm_Clone").keyforms,
        mesh(&model, "Palm_Clone").keyforms
    );
}
