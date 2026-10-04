//! モジュールのプロジェクトの今の編集と、前に書き出した `.ugoemod` の違い (T0654)。
//! エディタのプレビューの着せ替えの一覧で、同じ名前のファイルの行に文字で出す (`docs/editor.md#プレビューの着せ替え`)。
//!
//! 見るのは要素 (パーツ・デフォーマ・メッシュ・グルー) の ID と中身、テクスチャ (名前と画像)、条件、使うパラメータ、
//! 物理のリグと衝突体、ポーズ。違いは英語の 1 行ずつ (エディタの画面にそのまま出す) で、「ファイル → 今の編集」の向きに書く。
//! 足した・変えたパーツ・デフォーマ・メッシュは、何が違うか (メッシュなら頂点・キーフォーム・描画順など) を要素ごとに添え、
//! ツリーで選べるように作業用のモデルの ID を付ける (T0656)。グルー・物理のリグ・衝突体も同じように中身まで添え、
//! 除いた要素も添える。要素ごとに、プレビューのビューポートで強調するメッシュを付ける (T0657)。
//! ポーズはグループ (要素のパーツの並び) と追従 (本体の要素に加えるリンク) ごとに中身まで添える (T0659)。
//!
//! ファイルのテクスチャがアトラス化してあれば (`crate::atlas::build_module_atlas`)、アトラスの中で今の編集の画像を
//! 置いた場所を探し、メッシュのテクスチャと UV をアトラスの前に戻してから比べる (T0656)。ファイルには置いた場所が
//! 書かれていないので、対応するメッシュの UV の組から場所を推し量り、画素で確かめる。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;

use serde::Serialize;
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_format::ModuleFile;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::module::ModulePose;
use ugoe_model::physics::{
    ColliderId, ColliderShape, PhysicsCollider, PhysicsRig, PhysicsTarget, VertexOwner,
};
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{
    Deformer, DeformerKind, Glue, KeyformGrid, Mesh, Module, Part, PartId, TextureRef,
};

use crate::atlas::{ATLAS_NAME_PREFIX, Placement};
use crate::module_project::ModuleContext;
use crate::project::{Project, ProjectError, checked_module};
use crate::texture::{Textures, decode_rgba_png};

/// 違いの 1 行 (T0654)。足した・変えた要素ごとの中身の違いを添える (T0656)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ModuleDifference {
    /// 英語の 1 行 (例: `Meshes: 2 → 1 (removed: Ears; changed: Band)`)。
    pub text: String,
    /// 行の要素のうち、足したもの・変えたもの・除いたもの (要素の行だけ)。
    pub items: Vec<DifferenceItem>,
}

