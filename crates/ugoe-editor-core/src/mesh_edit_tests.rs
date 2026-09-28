//! メッシュの形の編集コマンド (`MoveMeshVertices` など) のテスト。式は `docs/editor.md#メッシュの編集`。

use ugoe_core::{Mat2, Vec2};
use ugoe_model::mesh::GluePair;
use ugoe_model::validate::validate;
use ugoe_model::{Mesh, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::{Command, Document, EditError, VertexMove};

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

#[track_caller]
fn assert_close(a: Vec2, b: Vec2) {
    assert!(a.abs_diff_eq(b, TOL), "{a} != {b}");
}

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model.meshes.iter().find(|m| m.id.as_str() == id).unwrap()
}

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model
        .meshes
        .iter_mut()
        .find(|m| m.id.as_str() == id)
        .unwrap()
}

/// 既定値のパラメータで評価したメッシュのキャンバス座標。
fn world(model: &Model, id: &str) -> Vec<Vec2> {
    let index = model
        .meshes
        .iter()
        .position(|m| m.id.as_str() == id)
        .unwrap();
    let instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    instance.drawables()[index].positions.clone()
}

fn moves(id: &str, moves: &[(u16, Vec2)]) -> Command {
    Command::MoveMeshVertices {
        id: id.into(),
        moves: moves
            .iter()
            .map(|&(index, delta)| VertexMove { index, delta })
            .collect(),
    }
}

fn add(id: &str, x: f32, y: f32) -> Command {
    Command::AddMeshVertex {
        id: id.into(),
        position: Vec2::new(x, y),
    }
}

fn remove(id: &str, indices: &[u16]) -> Command {
    Command::RemoveMeshVertices {
        id: id.into(),
        indices: indices.to_vec(),
    }
}

/// UV 空間での三角形の外積 (Y 下向きの画面で時計回りなら正)。
fn uv_cross(mesh: &Mesh) -> Vec<f32> {
    mesh.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| mesh.uvs[usize::from(t[k])]);
            (b - a).perp_dot(c - a)
        })
        .collect()
}

/// UV 空間での三角形の面積の合計。
fn uv_area(mesh: &Mesh) -> f32 {
    uv_cross(mesh).iter().map(|c| c.abs() / 2.0).sum()
}

/// 三角形がすべて同じ向き (元のフィクスチャと同じ、UV で時計回り) で、面積 0 のものがない。
#[track_caller]
fn assert_clockwise(mesh: &Mesh) {
    for c in uv_cross(mesh) {
        assert!(c > 1e-6, "{:?}", mesh.indices);
    }
}

// ---- JSON ----

#[test]
fn mesh_edit_commands_round_trip_json() {
    let command = moves("Quad", &[(2, Vec2::new(10.0, 5.0))]);
    let json = serde_json::to_value(&command).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "op": "move_mesh_vertices",
            "id": "Quad",
            "moves": [{ "index": 2, "delta": [10.0, 5.0] }],
        })
    );
    assert_eq!(serde_json::from_value::<Command>(json).unwrap(), command);
    for (command, op) in [
        (add("Quad", 1.0, 2.0), "add_mesh_vertex"),
        (remove("Quad", &[1]), "remove_mesh_vertices"),
        (
            Command::RetriangulateMesh { id: "Quad".into() },
            "retriangulate_mesh",
        ),
    ] {
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(json["op"], op);
        assert_eq!(serde_json::from_value::<Command>(json).unwrap(), command);
    }
}

// ---- 頂点の移動 ----

