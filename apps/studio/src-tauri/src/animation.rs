//! 指揮役の演技: モーション・表情・待機モーション・トラッキング・口パクの入力 (T0803、`docs/studio.md#入力`)。
//!
//! 入力は指揮役の `Puppet` に入れ、受け手には値を配るだけにする (ADR 0028 の 3.・4.)。規則はビューアの再生の層
//! (`packages/player/src/player.svelte.ts`・`animation.ts`、T0720〜T0722) と同じ:
//!
//! - 待機モーションは、何も再生していないときに待機 (idle) の優先度で流す。モーションが終わったら待機に戻る。
//! - トラッキング中のモーションの優先 (`Puppet::set_motion_over_tracking`) は、再生中のモーションが待機でないときだけ効かせる
//!   (ループする待機モーションが頭の向きなどをずっと上書きしないように)。
//! - トラッキングを外したら、トラッキングが書き込んでいたパラメータを既定値に戻す (最後の値が基準値として残るため)。
//!
//! 時計とスレッドは持たない ([`crate::conductor::Conductor::tick`] が `update` の前後に呼ぶ)。

use serde::Serialize;
use ugoe::Puppet;
use ugoe::model::expression::Expression;
use ugoe::model::motion::{CurveTarget, Motion};
use ugoe::motion::{Priority, TrackingInput, TrackingMap};
use ugoe::runtime::ParamIndex;

use crate::hotkeys::MotionPriority;
use crate::input::MotionSettings;

/// 名前のモーション・表情がモデルに無い。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum AnimationError {
    #[error("no model is on air")]
    #[serde(rename = "noModel")]
    NoModel,
    #[error("unknown motion `{name}`")]
    #[serde(rename = "unknownMotion")]
    UnknownMotion { name: String },
    #[error("unknown expression `{name}`")]
    #[serde(rename = "unknownExpression")]
    UnknownExpression { name: String },
}

/// 再生中のモーション。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMotion {
    pub name: String,
    pub priority: MotionPriority,
}

/// 画面に出すモーションの情報 (`@ugoe/player` の `MotionInfo` と同じ形)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionInfo {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub fade_in: f32,
    pub fade_out: f32,
    /// カーブが動かすパラメータの ID (重なりなし、カーブの順)。
    pub parameters: Vec<String>,
}

impl MotionInfo {
    fn of(m: &Motion) -> Self {
        let mut parameters: Vec<String> = Vec::new();
        for c in &m.curves {
            if let CurveTarget::Parameter(id) = &c.target
                && !parameters.iter().any(|p| id == p.as_str())
            {
                parameters.push(id.as_str().to_owned());
            }
        }
        Self {
            name: m.name.clone(),
            duration: m.duration,
            looping: m.looping,
            fade_in: m.fade_in,
            fade_out: m.fade_out,
            parameters,
        }
    }
}

/// 操作ウィンドウに出す演技の様子 (`lib.rs` の `Monitor::Animation`)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationView {
    pub motions: Vec<MotionInfo>,
    pub expressions: Vec<String>,
    pub current: Option<CurrentMotion>,
    /// 付けている表情 (付けた順)。
    pub active_expressions: Vec<String>,
    /// 流す待機モーション (設定とモデルから決めたもの)。無ければ `None`。
    pub idle_motion: Option<String>,
    /// トラッキングの入力が届いている。
    pub tracking: bool,
    pub tracking_paused: bool,
    pub lip_sync_paused: bool,
    /// トラッキングが書き込むパラメータの ID (モデルの定義順)。
    pub tracking_parameters: Vec<String>,
    /// モデルのスロット (着せ替えの割り当てに使う)。
    pub slots: Vec<String>,
    /// 今のモーションを始めた番号 (始めるたびに上がる)。同じモーションを始め直したことを見分ける (外部 API の通知)。
    #[serde(skip)]
    pub motion_start: u64,
}

