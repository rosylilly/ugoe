//! ugoe の C ABI。Unity (P/Invoke)・Unreal・Godot (GDExtension) などから使う。
//! 設計: `docs/portability.md#c-abi-ugoe-ffi-の規約`。ヘッダは `include/ugoe.h` (cbindgen で生成)。
//!
//! 規約:
//! - 公開シンボルはすべて `ugoe_` 接頭辞。
//! - オブジェクトは不透明ポインタ (`UgoePuppet*`) で渡し、`ugoe_puppet_destroy` で解放する。
//! - 失敗しうる関数は [`UgoeResult`] を返し、最後の引数 `error` に [`UgoeError`] を取る (NULL なら書かない)。
//!   失敗の中身はそこから読む。C ABI はグローバルな状態もスレッドローカルな状態も持たない (ADR 0031)。
//! - パニックを FFI 境界の外へ漏らさない (`std` がある形では `catch_unwind` して [`UGOE_ERROR_PANIC`] を返す。
//!   `std` の無い形では、[`ugoe_set_panic_handler`] で登録されたホストの関数にメッセージを渡してから、ホストの `abort` を呼ぶ)。ランタイム層と同じく、外から来るどんな値でもパニックしないように書く
//!   (`docs/portability.md#パニックしない`)。
//! - 毎フレームの描画の状態 (頂点位置・不透明度・描画順など) は 1 回の呼び出しでまとめて渡す。内部のバッファの
//!   借用 ([`ugoe_puppet_frame`]。次の `ugoe_puppet_update` まで有効) と、ホストのバッファへのコピー
//!   ([`ugoe_puppet_copy_frame`]) の両方を用意する (ADR 0031)。
//!
//! # feature
//!
//! - `std` (既定で有効): `std` を使う。外すと `#![no_std]` (`core` + `alloc`) でビルドし、ゲーム機や組み込み機器のように
//!   Rust の `std` が無い環境に静的ライブラリとして組み込める。アロケータはホストの C ライブラリの `malloc`・`free` など、
//!   パニックは (登録されたホストの関数にメッセージを渡してから) `abort` になる。関数の集合とヘッダは変わらない (crates/ugoe-ffi/README.md#std-の無い環境。ADR 0031)。
//! - `fuzzing`: fuzz のターゲット (`fuzz/fuzz_targets/ffi.rs`) が呼ぶ安全な Rust の入口 `ugoe_ffi::fuzzing` を足す。入力を C ABI の呼び出しの列に
//!   組み立てて回すので、`unsafe` はこのクレートの中に留まる。ヘッダには出さず、配るライブラリでは有効にしない (docs/testing.md#fuzz。T0497)。

#![cfg_attr(not(test), no_std)]
#![allow(unsafe_code)]
// `#[unsafe(no_mangle)]` と生ポインタ操作のため、このクレートに限り許可する。
// C ABI の中のパニックは、std の無い形ではアプリ全体を止める (docs/portability.md#パニックしない。ADR 0031)。
// テストのコードは clippy.toml で除く。
#![warn(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::string_slice,
    clippy::unimplemented,
    clippy::unreachable
)]

extern crate alloc;
// `catch_unwind` (error.rs の `guard`) が使う (テストは std の prelude で動く)
#[cfg(all(feature = "std", not(test)))]
extern crate std;

mod animation;
#[cfg(test)]
mod codes;
mod drawable;
mod dress_up;
mod error;
mod follow;
mod frame;
/// fuzz のための安全な入口 (feature `fuzzing`)。C ABI の関数ではないので、ヘッダに出さない。
/// cbindgen:ignore
#[cfg(feature = "fuzzing")]
pub mod fuzzing;
mod hit;
mod image;
mod lighting;
mod limits;
/// std の無い形のアロケータとパニックの処理。ホストの C ライブラリの宣言はヘッダに出さない。
/// cbindgen:ignore
#[cfg(all(not(feature = "std"), not(test)))]
mod nostd;
mod panic;
mod part;
mod puppet;
mod tracking;
mod warning;

use core::ffi::c_char;

pub use animation::*;
pub use drawable::*;
pub use dress_up::*;
pub use error::*;
pub use follow::*;
pub use frame::*;
pub use hit::*;
pub use image::*;
pub use lighting::*;
pub use limits::*;
pub use panic::*;
pub use part::*;
pub use puppet::*;
pub use tracking::*;
pub use warning::*;

/// C ABI のバージョン。互換性を壊す変更で上げる (ヘッダでは `UGOE_ABI_VERSION`)。
pub const ABI_VERSION: u32 = 3;

/// 読み込んだライブラリの C ABI のバージョン。ヘッダの `UGOE_ABI_VERSION` と一致するかを確かめる。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_abi_version() -> u32 {
    ABI_VERSION
}

/// ランタイムのバージョン (`"0.1.0"` の形。NUL 終端)。ポインタはずっと有効。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}