// 親なし: キャンバスのずれがそのまま局所のずれ。UV は基準の姿のテクスチャの対応で外挿する。
#[test]
fn move_vertices_of_root_mesh() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        moves(
            "Quad",
            &[(2, Vec2::new(16.0, 8.0)), (0, Vec2::new(32.0, 0.0))],
        ),
    );
    let (before, m) = (mesh(&model, "Quad"), mesh(&after, "Quad"));
    for (kb, ka) in before.keyforms.keyforms.iter().zip(&m.keyforms.keyforms) {
        assert_close(ka.positions[2], kb.positions[2] + Vec2::new(16.0, 8.0));
        assert_close(ka.positions[0], kb.positions[0] + Vec2::new(32.0, 0.0));
        assert_eq!(ka.positions[1], kb.positions[1]);
    }
    // 基準の姿は (64,64)〜(192,192) に UV (0,0)〜(1,1)。1px = 1/128
    assert_close(m.uvs[2], Vec2::new(1.0 + 16.0 / 128.0, 1.0 + 8.0 / 128.0));
    assert_close(m.uvs[0], Vec2::new(0.25, 0.0));
    assert_eq!(m.uvs[1], before.uvs[1]);
    assert_eq!(m.indices, before.indices);
}

// 親が回転デフォーマ: 線形部分 (回転と拡大率、祖先の合成) の逆で局所に直す。厳密。
#[test]
fn move_vertices_under_rotation_is_exact() {
    let mut model = fixtures::rotation_chain();
    // Arm を既定で 30 度回す (キー -30, 30 の中間より右)。Hand は拡大率 0.5。
    model.parameters[0].default = 15.0;
    let delta = Vec2::new(3.0, -2.0);
    let after = applied(&model, moves("Palm", &[(2, delta)]));
    let expected = Mat2::from_angle(15f32.to_radians()).inverse() * delta / 0.5;
    let (kb, ka) = (
        &mesh(&model, "Palm").keyforms.keyforms[0],
        &mesh(&after, "Palm").keyforms.keyforms[0],
    );
    assert_close(ka.positions[2], kb.positions[2] + expected);
    // 評価すると、キャンバスでちょうど delta だけ動く。
    let (wb, wa) = (world(&model, "Palm"), world(&after, "Palm"));
    assert_close(wa[2], wb[2] + delta);
    assert_close(wa[0], wb[0]);
}

// 親がワープ: その点での双線形写像のヤコビ行列の逆で局所 (UV) に直す。
#[test]
fn move_vertices_under_warp_uses_local_jacobian() {
    let mut model = fixtures::warp_grid();
    // 正方格子 (X = 0) では F(u,v) = (64 + 128u, 64 + 128v) なので 128px = 1
    let after = applied(&model, moves("Inner", &[(0, Vec2::new(12.8, -6.4))]));
    let (kb, ka) = (
        &mesh(&model, "Inner").keyforms.keyforms[0],
        &mesh(&after, "Inner").keyforms.keyforms[0],
    );
    assert_close(ka.positions[0], kb.positions[0] + Vec2::new(0.1, -0.05));

    // 台形の格子 (X = -1 を既定にする)。Inner の頂点 0 (UV (0.25,0.25)) は左上のセルの中心で、
    // P00 = (0,64)、P10 = (128,64)、P01 = (64,128)、P11 = (128,128)。
    // dF/du = 2 * (0.5 * (128,0) + 0.5 * (64,0)) = (192,0)
    // dF/dv = 2 * (0.5 * (64,64) + 0.5 * (0,64)) = (64,128)
    model.parameters[0].default = -1.0;
    let j = Mat2::from_cols(Vec2::new(192.0, 0.0), Vec2::new(64.0, 128.0));
    let delta = Vec2::new(4.0, 2.0);
    let after = applied(&model, moves("Inner", &[(0, delta)]));
    let local = mesh(&after, "Inner").keyforms.keyforms[0].positions[0]
        - mesh(&model, "Inner").keyforms.keyforms[0].positions[0];
    assert!((j * local).abs_diff_eq(delta, 1e-3), "{}", j * local);
    // 一次近似なので、評価した位置のずれは小さい (双線形の交差項の分だけ)。
    let (wb, wa) = (world(&model, "Inner"), world(&after, "Inner"));
    assert!((wa[0] - (wb[0] + delta)).length() < 0.1);
}

