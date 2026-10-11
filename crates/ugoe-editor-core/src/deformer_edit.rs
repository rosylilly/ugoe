//! デフォーマの作成・見た目を保つ付け替え・ギズモによるセルの編集。
//! 設計と式は `docs/editor.md#デフォーマの編集`。
//!
//! 作成と付け替えは **既定値のパラメータ** で評価した姿 (以下「基準の姿」) を基準にする。
//! 基準の姿での各デフォーマのワールド変換 ([`Frame`]) をランタイムが評価したもの
//! ([`ugoe_runtime::WorldTransform`]) から作り、子のキーフォームの局所座標を `新しい親⁻¹ ∘ 古い親` で写し直す。

use std::collections::HashMap;

use ugoe_core::{Affine2, Mat2, Vec2};
use ugoe_model::deformer::{
    DeformerKind, RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform,
};
use ugoe_model::{Deformer, DeformerId, KeyformGrid, MeshId, Model, PartId};
use ugoe_runtime::{CompiledModel, ModelInstance, WorldTransform};

use crate::EditError;
use crate::command::{ChildRef, KeyformTarget};
use crate::ids::{new_deformer_id, sanitize_id};

/// ワープの行数・列数の上限 (制御点の数が大きくなりすぎないように)。
pub const MAX_WARP_DIVISIONS: u16 = 64;
/// 作るデフォーマの外接矩形の余白 (矩形の長い辺に対する割合)。
const MARGIN: f32 = 0.05;
/// ワープの逆写像の許容誤差 (キャンバスのピクセル)。
const INVERSE_TOLERANCE: f32 = 1e-3;
const MAX_ITERATIONS: usize = 64;

fn failed(reason: impl Into<String>) -> EditError {
    EditError::DeformerEdit {
        reason: reason.into(),
    }
}

fn not_found(kind: &'static str, id: &impl ToString) -> EditError {
    EditError::NotFound {
        kind,
        id: id.to_string(),
    }
}

// ---- ワールド変換 ----

/// 基準の姿での、ある座標系 (デフォーマの局所座標) からキャンバス座標への写像。
/// 写像とその微分はランタイムの [`WorldTransform`] で求める
/// (`docs/spec/evaluation.md#デフォーマのワールド変換を読む`)。ここでは作ろうとしている
/// デフォーマの写像も持てるように、制御点を所有する。
#[derive(Debug, Clone)]
pub(crate) enum Frame {
    /// 親なし (キャンバス座標そのもの)。
    Identity,
    /// 回転デフォーマ (祖先を合成したアフィン変換)。
    Affine(Affine2),
    /// ワープデフォーマ。`points` はキャンバス座標の制御点 (行優先、x が速い)。
    Warp {
        points: Vec<Vec2>,
        cols: usize,
        rows: usize,
    },
}

