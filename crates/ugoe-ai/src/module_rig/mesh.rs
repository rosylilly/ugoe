//! 本体のメッシュのキーフォームへの追従 (T0519・T0521)。
//!
//! 目の開閉や口の開閉のように、デフォーマではなく本体のメッシュ自身のキーフォーム (と、衣装が従うワープより
//! 下の本体のデフォーマ) で形が変わる部分は、追従のワープでは写せない。衣装のメッシュ (アイシャドウ・化粧など)
//! の頂点の多くがそのような本体のメッシュの上にあれば、本体のメッシュの各キーでの頂点の変位を、衣装の頂点へ
//! 三角形の重心座標で写す (三角形の外の頂点は、いちばん近い辺の変位を距離で弱めて写す)。
//!
//! 変位は、衣装が従う本体のワープ (なければ取り付け先) の局所座標 ([`Frame`]) で測る。キーの組ごとに、本体の
//! メッシュの頂点をその組の姿のフレームで局所座標に戻してから基準の姿と比べるので、フレームの動き (顔の向きの
//! ワープ) を二重に数えず、フレームより下のデフォーマ (目の回転など) の動きは変位に入る。衣装の頂点は、
//! 局所座標で変位を足してから同じ姿のフレームで写す (本体のワープとメッシュの動きを合わせて写す)。

use std::collections::{BTreeMap, BTreeSet};

use ugoe_core::Vec2;
use ugoe_model::{DeformerId, DeformerKind, Mesh, MeshId, Model, ParamId};
use ugoe_rigging::ParentWorld;
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::{MAX_FOLLOW_CELLS, cells, find as find_deformer, grid_axes};
use crate::AiError;
use crate::rig::{Rect, template};

/// 衣装の頂点のうち、本体のメッシュの三角形の中 (か [`NEAR`] 以内) にあるべき割合。
const MIN_INSIDE: f32 = 0.5;
/// 三角形の外でも中とみなす距離 (px)。PSD の取り込みはレイヤーの絵の周りに余白を付けてメッシュを作るので、
/// 本体の絵の内側に描いた衣装でも、輪郭の頂点は本体のメッシュの少し外に出る。
const NEAR: f32 = 4.0;
/// 本体のメッシュがフレームに対して動いたとみなす変位 (px)。
const MIN_MOTION: f32 = 0.5;
/// 三角形の外の頂点の変位を 0 まで弱める距離 (本体のメッシュの外接矩形の長い辺に対する割合)。
const FALLOFF: f32 = 0.25;

/// 変位を測る座標系: 衣装が従う本体のワープか取り付け先 (`None` ならキャンバス) の子の座標系。
pub(super) struct Frame {
    id: Option<DeformerId>,
    world: ParentWorld,
    /// ワープなら、基準の姿で覆う矩形 (逆写像の初期値に使う)。
    region: Option<Rect>,
}

impl Frame {
    pub(super) fn new(model: &Model, id: Option<&DeformerId>) -> Result<Self, AiError> {
        let world =
            ParentWorld::new(model, id).map_err(|e| template(format!("frame {id:?}: {e}")))?;
        let is_warp = id
            .and_then(|id| find_deformer(model, id))
            .is_some_and(|d| matches!(d.kind, DeformerKind::Warp(_)));
        let region = if is_warp {
            let samples: Vec<Vec2> = (0..=4)
                .flat_map(|j| (0..=4).map(move |i| Vec2::new(i as f32 / 4.0, j as f32 / 4.0)))
                .collect();
            Rect::of(&world.to_canvas(&samples))
        } else {
            None
        };
        Ok(Self {
            id: id.cloned(),
            world,
            region,
        })
    }

    pub(super) fn id(&self) -> Option<&DeformerId> {
        self.id.as_ref()
    }

    /// `values` のパラメータ (ほかは既定値) の姿にする。
    pub(super) fn set_pose(&mut self, values: &[(ParamId, f32)]) {
        self.world
            .set_pose(values.iter().map(|(p, v)| (p.as_str(), *v)));
    }

    pub(super) fn canvas_of(&self, points: &[Vec2]) -> Vec<Vec2> {
        self.world.to_canvas(points)
    }

    /// キャンバス座標の点を局所座標に写す。`guesses` がなければ、ワープは覆う矩形の中の位置、ほかは原点から解く
    /// (回転デフォーマはアフィンなので 1 回で解ける)。
    pub(super) fn local_of(
        &self,
        points: &[Vec2],
        guesses: Option<&[Vec2]>,
    ) -> Result<Vec<Vec2>, AiError> {
        let initial: Vec<Vec2>;
        let guesses = match guesses {
            Some(g) => g,
            None => {
                initial = points
                    .iter()
                    .map(|p| self.region.map_or(Vec2::ZERO, |r| r.normalize(*p)))
                    .collect();
                &initial
            }
        };
        self.world
            .from_canvas(points, guesses)
            .map_err(|e| template(format!("frame {:?}: {e}", self.id)))
    }
}

