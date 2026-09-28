//! キーフォームの推定 (`docs/spec/keyform-inference.md`、ADR 0010)。
//!
//! 推定はツール層で行い、結果はふつうのキーフォームとして格子に焼き込む。
//! ランタイムの評価は推定を知らない。書き込むのは `derived` のセルだけで、`authored` のセルは変えない
//! (ミラーの `MirrorOptions::overwrite_authored` を除く)。書いたセルの由来には推定の方法
//! (`additive` / `depth_rotation` / `mirror`) を記録する。`InferScope::Own` なら別の方法で作ったセルは
//! 書き換えない。

mod additive;
mod depth;
mod error;
mod grid;
mod mirror;
mod space;
mod target;
mod values;
mod world;

pub use additive::{InferReport, InferScope, infer_additive};
pub use depth::infer_depth_rotation;
pub use error::InferError;
pub use mirror::{MirrorOptions, check_mirror, mirror_keyforms};
pub use target::{KeyformTarget, infer_additive_target};
pub use values::KeyformValues;
pub use world::{ParentWorld, WorldError};