impl Frame {
    /// ランタイムの写像。ワープの制御点の数が合わなければ (作らないので起きない) 恒等写像。
    fn world(&self) -> WorldTransform<'_> {
        match self {
            Frame::Identity => WorldTransform::IDENTITY,
            Frame::Affine(a) => WorldTransform::from_affine(*a),
            Frame::Warp { points, cols, rows } => {
                WorldTransform::from_warp_points(points, *cols, *rows)
                    .unwrap_or(WorldTransform::IDENTITY)
            }
        }
    }

    /// 局所座標 → キャンバス座標。
    pub(crate) fn map(&self, p: Vec2) -> Vec2 {
        self.world().point(p)
    }

    /// キャンバス座標 → 局所座標 ([`Frame::map`] の逆)。ワープはニュートン法で解く
    /// (格子が折り返していて解が複数あるときは、アフィン近似の逆から近い解)。
    pub(crate) fn inverse(&self, w: Vec2) -> Result<Vec2, String> {
        match self {
            Frame::Identity => Ok(w),
            Frame::Affine(a) => {
                let det = a.matrix2.determinant();
                if !det.is_finite() || det.abs() < 1e-12 {
                    return Err("the parent rotation deformer is degenerate".into());
                }
                Ok(a.inverse().transform_point2(w))
            }
            Frame::Warp { cols, rows, .. } => {
                // 四隅から作ったアフィン近似 (範囲外の外挿の軸)。両方向にはみ出した点での
                // ヤコビ行列の列がちょうどその軸になる。
                let world = self.world();
                let c00 = world.point(Vec2::ZERO);
                let m = world.jacobian(Vec2::splat(-1.0));
                let mut starts = Vec::new();
                if m.determinant().abs() > 1e-12 {
                    starts.push(m.inverse() * (w - c00));
                }
                // アフィン近似から収束しなければ、各セルの中心から試す。
                for y in 0..*rows {
                    for x in 0..*cols {
                        starts.push(Vec2::new(
                            (x as f32 + 0.5) / *cols as f32,
                            (y as f32 + 0.5) / *rows as f32,
                        ));
                    }
                }
                starts
                    .into_iter()
                    .find_map(|start| newton(&world, w, start))
                    .ok_or_else(|| {
                        format!(
                            "cannot map ({}, {}) into the warp deformer (the grid is degenerate)",
                            w.x, w.y
                        )
                    })
            }
        }
    }

    /// この座標系を親に持つ回転デフォーマの、原点 `origin` での親の線形部分。
    /// ワールドの線形部分は `rotation_linear(origin) · R(angle) · S` になる
    /// (親が回転ならアフィンの線形部分、ワープなら向き δ の回転)。
    pub(crate) fn rotation_linear(&self, origin: Vec2) -> Mat2 {
        self.rotation_child(&RotationKeyform {
            origin,
            angle: 0.0,
            scale: 1.0,
            reflect_x: false,
            reflect_y: false,
            opacity: 1.0,
        })
        .matrix2
    }

    /// この座標系を親に持つ回転デフォーマのワールド変換。
    fn rotation_child(&self, keyform: &RotationKeyform) -> Affine2 {
        self.world().rotation_child(keyform)
    }
}

/// `world.point(uv) = w` をニュートン法 (残差が減らなければ歩幅を半分にする) で解く。
fn newton(world: &WorldTransform<'_>, w: Vec2, start: Vec2) -> Option<Vec2> {
    let mut uv = start;
    let mut residual = world.point(uv) - w;
    for _ in 0..MAX_ITERATIONS {
        if residual.length() <= INVERSE_TOLERANCE {
            return Some(uv);
        }
        let j = world.jacobian(uv);
        let det = j.determinant();
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        let step = j.inverse() * residual;
        let mut t = 1.0;
        loop {
            let next = uv - step * t;
            let r = world.point(next) - w;
            if r.length() < residual.length() {
                (uv, residual) = (next, r);
                break;
            }
            t *= 0.5;
            if t < 1e-4 {
                return (residual.length() <= INVERSE_TOLERANCE).then_some(uv);
            }
        }
    }
    (residual.length() <= INVERSE_TOLERANCE).then_some(uv)
}

// ---- 基準の姿 ----

/// 基準の姿での、各デフォーマのワールド変換とメッシュのキャンバス座標。
pub(crate) struct Rest {
    frames: HashMap<DeformerId, Frame>,
    meshes: HashMap<MeshId, Vec<Vec2>>,
}

impl Rest {
    /// モデルを既定値のパラメータで 1 回評価し、ランタイムが評価したワールド変換を読んで作る。
    pub(crate) fn new(model: &Model) -> Result<Self, EditError> {
        let compiled = CompiledModel::compile(model.clone())
            .map_err(|e| failed(format!("the model cannot be evaluated: {e}")))?;
        let instance = ModelInstance::new(compiled);
        let meshes = model
            .meshes
            .iter()
            .zip(instance.drawables())
            .map(|(m, d)| (m.id.clone(), d.positions.clone()))
            .collect();
        // 既知の種類のデフォーマ `i` のワールド変換。
        let known = |i: usize| match &model.deformers[i].kind {
            DeformerKind::Warp(w) => Frame::Warp {
                points: instance.warp_world_points(i).unwrap_or_default().to_vec(),
                cols: usize::from(w.cols),
                rows: usize::from(w.rows),
            },
            _ => Frame::Affine(
                instance
                    .rotation_world_frame(i)
                    .unwrap_or(Affine2::IDENTITY),
            ),
        };
        let frames = model
            .deformers
            .iter()
            .enumerate()
            .map(|(i, d)| {
                // 未知の種類のデフォーマは恒等変換なので、ワールド変換は祖先の既知のデフォーマのもの。
                let frame = if matches!(d.kind, DeformerKind::Unknown) {
                    ancestor_known(model, d.parent.as_ref()).map_or(Frame::Identity, known)
                } else {
                    known(i)
                };
                (d.id.clone(), frame)
            })
            .collect();
        Ok(Self { frames, meshes })
    }

