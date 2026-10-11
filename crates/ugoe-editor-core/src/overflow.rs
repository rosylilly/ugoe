//! 親のワープデフォーマの格子からのはみ出しの検出 (`docs/editor.md#はみ出しの検証`)。
//!
//! ワープの子 (メッシュの頂点・子のワープの制御点・子の回転デフォーマの原点) は、親の UV 空間で表す。
//! UV が `[0,1]²` の外にある点は、範囲外の外挿で親に追従する (`docs/spec/evaluation.md#範囲外の外挿`)。
//! 壊れはしないが格子の中ほど思いどおりに曲がらないので、エディタで「はみ出し」として示す。
//!
//! 子の局所座標 (= 親の UV) は子自身のキーフォームだけで決まり、親の変形には依らない。
//! そこで、子のキーフォームをランタイムと同じ重み ([`ugoe_runtime::keyform::weights`]) で補間して調べる。
//! 読むだけで、モデルは変えない。

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::{DeformerId, KeyformBinding, KeyformGrid, KeyformInterpolation, Model, ParamId};
use ugoe_runtime::ParameterValues as RuntimeValues;
use ugoe_runtime::keyform::{self, ResolvedBinding, Weights};

use crate::command::KeyformTarget;
use crate::space::ParameterValues;

/// はみ出しとみなす UV の距離の既定値 (格子の幅・高さを 1 とした量)。
pub const DEFAULT_TOLERANCE: f32 = 1e-3;

/// [`verify_overflows`] が 1 つの要素で調べるパラメータの値の組み合わせの上限。
/// 超える要素は、各軸の両端のキーの組み合わせだけを調べる ([`OverflowReport::truncated`])。
pub const MAX_COMBINATIONS: usize = 4096;

/// [`verify_overflows`] が曲線補間の軸で、隣り合うキーの間に調べる点の数 (等間隔)。
pub const SMOOTH_SAMPLES: usize = 3;

/// はみ出しを調べられないモデル。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum OverflowError {
    /// 格子の軸が、モデルにないパラメータを指している。
    #[error("{target} refers to unknown parameter `{parameter}`")]
    UnknownParameter {
        target: KeyformTarget,
        parameter: String,
    },
}

/// はみ出した点の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPointKind {
    /// メッシュの頂点 (`index` は頂点の番号)。
    Vertex,
    /// 子のワープの制御点 (`index` は行優先の制御点の番号)。
    ControlPoint,
    /// 子の回転デフォーマの原点 (`index` は 0)。
    Origin,
}

/// はみ出した 1 点。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct OverflowPoint {
    pub index: u32,
    /// 親のワープの UV。
    #[ts(as = "ugoe_core::codegen::Vec2Repr")]
    pub uv: Vec2,
    /// `[0,1]²` からの UV の距離 (格子の幅・高さを 1 とした量)。
    pub amount: f32,
}

/// 1 つの要素の、はみ出した点。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ElementOverflow {
    pub target: KeyformTarget,
    /// 親のワープデフォーマ。
    pub parent: DeformerId,
    pub kind: OverflowPointKind,
    /// はみ出した点 (番号の順)。
    pub points: Vec<OverflowPoint>,
}

/// [`verify_overflows`] で見つけた、ある要素のあるセルでのはみ出し。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct OverflowFinding {
    pub target: KeyformTarget,
    pub parent: DeformerId,
    pub kind: OverflowPointKind,
    /// セル: 要素の格子 (とブレンドシェイプ格子) の軸のパラメータの値。ほかのパラメータは値に依らない。
    #[ts(as = "BTreeMap<String, f32>")]
    pub cell: BTreeMap<ParamId, f32>,
    /// はみ出した点の数。
    pub count: u32,
    /// 最もはみ出した点の番号と量。
    pub worst_index: u32,
    pub max_amount: f32,
}

/// [`verify_overflows`] の結果。
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
pub struct OverflowReport {
    /// 見つけたはみ出し。要素のモデルでの順 (デフォーマ、メッシュ)、同じ要素ではセルの順。
    pub findings: Vec<OverflowFinding>,
    /// 調べた要素 (親がワープの要素) の数。
    pub elements: u32,
    /// 調べたセルの数 (要素ごとの組み合わせの和)。
    pub cells: u32,
    /// 組み合わせが [`MAX_COMBINATIONS`] を超え、各軸の両端のキーだけを調べた要素。
    pub truncated: Vec<KeyformTarget>,
}

