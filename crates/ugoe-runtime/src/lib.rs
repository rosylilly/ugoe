//! ugoe ランタイム評価。
//!
//! 1 フレームの流れ (`docs/spec/evaluation.md`):
//! 1. ホスト (モーション・物理など) が [`ParameterValues`] を書き換える
//! 2. [`ModelInstance::update`] がキーフォーム補間 → デフォーマ変形 → 不透明度・描画順を計算
//! 3. 描画側 (`ugoe-render` やエンジン独自レンダラ) が [`ModelInstance::drawables`] を読む
//!    ([`ModelInstance::changes`] で前フレームから変わったものだけを転送できる)
//!
//! このクレートは GPU・I/O・スレッドに依存しない。決定的 (同入力 → 同出力) であること。

mod blend_shape;
pub mod compile;
pub mod compose;
mod deform;
pub mod dynamic;
mod hit;
pub mod instance;
pub mod keyform;
pub mod parameters;

pub use compile::{CompileError, CompiledModel, ResolvedMaskRef};
pub use compose::{ComposeError, compose, compose_physics};
pub use dynamic::DynamicOffsets;
pub use instance::{DrawableChanges, DrawableState, ModelInstance, WorldTransform};
pub use parameters::{ParamIndex, ParameterValues};
