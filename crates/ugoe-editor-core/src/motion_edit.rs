//! モーションと表情の編集 (T0630)。規則は `docs/editor.md#モーションと表情の編集`。
//!
//! モーション・表情は名前 (`Motion::name`・`Expression::name`) で、カーブは対象 (`CurveTarget`) で、
//! カーブの点 (キー) は時刻 `t` で指定する。名前はプロジェクトのファイル名になるので、テクスチャ名と
//! 同じ規則 (`[A-Za-z0-9_.-]`、`.` で始まらない) で、大文字と小文字を区別せずに一意にする。
//!
//! 適用の後は、編集したモーション・表情を検査し、**適用前になかった問題が生じたら拒否する**
//! (モデルの検証と同じ考え方。読み込んだ時点で不正なデータも、問題を増やさない編集はできる)。

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::Model;
use ugoe_model::effects::{EYE_BLINK_GROUP, LIP_SYNC_GROUP};
use ugoe_model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe_model::motion::{Curve, CurveTarget, Motion, MotionEvent, Point, Segment};

use crate::document::EditError;
use crate::texture::is_valid_texture_name;

/// ドキュメントが持つモーションと表情 (Undo の対象)。並びは保存・書き出しの順。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Animations {
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

/// [`crate::Command::SetMotionSegment`] で設定する区間の種類。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SegmentShape {
    Linear,
    /// 3 次ベジェ。取っ手を省略すると、今の区間がベジェならその取っ手を保ち、そうでなければ
    /// 区間の 1/3 と 2/3 の時刻に両端の値を置いた取っ手 (ゆっくり始まりゆっくり終わる) にする。
    Bezier {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        c1: Option<Point>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        c2: Option<Point>,
    },
    Stepped,
    InverseStepped,
}

// --- モーション ---

pub(crate) fn add_motion(
    anim: &mut Animations,
    model: &Model,
    motion: Motion,
) -> Result<(), EditError> {
    check_new_name("motion", &motion.name, anim.motions.iter().map(|m| &m.name))?;
    for curve in &motion.curves {
        check_target(model, &curve.target)?;
    }
    let errors = motion_errors(&motion);
    if let Some(reason) = errors.into_iter().next() {
        return Err(invalid_motion(&motion.name, reason));
    }
    anim.motions.push(motion);
    Ok(())
}

pub(crate) fn remove_motion(anim: &mut Animations, name: &str) -> Result<(), EditError> {
    let index = motion_index(anim, name)?;
    anim.motions.remove(index);
    Ok(())
}

pub(crate) fn rename_motion(
    anim: &mut Animations,
    name: &str,
    new_name: String,
) -> Result<(), EditError> {
    let index = motion_index(anim, name)?;
    check_new_name(
        "motion",
        &new_name,
        anim.motions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, m)| &m.name),
    )?;
    anim.motions[index].name = new_name;
    Ok(())
}

/// モーション全体の値。省略した (`None` の) ものは変えない。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MotionProperties {
    pub duration: Option<f32>,
    pub looping: Option<bool>,
    pub fade_in: Option<f32>,
    pub fade_out: Option<f32>,
}

pub(crate) fn set_motion_properties(
    anim: &mut Animations,
    name: &str,
    props: MotionProperties,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        if let Some(duration) = props.duration {
            motion.duration = duration;
        }
        if let Some(looping) = props.looping {
            motion.looping = looping;
        }
        if let Some(fade_in) = props.fade_in {
            motion.fade_in = fade_in;
        }
        if let Some(fade_out) = props.fade_out {
            motion.fade_out = fade_out;
        }
        Ok(())
    })
}

pub(crate) fn set_motion_events(
    anim: &mut Animations,
    name: &str,
    events: Vec<MotionEvent>,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        motion.events = events;
        // 時刻の順に並べる (同じ時刻は指定の順)。
        motion.events.sort_by(|a, b| a.t.total_cmp(&b.t));
        Ok(())
    })
}

// --- カーブ ---