/// 違いの行の要素 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct DifferenceItem {
    pub kind: DifferenceKind,
    /// モジュールの中の ID (表示用)。
    pub id: String,
    /// 何が違うか (英語。例: `vertices 12 → 14; draw order 500 → 520`。足したものは `added`、除いたものは `removed`)。
    pub text: String,
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
/// 今の編集が書き出しの検査に通らなければ、そのエラー。`bytes` が読めなければ [`ProjectError::Export`]。
pub fn compare_with_module_file(
    project: &Project,
    bytes: &[u8],
) -> Result<Vec<ModuleDifference>, ProjectError> {
    let file = ugoe_format::read_module(bytes).map_err(ProjectError::Export)?;
    let (module, images, _) = checked_module(project, None)?;
    let context = project.module.as_ref().ok_or(ProjectError::NotAModule)?;
    let mut differences = module_differences(&file, &module, &images, &|order| {
        context.map_draw_order(order)
    });
    for item in differences.iter_mut().flat_map(|d| &mut d.items) {
        // 除いた要素は今の編集に無い (同じ ID の本体の要素を指さないように探さない)。
        if item.text == REMOVED {
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
                            .any(|p| context.is_base_part(&p.id) && p.id.as_str() == part)
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

/// 除いた要素の [`DifferenceItem::text`]。
const REMOVED: &str = "removed";

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
        lines.push(line(format!("Name: {} → {}", old.name, module.name)));
    }
    if old.slot != module.slot {
        lines.push(line(format!("Slot: {} → {}", old.slot, module.slot)));
    }

    elements(
        &mut lines,
        "Parts",
        DifferenceKind::Part,
        &keyed(&old.parts, |p| p.id.to_string()),
        &keyed(&module.parts, |p| p.id.to_string()),
        part_changes,
    );
    elements(
        &mut lines,
        "Deformers",
        DifferenceKind::Deformer,
        &keyed(&old.deformers, |d| d.id.to_string()),
        &keyed(&module.deformers, |d| d.id.to_string()),
        deformer_changes,
    );
    // メッシュのテクスチャは番号なので、名前で比べる (テクスチャの並びが変わっただけなら同じ)。
    elements(
        &mut lines,
        "Meshes",
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
        "Glues",
        DifferenceKind::Glue,
        &keyed(&old.glues, |g| g.id.to_string()),
        &keyed(&module.glues, |g| g.id.to_string()),
        glue_changes,
    );
    textures(&mut lines, &view, module, images);

    let requires = (&old.requires, &module.requires);
    if requires.0.avatar_level != requires.1.avatar_level {
        let level = |l: Option<u8>| l.map_or_else(|| "none".to_owned(), |l| l.to_string());
        lines.push(line(format!(
            "Required avatar level: {} → {}",
            level(requires.0.avatar_level),
            level(requires.1.avatar_level)
        )));
    }
    names(
        &mut lines,
        "Required slots",
        &requires.0.slots,
        &requires.1.slots,
    );
    names(
        &mut lines,
        "Required parameters",
        &requires.0.parameters,
        &requires.1.parameters,
    );
    let used = (old.referenced_parameters(), module.referenced_parameters());
    names(
        &mut lines,
        "Parameters used",
        &used.0.into_iter().collect::<Vec<_>>(),
        &used.1.into_iter().collect::<Vec<_>>(),
    );

    elements(
        &mut lines,
        "Physics rigs",
        DifferenceKind::PhysicsRig,
        &keyed(&old.physics.rigs, |r| r.id.to_string()),
        &keyed(&module.physics.rigs, |r| r.id.to_string()),
        rig_changes,
    );
    elements(
        &mut lines,
        "Colliders",
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

fn line(text: String) -> ModuleDifference {
    ModuleDifference {
        text,
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
    label: &str,
    kind: DifferenceKind,
    old: &[(String, &T)],
    new: &[(String, &T)],
    changes: impl Fn(&T, &T) -> Vec<String>,
) {
    let mut items = Vec::new();
    let mut added = Vec::new();
    let mut changed = Vec::new();
    for (id, element) in new {
        let text = match find(old, id) {
            None => {
                added.push(id.as_str());
                "added".to_owned()
            }
            Some(o) => {
                let what = changes(o, element);
                if what.is_empty() {
                    continue;
                }
                changed.push(id.as_str());
                what.join("; ")
            }
        };
        items.push(item(kind, id, text));
    }
    let removed: Vec<&str> = old
        .iter()
        .filter(|(id, _)| find(new, id).is_none())
        .map(|(id, _)| id.as_str())
        .collect();
    items.extend(removed.iter().map(|id| item(kind, id, REMOVED.to_owned())));
    if let Some(details) = details(&added, &removed, &changed, "changed") {
        lines.push(ModuleDifference {
            text: format!("{label}: {} → {} ({details})", old.len(), new.len()),
            items,
        });
    }
}

fn item(kind: DifferenceKind, id: &str, text: String) -> DifferenceItem {
    DifferenceItem {
        kind,
        id: id.to_owned(),
        text,
        element: None,
        meshes: Vec::new(),
        parts: Vec::new(),
    }
}

fn find<'a, T>(list: &[(String, &'a T)], id: &str) -> Option<&'a T> {
    list.iter().find(|(i, _)| i == id).map(|(_, t)| *t)
}

/// 足した・除いた・変えたもの (`changed` の見出しは `verb`)。どれもなければ `None`。
fn details(added: &[&str], removed: &[&str], changed: &[&str], verb: &str) -> Option<String> {
    let parts: Vec<String> = [("added", added), ("removed", removed), (verb, changed)]
        .into_iter()
        .filter(|(_, ids)| !ids.is_empty())
        .map(|(what, ids)| format!("{what}: {}", ids.join(", ")))
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// 名前の集まりの違い (順序は見ない)。
fn names<T: Display + Ord>(lines: &mut Vec<ModuleDifference>, label: &str, old: &[T], new: &[T]) {
    let old: BTreeSet<String> = old.iter().map(ToString::to_string).collect();
    let new: BTreeSet<String> = new.iter().map(ToString::to_string).collect();
    let added: Vec<&str> = new.difference(&old).map(String::as_str).collect();
    let removed: Vec<&str> = old.difference(&new).map(String::as_str).collect();
    if let Some(details) = details(&added, &removed, &[], "changed") {
        lines.push(line(format!("{label}: {details}")));
    }
}

// ---- 要素の中身の違い ----

/// `n` 個の `singular` (`plural`)。
fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

fn or_none<T: Display>(value: Option<&T>) -> String {
    value.map_or_else(|| "none".to_owned(), ToString::to_string)
}

/// 名前・所属・親の違い (パーツ・デフォーマ・メッシュで共通)。
fn common_changes<P: Display + PartialEq, Q: Display + PartialEq>(
    out: &mut Vec<String>,
    names: (&str, &str),
    part: Option<(&P, &P)>,
    parent: (Option<&Q>, Option<&Q>),
) {
    if names.0 != names.1 {
        out.push(format!("name {} → {}", names.0, names.1));
    }
    if let Some((a, b)) = part
        && a != b
    {
        out.push(format!("part {a} → {b}"));
    }
    if parent.0 != parent.1 {
        out.push(format!(
            "parent {} → {}",
            or_none(parent.0),
            or_none(parent.1)
        ));
    }
}

/// キーフォームの格子の違い。数が同じなら、対応するキーフォームの組を `values` に渡して中身を比べる。
fn keyform_changes<T: PartialEq>(
    out: &mut Vec<String>,
    old: &KeyformGrid<T>,
    new: &KeyformGrid<T>,
    values: impl FnOnce(&[(&T, &T)], &mut Vec<String>),
) {
    if old.bindings != new.bindings {
        out.push("keyform parameters".to_owned());
    }
    if old.keyforms.len() == new.keyforms.len() {
        let pairs: Vec<(&T, &T)> = old.keyforms.iter().zip(&new.keyforms).collect();
        values(&pairs, out);
    } else {
        out.push(format!(
            "keyforms {} → {}",
            old.keyforms.len(),
            new.keyforms.len()
        ));
    }
    if old.blend_shapes != new.blend_shapes {
        out.push("blend shapes".to_owned());
    }
    if old.interpolation != new.interpolation
        || old.origins != new.origins
        || old.inference != new.inference
        || old.mirror_source != new.mirror_source
    {
        out.push("keyform settings".to_owned());
    }
}

/// `pairs` のうち `differs` なものの数を「`what` in N keyforms」で書く (無ければ書かない)。
fn in_keyforms<T>(
    out: &mut Vec<String>,
    pairs: &[(&T, &T)],
    what: &str,
    differs: impl Fn(&T, &T) -> bool,
) {
    let n = pairs.iter().filter(|(a, b)| differs(a, b)).count();
    if n > 0 {
        out.push(format!("{what} in {}", count(n, "keyform", "keyforms")));
    }
}

/// 違う要素の中身の違い。どこが違うか書き出せなかったら `other settings`。
fn or_other(mut out: Vec<String>) -> Vec<String> {
    if out.is_empty() {
        out.push("other settings".to_owned());
    }
    out
}

fn part_changes(old: &Part, new: &Part) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes::<String, _>(
        &mut out,
        (&old.name, &new.name),
        None,
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        in_keyforms(out, pairs, "opacity", |a, b| a != b);
    });
    or_other(out)
}

