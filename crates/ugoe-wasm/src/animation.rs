//! 物理・モーション・表情・エフェクトの操作 (JSON の受け渡しと名前の解決)。GPU には触れない。
//!
//! `WebPuppet` (`src/web/animation.rs`) はこれを JS へ出すだけにし、ここをネイティブでテストする。
//! フレームの進め方は [`Puppet::update`] そのもの (`docs/spec/motion.md#フレーム内の適用順`)。

use ugoe::Puppet;
use ugoe::Toggles;
use ugoe::model::expression::{Expression, ExpressionOp};
use ugoe::model::motion::{CurveTarget, Motion};
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::ValidationError;
use ugoe::motion::Priority;

use crate::load::LoadWarning;

#[derive(Debug, thiserror::Error)]
pub enum AnimationError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{}", validation_message("physics settings", .0))]
    InvalidPhysics(Vec<ValidationError>),
    #[error("unknown motion `{0}`")]
    UnknownMotion(String),
    #[error("unknown expression `{0}`")]
    UnknownExpression(String),
    #[error("unknown priority `{0}` (expected idle, normal or force)")]
    UnknownPriority(String),
    #[error("unknown stage `{0}` (expected physics, eye_blink, breath, lip_sync or pose)")]
    UnknownStage(String),
}

fn validation_message(what: &str, errors: &[ValidationError]) -> String {
    let list: Vec<String> = errors.iter().map(ToString::to_string).collect();
    format!("{what} failed validation: {}", list.join("; "))
}

/// バンドルに入っていたモーションと表情 (名前で再生するため)。
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

impl Library {
    pub fn motion_names(&self) -> Vec<String> {
        self.motions.iter().map(|m| m.name.clone()).collect()
    }

    pub fn expression_names(&self) -> Vec<String> {
        self.expressions.iter().map(|e| e.name.clone()).collect()
    }

