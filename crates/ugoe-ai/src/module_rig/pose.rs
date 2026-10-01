//! 衣装の差分 (A / B) をポーズにする提案 (T0519・T0521)。
//!
//! 差分は 2 通りの方法で見つける。
//!
//! - **名前:** 名前の末尾の文字 (`A`〜`E`) だけが違う衣装のパーツ (PSD のグループ) かメッシュ (PSD のレイヤー)
//!   (`Sleeve A` / `Sleeve B`、`ribbon_a` / `ribbon_b`、`HatA` / `HatB`)。区切りのない小文字は差分とみなさない
//!   (`Tuba` は `Tub` の差分ではない)。
//! - **絵の重なりと意味ラベル:** 名前で見つからなかった衣装のうち、同じ親のパーツの中の同じ種類 (パーツかメッシュ) で、
//!   意味ラベルと左右が同じ (どちらもラベルなしを含む) で、基準の姿の外接矩形の重なり (IoU) が [`MIN_IOU`] 以上のもの。
//!   PSD で見えている 1 つと、隠してある (不透明度 0 で取り込まれた) ものの組だけを差分とみなす。どれも見えている
//!   重なった絵は、影や線画のように重ねて描く部品のことが多いため。見えているものを最初に表示する。
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
use crate::classify::{Label, Side};
use crate::rig::{MeshPart, Rect, Rig};

/// 絵の重なりで差分とみなす、外接矩形の重なり (共通部分の面積 / 和の面積) の下限。
pub(super) const MIN_IOU: f32 = 0.6;

/// 差分の要素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Element {
    Part(PartId),
    Mesh(MeshId),
}

/// 差分の組。
#[derive(Debug, Clone)]
pub(super) struct Variants {
    /// 末尾の文字を除いた名前 (絵の重なりで見つけたものは、見えている差分の名前)。
    pub name: String,
    /// 文字 (大文字) の順の差分。絵の重なりで見つけたものは、見えているものを `A` にした順。
    pub entries: Vec<(char, Element)>,
    /// 名前で見つけた (偽なら絵の重なりと意味ラベル)。本体のポーズの要素への追従は、同じ文字の名前で決めるので
    /// 名前で見つけたものだけ。
    pub named: bool,
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

/// ポーズのグループの要素とリンクのパーツ。
fn posed_parts(model: &Model) -> Vec<&PartId> {
    model
        .pose
        .groups
        .iter()
        .flat_map(|g| &g.entries)
        .flat_map(|e| std::iter::once(&e.part).chain(&e.links))
        .collect()
}

/// 衣装 (モジュール) のパーツとメッシュのうち、差分の組になっているもの (名前で見つけたものが先)。
/// ポーズにすでに入っているパーツは除く。`labels` はメッシュの意味ラベル、`bounds` は基準の姿の外接矩形。
pub(super) fn variants(
    model: &Model,
    context: &ModuleContext,
    labels: &BTreeMap<MeshId, MeshPart>,
    bounds: &BTreeMap<MeshId, Rect>,
) -> Vec<Variants> {
    let mut out = named_variants(model, context);
    let taken: Vec<Element> = out
        .iter()
        .flat_map(|v| v.entries.iter().map(|(_, e)| e.clone()))
        .collect();
    out.extend(drawn_variants(model, context, labels, bounds, &taken));
    out
}

/// 名前の末尾の文字だけが違う差分の組。
fn named_variants(model: &Model, context: &ModuleContext) -> Vec<Variants> {
    let posed = posed_parts(model);
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
            named: true,
        });
    }
    out
}

/// 絵の重なりで差分を探す要素。
struct Drawn {
    element: Element,
    name: String,
    /// 同じ組になれる範囲 (種類と親のパーツ)。
    scope: (u8, Option<PartId>),
    /// 意味ラベルと左右 (パーツは、中のメッシュのラベルがすべて同じときだけ)。
    label: Option<(Label, Side)>,
    rect: Rect,
    hidden: bool,
}