/// 演技の状態。
#[derive(Debug, Default)]
pub struct Animation {
    motions: Vec<Motion>,
    expressions: Vec<Expression>,
    current: Option<(String, Priority)>,
    active_expressions: Vec<String>,
    settings: MotionSettings,
    /// トラッキングの入力 (届いた最後のもの。一時停止中は `None`)。
    tracking: Option<TrackingInput>,
    tracking_paused: bool,
    /// マイクの音量 (一時停止中も覚え、再開したら次の値から使う)。
    lip_sync_level: Option<f32>,
    lip_sync_paused: bool,
    /// トラッキング中に自動の目パチを止めるか (操作ウィンドウの設定)。
    auto_eye_blink: bool,
    /// 直前に `Puppet` に渡したモーションの優先 (変わったときだけ渡す)。
    over_applied: Option<bool>,
    /// モーションを始めた数 (始めるたびに上がる。同じ名前で始め直しても変わる)。
    starts: u64,
    /// 画面に知らせる変化があった。
    changed: bool,
}

impl Animation {
    pub fn new() -> Self {
        Self {
            auto_eye_blink: true,
            ..Self::default()
        }
    }

    /// 新しいモデルを読んだ: ライブラリを替え、モーションと表情を初めからにし、入力をそのまま渡し直して待機を流す。
    pub fn load(
        &mut self,
        puppet: &mut Puppet,
        motions: Vec<Motion>,
        expressions: Vec<Expression>,
    ) {
        self.motions = motions;
        self.expressions = expressions;
        self.current = None;
        self.active_expressions.clear();
        self.over_applied = None;
        puppet.set_tracking_kept(self.settings.tracking_kept.iter().cloned());
        puppet.set_tracking(self.tracking);
        puppet.set_lip_sync_level(self.effective_lip_sync());
        self.apply_eye_blink(puppet);
        self.start_idle(puppet);
        self.changed = true;
    }

    /// モデルを外した。
    pub fn unload(&mut self) {
        self.motions.clear();
        self.expressions.clear();
        self.current = None;
        self.active_expressions.clear();
        self.changed = true;
    }

    /// 変化があったか (読むと戻る)。
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    pub fn settings(&self) -> &MotionSettings {
        &self.settings
    }

    /// `update` の前: トラッキング中のモーションの優先を渡す。
    pub fn before_update(&mut self, puppet: &mut Puppet) {
        let over = self.settings.over_tracking
            && self.tracking.is_some()
            && self
                .current
                .as_ref()
                .is_some_and(|(_, priority)| *priority != Priority::Idle);
        if self.over_applied != Some(over) {
            self.over_applied = Some(over);
            puppet.set_motion_over_tracking(over);
        }
    }

    /// `update` の後: モーションが終わっていれば待機に戻る。
    pub fn after_update(&mut self, puppet: &mut Puppet) {
        if self.current.is_some() && !puppet.motions().is_playing() {
            self.current = None;
            self.changed = true;
            self.start_idle(puppet);
        }
    }

    // --- モーション ---

