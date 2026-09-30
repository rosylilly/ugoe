//! 本体のメッシュのキーフォームへの追従 (T0519)。
//!
//! 目の開閉や口の開閉のように、デフォーマではなく本体のメッシュ自身のキーフォームで形が変わる部分は、
//! 追従のワープでは写せない。衣装のメッシュ (アイシャドウ・化粧など) の頂点の多くがそのような本体のメッシュの
//! 上にあれば、本体のメッシュの各キーでの頂点の変位を、衣装の頂点へ三角形の重心座標で写す (三角形の外の頂点は、
//! いちばん近い辺の変位を距離で弱めて写す)。

use std::collections::{BTreeMap, BTreeSet};

use ugoe_core::Vec2;
use ugoe_model::{KeyformInterpolation, Mesh, MeshId, Model, ParamId};
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::{MAX_FOLLOW_CELLS, cells};
use crate::AiError;
use crate::rig::Rect;

/// 衣装の頂点のうち、本体のメッシュの三角形の中 (か [`NEAR`] 以内) にあるべき割合。
const MIN_INSIDE: f32 = 0.5;
/// 三角形の外でも中とみなす距離 (px)。PSD の取り込みはレイヤーの絵の周りに余白を付けてメッシュを作るので、
/// 本体の絵の内側に描いた衣装でも、輪郭の頂点は本体のメッシュの少し外に出る。
const NEAR: f32 = 4.0;
/// 本体のメッシュが動いたとみなす変位 (px)。
const MIN_MOTION: f32 = 0.5;
/// 三角形の外の頂点の変位を 0 まで弱める距離 (本体のメッシュの外接矩形の長い辺に対する割合)。
const FALLOFF: f32 = 0.25;

/// 作業用のモデルを、任意のパラメータの値で評価する (メッシュの頂点をキャンバス座標で返す)。
pub(super) struct Poser {
    instance: ModelInstance,
}

impl Poser {
    pub(super) fn new(model: &Model) -> Result<Self, AiError> {
        let compiled = CompiledModel::compile(model.clone())
            .map_err(|e| AiError::InvalidRequest(format!("the model cannot be evaluated: {e}")))?;
        Ok(Self {
            instance: ModelInstance::new(compiled),
        })
    }

    /// `values` のパラメータ (ほかは既定値) でのメッシュ `mesh` の頂点。
    pub(super) fn mesh(&mut self, values: &[(ParamId, f32)], mesh: &MeshId) -> Option<Vec<Vec2>> {
        let parameters = self.instance.parameters_mut();
        parameters.reset();
        for (id, value) in values {
            if let Some(index) = parameters.index_of(id.as_str()) {
                parameters.set(index, *value);
            }
        }
        self.instance.mark_all_changed();
        self.instance.update();
        let index = self.instance.compiled().mesh_index(mesh.as_str())?;
        Some(self.instance.drawables()[index].positions.clone())
    }
}

/// 衣装のメッシュが従う本体のメッシュと、軸のキーの組ごとの衣装の頂点の変位。
#[derive(Debug, Clone)]
pub(super) struct MeshFollow {
    /// 従う本体のメッシュ。
    pub base: MeshId,
    /// 本体のメッシュの軸のうち、使えるパラメータのもの (キーは既定値を含む)。
    pub axes: Vec<(ParamId, Vec<f32>)>,
    /// 本体のメッシュが曲線補間。
    pub smooth: bool,
    /// 既定値でないキーの組 (軸の並びの順の値) ごとの、衣装の頂点の変位 (キャンバス座標)。
    pub offsets: Vec<(Vec<f32>, Vec<Vec2>)>,
}

impl MeshFollow {
    /// キーの組 `values` (軸の並びの順) での衣装の頂点の変位。既定値の組や、ない組は `None`。
    pub(super) fn offsets_at(&self, values: &[f32]) -> Option<&[Vec2]> {
        self.offsets
            .iter()
            .find(|(cell, _)| cell.as_slice() == values)
            .map(|(_, o)| o.as_slice())
    }
}

/// 候補の本体のメッシュ。
struct Candidate<'a> {
    mesh: &'a Mesh,
    rest: Vec<Vec2>,
    inside: f32,
    gap: f32,
}

