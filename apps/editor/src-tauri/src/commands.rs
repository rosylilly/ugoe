//! Tauri IPC コマンド。フロントエンドからは `invoke("<name>", args)` で呼ぶ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_editor_core::{ApplyReport, Command};
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;

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

/// ドキュメントのモーションと表情 (`get_animations` の戻り値)。
#[derive(Debug, Serialize, TS)]
pub struct AnimationData {
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

/// ドキュメントのモーションと表情を返す (タイムラインの表示用。編集は `apply_command`)。
#[tauri::command]
pub fn get_animations(state: State<'_, AppState>) -> Result<AnimationData, String> {
    let document = lock(&state)?;
    Ok(AnimationData {
        motions: document.motions().to_vec(),
        expressions: document.expressions().to_vec(),
    })
}

/// 物理の設定 (`ugoe_model::physics::PhysicsSettings`)。なければ `null`。
#[tauri::command]
pub fn get_physics(
    state: State<'_, AppState>,
) -> Result<Option<ugoe_model::physics::PhysicsSettings>, String> {
    Ok(lock(&state)?.physics().cloned())
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
