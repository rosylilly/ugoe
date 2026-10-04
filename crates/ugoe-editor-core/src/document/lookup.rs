//! ID で要素を探す・ID の重なりと子の有無を確かめる補助。

use std::fmt::Display;

use ugoe_model::{DeformerId, MeshId, Model, ParamId, PartId};

use super::EditError;

/// ID で要素を探す。
pub(super) fn find_mut<'a, T, K: PartialEq + Display>(
    items: &'a mut [T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<&'a mut T, EditError> {
    items
        .iter_mut()
        .find(|item| id_of(item) == id)
        .ok_or_else(|| EditError::NotFound {
            kind,
            id: id.to_string(),
        })
}

/// ID が未使用であることを確かめる。
pub(super) fn ensure_new<T, K: PartialEq + Display>(
    items: &[T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<(), EditError> {
    if items.iter().any(|item| id_of(item) == id) {
        return Err(EditError::DuplicateId {
            kind,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// 子の一覧が空でなければ [`EditError::HasChildren`] にする。
pub(super) fn ensure_no_children(
    kind: &'static str,
    id: &impl Display,
    children: Vec<String>,
) -> Result<(), EditError> {
    if children.is_empty() {
        return Ok(());
    }
    Err(EditError::HasChildren {
        kind,
        id: id.to_string(),
        children,
    })
}

pub(super) fn parameter<'a>(
    model: &'a mut Model,
    id: &ParamId,
) -> Result<&'a mut ugoe_model::Parameter, EditError> {
    find_mut(&mut model.parameters, "parameter", id, |p| &p.id)
}

pub(super) fn part<'a>(
    model: &'a mut Model,
    id: &PartId,
) -> Result<&'a mut ugoe_model::Part, EditError> {
    find_mut(&mut model.parts, "part", id, |p| &p.id)
}

pub(super) fn deformer<'a>(
    model: &'a mut Model,
    id: &DeformerId,
) -> Result<&'a mut ugoe_model::Deformer, EditError> {
    find_mut(&mut model.deformers, "deformer", id, |d| &d.id)
}

pub(super) fn mesh<'a>(
    model: &'a mut Model,
    id: &MeshId,
) -> Result<&'a mut ugoe_model::Mesh, EditError> {
    find_mut(&mut model.meshes, "mesh", id, |m| &m.id)
}
