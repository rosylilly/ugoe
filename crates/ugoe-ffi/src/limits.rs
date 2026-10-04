//! 読み込みの上限 (`UgoeReadLimits`。`docs/spec/file-formats.md#読み込みの上限`、T0152)。
//!
//! 上限はファイル形式の決まりではなく、組み込む側の方針。`ugoe_puppet_create_from_bundle_with_limits` と
//! `ugoe_module_create_with_limits` に渡す。上限を指定しない関数 (`ugoe_puppet_create_from_bundle` など) は既定値で読む。
//! 超えたら `UGOE_ERROR_LIMIT_EXCEEDED` を返し、種類と補足の値は SDK の `ugoe::limits::ReadLimitExceeded` で、
//! wasm の読み込みのエラーの `code` と同じ。

use ugoe::format::FormatError;
use ugoe::limits::{ReadLimitExceeded, ReadLimits};

use crate::error::{Error, UGOE_ERROR_FORMAT, UGOE_ERROR_LIMIT_EXCEEDED};

/// 読み込みの上限 (Rust の `ugoe::limits::ReadLimits`)。値は `ugoe_read_limits_default` から始めて変える。
///
/// 数えるのは圧縮したチャンクを展開したバイトだけ (圧縮していないチャンクは入力の長さで抑えられるので数えない)。
/// 上限を足すときは、この構造体を変えずに別の構造体と関数を足す (crates/ugoe-ffi/README.md#abi-の版)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UgoeReadLimits {
    /// 圧縮したチャンク 1 つを展開した後の大きさの上限 (バイト)。既定は 256 MiB。
    pub max_chunk_decompressed: u64,
    /// 1 つのファイルで、圧縮したチャンクを展開したバイトの合計の上限。既定は 1 GiB。
    pub max_total_decompressed: u64,
}

impl From<ReadLimits> for UgoeReadLimits {
    fn from(limits: ReadLimits) -> Self {
        Self {
            max_chunk_decompressed: limits.max_chunk_decompressed,
            max_total_decompressed: limits.max_total_decompressed,
        }
    }
}

impl From<UgoeReadLimits> for ReadLimits {
    fn from(limits: UgoeReadLimits) -> Self {
        ReadLimits::default()
            .with_max_chunk_decompressed(limits.max_chunk_decompressed)
            .with_max_total_decompressed(limits.max_total_decompressed)
    }
}

/// 既定の読み込みの上限 (上限を指定しない関数が使う値)。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_read_limits_default() -> UgoeReadLimits {
    ReadLimits::default().into()
}

/// `limits` を読む。NULL なら既定値。
///
/// # Safety
///
/// `limits` は NULL か、`UgoeReadLimits` を読める領域を指すこと。
pub(crate) unsafe fn read_limits(limits: *const UgoeReadLimits) -> ReadLimits {
    // SAFETY: 呼び出し側の約束。
    unsafe { limits.as_ref() }.map_or_else(ReadLimits::default, |l| (*l).into())
}

/// 読み込みの失敗をエラーにする。上限を超えたものは `UGOE_ERROR_LIMIT_EXCEEDED` と種類
/// `"read_limit_exceeded"`・補足の値 (`limit`・`chunk`・`max`)、それ以外は `UGOE_ERROR_FORMAT` (種類は `other_kind`)。
pub(crate) fn format_error(error: &FormatError, other_kind: Option<&'static str>) -> Error {
    if let Some(exceeded) = ReadLimitExceeded::from_error(error) {
        return Error::new(UGOE_ERROR_LIMIT_EXCEEDED, error.to_string())
            .with_kind(ReadLimitExceeded::CODE, exceeded.fields());
    }
    let e = Error::new(UGOE_ERROR_FORMAT, error.to_string());
    match other_kind {
        Some(kind) => e.with_kind(kind, Vec::new()),
        None => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_match_the_sdk() {
        let limits = ugoe_read_limits_default();
        assert_eq!(ReadLimits::from(limits), ReadLimits::default());
        assert_eq!(limits.max_chunk_decompressed, 256 * 1024 * 1024);
        assert_eq!(limits.max_total_decompressed, 1024 * 1024 * 1024);
        // SAFETY: NULL は既定値。
        assert_eq!(
            unsafe { read_limits(std::ptr::null()) },
            ReadLimits::default()
        );
    }
}