    /// デフォーマ `parent` の局所座標系 (`None` ならキャンバス座標)。
    pub(crate) fn frame(&self, parent: Option<&DeformerId>) -> Frame {
        parent
            .and_then(|id| self.frames.get(id))
            .cloned()
            .unwrap_or(Frame::Identity)
    }
}

/// `from` とその祖先のうち、最も近い既知の種類 (ワープ・回転) のデフォーマの添字。
fn ancestor_known(model: &Model, from: Option<&DeformerId>) -> Option<usize> {
    let mut current = from;
    for _ in 0..=model.deformers.len() {
        let i = model
            .deformers
            .iter()
            .position(|d| Some(&d.id) == current)?;
        if !matches!(model.deformers[i].kind, DeformerKind::Unknown) {
            return Some(i);
        }
        current = model.deformers[i].parent.as_ref();
    }
    None
}

// ---- 木の上の関係 ----

fn find_deformer<'a>(model: &'a Model, id: &DeformerId) -> Result<&'a Deformer, EditError> {
    model
        .deformers
        .iter()
        .find(|d| &d.id == id)
        .ok_or_else(|| not_found("deformer", id))
}

fn parent_of(model: &Model, child: &ChildRef) -> Result<Option<DeformerId>, EditError> {
    Ok(match child {
        ChildRef::Mesh(id) => model
            .meshes
            .iter()
            .find(|m| &m.id == id)
            .ok_or_else(|| not_found("mesh", id))?
            .parent
            .clone(),
        ChildRef::Deformer(id) => find_deformer(model, id)?.parent.clone(),
    })
}

fn part_of(model: &Model, child: &ChildRef) -> Option<PartId> {
    match child {
        ChildRef::Mesh(id) => model
            .meshes
            .iter()
            .find(|m| &m.id == id)
            .map(|m| m.part.clone()),
        ChildRef::Deformer(id) => model
            .deformers
            .iter()
            .find(|d| &d.id == id)
            .map(|d| d.part.clone()),
    }
}

/// `from` から根までの親デフォーマの列 (`from` を含む、根 = `None` を最後に含む)。
fn deformer_chain(model: &Model, from: Option<&DeformerId>) -> Vec<Option<DeformerId>> {
    let mut chain = vec![from.cloned()];
    let mut current = from.cloned();
    for _ in 0..model.deformers.len() {
        let Some(id) = current else { break };
        current = model
            .deformers
            .iter()
            .find(|d| d.id == id)
            .and_then(|d| d.parent.clone());
        chain.push(current.clone());
    }
    chain
}

/// デフォーマ `candidate` が `ancestor` 自身かその子孫か。
fn is_within(model: &Model, candidate: &DeformerId, ancestor: &DeformerId) -> bool {
    deformer_chain(model, Some(candidate))
        .iter()
        .any(|d| d.as_ref() == Some(ancestor))
}

/// 子たちの親の、最も近い共通の祖先 (`None` はキャンバス直下)。
fn common_parent(model: &Model, parents: &[Option<DeformerId>]) -> Option<DeformerId> {
    let chains: Vec<_> = parents
        .iter()
        .map(|p| deformer_chain(model, p.as_ref()))
        .collect();
    let first = chains.first()?;
    first
        .iter()
        .find(|c| chains.iter().all(|chain| chain.contains(c)))
        .cloned()
        .flatten()
}

/// パーツの最も近い共通の祖先。共通の祖先がなければ最初のパーツ。
fn common_part(model: &Model, parts: &[PartId]) -> Option<PartId> {
    let chain = |from: &PartId| {
        let mut out = vec![from.clone()];
        let mut current = from.clone();
        for _ in 0..model.parts.len() {
            match model
                .parts
                .iter()
                .find(|p| p.id == current)
                .and_then(|p| p.parent.clone())
            {
                Some(parent) => {
                    out.push(parent.clone());
                    current = parent;
                }
                None => break,
            }
        }
        out
    };
    let chains: Vec<_> = parts.iter().map(chain).collect();
    let first = chains.first()?;
    first
        .iter()
        .find(|p| chains.iter().all(|c| c.contains(p)))
        .or(parts.first())
        .cloned()
}