fn deformer_changes(old: &Deformer, new: &Deformer) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes(
        &mut out,
        (&old.name, &new.name),
        Some((&old.part, &new.part)),
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    match (&old.kind, &new.kind) {
        (DeformerKind::Warp(a), DeformerKind::Warp(b)) => {
            let same_grid = (a.rows, a.cols) == (b.rows, b.cols);
            if !same_grid {
                out.push(format!(
                    "grid {}×{} → {}×{}",
                    a.cols, a.rows, b.cols, b.rows
                ));
            }
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                if same_grid {
                    in_keyforms(out, pairs, "control points", |a, b| a != b);
                }
            });
        }
        (DeformerKind::Rotation(a), DeformerKind::Rotation(b)) => {
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                in_keyforms(out, pairs, "origin or angle", |a, b| a != b);
            });
        }
        (a, b) => out.push(format!("type {} → {}", kind_name(a), kind_name(b))),
    }
    or_other(out)
}

fn kind_name(kind: &DeformerKind) -> &'static str {
    match kind {
        DeformerKind::Warp(_) => "warp",
        DeformerKind::Rotation(_) => "rotation",
        DeformerKind::Unknown => "unknown",
    }
}

/// グルーの中身の違い (両端のメッシュ・頂点の組・強さのキーフォーム)。
fn glue_changes(old: &Glue, new: &Glue) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if (&old.mesh_a, &old.mesh_b) != (&new.mesh_a, &new.mesh_b) {
        out.push(format!(
            "meshes {} + {} → {} + {}",
            old.mesh_a, old.mesh_b, new.mesh_a, new.mesh_b
        ));
    }
    if old.pairs.len() == new.pairs.len() {
        let n = old
            .pairs
            .iter()
            .zip(&new.pairs)
            .filter(|(a, b)| a != b)
            .count();
        if n > 0 {
            out.push(format!("{} of vertices", count(n, "pair", "pairs")));
        }
    } else {
        out.push(format!(
            "vertex pairs {} → {}",
            old.pairs.len(),
            new.pairs.len()
        ));
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        in_keyforms(out, pairs, "intensity", |a, b| a != b);
    });
    or_other(out)
}

/// 物理のリグの中身の違い (入力・出力・粒子・正規化・衝突体・座標系の基準)。
fn rig_changes(old: &PhysicsRig, new: &PhysicsRig) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(format!("name {} → {}", old.name, new.name));
    }
    listed(&mut out, "inputs", &old.inputs, &new.inputs);
    listed(&mut out, "outputs", &old.outputs, &new.outputs);
    if old.particles.len() == new.particles.len() {
        let n = old
            .particles
            .iter()
            .zip(&new.particles)
            .filter(|(a, b)| a != b)
            .count();
        if n > 0 {
            out.push(format!("settings of {}", count(n, "particle", "particles")));
        }
    } else {
        out.push(format!(
            "particles {} → {}",
            old.particles.len(),
            new.particles.len()
        ));
    }
    if old.normalization != new.normalization {
        out.push("normalization".to_owned());
    }
    let colliders = |c: &[ColliderId]| c.iter().cloned().collect::<BTreeSet<ColliderId>>();
    let (a, b) = (colliders(&old.colliders), colliders(&new.colliders));
    if a != b {
        let added: Vec<&str> = b.difference(&a).map(ColliderId::as_str).collect();
        let removed: Vec<&str> = a.difference(&b).map(ColliderId::as_str).collect();
        if let Some(details) = details(&added, &removed, &[], "changed") {
            out.push(format!("colliders ({details})"));
        }
    }
    if old.frame != new.frame {
        out.push(format!(
            "frame {} → {}",
            or_none(old.frame.as_ref()),
            or_none(new.frame.as_ref())
        ));
    }
    or_other(out)
}

