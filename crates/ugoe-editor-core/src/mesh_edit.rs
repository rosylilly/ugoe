//! メッシュの形と三角形の編集 (頂点の移動・追加・削除と三角形の作り直し)。
//! 設計と式は `docs/editor.md#メッシュの編集`。
//!
//! どの操作も **既定値のパラメータ** で評価したメッシュ (以下「基準の姿」) を基準にする。
//! キャンバス座標の入力は、基準の姿での親デフォーマの変換で局所座標に直し、
//! すべてのキーフォームに同じ量を加える。

use std::collections::HashMap;

use ugoe_core::{Mat2, Rgb, Vec2};
use ugoe_mesher::{MAX_VERTICES, TriangulateError, triangulate_points};
use ugoe_model::deformer::DeformerKind;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{KeyformGrid, Mesh, MeshId, Model};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::EditError;
use crate::command::VertexMove;
use crate::ids::new_mesh_id;

/// 重心座標が「三角形の内側 (辺の上を含む)」とみなす下限。
const INSIDE: f64 = -1e-6;
/// 重心座標がこれ以下なら、点はその頂点の対辺の上にあるとみなす。
const ON_EDGE: f64 = 1e-6;

fn geometry(id: &MeshId, reason: impl Into<String>) -> EditError {
    EditError::MeshGeometry {
        id: id.to_string(),
        reason: reason.into(),
    }
}

fn mesh_index(model: &Model, id: &MeshId) -> Result<usize, EditError> {
    model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })
}

fn check_vertex(mesh: &Mesh, index: u16) -> Result<(), EditError> {
    if usize::from(index) < mesh.uvs.len() {
        return Ok(());
    }
    Err(EditError::VertexOutOfRange {
        id: mesh.id.to_string(),
        index,
        vertices: mesh.uvs.len(),
    })
}

// ---- 基準の姿の評価 ----

/// 基準の姿での頂点位置。
struct Rest {
    /// キーフォームを補間した局所座標 (親デフォーマの座標系)。
    local: Vec<Vec2>,
    /// キャンバス座標 (グルーの適用後。ビューポートに既定値で表示される位置)。
    world: Vec<Vec2>,
}

/// `model` に `probes` を足して既定値のパラメータで評価し、足したメッシュの頂点位置を返す。
fn evaluate_probes(
    model: &Model,
    id: &MeshId,
    target: usize,
    probes: Vec<Mesh>,
) -> Result<(Vec<Vec2>, Vec<Vec<Vec2>>), EditError> {
    let first = model.meshes.len();
    let mut probed = model.clone();
    probed.meshes.extend(probes);
    let compiled = CompiledModel::compile(probed)
        .map_err(|e| geometry(id, format!("the model cannot be evaluated: {e}")))?;
    let instance = ModelInstance::new(compiled);
    let drawables = instance.drawables();
    let target = drawables[target].positions.clone();
    let probes = drawables[first..]
        .iter()
        .map(|d| d.positions.clone())
        .collect();
    Ok((target, probes))
}

/// 評価用に足す、描画に関わらないメッシュの複製。
fn probe_of(model: &Model, mesh: &Mesh, n: usize) -> Mesh {
    let mut probe = mesh.clone();
    probe.id = new_mesh_id(model, &format!("__ugoe_probe_{n}"));
    probe.masks.clear();
    probe.mirror = None;
    probe.mirror_axis = None;
    probe.depth = None;
    probe
}

/// メッシュ `index` の基準の姿。
fn rest(model: &Model, index: usize) -> Result<Rest, EditError> {
    let mesh = &model.meshes[index];
    // 親を外した複製の出力は、補間しただけの局所座標になる。
    let mut local = probe_of(model, mesh, 0);
    local.parent = None;
    let (world, mut probes) = evaluate_probes(model, &mesh.id, index, vec![local])?;
    Ok(Rest {
        local: probes.pop().unwrap_or_default(),
        world,
    })
}