/// 子 (とその子孫) の基準の姿でのキャンバス座標の点: メッシュの頂点、ワープの制御点、回転デフォーマの原点。
pub(crate) fn subtree_points(model: &Model, rest: &Rest, child: &ChildRef, out: &mut Vec<Vec2>) {
    match child {
        ChildRef::Mesh(id) => out.extend(rest.meshes.get(id).into_iter().flatten()),
        ChildRef::Deformer(id) => {
            match rest.frames.get(id) {
                Some(Frame::Warp { points, .. }) => out.extend(points),
                Some(Frame::Affine(a)) => out.push(a.translation),
                _ => {}
            }
            for m in model
                .meshes
                .iter()
                .filter(|m| m.parent.as_ref() == Some(id))
            {
                subtree_points(model, rest, &ChildRef::Mesh(m.id.clone()), out);
            }
            for d in model
                .deformers
                .iter()
                .filter(|d| d.parent.as_ref() == Some(id))
            {
                subtree_points(model, rest, &ChildRef::Deformer(d.id.clone()), out);
            }
        }
    }
}

// ---- 局所座標の写し直し ----

/// 要素のキーフォームを、座標系 `from` から `to` へ写し直す (基準の姿で見た目が同じになるように)。
///
/// 点 (メッシュの頂点・ワープの制御点・回転デフォーマの原点) は `to⁻¹ ∘ from` で写す。
/// 回転デフォーマの角度・拡大率・反転は、ワールドの線形部分が変わらないように直す。
fn reexpress(
    model: &mut Model,
    child: &ChildRef,
    from: &Frame,
    to: &Frame,
) -> Result<(), EditError> {
    let map = |p: Vec2| to.inverse(from.map(p)).map_err(failed);
    match child {
        ChildRef::Mesh(id) => {
            let mesh = model
                .meshes
                .iter_mut()
                .find(|m| &m.id == id)
                .ok_or_else(|| not_found("mesh", id))?;
            let reference = centroid(mesh.keyforms.keyforms.first().map(|k| &k.positions[..]));
            for kf in mesh.keyforms.all_keyforms_mut() {
                for p in &mut kf.positions {
                    *p = map(*p)?;
                }
            }
            if let Some(axis) = &mut mesh.mirror_axis {
                *axis = map(Vec2::new(*axis, reference.y))?.x;
            }
        }
        ChildRef::Deformer(id) => {
            let deformer = model
                .deformers
                .iter_mut()
                .find(|d| &d.id == id)
                .ok_or_else(|| not_found("deformer", id))?;
            let reference = match &mut deformer.kind {
                DeformerKind::Warp(w) => {
                    let reference = centroid(w.keyforms.keyforms.first().map(|k| &k.points[..]));
                    for kf in w.keyforms.all_keyforms_mut() {
                        for p in &mut kf.points {
                            *p = map(*p)?;
                        }
                    }
                    reference
                }
                DeformerKind::Rotation(r) => {
                    let reference = r.keyforms.keyforms.first().map_or(Vec2::ZERO, |k| k.origin);
                    let mut first_turn = None;
                    for kf in r.keyforms.all_keyforms_mut() {
                        let origin = map(kf.origin)?;
                        let q =
                            to.rotation_linear(origin).inverse() * from.rotation_linear(kf.origin);
                        reorient(kf, q, &mut first_turn);
                        kf.origin = origin;
                    }
                    reference
                }
                _ => {
                    return Err(failed(format!(
                        "deformer `{id}` has an unknown kind and cannot be moved"
                    )));
                }
            };
            if let Some(axis) = &mut deformer.mirror_axis {
                *axis = map(Vec2::new(*axis, reference.y))?.x;
            }
        }
    }
    Ok(())
}

fn centroid(points: Option<&[Vec2]>) -> Vec2 {
    match points {
        Some(p) if !p.is_empty() => p.iter().copied().sum::<Vec2>() / p.len() as f32,
        _ => Vec2::ZERO,
    }
}

