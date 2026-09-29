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

mod dress_up;

pub use dress_up::{DressUpChange, DressUpError, TextureSource};

// ライティングの補助 (T0210)。
pub mod lighting;

use ugoe_model::effects::{EYE_BLINK_GROUP, Effects, LIP_SYNC_GROUP};
use ugoe_model::expression::Expression;
use ugoe_model::motion::{Motion, MotionEvent};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::pose::Pose;
use ugoe_model::validate::ValidationError;
use ugoe_motion::{
    BreathPlayer, ExpressionManager, EyeBlinkPlayer, LipSyncPlayer, MotionManager, PosePlayer,
    Priority, TrackingInput, TrackingMap,
};
use ugoe_runtime::{CompileError, CompiledModel, ModelInstance, ParamIndex};

/// ランタイム SDK のバージョン。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// [`Puppet::update`] の各段を動かすか (既定はすべて有効)。
///
/// 無効にした段はパラメータ・パーツの不透明度に触らない (`docs/spec/motion.md#段の有効無効`)。
/// エディタのプレビューで、物理やエフェクトを切って確かめるために使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggles {
    pub physics: bool,
    pub eye_blink: bool,
    pub breath: bool,
    pub lip_sync: bool,
    pub pose: bool,
}

impl Default for Toggles {
    fn default() -> Self {
        Self {
            physics: true,
            eye_blink: true,
            breath: true,
            lip_sync: true,
            pose: true,
        }
    }
}

/// 1 体分の動作単位。
///
/// モデルの `effects` (目パチ・呼吸・リップシンク) と `pose` は作成時に読み込む。
/// パラメータは [`Puppet::update`] の中で「基準値」に戻してからモーションを適用する
/// (`docs/spec/motion.md#フレーム内の適用順`)。ホストが `update` の合間に
/// [`ModelInstance::parameters_mut`] で書き換えた値は、新しい基準値として扱う。
#[derive(Debug)]
pub struct Puppet {
    instance: ModelInstance,
    physics: Option<ugoe_physics::PhysicsSystem>,
    motions: MotionManager,
    expressions: ExpressionManager,
    eye_blink: EyeBlinkPlayer,
    breath: BreathPlayer,
    lip_sync: LipSyncPlayer,
    pose: PosePlayer,
    /// `pose.opacities()` の各要素のパーツの添字。
    pose_parts: Vec<Option<usize>>,
    /// パラメータの基準値 (前フレームのモーション適用後の値と、ホストが書き換えた値)。
    base: Vec<f32>,
    /// 前フレームの `update` が最後に書き込んだパラメータ値 (ホストの書き換えの検出用)。
    output: Vec<f32>,
    /// モデルの `avatar.tracking` を解決したトラッキングの対応。
    tracking_map: TrackingMap,
    tracking: Option<TrackingInput>,
    toggles: Toggles,
    /// 着せ替えの本体と取り付けているモジュール (`dress_up.rs`)。モジュールを付けるまで `None`。
    wardrobe: Option<dress_up::Wardrobe>,
}

impl Puppet {
    pub fn new(model: ugoe_model::Model) -> Result<Self, CompileError> {
        Ok(Self::from_compiled(CompiledModel::compile(model)?))
    }

    pub fn from_compiled(compiled: Arc<CompiledModel>) -> Self {
        let model = compiled.model();
        let effects = model.effects.clone();
        let pose = model.pose.clone();
        let instance = ModelInstance::new(compiled);
        let values = instance.parameters().as_slice().to_vec();
        let tracking_map =
            TrackingMap::for_model(instance.compiled().model(), instance.parameters());
        let mut puppet = Self {
            instance,
            physics: None,
            motions: MotionManager::new(),
            expressions: ExpressionManager::new(),
            eye_blink: EyeBlinkPlayer::new(effects.eye_blink.clone()),
            breath: BreathPlayer::new(effects.breath.clone()),
            lip_sync: LipSyncPlayer::new(effects.lip_sync.clone()),
            pose: PosePlayer::new(pose.clone()),
            pose_parts: Vec::new(),
            base: values.clone(),
            output: values,
            tracking_map,
            tracking: None,
            toggles: Toggles::default(),
            wardrobe: None,
        };
        puppet.set_pose(pose);
        puppet
    }

