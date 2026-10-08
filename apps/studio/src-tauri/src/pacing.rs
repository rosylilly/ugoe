//! 刻みの速さと、刻みの時刻の決め方 (ADR 0028 の 3.、`docs/studio.md#刻みの速さと正確さ`)。
//!
//! 時計とスレッドは持たない (時刻は引数で受け取る)。刻みのスレッド (`lib.rs` の `run_ticks`) が今の時刻を渡して使う。
//!
//! 刻みの時刻がずれても、指揮役は実測の `dt` を配り、受け手はその `dt` で物理を回すので、どの受け手も同じ形になる (決定性は崩れない)。
//! ずれて困るのは動きの滑らかさだけなので、取り戻そうとはせず、実際の速さと揺れを測って操作ウィンドウに出す。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 刻みの速さ (回/秒)。出力のフレームレートになる (受け手は刻みより速く描いても新しい形を出せない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickRate {
    Fps30,
    Fps60,
    Fps120,
}

impl TickRate {
    /// 選べる速さ (操作ウィンドウの選択肢の順)。
    pub const ALL: [Self; 3] = [Self::Fps30, Self::Fps60, Self::Fps120];
    /// 既定 (`docs/studio.md#出力ウィンドウ`)。
    pub const DEFAULT: Self = Self::Fps60;

    /// 回/秒。
    pub fn hz(self) -> u32 {
        match self {
            Self::Fps30 => 30,
            Self::Fps60 => 60,
            Self::Fps120 => 120,
        }
    }

    /// 回/秒から。選べない値なら `None`。
    pub fn from_hz(hz: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.hz() == hz)
    }

    /// 1 刻みの間隔。
    pub fn period(self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.hz()))
    }

    /// 操作ウィンドウへ値と負荷を送る間隔 (刻みの数。どの速さでも毎秒 10 回)。
    pub fn status_every(self) -> u32 {
        self.hz() / 10
    }
}

/// 刻みのスレッドが次に起きてすること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    /// 操作ウィンドウの入力の処理を促す (`subscribe_pulse`)。
    Pulse,
    /// 刻む。
    Tick,
}

/// 次の刻みと、入力の処理を促す時刻を決める。
///
/// 刻みは決まった間隔の格子に乗せる。遅れたときは取り戻そうとせず、今から数え直す (まとめて刻んで `dt` が 0 に近い刻みを作らない)。
///
/// 促しは 1 刻みに 1 回。トラッキングの値が届いていなければ刻みの直後に送る。届いていれば、促してから値が届くまでの時間
/// (検出と IPC) を直近の約 1 秒について測り、値が次の刻みの少し前に届くように促しを前へ寄せる (T0854。`docs/studio.md#入力`)。
/// 操作ウィンドウは促しを受けて検出するので、刻みのほうを値へ寄せると、寄せた刻みの促しがまた値を呼んで刻みが速くなる。
/// 刻みの格子は動かさず促しを動かすので、刻みの速さと `dt` は変わらない。
///
/// 促しには番号を付け、操作ウィンドウは検出を始めた促しの番号を値に添えて返す。検出が刻みの間隔より重くても
/// どの促しの値かを取り違えない (届いた時刻の前の最後の促しから測ると、促しを動かしたときに測った時間もずれて、寄せる先を追いかけ続ける)。
///
/// 刻むたびに、トラッキングの値が届いてから刻みで使われるまでの待ちと、刻みに遅れた値の数を返す (`tick`。T0863)。
/// 促しを寄せていれば促しが狙った刻み (`aim` の後) を覚えておき、値が使われる前か次の値に置き換わる前に狙った刻みが過ぎていれば
/// 「刻みに遅れた」とする (遅れた値は、狙った刻みの次の刻みまでに次の値が届いて置き換わることが多い)。
#[derive(Debug, Clone)]
pub struct Pacer {
    period: Duration,
    next: Instant,
    /// 次の刻みの前の促しを送ったか。
    pulsed: bool,
    /// 次の促しの番号。
    seq: u32,
    /// 直近の促し (古い順。`MAX_PULSES` まで)。これより古い促しの値は測らない。
    pulses: VecDeque<Pulse>,
    /// 届いた時刻と、促してから届くまでの時間 (古い順。`LEAD_WINDOW` より古いものは捨てる)。
    leads: VecDeque<(Instant, Duration)>,
    /// 促してから届くまでの時間をどれだけと見るか (`best_lead`。届くたびに選び直す)。
    lead: Option<Duration>,
    /// 届いて、まだ刻みで使われていない最後の値 (届いた時刻と、促しが狙った刻み)。
    unused: Option<(Instant, Option<Instant>)>,
    /// 前の刻みの時刻。
    last_tick: Option<Instant>,
    /// 前の刻みから、使われたか置き換わった値の数と、そのうち刻みに遅れた数。
    values: u32,
    late: u32,
}

