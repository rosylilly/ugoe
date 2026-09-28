//! ugoe ランタイム SDK。アプリ・ゲームエンジンはまずこのクレートを使う。
//!
//! [`Puppet`] が 1 体分の「モデル + 物理 + モーション + 常時エフェクト」をまとめ、
//! `update(dt)` 1 回で 1 フレーム分のパラメータ更新と変形評価を行う。
//! 描画は `render` feature 相当の [`ugoe_render`] か、エンジン独自のレンダラで行う。

use std::sync::Arc;

pub use ugoe_core as core;
pub use ugoe_format as format;
pub use ugoe_model as model;
pub use ugoe_motion as motion;
pub use ugoe_physics as physics;
pub use ugoe_render as render;
pub use ugoe_runtime as runtime;

use ugoe_runtime::{CompileError, CompiledModel, ModelInstance};

/// ランタイム SDK のバージョン。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 1 体分の動作単位。
#[derive(Debug)]
pub struct Puppet {
    instance: ModelInstance,
    physics: Option<ugoe_physics::PhysicsSystem>,
    motions: Vec<ugoe_motion::MotionPlayer>,
}

impl Puppet {
    pub fn new(model: ugoe_model::Model) -> Result<Self, CompileError> {
        Ok(Self::from_compiled(CompiledModel::compile(model)?))
    }

    pub fn from_compiled(compiled: Arc<CompiledModel>) -> Self {
        Self {
            instance: ModelInstance::new(compiled),
            physics: None,
            motions: Vec::new(),
        }
    }

    pub fn with_physics(mut self, settings: ugoe_model::physics::PhysicsSettings) -> Self {
        self.physics = Some(ugoe_physics::PhysicsSystem::new(settings));
        self
    }

    pub fn instance(&self) -> &ModelInstance {
        &self.instance
    }

    pub fn instance_mut(&mut self) -> &mut ModelInstance {
        &mut self.instance
    }

    pub fn play(&mut self, motion: ugoe_model::motion::Motion) {
        // TODO(T0303): 優先度・キュー・クロスフェードを持つモーションマネージャに置き換える。
        self.motions.push(ugoe_motion::MotionPlayer::new(motion));
    }

    /// 1 フレーム進める。適用順は `docs/spec/motion.md#フレーム内の適用順` に従う。
    pub fn update(&mut self, dt: f32) {
        let params = self.instance.parameters_mut();
        for m in &mut self.motions {
            m.update(params, dt, 1.0);
        }
        // TODO(T0303): 表情・目パチ・呼吸
        if let Some(physics) = &mut self.physics {
            physics.update(params, dt);
        }
        // TODO(T0303): ポーズ・リップシンク
        self.instance.update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puppet_updates_fixture() {
        let mut puppet = Puppet::new(ugoe_model::fixtures::single_quad()).unwrap();
        puppet.update(1.0 / 60.0);
        assert_eq!(puppet.instance().drawables().len(), 1);
    }
}
