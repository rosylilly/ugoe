//! Tauri IPC コマンド。フロントエンドからは `invoke("<name>", args)` で呼ぶ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_editor_core::{ApplyReport, Command};

use crate::AppState;

#[derive(Debug, Serialize, TS)]
pub struct AppInfo {
    pub version: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// 編集コマンドを適用する。JSON 形式は `ugoe_editor_core::Command` を参照。
/// 適用はしたが知らせたいこと (自動のミラーのやり直しの失敗) を返す。
#[tauri::command]
pub fn apply_command(state: State<'_, AppState>, command: Command) -> Result<ApplyReport, String> {
    lock(&state)?
        .apply_with_report(command)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_model_json(state: State<'_, AppState>) -> Result<String, String> {
    ugoe_format::model_to_json(lock(&state)?.model()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn undo(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(lock(&state)?.undo())
}

#[tauri::command]
pub fn redo(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(lock(&state)?.redo())
}

pub(crate) fn lock<'a>(
    state: &'a State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, ugoe_editor_core::Document>, String> {
    state
        .document
        .lock()
        .map_err(|_| "document lock poisoned".to_string())
}
