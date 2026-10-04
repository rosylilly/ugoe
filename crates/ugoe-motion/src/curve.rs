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
///
/// 先頭からセグメントを線形に探す。同じカーブを何度も評価するなら [`SegmentIndex`] を使う。
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
            return Some(segment_value(prev, seg, to, t));
        }
        prev = to;
    }
    Some(prev.v)
}

/// カーブ 1 本のセグメントを二分探索で探すための前処理。
///
/// [`evaluate`] は毎回先頭から線形に探すが、こちらは未知のセグメントの有無・2 つ目の `start` の位置・
/// 区間の終わりの時刻を前もって調べておき、`O(log n)` で区間を探す。結果は [`evaluate`] と同じ
/// (時刻が減るセグメントを含む、書き出しの規則に反するカーブでも同じ)。
///
/// 作ったときのセグメント列と組にして使う ([`evaluate`](Self::evaluate) に同じ列を渡す)。
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentIndex {
    /// カーブが値を持つか (セグメントがある・最初が `start`・未知のセグメントがない)。
    valid: bool,
    /// 評価に使うセグメントの数。2 つ目の `start` があればその位置、なければセグメントの数。
    len: usize,
    /// `segments[1..len]` の区間の終わりの時刻の累積最大値 (NaN は飛ばす)。
    ///
    /// 線形の探索は「`t <= to.t` となる最初の区間」を選ぶ。その区間より前の終わりの時刻はすべて `t` 未満なので、
    /// 累積最大値が初めて `t` 以上になる位置と一致する。累積最大値は単調なので二分探索できる。
    max_ends: Vec<f32>,
}

impl SegmentIndex {
    /// `segments` を前処理する。
    pub fn new(segments: &[Segment]) -> Self {
        let valid = matches!(segments.first(), Some(Segment::Start { .. }))
            && !segments.contains(&Segment::Unknown);
        if !valid {
            return Self {
                valid,
                len: 0,
                max_ends: Vec::new(),
            };
        }
        let len = segments[1..]
            .iter()
            .position(|s| matches!(s, Segment::Start { .. }))
            .map_or(segments.len(), |i| i + 1);
        let mut max = f32::NEG_INFINITY;
        let max_ends = segments[1..len]
            .iter()
            .map(|s| {
                // `f32::max` は NaN を捨てる。線形の探索でも NaN の終わりの時刻は選ばれない。
                max = max.max(end_point(s).t);
                max
            })
            .collect();
        Self {
            valid,
            len,
            max_ends,
        }
    }

    /// カーブを時刻 `t` で評価する。結果は [`evaluate`](crate::curve::evaluate) と同じ。
    ///
    /// `segments` は [`new`](Self::new) に渡したものと同じ列でなければならない。
    pub fn evaluate(&self, segments: &[Segment], t: f32) -> Option<f32> {
        if !self.valid {
            return None;
        }
        debug_assert!(self.len <= segments.len());
        let first = end_point(segments.first()?);
        if t <= first.t {
            return Some(first.v);
        }
        // `t <= m` を満たさない (NaN の `t` を含む) 間は探し続ける。
        let i = self.max_ends.partition_point(|&m| m < t || t.is_nan());
        if i == self.max_ends.len() {
            // どの区間にも入らない。2 つ目の `start` で終わるカーブは値を持たない。
            if self.len < segments.len() {
                return None;
            }
            return Some(end_point(segments.get(self.len - 1)?).v);
        }
        let prev = end_point(segments.get(i)?);
        let seg = segments.get(i + 1)?;
        Some(segment_value(prev, seg, end_point(seg), t))
    }
}

/// セグメントの終点 (`start` ならその点)。未知のセグメントは呼び出し側で除いておく。
fn end_point(seg: &Segment) -> Point {
    match seg {
        Segment::Start { p } => *p,
        Segment::Linear { to }
        | Segment::Bezier { to, .. }
        | Segment::Stepped { to }
        | Segment::InverseStepped { to } => *to,
        Segment::Unknown => Point {
            t: f32::NAN,
            v: f32::NAN,
        },
    }
}

