//! ugoe の C ABI。Unity (P/Invoke)・Unreal・Godot (GDExtension) などから使う。
//! 設計: `docs/portability.md#c-abi-ugoe-ffi-の規約`。ヘッダは `include/ugoe.h` (cbindgen で生成)。
//!
//! 規約:
//! - 公開シンボルはすべて `ugoe_` 接頭辞。
//! - オブジェクトは不透明ポインタ (`UgoePuppet*`) で渡し、`ugoe_puppet_destroy` で解放する。
//! - 関数は [`UgoeResult`] を返し、詳細は同じスレッドの [`ugoe_last_error`] で得る。
//! - パニックを FFI 境界の外へ漏らさない (`catch_unwind` して [`UGOE_ERROR_PANIC`] を返す)。
//! - 頂点位置は、ホストのバッファへのコピー ([`ugoe_puppet_copy_positions`]) と、内部のバッファの
//!   借用 ([`ugoe_puppet_positions`]。次の `ugoe_puppet_update` まで有効) の両方を用意する。
#![allow(unsafe_code)] // `#[unsafe(no_mangle)]` と生ポインタ操作のため、このクレートに限り許可する。

mod animation;
mod drawable;
mod dress_up;
mod error;
mod follow;
mod hit;
mod lighting;
mod part;
mod puppet;
mod tracking;
mod warning;

use std::ffi::c_char;

pub use animation::*;
pub use drawable::*;
pub use dress_up::*;
pub use error::*;
pub use follow::*;
pub use hit::*;
pub use lighting::*;
pub use part::*;
pub use puppet::*;
pub use tracking::*;
pub use warning::*;

/// C ABI のバージョン。互換性を壊す変更で上げる (ヘッダでは `UGOE_ABI_VERSION`)。
pub const ABI_VERSION: u32 = 1;

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
