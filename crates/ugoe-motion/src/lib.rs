//! モーション再生と「常時エフェクト」群。仕様: `docs/spec/motion.md`。
//!
//! 1 フレームでの適用順は [`ugoe`] ファサードが管理する:
//! モーション → 表情 → 目パチ → 呼吸 → (物理) → ポーズ → リップシンク。

pub mod curve;

use ugoe_model::motion::Motion;
use ugoe_runtime::ParameterValues;

/// モーションの再生状態 1 本分。
#[derive(Debug)]
pub struct MotionPlayer {
    motion: Motion,
    time: f32,
}

impl MotionPlayer {
    pub fn new(motion: Motion) -> Self {
        Self { motion, time: 0.0 }
    }

    pub fn motion(&self) -> &Motion {
        &self.motion
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    /// `dt` 秒進め、`weight` (フェード込み) でパラメータへ適用する。
    pub fn update(&mut self, _parameters: &mut ParameterValues, dt: f32, _weight: f32) {
        // TODO(T0302): カーブ評価・フェード・ループ・イベント発火。
        self.time += dt;
    }
}
