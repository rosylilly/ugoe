//! モーションカーブの評価。

use ugoe_model::motion::Segment;

/// カーブを時刻 `t` で評価する。値がない (カーブを適用しない) ときは `None`。
///
/// 未知のセグメント (`Segment::Unknown`) を含むカーブは、全体を無視する
/// (`docs/spec/file-formats.md#未知の値の読み込み`)。
///
/// TODO(T0302): ベジェ区間の t→値 解決 (単調 x に対する 3 次方程式) を実装する。
pub fn evaluate(segments: &[Segment], t: f32) -> Option<f32> {
    if segments.contains(&Segment::Unknown) {
        return None;
    }
    let mut prev = match segments.first()? {
        Segment::Start { p } => *p,
        _ => return None,
    };
    if t <= prev.t {
        return Some(prev.v);
    }
    for seg in &segments[1..] {
        let to = match seg {
            Segment::Start { .. } | Segment::Unknown => return None,
            Segment::Linear { to }
            | Segment::Bezier { to, .. }
            | Segment::Stepped { to }
            | Segment::InverseStepped { to } => *to,
        };
        if t <= to.t {
            let u = if to.t > prev.t {
                (t - prev.t) / (to.t - prev.t)
            } else {
                1.0
            };
            return Some(match seg {
                Segment::Linear { .. } | Segment::Bezier { .. } => prev.v + (to.v - prev.v) * u,
                Segment::Stepped { .. } => {
                    if u < 1.0 {
                        prev.v
                    } else {
                        to.v
                    }
                }
                Segment::InverseStepped { .. } => {
                    if u > 0.0 {
                        to.v
                    } else {
                        prev.v
                    }
                }
                Segment::Start { .. } | Segment::Unknown => unreachable!(),
            });
        }
        prev = to;
    }
    Some(prev.v)
}

#[cfg(test)]
mod tests {
    use ugoe_model::motion::Point;

    use super::*;

    #[test]
    fn linear_and_hold() {
        let segs = [
            Segment::Start {
                p: Point { t: 0.0, v: 0.0 },
            },
            Segment::Linear {
                to: Point { t: 1.0, v: 10.0 },
            },
            Segment::Stepped {
                to: Point { t: 2.0, v: 0.0 },
            },
        ];
        assert_eq!(evaluate(&segs, 0.5), Some(5.0));
        assert_eq!(evaluate(&segs, 1.5), Some(10.0));
        assert_eq!(evaluate(&segs, 2.0), Some(0.0));
        assert_eq!(evaluate(&segs, 9.0), Some(0.0));
    }

    #[test]
    fn curve_with_unknown_segment_is_ignored() {
        let segs = [
            Segment::Start {
                p: Point { t: 0.0, v: 0.0 },
            },
            Segment::Linear {
                to: Point { t: 1.0, v: 10.0 },
            },
            Segment::Unknown,
        ];
        assert_eq!(evaluate(&segs, 0.5), None);
        assert_eq!(evaluate(&segs, 2.0), None);
    }
}
