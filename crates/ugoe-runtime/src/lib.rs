//! ugoe ランタイム評価。
//!
//! 1 フレームの流れ (`docs/spec/evaluation.md`):
//! 1. ホスト (モーション・物理など) が [`ParameterValues`] を書き換える
//! 2. [`ModelInstance::update`] がキーフォーム補間 → デフォーマ変形 → 不透明度・描画順を計算
//! 3. 描画側 (`ugoe-render` やエンジン独自レンダラ) が [`ModelInstance::drawables`] を読む
//!    ([`ModelInstance::changes`] で前フレームから変わったものだけを転送できる)
//!
//! このクレートは GPU・I/O・スレッドに依存しない。決定的 (同入力 → 同出力) であること。
//!
//! `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない
//! (`docs/portability.md#ランタイム層が守る制約-移植性のため`)。

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]
// ランタイム層はパニックしない (docs/portability.md#パニックしない。ADR 0031)。テストのコードは clippy.toml で除く。
#![warn(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::string_slice,
    clippy::unimplemented,
    clippy::unreachable
)]

extern crate alloc;

mod blend_shape;
pub mod compile;
pub mod compose;
mod deform;
pub mod dynamic;
mod hit;
mod id_index;
/// 1 体分の評価の状態 ([`ModelInstance`]) と、描画要素の評価結果 ([`DrawableState`])。
pub mod instance;
pub mod keyform;
/// パラメータの値 ([`ParameterValues`]) と、ID からインデックスへの対応 ([`ParameterLayout`])。
pub mod parameters;

pub use compile::{CompileError, CompiledModel, ResolvedMaskRef};
pub use compose::{ComposeError, compose, compose_physics, compose_physics_without_base};
pub use dynamic::DynamicOffsets;
pub use instance::{DrawableChanges, DrawableState, ModelInstance, WorldTransform};
pub use parameters::{ParamIndex, ParameterLayout, ParameterValues};
