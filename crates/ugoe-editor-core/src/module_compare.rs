//! モジュールのプロジェクトの今の編集と、前に書き出した `.ugoemod` の違い (T0654)。
//! エディタのプレビューの着せ替えの一覧で、同じ名前のファイルの行に文字で出す (`docs/editor.md#プレビューの着せ替え`)。
//!
//! 見るのは要素 (パーツ・デフォーマ・メッシュ・グルー) の ID と中身、テクスチャ (名前と画像)、条件、使うパラメータ、
//! 物理のリグと衝突体、ポーズ。違いは 1 行ずつ、種類と引数 ([`DifferenceLine`]・[`ElementChange`]) で「ファイル → 今の編集」の
//! 向きに返し、エディタの画面が今の言語の文にする (`docs/editor.md#違いの種類と引数`。T1677)。
//! 足した・変えたパーツ・デフォーマ・メッシュは、何が違うか (メッシュなら頂点・キーフォーム・描画順など) を要素ごとに添え、
//! ツリーで選べるように作業用のモデルの ID を付ける (T0656)。グルー・物理のリグ・衝突体も同じように中身まで添え、
//! 除いた要素も添える。要素ごとに、プレビューのビューポートで強調するメッシュを付ける (T0657)。
//! ポーズはグループ (要素のパーツの並び) と追従 (本体の要素に加えるリンク) ごとに中身まで添える (T0659)。
//!
//! ファイルのテクスチャがアトラス化してあれば (`crate::atlas::build_module_atlas`)、アトラスの中で今の編集の画像を
//! 置いた場所を探し、メッシュのテクスチャと UV をアトラスの前に戻してから比べる (T0656)。ファイルには置いた場所が
//! 書かれていないので、対応するメッシュの UV の組から場所を推し量り、画素で確かめる。

use std::collections::BTreeSet;
use std::fmt::Display;

use serde::Serialize;
use ts_rs::TS;
use ugoe_format::ModuleFile;
use ugoe_model::module::ModulePose;
use ugoe_model::physics::{PhysicsTarget, VertexOwner};
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{Mesh, Module, PartId, TextureRef};

use crate::module_project::ModuleContext;
use crate::project::{Project, ProjectError, checked_module};
use crate::texture::Textures;

mod atlas;
mod changes;

use atlas::{FileImage, FileView};
pub use changes::{
    ChangedItems, ColliderShapeType, CountedItems, DeformerType, ElementChange, GridSize,
    KeyformValue,
};
use changes::{
    collider_changes, deformer_changes, glue_changes, mesh_changes, or_other, part_changes,
    rig_changes,
};

/// 違いの 1 行 (T0654)。足した・変えた要素ごとの中身の違いを添える (T0656)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ModuleDifference {
    /// 何が違うか (種類と引数。例: `{ kind: "elements", what: "mesh", before: 2, after: 1, removed: ["Ears"], … }`)。
    pub line: DifferenceLine,
    /// 行の要素のうち、足したもの・変えたもの・除いたもの (要素の行だけ)。
    pub items: Vec<DifferenceItem>,
}

/// 違いの 1 行の種類と引数 (T1677)。`before` はファイル、`after` は今の編集の値。ID の並びは要素の並び (名前の集まりは名前の順)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DifferenceLine {
    /// モジュールの名前。
    Name { before: String, after: String },
    /// 付けるスロット。
    Slot { before: String, after: String },
    /// 要素 (`what`) の数と、足した・除いた・変えたものの ID。
    Elements {
        what: DifferenceKind,
        before: usize,
        after: usize,
        added: Vec<String>,
        removed: Vec<String>,
        changed: Vec<String>,
    },
    /// テクスチャの数と、足した・除いた・画像 (か名前のほかの設定) が違うものの名前。
    Textures {
        before: usize,
        after: usize,
        added: Vec<String>,
        removed: Vec<String>,
        changed: Vec<String>,
    },
    /// 条件のアバタープロファイルのレベル (無しは `null`)。
    AvatarLevel {
        before: Option<u8>,
        after: Option<u8>,
    },
    /// 名前の集まり (`what`) の、足した・除いたもの。
    Names {
        what: NameList,
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// ポーズのグループの並びだけが違う。
    PoseOrder,
    /// ファイルのアトラスの中に、今の編集のテクスチャに戻せないメッシュがあった (アトラスを違いとして数える)。
    UnmatchedAtlas,
}

