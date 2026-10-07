//! モーションと表情の数値の検査。検査項目は `docs/spec/model.md#モーションと表情の検証`。
//!
//! モーションと表情はモデルと照らさない (対象が存在しないカーブと項目は効かないだけ)。
//! 値の場所 (`field`) はモーション・表情の中の場所 (`duration`、`curves[0].segments[1].to.t` など)。

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::{ValidationError, check_finite, check_non_negative};
use crate::expression::Expression;
use crate::motion::{Motion, Point, Segment};

/// モーションの数値を検査する。検出したエラーを全て返す。
///
/// 長さ・フェード・カーブの点 (ベジェの制御点を含む)・イベントの時刻が有限値で、`duration` が 0 以上であること。
/// カーブの点は、有限でない値の最初の 1 つだけをカーブごとに報告する。
/// 時刻が減る点はエラーにしない ([`motion_warnings`])。
pub fn validate_motion(motion: &Motion) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    check_non_negative("duration", motion.duration, &mut errors);
    check_finite("fade_in", motion.fade_in, &mut errors);
    check_finite("fade_out", motion.fade_out, &mut errors);
    for (index, curve) in motion.curves.iter().enumerate() {
        let path = format!("curves[{index}]");
        if let Some(fade_in) = curve.fade_in {
            check_finite(&format!("{path}.fade_in"), fade_in, &mut errors);
        }
        if let Some(fade_out) = curve.fade_out {
            check_finite(&format!("{path}.fade_out"), fade_out, &mut errors);
        }
        let non_finite = curve.segments.iter().enumerate().find_map(|(i, segment)| {
            points(segment).into_iter().find_map(|(name, point)| {
                [("t", point.t), ("v", point.v)]
                    .into_iter()
                    .find(|(_, value)| !value.is_finite())
                    .map(|(axis, value)| (format!("{path}.segments[{i}].{name}.{axis}"), value))
            })
        });
        if let Some((field, value)) = non_finite {
            check_finite(&field, value, &mut errors);
        }
    }
    for (index, event) in motion.events.iter().enumerate() {
        check_finite(&format!("events[{index}].t"), event.t, &mut errors);
    }
    errors
}

/// 表情の数値を検査する。検出したエラーを全て返す。
///
/// フェードと各項目の `value` が有限値であること (`op` が未知の項目も検査する)。
pub fn validate_expression(expression: &Expression) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    check_finite("fade_in", expression.fade_in, &mut errors);
    check_finite("fade_out", expression.fade_out, &mut errors);
    for (index, item) in expression.parameters.iter().enumerate() {
        check_finite(
            &format!("parameters[{index}].value"),
            item.value,
            &mut errors,
        );
    }
    errors
}

/// モーションの警告。エラーと違い、再生の結果は決まっている (`docs/spec/motion.md#カーブ`)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MotionWarning {
    /// カーブの点の時刻が、直前の点より前に戻る (書き出しの規則では減らないことが望ましい)。
    #[error("{field} = {time} is earlier than the previous point ({previous})")]
    DecreasingTime {
        /// 点の時刻の場所。
        field: String,
        /// 点の時刻 (秒)。
        time: f32,
        /// 直前の点の時刻 (秒)。
        previous: f32,
    },
}

/// モーションの警告 (エラーにしないもの) を返す。
///
/// カーブの点 (各セグメントの終点。ベジェの制御点は除く) の時刻が直前の点より小さいものを報告する。
/// `start` から数え直し、未知のセグメントを含むカーブ (全体を無視する) と有限でない時刻は見ない。
pub fn motion_warnings(motion: &Motion) -> Vec<MotionWarning> {
    let mut warnings = Vec::new();
    for (index, curve) in motion.curves.iter().enumerate() {
        if curve.segments.contains(&Segment::Unknown) {
            continue;
        }
        let mut previous: Option<f32> = None;
        for (i, segment) in curve.segments.iter().enumerate() {
            let (name, point) = match segment {
                Segment::Start { p } => {
                    previous = None;
                    ("p", p)
                }
                Segment::Linear { to }
                | Segment::Bezier { to, .. }
                | Segment::Stepped { to }
                | Segment::InverseStepped { to } => ("to", to),
                Segment::Unknown => continue,
            };
            if !point.t.is_finite() {
                continue;
            }
            if let Some(previous) = previous
                && point.t < previous
            {
                warnings.push(MotionWarning::DecreasingTime {
                    field: format!("curves[{index}].segments[{i}].{name}.t"),
                    time: point.t,
                    previous,
                });
            }
            previous = Some(point.t);
        }
    }
    warnings
}