/// 回転デフォーマのキーフォームの前に線形変換 `q` (相似変換) を掛ける:
/// `R(angle') · S' = q · R(angle) · S` となる角度・拡大率・反転にする。
///
/// 回転量は最初のキーフォームのものに近い値 (360 度の違いを除いたもの) にそろえ、キーフォーム間の
/// 角度の補間が変わらないようにする。
fn reorient(kf: &mut RotationKeyform, q: Mat2, first_turn: &mut Option<f32>) {
    let det = q.determinant();
    let s = det.abs().sqrt();
    // q = s·R(φ) (det > 0) または s·R(φ)·diag(-1, 1) (det < 0)。
    let x = if det < 0.0 { -q.x_axis } else { q.x_axis };
    let mut phi = x.y.atan2(x.x).to_degrees();
    let reference = *first_turn.get_or_insert(phi);
    phi += 360.0 * ((reference - phi) / 360.0).round();
    if det < 0.0 {
        // R(φ)·diag(-1,1)·R(a)·diag(sx,sy) = R(φ - a)·diag(-sx, sy)
        kf.angle = phi - kf.angle;
        kf.reflect_x = !kf.reflect_x;
    } else {
        kf.angle += phi;
    }
    kf.scale *= s;
}

fn set_parent(model: &mut Model, child: &ChildRef, parent: Option<DeformerId>) {
    match child {
        ChildRef::Mesh(id) => {
            if let Some(m) = model.meshes.iter_mut().find(|m| &m.id == id) {
                m.parent = parent;
            }
        }
        ChildRef::Deformer(id) => {
            if let Some(d) = model.deformers.iter_mut().find(|d| &d.id == id) {
                d.parent = parent;
            }
        }
    }
}

fn target(child: &ChildRef) -> KeyformTarget {
    match child {
        ChildRef::Mesh(id) => KeyformTarget::Mesh(id.clone()),
        ChildRef::Deformer(id) => KeyformTarget::Deformer(id.clone()),
    }
}

// ---- コマンド ----

/// `ReparentPreservingLook`: 親を付け替え、基準の姿で見た目が変わらないように局所座標を写し直す。
pub(crate) fn reparent(
    model: &mut Model,
    element: &ChildRef,
    new_parent: Option<&DeformerId>,
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
    let old = parent_of(model, element)?;
    if let Some(p) = new_parent {
        find_deformer(model, p)?;
        if let ChildRef::Deformer(id) = element
            && is_within(model, p, id)
        {
            return Err(failed(format!(
                "deformer `{p}` is `{id}` itself or its descendant"
            )));
        }
    }
    if old.as_ref() == new_parent {
        return Ok(());
    }
    let rest = Rest::new(model)?;
    let (from, to) = (rest.frame(old.as_ref()), rest.frame(new_parent));
    reexpress(model, element, &from, &to)?;
    set_parent(model, element, new_parent.cloned());
    touched.push(target(element));
    Ok(())
}

/// デフォーマ `id` の直接の子 (デフォーマ、メッシュの順)。
fn children_of(model: &Model, id: &DeformerId) -> Vec<ChildRef> {
    let deformers = model
        .deformers
        .iter()
        .filter(|d| d.parent.as_ref() == Some(id))
        .map(|d| ChildRef::Deformer(d.id.clone()));
    let meshes = model
        .meshes
        .iter()
        .filter(|m| m.parent.as_ref() == Some(id))
        .map(|m| ChildRef::Mesh(m.id.clone()));
    deformers.chain(meshes).collect()
}

/// `MoveRotationOrigin`: すべてのセルの原点に `delta` を足し、直接の子を基準の姿で見た目が変わらないように
/// 写し直す (子の座標系は `古い枠⁻¹` から `新しい枠⁻¹` へ。どちらもアフィンなので写し直しは厳密)。
///
/// 子の格子は回転デフォーマの格子と軸が別なので、補正できるのは 1 つの姿 (基準の姿) だけ。他のセルでは
/// 子が新しい原点のまわりで回る (中心を動かしたときに期待される動き)。
pub(crate) fn move_rotation_origin(
    model: &mut Model,
    id: &DeformerId,
    delta: Vec2,
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
    if !delta.is_finite() {
        return Err(failed("delta is not finite"));
    }
    if !matches!(find_deformer(model, id)?.kind, DeformerKind::Rotation(_)) {
        return Err(EditError::DeformerKindMismatch {
            id: id.to_string(),
            expected: "rotation",
        });
    }
    let from = Rest::new(model)?.frame(Some(id));
    if let Some(Deformer {
        kind: DeformerKind::Rotation(r),
        ..
    }) = model.deformers.iter_mut().find(|d| &d.id == id)
    {
        for kf in r.keyforms.all_keyforms_mut() {
            kf.origin += delta;
        }
    }
    let to = Rest::new(model)?.frame(Some(id));
    for child in children_of(model, id) {
        reexpress(model, &child, &from, &to)?;
        touched.push(target(&child));
    }
    touched.push(KeyformTarget::Deformer(id.clone()));
    Ok(())
}

