//! オーサリング用 [`Model`] を評価しやすい形へ変換する (ID → インデックス解決、トポロジカル順序付け)。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use ugoe_model::{DeformerKind, KeyformGrid, MaskMode, Model};

use crate::blend_shape::CompiledBlendShape;
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
    pub(crate) parts: Vec<CompiledPart>,
    /// パーツのトポロジカル順 (親が先)。
    pub(crate) part_order: Vec<u32>,
    pub(crate) deformers: Vec<CompiledDeformer>,
    /// デフォーマのトポロジカル順 (親が先)。
    pub(crate) deformer_order: Vec<u32>,
    pub(crate) meshes: Vec<CompiledMesh>,
    /// マスクとして参照されるメッシュの、マスクの依存順 (`mask_order`)。
    pub(crate) mask_order: Vec<u32>,
    pub(crate) glues: Vec<CompiledGlue>,
    /// 当たり判定ごとの判定に使うメッシュ (`Model::hit_areas` の並び)。
    pub(crate) hit_area_meshes: Vec<u32>,
    /// すべてのキーフォーム格子のうち最大の軸数。重みバッファの事前確保に使う。
    pub(crate) max_axes: usize,
}

#[derive(Debug)]
pub(crate) struct CompiledPart {
    pub(crate) parent: Option<u32>,
    pub(crate) bindings: Vec<ResolvedBinding>,
}

#[derive(Debug)]
pub(crate) struct CompiledDeformer {
    pub(crate) parent: Option<u32>,
    pub(crate) bindings: Vec<ResolvedBinding>,
    /// ブレンドシェイプ格子 (`KeyformGrid::blend_shapes` と同じ並び)。
    pub(crate) blend_shapes: Vec<CompiledBlendShape>,
}

#[derive(Debug)]
pub(crate) struct CompiledMesh {
    pub(crate) part: u32,
    pub(crate) parent: Option<u32>,
    pub(crate) bindings: Vec<ResolvedBinding>,
    /// ブレンドシェイプ格子 (`KeyformGrid::blend_shapes` と同じ並び)。
    pub(crate) blend_shapes: Vec<CompiledBlendShape>,
    pub(crate) masks: Vec<ResolvedMaskRef>,
    /// マスクの依存の深さ (`mask_depth`)。
    pub(crate) mask_depth: u32,
}

/// マスクの参照 (`ugoe_model::MaskRef`) の ID をメッシュのインデックスに解決したもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMaskRef {
    /// マスクとして使うメッシュ (`Model::meshes` のインデックス)。
    pub mesh: u32,
    /// `true` なら反転マスク。
    pub invert: bool,
}

#[derive(Debug)]
pub(crate) struct CompiledGlue {
    pub(crate) mesh_a: u32,
    pub(crate) mesh_b: u32,
    pub(crate) bindings: Vec<ResolvedBinding>,
}

