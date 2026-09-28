//! キーフォームの推定 (`docs/spec/keyform-inference.md`、ADR 0010)。
//!
//! 推定はツール層で行い、結果はふつうのキーフォームとして格子に焼き込む。
//! ランタイムの評価は推定を知らない。書き込むのは `derived` のセルだけで、`authored` のセルは変えない。

mod additive;
mod error;
mod target;
mod values;

pub use additive::{InferReport, infer_additive};
pub use error::InferError;
pub use target::{KeyformTarget, infer_additive_target};
pub use values::KeyformValues;
