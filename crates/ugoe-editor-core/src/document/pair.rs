//! 左右の対 (`mirror`) と対称軸の設定。

use std::fmt::Display;

use ugoe_model::{DeformerId, MeshId, Model};

use super::EditError;
use super::lookup::{deformer, find_mut, mesh};
use crate::command::KeyformTarget;

/// [`crate::Command::SetMirrorPair`]。対を双方向に張り直し、`target` の対称軸を設定する。
/// `mirror`・`mirror_axis` が変わった要素を返す。
pub(super) fn set_mirror_pair(
    model: &mut Model,
    target: &KeyformTarget,
    mirror: Option<String>,
    axis: Option<f32>,
) -> Result<Vec<KeyformTarget>, EditError> {
    match target {
        KeyformTarget::Mesh(id) => {
            let mut changed = pair(
                &mut model.meshes,
                "mesh",
                id,
                mirror.map(MeshId::new),
                |m| &m.id,
                |m| &mut m.mirror,
            )?;
            let item = mesh(model, id)?;
            set_axis(&mut item.mirror_axis, axis, id, &mut changed);
            Ok(changed.into_iter().map(KeyformTarget::Mesh).collect())
        }
        KeyformTarget::Deformer(id) => {
            let mut changed = pair(
                &mut model.deformers,
                "deformer",
                id,
                mirror.map(DeformerId::new),
                |d| &d.id,
                |d| &mut d.mirror,
            )?;
            let item = deformer(model, id)?;
            set_axis(&mut item.mirror_axis, axis, id, &mut changed);
            Ok(changed.into_iter().map(KeyformTarget::Deformer).collect())
        }
        KeyformTarget::Part(_) | KeyformTarget::Glue(_) => Err(EditError::MirrorPairNotSupported {
            target: target.to_string(),
        }),
    }
}

/// 対称軸を `axis` にし、変わったら `id` を `changed` に積む (積んでいなければ)。
/// 対称軸が有限の値であることは、適用後の検証 (`validate`) が確かめる。
fn set_axis<K: PartialEq + Clone>(
    field: &mut Option<f32>,
    axis: Option<f32>,
    id: &K,
    changed: &mut Vec<K>,
) {
    if *field != axis {
        *field = axis;
        if !changed.contains(id) {
            changed.push(id.clone());
        }
    }
}

/// `items` の中で `id` と `mirror` を双方向の対にする (`None` なら対を外す。`mirror` が `id` 自身なら
/// `id` だけを自分自身と対にする)。どちらかと対になっていた要素の対は外す。
/// `mirror` が変わった要素の ID を返す。メッシュ・デフォーマ・パラメータの対で共通。
pub(super) fn pair<T, K: PartialEq + Clone + Display>(
    items: &mut [T],
    kind: &'static str,
    id: &K,
    mirror: Option<K>,
    id_of: impl Fn(&T) -> &K,
    mirror_of: impl Fn(&mut T) -> &mut Option<K>,
) -> Result<Vec<K>, EditError> {
    find_mut(items, kind, id, &id_of)?;
    if let Some(mirror) = &mirror {
        find_mut(items, kind, mirror, &id_of)?;
    }
    let partner = mirror.clone().filter(|m| m != id);
    let mut changed = Vec::new();
    for item in items.iter_mut() {
        let item_id = id_of(item).clone();
        let item_mirror = mirror_of(item);
        let before = item_mirror.clone();
        if &item_id == id {
            *item_mirror = mirror.clone();
        } else if partner.as_ref() == Some(&item_id) {
            *item_mirror = Some(id.clone());
        } else if item_mirror
            .as_ref()
            .is_some_and(|m| m == id || partner.as_ref() == Some(m))
        {
            // 張り直す 2 つのどちらかと対になっていた要素。
            *item_mirror = None;
        }
        if *item_mirror != before {
            changed.push(item_id);
        }
    }
    Ok(changed)
}
