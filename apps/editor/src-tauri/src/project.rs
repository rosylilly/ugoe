//! プロジェクトの新規作成・開く・保存・書き出しの IPC コマンド。
//! 読み書きの中身は `ugoe_editor_core::project` にあり、ここはパスと状態の受け渡しだけを行う。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_editor_core::project::PROJECT_EXTENSION;
use ugoe_editor_core::{
    AutoMeshOptions, Document, Project, ProjectMeta, PsdImportOptions, PsdImportWarning, import,
};
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;

use crate::AppState;

/// 開いているプロジェクトのうち、`Document` の外にあるもの (保存先・メタ情報・物理など)。
#[derive(Debug)]
pub struct ProjectSession {
    /// 保存先の `.ugoeproj` ディレクトリ。一度も保存していなければ `None`。
    path: Option<PathBuf>,
    meta: ProjectMeta,
    physics: Option<PhysicsSettings>,
    motions: Vec<Motion>,
    expressions: Vec<Expression>,
}

impl ProjectSession {
    fn from_project(project: Project, path: Option<PathBuf>) -> (Self, Document) {
        let document = project.document();
        let session = Self {
            path,
            meta: project.meta,
            physics: project.physics,
            motions: project.motions,
            expressions: project.expressions,
        };
        (session, document)
    }

    fn to_project(&self, document: &Document) -> Project {
        Project::from_document(
            self.meta.clone(),
            document,
            self.physics.clone(),
            self.motions.clone(),
            self.expressions.clone(),
        )
    }
}

/// 起動時の状態: 空のモデルの、保存先のないプロジェクト (`new_project` と同じ)。
/// 空のモデルに PSD を取り込むとキャンバスが PSD の大きさになる (`docs/editor.md#psd-の取り込み`)。
pub fn initial_state() -> (ProjectSession, Document) {
    ProjectSession::from_project(Project::empty(UNTITLED), None)
}

/// 新しいプロジェクトの名前 (保存するまで)。
const UNTITLED: &str = "Untitled";

/// PSD から新しいプロジェクトを作る: `options.name` (PSD のファイル名) の空のプロジェクトに取り込む。
/// 空のモデルへの取り込みなので、キャンバスは PSD の大きさになる。取り込みは Undo の 1 単位で、
/// 未保存の状態から始まる (Undo すると空のプロジェクトに戻る)。
fn from_psd(
    bytes: &[u8],
    options: &PsdImportOptions,
) -> Result<(ProjectSession, Document, Vec<PsdImportWarning>), String> {
    let (session, mut document) =
        ProjectSession::from_project(Project::empty(options.name.clone()), None);
    let import = import::import_psd(document.model(), bytes, options).map_err(|e| e.to_string())?;
    document.apply(import.command).map_err(|e| e.to_string())?;
    Ok((session, document, import.warnings))
}

/// フロントエンドに見せるプロジェクトの状態。
#[derive(Debug, Serialize, TS)]
pub struct ProjectStatus {
    pub name: String,
    /// 保存先のディレクトリ。未保存なら `null`。
    pub path: Option<String>,
    /// 最後に保存 (または開いた) ときから変更があるか。
    pub modified: bool,
}

fn status(session: &ProjectSession, document: &Document) -> ProjectStatus {
    ProjectStatus {
        name: session.meta.name.clone(),
        path: session.path.as_ref().map(|p| p.display().to_string()),
        modified: document.is_modified(),
    }
}

/// ロックの順序は常に `document` → `project`。
fn lock_both<'a>(
    state: &'a State<'_, AppState>,
) -> Result<(MutexGuard<'a, Document>, MutexGuard<'a, ProjectSession>), String> {
    let document = state
        .document
        .lock()
        .map_err(|_| "document lock poisoned".to_string())?;
    let project = state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?;
    Ok((document, project))
}

