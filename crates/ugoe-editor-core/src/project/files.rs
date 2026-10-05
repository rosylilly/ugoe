//! ファイルの読み書きの補助 (JSON・一時ファイル経由の書き込み・古いファイルの削除)。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use ugoe_format::FormatError;

use super::{ProjectError, read_regular_file};

pub(crate) fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> ProjectError + '_ {
    move |source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// 2 スペースで整形した JSON (末尾に改行)。フィールドは型定義の順になる。
/// `path` は書き込み先 (エラーの表示用。プロジェクトからの相対パス)。
pub(super) fn to_json<T: Serialize>(path: &Path, value: &T) -> Result<Vec<u8>, ProjectError> {
    serde_json::to_string_pretty(value)
        .map(with_newline)
        .map_err(|source| ProjectError::Json {
            path: path.to_path_buf(),
            source,
        })
}

/// 未知の値 (新しい版のファイルから読んだ `Unknown`) は書けないので、`model.json` と同じく
/// [`FormatError::UnknownValues`] で拒否する。`unknown` はその場所の一覧。
pub(super) fn known(path: &Path, unknown: Vec<String>) -> Result<(), ProjectError> {
    if unknown.is_empty() {
        return Ok(());
    }
    Err(ProjectError::Format {
        path: path.to_path_buf(),
        source: FormatError::UnknownValues(unknown),
    })
}

pub(super) fn with_newline(mut json: String) -> Vec<u8> {
    json.push('\n');
    json.into_bytes()
}

/// `path` の JSON を読む。通常のファイルで `max` バイト以下のときだけ読む ([`read_regular_file`])。
pub(super) fn read_json<T: DeserializeOwned>(path: &Path, max: u64) -> Result<T, ProjectError> {
    let bytes = read_regular_file(path, max)?;
    serde_json::from_slice(&bytes).map_err(|source| ProjectError::Json {
        path: path.to_path_buf(),
        source,
    })
}

/// `dir` の中の `*<suffix>` を、ファイル名の順に読む。`dir` がなければ空。
///
/// 読んだ値はそれぞれ `check` で検査し、通らなければ [`ProjectError::Format`] (場所はそのファイル)。
/// ファイル 1 つの大きさの上限は `max` ([`read_json`])。
pub(super) fn read_json_dir<T: DeserializeOwned>(
    dir: &Path,
    suffix: &str,
    max: u64,
    check: impl Fn(&T) -> Result<(), FormatError>,
) -> Result<Vec<T>, ProjectError> {
    list_files(dir, suffix)?
        .iter()
        .map(|path| {
            let value = read_json(path, max)?;
            check(&value).map_err(|source| ProjectError::Format {
                path: path.clone(),
                source,
            })?;
            Ok(value)
        })
        .collect()
}

/// `dir` の中の、名前が `suffix` で終わるファイルをファイル名の順に返す。`dir` がなければ空。
pub(crate) fn list_files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, ProjectError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_error(dir))? {
        let entry = entry.map_err(io_error(dir))?;
        let path = entry.path();
        let matches = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(suffix) && !name.starts_with('.'));
        if matches && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// `dir/sub` の中の `*<suffix>` のうち、今回書いたもの (`files`) 以外を消す。
///
/// 大文字小文字だけが違う名前で書いたファイル (`idle` から `Idle` への名前の変更など) は、大文字小文字を
/// 区別しないファイルシステム (Windows・macOS) では今消したファイルと同じものなので、消した後に書き直す
/// (区別するファイルシステムでは中身が同じなので書き直さない)。
pub(super) fn remove_stale(
    dir: &Path,
    sub: &str,
    suffix: &str,
    files: &[(PathBuf, Vec<u8>)],
) -> Result<(), ProjectError> {
    let folded = |path: &Path| path.to_string_lossy().to_ascii_lowercase();
    for path in list_files(&dir.join(sub), suffix)? {
        let relative = path.strip_prefix(dir).unwrap_or(&path);
        if files.iter().any(|(p, _)| p == relative) {
            continue;
        }
        remove_if_exists(&path)?;
        let same_name = files.iter().find(|(p, _)| folded(p) == folded(relative));
        if let Some((written, bytes)) = same_name {
            write_if_changed(&dir.join(written), bytes)?;
        }
    }
    Ok(())
}

pub(super) fn remove_if_exists(path: &Path) -> Result<(), ProjectError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(ProjectError::Io {
            path: path.to_path_buf(),
            source: e,
        }),
        _ => Ok(()),
    }
}

/// 中身が変わるときだけ、同じディレクトリの一時ファイル (`.<名前>.<プロセス ID>.<連番>.tmp`) に書いてから
/// 名前を変えて置き換える (`ugoe_fs::write_atomic`)。一時ファイルは `.` で始まり `.tmp` で終わるので、
/// 書き込みに失敗して残っても読み込み・指紋 ([`list_files`]) には拾われない。
///
/// 今のファイルは、通常のファイルで書く中身と同じ大きさのときだけ読んで比べる (FIFO・装置を読んで止まらないように。T1607)。
/// それ以外は比べずに置き換える。
pub(super) fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let same_len = fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == bytes.len() as u64);
    if same_len && read_regular_file(path, bytes.len() as u64).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error(parent))?;
    }
    ugoe_fs::write_atomic(path, bytes).map_err(io_error(path))
}
