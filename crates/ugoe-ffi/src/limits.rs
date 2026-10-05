//! 読み込みの上限 (`UgoeReadLimits`。`docs/spec/file-formats.md#読み込みの上限`、T0152)。
//!
//! 上限はファイル形式の決まりではなく、組み込む側の方針。`ugoe_puppet_create_from_bundle_with_limits` と
//! `ugoe_module_create_with_limits` に渡す。上限を指定しない関数 (`ugoe_puppet_create_from_bundle` など) は既定値で読む。
//! 超えたら `UGOE_ERROR_LIMIT_EXCEEDED` を返し、種類と補足の値は SDK の `ugoe::limits::ReadLimitExceeded` で、
//! wasm の読み込みのエラーの `code` と同じ。

use ugoe::LoadError;
use ugoe::format::FormatError;
use ugoe::limits::{ReadLimitExceeded, ReadLimits};

use crate::error::{Error, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_MODEL, UGOE_ERROR_LIMIT_EXCEEDED};

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

/// モデルの読み込み (`ugoe_puppet_create_from_*`) の失敗をエラーにする (T0457)。種類と補足の値は SDK の
/// [`ugoe::LoadError::code`]・`fields` (wasm の `LoadError` と同じ。数は 10 進の文字列)。戻り値は T0457 の前と同じで、
/// 上限を超えたら `UGOE_ERROR_LIMIT_EXCEEDED`、ほかのファイルの失敗は `UGOE_ERROR_FORMAT`、コンパイルの失敗は
/// `UGOE_ERROR_INVALID_MODEL`。
pub(crate) fn load_error(error: LoadError) -> Error {
    // 文も T0457 の前と同じ (コンパイルの失敗に SDK の前置きを付けない)。
    let (code, message) = match &error {
        e if e.exceeds_limit() => (UGOE_ERROR_LIMIT_EXCEEDED, e.to_string()),
        LoadError::Format(e) => (UGOE_ERROR_FORMAT, e.to_string()),
        LoadError::Compile(e) => (UGOE_ERROR_INVALID_MODEL, e.to_string()),
    };
    let fields = error
        .fields()
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect();
    Error::new(code, message).with_kind(error.code(), fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 入力からは作りにくいコンパイルの失敗 (検査を通るが評価できない) も、SDK の種類で `UGOE_ERROR_INVALID_MODEL`。
    #[test]
    fn compile_errors_keep_their_code_and_text() {
        use std::ffi::CStr;

        use ugoe::runtime::CompileError;

        use crate::error::{guard, ugoe_last_error, ugoe_last_error_kind};

        let code = guard(|| Err(load_error(CompileError::Unsupported("x").into())));
        assert_eq!(code, UGOE_ERROR_INVALID_MODEL);
        // SAFETY: 失敗の直後なので、どちらも NUL 終端の文字列を指す。
        let (kind, message) = unsafe {
            (
                CStr::from_ptr(ugoe_last_error_kind()),
                CStr::from_ptr(ugoe_last_error()),
            )
        };
        assert_eq!(kind.to_str().unwrap(), "compile");
        assert_eq!(
            message.to_str().unwrap(),
            CompileError::Unsupported("x").to_string()
        );
    }

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