/// 衣装のメッシュ `costume` (基準の姿の頂点 `points`、描画順 `order`) が従う本体のメッシュを探す。
/// 頂点の半分以上がその三角形の中にあり、使えるパラメータの軸で形が変わる本体のメッシュのうち、描画順が
/// 衣装に最も近いもの (同じなら覆う割合の大きいもの)。なければ `None`。
pub(super) fn find(
    poser: &mut Poser,
    model: &Model,
    is_base: impl Fn(&MeshId) -> bool,
    usable: &BTreeSet<ParamId>,
    points: &[Vec2],
    order: f32,
) -> Option<MeshFollow> {
    if points.is_empty() {
        return None;
    }
    let mut candidates = Vec::new();
    for mesh in &model.meshes {
        if !is_base(&mesh.id)
            || mesh.mask_only
            || !mesh
                .keyforms
                .bindings
                .iter()
                .any(|b| usable.contains(&b.parameter))
        {
            continue;
        }
        let Some(rest) = poser.mesh(&[], &mesh.id) else {
            continue;
        };
        let inside = points
            .iter()
            .filter(|p| distance(&rest, &mesh.indices, **p) <= NEAR)
            .count() as f32
            / points.len() as f32;
        if inside < MIN_INSIDE {
            continue;
        }
        let gap = mesh
            .keyforms
            .keyforms
            .first()
            .map_or(f32::INFINITY, |k| (k.draw_order - order).abs());
        candidates.push(Candidate {
            mesh,
            rest,
            inside,
            gap,
        });
    }
    candidates.sort_by(|a, b| a.gap.total_cmp(&b.gap).then(b.inside.total_cmp(&a.inside)));
    candidates
        .into_iter()
        .find_map(|c| follow(poser, model, usable, &c, points))
}

/// 候補の本体のメッシュの各キーでの変位を衣装の頂点へ写す。形が変わらなければ `None`。
fn follow(
    poser: &mut Poser,
    model: &Model,
    usable: &BTreeSet<ParamId>,
    candidate: &Candidate<'_>,
    points: &[Vec2],
) -> Option<MeshFollow> {
    let mesh = candidate.mesh;
    let mut axes: Vec<(ParamId, Vec<f32>)> = Vec::new();
    for binding in &mesh.keyforms.bindings {
        if !usable.contains(&binding.parameter) {
            continue;
        }
        let default = model
            .parameters
            .iter()
            .find(|p| p.id == binding.parameter)
            .map_or(0.0, |p| p.default);
        let mut keys: Vec<f32> = binding.keys.clone();
        keys.push(default);
        keys.sort_by(f32::total_cmp);
        keys.dedup();
        axes.push((binding.parameter.clone(), keys));
    }
    let count: usize = axes.iter().map(|(_, k)| k.len()).product();
    if axes.is_empty() || count > MAX_FOLLOW_CELLS {
        return None;
    }
    let defaults: Vec<f32> = axes
        .iter()
        .map(|(p, _)| {
            model
                .parameters
                .iter()
                .find(|x| &x.id == p)
                .map_or(0.0, |x| x.default)
        })
        .collect();
    let rect = Rect::of(&candidate.rest)?;
    let falloff = FALLOFF * rect.size().x.max(rect.size().y).max(1.0);
    let binds: Vec<Binding> = points
        .iter()
        .map(|p| bind(&candidate.rest, &mesh.indices, *p, falloff))
        .collect();
    let mut offsets = Vec::new();
    let mut moved = 0.0_f32;
    for cell in cells(&axes) {
        if cell == defaults {
            continue;
        }
        let values: Vec<(ParamId, f32)> = axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.clone(), *v))
            .collect();
        let posed = poser.mesh(&values, &mesh.id)?;
        let motion: Vec<Vec2> = posed
            .iter()
            .zip(&candidate.rest)
            .map(|(a, b)| *a - *b)
            .collect();
        moved = motion.iter().map(|d| d.length()).fold(moved, f32::max);
        offsets.push((cell, binds.iter().map(|b| b.apply(&motion)).collect()));
    }
    (moved > MIN_MOTION).then(|| MeshFollow {
        base: mesh.id.clone(),
        axes,
        smooth: mesh.keyforms.interpolation == KeyformInterpolation::Smooth,
        offsets,
    })
}

/// 衣装の頂点の、本体のメッシュの頂点への重み。
#[derive(Debug, Clone, Copy)]
struct Binding {
    vertices: [usize; 3],
    weights: [f32; 3],
}