/// 送った促し。
#[derive(Debug, Clone, Copy)]
struct Pulse {
    seq: u32,
    at: Instant,
    /// 値を使わせたい刻みの時刻 (促しを寄せていなければ `None`)。
    target: Option<Instant>,
}

/// 1 回の刻みで使ったトラッキングの値 (`Pacer::tick`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ticked {
    /// 使った値が届いてからこの刻みまでの待ち (前の刻みから値が届いていなければ `None`)。
    pub wait: Option<Duration>,
    /// 前の刻みからこの刻みまでに、使われたか次の値に置き換わった値の数。
    pub values: u32,
    /// そのうち、促しが狙った刻みに遅れた値の数。
    pub late: u32,
}

impl Pacer {
    /// 値が次の刻みのどれだけ前に届くように促すか (測った時間の揺れと、届いてから刻みのスレッドが読むまでの余裕)。
    pub const LEAD_MARGIN: Duration = Duration::from_millis(1);
    /// 促してから届くまでの時間を見る長さ。これより長く届かなければ、促しを刻みの直後に戻す。
    pub const LEAD_WINDOW: Duration = Duration::from_secs(1);
    /// 覚える届いた時刻の数の上限 (促しは 1 刻みに 1 回なので、120 刻みでも 1 秒で 120 回まで。選ぶ計算は数の 2 乗)。
    const MAX_LEADS: usize = 128;
    /// 覚える促しの数 (120 刻みで約 0.5 秒、30 刻みで約 2 秒)。これより前の促しの値が届くほど重ければ寄せない。
    const MAX_PULSES: usize = 64;

    pub fn new(rate: TickRate, now: Instant) -> Self {
        let period = rate.period();
        Self {
            period,
            next: now + period,
            // 最初の刻みの前には促さない (今までどおり、刻みの直後から)。
            pulsed: true,
            seq: 0,
            pulses: VecDeque::new(),
            leads: VecDeque::new(),
            lead: None,
            unused: None,
            last_tick: None,
            values: 0,
            late: 0,
        }
    }

    /// 速さを変える。次の刻みは今から新しい間隔の後。
    pub fn set_rate(&mut self, rate: TickRate, now: Instant) {
        self.period = rate.period();
        self.next = now + self.period;
        self.lead = self.best_lead();
    }

    /// 次の刻みの時刻。
    pub fn deadline(&self) -> Instant {
        self.next
    }

    /// 次に起きる時刻と、起きてすること。時刻が `now` より前ならすぐにする。
    pub fn wake(&self, now: Instant) -> (Instant, Wake) {
        if self.pulsed {
            return (self.next, Wake::Tick);
        }
        let at = self
            .next
            .checked_sub(self.pulse_offset(now))
            .unwrap_or(self.next);
        if at < self.next {
            (at, Wake::Pulse)
        } else {
            (self.next, Wake::Tick)
        }
    }

    /// 促しを次の刻みのどれだけ前に送るか (`0` より大きく刻みの間隔以下)。値が届いていなければ刻みの間隔 (前の刻みの直後)。
    pub fn pulse_offset(&self, now: Instant) -> Duration {
        match self.aim(now) {
            Some(aim) => {
                let offset = Self::wrap(aim, self.period);
                if offset.is_zero() {
                    self.period
                } else {
                    offset
                }
            }
            None => self.period,
        }
    }

