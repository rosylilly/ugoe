//! プレビューの着せ替え (T0649、`docs/editor.md#プレビューの着せ替え`): `.ugoemod` の読み込みと、試しているモジュールの一覧。
//! 付け外しは webview の中 (`ugoe-wasm` の `WebPuppet`) で行い、モデルは変えない。
//!
//! 一覧は `project.json` の `editor.dress_up` に残し、次に開いたときに読み込み直す (Undo の対象ではない)。
//! パスはプロジェクトの `.ugoeproj` と同じフォルダの中なら相対パス (`/` 区切り) で、それ以外は絶対パスで書く
//! (プロジェクトとモジュールをフォルダごと移しても読めるように)。

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::AppState;
use crate::commands::off_main;
use crate::error::{EditorError, IpcResult};
use crate::project::{ProjectSession, ProjectStatus, absolute_path, lock_both, status};
use ugoe_fs::read_regular_file;

/// 読む `.ugoemod` の大きさの上限 (ビューア・スタジオのモデルと同じ)。
const MAX_MODULE_BYTES: u64 = 512 * 1024 * 1024;

/// `project.json` の `editor` のうち、プレビューの着せ替えを置く項目。
const DRESS_UP_KEY: &str = "dress_up";

/// `.ugoemod` の拡張子。
const MODULE_EXTENSION: &str = "ugoemod";

/// プレビューで試すモジュール 1 つ (`editor.dress_up.modules` の要素)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PreviewModule {
    /// `.ugoemod` のパス (IPC では絶対パス)。
    pub path: String,
    /// プレビューで付けるか。
    #[serde(default)]
    pub attached: bool,
}

/// `editor.dress_up` の中身。足りないフィールドは既定値で読む。
#[derive(Debug, Default, Serialize, Deserialize)]
struct DressUpSettings {
    #[serde(default)]
    modules: Vec<PreviewModule>,
}

/// 残した一覧を、絶対パスにして読む。読めない値は空の一覧、相対パスで保存先がなければその要素を捨てる。
fn read_modules(value: Option<&serde_json::Value>, base: Option<&Path>) -> Vec<PreviewModule> {
    let settings: DressUpSettings = value
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    settings
        .modules
        .into_iter()
        .filter_map(|m| {
            let path = Path::new(&m.path);
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                base?.join(path)
            };
            Some(PreviewModule {
                path: path.display().to_string(),
                attached: m.attached,
            })
        })
        .collect()
}

/// 一覧を `editor.dress_up` の値にする。空なら `None` (項目を書かない)。
fn write_modules(modules: &[PreviewModule], base: Option<&Path>) -> Option<serde_json::Value> {
    if modules.is_empty() {
        return None;
    }
    let modules = modules
        .iter()
        .map(|m| PreviewModule {
            path: relative_path(Path::new(&m.path), base).unwrap_or_else(|| m.path.clone()),
            attached: m.attached,
        })
        .collect();
    serde_json::to_value(DressUpSettings { modules }).ok()
}

/// `path` が `base` の中にあれば、`base` からの相対パス (`/` 区切り)。
fn relative_path(path: &Path, base: Option<&Path>) -> Option<String> {
    let rest = path.strip_prefix(base?).ok()?;
    let parts: Option<Vec<&str>> = rest
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    parts.filter(|p| !p.is_empty()).map(|p| p.join("/"))
}

/// 相対パスの基準: 保存先の `.ugoeproj` を置いたフォルダ。
fn base_dir(project: Option<&Path>) -> Option<PathBuf> {
    project.and_then(Path::parent).map(Path::to_path_buf)
}

fn is_module_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(MODULE_EXTENSION))
}

/// プロジェクトに残した、プレビューで試すモジュールの一覧 (パスは絶対パス)。
#[tauri::command]
pub fn get_preview_modules(state: State<'_, AppState>) -> IpcResult<Vec<PreviewModule>> {
    let (_document, session) = lock_both(&state)?;
    Ok(modules(&session))
}

/// `session` に残した一覧 (パスは絶対パス)。
pub(crate) fn modules(session: &ProjectSession) -> Vec<PreviewModule> {
    let base = base_dir(session.path());
    read_modules(session.editor_value(DRESS_UP_KEY), base.as_deref())
}

/// 保存先を `project` (`.ugoeproj`) に替える前に、一覧を `project` のフォルダーからの相対パスで書き直す (Save As)。
pub(crate) fn move_to(session: &mut ProjectSession, project: &Path) {
    let modules = modules(session);
    let value = write_modules(&modules, project.parent());
    session.set_editor_value(DRESS_UP_KEY, value);
}

/// `session` に一覧を残す (パスは絶対パス。保存先のフォルダーの中なら相対パスで書く)。
pub(crate) fn set_modules(session: &mut ProjectSession, modules: &[PreviewModule]) {
    let base = base_dir(session.path());
    let value = write_modules(modules, base.as_deref());
    session.set_editor_value(DRESS_UP_KEY, value);
}

