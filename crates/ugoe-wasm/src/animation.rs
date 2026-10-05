//! 物理・モーション・表情・エフェクトの操作 (JSON の受け渡しと名前の解決)。GPU には触れない。
//!
//! `WebPuppet` (`src/web/animation.rs`) はこれを JS へ出すだけにし、ここをネイティブでテストする。
//! フレームの進め方は [`Puppet::update`] そのもの (`docs/spec/motion.md#フレーム内の適用順`)。

use ugoe::Puppet;
use ugoe::Toggles;
use ugoe::model::expression::{Expression, ExpressionOp};
use ugoe::model::motion::{CurveTarget, Motion};
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::{ValidationError, describe_errors};
use ugoe::motion::Priority;
use ugoe::motion::curve::SegmentIndex;

use crate::load::{ErrorInfo, LoadWarning};

#[derive(Debug, thiserror::Error)]
pub enum AnimationError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{}", describe_errors("physics settings", .0))]
    InvalidPhysics(Vec<ValidationError>),
    /// エフェクトの設定が検査を通らない (`set_effects_json`)。
    #[error("{}", describe_errors("effects", .0))]
    InvalidEffects(Vec<ValidationError>),
    /// 本体の物理の設定を、付けているモジュールと合成できなかった (`set_base_physics_json`)。
    #[error(transparent)]
    DressUp(ugoe::DressUpError),
    #[error("unknown motion `{0}`")]
    UnknownMotion(String),
    #[error("unknown expression `{0}`")]
    UnknownExpression(String),
    #[error("unknown priority `{0}` (expected idle, normal or force)")]
    UnknownPriority(String),
    #[error("unknown stage `{0}` (expected physics, eye_blink, breath, lip_sync or pose)")]
    UnknownStage(String),
}

impl AnimationError {
    /// JS の例外に付ける種類と補足の値。エフェクトの設定の検査の失敗だけが持ち、読み込みの `invalid_model` と同じ
    /// (`code` は `"invalid_model"`、`errors` は検査のエラーの数。C ABI の `ugoe_last_error_kind` と同じ)。
    pub fn info(&self) -> Option<ErrorInfo> {
        match self {
            Self::InvalidEffects(errors) => Some(ErrorInfo {
                code: "invalid_model",
                fields: vec![("errors", errors.len().into())],
            }),
            _ => None,
        }
    }
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
///
/// 毎回 JSON を読んで索引を作る。同じモーションを何度も評価するなら [`MotionSampler`] を使う。
pub fn sample_motion_json(json: &str, t: f32) -> Result<String, AnimationError> {
    MotionSampler::from_json(json)?.sample_json(t)
}

/// モーション 1 つのカーブの値 ([`sample_motion_json`] と同じもの) を、何度も評価するための状態。
///
/// カーブごとにセグメントの索引 (`ugoe_motion::curve::SegmentIndex`) を持ち、区間を二分探索で引く
/// (モーションの再生と同じ。T0324)。モーションを差し替えると索引を作り直す。
#[derive(Debug, Clone)]
pub struct MotionSampler {
    /// 未知の対象のカーブを除いたモーション。
    motion: Motion,
    /// `motion.curves` と同じ順の索引。
    indices: Vec<SegmentIndex>,
}

impl MotionSampler {
    pub fn new(mut motion: Motion) -> Self {
        motion.curves.retain(|c| c.target != CurveTarget::Unknown);
        let indices = motion
            .curves
            .iter()
            .map(|c| SegmentIndex::new(&c.segments))
            .collect();
        Self { motion, indices }
    }

    /// JSON (`*.motion.json` の形) のモーションから作る。
    pub fn from_json(json: &str) -> Result<Self, AnimationError> {
        Ok(Self::new(ugoe::format::motion_from_json(json)?))
    }

    /// モーションを JSON のものに差し替え、索引を作り直す。読めなければエラーで、今のモーションを保つ。
    pub fn set_motion_json(&mut self, json: &str) -> Result<(), AnimationError> {
        *self = Self::from_json(json)?;
        Ok(())
    }