    /// 促しから、値を使わせたい刻みまでの時間 (促してから届くまでの時間と余裕)。値が届いていなければ `None` (寄せない)。
    fn aim(&self, now: Instant) -> Option<Duration> {
        let fresh = self
            .leads
            .back()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) <= Self::LEAD_WINDOW);
        self.lead
            .filter(|_| fresh)
            .map(|lead| lead + Self::LEAD_MARGIN)
    }

    /// `d` を `period` で割った余り。
    fn wrap(d: Duration, period: Duration) -> Duration {
        let period = period.as_nanos().max(1);
        // 余りは `period` (1 秒以下) より小さいので `u64` に収まる。
        Duration::from_nanos(u64::try_from(d.as_nanos() % period).unwrap_or(0))
    }

    /// 促してから届くまでの時間を、どれだけと見て促しを寄せるか。直近の値が刻みで使われるまでの待ちの合計が
    /// 最も小さくなるものを、届いた時間の中から選ぶ。
    ///
    /// 最も長いものに合わせると、刻みの間隔より長くかかった 1 回 (計算機が混んだときなど) で寄せる先が 1 刻み近く回り、
    /// ほかの値が刻みに遅れて 1 刻み待つ。待ちは、促しを寄せた先 `c` と届くまでの時間 `d` から `(c + 余裕 - d) mod 間隔`。
    fn best_lead(&self) -> Option<Duration> {
        let wait = |c: Duration, d: Duration| {
            let period = self.period.as_nanos().max(1);
            (c + Self::LEAD_MARGIN + self.period * 2)
                .as_nanos()
                .saturating_sub(Self::wrap(d, self.period).as_nanos())
                % period
        };
        self.leads
            .iter()
            .map(|(_, c)| *c)
            .min_by_key(|c| self.leads.iter().map(|(_, d)| wait(*c, *d)).sum::<u128>())
    }

    /// `now` に促した。促しに付ける番号を返す。
    pub fn pulse(&mut self, now: Instant) -> u32 {
        // 狙う刻みは、予定した促しの時刻 (次の刻みの `pulse_offset` 前) から `aim` 後。格子の上の刻みになる
        // (起きるのが遅れても狙いはずれない)。
        let target = self
            .aim(now)
            .map(|aim| self.next + aim.saturating_sub(self.pulse_offset(now)));
        self.pulsed = true;
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        if self.pulses.len() >= Self::MAX_PULSES {
            self.pulses.pop_front();
        }
        self.pulses.push_back(Pulse {
            seq,
            at: now,
            target,
        });
        seq
    }

    /// 番号 `pulse` の促しで検出したトラッキングの値が `at` に届いた。促してから届くまでの時間を覚える。
    /// 覚えていない番号 (古すぎる・送っていない) と、促しより前の時刻は寄せる量に使わない
    /// (刻みで使われるまでの待ちには数える。狙った刻みが分からないので遅れには数えない)。
    pub fn arrived(&mut self, at: Instant, pulse: u32) {
        let sent = self.pulses.iter().rev().find(|p| p.seq == pulse).copied();
        if let Some((_, target)) = self.unused.replace((at, sent.and_then(|p| p.target))) {
            // 刻みで使われないまま置き換わった。
            self.settle(target);
        }
        let Some(Pulse { at: pulse, .. }) = sent else {
            return;
        };
        if at < pulse {
            return;
        }
        if let Some(since) = at.checked_sub(Self::LEAD_WINDOW) {
            while self.leads.front().is_some_and(|(t, _)| *t < since) {
                self.leads.pop_front();
            }
        }
        if self.leads.len() >= Self::MAX_LEADS {
            self.leads.pop_front();
        }
        self.leads.push_back((at, at - pulse));
        self.lead = self.best_lead();
    }

    /// 狙った刻みが `target` の値を、使ったか置き換えた。狙った刻み (の半刻み前より後) を前の刻みで過ぎていれば遅れ。
    fn settle(&mut self, target: Option<Instant>) {
        self.values += 1;
        let late = target
            .zip(self.last_tick)
            .is_some_and(|(target, last)| last + self.period / 2 >= target);
        self.late += u32::from(late);
    }

    /// `now` まで待った (起きた) ので刻む。次の刻みの時刻を進め、この刻みで使った値を返す。
    /// この刻みの前に届いた値 (この刻みが使った値) は、先に `arrived` で渡しておく。
    pub fn tick(&mut self, now: Instant) -> Ticked {
        let wait = self.unused.take().map(|(at, target)| {
            self.settle(target);
            // 刻みの中で届いた値 (刻みの時刻より後) の待ちは 0。
            now.saturating_duration_since(at)
        });
        let ticked = Ticked {
            wait,
            values: self.values,
            late: self.late,
        };
        self.values = 0;
        self.late = 0;
        self.last_tick = Some(now);
        self.pulsed = false;
        self.next += self.period;
        if self.next < now {
            self.next = now + self.period;
        }
        ticked
    }
}