/// パラメータの値 `values` (含まないパラメータは既定値) での、はみ出した点を要素ごとに返す。
///
/// はみ出しは、親のワープの UV の `[0,1]²` からの距離が `tolerance` を超える点。はみ出した点のない要素は含めない。
pub fn overflows_at(
    model: &Model,
    values: &ParameterValues,
    tolerance: f32,
) -> Result<Vec<ElementOverflow>, OverflowError> {
    let mut params = RuntimeValues::from_model(model);
    for (id, &value) in values {
        if let Some(index) = params.index_of(id.as_str()) {
            params.set(index, value);
        }
    }
    let defaults = RuntimeValues::from_model(model);
    let mut out = Vec::new();
    for child in warp_children(model) {
        let resolved = child.resolve(&defaults)?;
        let points = collect(&child.local_points(&resolved, params.as_slice()), tolerance);
        if !points.is_empty() {
            out.push(ElementOverflow {
                target: child.target,
                parent: child.parent.clone(),
                kind: child.kind,
                points,
            });
        }
    }
    Ok(out)
}

/// 親がワープのすべての要素について、格子のすべてのセルではみ出しを調べる。
///
/// セルは、要素の格子とブレンドシェイプ格子の軸のパラメータのキーの組み合わせ (ほかのパラメータは
/// 局所座標に効かない)。線形の軸では補間はセルの中で線形なので、UV の `[0,1]²` からの距離 (凸関数) の
/// 最大はセルの角、つまりキーの組み合わせのどれかで起きる。キーの外の値はクランプされるので、軸の端
/// (最小値・最大値) の組み合わせも、両端のキーの組み合わせとして含まれる。曲線補間の軸はキーの間で
/// 行き過ぎることがあるので、キーの間の [`SMOOTH_SAMPLES`] 点も値に加える (その点は近似)。
pub fn verify_overflows(model: &Model, tolerance: f32) -> Result<OverflowReport, OverflowError> {
    let defaults = RuntimeValues::from_model(model);
    let mut report = OverflowReport::default();
    for child in warp_children(model) {
        let resolved = child.resolve(&defaults)?;
        let (axes, truncated) = child.axes(&defaults);
        if truncated {
            report.truncated.push(child.target.clone());
        }
        report.elements += 1;
        let mut values = defaults.as_slice().to_vec();
        for_each_combination(&axes, &mut values, &mut |values| {
            report.cells += 1;
            let points = collect(&child.local_points(&resolved, values), tolerance);
            // 同じ量なら番号の小さい点。
            let worst = points
                .iter()
                .reduce(|a, b| if b.amount > a.amount { b } else { a });
            let Some(worst) = worst else {
                return;
            };
            report.findings.push(OverflowFinding {
                target: child.target.clone(),
                parent: child.parent.clone(),
                kind: child.kind,
                cell: axes
                    .iter()
                    .map(|axis| (axis.id.clone(), values[axis.index]))
                    .collect(),
                count: points.len() as u32,
                worst_index: worst.index,
                max_amount: worst.amount,
            });
        });
    }
    Ok(report)
}

/// 点 `uv` の `[0,1]²` からの距離 (内側なら 0)。
pub fn uv_overflow(uv: Vec2) -> f32 {
    (uv - uv.clamp(Vec2::ZERO, Vec2::ONE)).length()
}

fn collect(points: &[Vec2], tolerance: f32) -> Vec<OverflowPoint> {
    points
        .iter()
        .enumerate()
        .filter_map(|(i, &uv)| {
            let amount = uv_overflow(uv);
            (amount > tolerance).then_some(OverflowPoint {
                index: i as u32,
                uv,
                amount,
            })
        })
        .collect()
}

// ---- 親がワープの要素 ----

