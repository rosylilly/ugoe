//! 深度からの回転 (`docs/spec/keyform-inference.md#深度からの回転`)。

use ugoe_core::Vec2;
use ugoe_model::{
    DeformerId, DeformerKind, KeyformGrid, KeyformInference, KeyformOrigin, Model, ParamId,
    Parameter,
};

use crate::additive::combine;
use crate::grid::{Layout, apply, build, check_sources, flatten, source_cells};
use crate::space::{SpaceKind, parent_space};
use crate::values::Points;
use crate::{InferError, InferReport, InferScope, KeyformTarget};

/// `target` の格子の `derived` のセルを深度からの回転で作り直す
/// (`docs/spec/keyform-inference.md#深度からの回転`)。
///
/// 設定は格子の `inference` (`depth_rotation`)、深度は要素の `depth` から読む。
/// 対象はワープデフォーマ (推奨) とメッシュで、親の座標系がアフィン (親なし・回転デフォーマ) で
/// なければならない。書き換えるのは `scope` のセルで、書いたセルの由来は `depth_rotation` になる。
/// `authored` のセルは変えない。エラーのときはモデルを変えない。
///
/// セル `c` の形は、基準のセルを `c` の `yaw` / `pitch` の角度で回した形 `Rot(c)` に、
/// `authored` のセルと `Rot` の差 (手で直した分) を加算合成したものを足して作る。
/// 手で直したセルがなければ、どのセルも回転の式どおりになる。`yaw` / `pitch` 以外の軸の変化は
/// 加算合成と同じように足し合わさる。
pub fn infer_depth_rotation(
    model: &mut Model,
    target: &KeyformTarget,
    scope: InferScope,
) -> Result<InferReport, InferError> {
    let unknown = || InferError::UnknownTarget(target.clone());
    match target {
        KeyformTarget::Mesh(id) => {
            let mesh = model
                .meshes
                .iter()
                .find(|m| &m.id == id)
                .ok_or_else(unknown)?;
            let unit = affine_unit(model, mesh.parent.as_ref(), target)?;
            let depth = mesh
                .depth
                .as_deref()
                .ok_or_else(|| InferError::MissingDepth(target.clone()))?;
            let writes = rotate_grid(
                &model.parameters,
                &mesh.keyforms,
                depth,
                unit,
                target,
                scope,
            )?;
            let mesh = model
                .meshes
                .iter_mut()
                .find(|m| &m.id == id)
                .ok_or_else(unknown)?;
            Ok(apply(&mut mesh.keyforms, writes, DEPTH))
        }
        KeyformTarget::Deformer(id) => {
            let deformer = model
                .deformers
                .iter()
                .find(|d| &d.id == id)
                .ok_or_else(unknown)?;
            // 深度を持つのはワープだけ。回転デフォーマや未知の種類は回せない。
            let DeformerKind::Warp(warp) = &deformer.kind else {
                return Err(InferError::UnsupportedTarget(target.clone()));
            };
            let unit = affine_unit(model, deformer.parent.as_ref(), target)?;
            let depth = warp
                .depth
                .as_deref()
                .ok_or_else(|| InferError::MissingDepth(target.clone()))?;
            let writes = rotate_grid(
                &model.parameters,
                &warp.keyforms,
                depth,
                unit,
                target,
                scope,
            )?;
            if let Some(DeformerKind::Warp(warp)) = model
                .deformers
                .iter_mut()
                .find(|d| &d.id == id)
                .map(|d| &mut d.kind)
            {
                return Ok(apply(&mut warp.keyforms, writes, DEPTH));
            }
            Err(unknown())
        }
        _ => Err(InferError::UnsupportedTarget(target.clone())),
    }
}

/// 深度からの回転で作ったセルの由来。
const DEPTH: KeyformOrigin = KeyformOrigin::DepthRotation;

/// 親の座標系の 1 単位のピクセル数。親がワープ (アフィンでない) ならエラー。
fn affine_unit(
    model: &Model,
    parent: Option<&DeformerId>,
    target: &KeyformTarget,
) -> Result<f32, InferError> {
    let space = parent_space(model, parent)?;
    if space.kind == SpaceKind::Warp {
        return Err(InferError::NonAffineParent(target.clone()));
    }
    Ok(space.unit)
}

