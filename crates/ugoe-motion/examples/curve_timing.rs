//! 長いモーションのカーブの評価にかかる時間を計測する (T0324、`docs/testing.md#性能の計測`)。
//!
//! `cargo run --release -p ugoe-motion --example curve_timing`
//!
//! フレームごとにキーを打ったモーション (30fps で 10 秒 = 1 カーブ 300 セグメント) を 100 カーブ分作り、
//! 60fps で 10 秒再生する。先頭から線形に探す `curve::evaluate` と、前処理して二分探索する
//! `SegmentIndex` (`MotionPlayer` が使う) の 1 フレームあたりの時間を比べる。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ugoe_model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe_motion::MotionPlayer;
use ugoe_motion::curve::{self, SegmentIndex};

const CURVES: usize = 100;
const KEY_FPS: f32 = 30.0;
const DURATION: f32 = 10.0;
const PLAY_FPS: f32 = 60.0;
const RUNS: usize = 5;

fn main() {
    let motion = long_motion();
    let frames = (DURATION * PLAY_FPS) as usize;
    let times: Vec<f32> = (0..frames).map(|i| i as f32 / PLAY_FPS).collect();
    let segments = motion.curves[0].segments.len();
    println!(
        "カーブ {CURVES} 本 × セグメント {segments}、{frames} フレーム ({PLAY_FPS}fps で {DURATION} 秒)"
    );

    let linear = best_of(|| {
        for &t in &times {
            for c in &motion.curves {
                black_box(curve::evaluate(black_box(&c.segments), t));
            }
        }
    });
    let indices: Vec<SegmentIndex> = motion
        .curves
        .iter()
        .map(|c| SegmentIndex::new(&c.segments))
        .collect();
    let indexed = best_of(|| {
        for &t in &times {
            for (c, index) in motion.curves.iter().zip(&indices) {
                black_box(index.evaluate(black_box(&c.segments), t));
            }
        }
    });
    let player = best_of(|| {
        let mut player = MotionPlayer::new(motion.clone());
        for _ in &times {
            player.advance(1.0 / PLAY_FPS);
            for value in player.curve_values() {
                black_box(value);
            }
        }
    });

    let per_frame = |d: Duration| d.as_secs_f64() * 1e6 / frames as f64;
    println!(
        "線形の探索 (curve::evaluate)      : {:8.2} µs/フレーム",
        per_frame(linear)
    );
    println!(
        "前処理した探索 (SegmentIndex)     : {:8.2} µs/フレーム",
        per_frame(indexed)
    );
    println!(
        "MotionPlayer (advance + 評価)     : {:8.2} µs/フレーム",
        per_frame(player)
    );
    println!(
        "比 (線形 / 前処理)                : {:8.2} 倍",
        linear.as_secs_f64() / indexed.as_secs_f64()
    );
}

/// `RUNS` 回測って最短の時間を返す。
fn best_of(mut f: impl FnMut()) -> Duration {
    (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .min()
        .unwrap_or_default()
}

/// フレームごとにキーを打ったモーション。セグメントの種類は直線とベジェを交互にする。
fn long_motion() -> Motion {
    let keys = (DURATION * KEY_FPS) as usize;
    let curves = (0..CURVES)
        .map(|c| {
            let value = |k: usize| ((k as f32 * 0.1) + c as f32).sin();
            let time = |k: usize| k as f32 / KEY_FPS;
            let mut segments = vec![Segment::Start {
                p: Point {
                    t: 0.0,
                    v: value(0),
                },
            }];
            segments.extend((1..=keys).map(|k| {
                let to = Point {
                    t: time(k),
                    v: value(k),
                };
                if k % 2 == 0 {
                    Segment::Linear { to }
                } else {
                    let (t0, t1) = (time(k - 1), time(k));
                    Segment::Bezier {
                        c1: Point {
                            t: t0 + (t1 - t0) / 3.0,
                            v: value(k - 1),
                        },
                        c2: Point {
                            t: t0 + (t1 - t0) * 2.0 / 3.0,
                            v: to.v,
                        },
                        to,
                    }
                }
            }));
            Curve {
                target: CurveTarget::Parameter(format!("Param{c}").as_str().into()),
                segments,
                fade_in: None,
                fade_out: None,
            }
        })
        .collect();
    Motion {
        name: "long".into(),
        duration: DURATION,
        looping: false,
        fade_in: 0.0,
        fade_out: 0.0,
        curves,
        events: vec![],
    }
}
