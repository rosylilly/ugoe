//! エディタのバックエンド。ここは **薄い IPC 層** に保ち、編集ロジックは `ugoe-editor-core` に置く。
//! 設計: `docs/editor.md`。

pub(crate) mod ai;
pub mod bindings;
mod commands;
mod overflow;
mod physics_check;
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
    /// AI の API キー (OS の資格情報の保存先と環境変数)。
    pub api_keys: ai::ApiKeys,
}

pub fn run() {
    let (session, document) = project::initial_state();
    let state = AppState {
        document: Mutex::new(document),
        project: Mutex::new(session),
        api_keys: ai::ApiKeys::default(),
    };
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::apply_command,
            commands::get_model_json,
            commands::get_animations,
            commands::get_physics,
            commands::undo,
            commands::redo,
            project::project_status,
            project::new_project,
            project::new_project_from_psd,
            project::open_project,
            project::save_project,
            project::save_project_as,
            project::get_export_settings,
            project::set_export_settings,
            project::export_bundle,
            project::get_model_bundle,
            psd::import_psd,
            space::local_to_canvas,
            space::canvas_to_local,
            overflow::find_overflows,
            overflow::verify_overflows,
            physics_check::check_physics,
            ai::ai_settings,
            ai::store_api_key,
            ai::delete_api_key,
            ai::set_ai_send_externally,
            ai::forget_ai_consent,
            ai::test_ai_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