/// 実際の刻みの速さと揺れを測る (直近の約 1 秒)。
#[derive(Debug, Clone, Default)]
pub struct RateMeter {
    window_start: Option<Instant>,
    count: u32,
    max_interval: Duration,
    last: Option<Instant>,
    /// 直近に測り終えた結果。
    measured: Option<Measured>,
}

/// 測った速さと揺れ。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measured {
    /// 実際の刻みの速さ (回/秒)。
    pub rate: f32,
    /// 刻みの間隔の最大 (ミリ秒)。設定の間隔より大きく外れていれば、OS のスリープが粗いか、刻みが重い。
    pub max_interval_ms: f32,
}

/// トラッキングの値が届いてから刻みで使われるまでの待ちを集める (直近の約 1 秒。T0863)。確保はしない。
#[derive(Debug, Clone, Default)]
pub struct WaitMeter {
    window_start: Option<Instant>,
    /// 刻みで使われた値の数 (待ちの平均の分母)。
    used: u32,
    /// 使われたか置き換わった値の数と、そのうち刻みに遅れた数 (遅れた割合の分母と分子)。
    values: u32,
    late: u32,
    total: Duration,
    max: Duration,
    /// 直近に集め終えた結果。
    measured: Option<TrackingWait>,
}

/// 集めた待ち。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingWait {
    /// 刻みで使われた値の待ちの平均 (ミリ秒)。
    pub mean_ms: f32,
    /// 待ちの最大 (ミリ秒)。
    pub max_ms: f32,
    /// 届いた値のうち、促しが狙った刻みに遅れた割合 (0〜1。`Ticked::late`)。
    pub late: f32,
}

impl WaitMeter {
    /// 集める長さ。
    const WINDOW: Duration = Duration::from_secs(1);

    /// `now` に刻んだ。`ticked` はその刻みで使った値 (`Pacer::tick`)。
    pub fn record(&mut self, now: Instant, ticked: Ticked) {
        let start = *self.window_start.get_or_insert(now);
        if let Some(wait) = ticked.wait {
            self.used += 1;
            self.total = self.total.saturating_add(wait);
            self.max = self.max.max(wait);
        }
        self.values = self.values.saturating_add(ticked.values);
        self.late = self.late.saturating_add(ticked.late);
        if now.saturating_duration_since(start) >= Self::WINDOW {
            // 約 1 秒の間に値が 1 つも使われなければ、無し (トラッキングを使っていない)。
            self.measured = (self.used > 0).then(|| TrackingWait {
                mean_ms: self.total.as_secs_f32() * 1e3 / self.used as f32,
                max_ms: self.max.as_secs_f32() * 1e3,
                late: self.late as f32 / self.values.max(1) as f32,
            });
            *self = Self {
                window_start: Some(now),
                measured: self.measured,
                ..Self::default()
            };
        }
    }

    /// 集め直す (速さを変えたとき)。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 直近に集め終えた結果 (まだ 1 秒経っていない・値が届いていなければ `None`)。
    pub fn measured(&self) -> Option<TrackingWait> {
        self.measured
    }
}

impl RateMeter {
    /// 測る長さ。
    const WINDOW: Duration = Duration::from_secs(1);

    /// 刻んだ時刻を記録する。
    pub fn record(&mut self, now: Instant) {
        if let Some(last) = self.last {
            self.max_interval = self.max_interval.max(now.saturating_duration_since(last));
        }
        self.last = Some(now);
        let Some(start) = self.window_start else {
            self.window_start = Some(now);
            return;
        };
        self.count += 1;
        let elapsed = now.saturating_duration_since(start);
        if elapsed >= Self::WINDOW {
            self.measured = Some(Measured {
                rate: self.count as f32 / elapsed.as_secs_f32(),
                max_interval_ms: self.max_interval.as_secs_f32() * 1e3,
            });
            self.window_start = Some(now);
            self.count = 0;
            self.max_interval = Duration::ZERO;
        }
    }

