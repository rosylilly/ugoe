//! モジュールのプロジェクトの今の編集と、前に書き出した `.ugoemod` の違い (T0654)。
//! エディタのプレビューの着せ替えの一覧で、同じ名前のファイルの行に文字で出す (`docs/editor.md#プレビューの着せ替え`)。
//!
//! 見るのは要素 (パーツ・デフォーマ・メッシュ・グルー) の ID と中身、テクスチャ (名前と画像)、条件、使うパラメータ、
//! 物理のリグと衝突体、ポーズ。違いは英語の 1 行ずつ (エディタの画面にそのまま出す) で、「ファイル → 今の編集」の向きに書く。

use std::collections::BTreeSet;
use std::fmt::Display;

use ugoe_format::ModuleFile;
use ugoe_model::{Mesh, Module, TextureRef};

use crate::atlas::ATLAS_NAME_PREFIX;
use crate::project::{Project, ProjectError, checked_module};
use crate::texture::Textures;

/// モジュールのプロジェクトの今の編集と、書き出した `.ugoemod` のバイト列 `bytes` の違い。同じなら空。
/// 今の編集が書き出しの検査に通らなければ、そのエラー。`bytes` が読めなければ [`ProjectError::Export`]。
pub fn compare_with_module_file(
    project: &Project,
    bytes: &[u8],
) -> Result<Vec<String>, ProjectError> {
    let file = ugoe_format::read_module(bytes).map_err(ProjectError::Export)?;
    let (module, images, _) = checked_module(project, None)?;
    Ok(module_differences(&file, &module, &images))
}

/// 書き出したファイル `file` から今の編集 (`module` と画像 `images`) への違い。同じなら空。
pub fn module_differences(file: &ModuleFile, module: &Module, images: &Textures) -> Vec<String> {
    let old = &file.module;
    let mut lines = Vec::new();
    if old.name != module.name {
        lines.push(format!("Name: {} → {}", old.name, module.name));
    }
    if old.slot != module.slot {
        lines.push(format!("Slot: {} → {}", old.slot, module.slot));
    }

    elements(
        &mut lines,
        "Parts",
        &keyed(&old.parts, |p| p.id.to_string()),
        &keyed(&module.parts, |p| p.id.to_string()),
        PartialEq::eq,
    );
    elements(
        &mut lines,
        "Deformers",
        &keyed(&old.deformers, |d| d.id.to_string()),
        &keyed(&module.deformers, |d| d.id.to_string()),
        PartialEq::eq,
    );
    // メッシュのテクスチャは番号なので、名前で比べる (テクスチャの並びが変わっただけなら同じ)。
    elements(
        &mut lines,
        "Meshes",
        &keyed(&old.meshes, |m| m.id.to_string()),
        &keyed(&module.meshes, |m| m.id.to_string()),
        |a: &Mesh, b: &Mesh| {
            texture_name(old, a) == texture_name(module, b) && {
                let mut b = b.clone();
                b.texture = a.texture;
                *a == b
            }
        },
    );
    elements(
        &mut lines,
        "Glues",
        &keyed(&old.glues, |g| g.id.clone()),
        &keyed(&module.glues, |g| g.id.clone()),
        PartialEq::eq,
    );
    textures(&mut lines, file, module, images);

    let requires = (&old.requires, &module.requires);
    if requires.0.avatar_level != requires.1.avatar_level {
        let level = |l: Option<u8>| l.map_or_else(|| "none".to_owned(), |l| l.to_string());
        lines.push(format!(
            "Required avatar level: {} → {}",
            level(requires.0.avatar_level),
            level(requires.1.avatar_level)
        ));
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
        &keyed(&old.physics.rigs, |r| r.id.clone()),
        &keyed(&module.physics.rigs, |r| r.id.clone()),
        PartialEq::eq,
    );
    elements(
        &mut lines,
        "Colliders",
        &keyed(&old.physics.colliders, |c| c.id.clone()),
        &keyed(&module.physics.colliders, |c| c.id.clone()),
        PartialEq::eq,
    );
    if old.pose != module.pose {
        lines.push("Pose groups or follows changed".to_owned());
    }
    lines
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
fn elements<T>(
    lines: &mut Vec<String>,
    label: &str,
    old: &[(String, &T)],
    new: &[(String, &T)],
    same: impl Fn(&T, &T) -> bool,
) {
    let added: Vec<&str> = new
        .iter()
        .filter(|(id, _)| find(old, id).is_none())
        .map(|(id, _)| id.as_str())
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .filter(|(id, _)| find(new, id).is_none())
        .map(|(id, _)| id.as_str())
        .collect();
    let changed: Vec<&str> = new
        .iter()
        .filter(|(id, item)| find(old, id).is_some_and(|o| !same(o, item)))
        .map(|(id, _)| id.as_str())
        .collect();
    if let Some(details) = details(&added, &removed, &changed, "changed") {
        lines.push(format!(
            "{label}: {} → {} ({details})",
            old.len(),
            new.len()
        ));
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
fn names<T: Display + Ord>(lines: &mut Vec<String>, label: &str, old: &[T], new: &[T]) {
    let old: BTreeSet<String> = old.iter().map(ToString::to_string).collect();
    let new: BTreeSet<String> = new.iter().map(ToString::to_string).collect();
    let added: Vec<&str> = new.difference(&old).map(String::as_str).collect();
    let removed: Vec<&str> = old.difference(&new).map(String::as_str).collect();
    if let Some(details) = details(&added, &removed, &[], "changed") {
        lines.push(format!("{label}: {details}"));
    }
}

/// テクスチャ (名前・法線マップの名前・画像) の違い。
fn textures(lines: &mut Vec<String>, file: &ModuleFile, module: &Module, images: &Textures) {
    let old = &file.module.textures;
    let find = |name: &str| old.iter().position(|t| t.name == name);
    let added: Vec<&str> = module
        .textures
        .iter()
        .filter(|t| find(&t.name).is_none())
        .map(|t| t.name.as_str())
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .filter(|t| !module.textures.iter().any(|n| n.name == t.name))
        .map(|t| t.name.as_str())
        .collect();
    let changed: Vec<&str> = module
        .textures
        .iter()
        .filter(|t| {
            find(&t.name).is_some_and(|i| {
                let image = |name: &str| images.get(name).map(|png| &png[..]);
                let normal = file.normal_map(i).map(|d| &d.bytes[..]);
                old[i] != **t
                    || image(&t.name) != file.textures.get(i).map(|d| &d.bytes[..])
                    || t.normal_map.as_deref().and_then(image) != normal
            })
        })
        .map(|t| t.name.as_str())
        .collect();
    if let Some(details) = details(&added, &removed, &changed, "different image") {
        lines.push(format!(
            "Textures: {} → {} ({details})",
            old.len(),
            module.textures.len()
        ));
        let atlased =
            |list: &[TextureRef]| list.iter().any(|t| t.name.starts_with(ATLAS_NAME_PREFIX));
        if atlased(old) && !atlased(&module.textures) {
            lines.push(
                "The file's textures are packed into atlases, so the meshes that use them count as changed"
                    .to_owned(),
            );
        }
    }
}