enum Grid<'a> {
    Mesh(&'a KeyformGrid<ugoe_model::mesh::MeshKeyform>),
    Warp(&'a KeyformGrid<ugoe_model::deformer::WarpKeyform>),
    Rotation(&'a KeyformGrid<ugoe_model::deformer::RotationKeyform>),
}

struct Child<'a> {
    target: KeyformTarget,
    parent: &'a DeformerId,
    kind: OverflowPointKind,
    grid: Grid<'a>,
}

/// 格子の軸を解決したもの。
struct Resolved {
    bindings: Vec<ResolvedBinding>,
    /// ブレンドシェイプ格子の軸と、既定値での重み。
    shapes: Vec<(Vec<ResolvedBinding>, Weights)>,
}

/// セルの列挙に使う 1 本の軸。
struct Axis {
    id: ParamId,
    index: usize,
    values: Vec<f32>,
}

/// 親がワープデフォーマの要素 (デフォーマ、メッシュの順)。
fn warp_children(model: &Model) -> Vec<Child<'_>> {
    fn warp_parent<'a>(model: &Model, id: &'a Option<DeformerId>) -> Option<&'a DeformerId> {
        let id = id.as_ref()?;
        let parent = model.deformers.iter().find(|d| &d.id == id)?;
        matches!(parent.kind, DeformerKind::Warp(_)).then_some(id)
    }
    let is_warp = |id| warp_parent(model, id);
    let deformers = model.deformers.iter().filter_map(|d| {
        let parent = is_warp(&d.parent)?;
        let (kind, grid) = match &d.kind {
            DeformerKind::Warp(w) => (OverflowPointKind::ControlPoint, Grid::Warp(&w.keyforms)),
            DeformerKind::Rotation(r) => (OverflowPointKind::Origin, Grid::Rotation(&r.keyforms)),
            _ => return None,
        };
        Some(Child {
            target: KeyformTarget::Deformer(d.id.clone()),
            parent,
            kind,
            grid,
        })
    });
    let meshes = model.meshes.iter().filter_map(|m| {
        Some(Child {
            target: KeyformTarget::Mesh(m.id.clone()),
            parent: is_warp(&m.parent)?,
            kind: OverflowPointKind::Vertex,
            grid: Grid::Mesh(&m.keyforms),
        })
    });
    deformers.chain(meshes).collect()
}