/// フロントエンドから受け取ったパス。絶対パスだけを受け付ける。
/// 相対パスはプロセスの作業ディレクトリ (開発時は `apps/editor/src-tauri`) を基準にしてしまうため拒否する
/// (パスはファイルダイアログで選ぶ。`docs/editor.md#ファイルダイアログ`)。
pub(crate) fn absolute_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is empty".into());
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(format!("path must be absolute: {trimmed}"));
    }
    Ok(path)
}

/// 絶対パスで、拡張子が `.ugoeproj` でなければ付け足す。
fn project_dir(path: &str) -> Result<PathBuf, String> {
    let path = absolute_path(path)?;
    if path.extension().is_some_and(|e| e == PROJECT_EXTENSION) {
        return Ok(path);
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(PROJECT_EXTENSION);
    Ok(PathBuf::from(name))
}

/// ディレクトリ名から拡張子を除いたもの。
fn stem(path: &Path) -> Option<String> {
    path.file_stem().map(|s| s.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn project_status(state: State<'_, AppState>) -> Result<ProjectStatus, String> {
    let (document, session) = lock_both(&state)?;
    Ok(status(&session, &document))
}

/// 空のモデルの新しいプロジェクトにする。未保存の変更は破棄する (確認はフロントエンドで行う)。
#[tauri::command]
pub fn new_project(state: State<'_, AppState>) -> Result<ProjectStatus, String> {
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(Project::empty(UNTITLED), None);
    Ok(status(&session, &document))
}

/// PSD ファイルから新しいプロジェクトを作る (File > New from PSD…)。プロジェクトの名前は
/// ファイル名 (拡張子を除く)、キャンバスは PSD の大きさ。未保存の変更は破棄する (確認はフロントエンドで行う)。
/// 失敗したら今のプロジェクトはそのまま。戻り値は取り込みの警告。
#[tauri::command]
pub fn new_project_from_psd(
    state: State<'_, AppState>,
    path: String,
    mesh_options: Option<AutoMeshOptions>,
) -> Result<Vec<PsdImportWarning>, String> {
    let (bytes, options) = crate::psd::read_psd(&path, mesh_options)?;
    let (new_session, new_document, warnings) = from_psd(&bytes, &options)?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = (new_session, new_document);
    Ok(warnings)
}

/// `.ugoeproj` ディレクトリを開く。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_project(state: State<'_, AppState>, path: String) -> Result<ProjectStatus, String> {
    let dir = absolute_path(&path)?;
    let project = Project::load(&dir).map_err(|e| e.to_string())?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, Some(dir));
    Ok(status(&session, &document))
}

/// 開いたときの (または前回保存した) 場所に保存する。
#[tauri::command]
pub fn save_project(state: State<'_, AppState>) -> Result<ProjectStatus, String> {
    let (mut document, session) = lock_both(&state)?;
    let dir = session
        .path
        .clone()
        .ok_or("the project has not been saved yet; use Save As")?;
    session
        .to_project(&document)
        .save(&dir)
        .map_err(|e| e.to_string())?;
    document.mark_saved();
    Ok(status(&session, &document))
}

/// 別の場所に保存し、以後の保存先をそこにする。拡張子 `.ugoeproj` がなければ付ける。
/// プロジェクト名はディレクトリ名 (拡張子を除く) にする。
#[tauri::command]
pub fn save_project_as(state: State<'_, AppState>, path: String) -> Result<ProjectStatus, String> {
    let dir = project_dir(&path)?;
    let (mut document, mut session) = lock_both(&state)?;
    let mut meta = session.meta.clone();
    if let Some(name) = stem(&dir) {
        meta.name = name;
    }
    let mut project = session.to_project(&document);
    project.meta = meta.clone();
    project.save(&dir).map_err(|e| e.to_string())?;
    session.meta = meta;
    session.path = Some(dir);
    document.mark_saved();
    Ok(status(&session, &document))
}

/// 今のドキュメントを `.ugoe` バンドルのバイト列で返す (ビューポートがテクスチャ付きで表示するため)。
/// JSON を経由しないよう、生のバイト列 (`tauri::ipc::Response`) で送る (`docs/editor.md#ビューポート`)。
#[tauri::command]
pub fn get_model_bundle(state: State<'_, AppState>) -> Result<tauri::ipc::Response, String> {
    let (document, session) = lock_both(&state)?;
    let bytes = ugoe_editor_core::export_bundle(&session.to_project(&document))
        .map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// `.ugoe` バンドルとしてファイルに書き出す。プロジェクトの保存状態は変えない。
#[tauri::command]
pub fn export_bundle(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let path = absolute_path(&path)?;
    let bytes = {
        let (document, session) = lock_both(&state)?;
        ugoe_editor_core::export_bundle(&session.to_project(&document))
            .map_err(|e| e.to_string())?
    };
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_dir_appends_extension() {
        let dir = std::env::temp_dir();
        assert_eq!(
            project_dir(&dir.join("a").display().to_string()).unwrap(),
            dir.join("a.ugoeproj")
        );
        assert_eq!(
            project_dir(&dir.join("a.ugoeproj").display().to_string()).unwrap(),
            dir.join("a.ugoeproj")
        );
        assert!(project_dir(" ").is_err());
    }

    #[test]
    fn relative_paths_are_rejected() {
        let err = absolute_path("test.ugoeproj").unwrap_err();
        assert!(err.contains("must be absolute"), "{err}");
        assert!(project_dir("test").is_err());
        assert!(crate::psd::read_psd("layers.psd", None).is_err());
        let dir = std::env::temp_dir();
        assert_eq!(absolute_path(&format!(" {} ", dir.display())).unwrap(), dir);
    }

    #[test]
    fn initial_state_is_empty_and_can_be_exported() {
        let (session, document) = initial_state();
        assert!(!document.is_modified());
        let model = document.model();
        assert!(model.parts.is_empty() && model.meshes.is_empty() && model.textures.is_empty());
        let bytes = ugoe_editor_core::export_bundle(&session.to_project(&document)).unwrap();
        let bundle = ugoe_format::read_bundle(&bytes).unwrap();
        assert!(bundle.model.meshes.is_empty());
    }

    fn character_psd() -> Vec<u8> {
        std::fs::read(crate::bindings::repo_root().join("assets/fixtures/character.psd")).unwrap()
    }

    /// 起動時の状態に取り込むと、キャンバスが PSD の大きさになり、キャンバスの警告は出ない。
    #[test]
    fn importing_into_initial_state_uses_psd_canvas() {
        let (_, mut document) = initial_state();
        let import = import::import_psd(
            document.model(),
            &character_psd(),
            &PsdImportOptions::new("character"),
        )
        .unwrap();
        assert!(import.warnings.is_empty(), "{:?}", import.warnings);
        document.apply(import.command).unwrap();
        let canvas = &document.model().meta.canvas;
        assert_eq!((canvas.width, canvas.height), (320.0, 400.0));
    }

    #[test]
    fn new_project_from_psd_is_named_after_the_file() {
        let (session, mut document, warnings) =
            from_psd(&character_psd(), &PsdImportOptions::new("character")).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let status = status(&session, &document);
        assert_eq!(status.name, "character");
        assert_eq!(status.path, None);
        assert!(status.modified);
        let canvas = &document.model().meta.canvas;
        assert_eq!((canvas.width, canvas.height), (320.0, 400.0));
        assert!(!document.model().meshes.is_empty());
        // 書き出せる。Undo で空のプロジェクトに戻る。
        ugoe_editor_core::export_bundle(&session.to_project(&document)).unwrap();
        assert!(document.undo());
        assert!(document.model().meshes.is_empty());
        assert!(!document.is_modified());
    }

    #[test]
    fn new_project_from_psd_rejects_invalid_files() {
        assert!(from_psd(b"not a psd", &PsdImportOptions::new("broken")).is_err());
    }
}