    /// 物理を設定する。動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)
    /// を使う設定なら、`update` の評価は 2 段階になる (`docs/spec/evaluation.md#2-評価の順序`)。
    ///
    /// 参照先は検査しない (見つからない参照は実行時に無視する)。`.ugoe` の読み込み
    /// (`ugoe_format::read_bundle`) は検査済みの設定を返す。検査するなら [`Puppet::set_physics`] を使う。
    pub fn with_physics(mut self, settings: PhysicsSettings) -> Self {
        self.replace_physics(Some(settings));
        self
    }

    /// 物理の設定を差し替える (`None` で外す)。粒子は次の `update` で、その時の入力に対する
    /// 静止姿勢から始める。動的物理の加算量は 0 に戻す。
    ///
    /// 設定はモデルと合わせて `ugoe_model::validate::validate_physics` で検査し、エラーがあれば
    /// 何も変えずにそれを返す。エディタで設定を変えながらプレビューするために使う。
    pub fn set_physics(
        &mut self,
        settings: Option<PhysicsSettings>,
    ) -> Result<(), Vec<ValidationError>> {
        if let Some(settings) = &settings {
            let errors =
                ugoe_model::validate::validate_physics(settings, self.instance.compiled().model());
            if !errors.is_empty() {
                return Err(errors);
            }
        }
        self.replace_physics(settings);
        Ok(())
    }

    fn replace_physics(&mut self, settings: Option<PhysicsSettings>) {
        self.clear_dynamic_offsets();
        self.physics = settings.map(|settings| {
            let mut physics = ugoe_physics::PhysicsSystem::new(settings);
            physics.bind(&mut self.instance);
            physics
        });
    }

    /// 物理の状態 (設定は `physics().settings()`)。物理が無ければ `None`。
    pub fn physics(&self) -> Option<&ugoe_physics::PhysicsSystem> {
        self.physics.as_ref()
    }

    /// 物理の粒子を初期姿勢に戻す (次の `update` で静止姿勢に置き直す)。
    pub fn reset_physics(&mut self) {
        if let Some(physics) = &mut self.physics {
            physics.reset();
        }
    }

    /// 動的物理の加算量を 0 に戻す (物理を外したり止めたりしたとき、揺れた形のまま残さないため)。
    fn clear_dynamic_offsets(&mut self) {
        if self.instance.dynamic_offsets().is_some() {
            self.instance.dynamic_offsets_mut().clear();
        }
    }

    pub fn toggles(&self) -> Toggles {
        self.toggles
    }

    /// 各段を動かすかを設定する。物理を止めると動的物理の加算量を 0 に戻し、
    /// 再び動かすと粒子を静止姿勢から始める。止めた段は再開すると続きから動く
    /// (目パチ・呼吸の時刻は止めている間は進まない)。
    pub fn set_toggles(&mut self, toggles: Toggles) {
        if self.toggles.physics != toggles.physics {
            self.clear_dynamic_offsets();
            self.reset_physics();
        }
        self.toggles = toggles;
    }

    pub fn instance(&self) -> &ModelInstance {
        &self.instance
    }

    pub fn instance_mut(&mut self) -> &mut ModelInstance {
        &mut self.instance
    }

    /// `priority` でモーションを開始する。再生中のモーションより低い優先度なら開始せず `false` を返す
    /// (`docs/spec/motion.md#モーションマネージャ`)。
    pub fn start_motion(&mut self, motion: Motion, priority: Priority) -> bool {
        self.motions.start(motion, priority)
    }

    pub fn motions(&self) -> &MotionManager {
        &self.motions
    }

    pub fn motions_mut(&mut self) -> &mut MotionManager {
        &mut self.motions
    }

    /// 直前の [`Puppet::update`] で発火したモーションのイベント。
    pub fn motion_events(&self) -> &[MotionEvent] {
        self.motions.fired_events()
    }

    /// 表情を `expression` だけにする (使用中の表情はフェードアウトする)。
    pub fn set_expression(&mut self, expression: Expression) {
        self.expressions.set(expression);
    }

    pub fn expressions_mut(&mut self) -> &mut ExpressionManager {
        &mut self.expressions
    }