impl Child<'_> {
    /// 格子とブレンドシェイプ格子の軸 (`bindings`) と補間の種類。
    fn bindings(&self) -> Vec<(&[KeyformBinding], KeyformInterpolation)> {
        fn all<T>(grid: &KeyformGrid<T>) -> Vec<(&[KeyformBinding], KeyformInterpolation)> {
            std::iter::once(grid)
                .chain(&grid.blend_shapes)
                .map(|g| (g.bindings.as_slice(), g.interpolation))
                .collect()
        }
        match self.grid {
            Grid::Mesh(g) => all(g),
            Grid::Warp(g) => all(g),
            Grid::Rotation(g) => all(g),
        }
    }

    fn resolve(&self, params: &RuntimeValues) -> Result<Resolved, OverflowError> {
        let resolve = |(bindings, interpolation): (&[KeyformBinding], KeyformInterpolation)| {
            bindings
                .iter()
                .map(|b| {
                    let param = params.index_of(b.parameter.as_str()).ok_or_else(|| {
                        OverflowError::UnknownParameter {
                            target: self.target.clone(),
                            parameter: b.parameter.to_string(),
                        }
                    })?;
                    Ok(ResolvedBinding::new(param, b.keys.clone(), interpolation))
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let all = self.bindings();
        let bindings = resolve(all[0])?;
        let shapes = all[1..]
            .iter()
            .map(|&b| {
                let bindings = resolve(b)?;
                let mut rest = Weights::new();
                keyform::weights(&bindings, params.as_slice(), &mut rest);
                Ok((bindings, rest))
            })
            .collect::<Result<_, OverflowError>>()?;
        Ok(Resolved { bindings, shapes })
    }

    /// セルの列挙に使う軸 (パラメータの宣言順) と、組み合わせが多すぎて両端のキーに絞ったか。
    ///
    /// 曲線補間の軸 (`docs/spec/evaluation.md#曲線補間`) はキーの間で行き過ぎる (オーバーシュート) ことが
    /// あるので、キーの間も [`SMOOTH_SAMPLES`] 点ずつ調べる。
    fn axes(&self, params: &RuntimeValues) -> (Vec<Axis>, bool) {
        let mut keys: BTreeMap<usize, (ParamId, Vec<f32>)> = BTreeMap::new();
        for (bindings, interpolation) in self.bindings() {
            for binding in bindings {
                let Some(index) = params.index_of(binding.parameter.as_str()) else {
                    continue;
                };
                let entry = keys
                    .entry(index.0 as usize)
                    .or_insert_with(|| (binding.parameter.clone(), Vec::new()));
                let finite: Vec<f32> = binding
                    .keys
                    .iter()
                    .copied()
                    .filter(|k| k.is_finite())
                    .collect();
                let smooth = ResolvedBinding::new(index, finite.clone(), interpolation).is_smooth();
                if smooth {
                    for pair in finite.windows(2) {
                        entry.1.extend((1..=SMOOTH_SAMPLES).map(|i| {
                            let t = i as f32 / (SMOOTH_SAMPLES + 1) as f32;
                            pair[0] + (pair[1] - pair[0]) * t
                        }));
                    }
                }
                entry.1.extend(finite);
            }
        }
        let mut axes: Vec<Axis> = keys
            .into_iter()
            .map(|(index, (id, mut values))| {
                values.sort_by(f32::total_cmp);
                values.dedup();
                Axis { id, index, values }
            })
            .filter(|a| !a.values.is_empty())
            .collect();
        let count = axes
            .iter()
            .try_fold(1usize, |n, a| n.checked_mul(a.values.len()));
        let truncated = count.is_none_or(|n| n > MAX_COMBINATIONS);
        if truncated {
            for axis in &mut axes {
                let (first, last) = (axis.values[0], axis.values[axis.values.len() - 1]);
                axis.values = if first == last {
                    vec![first]
                } else {
                    vec![first, last]
                };
            }
            // 両端に絞っても多すぎる軸 (2^12 を超える組み合わせ) は、先頭の軸だけを調べる。
            let limit = MAX_COMBINATIONS.ilog2() as usize;
            for axis in axes.iter_mut().skip(limit) {
                axis.values.truncate(1);
            }
        }
        (axes, truncated)
    }

    /// パラメータの値 `values` (ランタイムの並び) での、親の UV 空間の点。
    fn local_points(&self, resolved: &Resolved, values: &[f32]) -> Vec<Vec2> {
        match self.grid {
            Grid::Mesh(g) => interpolate(g, resolved, values, |k| &k.positions),
            Grid::Warp(g) => interpolate(g, resolved, values, |k| &k.points),
            Grid::Rotation(g) => {
                interpolate(g, resolved, values, |k| std::slice::from_ref(&k.origin))
            }
        }
    }
}

/// 格子の点をランタイムと同じ重みで補間し、ブレンドシェイプの差分を加える
/// (`docs/spec/evaluation.md#6-ブレンドシェイプ`。`ugoe_runtime::ModelInstance::mesh_local_positions` と同じ)。
fn interpolate<T>(
    grid: &KeyformGrid<T>,
    resolved: &Resolved,
    values: &[f32],
    points: impl Fn(&T) -> &[Vec2],
) -> Vec<Vec2> {
    let blend = |grid: &KeyformGrid<T>, weights: &Weights, out: &mut Vec<Vec2>, sign: f32| {
        for &(k, w) in weights {
            let Some(keyform) = grid.keyforms.get(k) else {
                continue;
            };
            for (p, q) in out.iter_mut().zip(points(keyform)) {
                *p += *q * (w * sign);
            }
        }
    };
    let count = grid.keyforms.first().map_or(0, |k| points(k).len());
    let mut out = vec![Vec2::ZERO; count];
    let mut weights = Weights::new();
    keyform::weights(&resolved.bindings, values, &mut weights);
    blend(grid, &weights, &mut out, 1.0);
    for (shape, (bindings, rest)) in grid.blend_shapes.iter().zip(&resolved.shapes) {
        keyform::weights(bindings, values, &mut weights);
        if &weights != rest {
            blend(shape, &weights, &mut out, 1.0);
            blend(shape, rest, &mut out, -1.0);
        }
    }
    out
}

/// 軸の値のすべての組み合わせで `f` を呼ぶ (`values` の軸の位置を書き換える。軸 0 が最も速く変わる)。
fn for_each_combination(axes: &[Axis], values: &mut [f32], f: &mut impl FnMut(&[f32])) {
    let Some((axis, rest)) = axes.split_last() else {
        f(values);
        return;
    };
    for &value in &axis.values {
        values[axis.index] = value;
        for_each_combination(rest, values, f);
    }
}
