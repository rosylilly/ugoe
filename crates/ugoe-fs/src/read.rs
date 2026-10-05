//! IPC で受けたパスのファイルを、通常のファイルのときだけ、大きさの上限までで読む (T0011)。
//!
//! 装置 (`/dev/zero`)・FIFO・ディレクトリを読むと、終わらない・メモリを使い果たす・止まる。webview が渡すパスは
//! 利用者が選んだものでも、名前だけでは中身の種類が分からない (`x.ugoe` という FIFO・`/dev/zero` へのリンク)。
//! 線引きは T0530 の案のとおり: サーバやアプリの IPC で受けるパスは通常のファイルだけを読み、利用者が自分で渡す
//! CLI の引数は FIFO を読めるままにする。3 つのアプリの写し (T0011) を T0025 でここにまとめた。
//! MCP サーバ (`ugoe-mcp`) と ugoe-headless の `read_file` (`--texture` などのパス) もこれで読む (T0035)。
//!
//! - 大きさを見てから読むまでの間にファイルが大きくなっても上限を超えて読まないよう、上限 + 1 バイトまでに限って読む。
//! - 開いた後にも、開いたものが通常のファイルかを確かめる (見てから開くまでの間に差し替えられたとき)。
//!   ただし見てから開くまでの間に FIFO へ差し替えられると、開くところで止まりうる (その窓は小さいので受け入れる)。

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 読めなかった理由。
#[derive(Debug)]
pub enum ReadError {
    /// 読めない (無い・権限が無いなど)。
    Io(std::io::Error),
    /// 通常のファイルでない (ディレクトリ・装置・FIFO など)。開かずに返す。
    NotRegularFile,
    /// 上限より大きい。`len` は分かった大きさ (読みながら超えたときは読んだバイト数)。
    TooLarge { len: u64, max: u64 },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::NotRegularFile => f.write_str("not a regular file"),
            Self::TooLarge { len, max } => {
                write!(
                    f,
                    "file is too large ({len} bytes; the limit is {max} bytes)"
                )
            }
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::NotRegularFile | Self::TooLarge { .. } => None,
        }
    }
}

/// `std::io::Error` で扱う呼び手のため。通常のファイルでなければ `InvalidInput`、大きすぎれば `FileTooLarge`
/// (文はどちらも [`ReadError`] の `Display`)。
impl From<ReadError> for std::io::Error {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Io(e) => e,
            ReadError::NotRegularFile => Self::new(std::io::ErrorKind::InvalidInput, e),
            ReadError::TooLarge { .. } => Self::new(std::io::ErrorKind::FileTooLarge, e),
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// `path` を読む。通常のファイル (リンクは辿る) でなければ開かずに `NotRegularFile`、`max` バイトより大きければ
/// 読まずに `TooLarge`。
pub fn read_regular_file(path: &Path, max: u64) -> Result<Vec<u8>, ReadError> {
    // 開く前に見る (FIFO は開くだけで書き手を待って止まる)。
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(ReadError::NotRegularFile);
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
        return Err(ReadError::NotRegularFile);
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

    #[test]
    fn reads_regular_files_up_to_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
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
    }

    #[test]
    fn refuses_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let error = read_regular_file(dir, 1024).unwrap_err();
        assert!(matches!(&error, ReadError::NotRegularFile), "{error}");
    }

    #[test]
    fn converts_into_io_errors() {
        let io = std::io::Error::from(ReadError::NotRegularFile);
        assert_eq!(io.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(io.to_string(), "not a regular file");
        let io = std::io::Error::from(ReadError::TooLarge { len: 5, max: 4 });
        assert_eq!(io.kind(), std::io::ErrorKind::FileTooLarge);
        assert_eq!(
            io.to_string(),
            "file is too large (5 bytes; the limit is 4 bytes)"
        );
        let io = std::io::Error::from(ReadError::Io(std::io::ErrorKind::NotFound.into()));
        assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
    }

    /// 装置 (`/dev/zero` は終わらない) と FIFO (開くだけで止まる) は、開かずにエラーにする。
    #[cfg(unix)]
    #[test]
    fn refuses_devices_and_fifos_without_opening_them() {
        let error = read_regular_file(Path::new("/dev/zero"), 1024).unwrap_err();
        assert!(matches!(&error, ReadError::NotRegularFile), "{error}");
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let fifo = dir.join("model.ugoe");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(status.success());
        let error = read_regular_file(&fifo, 1024).unwrap_err();
        assert!(matches!(&error, ReadError::NotRegularFile), "{error}");
        // 通常のファイルへのリンクは辿って読む。
        let target = dir.join("target.ugoe");
        std::fs::write(&target, b"ok").unwrap();
        let link = dir.join("link.ugoe");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(read_regular_file(&link, 1024).unwrap(), b"ok");
    }
}
