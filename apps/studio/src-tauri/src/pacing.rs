//! 刻みの速さと、刻みの時刻の決め方 (ADR 0028 の 3.、`docs/studio.md#刻みの速さと正確さ`)。
//!
//! 時計とスレッドは持たない (時刻は引数で受け取る)。刻みのスレッド (`lib.rs` の `run_ticks`) が今の時刻を渡して使う。
//!
//! 刻みの時刻がずれても、指揮役は実測の `dt` を配り、受け手はその `dt` で物理を回すので、どの受け手も同じ形になる (決定性は崩れない)。
//! ずれて困るのは動きの滑らかさだけなので、取り戻そうとはせず、実際の速さと揺れを測って操作ウィンドウに出す。

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

/// 次の刻みの時刻を決める。遅れたときは取り戻そうとせず、今から数え直す (まとめて刻んで `dt` が 0 に近い刻みを作らない)。
#[derive(Debug, Clone)]
pub struct Pacer {
    period: Duration,
    next: Instant,
}

impl Pacer {
    pub fn new(rate: TickRate, now: Instant) -> Self {
        let period = rate.period();
        Self {
            period,
            next: now + period,
        }
    }

    /// 速さを変える。次の刻みは今から新しい間隔の後。
    pub fn set_rate(&mut self, rate: TickRate, now: Instant) {
        self.period = rate.period();
        self.next = now + self.period;
    }

    /// 次の刻みの時刻。
    pub fn deadline(&self) -> Instant {
        self.next
    }

    /// `now` まで待った (起きた) ので刻む。次の刻みの時刻を進める。
    pub fn tick(&mut self, now: Instant) {
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
