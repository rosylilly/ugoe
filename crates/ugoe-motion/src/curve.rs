//! モーションカーブの評価。仕様: `docs/spec/motion.md#カーブ`。

use ugoe_model::motion::{Point, Segment};

/// ベジェの t→s を解くときの許容誤差 (正規化した区間 `[0, 1]` での時刻の差)。
const BEZIER_TOLERANCE: f32 = 1e-6;
/// ベジェの t→s を解くときの反復の上限。二分法だけでも 2^-40 まで縮むので十分。
const BEZIER_MAX_ITERATIONS: u32 = 40;

/// カーブを時刻 `t` で評価する。値がない (カーブを適用しない) ときは `None`。
///
/// 未知のセグメント (`Segment::Unknown`) を含むカーブは、全体を無視する
/// (`docs/spec/file-formats.md#未知の値の読み込み`)。
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
                Segment::Linear { .. } => prev.v + (to.v - prev.v) * u,
                Segment::Bezier { c1, c2, .. } => bezier(prev, *c1, *c2, to, t),
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

/// 3 次ベジェ区間 `p0, c1, c2, p3` を時刻 `t` (`p0.t < t <= p3.t`) で評価する。
///
/// `B_t(s) = t` を `s` について解き、`B_v(s)` を返す。制御点の時刻は区間内に
/// クランプするので `B_t` は単調非減少になり、解はただ 1 つに決まる。
fn bezier(p0: Point, c1: Point, c2: Point, p3: Point, t: f32) -> f32 {
    let span = p3.t - p0.t;
    if span <= 0.0 {
        return p3.v;
    }
    // 時刻を区間 [0, 1] に正規化する。
    let x1 = ((c1.t - p0.t) / span).clamp(0.0, 1.0);
    let x2 = ((c2.t - p0.t) / span).clamp(0.0, 1.0);
    let x = ((t - p0.t) / span).clamp(0.0, 1.0);
    let s = solve_monotonic_bezier(x1, x2, x);
    cubic(p0.v, c1.v, c2.v, p3.v, s)
}

/// 端点が 0 と 1、制御点が `x1`, `x2` (どちらも `[0, 1]`) の 3 次ベジェ `B(s)` について、
/// `B(s) = x` となる `s` (`[0, 1]`) を求める。
///
/// `B` の単調性を前提に、ニュートン法を二分法で守る (Newton が区間の外へ出るか
/// 導関数が 0 に近いときは二分する)。区間 `[lo, hi]` は常に解を含む。
pub fn solve_monotonic_bezier(x1: f32, x2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    // 直線 (x1 = 1/3, x2 = 2/3) なら s = x が解なので、初期値に使う。
    let mut s = x;
    for _ in 0..BEZIER_MAX_ITERATIONS {
        let err = cubic(0.0, x1, x2, 1.0, s) - x;
        if err.abs() <= BEZIER_TOLERANCE {
            return s;
        }
        if err > 0.0 {
            hi = s;
        } else {
            lo = s;
        }
        let d = cubic_derivative(0.0, x1, x2, 1.0, s);
        let newton = if d > f32::EPSILON {
            s - err / d
        } else {
            f32::NAN
        };
        s = if newton > lo && newton < hi {
            newton
        } else {
            0.5 * (lo + hi)
        };
        if hi - lo <= BEZIER_TOLERANCE {
            break;
        }
    }
    s
}

/// 1 次元の 3 次ベジェ `(1-s)³a + 3(1-s)²s b + 3(1-s)s² c + s³ d`。
fn cubic(a: f32, b: f32, c: f32, d: f32, s: f32) -> f32 {
    let r = 1.0 - s;
    r * r * r * a + 3.0 * r * r * s * b + 3.0 * r * s * s * c + s * s * s * d
}

/// [`cubic`] の `s` による微分。
fn cubic_derivative(a: f32, b: f32, c: f32, d: f32, s: f32) -> f32 {
    let r = 1.0 - s;
    3.0 * (r * r * (b - a) + 2.0 * r * s * (c - b) + s * s * (d - c))
}

#[cfg(test)]
mod tests {
    use ugoe_model::motion::Point;

    use super::*;

