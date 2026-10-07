//! テストで共有する補助。
#![allow(dead_code)] // テストのファイルごとに使う関数が違う。

use std::cell::Cell;
use std::ffi::{CStr, c_char};

use ugoe_ffi::*;

/// スレッドが終わるときに破棄する `UgoeError`。
struct ThreadError(Cell<*mut UgoeError>);

impl Drop for ThreadError {
    fn drop(&mut self) {
        // SAFETY: `ugoe_error_create` で作り、まだ破棄していない (か NULL)。
        unsafe { ugoe_error_destroy(self.0.get()) };
    }
}

thread_local! {
    static ERROR: ThreadError = const { ThreadError(Cell::new(std::ptr::null_mut())) };
}

/// このテストのスレッドの `UgoeError`。テストは失敗を読みやすいように、呼び出しのたびに同じものを渡す
/// (C ABI はスレッドローカルの状態を持たないので、どこに渡すかは呼び出し側が決める)。
pub fn err() -> *mut UgoeError {
    ERROR.with(|e| {
        if e.0.get().is_null() {
            e.0.set(ugoe_error_create());
        }
        e.0.get()
    })
}

/// このスレッドの `UgoeError` のメッセージ (`ugoe_error_message`)。
pub fn last_message() -> *const c_char {
    // SAFETY: `err()` は破棄していない `UgoeError`。
    unsafe { ugoe_error_message(err()) }
}

/// このスレッドの `UgoeError` の種類 (`ugoe_error_kind`)。
pub fn last_kind() -> *const c_char {
    // SAFETY: 同上。
    unsafe { ugoe_error_kind(err()) }
}

/// このスレッドの `UgoeError` の補足の値 (`ugoe_error_field`)。
pub fn last_field(key: &CStr) -> *const c_char {
    // SAFETY: 同上。`key` は NUL 終端。
    unsafe { ugoe_error_field(err(), key.as_ptr()) }
}

/// このスレッドの `UgoeError` の補足の値の数 (`ugoe_error_field_count`)。
pub fn last_field_count() -> usize {
    // SAFETY: 同上。
    unsafe { ugoe_error_field_count(err()) }
}

/// このスレッドの `UgoeError` の `index` 番目の補足の値のキー (`ugoe_error_field_key_at`)。
pub fn last_field_key_at(index: usize) -> *const c_char {
    // SAFETY: 同上。
    unsafe { ugoe_error_field_key_at(err(), index) }
}

/// このスレッドの `UgoeError` の `index` 番目の補足の値 (`ugoe_error_field_value_at`)。
pub fn last_field_value_at(index: usize) -> *const c_char {
    // SAFETY: 同上。
    unsafe { ugoe_error_field_value_at(err(), index) }
}