#[test]
fn move_vertices_rejects_bad_input() {
    let model = fixtures::single_quad();
    assert_eq!(
        rejected(&model, moves("Quad", &[(4, Vec2::X)])),
        EditError::VertexOutOfRange {
            id: "Quad".into(),
            index: 4,
            vertices: 4
        }
    );
    assert!(matches!(
        rejected(&model, moves("Quad", &[(1, Vec2::X), (1, Vec2::Y)])),
        EditError::MeshGeometry { .. }
    ));
    assert!(matches!(
        rejected(&model, moves("Quad", &[(1, Vec2::NAN)])),
        EditError::MeshGeometry { .. }
    ));
    assert_eq!(
        rejected(&model, moves("Nope", &[])),
        EditError::NotFound {
            kind: "mesh",
            id: "Nope".into()
        }
    );
}

// ---- 頂点の追加 ----

#[test]
fn add_vertex_inside_interpolates_every_keyform() {
    let mut model = fixtures::single_quad();
    mesh_mut(&mut model, "Quad").depth = Some(vec![0.0, 10.0, 20.0, 30.0]);
    let after = applied(&model, add("Quad", 128.0, 96.0));
    let m = mesh(&after, "Quad");
    assert_eq!(m.uvs.len(), 5);
    assert_close(m.uvs[4], Vec2::new(0.5, 0.25));
    // X = -1, 0, 1 のキーフォームで左右に 64px ずつずれる
    for (kf, dx) in m.keyforms.keyforms.iter().zip([-64.0, 0.0, 64.0]) {
        assert_close(kf.positions[4], Vec2::new(128.0 + dx, 96.0));
    }
    // 深度も重心座標で補間する (右上の三角形 0-1-2 の中: 0.5 * 0 + 0.25 * 10 + 0.25 * 20)
    let depth = m.depth.as_ref().unwrap();
    assert_eq!(depth.len(), 5);
    assert!((depth[4] - 7.5).abs() < TOL, "{depth:?}");
    // 内側の点なので輪郭は変わらず、4 枚の三角形になる
    assert_eq!(m.indices.len(), 12);
    assert!(m.indices.contains(&4));
    assert!((uv_area(m) - 1.0).abs() < 1e-5);
    assert_clockwise(m);
}

#[test]
fn add_vertex_outside_or_on_the_outline_extends_it() {
    let model = fixtures::single_quad();
    // 上辺 (y = 64) の外側: 五角形になる
    let after = applied(&model, add("Quad", 128.0, 32.0));
    let m = mesh(&after, "Quad");
    assert_close(m.uvs[4], Vec2::new(0.5, -0.25));
    assert_eq!(m.indices.len(), 9);
    assert!((uv_area(m) - 1.125).abs() < 1e-5);
    assert_clockwise(m);

    // 辺の上: 輪郭に載るだけで、面積は変わらない
    let after = applied(&model, add("Quad", 128.0, 64.0));
    let m = mesh(&after, "Quad");
    assert_close(m.uvs[4], Vec2::new(0.5, 0.0));
    assert_eq!(m.indices.len(), 9);
    assert!((uv_area(m) - 1.0).abs() < 1e-5);
    assert_clockwise(m);
}

// 親がワープでも、キャンバスの点を含む三角形の重心座標で局所座標を求める。
#[test]
fn add_vertex_under_warp() {
    let model = fixtures::warp_grid();
    // Inner は UV (0.25..0.75) → キャンバス (96..160)。中央の点
    let after = applied(&model, add("Inner", 128.0, 128.0));
    let m = mesh(&after, "Inner");
    assert_close(m.keyforms.keyforms[0].positions[4], Vec2::new(0.5, 0.5));
    assert_close(world(&after, "Inner")[4], Vec2::new(128.0, 128.0));
}

