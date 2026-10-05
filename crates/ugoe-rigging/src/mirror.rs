//! ミラー (`docs/spec/keyform-inference.md#ミラー`)。

use ugoe_core::{Mat2, Vec2};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    DeformerId, DeformerKind, KeyformGrid, KeyformOrigin, MirrorSource, Model, Parameter,
};

use crate::grid::{Layout, apply, rewrites};
use crate::grid_index::key_position;
use crate::space::{Space, parent_space};
use crate::values::Points;
use crate::{InferError, InferReport, InferScope, KeyformTarget, KeyformValues, ParentWorld};

/// ミラーの設定。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MirrorOptions {
    /// 対称軸 `x = a` の `a` (元の要素の親座標系)。指定するとこの実行に限り、元の要素の
    /// `mirror_axis` と親で決まる既定値を上書きする (`MirrorKeyforms` の `axis`)。
    /// 元と先の親が異なれば、キャンバス座標へ写した直線を使う (既定値はキャンバスの原点)。
    pub axis: Option<f32>,
    /// 頂点の対応付けの許容距離 (元の要素の親座標系)。
    /// `None` ならキャンバスの 1px にあたる距離 (親がワープなら UV 空間に換算する)。
    /// 元と先の親が異なれば、対応付けはキャンバス座標で行い、この値をピクセルに換算して使う。
    pub tolerance: Option<f32>,
    /// `true` なら先の `authored` のセルも上書きする (書いたセルは `mirror` になる)。
    pub overwrite_authored: bool,
    /// 先の `derived` のセルのうち書き換えるもの。`Own` ならミラーで作ったセル (`mirror`) だけ。
    pub scope: InferScope,
}

/// `source` のキーフォームを対称軸で反転して、`target` の格子のセルを作る
/// (`docs/spec/keyform-inference.md#ミラー`)。
///
/// - `target` は `source` の `mirror` で対にした要素でなければならない。`mirror` が省略または自分自身
///   なら `target` は `source` 自身 (左右対称な 1 つの要素) で、格子の中で左右のセルを写し合う。
/// - 先のセル `t` は、軸ごとにパラメータを対応させた元のセル `s` を反転して作る。書いたセルの由来は
///   `mirror` になり、先の格子の `mirror_source` に元の要素と `axis` / `tolerance` を記録する
///   (セルを 1 つも書かなければ変えない)。先の `authored` のセルは `overwrite_authored` がなければ
///   変えない。`derived` のセルは `scope` のものだけを書き換える。
/// - `source` と `target` が同じなら、自分自身に写るセル (`s == t`) と、元のセルが `derived` の
///   セルは書かない (推定したセルから推定しない。何度実行しても同じ結果になる)。
/// - `source` と `target` の親が同じなら親の座標系で反転する。親が異なれば、セルの姿勢での
///   親のワールド変換でキャンバス座標へ写して反転し、先の親の座標系へ戻す
///   (`docs/spec/keyform-inference.md#親が異なる対`)。
/// - 対象はメッシュ・ワープデフォーマ・回転デフォーマ。エラーのときはモデルを変えない。
pub fn mirror_keyforms(
    model: &mut Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: &MirrorOptions,
) -> Result<InferReport, InferError> {
    let writes = plan(model, source, target, options)?;
    let unknown = || InferError::UnknownTarget(target.clone());
    match (writes, target) {
        (Writes::Mesh(writes), KeyformTarget::Mesh(t)) => {
            let dst = model.meshes.iter_mut().find(|m| &m.id == t);
            Ok(record(
                &mut dst.ok_or_else(unknown)?.keyforms,
                writes,
                source,
                options,
            ))
        }
        (Writes::Warp(writes), KeyformTarget::Deformer(t)) => match deformer_kind(model, t) {
            Some(DeformerKind::Warp(dw)) => Ok(record(&mut dw.keyforms, writes, source, options)),
            _ => Err(unknown()),
        },
        (Writes::Rotation(writes), KeyformTarget::Deformer(t)) => match deformer_kind(model, t) {
            Some(DeformerKind::Rotation(dr)) => {
                Ok(record(&mut dr.keyforms, writes, source, options))
            }
            _ => Err(unknown()),
        },
        _ => Err(unknown()),
    }
}