impl CompiledModel {
    pub fn compile(model: Model) -> Result<Arc<Self>, CompileError> {
        let errors = ugoe_model::validate::validate(&model);
        if !errors.is_empty() {
            return Err(CompileError::Invalid(errors));
        }
        let params = ParameterValues::from_model(&model);
        let part_index = index_map(model.parts.iter().map(|p| p.id.as_str()));
        let deformer_index = index_map(model.deformers.iter().map(|d| d.id.as_str()));
        let mesh_index = index_map(model.meshes.iter().map(|m| m.id.as_str()));
        // validate 済みなので参照先は必ず存在する。念のため見つからなければ Invalid 相当で失敗させる。
        let lookup = |map: &HashMap<&str, u32>, id: &str| {
            map.get(id)
                .copied()
                .ok_or(CompileError::Unsupported("dangling reference"))
        };

        let parts = model
            .parts
            .iter()
            .map(|p| {
                Ok(CompiledPart {
                    parent: p
                        .parent
                        .as_ref()
                        .map(|id| lookup(&part_index, id.as_str()))
                        .transpose()?,
                    bindings: resolve(&params, &p.keyforms)?,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;

        let deformers = model
            .deformers
            .iter()
            .map(|d| {
                let (bindings, blend_shapes) = match &d.kind {
                    DeformerKind::Warp(warp) => {
                        if warp.rows == 0 || warp.cols == 0 {
                            return Err(CompileError::Unsupported(
                                "warp deformer with zero rows or cols",
                            ));
                        }
                        (
                            resolve(&params, &warp.keyforms)?,
                            resolve_blend_shapes(&params, &warp.keyforms)?,
                        )
                    }
                    DeformerKind::Rotation(rotation) => (
                        resolve(&params, &rotation.keyforms)?,
                        resolve_blend_shapes(&params, &rotation.keyforms)?,
                    ),
                    // 未知の種類はキーフォームを持たない恒等変換 (`docs/spec/file-formats.md`)。
                    DeformerKind::Unknown => (Vec::new(), Vec::new()),
                };
                Ok(CompiledDeformer {
                    parent: d
                        .parent
                        .as_ref()
                        .map(|id| lookup(&deformer_index, id.as_str()))
                        .transpose()?,
                    bindings,
                    blend_shapes,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;

        let mut meshes = model
            .meshes
            .iter()
            .map(|m| {
                Ok(CompiledMesh {
                    part: lookup(&part_index, m.part.as_str())?,
                    parent: m
                        .parent
                        .as_ref()
                        .map(|id| lookup(&deformer_index, id.as_str()))
                        .transpose()?,
                    bindings: resolve(&params, &m.keyforms)?,
                    blend_shapes: resolve_blend_shapes(&params, &m.keyforms)?,
                    masks: m
                        .masks
                        .iter()
                        .map(|r| {
                            Ok(ResolvedMaskRef {
                                mesh: lookup(&mesh_index, r.mesh.as_str())?,
                                invert: r.invert,
                            })
                        })
                        .collect::<Result<Vec<_>, CompileError>>()?,
                    mask_depth: 0,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        let depths = mask_depths(&meshes);
        for (mesh, depth) in meshes.iter_mut().zip(depths) {
            mesh.mask_depth = depth;
        }
        let mask_order = mask_order(&meshes);

        let glues = model
            .glues
            .iter()
            .map(|g| {
                Ok(CompiledGlue {
                    mesh_a: lookup(&mesh_index, g.mesh_a.as_str())?,
                    mesh_b: lookup(&mesh_index, g.mesh_b.as_str())?,
                    bindings: resolve(&params, &g.keyforms)?,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;

        let hit_area_meshes = model
            .hit_areas
            .iter()
            .map(|h| lookup(&mesh_index, h.mesh.as_str()))
            .collect::<Result<Vec<_>, CompileError>>()?;

        let part_order = topological_order(parts.iter().map(|p| p.parent));
        let deformer_order = topological_order(deformers.iter().map(|d| d.parent));
        let shape_axes = |shapes: &[CompiledBlendShape]| {
            shapes.iter().map(|s| s.bindings.len()).max().unwrap_or(0)
        };
        let max_axes = parts
            .iter()
            .map(|p| p.bindings.len())
            .chain(deformers.iter().map(|d| d.bindings.len()))
            .chain(deformers.iter().map(|d| shape_axes(&d.blend_shapes)))
            .chain(meshes.iter().map(|m| m.bindings.len()))
            .chain(meshes.iter().map(|m| shape_axes(&m.blend_shapes)))
            .chain(glues.iter().map(|g| g.bindings.len()))
            .max()
            .unwrap_or(0);

        Ok(Arc::new(Self {
            model,
            parts,
            part_order,
            deformers,
            deformer_order,
            meshes,
            mask_order,
            glues,
            hit_area_meshes,
            max_axes,
        }))
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// ID が `id` のデフォーマの添字 (`Model::deformers` のインデックス)。
    pub fn deformer_index(&self, id: &str) -> Option<usize> {
        self.model
            .deformers
            .iter()
            .position(|d| d.id.as_str() == id)
    }

    /// ID が `id` のパーツの添字 (`Model::parts` のインデックス)。
    pub fn part_index(&self, id: &str) -> Option<usize> {
        self.model.parts.iter().position(|p| p.id.as_str() == id)
    }

    /// ID が `id` のメッシュの添字 (`Model::meshes` のインデックス、`drawables` と同じ並び)。
    pub fn mesh_index(&self, id: &str) -> Option<usize> {
        self.model.meshes.iter().position(|m| m.id.as_str() == id)
    }

    /// デフォーマの評価順 (親が先のトポロジカル順、`Model::deformers` のインデックス)。
    pub fn deformer_order(&self) -> &[u32] {
        &self.deformer_order
    }

    /// メッシュ `mesh` (`Model::meshes` のインデックス) のマスクの参照。並びは `Mesh::masks` と同じ。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mask_refs(&self, mesh: usize) -> &[ResolvedMaskRef] {
        &self.meshes[mesh].masks
    }

    /// メッシュ `mesh` の通常マスクの結合方法 (`Mesh::mask_mode`)。
    /// 未知の結合方法には和集合のフォールバックを適用して返す (`MaskMode::effective`)。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mask_mode(&self, mesh: usize) -> MaskMode {
        self.model.meshes[mesh].mask_mode.effective()
    }

    /// メッシュ `mesh` のマスクの依存の深さ。マスクを持たなければ 0、
    /// 持てば 1 + (マスクとして使うメッシュの深さの最大値)。
    ///
    /// 深さの小さいメッシュから順に評価すれば、マスクは必ずマスクされる側より先に評価される
    /// (入れ子の継承、`docs/spec/rendering.md#マスク`)。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mask_depth(&self, mesh: usize) -> u32 {
        self.meshes[mesh].mask_depth
    }

    /// マスクとして 1 回以上参照されるメッシュ (`Model::meshes` のインデックス) を、
    /// マスクの依存の深さの昇順 (同じ深さは宣言順) に並べたもの。
    ///
    /// 各メッシュのマスクは、そのメッシュより前に現れる。レンダラはこの順にマスクを評価すれば、
    /// 入れ子のマスクの切り抜き後の形を使える。
    pub fn mask_order(&self) -> &[u32] {
        &self.mask_order
    }

    /// ID が `id` の当たり判定の添字 (`Model::hit_areas` のインデックス)。
    pub fn hit_area_index(&self, id: &str) -> Option<usize> {
        self.model.hit_areas.iter().position(|h| h.id == id)
    }

    /// 当たり判定 `hit_area` の判定に使うメッシュ (`Model::meshes` のインデックス)。
    ///
    /// # Panics
    ///
    /// `hit_area` が範囲外のとき。
    pub fn hit_area_mesh(&self, hit_area: usize) -> usize {
        self.hit_area_meshes[hit_area] as usize
    }
}

fn index_map<K: Eq + Hash>(ids: impl Iterator<Item = K>) -> HashMap<K, u32> {
    ids.enumerate().map(|(i, id)| (id, i as u32)).collect()
}

/// 親が先になる順序。循環がないこと (validate で検査済み) を前提にする。
///
/// 配列順に要素を見て、未訪問の祖先を根の側から並べる。結果は入力だけで決まる (決定的)。
fn topological_order(parents: impl Iterator<Item = Option<u32>>) -> Vec<u32> {
    let parents: Vec<Option<u32>> = parents.collect();
    let mut visited = vec![false; parents.len()];
    let mut order = Vec::with_capacity(parents.len());
    let mut chain = Vec::new();
    for start in 0..parents.len() {
        let mut cursor = Some(start as u32);
        while let Some(i) = cursor {
            // 循環があっても止まるように、要素数を超えたら打ち切る。
            if visited[i as usize] || chain.len() > parents.len() {
                break;
            }
            chain.push(i);
            cursor = parents[i as usize];
        }
        for &i in chain.iter().rev() {
            if !visited[i as usize] {
                visited[i as usize] = true;
                order.push(i);
            }
        }
        chain.clear();
    }
    order
}

/// メッシュごとのマスクの依存の深さ。循環がないこと (validate で検査済み) を前提にする。
fn mask_depths(meshes: &[CompiledMesh]) -> Vec<u32> {
    let mut depth: Vec<Option<u32>> = vec![None; meshes.len()];
    let mut on_stack = vec![false; meshes.len()];
    let mut stack = Vec::new();
    for start in 0..meshes.len() {
        if depth[start].is_some() {
            continue;
        }
        stack.push(start);
        on_stack[start] = true;
        while let Some(&i) = stack.last() {
            // 深さが未確定のマスクがあれば先に求める。循環 (検査済みなので起きない) は深さ 0 とみなして止める。
            let pending = meshes[i]
                .masks
                .iter()
                .map(|r| r.mesh as usize)
                .find(|&m| depth[m].is_none() && !on_stack[m]);
            if let Some(m) = pending {
                stack.push(m);
                on_stack[m] = true;
                continue;
            }
            depth[i] = Some(
                meshes[i]
                    .masks
                    .iter()
                    .map(|r| depth[r.mesh as usize].map_or(0, |d| d + 1))
                    .max()
                    .unwrap_or(0),
            );
            on_stack[i] = false;
            stack.pop();
        }
    }
    depth.into_iter().map(|d| d.unwrap_or(0)).collect()
}

/// マスクとして参照されるメッシュを、深さの昇順 (同じ深さは宣言順) に並べる。
fn mask_order(meshes: &[CompiledMesh]) -> Vec<u32> {
    let mut used = vec![false; meshes.len()];
    for r in meshes.iter().flat_map(|m| &m.masks) {
        used[r.mesh as usize] = true;
    }
    let mut order: Vec<u32> = (0..meshes.len() as u32)
        .filter(|&i| used[i as usize])
        .collect();
    order.sort_by_key(|&i| (meshes[i as usize].mask_depth, i));
    order
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

/// ブレンドシェイプ格子の軸を解決し、既定値での重みを求める。
fn resolve_blend_shapes<T>(
    params: &ParameterValues,
    grid: &KeyformGrid<T>,
) -> Result<Vec<CompiledBlendShape>, CompileError> {
    grid.blend_shapes
        .iter()
        .map(|shape| {
            Ok(CompiledBlendShape::new(
                resolve(params, shape)?,
                params.as_slice(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topological_order_puts_parents_first() {
        // 0 → 2 → 1 (1 が根)、3 は独立
        let order = topological_order([Some(2), None, Some(1), None].into_iter());
        assert_eq!(order, vec![1, 2, 0, 3]);
    }
}