#[test]
fn add_vertex_rejects_duplicates() {
    let model = fixtures::single_quad();
    assert!(matches!(
        rejected(&model, add("Quad", 64.0, 64.0)),
        EditError::MeshGeometry { .. }
    ));
    assert!(matches!(
        rejected(&model, add("Quad", f32::NAN, 0.0)),
        EditError::MeshGeometry { .. }
    ));
}

// ---- 頂点の削除 ----

#[test]
fn remove_vertices_retriangulates_and_keeps_outline() {
    let mut model = fixtures::single_quad();
    mesh_mut(&mut model, "Quad").depth = Some(vec![0.0, 10.0, 20.0, 30.0]);
    // 内側の頂点を足してから消すと、元の 2 枚に戻る
    let with_center = applied(&model, add("Quad", 128.0, 128.0));
    let after = applied(&with_center, remove("Quad", &[4]));
    let m = mesh(&after, "Quad");
    assert_eq!(m.uvs, mesh(&model, "Quad").uvs);
    assert_eq!(m.indices.len(), 6);
    assert!((uv_area(m) - 1.0).abs() < 1e-5);
    assert_eq!(m.depth.as_deref(), Some(&[0.0, 10.0, 20.0, 30.0][..]));
    assert_clockwise(m);

    // 角を消すと三角形 1 枚。後ろの頂点は番号が詰まる
    let after = applied(&model, remove("Quad", &[1]));
    let m = mesh(&after, "Quad");
    let before = mesh(&model, "Quad");
    assert_eq!(m.uvs, vec![before.uvs[0], before.uvs[2], before.uvs[3]]);
    for (ka, kb) in m.keyforms.keyforms.iter().zip(&before.keyforms.keyforms) {
        assert_eq!(
            ka.positions,
            vec![kb.positions[0], kb.positions[2], kb.positions[3]]
        );
    }
    assert_eq!(m.depth.as_deref(), Some(&[0.0, 20.0, 30.0][..]));
    assert_eq!(m.indices.len(), 3);
    assert_clockwise(m);
}

/// 四角形 (64,64)〜(192,192) の上に、外側へ頂点を足して M 字の輪郭にしたもの。
/// 輪郭は 0 (64,64) → 5 (96,16) → 4 (128,56) → 6 (160,16) → 1 (192,64) → 2 → 3 で、4 が凹み。
/// UV の面積は 1 + 0.203125 (凸包なら 1 + 0.28125)。
fn m_shape() -> Model {
    let mut model = fixtures::single_quad();
    for (x, y) in [(128.0, 56.0), (96.0, 16.0), (160.0, 16.0)] {
        model = applied(&model, add("Quad", x, y));
    }
    let m = mesh(&model, "Quad");
    assert_close(m.uvs[4], Vec2::new(0.5, -0.0625));
    assert_close(m.uvs[5], Vec2::new(0.25, -0.375));
    assert_close(m.uvs[6], Vec2::new(0.75, -0.375));
    assert!((uv_area(m) - 1.203125).abs() < 1e-5, "{}", uv_area(m));
    model
}

// 凹んだ輪郭は、作り直しても頂点を消しても埋めない。
#[test]
fn concave_outline_is_kept() {
    let model = m_shape();
    let after = applied(&model, Command::RetriangulateMesh { id: "Quad".into() });
    let m = mesh(&after, "Quad");
    assert!((uv_area(m) - 1.203125).abs() < 1e-5, "{}", uv_area(m));
    assert_clockwise(m);

    // 右下の角 (三角形 1-2-3、面積 0.5) を消す
    let after = applied(&model, remove("Quad", &[2]));
    let m = mesh(&after, "Quad");
    assert!((uv_area(m) - 0.703125).abs() < 1e-5, "{}", uv_area(m));
    assert_clockwise(m);

    // 凹みの頂点を消すと、前後 (5 と 6) を直接つなぐので凹みは埋まる
    let after = applied(&model, remove("Quad", &[4]));
    let m = mesh(&after, "Quad");
    assert!((uv_area(m) - 1.28125).abs() < 1e-5, "{}", uv_area(m));
}

