//! 要素の種類によらないキーフォーム格子の編集 (`docs/editor.md#キーフォームの編集`)。
//!
//! 要素は [`KeyformTarget`] (種類と ID、JSON では `{"kind": "mesh", "id": "Face"}`) で指定する。格子そのものの操作は `crate::grid` にある。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::deformer::{DeformerKind, RotationKeyform, WarpKeyform};
use ugoe_model::mesh::{GlueKeyform, MeshKeyform};
use ugoe_model::part::PartKeyform;
use ugoe_model::{KeyformGrid, Model, ParamId};

use crate::command::{KeyformCell, KeyformTarget};
use crate::document::EditError;
use crate::grid::{self, CellError};

/// エラーの表示に使う要素の種類の名前 (`part`、`deformer`、`mesh`、`glue`) と ID。
fn kind_and_id(target: &KeyformTarget) -> (&'static str, &str) {
    match target {
        KeyformTarget::Part(id) => ("part", id.as_str()),
        KeyformTarget::Deformer(id) => ("deformer", id.as_str()),
        KeyformTarget::Mesh(id) => ("mesh", id.as_str()),
        KeyformTarget::Glue(id) => ("glue", id),
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

/// `Command::AddKeyformKey` を適用する。
pub(crate) fn add_key(
    model: &mut Model,
    target: &KeyformTarget,
    parameter: &ParamId,
    key: f32,
) -> Result<(), EditError> {
    if !model.parameters.iter().any(|p| &p.id == parameter) {
        return Err(not_found("parameter", parameter.as_str()));
    }
    let grid = grid_mut(model, target)?;
    with_grid!(grid, |g| grid::add_key(g, parameter, key)).map_err(cell_error(target))
}

/// `Command::RemoveKeyformKey` を適用する。
pub(crate) fn remove_key(
    model: &mut Model,
    target: &KeyformTarget,
    parameter: &ParamId,
    key: f32,
) -> Result<(), EditError> {
    let grid = grid_mut(model, target)?;
    with_grid!(grid, |g| grid::remove_key(g, parameter, key)).map_err(cell_error(target))
}

/// `Command::SetKeyformCell` を適用する。
pub(crate) fn set_cell(
    model: &mut Model,
    target: &KeyformTarget,
    cell: &KeyformCell,
    keyform: Keyform,
) -> Result<(), EditError> {
    let grid = grid_mut(model, target)?;
    let result = match (grid, keyform) {
        (GridMut::Part(g), Keyform::Part(k)) => grid::set_cell(g, cell, k),
        (GridMut::Warp(g), Keyform::Warp(k)) => grid::set_cell(g, cell, k),
        (GridMut::Rotation(g), Keyform::Rotation(k)) => grid::set_cell(g, cell, k),
        (GridMut::Mesh(g), Keyform::Mesh(k)) => grid::set_cell(g, cell, k),
        (GridMut::Glue(g), Keyform::Glue(k)) => grid::set_cell(g, cell, k),
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
