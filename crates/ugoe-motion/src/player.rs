//! モーション 1 本の再生 (時刻・フェード・ループ・イベント)。
//! 仕様: `docs/spec/motion.md#再生とフェード`。

use ugoe_core::math;
use ugoe_model::motion::{Curve, CurveTarget, Motion, MotionEvent};
use ugoe_runtime::ParameterValues;

use crate::curve::SegmentIndex;
use crate::resolve::ResolvedParams;

/// 1 回の [`MotionPlayer::advance`] で扱うループの周回の上限。巨大な `dt` や極端に短い長さで止まらないようにする。
pub(crate) const MAX_LAPS_PER_ADVANCE: u32 = 1000;

/// フェードの曲線 `0.5 - 0.5 cos(π x)`。`x` は `[0, 1]` にクランプする。
pub fn easing(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    0.5 - 0.5 * math::cos(std::f32::consts::PI * x)
}

/// フェードイン (経過時間 `elapsed`、長さ `duration`) の重み。長さが 0 以下なら 1。
fn fade_in_weight(elapsed: f32, duration: f32) -> f32 {
    if duration > 0.0 {
        easing(elapsed / duration)
    } else {
        1.0
    }
}

/// フェードアウト (終了時刻までの残り `remaining`、長さ `duration`) の重み。
/// 長さが 0 以下なら、終了時刻までは 1、以降は 0。
fn fade_out_weight(remaining: f32, duration: f32) -> f32 {
    if duration > 0.0 {
        easing(remaining / duration)
    } else if remaining > 0.0 {
        1.0
    } else {
        0.0
    }
}

/// 経過時間 `elapsed`、終了時刻 `end` での `fade_in_weight * fade_out_weight`。
///
/// 経過時間は `f64` で持つ (長時間動かしても `dt` を足せなくならない)。差を取ってから `f32` にする。
pub(crate) fn fade_weight(fade_in: f32, fade_out: f32, elapsed: f64, end: Option<f64>) -> f32 {
    let w_out = end.map_or(1.0, |end| fade_out_weight((end - elapsed) as f32, fade_out));
    fade_in_weight(elapsed.min(f64::from(f32::MAX)) as f32, fade_in) * w_out
}

/// 経過時間を `dt` 進める。終了時刻があれば越えない。`dt` が正の有限値でなければ何もしない。
///
/// 進めた時間 (終了時刻で切ったもの) を返す。
pub(crate) fn advance_clock(elapsed: &mut f64, end: Option<f64>, dt: f32) -> f32 {
    if !(dt > 0.0 && dt.is_finite()) {
        return 0.0;
    }
    match end {
        Some(end) if *elapsed + f64::from(dt) >= end => {
            let dt = (end - *elapsed).max(0.0) as f32;
            *elapsed = end;
            dt
        }
        _ => {
            *elapsed += f64::from(dt);
            dt
        }
    }
}

/// カーブ個別のフェード (未指定ならモーション全体の値) を使った重み。
fn curve_fade_weight(motion: &Motion, curve: &Curve, elapsed: f64, end: Option<f64>) -> f32 {
    fade_weight(
        curve.fade_in.unwrap_or(motion.fade_in),
        curve.fade_out.unwrap_or(motion.fade_out),
        elapsed,
        end,
    )
}

/// モーションの再生状態 1 本分。
///
/// 時刻は 2 つ持つ。
///
/// - 経過時間 ([`elapsed`](Self::elapsed)): 再生開始からの時間。フェードに使う。
/// - 再生位置 ([`time`](Self::time)): モーション内の時刻。ループすると `duration` で 0 に戻る。
///
/// ループしないモーションは `duration` で終わり、その `fade_out` 秒前からフェードアウトする。
/// [`stop`](Self::stop) を呼ぶと、その時点から `fade_out` 秒かけてフェードアウトして終わる
/// (モーションマネージャが新しいモーションとクロスフェードするときに使う)。
#[derive(Debug)]
pub struct MotionPlayer {
    motion: Motion,
    /// `motion.events` の添字を時刻の順に並べたもの。
    event_order: Vec<usize>,
    /// 各カーブのセグメントの探索の前処理 (`motion.curves` と同じ順)。毎フレーム先頭から探さないため。
    segment_indices: Vec<SegmentIndex>,
    /// 各カーブの対象パラメータの添字 (適用時に解決し、パラメータの並びが変わったら解決し直す)。
    param_indices: ResolvedParams,
    time: f32,
    /// 経過時間。長時間ループさせても `dt` を足せるように `f64` で持つ。
    elapsed: f64,
    /// 終了する経過時間。`None` なら終わらない (ループ中)。
    end: Option<f64>,
    /// 今の周回が始まった経過時間。ループするモーションの再生位置は `elapsed - lap_start` (T0332)。
    lap_start: f64,
    /// 次の [`advance`](Self::advance) で、再生位置ちょうどのイベントも発火するか
    /// (再生の開始直後とループの折り返し直後)。
    inclusive_start: bool,
    /// 直前の [`advance`](Self::advance) で発火したイベント (`motion.events` の添字)。
    fired: Vec<usize>,
}

