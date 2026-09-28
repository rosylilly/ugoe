//! ブレンドシェイプのパラメータと格子の編集のテスト (`docs/editor.md#ブレンドシェイプ`)。

use ugoe_core::Vec2;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::parameter::ParameterKind;
use ugoe_model::{Model, Parameter, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::{CellError, Command, Document, EditError, Keyform, KeyformCell, KeyformTarget};

fn quad() -> KeyformTarget {
    KeyformTarget::Mesh("Quad".into())
}

fn cell(entries: &[(&str, f32)]) -> KeyformCell {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

fn blend_shape_parameter(id: &str) -> Command {
    Command::AddParameter {
        parameter: Parameter {
            id: id.into(),
            name: id.into(),
            min: 0.0,
            max: 1.0,
            default: 0.0,
            kind: ParameterKind::BlendShape,
            mirror: None,
            group: None,
            internal: false,
        },
    }
}

fn add_key(parameter: &str, key: f32) -> Command {
    Command::AddKeyformKey {
        target: quad(),
        parameter: parameter.into(),
        key,
    }
}

fn remove_key(parameter: &str, key: f32) -> Command {
    Command::RemoveKeyformKey {
        target: quad(),
        parameter: parameter.into(),
        key,
    }
}

/// `single_quad` にブレンドシェイプのパラメータ `Smile` と `Wide` を加えた文書。
fn document() -> Document {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(blend_shape_parameter("Smile")).unwrap();
    doc.apply(blend_shape_parameter("Wide")).unwrap();
    doc
}

fn quad_grid(model: &Model) -> &ugoe_model::KeyformGrid<MeshKeyform> {
    &model.meshes[0].keyforms
}

/// `Smile = 1` の形: 基準の形の頂点 0 を `(-8, -16)` ずらしたもの。
fn smiling(model: &Model) -> MeshKeyform {
    let mut kf = quad_grid(model).keyforms[1].clone();
    kf.positions[0] += Vec2::new(-8.0, -16.0);
    kf
}

fn positions(model: &Model, values: &[(&str, f32)]) -> Vec<Vec2> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for &(id, v) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, v);
    }
    instance.update();
    instance.drawables()[0].positions.clone()
}

// ブレンドシェイプのパラメータのキーは、通常の格子ではなくブレンドシェイプ格子に加える。
#[test]
fn add_key_creates_a_blend_shape_grid_with_the_rest_shape() {
    let mut doc = document();
    let before = doc.model().clone();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    let grid = quad_grid(doc.model());
    assert_eq!(grid.bindings, quad_grid(&before).bindings);
    assert_eq!(grid.keyforms, quad_grid(&before).keyforms);
    let [shape] = &grid.blend_shapes[..] else {
        panic!("expected one blend shape grid");
    };
    // キーは既定値と指定したキー。セルはどちらも基準の形 (X = 0 のセル)。
    assert_eq!(shape.bindings[0].parameter.as_str(), "Smile");
    assert_eq!(shape.bindings[0].keys, vec![0.0, 1.0]);
    assert!(shape.keyforms.iter().all(|k| *k == grid.keyforms[1]));
    // 差分は 0 なので見た目は変わらない。
    assert_eq!(
        positions(doc.model(), &[("Smile", 1.0)]),
        positions(&before, &[])
    );

    // 同じパラメータのキーは同じ格子へ、別のパラメータは新しい格子へ。
    doc.apply(add_key("Smile", 0.5)).unwrap();
    doc.apply(add_key("Wide", 1.0)).unwrap();
    let shapes = &quad_grid(doc.model()).blend_shapes;
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].bindings[0].keys, vec![0.0, 0.5, 1.0]);
    assert_eq!(shapes[1].bindings[0].parameter.as_str(), "Wide");
}

// 基準の形は、通常の格子を既定値で補間した形 (既定値がキーでなくてもよい)。
#[test]
fn rest_shape_interpolates_the_normal_grid_at_defaults() {
    let mut model = fixtures::single_quad();
    model.parameters[0].default = 0.5;
    let mut doc = Document::new(model);
    doc.apply(blend_shape_parameter("Smile")).unwrap();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    let shape = &quad_grid(doc.model()).blend_shapes[0];
    assert_eq!(shape.keyforms[0].positions[0], Vec2::new(96.0, 64.0));
}

