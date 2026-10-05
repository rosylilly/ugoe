//! 要素の削除の連鎖 (`docs/editor.md#削除の連鎖と拒否`)。

use ugoe_model::deformer::DeformerKind;
use ugoe_model::{DeformerId, MeshId, Model, ParamId, ParameterGroupId, PartId};

use super::EditError;
use super::lookup::ensure_no_children;
use crate::{grid, slot_edit};

/// パラメータのグループを削除し、属するパラメータの割り当てを外す。
pub(super) fn remove_parameter_group(
    model: &mut Model,
    id: &ParameterGroupId,
) -> Result<(), EditError> {
    let index = model
        .parameter_groups
        .iter()
        .position(|g| &g.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "parameter group",
            id: id.to_string(),
        })?;
    model.parameter_groups.remove(index);
    for parameter in &mut model.parameters {
        if parameter.group.as_ref() == Some(id) {
            parameter.group = None;
        }
    }
    Ok(())
}

/// パラメータを削除し、キーフォーム格子の軸を既定値の断面に畳む。
/// 他の要素からの参照は適用の後に [`crate::cleanup::forget_removed`] が外す。
pub(super) fn remove_parameter(model: &mut Model, id: &ParamId) -> Result<(), EditError> {
    let index = model
        .parameters
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "parameter",
            id: id.to_string(),
        })?;
    let removed = model.parameters.remove(index);
    let default = removed.default;

    for part in &mut model.parts {
        grid::collapse_axis(&mut part.keyforms, id, default);
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(warp) => grid::collapse_axis(&mut warp.keyforms, id, default),
            DeformerKind::Rotation(rotation) => {
                grid::collapse_axis(&mut rotation.keyforms, id, default)
            }
            // 未知の種類はキーフォームを持たない。
            DeformerKind::Unknown => {}
        }
    }
    for mesh in &mut model.meshes {
        grid::collapse_axis(&mut mesh.keyforms, id, default);
    }
    for glue in &mut model.glues {
        grid::collapse_axis(&mut glue.keyforms, id, default);
    }
    Ok(())
}

/// パーツを削除する。子パーツ・所属する要素があれば拒否する。
/// ポーズなどからの参照は適用の後に [`crate::cleanup::forget_removed`] が外す。
pub(super) fn remove_part(model: &mut Model, id: &PartId) -> Result<(), EditError> {
    let index = model
        .parts
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "part",
            id: id.to_string(),
        })?;
    let children = model
        .parts
        .iter()
        .filter(|p| p.parent.as_ref() == Some(id))
        .map(|p| format!("part `{}`", p.id))
        .chain(
            model
                .deformers
                .iter()
                .filter(|d| &d.part == id)
                .map(|d| format!("deformer `{}`", d.id)),
        )
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| &m.part == id)
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .chain(slot_edit::parented_to(model, id))
        .collect();
    ensure_no_children("part", id, children)?;
    model.parts.remove(index);
    Ok(())
}

/// デフォーマを削除する。子デフォーマ・子メッシュがあれば拒否する。
/// 他の要素からの参照は適用の後に [`crate::cleanup::forget_removed`] が外す。
pub(super) fn remove_deformer(model: &mut Model, id: &DeformerId) -> Result<(), EditError> {
    let index = model
        .deformers
        .iter()
        .position(|d| &d.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "deformer",
            id: id.to_string(),
        })?;
    let children = model
        .deformers
        .iter()
        .filter(|d| d.parent.as_ref() == Some(id))
        .map(|d| format!("deformer `{}`", d.id))
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| m.parent.as_ref() == Some(id))
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .chain(slot_edit::anchored_to(model, id))
        .collect();
    ensure_no_children("deformer", id, children)?;
    model.deformers.remove(index);
    Ok(())
}

/// メッシュを削除し、このメッシュを使うグルーと当たり判定も削除する。
/// マスク・対などからの参照は適用の後に [`crate::cleanup::forget_removed`] が外す。
pub(super) fn remove_mesh(model: &mut Model, id: &MeshId) -> Result<(), EditError> {
    let index = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    model.meshes.remove(index);
    model.glues.retain(|g| &g.mesh_a != id && &g.mesh_b != id);
    model.hit_areas.retain(|h| &h.mesh != id);
    Ok(())
}
