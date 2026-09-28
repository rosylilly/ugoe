//! モーション再生と「常時エフェクト」群。仕様: `docs/spec/motion.md`。
//!
//! 1 フレームでの適用順は `ugoe` ファサード (`Puppet::update`) が管理する
//! (`docs/spec/motion.md#フレーム内の適用順`)。このクレートは個々の処理を提供するだけ。
//!
//! フェイストラッキングの対応付け ([`tracking`]) もここに置く (`docs/spec/avatar-profile.md`)。

pub mod curve;
mod effects;
mod expression;
mod manager;
mod player;
mod pose;
pub mod tracking;

pub use effects::{BlinkPhase, BreathPlayer, EyeBlinkPlayer, LipSyncPlayer, Rng};
pub use expression::{ExpressionManager, ExpressionPlayer};
pub use manager::{MotionManager, Priority};
pub use player::{MotionPlayer, easing};
pub use pose::PosePlayer;
pub use tracking::{Rotation, TrackingInput, TrackingMap};