    fn p(t: f32, v: f32) -> Point {
        Point { t, v }
    }

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn linear_and_hold() {
        let segs = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Linear { to: p(1.0, 10.0) },
            Segment::Stepped { to: p(2.0, 0.0) },
        ];
        assert_eq!(evaluate(&segs, 0.5), Some(5.0));
        assert_eq!(evaluate(&segs, 1.5), Some(10.0));
        assert_eq!(evaluate(&segs, 2.0), Some(0.0));
        assert_eq!(evaluate(&segs, 9.0), Some(0.0));
    }

    #[test]
    fn inverse_stepped_switches_right_after_start() {
        let segs = [
            Segment::Start { p: p(0.0, 1.0) },
            Segment::InverseStepped { to: p(1.0, 5.0) },
        ];
        assert_eq!(evaluate(&segs, 0.0), Some(1.0));
        assert_eq!(evaluate(&segs, 0.001), Some(5.0));
        assert_eq!(evaluate(&segs, 1.0), Some(5.0));
    }

    #[test]
    fn before_first_and_after_last_point_hold_end_values() {
        let segs = [
            Segment::Start { p: p(1.0, 3.0) },
            Segment::Linear { to: p(2.0, 7.0) },
        ];
        assert_eq!(evaluate(&segs, -5.0), Some(3.0));
        assert_eq!(evaluate(&segs, 5.0), Some(7.0));
    }

    #[test]
    fn curve_with_unknown_segment_is_ignored() {
        let segs = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Linear { to: p(1.0, 10.0) },
            Segment::Unknown,
        ];
        assert_eq!(evaluate(&segs, 0.5), None);
        assert_eq!(evaluate(&segs, 2.0), None);
    }

    /// 制御点を 1/3, 2/3 に置いたベジェは直線と一致する (`docs/spec/motion.md#カーブ`)。
    #[test]
    fn bezier_with_thirds_equals_linear() {
        let segs = [
            Segment::Start { p: p(1.0, 2.0) },
            Segment::Bezier {
                c1: p(2.0, 4.0),
                c2: p(3.0, 6.0),
                to: p(4.0, 8.0),
            },
        ];
        for i in 0..=30 {
            let t = 1.0 + i as f32 * 0.1;
            let v = evaluate(&segs, t).unwrap();
            assert!(close(v, 2.0 * t, 1e-4), "t={t} v={v}");
        }
    }

    /// 解いた s を B_t に戻すと元の t になり、値は B_v(s) と一致する。
    #[test]
    fn bezier_solves_t_to_s() {
        let (p0, c1, c2, p3) = (p(0.0, 0.0), p(0.9, 1.0), p(0.1, -1.0), p(2.0, 3.0));
        let segs = [Segment::Start { p: p0 }, Segment::Bezier { c1, c2, to: p3 }];
        for i in 1..=100 {
            let t = i as f32 * 0.02;
            let x = t / 2.0;
            let s = solve_monotonic_bezier(c1.t / 2.0, c2.t / 2.0, x);
            assert!((0.0..=1.0).contains(&s));
            assert!(
                close(cubic(0.0, c1.t, c2.t, 2.0, s), t, 1e-5),
                "t={t} s={s}"
            );
            let expected = cubic(p0.v, c1.v, c2.v, p3.v, s);
            assert!(close(evaluate(&segs, t).unwrap(), expected, 1e-5));
        }
        assert_eq!(evaluate(&segs, 0.0), Some(0.0));
        assert_eq!(evaluate(&segs, 2.0), Some(3.0));
    }

    /// ease-in-out (制御点の時刻が端に寄る) でも、解は t について単調になる。
    #[test]
    fn bezier_solution_is_monotonic_for_steep_controls() {
        for &(x1, x2) in &[(0.0, 1.0), (1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (0.5, 0.5)] {
            let mut prev = 0.0;
            for i in 0..=1000 {
                let x = i as f32 / 1000.0;
                let s = solve_monotonic_bezier(x1, x2, x);
                assert!(s + 1e-6 >= prev, "x1={x1} x2={x2} x={x} s={s} prev={prev}");
                assert!(
                    close(cubic(0.0, x1, x2, 1.0, s), x, 1e-5),
                    "x1={x1} x2={x2} x={x}"
                );
                prev = s;
            }
        }
    }

    /// 区間外の制御点の時刻はクランプして扱う (`docs/spec/motion.md#カーブ`)。
    #[test]
    fn bezier_clamps_out_of_range_controls() {
        let raw = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Bezier {
                c1: p(-1.0, 0.0),
                c2: p(5.0, 1.0),
                to: p(1.0, 1.0),
            },
        ];
        let clamped = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Bezier {
                c1: p(0.0, 0.0),
                c2: p(1.0, 1.0),
                to: p(1.0, 1.0),
            },
        ];
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let a = evaluate(&raw, t).unwrap();
            let b = evaluate(&clamped, t).unwrap();
            assert!(close(a, b, 1e-6), "t={t}");
        }
    }

    #[test]
    fn zero_length_bezier_takes_end_value() {
        let segs = [
            Segment::Start { p: p(1.0, 0.0) },
            Segment::Bezier {
                c1: p(1.0, 0.0),
                c2: p(1.0, 0.0),
                to: p(1.0, 4.0),
            },
            Segment::Linear { to: p(2.0, 4.0) },
        ];
        assert_eq!(evaluate(&segs, 1.0), Some(0.0));
        assert_eq!(evaluate(&segs, 1.5), Some(4.0));
    }
}
