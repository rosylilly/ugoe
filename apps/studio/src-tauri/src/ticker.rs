//! 刻みのスレッドの本体 (ADR 0028 の 3.、`docs/studio.md#刻みの速さと正確さ`)。
//!
//! 時刻の決め方は `pacing.rs` (時計を持たない)。ここは時計とスリープを使って回し、することは `TickHost` に任せる
//! (`lib.rs` の `Studio` と、遅れを測るテストの偽のホスト)。

use std::time::Instant;

use crate::pacing::{Measured, Pacer, RateMeter, TickRate, TrackingWait, WaitMeter, Wake};

/// 刻みのスレッドがすること。
pub trait TickHost {
    /// 刻みの速さ (刻みごとに読む)。
    fn tick_rate(&self) -> TickRate;
    /// 1 刻み進めてフレームを配る。`status` なら操作ウィンドウにも値を送る。
    /// この刻みで使ったトラッキングの値のうち、まだ `take_arrival` で渡していないものがあれば、最後に届いた時刻と促しの番号を返す
    /// (値を指揮役に渡すのと同じロックの中で取る。値が届いてから刻みで使われるまでの待ちを、使った刻みで数えるため)。
    fn tick(&self, dt: f32, status: bool) -> Option<(Instant, u32)>;
    /// 操作ウィンドウの入力の処理を促す (`subscribe_pulse`)。`seq` は促しの番号 (値に添えて返ってくる)。
    fn pulse(&self, seq: u32);
    /// 前に聞いてからトラッキングの値が届いていれば、最後に届いた時刻と、その値を検出した促しの番号 (`set_tracking`)。
    fn take_arrival(&self) -> Option<(Instant, u32)>;
    /// 実際の刻みの速さと揺れが変わった。
    fn set_measured(&self, measured: Option<Measured>);
    /// トラッキングの値が届いてから刻みで使われるまでの待ちが変わった (T0863)。
    fn set_tracking_wait(&self, wait: Option<TrackingWait>);
    /// 回し続けるか (テストで止める)。
    fn running(&self) -> bool {
        true
    }
}