    /// 目パチ・呼吸・リップシンクの設定を差し替える (再生状態は初めからになる)。
    /// リップシンクの音量は保つ。
    pub fn set_effects(&mut self, effects: Effects) {
        let level = self.lip_sync.level();
        self.eye_blink = EyeBlinkPlayer::new(effects.eye_blink);
        self.breath = BreathPlayer::new(effects.breath);
        self.lip_sync = LipSyncPlayer::new(effects.lip_sync);
        self.lip_sync.set_level(level);
    }

    /// リップシンクの音量 (0〜1)。`None` ならリップシンクは口のパラメータに触らない。
    /// 音声の解析はホストが行う。
    pub fn set_lip_sync_level(&mut self, level: Option<f32>) {
        self.lip_sync.set_level(level);
    }

    /// フェイストラッキングの入力を設定する。`None` (既定) の間はトラッキングでパラメータに触らない。
    ///
    /// 入力は次の [`Puppet::update`] から毎フレーム、モーションの直後にモデルの `avatar.tracking` に従って
    /// パラメータに書き込む (`docs/spec/avatar-profile.md#フェイストラッキングとの対応`)。
    /// 書き込んだ値は基準値に含まれるので、表情・目パチ・呼吸はその上に重なる。`None` に戻しても、
    /// 最後に書き込んだ値は基準値として残る。平滑化などはホストが行ってから渡す。
    pub fn set_tracking(&mut self, input: Option<TrackingInput>) {
        self.tracking = input;
    }

    pub fn tracking(&self) -> Option<&TrackingInput> {
        self.tracking.as_ref()
    }

    /// ポーズの設定を差し替える (各グループは先頭の要素の表示から始める)。
    pub fn set_pose(&mut self, pose: Pose) {
        self.pose = PosePlayer::new(pose);
        let compiled = self.instance.compiled();
        self.pose_parts = self
            .pose
            .opacities()
            .map(|(part, _)| compiled.part_index(part.as_str()))
            .collect();
    }

    pub fn pose(&self) -> &PosePlayer {
        &self.pose
    }

    pub fn pose_mut(&mut self) -> &mut PosePlayer {
        &mut self.pose
    }

    /// 1 フレーム進める。適用順は `docs/spec/motion.md#フレーム内の適用順` に従う。
    ///
    /// パーツの不透明度の上書き ([`ModelInstance::set_part_opacity_override`]) は
    /// `Puppet` が毎フレーム作り直す。
    ///
    /// 動的物理を使う設定では、物理の段で 1 段目の評価を行い、最後の `ModelInstance::update()` が
    /// 2 段目になる (動的物理を使わなければ評価は 1 回)。
    pub fn update(&mut self, dt: f32) {
        let params = self.instance.parameters_mut();

        // 1. 基準値に戻す。前フレームの出力から変わっている値はホストが書き換えたので、基準値にする。
        for (i, (base, output)) in self.base.iter_mut().zip(&self.output).enumerate() {
            let index = ParamIndex(i as u32);
            let current = params.get(index);
            if current.to_bits() != output.to_bits() {
                *base = current;
            }
            params.set(index, *base);
        }

        // 2. モーション
        self.motions.update(params, dt);
        // 3. トラッキング (基準値に含める)
        if let Some(input) = &self.tracking {
            self.tracking_map.apply(input, params);
        }

        // 4. 基準値を保存する。これより後の処理は毎フレーム基準値に重ねるので累積しない。
        self.base.copy_from_slice(params.as_slice());

        // 5. 表情
        self.expressions.update(params, dt);
        // 6. 目パチ
        if self.toggles.eye_blink {
            self.eye_blink.advance(dt);
            let strength = self.motions.effect_group_weight(EYE_BLINK_GROUP);
            self.eye_blink.apply(params, strength);
        }
        // 7. 呼吸
        if self.toggles.breath {
            self.breath.advance(dt);
            self.breath.apply(params);
        }
        // 8. 物理
        // 動的物理を使う設定では、ここで 1 段目の評価を行い、11. の評価が 2 段目になる。
        if let Some(physics) = self.physics.as_mut().filter(|_| self.toggles.physics) {
            physics.update_instance(&mut self.instance, dt);
        }
        let params = self.instance.parameters_mut();
        // 9. リップシンク
        if self.toggles.lip_sync {
            let strength = self.motions.effect_group_weight(LIP_SYNC_GROUP);
            self.lip_sync.apply(params, strength);
        }
        self.output.copy_from_slice(params.as_slice());

        // 10. パーツの不透明度 (モーションのカーブとポーズ)
        self.apply_part_opacities(dt);

        // 11. 評価
        self.instance.update();
    }