    /// 各カーブを再生位置 `t` (秒) で評価した値の JSON。形は [`sample_motion_json`] と同じ。
    pub fn sample_json(&self, t: f32) -> Result<String, AnimationError> {
        let values: Vec<serde_json::Value> = self
            .motion
            .curves
            .iter()
            .zip(&self.indices)
            .filter_map(|(c, index)| {
                let value = index.evaluate(&c.segments, t)?;
                Some(serde_json::json!({ "target": c.target, "value": value }))
            })
            .collect();
        Ok(serde_json::to_string(&values)?)
    }
}

/// JSON (`*.expression.json` の形) の表情だけにする。表情の名前を返す。
pub fn set_expression_json(puppet: &mut Puppet, json: &str) -> Result<String, AnimationError> {
    let expression = ugoe::format::expression_from_json(json)?;
    let name = expression.name.clone();
    puppet.set_expression(expression);
    Ok(name)
}

/// 使っている表情の名前 (開始した順、重なりなし。止めてフェードアウト中のものは含めない。T0721)。
///
/// `ExpressionManager::players` はフェードアウト中のものも含み、止めたかを返さないので、`WebPuppet` の表情の操作
/// (set・add・stop・stop_all) を写して持つ。規則は `ExpressionManager` と同じ (set はほかをすべて止める、stop は
/// 名前の一致するものをすべて止める)。同じ表情を重ねても名前は 1 つ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveExpressions(Vec<String>);

impl ActiveExpressions {
    /// `set`: `name` だけにする。
    pub fn set(&mut self, name: &str) {
        self.0.clear();
        self.0.push(name.to_owned());
    }

    /// `add`: 重ねる (使っていれば何もしない)。
    pub fn add(&mut self, name: &str) {
        if !self.0.iter().any(|n| n == name) {
            self.0.push(name.to_owned());
        }
    }

    /// `stop(name)`: 名前の一致するものを外す。
    pub fn stop(&mut self, name: &str) {
        self.0.retain(|n| n != name);
    }