/// 設定した速さで `dt` を測って `host` を進める (WebView の抑制を受けない)。
/// 遅れたときは取り戻そうとせず、次の刻みを今から数え直す (`dt` は実測なので時間は失わない。`pacing.rs`)。
/// 入力の処理の促しは 1 刻みに 1 回で、トラッキングの値が届いていれば値が刻みの少し前に届くように前へ寄せる (`Pacer`)。
/// 値が届いてから刻みで使われるまでの待ちを直近の約 1 秒について集めて渡す (`WaitMeter`。T0863)。
pub fn run(host: &impl TickHost) {
    let mut rate = host.tick_rate();
    let mut last = Instant::now();
    let mut pacer = Pacer::new(rate, last);
    let mut meter = RateMeter::default();
    let mut measured = None;
    let mut waits = WaitMeter::default();
    let mut waited = None;
    let mut count = 0u32;
    // 1 回の繰り返しは促しか刻みのどちらか 1 つで、どちらも次の刻みまでに 1 回ずつなので、待たずに回り続けることはない。
    while host.running() {
        if let Some((at, seq)) = host.take_arrival() {
            pacer.arrived(at, seq);
        }
        let now = Instant::now();
        let (at, wake) = pacer.wake(now);
        if at > now {
            std::thread::sleep(at - now);
        }
        let now = Instant::now();
        if wake == Wake::Pulse {
            // 操作ウィンドウの入力の処理を促す (隠れていても Channel のメッセージは届く。docs/studio.md#隠れても止まらない)。
            host.pulse(pacer.pulse(now));
            continue;
        }
        meter.record(now);
        if meter.measured() != measured {
            measured = meter.measured();
            host.set_measured(measured);
        }
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        count = (count + 1) % rate.status_every();
        // 刻みが使った値を先に渡してから、刻みの時刻を進める (`Pacer::tick` がその値の待ちを返す)。
        if let Some((at, seq)) = host.tick(dt, count == 0) {
            pacer.arrived(at, seq);
        }
        waits.record(now, pacer.tick(now));
        if waits.measured() != waited {
            waited = waits.measured();
            host.set_tracking_wait(waited);
        }
        let wanted = host.tick_rate();
        if wanted != rate {
            rate = wanted;
            pacer.set_rate(rate, Instant::now());
            meter.reset();
            measured = None;
            host.set_measured(None);
            waits.reset();
            waited = None;
            host.set_tracking_wait(None);
            count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// 値が届いてから刻みで使われるまでの待ち。
    #[derive(Debug, Default)]
    struct WaitMeter {
        /// まだ刻みで使われていない値が届いた時刻 (後から届いた値が前の値を置き換える)。
        pending: Option<Instant>,
        /// 使った刻みの時刻と待ち。
        waits: Vec<(Instant, Duration)>,
    }

    impl WaitMeter {
        fn arrived(&mut self, at: Instant) {
            self.pending = Some(at);
        }

        fn ticked(&mut self, now: Instant) {
            if let Some(at) = self.pending.take() {
                self.waits.push((now, now.saturating_duration_since(at)));
            }
        }

        /// `from` より後の待ちの数・平均・95 パーセンタイル・最大 (ミリ秒)。
        fn stats(&self, from: Instant) -> (usize, f64, f64, f64) {
            let mut waits: Vec<f64> = self
                .waits
                .iter()
                .filter(|(t, _)| *t >= from)
                .map(|(_, w)| w.as_secs_f64() * 1e3)
                .collect();
            if waits.is_empty() {
                return (0, 0.0, 0.0, 0.0);
            }
            waits.sort_by(f64::total_cmp);
            let mean = waits.iter().sum::<f64>() / waits.len() as f64;
            let p95 = waits[(waits.len() * 95 / 100).min(waits.len() - 1)];
            (waits.len(), mean, p95, waits[waits.len() - 1])
        }
    }

    /// 刻みの記録。
    #[derive(Debug, Default)]
    struct Log {
        wait: WaitMeter,
        ticks: u32,
        /// `dt` の最大と最小 (秒)。
        dt_max: f32,
        dt_min: f32,
        measured: Vec<Measured>,
        /// 刻みのスレッドが集めた待ち (`set_tracking_wait`)。
        reported: Vec<TrackingWait>,
        /// 刻みのスレッドがまだ読んでいない、値が届いた時刻と促しの番号。
        arrival: Option<(Instant, u32)>,
    }

    /// 偽のホスト: 刻みの記録をつけ、促しを偽の操作ウィンドウへ送る。
    struct FakeHost {
        rate: TickRate,
        log: Arc<Mutex<Log>>,
        pulses: Mutex<Sender<u32>>,
        until: Instant,
    }

    impl TickHost for FakeHost {
        fn tick_rate(&self) -> TickRate {
            self.rate
        }

        fn tick(&self, dt: f32, _status: bool) -> Option<(Instant, u32)> {
            let now = Instant::now();
            let mut log = self.log.lock().unwrap();
            log.wait.ticked(now);
            if log.ticks > 0 {
                log.dt_max = log.dt_max.max(dt);
                log.dt_min = log.dt_min.min(dt);
            } else {
                log.dt_min = f32::INFINITY;
            }
            log.ticks += 1;
            log.arrival.take()
        }

        fn take_arrival(&self) -> Option<(Instant, u32)> {
            self.log.lock().unwrap().arrival.take()
        }

        fn pulse(&self, seq: u32) {
            let _ = self.pulses.lock().unwrap().send(seq);
        }

        fn set_measured(&self, measured: Option<Measured>) {
            if let Some(m) = measured {
                self.log.lock().unwrap().measured.push(m);
            }
        }

        fn set_tracking_wait(&self, wait: Option<TrackingWait>) {
            if let Some(w) = wait {
                self.log.lock().unwrap().reported.push(w);
            }
        }

        fn running(&self) -> bool {
            Instant::now() < self.until
        }
    }

    /// 偽の操作ウィンドウ: 促しを受けたとき、カメラ (`camera_hz`) に前の検出より新しいフレームがあれば、
    /// `cost` (検出と IPC) の後に値を届ける。前の処理にかかった時間が経つまでは促しを飛ばす (`pulse.ts` の `Pulse.beat`)。
    /// `tagged` でなければ促しの番号を添えない (促しを寄せない、T0854 の前と同じ刻み)。
    fn fake_webview(
        pulses: Receiver<u32>,
        tagged: bool,
        camera_hz: u32,
        cost: Duration,
        start: Instant,
        log: Arc<Mutex<Log>>,
    ) {
        let frame_of = |t: Instant| {
            (t.saturating_duration_since(start).as_secs_f64() * f64::from(camera_hz)) as u64
        };
        let mut detected = None;
        let mut last_end: Option<Instant> = None;
        let mut last_cost = Duration::ZERO;
        while let Ok(seq) = pulses.recv() {
            let now = Instant::now();
            if let Some(end) = last_end
                && now.saturating_duration_since(end) < last_cost
            {
                continue;
            }
            let frame = frame_of(now);
            if detected.is_some_and(|d| d >= frame) {
                last_end = Some(Instant::now());
                last_cost = last_end.unwrap().saturating_duration_since(now);
                continue;
            }
            detected = Some(frame);
            std::thread::sleep(cost);
            let arrived = Instant::now();
            let mut log = log.lock().unwrap();
            log.wait.arrived(arrived);
            if tagged {
                log.arrival = Some((arrived, seq));
            }
            drop(log);
            last_end = Some(arrived);
            last_cost = arrived.saturating_duration_since(now);
        }
    }

    /// 偽のトラッキングで、値が届いてから刻みで使われるまでの待ちを測る (T0854)。
    /// 同じ組で、促しを寄せない (番号を添えない) ときと寄せるときを続けて測る。検出と IPC にかかる時間は
    /// `UGOE_TRACKING_COST_MS` (既定 6)。実時間で動くので通常のテストでは流さない:
    /// `cargo test -p ugoe-studio --lib ticker -- --ignored --nocapture`。
    #[test]
    #[ignore = "実時間で約 1 分かかる測定"]
    fn measure_tracking_wait() {
        let seconds = 5;
        let cost = Duration::from_millis(
            std::env::var("UGOE_TRACKING_COST_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6),
        );
        println!("cost {cost:?} / {seconds} s each");
        println!(
            "mode   tick camera | wait mean p95 max (after 1 s) | rate min | max interval | dt min max | values | reported mean max late (last)"
        );
        for rate in TickRate::ALL {
            for (camera_hz, tagged) in [(30, false), (30, true), (60, false), (60, true)] {
                let start = Instant::now();
                let log = Arc::new(Mutex::new(Log::default()));
                let (tx, rx) = mpsc::channel();
                let webview_log = Arc::clone(&log);
                let webview = std::thread::spawn(move || {
                    fake_webview(rx, tagged, camera_hz, cost, start, webview_log);
                });
                let host = FakeHost {
                    rate,
                    log: Arc::clone(&log),
                    pulses: Mutex::new(tx),
                    until: start + Duration::from_secs(seconds),
                };
                run(&host);
                drop(host);
                webview.join().unwrap();
                let log = log.lock().unwrap();
                let rate_min = log
                    .measured
                    .iter()
                    .map(|m| m.rate)
                    .fold(f32::INFINITY, f32::min);
                let interval_max = log
                    .measured
                    .iter()
                    .map(|m| m.max_interval_ms)
                    .fold(0.0, f32::max);
                // 最初の 1 秒 (寄せる前) を除く。
                let (count, mean, p95, max) = log.wait.stats(start + Duration::from_secs(1));
                // 刻みのスレッドが集めて操作ウィンドウに出す値 (直近の約 1 秒。番号を添えない「前」では出ない)。
                let reported = log.reported.last().map_or_else(
                    || "-".to_owned(),
                    |w| {
                        format!(
                            "{:>5.2} {:>5.2} ms {:>5.1}%",
                            w.mean_ms,
                            w.max_ms,
                            w.late * 100.0
                        )
                    },
                );
                println!(
                    "{:<6} {:>4} {:>6} | {mean:>5.2} {p95:>5.2} {max:>5.2} ms | {rate_min:>6.2} | {interval_max:>6.2} ms | {:>5.2} {:>5.2} ms | {count} | {reported}",
                    if tagged { "after" } else { "before" },
                    rate.hz(),
                    camera_hz,
                    log.dt_min * 1e3,
                    log.dt_max * 1e3,
                );
            }
        }
    }
}