    /// モーションの一覧 (チャンクの順) の JSON。
    /// `[{"name", "duration", "looping", "fade_in", "fade_out", "parameters": [ID, …], "events": [値, …]}]`。
    /// `parameters` はカーブが動かすパラメータ (重なりなし、カーブの順)。ビューアのモーションのパネル用 (T0720)。
    pub fn motions_json(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .motions
            .iter()
            .map(|m| {
                let mut parameters: Vec<&str> = Vec::new();
                for c in &m.curves {
                    if let CurveTarget::Parameter(id) = &c.target
                        && !parameters.contains(&id.as_str())
                    {
                        parameters.push(id.as_str());
                    }
                }
                let events: Vec<&str> = m.events.iter().map(|e| e.value.as_str()).collect();
                serde_json::json!({
                    "name": m.name,
                    "duration": m.duration,
                    "looping": m.looping,
                    "fade_in": m.fade_in,
                    "fade_out": m.fade_out,
                    "parameters": parameters,
                    "events": events,
                })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// 表情の一覧 (チャンクの順) の JSON。`*.expression.json` の形
    /// (`[{"name", "fade_in", "fade_out", "parameters": [{"parameter", "value", "op"}]}]`)。
    /// 未知の `op` の項目 (新しい版のファイル。再生でも無視する) は出さない。
    pub fn expressions_json(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .expressions
            .iter()
            .map(|e| {
                let parameters: Vec<serde_json::Value> = e
                    .parameters
                    .iter()
                    .filter(|p| p.op != ExpressionOp::Unknown)
                    .map(|p| serde_json::json!({ "parameter": p.parameter, "value": p.value, "op": p.op }))
                    .collect();
                serde_json::json!({
                    "name": e.name,
                    "fade_in": e.fade_in,
                    "fade_out": e.fade_out,
                    "parameters": parameters,
                })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// 名前が `name` の最初のモーション。
    pub fn motion(&self, name: &str) -> Result<&Motion, AnimationError> {
        self.motions
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| AnimationError::UnknownMotion(name.to_owned()))
    }

    /// 名前が `name` の最初の表情。
    pub fn expression(&self, name: &str) -> Result<&Expression, AnimationError> {
        self.expressions
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| AnimationError::UnknownExpression(name.to_owned()))
    }
}

/// 優先度の名前 (`"idle"`・`"normal"`・`"force"`)。省略すると `normal`。
pub fn parse_priority(name: Option<&str>) -> Result<Priority, AnimationError> {
    match name.unwrap_or("normal") {
        "idle" => Ok(Priority::Idle),
        "normal" => Ok(Priority::Normal),
        "force" => Ok(Priority::Force),
        other => Err(AnimationError::UnknownPriority(other.to_owned())),
    }
}

/// 段の名前 (`"physics"`・`"eye_blink"`・`"breath"`・`"lip_sync"`・`"pose"`) に対応する [`Toggles`] のフラグ。
pub fn stage_flag<'a>(
    toggles: &'a mut Toggles,
    name: &str,
) -> Result<&'a mut bool, AnimationError> {
    match name {
        "physics" => Ok(&mut toggles.physics),
        "eye_blink" => Ok(&mut toggles.eye_blink),
        "breath" => Ok(&mut toggles.breath),
        "lip_sync" => Ok(&mut toggles.lip_sync),
        "pose" => Ok(&mut toggles.pose),
        other => Err(AnimationError::UnknownStage(other.to_owned())),
    }
}

/// 段 `name` を動かすかを設定する。
pub fn set_stage_enabled(
    puppet: &mut Puppet,
    name: &str,
    enabled: bool,
) -> Result<(), AnimationError> {
    let mut toggles = puppet.toggles();
    *stage_flag(&mut toggles, name)? = enabled;
    puppet.set_toggles(toggles);
    Ok(())
}

/// 段 `name` を動かしているか。
pub fn stage_enabled(puppet: &Puppet, name: &str) -> Result<bool, AnimationError> {
    let mut toggles = puppet.toggles();
    Ok(*stage_flag(&mut toggles, name)?)
}

/// JSON (`*.motion.json` の形) のモーションを `priority` で開始する。開始できなければ `false`。
pub fn start_motion_json(
    puppet: &mut Puppet,
    json: &str,
    priority: Option<&str>,
) -> Result<bool, AnimationError> {
    let priority = parse_priority(priority)?;
    let motion = ugoe::format::motion_from_json(json)?;
    Ok(puppet.start_motion(motion, priority))
}

/// モーション (JSON、`*.motion.json` の形) の各カーブを、再生位置 `t` (秒) で評価した値。
///
/// フェード・ループの折り返し・ほかの段は掛けない、カーブの値そのもの (`ugoe_motion::curve::evaluate`)。
/// エディタのタイムラインがシーク・再生の表示に使う (`docs/editor.md#タイムライン`)。
/// 戻り値は `[{"target": {"kind": "parameter", "id": "AngleX"}, "value": 7.5}, …]` の JSON で、
/// 値のないカーブ (未知の対象・セグメント) は含めない。
pub fn sample_motion_json(json: &str, t: f32) -> Result<String, AnimationError> {
    let motion = ugoe::format::motion_from_json(json)?;
    let values: Vec<serde_json::Value> = motion
        .curves
        .iter()
        .filter(|c| c.target != ugoe::model::motion::CurveTarget::Unknown)
        .filter_map(|c| {
            let value = ugoe::motion::curve::evaluate(&c.segments, t)?;
            Some(serde_json::json!({ "target": c.target, "value": value }))
        })
        .collect();
    Ok(serde_json::to_string(&values)?)
}

/// JSON (`*.expression.json` の形) の表情だけにする。
pub fn set_expression_json(puppet: &mut Puppet, json: &str) -> Result<(), AnimationError> {
    puppet.set_expression(ugoe::format::expression_from_json(json)?);
    Ok(())
}

/// 目パチ・呼吸・リップシンクの設定 (モデルの `effects` と同じ JSON) を差し替える。
pub fn set_effects_json(puppet: &mut Puppet, json: &str) -> Result<(), AnimationError> {
    puppet.set_effects(serde_json::from_str(json)?);
    Ok(())
}

/// ポーズで、`part` を `part` に持つ要素をそのグループの表示中の要素にする (`links` のパーツは対象外)。
/// どの要素の `part` でもなければ何も変えず `false` (`ugoe::motion::PosePlayer::show`、`docs/spec/motion.md#ポーズ`)。
pub fn show_pose(puppet: &mut Puppet, part: &str) -> bool {
    puppet.pose_mut().show(&part.into())
}

/// 物理の設定 (`*.physics.json` の形) を差し替える。`None` で外す。
/// モデルと合わせて検査し、エラーがあれば何も変えない。
pub fn set_physics_json(puppet: &mut Puppet, json: Option<&str>) -> Result<(), AnimationError> {
    let settings = json.map(ugoe::format::physics_from_json).transpose()?;
    set_physics(puppet, settings)
}

fn set_physics(
    puppet: &mut Puppet,
    settings: Option<PhysicsSettings>,
) -> Result<(), AnimationError> {
    puppet
        .set_physics(settings)
        .map_err(AnimationError::InvalidPhysics)
}

/// 今の物理の設定の JSON。物理が無ければ `None`。
pub fn physics_json(puppet: &Puppet) -> Result<Option<String>, AnimationError> {
    puppet
        .physics()
        .map(|p| ugoe::format::physics_to_json(p.settings()))
        .transpose()
        .map_err(AnimationError::from)
}

/// 直前の `update` で発火したモーションのイベントの値。
pub fn motion_events(puppet: &Puppet) -> Vec<String> {
    puppet
        .motion_events()
        .iter()
        .map(|e| e.value.clone())
        .collect()
}

/// モデルを読み直したとき (`load_json`) に、前の `Puppet` の設定を新しい `Puppet` へ写す。
///
/// 段の有効・無効は常に引き継ぐ。`keep_physics` なら物理の設定も引き継ぐが、新しいモデルと
/// 合わなくなった設定は外して警告を返す (エディタでモデルを編集しながら物理をプレビューするため)。
/// 着せ替えのモジュールを付けていれば、合成した設定ではなく本体の設定を引き継ぐ (モジュールは付け直す側で加わる)。
pub fn carry_over(old: &Puppet, new: &mut Puppet, keep_physics: bool) -> Vec<LoadWarning> {
    new.set_toggles(old.toggles());
    let mut warnings = Vec::new();
    if keep_physics
        && let Some(physics) = old.base_physics()
        && let Err(e) = set_physics(new, Some(physics.clone()))
    {
        warnings.push(LoadWarning::PhysicsRemoved {
            reason: e.to_string(),
        });
    }
    warnings
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;
    use ugoe::model::standard;

    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn param(puppet: &Puppet, id: &str) -> f32 {
        let p = puppet.instance().parameters();
        p.get(p.index_of(id).unwrap())
    }

    fn set_param(puppet: &mut Puppet, id: &str, value: f32) {
        let p = puppet.instance_mut().parameters_mut();
        let i = p.index_of(id).unwrap();
        p.set(i, value);
    }

    fn quiet(model: ugoe::model::Model) -> Puppet {
        let mut puppet = Puppet::new(model).unwrap();
        puppet.set_effects(Default::default());
        puppet
    }

    const MOTION: &str = r#"{
        "name": "nod",
        "duration": 1.0,
        "looping": false,
        "fade_in": 0.0,
        "fade_out": 0.0,
        "curves": [{
            "target": { "kind": "parameter", "id": "AngleX" },
            "segments": [
                { "type": "start", "p": { "t": 0.0, "v": 0.0 } },
                { "type": "linear", "to": { "t": 1.0, "v": 30.0 } }
            ]
        }],
        "events": [{ "t": 0.5, "value": "half" }]
    }"#;

    #[test]
    fn motion_json_plays_and_reports_events() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        assert!(start_motion_json(&mut puppet, MOTION, None).unwrap());
        puppet.update(0.25);
        assert!((param(&puppet, standard::ANGLE_X) - 7.5).abs() < 1e-3);
        assert!(motion_events(&puppet).is_empty());
        puppet.update(0.5);
        assert_eq!(motion_events(&puppet), vec!["half"]);
        // 優先度の低いモーションは再生中なら始まらない。
        assert!(!start_motion_json(&mut puppet, MOTION, Some("idle")).unwrap());
        assert!(matches!(
            start_motion_json(&mut puppet, MOTION, Some("urgent")),
            Err(AnimationError::UnknownPriority(_))
        ));
        assert!(start_motion_json(&mut puppet, "{", None).is_err());
        puppet.motions_mut().stop_all();
        assert!(!puppet.motions().is_playing());
    }

