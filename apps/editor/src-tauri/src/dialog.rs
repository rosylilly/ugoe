//! OS のファイルのダイアログ (T1616。`docs/editor.md#ファイルダイアログ`)。ダイアログをバックエンドで開くのは、利用者が
//! 選んだパスをバックエンドが知り、コマンドが読み書きできるパスをそれに絞るため (`crate::access`。ビューア・スタジオと同じ)。
//! 文言 (`title`)・最初の場所・拡張子のフィルタは画面が渡す。

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{Runtime, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};
use ugoe_editor_core::Message;

use crate::AppState;
use crate::commands::off_main;
use crate::error::{EditorError, IpcResult};

/// ダイアログの拡張子のフィルタ (`src/lib/filePaths.ts` の `FileFilter`)。`extensions` は `.` を付けない。
#[derive(Debug, Clone, Deserialize)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// 開くダイアログを開き、選ばれたパスを返す (取り消したら `null`)。`directory` ならフォルダーを選ぶ (`.ugoeproj` は
/// ディレクトリ)。`default_path` は最初の場所 (ディレクトリか、ファイル名まで含むパス)。選ばれたパスは読めるパスに加える。
#[tauri::command]
pub async fn open_file_dialog(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
    default_path: Option<String>,
    filters: Vec<FileFilter>,
    directory: bool,
) -> IpcResult<Option<String>> {
    let picked = off_main(move || {
        if let Some(answer) = test_answer(if directory {
            "open-directory"
        } else {
            "open-file"
        }) {
            return Ok(answer);
        }
        let dialog = builder(&window, title, default_path, &filters);
        // tauri-plugin-dialog の `open` と同じく、親のウィンドウは Windows と macOS でだけ付ける。
        #[cfg(any(windows, target_os = "macos"))]
        let dialog = dialog.set_parent(&window);
        let picked = if directory {
            dialog.blocking_pick_folder()
        } else {
            dialog.blocking_pick_file()
        };
        picked
            .map(|p| p.into_path().map_err(dialog_failed))
            .transpose()
    })
    .await?;
    let Some(path) = picked.map(path_string) else {
        return Ok(None);
    };
    state.files.grant_readable([PathBuf::from(&path)]);
    Ok(Some(path))
}

/// 保存のダイアログを開き、選ばれたパスを返す (取り消したら `null`)。名前に最初のフィルタの最初の拡張子が無ければ付ける
/// (Linux のダイアログはフィルタの拡張子を付けずに返すことがある)。返したパスは 1 回だけ書ける (`crate::access`)。
#[tauri::command]
pub async fn save_file_dialog(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
    default_path: Option<String>,
    filters: Vec<FileFilter>,
) -> IpcResult<Option<String>> {
    let extension = filters.first().and_then(|f| f.extensions.first()).cloned();
    let picked = off_main(move || {
        if let Some(answer) = test_answer("save-file") {
            return Ok(answer);
        }
        builder(&window, title, default_path, &filters)
            .set_can_create_directories(true)
            .set_parent(&window)
            .blocking_save_file()
            .map(|p| p.into_path().map_err(dialog_failed))
            .transpose()
    })
    .await?;
    let Some(mut path) = picked.map(path_string) else {
        return Ok(None);
    };
    if let Some(extension) = extension {
        path = with_extension(path, &extension);
    }
    state.files.grant_writable(PathBuf::from(&path));
    Ok(Some(path))
}

/// 題・最初の場所・フィルタを付けたダイアログ。
fn builder<R: Runtime>(
    window: &WebviewWindow<R>,
    title: String,
    default_path: Option<String>,
    filters: &[FileFilter],
) -> FileDialogBuilder<R> {
    let mut dialog = window.dialog().file().set_title(title);
    if let Some(path) = default_path.filter(|p| !p.is_empty()) {
        dialog = with_default_path(dialog, Path::new(&path));
    }
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }
    dialog
}

/// 最初の場所を付ける。ディレクトリならそこを開き、そうでなければ (まだ無いファイルの名前も) 親を開いて名前を入れておく
/// (`@tauri-apps/plugin-dialog` の `defaultPath` と同じ)。
fn with_default_path<R: Runtime>(
    dialog: FileDialogBuilder<R>,
    path: &Path,
) -> FileDialogBuilder<R> {
    if path.is_dir() {
        return dialog.set_directory(path);
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => {
            let dialog = dialog.set_file_name(name.to_string_lossy());
            if parent.as_os_str().is_empty() {
                dialog
            } else {
                dialog.set_directory(parent)
            }
        }
        _ => dialog.set_directory(path),
    }
}

fn dialog_failed(error: impl ToString) -> Message {
    EditorError::DialogFailed(error.to_string()).into()
}

/// 拡張子 (大文字小文字は区別しない) が無ければ付け足す。画面の `withExtension` (`src/lib/filePaths.ts`) と同じ規則
/// (画面が付け直しても同じパスになり、書けるパスと食い違わない)。
fn with_extension(path: String, extension: &str) -> String {
    let suffix = format!(".{}", extension.to_lowercase());
    if path.to_lowercase().ends_with(&suffix) {
        path
    } else {
        format!("{path}.{extension}")
    }
}

/// 画面に渡すパスの文字列。画面はこの文字列のまま返してくるので、許したパスもこの文字列から作る。
fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

/// e2e (tests/e2e) の差し替えの答えを置くディレクトリを指す環境変数 (デバッグビルドだけ)。
#[cfg(debug_assertions)]
const TEST_ANSWERS_ENV: &str = "UGOE_EDITOR_DIALOG_ANSWERS";

/// e2e の差し替え (デバッグビルドで環境変数 [`TEST_ANSWERS_ENV`] があるときだけ): OS のダイアログを開かず、そのディレクトリの
/// `kind` (`open-file`・`open-directory`・`save-file`) のファイルに書いたパスを選んだことにする (読んだら消す。無ければ取り消し)。
/// 環境変数はアプリを起動する側が決めるので、webview はこれで選ばせるパスを決められない。
/// 返したパスは OS のダイアログで選んだものと同じく、読める・書けるパスになる。
#[cfg(debug_assertions)]
fn test_answer(kind: &str) -> Option<Option<PathBuf>> {
    let dir = std::env::var_os(TEST_ANSWERS_ENV)?;
    let file = Path::new(&dir).join(kind);
    let answer = std::fs::read_to_string(&file).ok();
    let _ = std::fs::remove_file(&file);
    Some(answer.map(|a| PathBuf::from(a.trim_end_matches(['\r', '\n']))))
}

#[cfg(not(debug_assertions))]
fn test_answer(_kind: &str) -> Option<Option<PathBuf>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_extension_the_save_dialog_left_out() {
        assert_eq!(with_extension("/a/shot".into(), "png"), "/a/shot.png");
        assert_eq!(with_extension("/a/shot.PNG".into(), "png"), "/a/shot.PNG");
        assert_eq!(
            with_extension("/a/b.ugoe".into(), "ugoeproj"),
            "/a/b.ugoe.ugoeproj"
        );
        // 名前が拡張子だけでも、画面の `withExtension` と同じく付けない
        assert_eq!(with_extension("/a/.json".into(), "json"), "/a/.json");
    }
}
