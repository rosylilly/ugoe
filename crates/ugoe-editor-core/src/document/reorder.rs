//! 要素の並べ替え。

use ugoe_model::{MeshId, Model};

use super::EditError;

/// `meshes` の中でメッシュ `id` を `before` の直前 (`None` なら末尾) へ移す。
pub(super) fn reorder_mesh(
    model: &mut Model,
    id: &MeshId,
    before: Option<&MeshId>,
) -> Result<(), EditError> {
    if before == Some(id) {
        return Err(EditError::SelfReference {
            kind: "mesh",
            id: id.to_string(),
            field: "before",
        });
    }
    let from = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    if let Some(before) = before
        && !model.meshes.iter().any(|m| &m.id == before)
    {
        return Err(EditError::NotFound {
            kind: "mesh",
            id: before.to_string(),
        });
    }
    let moved = model.meshes.remove(from);
    let to = match before {
        Some(before) => model
            .meshes
            .iter()
            .position(|m| &m.id == before)
            .unwrap_or(model.meshes.len()),
        None => model.meshes.len(),
    };
    model.meshes.insert(to, moved);
    Ok(())
}