/// 局所座標の点 `at` のそれぞれでの、親デフォーマの変換 (局所 → キャンバス) のヤコビ行列。
///
/// 親なしは単位行列。親が回転デフォーマ (アフィン) なら中心差分は厳密に線形部分になる。
/// 親がワープなら、その点での双線形写像の微分 (格子の外では外挿のアフィン近似の微分) になる。
fn parent_jacobians(model: &Model, index: usize, at: &[Vec2]) -> Result<Vec<Mat2>, EditError> {
    let mesh = &model.meshes[index];
    let Some(parent) = &mesh.parent else {
        return Ok(vec![Mat2::IDENTITY; at.len()]);
    };
    // 局所座標の単位: ワープは UV (0..1)、回転デフォーマはピクセル。
    let eps = match model
        .deformers
        .iter()
        .find(|d| &d.id == parent)
        .map(|d| &d.kind)
    {
        Some(DeformerKind::Warp(_)) => 1e-3,
        _ => 1.0,
    };
    let (dx, dy) = (Vec2::new(eps, 0.0), Vec2::new(0.0, eps));
    let points: Vec<Vec2> = at
        .iter()
        .flat_map(|&p| [p + dx, p - dx, p + dy, p - dy])
        .collect();
    let mut probe = probe_of(model, mesh, 0);
    probe.uvs = vec![Vec2::ZERO; points.len()];
    probe.indices.clear();
    probe.keyforms = KeyformGrid::constant(MeshKeyform {
        positions: points,
        opacity: 1.0,
        draw_order: 0.0,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
        mask_strength: 1.0,
    });
    let (_, mut probes) = evaluate_probes(model, &mesh.id, index, vec![probe])?;
    let mapped = probes.pop().unwrap_or_default();
    Ok(mapped
        .as_chunks::<4>()
        .0
        .iter()
        .map(|m| Mat2::from_cols((m[0] - m[1]) / (2.0 * eps), (m[2] - m[3]) / (2.0 * eps)))
        .collect())
}

// ---- 三角形の上の位置 ----

/// 点 `p` の三角形 `[a, b, c]` に対する重心座標 (三角形の外なら負の成分を持つ)。退化していれば `None`。
fn barycentric(p: Vec2, [a, b, c]: [Vec2; 3]) -> Option<[f64; 3]> {
    let (p, a, b, c) = (p.as_dvec2(), a.as_dvec2(), b.as_dvec2(), c.as_dvec2());
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let det = v0.x * v1.y - v1.x * v0.y;
    let scale = v0.length_squared().max(v1.length_squared());
    if det.abs() <= 1e-12 * scale || scale == 0.0 {
        return None;
    }
    let wb = (v2.x * v1.y - v1.x * v2.y) / det;
    let wc = (v0.x * v2.y - v2.x * v0.y) / det;
    Some([1.0 - wb - wc, wb, wc])
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

fn triangle(indices: &[u16], t: usize) -> [usize; 3] {
    [0, 1, 2].map(|k| usize::from(indices[3 * t + k]))
}

/// 点 `p` を含む三角形 (なければ最も近い三角形) の番号と、その三角形での重心座標。
/// 三角形の外の点では重心座標は外挿になる (その三角形のアフィン写像を延長する)。
fn locate(positions: &[Vec2], indices: &[u16], p: Vec2) -> Option<(usize, [f64; 3])> {
    let mut nearest: Option<(f32, usize, [f64; 3])> = None;
    for t in 0..indices.len() / 3 {
        let tri = triangle(indices, t).map(|i| positions[i]);
        let Some(w) = barycentric(p, tri) else {
            continue;
        };
        if w.iter().all(|&x| x >= INSIDE) {
            return Some((t, w));
        }
        let d = (0..3)
            .map(|k| segment_distance(p, tri[k], tri[(k + 1) % 3]))
            .fold(f32::INFINITY, f32::min);
        if nearest.is_none_or(|(best, ..)| d < best) {
            nearest = Some((d, t, w));
        }
    }
    nearest.map(|(_, t, w)| (t, w))
}

fn blend(values: [Vec2; 3], w: [f64; 3]) -> Vec2 {
    let v = values[0].as_dvec2() * w[0] + values[1].as_dvec2() * w[1] + values[2].as_dvec2() * w[2];
    v.as_vec2()
}

fn blend_scalar(values: [f32; 3], w: [f64; 3]) -> f32 {
    (f64::from(values[0]) * w[0] + f64::from(values[1]) * w[1] + f64::from(values[2]) * w[2]) as f32
}

// ---- 三角形の作り直し ----

/// 境界辺 (1 つの三角形だけが使う辺) を、その三角形での向きのまま宣言順に返す。
fn boundary_edges(indices: &[u16]) -> Vec<[u16; 2]> {
    let mut count: HashMap<[u16; 2], u32> = HashMap::new();
    let directed: Vec<[u16; 2]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
        .collect();
    for &[a, b] in &directed {
        *count.entry([a.min(b), a.max(b)]).or_default() += 1;
    }
    directed
        .into_iter()
        .filter(|&[a, b]| a != b && count[&[a.min(b), a.max(b)]] == 1)
        .collect()
}

/// 三角形の UV 空間での向きの合計 (外積の和。正なら Y 下向きの画面で時計回りが多い)。
fn uv_winding(uvs: &[Vec2], indices: &[u16]) -> f64 {
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| uvs[usize::from(t[k])].as_dvec2());
            (b - a).perp_dot(c - a)
        })
        .sum()
}