/// [`mirror_keyforms`] を、モデルを変えずに試す。ミラーできれば `Ok` (書くセルがなくても `Ok`)。
///
/// エディタが、付け替えの後に左右の対がまだ対応するかを確かめるのに使う
/// (`docs/spec/keyform-inference.md#自動の作り直し`)。
pub fn check_mirror(
    model: &Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: &MirrorOptions,
) -> Result<(), InferError> {
    plan(model, source, target, options).map(drop)
}

fn deformer_kind<'a>(model: &'a mut Model, id: &DeformerId) -> Option<&'a mut DeformerKind> {
    model
        .deformers
        .iter_mut()
        .find(|d| &d.id == id)
        .map(|d| &mut d.kind)
}

/// 先の格子に書き込むセルと値。
enum Writes {
    Mesh(Vec<(usize, MeshKeyform)>),
    Warp(Vec<(usize, WarpKeyform)>),
    Rotation(Vec<(usize, RotationKeyform)>),
}

/// 書き込むセルと値を作る (モデルは変えない)。
fn plan(
    model: &Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: &MirrorOptions,
) -> Result<Writes, InferError> {
    let unknown = |t: &KeyformTarget| InferError::UnknownTarget(t.clone());
    let not_pair = || InferError::NotMirrorPair {
        from: source.clone(),
        to: target.clone(),
    };
    let incompatible = || InferError::IncompatibleMirror {
        from: source.clone(),
        to: target.clone(),
    };
    let same = source == target;
    let params = &model.parameters;
    match (source, target) {
        (KeyformTarget::Mesh(s), KeyformTarget::Mesh(t)) => {
            let src = model
                .meshes
                .iter()
                .find(|m| &m.id == s)
                .ok_or_else(|| unknown(source))?;
            let dst = model
                .meshes
                .iter()
                .find(|m| &m.id == t)
                .ok_or_else(|| unknown(target))?;
            if src.mirror.as_ref().unwrap_or(s) != t {
                return Err(not_pair());
            }
            let space = parent_space(model, src.parent.as_ref())?;
            let explicit = options.axis.or(src.mirror_axis);
            let src_base = base_cell(params, &src.keyforms)?;
            let dst_base = base_cell(params, &dst.keyforms)?;
            if src.parent == dst.parent {
                let axis = explicit.unwrap_or_else(|| space.default_axis(model));
                let tolerance = options.tolerance.unwrap_or(1.0 / space.unit);
                let map = match_vertices(src_base.points(), dst_base.points(), axis, tolerance)?;
                let writes = mirror_grid(
                    params,
                    &src.keyforms,
                    &dst.keyforms,
                    same,
                    options,
                    |_, _, kf| {
                        let mut out = kf.clone();
                        out.positions = map
                            .iter()
                            .map(|&i| reflect(kf.positions[i], axis))
                            .collect();
                        Ok(out)
                    },
                )?;
                return Ok(Writes::Mesh(writes));
            }
            let mut canvas = Canvas::new(
                model,
                (src.parent.as_ref(), dst.parent.as_ref()),
                explicit,
                src_base.points(),
                space,
                options.tolerance,
            )?;
            let map = match_vertices(
                &canvas.source.to_canvas(src_base.points()),
                &canvas.target.to_canvas(dst_base.points()),
                canvas.axis,
                canvas.tolerance,
            )?;
            let layouts = (
                Layout::new(params, &src.keyforms)?,
                Layout::new(params, &dst.keyforms)?,
            );
            let writes = mirror_grid(
                params,
                &src.keyforms,
                &dst.keyforms,
                same,
                options,
                |s, t, kf| {
                    canvas.set_pose(&layouts, s, t);
                    let points: Vec<Vec2> = map
                        .iter()
                        .map(|&i| canvas.reflect(kf.positions[i]))
                        .collect();
                    let mut out = kf.clone();
                    out.positions = canvas.back(
                        &points,
                        &dst.keyforms.keyforms[t].positions,
                        &dst_base.positions,
                    )?;
                    Ok(out)
                },
            )?;
            Ok(Writes::Mesh(writes))
        }
        (KeyformTarget::Deformer(s), KeyformTarget::Deformer(t)) => {
            let src = model
                .deformers
                .iter()
                .find(|d| &d.id == s)
                .ok_or_else(|| unknown(source))?;
            let dst = model
                .deformers
                .iter()
                .find(|d| &d.id == t)
                .ok_or_else(|| unknown(target))?;
            if src.mirror.as_ref().unwrap_or(s) != t {
                return Err(not_pair());
            }
            let space = parent_space(model, src.parent.as_ref())?;
            let explicit = options.axis.or(src.mirror_axis);
            let axis = explicit.unwrap_or_else(|| space.default_axis(model));
            let parents = (src.parent.as_ref(), dst.parent.as_ref());
            let same_parent = parents.0 == parents.1;
            // 未知の種類 (前方互換で読み飛ばしたもの) はミラーできない。
            if let (DeformerKind::Warp(sw), DeformerKind::Warp(dw)) = (&src.kind, &dst.kind) {
                if (sw.rows, sw.cols) != (dw.rows, dw.cols) {
                    return Err(incompatible());
                }
                // 1 行の制御点の数。
                let width = usize::from(sw.cols) + 1;
                let count = width * (usize::from(sw.rows) + 1);
                if let Some((cell, kf)) = sw
                    .keyforms
                    .keyforms
                    .iter()
                    .enumerate()
                    .find(|(_, kf)| kf.points.len() != count)
                {
                    return Err(InferError::ShapeMismatch {
                        cell,
                        expected: count,
                        found: kf.points.len(),
                    });
                }
                // 列を左右反転する: (行 r, 列 c) ← (行 r, 列 cols − c)。
                let column = move |i: usize| (i / width) * width + (width - 1 - i % width);
                if same_parent {
                    let writes = mirror_grid(
                        params,
                        &sw.keyforms,
                        &dw.keyforms,
                        same,
                        options,
                        |_, _, kf| {
                            let mut out = kf.clone();
                            for (i, p) in out.points.iter_mut().enumerate() {
                                *p = reflect(kf.points[column(i)], axis);
                            }
                            Ok(out)
                        },
                    )?;
                    return Ok(Writes::Warp(writes));
                }
                let src_base = base_cell(params, &sw.keyforms)?;
                let dst_base = base_cell(params, &dw.keyforms)?;
                let mut canvas = Canvas::new(
                    model,
                    parents,
                    explicit,
                    src_base.points(),
                    space,
                    options.tolerance,
                )?;
                let layouts = (
                    Layout::new(params, &sw.keyforms)?,
                    Layout::new(params, &dw.keyforms)?,
                );
                let writes = mirror_grid(
                    params,
                    &sw.keyforms,
                    &dw.keyforms,
                    same,
                    options,
                    |s, t, kf| {
                        canvas.set_pose(&layouts, s, t);
                        let points: Vec<Vec2> = (0..count)
                            .map(|i| canvas.reflect(kf.points[column(i)]))
                            .collect();
                        let mut out = kf.clone();
                        out.points = canvas.back(
                            &points,
                            &dw.keyforms.keyforms[t].points,
                            &dst_base.points,
                        )?;
                        Ok(out)
                    },
                )?;
                return Ok(Writes::Warp(writes));
            }
            if let (DeformerKind::Rotation(sr), DeformerKind::Rotation(dr)) = (&src.kind, &dst.kind)
            {
                if same_parent {
                    let writes = mirror_grid(
                        params,
                        &sr.keyforms,
                        &dr.keyforms,
                        same,
                        options,
                        |_, _, kf| {
                            let mut out = *kf;
                            out.origin = reflect(kf.origin, axis);
                            out.angle = -kf.angle;
                            Ok(out)
                        },
                    )?;
                    return Ok(Writes::Rotation(writes));
                }
                let src_base = base_cell(params, &sr.keyforms)?;
                let dst_base = base_cell(params, &dr.keyforms)?;
                let mut canvas = Canvas::new(
                    model,
                    parents,
                    explicit,
                    &[src_base.origin],
                    space,
                    options.tolerance,
                )?;
                let layouts = (
                    Layout::new(params, &sr.keyforms)?,
                    Layout::new(params, &dr.keyforms)?,
                );
                let writes = mirror_grid(
                    params,
                    &sr.keyforms,
                    &dr.keyforms,
                    same,
                    options,
                    |s, t, kf| {
                        canvas.set_pose(&layouts, s, t);
                        canvas.rotation(kf, dr.keyforms.keyforms[t].origin, dst_base.origin)
                    },
                )?;
                return Ok(Writes::Rotation(writes));
            }
            Err(incompatible())
        }
        (KeyformTarget::Mesh(_) | KeyformTarget::Deformer(_), _) => Err(incompatible()),
        _ => Err(InferError::UnsupportedTarget(source.clone())),
    }
}

