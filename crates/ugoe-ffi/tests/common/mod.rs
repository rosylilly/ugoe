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

/// `ugoe_puppet_frame` で借りたもの (次の update や破棄の前に読む)。
pub fn frame(puppet: *mut UgoePuppet) -> UgoeFrame {
    let mut frame = UgoeFrame::default();
    // SAFETY: `puppet` はテストが作って破棄していない `UgoePuppet`。
    let code = unsafe { ugoe_puppet_frame(puppet, &mut frame, err()) };
    assert_eq!(code, UGOE_OK);
    frame
}

/// `ptr` から `len` 個を写す (`len` が 0 なら NULL でもよい)。
fn to_vec<T: Copy>(ptr: *const T, len: usize) -> Vec<T> {
    if len == 0 {
        return Vec::new();
    }
    // SAFETY: C ABI が貸した `len` 個の並び。
    unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
}

/// すべての描画可能要素の状態 (番号順。`ugoe_puppet_frame`)。
pub fn states(puppet: *mut UgoePuppet) -> Vec<UgoeDrawableState> {
    let f = frame(puppet);
    to_vec(f.states, f.drawable_count as usize)
}

/// `drawable` 番目の描画可能要素の頂点の位置 (x と y の組。`ugoe_puppet_frame`)。
pub fn positions(puppet: *mut UgoePuppet, drawable: u32) -> Vec<f32> {
    let f = frame(puppet);
    let ranges = to_vec(f.position_ranges, f.drawable_count as usize);
    let all = to_vec(f.positions, f.positions_len);
    let r = ranges[drawable as usize];
    all[r.first..r.first + r.count].to_vec()
}

/// 描画順と、それが変わったか (`ugoe_puppet_frame`)。
pub fn render_order(puppet: *mut UgoePuppet) -> (Vec<u32>, bool) {
    let f = frame(puppet);
    (
        to_vec(f.render_order, f.drawable_count as usize),
        f.render_order_changed,
    )
}
