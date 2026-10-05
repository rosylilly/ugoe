//! PSD の取り込みの IPC コマンド。取り込みの規則は `ugoe_editor_core::import`
//! (`docs/editor.md#psd-の取り込み`) にあり、ここはファイルを読んでコマンドを適用するだけ。
//! PSD から新しいプロジェクトを作る `new_project_from_psd` は `project.rs` にある。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use tauri::AppHandle;
use ugoe_editor_core::{AutoMeshOptions, PsdImportOptions, PsdImportWarning, import};

use crate::AppState;
use crate::commands::blocking;
use crate::error::{EditorError, IpcResult};
use crate::project::lock_both;
use crate::proposals::Base;
use ugoe_fs::read_regular_file;

/// 読む PSD の大きさの上限 (PSD の形式の上限は 2 GB。PSB は読まない)。
pub(crate) const MAX_PSD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// PSD ファイルを読み、取り込みの設定 (最上位のパーツの名前はファイル名) と合わせて返す。
pub(crate) fn read_psd(
    path: &str,
    mesh_options: Option<AutoMeshOptions>,
) -> IpcResult<(Vec<u8>, PsdImportOptions)> {
    let path = crate::project::absolute_path(path)?;
    // 通常のファイルだけを読む (装置・FIFO・ディレクトリは読まない。`ugoe_fs::read_regular_file`)。
    let bytes = read_regular_file(&path, MAX_PSD_BYTES).map_err(|e| EditorError::io(&path, e))?;
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
pub async fn import_psd(
    app: AppHandle,
    path: String,
    mesh_options: Option<AutoMeshOptions>,
) -> IpcResult<Vec<PsdImportWarning>> {
    blocking(app, move |state| {
        crate::access::readable_path(state, &path)?;
        import_psd_in(state, &path, mesh_options)
    })
    .await
}

/// [`import_psd`] の本体。PSD のデコードと自動メッシュはロックの外で、モデルの写しに対して行い、適用だけを
/// ロックの中で行う。作る間にモデルが変わっていたら適用せずに [`EditorError::ModelChanged`]。
pub(crate) fn import_psd_in(
    state: &AppState,
    path: &str,
    mesh_options: Option<AutoMeshOptions>,
) -> IpcResult<Vec<PsdImportWarning>> {
    let (bytes, options) = read_psd(path, mesh_options)?;
    let (model, base) = {
        let (document, session) = lock_both(state)?;
        (document.model().clone(), Base::of(&document, &session))
    };
    let mut import = import::import_psd(&model, &bytes, &options)?;
    let (mut document, mut session) = lock_both(state)?;
    base.check_edit(&document, &session)?;
    // モジュールのプロジェクトでは、描画順 (0〜1000) をスロットの範囲に写す。最上位の要素は適用のときに
    // 見た目を保ったままスロットの取り付け先の下へ移る (`docs/editor.md#モジュールのプロジェクト`)。
    if let Some(context) = document.module() {
        context.map_new_meshes(&mut import.command);
    }
    document.apply(import.command)?;
    // AI の提案 (自動リギング) の元にする PSD を覚えておく。
    session
        .proposals
        .set_psd(crate::project::absolute_path(path)?);
    Ok(import.warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 通常のファイルでなければ (ディレクトリ・装置・FIFO) 読まない (T0011)。
    #[test]
    fn reads_only_regular_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let folder = dir.join("folder.psd");
        std::fs::create_dir_all(&folder).unwrap();
        let error = read_psd(&folder.display().to_string(), None).unwrap_err();
        assert_eq!(error.code, "editor.io");
        assert!(
            error.message.contains("not a regular file"),
            "{}",
            error.message
        );
        #[cfg(unix)]
        {
            let error = read_psd("/dev/zero", None).unwrap_err();
            assert!(
                error.message.contains("not a regular file"),
                "{}",
                error.message
            );
        }
        let file = dir.join("character.psd");
        std::fs::write(&file, b"8BPS").unwrap();
        let (bytes, options) = read_psd(&file.display().to_string(), None).unwrap();
        assert_eq!(bytes, b"8BPS");
        assert_eq!(options.name, "character");
    }
}
