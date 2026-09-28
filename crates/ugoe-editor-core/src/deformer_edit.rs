//! デフォーマの作成・見た目を保つ付け替え・ギズモによるセルの編集。
//! 設計と式は `docs/editor.md#デフォーマの編集`。
//!
//! 作成と付け替えは **既定値のパラメータ** で評価した姿 (以下「基準の姿」) を基準にする。
//! 基準の姿での各デフォーマのワールド変換 ([`Frame`]) をランタイムの評価 (評価用のメッシュを足した
//! モデルの複製) から作り、子のキーフォームの局所座標を `新しい親⁻¹ ∘ 古い親` で写し直す。

use std::collections::HashMap;

use ugoe_core::{Affine2, Mat2, Rgb, Vec2};
use ugoe_model::deformer::{
    DeformerKind, RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform,
};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    BlendMode, Deformer, DeformerId, KeyformGrid, MaskMode, Mesh, MeshId, Model, PartId, TextureRef,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::EditError;
use crate::command::{ChildRef, KeyformTarget};
use crate::ids::{new_deformer_id, new_mesh_id};

/// ワープの行数・列数の上限 (制御点の数が大きくなりすぎないように)。
pub const MAX_WARP_DIVISIONS: u16 = 64;
/// 作るデフォーマの外接矩形の余白 (矩形の長い辺に対する割合)。
const MARGIN: f32 = 0.05;
/// ワープの逆写像の許容誤差 (キャンバスのピクセル)。
const INVERSE_TOLERANCE: f32 = 1e-3;
const MAX_ITERATIONS: usize = 64;
/// 親がワープの回転デフォーマの向きを求める差分の幅 (UV 単位、`docs/spec/evaluation.md` の ε)。
const ROTATION_EPSILON: f32 = 0.01;

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
/// `docs/spec/evaluation.md#3-デフォーマのワールド変換` と同じ式。
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

fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

impl Frame {
    fn warp_point(points: &[Vec2], cols: usize, x: usize, y: usize) -> Vec2 {
        points[y * (cols + 1) + x]
    }

    /// 四隅から作ったアフィン近似の u 方向・v 方向の軸と、`(0, 0)` の角。
    fn warp_axes(points: &[Vec2], cols: usize, rows: usize) -> (Vec2, Vec2, Vec2) {
        let c00 = Self::warp_point(points, cols, 0, 0);
        let c10 = Self::warp_point(points, cols, cols, 0);
        let c01 = Self::warp_point(points, cols, 0, rows);
        let c11 = Self::warp_point(points, cols, cols, rows);
        (
            c00,
            ((c10 - c00) + (c11 - c01)) * 0.5,
            ((c01 - c00) + (c11 - c10)) * 0.5,
        )
    }