/// プレビューで試すモジュールの一覧をプロジェクトに残す (保存するまでファイルには書かない。変われば未保存の印が付く)。
/// パスは絶対パスだけを受け付ける。
#[tauri::command]
pub fn set_preview_modules(
    state: State<'_, AppState>,
    modules: Vec<PreviewModule>,
) -> IpcResult<ProjectStatus> {
    for m in &modules {
        absolute_path(&m.path)?;
    }
    let (document, mut session) = lock_both(&state)?;
    set_modules(&mut session, &modules);
    Ok(status(&session, &document))
}

/// `.ugoemod` のバイト列を返す (ファイルのダイアログで選んだ・ドロップした・プロジェクトに残したもの)。
/// `.ugoemod` だけを読む。中身の検査と付け外しは webview の中の `WebPuppet.load_module` が行う。
#[tauri::command]
pub async fn read_module_file(path: String) -> IpcResult<tauri::ipc::Response> {
    off_main(move || read_module(&path).map(tauri::ipc::Response::new)).await
}

/// [`read_module_file`] の中身。通常のファイルだけを読む (装置・FIFO・ディレクトリは読まない。`ugoe_fs::read_regular_file`)。
fn read_module(path: &str) -> IpcResult<Vec<u8>> {
    let path = absolute_path(path)?;
    if !is_module_path(&path) {
        return Err(EditorError::NotModuleFile(path.display().to_string()).into());
    }
    Ok(read_regular_file(&path, MAX_MODULE_BYTES).map_err(|e| EditorError::io(&path, e))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `.ugoemod` の名前でも、通常のファイルでなければ読まない (T0011)。
    #[test]
    fn reads_only_regular_module_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let file = dir.join("hair.ugoemod");
        let folder = dir.join("folder.ugoemod");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&file, b"module").unwrap();
        assert_eq!(read_module(&file.display().to_string()).unwrap(), b"module");
        let error = read_module(&folder.display().to_string()).unwrap_err();
        assert_eq!(error.code, "editor.io");
        assert!(
            error.message.contains("not a regular file"),
            "{}",
            error.message
        );
        let other = dir.join("notes.txt");
        assert_eq!(
            read_module(&other.display().to_string()).unwrap_err().code,
            "editor.not_module_file"
        );
    }

    fn module(path: &str, attached: bool) -> PreviewModule {
        PreviewModule {
            path: path.into(),
            attached,
        }
    }

    #[test]
    fn stores_paths_next_to_the_project_as_relative_paths() {
        let root = std::env::temp_dir().join("ugoe-dress-up");
        let base = root.join("work");
        let inside = base.join("mods").join("hair.ugoemod");
        let outside = root.join("other").join("dress.ugoemod");
        let modules = vec![
            module(&inside.display().to_string(), true),
            module(&outside.display().to_string(), false),
        ];
        let value = write_modules(&modules, Some(&base)).unwrap();
        assert_eq!(value["modules"][0]["path"], "mods/hair.ugoemod");
        assert_eq!(value["modules"][0]["attached"], true);
        assert_eq!(value["modules"][1]["path"], outside.display().to_string());
        assert_eq!(read_modules(Some(&value), Some(&base)), modules);
    }

    #[test]
    fn empty_list_is_not_written_and_bad_values_read_as_empty() {
        assert_eq!(write_modules(&[], None), None);
        assert!(read_modules(Some(&serde_json::json!("x")), None).is_empty());
        assert!(read_modules(None, None).is_empty());
        // 相対パスは保存先がなければ読めない。attached の欠けは付けない扱い。
        let value = serde_json::json!({ "modules": [{ "path": "hair.ugoemod" }] });
        assert!(read_modules(Some(&value), None).is_empty());
        let base = std::env::temp_dir();
        assert_eq!(
            read_modules(Some(&value), Some(&base)),
            vec![module(
                &base.join("hair.ugoemod").display().to_string(),
                false
            )]
        );
    }

    #[test]
    fn reads_module_files_only() {
        use tauri::async_runtime::block_on;
        assert!(is_module_path(Path::new("Hair.UGOEMOD")));
        assert!(!is_module_path(Path::new("avatar.ugoe")));
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let text = dir.join("notes.txt");
        std::fs::write(&text, "x").unwrap();
        assert!(block_on(read_module_file(text.display().to_string())).is_err());
        assert!(block_on(read_module_file("hair.ugoemod".into())).is_err());
        assert!(
            block_on(read_module_file(
                dir.join("missing.ugoemod").display().to_string()
            ))
            .is_err()
        );
        let file = dir.join("hair.ugoemod");
        std::fs::write(&file, [1, 2, 3]).unwrap();
        assert!(block_on(read_module_file(file.display().to_string())).is_ok());
    }
}
