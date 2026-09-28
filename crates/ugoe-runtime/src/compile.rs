//! オーサリング用 [`Model`] を評価しやすい形へ変換する (ID → インデックス解決、トポロジカル順序付け)。

use std::sync::Arc;

use ugoe_model::{KeyformGrid, Model};

use crate::keyform::ResolvedBinding;
use crate::{ParamIndex, ParameterValues};

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("model failed validation: {0:?}")]
    Invalid(Vec<ugoe_model::validate::ValidationError>),
    #[error("unknown parameter `{0}`")]
    UnknownParameter(String),
    #[error("unsupported feature: {0}")]
    Unsupported(&'static str),
}

/// 評価用に前処理したモデル。複数インスタンスで共有できる (不変)。
#[derive(Debug)]
pub struct CompiledModel {
    pub(crate) model: Model,
    pub(crate) mesh_bindings: Vec<Vec<ResolvedBinding>>,
}

impl CompiledModel {
    pub fn compile(model: Model) -> Result<Arc<Self>, CompileError> {
        let errors = ugoe_model::validate::validate(&model);
        if !errors.is_empty() {
            return Err(CompileError::Invalid(errors));
        }
        // TODO(T0104): デフォーマ・パーツ階層・グルーのコンパイル。
        if !model.deformers.is_empty() {
            return Err(CompileError::Unsupported("deformers"));
        }
        let params = ParameterValues::from_model(&model);
        let mesh_bindings = model
            .meshes
            .iter()
            .map(|m| resolve(&params, &m.keyforms))
            .collect::<Result<_, _>>()?;
        Ok(Arc::new(Self {
            model,
            mesh_bindings,
        }))
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}

fn resolve<T>(
    params: &ParameterValues,
    grid: &KeyformGrid<T>,
) -> Result<Vec<ResolvedBinding>, CompileError> {
    grid.bindings
        .iter()
        .map(|b| {
            let param: ParamIndex = params
                .index_of(b.parameter.as_str())
                .ok_or_else(|| CompileError::UnknownParameter(b.parameter.to_string()))?;
            Ok(ResolvedBinding {
                param,
                keys: b.keys.clone(),
            })
        })
        .collect()
}
