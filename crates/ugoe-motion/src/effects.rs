//! 目パチ・呼吸・リップシンク。仕様: `docs/spec/motion.md#エフェクト`。

use ugoe_model::ParamId;
use ugoe_model::effects::{Breath, EyeBlink, LipSync};
use ugoe_runtime::ParameterValues;

use crate::resolve::ResolvedParams;

/// パラメータ ID の列を、[`ResolvedParams::get`] に渡す形にする。
fn ids<'a>(ids: impl IntoIterator<Item = &'a ParamId>) -> impl Iterator<Item = Option<&'a str>> {
    ids.into_iter().map(|id| Some(id.as_str()))
}

/// 1 回の [`EyeBlinkPlayer::advance`] で進める段階の数の上限 (1000 回のまばたき)。
/// 巨大な `dt` や極端に短い設定で止まらないようにする。
const MAX_BLINK_PHASES_PER_ADVANCE: u32 = 4000;

/// 決定的な擬似乱数 (SplitMix64)。シードが 0 でも偏らない。
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// 状態を `seed` にした生成器を作る。
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// 次の 64 ビットの値 (SplitMix64。`docs/spec/motion.md#目パチ`)。
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// `[0, 1)` の一様乱数 (上位 24 ビットを使う)。
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// 目パチの段階。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkPhase {
    /// 開いたまま次のまばたきを待つ。
    Open,
    /// 1 → 0 へ閉じていく (`closing` 秒)。
    Closing,
    /// 閉じたまま (`closed` 秒)。
    Closed,
    /// 0 → 1 へ開いていく (`opening` 秒)。
    Opening,
}

/// 目パチの再生状態。
///
/// 対象パラメータを、開いている間は 1、閉じる間は 1 → 0 (線形)、閉じている間は 0、
/// 開く間は 0 → 1 (線形) と動かす「開き具合」を作り、現在の値に掛ける。
/// 次のまばたきまでの間隔は `[0.5, 1.5) * interval` の一様分布 (平均 `interval`) で、
/// シード付きの [`Rng`] で決める。
#[derive(Debug, Clone)]
pub struct EyeBlinkPlayer {
    config: EyeBlink,
    rng: Rng,
    phase: BlinkPhase,
    /// 現在の段階に入ってからの時間。
    t: f32,
    /// 今回の `Open` の長さ。
    wait: f32,
    param_indices: ResolvedParams,
}

impl EyeBlinkPlayer {
    /// `config` の目パチを `Open` の始めから始める。乱数を `config.seed` で初期化し、最初の間隔を引く。
    pub fn new(config: EyeBlink) -> Self {
        let mut rng = Rng::new(config.seed);
        let wait = next_interval(&mut rng, config.interval);
        Self {
            config,
            rng,
            phase: BlinkPhase::Open,
            t: 0.0,
            wait,
            param_indices: ResolvedParams::default(),
        }
    }

    /// 目パチの設定。
    pub fn config(&self) -> &EyeBlink {
        &self.config
    }

    /// 今の段階。
    pub fn phase(&self) -> BlinkPhase {
        self.phase
    }

    /// 現在の開き具合 (0 = 閉じている、1 = 開いている)。
    pub fn openness(&self) -> f32 {
        let ratio = |t: f32, len: f32| if len > 0.0 { (t / len).min(1.0) } else { 1.0 };
        match self.phase {
            BlinkPhase::Open => 1.0,
            BlinkPhase::Closing => 1.0 - ratio(self.t, self.config.closing),
            BlinkPhase::Closed => 0.0,
            BlinkPhase::Opening => ratio(self.t, self.config.opening),
        }
    }

