//! 読み込みの警告。種類 (`code`) と補足の値は SDK の `ugoe::LoadWarning` のもので、wasm の
//! `load_warnings_json` と同じ (crates/ugoe-wasm/README.md#読み込みの警告-t0125。T0412)。

use alloc::ffi::CString;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::c_char;

use ugoe::LoadWarning;
use ugoe::format::{ReadReport, TextureData};

use crate::error::{Error, UGOE_ERROR_NOT_FOUND, c_string};
use crate::puppet::{c_str, write};

/// 読み込みの警告 1 つ。文字列は、警告を持つオブジェクト (`UgoePuppet`・`UgoeModule`) を破棄するまで有効。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeWarning {
    /// 機械が読む種類 (UTF-8、NUL 終端)。`"texture_missing"`・`"unknown_optional_feature"`・`"unknown_value"`。
    /// 知らない種類 (将来の版で増えたもの) は `message` をそのまま出せばよい。
    pub code: *const c_char,
    /// 英語の文 (UTF-8、NUL 終端)。文面は安定した値として扱わない。
    pub message: *const c_char,
}

/// 警告の中身 (C の文字列にしたもの)。
#[derive(Debug)]
pub(crate) struct Warning {
    code: CString,
    message: CString,
    fields: Vec<(CString, CString)>,
}

impl Warning {
    /// `warning` の種類と補足の値に、文 `message` を添える。
    pub(crate) fn new(warning: &LoadWarning, message: &str) -> Self {
        Self {
            code: c_string(warning.code()),
            message: c_string(message),
            fields: warning
                .fields()
                .iter()
                .map(|(k, v)| (c_string(k), c_string(&v.to_string())))
                .collect(),
        }
    }

    /// 種類 (テストで読む)。
    #[cfg(test)]
    pub(crate) fn code(&self) -> &str {
        self.code.to_str().unwrap_or_default()
    }

    /// 補足の値のキーと値 (テストで読む)。
    #[cfg(test)]
    pub(crate) fn fields(&self) -> Vec<(&str, &str)> {
        self.fields
            .iter()
            .map(|(k, v)| {
                (
                    k.to_str().unwrap_or_default(),
                    v.to_str().unwrap_or_default(),
                )
            })
            .collect()
    }

    fn to_c(&self) -> UgoeWarning {
        UgoeWarning {
            code: self.code.as_ptr(),
            message: self.message.as_ptr(),
        }
    }
}

/// ファイルの読み込みの報告の警告 (知らない任意の機能・未知の値)。
pub(crate) fn read_warnings(report: &ReadReport) -> Vec<Warning> {
    report
        .warnings
        .iter()
        .map(|w| {
            let warning = LoadWarning::from(w);
            Warning::new(&warning, &warning.to_string())
        })
        .collect()
}

/// 画像の無いテクスチャの番号の警告 (`texture_missing`。読み込みがテクスチャの数を検査するので、通常は起きない)。
/// C ABI は代わりの画像を用意しないので、文は wasm (白で代用する) と違う。
pub(crate) fn missing_textures(textures: &[Option<TextureData>]) -> Vec<Warning> {
    textures
        .iter()
        .enumerate()
        .filter(|(_, t)| t.is_none())
        .map(|(i, _)| {
            Warning::new(
                &LoadWarning::TextureMissing { texture: i },
                &format!("no data for texture {i}"),
            )
        })
        .collect()
}

/// `warnings` の `index` 番目を `out` に書く。
///
/// # Safety
///
/// `out` は NULL か、書き込める領域を指すこと。
pub(crate) unsafe fn write_warning(
    warnings: &[Warning],
    index: u32,
    out: *mut UgoeWarning,
) -> Result<(), Error> {
    let w = warnings
        .get(index as usize)
        .ok_or_else(|| Error::out_of_range("warning", index as usize, warnings.len()))?;
    // SAFETY: 呼び出し側の約束。
    unsafe { write(out, "out", w.to_c()) }
}

/// `warnings` の `index` 番目の補足の値 `key` を `out` に書く。無ければ `UGOE_ERROR_NOT_FOUND`。
///
/// # Safety
///
/// `key` は NULL か NUL 終端の文字列、`out` は NULL か書き込める領域を指すこと。
pub(crate) unsafe fn write_warning_field(
    warnings: &[Warning],
    index: u32,
    key: *const c_char,
    out: *mut *const c_char,
) -> Result<(), Error> {
    let w = warnings
        .get(index as usize)
        .ok_or_else(|| Error::out_of_range("warning", index as usize, warnings.len()))?;
    // SAFETY: 呼び出し側の約束。
    let key = unsafe { c_str(key, "key")? };
    let value = w
        .fields
        .iter()
        .find(|(k, _)| k.to_bytes() == key.as_bytes())
        .ok_or_else(|| Error::new(UGOE_ERROR_NOT_FOUND, format!("the warning has no `{key}`")))?;
    // SAFETY: 呼び出し側の約束。
    unsafe { write(out, "out", value.1.as_ptr()) }
}