pub(crate) fn add_curve(
    anim: &mut Animations,
    model: &Model,
    name: &str,
    curve: Curve,
) -> Result<(), EditError> {
    check_target(model, &curve.target)?;
    edit_motion(anim, name, |motion| {
        if motion.curves.iter().any(|c| c.target == curve.target) {
            return Err(EditError::DuplicateId {
                kind: "curve",
                id: target_label(&curve.target),
            });
        }
        motion.curves.push(curve);
        Ok(())
    })
}

pub(crate) fn remove_curve(
    anim: &mut Animations,
    name: &str,
    target: &CurveTarget,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        let index = curve_index(motion, target)?;
        motion.curves.remove(index);
        Ok(())
    })
}

pub(crate) fn set_curve_fade(
    anim: &mut Animations,
    name: &str,
    target: &CurveTarget,
    fade_in: Option<f32>,
    fade_out: Option<f32>,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        let index = curve_index(motion, target)?;
        let curve = &mut motion.curves[index];
        curve.fade_in = fade_in;
        curve.fade_out = fade_out;
        Ok(())
    })
}

// --- キー (カーブの点) ---

/// 時刻 `point.t` にキーを打つ。その時刻にキーがあれば値を変え、なければ加える
/// (カーブがなければ作る)。規則は `docs/editor.md#モーションと表情の編集`。
pub(crate) fn set_key(
    anim: &mut Animations,
    model: &Model,
    name: &str,
    target: &CurveTarget,
    point: Point,
) -> Result<(), EditError> {
    if !(point.t.is_finite() && point.v.is_finite()) {
        return Err(invalid_motion(name, "key time and value must be finite"));
    }
    let exists = motion(anim, name)?
        .curves
        .iter()
        .any(|c| &c.target == target);
    if !exists {
        check_target(model, target)?;
    }
    edit_motion(anim, name, |motion| {
        let Some(curve) = motion.curves.iter_mut().find(|c| &c.target == target) else {
            motion.curves.push(Curve {
                target: target.clone(),
                segments: vec![Segment::Start { p: point }],
                fade_in: None,
                fade_out: None,
            });
            return Ok(());
        };
        let segments = &mut curve.segments;
        let points = points(segments);
        if let Some(i) = points.iter().position(|p| p.t == point.t) {
            move_point(segments, i, point);
            return Ok(());
        }
        insert_point(segments, point);
        Ok(())
    })
}

/// 時刻 `t` のキーを `to` へ動かす。隣のキーを越えられない。隣の区間のベジェの取っ手は一緒に動かす。
pub(crate) fn move_key(
    anim: &mut Animations,
    name: &str,
    target: &CurveTarget,
    t: f32,
    to: Point,
) -> Result<(), EditError> {
    if !(to.t.is_finite() && to.v.is_finite()) {
        return Err(invalid_motion(name, "key time and value must be finite"));
    }
    edit_motion(anim, name, |motion| {
        let segments = &mut curve_mut(motion, target)?.segments;
        let points = points(segments);
        let i = key_index(&points, target, t)?;
        let after_prev = i == 0 || points[i - 1].t < to.t;
        let before_next = points.get(i + 1).is_none_or(|next| to.t < next.t);
        if !(after_prev && before_next) {
            return Err(EditError::InvalidMotion {
                motion: motion.name.clone(),
                reason: format!(
                    "{}: a key cannot be moved past its neighbor",
                    target_label(target)
                ),
            });
        }
        move_point(segments, i, to);
        Ok(())
    })
}

/// 時刻 `t` のキーを消す。前後の区間は 1 つにつなぐ。最後のキーを消すとカーブを消す。
pub(crate) fn remove_key(
    anim: &mut Animations,
    name: &str,
    target: &CurveTarget,
    t: f32,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        let index = curve_index(motion, target)?;
        let segments = &mut motion.curves[index].segments;
        let i = key_index(&points(segments), target, t)?;
        if segments.len() == 1 {
            motion.curves.remove(index);
            return Ok(());
        }
        remove_point(segments, i);
        Ok(())
    })
}

