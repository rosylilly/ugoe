//! File > Open Sample…: 着せ替えの見本 (`assets/samples/dress-up/`、T0710) を、エディタで開けるプロジェクトとして
//! 書き出して開く (T0715)。見本の `.ugoe`・`.ugoemod` はバイナリに埋め込み、`ugoe sample dress-up --project` と
//! 同じ形 (本体 `dress-up.ugoeproj` と、それを参照するモジュールのプロジェクト) にする。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::State;
use ugoe_editor_core::{Project, ProjectMeta, Textures};
use ugoe_format::{ModuleFile, TextureData, TextureFormat};
use ugoe_model::TextureRef;

use crate::AppState;
use crate::project::{ProjectSession, ProjectStatus, absolute_path, lock_both, status};

/// 見本の本体。
const BASE: &[u8] = include_bytes!("../../../../assets/samples/dress-up/dress-up.ugoe");

/// 見本のモジュール (プロジェクトのディレクトリ名の元にするファイル名と中身)。
const MODULES: [(&str, &[u8]); 4] = [
    (
        "long-hair",
        include_bytes!("../../../../assets/samples/dress-up/long-hair.ugoemod"),
    ),
    (
        "twin-tails",
        include_bytes!("../../../../assets/samples/dress-up/twin-tails.ugoemod"),
    ),
    (
        "cat-ears",
        include_bytes!("../../../../assets/samples/dress-up/cat-ears.ugoemod"),
    ),
    (
        "sailor",
        include_bytes!("../../../../assets/samples/dress-up/sailor.ugoemod"),
    ),
];

/// 本体のプロジェクトのディレクトリ名。
const BASE_PROJECT: &str = "dress-up.ugoeproj";

/// 見本を書き出すフォルダーの名前 (`parent` の中に作る。あれば `-2`・`-3`… を付ける)。
const FOLDER: &str = "ugoe-dress-up-sample";

/// 見本のプロジェクトを `parent` の中の新しいフォルダーに書き出し、本体のプロジェクトを開く。
/// 既にあるフォルダーには書かない (前に書き出して編集した見本を上書きしない)。未保存の変更は破棄する
/// (確認はフロントエンドで行う)。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_sample_project(
    state: State<'_, AppState>,
    parent: String,
) -> Result<ProjectStatus, String> {
    let dir = new_folder(&absolute_path(&parent)?)?;
    let written = write_projects(&dir)?;
    let (project, disk) = Project::load_tracked(&written[0]).map_err(|e| e.to_string())?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, Some((written[0].clone(), disk)));
    Ok(status(&session, &document))
}

/// `parent` の中の、まだ無いフォルダーを作る。
fn new_folder(parent: &Path) -> Result<PathBuf, String> {
    for n in 1..100 {
        let name = if n == 1 {
            FOLDER.to_owned()
        } else {
            format!("{FOLDER}-{n}")
        };
        let dir = parent.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("creating {}: {e}", dir.display())),
        }
    }
    Err(format!(
        "too many sample folders in {}; remove old ones",
        parent.display()
    ))
}

/// 見本の本体とモジュールのプロジェクトを `dir` に書く。書いたプロジェクトのパスを、本体を先に返す。
/// 本体を保存してから、エディタの「New Module…」と同じ `Project::new_module` でモジュールのプロジェクトを作り、
/// 見本のモジュールを入れる (`ugoe sample dress-up --project` と同じ手順)。
fn write_projects(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let bundle = ugoe_format::read_bundle(BASE).map_err(|e| e.to_string())?;
    let base_dir = dir.join(BASE_PROJECT);
    let model = bundle.model;
    let mut base = Project::new(
        ProjectMeta::new(model.meta.name.clone()),
        model.clone(),
        images(&model.textures, &bundle.textures)?,
    );
    base.physics = bundle.physics;
    base.save(&base_dir).map_err(|e| e.to_string())?;
    let mut written = vec![base_dir.clone()];
    for (file_name, bytes) in MODULES {
        let ModuleFile {
            module, textures, ..
        } = ugoe_format::read_module(bytes).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{file_name}.ugoeproj"));
        let empty = Project::new_module(&module.name, &base_dir, &module.slot)
            .map_err(|e| e.to_string())?;
        let context = empty
            .module
            .as_ref()
            .ok_or_else(|| format!("`{}` is not a module project", module.name))?
            .reconfigured(&module.slot, module.name.clone(), module.requires.clone())
            .map_err(|e| e.to_string())?;
        let workspace = context
            .workspace(&module, &images(&module.textures, &textures)?)
            .map_err(|e| e.to_string())?;
        let project = Project {
            model: workspace.model,
            textures: workspace.textures,
            physics: workspace.physics,
            module: Some(Arc::new(context)),
            ..empty
        };
        project.save(&path).map_err(|e| e.to_string())?;
        written.push(path);
    }
    Ok(written)
}

/// テクスチャの参照と中身 (PNG) を、プロジェクトの画像 (名前 → PNG) にする。
fn images(refs: &[TextureRef], data: &[TextureData]) -> Result<Textures, String> {
    if refs.len() != data.len() {
        return Err(format!("{} textures but {} images", refs.len(), data.len()));
    }
    refs.iter()
        .zip(data)
        .map(|(r, d)| {
            if d.format != TextureFormat::Png {
                return Err(format!("texture `{}` is not a PNG", r.name));
            }
            Ok((r.name.clone(), Arc::from(d.bytes.as_slice())))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ugoe-editor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 書き出した見本を開ける: 本体は見本の本体と同じモデルと物理、モジュールのプロジェクトは本体を参照し、
    /// 書き出す (Export Module) と埋め込んだ `.ugoemod` とバイト列で同じになる。
    #[test]
    fn sample_projects_open_and_export_the_same_modules() {
        let dir = temp_dir("sample");
        let written = write_projects(&dir).unwrap();
        assert_eq!(written.len(), 1 + MODULES.len());

        let bundle = ugoe_format::read_bundle(BASE).unwrap();
        let base = Project::load(&written[0]).unwrap();
        assert!(base.module.is_none());
        assert_eq!(base.model, bundle.model);
        assert_eq!(base.physics, bundle.physics);

        for (path, (_, bytes)) in written[1..].iter().zip(MODULES) {
            let project = Project::load(path).unwrap();
            assert!(project.module.is_some(), "{}", path.display());
            let export = ugoe_editor_core::export_module(&project).unwrap();
            assert!(export.warnings.is_empty(), "{:?}", export.warnings);
            assert!(
                export.bytes == bytes,
                "{} is exported differently",
                path.display()
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 書き出すフォルダーは毎回新しく作り、前の見本を上書きしない。
    #[test]
    fn sample_folders_do_not_overwrite_earlier_ones() {
        let dir = temp_dir("sample-folders");
        let first = new_folder(&dir).unwrap();
        let second = new_folder(&dir).unwrap();
        assert_eq!(first.file_name().unwrap(), FOLDER);
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            format!("{FOLDER}-2")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