    /// モーションのパーツの不透明度のカーブとポーズを、パーツの不透明度の上書きに書き込む。
    ///
    /// ポーズの要素のパーツへのカーブはポーズへの表示の要求 (値 × 重み) として扱い、
    /// それ以外のパーツへのカーブはそのまま上書きする。ポーズを止めているときは、
    /// すべてのカーブをそのまま上書きし、ポーズは何も書かない。
    fn apply_part_opacities(&mut self, dt: f32) {
        self.instance.clear_part_opacity_overrides();
        let pose = self.toggles.pose;
        let mut requests = Vec::new();
        for (part, value, weight) in self.motions.part_opacities() {
            if pose && self.pose.contains(&part) {
                requests.push((part, value * weight));
            } else if let Some(i) = self.instance.compiled().part_index(part.as_str()) {
                self.instance.set_part_opacity_override(i, value, weight);
            }
        }
        if !pose {
            return;
        }
        self.pose.update(dt, &requests);
        for ((_, opacity), part) in self.pose.opacities().zip(&self.pose_parts) {
            if let Some(part) = *part {
                self.instance.set_part_opacity_override(part, opacity, 1.0);
            }
        }
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

    /// アバタープロファイルの標準の当たり判定の名前 (`Head`・`Body`、`Model.avatar.hit_areas`) に
    /// 対応する当たり判定の ID。対応が無ければ `None` (`docs/spec/avatar-profile.md#アプリ向けの情報`)。
    pub fn standard_hit_area(&self, name: &str) -> Option<&str> {
        let avatar = self.instance.compiled().model().avatar.as_ref()?;
        avatar
            .hit_areas
            .iter()
            .find(|h| h.name == name)
            .map(|h| h.hit_area.as_str())
    }

    /// 標準の名前 `name` の当たり判定がキャンバス座標 `point` を含むか。対応が無ければ `false`。
    /// 規則は [`Puppet::hit_areas_at`] と同じ。
    pub fn hit_test_standard(&self, name: &str, point: ugoe_core::Vec2) -> bool {
        self.standard_hit_area(name)
            .is_some_and(|id| self.hit_test(id, point))
    }

    /// キャンバス座標 `point` を含む当たり判定のうち、標準の名前を持つものの名前を手前から順に並べる。
    /// 1 つの当たり判定に複数の名前があれば `Model.avatar.hit_areas` の順に並べる。
    pub fn standard_hit_areas_at(&self, point: ugoe_core::Vec2) -> Vec<&str> {
        let Some(avatar) = self.instance.compiled().model().avatar.as_ref() else {
            return Vec::new();
        };
        self.hit_areas_at(point)
            .into_iter()
            .flat_map(|id| {
                avatar
                    .hit_areas
                    .iter()
                    .filter(move |h| h.hit_area == id)
                    .map(|h| h.name.as_str())
            })
            .collect()
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

    use ugoe_model::expression::{ExpressionOp, ExpressionParameter};
    use ugoe_model::motion::{Curve, CurveTarget, Point, Segment};
    use ugoe_model::{fixtures, standard};

    const DT: f32 = 1.0 / 60.0;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4
    }

    fn param(puppet: &Puppet, id: &str) -> f32 {
        let p = puppet.instance().parameters();
        p.get(p.index_of(id).unwrap())
    }

    #[test]
    fn puppet_applies_tracking_into_the_base_values() {
        let mut model = fixtures::avatar_profile();
        // 目パチと呼吸を外し、トラッキングの値だけを見る。
        model.effects = ugoe_model::effects::Effects::default();
        let mut puppet = Puppet::new(model).unwrap();
        let mut input = TrackingInput::default();
        input.set_blend_shape("eyeBlinkLeft", 0.75);
        input.head.yaw = 10.0;
        puppet.set_tracking(Some(input));
        puppet.update(DT);
        assert!(close(param(&puppet, standard::EYE_L_OPEN), 0.25));
        assert!(close(param(&puppet, standard::ANGLE_X), 10.0));

        // 入力をやめると、最後の値が基準値として残る。
        puppet.set_tracking(None);
        puppet.update(DT);
        assert!(close(param(&puppet, standard::EYE_L_OPEN), 0.25));
        // ホストの書き換えはそのまま効く。
        let eye = puppet
            .instance()
            .parameters()
            .index_of(standard::EYE_L_OPEN)
            .unwrap();
        puppet.instance_mut().parameters_mut().set(eye, 0.5);
        puppet.update(DT);
        assert!(close(param(&puppet, standard::EYE_L_OPEN), 0.5));
    }

    fn set_param(puppet: &mut Puppet, id: &str, value: f32) {
        let p = puppet.instance_mut().parameters_mut();
        let i = p.index_of(id).unwrap();
        p.set(i, value);
    }

    fn constant(target: CurveTarget, v: f32) -> Curve {
        Curve {
            target,
            segments: vec![Segment::Start {
                p: Point { t: 0.0, v },
            }],
            fade_in: None,
            fade_out: None,
        }
    }

    fn motion(duration: f32, looping: bool, curves: Vec<Curve>) -> Motion {
        Motion {
            name: "m".into(),
            duration,
            looping,
            fade_in: 0.0,
            fade_out: 0.0,
            curves,
            events: vec![],
        }
    }

    fn param_curve(id: &str, v: f32) -> Curve {
        constant(CurveTarget::Parameter(id.into()), v)
    }

    /// エフェクトなしの `pose_and_effects` (個々の検査で必要なものだけ有効にする)。
    fn quiet_puppet() -> Puppet {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        puppet.set_effects(Effects::default());
        puppet
    }

    /// 呼吸は毎フレーム基準値に加えるので累積しない。
    #[test]
    fn breath_does_not_accumulate() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        let mut t = 0.0;
        for _ in 0..600 {
            puppet.update(DT);
            t += DT;
        }
        let expected = 0.5 * 15.0 * (std::f32::consts::TAU * t / 6.5345).sin();
        assert!(
            (param(&puppet, standard::ANGLE_X) - expected).abs() < 1e-2,
            "{} != {expected}",
            param(&puppet, standard::ANGLE_X)
        );
    }

