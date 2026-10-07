//! モーション再生と「常時エフェクト」群。仕様: `docs/spec/motion.md`。
//!
//! 1 フレームでの適用順は `ugoe` ファサード (`Puppet::update`) が管理する
//! (`docs/spec/motion.md#フレーム内の適用順`)。このクレートは個々の処理を提供するだけ。
//!
//! フェイストラッキングの対応付け ([`tracking`]) もここに置く (`docs/spec/avatar-profile.md`)。
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

pub mod curve;
mod effects;
mod expression;
mod manager;
mod player;
mod pose;
mod resolve;
pub mod tracking;

pub use effects::{BlinkPhase, BreathPlayer, EyeBlinkPlayer, LipSyncPlayer, Rng};
pub use expression::{ExpressionManager, ExpressionPlayer};
pub use manager::{EventSource, MotionManager, PartOpacityBuffer, Priority};
pub use player::{MotionPlayer, easing};
pub use pose::PosePlayer;
pub use tracking::{BodyInput, Rotation, TrackingInput, TrackingMap};
