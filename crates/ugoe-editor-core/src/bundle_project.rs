//! `.ugoe` バンドルと `.ugoemod` モジュールから、エディタで開けるプロジェクトを作る。
//! 着せ替えの見本 (`ugoe sample dress-up --project` と、エディタの File > Open Sample…) が使う (T0712・T0715・T0655)。
//!
//! 本体は `Project::new` で保存し、モジュールは保存した本体から `Project::new_module` (エディタの「New Module…」と同じ)
//! で作って中身を入れる。書いたモジュールのプロジェクトを書き出す (`export_module`) と、元の `.ugoemod` と同じになる。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ugoe_format::{Bundle, ModuleFile, TextureData, TextureFormat};
use ugoe_model::TextureRef;

use crate::project::{Project, ProjectError, ProjectMeta};
use crate::texture::Textures;

/// バンドルの中身 (モデル・テクスチャと法線マップ・物理・モーション・表情) を持つ本体のプロジェクト。
/// プロジェクトの名前はモデルの名前。バンドルのメタデータ (`Bundle::meta`) は入らない。
pub fn project_from_bundle(bundle: &Bundle) -> Result<Project, ProjectError> {
    let model = &bundle.model;
    let textures = images(
        &model.textures,
        &bundle.textures,
        &bundle.normal_maps,
        |name| ProjectError::UnsupportedTexture { name },
    )?;
    let mut project = Project::new(
        ProjectMeta::new(model.meta.name.clone()),
        model.clone(),
        textures,
    );
    project.physics = bundle.physics.clone();
    project.motions = bundle.motions.clone();
    project.expressions = bundle.expressions.clone();
    Ok(project)
}

/// 保存した本体のプロジェクト `base` (`.ugoeproj` のディレクトリか `.ugoe`) に付ける、モジュール `file` のプロジェクト。
/// 本体への参照は `base` のまま (保存するときに保存先からの相対パスにする)。
pub fn module_project(base: &Path, file: &ModuleFile) -> Result<Project, ProjectError> {
    let module = &file.module;
    let empty = Project::new_module(&module.name, base, &module.slot)?;
    let context = empty
        .module
        .as_ref()
        .ok_or(ProjectError::NotAModule)?
        .reconfigured(&module.slot, module.name.clone(), module.requires.clone())?;
    let textures = images(
        &module.textures,
        &file.textures,
        &file.normal_maps,
        |name| ProjectError::UnsupportedTexture { name },
    )?;
    let workspace = context.workspace(module, &textures)?;
    Ok(Project {
        model: workspace.model,
        textures: workspace.textures,
        physics: workspace.physics,
        module: Some(Arc::new(context)),
        ..empty
    })
}

/// 本体 `bundle` とモジュール `modules` を、プロジェクトとして `dir` に並べて書く: 本体は `base_name`
/// (例: `dress-up.ugoeproj`)、モジュールは [`module_project_name`]。書いたプロジェクトのパスを、本体を先に、
/// モジュールは `modules` の順に返す。
pub fn save_bundle_projects(
    dir: &Path,
    base_name: &str,
    bundle: &Bundle,
    modules: &[ModuleFile],
) -> Result<Vec<PathBuf>, ProjectError> {
    let base_dir = dir.join(base_name);
    project_from_bundle(bundle)?.save(&base_dir)?;
    let mut written = vec![base_dir.clone()];
    for file in modules {
        let path = dir.join(module_project_name(&file.module.name));
        module_project(&base_dir, file)?.save(&path)?;
        written.push(path);
    }
    Ok(written)
}

/// モジュールのプロジェクトのディレクトリ名: 名前の kebab-case に `.ugoeproj` (`LongHair` → `long-hair.ugoeproj`)。
pub fn module_project_name(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    format!("{out}.{}", crate::project::PROJECT_EXTENSION)
}

/// テクスチャの参照と中身 (PNG) を、プロジェクトの画像 (名前 → PNG) にする。法線マップは `normal_map` の名前で入れる
/// (`normal_maps` は `normal_map` を持つテクスチャの順)。PNG でなければ `unsupported` のエラー。
pub(crate) fn images(
    refs: &[TextureRef],
    textures: &[TextureData],
    normal_maps: &[TextureData],
    unsupported: impl Fn(String) -> ProjectError,
) -> Result<Textures, ProjectError> {
    let mut images = Textures::new();
    let mut insert = |name: &str, data: Option<&TextureData>| {
        let data = data.ok_or_else(|| ProjectError::MissingTexture {
            name: name.to_owned(),
        })?;
        if data.format != TextureFormat::Png {
            return Err(unsupported(name.to_owned()));
        }
        images.insert(name.to_owned(), Arc::from(data.bytes.as_slice()));
        Ok(())
    };
    for (i, texture) in refs.iter().enumerate() {
        insert(&texture.name, textures.get(i))?;
    }
    let normal_names = refs.iter().filter_map(|t| t.normal_map.as_deref());
    for (i, name) in normal_names.enumerate() {
        insert(name, normal_maps.get(i))?;
    }
    Ok(images)
}