/// [`DifferenceLine::Names`] の名前の集まり。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NameList {
    /// 条件のスロット。
    RequiredSlots,
    /// 条件のパラメータ。
    RequiredParameters,
    /// 使うパラメータ。
    ParametersUsed,
}

/// 違いの行の要素 1 つ。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct DifferenceItem {
    pub kind: DifferenceKind,
    /// モジュールの中の ID (表示用)。
    pub id: String,
    /// 足したか・除いたか・変えたか。
    pub state: ItemState,
    /// 変えた要素の、何が違うか (足した・除いた要素では空)。
    pub changes: Vec<ElementChange>,
    /// 選ぶときの、今の編集の ID (パーツ・デフォーマ・メッシュ・グルーは作業用のモデル、物理のリグと衝突体は
    /// 作業用の物理の ID)。見つからなければ (除いた要素など) `None`。
    pub element: Option<String>,
    /// プレビューのビューポートで強調するメッシュの、モジュールの中の ID (ファイルと今の編集のどちらかにあるもの)。
    /// パーツはその下のメッシュ、デフォーマはその下のメッシュ、グルーは両端、物理のリグは出力先のメッシュとデフォーマの
    /// 下のメッシュ。衝突体は空 (形をそのまま描く)。ポーズのグループは要素とリンクのパーツの下、追従はリンクの
    /// パーツの下のメッシュ。
    pub meshes: Vec<String>,
    /// ポーズのグループと追従を選ぶときの、今の編集のパーツの ID (作業用のモデル。グループは要素のパーツ、
    /// 追従は本体の要素のパーツとリンクのパーツ)。ほかの種類と除いた要素は空 (T0659)。
    pub parts: Vec<String>,
}

/// 違いの行の要素が、ファイルから今の編集でどうなったか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Added,
    Removed,
    Changed,
}

/// 違いの行の要素の種類 (パーツ・デフォーマ・メッシュはエディタの選択の種類と同じ名前)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceKind {
    Part,
    Deformer,
    Mesh,
    Glue,
    PhysicsRig,
    Collider,
    /// ポーズのグループ。ID は要素のパーツを ` / ` でつないだもの (例: `HatA / HatB`)。
    PoseGroup,
    /// 本体のポーズの要素に追従させるパーツ (`ModulePose::links`)。ID は本体の要素のパーツ。
    PoseFollow,
}

/// モジュールのプロジェクトの今の編集と、書き出した `.ugoemod` のバイト列 `bytes` の違い。同じなら空。
/// 今の編集が書き出しの検査に通らなければ、そのエラー。`bytes` が読めなければ [`ProjectError::UnreadableModuleFile`]。
pub fn compare_with_module_file(
    project: &Project,
    bytes: &[u8],
) -> Result<Vec<ModuleDifference>, ProjectError> {
    let file = ugoe_format::read_module(bytes).map_err(ProjectError::UnreadableModuleFile)?;
    let (module, images, _) = checked_module(project, None)?;
    let context = project.module.as_ref().ok_or(ProjectError::NotAModule)?;
    let mut differences = module_differences(&file, &module, &images, &|order| {
        context.map_draw_order(order)
    });
    for item in differences.iter_mut().flat_map(|d| &mut d.items) {
        // 除いた要素は今の編集に無い (同じ ID の本体の要素を指さないように探さない)。
        if item.state == ItemState::Removed {
            continue;
        }
        if matches!(
            item.kind,
            DifferenceKind::PoseGroup | DifferenceKind::PoseFollow
        ) {
            // パーツはモジュールのもの (接頭辞を付けた ID) か、追従の本体の要素 (ID はそのまま)。
            item.parts = pose_parts(&module, item.kind, &item.id)
                .into_iter()
                .filter_map(|part| {
                    workspace_id(project, context, DifferenceKind::Part, part).or_else(|| {
                        project
                            .model
                            .parts
                            .iter()
                            .any(|p| context.is_base_part(&p.id) && p.id == part)
                            .then(|| part.to_owned())
                    })
                })
                .collect();
            item.element = (!item.parts.is_empty()).then(|| item.id.clone());
        } else {
            item.element = workspace_id(project, context, item.kind, &item.id);
        }
    }
    Ok(differences)
}