    /// `dt` 秒進める。1 回で複数の段階をまたいでもよい。
    ///
    /// `interval <= 0` (不正な設定。NaN を含む) なら、まばたきしない。`dt` が 0 以下か有限値でない
    /// (NaN・無限大) ときは何もしない。1 回で進める段階の数には上限があり、超えた分の時間は捨てる。
    pub fn advance(&mut self, dt: f32) {
        if !(dt > 0.0 && dt.is_finite() && self.config.interval > 0.0) {
            return;
        }
        let mut dt = dt;
        for _ in 0..MAX_BLINK_PHASES_PER_ADVANCE {
            let len = self.phase_len();
            if self.t + dt < len {
                self.t += dt;
                return;
            }
            dt -= (len - self.t).max(0.0);
            self.t = 0.0;
            self.phase = match self.phase {
                BlinkPhase::Open => BlinkPhase::Closing,
                BlinkPhase::Closing => BlinkPhase::Closed,
                BlinkPhase::Closed => BlinkPhase::Opening,
                BlinkPhase::Opening => {
                    self.wait = next_interval(&mut self.rng, self.config.interval);
                    BlinkPhase::Open
                }
            };
        }
        // 段階の数の上限に達した。残りの時間は捨てる (今の段階の始めに留まる)。
    }

    /// 対象パラメータに `v * lerp(1, openness, strength)` を書き込む。
    /// `strength` はエフェクトグループ `EyeBlink` の強さ。
    pub fn apply(&mut self, parameters: &mut ParameterValues, strength: f32) {
        let factor = 1.0 + (self.openness() - 1.0) * strength;
        let indices = self
            .param_indices
            .get(parameters, ids(&self.config.parameters));
        for index in indices.iter().flatten() {
            parameters.set(*index, parameters.get(*index) * factor);
        }
    }

    fn phase_len(&self) -> f32 {
        match self.phase {
            BlinkPhase::Open => self.wait,
            BlinkPhase::Closing => self.config.closing,
            BlinkPhase::Closed => self.config.closed,
            BlinkPhase::Opening => self.config.opening,
        }
    }
}

/// 次のまばたきまでの間隔。`interval` は 0 より大きいこと。
fn next_interval(rng: &mut Rng, interval: f32) -> f32 {
    interval * (0.5 + rng.next_f32())
}

/// 呼吸の再生状態。各要素の位相を周期で折り返して持つ (長時間でも精度が落ちない)。
#[derive(Debug, Clone)]
pub struct BreathPlayer {
    config: Breath,
    /// 各要素の、周期内の時刻 `[0, cycle)`。
    phases: Vec<f32>,
    param_indices: ResolvedParams,
}

impl BreathPlayer {
    /// `config` の呼吸を、どの要素も周期内の時刻 0 から始める。
    pub fn new(config: Breath) -> Self {
        let phases = vec![0.0; config.parameters.len()];
        Self {
            config,
            phases,
            param_indices: ResolvedParams::default(),
        }
    }

    /// 呼吸の設定。
    pub fn config(&self) -> &Breath {
        &self.config
    }

    /// `dt` 秒進める。
    ///
    /// `dt` が 0 以下か有限値でない (NaN・無限大) ときは何もしない。
    pub fn advance(&mut self, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        for (phase, p) in self.phases.iter_mut().zip(&self.config.parameters) {
            *phase = if p.cycle > 0.0 {
                (*phase + dt).rem_euclid(p.cycle)
            } else {
                0.0
            };
        }
    }

    /// 各対象パラメータに `weight * (offset + peak * sin(2π t / cycle))` を加える。
    /// `cycle <= 0` の要素は `sin` の項を 0 とする。
    pub fn apply(&mut self, parameters: &mut ParameterValues) {
        let indices = self.param_indices.get(
            parameters,
            ids(self.config.parameters.iter().map(|p| &p.parameter)),
        );
        for ((p, phase), index) in self.config.parameters.iter().zip(&self.phases).zip(indices) {
            let Some(index) = *index else { continue };
            let wave = if p.cycle > 0.0 {
                (std::f32::consts::TAU * phase / p.cycle).sin()
            } else {
                0.0
            };
            let delta = p.weight * (p.offset + p.peak * wave);
            parameters.set(index, parameters.get(index) + delta);
        }
    }
}

