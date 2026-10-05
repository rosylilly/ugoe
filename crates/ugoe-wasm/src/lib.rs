//! ugoe の WebAssembly バインディング。
//!
//! 用途: (1) Web ビューア SDK、(2) エディタ (Tauri WebView) のビューポート描画。
//! エディタではバックエンドからモデルを受け取り、パラメータ操作は
//! wasm 内で完結させて 60fps を保つ (`docs/editor.md#ビューポート`)。
//!
//! JS へ公開するのは `WebPuppet` (wasm32 のときだけ)。GPU に触れない部分
//! ([`load`]・[`view`]・[`hit`]・[`gizmo`]・[`animation`]・[`tracking`]・[`physics_view`]・[`lighting`]・[`readback`]・[`frame`]) はネイティブでもビルドしてテストする。
//!
//! ビルド: `just wasm` (wasm-bindgen-cli が必要。README を参照)。

pub mod animation;
#[cfg(test)]
mod codes;
pub mod dress_up;
pub mod frame;
pub mod gizmo;
pub mod hit;
pub mod lighting;
pub mod load;
pub mod physics_view;
pub mod readback;
pub mod tracking;
pub mod view;
#[cfg(target_arch = "wasm32")]
mod web;

use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
pub use web::WebPuppet;

/// ランタイムのバージョン。
#[wasm_bindgen]
pub fn version() -> String {
    ugoe::VERSION.to_string()
}
