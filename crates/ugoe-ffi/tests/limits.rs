//! 読み込みの上限 (`UgoeReadLimits`。T0152): 小さな上限で読むと `UGOE_ERROR_LIMIT_EXCEEDED` になり、
//! 種類と補足の値が SDK の `ugoe::limits::ReadLimitExceeded` と同じになる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作ったバイト列・生きている Rust の値を指すポインタ、
// または意図的に NULL を渡し、作ったオブジェクトはその場で破棄する。借りた文字列は次の失敗の前に読む。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use ugoe::format::module_file::{ModuleFile, write_module_with};
use ugoe::format::{Bundle, TextureData, TextureFormat, WriteOptions, write_bundle_with};
use ugoe::model::fixtures;
use ugoe_ffi::*;

/// 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
fn png() -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
    }
}

fn text(ptr: *const c_char) -> String {
    assert!(!ptr.is_null());
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn field(key: &str) -> String {
    let key = CString::new(key).unwrap();
    text(unsafe { ugoe_last_error_field(key.as_ptr()) })
}

/// 圧縮したバンドル (展開するチャンクを持つ)。
fn compressed_bundle() -> Vec<u8> {
    let model = fixtures::single_quad();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png()).collect();
    write_bundle_with(&bundle, WriteOptions { compress: true }).unwrap()
}

/// 圧縮したモジュール。
fn compressed_module() -> Vec<u8> {
    let mut file = ModuleFile::new(fixtures::long_hair_module());
    file.textures = file.module.textures.iter().map(|_| png()).collect();
    write_module_with(&file, WriteOptions { compress: true }).unwrap()
}

/// 合計の上限だけを `max` にした上限。
fn total(max: u64) -> UgoeReadLimits {
    UgoeReadLimits {
        max_total_decompressed: max,
        ..ugoe_read_limits_default()
    }
}

#[track_caller]
fn assert_total_limit_exceeded(code: UgoeResult, max: u64) {
    assert_eq!(
        code,
        UGOE_ERROR_LIMIT_EXCEEDED,
        "{}",
        text(ugoe_last_error())
    );
    assert_eq!(text(ugoe_last_error_kind()), "read_limit_exceeded");
    assert_eq!(field("limit"), "total_decompressed");
    assert_eq!(field("max"), max.to_string());
    assert!(!field("chunk").is_empty());
}

#[test]
fn bundles_over_the_limits_are_rejected() {
    let bytes = compressed_bundle();
    let mut p = ptr::null_mut();
    let small = total(1);
    assert_total_limit_exceeded(
        unsafe {
            ugoe_puppet_create_from_bundle_with_limits(bytes.as_ptr(), bytes.len(), &small, &mut p)
        },
        1,
    );
    assert!(p.is_null());

    // 1 チャンクの上限も同じ種類で、`limit` で見分ける。
    let chunk = UgoeReadLimits {
        max_chunk_decompressed: 1,
        ..ugoe_read_limits_default()
    };
    assert_eq!(
        unsafe {
            ugoe_puppet_create_from_bundle_with_limits(bytes.as_ptr(), bytes.len(), &chunk, &mut p)
        },
        UGOE_ERROR_LIMIT_EXCEEDED
    );
    assert_eq!(field("limit"), "chunk_decompressed");

    // NULL と既定値は、上限を指定しない関数と同じに読める。
    for limits in [ptr::null(), &ugoe_read_limits_default() as *const _] {
        let mut p = ptr::null_mut();
        assert_eq!(
            unsafe {
                ugoe_puppet_create_from_bundle_with_limits(
                    bytes.as_ptr(),
                    bytes.len(),
                    limits,
                    &mut p,
                )
            },
            UGOE_OK
        );
        unsafe { ugoe_puppet_destroy(p) };
    }
}

#[test]
fn modules_over_the_limits_are_rejected() {
    let bytes = compressed_module();
    let mut m = ptr::null_mut();
    let small = total(1);
    assert_total_limit_exceeded(
        unsafe { ugoe_module_create_with_limits(bytes.as_ptr(), bytes.len(), &small, &mut m) },
        1,
    );
    assert!(m.is_null());

    // 上限の外の失敗は今までどおり `UGOE_ERROR_FORMAT` と `invalid_file`。
    let garbage = b"not a module";
    assert_eq!(
        unsafe { ugoe_module_create_with_limits(garbage.as_ptr(), garbage.len(), &small, &mut m) },
        UGOE_ERROR_FORMAT
    );
    assert_eq!(text(ugoe_last_error_kind()), "invalid_file");

    assert_eq!(
        unsafe { ugoe_module_create_with_limits(bytes.as_ptr(), bytes.len(), ptr::null(), &mut m) },
        UGOE_OK
    );
    unsafe { ugoe_module_destroy(m) };
}

/// 上限の外の読み込みの失敗は、上限を指定してもしなくても今までどおり `UGOE_ERROR_FORMAT` (種類なし)。
#[test]
fn bundle_format_errors_keep_their_code() {
    let garbage = b"not a bundle";
    let mut p = ptr::null_mut();
    assert_eq!(
        unsafe { ugoe_puppet_create_from_bundle(garbage.as_ptr(), garbage.len(), &mut p) },
        UGOE_ERROR_FORMAT
    );
    assert_eq!(
        unsafe {
            ugoe_puppet_create_from_bundle_with_limits(
                garbage.as_ptr(),
                garbage.len(),
                &total(1),
                &mut p,
            )
        },
        UGOE_ERROR_FORMAT
    );
    // 種類を持たない失敗。
    assert!(ugoe_last_error_kind().is_null());
}