/// `UnwrapDeformer` の前半: デフォーマ `id` の直接の子を、見た目を保ってその親へ付け替える。
/// デフォーマの削除は呼び出し側 (`document.rs`) が行う。
pub(crate) fn unwrap(
    model: &mut Model,
    id: &DeformerId,
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
    let parent = find_deformer(model, id)?.parent.clone();
    for child in children_of(model, id) {
        reparent(model, &child, parent.as_ref(), touched)?;
    }
    Ok(())
}

/// 作るデフォーマの種類ごとの指定。
pub(crate) enum NewKind {
    Warp {
        rows: u16,
        cols: u16,
    },
    /// 原点 (キャンバス座標)。`None` なら子の外接矩形の中心。
    Rotation {
        origin: Option<Vec2>,
    },
}

/// パーツを包むときの子: `parts` (と子孫のパーツ) に属するメッシュ・デフォーマのうち、同じ集まりの
/// デフォーマと `given` のデフォーマの下にないもの (いちばん外側の要素)。`given` にあるものは除く。
/// 並びはモデルの順 (デフォーマ、メッシュの順)。パーツがなければ [`EditError::NotFound`]。
pub(crate) fn part_children(
    model: &Model,
    parts: &[PartId],
    given: &[ChildRef],
) -> Result<Vec<ChildRef>, EditError> {
    if parts.is_empty() {
        return Ok(Vec::new());
    }
    for part in parts {
        if !model.parts.iter().any(|p| &p.id == part) {
            return Err(not_found("part", part));
        }
    }
    // パーツ `part` が `parts` のどれか自身かその子孫か。
    let in_parts = |part: &PartId| {
        let mut current = Some(part.clone());
        for _ in 0..=model.parts.len() {
            let Some(id) = current else { break };
            if parts.contains(&id) {
                return true;
            }
            current = model
                .parts
                .iter()
                .find(|p| p.id == id)
                .and_then(|p| p.parent.clone());
        }
        false
    };
    let mut inside: Vec<&DeformerId> = model
        .deformers
        .iter()
        .filter(|d| in_parts(&d.part))
        .map(|d| &d.id)
        .collect();
    inside.extend(given.iter().filter_map(|c| match c {
        ChildRef::Deformer(id) => Some(id),
        ChildRef::Mesh(_) => None,
    }));
    // 親の列 (親自身を含む) に、集まりのデフォーマがあるか。
    let under = |parent: Option<&DeformerId>| {
        deformer_chain(model, parent)
            .iter()
            .flatten()
            .any(|d| inside.contains(&d))
    };
    let deformers = model
        .deformers
        .iter()
        .filter(|d| in_parts(&d.part) && !under(d.parent.as_ref()))
        .map(|d| ChildRef::Deformer(d.id.clone()));
    let meshes = model
        .meshes
        .iter()
        .filter(|m| in_parts(&m.part) && !under(m.parent.as_ref()))
        .map(|m| ChildRef::Mesh(m.id.clone()));
    Ok(deformers
        .chain(meshes)
        .filter(|c| !given.contains(c))
        .collect())
}

