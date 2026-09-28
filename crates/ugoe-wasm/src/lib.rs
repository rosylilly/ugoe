//! ugoe の WebAssembly バインディング。
//!
//! 用途: (1) Web ビューア SDK、(2) エディタ (Tauri WebView) のビューポート描画。
//! エディタではバックエンドから `.ugoe` バイト列を受け取り、パラメータ操作は
//! wasm 内で完結させて 60fps を保つ (`docs/editor.md#ビューポート`)。
//!
//! ビルド: `just wasm` (wasm-bindgen-cli が必要)。

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    ugoe::VERSION.to_string()
}

// TODO(T0402): `WebPuppet` (モデル読み込み・パラメータ設定・canvas への描画) を公開する。