#[test]
fn remove_vertices_fixes_glue_pairs() {
    let model = fixtures::glued_quads();
    let after = applied(&model, remove("Left", &[1]));
    let pair = |a, b, weight| GluePair { a, b, weight };
    // Seam: (1,0) は消え、(2,3) は (1,3) に。Pull: (1,1) は消える
    assert_eq!(after.glues[0].pairs, vec![pair(1, 3, 0.5)]);
    assert_eq!(after.glues[1].pairs, vec![]);

    let after = applied(&model, remove("Right", &[0]));
    assert_eq!(after.glues[0].pairs, vec![pair(2, 2, 0.5)]);
    assert_eq!(after.glues[1].pairs, vec![pair(1, 0, 0.0)]);
}

#[test]
fn remove_vertices_rejects_too_few_and_out_of_range() {
    let model = fixtures::single_quad();
    assert_eq!(
        rejected(&model, remove("Quad", &[0, 1])),
        EditError::TooFewVertices {
            id: "Quad".into(),
            remaining: 2
        }
    );
    assert_eq!(
        rejected(&model, remove("Quad", &[9])),
        EditError::VertexOutOfRange {
            id: "Quad".into(),
            index: 9,
            vertices: 4
        }
    );
}

// ---- 三角形の作り直し ----

#[test]
fn retriangulate_keeps_outline_and_winding() {
    let mut model = fixtures::single_quad();
    for (x, y) in [(128.0, 32.0), (100.0, 100.0), (150.0, 170.0)] {
        model = applied(&model, add("Quad", x, y));
    }
    let area = uv_area(mesh(&model, "Quad"));
    assert!((area - 1.125).abs() < 1e-5);
    let after = applied(&model, Command::RetriangulateMesh { id: "Quad".into() });
    let m = mesh(&after, "Quad");
    assert!((uv_area(m) - area).abs() < 1e-5);
    assert_clockwise(m);
    // 三角形の向きが元と逆 (画面で反時計回り) なら、それに合わせる
    let mut ccw = model.clone();
    for t in mesh_mut(&mut ccw, "Quad").indices.chunks_mut(3) {
        t.swap(1, 2);
    }
    let after = applied(&ccw, Command::RetriangulateMesh { id: "Quad".into() });
    assert!(uv_cross(mesh(&after, "Quad")).iter().all(|&c| c < 0.0));
}

#[test]
fn retriangulate_rejects_duplicate_uvs() {
    let mut model = fixtures::single_quad();
    let m = mesh_mut(&mut model, "Quad");
    m.uvs[3] = m.uvs[0];
    assert!(matches!(
        rejected(&model, Command::RetriangulateMesh { id: "Quad".into() }),
        EditError::MeshGeometry { .. }
    ));
}

// 頂点の編集は格子の `derived` のセルを作り直す (`docs/spec/keyform-inference.md`)。
#[test]
fn vertex_edits_rederive_derived_cells() {
    use ugoe_model::KeyformOrigin;

    use crate::{InferMethod, KeyformTarget};

    let mut model = fixtures::single_quad();
    // X = 1 のセルを `derived` にし、推定の結果に合わせておく
    mesh_mut(&mut model, "Quad").keyforms.origins = vec![
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    let infer = Command::InferKeyforms {
        target: KeyformTarget::Mesh("Quad".into()),
        method: InferMethod::FromGrid,
    };
    let model = applied(&model, infer.clone());
    for command in [
        moves("Quad", &[(2, Vec2::new(16.0, 8.0))]),
        add("Quad", 128.0, 96.0),
        remove("Quad", &[1]),
    ] {
        let after = applied(&model, command);
        // 作り直し済みなので、もう一度推定しても変わらない
        assert_eq!(applied(&after, infer.clone()), after);
    }
}
