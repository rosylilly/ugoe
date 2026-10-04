//! Tauri IPC コマンド。フロントエンドからは `invoke("<name>", args)` で呼ぶ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;
use ugoe_editor_core::{ApplyReport, Coalesce, Command, Message};
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;

use crate::AppState;
use crate::error::{EditorError, IpcResult};

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
/// `coalesce` (数値の欄の確定の印) があれば、同じ印の続けての確定を 1 つの Undo にまとめる
/// (`Document::apply_coalescing`。docs/editor.md#undo)。
#[tauri::command]
pub fn apply_command(
    state: State<'_, AppState>,
    command: Command,
    coalesce: Option<String>,
) -> IpcResult<ApplyReport> {
    let mut document = lock(&state)?;
    match coalesce {
        Some(key) => document.apply_coalescing(command, Coalesce::new(key, Instant::now())),
        None => document.apply_with_report(command),
    }
    .map_err(Message::from)
}

#[tauri::command]
pub fn get_model_json(state: State<'_, AppState>) -> IpcResult<String> {
    ugoe_format::model_to_json(lock(&state)?.model()).map_err(Message::from)
}

/// ドキュメントのモーションと表情 (`get_animations` の戻り値)。
#[derive(Debug, Serialize, TS)]
pub struct AnimationData {
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

/// ドキュメントのモーションと表情を返す (タイムラインの表示用。編集は `apply_command`)。
#[tauri::command]
pub fn get_animations(state: State<'_, AppState>) -> IpcResult<AnimationData> {
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
) -> IpcResult<Option<ugoe_model::physics::PhysicsSettings>> {
    Ok(lock(&state)?.physics().cloned())
}

#[tauri::command]
pub fn undo(state: State<'_, AppState>) -> IpcResult<bool> {
    Ok(lock(&state)?.undo())
}

#[tauri::command]
pub fn redo(state: State<'_, AppState>) -> IpcResult<bool> {
    Ok(lock(&state)?.redo())
}

pub(crate) fn lock(
    state: &AppState,
) -> IpcResult<std::sync::MutexGuard<'_, ugoe_editor_core::Document>> {
    state
        .document
        .lock()
        .map_err(|_| EditorError::LockPoisoned { what: "document" }.into())
}

/// 重いコマンド (ファイルの読み書き・バンドルを作る・画像の計算・AI の応答待ちなど) の本体を、Tauri のメインのスレッド
/// (イベントのループ) ではなく `spawn_blocking` のスレッドで動かす。同期のコマンドはメインのスレッドで動き、その間は
/// ウィンドウが止まるため、重いコマンドは `async` にしてここへ渡す (`docs/editor.md#メインのスレッドの外で動くコマンド`)。
pub(crate) async fn blocking<T: Send + 'static>(
    app: AppHandle,
    task: impl FnOnce(&AppState) -> IpcResult<T> + Send + 'static,
) -> IpcResult<T> {
    off_main(move || task(&app.state::<AppState>())).await
}

/// [`blocking`] の、アプリの状態を使わないもの (ファイルを読むだけのコマンドなど)。
pub(crate) async fn off_main<T: Send + 'static>(
    task: impl FnOnce() -> IpcResult<T> + Send + 'static,
) -> IpcResult<T> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| EditorError::Task(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 重いコマンドの本体は呼んだスレッド (Tauri のメインのスレッド) の外で動き、エラーはそのまま返る (T0685)。
    #[test]
    fn heavy_commands_run_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker =
            tauri::async_runtime::block_on(off_main(|| Ok(std::thread::current().id()))).unwrap();
        assert_ne!(worker, caller);
        let err: IpcResult<()> =
            tauri::async_runtime::block_on(off_main(|| Err(EditorError::NotSaved.into())));
        assert_eq!(err.unwrap_err().code, "editor.not_saved");
    }
}
