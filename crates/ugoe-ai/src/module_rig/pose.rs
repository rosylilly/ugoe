//! 衣装の差分 (A / B) をポーズにする提案 (T0519)。
//!
//! 名前の末尾の文字 (`A`〜`E`) だけが違う衣装のパーツ (PSD のグループ) かメッシュ (PSD のレイヤー) を差分とみなす
//! (`Sleeve A` / `Sleeve B`、`ribbon_a` / `ribbon_b`、`HatA` / `HatB`)。区切りのない小文字は差分とみなさない
//! (`Tuba` は `Tub` の差分ではない)。
//!
//! - 本体のポーズのグループの要素のうち、スロットが公開するパーツ (`Slot::pose_parts`) の名前が同じ文字の差分に
//!   なっていて、同じ文字の衣装の差分と本体の差分の絵が基準の姿で重なれば、衣装の差分を本体の要素の `links` に
//!   加える (本体の腕の差分に合わせて袖の差分を切り替える)。
//! - そうでなければ、衣装の差分だけのポーズのグループを足す (帽子の差分 A / B)。
//!
//! メッシュの差分は、差分ごとにメッシュだけを持つパーツを作って入れる (ポーズはパーツを切り替えるため)。

use std::collections::BTreeMap;

use ugoe_editor_core::{Command, ModuleContext};
use ugoe_model::part::PartKeyform;
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{KeyformGrid, MeshId, Model, Part, PartId};

use super::{fresh_id, overlap};
use crate::AiError;
use crate::rig::{Rect, Rig};

/// 差分の要素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Element {
    Part(PartId),
    Mesh(MeshId),
}

/// 同じ元の名前の差分の組。
#[derive(Debug, Clone)]
pub(super) struct Variants {
    /// 末尾の文字を除いた名前。
    pub name: String,
    /// 文字 (大文字) の順の差分。
    pub entries: Vec<(char, Element)>,
}

/// 名前の末尾の差分の文字 (大文字) と、それを除いた名前 (小文字にし、区切りを除いたもの)。
fn split_variant(name: &str) -> Option<(String, char)> {
    let name = name.trim();
    let last = name.chars().last()?;
    let letter = last.to_ascii_uppercase();
    if !('A'..='E').contains(&letter) {
        return None;
    }
    let head = &name[..name.len() - last.len_utf8()];
    let trimmed = head.trim_end_matches([' ', '_', '-', '.']);
    let separated = trimmed.len() < head.len();
    let before = trimmed.chars().last()?;
    // 区切りがなければ、大文字で、前が大文字でない (`ArmA`・`袖A`。`Tuba`・`ABC` は違う)。
    if !separated && !(last.is_ascii_uppercase() && !before.is_ascii_uppercase()) {
        return None;
    }
    let key: String = trimmed
        .chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-' | '.'))
        .flat_map(char::to_lowercase)
        .collect();
    (!key.is_empty()).then_some((key, letter))
}

/// 見つけた差分 (文字・要素・名前)。
type Found = (char, Element, String);

/// 衣装 (モジュール) のパーツとメッシュのうち、差分の組になっているもの。ポーズにすでに入っているパーツは除く。
pub(super) fn variants(model: &Model, context: &ModuleContext) -> Vec<Variants> {
    let posed: Vec<&PartId> = model
        .pose
        .groups
        .iter()
        .flat_map(|g| &g.entries)
        .flat_map(|e| std::iter::once(&e.part).chain(&e.links))
        .collect();
    // (種類, 親のパーツ, 元の名前) → 文字ごとの要素
    let mut found: BTreeMap<(u8, Option<PartId>, String), Vec<Found>> = BTreeMap::new();
    for part in &model.parts {
        if context.is_base_part(&part.id) || posed.contains(&&part.id) {
            continue;
        }
        if let Some((key, letter)) = split_variant(&part.name) {
            found
                .entry((0, part.parent.clone(), key))
                .or_default()
                .push((letter, Element::Part(part.id.clone()), part.name.clone()));
        }
    }
    for mesh in &model.meshes {
        if context.is_base_mesh(&mesh.id) || mesh.mask_only {
            continue;
        }
        if let Some((key, letter)) = split_variant(&mesh.name) {
            found
                .entry((1, Some(mesh.part.clone()), key))
                .or_default()
                .push((letter, Element::Mesh(mesh.id.clone()), mesh.name.clone()));
        }
    }
    let mut out = Vec::new();
    for ((_, _, _), mut entries) in found {
        entries.sort_by_key(|(letter, ..)| *letter);
        let distinct = entries.windows(2).all(|w| w[0].0 != w[1].0);
        if entries.len() < 2 || !distinct {
            continue;
        }
        let name = entries[0].2.clone();
        let name = name[..name.len() - 1]
            .trim_end_matches([' ', '_', '-', '.'])
            .to_owned();
        out.push(Variants {
            name,
            entries: entries.into_iter().map(|(l, e, _)| (l, e)).collect(),
        });
    }
    out
}

