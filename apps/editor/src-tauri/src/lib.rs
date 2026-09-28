//! エディタのバックエンド。ここは **薄い IPC 層** に保ち、編集ロジックは `ugoe-editor-core` に置く。
//! 設計: `docs/editor.md`。

pub mod bindings;
mod commands;
mod project;
mod psd;
mod space;

use std::sync::Mutex;

use ugoe_editor_core::Document;

/// アプリ全体の状態。現状は単一ドキュメント。
/// 両方をロックするときは `document` → `project` の順にする。
#[derive(Debug)]
pub struct AppState {
    pub document: Mutex<Document>,
    /// 開いているプロジェクトの保存先・メタ情報など (`Document` の外のもの)。
    pub project: Mutex<project::ProjectSession>,
}

pub fn run() {
    let (session, document) = project::initial_state();
    let state = AppState {
        document: Mutex::new(document),
        project: Mutex::new(session),
    };
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::apply_command,
            commands::get_model_json,
            commands::undo,
            commands::redo,
            project::project_status,
            project::new_project,
            project::new_project_from_psd,
            project::open_project,
            project::save_project,
            project::save_project_as,
            project::export_bundle,
            project::get_model_bundle,
            psd::import_psd,
            space::local_to_canvas,
            space::canvas_to_local,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
