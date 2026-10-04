//! IPC で受けたパスのファイルを、通常のファイルのときだけ、大きさの上限までで読む (T0011)。
//!
//! 装置 (`/dev/zero`)・FIFO・ディレクトリを読むと、終わらない・メモリを使い果たす・止まる。webview が渡すパスは
//! 利用者が選んだものでも、名前だけでは中身の種類が分からない (`x.ugoe` という FIFO・`/dev/zero` へのリンク)。
//! 線引きは T0530 の案のとおり: サーバやアプリの IPC で受けるパスは通常のファイルだけを読み、利用者が自分で渡す
//! CLI の引数は FIFO を読めるままにする。3 つのアプリで同じ形の写しを持つ (`apps/{editor,viewer,studio}/src-tauri/src/read_file.rs`。
//! アプリどうしで共有するクレートが無いため。直すときは 3 つとも直す)。MCP サーバ・ugoe-headless の `read_file` と同じ考え。
//!
//! - 大きさを見てから読むまでの間にファイルが大きくなっても上限を超えて読まないよう、上限 + 1 バイトまでに限って読む。
//! - 開いた後にも、開いたものが通常のファイルかを確かめる (見てから開くまでの間に差し替えられたとき)。
//!   ただし見てから開くまでの間に FIFO へ差し替えられると、開くところで止まりうる (その窓は小さいので受け入れる)。

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 読めなかった理由。
#[derive(Debug)]
pub(crate) enum ReadError {
    /// 読めない (無い・権限が無いなど)。通常のファイルでなければ `InvalidInput` ([`not_regular_file`])。
    Io(std::io::Error),
    /// 上限より大きい。`len` は分かった大きさ (読みながら超えたときは読んだバイト数)。
    TooLarge { len: u64, max: u64 },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::TooLarge { len, max } => {
                write!(
                    f,
                    "file is too large ({len} bytes; the limit is {max} bytes)"
                )
            }
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// 通常のファイルでないときのエラー。
fn not_regular_file() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a regular file")
}

/// `path` を読む。通常のファイル (リンクは辿る) でなければ読まずにエラー、`max` バイトより大きければ `TooLarge`。
pub(crate) fn read_regular_file(path: &Path, max: u64) -> Result<Vec<u8>, ReadError> {
    // 開く前に見る (FIFO は開くだけで書き手を待って止まる)。
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(not_regular_file().into());
    }
    if metadata.len() > max {
        return Err(ReadError::TooLarge {
            len: metadata.len(),
            max,
        });
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(not_regular_file().into());
    }
    let len = metadata.len();
    if len > max {
        return Err(ReadError::TooLarge { len, max });
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(ReadError::TooLarge {
            len: bytes.len() as u64,
            max,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ugoe-read-file-{name}-{}-{}",
            env!("CARGO_PKG_NAME"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_regular_files_up_to_the_limit() {
        let dir = temp_dir("regular");
        let path = dir.join("a.bin");
        std::fs::write(&path, b"12345").unwrap();
        assert_eq!(read_regular_file(&path, 5).unwrap(), b"12345");
        assert!(matches!(
            read_regular_file(&path, 4),
            Err(ReadError::TooLarge { len: 5, max: 4 })
        ));
        let missing = read_regular_file(&dir.join("missing.bin"), 5).unwrap_err();
        assert!(
            matches!(&missing, ReadError::Io(e) if e.kind() == std::io::ErrorKind::NotFound),
            "{missing}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_directories() {
        let dir = temp_dir("directory");
        let error = read_regular_file(&dir, 1024).unwrap_err();
        assert!(
            matches!(&error, ReadError::Io(e) if e.kind() == std::io::ErrorKind::InvalidInput),
            "{error}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 装置 (`/dev/zero` は終わらない) と FIFO (開くだけで止まる) は、開かずにエラーにする。
    #[cfg(unix)]
    #[test]
    fn refuses_devices_and_fifos_without_opening_them() {
        let error = read_regular_file(Path::new("/dev/zero"), 1024).unwrap_err();
        assert!(
            matches!(&error, ReadError::Io(e) if e.kind() == std::io::ErrorKind::InvalidInput),
            "{error}"
        );
        let dir = temp_dir("fifo");
        let fifo = dir.join("model.ugoe");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(status.success());
        let error = read_regular_file(&fifo, 1024).unwrap_err();
        assert!(
            matches!(&error, ReadError::Io(e) if e.kind() == std::io::ErrorKind::InvalidInput),
            "{error}"
        );
        // 通常のファイルへのリンクは辿って読む。
        let target = dir.join("target.ugoe");
        std::fs::write(&target, b"ok").unwrap();
        let link = dir.join("link.ugoe");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(read_regular_file(&link, 1024).unwrap(), b"ok");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