/// 作業用のモデルを、任意のパラメータの値で評価する (メッシュの頂点をキャンバス座標で返す)。
pub(super) struct Poser {
    instance: ModelInstance,
    /// 基準の姿 (すべて既定値) のメッシュの頂点 (衣装のメッシュごと・本体のメッシュごとに何度も引くので、1 回だけ評価する)。
    rest: Option<Vec<Vec<Vec2>>>,
}

impl Poser {
    pub(super) fn new(model: &Model) -> Result<Self, AiError> {
        let compiled = CompiledModel::compile(model.clone())
            .map_err(|e| AiError::InvalidRequest(format!("the model cannot be evaluated: {e}")))?;
        Ok(Self {
            instance: ModelInstance::new(compiled),
            rest: None,
        })
    }

    /// `values` のパラメータ (ほかは既定値) でのメッシュ `mesh` の頂点。
    pub(super) fn mesh(&mut self, values: &[(ParamId, f32)], mesh: &MeshId) -> Option<Vec<Vec2>> {
        let index = self.instance.compiled().mesh_index(mesh.as_str())?;
        if values.is_empty() {
            if self.rest.is_none() {
                self.pose(values);
                let rest = self.instance.drawables().iter();
                self.rest = Some(rest.map(|d| d.positions.clone()).collect());
            }
            return self.rest.as_ref()?.get(index).cloned();
        }
        self.pose(values);
        Some(self.instance.drawables()[index].positions.clone())
    }

    fn pose(&mut self, values: &[(ParamId, f32)]) {
        let parameters = self.instance.parameters_mut();
        parameters.reset();
        for (id, value) in values {
            if let Some(index) = parameters.index_of(id.as_str()) {
                parameters.set(index, *value);
            }
        }
        self.instance.mark_all_changed();
        self.instance.update();
    }
}

/// 衣装のメッシュが従う本体のメッシュと、軸のキーの組ごとの衣装の頂点の変位。
#[derive(Debug, Clone)]
pub(super) struct MeshFollow {
    /// 従う本体のメッシュ。
    pub base: MeshId,
    /// 本体のメッシュと、フレームより下のその祖先のデフォーマの軸のうち、使えるパラメータのもの
    /// (キーは既定値を含む)。
    pub axes: Vec<(ParamId, Vec<f32>)>,
    /// 本体のメッシュか祖先が曲線補間。
    pub smooth: bool,
    /// 変位を測ったフレーム ([`Frame`])。
    pub frame: Option<DeformerId>,
    /// 既定値でないキーの組 (軸の並びの順の値) ごとの、衣装の頂点の変位 (フレームの局所座標)。
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
    /// 本体のメッシュと、フレーム (か取り付け先) より下の祖先のデフォーマの軸 (使えるパラメータのもの)。
    axes: Vec<(ParamId, Vec<f32>)>,
    smooth: bool,
}

/// 本体のメッシュ `mesh` と、`stop` (フレームか取り付け先) に着くまでの祖先のデフォーマの、使えるパラメータの軸
/// (キーに既定値を足したもの) と、曲線補間があるか。
fn chain_axes(
    model: &Model,
    mesh: &Mesh,
    usable: &BTreeSet<ParamId>,
    stop: &[Option<&DeformerId>],
) -> (Vec<(ParamId, Vec<f32>)>, bool) {
    let mut grids = vec![grid_axes(&mesh.keyforms)];
    let mut parent = mesh.parent.as_ref();
    while let Some(id) = parent {
        if stop.contains(&Some(id)) {
            break;
        }
        let Some(deformer) = find_deformer(model, id) else {
            break;
        };
        match &deformer.kind {
            DeformerKind::Warp(w) => grids.push(grid_axes(&w.keyforms)),
            DeformerKind::Rotation(r) => grids.push(grid_axes(&r.keyforms)),
            DeformerKind::Unknown => {}
        }
        parent = deformer.parent.as_ref();
    }
    let mut axes: Vec<(ParamId, Vec<f32>)> = Vec::new();
    let mut smooth = false;
    for (grid, s) in grids {
        let mut used = false;
        for (parameter, keys) in grid {
            if !usable.contains(&parameter) {
                continue;
            }
            used = true;
            let default = model
                .parameters
                .iter()
                .find(|p| p.id == parameter)
                .map_or(0.0, |p| p.default);
            let keys = keys.into_iter().chain([default]);
            match axes.iter_mut().find(|(p, _)| *p == parameter) {
                Some((_, existing)) => existing.extend(keys),
                None => axes.push((parameter, keys.collect())),
            }
        }
        smooth |= s && used;
    }
    for (_, keys) in &mut axes {
        keys.sort_by(f32::total_cmp);
        keys.dedup();
    }
    (axes, smooth)
}

