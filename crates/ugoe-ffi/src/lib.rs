//! ugoe の C ABI。Unity (P/Invoke)・Unreal・Godot (GDExtension) などから使う。
//! 設計: `docs/portability.md`。
//!
//! 規約:
//! - 公開シンボルはすべて `ugoe_` 接頭辞。
//! - オブジェクトは不透明ポインタ (`*mut UgoePuppet` 等) で渡し、`*_destroy` で解放する。
//! - パニックを FFI 境界の外へ漏らさない (`catch_unwind` してエラーコードを返す)。
//! - ヘッダは cbindgen で生成する (TODO(T0401))。
#![allow(unsafe_code)] // `#[unsafe(no_mangle)]` と生ポインタ操作のため、このクレートに限り許可する。

/// C ABI のバージョン。互換性を壊す変更で上げる。
pub const ABI_VERSION: u32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn ugoe_abi_version() -> u32 {
    ABI_VERSION
}
