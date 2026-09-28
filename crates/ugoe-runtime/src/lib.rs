//! ugoe ランタイム評価。
//!
//! 1 フレームの流れ (`docs/spec/evaluation.md`):
//! 1. ホスト (モーション・物理など) が [`ParameterValues`] を書き換える
//! 2. [`ModelInstance::update`] がキーフォーム補間 → デフォーマ変形 → 不透明度・描画順を計算
//! 3. 描画側 (`ugoe-render` やエンジン独自レンダラ) が [`ModelInstance::drawables`] を読む
//!
//! このクレートは GPU・I/O・スレッドに依存しない。決定的 (同入力 → 同出力) であること。

pub mod compile;
mod deform;
pub mod instance;
pub mod keyform;
pub mod parameters;

pub use compile::{CompileError, CompiledModel};
pub use instance::{DrawableState, ModelInstance};
pub use parameters::{ParamIndex, ParameterValues};
