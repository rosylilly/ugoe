//! ミラー (`docs/spec/keyform-inference.md#ミラー`)。

use ugoe_core::Vec2;
use ugoe_model::{DeformerKind, KeyformGrid, KeyformOrigin, MirrorSource, Model, Parameter};

use crate::grid::{KEY_EPSILON, Layout, apply, rewrites};
use crate::space::parent_space;
use crate::values::Points;
use crate::{InferError, InferReport, InferScope, KeyformTarget, KeyformValues};

/// ミラーの設定。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MirrorOptions {
    /// 対称軸 `x = a` の `a` (元の要素の親座標系)。指定するとこの実行に限り、元の要素の
    /// `mirror_axis` と親で決まる既定値を上書きする (`MirrorKeyforms` の `axis`)。
    pub axis: Option<f32>,
    /// 頂点の対応付けの許容距離 (元の要素の親座標系)。
    /// `None` ならキャンバスの 1px にあたる距離 (親がワープなら UV 空間に換算する)。
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
/// - 対象はメッシュ・ワープデフォーマ・回転デフォーマ。エラーのときはモデルを変えない。
pub fn mirror_keyforms(
    model: &mut Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: &MirrorOptions,
) -> Result<InferReport, InferError> {
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
            let axis = options
                .axis
                .or(src.mirror_axis)
                .unwrap_or_else(|| space.default_axis(model));
            let tolerance = options.tolerance.unwrap_or(1.0 / space.unit);
            let params = &model.parameters;
            let src_base = base_cell(params, &src.keyforms)?;
            let dst_base = base_cell(params, &dst.keyforms)?;
            let map = match_vertices(src_base.points(), dst_base.points(), axis, tolerance)?;
            let writes = mirror_grid(params, &src.keyforms, &dst.keyforms, same, options, |kf| {
                let mut out = kf.clone();
                out.positions = map
                    .iter()
                    .map(|&i| reflect(kf.positions[i], axis))
                    .collect();
                out
            })?;
            let dst = model
                .meshes
                .iter_mut()
                .find(|m| &m.id == t)
                .ok_or_else(|| unknown(target))?;
            Ok(record(&mut dst.keyforms, writes, source, options))
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
            let axis = options
                .axis
                .or(src.mirror_axis)
                .unwrap_or_else(|| space.default_axis(model));
            let params = &model.parameters;
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
                let writes =
                    mirror_grid(params, &sw.keyforms, &dw.keyforms, same, options, |kf| {
                        let mut out = kf.clone();
                        for (i, p) in out.points.iter_mut().enumerate() {
                            // 列を左右反転する: (行 r, 列 c) ← (行 r, 列 cols − c)。
                            let (r, c) = (i / width, i % width);
                            *p = reflect(kf.points[r * width + (width - 1 - c)], axis);
                        }
                        out
                    })?;
                if let Some(DeformerKind::Warp(dw)) = model
                    .deformers
                    .iter_mut()
                    .find(|d| &d.id == t)
                    .map(|d| &mut d.kind)
                {
                    return Ok(record(&mut dw.keyforms, writes, source, options));
                }
                return Err(unknown(target));
            }
            if let (DeformerKind::Rotation(sr), DeformerKind::Rotation(dr)) = (&src.kind, &dst.kind)
            {
                let writes =
                    mirror_grid(params, &sr.keyforms, &dr.keyforms, same, options, |kf| {
                        let mut out = *kf;
                        out.origin = reflect(kf.origin, axis);
                        out.angle = -kf.angle;
                        out
                    })?;
                if let Some(DeformerKind::Rotation(dr)) = model
                    .deformers
                    .iter_mut()
                    .find(|d| &d.id == t)
                    .map(|d| &mut d.kind)
                {
                    return Ok(record(&mut dr.keyforms, writes, source, options));
                }
                return Err(unknown(target));
            }
            Err(incompatible())
        }
        (KeyformTarget::Mesh(_) | KeyformTarget::Deformer(_), _) => Err(incompatible()),
        _ => Err(InferError::UnsupportedTarget(source.clone())),
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
        KeyformTarget::Glue(id) => id.clone(),
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
    mirror: impl Fn(&T) -> T,
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
                src.axes[k]
                    .keys
                    .iter()
                    .position(|&key| (key - w).abs() <= KEY_EPSILON)
                    .ok_or_else(|| InferError::MissingMirrorKey {
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
        writes.push((t, mirror(kf)));
    }
    Ok(writes)
}