impl MotionPlayer {
    /// `motion` を時刻 0 から再生するプレイヤーを作る。ループしないモーションは `duration` で終わる。
    pub fn new(motion: Motion) -> Self {
        let mut event_order: Vec<usize> = (0..motion.events.len()).collect();
        event_order.sort_by(|&a, &b| motion.events[a].t.total_cmp(&motion.events[b].t));
        let end = (!motion.looping).then_some(f64::from(motion.duration.max(0.0)));
        let segment_indices = motion
            .curves
            .iter()
            .map(|c| SegmentIndex::new(&c.segments))
            .collect();
        Self {
            motion,
            event_order,
            segment_indices,
            param_indices: ResolvedParams::default(),
            time: 0.0,
            elapsed: 0.0,
            end,
            lap_start: 0.0,
            inclusive_start: true,
            fired: Vec::new(),
        }
    }

    /// 再生しているモーション。
    pub fn motion(&self) -> &Motion {
        &self.motion
    }

    /// モーション内の再生位置 (秒)。ループするモーションでは `[0, duration)`。
    pub fn time(&self) -> f32 {
        self.time
    }

    /// 再生開始からの経過時間 (秒)。ループしても戻らない。
    pub fn elapsed(&self) -> f32 {
        self.elapsed as f32
    }

    /// 再生が終わった (フェードアウトし切った) か。終わったプレイヤーは何も適用しない。
    pub fn is_finished(&self) -> bool {
        self.end.is_some_and(|end| self.elapsed >= end)
    }

    /// 今の時点から `fade_out` 秒かけてフェードアウトして終わる。
    ///
    /// 既に決まっている終了時刻のほうが早ければ、そちらを保つ。
    pub fn stop(&mut self) {
        let end = self.elapsed + f64::from(self.motion.fade_out.max(0.0));
        self.end = Some(self.end.map_or(end, |e| e.min(end)));
    }

    /// モーション全体のフェードの重み (`fade_in_weight * fade_out_weight`)。
    pub fn fade_weight(&self) -> f32 {
        fade_weight(
            self.motion.fade_in,
            self.motion.fade_out,
            self.elapsed,
            self.end,
        )
    }

    /// `dt` 秒進め、`weight` を掛けてパラメータへ適用する。
    ///
    /// フェードの重みはプレイヤー自身が掛ける。`weight` はそれに加えて掛ける外部の重み
    /// (モーションマネージャやアプリが使う。普通は 1)。
    pub fn update(&mut self, parameters: &mut ParameterValues, dt: f32, weight: f32) {
        self.advance(dt);
        self.apply(parameters, weight);
    }

