//! 要素の種類によらないキーフォーム格子の編集 (`docs/editor.md#キーフォームの編集`)。
//!
//! 要素は [`KeyformTarget`] (種類と ID、JSON では `{"kind": "mesh", "id": "Face"}`) で指定する。格子そのものの操作は `crate::grid` にある。

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::deformer::{DeformerKind, RotationKeyform, WarpKeyform};
use ugoe_model::mesh::{GlueKeyform, MeshKeyform};
use ugoe_model::parameter::ParameterKind;
use ugoe_model::part::PartKeyform;
use ugoe_model::{KeyformGrid, KeyformInference, KeyformInterpolation, Model, ParamId};
use ugoe_rigging::{InferScope, KeyformValues, infer_additive_target};

use crate::command::{KeyformCell, KeyformTarget};
use crate::document::EditError;
use crate::grid::{self, CellError};

/// エラーの表示に使う要素の種類の名前 (`part`、`deformer`、`mesh`、`glue`) と ID。
fn kind_and_id(target: &KeyformTarget) -> (&'static str, &str) {
    match target {
        KeyformTarget::Part(id) => ("part", id.as_str()),
        KeyformTarget::Deformer(id) => ("deformer", id.as_str()),
        KeyformTarget::Mesh(id) => ("mesh", id.as_str()),
        KeyformTarget::Glue(id) => ("glue", id.as_str()),
    }
}

/// 1 つのセルのキーフォーム。種類は対象の格子と一致させる。
/// JSON では `{"kind": "mesh", "positions": [..], "opacity": 1, ..}` のようにフィールドを並べる。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Keyform {
    Part(PartKeyform),
    Warp(WarpKeyform),
    Rotation(RotationKeyform),
    Mesh(MeshKeyform),
    Glue(GlueKeyform),
}

impl Keyform {
    fn kind(&self) -> &'static str {
        match self {
            Self::Part(_) => "part",
            Self::Warp(_) => "warp",
            Self::Rotation(_) => "rotation",
            Self::Mesh(_) => "mesh",
            Self::Glue(_) => "glue",
        }
    }
}

/// 要素の格子への可変参照。種類ごとに型が違うので列挙で持つ。
enum GridMut<'a> {
    Part(&'a mut KeyformGrid<PartKeyform>),
    Warp(&'a mut KeyformGrid<WarpKeyform>),
    Rotation(&'a mut KeyformGrid<RotationKeyform>),
    Mesh(&'a mut KeyformGrid<MeshKeyform>),
    Glue(&'a mut KeyformGrid<GlueKeyform>),
}

impl GridMut<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Self::Part(_) => "part",
            Self::Warp(_) => "warp",
            Self::Rotation(_) => "rotation",
            Self::Mesh(_) => "mesh",
            Self::Glue(_) => "glue",
        }
    }
}

/// `$grid` (`GridMut`) の種類ごとに、格子を `$g` に束縛して `$body` を評価する。
macro_rules! with_grid {
    ($grid:expr, |$g:ident| $body:expr) => {
        match $grid {
            GridMut::Part($g) => $body,
            GridMut::Warp($g) => $body,
            GridMut::Rotation($g) => $body,
            GridMut::Mesh($g) => $body,
            GridMut::Glue($g) => $body,
        }
    };
}

fn not_found(kind: &'static str, id: &str) -> EditError {
    EditError::NotFound {
        kind,
        id: id.to_owned(),
    }
}

fn grid_mut<'a>(model: &'a mut Model, target: &KeyformTarget) -> Result<GridMut<'a>, EditError> {
    let missing = || {
        let (kind, id) = kind_and_id(target);
        not_found(kind, id)
    };
    Ok(match target {
        KeyformTarget::Part(id) => {
            let part = model.parts.iter_mut().find(|p| &p.id == id);
            GridMut::Part(&mut part.ok_or_else(missing)?.keyforms)
        }
        KeyformTarget::Deformer(id) => {
            let deformer = model.deformers.iter_mut().find(|d| &d.id == id);
            match &mut deformer.ok_or_else(missing)?.kind {
                DeformerKind::Warp(warp) => GridMut::Warp(&mut warp.keyforms),
                DeformerKind::Rotation(rotation) => GridMut::Rotation(&mut rotation.keyforms),
                DeformerKind::Unknown => {
                    return Err(EditError::DeformerKindMismatch {
                        id: id.to_string(),
                        expected: "warp or rotation",
                    });
                }
            }
        }
        KeyformTarget::Mesh(id) => {
            let mesh = model.meshes.iter_mut().find(|m| &m.id == id);
            GridMut::Mesh(&mut mesh.ok_or_else(missing)?.keyforms)
        }
        KeyformTarget::Glue(id) => {
            let glue = model.glues.iter_mut().find(|g| &g.id == id);
            GridMut::Glue(&mut glue.ok_or_else(missing)?.keyforms)
        }
    })
}

