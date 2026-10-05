//! 要素の並べ替え。

use std::fmt::Display;

use ugoe_model::{MeshId, Model, ParameterGroupId};

use super::EditError;

/// `meshes` の中でメッシュ `id` を `before` の直前 (`None` なら末尾) へ移す。
pub(super) fn reorder_mesh(
    model: &mut Model,
    id: &MeshId,
    before: Option<&MeshId>,
) -> Result<(), EditError> {
    reorder(&mut model.meshes, "mesh", id, before, |m| &m.id)
}

/// `parameter_groups` の中でグループ `id` を `before` の直前 (`None` なら末尾) へ移す。
pub(super) fn reorder_parameter_group(
    model: &mut Model,
    id: &ParameterGroupId,
    before: Option<&ParameterGroupId>,
) -> Result<(), EditError> {
    reorder(
        &mut model.parameter_groups,
        "parameter group",
        id,
        before,
        |g| &g.id,
    )
}

/// `items` の中で要素 `id` を `before` の直前 (`None` なら末尾) へ移す。
fn reorder<T, K: PartialEq + Display>(
    items: &mut Vec<T>,
    kind: &'static str,
    id: &K,
    before: Option<&K>,
    id_of: impl Fn(&T) -> &K,
) -> Result<(), EditError> {
    if before == Some(id) {
        return Err(EditError::SelfReference {
            kind,
            id: id.to_string(),
            field: "before",
        });
    }
    let from = items
        .iter()
        .position(|item| id_of(item) == id)
        .ok_or_else(|| EditError::NotFound {
            kind,
            id: id.to_string(),
        })?;
    if let Some(before) = before
        && !items.iter().any(|item| id_of(item) == before)
    {
        return Err(EditError::NotFound {
            kind,
            id: before.to_string(),
        });
    }
    let moved = items.remove(from);
    let to = match before {
        Some(before) => items
            .iter()
            .position(|item| id_of(item) == before)
            .unwrap_or(items.len()),
        None => items.len(),
    };
    items.insert(to, moved);
    Ok(())
}
