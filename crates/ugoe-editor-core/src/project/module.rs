//! モジュールのプロジェクトの読み書きと、その本体の読み込み (`docs/editor.md#モジュールのプロジェクト`)。

use std::path::{Path, PathBuf};

use ugoe_model::SlotId;

use super::paths::{normalize_path, resolve_base};
use super::{
    GITIGNORE, GITIGNORE_CONTENT, MODULE_JSON, PROJECT_JSON, Project, ProjectError, ProjectKind,
    ProjectMeta, ProjectReadLimits, TEXTURE_SUFFIX, TEXTURES_DIR, check_names, image_names,
    read_images, read_json, read_regular_file, read_regular_file_as, with_newline,
};
use crate::module_project::{ColliderReference, ModuleBase, ModuleContext};

impl Project {
    /// モジュールのプロジェクトを読む: `module.json` と画像を読み、本体を読んで作業用のモデルを作る。
    /// `base` があれば `project.json` の参照の代わりにそれを本体にする。`drop_colliders` なら、本体に無い衝突体への
    /// リグの参照を外し、外したものを返す (でなければ空)。本体は `check` で確かめてから読む ([`load_base_with_limits`])。
    /// ファイル 1 つの大きさの上限は `limits` (本体にも同じ上限を使う)。
    pub(super) fn load_module(
        dir: &Path,
        meta: ProjectMeta,
        base: Option<&Path>,
        drop_colliders: bool,
        check: BaseCheck<'_>,
        limits: ProjectReadLimits,
    ) -> Result<(Self, Vec<ColliderReference>), ProjectError> {
        let link = meta.module.clone().ok_or(ProjectError::MissingModuleLink)?;
        let module_path = dir.join(MODULE_JSON);
        let json = read_regular_file(&module_path, limits.max_json)?;
        let json = String::from_utf8(json).map_err(|e| ProjectError::Io {
            path: module_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;
        let mut module =
            ugoe_format::module_from_json(&json).map_err(|source| ProjectError::Format {
                path: module_path.clone(),
                source,
            })?;
        let images = read_images(dir, &module.textures, limits.max_image)?;
        let base_path = match base {
            Some(base) => normalize_path(base),
            None => resolve_base(dir, &link.base),
        };
        let base = load_base_with_limits(&base_path, check, limits)?;
        let context = ModuleContext::new(
            base,
            &link.slot,
            module.name.clone(),
            module.requires.clone(),
        )?;
        let dropped = if drop_colliders {
            context.drop_unavailable_colliders(&mut module)
        } else {
            Vec::new()
        };
        let workspace = context.workspace(&module, &images)?;
        Ok((Self::from_workspace(meta, workspace, context), dropped))
    }

    /// モジュールのプロジェクトで保存するファイル (`module.json` と画像)。
    pub(super) fn module_files(
        &self,
        context: &ModuleContext,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectError> {
        let (module, images) = context.split(&self.model, self.physics.as_ref(), &self.textures)?;
        let json = ugoe_format::module_to_json(&module).map_err(|source| ProjectError::Format {
            path: PathBuf::from(MODULE_JSON),
            source,
        })?;
        let mut files = vec![
            (PathBuf::from(GITIGNORE), GITIGNORE_CONTENT.into()),
            (PathBuf::from(MODULE_JSON), with_newline(json)),
        ];
        let names = image_names(&module.textures);
        check_names("texture", names.iter().copied())?;
        for name in names {
            let png = images
                .get(name)
                .ok_or_else(|| ProjectError::MissingTexture {
                    name: name.to_owned(),
                })?;
            let path = Path::new(TEXTURES_DIR).join(format!("{name}{TEXTURE_SUFFIX}"));
            files.push((path, png.to_vec()));
        }
        Ok(files)
    }
}

/// モジュールのプロジェクト `dir` の `project.json` を読み、`slot` があればスロットを替える
/// (モジュールのプロジェクトでなければ [`ProjectError::NotAModule`])。`max` は `project.json` の大きさの上限。
pub(super) fn module_meta_with(
    dir: &Path,
    slot: Option<&SlotId>,
    max: u64,
) -> Result<ProjectMeta, ProjectError> {
    let mut meta = read_module_meta(dir, max)?;
    if let (Some(link), Some(slot)) = (&mut meta.module, slot) {
        link.slot = slot.clone();
    }
    Ok(meta)
}

fn read_module_meta(dir: &Path, max: u64) -> Result<ProjectMeta, ProjectError> {
    let meta_path = dir.join(PROJECT_JSON);
    if !meta_path.exists() {
        return Err(ProjectError::NotAProject {
            path: dir.to_path_buf(),
        });
    }
    let meta: ProjectMeta = read_json(&meta_path, max)?;
    if meta.kind != ProjectKind::Module {
        return Err(ProjectError::NotAModule);
    }
    Ok(meta)
}

/// モジュールのプロジェクト `dir` が参照する本体 (絶対パスに解決したもの) とスロット。開けなかったときに
/// 直し方を選ぶために使う (本体を読まない)。
pub fn read_module_link(dir: &Path) -> Result<(PathBuf, SlotId), ProjectError> {
    let meta = read_module_meta(dir, ProjectReadLimits::default().max_json)?;
    let link = meta.module.ok_or(ProjectError::MissingModuleLink)?;
    Ok((resolve_base(dir, &link.base), link.slot))
}

/// 本体を読む前に、解決した本体のパス (絶対パス) を確かめる関数 (T0524)。読んでよければ読むパス (正規化した
/// ものなど) を、だめなら理由を返す。本体を開く前に一度だけ呼ぶ。MCP の `--root` が本体を root の下に限るのに使う
/// (editor-core はパスの規則を持たない)。
pub type BaseCheck<'a> = &'a dyn Fn(&Path) -> Result<PathBuf, String>;

/// 確かめずにそのまま読む [`BaseCheck`]。
pub(super) fn unchecked(path: &Path) -> Result<PathBuf, String> {
    Ok(path.to_path_buf())
}

/// モジュールのプロジェクトの本体を読む。`path` は `.ugoeproj` のディレクトリか `.ugoe`。
pub fn load_base(path: &Path) -> Result<ModuleBase, ProjectError> {
    load_base_checked(path, &unchecked)
}

/// [`load_base`] と同じだが、読む前に `path` を `check` に渡し、返したパスから読む。`check` がエラーなら
/// 何も開かずに [`ProjectError::BaseRefused`]。[`ModuleBase::path`] (保存する参照の元) は `path` のまま。
pub fn load_base_checked(path: &Path, check: BaseCheck<'_>) -> Result<ModuleBase, ProjectError> {
    load_base_with_limits(path, check, ProjectReadLimits::default())
}

/// [`load_base_checked`] と同じだが、ファイル 1 つの大きさの上限を `limits` にする (T1607)。本体が `.ugoe` なら
/// [`ProjectReadLimits::max_bundle`]、プロジェクトなら [`Project::load_with_limits`] と同じ上限で読む。
/// 通常のファイルでない本体 (FIFO・装置) は開かずに [`ProjectError::NotRegularFile`]。
pub fn load_base_with_limits(
    path: &Path,
    check: BaseCheck<'_>,
    limits: ProjectReadLimits,
) -> Result<ModuleBase, ProjectError> {
    let wrap = |source: ProjectError| ProjectError::Base {
        path: path.to_path_buf(),
        source: Box::new(source),
    };
    let read = check(path).map_err(|reason| ProjectError::BaseRefused {
        path: path.to_path_buf(),
        reason,
    })?;
    if !read.exists() {
        return Err(ProjectError::BaseNotFound {
            path: path.to_path_buf(),
        });
    }
    if read.is_dir() {
        // 本体がモジュールなら、その本体まで読みに行かないように、読む前に種類を見る。
        let meta_path = read.join(PROJECT_JSON);
        if meta_path.is_file()
            && read_json::<ProjectMeta>(&meta_path, limits.max_json)
                .map_err(wrap)?
                .kind
                == ProjectKind::Module
        {
            return Err(ProjectError::BaseIsModule {
                path: path.to_path_buf(),
            });
        }
        let project = Project::load_with_limits(&read, &unchecked, limits).map_err(wrap)?;
        return Ok(ModuleBase {
            path: path.to_path_buf(),
            model: project.model,
            physics: project.physics,
            textures: project.textures,
        });
    }
    let bytes = read_regular_file_as(&read, path, limits.max_bundle)?;
    let bundle = ugoe_format::read_bundle(&bytes).map_err(|source| {
        wrap(ProjectError::Format {
            path: path.to_path_buf(),
            source,
        })
    })?;
    let textures = crate::bundle_project::images(
        &bundle.model.textures,
        &bundle.textures,
        &bundle.normal_maps,
        |name| ProjectError::UnsupportedBaseTexture { name },
    )?;
    Ok(ModuleBase {
        path: path.to_path_buf(),
        model: bundle.model,
        physics: bundle.physics,
        textures,
    })
}