/// 始点 `prev`・終点 `to` のセグメント `seg` の、時刻 `t` (`t <= to.t`) での値。
fn segment_value(prev: Point, seg: &Segment, to: Point, t: f32) -> f32 {
    let u = if to.t > prev.t {
        (t - prev.t) / (to.t - prev.t)
    } else {
        1.0
    };
    match seg {
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
        // `start` と未知のセグメントは区間にならない (呼び出し側で除く)。直線として扱っておく。
        Segment::Linear { .. } | Segment::Start { .. } | Segment::Unknown => {
            prev.v + (to.v - prev.v) * u
        }
    }
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

    /// セグメントがない・最初が `start` でないカーブは値を持たない (`docs/spec/motion.md#カーブ`)。
    #[test]
    fn empty_or_headless_curve_has_no_value() {
        assert_eq!(evaluate(&[], 0.0), None);
        let segs = [Segment::Linear { to: p(1.0, 1.0) }];
        assert_eq!(evaluate(&segs, 0.5), None);
    }

    /// 2 つ目以降の `start` より前は通常どおり評価し、その後は値を持たない (仕様の例)。
    #[test]
    fn second_start_ends_the_curve() {
        let segs = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Linear { to: p(1.0, 1.0) },
            Segment::Start { p: p(2.0, 5.0) },
            Segment::Linear { to: p(3.0, 6.0) },
        ];
        assert_eq!(evaluate(&segs, 0.5), Some(0.5));
        assert_eq!(evaluate(&segs, 1.0), Some(1.0));
        assert_eq!(evaluate(&segs, 1.5), None);
        assert_eq!(evaluate(&segs, 2.5), None);
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

    /// 線形の探索と前処理した探索の結果が、ビット単位で同じ (NaN どうしも同じとみなす)。
    fn assert_same(segs: &[Segment], t: f32) {
        let index = SegmentIndex::new(segs);
        let (a, b) = (evaluate(segs, t), index.evaluate(segs, t));
        assert_eq!(
            a.map(f32::to_bits),
            b.map(f32::to_bits),
            "t={t} linear={a:?} indexed={b:?} segs={segs:?}"
        );
    }

    /// 既存のテストのカーブで、前処理した探索が線形の探索と一致する。
    #[test]
    fn index_matches_linear_on_examples() {
        let curves: Vec<Vec<Segment>> = vec![
            vec![],
            vec![Segment::Linear { to: p(1.0, 1.0) }],
            vec![Segment::Start { p: p(0.5, 2.0) }],
            vec![
                Segment::Start { p: p(0.0, 0.0) },
                Segment::Linear { to: p(1.0, 10.0) },
                Segment::Stepped { to: p(2.0, 0.0) },
                Segment::InverseStepped { to: p(3.0, 4.0) },
            ],
            vec![
                Segment::Start { p: p(0.0, 0.0) },
                Segment::Linear { to: p(1.0, 10.0) },
                Segment::Unknown,
            ],
            vec![
                Segment::Start { p: p(0.0, 0.0) },
                Segment::Linear { to: p(1.0, 1.0) },
                Segment::Start { p: p(2.0, 5.0) },
                Segment::Linear { to: p(3.0, 6.0) },
            ],
            vec![
                Segment::Start { p: p(1.0, 0.0) },
                Segment::Bezier {
                    c1: p(1.0, 0.0),
                    c2: p(1.0, 0.0),
                    to: p(1.0, 4.0),
                },
                Segment::Linear { to: p(2.0, 4.0) },
            ],
        ];
        for segs in &curves {
            for i in -10..=40 {
                assert_same(segs, i as f32 * 0.1);
            }
            for t in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert_same(segs, t);
            }
        }
    }

    /// 時刻が減るセグメント (書き出しの規則に反する) を含むカーブでも、線形の探索と同じ区間を選ぶ。
    #[test]
    fn index_matches_linear_with_decreasing_times() {
        let segs = [
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Linear { to: p(2.0, 2.0) },
            Segment::Linear { to: p(1.0, 5.0) },
            Segment::Stepped { to: p(3.0, 7.0) },
            Segment::Linear { to: p(2.5, 1.0) },
        ];
        // t = 1.5 は 1 つ目の区間 (終わりが 2) に入り、時刻が戻った 2 つ目の区間は選ばない。
        assert_eq!(SegmentIndex::new(&segs).evaluate(&segs, 1.5), Some(1.5));
        assert_eq!(SegmentIndex::new(&segs).evaluate(&segs, 2.75), Some(5.0));
        assert_eq!(SegmentIndex::new(&segs).evaluate(&segs, 9.0), Some(1.0));
        for i in -5..=40 {
            assert_same(&segs, i as f32 * 0.1);
        }
    }

    /// 乱数で作ったカーブ (時刻が前後する・NaN の点・2 つ目の `start`・未知のセグメントを含む) で、
    /// 前処理した探索が線形の探索と一致する。
    #[test]
    fn index_matches_linear_on_random_curves() {
        // SplitMix64。テストを決定的にするため固定のシードを使う。
        let mut state = 0x5eed_u64;
        let mut next = move || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let mut unit = move || (next() >> 40) as f32 / (1u64 << 24) as f32;
        for _ in 0..500 {
            let n = (unit() * 12.0) as usize;
            let mut t = unit() * 2.0 - 1.0;
            let mut point = |unit: &mut dyn FnMut() -> f32| {
                // おおむね増えるが、ときどき戻る。まれに NaN にする。
                t += unit() * 1.2 - 0.2;
                let pt = if unit() < 0.03 { f32::NAN } else { t };
                p(pt, unit() * 10.0 - 5.0)
            };
            let mut segs = vec![Segment::Start {
                p: point(&mut unit),
            }];
            for _ in 0..n {
                let kind = unit();
                segs.push(if kind < 0.02 {
                    Segment::Unknown
                } else if kind < 0.06 {
                    Segment::Start {
                        p: point(&mut unit),
                    }
                } else if kind < 0.35 {
                    Segment::Linear {
                        to: point(&mut unit),
                    }
                } else if kind < 0.6 {
                    let (c1, c2) = (point(&mut unit), point(&mut unit));
                    Segment::Bezier {
                        c1,
                        c2,
                        to: point(&mut unit),
                    }
                } else if kind < 0.8 {
                    Segment::Stepped {
                        to: point(&mut unit),
                    }
                } else {
                    Segment::InverseStepped {
                        to: point(&mut unit),
                    }
                });
            }
            for i in -20..=200 {
                assert_same(&segs, i as f32 * 0.05);
            }
            // 点の時刻ちょうども試す。
            for seg in &segs[..] {
                if *seg != Segment::Unknown {
                    assert_same(&segs, end_point(seg).t);
                }
            }
        }
    }
}
