//! PSD の取り込みの IPC コマンド。取り込みの規則は `ugoe_editor_core::import`
//! (`docs/editor.md#psd-の取り込み`) にあり、ここはファイルを読んでコマンドを適用するだけ。
//! PSD から新しいプロジェクトを作る `new_project_from_psd` は `project.rs` にある。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use tauri::State;
use ugoe_editor_core::{AutoMeshOptions, PsdImportOptions, PsdImportWarning, import};

use crate::AppState;

/// PSD ファイルを読み、取り込みの設定 (最上位のパーツの名前はファイル名) と合わせて返す。
pub(crate) fn read_psd(
    path: &str,
    mesh_options: Option<AutoMeshOptions>,
) -> Result<(Vec<u8>, PsdImportOptions), String> {
    let path = crate::project::absolute_path(path)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path
        .file_stem()
        .map_or_else(|| "PSD".to_owned(), |s| s.to_string_lossy().into_owned());
    let options = PsdImportOptions {
        name,
        mesh: mesh_options.unwrap_or_default(),
    };
    Ok((bytes, options))
}

/// PSD ファイルを読み、今のドキュメントに取り込む (Undo の 1 単位)。
/// 失敗したらドキュメントは変わらない。戻り値は取り込みの警告。
#[tauri::command]
pub fn import_psd(
    state: State<'_, AppState>,
    path: String,
    mesh_options: Option<AutoMeshOptions>,
) -> Result<Vec<PsdImportWarning>, String> {
    let (bytes, options) = read_psd(&path, mesh_options)?;
    let mut document = state
        .document
        .lock()
        .map_err(|_| "document lock poisoned".to_string())?;
    let mut import =
        import::import_psd(document.model(), &bytes, &options).map_err(|e| e.to_string())?;
    // モジュールのプロジェクトでは、描画順 (0〜1000) をスロットの範囲に写す。最上位の要素は適用のときに
    // 見た目を保ったままスロットの取り付け先の下へ移る (`docs/editor.md#モジュールのプロジェクト`)。
    if let Some(context) = document.module() {
        context.map_new_meshes(&mut import.command);
    }
    document.apply(import.command).map_err(|e| e.to_string())?;
    // AI の提案 (自動リギング) の元にする PSD を覚えておく (ロックの順序は document → project)。
    crate::ai::lock_project(&state)?
        .proposals
        .set_psd(crate::project::absolute_path(&path)?);
    Ok(import.warnings)
}