/// 親が異なる対のミラー (`docs/spec/keyform-inference.md#親が異なる対`)。
///
/// 元の点を元の親のワールド変換でキャンバス座標へ写し、キャンバス座標の対称軸で反転し、
/// 先の親のワールド変換の逆で先の親の座標系へ戻す。親の姿勢は、セルごとにその格子の軸の
/// パラメータをセルのキーにし、残りを既定値にしたもの。
struct Canvas {
    source: ParentWorld,
    target: ParentWorld,
    /// キャンバス座標の対称軸 `x = axis`。
    axis: f32,
    /// 頂点の対応付けの許容距離 (キャンバスのピクセル)。
    tolerance: f32,
}

impl Canvas {
    /// `parents` は (元の親, 先の親)。`explicit` は指定した対称軸 (元の親座標系)、`base` は元の
    /// 基準のセルの点 (元の親座標系)、`space` は元の親の座標系、`tolerance` は指定した許容距離
    /// (元の親座標系)。
    fn new(
        model: &Model,
        parents: (Option<&DeformerId>, Option<&DeformerId>),
        explicit: Option<f32>,
        base: &[Vec2],
        space: Space,
        tolerance: Option<f32>,
    ) -> Result<Self, InferError> {
        // 先の親も、元の親と同じく扱えるものか検査する。
        parent_space(model, parents.1)?;
        let source = ParentWorld::new(model, parents.0).map_err(InferError::ParentTransform)?;
        let target = ParentWorld::new(model, parents.1).map_err(InferError::ParentTransform)?;
        // 指定した軸 (元の親座標系の縦の直線) は、既定の姿勢で、元の基準のセルの点の重心の高さの
        // 1 点をキャンバス座標へ写した x にする (親がワープなら近似)。指定がなければキャンバスの原点。
        let axis = match explicit {
            Some(a) => {
                let y = if base.is_empty() {
                    0.0
                } else {
                    base.iter().map(|p| p.y).sum::<f32>() / base.len() as f32
                };
                source.world().point(Vec2::new(a, y)).x
            }
            None => model.meta.canvas.origin.x,
        };
        Ok(Self {
            source,
            target,
            axis,
            tolerance: tolerance.map_or(1.0, |t| t * space.unit),
        })
    }