/// 外接矩形の重なり (共通部分の面積 / 和の面積)。
fn iou(a: Rect, b: Rect) -> f32 {
    let inter = (a.max.min(b.max) - a.min.max(b.min)).max(ugoe_core::Vec2::ZERO);
    let inter = inter.x * inter.y;
    let area = |r: Rect| r.size().x.max(0.0) * r.size().y.max(0.0);
    let union = area(a) + area(b) - inter;
    if union <= f32::EPSILON {
        0.0
    } else {
        inter / union
    }
}

/// 絵の重なりと意味ラベルで見つけた差分の組 (モジュールの説明は [`self`])。`taken` は名前で見つけた要素。
fn drawn_variants(
    model: &Model,
    context: &ModuleContext,
    labels: &BTreeMap<MeshId, MeshPart>,
    bounds: &BTreeMap<MeshId, Rect>,
    taken: &[Element],
) -> Vec<Variants> {
    let posed = posed_parts(model);
    let mut drawn = Vec::new();
    for mesh in &model.meshes {
        let element = Element::Mesh(mesh.id.clone());
        if context.is_base_mesh(&mesh.id)
            || mesh.mask_only
            || taken.contains(&element)
            || posed.contains(&&mesh.part)
        {
            continue;
        }
        let Some(rect) = bounds.get(&mesh.id).copied() else {
            continue;
        };
        drawn.push(Drawn {
            element,
            name: mesh.name.clone(),
            scope: (1, Some(mesh.part.clone())),
            label: labels.get(&mesh.id).map(|p| (p.label, p.side)),
            rect,
            hidden: mesh_hidden(&mesh.keyforms),
        });
    }
    for part in &model.parts {
        let element = Element::Part(part.id.clone());
        if context.is_base_part(&part.id) || taken.contains(&element) || posed.contains(&&part.id) {
            continue;
        }
        let Some(rect) = part_bounds(model, bounds, &part.id) else {
            continue;
        };
        let inside = part_meshes(model, &part.id);
        let mut found = inside
            .iter()
            .map(|m| labels.get(m).map(|p| (p.label, p.side)));
        let first = found.next().flatten();
        let label = if found.all(|l| l == first) {
            first
        } else {
            None
        };
        drawn.push(Drawn {
            element,
            name: part.name.clone(),
            scope: (0, part.parent.clone()),
            label,
            rect,
            hidden: part.keyforms.keyforms.iter().all(|k| k.opacity <= 0.0),
        });
    }
    // 隠してあるものを、同じ範囲・同じラベルで最もよく重なる見えているものに付ける。
    let mut sets: Vec<(usize, Vec<usize>)> = Vec::new();
    for (h, hidden) in drawn.iter().enumerate().filter(|(_, d)| d.hidden) {
        let best = drawn
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.hidden && v.scope == hidden.scope && v.label == hidden.label)
            .map(|(i, v)| (i, iou(v.rect, hidden.rect)))
            .filter(|(_, score)| *score >= MIN_IOU)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let Some((v, _)) = best else {
            continue;
        };
        match sets.iter_mut().find(|(shown, _)| *shown == v) {
            Some((_, hiddens)) => hiddens.push(h),
            None => sets.push((v, vec![h])),
        }
    }
    sets.into_iter()
        .map(|(shown, hiddens)| {
            let entries = std::iter::once(shown)
                .chain(hiddens)
                .zip('A'..='E')
                .map(|(i, letter)| (letter, drawn[i].element.clone()))
                .collect();
            Variants {
                name: drawn[shown].name.clone(),
                entries,
                named: false,
            }
        })
        .collect()
}