/// リップシンク。ホストが与えた音量を対象パラメータに書き込む (音声の処理はしない)。
#[derive(Debug, Clone)]
pub struct LipSyncPlayer {
    config: LipSync,
    level: Option<f32>,
    param_indices: ResolvedParams,
}

impl LipSyncPlayer {
    /// `config` のリップシンクを、音量なし (`None`) で作る。
    pub fn new(config: LipSync) -> Self {
        Self {
            config,
            level: None,
            param_indices: ResolvedParams::default(),
        }
    }

    /// リップシンクの設定。
    pub fn config(&self) -> &LipSync {
        &self.config
    }

    /// 音量 (0〜1 にクランプする)。`None` ならリップシンクは何も書き込まない。NaN は `None` と同じに扱う。
    pub fn set_level(&mut self, level: Option<f32>) {
        self.level = level.filter(|l| !l.is_nan()).map(|l| l.clamp(0.0, 1.0));
    }

    /// 今の音量 (クランプした後の値)。
    pub fn level(&self) -> Option<f32> {
        self.level
    }

    /// 対象パラメータに `lerp(v, level, strength)` を書き込む。
    /// `strength` はエフェクトグループ `LipSync` の強さ。
    pub fn apply(&mut self, parameters: &mut ParameterValues, strength: f32) {
        let Some(level) = self.level else { return };
        let indices = self
            .param_indices
            .get(parameters, ids(&self.config.parameters));
        for index in indices.iter().flatten() {
            parameters.blend(*index, level, strength);
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::effects::{BreathParameter, Effects};
    use ugoe_model::{fixtures, standard};

    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    fn params() -> ParameterValues {
        ParameterValues::from_model(&fixtures::pose_and_effects())
    }

    fn get(p: &ParameterValues, id: &str) -> f32 {
        p.get(p.index_of(id).unwrap())
    }

    #[test]
    fn rng_is_deterministic_and_uniform() {
        let a: Vec<u64> = (0..4)
            .map({
                let mut r = Rng::new(7);
                move |_| r.next_u64()
            })
            .collect();
        let b: Vec<u64> = (0..4)
            .map({
                let mut r = Rng::new(7);
                move |_| r.next_u64()
            })
            .collect();
        assert_eq!(a, b);
        assert_ne!(Rng::new(0).next_u64(), Rng::new(1).next_u64());
        let mut r = Rng::new(0);
        let mean = (0..10_000).map(|_| r.next_f32()).sum::<f32>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
        let mut r = Rng::new(3);
        assert!(
            (0..1000)
                .map(|_| r.next_f32())
                .all(|x| (0.0..1.0).contains(&x))
        );
    }

    fn blink(seed: u64) -> EyeBlinkPlayer {
        EyeBlinkPlayer::new(EyeBlink {
            seed,
            ..Effects::standard().eye_blink
        })
    }

    /// 1 → 0 → 1 と、`closing` / `closed` / `opening` の時間で動く。
    #[test]
    fn eye_blink_cycle_follows_durations() {
        let mut b = blink(1);
        let wait = b.wait;
        assert!((2.0..6.0).contains(&wait));
        assert_eq!(b.openness(), 1.0);
        b.advance(wait + 0.05);
        assert_eq!(b.phase(), BlinkPhase::Closing);
        assert!(close(b.openness(), 0.5));
        b.advance(0.05 + 0.025);
        assert_eq!(b.phase(), BlinkPhase::Closed);
        assert_eq!(b.openness(), 0.0);
        b.advance(0.025 + 0.075);
        assert_eq!(b.phase(), BlinkPhase::Opening);
        assert!(close(b.openness(), 0.5));
        b.advance(0.1);
        assert_eq!(b.phase(), BlinkPhase::Open);
        assert_eq!(b.openness(), 1.0);
    }

    /// 同じシードなら同じ列、違うシードなら違う列。間隔の平均は `interval`。
    #[test]
    fn eye_blink_is_deterministic_per_seed() {
        let trace = |seed| {
            let mut b = blink(seed);
            (0..600)
                .map(|_| {
                    b.advance(1.0 / 30.0);
                    b.openness()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(trace(5), trace(5));
        assert_ne!(trace(5), trace(6));

        let mut rng = Rng::new(9);
        let n = 10_000;
        let mean = (0..n).map(|_| next_interval(&mut rng, 4.0)).sum::<f32>() / n as f32;
        assert!((mean - 4.0).abs() < 0.05, "{mean}");
    }

    #[test]
    fn eye_blink_multiplies_parameters_by_strength() {
        let mut p = params();
        let eye_l = p.index_of(standard::EYE_L_OPEN).unwrap();
        p.set(eye_l, 0.8);
        let mut b = blink(1);
        b.advance(b.wait + 0.12);
        assert_eq!(b.phase(), BlinkPhase::Closed);
        let mut q = p.clone();
        b.apply(&mut q, 1.0);
        assert_eq!(q.get(eye_l), 0.0);
        let mut q = p.clone();
        b.apply(&mut q, 0.5);
        assert!(close(q.get(eye_l), 0.4));
        let mut q = p.clone();
        b.apply(&mut q, 0.0);
        assert!(close(q.get(eye_l), 0.8));
    }

    #[test]
    fn eye_blink_with_non_positive_interval_does_nothing() {
        let mut b = EyeBlinkPlayer::new(EyeBlink {
            interval: 0.0,
            ..EyeBlink::standard()
        });
        b.advance(100.0);
        assert_eq!(b.openness(), 1.0);
    }

    /// NaN・無限大・巨大な `dt` や、極端に短い設定・NaN の設定でも、1 回の `advance` が終わる。
    #[test]
    fn eye_blink_advance_terminates_for_extreme_values() {
        let tiny = EyeBlink {
            interval: 1e-30,
            closing: 0.0,
            closed: 0.0,
            opening: 0.0,
            ..EyeBlink::standard()
        };
        let nan = EyeBlink {
            interval: f32::NAN,
            ..EyeBlink::standard()
        };
        for (config, dt) in [
            (EyeBlink::standard(), f32::NAN),
            (EyeBlink::standard(), f32::INFINITY),
            (EyeBlink::standard(), 1e30),
            (EyeBlink::standard(), f32::MAX),
            (tiny.clone(), 1.0 / 60.0),
            (nan, 1.0 / 60.0),
        ] {
            let mut b = EyeBlinkPlayer::new(config);
            b.advance(dt);
            assert!(b.openness().is_finite(), "dt={dt}");
            b.advance(1.0 / 60.0);
            assert!(b.openness().is_finite(), "dt={dt}");
        }
        // NaN と無限大の `dt` は何も進めない。
        let mut b = blink(1);
        b.advance(0.5);
        let t = b.t;
        b.advance(f32::NAN);
        b.advance(f32::INFINITY);
        assert_eq!((b.phase(), b.t), (BlinkPhase::Open, t));
    }

    /// NaN や無限大の `dt` で位相を壊さない (以降の呼吸が NaN のままにならない)。
    #[test]
    fn breath_ignores_non_finite_dt() {
        let mut breath = BreathPlayer::new(Breath::standard());
        breath.advance(0.3);
        breath.advance(f32::NAN);
        breath.advance(f32::INFINITY);
        assert!(
            breath.phases.iter().all(|t| close(*t, 0.3)),
            "{:?}",
            breath.phases
        );
        breath.advance(1e30);
        let mut p = params();
        breath.apply(&mut p);
        assert!(get(&p, standard::BREATH).is_finite());
    }

    #[test]
    fn lip_sync_treats_nan_level_as_no_level() {
        let mut lip = LipSyncPlayer::new(LipSync::standard());
        lip.set_level(Some(f32::NAN));
        assert_eq!(lip.level(), None);
        let mut p = params();
        lip.apply(&mut p, 1.0);
        assert_eq!(get(&p, standard::MOUTH_OPEN_Y), 0.0);
    }

    #[test]
    fn breath_adds_sine_wave() {
        let mut breath = BreathPlayer::new(Breath::standard());
        let quarter = 3.2345 / 4.0;
        breath.advance(quarter);
        let mut p = params();
        breath.apply(&mut p);
        // Breath: 1 * (0.5 + 0.5 * sin(π/2)) = 1。既定値 0 に加える。
        assert!(close(get(&p, standard::BREATH), 1.0));
        let t = quarter;
        let expected = 0.5 * 15.0 * (std::f32::consts::TAU * t / 6.5345).sin();
        assert!(close(get(&p, standard::ANGLE_X), expected));

        // 周期で折り返しても同じ値になる。
        let mut a = BreathPlayer::new(Breath::standard());
        a.advance(0.3);
        let mut b = BreathPlayer::new(Breath::standard());
        b.advance(0.3 + 6.5345);
        let (mut pa, mut pb) = (params(), params());
        a.apply(&mut pa);
        b.apply(&mut pb);
        assert!((get(&pa, standard::ANGLE_X) - get(&pb, standard::ANGLE_X)).abs() < 1e-3);
    }

    #[test]
    fn breath_ignores_unknown_parameters_and_zero_cycle() {
        let mut breath = BreathPlayer::new(Breath {
            parameters: vec![
                BreathParameter {
                    parameter: "Missing".into(),
                    offset: 1.0,
                    peak: 1.0,
                    cycle: 1.0,
                    weight: 1.0,
                },
                BreathParameter {
                    parameter: "X".into(),
                    offset: 0.25,
                    peak: 1.0,
                    cycle: 0.0,
                    weight: 1.0,
                },
            ],
        });
        breath.advance(0.3);
        let mut p = params();
        breath.apply(&mut p);
        assert!(close(get(&p, "X"), 0.25));
    }

    #[test]
    fn lip_sync_writes_level_with_strength() {
        let mut lip = LipSyncPlayer::new(LipSync::standard());
        let mut p = params();
        lip.apply(&mut p, 1.0);
        assert_eq!(get(&p, standard::MOUTH_OPEN_Y), 0.0);
        lip.set_level(Some(2.0));
        assert_eq!(lip.level(), Some(1.0));
        lip.set_level(Some(0.6));
        lip.apply(&mut p, 1.0);
        assert!(close(get(&p, standard::MOUTH_OPEN_Y), 0.6));
        let mut p = params();
        lip.apply(&mut p, 0.5);
        assert!(close(get(&p, standard::MOUTH_OPEN_Y), 0.3));
    }

    /// T0325: 並びの違う `ParameterValues` を渡すと添字を解決し直す。
    #[test]
    fn effects_resolve_parameters_again_when_layout_changes() {
        let mut lip = LipSyncPlayer::new(LipSync::standard());
        lip.set_level(Some(0.6));
        let mut breath = BreathPlayer::new(Breath::standard());
        breath.advance(0.5);
        let mut p = params();
        lip.apply(&mut p, 1.0);
        breath.apply(&mut p);
        let expected_breath = get(&p, standard::BREATH);

        let mut model = fixtures::pose_and_effects();
        model.parameters.reverse();
        let mut reversed = ParameterValues::from_model(&model);
        lip.apply(&mut reversed, 1.0);
        breath.apply(&mut reversed);
        assert!(close(get(&reversed, standard::MOUTH_OPEN_Y), 0.6));
        assert!(close(get(&reversed, standard::BREATH), expected_breath));
        for id in reversed.ids() {
            let id = id.as_str();
            assert!(close(get(&reversed, id), get(&p, id)), "{id}");
        }
    }
}