/// モジュールの中の ID `id` の要素の、作業用のモデル・物理での ID (本体の要素は除く)。
fn workspace_id(
    project: &Project,
    context: &ModuleContext,
    kind: DifferenceKind,
    id: &str,
) -> Option<String> {
    let model = &project.model;
    let own = |workspace: &str| context.display_id(workspace) == id;
    let base = context.base();
    let base_physics = base.physics.as_ref();
    let physics = project.physics.as_ref();
    match kind {
        DifferenceKind::Part => model
            .parts
            .iter()
            .find(|p| !context.is_base_part(&p.id) && own(p.id.as_str()))
            .map(|p| p.id.to_string()),
        DifferenceKind::Deformer => model
            .deformers
            .iter()
            .find(|d| !context.is_base_deformer(&d.id) && own(d.id.as_str()))
            .map(|d| d.id.to_string()),
        DifferenceKind::Mesh => model
            .meshes
            .iter()
            .find(|m| !context.is_base_mesh(&m.id) && own(m.id.as_str()))
            .map(|m| m.id.to_string()),
        DifferenceKind::Glue => model
            .glues
            .iter()
            .find(|g| !base.model.glues.iter().any(|b| b.id == g.id) && own(g.id.as_str()))
            .map(|g| g.id.to_string()),
        DifferenceKind::PhysicsRig => physics?
            .rigs
            .iter()
            .find(|r| {
                !base_physics.is_some_and(|b| b.rigs.iter().any(|b| b.id == r.id))
                    && own(r.id.as_str())
            })
            .map(|r| r.id.to_string()),
        DifferenceKind::Collider => physics?
            .colliders
            .iter()
            .find(|c| {
                !base_physics.is_some_and(|b| b.colliders.iter().any(|b| b.id == c.id))
                    && own(c.id.as_str())
            })
            .map(|c| c.id.to_string()),
        // [`compare_with_module_file`] がパーツから選ぶ。
        DifferenceKind::PoseGroup | DifferenceKind::PoseFollow => None,
    }
}