/// 並び (入力・出力) の違い。数が違えば数、同じなら違うものの数。
fn listed<T: PartialEq>(out: &mut Vec<String>, what: &str, old: &[T], new: &[T]) {
    if old.len() == new.len() {
        let n = old.iter().zip(new).filter(|(a, b)| a != b).count();
        if n > 0 {
            out.push(format!("{n} {what}"));
        }
    } else {
        out.push(format!("{what} {} → {}", old.len(), new.len()));
    }
}

/// 衝突体の中身の違い (名前・取り付け先・形・位置・半径)。
fn collider_changes(old: &PhysicsCollider, new: &PhysicsCollider) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(format!("name {} → {}", old.name, new.name));
    }
    if old.attach != new.attach {
        out.push(format!(
            "attached to {} → {}",
            or_none(old.attach.as_ref()),
            or_none(new.attach.as_ref())
        ));
    }
    match (&old.shape, &new.shape) {
        (
            ColliderShape::Circle {
                center: c0,
                radius: r0,
            },
            ColliderShape::Circle {
                center: c1,
                radius: r1,
            },
        ) => {
            if c0 != c1 {
                out.push("position".to_owned());
            }
            radius(&mut out, *r0, *r1);
        }
        (
            ColliderShape::Capsule {
                a: a0,
                b: b0,
                radius: r0,
            },
            ColliderShape::Capsule {
                a: a1,
                b: b1,
                radius: r1,
            },
        ) => {
            if (a0, b0) != (a1, b1) {
                out.push("position".to_owned());
            }
            radius(&mut out, *r0, *r1);
        }
        (a, b) => out.push(format!("shape {} → {}", shape_name(a), shape_name(b))),
    }
    or_other(out)
}

fn radius(out: &mut Vec<String>, old: f32, new: f32) {
    if old != new {
        out.push(format!("radius {} → {}", number(old), number(new)));
    }
}

fn shape_name(shape: &ColliderShape) -> &'static str {
    match shape {
        ColliderShape::Circle { .. } => "circle",
        ColliderShape::Capsule { .. } => "capsule",
        ColliderShape::Unknown => "unknown",
    }
}