/// メッシュのどのキーフォームでも不透明度が 0 (PSD で隠してあったレイヤー)。
fn mesh_hidden(keyforms: &KeyformGrid<ugoe_model::mesh::MeshKeyform>) -> bool {
    keyforms.keyforms.iter().all(|k| k.opacity <= 0.0)
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
        let linked = if set.named {
            linked_group(rig.model(), context, &parts, bounds)
        } else {
            None
        };
        match linked {
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
                    "Pose: the costume variants “{}” ({letters}) follow the base pose entries {} (variants with the same letter switch together)",
                    set.name,
                    base.iter()
                        .map(|p| format!("`{p}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            None if !set.named => {
                pose.groups.push(PoseGroup {
                    entries: parts
                        .iter()
                        .map(|(_, part)| PoseEntry {
                            part: part.clone(),
                            links: Vec::new(),
                        })
                        .collect(),
                });
                let names: Vec<String> = parts
                    .iter()
                    .map(|(_, part)| {
                        let model = rig.model();
                        let name = model
                            .parts
                            .iter()
                            .find(|p| &p.id == part)
                            .map_or(part.as_str(), |p| p.name.as_str());
                        format!("“{name}”")
                    })
                    .collect();
                rationale.push(format!(
                    "Pose: {} overlap, have the same label and are shown and hidden in the PSD, so they become a costume-only pose group (first shown: “{}”)",
                    names.join(", "),
                    set.name
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
                    "Pose: the costume variants “{}” ({letters}) become a costume-only pose group (one is shown at a time; first {})",
                    set.name,
                    parts.first().map_or('A', |(l, _)| *l)
                ));
            }
        }
    }
    rig.apply(Command::SetPose { pose })
}

/// メッシュの差分を入れるパーツを、元のパーツの子として作る (ポーズはパーツを切り替えるため)。
///
/// PSD で隠してあったメッシュ (どのキーフォームでも不透明度 0) は、ポーズが表示を切り替えられるよう
/// メッシュの不透明度を 1 にし、代わりに作るパーツの不透明度を 0 にする (基準の姿の見た目は変わらない。
/// ポーズはパーツの不透明度を上書きする)。
fn mesh_part(rig: &mut Rig, context: &ModuleContext, mesh: &MeshId) -> Result<PartId, AiError> {
    let model = rig.model();
    let Some(found) = model.meshes.iter().find(|m| &m.id == mesh) else {
        return Err(crate::rig::template(format!("no mesh `{mesh}`")));
    };
    let (name, parent) = (found.name.clone(), found.part.clone());
    let hidden = mesh_hidden(&found.keyforms);
    let mut shown = found.keyforms.clone();
    for keyform in &mut shown.keyforms {
        keyform.opacity = 1.0;
    }
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
            keyforms: KeyformGrid::constant(PartKeyform {
                opacity: if hidden { 0.0 } else { 1.0 },
            }),
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
    if hidden {
        rig.apply(Command::SetMeshKeyforms {
            id: mesh.clone(),
            keyforms: shown,
        })?;
    }
    Ok(id)
}

/// パーツ `part` とその子孫のパーツのメッシュ。
fn part_meshes(model: &Model, part: &PartId) -> Vec<MeshId> {
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
        .map(|m| m.id.clone())
        .collect()
}

/// パーツ `part` とその子孫のパーツのメッシュの、基準の姿の外接矩形。
fn part_bounds(model: &Model, bounds: &BTreeMap<MeshId, Rect>, part: &PartId) -> Option<Rect> {
    part_meshes(model, part)
        .iter()
        .filter_map(|m| bounds.get(m).copied())
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

    #[test]
    fn iou_is_the_shared_fraction_of_the_union() {
        let rect = |x0: f32, y0: f32, x1: f32, y1: f32| Rect {
            min: ugoe_core::Vec2::new(x0, y0),
            max: ugoe_core::Vec2::new(x1, y1),
        };
        let a = rect(0.0, 0.0, 10.0, 10.0);
        assert!((iou(a, a) - 1.0).abs() < 1e-6);
        // 半分ずらすと、共通部分 50・和 150。
        assert!((iou(a, rect(5.0, 0.0, 15.0, 10.0)) - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(iou(a, rect(20.0, 20.0, 30.0, 30.0)), 0.0);
    }
}