    /// `stop_all` とモデルの読み直し: 空にする。
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// 名前の一覧。`puppet` に再生中の表情が無い名前は除く (念のため。操作を写していれば起こらない)。
    pub fn names(&self, puppet: &mut Puppet) -> Vec<String> {
        let players = puppet.expressions_mut().players();
        self.0
            .iter()
            .filter(|n| {
                players
                    .iter()
                    .any(|p| &p.expression().name == *n && !p.is_finished())
            })
            .cloned()
            .collect()
    }
}

/// 目パチ・呼吸・リップシンクの設定 (モデルの `effects` と同じ JSON) を差し替える。
/// モデルの読み込みと同じに検査し (`Puppet::try_set_effects`)、通らなければ何も変えない。
pub fn set_effects_json(puppet: &mut Puppet, json: &str) -> Result<(), AnimationError> {
    puppet
        .try_set_effects(serde_json::from_str(json)?)
        .map_err(AnimationError::InvalidEffects)
}

/// ポーズで、`part` を `part` に持つ要素をそのグループの表示中の要素にする (`links` のパーツは対象外)。
/// どの要素の `part` でもなければ何も変えず `false` (`ugoe::motion::PosePlayer::show`、`docs/spec/motion.md#ポーズ`)。
pub fn show_pose(puppet: &mut Puppet, part: &str) -> bool {
    puppet.pose_mut().show(&part.into())
}

/// 今のポーズ (モデルの `pose` と同じ JSON)。着せ替えで付けたモジュールのグループ (`<モジュール>/<パーツ>`) と
/// 本体の要素への追従も入る (合成したモデルのポーズ)。
pub fn pose_json(puppet: &Puppet) -> Result<String, AnimationError> {
    Ok(serde_json::to_string(puppet.pose().pose())?)
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

/// 本体の物理の設定 (`*.physics.json` の形) を差し替える。`None` で外す (`ugoe::Puppet::set_base_physics`)。
/// 着せ替えのモジュールを付けていれば合成し直し、この後の付け外しでもこの設定から合成する。
/// 検査か合成に失敗すれば何も変えない。
pub fn set_base_physics_json(
    puppet: &mut Puppet,
    json: Option<&str>,
) -> Result<(), AnimationError> {
    let settings = json.map(ugoe::format::physics_from_json).transpose()?;
    puppet
        .set_base_physics(settings)
        .map_err(|error| match error {
            ugoe::DressUpError::Physics(errors) => AnimationError::InvalidPhysics(errors),
            error => AnimationError::DressUp(error),
        })
}

/// 本体の物理の設定の JSON (`ugoe::Puppet::base_physics`)。物理が無ければ `None`。
pub fn base_physics_json(puppet: &Puppet) -> Result<Option<String>, AnimationError> {
    puppet
        .base_physics()
        .map(ugoe::format::physics_to_json)
        .transpose()
        .map_err(AnimationError::from)
}

/// パーツ `part` の評価後の不透明度 (祖先を掛けた値。`ugoe::runtime::ModelInstance::part_opacity`)。
/// パーツが無ければ `None`。
pub fn part_evaluated_opacity(puppet: &Puppet, part: &str) -> Option<f32> {
    let instance = puppet.instance();
    let index = instance.compiled().part_index(part)?;
    Some(instance.part_opacity(index))
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
        // 検査のエラーの種類は、着せ替えの `invalid_physics` と同じく補足の値に載せる (T0168)。
        let codes = match &e {
            AnimationError::InvalidPhysics(errors) => {
                errors.iter().map(ValidationError::code).collect()
            }
            _ => Vec::new(),
        };
        warnings.push(LoadWarning::PhysicsRemoved {
            reason: e.to_string(),
            codes,
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
    fn active_expressions_follow_the_operations() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        let mut active = ActiveExpressions::default();
        let json = |name: &str| {
            format!(r#"{{"name": "{name}", "fade_in": 0.0, "fade_out": 0.5, "parameters": []}}"#)
        };
        let expression = |name: &str| ugoe::format::expression_from_json(&json(name)).unwrap();
        puppet.set_expression(expression("smile"));
        active.set("smile");
        puppet.expressions_mut().add(expression("wink"));
        active.add("wink");
        puppet.expressions_mut().add(expression("smile"));
        active.add("smile");
        puppet.update(DT);
        assert_eq!(active.names(&mut puppet), ["smile", "wink"]);
        // 止めた表情はフェードアウト中でも含めない。
        puppet.expressions_mut().stop("smile");
        active.stop("smile");
        puppet.update(DT);
        assert_eq!(puppet.expressions_mut().players().len(), 3);
        assert_eq!(active.names(&mut puppet), ["wink"]);
        // set はほかを止める。
        puppet.set_expression(expression("sad"));
        active.set("sad");
        assert_eq!(active.names(&mut puppet), ["sad"]);
        // 写した名前に再生中の表情が無ければ除く。
        active.add("angry");
        assert_eq!(active.names(&mut puppet), ["sad"]);
        puppet.expressions_mut().stop_all();
        active.clear();
        assert!(active.names(&mut puppet).is_empty());
    }

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
        // 有限でない時刻も端の値 (NaN と無限大は終わり、負の無限大は始め。T0433)。
        assert_eq!(sample(f32::NAN)[0]["value"], 30.0);
        assert_eq!(sample(f32::INFINITY)[0]["value"], 30.0);
        assert_eq!(sample(f32::NEG_INFINITY)[0]["value"], 0.0);
        assert!(sample_motion_json("{", 0.0).is_err());
    }

    /// 区間の多いカーブ・ベジェ・ステップ・2 つ目の `start`・未知のセグメント・未知の対象を含むモーション。
    const CURVES: &str = r#"{
        "name": "many",
        "duration": 4.0,
        "curves": [
            {
                "target": { "kind": "parameter", "id": "AngleX" },
                "segments": [
                    { "type": "start", "p": { "t": 0.0, "v": 0.0 } },
                    { "type": "linear", "to": { "t": 0.5, "v": 10.0 } },
                    { "type": "bezier", "c1": { "t": 0.7, "v": 30.0 }, "c2": { "t": 1.2, "v": -5.0 }, "to": { "t": 1.5, "v": 5.0 } },
                    { "type": "stepped", "to": { "t": 2.0, "v": -10.0 } },
                    { "type": "inverse_stepped", "to": { "t": 2.5, "v": 20.0 } },
                    { "type": "linear", "to": { "t": 4.0, "v": 0.0 } }
                ]
            },
            {
                "target": { "kind": "parameter", "id": "AngleY" },
                "segments": [
                    { "type": "start", "p": { "t": 1.0, "v": 3.0 } },
                    { "type": "linear", "to": { "t": 2.0, "v": 6.0 } },
                    { "type": "start", "p": { "t": 2.0, "v": 9.0 } }
                ]
            },
            {
                "target": { "kind": "parameter", "id": "AngleZ" },
                "segments": [
                    { "type": "start", "p": { "t": 0.0, "v": 1.0 } },
                    { "type": "wobble", "to": { "t": 1.0, "v": 2.0 } }
                ]
            },
            {
                "target": { "kind": "telepathy", "id": "X" },
                "segments": [
                    { "type": "start", "p": { "t": 0.0, "v": 1.0 } }
                ]
            }
        ]
    }"#;

    fn sampled(sampler: &MotionSampler, t: f32) -> Vec<(String, f32)> {
        let values: serde_json::Value =
            serde_json::from_str(&sampler.sample_json(t).unwrap()).unwrap();
        values
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let id = v["target"]["id"].as_str().unwrap().to_owned();
                (id, v["value"].as_f64().unwrap() as f32)
            })
            .collect()
    }