    /// モーション `name` を `priority` で始める。再生中のモーションより優先度が低ければ始めず `Ok(false)`。
    pub fn play(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
        priority: Priority,
    ) -> Result<bool, AnimationError> {
        let motion = self
            .motions
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| AnimationError::UnknownMotion {
                name: name.to_owned(),
            })?
            .clone();
        if !puppet.start_motion(motion, priority) {
            return Ok(false);
        }
        self.current = Some((name.to_owned(), priority));
        self.starts += 1;
        self.changed = true;
        Ok(true)
    }

    /// 再生中のモーションをフェードアウトさせる。待機モーションそのものを止めたのでなければ、待機に戻る。
    pub fn stop(&mut self, puppet: &mut Puppet) {
        puppet.motions_mut().stop_all();
        let stopped = self.current.take();
        self.changed = true;
        if stopped.is_some_and(|(name, _)| Some(name.as_str()) != self.idle_motion()) {
            self.start_idle(puppet);
        }
    }

    /// 流す待機モーションの名前: 設定の名前がモデルにあればそれ、無ければ `idle` (大文字小文字を区別しない)、
    /// 無ければ `idle` で始まるもの (ループするものを先に)。`auto_idle` が切なら `None`。
    pub fn idle_motion(&self) -> Option<&str> {
        if !self.settings.auto_idle {
            return None;
        }
        if let Some(name) = &self.settings.idle_motion
            && self.motions.iter().any(|m| &m.name == name)
        {
            return Some(name);
        }
        default_idle_motion(&self.motions)
    }

    fn start_idle(&mut self, puppet: &mut Puppet) {
        let Some(name) = self.idle_motion().map(str::to_owned) else {
            return;
        };
        // 待機は低い優先度なので、ほかのモーションが再生中なら始まらない。
        if let Ok(true) = self.play(puppet, &name, Priority::Idle) {
            self.changed = true;
        }
    }

    /// 設定を替える。待機を入れた (何も再生していなければ流す)・切った (流している待機を止める)・待機を替えた
    /// (流しているなら替える) ことを反映する。
    pub fn set_settings(&mut self, puppet: Option<&mut Puppet>, settings: MotionSettings) {
        let playing_idle = self
            .current
            .as_ref()
            .is_some_and(|(_, priority)| *priority == Priority::Idle);
        let before = self.idle_motion().map(str::to_owned);
        let kept_changed = settings.tracking_kept != self.settings.tracking_kept;
        self.settings = settings;
        self.changed = true;
        let Some(puppet) = puppet else {
            return;
        };
        if kept_changed {
            puppet.set_tracking_kept(self.settings.tracking_kept.iter().cloned());
        }
        let after = self.idle_motion().map(str::to_owned);
        if playing_idle && before != after {
            puppet.motions_mut().stop_all();
            self.current = None;
            self.start_idle(puppet);
        } else if self.current.is_none() {
            self.start_idle(puppet);
        }
    }

    // --- 表情 ---

    fn expression(&self, name: &str) -> Result<Expression, AnimationError> {
        self.expressions
            .iter()
            .find(|e| e.name == name)
            .cloned()
            .ok_or_else(|| AnimationError::UnknownExpression {
                name: name.to_owned(),
            })
    }

    /// 表情を `name` だけにする。
    pub fn set_expression(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
    ) -> Result<(), AnimationError> {
        let expression = self.expression(name)?;
        puppet.set_expression(expression);
        self.active_expressions = vec![name.to_owned()];
        self.changed = true;
        Ok(())
    }

    /// 表情 `name` を重ねる (付けていれば何もしない)。
    pub fn add_expression(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
    ) -> Result<(), AnimationError> {
        let expression = self.expression(name)?;
        if self.active_expressions.iter().any(|n| n == name) {
            return Ok(());
        }
        puppet.expressions_mut().add(expression);
        self.active_expressions.push(name.to_owned());
        self.changed = true;
        Ok(())
    }

    /// 表情 `name` を外す (フェードアウト)。
    pub fn remove_expression(&mut self, puppet: &mut Puppet, name: &str) {
        puppet.expressions_mut().stop(name);
        self.active_expressions.retain(|n| n != name);
        self.changed = true;
    }

    /// 付けていれば外し、外していれば重ねる。
    pub fn toggle_expression(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
    ) -> Result<(), AnimationError> {
        if self.active_expressions.iter().any(|n| n == name) {
            self.remove_expression(puppet, name);
            Ok(())
        } else {
            self.add_expression(puppet, name)
        }
    }

    /// 表情をすべて外す。
    pub fn clear_expressions(&mut self, puppet: &mut Puppet) {
        puppet.expressions_mut().stop_all();
        self.active_expressions.clear();
        self.changed = true;
    }

    // --- トラッキング ---

    /// トラッキングの入力。`None` で外し、トラッキングが書き込んでいたパラメータを既定値に戻す。一時停止中は捨てる。
    pub fn set_tracking(&mut self, puppet: Option<&mut Puppet>, input: Option<TrackingInput>) {
        if self.tracking_paused {
            return;
        }
        self.replace_tracking(puppet, input);
    }

    fn replace_tracking(&mut self, puppet: Option<&mut Puppet>, input: Option<TrackingInput>) {
        let was = self.tracking.is_some();
        self.tracking = input;
        if was != input.is_some() {
            self.changed = true;
        }
        let Some(puppet) = puppet else {
            return;
        };
        puppet.set_tracking(input);
        if was && input.is_none() {
            reset_tracked_parameters(puppet);
        }
    }

    /// トラッキングを一時停止する・再開する。止めるときはトラッキングを外す (パラメータは既定値に戻る)。
    pub fn set_tracking_paused(&mut self, puppet: Option<&mut Puppet>, paused: bool) {
        if paused == self.tracking_paused {
            return;
        }
        if paused {
            self.replace_tracking(puppet, None);
        }
        self.tracking_paused = paused;
        self.changed = true;
    }

    pub fn tracking_paused(&self) -> bool {
        self.tracking_paused
    }

    /// 自動の目パチを動かすか (トラッキング中は目の開閉に重なるので、操作ウィンドウが止める)。
    pub fn set_auto_eye_blink(&mut self, puppet: Option<&mut Puppet>, enabled: bool) {
        self.auto_eye_blink = enabled;
        if let Some(puppet) = puppet {
            self.apply_eye_blink(puppet);
        }
    }

    fn apply_eye_blink(&self, puppet: &mut Puppet) {
        let mut toggles = puppet.toggles();
        if toggles.eye_blink != self.auto_eye_blink {
            toggles.eye_blink = self.auto_eye_blink;
            puppet.set_toggles(toggles);
        }
    }

    // --- 口パク ---

    /// マイクの音量 (0〜1)。`None` で口パクを止める (口のパラメータに触らない)。
    pub fn set_lip_sync_level(&mut self, puppet: Option<&mut Puppet>, level: Option<f32>) {
        self.lip_sync_level = level.filter(|l| l.is_finite());
        if let Some(puppet) = puppet {
            puppet.set_lip_sync_level(self.effective_lip_sync());
        }
    }

    /// 口パクを一時停止する・再開する。
    pub fn set_lip_sync_paused(&mut self, puppet: Option<&mut Puppet>, paused: bool) {
        if paused == self.lip_sync_paused {
            return;
        }
        self.lip_sync_paused = paused;
        self.changed = true;
        if let Some(puppet) = puppet {
            puppet.set_lip_sync_level(self.effective_lip_sync());
        }
    }

    pub fn lip_sync_paused(&self) -> bool {
        self.lip_sync_paused
    }

    fn effective_lip_sync(&self) -> Option<f32> {
        self.lip_sync_level.filter(|_| !self.lip_sync_paused)
    }

    /// 画面に出す様子。
    pub fn view(&self, puppet: Option<&Puppet>) -> AnimationView {
        let (tracking_parameters, slots) = puppet.map_or_else(Default::default, |puppet| {
            let model = puppet.instance().compiled().model();
            let mut ids: Vec<ParamIndex> = tracking_map(puppet).targets().collect();
            ids.sort_unstable_by_key(|i| i.0);
            ids.dedup();
            let tracked = ids
                .iter()
                .filter_map(|i| model.parameters.get(i.0 as usize))
                .map(|p| p.id.as_str().to_owned())
                .collect();
            let slots = puppet
                .base_model()
                .slots
                .iter()
                .map(|s| s.id.as_str().to_owned())
                .collect();
            (tracked, slots)
        });
        AnimationView {
            motions: self.motions.iter().map(MotionInfo::of).collect(),
            expressions: self.expressions.iter().map(|e| e.name.clone()).collect(),
            current: self.current.as_ref().map(|(name, priority)| CurrentMotion {
                name: name.clone(),
                priority: (*priority).into(),
            }),
            active_expressions: self.active_expressions.clone(),
            idle_motion: self.idle_motion().map(str::to_owned),
            tracking: self.tracking.is_some(),
            tracking_paused: self.tracking_paused,
            lip_sync_paused: self.lip_sync_paused,
            tracking_parameters,
            slots,
            motion_start: self.starts,
        }
    }
}