/// 時刻 `t` のキーで終わる区間の種類を変える。
pub(crate) fn set_segment(
    anim: &mut Animations,
    name: &str,
    target: &CurveTarget,
    t: f32,
    shape: SegmentShape,
) -> Result<(), EditError> {
    edit_motion(anim, name, |motion| {
        let segments = &mut curve_mut(motion, target)?.segments;
        let points = points(segments);
        let i = key_index(&points, target, t)?;
        if i == 0 {
            return Err(EditError::InvalidMotion {
                motion: motion.name.clone(),
                reason: format!(
                    "{}: the first key has no segment before it",
                    target_label(target)
                ),
            });
        }
        let (p0, to) = (points[i - 1], points[i]);
        segments[i] = match shape {
            SegmentShape::Linear => Segment::Linear { to },
            SegmentShape::Stepped => Segment::Stepped { to },
            SegmentShape::InverseStepped => Segment::InverseStepped { to },
            SegmentShape::Bezier { c1, c2 } => {
                let (d1, d2) = match segments[i] {
                    Segment::Bezier { c1, c2, .. } => (c1, c2),
                    _ => ease_handles(p0, to),
                };
                Segment::Bezier {
                    c1: c1.unwrap_or(d1),
                    c2: c2.unwrap_or(d2),
                    to,
                }
            }
        };
        Ok(())
    })
}

// --- 表情 ---

pub(crate) fn add_expression(
    anim: &mut Animations,
    model: &Model,
    expression: Expression,
) -> Result<(), EditError> {
    check_new_name(
        "expression",
        &expression.name,
        anim.expressions.iter().map(|e| &e.name),
    )?;
    check_expression_parameters(model, &expression.parameters)?;
    if let Some(reason) = expression_errors(&expression).into_iter().next() {
        return Err(invalid_expression(&expression.name, reason));
    }
    anim.expressions.push(expression);
    Ok(())
}

pub(crate) fn remove_expression(anim: &mut Animations, name: &str) -> Result<(), EditError> {
    let index = expression_index(anim, name)?;
    anim.expressions.remove(index);
    Ok(())
}

pub(crate) fn rename_expression(
    anim: &mut Animations,
    name: &str,
    new_name: String,
) -> Result<(), EditError> {
    let index = expression_index(anim, name)?;
    check_new_name(
        "expression",
        &new_name,
        anim.expressions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, e)| &e.name),
    )?;
    anim.expressions[index].name = new_name;
    Ok(())
}

pub(crate) fn set_expression_properties(
    anim: &mut Animations,
    name: &str,
    fade_in: Option<f32>,
    fade_out: Option<f32>,
) -> Result<(), EditError> {
    edit_expression(anim, name, |expression| {
        if let Some(fade_in) = fade_in {
            expression.fade_in = fade_in;
        }
        if let Some(fade_out) = fade_out {
            expression.fade_out = fade_out;
        }
    })
}

pub(crate) fn set_expression_parameters(
    anim: &mut Animations,
    model: &Model,
    name: &str,
    parameters: Vec<ExpressionParameter>,
) -> Result<(), EditError> {
    check_expression_parameters(model, &parameters)?;
    edit_expression(anim, name, |expression| expression.parameters = parameters)
}

// --- 検査 ---

/// モーションの問題の一覧 (空なら問題なし)。編集の前後で比べ、増えたら拒否する。
pub fn motion_errors(motion: &Motion) -> Vec<String> {
    let mut errors = Vec::new();
    if !(motion.duration.is_finite() && motion.duration > 0.0) {
        errors.push(format!(
            "duration {} must be a positive number",
            motion.duration
        ));
    }
    for (field, value) in [("fade_in", motion.fade_in), ("fade_out", motion.fade_out)] {
        if !(value.is_finite() && value >= 0.0) {
            errors.push(format!("{field} {value} must be zero or positive"));
        }
    }
    for (i, curve) in motion.curves.iter().enumerate() {
        let label = target_label(&curve.target);
        if curve.target == CurveTarget::Unknown {
            errors.push(format!("curve {i} has an unknown target"));
        }
        if motion.curves[..i].iter().any(|c| c.target == curve.target) {
            errors.push(format!("{label}: more than one curve"));
        }
        for (field, value) in [("fade_in", curve.fade_in), ("fade_out", curve.fade_out)] {
            if let Some(value) = value
                && !(value.is_finite() && value >= 0.0)
            {
                errors.push(format!("{label}: {field} {value} must be zero or positive"));
            }
        }
        if let Err(reason) = check_segments(&curve.segments) {
            errors.push(format!("{label}: {reason}"));
        }
    }
    for event in &motion.events {
        if !(event.t.is_finite() && event.t >= 0.0) {
            errors.push(format!(
                "event `{}`: time {} must be zero or positive",
                event.value, event.t
            ));
        }
    }
    errors
}