/// 差分の組をポーズにするコマンドを加え、理由を返す。組むものがなければ何もしない。
pub(super) fn propose(
    rig: &mut Rig,
    context: &ModuleContext,
    variants: &[Variants],
    bounds: &BTreeMap<MeshId, Rect>,
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    if variants.is_empty() {
        return Ok(());
    }
    let mut pose = rig.model().pose.clone();
    for set in variants {
        let mut parts: Vec<(char, PartId)> = Vec::new();
        for (letter, element) in &set.entries {
            let part = match element {
                Element::Part(id) => id.clone(),
                Element::Mesh(id) => mesh_part(rig, context, id)?,
            };
            parts.push((*letter, part));
        }
        let letters: String = parts.iter().map(|(l, _)| *l).collect();
        match linked_group(rig.model(), context, &parts, bounds) {
            Some((group, base)) => {
                for ((_, part), base_part) in parts.iter().zip(&base) {
                    if let Some(entry) = pose.groups[group]
                        .entries
                        .iter_mut()
                        .find(|e| &e.part == base_part)
                    {
                        entry.links.push(part.clone());
                    }
                }
                rationale.push(format!(
                    "ポーズ: 衣装の差分「{}」({letters}) を、本体のポーズの要素 {} に追従させる (同じ文字の差分を一緒に切り替える)",
                    set.name,
                    base.iter()
                        .map(|p| format!("`{p}`"))
                        .collect::<Vec<_>>()
                        .join("・")
                ));
            }
            None => {
                pose.groups.push(PoseGroup {
                    entries: parts
                        .iter()
                        .map(|(_, part)| PoseEntry {
                            part: part.clone(),
                            links: Vec::new(),
                        })
                        .collect(),
                });
                rationale.push(format!(
                    "ポーズ: 衣装の差分「{}」({letters}) を、衣装だけのポーズのグループにする (表示するのは 1 つ。最初は {})",
                    set.name,
                    parts.first().map_or('A', |(l, _)| *l)
                ));
            }
        }
    }
    rig.apply(Command::SetPose { pose })
}

/// メッシュの差分を入れるパーツを、元のパーツの子として作る (ポーズはパーツを切り替えるため)。
fn mesh_part(rig: &mut Rig, context: &ModuleContext, mesh: &MeshId) -> Result<PartId, AiError> {
    let model = rig.model();
    let Some(found) = model.meshes.iter().find(|m| &m.id == mesh) else {
        return Err(crate::rig::template(format!("no mesh `{mesh}`")));
    };
    let (name, parent) = (found.name.clone(), found.part.clone());
    let id = PartId::new(fresh_id(
        context,
        context.display_id(mesh.as_str()),
        model.parts.iter().map(|p| p.id.as_str()),
    ));
    rig.apply(Command::AddPart {
        part: Part {
            id: id.clone(),
            name,
            parent: Some(parent),
            keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        },
    })?;
    rig.apply(Command::SetMeshProperties {
        id: mesh.clone(),
        part: Some(id.clone()),
        texture: None,
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
        lit: None,
    })?;
    Ok(id)
}

/// パーツ `part` とその子孫のパーツのメッシュの、基準の姿の外接矩形。
fn part_bounds(model: &Model, bounds: &BTreeMap<MeshId, Rect>, part: &PartId) -> Option<Rect> {
    let mut inside = vec![part.clone()];
    let mut i = 0;
    while let Some(p) = inside.get(i).cloned() {
        inside.extend(
            model
                .parts
                .iter()
                .filter(|c| c.parent.as_ref() == Some(&p))
                .map(|c| c.id.clone()),
        );
        i += 1;
    }
    model
        .meshes
        .iter()
        .filter(|m| inside.contains(&m.part))
        .filter_map(|m| bounds.get(&m.id).copied())
        .reduce(Rect::union)
}

/// 衣装の差分 `parts` と同じ文字の差分を、スロットが公開するパーツで持つ本体のポーズのグループ
/// (グループの添字と、`parts` の順の本体のパーツ)。同じ文字の衣装と本体の絵が基準の姿で重ならなければ
/// 従わない (腕の差分と関係のない、胸の飾りの差分は追従させない)。
fn linked_group(
    model: &Model,
    context: &ModuleContext,
    parts: &[(char, PartId)],
    bounds: &BTreeMap<MeshId, Rect>,
) -> Option<(usize, Vec<PartId>)> {
    let exposed = &context.slot().pose_parts;
    model
        .pose
        .groups
        .iter()
        .enumerate()
        .find_map(|(index, group)| {
            let letters: BTreeMap<char, &PartId> = group
                .entries
                .iter()
                .filter(|e| exposed.contains(&e.part))
                .filter_map(|e| {
                    let name = model
                        .parts
                        .iter()
                        .find(|p| p.id == e.part)
                        .map_or(e.part.as_str(), |p| p.name.as_str());
                    split_variant(name).map(|(_, letter)| (letter, &e.part))
                })
                .collect();
            parts
                .iter()
                .map(|(letter, costume)| {
                    let base = letters.get(letter)?;
                    let a = part_bounds(model, bounds, costume)?;
                    let b = part_bounds(model, bounds, base)?;
                    (overlap(a, b) > 0.0).then(|| (*base).clone())
                })
                .collect::<Option<Vec<_>>>()
                .map(|base| (index, base))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_names() {
        assert_eq!(split_variant("Sleeve A"), Some(("sleeve".into(), 'A')));
        assert_eq!(split_variant("ribbon_b"), Some(("ribbon".into(), 'B')));
        assert_eq!(split_variant("HatA"), Some(("hat".into(), 'A')));
        assert_eq!(split_variant("袖B"), Some(("袖".into(), 'B')));
        assert_eq!(split_variant("Tuba"), None);
        assert_eq!(split_variant("ABC"), None);
        assert_eq!(split_variant("Hat F"), None);
        assert_eq!(split_variant("B"), None);
    }
}