/// UV を制約付きドロネー三角形分割する。三角形の向きは元の三角形 (`winding`) に合わせる。
fn triangulate(
    id: &MeshId,
    uvs: &[Vec2],
    constraints: &[[u16; 2]],
    winding: f64,
) -> Result<Vec<u16>, EditError> {
    let mut indices = triangulate_points(uvs, constraints).map_err(|e| match e {
        TriangulateError::Duplicate(a, b) => geometry(
            id,
            format!("vertices {a} and {b} have the same UV; move one of them first"),
        ),
        e => geometry(id, e.to_string()),
    })?;
    if indices.is_empty() {
        return Err(geometry(id, "no triangles can be made from the vertices"));
    }
    // 出力は画面上で反時計回り (UV の外積が負)。元が時計回りなら合わせる。
    if winding > 0.0 {
        for t in indices.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
    }
    Ok(indices)
}

// ---- コマンド ----

/// `MoveMeshVertices`: キャンバス座標のずれを局所座標に直してすべてのキーフォームに加え、
/// UV を基準の姿でのテクスチャの対応から求め直す。
pub(crate) fn move_vertices(
    model: &mut Model,
    id: &MeshId,
    moves: &[VertexMove],
) -> Result<(), EditError> {
    let index = mesh_index(model, id)?;
    let mesh = &model.meshes[index];
    let mut seen = vec![false; mesh.uvs.len()];
    for m in moves {
        check_vertex(mesh, m.index)?;
        if std::mem::replace(&mut seen[usize::from(m.index)], true) {
            return Err(geometry(id, format!("vertex {} is moved twice", m.index)));
        }
        if !m.delta.is_finite() {
            return Err(geometry(
                id,
                format!("delta of vertex {} is not finite", m.index),
            ));
        }
    }
    if moves.is_empty() {
        return Ok(());
    }
    let rest = rest(model, index)?;
    let at: Vec<Vec2> = moves
        .iter()
        .map(|m| rest.local[usize::from(m.index)])
        .collect();
    let jacobians = parent_jacobians(model, index, &at)?;
    let mut updates = Vec::with_capacity(moves.len());
    for (m, j) in moves.iter().zip(&jacobians) {
        let i = usize::from(m.index);
        let det = j.determinant();
        if !det.is_finite() || det.abs() < 1e-9 {
            return Err(geometry(
                id,
                format!("the parent deformer is degenerate at vertex {i}"),
            ));
        }
        let local = j.inverse() * m.delta;
        let target = rest.world[i] + m.delta;
        let uv = locate(&rest.world, &mesh.indices, target)
            .map(|(t, w)| blend(triangle(&mesh.indices, t).map(|k| mesh.uvs[k]), w))
            .unwrap_or(mesh.uvs[i]);
        updates.push((i, local, uv));
    }
    let mesh = &mut model.meshes[index];
    for (i, local, uv) in updates {
        mesh.uvs[i] = uv;
        for kf in &mut mesh.keyforms.keyforms {
            if let Some(p) = kf.positions.get_mut(i) {
                *p += local;
            }
        }
    }
    Ok(())
}