/// 衣装のメッシュ `costume` (基準の姿の頂点 `points`、描画順 `order`) が従う本体のメッシュを探す。
/// 頂点の半分以上がその三角形の中にあり、使えるパラメータの軸 (メッシュ自身か、フレームより下の祖先の
/// デフォーマのもの) でフレームに対して形が変わる本体のメッシュのうち、描画順が衣装に最も近いもの
/// (同じなら覆う割合の大きいもの)。なければ `None`。`anchor` はスロットの取り付け先。
#[allow(clippy::too_many_arguments)]
pub(super) fn find(
    poser: &mut Poser,
    model: &Model,
    is_base: impl Fn(&MeshId) -> bool,
    usable: &BTreeSet<ParamId>,
    frame: &mut Frame,
    anchor: Option<&DeformerId>,
    points: &[Vec2],
    order: f32,
) -> Result<Option<MeshFollow>, AiError> {
    if points.is_empty() {
        return Ok(None);
    }
    let stop = [frame.id().cloned(), anchor.cloned()];
    let stop: Vec<Option<&DeformerId>> = stop.iter().map(Option::as_ref).collect();
    let mut candidates = Vec::new();
    for mesh in &model.meshes {
        if !is_base(&mesh.id) || mesh.mask_only {
            continue;
        }
        let (axes, smooth) = chain_axes(model, mesh, usable, &stop);
        if axes.is_empty() {
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
            axes,
            smooth,
        });
    }
    candidates.sort_by(|a, b| a.gap.total_cmp(&b.gap).then(b.inside.total_cmp(&a.inside)));
    for candidate in &candidates {
        if let Some(found) = follow(poser, model, frame, candidate, points)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// 候補の本体のメッシュの、各キーでのフレームに対する変位を衣装の頂点へ写す。フレームに対して形が変わらなければ
/// `None`。
fn follow(
    poser: &mut Poser,
    model: &Model,
    frame: &mut Frame,
    candidate: &Candidate<'_>,
    points: &[Vec2],
) -> Result<Option<MeshFollow>, AiError> {
    let mesh = candidate.mesh;
    let axes = &candidate.axes;
    let count: usize = axes.iter().map(|(_, k)| k.len()).product();
    if count > MAX_FOLLOW_CELLS {
        return Ok(None);
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
    let Some(rect) = Rect::of(&candidate.rest) else {
        return Ok(None);
    };
    let falloff = FALLOFF * rect.size().x.max(rect.size().y).max(1.0);
    let binds: Vec<Binding> = points
        .iter()
        .map(|p| bind(&candidate.rest, &mesh.indices, *p, falloff))
        .collect();
    frame.set_pose(&[]);
    let rest_inner = frame.local_of(&candidate.rest, None)?;
    let mut offsets = Vec::new();
    let mut moved = 0.0_f32;
    for cell in cells(axes) {
        if cell == defaults {
            continue;
        }
        let values: Vec<(ParamId, f32)> = axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.clone(), *v))
            .collect();
        let Some(posed) = poser.mesh(&values, &mesh.id) else {
            return Ok(None);
        };
        frame.set_pose(&values);
        // フレームだけが動いたときの位置との差が、本体のメッシュ自身 (と下のデフォーマ) の動き。
        let carried = frame.canvas_of(&rest_inner);
        moved = posed
            .iter()
            .zip(&carried)
            .map(|(a, b)| (*a - *b).length())
            .fold(moved, f32::max);
        let posed_inner = frame.local_of(&posed, Some(&rest_inner))?;
        let motion: Vec<Vec2> = posed_inner
            .iter()
            .zip(&rest_inner)
            .map(|(a, b)| *a - *b)
            .collect();
        offsets.push((cell, binds.iter().map(|b| b.apply(&motion)).collect()));
    }
    frame.set_pose(&[]);
    Ok((moved > MIN_MOTION).then(|| MeshFollow {
        base: mesh.id.clone(),
        axes: axes.clone(),
        smooth: candidate.smooth,
        frame: frame.id().cloned(),
        offsets,
    }))
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
