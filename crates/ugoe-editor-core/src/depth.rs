//! 深度 (`Mesh::depth` / `WarpDeformer::depth`) の編集 (`docs/editor.md#深度の編集`、
//! `docs/spec/keyform-inference.md#深度の入力`)。
//!
//! 深度は頂点 (制御点) ごとの奥行き (キャンバスのピクセル単位、手前が正) で、深度からの回転の入力になる。
//! 作り方のプリセット ([`DepthPreset`]) の計算は純粋な関数 ([`uniform_depth`]・[`dome_depth`]・
//! [`points_depth`]) にしてある。

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::{KeyformGrid, Model, ParamId};

use crate::command::{ChildRef, KeyformTarget};
use crate::document::EditError;
use crate::grid;
use crate::keyform::parameter_defaults;

/// [`crate::Command::ApplyDepthPreset`] の深度の作り方。JSON では `{"kind": "dome", "peak": 20}` と書く。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DepthPreset {
    /// すべての点を同じ深度 `value` にする ([`uniform_depth`])。
    Uniform { value: f32 },
    /// 基準の姿の外接矩形に内接する楕円の上に、中心の深度が `peak` の半球のふくらみを作る
    /// ([`dome_depth`])。楕円の外の点は 0。
    Dome { peak: f32 },
    /// 点 `indices` (頂点・制御点の番号) の深度を `value` にし、他の点は今の深度 (なければ 0) のまま
    /// にする ([`points_depth`])。
    Points { indices: Vec<u32>, value: f32 },
}

/// `count` 個の点をすべて深度 `value` にする。
pub fn uniform_depth(count: usize, value: f32) -> Vec<f32> {
    vec![value; count]
}

/// 点 `points` (基準の姿) のドーム状の深度。
///
/// 点の外接矩形の中心を `c`、半分の幅・高さを `r` とし、内接する楕円で正規化した距離
/// `ρ = |((x − cx) / rx, (y − cy) / ry)|` から `peak · sqrt(max(0, 1 − ρ²))` とする
/// (中心で `peak`、楕円の上と外で 0 の半球)。幅や高さが 0 の向きは距離に数えない。
pub fn dome_depth(points: &[Vec2], peak: f32) -> Vec<f32> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let (min, max) = points
        .iter()
        .fold((*first, *first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let center = (min + max) * 0.5;
    let radius = (max - min) * 0.5;
    let term = |d: f32, r: f32| if r > 0.0 { (d / r).powi(2) } else { 0.0 };
    points
        .iter()
        .map(|p| {
            let rho2 = term(p.x - center.x, radius.x) + term(p.y - center.y, radius.y);
            peak * (1.0 - rho2).max(0.0).sqrt()
        })
        .collect()
}

/// 今の深度 `current` (なければ 0) の、点 `indices` だけを `value` にした `count` 個の深度。
/// 範囲外の番号があれば、その番号を `Err` で返す。
pub fn points_depth(
    current: Option<&[f32]>,
    count: usize,
    indices: &[u32],
    value: f32,
) -> Result<Vec<f32>, u32> {
    let mut depth = match current {
        Some(d) if d.len() == count => d.to_vec(),
        _ => vec![0.0; count],
    };
    for &index in indices {
        *depth.get_mut(index as usize).ok_or(index)? = value;
    }
    Ok(depth)
}

/// 深度を持つ要素 (メッシュかワープデフォーマ) の、深度と点の数と基準の姿。
struct DepthSlot<'a> {
    depth: &'a mut Option<Vec<f32>>,
    count: usize,
    /// 基準の姿のセル ([`grid::rest_cell_index`]) の点 (親座標系)。
    rest: Vec<Vec2>,
}