/// `AddMeshVertex`: キャンバス座標 `position` に頂点を足し (末尾に追加)、三角形を作り直す。
pub(crate) fn add_vertex(model: &mut Model, id: &MeshId, position: Vec2) -> Result<(), EditError> {
    let index = mesh_index(model, id)?;
    if !position.is_finite() {
        return Err(geometry(id, "position is not finite"));
    }
    let mesh = &model.meshes[index];
    let n = mesh.uvs.len();
    if n >= MAX_VERTICES {
        return Err(geometry(id, "the mesh already has 65535 vertices"));
    }
    let rest = rest(model, index)?;
    let indices = &mesh.indices;
    let Some((mut t, mut w)) = locate(&rest.world, indices, position) else {
        return Err(geometry(
            id,
            "the mesh has no triangles to place a vertex on",
        ));
    };
    let mut boundary = boundary_edges(indices);
    let new = n as u16;
    let inside = w.iter().all(|&x| x >= INSIDE);
    // 分ける境界辺 (新しい頂点を輪郭に入れる)。内側の点は輪郭に載っているときだけ。
    let split = if inside {
        let tri = triangle(indices, t);
        (0..3).find_map(|k| {
            let edge = [tri[(k + 1) % 3] as u16, tri[(k + 2) % 3] as u16];
            (w[k] <= ON_EDGE).then(|| boundary.iter().position(|e| *e == edge))?
        })
    } else {
        // 外側の点は、最も近い境界辺を外へ押し出す。重心座標はその辺の三角形で外挿する。
        let nearest = boundary
            .iter()
            .enumerate()
            .map(|(k, &[a, b])| {
                let d = segment_distance(
                    position,
                    rest.world[usize::from(a)],
                    rest.world[usize::from(b)],
                );
                (k, d)
            })
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(k, _)| k);
        if let Some(k) = nearest {
            let [a, b] = boundary[k];
            let owner = indices
                .as_chunks::<3>()
                .0
                .iter()
                .position(|tri| (0..3).any(|j| tri[j] == a && tri[(j + 1) % 3] == b));
            if let Some(owner) = owner
                && let Some(owner_w) =
                    barycentric(position, triangle(indices, owner).map(|i| rest.world[i]))
            {
                (t, w) = (owner, owner_w);
            }
        }
        nearest
    };
    if let Some(k) = split {
        let [a, b] = boundary[k];
        boundary.splice(k..=k, [[a, new], [new, b]]);
    }

    let tri = triangle(indices, t);
    let winding = uv_winding(&mesh.uvs, indices);
    let mut uvs = mesh.uvs.clone();
    uvs.push(blend(tri.map(|i| mesh.uvs[i]), w));
    let new_indices = triangulate(id, &uvs, &boundary, winding)?;
    if !new_indices.contains(&new) {
        return Err(geometry(
            id,
            "the new vertex is not inside the mesh outline",
        ));
    }

    let mesh = &mut model.meshes[index];
    mesh.uvs = uvs;
    mesh.indices = new_indices;
    for kf in &mut mesh.keyforms.keyforms {
        if kf.positions.len() == n {
            let p = blend(tri.map(|i| kf.positions[i]), w);
            kf.positions.push(p);
        }
    }
    if let Some(depth) = &mut mesh.depth
        && depth.len() == n
    {
        let d = blend_scalar(tri.map(|i| depth[i]), w);
        depth.push(d);
    }
    Ok(())
}