    /// 元のセル `s` と先のセル `t` の姿勢にする。`layouts` は (元の格子, 先の格子)。
    fn set_pose(&mut self, layouts: &(Layout<'_>, Layout<'_>), s: usize, t: usize) {
        self.source.set_pose(layouts.0.pose(s));
        self.target.set_pose(layouts.1.pose(t));
    }

    /// 元の親の座標系の点を、キャンバス座標へ写して反転した点。
    fn reflect(&self, p: Vec2) -> Vec2 {
        reflect(self.source.world().point(p), self.axis)
    }

    /// キャンバス座標の点を先の親の座標系へ戻す。反復の初期値は先のセルの今の値 (`current`)、
    /// 解けなければ先の基準のセル (`base`) にする。長さが合わなければ使わない。
    fn back(
        &self,
        points: &[Vec2],
        current: &[Vec2],
        base: &[Vec2],
    ) -> Result<Vec<Vec2>, InferError> {
        let mut error = None;
        for guesses in [current, base] {
            if guesses.len() != points.len() {
                continue;
            }
            match self.target.from_canvas(points, guesses) {
                Ok(local) => return Ok(local),
                Err(e) => error = Some(e),
            }
        }
        match error {
            Some(e) => Err(InferError::ParentTransform(e)),
            None => self
                .target
                .from_canvas(points, points)
                .map_err(InferError::ParentTransform),
        }
    }