/// メッシュの中身の違い (頂点・三角形・キーフォーム・描画順・テクスチャなど)。テクスチャは名前で比べる。
/// 描画順は `map` で写して書く。
fn mesh_changes(
    (old, old_texture): (&Mesh, Option<&str>),
    (new, new_texture): (&Mesh, Option<&str>),
    map: &dyn Fn(f32) -> f32,
) -> Vec<String> {
    let same = old_texture == new_texture && {
        let mut new = new.clone();
        new.texture = old.texture;
        *old == new
    };
    if same {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes(
        &mut out,
        (&old.name, &new.name),
        Some((&old.part, &new.part)),
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    if old_texture != new_texture {
        out.push(format!(
            "texture {} → {}",
            old_texture.unwrap_or("none"),
            new_texture.unwrap_or("none")
        ));
    }
    let same_vertices = old.uvs.len() == new.uvs.len();
    if same_vertices {
        let n = old.uvs.iter().zip(&new.uvs).filter(|(a, b)| a != b).count();
        if n > 0 {
            out.push(format!("UVs of {}", count(n, "vertex", "vertices")));
        }
    } else {
        out.push(format!("vertices {} → {}", old.uvs.len(), new.uvs.len()));
    }
    if old.indices != new.indices {
        out.push(if old.indices.len() == new.indices.len() {
            "triangles".to_owned()
        } else {
            format!(
                "triangles {} → {}",
                old.indices.len() / 3,
                new.indices.len() / 3
            )
        });
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        if same_vertices {
            in_keyforms(out, pairs, "vertex positions", |a, b| {
                a.positions != b.positions
            });
        }
        draw_order(out, pairs, map);
        in_keyforms(out, pairs, "opacity or colors", |a, b| {
            a.opacity != b.opacity
                || a.multiply_color != b.multiply_color
                || a.screen_color != b.screen_color
                || a.mask_strength != b.mask_strength
        });
    });
    if old.blend != new.blend
        || old.culling != new.culling
        || old.masks != new.masks
        || old.mask_mode != new.mask_mode
        || old.mask_only != new.mask_only
    {
        out.push("blending or masks".to_owned());
    }
    if old.mirror != new.mirror
        || old.mirror_axis != new.mirror_axis
        || old.depth != new.depth
        || old.lit != new.lit
    {
        out.push("other settings".to_owned());
    }
    or_other(out)
}

/// 描画順の違い。どのキーフォームでも同じ値なら `draw order 500 → 520`、そうでなければ変わったキーフォームの数。
fn draw_order(
    out: &mut Vec<String>,
    pairs: &[(&MeshKeyform, &MeshKeyform)],
    map: &dyn Fn(f32) -> f32,
) {
    let n = pairs
        .iter()
        .filter(|(a, b)| a.draw_order != b.draw_order)
        .count();
    if n == 0 {
        return;
    }
    let single = |values: Vec<f32>| {
        let first = values[0];
        values.iter().all(|&v| v == first).then_some(first)
    };
    let olds = pairs.iter().map(|(a, _)| a.draw_order).collect();
    let news = pairs.iter().map(|(_, b)| b.draw_order).collect();
    match (single(olds), single(news)) {
        (Some(a), Some(b)) => out.push(format!(
            "draw order {} → {}",
            number(map(a)),
            number(map(b))
        )),
        _ => out.push(format!("draw order in {}", count(n, "keyform", "keyforms"))),
    }
}

/// 表示用の数 (写したときの丸めのずれを出さないように、小数 2 桁まで)。
fn number(value: f32) -> String {
    let rounded = (f64::from(value) * 100.0).round() / 100.0;
    format!("{rounded}")
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
                if module.meshes.iter().any(|m| m.id.as_str() == id) {
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
                if let Some(glue) = module.glues.iter().find(|g| g.id.as_str() == id) {
                    push(glue.mesh_a.as_str());
                    push(glue.mesh_b.as_str());
                }
            }
            DifferenceKind::PhysicsRig => {
                let Some(rig) = module.physics.rigs.iter().find(|r| r.id.as_str() == id) else {
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
                        PhysicsTarget::Parameter { .. } | PhysicsTarget::Unknown => {}
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
            .filter(|e| e.part.as_str() == id)
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
            .find(|p| p.id.as_str() == id)
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
                .find(|d| d.id.as_str() == id)
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
            added.push(id.clone());
            items.push(item(DifferenceKind::PoseGroup, &id, "added".to_owned()));
            continue;
        };
        matched[k] = true;
        let what = group_changes(&old.groups[k], group);
        if !what.is_empty() {
            changed.push(id.clone());
            items.push(item(DifferenceKind::PoseGroup, &id, what.join("; ")));
        }
    }
    let removed: Vec<String> = old
        .groups
        .iter()
        .zip(&matched)
        .filter(|(_, m)| !**m)
        .map(|(g, _)| group_label(g))
        .collect();
    items.extend(
        removed
            .iter()
            .map(|id| item(DifferenceKind::PoseGroup, id, REMOVED.to_owned())),
    );
    if let Some(details) = details(&strs(&added), &strs(&removed), &strs(&changed), "changed") {
        lines.push(ModuleDifference {
            text: format!(
                "Pose groups: {} → {} ({details})",
                old.groups.len(),
                new.groups.len()
            ),
            items,
        });
    }

    elements(
        lines,
        "Pose follows",
        DifferenceKind::PoseFollow,
        &keyed(&old.links, |e| e.part.to_string()),
        &keyed(&new.links, |e| e.part.to_string()),
        |a: &PoseEntry, b: &PoseEntry| {
            let mut out = Vec::new();
            linked(&mut out, "links", &a.links, &b.links);
            out
        },
    );
    if lines.len() == before {
        lines.push(line("Pose: order of groups changed".to_owned()));
    }
}

fn strs(ids: &[String]) -> Vec<&str> {
    ids.iter().map(String::as_str).collect()
}

/// 同じとみなしたグループの中身の違い (要素の並びと、要素ごとのリンク)。
fn group_changes(old: &PoseGroup, new: &PoseGroup) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    let (a, b) = (group_label(old), group_label(new));
    if a != b {
        out.push(format!("entries {a} → {b}"));
    }
    for entry in &new.entries {
        if let Some(before) = old.entries.iter().find(|e| e.part == entry.part) {
            linked(
                &mut out,
                &format!("links of {}", entry.part),
                &before.links,
                &entry.links,
            );
        }
    }
    or_other(out)
}

/// リンクのパーツの違い (足した・除いた。並びだけなら `reordered`)。
fn linked(out: &mut Vec<String>, what: &str, old: &[PartId], new: &[PartId]) {
    if old == new {
        return;
    }
    let added: Vec<&str> = new
        .iter()
        .filter(|p| !old.contains(p))
        .map(|p| p.as_str())
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .filter(|p| !new.contains(p))
        .map(|p| p.as_str())
        .collect();
    match details(&added, &removed, &[], "changed") {
        Some(details) => out.push(format!("{what} ({details})")),
        None => out.push(format!("{what} reordered")),
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
    let added: Vec<&str> = module
        .textures
        .iter()
        .filter(|t| find(&t.name).is_none())
        .map(|t| t.name.as_str())
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .filter(|(_, t)| !module.textures.iter().any(|n| n.name == t.name))
        .map(|(_, t)| t.name.as_str())
        .collect();
    let file = view.file;
    let changed: Vec<&str> = module
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
        .map(|t| t.name.as_str())
        .collect();
    if let Some(details) = details(&added, &removed, &changed, "different image") {
        lines.push(line(format!(
            "Textures: {} → {} ({details})",
            old.len(),
            module.textures.len()
        )));
    }
    if view.unresolved {
        lines.push(line(
            "Some meshes in the file's texture atlases could not be matched to your current textures, so they count as changed"
                .to_owned(),
        ));
    }
}

// ---- アトラス化したファイルを戻す ----

/// 比べる側のファイル。テクスチャがアトラス化してあれば、メッシュのテクスチャと UV を今の編集のテクスチャに戻したもの。
struct FileView<'a> {
    file: &'a ModuleFile,
    /// ファイルのモジュール。戻せたメッシュは今の編集のテクスチャを指し、そのテクスチャは `textures` の後ろに足してある。
    module: Module,
    /// `module.textures` の各番号の画像。
    images: Vec<FileImage>,
    /// アトラスの中で、今の編集のテクスチャに戻せなかったメッシュがあるか (そのアトラスはテクスチャとして残る)。
    unresolved: bool,
}

