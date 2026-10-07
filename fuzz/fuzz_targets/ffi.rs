//! C ABI (`ugoe-ffi`) の入口に、C のホストと同じ呼び方で入力を渡す (読み込み → 問い合わせ → 設定と `update` → 解放)。
//! 入力は `[ファイル] [ugoe_ffi::fuzzing::SCRIPT_MARKER] [台本]` の形で、呼び出しの列は `ugoe_ffi::fuzzing` が組み立てる
//! (`unsafe` は `ugoe-ffi` の中に留まる。T0497)。種は `assets/compat` と `fuzz/seeds/ffi` (`just fuzz ffi`)。
//!
//! C ABI の中のパニックは、libfuzzer-sys のパニックのフックが `catch_unwind` より先に `abort` するので、そのまま見つかる。
//! フックを通らずに `UGOE_ERROR_PANIC` が返ったときや、返した値がホストへの約束を破ったときは、ここでパニックする。

#![no_main]

use libfuzzer_sys::fuzz_target;

/// 着せ替えの本体 (`assets/compat/dev/dress-up-base.ugoe`)。モジュールとして読めた入力を取り付ける。
const DRESS_UP_BASE: &[u8] = include_bytes!("../../assets/compat/dev/dress-up-base.ugoe");

fuzz_target!(|data: &[u8]| {
    if let Err(failure) = ugoe_ffi::fuzzing::run(data, DRESS_UP_BASE) {
        panic!("{failure}");
    }
});