    /// 回転デフォーマのキーフォームを反転する。キャンバス座標のワールド変換 `A` を反転した
    /// `F·A·F` (`F` は x の反転) に、先の親の下で最も近くなるように原点・角度・拡大率を決める。
    /// `current` / `base` は先のセルと先の基準のセルの原点 (反復の初期値)。
    fn rotation(
        &self,
        kf: &RotationKeyform,
        current: Vec2,
        base: Vec2,
    ) -> Result<RotationKeyform, InferError> {
        // 拡大率と反転を除いた、元のワールド変換 (Q_s·R(θ_s))。
        let bare = RotationKeyform {
            scale: 1.0,
            reflect_x: false,
            reflect_y: false,
            ..*kf
        };
        let frame = self.source.world().rotation_child(&bare);
        let origin = self.back(
            &[reflect(frame.translation, self.axis)],
            &[current],
            &[base],
        )?[0];
        // 先の親の向き Q_t。
        let parent = self
            .target
            .world()
            .rotation_child(&RotationKeyform {
                origin,
                angle: 0.0,
                ..bare
            })
            .matrix2;
        let flip = Mat2::from_diagonal(Vec2::new(-1.0, 1.0));
        // Q_t·R(θ_t)·S_t = F·Q_s·R(θ_s)·F·S_s になる θ_t と S_t (反転の対角行列は F と可換)。
        let m = parent.inverse() * flip * frame.matrix2 * flip;
        let det = m.determinant();
        if !det.is_finite() || det.abs() < 1e-12 {
            return Err(InferError::ParentTransform(crate::WorldError::Singular {
                index: 0,
            }));
        }
        // 補間が遠回りしないように、親が同じときの値 (−θ_s) に最も近い角度を選ぶ。
        let naive = -kf.angle;
        let angle = m.x_axis.y.atan2(m.x_axis.x).to_degrees();
        Ok(RotationKeyform {
            origin,
            angle: naive + (angle - naive + 180.0).rem_euclid(360.0) - 180.0,
            scale: kf.scale * det.abs().sqrt(),
            ..*kf
        })
    }
}

/// 作ったセルを先の格子に書き込み、由来を `mirror` にして、ミラー元を記録する。
fn record<T>(
    grid: &mut KeyformGrid<T>,
    writes: Vec<(usize, T)>,
    source: &KeyformTarget,
    options: &MirrorOptions,
) -> InferReport {
    if writes.is_empty() {
        return InferReport::default();
    }
    let id = match source {
        KeyformTarget::Part(id) => id.to_string(),
        KeyformTarget::Mesh(id) => id.to_string(),
        KeyformTarget::Deformer(id) => id.to_string(),
        KeyformTarget::Glue(id) => id.to_string(),
    };
    grid.mirror_source = Some(MirrorSource {
        id,
        axis: options.axis,
        tolerance: options.tolerance,
    });
    apply(grid, writes, KeyformOrigin::Mirror)
}

/// 対称軸 `x = axis` での反転。
fn reflect(p: Vec2, axis: f32) -> Vec2 {
    Vec2::new(2.0 * axis - p.x, p.y)
}

fn base_cell<'a, T>(
    parameters: &[Parameter],
    grid: &'a KeyformGrid<T>,
) -> Result<&'a T, InferError> {
    let base = Layout::new(parameters, grid)?.base();
    Ok(&grid.keyforms[base])
}

