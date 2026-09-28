//! キーフォームの推定 (`docs/spec/keyform-inference.md`、ADR 0010)。
//!
//! 推定はツール層で行い、結果はふつうのキーフォームとして格子に焼き込む。
//! ランタイムの評価は推定を知らない。書き込むのは `derived` のセルだけで、`authored` のセルは変えない
//! (ミラーの `MirrorOptions::overwrite_authored` を除く)。

mod additive;
mod depth;
mod error;
mod grid;
mod mirror;
mod space;
mod target;
mod values;

pub use additive::{InferReport, infer_additive};
pub use depth::infer_depth_rotation;
pub use error::InferError;
pub use mirror::{MirrorOptions, mirror_keyforms};
pub use target::{KeyformTarget, infer_additive_target};
pub use values::KeyformValues;
