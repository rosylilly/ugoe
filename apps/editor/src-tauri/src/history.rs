//! Undo / Redo の状態と、Undo の上限の設定 (エディタ全体の設定。`docs/editor.md#undo`)。
//!
//! 上限は起動時にアプリの設定のディレクトリの [`SETTINGS_FILE`] から読み、変えるたびに書く。開いているドキュメントに
//! 当て、プロジェクトを開き直したとき (別のドキュメントに置き換えたとき) は前のドキュメントの上限を引き継ぐ
//! (`project::replace_document`)。

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_editor_core::Document;
use ugoe_editor_core::document::DEFAULT_UNDO_LIMIT;

use crate::AppState;
use crate::commands::{blocking, lock};
use crate::error::{EditorError, IpcResult};

/// 設定のファイルの名前 (アプリの設定のディレクトリの中)。
const SETTINGS_FILE: &str = "editor-settings.json";

/// 選べる Undo の上限。0 (履歴なし) は誤った編集を戻せなくなるので選ばせない。
pub const UNDO_LIMIT_RANGE: std::ops::RangeInclusive<u32> = 1..=1000;

/// 編集の履歴の状態 (`history_status` の戻り値)。ボタンを押せるかと設定の画面に使う。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct HistoryStatus {
    /// Undo で戻れるか。
    pub can_undo: bool,
    /// Redo で進めるか。
    pub can_redo: bool,
    /// 続けて Undo できる数。
    pub undo_count: u32,
    /// 続けて Redo できる数。
    pub redo_count: u32,
    /// 覚える履歴 (Undo と Redo を合わせた数) の上限。
    pub undo_limit: u32,
}

impl HistoryStatus {
    pub fn of(document: &Document) -> Self {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Self {
            can_undo: document.can_undo(),
            can_redo: document.can_redo(),
            undo_count: count(document.undo_count()),
            redo_count: count(document.redo_count()),
            undo_limit: count(document.undo_limit()),
        }
    }
}

/// [`SETTINGS_FILE`] の中身。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct EditorSettings {
    undo_limit: u32,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            undo_limit: DEFAULT_UNDO_LIMIT as u32,
        }
    }
}

fn check_undo_limit(limit: u32) -> Result<(), EditorError> {
    if UNDO_LIMIT_RANGE.contains(&limit) {
        Ok(())
    } else {
        Err(EditorError::InvalidUndoLimit {
            min: *UNDO_LIMIT_RANGE.start(),
            max: *UNDO_LIMIT_RANGE.end(),
        })
    }
}

/// Undo の上限の置き場。起動時に [`UndoLimitStore::load`] でファイルを読み、変えるたびに書く。
#[derive(Debug, Default)]
pub struct UndoLimitStore {
    path: OnceLock<PathBuf>,
    value: Mutex<EditorSettings>,
}

