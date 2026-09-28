//! 要素の ID から格子を引いて推定する。エディタのコマンド (T0611) から使う。

use ugoe_model::{DeformerId, DeformerKind, MeshId, Model, PartId};

use crate::additive::infer_additive_with;
use crate::{InferError, InferReport, InferScope};

/// キーフォームの格子を持つ要素。どの要素も格子を 1 つだけ持つ。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyformTarget {
    Part(PartId),
    Mesh(MeshId),
    /// ワープデフォーマまたは回転デフォーマ。
    Deformer(DeformerId),
    /// グルー (`Glue::id`)。
    Glue(String),
}

/// `target` の要素の格子の `derived` のセルを加算合成で作り直す ([`crate::infer_additive`])。
/// エラーのときはモデルを変えない。
pub fn infer_additive_target(
    model: &mut Model,
    target: &KeyformTarget,
    scope: InferScope,
) -> Result<InferReport, InferError> {
    let Model {
        parameters,
        parts,
        deformers,
        meshes,
        glues,
        ..
    } = model;
    let missing = || InferError::UnknownTarget(target.clone());
    match target {
        KeyformTarget::Part(id) => {
            let part = parts.iter_mut().find(|p| &p.id == id).ok_or_else(missing)?;
            infer_additive_with(parameters, &mut part.keyforms, scope)
        }
        KeyformTarget::Mesh(id) => {
            let mesh = meshes
                .iter_mut()
                .find(|m| &m.id == id)
                .ok_or_else(missing)?;
            infer_additive_with(parameters, &mut mesh.keyforms, scope)
        }
        KeyformTarget::Deformer(id) => {
            let deformer = deformers
                .iter_mut()
                .find(|d| &d.id == id)
                .ok_or_else(missing)?;
            // 未知の種類 (前方互換で読み飛ばしたもの) は加算合成できない。
            if let DeformerKind::Warp(warp) = &mut deformer.kind {
                return infer_additive_with(parameters, &mut warp.keyforms, scope);
            }
            if let DeformerKind::Rotation(rotation) = &mut deformer.kind {
                return infer_additive_with(parameters, &mut rotation.keyforms, scope);
            }
            Err(InferError::UnsupportedTarget(target.clone()))
        }
        KeyformTarget::Glue(id) => {
            let glue = glues.iter_mut().find(|g| &g.id == id).ok_or_else(missing)?;
            infer_additive_with(parameters, &mut glue.keyforms, scope)
        }
    }
}