/// 名前を省略して作るデフォーマの名前と、ID の元。
///
/// 元にする要素は、最初のパーツ (なければ最初の子)。名前はその要素の名前に種類を付けたもの:
/// 名前に ASCII 以外の文字 (日本語のレイヤー名など) があれば `顔のワープ` / `顔の回転`、なければ
/// `Face warp` / `Face rotation`。ID の元は種類とその要素の ID (`WarpFace`・`RotationLayer8`)。
fn default_name(
    model: &Model,
    kind: &NewKind,
    parts: &[PartId],
    children: &[ChildRef],
) -> (String, String) {
    let source = parts
        .first()
        .and_then(|id| model.parts.iter().find(|p| &p.id == id))
        .map(|p| (p.name.as_str(), p.id.as_str()))
        .or_else(|| match children.first()? {
            ChildRef::Mesh(id) => model
                .meshes
                .iter()
                .find(|m| &m.id == id)
                .map(|m| (m.name.as_str(), m.id.as_str())),
            ChildRef::Deformer(id) => model
                .deformers
                .iter()
                .find(|d| &d.id == id)
                .map(|d| (d.name.as_str(), d.id.as_str())),
        });
    let (kind_id, en, ja) = match kind {
        NewKind::Warp { .. } => ("Warp", "warp", "ワープ"),
        NewKind::Rotation { .. } => ("Rotation", "rotation", "回転"),
    };
    let Some((name, id)) = source else {
        return (kind_id.to_owned(), kind_id.to_owned());
    };
    let name = name.trim();
    let name = if name.is_empty() {
        kind_id.to_owned()
    } else if name.is_ascii() {
        format!("{name} {en}")
    } else {
        format!("{name}の{ja}")
    };
    // 名前から作ったデフォーマの ID (`WarpFace`) を包むときは、種類の接頭辞を重ねない (`RotationFace`)。
    let id = ["Warp", "Rotation"]
        .iter()
        .find_map(|prefix| {
            id.strip_prefix(prefix)
                .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
        })
        .unwrap_or(id);
    let mut id = sanitize_id(id);
    if let Some(first) = id.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    (name, format!("{kind_id}{id}"))
}

/// 作るデフォーマの ID・名前・親・種類 (`CreateWarpDeformer` / `CreateRotationDeformer` の値)。
pub(crate) struct NewDeformer {
    pub id: Option<DeformerId>,
    pub name: String,
    pub parent: Option<DeformerId>,
    pub kind: NewKind,
}