/// 書き出したファイル `file` から今の編集 (`module` と画像 `images`) への違い。同じなら空。
/// 要素の `element` (作業用のモデルの ID) は埋めない ([`compare_with_module_file`] が埋める)。
/// メッシュの描画順は、モジュールの空間 (0〜1000) の値を `draw_order` で写して書く (エディタでは、ツリーや
/// インスペクタと同じスロットの範囲の値にする)。
pub fn module_differences(
    file: &ModuleFile,
    module: &Module,
    images: &Textures,
    draw_order: &dyn Fn(f32) -> f32,
) -> Vec<ModuleDifference> {
    let view = FileView::new(file, module, images);
    let old = &view.module;
    let mut lines = Vec::new();
    if old.name != module.name {
        lines.push(line(DifferenceLine::Name {
            before: old.name.clone(),
            after: module.name.clone(),
        }));
    }
    if old.slot != module.slot {
        lines.push(line(DifferenceLine::Slot {
            before: old.slot.to_string(),
            after: module.slot.to_string(),
        }));
    }

    elements(
        &mut lines,
        DifferenceKind::Part,
        &keyed(&old.parts, |p| p.id.to_string()),
        &keyed(&module.parts, |p| p.id.to_string()),
        part_changes,
    );
    elements(
        &mut lines,
        DifferenceKind::Deformer,
        &keyed(&old.deformers, |d| d.id.to_string()),
        &keyed(&module.deformers, |d| d.id.to_string()),
        deformer_changes,
    );
    // メッシュのテクスチャは番号なので、名前で比べる (テクスチャの並びが変わっただけなら同じ)。
    elements(
        &mut lines,
        DifferenceKind::Mesh,
        &keyed(&old.meshes, |m| m.id.to_string()),
        &keyed(&module.meshes, |m| m.id.to_string()),
        |a: &Mesh, b: &Mesh| {
            mesh_changes(
                (a, texture_name(old, a)),
                (b, texture_name(module, b)),
                draw_order,
            )
        },
    );
    elements(
        &mut lines,
        DifferenceKind::Glue,
        &keyed(&old.glues, |g| g.id.to_string()),
        &keyed(&module.glues, |g| g.id.to_string()),
        glue_changes,
    );
    textures(&mut lines, &view, module, images);

    let requires = (&old.requires, &module.requires);
    if requires.0.avatar_level != requires.1.avatar_level {
        lines.push(line(DifferenceLine::AvatarLevel {
            before: requires.0.avatar_level,
            after: requires.1.avatar_level,
        }));
    }
    names(
        &mut lines,
        NameList::RequiredSlots,
        &requires.0.slots,
        &requires.1.slots,
    );
    names(
        &mut lines,
        NameList::RequiredParameters,
        &requires.0.parameters,
        &requires.1.parameters,
    );
    let used = (old.referenced_parameters(), module.referenced_parameters());
    names(
        &mut lines,
        NameList::ParametersUsed,
        &used.0.into_iter().collect::<Vec<_>>(),
        &used.1.into_iter().collect::<Vec<_>>(),
    );

    elements(
        &mut lines,
        DifferenceKind::PhysicsRig,
        &keyed(&old.physics.rigs, |r| r.id.to_string()),
        &keyed(&module.physics.rigs, |r| r.id.to_string()),
        rig_changes,
    );
    elements(
        &mut lines,
        DifferenceKind::Collider,
        &keyed(&old.physics.colliders, |c| c.id.to_string()),
        &keyed(&module.physics.colliders, |c| c.id.to_string()),
        collider_changes,
    );
    pose_differences(&mut lines, &old.pose, &module.pose);
    for item in lines.iter_mut().flat_map(|d| &mut d.items) {
        item.meshes = highlighted_meshes(item.kind, &item.id, &[module, old]);
    }
    lines
}

fn line(line: DifferenceLine) -> ModuleDifference {
    ModuleDifference {
        line,
        items: Vec::new(),
    }
}

fn keyed<T>(items: &[T], id: impl Fn(&T) -> String) -> Vec<(String, &T)> {
    items.iter().map(|item| (id(item), item)).collect()
}

fn texture_name<'a>(module: &'a Module, mesh: &Mesh) -> Option<&'a str> {
    module
        .textures
        .get(mesh.texture as usize)
        .map(|t| t.name.as_str())
}

/// ID で対応を取った要素の、数と、足した・除いた・変えたものの行 (違いがなければ書かない)。
/// `changes` は要素の中身の違い (同じなら空)。足した・変えた・除いた要素を行に添える。
fn elements<T>(
    lines: &mut Vec<ModuleDifference>,
    kind: DifferenceKind,
    old: &[(String, &T)],
    new: &[(String, &T)],
    changes: impl Fn(&T, &T) -> Vec<ElementChange>,
) {
    let mut items = Vec::new();
    let mut added = Vec::new();
    let mut changed = Vec::new();
    for (id, element) in new {
        match find(old, id) {
            None => {
                added.push(id.clone());
                items.push(item(kind, id, ItemState::Added, Vec::new()));
            }
            Some(o) => {
                let what = changes(o, element);
                if what.is_empty() {
                    continue;
                }
                changed.push(id.clone());
                items.push(item(kind, id, ItemState::Changed, what));
            }
        }
    }
    let removed: Vec<String> = old
        .iter()
        .filter(|(id, _)| find(new, id).is_none())
        .map(|(id, _)| id.clone())
        .collect();
    items.extend(
        removed
            .iter()
            .map(|id| item(kind, id, ItemState::Removed, Vec::new())),
    );
    push_elements(
        lines,
        kind,
        (old.len(), new.len()),
        added,
        removed,
        changed,
        items,
    );
}