    /// `dt` 秒進める。通過したイベントは [`fired_events`](Self::fired_events) で得られる。
    ///
    /// `dt` が 0 以下か有限値でない (NaN・無限大) ときは何もしない。
    pub fn advance(&mut self, dt: f32) {
        self.fired.clear();
        if self.is_finished() {
            return;
        }
        // 終了時刻を越えて再生位置やイベントを進めない。
        let dt = advance_clock(&mut self.elapsed, self.end, dt);
        if dt <= 0.0 {
            return;
        }

        let duration = self.motion.duration;
        // NaN の長さも 0 以下と同じに扱う (ループさせると周回が終わらない)。
        if !self.motion.looping || duration.is_nan() || duration <= 0.0 {
            // 再生位置は経過時間 (終了時刻で切ったもの) と同じ。`f32` の再生位置に `dt` を足していくと、
            // 丸めで長さに届かないまま終わり、t = 長さのイベントを逃すことがあったので、`f64` の経過時間から求める (T0331)。
            let to = self.elapsed.min(f64::from(duration.max(0.0))) as f32;
            self.fire_range(self.time, to);
            self.time = to;
            return;
        }

        // 再生位置は周回の始まりからの経過時間の差。`f32` の再生位置に `dt` を足していくと、`stop()` の
        // 終わりが周回の終わりに重なるときに丸めで周回の終わりに届かず、t = 長さのイベントを逃した (T0332)。
        let lap = f64::from(duration);
        for _ in 0..MAX_LAPS_PER_ADVANCE {
            let to = (self.elapsed - self.lap_start).max(0.0) as f32;
            if to < duration {
                self.fire_range(self.time, to);
                self.time = to;
                return;
            }
            // 折り返す。周回の終わりまでのイベントを発火し、次の周回は 0 から始める。
            self.fire_range(self.time, duration);
            self.lap_start += lap;
            self.time = 0.0;
            self.inclusive_start = true;
            if self.elapsed <= self.lap_start {
                return;
            }
        }
        // 周回の上限を超えた分は発火せず、再生位置だけを合わせる。巨大な経過時間に周回の長さを
        // 足し続けても桁落ちで変わらない (極端に短い長さでも同じ) ので、剰余で [0, duration) に収める。
        let time = math::rem_euclid_f64(self.elapsed - self.lap_start, lap);
        self.lap_start = self.elapsed - time;
        let time = (self.elapsed - self.lap_start).max(0.0) as f32;
        if time < duration {
            self.time = time;
        } else {
            // 桁落ちで差が周回の長さに届くときは、今を周回の始まりにする (次の `advance` で周回を回し続けない)。
            self.lap_start = self.elapsed;
            self.time = 0.0;
        }
    }

    /// 直前の [`advance`](Self::advance) (または [`update`](Self::update)) で発火したイベント。
    /// 発火した順 (ループをまたいだ場合は周回の順、周回の中では時刻の順) に並ぶ。
    pub fn fired_events(&self) -> impl Iterator<Item = &MotionEvent> {
        self.fired.iter().map(|&i| &self.motion.events[i])
    }

    /// 現在の時刻でカーブを評価し、パラメータを対象とするカーブを適用する。
    ///
    /// 適用は `value = lerp(current, curve_value, w)`。`w` はフェードの重みに `weight` を掛けたもの。
    /// パラメータ以外の対象 (パーツの不透明度・エフェクトグループ) は
    /// [`curve_values`](Self::curve_values) で取り出して呼び出し側が扱う。
    pub fn apply(&mut self, parameters: &mut ParameterValues, weight: f32) {
        if self.is_finished() {
            return;
        }
        let indices = self.param_indices.get(
            parameters,
            self.motion.curves.iter().map(|c| match &c.target {
                CurveTarget::Parameter(id) => Some(id.as_str()),
                _ => None,
            }),
        );
        for ((curve, index), lookup) in self
            .motion
            .curves
            .iter()
            .zip(indices)
            .zip(&self.segment_indices)
        {
            let Some(index) = *index else { continue };
            let Some(value) = lookup.evaluate(&curve.segments, self.time) else {
                continue;
            };
            let w = weight * curve_fade_weight(&self.motion, curve, self.elapsed, self.end);
            parameters.blend(index, value, w);
        }
    }

    /// 現在の時刻での各カーブの `(対象, 値, フェードの重み)`。評価できないカーブは含めない。
    ///
    /// 重みには外部の重みを含めない。終わったプレイヤーは何も返さない。
    pub fn curve_values(&self) -> impl Iterator<Item = (&CurveTarget, f32, f32)> {
        self.curve_values_where(|target| *target != CurveTarget::Unknown)
            .map(|(_, target, value, w)| (target, value, w))
    }

    /// [`curve_values`](Self::curve_values) のうち、対象が `filter` を満たすカーブだけ。先頭はカーブの添字
    /// (`motion.curves` の)。評価の前に対象で絞るので、ほかの対象のカーブは評価しない
    /// (モーションマネージャが毎フレーム使う)。
    pub(crate) fn curve_values_where(
        &self,
        filter: impl Fn(&CurveTarget) -> bool,
    ) -> impl Iterator<Item = (usize, &CurveTarget, f32, f32)> {
        let finished = self.is_finished();
        self.motion
            .curves
            .iter()
            .zip(&self.segment_indices)
            .enumerate()
            .filter(move |(_, (c, _))| !finished && filter(&c.target))
            .filter_map(|(i, (c, lookup))| {
                let value = lookup.evaluate(&c.segments, self.time)?;
                let w = curve_fade_weight(&self.motion, c, self.elapsed, self.end);
                Some((i, &c.target, value, w))
            })
    }

