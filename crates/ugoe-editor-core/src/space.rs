//! 要素の親の座標系とキャンバス座標の変換 (`docs/editor.md#キーフォームの編集`)。
//!
//! ビューポートでの編集はキャンバス座標で行うが、キーフォームの頂点・制御点・原点は親の座標系
//! (親なしならキャンバス座標、親がワープなら UV 空間、回転デフォーマなら原点を (0,0) とした座標)
//! で表す。親のワールド変換は、親の祖先だけを残したモデルをランタイム (`ugoe-runtime`) で評価し、
//! ランタイムが公開するワールド変換 ([`WorldTransform`]) をそのまま使う。
//! 逆変換 (キャンバス座標 → 親の座標系) は、その写像と解析的なヤコビ行列でニュートン法を解く。

use std::collections::BTreeMap;

pub use ugoe_core::Vec2;
use ugoe_model::{DeformerId, Model, ParamId};
use ugoe_runtime::{CompiledModel, ModelInstance, WorldTransform};

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

/// ニュートン法の反復の上限。
const MAX_ITERATIONS: usize = 24;

/// `target` の要素の親の座標系の点 `points` を、キャンバス座標に写す。
pub fn local_to_canvas(
    model: &Model,
    target: &KeyformTarget,
    values: &ParameterValues,
    points: &[Vec2],
) -> Result<Vec<Vec2>, SpaceError> {
    let space = ParentSpace::new(model, target, values)?;
    let world = space.world();
    Ok(points.iter().map(|&p| world.point(p)).collect())
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
    let space = ParentSpace::new(model, target, values)?;
    let world = space.world();
    if world.is_identity() {
        return Ok(points.to_vec());
    }
    let mut x = guesses.to_vec();
    for _ in 0..MAX_ITERATIONS {
        let mut converged = true;
        for (i, p) in x.iter_mut().enumerate() {
            let r = points[i] - world.point(*p);
            if r.length() <= 1e-2 * tolerance(points[i]) {
                continue;
            }
            let jacobian = world.jacobian(*p);
            let det = jacobian.determinant();
            if !det.is_finite() || det.abs() < 1e-12 {
                return Err(SpaceError::Singular { index: i });
            }
            let step = jacobian.inverse() * r;
            *p += step;
            // `f32` の精度の限界まで来たら、それ以上は動かさない。
            if step.length() > 1e-6 * p.abs().max_element().max(1.0) {
                converged = false;
            }
        }
        if converged {
            return Ok(x);
        }
    }
    // 最後の更新の後の残差を確かめる。
    for (i, (&p, &target)) in x.iter().zip(points).enumerate() {
        let residual = (target - world.point(p)).length();
        if residual > tolerance(target) {
            return Err(SpaceError::NotConverged { index: i, residual });
        }
    }
    Ok(x)
}

/// 解けたとみなす残差 (キャンバスのピクセル)。座標が大きいほど `f32` の誤差が大きいので広げる。
/// 反復はこの 1/100 まで、または `f32` の精度の限界まで続ける。
fn tolerance(target: Vec2) -> f32 {
    1e-2 + 1e-5 * target.abs().max_element()
}

/// 要素の親の座標系。親の祖先だけを残したモデルを評価したインスタンスを持つ。
struct ParentSpace {
    /// インスタンスと、その中での親の添字。親がなければ `None` (恒等変換)。
    parent: Option<(ModelInstance, usize)>,
}

impl ParentSpace {
    fn new(
        model: &Model,
        target: &KeyformTarget,
        values: &ParameterValues,
    ) -> Result<Self, SpaceError> {
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
                    id: id.clone(),
                });
            }
        };
        let Some(parent) = parent else {
            return Ok(Self { parent: None });
        };
        let compiled = CompiledModel::compile(ancestors_only(model, &parent))
            .map_err(|e| SpaceError::Evaluate(e.to_string()))?;
        let mut instance = ModelInstance::new(compiled);
        for (id, &value) in values {
            if let Some(index) = instance.parameters().index_of(id.as_str()) {
                instance.parameters_mut().set(index, value);
            }
        }
        instance.update();
        let index = instance
            .compiled()
            .deformer_index(parent.as_str())
            .ok_or_else(|| SpaceError::Evaluate(format!("deformer `{parent}` not found")))?;
        Ok(Self {
            parent: Some((instance, index)),
        })
    }

    /// 親の座標系 → キャンバス座標の写像。
    fn world(&self) -> WorldTransform<'_> {
        match &self.parent {
            Some((instance, index)) => instance.deformer_world(*index),
            None => WorldTransform::IDENTITY,
        }
    }
}

/// `parent` とその祖先のデフォーマだけを残したモデル (メッシュ・グルーなどは外す)。
/// 評価に関係しない部分の不正で失敗しないように、参照も外す。
fn ancestors_only(model: &Model, parent: &DeformerId) -> Model {
    let mut chain = Vec::new();
    let mut next = Some(parent);
    while let Some(id) = next {
        if chain.contains(id) {
            break; // 循環 (不正なモデル)。評価で失敗する。
        }
        chain.push(id.clone());
        next = model
            .deformers
            .iter()
            .find(|d| &d.id == id)
            .and_then(|d| d.parent.as_ref());
    }
    let mut out = model.clone();
    out.deformers.retain(|d| chain.contains(&d.id));
    for d in &mut out.deformers {
        d.mirror = None;
    }
    out.meshes.clear();
    out.glues.clear();
    out.hit_areas.clear();
    out.pose = Default::default();
    out.effects = Default::default();
    out
}