/// `CreateWarpDeformer` / `CreateRotationDeformer`: 子を包むデフォーマを作り、子を見た目を保って付け替える。
///
/// `parts` のパーツの要素 ([`part_children`]) も子にする。`name` が空なら子から名前を作り、`id` を
/// 省略すると名前 (名前が空か英数字を含まなければ子の ID) から作る ([`default_name`])。
pub(crate) fn create(
    model: &mut Model,
    new: NewDeformer,
    children: &[ChildRef],
    parts: &[PartId],
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
    let NewDeformer {
        id,
        name,
        parent,
        kind,
    } = new;
    let mut children = children.to_vec();
    for child in part_children(model, parts, &children)? {
        if !children.contains(&child) {
            children.push(child);
        }
    }
    // 子に指定したデフォーマの下にある子は、そのデフォーマごと移るので付け替えない (パーツを包むときと同じ)。
    let wrapped: Vec<&DeformerId> = children
        .iter()
        .filter_map(|c| match c {
            ChildRef::Deformer(id) => Some(id),
            ChildRef::Mesh(_) => None,
        })
        .collect();
    let nested: Vec<ChildRef> = children
        .iter()
        .filter(|child| {
            let parent = parent_of(model, child).ok().flatten();
            deformer_chain(model, parent.as_ref())
                .iter()
                .flatten()
                .any(|d| wrapped.contains(&d))
        })
        .cloned()
        .collect();
    children.retain(|child| !nested.contains(child));
    let children = &children[..];
    if children.is_empty() {
        return Err(failed(if parts.is_empty() {
            "no children are given"
        } else {
            "the parts have no meshes or deformers to wrap"
        }));
    }
    let (name, id_base) = if name.trim().is_empty() {
        let (name, id_base) = default_name(model, &kind, parts, children);
        (name, id_base)
    } else if name.chars().any(|c| c.is_ascii_alphanumeric()) {
        (name.clone(), name)
    } else {
        let (_, id_base) = default_name(model, &kind, parts, children);
        (name, id_base)
    };
    let mut parents = Vec::new();
    for (i, child) in children.iter().enumerate() {
        if children[..i].contains(child) {
            return Err(failed(format!("{} is given twice", target(child))));
        }
        parents.push(parent_of(model, child)?);
        if let ChildRef::Deformer(d) = child
            && matches!(find_deformer(model, d)?.kind, DeformerKind::Unknown)
        {
            return Err(failed(format!(
                "deformer `{d}` has an unknown kind and cannot be moved"
            )));
        }
    }
    if let NewKind::Warp { rows, cols } = kind
        && ![rows, cols]
            .iter()
            .all(|n| (1..=MAX_WARP_DIVISIONS).contains(n))
    {
        return Err(failed(format!(
            "rows and cols must be between 1 and {MAX_WARP_DIVISIONS}"
        )));
    }
    if let NewKind::Rotation {
        origin: Some(origin),
    } = kind
        && !origin.is_finite()
    {
        return Err(failed("origin is not finite"));
    }
    let parent = match parent {
        Some(p) => {
            find_deformer(model, &p)?;
            for child in children {
                if let ChildRef::Deformer(c) = child
                    && is_within(model, &p, c)
                {
                    return Err(failed(format!(
                        "parent `{p}` is the child `{c}` itself or its descendant"
                    )));
                }
            }
            Some(p)
        }
        None => common_parent(model, &parents),
    };
    let id = match id {
        Some(id) => {
            if model.deformers.iter().any(|d| d.id == id) {
                return Err(EditError::DuplicateId {
                    kind: "deformer",
                    id: id.to_string(),
                });
            }
            id
        }
        None => new_deformer_id(model, &id_base),
    };
    let parts: Vec<PartId> = children.iter().filter_map(|c| part_of(model, c)).collect();
    let part = common_part(model, &parts).ok_or_else(|| failed("the children have no part"))?;

    let rest = Rest::new(model)?;
    let base = rest.frame(parent.as_ref());
    let mut world = Vec::new();
    for child in children {
        subtree_points(model, &rest, child, &mut world);
    }
    if world.is_empty() {
        return Err(failed("the children have no points to wrap"));
    }

    let (kind, frame) = match kind {
        NewKind::Warp { rows, cols } => {
            let local = world
                .iter()
                .map(|&w| base.inverse(w).map_err(failed))
                .collect::<Result<Vec<_>, _>>()?;
            let (min, max) = bounds(&local);
            let size = max - min;
            let fallback = if matches!(base, Frame::Warp { .. }) {
                0.01
            } else {
                1.0
            };
            let pad = (size.max_element() * MARGIN).max(if size.max_element() > 0.0 {
                0.0
            } else {
                fallback
            });
            let (min, max) = (min - Vec2::splat(pad), max + Vec2::splat(pad));
            let (min, max) = expand_degenerate(min, max);
            let mut points = Vec::new();
            for y in 0..=rows {
                for x in 0..=cols {
                    let t = Vec2::new(
                        f32::from(x) / f32::from(cols),
                        f32::from(y) / f32::from(rows),
                    );
                    points.push(min + (max - min) * t);
                }
            }
            let frame = Frame::Warp {
                points: points.iter().map(|&p| base.map(p)).collect(),
                cols: usize::from(cols),
                rows: usize::from(rows),
            };
            let kind = DeformerKind::Warp(WarpDeformer {
                rows,
                cols,
                keyforms: KeyformGrid::constant(WarpKeyform {
                    points,
                    opacity: 1.0,
                }),
                depth: None,
            });
            (kind, frame)
        }
        NewKind::Rotation { origin } => {
            let origin = origin.unwrap_or_else(|| {
                let (min, max) = bounds(&world);
                (min + max) * 0.5
            });
            let keyform = RotationKeyform {
                origin: base.inverse(origin).map_err(failed)?,
                angle: 0.0,
                scale: 1.0,
                reflect_x: false,
                reflect_y: false,
                opacity: 1.0,
            };
            let frame = Frame::Affine(base.rotation_child(&keyform));
            let kind = DeformerKind::Rotation(RotationDeformer {
                keyforms: KeyformGrid::constant(keyform),
            });
            (kind, frame)
        }
    };
    model.deformers.push(Deformer {
        id: id.clone(),
        name,
        part,
        parent,
        kind,
        mirror: None,
        mirror_axis: None,
    });
    for (child, old) in children.iter().zip(&parents) {
        reexpress(model, child, &rest.frame(old.as_ref()), &frame)?;
        set_parent(model, child, Some(id.clone()));
        touched.push(target(child));
    }
    Ok(())
}

fn bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    points.iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(lo, hi), &p| (lo.min(p), hi.max(p)),
    )
}

/// 片方の辺だけが 0 の矩形 (横一列に並んだ点など) は、もう片方の辺の長さで正方形に近づける。
fn expand_degenerate(min: Vec2, max: Vec2) -> (Vec2, Vec2) {
    let size = max - min;
    let side = size.max_element();
    let grow = Vec2::new(
        if size.x <= 0.0 { side * 0.5 } else { 0.0 },
        if size.y <= 0.0 { side * 0.5 } else { 0.0 },
    );
    (min - grow, max + grow)
}