/// 要素の行 (足した・除いた・変えたものがあるときだけ)。
fn push_elements(
    lines: &mut Vec<ModuleDifference>,
    what: DifferenceKind,
    (before, after): (usize, usize),
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
    items: Vec<DifferenceItem>,
) {
    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return;
    }
    lines.push(ModuleDifference {
        line: DifferenceLine::Elements {
            what,
            before,
            after,
            added,
            removed,
            changed,
        },
        items,
    });
}

fn item(
    kind: DifferenceKind,
    id: &str,
    state: ItemState,
    changes: Vec<ElementChange>,
) -> DifferenceItem {
    DifferenceItem {
        kind,
        id: id.to_owned(),
        state,
        changes,
        element: None,
        meshes: Vec::new(),
        parts: Vec::new(),
    }
}

fn find<'a, T>(list: &[(String, &'a T)], id: &str) -> Option<&'a T> {
    list.iter().find(|(i, _)| i == id).map(|(_, t)| *t)
}

/// 名前の集まりの違い (順序は見ない)。
fn names<T: Display + Ord>(
    lines: &mut Vec<ModuleDifference>,
    what: NameList,
    old: &[T],
    new: &[T],
) {
    let old: BTreeSet<String> = old.iter().map(ToString::to_string).collect();
    let new: BTreeSet<String> = new.iter().map(ToString::to_string).collect();
    let added: Vec<String> = new.difference(&old).cloned().collect();
    let removed: Vec<String> = old.difference(&new).cloned().collect();
    if !added.is_empty() || !removed.is_empty() {
        lines.push(line(DifferenceLine::Names {
            what,
            added,
            removed,
        }));
    }
}

// ---- ビューポートで強調するメッシュ ----

/// 要素 `id` を強調するときに輪郭を描くメッシュ (モジュールの中の ID)。`modules` (今の編集とファイル) のどちらかに
/// あるものを、最初に見つけた順に重ねずに並べる。
fn highlighted_meshes(kind: DifferenceKind, id: &str, modules: &[&Module]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |mesh: &str| {
        if !out.iter().any(|m| m == mesh) {
            out.push(mesh.to_owned());
        }
    };
    for module in modules {
        match kind {
            DifferenceKind::Mesh => {
                if module.meshes.iter().any(|m| m.id == id) {
                    push(id);
                }
            }
            DifferenceKind::Part => {
                for mesh in &module.meshes {
                    if in_part(module, mesh.part.as_str(), id) {
                        push(mesh.id.as_str());
                    }
                }
            }
            DifferenceKind::Deformer => {
                for mesh in under_deformer(module, id) {
                    push(mesh);
                }
            }
            DifferenceKind::Glue => {
                if let Some(glue) = module.glues.iter().find(|g| g.id == id) {
                    push(glue.mesh_a.as_str());
                    push(glue.mesh_b.as_str());
                }
            }
            DifferenceKind::PhysicsRig => {
                let Some(rig) = module.physics.rigs.iter().find(|r| r.id == id) else {
                    continue;
                };
                for output in &rig.outputs {
                    match &output.target {
                        PhysicsTarget::DeformerAngle { deformer }
                        | PhysicsTarget::DeformerOffset { deformer }
                        | PhysicsTarget::Vertices {
                            owner: VertexOwner::Warp(deformer),
                            ..
                        } => {
                            for mesh in under_deformer(module, deformer.as_str()) {
                                push(mesh);
                            }
                        }
                        PhysicsTarget::Vertices {
                            owner: VertexOwner::Mesh(mesh),
                            ..
                        } => push(mesh.as_str()),
                        PhysicsTarget::Parameter { .. }
                        | PhysicsTarget::Unknown
                        | PhysicsTarget::Vertices {
                            owner: VertexOwner::Unknown,
                            ..
                        } => {}
                    }
                }
            }
            DifferenceKind::Collider => {}
            DifferenceKind::PoseGroup | DifferenceKind::PoseFollow => {
                for part in pose_parts(module, kind, id) {
                    for mesh in &module.meshes {
                        if in_part(module, mesh.part.as_str(), part) {
                            push(mesh.id.as_str());
                        }
                    }
                }
            }
        }
    }
    out
}