#[derive(Debug, Clone, Copy)]
enum FileImage {
    /// ファイルの番号 `.0` のテクスチャ (と法線マップ)。
    File(usize),
    /// アトラスから見つけた今の編集のテクスチャ。`same` は画素が同じか。
    Packed { same: bool },
    /// 使うメッシュをすべて今の編集のテクスチャに戻したアトラス (テクスチャとしては比べない)。
    Hidden,
}

impl<'a> FileView<'a> {
    fn new(file: &'a ModuleFile, module: &Module, images: &Textures) -> Self {
        let old = &file.module;
        let mut view = Self {
            file,
            module: old.clone(),
            images: (0..old.textures.len()).map(FileImage::File).collect(),
            unresolved: false,
        };
        // 今の編集に同じ名前のテクスチャがあれば、アトラスではなくそのテクスチャとして比べる。
        let atlases: BTreeSet<usize> = old
            .textures
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.name.starts_with(ATLAS_NAME_PREFIX)
                    && t.normal_map.is_none()
                    && !module.textures.iter().any(|n| n.name == t.name)
            })
            .map(|(i, _)| i)
            .collect();
        if atlases.is_empty() {
            return view;
        }
        // 戻し先の候補は、ファイルにアトラスでないテクスチャとして同じ名前が無い今の編集のテクスチャ。
        let candidates: Vec<usize> = (0..module.textures.len())
            .filter(|&t| {
                !old.textures
                    .iter()
                    .any(|o| o.name == module.textures[t].name)
            })
            .collect();
        let mut finder = Finder::new(file, module, images);
        let found = finder.resolve(&atlases, &candidates);

        // 戻せたテクスチャを後ろに足す。
        let mut index: BTreeMap<usize, u32> = BTreeMap::new();
        let mut same: BTreeMap<usize, bool> = BTreeMap::new();
        for &(t, _, place_same) in found.iter().flatten() {
            *same.entry(t).or_insert(true) &= place_same;
        }
        for (&t, &same) in &same {
            index.insert(
                t,
                u32::try_from(view.module.textures.len()).unwrap_or(u32::MAX),
            );
            view.module.textures.push(module.textures[t].clone());
            view.images.push(FileImage::Packed { same });
        }
        // アトラスを使うメッシュのうち、戻せなかったものが無いアトラスは隠す。
        for &a in &atlases {
            let on_atlas = |k: &usize| old.meshes[*k].texture as usize == a;
            let meshes: Vec<usize> = (0..old.meshes.len()).filter(on_atlas).collect();
            if meshes.iter().all(|&k| found[k].is_some()) && !meshes.is_empty() {
                view.images[a] = FileImage::Hidden;
            } else if !meshes.is_empty() {
                view.unresolved = true;
            }
        }
        for (mesh, found) in view.module.meshes.iter_mut().zip(&found) {
            let Some((t, place, _)) = found else {
                continue;
            };
            let current = module.meshes.iter().find(|m| m.id == mesh.id);
            mesh.texture = index[t];
            for (i, uv) in mesh.uvs.iter_mut().enumerate() {
                // 今の編集の UV を写すと同じ値になるなら、その UV (戻したときの丸めのずれを残さない)。
                *uv = match current.and_then(|c| c.uvs.get(i)) {
                    Some(&c) if place.map(c) == *uv => c,
                    _ => place.unmap(*uv),
                };
            }
        }
        view
    }
}

/// 画素を RGBA8 に広げた画像。
struct Rgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Rgba {
    fn decode(png: &[u8]) -> Option<Self> {
        let (width, height, pixels) = decode_rgba_png(png)?;
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        &self.pixels[i..i + 4]
    }

    /// `(x, y)` に `image` と同じ画素が並んでいるか。先に `samples` の画素だけを見る。
    fn contains_at(&self, image: &Self, x: u32, y: u32, samples: &[(u32, u32)]) -> bool {
        if x + image.width > self.width || y + image.height > self.height {
            return false;
        }
        let row = image.width as usize * 4;
        samples
            .iter()
            .all(|&(sx, sy)| self.pixel(x + sx, y + sy) == image.pixel(sx, sy))
            && (0..image.height).all(|iy| {
                let at = ((y + iy) as usize * self.width as usize + x as usize) * 4;
                let from = iy as usize * row;
                self.pixels[at..at + row] == image.pixels[from..from + row]
            })
    }
}