/// 表情の問題の一覧 (空なら問題なし)。
pub fn expression_errors(expression: &Expression) -> Vec<String> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("fade_in", expression.fade_in),
        ("fade_out", expression.fade_out),
    ] {
        if !(value.is_finite() && value >= 0.0) {
            errors.push(format!("{field} {value} must be zero or positive"));
        }
    }
    for (i, p) in expression.parameters.iter().enumerate() {
        if p.op == ExpressionOp::Unknown {
            errors.push(format!("parameter `{}`: unknown op", p.parameter));
        }
        if !p.value.is_finite() {
            errors.push(format!(
                "parameter `{}`: value {} is not finite",
                p.parameter, p.value
            ));
        }
        if expression.parameters[..i]
            .iter()
            .any(|q| q.parameter == p.parameter)
        {
            errors.push(format!(
                "parameter `{}` is listed more than once",
                p.parameter
            ));
        }
    }
    errors
}

/// セグメント列: 先頭だけが `start`、点の時刻は増え続け、値と取っ手は有限、
/// 取っ手の時刻は区間の中 (`docs/spec/motion.md#カーブ`)。
fn check_segments(segments: &[Segment]) -> Result<(), String> {
    let Some(Segment::Start { .. }) = segments.first() else {
        return Err("the first segment must be `start`".into());
    };
    let mut prev: Option<Point> = None;
    for segment in segments {
        let to = match segment {
            Segment::Start { p } if prev.is_none() => *p,
            Segment::Start { .. } => return Err("only the first segment can be `start`".into()),
            Segment::Unknown => return Err("unknown segment type".into()),
            Segment::Linear { to }
            | Segment::Stepped { to }
            | Segment::InverseStepped { to }
            | Segment::Bezier { to, .. } => *to,
        };
        if !(to.t.is_finite() && to.v.is_finite()) {
            return Err(format!("key ({}, {}) is not finite", to.t, to.v));
        }
        if let Some(p0) = prev {
            if to.t <= p0.t {
                return Err(format!("key times must increase ({} after {})", to.t, p0.t));
            }
            if let Segment::Bezier { c1, c2, .. } = segment {
                for c in [c1, c2] {
                    if !(c.t.is_finite() && c.v.is_finite()) {
                        return Err(format!("bezier handle ({}, {}) is not finite", c.t, c.v));
                    }
                    if c.t < p0.t || c.t > to.t {
                        return Err(format!(
                            "bezier handle time {} is outside its segment [{}, {}]",
                            c.t, p0.t, to.t
                        ));
                    }
                }
            }
        }
        prev = Some(to);
    }
    Ok(())
}

/// カーブの対象がモデルにあること (パラメータ・パーツ・既知のエフェクトグループ)。
fn check_target(model: &Model, target: &CurveTarget) -> Result<(), EditError> {
    let exists = match target {
        CurveTarget::Parameter(id) => model.parameters.iter().any(|p| &p.id == id),
        CurveTarget::PartOpacity(id) => model.parts.iter().any(|p| &p.id == id),
        CurveTarget::EffectGroup(name) => {
            [EYE_BLINK_GROUP, LIP_SYNC_GROUP].contains(&name.as_str())
        }
        CurveTarget::Unknown => false,
    };
    if exists {
        return Ok(());
    }
    let (kind, id) = match target {
        CurveTarget::Parameter(id) => ("parameter", id.to_string()),
        CurveTarget::PartOpacity(id) => ("part", id.to_string()),
        CurveTarget::EffectGroup(name) => ("effect group", name.clone()),
        CurveTarget::Unknown => ("curve target", "unknown".to_owned()),
    };
    Err(EditError::NotFound { kind, id })
}