fn cell_error(target: &KeyformTarget) -> impl Fn(CellError) -> EditError + '_ {
    |error| {
        let (kind, id) = kind_and_id(target);
        EditError::InvalidCell {
            kind,
            id: id.to_owned(),
            error,
        }
    }
}

/// `Command::SetKeyformInterpolation` を適用する (`docs/spec/evaluation.md#曲線補間`)。
///
/// `blend_shape` を指定すると、そのパラメータを軸に持つブレンドシェイプ格子に設定する。
pub(crate) fn set_interpolation(
    model: &mut Model,
    target: &KeyformTarget,
    blend_shape: Option<&ParamId>,
    interpolation: KeyformInterpolation,
) -> Result<(), EditError> {
    let grid = grid_mut(model, target)?;
    // パーツとグルーの格子は形を持たないので、曲線補間は効かない (ランタイムはいつも線形で補間する)。
    let shapeless = matches!(grid, GridMut::Part(_) | GridMut::Glue(_));
    if interpolation == KeyformInterpolation::Unknown
        || (shapeless && interpolation == KeyformInterpolation::Smooth)
    {
        return Err(EditError::UnsupportedInterpolation {
            target: target.to_string(),
            interpolation,
        });
    }
    with_grid!(grid, |g| {
        let g = match blend_shape {
            None => g,
            Some(parameter) => {
                let index = g
                    .blend_shape_index(parameter)
                    .ok_or_else(|| not_found("blend shape grid", parameter.as_str()))?;
                &mut g.blend_shapes[index]
            }
        };
        g.interpolation = interpolation;
    });
    Ok(())
}

/// `Command::AddKeyformKey` を適用する (`docs/spec/keyform-inference.md#キーの追加とセルの由来`)。
///
/// 新しいセルは補間した値で埋め (`grid::add_key`)、そのうち組み合わせのセルを
/// [`derive_combinations`] で加算合成のセルにする。既存のセルは作り直さない。
pub(crate) fn add_key(
    model: &mut Model,
    target: &KeyformTarget,
    parameter: &ParamId,
    key: f32,
) -> Result<(), EditError> {
    if !model.parameters.iter().any(|p| &p.id == parameter) {
        return Err(not_found("parameter", parameter.as_str()));
    }
    let defaults = parameter_defaults(model);
    let default_of = |p: &ParamId| defaults.get(p).copied();
    let blend_shape = blend_shape_parameters(model).contains(parameter);
    let grid = grid_mut(model, target)?;
    if blend_shape {
        // ブレンドシェイプ格子は推定の対象にしない (組み合わせのセルも作り直さない)。
        return with_grid!(grid, |g| grid::add_blend_shape_key(
            g, parameter, key, default_of
        ))
        .map_err(cell_error(target));
    }
    let added = with_grid!(grid, |g| grid::add_key(g, parameter, key, default_of))
        .map_err(cell_error(target))?;
    derive_combinations(model, target, &added);
    Ok(())
}

/// `kind = blend_shape` のパラメータの ID (`docs/spec/model.md#ブレンドシェイプ`)。
/// キーの追加・削除とセルの指定は、これらのパラメータならブレンドシェイプ格子に振り分ける。
pub(crate) fn blend_shape_parameters(model: &Model) -> HashSet<ParamId> {
    model
        .parameters
        .iter()
        .filter(|p| p.kind == ParameterKind::BlendShape)
        .map(|p| p.id.clone())
        .collect()
}

/// パラメータ ID → 既定値。
pub(crate) fn parameter_defaults(model: &Model) -> HashMap<ParamId, f32> {
    model
        .parameters
        .iter()
        .map(|p| (p.id.clone(), p.default))
        .collect()
}

