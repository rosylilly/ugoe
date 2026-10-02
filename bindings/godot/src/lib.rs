//! ugoe の Godot 4 向け GDExtension。
//!
//! `UgoePuppet2D` ノードが `.ugoe` を読み込み、毎フレーム `ugoe::Puppet` を進めて、
//! 描画可能要素を Godot の 2D の描画 (RenderingServer のキャンバスアイテム) で描く。
//! 評価は ugoe で行い、描画は Godot で行う (`docs/portability.md` の段階 B)。
//!
//! `.ugoe` と `.ugoemod` をリソース (`UgoeModel`・`UgoeModule`) として読む部分は GDScript で書いている
//! (`example/addons/ugoe/`)。Godot はリソースの読み込み器をワーカースレッドからも呼ぶが、godot-rust の
//! クラスはメインスレッド以外から呼べない (`experimental-threads` を使わない限り) ため。

pub mod model;
mod node;
mod render;
pub mod shader;
pub mod xr;

// GDExtension の入口。godot-rust の決まりで `unsafe impl` が要る (中で unsafe な操作はしない)。
#[allow(unsafe_code)]
mod entry {
    use godot::prelude::*;

    struct UgoeExtension;

    #[gdextension]
    unsafe impl ExtensionLibrary for UgoeExtension {}
}