fn check_expression_parameters(
    model: &Model,
    parameters: &[ExpressionParameter],
) -> Result<(), EditError> {
    for p in parameters {
        if !model.parameters.iter().any(|q| q.id == p.parameter) {
            return Err(EditError::NotFound {
                kind: "parameter",
                id: p.parameter.to_string(),
            });
        }
    }
    Ok(())
}

/// 新しい名前がファイル名に使え、`others` と大文字小文字を区別せずに重ならないこと。
fn check_new_name<'a>(
    kind: &'static str,
    name: &str,
    mut others: impl Iterator<Item = &'a String>,
) -> Result<(), EditError> {
    if !is_valid_texture_name(name) {
        return Err(EditError::InvalidName {
            kind,
            name: name.to_owned(),
        });
    }
    if others.any(|other| other.eq_ignore_ascii_case(name)) {
        return Err(EditError::DuplicateId {
            kind,
            id: name.to_owned(),
        });
    }
    Ok(())
}

// --- 探す・適用する ---

fn motion_index(anim: &Animations, name: &str) -> Result<usize, EditError> {
    anim.motions
        .iter()
        .position(|m| m.name == name)
        .ok_or_else(|| EditError::NotFound {
            kind: "motion",
            id: name.to_owned(),
        })
}

fn motion<'a>(anim: &'a Animations, name: &str) -> Result<&'a Motion, EditError> {
    Ok(&anim.motions[motion_index(anim, name)?])
}

fn expression_index(anim: &Animations, name: &str) -> Result<usize, EditError> {
    anim.expressions
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| EditError::NotFound {
            kind: "expression",
            id: name.to_owned(),
        })
}

/// モーション `name` を `edit` で書き換え、問題が増えたら拒否する (呼び出し側が状態を捨てる)。
fn edit_motion(
    anim: &mut Animations,
    name: &str,
    edit: impl FnOnce(&mut Motion) -> Result<(), EditError>,
) -> Result<(), EditError> {
    let index = motion_index(anim, name)?;
    let motion = &mut anim.motions[index];
    let before = motion_errors(motion);
    edit(motion)?;
    let introduced = motion_errors(motion)
        .into_iter()
        .find(|e| !before.contains(e));
    match introduced {
        Some(reason) => Err(invalid_motion(name, reason)),
        None => Ok(()),
    }
}

fn edit_expression(
    anim: &mut Animations,
    name: &str,
    edit: impl FnOnce(&mut Expression),
) -> Result<(), EditError> {
    let index = expression_index(anim, name)?;
    let expression = &mut anim.expressions[index];
    let before = expression_errors(expression);
    edit(expression);
    let introduced = expression_errors(expression)
        .into_iter()
        .find(|e| !before.contains(e));
    match introduced {
        Some(reason) => Err(invalid_expression(name, reason)),
        None => Ok(()),
    }
}

fn invalid_motion(name: &str, reason: impl Into<String>) -> EditError {
    EditError::InvalidMotion {
        motion: name.to_owned(),
        reason: reason.into(),
    }
}

fn invalid_expression(name: &str, reason: impl Into<String>) -> EditError {
    EditError::InvalidExpression {
        expression: name.to_owned(),
        reason: reason.into(),
    }
}

fn curve_index(motion: &Motion, target: &CurveTarget) -> Result<usize, EditError> {
    motion
        .curves
        .iter()
        .position(|c| &c.target == target)
        .ok_or_else(|| EditError::NotFound {
            kind: "curve",
            id: target_label(target),
        })
}

fn curve_mut<'a>(motion: &'a mut Motion, target: &CurveTarget) -> Result<&'a mut Curve, EditError> {
    let index = curve_index(motion, target)?;
    Ok(&mut motion.curves[index])
}

fn key_index(points: &[Point], target: &CurveTarget, t: f32) -> Result<usize, EditError> {
    points
        .iter()
        .position(|p| p.t == t)
        .ok_or_else(|| EditError::NotFound {
            kind: "key",
            id: format!("{} at {t}", target_label(target)),
        })
}