/// 画素で探すときに試す位置の数の上限 (大きな画像で時間がかかりすぎないように)。
const SEARCH_LIMIT: usize = 1 << 16;
/// 画素で探すとき、先に見る不透明な画素の数。
const SAMPLE_COUNT: usize = 32;

/// アトラスの中で今の編集の画像を置いた場所を探す。
struct Finder<'a> {
    file: &'a ModuleFile,
    module: &'a Module,
    images: &'a Textures,
    atlases: BTreeMap<usize, Option<Rgba>>,
    textures: BTreeMap<usize, Option<Rgba>>,
}

/// ファイルのメッシュ 1 つの戻し先 (今の編集のテクスチャの番号・置いた場所・画素が同じか)。
type Found = Option<(usize, Placement, bool)>;

impl<'a> Finder<'a> {
    fn new(file: &'a ModuleFile, module: &'a Module, images: &'a Textures) -> Self {
        Self {
            file,
            module,
            images,
            atlases: BTreeMap::new(),
            textures: BTreeMap::new(),
        }
    }

    /// アトラス `atlases` を使うファイルのメッシュごとの戻し先 (並びはファイルのメッシュと同じ)。
    ///
    /// 1. 今の編集に同じ ID のメッシュがあれば、そのテクスチャに戻す。場所は UV の組から推し量り (多数決)、
    ///    画素で確かめる。確かめられず、多数決も過半数に届かなければ、画素で探す。
    /// 2. 今の編集に無いメッシュ (除いたもの) は、見つけた場所の中にあればそのテクスチャ、無ければ
    ///    まだ見つけていない今の編集のテクスチャを画素で探す。
    fn resolve(&mut self, atlases: &BTreeSet<usize>, candidates: &[usize]) -> Vec<Found> {
        let old = &self.file.module;
        let mut found: Vec<Found> = vec![None; old.meshes.len()];
        let mut groups: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
        let mut orphans = Vec::new();
        for (k, mesh) in old.meshes.iter().enumerate() {
            let a = mesh.texture as usize;
            if !atlases.contains(&a) {
                continue;
            }
            match self.module.meshes.iter().find(|m| m.id == mesh.id) {
                Some(current) if candidates.contains(&(current.texture as usize)) => {
                    groups
                        .entry((a, current.texture as usize))
                        .or_default()
                        .push(k);
                }
                Some(_) => {}
                None => orphans.push(k),
            }
        }
        let mut placed: Vec<(usize, usize, Placement, bool)> = Vec::new();
        for ((a, t), members) in groups {
            if let Some((place, same)) = self.locate(a, t, &members) {
                for &k in &members {
                    found[k] = Some((t, place, same));
                }
                placed.push((a, t, place, same));
            }
        }
        for k in orphans {
            let a = old.meshes[k].texture as usize;
            let Some(bounds) = self.bounds(a, &[k]) else {
                continue;
            };
            let inside = placed.iter().find(|(pa, _, place, _)| {
                *pa == a
                    && f64::from(place.x) <= bounds.0 + EPSILON
                    && f64::from(place.y) <= bounds.1 + EPSILON
                    && f64::from(place.x + place.width) >= bounds.2 - EPSILON
                    && f64::from(place.y + place.height) >= bounds.3 - EPSILON
            });
            if let Some(&(_, t, place, same)) = inside {
                found[k] = Some((t, place, same));
                continue;
            }
            for &t in candidates {
                if placed.iter().any(|(_, pt, _, _)| *pt == t) {
                    continue;
                }
                if let Some(place) = self.search(a, t, bounds) {
                    found[k] = Some((t, place, true));
                    placed.push((a, t, place, true));
                    break;
                }
            }
        }
        found
    }

