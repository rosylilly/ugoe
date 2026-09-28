//! PSD の取り込みの IPC コマンド。取り込みの規則は `ugoe_editor_core::import`
//! (`docs/editor.md#psd-の取り込み`) にあり、ここはファイルを読んでコマンドを適用するだけ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::Path;

use tauri::State;
use ugoe_editor_core::{AutoMeshOptions, PsdImportOptions, PsdImportWarning, import};

use crate::AppState;

/// PSD ファイルを読み、今のドキュメントに取り込む (Undo の 1 単位)。
/// 失敗したらドキュメントは変わらない。戻り値は取り込みの警告。
#[tauri::command]
pub fn import_psd(
    state: State<'_, AppState>,
    path: String,
    mesh_options: Option<AutoMeshOptions>,
) -> Result<Vec<PsdImportWarning>, String> {
    let path = Path::new(path.trim());
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path
        .file_stem()
        .map_or_else(|| "PSD".to_owned(), |s| s.to_string_lossy().into_owned());
    let options = PsdImportOptions {
        name,
        mesh: mesh_options.unwrap_or_default(),
    };
    let mut document = state
        .document
        .lock()
        .map_err(|_| "document lock poisoned".to_string())?;
    let import =
        import::import_psd(document.model(), &bytes, &options).map_err(|e| e.to_string())?;
    document.apply(import.command).map_err(|e| e.to_string())?;
    Ok(import.warnings)
}