    #[test]
    fn motion_json_is_sampled_without_fades() {
        let json = MOTION.replace(r#""fade_in": 0.0"#, r#""fade_in": 5.0"#);
        let sample = |t| -> serde_json::Value {
            serde_json::from_str(&sample_motion_json(&json, t).unwrap()).unwrap()
        };
        let at = sample(0.25);
        assert_eq!(at[0]["target"]["kind"], "parameter");
        assert_eq!(at[0]["target"]["id"], "AngleX");
        assert!((at[0]["value"].as_f64().unwrap() - 7.5).abs() < 1e-4);
        // 範囲の外は端の値。
        assert!((sample(3.0)[0]["value"].as_f64().unwrap() - 30.0).abs() < 1e-4);
        assert!(sample_motion_json("{", 0.0).is_err());
    }

    #[test]
    fn expression_json_applies_over_base_values() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        let json = r#"{
            "name": "wide",
            "fade_in": 0.0,
            "fade_out": 0.0,
            "parameters": [{ "parameter": "AngleX", "value": 5.0, "op": "add" }]
        }"#;
        set_expression_json(&mut puppet, json).unwrap();
        set_param(&mut puppet, standard::ANGLE_X, 10.0);
        for _ in 0..3 {
            puppet.update(DT);
            assert!((param(&puppet, standard::ANGLE_X) - 15.0).abs() < 1e-4);
        }
    }

    #[test]
    fn library_lists_motions_and_expressions() {
        let looping = MOTION
            .replace(r#""name": "nod""#, r#""name": "idle""#)
            .replace(r#""looping": false"#, r#""looping": true"#);
        let expression = r#"{
            "name": "smile",
            "fade_in": 0.3,
            "fade_out": 0.2,
            "parameters": [
                { "parameter": "AngleX", "value": 5.0, "op": "add" },
                { "parameter": "AngleY", "value": 1.0, "op": "future_op" }
            ]
        }"#;
        let library = Library {
            motions: vec![
                ugoe::format::motion_from_json(MOTION).unwrap(),
                ugoe::format::motion_from_json(&looping).unwrap(),
            ],
            expressions: vec![ugoe::format::expression_from_json(expression).unwrap()],
        };
        let motions: serde_json::Value = serde_json::from_str(&library.motions_json()).unwrap();
        assert_eq!(motions[0]["name"], "nod");
        assert_eq!(motions[0]["duration"], 1.0);
        assert_eq!(motions[0]["looping"], false);
        assert_eq!(motions[0]["parameters"], serde_json::json!(["AngleX"]));
        assert_eq!(motions[0]["events"], serde_json::json!(["half"]));
        assert_eq!(motions[1]["looping"], true);
        let expressions: serde_json::Value =
            serde_json::from_str(&library.expressions_json()).unwrap();
        assert_eq!(expressions[0]["name"], "smile");
        assert!((expressions[0]["fade_in"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        // 未知の op の項目は出さない。
        assert_eq!(
            expressions[0]["parameters"],
            serde_json::json!([{ "parameter": "AngleX", "value": 5.0, "op": "add" }])
        );
        assert_eq!(Library::default().motions_json(), "[]");
    }

    #[test]
    fn expressions_are_added_over_others_and_stopped_by_name() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        let json = |name: &str, value: f32| {
            format!(
                r#"{{"name": "{name}", "parameters": [{{ "parameter": "AngleX", "value": {value}, "op": "add" }}]}}"#
            )
        };
        let library = Library {
            motions: vec![],
            expressions: vec![
                ugoe::format::expression_from_json(&json("a", 5.0)).unwrap(),
                ugoe::format::expression_from_json(&json("b", 2.0)).unwrap(),
            ],
        };
        puppet.set_expression(library.expression("a").unwrap().clone());
        puppet
            .expressions_mut()
            .add(library.expression("b").unwrap().clone());
        puppet.update(DT);
        assert!((param(&puppet, standard::ANGLE_X) - 7.0).abs() < 1e-4);
        puppet.expressions_mut().stop("a");
        puppet.update(DT);
        assert!((param(&puppet, standard::ANGLE_X) - 2.0).abs() < 1e-4);
        // set はほかの表情を止める。
        puppet.set_expression(library.expression("a").unwrap().clone());
        puppet.update(DT);
        assert!((param(&puppet, standard::ANGLE_X) - 5.0).abs() < 1e-4);
    }

    #[test]
    fn library_resolves_names() {
        let library = Library {
            motions: vec![ugoe::format::motion_from_json(MOTION).unwrap()],
            expressions: vec![],
        };
        assert_eq!(library.motion_names(), vec!["nod"]);
        assert!(library.motion("nod").is_ok());
        assert!(matches!(
            library.motion("wave"),
            Err(AnimationError::UnknownMotion(_))
        ));
        assert!(matches!(
            library.expression("smile"),
            Err(AnimationError::UnknownExpression(_))
        ));
    }

    #[test]
    fn physics_json_is_validated_and_swappable() {
        let mut puppet = quiet(fixtures::physics_hair());
        assert_eq!(physics_json(&puppet).unwrap(), None);

        let settings = fixtures::physics_hair_settings();
        let json = ugoe::format::physics_to_json(&settings).unwrap();
        set_physics_json(&mut puppet, Some(&json)).unwrap();
        let back = physics_json(&puppet).unwrap().unwrap();
        assert_eq!(ugoe::format::physics_from_json(&back).unwrap(), settings);

        let mut bad = settings.clone();
        bad.rigs[0].inputs[0].parameter = "Missing".into();
        let bad = ugoe::format::physics_to_json(&bad).unwrap();
        let err = set_physics_json(&mut puppet, Some(&bad)).unwrap_err();
        assert!(err.to_string().contains("Missing"), "{err}");
        // 失敗しても前の設定のまま。
        assert!(puppet.physics().is_some());

        set_physics_json(&mut puppet, None).unwrap();
        assert!(puppet.physics().is_none());
    }

    /// 物理の出力は基準値に重ねるので、スライダーで出力のパラメータを動かしても物理が上書きする。
    /// 物理を止めればスライダーの値が見える。
    #[test]
    fn physics_output_overrides_slider_until_disabled() {
        let settings = fixtures::physics_hair_settings();
        let output = settings.rigs[0].outputs[0]
            .parameter()
            .unwrap()
            .as_str()
            .to_owned();
        let mut puppet = quiet(fixtures::physics_hair()).with_physics(settings);
        set_param(&mut puppet, standard::ANGLE_X, 30.0);
        for _ in 0..10 {
            puppet.update(DT);
        }
        set_param(&mut puppet, &output, 0.25);
        puppet.update(DT);
        assert_ne!(param(&puppet, &output), 0.25);

        set_stage_enabled(&mut puppet, "physics", false).unwrap();
        assert!(!stage_enabled(&puppet, "physics").unwrap());
        set_param(&mut puppet, &output, 0.25);
        puppet.update(DT);
        assert_eq!(param(&puppet, &output), 0.25);
        assert!(matches!(
            set_stage_enabled(&mut puppet, "gravity", true),
            Err(AnimationError::UnknownStage(_))
        ));
    }

    #[test]
    fn effects_json_replaces_effects() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        let json = r#"{ "lip_sync": { "parameters": ["MouthOpenY"] } }"#;
        set_effects_json(&mut puppet, json).unwrap();
        puppet.set_lip_sync_level(Some(0.5));
        puppet.update(DT);
        assert!((param(&puppet, standard::MOUTH_OPEN_Y) - 0.5).abs() < 1e-4);
        set_stage_enabled(&mut puppet, "lip_sync", false).unwrap();
        puppet.update(DT);
        assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
    }

    /// 表示を選んだ要素へ次の `update` からフェードする。モーションの要求が無ければ選択が残る。
    #[test]
    fn show_pose_switches_the_shown_entry() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        let arm_b = puppet.instance().compiled().part_index("ArmB").unwrap();
        assert!(!show_pose(&mut puppet, "SleeveA"));
        assert!(!show_pose(&mut puppet, "Missing"));
        puppet.update(DT);
        assert_eq!(puppet.instance().part_opacity_override(arm_b), (0.0, 1.0));

        assert!(show_pose(&mut puppet, "ArmB"));
        for _ in 0..15 {
            puppet.update(DT);
        }
        let (value, weight) = puppet.instance().part_opacity_override(arm_b);
        assert!((value - 0.5).abs() < 1e-4, "{value}");
        assert_eq!(weight, 1.0);
        for _ in 0..30 {
            puppet.update(DT);
        }
        assert_eq!(puppet.instance().part_opacity_override(arm_b), (1.0, 1.0));
    }

    #[test]
    fn carry_over_keeps_physics_and_toggles() {
        let mut old =
            quiet(fixtures::physics_hair()).with_physics(fixtures::physics_hair_settings());
        set_stage_enabled(&mut old, "breath", false).unwrap();

        let mut new = Puppet::new(fixtures::physics_hair()).unwrap();
        assert!(carry_over(&old, &mut new, true).is_empty());
        assert!(new.physics().is_some());
        assert!(!new.toggles().breath);

        // 合わなくなった設定は外して警告する。
        let mut new = Puppet::new(fixtures::single_quad()).unwrap();
        let warnings = carry_over(&old, &mut new, true);
        assert!(
            matches!(&warnings[..], [LoadWarning::PhysicsRemoved { reason }] if reason.contains("failed validation")),
            "{warnings:?}"
        );
        assert_eq!(
            crate::load::warning_json(&warnings[0])["code"],
            "physics_removed"
        );
        assert!(new.physics().is_none());

        let mut new = Puppet::new(fixtures::physics_hair()).unwrap();
        assert!(carry_over(&old, &mut new, false).is_empty());
        assert!(new.physics().is_none());
    }
}