// セルの指定にブレンドシェイプのパラメータを含めると、ブレンドシェイプ格子のセルを置き換える。
#[test]
fn set_cell_routes_blend_shape_cells() {
    let mut doc = document();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    let target = smiling(doc.model());
    doc.apply(Command::SetKeyformCell {
        target: quad(),
        cell: cell(&[("Smile", 1.0)]),
        keyform: Keyform::Mesh(target.clone()),
    })
    .unwrap();
    assert_eq!(quad_grid(doc.model()).blend_shapes[0].keyforms[1], target);
    // 通常の格子と組み合わせても、差分として加わる (X = 1 で +64 平行移動)。
    let p = positions(doc.model(), &[("Smile", 1.0), ("X", 1.0)]);
    assert_eq!(p[0], Vec2::new(120.0, 48.0));

    // SetMeshKeyform も同じ規則で振り分ける。
    let mut half = target.clone();
    half.opacity = 0.5;
    doc.apply(Command::SetMeshKeyform {
        id: "Quad".into(),
        cell: cell(&[("Smile", 1.0)]),
        keyform: half.clone(),
    })
    .unwrap();
    assert_eq!(quad_grid(doc.model()).blend_shapes[0].keyforms[1], half);

    // ブレンドシェイプ格子のないパラメータ・軸の足りない指定は拒否する。
    let set = |cell| Command::SetKeyformCell {
        target: quad(),
        cell,
        keyform: Keyform::Mesh(target.clone()),
    };
    let invalid = |error| EditError::InvalidCell {
        kind: "mesh",
        id: "Quad".into(),
        error,
    };
    assert_eq!(
        doc.apply(set(cell(&[("Wide", 1.0)]))),
        Err(invalid(CellError::UnknownAxis("Wide".into())))
    );
    assert_eq!(
        doc.apply(set(cell(&[("Smile", 1.0), ("X", 0.0)]))),
        Err(invalid(CellError::UnknownAxis("X".into())))
    );
}

// 最後のキーを取り除くと、軸のなくなったブレンドシェイプ格子ごと取り除く。
#[test]
fn remove_key_drops_empty_blend_shape_grids() {
    let mut doc = document();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    doc.apply(remove_key("Smile", 1.0)).unwrap();
    assert_eq!(
        quad_grid(doc.model()).blend_shapes[0].bindings[0].keys,
        [0.0]
    );
    doc.apply(remove_key("Smile", 0.0)).unwrap();
    assert!(quad_grid(doc.model()).blend_shapes.is_empty());
    assert_eq!(
        doc.apply(remove_key("Smile", 0.0)),
        Err(EditError::InvalidCell {
            kind: "mesh",
            id: "Quad".into(),
            error: CellError::UnknownAxis("Smile".into()),
        })
    );
}

// パラメータを削除すると、そのブレンドシェイプ格子は (差分が常に 0 になるので) 取り除く。
#[test]
fn remove_parameter_drops_its_blend_shape_grid() {
    let mut doc = document();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    doc.apply(add_key("Wide", 1.0)).unwrap();
    doc.apply(Command::RemoveParameter { id: "Smile".into() })
        .unwrap();
    let shapes = &quad_grid(doc.model()).blend_shapes;
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].bindings[0].parameter.as_str(), "Wide");
}

// 頂点数を変える編集は、ブレンドシェイプ格子のセルにも同じ操作をする。
#[test]
fn vertex_edits_keep_blend_shape_cells_in_sync() {
    let mut doc = document();
    doc.apply(add_key("Smile", 1.0)).unwrap();
    let target = smiling(doc.model());
    doc.apply(Command::SetKeyformCell {
        target: quad(),
        cell: cell(&[("Smile", 1.0)]),
        keyform: Keyform::Mesh(target),
    })
    .unwrap();
    doc.apply(Command::AddMeshVertex {
        id: "Quad".into(),
        position: Vec2::new(128.0, 128.0),
    })
    .unwrap();
    let grid = quad_grid(doc.model());
    assert!(grid.all_keyforms().all(|k| k.positions.len() == 5));
    // 足した頂点は各セルで三角形の重心座標から補間する (基準の形のセルでは指定した位置)。
    let shape = &grid.blend_shapes[0];
    assert_eq!(shape.keyforms[0].positions[4], Vec2::new(128.0, 128.0));

    doc.apply(Command::RemoveMeshVertices {
        id: "Quad".into(),
        indices: vec![4],
    })
    .unwrap();
    assert!(
        quad_grid(doc.model())
            .all_keyforms()
            .all(|k| k.positions.len() == 4)
    );

    // 描画順を揃えると、ブレンドシェイプ格子の描画順の差分もなくなる。
    doc.apply(Command::SetMeshDrawOrder {
        id: "Quad".into(),
        draw_order: 42.0,
    })
    .unwrap();
    assert!(
        quad_grid(doc.model())
            .all_keyforms()
            .all(|k| k.draw_order == 42.0)
    );
}

// 通常の格子にブレンドシェイプのパラメータの軸を持たせる置き換えは、検証で拒否する。
#[test]
fn blend_shape_axis_in_the_normal_grid_is_rejected() {
    let mut doc = document();
    let mut keyforms = quad_grid(doc.model()).clone();
    keyforms.bindings[0].parameter = "Smile".into();
    let result = doc.apply(Command::SetMeshKeyforms {
        id: "Quad".into(),
        keyforms,
    });
    assert!(matches!(result, Err(EditError::Invalid(_))), "{result:?}");
}