    /// 再生位置 `from` から `to` までに通過したイベントを `fired` に積む。
    /// 範囲は `(from, to]` (開始直後・折り返し直後は `[from, to]`)。
    fn fire_range(&mut self, from: f32, to: f32) {
        let inclusive = std::mem::take(&mut self.inclusive_start);
        for &i in &self.event_order {
            let t = self.motion.events[i].t;
            let after_from = if inclusive { t >= from } else { t > from };
            if after_from && t <= to {
                self.fired.push(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;
    use ugoe_model::motion::{Point, Segment};

    use ugoe_runtime::ParamIndex;

    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    fn param_curve(id: &str, segments: Vec<Segment>) -> Curve {
        Curve {
            target: CurveTarget::Parameter(id.into()),
            segments,
            fade_in: None,
            fade_out: None,
        }
    }

    /// `v` を保つカーブ。
    fn constant(id: &str, v: f32) -> Curve {
        param_curve(
            id,
            vec![Segment::Start {
                p: Point { t: 0.0, v },
            }],
        )
    }

    /// 時刻 0 で 0、`t` で `v` になる直線のカーブ。
    fn ramp(id: &str, t: f32, v: f32) -> Curve {
        param_curve(
            id,
            vec![
                Segment::Start {
                    p: Point { t: 0.0, v: 0.0 },
                },
                Segment::Linear { to: Point { t, v } },
            ],
        )
    }

    fn motion(duration: f32, looping: bool, curves: Vec<Curve>) -> Motion {
        Motion {
            name: "test".into(),
            duration,
            looping,
            fade_in: 0.0,
            fade_out: 0.0,
            curves,
            events: vec![],
        }
    }

    fn event(t: f32, value: &str) -> MotionEvent {
        MotionEvent {
            t,
            value: value.into(),
        }
    }

    /// `single_quad` のパラメータ `X` (値域 -1〜1、既定値 0)。
    fn params() -> ParameterValues {
        ParameterValues::from_model(&fixtures::single_quad())
    }

    /// 既定値 0 のパラメータに適用した結果 (= 値 1 のカーブなら重みそのもの)。
    fn applied(player: &mut MotionPlayer, weight: f32) -> f32 {
        let mut p = params();
        player.apply(&mut p, weight);
        p.get(p.index_of("X").unwrap())
    }

    fn fired(player: &MotionPlayer) -> Vec<&str> {
        player.fired_events().map(|e| e.value.as_str()).collect()
    }

    #[test]
    fn easing_curve() {
        assert_eq!(easing(0.0), 0.0);
        assert!(close(easing(0.5), 0.5));
        assert!(close(easing(1.0), 1.0));
        assert!(close(
            easing(0.25),
            0.5 - 0.5 * math::cos(std::f32::consts::FRAC_PI_4)
        ));
        assert_eq!(easing(-1.0), 0.0);
        assert!(close(easing(2.0), 1.0));
    }

    #[test]
    fn applies_curve_value_to_parameter() {
        let mut player = MotionPlayer::new(motion(2.0, false, vec![ramp("X", 2.0, 1.0)]));
        let mut p = params();
        player.update(&mut p, 0.5, 1.0);
        assert!(close(p.get(ParamIndex(0)), 0.25));
        player.update(&mut p, 0.5, 1.0);
        assert!(close(p.get(ParamIndex(0)), 0.5));
    }

    /// `docs/spec/motion.md#再生とフェード`: フェードインの重みは easing(elapsed / fade_in)。
    #[test]
    fn fade_in_uses_easing() {
        let mut m = motion(10.0, false, vec![constant("X", 1.0)]);
        m.fade_in = 1.0;
        let mut player = MotionPlayer::new(m);
        assert!(close(applied(&mut player, 1.0), 0.0));
        for step in 1..=5 {
            player.advance(0.25);
            let expected = easing(step as f32 * 0.25);
            assert!(close(player.fade_weight(), expected), "step={step}");
            assert!(close(applied(&mut player, 1.0), expected), "step={step}");
        }
    }

    /// ループしないモーションは、終わりの `fade_out` 秒前からフェードアウトして終わる。
    #[test]
    fn non_looping_motion_fades_out_at_end() {
        let mut m = motion(2.0, false, vec![constant("X", 1.0)]);
        m.fade_out = 1.0;
        let mut player = MotionPlayer::new(m);
        player.advance(1.0);
        assert!(close(applied(&mut player, 1.0), 1.0));
        player.advance(0.5);
        assert!(close(applied(&mut player, 1.0), 0.5));
        assert!(!player.is_finished());
        player.advance(0.75);
        assert!(player.is_finished());
        assert!(close(player.time(), 2.0));
        assert!(close(player.elapsed(), 2.0));
        assert_eq!(applied(&mut player, 1.0), 0.0);
        assert_eq!(player.curve_values().count(), 0);
    }

    /// `stop` はその時点から `fade_out` 秒かけてフェードアウトする (クロスフェード用)。
    #[test]
    fn stop_fades_out_from_now() {
        let mut m = motion(1.0, true, vec![constant("X", 1.0)]);
        m.fade_out = 1.0;
        let mut player = MotionPlayer::new(m);
        player.advance(3.3);
        assert!(close(player.fade_weight(), 1.0));
        player.stop();
        player.advance(0.25);
        assert!(close(player.fade_weight(), easing(0.75)));
        // 2 回目の stop は終了時刻を延ばさない。
        player.stop();
        player.advance(0.5);
        assert!(close(player.fade_weight(), easing(0.25)));
        player.advance(1.0);
        assert!(player.is_finished());
        assert!(close(player.elapsed(), 4.3));
    }

    #[test]
    fn stop_without_fade_out_finishes_immediately() {
        let mut player = MotionPlayer::new(motion(1.0, true, vec![constant("X", 1.0)]));
        player.advance(0.5);
        player.stop();
        assert!(player.is_finished());
        assert_eq!(applied(&mut player, 1.0), 0.0);
    }

    /// カーブ個別のフェードはモーション全体の値の代わりに使う。
    #[test]
    fn curve_fade_overrides_motion_fade() {
        let mut own = constant("X", 1.0);
        own.fade_in = Some(2.0);
        let mut m = motion(10.0, false, vec![own]);
        m.fade_in = 1.0;
        let mut player = MotionPlayer::new(m);
        player.advance(1.0);
        assert!(close(player.fade_weight(), 1.0));
        assert!(close(applied(&mut player, 1.0), 0.5));
    }

    #[test]
    fn external_weight_multiplies_fade() {
        let mut m = motion(10.0, false, vec![constant("X", 1.0)]);
        m.fade_in = 1.0;
        let mut player = MotionPlayer::new(m);
        player.advance(0.5);
        assert!(close(applied(&mut player, 0.5), 0.25));
    }

    /// ループは `duration` で折り返し、境目ではフェードをかけない。
    #[test]
    fn looping_wraps_without_fade() {
        let mut m = motion(1.0, true, vec![ramp("X", 1.0, 1.0)]);
        m.fade_in = 0.5;
        m.fade_out = 0.5;
        let mut player = MotionPlayer::new(m);
        player.advance(2.25);
        assert!(close(player.time(), 0.25));
        assert!(close(player.elapsed(), 2.25));
        assert!(close(player.fade_weight(), 1.0));
        assert!(close(applied(&mut player, 1.0), 0.25));
        // 折り返しの直前・直後もフェードの重みは 1 のまま。
        player.advance(0.74);
        assert!(close(player.fade_weight(), 1.0));
        player.advance(0.02);
        assert!(close(player.time(), 0.01));
        assert!(close(player.fade_weight(), 1.0));
        assert!(!player.is_finished());
    }

    /// 周回の終わりちょうどで止まると、再生位置は 0 に戻る。
    #[test]
    fn looping_exact_boundary_wraps_to_zero() {
        let mut player = MotionPlayer::new(motion(1.0, true, vec![]));
        player.advance(0.5);
        player.advance(0.5);
        assert_eq!(player.time(), 0.0);
    }

    /// 長さ 0 のループしないモーションは、開始した時点で終わる (時刻 0 のイベントも発火しない)。
    #[test]
    fn zero_length_motion_finishes_at_start() {
        let mut m = motion(0.0, false, vec![constant("X", 1.0)]);
        m.events = vec![event(0.0, "zero")];
        let mut player = MotionPlayer::new(m);
        assert!(player.is_finished());
        player.advance(0.1);
        assert!(fired(&player).is_empty());
        assert_eq!(applied(&mut player, 1.0), 0.0);
    }

    #[test]
    fn events_fire_when_passed() {
        let mut m = motion(2.0, false, vec![]);
        // 時刻の順に並んでいなくてもよい。
        m.events = vec![event(1.0, "b"), event(0.0, "a"), event(2.0, "c")];
        let mut player = MotionPlayer::new(m);
        player.advance(0.5);
        assert_eq!(fired(&player), ["a"]);
        player.advance(0.4);
        assert!(fired(&player).is_empty());
        player.advance(0.1);
        assert_eq!(fired(&player), ["b"]);
        // 終わりの時刻ちょうどのイベントも発火し、その後は何も起きない。
        player.advance(5.0);
        assert_eq!(fired(&player), ["c"]);
        player.advance(1.0);
        assert!(fired(&player).is_empty());
    }

    /// `docs/spec/motion.md#再生とフェード`: ループをまたいでも漏らさず発火する。
    #[test]
    fn events_fire_across_loop_boundary() {
        let mut m = motion(1.0, true, vec![]);
        m.events = vec![event(0.0, "start"), event(0.5, "mid"), event(1.0, "end")];
        let mut player = MotionPlayer::new(m);
        player.advance(0.25);
        assert_eq!(fired(&player), ["start"]);
        player.advance(0.5);
        assert_eq!(fired(&player), ["mid"]);
        player.advance(0.5);
        assert_eq!(fired(&player), ["end", "start"]);
        // 1 フレームで複数周回しても、周回ごとにすべて発火する。
        player.advance(2.0);
        assert_eq!(
            fired(&player),
            ["mid", "end", "start", "mid", "end", "start"]
        );
        assert!(close(player.time(), 0.25));
    }

    /// 60fps で 10 秒ループさせると、周回ごとにちょうど 1 回ずつ発火する。
    #[test]
    fn events_fire_once_per_lap_at_frame_rate() {
        let mut m = motion(1.0, true, vec![]);
        m.events = vec![event(0.3, "e"), event(0.0, "zero")];
        let mut player = MotionPlayer::new(m);
        let (mut e, mut zero) = (0, 0);
        for _ in 0..600 {
            player.advance(1.0 / 60.0);
            e += player.fired_events().filter(|ev| ev.value == "e").count();
            zero += player
                .fired_events()
                .filter(|ev| ev.value == "zero")
                .count();
        }
        assert_eq!(e, 10);
        // 開始時の 1 回 + 折り返し 9〜10 回 (累積誤差で 10 秒ちょうどの折り返しが次の枠にずれうる)。
        assert!((10..=11).contains(&zero), "zero={zero}");
    }

    /// 長さを割り切れない刻みで再生しても、t = 長さのイベントは終わる前の `advance` でちょうど 1 回発火する
    /// (再生位置を `f32` で足していくと、丸めで長さに届かないまま終わることがあった。T0331)。
    #[test]
    fn end_event_fires_once_regardless_of_rounding() {
        let dts = [1.0 / 60.0, 1.0 / 30.0, 1.0 / 144.0, 0.1, 0.007, 0.0333];
        let durations = [0.7, 1.0, 1.3, 1.5, 2.2, 3.1, 4.7, 10.3];
        for duration in durations {
            for dt in dts {
                let mut m = motion(duration, false, vec![]);
                m.events = vec![event(duration, "end")];
                let mut player = MotionPlayer::new(m);
                let mut count = 0;
                let mut finished_at = None;
                for frame in 0..100_000 {
                    player.advance(dt);
                    count += fired(&player).len();
                    if player.is_finished() {
                        finished_at = Some(frame);
                        break;
                    }
                }
                assert!(finished_at.is_some(), "duration={duration} dt={dt}");
                assert_eq!(count, 1, "duration={duration} dt={dt}");
                assert_eq!(player.time(), duration, "duration={duration} dt={dt}");
                player.advance(dt);
                assert!(fired(&player).is_empty(), "duration={duration} dt={dt}");
            }
        }
    }

    /// ループするモーションでも、t = 長さのイベントは周回ごとにちょうど 1 回発火する (T0331)。
    #[test]
    fn loop_end_event_fires_once_per_lap_regardless_of_rounding() {
        let dts = [1.0 / 60.0, 1.0 / 30.0, 1.0 / 144.0, 0.1, 0.007, 0.0333];
        let durations = [0.7, 1.0, 1.3, 1.5, 2.2, 3.1];
        for duration in durations {
            for dt in dts {
                let mut m = motion(duration, true, vec![]);
                m.events = vec![event(duration, "end"), event(0.0, "zero")];
                let mut player = MotionPlayer::new(m);
                let (mut end, mut zero) = (0, 0);
                let mut laps = 0;
                let mut prev = player.time();
                // 10 周より少し手前まで進める (周回の境目の丸めで数がぶれないように、境目から離れたところで止める)。
                let frames = ((10.0 * f64::from(duration) - f64::from(duration) / 2.0)
                    / f64::from(dt)) as usize;
                for _ in 0..frames {
                    player.advance(dt);
                    let names = fired(&player);
                    end += names.iter().filter(|n| **n == "end").count();
                    zero += names.iter().filter(|n| **n == "zero").count();
                    if player.time() < prev {
                        laps += 1;
                    }
                    prev = player.time();
                }
                assert_eq!(laps, 9, "duration={duration} dt={dt}");
                assert_eq!(end, laps, "duration={duration} dt={dt}");
                assert_eq!(zero, laps + 1, "duration={duration} dt={dt}");
            }
        }
    }

    /// ループするモーションを止め、終わりがちょうど周回の終わりに重なっても、t = 長さのイベントは
    /// 終わる前にちょうど 1 回発火する (再生位置を `f32` で足していくと、丸めで周回の終わりに届かないまま終わった。T0332)。
    #[test]
    fn stopped_loop_end_event_fires_when_end_meets_lap_end() {
        let dts = [1.0 / 60.0, 1.0 / 30.0, 1.0 / 144.0, 0.1, 0.007, 0.0333];
        let durations = [0.7, 1.0, 1.3, 1.5, 2.2, 3.1, 4.7, 10.3];
        for duration in durations {
            for dt in dts {
                let mut m = motion(duration, true, vec![]);
                m.fade_out = duration;
                m.events = vec![event(duration, "end")];
                let mut player = MotionPlayer::new(m);
                player.stop();
                let mut count = 0;
                let mut finished = false;
                for _ in 0..100_000 {
                    player.advance(dt);
                    count += fired(&player).len();
                    if player.is_finished() {
                        finished = true;
                        break;
                    }
                }
                assert!(finished, "duration={duration} dt={dt}");
                assert_eq!(count, 1, "duration={duration} dt={dt}");
            }
        }
    }

    #[test]
    fn stopped_looping_motion_fires_events_until_end() {
        let mut m = motion(1.0, true, vec![]);
        m.fade_out = 0.5;
        m.events = vec![event(0.2, "early"), event(0.8, "late")];
        let mut player = MotionPlayer::new(m);
        player.advance(0.1);
        player.stop();
        player.advance(10.0);
        assert_eq!(fired(&player), ["early"]);
        assert!(close(player.time(), 0.6));
        assert!(player.is_finished());
    }

    /// NaN や無限大の `dt` では何も進めない (ループするモーションで止まらず、ループしないモーションを終わらせない)。
    #[test]
    fn non_finite_dt_does_nothing() {
        for looping in [false, true] {
            let mut player = MotionPlayer::new(motion(2.0, looping, vec![]));
            player.advance(0.5);
            for dt in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                player.advance(dt);
                assert!(close(player.time(), 0.5), "looping={looping} dt={dt}");
                assert!(close(player.elapsed(), 0.5), "looping={looping} dt={dt}");
                assert!(!player.is_finished(), "looping={looping} dt={dt}");
            }
        }
    }

    /// 巨大な `dt`・極端に短い長さ・NaN の長さのループでも、1 回の `advance` が終わる。
    #[test]
    fn looping_advance_terminates_for_extreme_values() {
        for (duration, dt) in [
            (1.0, 1e30),
            (1.0, f32::MAX),
            (1e-30, 1.0 / 60.0),
            (f32::MIN_POSITIVE, 1.0),
            (f32::NAN, 1.0 / 60.0),
        ] {
            let mut m = motion(duration, true, vec![]);
            m.events = vec![event(0.0, "zero")];
            let mut player = MotionPlayer::new(m);
            player.advance(dt);
            let time = player.time();
            assert!(
                time.is_finite() && time >= 0.0,
                "duration={duration} dt={dt} time={time}"
            );
            if duration > 0.0 {
                assert!(time < duration, "duration={duration} dt={dt} time={time}");
            }
            assert!(fired(&player).len() <= MAX_LAPS_PER_ADVANCE as usize + 1);
            // その後も止まらず、1 回の `advance` で扱う周回は上限まで。
            for _ in 0..3 {
                player.advance(1.0 / 60.0);
                let time = player.time();
                assert!(time.is_finite(), "duration={duration} dt={dt} time={time}");
                if duration > 0.0 {
                    assert!(time < duration, "duration={duration} dt={dt} time={time}");
                }
                assert!(fired(&player).len() <= MAX_LAPS_PER_ADVANCE as usize + 1);
            }
        }
    }

    /// 長時間ループした後でも、再生位置は `dt` ずつ進み、周回ごとにイベントが発火する
    /// (再生位置を周回の始まりの経過時間 (`f64`) との差で求めても桁落ちしない。T0332)。
    #[test]
    fn looping_keeps_playing_after_a_long_run() {
        let mut m = motion(10.0, true, vec![]);
        m.events = vec![event(0.0, "zero")];
        let mut player = MotionPlayer::new(m);
        // 約 12 日 (周回の上限を超えるので、剰余で再生位置を合わせる)。1_048_576 = 10 * 104_857 + 6。
        player.advance(1_048_576.0);
        assert!(close(player.time(), 6.0), "{}", player.time());
        let mut zero = 0;
        for _ in 0..(5 * 60) {
            player.advance(1.0 / 60.0);
            zero += fired(&player).len();
        }
        assert!((player.time() - 1.0).abs() < 1e-4, "{}", player.time());
        assert_eq!(zero, 1);
    }

    /// 長時間ループした後でも、`stop` から `fade_out` 秒で終わる (経過時間の精度が落ちて止まらない)。
    #[test]
    fn stop_finishes_after_a_long_run() {
        let mut m = motion(10.0, true, vec![constant("X", 1.0)]);
        m.fade_out = 0.5;
        let mut player = MotionPlayer::new(m);
        // 約 12 日。f32 の経過時間では 1/60 秒を足しても変わらなくなる。
        player.advance(1_048_576.0);
        player.stop();
        player.advance(0.25);
        assert!(
            close(player.fade_weight(), easing(0.5)),
            "{}",
            player.fade_weight()
        );
        for _ in 0..16 {
            player.advance(1.0 / 60.0);
        }
        assert!(player.is_finished());
    }

    /// T0325: 並びの違う `ParameterValues` を渡すと添字を解決し直し、同じ ID のパラメータに書く。
    #[test]
    fn resolves_parameters_again_when_layout_changes() {
        let mut player = MotionPlayer::new(motion(10.0, false, vec![constant("X", 0.5)]));
        let mut p = params();
        player.apply(&mut p, 1.0);
        assert!(close(p.get(ParamIndex(0)), 0.5));

        // 先頭に `Y` を足した並び (`X` は 1 番目)。
        let mut model = fixtures::single_quad();
        let mut y = model.parameters[0].clone();
        y.id = "Y".into();
        model.parameters.insert(0, y);
        let mut shifted = ParameterValues::from_model(&model);
        player.apply(&mut shifted, 1.0);
        assert!(close(shifted.get(shifted.index_of("X").unwrap()), 0.5));
        assert_eq!(shifted.get(shifted.index_of("Y").unwrap()), 0.0);

        // 元の並びに戻しても正しく書く。
        let mut p = params();
        player.apply(&mut p, 1.0);
        assert!(close(p.get(ParamIndex(0)), 0.5));
    }

    /// パラメータ以外の対象は `curve_values` で取り出す。未知の対象と評価できないカーブは除く。
    #[test]
    fn curve_values_include_non_parameter_targets() {
        let mut opacity = constant("_", 0.5);
        opacity.target = CurveTarget::PartOpacity("ArmA".into());
        let mut blink = constant("_", 0.25);
        blink.target = CurveTarget::EffectGroup("EyeBlink".into());
        blink.fade_in = Some(1.0);
        let mut unknown = constant("_", 1.0);
        unknown.target = CurveTarget::Unknown;
        let broken = param_curve("X", vec![Segment::Unknown]);
        let mut player = MotionPlayer::new(motion(
            5.0,
            false,
            vec![opacity, blink, unknown, broken, constant("Missing", 1.0)],
        ));
        player.advance(0.5);
        let values: Vec<_> = player.curve_values().collect();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0].0, &CurveTarget::PartOpacity("ArmA".into()));
        assert_eq!((values[0].1, values[0].2), (0.5, 1.0));
        assert_eq!(values[1].0, &CurveTarget::EffectGroup("EyeBlink".into()));
        assert!(close(values[1].1, 0.25) && close(values[1].2, 0.5));
        assert_eq!(values[2].0, &CurveTarget::Parameter("Missing".into()));
        // 存在しないパラメータへの適用は無視する。
        assert_eq!(applied(&mut player, 1.0), 0.0);
    }
}