/// エラーメッセージ用のカーブの対象の表記 (例: ``parameter `AngleX` ``)。
pub fn target_label(target: &CurveTarget) -> String {
    match target {
        CurveTarget::Parameter(id) => format!("parameter `{id}`"),
        CurveTarget::PartOpacity(id) => format!("part opacity `{id}`"),
        CurveTarget::EffectGroup(name) => format!("effect group `{name}`"),
        CurveTarget::Unknown => "unknown target".to_owned(),
    }
}

// --- セグメント列の操作 ---

/// 各セグメントの終点 (`start` は始点)。未知のセグメントは直前の点で埋める。
fn points(segments: &[Segment]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(segments.len());
    for segment in segments {
        let p = end_point(segment).or_else(|| out.last().copied());
        out.push(p.unwrap_or(Point { t: 0.0, v: 0.0 }));
    }
    out
}

fn end_point(segment: &Segment) -> Option<Point> {
    match segment {
        Segment::Start { p } => Some(*p),
        Segment::Linear { to }
        | Segment::Bezier { to, .. }
        | Segment::Stepped { to }
        | Segment::InverseStepped { to } => Some(*to),
        Segment::Unknown => None,
    }
}

fn set_end_point(segment: &mut Segment, point: Point) {
    match segment {
        Segment::Start { p } => *p = point,
        Segment::Linear { to }
        | Segment::Bezier { to, .. }
        | Segment::Stepped { to }
        | Segment::InverseStepped { to } => *to = point,
        Segment::Unknown => {}
    }
}

/// 同じ種類で終点だけを替えたセグメント (ベジェは既定の取っ手)。`p0` は始点。
fn same_kind(segment: &Segment, p0: Point, to: Point) -> Segment {
    match segment {
        Segment::Bezier { .. } => {
            let (c1, c2) = ease_handles(p0, to);
            Segment::Bezier { c1, c2, to }
        }
        Segment::Stepped { .. } => Segment::Stepped { to },
        Segment::InverseStepped { .. } => Segment::InverseStepped { to },
        Segment::Start { .. } | Segment::Linear { .. } | Segment::Unknown => Segment::Linear { to },
    }
}

/// 区間の 1/3 と 2/3 の時刻に、両端の値を置いた取っ手 (ゆっくり始まりゆっくり終わる)。
fn ease_handles(p0: Point, to: Point) -> (Point, Point) {
    let dt = (to.t - p0.t) / 3.0;
    (
        Point {
            t: p0.t + dt,
            v: p0.v,
        },
        Point {
            t: to.t - dt,
            v: to.v,
        },
    )
}

fn lerp(a: Point, b: Point, s: f32) -> Point {
    Point {
        t: a.t + (b.t - a.t) * s,
        v: a.v + (b.v - a.v) * s,
    }
}

fn offset(p: Point, dt: f32, dv: f32) -> Point {
    Point {
        t: p.t + dt,
        v: p.v + dv,
    }
}

/// 取っ手の時刻を区間 `[lo, hi]` に収める。
fn clamp_handle(c: &mut Point, lo: f32, hi: f32) {
    c.t = c.t.clamp(lo, hi);
}