/// 名前で選ぶ待機モーション (`@ugoe/player` の `defaultIdleMotion` と同じ規則)。
pub fn default_idle_motion(motions: &[Motion]) -> Option<&str> {
    if let Some(m) = motions.iter().find(|m| m.name.eq_ignore_ascii_case("idle")) {
        return Some(&m.name);
    }
    let prefixed: Vec<&Motion> = motions
        .iter()
        .filter(|m| m.name.to_lowercase().starts_with("idle"))
        .collect();
    prefixed
        .iter()
        .find(|m| m.looping)
        .or(prefixed.first())
        .map(|m| m.name.as_str())
}

fn tracking_map(puppet: &Puppet) -> TrackingMap {
    let instance = puppet.instance();
    TrackingMap::for_model(instance.compiled().model(), instance.parameters())
}

/// トラッキングが書き込むパラメータを既定値に戻す (ホストの書き換えなので、次の `update` から基準値になる)。
fn reset_tracked_parameters(puppet: &mut Puppet) {
    let targets: Vec<ParamIndex> = tracking_map(puppet).targets().collect();
    let defaults: Vec<f32> = puppet
        .instance()
        .compiled()
        .model()
        .parameters
        .iter()
        .map(|p| p.default)
        .collect();
    let params = puppet.instance_mut().parameters_mut();
    for index in targets {
        if let Some(default) = defaults.get(index.0 as usize) {
            params.set(index, *default);
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe::model::effects::Effects;
    use ugoe::model::fixtures;
    use ugoe::motion::Rotation;

    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn motion(name: &str, id: &str, value: f32, duration: f32, looping: bool) -> Motion {
        ugoe::format::motion_from_json(&format!(
            r#"{{
              "name": "{name}", "duration": {duration}, "looping": {looping}, "fade_in": 0.0, "fade_out": 0.0,
              "curves": [{{
                "target": {{ "kind": "parameter", "id": "{id}" }},
                "segments": [
                  {{ "type": "start", "p": {{ "t": 0.0, "v": {value} }} }},
                  {{ "type": "linear", "to": {{ "t": {duration}, "v": {value} }} }}
                ]
              }}]
            }}"#
        ))
        .unwrap()
    }

    fn expression(name: &str) -> Expression {
        ugoe::format::expression_from_json(&format!(
            r#"{{ "name": "{name}", "fade_in": 0.0, "fade_out": 0.0,
                 "parameters": [{{ "parameter": "MouthForm", "value": 1.0, "op": "add" }}] }}"#
        ))
        .unwrap()
    }

    fn setup(motions: Vec<Motion>) -> (Animation, Puppet) {
        let mut puppet = Puppet::new(fixtures::avatar_profile()).unwrap();
        puppet.set_effects(Effects::default());
        let mut animation = Animation::new();
        animation.load(
            &mut puppet,
            motions,
            vec![expression("Smile"), expression("Angry")],
        );
        (animation, puppet)
    }

    fn step(animation: &mut Animation, puppet: &mut Puppet, dt: f32) {
        animation.before_update(puppet);
        puppet.update(dt);
        animation.after_update(puppet);
    }

    fn param(puppet: &Puppet, id: &str) -> f32 {
        let p = puppet.instance().parameters();
        p.get(p.index_of(id).unwrap()).unwrap()
    }

    #[test]
    fn the_idle_motion_plays_when_nothing_else_does() {
        let (mut animation, mut puppet) = setup(vec![
            motion("Idle_02", "AngleZ", 3.0, 1.0, false),
            motion("Idle_01", "AngleZ", 5.0, 1.0, true),
            motion("Wave", "AngleX", 20.0, 0.5, false),
        ]);
        // `idle` が無いので、`idle` で始まるループするもの。
        assert_eq!(animation.idle_motion(), Some("Idle_01"));
        let view = animation.view(Some(&puppet));
        assert_eq!(
            view.current,
            Some(CurrentMotion {
                name: "Idle_01".into(),
                priority: MotionPriority::Idle
            })
        );
        assert!(animation.take_changed());
        // 普通のモーションを流し、終わったら待機に戻る。
        assert!(
            animation
                .play(&mut puppet, "Wave", Priority::Normal)
                .unwrap()
        );
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 20.0).abs() < 1e-4);
        // 待機は低い優先度なので割り込めない。
        assert!(
            !animation
                .play(&mut puppet, "Idle_02", Priority::Idle)
                .unwrap()
        );
        for _ in 0..40 {
            step(&mut animation, &mut puppet, DT);
        }
        assert_eq!(
            animation.view(None).current.map(|c| c.name),
            Some("Idle_01".into())
        );
        assert_eq!(
            animation.play(&mut puppet, "Nope", Priority::Force),
            Err(AnimationError::UnknownMotion {
                name: "Nope".into()
            })
        );
    }

    #[test]
    fn settings_switch_the_idle_motion() {
        let (mut animation, mut puppet) = setup(vec![
            motion("idle", "AngleZ", 3.0, 1.0, true),
            motion("Calm", "AngleZ", 7.0, 1.0, true),
        ]);
        assert_eq!(animation.idle_motion(), Some("idle"));
        animation.set_settings(
            Some(&mut puppet),
            MotionSettings {
                idle_motion: Some("Calm".into()),
                ..MotionSettings::default()
            },
        );
        assert_eq!(
            animation.view(None).current.map(|c| c.name),
            Some("Calm".into())
        );
        // モデルに無い名前なら名前で選ぶ。
        animation.set_settings(
            Some(&mut puppet),
            MotionSettings {
                idle_motion: Some("Missing".into()),
                ..MotionSettings::default()
            },
        );
        assert_eq!(animation.idle_motion(), Some("idle"));
        // 切ると流している待機を止める。
        animation.set_settings(
            Some(&mut puppet),
            MotionSettings {
                auto_idle: false,
                ..MotionSettings::default()
            },
        );
        assert_eq!(animation.view(None).current, None);
        // 待機ではないモーションを止めても、待機は切ってあるので何も流さない。
        animation
            .play(&mut puppet, "Calm", Priority::Normal)
            .unwrap();
        animation.stop(&mut puppet);
        assert_eq!(animation.view(None).current, None);
    }

    fn tracking(yaw: f32) -> TrackingInput {
        TrackingInput {
            head: Rotation {
                yaw,
                ..Rotation::default()
            },
            ..TrackingInput::default()
        }
    }

    #[test]
    fn motions_win_over_tracking_only_when_they_are_not_idle() {
        let (mut animation, mut puppet) = setup(vec![
            motion("idle", "AngleX", 5.0, 1.0, true),
            motion("Nod", "AngleX", 25.0, 1.0, false),
        ]);
        animation.set_tracking(Some(&mut puppet), Some(tracking(10.0)));
        step(&mut animation, &mut puppet, DT);
        // 待機の間はトラッキングが勝つ。
        assert!((param(&puppet, "AngleX") - 10.0).abs() < 1e-4);
        animation
            .play(&mut puppet, "Nod", Priority::Normal)
            .unwrap();
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 25.0).abs() < 1e-4);
        // 優先を切ると、トラッキングが勝つ (仕様の適用順)。
        animation.set_settings(
            Some(&mut puppet),
            MotionSettings {
                over_tracking: false,
                ..MotionSettings::default()
            },
        );
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 10.0).abs() < 1e-4);
        // 優先しても、トラッキングのままにするパラメータ。
        animation.set_settings(
            Some(&mut puppet),
            MotionSettings {
                tracking_kept: vec!["AngleX".into()],
                ..MotionSettings::default()
            },
        );
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 10.0).abs() < 1e-4);
    }

    #[test]
    fn removing_or_pausing_tracking_resets_the_tracked_parameters() {
        let (mut animation, mut puppet) = setup(Vec::new());
        animation.set_tracking(Some(&mut puppet), Some(tracking(15.0)));
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 15.0).abs() < 1e-4);
        assert!(animation.view(Some(&puppet)).tracking);
        assert!(
            animation
                .view(Some(&puppet))
                .tracking_parameters
                .iter()
                .any(|id| id == "AngleX")
        );
        animation.set_tracking(Some(&mut puppet), None);
        step(&mut animation, &mut puppet, DT);
        assert_eq!(param(&puppet, "AngleX"), 0.0);

        animation.set_tracking(Some(&mut puppet), Some(tracking(15.0)));
        animation.set_tracking_paused(Some(&mut puppet), true);
        // 一時停止中の入力は捨てる。
        animation.set_tracking(Some(&mut puppet), Some(tracking(30.0)));
        step(&mut animation, &mut puppet, DT);
        assert_eq!(param(&puppet, "AngleX"), 0.0);
        assert!(!animation.view(None).tracking);
        animation.set_tracking_paused(Some(&mut puppet), false);
        animation.set_tracking(Some(&mut puppet), Some(tracking(30.0)));
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "AngleX") - 30.0).abs() < 1e-4);
    }

    #[test]
    fn expressions_follow_set_add_toggle_and_clear() {
        let (mut animation, mut puppet) = setup(Vec::new());
        animation.set_expression(&mut puppet, "Smile").unwrap();
        animation.add_expression(&mut puppet, "Angry").unwrap();
        animation.add_expression(&mut puppet, "Angry").unwrap();
        assert_eq!(animation.view(None).active_expressions, ["Smile", "Angry"]);
        animation.toggle_expression(&mut puppet, "Smile").unwrap();
        assert_eq!(animation.view(None).active_expressions, ["Angry"]);
        animation.set_expression(&mut puppet, "Smile").unwrap();
        assert_eq!(animation.view(None).active_expressions, ["Smile"]);
        step(&mut animation, &mut puppet, DT);
        assert!((param(&puppet, "MouthForm") - 1.0).abs() < 1e-4);
        animation.clear_expressions(&mut puppet);
        assert!(animation.view(None).active_expressions.is_empty());
        assert!(matches!(
            animation.set_expression(&mut puppet, "Nope"),
            Err(AnimationError::UnknownExpression { .. })
        ));
    }

    #[test]
    fn lip_sync_can_be_paused_and_follows_the_model() {
        let (mut animation, mut puppet) = setup(Vec::new());
        animation.set_lip_sync_level(Some(&mut puppet), Some(0.75));
        assert_eq!(puppet.lip_sync_applied(), None);
        let mut model_puppet = Puppet::new(fixtures::avatar_profile()).unwrap();
        model_puppet.set_effects(Effects {
            lip_sync: ugoe::model::effects::LipSync::standard(),
            ..Effects::default()
        });
        // 新しいモデルにも音量を渡し直す。
        animation.load(&mut model_puppet, Vec::new(), Vec::new());
        model_puppet.update(DT);
        assert_eq!(model_puppet.lip_sync_applied().map(|l| l.level), Some(0.75));
        animation.set_lip_sync_paused(Some(&mut model_puppet), true);
        model_puppet.update(DT);
        assert_eq!(model_puppet.lip_sync_applied(), None);
        animation.set_lip_sync_paused(Some(&mut model_puppet), false);
        model_puppet.update(DT);
        assert_eq!(model_puppet.lip_sync_applied().map(|l| l.level), Some(0.75));
    }

    #[test]
    fn eye_blink_follows_the_setting_across_models() {
        let (mut animation, mut puppet) = setup(Vec::new());
        animation.set_auto_eye_blink(Some(&mut puppet), false);
        assert!(!puppet.toggles().eye_blink);
        let mut next = Puppet::new(fixtures::avatar_profile()).unwrap();
        animation.load(&mut next, Vec::new(), Vec::new());
        assert!(!next.toggles().eye_blink);
        // ほかの段は変えない。
        assert!(next.toggles().physics);
    }
}
