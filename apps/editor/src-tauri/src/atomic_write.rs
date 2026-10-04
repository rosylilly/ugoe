//! ファイルを、途中で失敗しても前の中身を壊さずに置き換える (T0687)。
//!
//! 行き先と同じディレクトリの一時ファイル `.<名前>.<プロセス ID>.<連番>.tmp` に書いて `sync_all` し、
//! 閉じてから名前を変えて (`std::fs::rename`) 行き先を置き換える。書く・`sync`・名前の変更のどこで失敗しても
//! 一時ファイルを消し、行き先にある前のファイルはそのまま残る。形はビューア (`apps/viewer/src-tauri/src/atomic_write.rs`、
//! T0728) と MCP サーバの `StagedFile` (`crates/ugoe-mcp/src/host.rs`、T0552) と同じ。
//!
//! - 同じディレクトリに置くのは、名前の変更が同じファイルシステムの中で 1 回で済む (別のファイルシステムへは
//!   名前を変えられない) ため。先頭の `.` で、Unix の一覧では隠れる。
//! - Windows の `std::fs::rename` は `MoveFileExW` (`MOVEFILE_REPLACE_EXISTING`) で、既にあるファイルも置き換える。
//!   行き先をほかのアプリが削除の共有なしで開いているなどで置き換えられなければ、エラーを返して前のファイルを残す。
//! - [`Access::Private`] は、一時ファイルを作るときから利用者だけが読める権限 (Unix の `0o600`) にする。
//!   名前を変えると行き先はその権限を引き継ぐので、前のファイルの権限が広くても狭くなる。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// 書くファイルの権限。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    /// ふつうの権限 (umask に従う)。
    Default,
    /// 利用者だけが読み書きできる (Unix の `0o600`。トークンを含むファイル)。Windows では既定のまま。
    Private,
}

/// `bytes` を `path` に書く。失敗しても `path` にある前のファイルは残る (モジュールの説明)。
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    StagedFile::write(path, bytes, Access::Default)?.commit()
}

/// 置き換える前のファイル: 行き先と同じディレクトリの一時ファイルに書いておき、[`StagedFile::commit`] で
/// 名前を変えて置き換える。置き換えずに捨てると一時ファイルを消す。
///
/// ほかの保存 (プロジェクトの保存) と組にして、両方が書けてから置き換えるときに使う (MCP の `save` の `bundle`)。
pub(crate) struct StagedFile {
    temp: PathBuf,
    target: PathBuf,
    committed: bool,
}

impl StagedFile {
    /// `bytes` を一時ファイルに書いて `sync_all` し、閉じる。行き先はまだ変えない。
    pub(crate) fn write(target: &Path, bytes: &[u8], access: Access) -> std::io::Result<Self> {
        Self::write_with(target, access, |file| file.write_all(bytes))
    }

    /// [`Self::write`] の中身。一時ファイルへの書き込みを `write` に任せる (テストで途中の失敗を起こすため)。
    fn write_with(
        target: &Path,
        access: Access,
        write: impl FnOnce(&mut File) -> std::io::Result<()>,
    ) -> std::io::Result<Self> {
        // 作る前に組み立てておき、以後の失敗では drop が一時ファイルを消す。
        let staged = Self {
            temp: temp_path(target)?,
            target: target.to_path_buf(),
            committed: false,
        };
        let mut file = create(&staged.temp, access)?;
        write(&mut file)?;
        file.sync_all()?;
        // Windows では開いたままのファイルの名前を変えられないので、ここで閉じる。
        drop(file);
        Ok(staged)
    }

    /// 名前を変えて行き先を置き換える。失敗すれば一時ファイルを消し、前のファイルを残す。
    pub(crate) fn commit(mut self) -> std::io::Result<()> {
        std::fs::rename(&self.temp, &self.target)?;
        self.committed = true;
        Ok(())
    }

    /// 置き換える先のパス。
    pub(crate) fn target(&self) -> &Path {
        &self.target
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.temp);
        }
    }
}

/// 一時ファイルを新しく作る (同じ名前が残っていれば失敗する。連番で重ならない)。
fn create(temp: &Path, access: Access) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if access == Access::Private {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = access;
    options.open(temp)
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
            std::env::temp_dir().join(format!("ugoe-editor-atomic-{name}-{}", std::process::id()));
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
        let path = dir.join("model.ugoe");
        write_atomic(&path, b"new file").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new file");
        write_atomic(&path, b"next").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"next");
        assert_eq!(names(&dir), ["model.ugoe"]);
        let temp = temp_path(&path).unwrap();
        assert_eq!(temp.parent(), Some(dir.as_path()));
        let temp_name = temp.file_name().unwrap().to_string_lossy().into_owned();
        assert!(temp_name.starts_with(&format!(".model.ugoe.{}.", std::process::id())));
        assert!(temp_name.ends_with(".tmp"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_the_previous_file_when_writing_fails() {
        let dir = temp_dir("write-fails");
        let path = dir.join("model.ugoe");
        std::fs::write(&path, b"previous").unwrap();
        let err = StagedFile::write_with(&path, Access::Default, |file| {
            // 途中まで書いてから失敗する (ディスクがいっぱいなど)。
            file.write_all(b"half")?;
            Err(std::io::Error::other("disk full"))
        })
        .err()
        .unwrap();
        assert_eq!(err.to_string(), "disk full");
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(names(&dir), ["model.ugoe"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn removes_the_temporary_file_when_replacing_fails() {
        let dir = temp_dir("rename-fails");
        // 行き先が中身のあるディレクトリだと、名前を変えて置き換えられない。
        let path = dir.join("model.ugoe");
        std::fs::create_dir_all(path.join("inside")).unwrap();
        assert!(write_atomic(&path, b"bundle").is_err());
        assert!(path.join("inside").is_dir());
        assert_eq!(names(&dir), ["model.ugoe"]);
        // 行き先のディレクトリが無ければ、一時ファイルも作れずに失敗する。
        assert!(write_atomic(&dir.join("missing/model.ugoe"), b"bundle").is_err());
        assert_eq!(names(&dir), ["model.ugoe"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dropping_a_staged_file_keeps_the_previous_file() {
        let dir = temp_dir("drop");
        let path = dir.join("model.ugoe");
        std::fs::write(&path, b"previous").unwrap();
        let staged = StagedFile::write(&path, b"next", Access::Default).unwrap();
        assert_eq!(staged.target(), path);
        assert_eq!(names(&dir).len(), 2);
        drop(staged);
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(names(&dir), ["model.ugoe"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn private_files_are_readable_only_by_the_user() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("private");
        let path = dir.join("mcp-bridge.json");
        std::fs::write(&path, b"{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let staged = StagedFile::write(&path, b"{\"token\":1}", Access::Private).unwrap();
        // 名前を変える前の一時ファイルも、利用者だけが読める。
        let temp = dir.join(
            names(&dir)
                .into_iter()
                .find(|n| n.ends_with(".tmp"))
                .unwrap(),
        );
        let mode = std::fs::metadata(&temp).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        staged.commit().unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(names(&dir), ["mcp-bridge.json"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