/// キーのない時刻 `point.t` にキーを加える。
///
/// - 最初のキーより前: 新しい `start` にし、元の最初のキーへは最初の区間と同じ種類でつなぐ。
/// - 最後のキーより後: 最後の区間と同じ種類で加える (ベジェは既定の取っ手。区間がなければ直線)。
/// - 区間の中: 区間を同じ種類の 2 つに分ける。ベジェは de Casteljau で分け、新しいキーの側の取っ手を
///   値の差だけずらすので、今のカーブの上に打つと形が変わらない。
fn insert_point(segments: &mut Vec<Segment>, point: Point) {
    let points = points(segments);
    let first = points[0];
    if point.t < first.t {
        let next = segments
            .get(1)
            .cloned()
            .unwrap_or(Segment::Linear { to: first });
        segments[0] = same_kind(&next, point, first);
        segments.insert(0, Segment::Start { p: point });
        return;
    }
    let Some(i) = points.iter().position(|p| p.t > point.t) else {
        let last = points[points.len() - 1];
        let appended = match segments.last() {
            Some(segment) if segments.len() > 1 => same_kind(segment, last, point),
            _ => Segment::Linear { to: point },
        };
        segments.push(appended);
        return;
    };
    let (p0, to) = (points[i - 1], points[i]);
    let (first_half, second_half) = match segments[i] {
        Segment::Bezier { c1, c2, .. } => {
            let span = to.t - p0.t;
            let x1 = ((c1.t - p0.t) / span).clamp(0.0, 1.0);
            let x2 = ((c2.t - p0.t) / span).clamp(0.0, 1.0);
            let x = ((point.t - p0.t) / span).clamp(0.0, 1.0);
            let s = ugoe_motion::curve::solve_monotonic_bezier(x1, x2, x);
            // de Casteljau。
            let a = lerp(p0, c1, s);
            let b = lerp(c1, c2, s);
            let c = lerp(c2, to, s);
            let d = lerp(a, b, s);
            let e = lerp(b, c, s);
            let mid = lerp(d, e, s);
            // 分けた点を新しいキーに合わせる (時刻は解の誤差の分、値は打った値の分ずらす)。
            let (dt, dv) = (point.t - mid.t, point.v - mid.v);
            let mut d = offset(d, dt, dv);
            let mut e = offset(e, dt, dv);
            clamp_handle(&mut d, p0.t, point.t);
            clamp_handle(&mut e, point.t, to.t);
            let mut a = a;
            let mut c = c;
            clamp_handle(&mut a, p0.t, point.t);
            clamp_handle(&mut c, point.t, to.t);
            (
                Segment::Bezier {
                    c1: a,
                    c2: d,
                    to: point,
                },
                Segment::Bezier { c1: e, c2: c, to },
            )
        }
        ref other => (same_kind(other, p0, point), same_kind(other, point, to)),
    };
    segments[i] = second_half;
    segments.insert(i, first_half);
}

/// `i` 番目のキーを `to` へ動かす (隣を越えないことは呼び出し側が確かめる)。
/// 前後の区間のベジェの、このキーの側の取っ手を同じだけ動かし、取っ手の時刻を区間に収める。
fn move_point(segments: &mut [Segment], i: usize, to: Point) {
    let points = points(segments);
    let (dt, dv) = (to.t - points[i].t, to.v - points[i].v);
    set_end_point(&mut segments[i], to);
    if i > 0
        && let Segment::Bezier { c1, c2, .. } = &mut segments[i]
    {
        let lo = points[i - 1].t;
        *c2 = offset(*c2, dt, dv);
        clamp_handle(c1, lo, to.t);
        clamp_handle(c2, lo, to.t);
    }
    if let Some(Segment::Bezier { c1, c2, to: next }) = segments.get_mut(i + 1) {
        let hi = next.t;
        *c1 = offset(*c1, dt, dv);
        clamp_handle(c1, to.t, hi);
        clamp_handle(c2, to.t, hi);
    }
}

/// `i` 番目のキーを消す (キーが 2 つ以上あること)。
///
/// - 最初のキー: 次のキーを `start` にする。
/// - 途中のキー: 前後の区間を、消したキーに入る区間の種類で 1 つにする。ベジェなら前の区間の
///   始点側の取っ手と、後の区間の終点側の取っ手 (後の区間がベジェでなければ既定の取っ手) を使う。
fn remove_point(segments: &mut Vec<Segment>, i: usize) {
    let points = points(segments);
    if i == 0 {
        segments.remove(0);
        segments[0] = Segment::Start { p: points[1] };
        return;
    }
    if i + 1 == segments.len() {
        segments.remove(i);
        return;
    }
    let (p0, to) = (points[i - 1], points[i + 1]);
    let merged = match (&segments[i], &segments[i + 1]) {
        (Segment::Bezier { c1, .. }, next) => {
            let c2 = match next {
                Segment::Bezier { c2, .. } => *c2,
                _ => ease_handles(p0, to).1,
            };
            Segment::Bezier { c1: *c1, c2, to }
        }
        (incoming, _) => same_kind(incoming, p0, to),
    };
    segments[i + 1] = merged;
    segments.remove(i);
}