/// セグメントの点と、その場所の名前。
fn points(segment: &Segment) -> Vec<(&'static str, Point)> {
    match *segment {
        Segment::Start { p } => vec![("p", p)],
        Segment::Bezier { c1, c2, to } => vec![("c1", c1), ("c2", c2), ("to", to)],
        Segment::Linear { to } | Segment::Stepped { to } | Segment::InverseStepped { to } => {
            vec![("to", to)]
        }
        Segment::Unknown => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParamId;
    use crate::expression::{ExpressionOp, ExpressionParameter};
    use crate::motion::{Curve, CurveTarget, MotionEvent};
    use alloc::borrow::ToOwned;

    fn point(t: f32, v: f32) -> Point {
        Point { t, v }
    }

    fn curve(segments: Vec<Segment>) -> Curve {
        Curve {
            target: CurveTarget::Parameter(ParamId::new("ParamAngleX")),
            segments,
            fade_in: None,
            fade_out: None,
        }
    }

    fn motion(curves: Vec<Curve>) -> Motion {
        Motion {
            name: "nod".to_owned(),
            duration: 1.0,
            looping: false,
            fade_in: 0.2,
            fade_out: 0.2,
            curves,
            events: vec![MotionEvent {
                t: 0.5,
                value: "half".to_owned(),
            }],
        }
    }

    fn fields(errors: &[ValidationError]) -> Vec<&str> {
        errors
            .iter()
            .map(|e| match e {
                ValidationError::InvalidValue { field, .. } => field.as_str(),
                other => panic!("unexpected error {other}"),
            })
            .collect()
    }

    #[test]
    fn valid_motion_passes() {
        let m = motion(vec![curve(vec![
            Segment::Start { p: point(0.0, 0.0) },
            Segment::Bezier {
                c1: point(0.1, 1.0),
                c2: point(0.4, 1.0),
                to: point(0.5, 1.0),
            },
            Segment::Linear {
                to: point(1.0, 0.0),
            },
        ])]);
        assert!(validate_motion(&m).is_empty());
        assert!(motion_warnings(&m).is_empty());
    }

    #[test]
    fn reports_non_finite_motion_values() {
        let mut m = motion(vec![
            curve(vec![
                Segment::Start { p: point(0.0, 0.0) },
                // 1 本のカーブでは最初の 1 つだけを報告する。
                Segment::Bezier {
                    c1: point(0.1, 1.0),
                    c2: point(f32::NAN, 1.0),
                    to: point(0.5, f32::INFINITY),
                },
            ]),
            curve(vec![Segment::Start {
                p: point(0.0, f32::NEG_INFINITY),
            }]),
        ]);
        m.duration = f32::NAN;
        m.fade_in = f32::INFINITY;
        m.fade_out = f32::NAN;
        m.curves[0].fade_in = Some(f32::NAN);
        m.curves[1].fade_out = Some(f32::INFINITY);
        m.events[0].t = f32::NAN;
        assert_eq!(
            fields(&validate_motion(&m)),
            [
                "duration",
                "fade_in",
                "fade_out",
                "curves[0].fade_in",
                "curves[0].segments[1].c2.t",
                "curves[1].fade_out",
                "curves[1].segments[0].p.v",
                "events[0].t",
            ]
        );
    }

    #[test]
    fn curves_ignored_by_playback_are_validated() {
        // 未知の対象・未知のエフェクトグループ名・未知のセグメントを含むカーブも、フェードと既知のセグメントの点を検査する
        // (`docs/spec/model.md#モーションと表情の検証`)。
        let mut m = motion(vec![
            curve(vec![Segment::Start {
                p: point(f32::NAN, 0.0),
            }]),
            curve(vec![Segment::Start {
                p: point(0.0, f32::INFINITY),
            }]),
            curve(vec![
                Segment::Start { p: point(0.0, 0.0) },
                Segment::Unknown,
                Segment::Linear {
                    to: point(1.0, f32::NAN),
                },
            ]),
        ]);
        m.curves[0].target = CurveTarget::Unknown;
        m.curves[0].fade_in = Some(f32::NAN);
        m.curves[1].target = CurveTarget::EffectGroup("Unknown".to_owned());
        m.curves[2].fade_out = Some(f32::INFINITY);
        assert_eq!(
            fields(&validate_motion(&m)),
            [
                "curves[0].fade_in",
                "curves[0].segments[0].p.t",
                "curves[1].segments[0].p.v",
                "curves[2].fade_out",
                "curves[2].segments[2].to.v",
            ]
        );
    }

    #[test]
    fn duration_must_not_be_negative() {
        let mut m = motion(Vec::new());
        m.duration = -1.0;
        assert_eq!(
            validate_motion(&m),
            [ValidationError::InvalidValue {
                field: "duration".to_owned(),
                value: -1.0,
                expected: "finite and >= 0",
            }]
        );
        m.duration = 0.0;
        assert!(validate_motion(&m).is_empty());
    }

    #[test]
    fn runtime_fallbacks_are_not_errors() {
        // 負のフェードはフェードなし、区間の外の制御点は区間へのクランプ、範囲の外のイベントは発火しないだけ
        // (`docs/spec/motion.md`)。
        let mut m = motion(vec![curve(vec![
            Segment::Start {
                p: point(-1.0, 0.0),
            },
            Segment::Bezier {
                c1: point(-5.0, 1.0),
                c2: point(9.0, 1.0),
                to: point(2.0, 1.0),
            },
        ])]);
        m.fade_in = -1.0;
        m.fade_out = -1.0;
        m.curves[0].fade_in = Some(-1.0);
        m.events[0].t = 5.0;
        assert!(validate_motion(&m).is_empty());
    }

    #[test]
    fn decreasing_times_are_warnings() {
        let m = motion(vec![
            curve(vec![
                Segment::Start { p: point(0.0, 0.0) },
                Segment::Linear {
                    to: point(1.0, 1.0),
                },
                Segment::Stepped {
                    to: point(0.5, 0.0),
                },
                // 同じ時刻は減っていない。
                Segment::InverseStepped {
                    to: point(0.5, 1.0),
                },
                // 2 つ目の `start` から数え直す。
                Segment::Start { p: point(0.2, 0.0) },
                Segment::Linear {
                    to: point(0.1, 1.0),
                },
            ]),
            // 未知のセグメントを含むカーブは全体を無視するので見ない。
            curve(vec![
                Segment::Start { p: point(1.0, 0.0) },
                Segment::Unknown,
                Segment::Linear {
                    to: point(0.0, 0.0),
                },
            ]),
        ]);
        assert!(validate_motion(&m).is_empty());
        assert_eq!(
            motion_warnings(&m),
            [
                MotionWarning::DecreasingTime {
                    field: "curves[0].segments[2].to.t".to_owned(),
                    time: 0.5,
                    previous: 1.0,
                },
                MotionWarning::DecreasingTime {
                    field: "curves[0].segments[5].to.t".to_owned(),
                    time: 0.1,
                    previous: 0.2,
                },
            ]
        );
    }

    #[test]
    fn reports_non_finite_expression_values() {
        let item = |value, op| ExpressionParameter {
            parameter: ParamId::new("ParamEyeLOpen"),
            value,
            op,
        };
        let mut expression = Expression {
            name: "smile".to_owned(),
            fade_in: 0.5,
            fade_out: -1.0,
            parameters: vec![
                item(0.5, ExpressionOp::Add),
                item(f32::NAN, ExpressionOp::Multiply),
                item(f32::INFINITY, ExpressionOp::Unknown),
            ],
        };
        assert_eq!(
            fields(&validate_expression(&expression)),
            ["parameters[1].value", "parameters[2].value"]
        );
        expression.fade_in = f32::NAN;
        expression.fade_out = f32::INFINITY;
        expression.parameters.truncate(1);
        assert_eq!(
            fields(&validate_expression(&expression)),
            ["fade_in", "fade_out"]
        );
    }
}
