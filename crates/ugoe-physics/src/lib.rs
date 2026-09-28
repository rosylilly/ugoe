//! 振り子チェーンによる物理演算。仕様: `docs/spec/physics.md`。
//!
//! 入力パラメータ (顔の角度など) から根元の変位を作り、粒子チェーンを固定ステップで
//! シミュレーションし、セグメント角度を出力パラメータ (髪揺れなど) に書き戻す。

use ugoe_model::physics::PhysicsSettings;
use ugoe_runtime::ParameterValues;

/// 物理演算の実行時状態。
#[derive(Debug)]
pub struct PhysicsSystem {
    settings: PhysicsSettings,
    accumulator: f32,
}

impl PhysicsSystem {
    pub fn new(settings: PhysicsSettings) -> Self {
        Self {
            settings,
            accumulator: 0.0,
        }
    }

    pub fn settings(&self) -> &PhysicsSettings {
        &self.settings
    }

    /// `dt` 秒進め、出力パラメータを書き換える。
    pub fn update(&mut self, _parameters: &mut ParameterValues, dt: f32) {
        // TODO(T0301): 固定ステップ積分と入出力の実装。
        self.accumulator += dt;
    }

    /// 粒子を初期姿勢に戻す (モデル切替・テレポート時)。
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}