/// `RemoveMeshVertices`: 頂点を消して番号を詰め、三角形を作り直す。
/// グルーのペアのうち消した頂点を使うものは消し、残りは番号を付け替える。
pub(crate) fn remove_vertices(
    model: &mut Model,
    id: &MeshId,
    remove: &[u16],
) -> Result<(), EditError> {
    let index = mesh_index(model, id)?;
    let mesh = &model.meshes[index];
    let n = mesh.uvs.len();
    let mut removed = vec![false; n];
    for &i in remove {
        check_vertex(mesh, i)?;
        removed[usize::from(i)] = true;
    }
    let remaining = removed.iter().filter(|r| !**r).count();
    if remaining < 3 {
        return Err(EditError::TooFewVertices {
            id: id.to_string(),
            remaining,
        });
    }

    // 輪郭を保つ: 消した頂点を飛ばして、残る前後の頂点を直接つなぐ。
    let boundary = boundary_edges(&mesh.indices);
    let mut next: HashMap<u16, u16> = HashMap::new();
    for &[a, b] in &boundary {
        next.entry(a).or_insert(b);
    }
    let mut remap = vec![None; n];
    let mut kept = 0u16;
    for (i, slot) in remap.iter_mut().enumerate() {
        if !removed[i] {
            *slot = Some(kept);
            kept += 1;
        }
    }
    let mut constraints = Vec::new();
    for &[a, b] in &boundary {
        let Some(from) = remap[usize::from(a)] else {
            continue;
        };
        let mut c = b;
        let mut steps = 0;
        while removed[usize::from(c)] && steps <= boundary.len() {
            let Some(&following) = next.get(&c) else {
                break;
            };
            c = following;
            steps += 1;
        }
        if let Some(to) = remap[usize::from(c)]
            && to != from
        {
            constraints.push([from, to]);
        }
    }

    let keep = |values: &[Vec2]| -> Vec<Vec2> {
        values
            .iter()
            .zip(&removed)
            .filter(|(_, r)| !**r)
            .map(|(v, _)| *v)
            .collect()
    };
    let uvs = keep(&mesh.uvs);
    let indices = triangulate(id, &uvs, &constraints, uv_winding(&mesh.uvs, &mesh.indices))?;

    let mesh = &mut model.meshes[index];
    mesh.uvs = uvs;
    mesh.indices = indices;
    for kf in &mut mesh.keyforms.keyforms {
        if kf.positions.len() == n {
            kf.positions = keep(&kf.positions);
        }
    }
    if let Some(depth) = &mut mesh.depth
        && depth.len() == n
    {
        *depth = depth
            .iter()
            .zip(&removed)
            .filter(|(_, r)| !**r)
            .map(|(d, _)| *d)
            .collect();
    }
    for glue in &mut model.glues {
        let (on_a, on_b) = (&glue.mesh_a == id, &glue.mesh_b == id);
        if !on_a && !on_b {
            continue;
        }
        let map = |i: u16, on: bool| {
            if on {
                remap.get(usize::from(i)).copied().flatten()
            } else {
                Some(i)
            }
        };
        glue.pairs
            .retain_mut(|pair| match (map(pair.a, on_a), map(pair.b, on_b)) {
                (Some(a), Some(b)) => {
                    (pair.a, pair.b) = (a, b);
                    true
                }
                _ => false,
            });
    }
    Ok(())
}

/// `RetriangulateMesh`: 今の輪郭 (境界辺) を保ったまま、UV の制約付きドロネー三角形分割で作り直す。
pub(crate) fn retriangulate(model: &mut Model, id: &MeshId) -> Result<(), EditError> {
    let index = mesh_index(model, id)?;
    let mesh = &mut model.meshes[index];
    let boundary = boundary_edges(&mesh.indices);
    let winding = uv_winding(&mesh.uvs, &mesh.indices);
    mesh.indices = triangulate(id, &mesh.uvs, &boundary, winding)?;
    Ok(())
}