/// 格子の `derived` のセルと、その値を作る。`unit` は親の座標系の 1 単位のピクセル数で、
/// ピクセル単位の深度を親の座標系に直すのに使う。
fn rotate_grid<T: Points>(
    parameters: &[Parameter],
    grid: &KeyformGrid<T>,
    depth: &[f32],
    unit: f32,
    target: &KeyformTarget,
    scope: InferScope,
) -> Result<Vec<(usize, T)>, InferError> {
    let Some(KeyformInference::DepthRotation(settings)) = &grid.inference else {
        return Err(InferError::NotDepthRotation(target.clone()));
    };
    let layout = Layout::new(parameters, grid)?;
    let authored = source_cells(grid, &layout, DEPTH, scope);
    let base = layout.base();
    check_sources(grid, &authored, base)?;
    let base_kf = &grid.keyforms[base];
    if depth.len() != base_kf.shape_len() {
        return Err(InferError::DepthLengthMismatch {
            expected: base_kf.shape_len(),
            found: depth.len(),
        });
    }
    let axis_of = |parameter: &Option<ParamId>| {
        parameter
            .as_ref()
            .map(|id| {
                layout
                    .axes
                    .iter()
                    .position(|a| &a.parameter.id == id)
                    .ok_or_else(|| InferError::MissingRotationAxis {
                        parameter: id.clone(),
                    })
            })
            .transpose()
    };
    let yaw_axis = axis_of(&settings.yaw)?;
    let pitch_axis = axis_of(&settings.pitch)?;
    let pivot = Pivot {
        x: settings.pivot.x,
        y: settings.pivot.y,
        depth: settings.pivot_depth / unit,
    };

    // すべてのセルの回した形 `Rot(c)`。角度は軸の値とパラメータの `default` の差 × angle_scale。
    let rotated: Vec<Vec<f32>> = (0..layout.len)
        .map(|cell| {
            let coords = layout.coords(cell);
            let angle = |axis: Option<usize>| {
                axis.map_or(0.0, |k| layout.offset(k, &coords)) * settings.angle_scale
            };
            let (yaw, pitch) = (angle(yaw_axis).to_radians(), angle(pitch_axis).to_radians());
            let mut kf = base_kf.clone();
            for (p, &d) in kf.points_mut().iter_mut().zip(depth) {
                *p = rotate(*p, d / unit, &pivot, yaw, pitch);
            }
            flatten(&kf)
        })
        .collect();

    // `authored` のセルと `Rot` の差を加算合成し、`Rot` に足す。
    let diff = |v: &[f32], r: &[f32]| v.iter().zip(r).map(|(a, b)| a - b).collect::<Vec<_>>();
    let sum = |e: &[f32], r: &[f32]| e.iter().zip(r).map(|(a, b)| a + b).collect::<Vec<_>>();
    let residuals = (0..layout.len)
        .map(|i| authored[i].then(|| diff(&flatten(&grid.keyforms[i]), &rotated[i])))
        .collect();
    let zero = vec![0.0; rotated[base].len()];
    let written = combine(&layout, &authored, residuals, &zero, |cell, e| {
        let r = &rotated[cell];
        diff(&flatten(&build(base_kf, &sum(e, r))), r)
    });
    Ok(written
        .into_iter()
        .map(|(cell, e)| (cell, build(base_kf, &sum(&e, &rotated[cell]))))
        .collect())
}

/// 回転の中心 `o = (ox, oy, od)`。
struct Pivot {
    x: f32,
    y: f32,
    depth: f32,
}

/// `q = R_y(yaw) · R_x(pitch) · (p − o) + o` の `(q.x, q.y)` (正射影)。角度はラジアン。
///
/// pitch を頭自身の横軸まわりに先に回し、そのあと縦軸 (首) まわりに yaw を回す。
/// 結果の y が x によらないので、斜めのセルでも目の線などの水平な線が傾かない。
///
/// 座標は x が右、y が下、深度 z が手前。正の yaw で手前 (z > 0) の点が右 (+x) へ、
/// 正の pitch で手前の点が上 (−y) へ動く (`AngleX` / `AngleY` の向き、`docs/spec/model.md#標準パラメータ`)。
fn rotate(p: Vec2, depth: f32, o: &Pivot, yaw: f32, pitch: f32) -> Vec2 {
    let (x, y, z) = (p.x - o.x, p.y - o.y, depth - o.depth);
    let (sp, cp) = pitch.sin_cos();
    let (y1, z1) = (y * cp - z * sp, y * sp + z * cp);
    let (sy, cy) = yaw.sin_cos();
    let x2 = x * cy + z1 * sy;
    Vec2::new(x2 + o.x, y1 + o.y)
}
