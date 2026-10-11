//! 削除の後始末。規則は `docs/editor.md#削除の連鎖と拒否`。
//!
//! 要素を消すコマンドは要素 (と、それに従属して消える要素) を消すだけにし、残った要素から消えた要素への
//! 参照は、[`crate::Document::apply_with_report`] が適用の後にここでまとめて外す。適用の前後のモデルを
//! 比べて、前にあって後にない ID だけを外すので、どのコマンド (と `batch` の組み合わせ) で消えても
//! 同じ後始末になり、読み込んだ時点で壊れていた参照は変えない。

use std::collections::HashSet;
use std::hash::Hash;

use ugoe_model::deformer::DeformerKind;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{KeyformGrid, KeyformOrigin, Model};

use crate::physics_edit;

/// `before` の要素のうち `after` にないものの ID。
fn removed<T, K: Eq + Hash + Clone>(
    before: &[T],
    after: &[T],
    id: impl Fn(&T) -> &K,
) -> HashSet<K> {
    let kept: HashSet<&K> = after.iter().map(&id).collect();
    before
        .iter()
        .map(&id)
        .filter(|k| !kept.contains(k))
        .cloned()
        .collect()
}

/// コマンドの適用で消えた要素への参照を `after` (とその物理 `physics`) から外す。
/// `before` と `before_physics` は適用前の状態。
pub(crate) fn forget_removed(
    before: &Model,
    before_physics: Option<&PhysicsSettings>,
    after: &mut Model,
    physics: &mut Option<PhysicsSettings>,
) {
    let parameters = removed(&before.parameters, &after.parameters, |p| &p.id);
    let parts = removed(&before.parts, &after.parts, |p| &p.id);
    let deformers = removed(&before.deformers, &after.deformers, |d| &d.id);
    let meshes = removed(&before.meshes, &after.meshes, |m| &m.id);
    let glues = removed(&before.glues, &after.glues, |g| &g.id);
    let hit_areas = removed(&before.hit_areas, &after.hit_areas, |h| &h.id);
    let slots = removed(&before.slots, &after.slots, |s| &s.id);

    // パラメータ: 対・エフェクトの対象・スロットの公開するパラメータ。
    // キーフォーム格子の軸は値を選んで畳む必要があるので `remove_parameter` が畳む。
    if !parameters.is_empty() {
        for param in &mut after.parameters {
            if param
                .mirror
                .as_ref()
                .is_some_and(|m| parameters.contains(m))
            {
                param.mirror = None;
            }
        }
        let effects = &mut after.effects;
        effects
            .eye_blink
            .parameters
            .retain(|p| !parameters.contains(p));
        effects
            .breath
            .parameters
            .retain(|b| !parameters.contains(&b.parameter));
        effects
            .lip_sync
            .parameters
            .retain(|p| !parameters.contains(p));
        for slot in &mut after.slots {
            slot.parameters.retain(|p| !parameters.contains(p));
        }
    }

    // パーツ: ポーズ (要素と `links`。消して空になったグループは消す)・スロットのリスト。
    if !parts.is_empty() {
        after.pose.groups.retain_mut(|group| {
            let had_entries = !group.entries.is_empty();
            group.entries.retain(|e| !parts.contains(&e.part));
            for entry in &mut group.entries {
                entry.links.retain(|l| !parts.contains(l));
            }
            !(had_entries && group.entries.is_empty())
        });
        for slot in &mut after.slots {
            slot.replaces.retain(|p| !parts.contains(p));
            slot.pose_parts.retain(|p| !parts.contains(p));
        }
    }

    // デフォーマとメッシュ: 対・マスク・スロットのマスクに使ってよいメッシュ。
    for deformer in &mut after.deformers {
        if deformer
            .mirror
            .as_ref()
            .is_some_and(|m| deformers.contains(m))
        {
            deformer.mirror = None;
        }
    }
    if !meshes.is_empty() {
        for mesh in &mut after.meshes {
            mesh.masks.retain(|m| !meshes.contains(&m.mesh));
            if mesh.mirror.as_ref().is_some_and(|m| meshes.contains(m)) {
                mesh.mirror = None;
            }
        }
        for slot in &mut after.slots {
            slot.mask_sources.retain(|m| !meshes.contains(m));
        }
    }

    // キーフォーム格子のミラー元 (ブレンドシェイプ格子を含む)。
    let (part_ids, deformer_ids, mesh_ids, glue_ids) = (
        strings(&parts),
        strings(&deformers),
        strings(&meshes),
        strings(&glues),
    );
    for part in &mut after.parts {
        forget_mirror_source(&mut part.keyforms, &part_ids);
    }
    for deformer in &mut after.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(warp) => forget_mirror_source(&mut warp.keyforms, &deformer_ids),
            DeformerKind::Rotation(rotation) => {
                forget_mirror_source(&mut rotation.keyforms, &deformer_ids);
            }
            _ => {}
        }
    }
    for mesh in &mut after.meshes {
        forget_mirror_source(&mut mesh.keyforms, &mesh_ids);
    }
    for glue in &mut after.glues {
        forget_mirror_source(&mut glue.keyforms, &glue_ids);
    }

    // アバタープロファイル: 標準の当たり判定とアプリに見せるスロット。
    if let Some(avatar) = &mut after.avatar {
        avatar
            .hit_areas
            .retain(|a| !hit_areas.contains(&a.hit_area));
        avatar.slots.retain(|s| !slots.contains(s));
    }

    // 物理: パラメータ・デフォーマ・メッシュへの参照と、消えた衝突体 (リグとスロットのリスト)。
    physics_edit::forget_removed(before, after, physics);
    if let Some(before_physics) = before_physics {
        let after_colliders = physics.as_ref().map_or(&[][..], |p| &p.colliders[..]);
        let colliders = removed(&before_physics.colliders, after_colliders, |c| &c.id);
        if !colliders.is_empty() {
            if let Some(settings) = physics.as_mut() {
                for rig in &mut settings.rigs {
                    rig.colliders.retain(|c| !colliders.contains(c));
                }
            }
            for slot in &mut after.slots {
                slot.colliders.retain(|c| !colliders.contains(c));
            }
        }
    }
}

/// ID の文字列 (ミラー元の `MirrorSource::id` は文字列)。
fn strings<K: ToString>(ids: &HashSet<K>) -> HashSet<String> {
    ids.iter().map(ToString::to_string).collect()
}

/// 格子 (とそのブレンドシェイプ格子) のミラー元が `removed` の要素なら記録を外す。
/// ミラーのセルは作り直せなくなるので `authored` にする (値は変えない)。
fn forget_mirror_source<T>(grid: &mut KeyformGrid<T>, removed: &HashSet<String>) {
    for shape in &mut grid.blend_shapes {
        forget_mirror_source(shape, removed);
    }
    if grid
        .mirror_source
        .as_ref()
        .is_some_and(|s| removed.contains(&s.id))
    {
        grid.mirror_source = None;
        for origin in &mut grid.origins {
            if *origin == KeyformOrigin::Mirror {
                *origin = KeyformOrigin::Authored;
            }
        }
    }
}