    /// 局所座標 → キャンバス座標。
    pub(crate) fn map(&self, p: Vec2) -> Vec2 {
        match self {
            Frame::Identity => p,
            Frame::Affine(a) => a.transform_point2(p),
            Frame::Warp { points, cols, rows } => {
                let (cols, rows) = (*cols, *rows);
                let c = p.clamp(Vec2::ZERO, Vec2::ONE);
                let gx = c.x * cols as f32;
                let gy = c.y * rows as f32;
                let i = (gx.floor().max(0.0) as usize).min(cols - 1);
                let j = (gy.floor().max(0.0) as usize).min(rows - 1);
                let (fx, fy) = (gx - i as f32, gy - j as f32);
                let at = |x, y| Self::warp_point(points, cols, x, y);
                let top = lerp(at(i, j), at(i + 1, j), fx);
                let bottom = lerp(at(i, j + 1), at(i + 1, j + 1), fx);
                let base = lerp(top, bottom, fy);
                if c == p {
                    return base;
                }
                let (_, au, av) = Self::warp_axes(points, cols, rows);
                let d = p - c;
                base + au * d.x + av * d.y
            }
        }
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
            Frame::Warp { points, cols, rows } => {
                let (c00, au, av) = Self::warp_axes(points, *cols, *rows);
                let m = Mat2::from_cols(au, av);
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
                    .find_map(|start| self.newton(w, start))
                    .ok_or_else(|| {
                        format!(
                            "cannot map ({}, {}) into the warp deformer (the grid is degenerate)",
                            w.x, w.y
                        )
                    })
            }
        }
    }

    /// 点 `p` での写像のヤコビ行列 (中心差分)。
    fn jacobian(&self, p: Vec2) -> Mat2 {
        let h = 1e-3;
        let (dx, dy) = (Vec2::new(h, 0.0), Vec2::new(0.0, h));
        Mat2::from_cols(
            (self.map(p + dx) - self.map(p - dx)) / (2.0 * h),
            (self.map(p + dy) - self.map(p - dy)) / (2.0 * h),
        )
    }

    /// `map(uv) = w` をニュートン法 (残差が減らなければ歩幅を半分にする) で解く。
    fn newton(&self, w: Vec2, start: Vec2) -> Option<Vec2> {
        let mut uv = start;
        let mut residual = self.map(uv) - w;
        for _ in 0..MAX_ITERATIONS {
            if residual.length() <= INVERSE_TOLERANCE {
                return Some(uv);
            }
            let j = self.jacobian(uv);
            let det = j.determinant();
            if !det.is_finite() || det.abs() < 1e-12 {
                return None;
            }
            let step = j.inverse() * residual;
            let mut t = 1.0;
            loop {
                let next = uv - step * t;
                let r = self.map(next) - w;
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

    /// この座標系を親に持つ回転デフォーマの、原点 `origin` での親の線形部分。
    /// ワールドの線形部分は `rotation_linear(origin) · R(angle) · S` になる
    /// (親が回転ならアフィンの線形部分、ワープなら向き δ の回転)。
    pub(crate) fn rotation_linear(&self, origin: Vec2) -> Mat2 {
        match self {
            Frame::Identity => Mat2::IDENTITY,
            Frame::Affine(a) => a.matrix2,
            Frame::Warp { .. } => {
                let o = self.map(origin);
                let up = self.map(origin + Vec2::new(0.0, -ROTATION_EPSILON)) - o;
                Mat2::from_angle(up.x.atan2(-up.y))
            }
        }
    }

    /// この座標系を親に持つ回転デフォーマのワールド変換。
    fn rotation_child(&self, keyform: &RotationKeyform) -> Affine2 {
        Affine2::from_mat2_translation(
            self.rotation_linear(keyform.origin) * rotation_matrix(keyform),
            self.map(keyform.origin),
        )
    }
}

/// `R(angle) · S` (`docs/spec/evaluation.md` の回転デフォーマ)。
fn rotation_matrix(k: &RotationKeyform) -> Mat2 {
    let sx = if k.reflect_x { -k.scale } else { k.scale };
    let sy = if k.reflect_y { -k.scale } else { k.scale };
    Mat2::from_angle(k.angle.to_radians()) * Mat2::from_diagonal(Vec2::new(sx, sy))
}

// ---- 基準の姿 ----

/// 基準の姿での、各デフォーマのワールド変換とメッシュのキャンバス座標。
pub(crate) struct Rest {
    frames: HashMap<DeformerId, Frame>,
    meshes: HashMap<MeshId, Vec<Vec2>>,
}

impl Rest {
    /// モデルに評価用のメッシュを足し、既定値のパラメータで 1 回評価して作る。
    ///
    /// - ワープ: 制御点 (と同じ格子) を頂点とするメッシュを親の下に置くと、出力がキャンバス座標の制御点になる。
    /// - 回転デフォーマ: `(0,0)`・`(1,0)`・`(0,1)` を頂点とするメッシュを子に置くと、アフィン変換が分かる。
    pub(crate) fn new(model: &Model) -> Result<Self, EditError> {
        let mut probed = model.clone();
        if probed.textures.is_empty() {
            probed.textures.push(TextureRef {
                name: "__ugoe_probe".into(),
                width: 1,
                height: 1,
            });
        }
        let first = probed.meshes.len();
        let mut probes = Vec::new();
        for d in &model.deformers {
            let (parent, keyforms) = match &d.kind {
                DeformerKind::Warp(w) => (
                    d.parent.clone(),
                    KeyformGrid::new(
                        w.keyforms.bindings.clone(),
                        w.keyforms
                            .keyforms
                            .iter()
                            .map(|k| probe_keyform(k.points.clone()))
                            .collect(),
                    ),
                ),
                DeformerKind::Rotation(_) => (
                    Some(d.id.clone()),
                    KeyformGrid::constant(probe_keyform(vec![Vec2::ZERO, Vec2::X, Vec2::Y])),
                ),
                DeformerKind::Unknown => continue,
            };
            let id = new_mesh_id(&probed, "__ugoe_deformer_probe");
            probed
                .meshes
                .push(probe_mesh(id, d.part.clone(), parent, keyforms));
            probes.push(d);
        }
        let compiled = CompiledModel::compile(probed)
            .map_err(|e| failed(format!("the model cannot be evaluated: {e}")))?;
        let instance = ModelInstance::new(compiled);
        let drawables = instance.drawables();

        let meshes = model
            .meshes
            .iter()
            .zip(drawables)
            .map(|(m, d)| (m.id.clone(), d.positions.clone()))
            .collect();
        let mut own: HashMap<DeformerId, Frame> = HashMap::new();
        for (d, out) in probes.into_iter().zip(&drawables[first..]) {
            let p = &out.positions;
            let frame = match &d.kind {
                DeformerKind::Warp(w) => Frame::Warp {
                    points: p.clone(),
                    cols: usize::from(w.cols),
                    rows: usize::from(w.rows),
                },
                _ => Frame::Affine(Affine2::from_mat2_translation(
                    Mat2::from_cols(p[1] - p[0], p[2] - p[0]),
                    p[0],
                )),
            };
            own.insert(d.id.clone(), frame);
        }
        // 未知の種類のデフォーマは恒等変換なので、ワールド変換は祖先の既知のデフォーマのもの。
        let mut frames = HashMap::new();
        for d in &model.deformers {
            let mut current = Some(&d.id);
            let mut frame = Frame::Identity;
            for _ in 0..=model.deformers.len() {
                let Some(id) = current else { break };
                if let Some(f) = own.get(id) {
                    frame = f.clone();
                    break;
                }
                current = model
                    .deformers
                    .iter()
                    .find(|x| &x.id == id)
                    .and_then(|x| x.parent.as_ref());
            }
            frames.insert(d.id.clone(), frame);
        }
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

fn probe_keyform(positions: Vec<Vec2>) -> MeshKeyform {
    MeshKeyform {
        positions,
        opacity: 1.0,
        draw_order: 0.0,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
        mask_strength: 1.0,
    }
}

/// 評価用の、描画に関わらないメッシュ (三角形なし)。
fn probe_mesh(
    id: MeshId,
    part: PartId,
    parent: Option<DeformerId>,
    keyforms: KeyformGrid<MeshKeyform>,
) -> Mesh {
    let n = keyforms.keyforms.first().map_or(0, |k| k.positions.len());
    Mesh {
        name: id.to_string(),
        id,
        part,
        parent,
        texture: 0,
        uvs: vec![Vec2::ZERO; n],
        indices: Vec::new(),
        blend: BlendMode::Normal,
        culling: false,
        masks: Vec::new(),
        mask_mode: MaskMode::Union,
        mask_only: true,
        keyforms,
        mirror: None,
        mirror_axis: None,
        depth: None,
    }
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
fn subtree_points(model: &Model, rest: &Rest, child: &ChildRef, out: &mut Vec<Vec2>) {
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
            for kf in &mut mesh.keyforms.keyforms {
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
                    for kf in &mut w.keyforms.keyforms {
                        for p in &mut kf.points {
                            *p = map(*p)?;
                        }
                    }
                    reference
                }
                DeformerKind::Rotation(r) => {
                    let reference = r.keyforms.keyforms.first().map_or(Vec2::ZERO, |k| k.origin);
                    let mut first_turn = None;
                    for kf in &mut r.keyforms.keyforms {
                        let origin = map(kf.origin)?;
                        let q =
                            to.rotation_linear(origin).inverse() * from.rotation_linear(kf.origin);
                        reorient(kf, q, &mut first_turn);
                        kf.origin = origin;
                    }
                    reference
                }
                DeformerKind::Unknown => {
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

/// `CreateWarpDeformer` / `CreateRotationDeformer`: 子を包むデフォーマを作り、子を見た目を保って付け替える。
pub(crate) fn create(
    model: &mut Model,
    id: Option<DeformerId>,
    name: String,
    children: &[ChildRef],
    parent: Option<DeformerId>,
    kind: NewKind,
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
    if children.is_empty() {
        return Err(failed("no children are given"));
    }
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
        None => new_deformer_id(model, &name),
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