/// 先の頂点ごとに、対称軸で反転した元の頂点のうち最も近いものの添字を返す
/// (同じ距離なら添字の小さいほう)。許容距離を超える先の頂点があればエラー。
fn match_vertices(
    source: &[Vec2],
    target: &[Vec2],
    axis: f32,
    tolerance: f32,
) -> Result<Vec<usize>, InferError> {
    let mirrored: Vec<Vec2> = source.iter().map(|&p| reflect(p, axis)).collect();
    let mut map = Vec::with_capacity(target.len());
    let mut unmatched = Vec::new();
    for (j, &t) in target.iter().enumerate() {
        let nearest = mirrored
            .iter()
            .enumerate()
            .map(|(i, &p)| (i, p.distance(t)))
            .fold(None, |best: Option<(usize, f32)>, (i, d)| match best {
                Some((_, bd)) if bd <= d => best,
                _ => Some((i, d)),
            });
        match nearest {
            Some((i, d)) if d <= tolerance => map.push(i),
            _ => unmatched.push(j),
        }
    }
    if unmatched.is_empty() {
        Ok(map)
    } else {
        Err(InferError::UnmatchedVertices {
            vertices: unmatched,
            tolerance,
        })
    }
}

/// 先の格子の書き込むセルと、その値を作る。
///
/// 先の軸ごとに、パラメータを対応させた元の軸とキーを引く (`docs/spec/keyform-inference.md#対応付け`)。
/// `mirror` で別のパラメータと対にしたパラメータは相手のパラメータの同じキー、自分自身と対にした
/// パラメータは符号を反転したキー、`mirror` のないパラメータは自分の同じキーに対応する。
fn mirror_grid<T: KeyformValues>(
    parameters: &[Parameter],
    source: &KeyformGrid<T>,
    target: &KeyformGrid<T>,
    same: bool,
    options: &MirrorOptions,
    mut mirror: impl FnMut(usize, usize, &T) -> Result<T, InferError>,
) -> Result<Vec<(usize, T)>, InferError> {
    let src = Layout::new(parameters, source)?;
    let dst = Layout::new(parameters, target)?;

    // 先の軸 k → (元の軸, 先のキーの添字 → 元のキーの添字)。
    let mut used = vec![false; src.axes.len()];
    let mut axes = Vec::with_capacity(dst.axes.len());
    for axis in &dst.axes {
        let p = axis.parameter;
        let (q, negate) = match &p.mirror {
            Some(m) if *m != p.id => (m, false),
            Some(_) => (&p.id, true),
            None => (&p.id, false),
        };
        let k = src
            .axes
            .iter()
            .position(|a| &a.parameter.id == q)
            .filter(|&k| !used[k])
            .ok_or_else(|| InferError::MirrorAxisMismatch {
                parameter: q.clone(),
            })?;
        used[k] = true;
        let keys = axis
            .keys
            .iter()
            .map(|&v| {
                let w = if negate { -v } else { v };
                key_position(src.axes[k].keys, w).ok_or_else(|| InferError::MissingMirrorKey {
                    parameter: q.clone(),
                    key: w,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        axes.push((k, keys));
    }
    if let Some(k) = used.iter().position(|u| !u) {
        return Err(InferError::MirrorAxisMismatch {
            parameter: src.axes[k].parameter.id.clone(),
        });
    }

    let shape = source.keyforms[src.base()].shape_len();
    let mut writes = Vec::new();
    for t in 0..dst.len {
        let write = match target.origin(t) {
            KeyformOrigin::Authored => options.overwrite_authored,
            _ => rewrites(target, t, KeyformOrigin::Mirror, options.scope),
        };
        if !write {
            continue;
        }
        let coords = dst.coords(t);
        let s: usize = axes
            .iter()
            .zip(&coords)
            .map(|((k, keys), &i)| keys[i] * src.axes[*k].stride)
            .sum();
        if same && (s == t || source.origin(s) != KeyformOrigin::Authored) {
            continue;
        }
        let kf = &source.keyforms[s];
        if kf.shape_len() != shape {
            return Err(InferError::ShapeMismatch {
                cell: s,
                expected: shape,
                found: kf.shape_len(),
            });
        }
        writes.push((t, mirror(s, t, kf)?));
    }
    Ok(writes)
}
