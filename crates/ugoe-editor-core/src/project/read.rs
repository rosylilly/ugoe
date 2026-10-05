//! プロジェクトのファイルを、通常のファイルのときだけ、大きさの上限までで読む (T1607)。
//! 設計: `docs/editor.md#読むファイルの種類と大きさ`。
//!
//! 装置 (`/dev/zero`)・FIFO を読むと、終わらない・書き手を待って止まる・メモリを使い果たす。プロジェクトの中のファイルは
//! 外 (`ugoe-mcp`・手での編集・受け取ったプロジェクト) が置くので、名前だけでは中身の種類が分からない。
//! 読み方は 3 つのアプリと同じ `ugoe_fs::read_regular_file` (T0011・T0025。開く前と開いた後に通常のファイルかと大きさを見て、
//! 上限 + 1 バイトまでに限って読む)。ここはエラーを [`ProjectError`] に替える。

use std::fs::Metadata;
use std::path::Path;

use ugoe_fs::ReadError;

use super::ProjectError;

/// JSON のファイル (`project.json`・`model.json`・`module.json`・`physics.json`・モーション・表情) 1 つの大きさの
/// 既定の上限 (256 MiB)。
pub const DEFAULT_MAX_JSON_BYTES: u64 = 256 * 1024 * 1024;

/// 画像 (`textures/*.png`) 1 つの大きさの既定の上限 (512 MiB)。
pub const DEFAULT_MAX_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

/// モジュールのプロジェクトの本体の `.ugoe` の大きさの既定の上限 (512 MiB)。
pub const DEFAULT_MAX_BUNDLE_BYTES: u64 = 512 * 1024 * 1024;

/// プロジェクトを読む・指紋を取るときの、ファイル 1 つの大きさの上限 (バイト)。読む側が決める。
/// 既定 ([`Default`]) の値と理由は `docs/editor.md#読むファイルの種類と大きさ`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectReadLimits {
    /// JSON のファイル 1 つ。既定は [`DEFAULT_MAX_JSON_BYTES`]。
    pub max_json: u64,
    /// 画像 1 つ。既定は [`DEFAULT_MAX_IMAGE_BYTES`]。
    pub max_image: u64,
    /// 本体の `.ugoe`。既定は [`DEFAULT_MAX_BUNDLE_BYTES`]。
    pub max_bundle: u64,
}

impl Default for ProjectReadLimits {
    fn default() -> Self {
        Self {
            max_json: DEFAULT_MAX_JSON_BYTES,
            max_image: DEFAULT_MAX_IMAGE_BYTES,
            max_bundle: DEFAULT_MAX_BUNDLE_BYTES,
        }
    }
}

impl ProjectReadLimits {
    /// JSON のファイルの上限を `bytes` にしたもの。
    pub const fn with_max_json(self, bytes: u64) -> Self {
        Self {
            max_json: bytes,
            ..self
        }
    }

    /// 画像の上限を `bytes` にしたもの。
    pub const fn with_max_image(self, bytes: u64) -> Self {
        Self {
            max_image: bytes,
            ..self
        }
    }

    /// 本体の `.ugoe` の上限を `bytes` にしたもの。
    pub const fn with_max_bundle(self, bytes: u64) -> Self {
        Self {
            max_bundle: bytes,
            ..self
        }
    }
}

/// `metadata` が通常のファイルで、`max` バイト以下であることを確かめる。
pub(crate) fn check_regular(
    path: &Path,
    metadata: &Metadata,
    max: u64,
) -> Result<(), ProjectError> {
    if !metadata.is_file() {
        return Err(ProjectError::NotRegularFile {
            path: path.to_path_buf(),
        });
    }
    if metadata.len() > max {
        return Err(ProjectError::FileTooLarge {
            path: path.to_path_buf(),
            len: metadata.len(),
            max,
        });
    }
    Ok(())
}

/// `path` を読む。通常のファイル (リンクは辿る) でなければ開かずに [`ProjectError::NotRegularFile`]、
/// `max` バイトより大きければ読まずに [`ProjectError::FileTooLarge`]。読めなければ [`ProjectError::Io`]。
pub(crate) fn read_regular_file(path: &Path, max: u64) -> Result<Vec<u8>, ProjectError> {
    read_regular_file_as(path, path, max)
}

/// [`read_regular_file`] と同じだが、エラーに載せるパスを `shown` にする (確かめて替えたパスから読むとき)。
pub(crate) fn read_regular_file_as(
    path: &Path,
    shown: &Path,
    max: u64,
) -> Result<Vec<u8>, ProjectError> {
    ugoe_fs::read_regular_file(path, max).map_err(|e| match e {
        ReadError::Io(source) => ProjectError::Io {
            path: shown.to_path_buf(),
            source,
        },
        ReadError::NotRegularFile => ProjectError::NotRegularFile {
            path: shown.to_path_buf(),
        },
        ReadError::TooLarge { len, max } => ProjectError::FileTooLarge {
            path: shown.to_path_buf(),
            len,
            max,
        },
    })
}
