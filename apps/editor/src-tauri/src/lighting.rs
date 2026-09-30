//! ライティングの画面 (`docs/editor.md#ライティングの画面`) の IPC コマンド。
//!
//! 法線マップのコマンドの組み立ては `ugoe_editor_core::lighting` にあり、ここは画像の受け渡しと適用だけを行う。
//! プレビューの光源の読み書きは `project.rs` (`get_preview_lighting` / `set_preview_lighting`)。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use tauri::State;
use ugoe_editor_core::ApplyReport;
use ugoe_editor_core::lighting::{BulgeSettings, assign_normal_map, generate_normal_map};

use crate::AppState;
use crate::commands::lock;
use crate::project::absolute_path;

/// テクスチャ `texture` のアルファの形からふくらみの法線マップを作って付ける (AI は使わない)。
/// モデルのライティングが無効なら有効にする。1 回の Undo で戻る。
#[tauri::command]
pub fn generate_texture_normal_map(
    state: State<'_, AppState>,
    texture: String,
    settings: BulgeSettings,
) -> Result<ApplyReport, String> {
    let mut document = lock(&state)?;
    let command = generate_normal_map(document.model(), document.textures(), &texture, settings)
        .map_err(|e| e.to_string())?;
    document
        .apply_with_report(command)
        .map_err(|e| e.to_string())
}

/// PNG ファイル `path` (開くのダイアログで選んだもの) をテクスチャ `texture` の法線マップとして付ける。
/// `flip_green` なら G を反転する (DirectX 形式の画像)。1 回の Undo で戻る。
#[tauri::command]
pub fn assign_texture_normal_map(
    state: State<'_, AppState>,
    texture: String,
    path: String,
    flip_green: bool,
) -> Result<ApplyReport, String> {
    let path = absolute_path(&path)?;
    let png = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut document = lock(&state)?;
    let command = assign_normal_map(document.model(), &texture, png, flip_green)
        .map_err(|e| e.to_string())?;
    document
        .apply_with_report(command)
        .map_err(|e| e.to_string())
}

/// テクスチャか法線マップの画像 `name` の PNG (画面で確かめるため)。生のバイト列で送る。
#[tauri::command]
pub fn get_texture_png(
    state: State<'_, AppState>,
    name: String,
) -> Result<tauri::ipc::Response, String> {
    let document = lock(&state)?;
    let png = document
        .texture(&name)
        .ok_or_else(|| format!("image `{name}` not found"))?;
    Ok(tauri::ipc::Response::new(png.to_vec()))
}