    /// 測り直す (速さを変えたとき)。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 直近に測り終えた結果 (まだ 1 秒経っていなければ `None`)。
    pub fn measured(&self) -> Option<Measured> {
        self.measured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_round_trip() {
        for rate in TickRate::ALL {
            assert_eq!(TickRate::from_hz(rate.hz()), Some(rate));
            assert_eq!(rate.status_every() * 10, rate.hz());
        }
        assert_eq!(TickRate::from_hz(0), None);
        assert_eq!(TickRate::from_hz(144), None);
        assert_eq!(TickRate::DEFAULT.hz(), 60);
    }

    #[test]
    fn pacer_keeps_the_grid_when_on_time() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let period = TickRate::Fps60.period();
        // 少し遅れて起きても、次の刻みは格子の上 (遅れを次で吸収する)。
        let woke = pacer.deadline() + Duration::from_millis(2);
        pacer.tick(woke);
        assert_eq!(pacer.deadline(), start + period * 2);
    }

    #[test]
    fn pacer_does_not_catch_up_after_a_stall() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let period = TickRate::Fps60.period();
        // 0.5 秒止まっていた: まとめて刻まず、今から 1 間隔後にする。
        let woke = start + Duration::from_millis(500);
        pacer.tick(woke);
        assert_eq!(pacer.deadline(), woke + period);
    }

    #[test]
    fn pacer_switches_rate_from_now() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps30, start);
        let now = start + Duration::from_millis(5);
        pacer.set_rate(TickRate::Fps120, now);
        assert_eq!(pacer.deadline(), now + TickRate::Fps120.period());
    }

    /// 刻みのスレッドを仮の時刻で回す。促しから `cost` 後に値が届く (操作ウィンドウは促しを受けて検出する)。
    /// `cost` が `None` の促しでは値が届かない。刻んだ時刻と、値が届いてから刻みで使われるまでの待ちと、
    /// `Pacer::tick` が遅れたとした値の数を返す (`tick` が返す待ちは外から測った待ちと等しいことを確かめる)。
    fn simulate(
        rate: TickRate,
        start: Instant,
        until: Instant,
        mut cost: impl FnMut(Instant) -> Option<Duration>,
    ) -> (Pacer, Vec<Instant>, Vec<Duration>, usize) {
        let mut pacer = Pacer::new(rate, start);
        let mut ticks = Vec::new();
        let mut waits = Vec::new();
        let mut late = 0;
        // 届く前の値 (届く時刻と促しの番号。届く時刻の順)。
        let mut pending: VecDeque<(Instant, u32)> = VecDeque::new();
        let mut now = start;
        for _ in 0..100_000 {
            let (at, wake) = pacer.wake(now);
            now = now.max(at);
            if now >= until {
                break;
            }
            match wake {
                Wake::Pulse => {
                    let seq = pacer.pulse(now);
                    if let Some(cost) = cost(now) {
                        pending.push_back((now + cost, seq));
                    }
                }
                Wake::Tick => {
                    // 前の刻みからこの刻みまでに届いた値。刻みで使われるのは最後のもの。
                    let mut used = None;
                    while let Some((arrived, seq)) =
                        pending.front().copied().filter(|(a, _)| *a <= now)
                    {
                        pending.pop_front();
                        pacer.arrived(arrived, seq);
                        used = Some(arrived);
                    }
                    if let Some(arrived) = used {
                        waits.push(now - arrived);
                    }
                    let ticked = pacer.tick(now);
                    assert_eq!(ticked.wait, used.map(|a| now - a));
                    late += ticked.late as usize;
                    ticks.push(now);
                }
            }
        }
        (pacer, ticks, waits, late)
    }

    #[test]
    fn pulse_follows_the_tick_without_tracking() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let period = TickRate::Fps60.period();
        // 最初の刻みの前には促さない。
        assert_eq!(pacer.wake(start), (start + period, Wake::Tick));
        let now = start + period;
        pacer.tick(now);
        // 刻みの直後に促す (今より前の時刻 = すぐ)。
        let (at, wake) = pacer.wake(now);
        assert_eq!(wake, Wake::Pulse);
        assert!(at <= now);
        assert_eq!(pacer.pulse(now), 0);
        assert_eq!(pacer.wake(now), (start + period * 2, Wake::Tick));
        assert_eq!(pacer.pulse_offset(now), period);
    }

    #[test]
    fn arrivals_move_the_pulse_before_the_next_tick() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let period = TickRate::Fps60.period();
        let tick = start + period;
        pacer.tick(tick);
        let seq = pacer.pulse(tick);
        // 促してから 6 ms 後に値が届いた: 次の刻みからは、刻みの 6 + 1 ms 前に促す。
        pacer.arrived(tick + Duration::from_millis(6), seq);
        let tick = tick + period;
        pacer.tick(tick);
        let pulse = tick + period - Duration::from_millis(7);
        assert_eq!(pacer.wake(tick), (pulse, Wake::Pulse));
        // 揺れたときは、遅れて届いて 1 刻み待つものが無いように寄せる (待ちの合計が小さいもの)。
        let seq = pacer.pulse(pulse);
        pacer.arrived(pulse + Duration::from_millis(4), seq);
        assert_eq!(pacer.pulse_offset(tick + period), Duration::from_millis(7));
        pacer.arrived(pulse + Duration::from_millis(9), seq);
        assert_eq!(pacer.pulse_offset(tick + period), Duration::from_millis(10));
    }

    #[test]
    fn a_single_slow_arrival_does_not_turn_the_pulse_around() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let mut now = start;
        for i in 0..30 {
            let seq = pacer.pulse(now);
            // 1 回だけ (混んで) 刻みの間隔より長くかかった。
            let cost = if i == 10 { 20 } else { 6 };
            pacer.arrived(now + Duration::from_millis(cost), seq);
            now += TickRate::Fps60.period();
        }
        // 最も長いもの (20 ms。間隔で回すと 3.3 ms) に合わせず、ほかの値が刻みに遅れないようにする。
        assert_eq!(pacer.pulse_offset(now), Duration::from_millis(7));
    }

    #[test]
    fn slow_detection_pulses_whole_ticks_earlier() {
        // 刻みの間隔より長くかかるなら、その分だけ前の刻みの間で促す (値が届くのは何刻みか後の刻みの少し前)。
        let start = Instant::now();
        let rate = TickRate::Fps120;
        let mut pacer = Pacer::new(rate, start);
        pacer.tick(start + rate.period());
        let pulse = start + rate.period();
        let seq = pacer.pulse(pulse);
        // 次の促しを送った後に、前の促しの値が届く。
        pacer.pulse(pulse + rate.period());
        pacer.arrived(pulse + Duration::from_millis(12), seq);
        let expected = Duration::from_millis(13).as_nanos() % rate.period().as_nanos();
        assert_eq!(pacer.pulse_offset(pulse).as_nanos(), expected);
    }

    #[test]
    fn unknown_pulses_are_ignored() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps60, start);
        let period = TickRate::Fps60.period();
        let first = pacer.pulse(start);
        // 送っていない番号・促しより前の時刻は数えない。
        pacer.arrived(start + Duration::from_millis(3), first + 1);
        pacer.arrived(start - Duration::from_millis(3), first);
        assert_eq!(pacer.pulse_offset(start), period);
        // 覚えている数より前の促しの値も数えない (重すぎるときは寄せない)。
        for i in 1..=Pacer::MAX_PULSES as u32 {
            pacer.pulse(start + Duration::from_millis(u64::from(i)));
        }
        pacer.arrived(start + Duration::from_millis(100), first);
        assert_eq!(pacer.pulse_offset(start), period);
    }

    #[test]
    fn ticks_keep_the_rate_and_tracking_waits_shrink() {
        let start = Instant::now();
        let until = start + Duration::from_secs(3);
        for rate in TickRate::ALL {
            let period = rate.period();
            for cost_ms in [2, 6, 12, 20, 45] {
                let cost = Duration::from_millis(cost_ms);
                let (_, ticks, waits, late) = simulate(rate, start, until, |_| Some(cost));
                // 刻みは格子のまま (速さは設定を超えず、`dt` も揺れない)。
                for pair in ticks.windows(2) {
                    assert_eq!(pair[1] - pair[0], period, "{rate:?} {cost_ms} ms");
                }
                assert!(ticks.len() as u32 <= rate.hz() * 3, "{rate:?}");
                // 寄せた後は、値は刻みの `LEAD_MARGIN` 前に届く。
                let settled = &waits[waits.len() / 2..];
                assert!(!settled.is_empty());
                for wait in settled {
                    assert!(
                        wait.abs_diff(Pacer::LEAD_MARGIN) < Duration::from_micros(1),
                        "{rate:?} {cost_ms} ms: {wait:?}"
                    );
                }
                // 揺れなければ、刻みに遅れる値は無い。
                assert_eq!(late, 0, "{rate:?} {cost_ms} ms");
            }
        }
    }

    #[test]
    fn pulse_returns_after_the_tick_when_tracking_stops() {
        let start = Instant::now();
        let rate = TickRate::Fps60;
        let stop = start + Duration::from_secs(1);
        let until = stop + Pacer::LEAD_WINDOW + Duration::from_millis(100);
        let (pacer, ticks, _, _) = simulate(rate, start, until, |now| {
            (now < stop).then_some(Duration::from_millis(5))
        });
        let last = *ticks.last().unwrap();
        assert_eq!(pacer.pulse_offset(last), rate.period());
        // 届いている間に寄せた促しは、届かなくなっても刻みの格子を動かさない。
        for pair in ticks.windows(2) {
            assert_eq!(pair[1] - pair[0], rate.period());
        }
    }

    #[test]
    fn leads_are_bounded() {
        let start = Instant::now();
        let mut pacer = Pacer::new(TickRate::Fps120, start);
        let seq = pacer.pulse(start);
        // 1 秒の間に上限より多く届いても、覚える数は上限まで。
        for i in 0..1000 {
            pacer.arrived(start + Duration::from_micros(i * 500), seq);
        }
        assert_eq!(pacer.leads.len(), Pacer::MAX_LEADS);
    }

    #[test]
    fn tick_reports_the_value_it_used_and_late_values() {
        let start = Instant::now();
        let rate = TickRate::Fps60;
        let period = rate.period();
        let mut pacer = Pacer::new(rate, start);
        // 届いていなければ何も出さない。
        let tick = start + period;
        assert_eq!(pacer.tick(tick), Ticked::default());
        // 寄せる前の促し (刻みの直後) の値は、狙った刻みが無いので遅れには数えない。
        let seq = pacer.pulse(tick);
        pacer.arrived(tick + Duration::from_millis(6), seq);
        let tick = tick + period;
        let used = Ticked {
            wait: Some(period - Duration::from_millis(6)),
            values: 1,
            late: 0,
        };
        assert_eq!(pacer.tick(tick), used);
        // 寄せた促し (次の刻みの 7 ms 前) の値が 6 ms で届けば、刻みの 1 ms 前に届いて間に合う。
        let (pulse, wake) = pacer.wake(tick);
        assert_eq!(wake, Wake::Pulse);
        let seq = pacer.pulse(pulse);
        pacer.arrived(pulse + Duration::from_millis(6), seq);
        let tick = tick + period;
        let used = Ticked {
            wait: Some(Pacer::LEAD_MARGIN),
            values: 1,
            late: 0,
        };
        assert_eq!(pacer.tick(tick), used);
        // 混んで 9 ms かかると、狙った刻みに遅れる。その刻みは新しい値を使わない。
        let (pulse, _) = pacer.wake(tick);
        let slow = pacer.pulse(pulse);
        let missed = tick + period;
        assert_eq!(pacer.tick(missed), Ticked::default());
        // 遅れた値は次の促しの値に置き換わる (使われない) が、遅れた値として数える。
        let slow_at = pulse + Duration::from_millis(9);
        pacer.arrived(slow_at, slow);
        let (pulse, _) = pacer.wake(slow_at);
        let seq = pacer.pulse(pulse);
        let arrived = pulse + Duration::from_millis(6);
        pacer.arrived(arrived, seq);
        let tick = missed + period;
        let used = Ticked {
            wait: Some(tick - arrived),
            values: 2,
            late: 1,
        };
        assert_eq!(pacer.tick(tick), used);
        // 覚えていない番号の値も、待ちには数える (遅れには数えない)。
        pacer.arrived(tick + Duration::from_millis(3), u32::MAX);
        let next = tick + period;
        let used = Ticked {
            wait: Some(period - Duration::from_millis(3)),
            values: 1,
            late: 0,
        };
        assert_eq!(pacer.tick(next), used);
    }

    #[test]
    fn late_values_are_counted_when_detection_sometimes_misses_the_tick() {
        let start = Instant::now();
        let until = start + Duration::from_secs(3);
        let rate = TickRate::Fps60;
        let mut pulses = 0u32;
        // 10 回に 1 回、検出と IPC が 3 ms 長くかかる (余裕の 1 ms を超えて刻みに遅れる)。
        let (_, _, waits, late) = simulate(rate, start, until, |_| {
            pulses += 1;
            Some(Duration::from_millis(if pulses.is_multiple_of(10) {
                9
            } else {
                6
            }))
        });
        let slow = (pulses / 10) as usize;
        // 寄せる前 (最初の値) のほかは、遅い値をどれも遅れに数える。
        assert!(slow - 1 <= late && late <= slow, "{late} / {slow}");
        // 遅れた値は次の値に置き換わるので、使われた値の待ちは余裕のまま。
        let settled = &waits[waits.len() / 2..];
        assert!(
            settled
                .iter()
                .all(|w| w.abs_diff(Pacer::LEAD_MARGIN) < Duration::from_micros(1))
        );
    }

    #[test]
    fn wait_meter_collects_a_second_of_waits() {
        let start = Instant::now();
        let period = Duration::from_micros(16_700);
        let mut meter = WaitMeter::default();
        let mut t = start;
        meter.record(t, Ticked::default());
        // 60 刻みのうち 40 刻みで値を使う: 待ちは 1 ms と、4 回だけ 15 ms。遅れた値が 5 つあった (使われずに置き換わったものを含む)。
        for i in 1..=60u32 {
            t += period;
            let used = !i.is_multiple_of(3);
            let long = used && i % 10 == 1;
            meter.record(
                t,
                Ticked {
                    wait: used.then(|| Duration::from_millis(if long { 15 } else { 1 })),
                    values: u32::from(used) + u32::from(i == 20),
                    late: u32::from(long) + u32::from(i == 20),
                },
            );
        }
        let measured = meter.measured().unwrap();
        // 1・11・31・41 が長い (21・51 は 3 の倍数で値が無い)。
        let expected_mean = (36.0 * 1.0 + 4.0 * 15.0) / 40.0;
        assert!(
            (measured.mean_ms - expected_mean).abs() < 1e-3,
            "{measured:?}"
        );
        assert!((measured.max_ms - 15.0).abs() < 1e-3, "{measured:?}");
        assert!((measured.late - 5.0 / 41.0).abs() < 1e-6, "{measured:?}");
        // 次の約 1 秒に値が使われなければ、無しに戻る (トラッキングを止めた)。
        for _ in 0..60 {
            t += period;
            meter.record(t, Ticked::default());
        }
        assert_eq!(meter.measured(), None);
        meter.record(
            t + period,
            Ticked {
                wait: Some(period),
                values: 1,
                late: 0,
            },
        );
        meter.reset();
        assert_eq!(meter.measured(), None);
    }

    #[test]
    fn meter_measures_rate_and_the_longest_interval() {
        let start = Instant::now();
        let mut meter = RateMeter::default();
        let mut t = start;
        meter.record(t);
        // 60 回刻む。1 回だけ 30 ms 空く。
        for i in 0..60 {
            t += if i == 10 {
                Duration::from_millis(30)
            } else {
                Duration::from_micros(16_500)
            };
            meter.record(t);
        }
        let measured = meter.measured().unwrap();
        let expected = 60.0 / (t - start).as_secs_f32();
        assert!((measured.rate - expected).abs() < 1e-3, "{measured:?}");
        assert!(
            (measured.max_interval_ms - 30.0).abs() < 1e-3,
            "{measured:?}"
        );
        meter.reset();
        assert_eq!(meter.measured(), None);
    }
}