impl Binding {
    fn apply(&self, motion: &[Vec2]) -> Vec2 {
        self.vertices
            .iter()
            .zip(self.weights)
            .map(|(i, w)| motion.get(*i).copied().unwrap_or(Vec2::ZERO) * w)
            .fold(Vec2::ZERO, |a, b| a + b)
    }
}

fn triangles(indices: &[u16]) -> impl Iterator<Item = [usize; 3]> + '_ {
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| t.map(usize::from))
}

/// 点 `p` を含む三角形と重心座標。
fn containing(positions: &[Vec2], indices: &[u16], p: Vec2) -> Option<Binding> {
    triangles(indices).find_map(|t| {
        let [a, b, c] = t.map(|i| positions.get(i).copied());
        let weights = barycentric(p, a?, b?, c?)?;
        weights.iter().all(|w| *w >= -1e-4).then_some(Binding {
            vertices: t,
            weights,
        })
    })
}

fn barycentric(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> Option<[f32; 3]> {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let det = v0.x * v1.y - v1.x * v0.y;
    if det.abs() <= f32::EPSILON {
        return None;
    }
    let v = (v2.x * v1.y - v1.x * v2.y) / det;
    let w = (v0.x * v2.y - v2.x * v0.y) / det;
    Some([1.0 - v - w, v, w])
}

/// 点 `p` から本体のメッシュまでの距離 (三角形の中なら 0)。
fn distance(positions: &[Vec2], indices: &[u16], p: Vec2) -> f32 {
    if containing(positions, indices, p).is_some() {
        return 0.0;
    }
    nearest_edge(positions, indices, p).map_or(f32::INFINITY, |(d, ..)| d)
}

/// 点 `p` にいちばん近い三角形の辺 (距離・辺の両端・辺の上の位置 0〜1)。
fn nearest_edge(positions: &[Vec2], indices: &[u16], p: Vec2) -> Option<(f32, [usize; 2], f32)> {
    let mut best: Option<(f32, [usize; 2], f32)> = None;
    for t in triangles(indices) {
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            let (Some(a), Some(b)) = (positions.get(t[i]), positions.get(t[j])) else {
                continue;
            };
            let ab = *b - *a;
            let s = ((p - *a).dot(ab) / ab.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
            let d = (p - (*a + ab * s)).length();
            if best.as_ref().is_none_or(|(e, ..)| d < *e) {
                best = Some((d, [t[i], t[j]], s));
            }
        }
    }
    best
}

/// 点 `p` の重み: 三角形の中なら重心座標、外ならいちばん近い辺の上の点の重みを、距離で弱めたもの。
fn bind(positions: &[Vec2], indices: &[u16], p: Vec2, falloff: f32) -> Binding {
    if let Some(inside) = containing(positions, indices, p) {
        return inside;
    }
    match nearest_edge(positions, indices, p) {
        Some((d, [a, b], t)) => {
            let fade = (1.0 - d / falloff).max(0.0);
            Binding {
                vertices: [a, b, a],
                weights: [(1.0 - t) * fade, t * fade, 0.0],
            }
        }
        None => Binding {
            vertices: [0; 3],
            weights: [0.0; 3],
        },
    }
}

/// 衣装のメッシュごとの、従う本体のメッシュ (見つかったものだけ)。
pub(super) type MeshFollows = BTreeMap<MeshId, MeshFollow>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_inside_use_barycentric_weights_and_outside_fade() {
        let positions = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 10.0),
        ];
        let indices = [0, 1, 2];
        let motion = vec![Vec2::ZERO, Vec2::new(0.0, 4.0), Vec2::ZERO];
        let inside = bind(&positions, &indices, Vec2::new(5.0, 0.0), 10.0);
        assert!((inside.apply(&motion).y - 2.0).abs() < 1e-5);
        // 辺 0–1 の中点から 5 px 下: 変位は半分に弱まる。
        let outside = bind(&positions, &indices, Vec2::new(5.0, -5.0), 10.0);
        assert!((outside.apply(&motion).y - 1.0).abs() < 1e-5);
        let far = bind(&positions, &indices, Vec2::new(5.0, -50.0), 10.0);
        assert_eq!(far.apply(&motion), Vec2::ZERO);
    }
}
