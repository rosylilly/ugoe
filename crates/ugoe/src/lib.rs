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

    /// キャンバス座標 `point` を含む当たり判定 (`Model::hit_areas`) の ID を、手前から順に並べる。
    ///
    /// 直前の [`Puppet::update`] で評価した変形後の形で判定する。判定の規則は
    /// `docs/spec/evaluation.md#9-当たり判定` (不透明度 0 や非表示のメッシュでも当たる)。
    pub fn hit_areas_at(&self, point: ugoe_core::Vec2) -> Vec<&str> {
        let hit_areas = &self.instance.compiled().model().hit_areas;
        self.instance
            .hit_areas_at(point)
            .into_iter()
            .map(|h| hit_areas[h].id.as_str())
            .collect()
    }

    /// ID が `hit_area` の当たり判定がキャンバス座標 `point` を含むか。未知の ID なら `false`。
    /// 規則は [`Puppet::hit_areas_at`] と同じ。
    pub fn hit_test(&self, hit_area: &str, point: ugoe_core::Vec2) -> bool {
        self.instance
            .compiled()
            .hit_area_index(hit_area)
            .is_some_and(|h| self.instance.hit_area_contains(h, point))
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

    #[test]
    fn puppet_hit_areas_follow_parameters() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.hit_areas.push(ugoe_model::HitArea {
            id: "Body".into(),
            mesh: "Quad".into(),
        });
        let mut puppet = Puppet::new(model).unwrap();
        let point = ugoe_core::Vec2::new(220.0, 100.0);
        assert!(puppet.hit_areas_at(point).is_empty());
        assert!(!puppet.hit_test("Body", point));

        let instance = puppet.instance_mut();
        let x = instance.parameters().index_of("X").unwrap();
        instance.parameters_mut().set(x, 1.0);
        puppet.update(1.0 / 60.0);
        assert_eq!(puppet.hit_areas_at(point), vec!["Body"]);
        assert!(puppet.hit_test("Body", point));
        assert!(!puppet.hit_test("Head", point));
    }
}