    /// `update` の合間にホストが書き換えた値は基準値になり、エフェクトはその上に重なる。
    #[test]
    fn host_edits_become_base_values() {
        let mut puppet = quiet_puppet();
        puppet.set_effects(Effects {
            breath: ugoe_model::effects::Breath::standard(),
            ..Effects::default()
        });
        set_param(&mut puppet, standard::ANGLE_X, 10.0);
        puppet.update(3.2345 / 4.0);
        let breath = param(&puppet, standard::ANGLE_X) - 10.0;
        assert!(breath > 0.0);
        puppet.update(3.2345 / 4.0);
        let t = 3.2345 / 2.0;
        let expected = 10.0 + 7.5 * (std::f32::consts::TAU * t / 6.5345).sin();
        assert!(close(param(&puppet, standard::ANGLE_X), expected));
    }

    /// モーションが終わっても、最後に適用した値は基準値として残る。
    #[test]
    fn motion_result_persists_after_motion_ends() {
        let mut puppet = quiet_puppet();
        assert!(puppet.start_motion(
            motion(0.5, false, vec![param_curve(standard::ANGLE_X, 20.0)]),
            Priority::Normal
        ));
        puppet.update(0.25);
        assert!(close(param(&puppet, standard::ANGLE_X), 20.0));
        puppet.update(0.5);
        assert!(!puppet.motions().is_playing());
        puppet.update(0.1);
        assert!(close(param(&puppet, standard::ANGLE_X), 20.0));
    }

