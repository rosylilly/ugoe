//! エディタのバックエンド。ここは **薄い IPC 層** に保ち、編集ロジックは `ugoe-editor-core` に置く。
//! 設計: `docs/editor.md`。

mod commands;

use std::sync::Mutex;

use ugoe_editor_core::Document;

/// アプリ全体の状態。現状は単一ドキュメント。
#[derive(Debug)]
pub struct AppState {
    pub document: Mutex<Document>,
}

pub fn run() {
    let state = AppState {
        document: Mutex::new(Document::new(ugoe_model::fixtures::single_quad())),
    };
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::apply_command,
            commands::get_model_json,
            commands::undo,
            commands::redo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
