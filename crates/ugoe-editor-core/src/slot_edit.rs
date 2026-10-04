//! 着せ替えのスロット (`Model::slots`) の編集。規則は `docs/editor.md#スロットの編集`、
//! データの意味は `docs/spec/modules.md#スロット`。
//!
//! 参照先 (取り付け先のデフォーマ・パーツ・パラメータ・メッシュ・ポーズの要素) と描画順の範囲の値は、
//! `Document::apply` が適用の後に `validate` で検査する (新しいエラーが生じたら拒否)。
//! ここでは ID の形式と重複、衝突体の参照、ほかのスロットとの描画順の範囲の重なりを検査する。

use ugoe_model::module::{DrawOrderRange, Slot, SlotId};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{DeformerId, Model, PartId};

use crate::document::EditError;

fn invalid(id: &SlotId, reason: impl Into<String>) -> EditError {
    EditError::InvalidSlot {
        id: id.to_string(),
        reason: reason.into(),
    }
}

fn not_found(id: &SlotId) -> EditError {
    EditError::NotFound {
        kind: "slot",
        id: id.to_string(),
    }
}

/// [`crate::Command::AddSlot`]。スロットを末尾に足す。
pub(crate) fn add(
    model: &mut Model,
    physics: Option<&PhysicsSettings>,
    slot: Slot,
) -> Result<(), EditError> {
    check_id(model, &slot.id, None)?;
    check_colliders(physics, &slot)?;
    check_overlap(model, &slot, None)?;
    model.slots.push(slot);
    Ok(())
}

/// [`crate::Command::SetSlot`]。スロット `id` を置き換える。ID が変われば、
/// アバタープロファイルの `slots` の参照も付け替える。
pub(crate) fn set(
    model: &mut Model,
    physics: Option<&PhysicsSettings>,
    id: &SlotId,
    slot: Slot,
) -> Result<(), EditError> {
    let index = model
        .slots
        .iter()
        .position(|s| &s.id == id)
        .ok_or_else(|| not_found(id))?;
    check_id(model, &slot.id, Some(index))?;
    check_colliders(physics, &slot)?;
    check_overlap(model, &slot, Some(index))?;
    if &slot.id != id
        && let Some(avatar) = &mut model.avatar
    {
        for exposed in &mut avatar.slots {
            if exposed == id {
                *exposed = slot.id.clone();
            }
        }
    }
    model.slots[index] = slot;
    Ok(())
}

/// [`crate::Command::RemoveSlot`]。スロットを消す。アバタープロファイルの `slots` からは、
/// 適用の後に `cleanup::forget_removed` が外す。
pub(crate) fn remove(model: &mut Model, id: &SlotId) -> Result<(), EditError> {
    let index = model
        .slots
        .iter()
        .position(|s| &s.id == id)
        .ok_or_else(|| not_found(id))?;
    model.slots.remove(index);
    Ok(())
}

/// ID が `[A-Za-z0-9_]+` の形式で (`docs/spec/model.md#ID`)、ほかのスロットと重ならないこと。
/// `current` は置き換えるスロットの番号 (自分とは重なってよい)。
fn check_id(model: &Model, id: &SlotId, current: Option<usize>) -> Result<(), EditError> {
    let valid = !id.as_str().is_empty()
        && id
            .as_str()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err(invalid(id, "slot id must match [A-Za-z0-9_]+"));
    }
    let taken = model
        .slots
        .iter()
        .enumerate()
        .any(|(i, s)| Some(i) != current && &s.id == id);
    if taken {
        return Err(EditError::DuplicateId {
            kind: "slot",
            id: id.to_string(),
        });
    }
    Ok(())
}

/// 公開する衝突体が物理の設定にあること。物理の設定がないと適用後の検証 (`validate_physics`) が
/// 走らないので、ここで調べる。
fn check_colliders(physics: Option<&PhysicsSettings>, slot: &Slot) -> Result<(), EditError> {
    for collider in &slot.colliders {
        let exists = physics.is_some_and(|p| p.colliders.iter().any(|c| &c.id == collider));
        if !exists {
            return Err(EditError::NotFound {
                kind: "physics collider",
                id: collider.to_string(),
            });
        }
    }
    Ok(())
}

/// 描画順の範囲が重なるか。端が接するだけ (`a.max == b.min`) は重ならないとみなす。
/// 幅 0 の範囲は、ほかの範囲と 1 点でも共有すれば重なる。
pub fn ranges_overlap(a: DrawOrderRange, b: DrawOrderRange) -> bool {
    let lo = a.min.max(b.min);
    let hi = a.max.min(b.max);
    lo < hi || (lo == hi && (a.min == a.max || b.min == b.max))
}

/// ほかのスロットと描画順の範囲が重ならないこと。重なるとモジュールのメッシュが互いに入り組む。
/// 読み込んだ時点で重なっていた組 (置き換える前の範囲とも重なるもの) は、ほかの項目の編集を妨げないよう見逃す。
fn check_overlap(model: &Model, slot: &Slot, current: Option<usize>) -> Result<(), EditError> {
    let before = current.map(|i| model.slots[i].draw_order);
    let range = slot.draw_order;
    for (i, other) in model.slots.iter().enumerate() {
        if Some(i) == current || !ranges_overlap(range, other.draw_order) {
            continue;
        }
        if before.is_some_and(|b| ranges_overlap(b, other.draw_order)) {
            continue;
        }
        return Err(invalid(
            &slot.id,
            format!(
                "draw order {}..{} overlaps slot `{}` ({}..{})",
                range.min, range.max, other.id, other.draw_order.min, other.draw_order.max
            ),
        ));
    }
    Ok(())
}

/// デフォーマ `id` を取り付け先にしているスロット (`remove_deformer` の拒否に使う)。
pub(crate) fn anchored_to(model: &Model, id: &DeformerId) -> Vec<String> {
    model
        .slots
        .iter()
        .filter(|s| s.anchor.as_ref() == Some(id))
        .map(|s| format!("slot `{}`", s.id))
        .collect()
}

/// パーツ `id` をモジュールの親 (`Slot::part`) にしているスロット (`remove_part` の拒否に使う)。
pub(crate) fn parented_to(model: &Model, id: &PartId) -> Vec<String> {
    model
        .slots
        .iter()
        .filter(|s| s.part.as_ref() == Some(id))
        .map(|s| format!("slot `{}`", s.id))
        .collect()
}