/// ポーズのグループ `id` (要素のパーツと各要素のリンク) か、追従 `id` (本体の要素のパーツとリンク) のパーツ。
/// `module` に無ければ空。
fn pose_parts<'a>(module: &'a Module, kind: DifferenceKind, id: &'a str) -> Vec<&'a str> {
    let entry_parts = |entry: &'a PoseEntry| {
        std::iter::once(entry.part.as_str()).chain(entry.links.iter().map(|p| p.as_str()))
    };
    match kind {
        DifferenceKind::PoseGroup => module
            .pose
            .groups
            .iter()
            .find(|g| group_label(g) == id)
            .map(|g| g.entries.iter().flat_map(entry_parts).collect())
            .unwrap_or_default(),
        DifferenceKind::PoseFollow => module
            .pose
            .links
            .iter()
            .filter(|e| e.part == id)
            .flat_map(entry_parts)
            .collect(),
        _ => Vec::new(),
    }
}

/// パーツ `part` が `ancestor` か、その下 (モジュールの中の親をたどる) か。
fn in_part(module: &Module, part: &str, ancestor: &str) -> bool {
    let mut current = Some(part);
    // 親の輪があっても止まるように、パーツの数だけたどる。
    for _ in 0..=module.parts.len() {
        let Some(id) = current else {
            return false;
        };
        if id == ancestor {
            return true;
        }
        current = module
            .parts
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.parent.as_ref())
            .map(|p| p.as_str());
    }
    false
}

/// デフォーマ `deformer` の下 (子孫のデフォーマを含む) のメッシュ。
fn under_deformer<'a>(module: &'a Module, deformer: &str) -> Vec<&'a str> {
    let below = |parent: Option<&str>| {
        let mut current = parent;
        for _ in 0..=module.deformers.len() {
            let Some(id) = current else {
                return false;
            };
            if id == deformer {
                return true;
            }
            current = module
                .deformers
                .iter()
                .find(|d| d.id == id)
                .and_then(|d| d.parent.as_ref())
                .map(|p| p.as_str());
        }
        false
    };
    module
        .meshes
        .iter()
        .filter(|m| below(m.parent.as_ref().map(|p| p.as_str())))
        .map(|m| m.id.as_str())
        .collect()
}

// ---- ポーズ ----

/// グループの ID (要素のパーツを ` / ` でつなぐ。並びは最初に表示する要素から)。
fn group_label(group: &PoseGroup) -> String {
    group
        .entries
        .iter()
        .map(|e| e.part.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}

/// ポーズの違い: グループ (要素のパーツを 1 つでも共有するものを同じグループとみなす) と、追従 (本体の要素の
/// パーツごと)。どちらにも出ない違い (グループの並びだけ) は 1 行にまとめる。
fn pose_differences(lines: &mut Vec<ModuleDifference>, old: &ModulePose, new: &ModulePose) {
    if old == new {
        return;
    }
    let before = lines.len();
    let shares = |a: &PoseGroup, b: &PoseGroup| {
        a.entries
            .iter()
            .any(|x| b.entries.iter().any(|y| x.part == y.part))
    };
    let mut matched = vec![false; old.groups.len()];
    let (mut added, mut changed) = (Vec::new(), Vec::new());
    let mut items = Vec::new();
    for group in &new.groups {
        let id = group_label(group);
        let found = (0..old.groups.len()).find(|&k| !matched[k] && shares(&old.groups[k], group));
        let Some(k) = found else {
            items.push(item(
                DifferenceKind::PoseGroup,
                &id,
                ItemState::Added,
                Vec::new(),
            ));
            added.push(id);
            continue;
        };
        matched[k] = true;
        let what = group_changes(&old.groups[k], group);
        if !what.is_empty() {
            items.push(item(
                DifferenceKind::PoseGroup,
                &id,
                ItemState::Changed,
                what,
            ));
            changed.push(id);
        }
    }
    let removed: Vec<String> = old
        .groups
        .iter()
        .zip(&matched)
        .filter(|(_, m)| !**m)
        .map(|(g, _)| group_label(g))
        .collect();
    items.extend(removed.iter().map(|id| {
        item(
            DifferenceKind::PoseGroup,
            id,
            ItemState::Removed,
            Vec::new(),
        )
    }));
    push_elements(
        lines,
        DifferenceKind::PoseGroup,
        (old.groups.len(), new.groups.len()),
        added,
        removed,
        changed,
        items,
    );

    elements(
        lines,
        DifferenceKind::PoseFollow,
        &keyed(&old.links, |e| e.part.to_string()),
        &keyed(&new.links, |e| e.part.to_string()),
        |a: &PoseEntry, b: &PoseEntry| {
            let mut out = Vec::new();
            linked(&mut out, None, &a.links, &b.links);
            out
        },
    );
    if lines.len() == before {
        lines.push(line(DifferenceLine::PoseOrder));
    }
}

/// 同じとみなしたグループの中身の違い (要素の並びと、要素ごとのリンク)。
fn group_changes(old: &PoseGroup, new: &PoseGroup) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    let (before, after) = (group_label(old), group_label(new));
    if before != after {
        out.push(ElementChange::Entries { before, after });
    }
    for entry in &new.entries {
        if let Some(before) = old.entries.iter().find(|e| e.part == entry.part) {
            linked(
                &mut out,
                Some(entry.part.to_string()),
                &before.links,
                &entry.links,
            );
        }
    }
    or_other(out)
}