/// キーの追加でできたセル `added` のうち組み合わせのセル (中立でない軸が 2 本以上) を、その場で
/// 加算合成で推定し、推定した値が補間した値と変わらない (見た目が変わらない) セルだけを `additive` にする。
///
/// 推定は、組み合わせのセルだけを `derived` にし、他のセル (既存の `derived` のセルと新しい 1 軸の
/// セルを含む) を書き換えずに元にして行う。格子の推定の方法が加算合成でない (深度からの回転・未知) か、
/// 中立のキーがない軸があるか、推定に失敗したときは何もしない。推定しなかったセルと、推定した値が
/// 補間した値と違うセルの由来は `grid::add_key` が決めたまま。
fn derive_combinations(model: &mut Model, target: &KeyformTarget, added: &[usize]) {
    let defaults = parameter_defaults(model);
    let default_of = |p: &ParamId| defaults.get(p).copied();
    let Ok(grid) = grid_mut(model, target) else {
        return;
    };
    let cells = with_grid!(grid, |g| match g.inference {
        None | Some(KeyformInference::Additive) => {
            grid::combination_cells(&g.bindings, default_of, added)
        }
        Some(_) => Vec::new(),
    });
    if cells.is_empty() {
        return;
    }
    let mut scratch = model.clone();
    if let Ok(g) = grid_mut(&mut scratch, target) {
        with_grid!(g, |g| grid::derive_only(g, &cells));
    }
    if infer_additive_target(&mut scratch, &target.into(), InferScope::Own).is_err() {
        return;
    }
    let (Ok(grid), Ok(inferred)) = (grid_mut(model, target), grid_mut(&mut scratch, target)) else {
        return;
    };
    match (grid, inferred) {
        (GridMut::Part(g), GridMut::Part(s)) => grid::adopt_inferred(g, s, &cells),
        (GridMut::Warp(g), GridMut::Warp(s)) => grid::adopt_inferred(g, s, &cells),
        (GridMut::Rotation(g), GridMut::Rotation(s)) => grid::adopt_inferred(g, s, &cells),
        (GridMut::Mesh(g), GridMut::Mesh(s)) => grid::adopt_inferred(g, s, &cells),
        (GridMut::Glue(g), GridMut::Glue(s)) => grid::adopt_inferred(g, s, &cells),
        _ => {}
    }
}

/// 要素の格子の未編集のセル (`grid::unedited_cells`) の線形インデックス。
pub(crate) fn unedited_cells(
    model: &mut Model,
    target: &KeyformTarget,
) -> Result<Vec<usize>, EditError> {
    let defaults = parameter_defaults(model);
    let grid = grid_mut(model, target)?;
    Ok(with_grid!(grid, |g| grid::unedited_cells(g, |p| defaults
        .get(p)
        .copied())))
}

/// `Command::RemoveKeyformKey` を適用する。
pub(crate) fn remove_key(
    model: &mut Model,
    target: &KeyformTarget,
    parameter: &ParamId,
    key: f32,
) -> Result<(), EditError> {
    let blend_shape = blend_shape_parameters(model).contains(parameter);
    let grid = grid_mut(model, target)?;
    if blend_shape {
        return with_grid!(grid, |g| grid::remove_blend_shape_key(g, parameter, key))
            .map_err(cell_error(target));
    }
    with_grid!(grid, |g| grid::remove_key(g, parameter, key)).map_err(cell_error(target))
}

/// `Command::SetKeyformCell` を適用する。
pub(crate) fn set_cell(
    model: &mut Model,
    target: &KeyformTarget,
    cell: &KeyformCell,
    keyform: Keyform,
) -> Result<(), EditError> {
    let shapes = blend_shape_parameters(model);
    let grid = grid_mut(model, target)?;
    let result = match (grid, keyform) {
        (GridMut::Part(g), Keyform::Part(k)) => set_routed(g, cell, k, &shapes),
        (GridMut::Warp(g), Keyform::Warp(k)) => set_routed(g, cell, k, &shapes),
        (GridMut::Rotation(g), Keyform::Rotation(k)) => set_routed(g, cell, k, &shapes),
        (GridMut::Mesh(g), Keyform::Mesh(k)) => set_routed(g, cell, k, &shapes),
        (GridMut::Glue(g), Keyform::Glue(k)) => set_routed(g, cell, k, &shapes),
        (grid, keyform) => {
            let (kind, id) = kind_and_id(target);
            return Err(EditError::KeyformKindMismatch {
                kind,
                id: id.to_owned(),
                expected: grid.kind(),
                actual: keyform.kind(),
            });
        }
    };
    result.map_err(cell_error(target))
}

/// セル `cell` を、指定が指す格子 (`grid::cell_grid_mut`) で置き換える。
fn set_routed<T: KeyformValues>(
    grid: &mut KeyformGrid<T>,
    cell: &KeyformCell,
    value: T,
    shapes: &HashSet<ParamId>,
) -> Result<(), CellError> {
    let grid = grid::cell_grid_mut(grid, cell, |p| shapes.contains(p))?;
    grid::set_cell(grid, cell, value)
}
