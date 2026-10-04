//! 要素の親の座標系とキャンバス座標の変換 (`docs/editor.md#キーフォームの編集`)。
//!
//! ビューポートでの編集はキャンバス座標で行うが、キーフォームの頂点・制御点・原点は親の座標系
//! (親なしならキャンバス座標、親がワープなら UV 空間、回転デフォーマなら原点を (0,0) とした座標)
//! で表す。写像と逆写像 (ニュートン法) は、親が異なる左右の対のミラーと共通の
//! [`ugoe_rigging::ParentWorld`] で求める。

use std::collections::BTreeMap;

pub use ugoe_core::Vec2;
use ugoe_model::{Model, ParamId};
use ugoe_rigging::{ParentWorld, WorldError};

use crate::command::KeyformTarget;

/// 変換に失敗した理由。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SpaceError {
    #[error("{kind} `{id}` not found")]
    NotFound { kind: &'static str, id: String },
    /// 座標を持たない要素 (パーツ・グルー)。
    #[error("{kind} `{id}` has no coordinate space")]
    NoSpace { kind: &'static str, id: String },
    /// 親の祖先のデフォーマを評価できない (不正なモデルなど)。
    #[error("cannot evaluate the parent deformers: {0}")]
    Evaluate(String),
    #[error("{points} points but {guesses} initial guesses")]
    LengthMismatch { points: usize, guesses: usize },
    /// 親の変換が点 `index` の近くで潰れていて、逆に解けない。
    #[error("the parent transform is degenerate near point {index}")]
    Singular { index: usize },
    #[error("point {index} did not converge (residual {residual} px)")]
    NotConverged { index: usize, residual: f32 },
}

/// 評価に使うパラメータの値 (ID → 値)。含まないパラメータは既定値。
pub type ParameterValues = BTreeMap<ParamId, f32>;

/// `target` の要素の親の座標系の点 `points` を、キャンバス座標に写す。
pub fn local_to_canvas(
    model: &Model,
    target: &KeyformTarget,
    values: &ParameterValues,
    points: &[Vec2],
) -> Result<Vec<Vec2>, SpaceError> {
    Ok(parent_world(model, target, values)?.to_canvas(points))
}

/// キャンバス座標の点 `points` を、`target` の要素の親の座標系に写す (`local_to_canvas` の逆)。
///
/// `guesses` は反復の初期値 (親の座標系、`points` と同じ長さ)。ドラッグの前の位置を渡すと早く収束する。
/// 親がワープデフォーマのときは、格子が折り返していると解が複数あり、初期値に近い解を返す。
pub fn canvas_to_local(
    model: &Model,
    target: &KeyformTarget,
    values: &ParameterValues,
    points: &[Vec2],
    guesses: &[Vec2],
) -> Result<Vec<Vec2>, SpaceError> {
    if points.len() != guesses.len() {
        return Err(SpaceError::LengthMismatch {
            points: points.len(),
            guesses: guesses.len(),
        });
    }
    parent_world(model, target, values)?
        .from_canvas(points, guesses)
        .map_err(SpaceError::from)
}

impl From<WorldError> for SpaceError {
    fn from(error: WorldError) -> Self {
        match error {
            WorldError::Evaluate(reason) => Self::Evaluate(reason),
            WorldError::LengthMismatch { points, guesses } => {
                Self::LengthMismatch { points, guesses }
            }
            WorldError::Singular { index } => Self::Singular { index },
            WorldError::NotConverged { index, residual } => Self::NotConverged { index, residual },
        }
    }
}

/// 要素の親の座標系の写像を、パラメータの値 `values` の姿勢で作る。
fn parent_world(
    model: &Model,
    target: &KeyformTarget,
    values: &ParameterValues,
) -> Result<ParentWorld, SpaceError> {
    let missing = |kind, id: &dyn std::fmt::Display| SpaceError::NotFound {
        kind,
        id: id.to_string(),
    };
    let parent = match target {
        KeyformTarget::Mesh(id) => {
            let mesh = model.meshes.iter().find(|m| &m.id == id);
            mesh.ok_or_else(|| missing("mesh", id))?.parent.clone()
        }
        KeyformTarget::Deformer(id) => {
            let deformer = model.deformers.iter().find(|d| &d.id == id);
            deformer
                .ok_or_else(|| missing("deformer", id))?
                .parent
                .clone()
        }
        KeyformTarget::Part(id) => {
            return Err(SpaceError::NoSpace {
                kind: "part",
                id: id.to_string(),
            });
        }
        KeyformTarget::Glue(id) => {
            return Err(SpaceError::NoSpace {
                kind: "glue",
                id: id.to_string(),
            });
        }
    };
    let mut world = ParentWorld::new(model, parent.as_ref())?;
    world.set_pose(values.iter().map(|(id, &value)| (id.as_str(), value)));
    Ok(world)
}