    /// 線形の `curve::evaluate` で評価した値 (索引を使わない、元の実装)。
    fn linear(json: &str, t: f32) -> Vec<(String, f32)> {
        let motion = ugoe::format::motion_from_json(json).unwrap();
        motion
            .curves
            .iter()
            .filter(|c| c.target != CurveTarget::Unknown)
            .filter_map(|c| {
                let value = ugoe::motion::curve::evaluate(&c.segments, t)?;
                let CurveTarget::Parameter(id) = &c.target else {
                    panic!("unexpected target {:?}", c.target);
                };
                Some((id.to_string(), value))
            })
            .collect()
    }

    #[test]
    fn motion_sampler_matches_linear_evaluation() {
        let sampler = MotionSampler::from_json(CURVES).unwrap();
        let mut times: Vec<f32> = (-10..=50).map(|i| i as f32 * 0.1).collect();
        times.extend([
            0.5,
            1.5,
            2.0,
            2.5,
            4.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ]);
        for t in times {
            let got = sampled(&sampler, t);
            let want = linear(CURVES, t);
            assert_eq!(got.len(), want.len(), "t = {t}");
            for ((gid, g), (wid, w)) in got.iter().zip(&want) {
                assert_eq!(gid, wid, "t = {t}");
                assert!((g - w).abs() < 1e-5, "{gid} at t = {t}: {g} != {w}");
            }
            assert_eq!(
                sample_motion_json(CURVES, t).unwrap(),
                sampler.sample_json(t).unwrap()
            );
        }
        // 未知のセグメントのカーブ (AngleZ) と未知の対象は含めない。2 つ目の `start` で終わるカーブ (AngleY) は、
        // 区間の外で値を持たない。
        let ids: Vec<String> = sampled(&sampler, 1.5)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, ["AngleX", "AngleY"]);
        assert_eq!(sampled(&sampler, 3.0).len(), 1);
    }

    #[test]
    fn motion_sampler_rebuilds_indices_when_the_motion_is_replaced() {
        let mut sampler = MotionSampler::from_json(CURVES).unwrap();
        sampler.set_motion_json(MOTION).unwrap();
        assert_eq!(sampler.indices.len(), sampler.motion.curves.len());
        let at = sampled(&sampler, 0.25);
        assert_eq!(at.len(), 1);
        assert_eq!(at[0].0, "AngleX");
        assert!((at[0].1 - 7.5).abs() < 1e-4);
        assert_eq!(sampled(&sampler, 3.0), linear(MOTION, 3.0));

        // 読めない JSON では、今のモーションを保つ。
        assert!(sampler.set_motion_json("{").is_err());
        assert_eq!(sampled(&sampler, 0.25), at);

        // 戻すと、区間の多いカーブも索引で引ける。
        sampler.set_motion_json(CURVES).unwrap();
        assert_eq!(sampled(&sampler, 1.75), linear(CURVES, 1.75));
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

    /// 有限でない値・範囲の外の値・モデルにないパラメータは拒み、今の設定を変えない (T0461)。
    /// 種類と補足の値は C ABI と同じ (`invalid_model`・`errors`)。
    #[test]
    fn invalid_effects_json_is_rejected_without_changes() {
        let mut puppet = quiet(fixtures::pose_and_effects());
        set_effects_json(
            &mut puppet,
            r#"{ "lip_sync": { "parameters": ["MouthOpenY"] } }"#,
        )
        .unwrap();
        puppet.set_lip_sync_level(Some(0.5));
        for (json, count) in [
            // `1e39` は `f32` の無限大になる。
            (r#"{ "eye_blink": { "interval": 1e39 } }"#, 1),
            (r#"{ "eye_blink": { "interval": 0, "closing": -1 } }"#, 2),
            (r#"{ "lip_sync": { "parameters": ["Missing"] } }"#, 1),
        ] {
            let err = set_effects_json(&mut puppet, json).unwrap_err();
            let AnimationError::InvalidEffects(errors) = &err else {
                panic!("not invalid effects: {err}");
            };
            assert_eq!(errors.len(), count, "{json}");
            assert_eq!(err.to_string(), describe_errors("effects", errors));
            let info = err.info().unwrap();
            assert_eq!(info.code, "invalid_model");
            assert_eq!(info.fields, vec![("errors", count.into())]);
            puppet.update(DT);
            assert!(
                (param(&puppet, standard::MOUTH_OPEN_Y) - 0.5).abs() < 1e-5,
                "{json}"
            );
        }
        // 読めない JSON は種類を持たない。
        assert!(
            set_effects_json(&mut puppet, "{")
                .unwrap_err()
                .info()
                .is_none()
        );
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

    /// 評価後のパーツの不透明度は祖先を掛けた値で、無いパーツは `None`。
    #[test]
    fn part_evaluated_opacity_reads_the_evaluated_value() {
        let mut puppet = quiet(fixtures::part_opacity_hierarchy());
        assert_eq!(part_evaluated_opacity(&puppet, "Missing"), None);
        let child = puppet.instance().compiled().part_index("Child").unwrap();
        puppet
            .instance_mut()
            .set_part_opacity_override(child, 1.0, 1.0);
        puppet.instance_mut().update();
        let grandchild = part_evaluated_opacity(&puppet, "Grandchild").unwrap();
        assert!((grandchild - 0.8).abs() < 1e-6, "{grandchild}");
    }

    /// 本体の物理の設定は付け外しの後も保たれる。合成できない・検査を通らない設定では何も変えない。
    #[test]
    fn base_physics_json_survives_dress_up() {
        let mut puppet = quiet(fixtures::dress_up_base());
        puppet
            .attach_module(std::sync::Arc::new(fixtures::long_hair_module()))
            .unwrap();
        assert_eq!(base_physics_json(&puppet).unwrap(), None);
        let windy = ugoe::model::physics::PhysicsSettings {
            gravity: ugoe::core::Vec2::new(0.0, 1.0),
            wind: ugoe::core::Vec2::new(0.3, 0.0),
            fps: ugoe::physics::DEFAULT_FPS,
            rigs: Vec::new(),
            colliders: Vec::new(),
        };
        let json = ugoe::format::physics_to_json(&windy).unwrap();
        set_base_physics_json(&mut puppet, Some(&json)).unwrap();
        puppet
            .attach_module(std::sync::Arc::new(fixtures::dress_module()))
            .unwrap();
        assert_eq!(base_physics_json(&puppet).unwrap(), Some(json));
        let composed = puppet.physics().unwrap().settings();
        assert_eq!(composed.wind, windy.wind);
        assert_eq!(composed.rigs[0].id.as_str(), "LongHair/Sway");

        let mut bad = windy.clone();
        bad.colliders.push(ugoe::model::physics::PhysicsCollider {
            id: "Chest".into(),
            name: "Chest".into(),
            attach: Some("Missing".into()),
            shape: ugoe::model::physics::ColliderShape::Circle {
                center: ugoe::core::Vec2::ZERO,
                radius: 1.0,
            },
        });
        let bad = ugoe::format::physics_to_json(&bad).unwrap();
        let err = set_base_physics_json(&mut puppet, Some(&bad)).unwrap_err();
        let AnimationError::InvalidPhysics(errors) = &err else {
            panic!("not invalid physics: {err}");
        };
        // 文は検証エラーの列の共通の形 (T0015。`docs/spec/model.md#検証エラーの文`)。
        assert_eq!(err.to_string(), describe_errors("physics settings", errors));
        assert!(set_base_physics_json(&mut puppet, Some("{")).is_err());
        assert_eq!(puppet.base_physics(), Some(&windy));

        set_base_physics_json(&mut puppet, None).unwrap();
        assert_eq!(base_physics_json(&puppet).unwrap(), None);
        assert_eq!(
            puppet.physics().unwrap().settings().wind,
            ugoe::core::Vec2::ZERO
        );
    }

    /// ポーズの JSON はモデルの `pose` の形で、付けたモジュールのグループも入る。
    #[test]
    fn pose_json_includes_module_groups() {
        let puppet = quiet(fixtures::pose_and_effects());
        let pose: ugoe::model::pose::Pose =
            serde_json::from_str(&pose_json(&puppet).unwrap()).unwrap();
        assert_eq!(pose, fixtures::pose_and_effects().pose);

        let mut puppet = quiet(fixtures::dress_up_base());
        let mut module = fixtures::long_hair_module();
        module.pose.groups = vec![ugoe::model::pose::PoseGroup {
            entries: vec![ugoe::model::pose::PoseEntry {
                part: "Hair".into(),
                links: Vec::new(),
            }],
        }];
        puppet.attach_module(std::sync::Arc::new(module)).unwrap();
        let pose: ugoe::model::pose::Pose =
            serde_json::from_str(&pose_json(&puppet).unwrap()).unwrap();
        let parts: Vec<&str> = pose
            .groups
            .iter()
            .flat_map(|g| &g.entries)
            .map(|e| e.part.as_str())
            .collect();
        assert_eq!(parts, ["LongHair/Hair"]);
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
            matches!(&warnings[..], [LoadWarning::PhysicsRemoved { reason, codes }] if reason.contains("failed validation") && !codes.is_empty()),
            "{warnings:?}"
        );
        let json = crate::load::warning_json(&warnings[0]);
        assert_eq!(json["code"], "physics_removed");
        // 外した理由の検査のエラーの種類が、補足の値に入る (T0168)。
        assert_eq!(json["error_codes"], "unknown_reference", "{json}");
        assert!(json["errors"].as_u64().is_some_and(|n| n >= 1), "{json}");
        assert!(new.physics().is_none());

        let mut new = Puppet::new(fixtures::physics_hair()).unwrap();
        assert!(carry_over(&old, &mut new, false).is_empty());
        assert!(new.physics().is_none());
    }

    /// パラメータの値と、評価した頂点をつなげたもの (有限かを確かめる)。
    fn state(puppet: &Puppet) -> Vec<f32> {
        let instance = puppet.instance();
        let mut out = instance.parameters().as_slice().to_vec();
        out.extend(
            instance
                .drawables()
                .iter()
                .flat_map(|d| d.positions.iter().flat_map(|p| [p.x, p.y])),
        );
        out
    }

    /// `WebPuppet::update` の負・NaN・無限大の `dt` は戻り、0 とみなされる (T0433)。
    /// 無限大の `dt` でループするモーションが止まらなかった (T0424) ので、物理とループのモーションで確かめる。
    #[test]
    fn update_with_non_finite_dt_returns_and_stays_finite() {
        let run = |bad: Option<f32>| {
            let mut puppet =
                quiet(fixtures::physics_hair()).with_physics(fixtures::physics_hair_settings());
            let looping = MOTION.replace(r#""looping": false"#, r#""looping": true"#);
            assert!(start_motion_json(&mut puppet, &looping, None).unwrap());
            let mut trace = Vec::new();
            for frame in 0..90 {
                if let Some(dt) = bad.filter(|_| frame % 10 == 5) {
                    puppet.update(dt);
                }
                puppet.update(DT);
                trace.extend(state(&puppet));
            }
            trace
        };
        let expected = run(None);
        assert!(expected.iter().all(|v| v.is_finite()));
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            let actual = run(Some(bad));
            assert!(actual.iter().all(|v| v.is_finite()), "dt {bad}");
            assert_eq!(actual, expected, "dt {bad}");
        }
    }

    /// `set_parameter` の NaN は書かず、`set_lip_sync_level` の NaN は `undefined` と同じ。どちらの後の `update` でも
    /// パニックせず、値と頂点は有限のまま (T0433)。無限大は値域の端にクランプする。
    #[test]
    fn non_finite_parameter_and_lip_sync_writes_keep_state_finite() {
        use ugoe::model::effects::{Effects, LipSync};

        let mut puppet =
            quiet(fixtures::physics_hair()).with_physics(fixtures::physics_hair_settings());
        // リップシンクを物理の入力 (`AngleZ`) に当て、NaN が物理と頂点まで届く経路を作る。
        puppet.set_effects(Effects {
            lip_sync: LipSync {
                parameters: vec![standard::ANGLE_Z.into()],
            },
            ..Default::default()
        });
        assert!(crate::load::set_parameter(
            puppet.instance_mut(),
            standard::ANGLE_X,
            20.0
        ));
        puppet.set_lip_sync_level(Some(0.5));
        for _ in 0..10 {
            puppet.update(DT);
        }

        // ID はあるので `true` を返すが、値は変えない。
        let before = param(&puppet, standard::ANGLE_X);
        assert!(crate::load::set_parameter(
            puppet.instance_mut(),
            standard::ANGLE_X,
            f32::NAN
        ));
        assert_eq!(param(&puppet, standard::ANGLE_X), before);
        assert!(!crate::load::set_parameter(
            puppet.instance_mut(),
            "Missing",
            f32::NAN
        ));
        puppet.set_lip_sync_level(Some(f32::NAN));
        for _ in 0..30 {
            puppet.update(DT);
            assert!(state(&puppet).iter().all(|v| v.is_finite()));
        }

        // 無限大は値域の端にクランプする。
        crate::load::set_parameter(puppet.instance_mut(), standard::ANGLE_X, f32::INFINITY);
        assert_eq!(param(&puppet, standard::ANGLE_X), 30.0);
        crate::load::set_parameter(puppet.instance_mut(), standard::ANGLE_X, f32::NEG_INFINITY);
        assert_eq!(param(&puppet, standard::ANGLE_X), -30.0);
        puppet.set_lip_sync_level(Some(f32::INFINITY));
        for _ in 0..30 {
            puppet.update(DT);
            assert!(state(&puppet).iter().all(|v| v.is_finite()));
        }
    }
}