    /// ファイルのメッシュ `members` (今の編集ではテクスチャ `t` を使う) がアトラス `a` で使う場所と、画素が同じか。
    fn locate(&mut self, a: usize, t: usize, members: &[usize]) -> Option<(Placement, bool)> {
        let old = &self.file.module;
        let (w, h) = self.texture_size(t);
        let atlas = &old.textures[a];
        let (aw, ah) = (atlas.width, atlas.height);
        // UV の組ごとに、置いた左上の画素を推し量る。
        let mut votes: BTreeMap<(i64, i64), usize> = BTreeMap::new();
        let mut total = 0;
        for &k in members {
            let mesh = &old.meshes[k];
            let Some(current) = self.module.meshes.iter().find(|m| m.id == mesh.id) else {
                continue;
            };
            if current.uvs.len() != mesh.uvs.len() {
                continue;
            }
            for (f, c) in mesh.uvs.iter().zip(&current.uvs) {
                let offset = |f: f32, c: f32, size: u32, total: u32| {
                    (f64::from(f) * f64::from(total) - f64::from(c) * f64::from(size)).round()
                        as i64
                };
                *votes
                    .entry((offset(f.x, c.x, w, aw), offset(f.y, c.y, h, ah)))
                    .or_default() += 1;
                total += 1;
            }
        }
        // 票の多い順 (同数なら左上から)。
        let best = votes
            .iter()
            .max_by(|x, y| x.1.cmp(y.1).then(y.0.cmp(x.0)))
            .map(|(&xy, &n)| (xy, n));
        if let Some(((x, y), n)) = best
            && let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y))
            && u64::from(x) + u64::from(w) <= u64::from(aw)
            && u64::from(y) + u64::from(h) <= u64::from(ah)
        {
            let place = placement(x, y, (w, h), (aw, ah));
            let same = self.pixels_match(a, t, x, y);
            if same || n * 2 > total {
                return Some((place, same));
            }
        }
        let bounds = self.bounds(a, members)?;
        self.search(a, t, bounds).map(|place| (place, true))
    }

    /// アトラス `a` の上で、ファイルのメッシュ `members` の UV が占める範囲 (画素。左・上・右・下)。
    fn bounds(&self, a: usize, members: &[usize]) -> Option<(f64, f64, f64, f64)> {
        let old = &self.file.module;
        let atlas = &old.textures[a];
        let uvs: Vec<Vec2> = members
            .iter()
            .flat_map(|&k| old.meshes[k].uvs.iter().copied())
            .collect();
        if uvs.is_empty() || uvs.iter().any(|uv| !uv.is_finite()) {
            return None;
        }
        let mut bounds = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for uv in uvs {
            let (x, y) = (
                f64::from(uv.x) * f64::from(atlas.width),
                f64::from(uv.y) * f64::from(atlas.height),
            );
            bounds = (
                bounds.0.min(x),
                bounds.1.min(y),
                bounds.2.max(x),
                bounds.3.max(y),
            );
        }
        Some(bounds)
    }

    /// アトラス `a` の中で、範囲 `bounds` を覆う位置に今の編集の画像 `t` と同じ画素が並ぶ場所を探す。
    fn search(&mut self, a: usize, t: usize, bounds: (f64, f64, f64, f64)) -> Option<Placement> {
        self.decode(a, t);
        let atlas = self.atlases.get(&a)?.as_ref()?;
        let image = self.textures.get(&t)?.as_ref()?;
        if image.width > atlas.width || image.height > atlas.height {
            return None;
        }
        let range = |min: f64, max: f64, size: u32, total: u32| {
            let from = (max - f64::from(size) - EPSILON).ceil().max(0.0);
            let to = (min + EPSILON).floor().min(f64::from(total - size));
            (from <= to).then_some((from as u32, to as u32))
        };
        let (x0, x1) = range(bounds.0, bounds.2, image.width, atlas.width)?;
        let (y0, y1) = range(bounds.1, bounds.3, image.height, atlas.height)?;
        let tries = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
        if tries > SEARCH_LIMIT {
            return None;
        }
        // 透明な画素はアトラスの余白にもあるので、不透明な画素を先に見る。すべて透明な画像は探さない。
        let opaque: Vec<(u32, u32)> = (0..image.height)
            .flat_map(|y| (0..image.width).map(move |x| (x, y)))
            .filter(|&(x, y)| image.pixel(x, y)[3] != 0)
            .collect();
        if opaque.is_empty() {
            return None;
        }
        let step = opaque.len().div_ceil(SAMPLE_COUNT);
        let samples: Vec<(u32, u32)> = opaque.into_iter().step_by(step).collect();
        let (w, h) = (image.width, image.height);
        (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .find(|&(x, y)| atlas.contains_at(image, x, y, &samples))
            .map(|(x, y)| placement(x, y, (w, h), (atlas.width, atlas.height)))
    }

    /// アトラス `a` の `(x, y)` に今の編集の画像 `t` と同じ画素が並ぶか。
    fn pixels_match(&mut self, a: usize, t: usize, x: u32, y: u32) -> bool {
        self.decode(a, t);
        match (self.atlases.get(&a), self.textures.get(&t)) {
            (Some(Some(atlas)), Some(Some(image))) => atlas.contains_at(image, x, y, &[]),
            _ => false,
        }
    }

    /// 今の編集の画像 `t` の大きさ (読めなければテクスチャの参照の大きさ)。
    fn texture_size(&mut self, t: usize) -> (u32, u32) {
        self.decode_texture(t);
        match self.textures.get(&t) {
            Some(Some(image)) => (image.width, image.height),
            _ => {
                let texture = &self.module.textures[t];
                (texture.width, texture.height)
            }
        }
    }

    fn decode(&mut self, a: usize, t: usize) {
        let file = self.file;
        self.atlases.entry(a).or_insert_with(|| {
            file.textures
                .get(a)
                .and_then(|data| Rgba::decode(&data.bytes))
        });
        self.decode_texture(t);
    }

    fn decode_texture(&mut self, t: usize) {
        let (module, images) = (self.module, self.images);
        self.textures.entry(t).or_insert_with(|| {
            images
                .get(&module.textures[t].name)
                .and_then(|png| Rgba::decode(png))
        });
    }
}

/// UV から求めた画素の位置の丸めの許し (画素)。
const EPSILON: f64 = 1e-3;

fn placement(x: u32, y: u32, (width, height): (u32, u32), (aw, ah): (u32, u32)) -> Placement {
    Placement {
        x,
        y,
        width,
        height,
        atlas_width: aw,
        atlas_height: ah,
    }
}