fn slot<'a>(model: &'a mut Model, target: &ChildRef) -> Result<DepthSlot<'a>, EditError> {
    let defaults = parameter_defaults(model);
    match target {
        ChildRef::Mesh(id) => {
            let mesh = model
                .meshes
                .iter_mut()
                .find(|m| &m.id == id)
                .ok_or_else(|| EditError::NotFound {
                    kind: "mesh",
                    id: id.to_string(),
                })?;
            Ok(DepthSlot {
                count: mesh.uvs.len(),
                rest: rest_points(&mesh.keyforms, &defaults, |k| &k.positions),
                depth: &mut mesh.depth,
            })
        }
        ChildRef::Deformer(id) => {
            let deformer = model
                .deformers
                .iter_mut()
                .find(|d| &d.id == id)
                .ok_or_else(|| EditError::NotFound {
                    kind: "deformer",
                    id: id.to_string(),
                })?;
            let DeformerKind::Warp(warp) = &mut deformer.kind else {
                return Err(EditError::DeformerKindMismatch {
                    id: id.to_string(),
                    expected: "warp",
                });
            };
            Ok(DepthSlot {
                count: (usize::from(warp.rows) + 1) * (usize::from(warp.cols) + 1),
                rest: rest_points(&warp.keyforms, &defaults, |k| &k.points),
                depth: &mut warp.depth,
            })
        }
    }
}

/// 基準の姿のセル ([`grid::rest_cell_index`]) の点。格子が空なら空。
fn rest_points<T>(
    grid: &KeyformGrid<T>,
    defaults: &HashMap<ParamId, f32>,
    points: impl Fn(&T) -> &Vec<Vec2>,
) -> Vec<Vec2> {
    let index = grid::rest_cell_index(grid, |p| defaults.get(p).copied());
    grid.keyforms
        .get(index)
        .map(points)
        .cloned()
        .unwrap_or_default()
}

fn keyform_target(target: &ChildRef) -> KeyformTarget {
    match target {
        ChildRef::Mesh(id) => KeyformTarget::Mesh(id.clone()),
        ChildRef::Deformer(id) => KeyformTarget::Deformer(id.clone()),
    }
}

/// [`crate::Command::SetDepth`]。深度を検査して置き換え、深度からの回転のセルを作り直すために
/// 格子の要素を返す (呼び出し側が `touched` に積む)。
pub(crate) fn set_depth(
    model: &mut Model,
    target: &ChildRef,
    depth: Option<Vec<f32>>,
) -> Result<KeyformTarget, EditError> {
    let slot = slot(model, target)?;
    if let Some(depth) = &depth {
        check(target, slot.count, depth)?;
    }
    *slot.depth = depth;
    Ok(keyform_target(target))
}

/// [`crate::Command::ApplyDepthPreset`]。プリセットで作った深度で [`set_depth`] と同じく置き換える。
pub(crate) fn apply_preset(
    model: &mut Model,
    target: &ChildRef,
    preset: &DepthPreset,
) -> Result<KeyformTarget, EditError> {
    let slot = slot(model, target)?;
    let depth = match preset {
        DepthPreset::Uniform { value } => uniform_depth(slot.count, *value),
        // 基準のセルの点の数が合わない (不正な格子) ときは、長さの検査で拒否される。
        DepthPreset::Dome { peak } => dome_depth(&slot.rest, *peak),
        DepthPreset::Points { indices, value } => {
            points_depth(slot.depth.as_deref(), slot.count, indices, *value).map_err(|index| {
                EditError::DepthPointOutOfRange {
                    target: keyform_target(target).to_string(),
                    index,
                    points: slot.count,
                }
            })?
        }
    };
    check(target, slot.count, &depth)?;
    *slot.depth = Some(depth);
    Ok(keyform_target(target))
}

/// 長さが点の数 `count` と同じで、すべての値が有限であること。
fn check(target: &ChildRef, count: usize, depth: &[f32]) -> Result<(), EditError> {
    let name = || keyform_target(target).to_string();
    if depth.len() != count {
        return Err(EditError::DepthLength {
            target: name(),
            expected: count,
            actual: depth.len(),
        });
    }
    if let Some(&value) = depth.iter().find(|v| !v.is_finite()) {
        return Err(EditError::NonFiniteDepth {
            target: name(),
            value,
        });
    }
    Ok(())
}