    /// 表情はモーションの結果に重ね、目パチはさらにその結果に掛ける。
    #[test]
    fn expression_then_eye_blink_apply_after_motion() {
        let mut puppet = quiet_puppet();
        puppet.start_motion(
            motion(10.0, true, vec![param_curve(standard::EYE_L_OPEN, 0.8)]),
            Priority::Normal,
        );
        puppet.set_expression(Expression {
            name: "half".into(),
            fade_in: 0.0,
            fade_out: 0.0,
            parameters: vec![ExpressionParameter {
                parameter: standard::EYE_L_OPEN.into(),
                value: 0.5,
                op: ExpressionOp::Multiply,
            }],
        });
        puppet.update(DT);
        assert!(close(param(&puppet, standard::EYE_L_OPEN), 0.4));
        // 目パチを有効にし、閉じている段階まで進める。
        puppet.set_effects(Effects {
            eye_blink: ugoe_model::effects::EyeBlink {
                interval: 1.0,
                ..ugoe_model::effects::EyeBlink::standard()
            },
            ..Effects::default()
        });
        let mut closed = false;
        for _ in 0..240 {
            puppet.update(DT);
            if param(&puppet, standard::EYE_L_OPEN) == 0.0 {
                closed = true;
            }
            assert!(param(&puppet, standard::EYE_L_OPEN) <= 0.4 + 1e-6);
        }
        assert!(closed);
    }

    /// モーションのエフェクトグループ `EyeBlink` が 0 なら目パチしない。
    #[test]
    fn effect_group_curve_suppresses_eye_blink() {
        let mut puppet = quiet_puppet();
        puppet.set_effects(Effects {
            eye_blink: ugoe_model::effects::EyeBlink {
                interval: 0.5,
                ..ugoe_model::effects::EyeBlink::standard()
            },
            ..Effects::default()
        });
        puppet.start_motion(
            motion(
                10.0,
                true,
                vec![constant(
                    CurveTarget::EffectGroup(EYE_BLINK_GROUP.into()),
                    0.0,
                )],
            ),
            Priority::Normal,
        );
        for _ in 0..240 {
            puppet.update(DT);
            assert_eq!(param(&puppet, standard::EYE_L_OPEN), 1.0);
        }
    }