/// リンクのパーツの違い (足した・除いた。並びだけなら [`ElementChange::LinksReordered`])。`entry` はグループの要素のパーツ。
fn linked(out: &mut Vec<ElementChange>, entry: Option<String>, old: &[PartId], new: &[PartId]) {
    if old == new {
        return;
    }
    let added: Vec<String> = new
        .iter()
        .filter(|p| !old.contains(p))
        .map(ToString::to_string)
        .collect();
    let removed: Vec<String> = old
        .iter()
        .filter(|p| !new.contains(p))
        .map(ToString::to_string)
        .collect();
    if added.is_empty() && removed.is_empty() {
        out.push(ElementChange::LinksReordered { entry });
    } else {
        out.push(ElementChange::Links {
            entry,
            added,
            removed,
        });
    }
}

// ---- テクスチャ ----

/// テクスチャ (名前・法線マップの名前・画像) の違い。ファイルのアトラスは、今の編集のテクスチャに戻したもので比べる。
fn textures(
    lines: &mut Vec<ModuleDifference>,
    view: &FileView<'_>,
    module: &Module,
    images: &Textures,
) {
    let old: Vec<(usize, &TextureRef)> = view
        .module
        .textures
        .iter()
        .enumerate()
        .filter(|(i, _)| !matches!(view.images[*i], FileImage::Hidden))
        .collect();
    let find = |name: &str| old.iter().find(|(_, t)| t.name == name).map(|(i, _)| *i);
    let added: Vec<String> = module
        .textures
        .iter()
        .filter(|t| find(&t.name).is_none())
        .map(|t| t.name.clone())
        .collect();
    let removed: Vec<String> = old
        .iter()
        .filter(|(_, t)| !module.textures.iter().any(|n| n.name == t.name))
        .map(|(_, t)| t.name.clone())
        .collect();
    let file = view.file;
    let changed: Vec<String> = module
        .textures
        .iter()
        .filter(|t| {
            find(&t.name).is_some_and(|i| {
                let image = |name: &str| images.get(name).map(|png| &png[..]);
                view.module.textures[i] != **t
                    || match view.images[i] {
                        FileImage::File(j) => {
                            let normal = file.normal_map(j).map(|d| &d.bytes[..]);
                            image(&t.name) != file.textures.get(j).map(|d| &d.bytes[..])
                                || t.normal_map.as_deref().and_then(image) != normal
                        }
                        FileImage::Packed { same } => !same,
                        FileImage::Hidden => false,
                    }
            })
        })
        .map(|t| t.name.clone())
        .collect();
    if !added.is_empty() || !removed.is_empty() || !changed.is_empty() {
        lines.push(line(DifferenceLine::Textures {
            before: old.len(),
            after: module.textures.len(),
            added,
            removed,
            changed,
        }));
    }
    if view.unresolved {
        lines.push(line(DifferenceLine::UnmatchedAtlas));
    }
}
