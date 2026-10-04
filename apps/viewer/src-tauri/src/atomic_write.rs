//! ファイルを、途中で失敗しても前の中身を壊さずに置き換える (T0728)。
//!
//! 行き先と同じディレクトリの一時ファイル `.<名前>.<プロセス ID>.<連番>.tmp` に書いて `sync_all` し、
//! 名前を変えて (`std::fs::rename`) 行き先を置き換える。書く・`sync`・名前の変更のどこで失敗しても
//! 一時ファイルを消し、行き先にある前のファイルはそのまま残る。形は MCP サーバの `StagedFile`
//! (`crates/ugoe-mcp/src/host.rs`、T0552) と同じ。
//!
//! - 同じディレクトリに置くのは、名前の変更が同じファイルシステムの中で 1 回で済む (別のファイルシステムへは
//!   名前を変えられない) ため。先頭の `.` で、Unix の一覧では隠れる。
//! - Windows の `std::fs::rename` は `MoveFileExW` (`MOVEFILE_REPLACE_EXISTING`) で、既にあるファイルも置き換える。
//!   行き先をほかのアプリが削除の共有なしで開いているなどで置き換えられなければ、エラーを返して前のファイルを残す。

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// `bytes` を `path` に書く。失敗しても `path` にある前のファイルは残る (モジュールの説明)。
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_atomic_with(path, |file| file.write_all(bytes))
}

/// [`write_atomic`] の中身。一時ファイルへの書き込みを `write` に任せる (テストで途中の失敗を起こすため)。
fn write_atomic_with(
    path: &Path,
    write: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let temp = temp_path(path)?;
    let result = (|| {
        let mut file = File::create(&temp)?;
        write(&mut file)?;
        file.sync_all()?;
        // Windows では開いたままのファイルの名前を変えられないので、先に閉じる。
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// `path` と同じディレクトリの一時ファイルのパス。
fn temp_path(path: &Path) -> std::io::Result<PathBuf> {
    /// 同じプロセスの中で同じ行き先に同時に書いても、一時ファイルが重ならないように。
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(path.with_file_name(temp_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ugoe-viewer-atomic-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn replaces_the_file_through_a_temporary_file() {
        let dir = temp_dir("replace");
        let path = dir.join("rec.json");
        write_atomic(&path, b"new file").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new file");
        write_atomic(&path, b"{}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{}");
        assert_eq!(names(&dir), ["rec.json"]);
        let temp = temp_path(&path).unwrap();
        assert_eq!(temp.parent(), Some(dir.as_path()));
        assert!(
            temp.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".rec.json.")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_the_previous_file_when_writing_fails() {
        let dir = temp_dir("write-fails");
        let path = dir.join("shot.png");
        std::fs::write(&path, b"previous").unwrap();
        let err = write_atomic_with(&path, |file| {
            // 途中まで書いてから失敗する (ディスクがいっぱいなど)。
            file.write_all(b"half")?;
            Err(std::io::Error::other("disk full"))
        })
        .unwrap_err();
        assert_eq!(err.to_string(), "disk full");
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(names(&dir), ["shot.png"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn removes_the_temporary_file_when_replacing_fails() {
        let dir = temp_dir("rename-fails");
        // 行き先が中身のあるディレクトリだと、名前を変えて置き換えられない。
        let path = dir.join("rec.json");
        std::fs::create_dir_all(path.join("inside")).unwrap();
        assert!(write_atomic(&path, b"{}").is_err());
        assert!(path.join("inside").is_dir());
        assert_eq!(names(&dir), ["rec.json"]);
        // 行き先のディレクトリが無ければ、一時ファイルも作れずに失敗する。
        assert!(write_atomic(&dir.join("missing/rec.json"), b"{}").is_err());
        assert_eq!(names(&dir), ["rec.json"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
