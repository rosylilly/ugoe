//! 要素の親の座標系とキャンバス座標の写像 (`docs/spec/keyform-inference.md#親が異なる対`)。
//!
//! 親のワールド変換は、親の祖先だけを残したモデルをランタイム (`ugoe-runtime`) で評価し、
//! ランタイムが公開するワールド変換 ([`WorldTransform`]) をそのまま使う。
//! 逆写像 (キャンバス座標 → 親の座標系) は、その写像と解析的なヤコビ行列でニュートン法を解く。
//! 親が異なる左右の対のミラーと、エディタの座標変換 (`ugoe_editor_core::space`) が使う。

use ugoe_core::Vec2;
use ugoe_model::{DeformerId, Model};
use ugoe_runtime::{CompiledModel, ModelInstance, WorldTransform};

/// 写像の失敗。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WorldError {
    /// 親の祖先のデフォーマを評価できない (不正なモデルなど)。
    #[error("cannot evaluate the parent deformers: {0}")]
    Evaluate(String),
    /// [`ParentWorld::from_canvas`] の点と初期値の数が合わない。
    #[error("{points} points but {guesses} initial guesses")]
    LengthMismatch { points: usize, guesses: usize },
    /// 親の変換が点 `index` の近くで潰れていて、逆に解けない。
    #[error("the parent transform is degenerate near point {index}")]
    Singular { index: usize },
    /// 点 `index` が反復の上限までに解けなかった (`residual` は最後の残差、キャンバスのピクセル)。
    #[error("point {index} did not converge (residual {residual} px)")]
    NotConverged { index: usize, residual: f32 },
}

/// ニュートン法の反復の上限。
const MAX_ITERATIONS: usize = 24;

/// 要素の親の座標系 → キャンバス座標の写像。親の祖先だけを残したモデルを評価したインスタンスを持つ。
///
/// 作った直後はすべてのパラメータが既定値の姿勢で、[`ParentWorld::set_pose`] で変えられる。
#[derive(Debug)]
pub struct ParentWorld {
    /// インスタンスと、その中での親の添字。親がなければ `None` (恒等変換)。
    parent: Option<(ModelInstance, usize)>,
}

impl ParentWorld {
    /// 親 `parent` (`None` ならキャンバス直下) の写像を、既定値の姿勢で作る。
    pub fn new(model: &Model, parent: Option<&DeformerId>) -> Result<Self, WorldError> {
        let Some(parent) = parent else {
            return Ok(Self { parent: None });
        };
        let compiled = CompiledModel::compile(ancestors_only(model, parent))
            .map_err(|e| WorldError::Evaluate(e.to_string()))?;
        let mut instance = ModelInstance::new(compiled);
        instance.update();
        let index = instance
            .compiled()
            .deformer_index(parent.as_str())
            .ok_or_else(|| WorldError::Evaluate(format!("deformer `{parent}` not found")))?;
        Ok(Self {
            parent: Some((instance, index)),
        })
    }

    /// 姿勢を、`values` のパラメータ (ID と値) だけをその値にし、残りを既定値にしたものにする。
    /// モデルにない ID は無視する。
    pub fn set_pose<'a>(&mut self, values: impl IntoIterator<Item = (&'a str, f32)>) {
        let Some((instance, _)) = &mut self.parent else {
            return;
        };
        instance.parameters_mut().reset();
        for (id, value) in values {
            if let Some(index) = instance.parameters().index_of(id) {
                instance.parameters_mut().set(index, value);
            }
        }
        instance.update();
    }

    /// 今の姿勢での、親の座標系 → キャンバス座標の写像。
    pub fn world(&self) -> WorldTransform<'_> {
        match &self.parent {
            Some((instance, index)) => instance.deformer_world(*index),
            None => WorldTransform::IDENTITY,
        }
    }

    /// 親の座標系の点をキャンバス座標に写す。
    pub fn to_canvas(&self, points: &[Vec2]) -> Vec<Vec2> {
        let world = self.world();
        points.iter().map(|&p| world.point(p)).collect()
    }

    /// キャンバス座標の点 `points` を親の座標系に写す ([`ParentWorld::to_canvas`] の逆)。
    ///
    /// `guesses` は反復の初期値 (親の座標系、`points` と同じ長さ)。前の位置を渡すと早く収束する。
    /// 親がワープデフォーマのときは、格子が折り返していると解が複数あり、初期値に近い解を返す。
    pub fn from_canvas(&self, points: &[Vec2], guesses: &[Vec2]) -> Result<Vec<Vec2>, WorldError> {
        if points.len() != guesses.len() {
            return Err(WorldError::LengthMismatch {
                points: points.len(),
                guesses: guesses.len(),
            });
        }
        let world = self.world();
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
                    return Err(WorldError::Singular { index: i });
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
                return Err(WorldError::NotConverged { index: i, residual });
            }
        }
        Ok(x)
    }
}

/// 解けたとみなす残差 (キャンバスのピクセル)。座標が大きいほど `f32` の誤差が大きいので広げる。
/// 反復はこの 1/100 まで、または `f32` の精度の限界まで続ける。
fn tolerance(target: Vec2) -> f32 {
    1e-2 + 1e-5 * target.abs().max_element()
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
    // スロットは消したデフォーマを取り付け先にしていることがある (着せ替えの本体・モジュールの作業用のモデル)。
    // アバタープロファイルは、消したスロットと当たり判定を名前で参照する。
    out.slots.clear();
    out.avatar = None;
    out.hit_areas.clear();
    out.pose = Default::default();
    out.effects = Default::default();
    out
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures::dress_up_base;

    use super::*;

    /// 親の祖先でないデフォーマを取り付け先にするスロットがあっても、写像を作れる。
    #[test]
    fn slots_on_other_deformers_do_not_break_the_world() {
        let mut model = dress_up_base();
        assert!(model.slots.iter().any(|s| s.anchor == Some("Torso".into())));
        model.avatar = Some(ugoe_model::AvatarProfile {
            slots: model.slots.iter().map(|s| s.id.clone()).collect(),
            ..Default::default()
        });
        let world = ParentWorld::new(&model, Some(&"Head".into())).unwrap();
        let p = Vec2::new(10.0, -20.0);
        let back = world
            .from_canvas(&world.to_canvas(&[p]), &[Vec2::ZERO])
            .unwrap();
        assert!((back[0] - p).length() < 1e-3);
    }
}
