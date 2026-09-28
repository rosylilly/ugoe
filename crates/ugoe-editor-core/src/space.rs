//! 要素の親の座標系とキャンバス座標の変換 (`docs/editor.md#キーフォームの編集`)。
//!
//! ビューポートでの編集はキャンバス座標で行うが、キーフォームの頂点・制御点・原点は親の座標系
//! (親なしならキャンバス座標、親がワープなら UV 空間、回転デフォーマなら原点を (0,0) とした座標)
//! で表す。親のワールド変換はランタイムの評価 (`ugoe-runtime`) と同じものを使うため、
//! 親の祖先だけを残したモデルに点を頂点とする仮のメッシュを置いて評価する。
//! 逆変換 (キャンバス座標 → 親の座標系) はニュートン法で求める。

use std::collections::BTreeMap;

pub use ugoe_core::Vec2;
use ugoe_core::{Mat2, Rgb};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    BlendMode, DeformerId, KeyformGrid, MaskMode, Mesh, Model, ParamId, Part, TextureRef,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

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
    ParentSpace::new(model, target, values)?.map(points)
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
    if space.probe.is_none() {
        return Ok(points.to_vec());
    }
    let mut x = guesses.to_vec();
    for _ in 0..MAX_ITERATIONS {
        // 各点について、点とその x・y 方向に少しずらした点を評価し、差分でヤコビアンを作る。
        let steps: Vec<f32> = x
            .iter()
            .map(|p| 1e-2 * p.abs().max_element().max(1.0))
            .collect();
        let probe: Vec<Vec2> = x
            .iter()
            .zip(&steps)
            .flat_map(|(&p, &h)| [p, p + Vec2::new(h, 0.0), p + Vec2::new(0.0, h)])
            .collect();
        let mapped = space.map(&probe)?;
        let mut converged = true;
        for (i, (p, &h)) in x.iter_mut().zip(&steps).enumerate() {
            let [f0, fx, fy] = [mapped[3 * i], mapped[3 * i + 1], mapped[3 * i + 2]];
            let r = points[i] - f0;
            if r.length() <= 1e-2 * tolerance(points[i]) {
                continue;
            }
            let jacobian = Mat2::from_cols((fx - f0) / h, (fy - f0) / h);
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
    let mapped = space.map(&x)?;
    for (i, (&f, &target)) in mapped.iter().zip(points).enumerate() {
        let residual = (target - f).length();
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

/// 親の祖先だけを残したモデルと、評価に使うパラメータの値。
struct ParentSpace<'a> {
    /// 親がなければ `None` (恒等変換)。
    probe: Option<(Model, DeformerId)>,
    values: &'a ParameterValues,
}

impl<'a> ParentSpace<'a> {
    fn new(
        model: &Model,
        target: &KeyformTarget,
        values: &'a ParameterValues,
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
        let probe = parent.map(|parent| (ancestors_only(model, &parent), parent));
        Ok(Self { probe, values })
    }

    /// 親の座標系の点をキャンバス座標に写す。
    fn map(&self, points: &[Vec2]) -> Result<Vec<Vec2>, SpaceError> {
        let Some((base, parent)) = &self.probe else {
            return Ok(points.to_vec());
        };
        if points.is_empty() {
            return Ok(Vec::new());
        }
        let mut model = base.clone();
        let part = model.parts[0].id.clone();
        model.meshes.push(Mesh {
            id: "__probe".into(),
            name: String::new(),
            part,
            parent: Some(parent.clone()),
            texture: 0,
            uvs: vec![Vec2::ZERO; points.len()],
            indices: Vec::new(),
            blend: BlendMode::Normal,
            culling: false,
            masks: Vec::new(),
            mask_mode: MaskMode::default(),
            mask_only: false,
            keyforms: KeyformGrid::constant(MeshKeyform {
                positions: points.to_vec(),
                opacity: 1.0,
                draw_order: 0.0,
                multiply_color: Rgb::WHITE,
                screen_color: Rgb::BLACK,
                mask_strength: 1.0,
            }),
            mirror: None,
            mirror_axis: None,
            depth: None,
        });
        let compiled =
            CompiledModel::compile(model).map_err(|e| SpaceError::Evaluate(e.to_string()))?;
        let mut instance = ModelInstance::new(compiled);
        for (id, &value) in self.values {
            if let Some(index) = instance.parameters().index_of(id.as_str()) {
                instance.parameters_mut().set(index, value);
            }
        }
        instance.update();
        Ok(instance.drawables()[0].positions.clone())
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
    if out.parts.is_empty() {
        out.parts.push(Part {
            id: "__probe".into(),
            name: String::new(),
            parent: None,
            keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
        });
    }
    if out.textures.is_empty() {
        out.textures.push(TextureRef {
            name: "__probe".into(),
            width: 1,
            height: 1,
        });
    }
    out
}
