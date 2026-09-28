//! モーション再生と「常時エフェクト」群。仕様: `docs/spec/motion.md`。
//!
//! 1 フレームでの適用順は [`ugoe`] ファサードが管理する:
//! モーション → 表情 → 目パチ → 呼吸 → (物理) → ポーズ → リップシンク。

pub mod curve;
mod player;

pub use player::{MotionPlayer, easing};