    #[test]
    fn lip_sync_writes_host_level() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        puppet.update(DT);
        assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
        puppet.set_lip_sync_level(Some(0.7));
        puppet.update(DT);
        assert!(close(param(&puppet, standard::MOUTH_OPEN_Y), 0.7));
        // 音量は基準値に入らない。
        puppet.set_lip_sync_level(None);
        puppet.update(DT);
        assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
    }

    /// ポーズの要素へのパーツの不透明度のカーブは表示の要求になり、クロスフェードで切り替わる。
    #[test]
    fn pose_switches_by_part_opacity_curve() {
        let mut puppet = quiet_puppet();
        let part = |puppet: &Puppet, id: &str| {
            let i = puppet.instance().compiled().part_index(id).unwrap();
            puppet.instance().part_opacity_override(i)
        };
        puppet.update(DT);
        assert_eq!(part(&puppet, "ArmA"), (1.0, 1.0));
        assert_eq!(part(&puppet, "SleeveA"), (1.0, 1.0));
        assert_eq!(part(&puppet, "ArmB"), (0.0, 1.0));

        puppet.start_motion(
            motion(
                10.0,
                true,
                vec![
                    constant(CurveTarget::PartOpacity("ArmA".into()), 0.0),
                    constant(CurveTarget::PartOpacity("ArmB".into()), 1.0),
                ],
            ),
            Priority::Normal,
        );
        puppet.update(0.25);
        let (b, _) = part(&puppet, "ArmB");
        assert!(close(b, 0.5), "{b}");
        assert!(close(part(&puppet, "SleeveA").0, 0.5));
        puppet.update(0.5);
        assert_eq!(part(&puppet, "ArmB"), (1.0, 1.0));
        assert_eq!(part(&puppet, "ArmA"), (0.0, 1.0));
    }

    /// ポーズに属さないパーツへのカーブは、パーツの不透明度を直接上書きする。
    #[test]
    fn part_opacity_curve_overrides_other_parts() {
        let mut puppet = quiet_puppet();
        puppet.start_motion(
            motion(
                10.0,
                true,
                vec![constant(CurveTarget::PartOpacity("Root".into()), 0.25)],
            ),
            Priority::Normal,
        );
        puppet.update(DT);
        assert!(close(puppet.instance().drawables()[0].opacity, 0.25));
        puppet.motions_mut().stop_all();
        puppet.update(DT);
        assert!(close(puppet.instance().drawables()[0].opacity, 1.0));
    }

    #[test]
    fn motion_events_are_reported() {
        let mut puppet = quiet_puppet();
        let mut m = motion(1.0, false, vec![]);
        m.events.push(MotionEvent {
            t: 0.5,
            value: "hello".into(),
        });
        puppet.start_motion(m, Priority::Normal);
        puppet.update(0.25);
        assert!(puppet.motion_events().is_empty());
        puppet.update(0.5);
        assert_eq!(puppet.motion_events()[0].value, "hello");
    }

    /// 同じ入力列からは同じ出力になる (目パチの乱数もシードで決まる)。
    #[test]
    fn update_is_deterministic() {
        let run = || {
            let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
            let mut trace = Vec::new();
            for _ in 0..300 {
                puppet.update(DT);
                trace.extend_from_slice(puppet.instance().parameters().as_slice());
            }
            trace
        };
        assert_eq!(run(), run());
    }

    // 動的物理: 揺れ用のパラメータなしで髪のデフォーマが揺れる。評価は 1 フレームに 1 回と数える。
    #[test]
    fn puppet_runs_dynamic_physics() {
        let mut puppet = Puppet::new(ugoe_model::fixtures::dynamic_hair())
            .unwrap()
            .with_physics(ugoe_model::fixtures::dynamic_hair_settings());
        let hair1 = puppet
            .instance()
            .compiled()
            .deformer_index("Hair1")
            .unwrap();
        puppet.update(1.0 / 60.0);
        let rest = puppet.instance().rotation_world_frame(hair1).unwrap();
        let x = puppet.instance().parameters().index_of("AngleX").unwrap();
        puppet.instance_mut().parameters_mut().set(x, 30.0);
        let count = puppet.instance().update_count();
        for _ in 0..5 {
            puppet.update(1.0 / 60.0);
        }
        assert_eq!(puppet.instance().update_count(), count + 5);
        let moved = puppet.instance().rotation_world_frame(hair1).unwrap();
        // 頭と一緒に平行移動するだけでなく、遅れて回る。
        assert!(!moved.matrix2.abs_diff_eq(rest.matrix2, 1e-3));
    }

    #[test]
    fn puppet_without_dynamic_physics_evaluates_once() {
        let mut puppet = Puppet::new(ugoe_model::fixtures::physics_hair())
            .unwrap()
            .with_physics(ugoe_model::fixtures::physics_hair_settings());
        let count = puppet.instance().update_count();
        puppet.update(1.0 / 60.0);
        assert_eq!(puppet.instance().update_count(), count + 1);
        assert!(puppet.instance().dynamic_offsets().is_none());
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

    #[test]
    fn standard_hit_area_names_resolve_through_the_avatar_profile() {
        use ugoe_model::avatar::{AvatarHitArea, AvatarProfile, HIT_AREA_BODY, HIT_AREA_HEAD};
        let mut model = fixtures::single_quad();
        model.hit_areas.push(ugoe_model::HitArea {
            id: "QuadArea".into(),
            mesh: "Quad".into(),
        });
        model.avatar = Some(AvatarProfile {
            hit_areas: vec![AvatarHitArea {
                name: HIT_AREA_BODY.into(),
                hit_area: "QuadArea".into(),
            }],
            ..AvatarProfile::default()
        });
        let puppet = Puppet::new(model).unwrap();
        let inside = ugoe_core::Vec2::new(100.0, 100.0);
        assert_eq!(puppet.standard_hit_area(HIT_AREA_BODY), Some("QuadArea"));
        assert_eq!(puppet.standard_hit_area(HIT_AREA_HEAD), None);
        assert!(puppet.hit_test_standard(HIT_AREA_BODY, inside));
        assert!(!puppet.hit_test_standard(HIT_AREA_HEAD, inside));
        assert!(!puppet.hit_test_standard(HIT_AREA_BODY, ugoe_core::Vec2::new(10.0, 10.0)));
        assert_eq!(puppet.standard_hit_areas_at(inside), vec![HIT_AREA_BODY]);

        // アバタープロファイルが無ければ当たらない。
        let puppet = Puppet::new(fixtures::single_quad()).unwrap();
        assert!(puppet.standard_hit_areas_at(inside).is_empty());
    }

    #[test]
    fn set_physics_validates_against_the_model() {
        let mut puppet = Puppet::new(fixtures::physics_hair()).unwrap();
        let mut bad = fixtures::physics_hair_settings();
        bad.rigs[0].inputs[0].parameter = "Missing".into();
        assert!(!puppet.set_physics(Some(bad)).unwrap_err().is_empty());
        assert!(puppet.physics().is_none());
        puppet
            .set_physics(Some(fixtures::physics_hair_settings()))
            .unwrap();
        assert!(puppet.physics().is_some());
        puppet.set_physics(None).unwrap();
        assert!(puppet.physics().is_none());
    }

    /// 物理を止めると出力のパラメータは基準値に戻り、動的物理の加算量も 0 になる。
    #[test]
    fn physics_toggle_restores_outputs_and_offsets() {
        let mut puppet = Puppet::new(fixtures::dynamic_hair())
            .unwrap()
            .with_physics(fixtures::dynamic_hair_settings());
        puppet.update(DT);
        set_param(&mut puppet, "AngleX", 30.0);
        for _ in 0..5 {
            puppet.update(DT);
        }
        let hair1 = puppet
            .instance()
            .compiled()
            .deformer_index("Hair1")
            .unwrap();
        let swinging = puppet.instance().rotation_world_frame(hair1).unwrap();

        puppet.set_toggles(Toggles {
            physics: false,
            ..Toggles::default()
        });
        puppet.update(DT);
        let still = puppet.instance().rotation_world_frame(hair1).unwrap();
        assert!(!swinging.matrix2.abs_diff_eq(still.matrix2, 1e-3));
        // 物理なしで作ったものと同じ形になる。
        let mut plain = Puppet::new(fixtures::dynamic_hair()).unwrap();
        set_param(&mut plain, "AngleX", 30.0);
        plain.update(DT);
        let expected = plain.instance().rotation_world_frame(hair1).unwrap();
        assert!(still.matrix2.abs_diff_eq(expected.matrix2, 1e-4));
        assert!(still.translation.abs_diff_eq(expected.translation, 1e-3));
    }

    #[test]
    fn disabled_effects_do_not_touch_parameters() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        puppet.set_toggles(Toggles {
            breath: false,
            eye_blink: false,
            lip_sync: false,
            ..Toggles::default()
        });
        puppet.set_lip_sync_level(Some(0.7));
        for _ in 0..240 {
            puppet.update(DT);
            assert_eq!(param(&puppet, standard::ANGLE_X), 0.0);
            assert_eq!(param(&puppet, standard::EYE_L_OPEN), 1.0);
            assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
        }
        puppet.set_toggles(Toggles::default());
        puppet.update(DT);
        assert!(close(param(&puppet, standard::MOUTH_OPEN_Y), 0.7));
    }

    /// ポーズを止めると、ポーズの要素へのカーブもそのまま上書きになり、ポーズは何も書かない。
    #[test]
    fn disabled_pose_leaves_part_opacities_to_curves() {
        let mut puppet = quiet_puppet();
        puppet.set_toggles(Toggles {
            pose: false,
            ..Toggles::default()
        });
        let part = |puppet: &Puppet, id: &str| {
            let i = puppet.instance().compiled().part_index(id).unwrap();
            puppet.instance().part_opacity_override(i)
        };
        puppet.update(DT);
        assert_eq!(part(&puppet, "ArmA").1, 0.0);
        assert_eq!(part(&puppet, "ArmB").1, 0.0);
        puppet.start_motion(
            motion(
                10.0,
                true,
                vec![constant(CurveTarget::PartOpacity("ArmB".into()), 1.0)],
            ),
            Priority::Normal,
        );
        puppet.update(DT);
        assert_eq!(part(&puppet, "ArmB"), (1.0, 1.0));
        assert_eq!(part(&puppet, "ArmA").1, 0.0);
    }
}
