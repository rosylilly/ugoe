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
#[derive(Debug, Clone)]
pub struct Pacer {
    period: Duration,
    next: Instant,
    /// 次の刻みの前の促しを送ったか。
    pulsed: bool,
    /// 次の促しの番号。
    seq: u32,
    /// 直近の促しの番号と時刻 (古い順。`MAX_PULSES` まで)。これより古い促しの値は測らない。
    pulses: VecDeque<(u32, Instant)>,
    /// 届いた時刻と、促してから届くまでの時間 (古い順。`LEAD_WINDOW` より古いものは捨てる)。
    leads: VecDeque<(Instant, Duration)>,
    /// 促してから届くまでの時間をどれだけと見るか (`best_lead`。届くたびに選び直す)。
    lead: Option<Duration>,
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
        let fresh = self
            .leads
            .back()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) <= Self::LEAD_WINDOW);
        match self.lead {
            Some(lead) if fresh => {
                let offset = Self::wrap(lead + Self::LEAD_MARGIN, self.period);
                if offset.is_zero() {
                    self.period
                } else {
                    offset
                }
            }
            _ => self.period,
        }
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
        self.pulsed = true;
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        if self.pulses.len() >= Self::MAX_PULSES {
            self.pulses.pop_front();
        }
        self.pulses.push_back((seq, now));
        seq
    }

    /// 番号 `pulse` の促しで検出したトラッキングの値が `at` に届いた。促してから届くまでの時間を覚える。
    /// 覚えていない番号 (古すぎる・送っていない) と、促しより前の時刻は無視する。
    pub fn arrived(&mut self, at: Instant, pulse: u32) {
        let Some(&(_, pulse)) = self.pulses.iter().rev().find(|(seq, _)| *seq == pulse) else {
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

    /// `now` まで待った (起きた) ので刻む。次の刻みの時刻を進める。
    pub fn tick(&mut self, now: Instant) {
        self.pulsed = false;
        self.next += self.period;
        if self.next < now {
            self.next = now + self.period;
        }
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
    /// `cost` が `None` の促しでは値が届かない。刻んだ時刻と、値が届いてから刻みで使われるまでの待ちを返す。
    fn simulate(
        rate: TickRate,
        start: Instant,
        until: Instant,
        mut cost: impl FnMut(Instant) -> Option<Duration>,
    ) -> (Pacer, Vec<Instant>, Vec<Duration>) {
        let mut pacer = Pacer::new(rate, start);
        let mut ticks = Vec::new();
        let mut waits = Vec::new();
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
                    pacer.tick(now);
                    ticks.push(now);
                }
            }
        }
        (pacer, ticks, waits)
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
                let (_, ticks, waits) = simulate(rate, start, until, |_| Some(cost));
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
            }
        }
    }

    #[test]
    fn pulse_returns_after_the_tick_when_tracking_stops() {
        let start = Instant::now();
        let rate = TickRate::Fps60;
        let stop = start + Duration::from_secs(1);
        let until = stop + Pacer::LEAD_WINDOW + Duration::from_millis(100);
        let (pacer, ticks, _) = simulate(rate, start, until, |now| {
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
