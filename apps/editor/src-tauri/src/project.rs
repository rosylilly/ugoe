//! プロジェクトの新規作成・開く・保存・書き出しの IPC コマンド。
//! 読み書きの中身は `ugoe_editor_core::project` にあり、ここはパスと状態の受け渡しだけを行う。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::{Path, PathBuf};
use std::sync::{Arc, MutexGuard};

use serde::Serialize;
use tauri::State;
use ugoe_editor_core::project::PROJECT_EXTENSION;
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Document, Project, ProjectMeta, Textures};
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

/// 起動時の状態: 白いテクスチャを持つ `single_quad` の、保存先のないプロジェクト。
pub fn initial_state() -> (ProjectSession, Document) {
    let model = ugoe_model::fixtures::single_quad();
    let textures: Textures = model
        .textures
        .iter()
        .map(|t| {
            let png = solid_png(t.width, t.height, [255; 4]);
            (t.name.clone(), Arc::from(png))
        })
        .collect();
    let project = Project::new(ProjectMeta::new("Untitled"), model, textures);
    ProjectSession::from_project(project, None)
}

/// フロントエンドに見せるプロジェクトの状態。
#[derive(Debug, Serialize)]
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

/// 拡張子が `.ugoeproj` でなければ付け足す。
fn project_dir(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("path is empty".into());
    }
    let path = PathBuf::from(path);
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
    (*session, *document) = ProjectSession::from_project(Project::empty("Untitled"), None);
    Ok(status(&session, &document))
}

/// `.ugoeproj` ディレクトリを開く。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_project(state: State<'_, AppState>, path: String) -> Result<ProjectStatus, String> {
    let dir = PathBuf::from(path);
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
    if path.trim().is_empty() {
        return Err("path is empty".into());
    }
    let bytes = {
        let (document, session) = lock_both(&state)?;
        ugoe_editor_core::export_bundle(&session.to_project(&document))
            .map_err(|e| e.to_string())?
    };
    std::fs::write(&path, bytes).map_err(|e| format!("{path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_dir_appends_extension() {
        assert_eq!(
            project_dir("/tmp/a").unwrap(),
            PathBuf::from("/tmp/a.ugoeproj")
        );
        assert_eq!(
            project_dir("/tmp/a.ugoeproj").unwrap(),
            PathBuf::from("/tmp/a.ugoeproj")
        );
        assert!(project_dir(" ").is_err());
    }

    #[test]
    fn initial_state_can_be_exported() {
        let (session, document) = initial_state();
        assert!(!document.is_modified());
        let bytes = ugoe_editor_core::export_bundle(&session.to_project(&document)).unwrap();
        assert!(ugoe_format::read_bundle(&bytes).is_ok());
    }
}