impl UndoLimitStore {
    /// `dir` (アプリの設定のディレクトリ) のファイルを読む。なければ既定のまま。
    /// 読めない・範囲の外なら既定のままエラーを返す (起動は止めない)。
    pub fn load(&self, dir: PathBuf) -> Result<(), EditorError> {
        let path = dir.join(SETTINGS_FILE);
        let _ = self.path.set(path.clone());
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(EditorError::io(&path, e)),
        };
        let settings: EditorSettings =
            serde_json::from_str(&text).map_err(|e| EditorError::io(&path, e))?;
        check_undo_limit(settings.undo_limit)?;
        *self.lock() = settings;
        Ok(())
    }

    /// 今の上限。
    pub fn get(&self) -> usize {
        self.lock().undo_limit as usize
    }

    /// 検査してから置き換え、ファイルに書く (ファイルの場所が決まっていれば)。
    pub fn set(&self, limit: u32) -> Result<(), EditorError> {
        check_undo_limit(limit)?;
        let settings = EditorSettings { undo_limit: limit };
        if let Some(path) = self.path.get() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| EditorError::io(dir, e))?;
            }
            let mut text = serde_json::to_string_pretty(&settings)
                .map_err(|e| EditorError::Internal(e.to_string()))?;
            text.push('\n');
            crate::atomic_write::write_atomic(path, text.as_bytes())
                .map_err(|e| EditorError::io(path, e))?;
        }
        *self.lock() = settings;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, EditorSettings> {
        self.value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// 編集の履歴の状態。編集・Undo・Redo の後に画面が読み直す。
#[tauri::command]
pub fn history_status(state: State<'_, AppState>) -> IpcResult<HistoryStatus> {
    Ok(HistoryStatus::of(&*lock(&state)?))
}

/// Undo の上限を変えて覚え (次の起動でも使う)、開いているドキュメントに当てる。範囲は [`UNDO_LIMIT_RANGE`]。
/// 今の履歴が超えていれば古いものから捨てる。変えた後の履歴の状態を返す。
#[tauri::command]
pub async fn set_undo_limit(app: AppHandle, limit: u32) -> IpcResult<HistoryStatus> {
    blocking(app, move |state| apply_undo_limit(state, limit)).await
}

fn apply_undo_limit(state: &AppState, limit: u32) -> IpcResult<HistoryStatus> {
    state.undo_limit.set(limit)?;
    let mut document = lock(state)?;
    document.set_undo_limit(limit as usize);
    Ok(HistoryStatus::of(&document))
}

#[cfg(test)]
mod tests {
    use ugoe_editor_core::Command;

    use super::*;

    /// テスト用の一時ディレクトリ (前の実行の残りは消す)。
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ugoe-editor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn store_in(dir: &std::path::Path) -> UndoLimitStore {
        let store = UndoLimitStore::default();
        store.load(dir.to_path_buf()).unwrap();
        store
    }

    #[test]
    fn the_limit_is_remembered_in_the_settings_file() {
        let dir = temp_dir("undo-limit");
        let store = store_in(&dir);
        assert_eq!(store.get(), DEFAULT_UNDO_LIMIT);
        store.set(20).unwrap();
        assert_eq!(store.get(), 20);
        // 次の起動で読む。
        assert_eq!(store_in(&dir).get(), 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn limits_out_of_the_range_are_rejected() {
        let dir = temp_dir("undo-limit-range");
        let store = store_in(&dir);
        for limit in [0, *UNDO_LIMIT_RANGE.end() + 1] {
            let err = store.set(limit).unwrap_err();
            assert!(matches!(
                err,
                EditorError::InvalidUndoLimit { min: 1, max: 1000 }
            ));
        }
        assert_eq!(store.get(), DEFAULT_UNDO_LIMIT);
        // ファイルが範囲の外なら既定のまま。
        // ファイルを書く前に作っておく (範囲の外では書かない)。
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"undo_limit":0}"#).unwrap();
        let store = UndoLimitStore::default();
        assert!(store.load(dir.clone()).is_err());
        assert_eq!(store.get(), DEFAULT_UNDO_LIMIT);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn setting_the_limit_trims_the_open_document() {
        let state = crate::ai::tests::state(None);
        let set_default = |default: f32| Command::SetParameterRange {
            id: "X".into(),
            min: -1.0,
            default,
            max: 1.0,
        };
        {
            let mut document = state.document.lock().unwrap();
            *document = Document::new(ugoe_model::fixtures::single_quad());
            for i in 1..=3 {
                document.apply(set_default(i as f32 * 0.1)).unwrap();
            }
            assert!(document.undo());
        }
        let status = apply_undo_limit(&state, 2).unwrap();
        assert_eq!(
            status,
            HistoryStatus {
                can_undo: true,
                can_redo: true,
                undo_count: 1,
                redo_count: 1,
                undo_limit: 2,
            }
        );
        assert!(apply_undo_limit(&state, 0).is_err());
        assert_eq!(state.document.lock().unwrap().undo_limit(), 2);
    }
}
