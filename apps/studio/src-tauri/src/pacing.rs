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
/// (検出と IPC) を直近の値について測り、値が次の刻みの少し前に届くように促しを前へ寄せる (T0854。`docs/studio.md#入力`)。
/// 見る値は直近の約 1 秒の値と、それが少なければ (検出が重い) 最低 `MIN_LEADS` 個まで遡った値 (5 秒より古いものは使わない。T0868)。
/// 操作ウィンドウは促しを受けて検出するので、刻みのほうを値へ寄せると、寄せた刻みの促しがまた値を呼んで刻みが速くなる。
/// 刻みの格子は動かさず促しを動かすので、刻みの速さと `dt` は変わらない。
///
/// 促しには番号を付け、操作ウィンドウは検出を始めた促しの番号を値に添えて返す。検出が刻みの間隔より重くても
/// どの促しの値かを取り違えない (届いた時刻の前の最後の促しから測ると、促しを動かしたときに測った時間もずれて、寄せる先を追いかけ続ける)。
///
/// 刻むたびに、トラッキングの値が届いてから刻みで使われるまでの待ちと、刻みに遅れた値の数を返す (`tick`。T0863)。
/// 促しを寄せていれば促しが狙った刻み (`aim` の後) を覚えておき、値が使われる前か次の値に置き換わる前に狙った刻みが過ぎていれば
/// 「刻みに遅れた」とする (遅れた値は、狙った刻みの次の刻みまでに次の値が届いて置き換わることが多い)。
///
/// 値が刻みのどれだけ前に届くように寄せるか (余裕) は、起きる時刻の揺れで刻みに遅れた値の割合を見て、1〜4 ms の間で増減する
/// (`adjust_margin`。T0864)。
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
    /// 届いた時刻と、促してから届くまでの時間 (古い順。`LEAD_WINDOW` より古いものは、`min_leads` 個に足りない分だけ
    /// `LEAD_MAX_AGE` まで残す。`prune_leads`)。
    leads: VecDeque<(Instant, Duration)>,
    /// `LEAD_WINDOW` より古い値を残して、見る値をこの数まで増やす (`MIN_LEADS`。テストでは 0 にして T0868 の前と比べる)。
    min_leads: usize,
    /// 促してから届くまでの時間をどれだけと見るか (`best_lead`。届くたびに選び直す)。
    lead: Option<Duration>,
    /// 届いて、まだ刻みで使われていない最後の値。
    unused: Option<Unused>,
    /// 前の刻みの時刻。
    last_tick: Option<Instant>,
    /// 前の刻みから、使われたか置き換わった値の数と、そのうち刻みに遅れた数。
    values: u32,
    late: u32,
    /// 値が次の刻みのどれだけ前に届くように促すか (`MIN_MARGIN`〜`MAX_MARGIN`。刻みに遅れた値の割合で増減する。T0864)。
    margin: Duration,
    /// 余裕を見直す区切りの始まりと、その間に使われたか置き換わった値の数と、そのうち起きる時刻の揺れで遅れた数 (`adjust_margin`)。
    judge_start: Option<Instant>,
    judged: u32,
    missed: u32,
    /// 遅れが無かった区切りが続いた数。
    calm: u32,
}

/// 送った促し。
#[derive(Debug, Clone, Copy)]
struct Pulse {
    seq: u32,
    at: Instant,
    /// 値を使わせたい刻みの時刻 (促しを寄せていなければ `None`)。
    target: Option<Instant>,
    /// 促しから値を使わせたい刻みまでの時間 (`aim`。促しを寄せていなければ `None`)。
    aim: Option<Duration>,
}

/// 届いて、まだ刻みで使われていない値。
#[derive(Debug, Clone, Copy)]
struct Unused {
    at: Instant,
    /// 促しが狙った刻み。
    target: Option<Instant>,
    /// 促してから届くまでの時間が見込み (`aim`) のうちだったか。そのうえで刻みに遅れたなら、起きる時刻の揺れで遅れた
    /// (余裕を増やせば間に合う)。
    expected: bool,
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
    /// この刻みの後の余裕 (`adjust_margin`)。促しを寄せていなければ (値が直近に届いていない) `None` (T0867)。
    pub margin: Option<Duration>,
}

impl Pacer {
    /// 値が次の刻みのどれだけ前に届くように促すか (余裕) の下限と上限 (T0864。`docs/studio.md#刻みの速さと正確さ`)。
    /// 余裕は、促しと刻みの起きる時刻の揺れ (OS のスリープの粒度) と、届いてから刻みのスレッドが読むまでの時間を見込む。
    /// 余裕の分だけ値は古くなるので、上限は刻みの間隔の半分 (120 刻みで約 4.2 ms) より小さくする。
    const MIN_MARGIN: Duration = Duration::from_millis(1);
    const MAX_MARGIN: Duration = Duration::from_millis(4);
    /// 余裕を見直す区切りの長さ。
    const MARGIN_WINDOW: Duration = Duration::from_secs(1);
    /// 1 つの区切りで、起きる時刻の揺れで遅れた値が 5% を超えたら増やす量。
    const MARGIN_UP: Duration = Duration::from_micros(500);
    /// 遅れた値の無い区切りが `MARGIN_CALM` 回続いたら減らす量 (増やすより小さくし、少しずつ戻す)。
    const MARGIN_DOWN: Duration = Duration::from_micros(250);
    const MARGIN_CALM: u32 = 2;
    /// 1 つの区切りで、これより少ない値しか無ければ見直さない (トラッキングを使っていない)。
    const MARGIN_MIN_VALUES: u32 = 10;
    /// 促してから届くまでの時間を見る長さ。これより長く届かなければ、促しを刻みの直後に戻す。
    pub const LEAD_WINDOW: Duration = Duration::from_secs(1);
    /// 促してから届くまでの時間を見る値の数の下限 (T0868。`docs/studio.md#入力`)。直近の約 1 秒の値がこれより少なければ
    /// (検出が重く 1 秒に数個しか届かない)、`LEAD_WINDOW` より古い値も残してこの数まで見る。1 秒に 2 個では、1 つの値の揺れで
    /// 寄せる先が決まり、それより遅い値が刻みに遅れる (遅れた値が 33〜50%。T0864 の e2e)。10 個あれば、まれに遅い 1 個 (10%) を
    /// 遅れさせるか、ほかを待たせて間に合わせるかを費用で比べられる。
    const MIN_LEADS: usize = 10;
    /// 数の下限のために残す値の古さの上限 (計算機の混み具合が変わった古い値をいつまでも使わない)。1 秒に 2 個でも 10 個そろう長さ。
    const LEAD_MAX_AGE: Duration = Duration::from_secs(5);
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
            pulses: VecDeque::with_capacity(Self::MAX_PULSES),
            leads: VecDeque::with_capacity(Self::MAX_LEADS),
            min_leads: Self::MIN_LEADS,
            lead: None,
            unused: None,
            last_tick: None,
            values: 0,
            late: 0,
            margin: Self::MIN_MARGIN,
            judge_start: None,
            judged: 0,
            missed: 0,
            calm: 0,
        }
    }

    /// 速さを変える。次の刻みは今から新しい間隔の後。
    pub fn set_rate(&mut self, rate: TickRate, now: Instant) {
        self.period = rate.period();
        self.next = now + self.period;
        self.lead = self.best_lead();
        // 余裕はそのまま (計算機の性質なので速さで変わらない)。見直しの区切りは数え直す。
        self.judge_start = None;
        self.judged = 0;
        self.missed = 0;
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
        self.lead.filter(|_| fresh).map(|lead| lead + self.margin)
    }

    /// `d` を `period` で割った余り。
    fn wrap(d: Duration, period: Duration) -> Duration {
        let period = period.as_nanos().max(1);
        // 余りは `period` (1 秒以下) より小さいので `u64` に収まる。
        Duration::from_nanos(u64::try_from(d.as_nanos() % period).unwrap_or(0))
    }

    /// 促してから届くまでの時間を、どれだけと見て促しを寄せるか。直近の値について、狙った刻みでの費用の合計が
    /// 最も小さくなるものを、届いた時間の中から選ぶ。
    ///
    /// 費用は、間に合う値では刻みで使われるまでの待ち。促しを寄せた先 `c` と届くまでの時間 `d` から `(c + 余裕 - d) mod 間隔`。
    /// 狙った刻みに遅れる値 (`d > c + 余裕`) は、その待ちに 1 刻みの間隔を足す (狙った刻みが新しい値を使えず、前の値を
    /// もう 1 刻み出す。T0864)。足さないと、遅れて次の刻みのすぐ前に届く値の費用が小さく見え、揺れが大きいときに
    /// 多くの値を遅れさせる寄せ方を選ぶ (T0863 の測定で 40%)。
    ///
    /// 最も長いものに合わせると、刻みの間隔より長くかかった 1 回 (計算機が混んだときなど) で寄せる先が 1 刻み近く回り、
    /// ほかの値が刻みに遅れて 1 刻み待つ。まれに遅いものは遅れさせ、多くが遅いなら間に合わせる。
    fn best_lead(&self) -> Option<Duration> {
        let period = self.period.as_nanos().max(1);
        let cost = |c: Duration, d: Duration| {
            let aim = c + self.margin;
            let wait = (aim + self.period * 2)
                .as_nanos()
                .saturating_sub(Self::wrap(d, self.period).as_nanos())
                % period;
            if d > aim { wait + period } else { wait }
        };
        self.leads
            .iter()
            .map(|(_, c)| *c)
            .min_by_key(|c| self.leads.iter().map(|(_, d)| cost(*c, *d)).sum::<u128>())
    }

    /// `now` に促した。促しに付ける番号を返す。
    pub fn pulse(&mut self, now: Instant) -> u32 {
        // 狙う刻みは、予定した促しの時刻 (次の刻みの `pulse_offset` 前) から `aim` 後。格子の上の刻みになる
        // (起きるのが遅れても狙いはずれない)。
        let aim = self.aim(now);
        let target = aim.map(|aim| self.next + aim.saturating_sub(self.pulse_offset(now)));
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
            aim,
        });
        seq
    }

    /// 番号 `pulse` の促しで検出したトラッキングの値が `at` に届いた。促してから届くまでの時間を覚える。
    /// 覚えていない番号 (古すぎる・送っていない) と、促しより前の時刻は寄せる量に使わない
    /// (刻みで使われるまでの待ちには数える。狙った刻みが分からないので遅れには数えない)。
    pub fn arrived(&mut self, at: Instant, pulse: u32) {
        let sent = self.pulses.iter().rev().find(|p| p.seq == pulse).copied();
        let value = Unused {
            at,
            target: sent.and_then(|p| p.target),
            expected: sent.is_some_and(|p| {
                p.aim
                    .is_some_and(|aim| at.saturating_duration_since(p.at) <= aim)
            }),
        };
        if let Some(replaced) = self.unused.replace(value) {
            // 刻みで使われないまま置き換わった。
            self.settle(replaced);
        }
        let Some(Pulse { at: pulse, .. }) = sent else {
            return;
        };
        if at < pulse {
            return;
        }
        if self.leads.len() >= Self::MAX_LEADS {
            self.leads.pop_front();
        }
        self.leads.push_back((at, at - pulse));
        self.prune_leads(at);
        self.lead = self.best_lead();
    }

    /// 見る値を決める (`at` は最後に届いた時刻)。`LEAD_WINDOW` のうちの値はすべて、それより古い値は `min_leads` 個に足りない分だけ
    /// `LEAD_MAX_AGE` のうちのものを残す。
    fn prune_leads(&mut self, at: Instant) {
        while let Some(&(t, _)) = self.leads.front() {
            let age = at.saturating_duration_since(t);
            let keep = age <= Self::LEAD_WINDOW
                || (age <= Self::LEAD_MAX_AGE && self.leads.len() <= self.min_leads);
            if keep {
                break;
            }
            self.leads.pop_front();
        }
    }

    /// 値を使ったか置き換えた。狙った刻み (の半刻み前より後) を前の刻みで過ぎていれば遅れ。
    fn settle(&mut self, value: Unused) {
        self.values += 1;
        let late = value
            .target
            .zip(self.last_tick)
            .is_some_and(|(target, last)| last + self.period / 2 >= target);
        self.late += u32::from(late);
        self.judged += 1;
        self.missed += u32::from(late && value.expected);
    }

    /// 余裕を見直す (区切りごと。`now` は刻みの時刻)。
    ///
    /// 数えるのは、届くまでの時間は見込みのうちなのに刻みに遅れた値 (促しが遅く起きた・刻みが早く起きたなど、起きる時刻の揺れで
    /// 遅れたもの)。届くまでの時間が見込みより長くて遅れた値は寄せる先の選び方 (`best_lead`) が受け持つので、余裕では数えない
    /// (まれにとても遅い値で余裕を上限まで増やさない)。5% を超えたら増やし、遅れの無い区切りが続いたら少しずつ戻す
    /// (余裕の分だけ値は古くなる)。
    fn adjust_margin(&mut self, now: Instant) {
        let start = *self.judge_start.get_or_insert(now);
        if now.saturating_duration_since(start) < Self::MARGIN_WINDOW {
            return;
        }
        let mut margin = self.margin;
        if self.judged >= Self::MARGIN_MIN_VALUES {
            if self.missed.saturating_mul(20) > self.judged {
                self.calm = 0;
                margin = (margin + Self::MARGIN_UP).min(Self::MAX_MARGIN);
            } else if self.missed == 0 {
                self.calm += 1;
                if self.calm >= Self::MARGIN_CALM {
                    self.calm = 0;
                    margin = margin
                        .saturating_sub(Self::MARGIN_DOWN)
                        .max(Self::MIN_MARGIN);
                }
            } else {
                self.calm = 0;
            }
        }
        self.judge_start = Some(now);
        self.judged = 0;
        self.missed = 0;
        if margin != self.margin {
            self.margin = margin;
            self.lead = self.best_lead();
        }
    }

    /// `now` まで待った (起きた) ので刻む。次の刻みの時刻を進め、この刻みで使った値と今の余裕を返す。
    /// この刻みの前に届いた値 (この刻みが使った値) は、先に `arrived` で渡しておく。
    pub fn tick(&mut self, now: Instant) -> Ticked {
        let wait = self.unused.take().map(|value| {
            self.settle(value);
            // 刻みの中で届いた値 (刻みの時刻より後) の待ちは 0。
            now.saturating_duration_since(value.at)
        });
        self.adjust_margin(now);
        let ticked = Ticked {
            wait,
            values: self.values,
            late: self.late,
            margin: self.aim(now).map(|_| self.margin),
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
    /// 最後の刻みの余裕 (`Ticked::margin`)。
    margin: Option<Duration>,
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
    /// 集め終えた刻みの余裕 (ミリ秒。`Ticked::margin`)。促しを寄せていなければ無し (T0867)。
    pub margin_ms: Option<f32>,
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
        self.margin = ticked.margin;
        if now.saturating_duration_since(start) >= Self::WINDOW {
            // 約 1 秒の間に値が 1 つも使われなければ、無し (トラッキングを使っていない)。
            self.measured = (self.used > 0).then(|| TrackingWait {
                mean_ms: self.total.as_secs_f32() * 1e3 / self.used as f32,
                max_ms: self.max.as_secs_f32() * 1e3,
                late: self.late as f32 / self.values.max(1) as f32,
                margin_ms: self.margin.map(|m| m.as_secs_f32() * 1e3),
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

    /// 仮の時刻で回した結果 (`simulate`)。
    struct Run {
        pacer: Pacer,
        /// 刻んだ時刻。
        ticks: Vec<Instant>,
        /// 刻みの前の、次の刻みの予定の時刻 (`Pacer::deadline`)。
        deadlines: Vec<Instant>,
        /// 値が届いてから刻みで使われるまでの待ち。
        waits: Vec<Duration>,
        /// 待ちと、使った刻みの時刻。
        used: Vec<(Instant, Duration)>,
        /// 刻みごとの、使われたか置き換わった値の数と、そのうち遅れた数 (`Pacer::tick`)。
        values: Vec<(Instant, u32, u32)>,
        /// 回している間の余裕の最大。
        max_margin: Duration,
        /// 刻みのスレッドと同じく刻みごとに集めた待ち (負荷の報告。T0867)。
        meter: WaitMeter,
    }

    impl Run {
        /// `from` から `to` までの刻みで数えた、遅れた値の割合と、使われた値の待ちの平均。
        fn between(&self, from: Instant, to: Instant) -> (f64, Duration) {
            let within = |t: &Instant| *t >= from && *t < to;
            let (values, late) = self
                .values
                .iter()
                .filter(|(t, _, _)| within(t))
                .fold((0, 0), |(v, l), (_, values, late)| (v + values, l + late));
            let waits: Vec<Duration> = self
                .used
                .iter()
                .filter(|(t, _)| within(t))
                .map(|(_, w)| *w)
                .collect();
            let mean = waits.iter().sum::<Duration>() / (waits.len().max(1) as u32);
            (f64::from(late) / f64::from(values.max(1)), mean)
        }

        /// 遅れた値の数の合計。
        fn late(&self) -> usize {
            self.values.iter().map(|(_, _, late)| *late as usize).sum()
        }

        /// `from` からの刻みで数えた、遅れた値の割合。
        fn late_ratio(&self, from: Instant) -> f64 {
            let (values, late) = self
                .values
                .iter()
                .filter(|(t, _, _)| *t >= from)
                .fold((0, 0), |(v, l), (_, values, late)| (v + values, l + late));
            f64::from(late) / f64::from(values.max(1))
        }
    }

    /// 刻みのスレッドを仮の時刻で回す。促しから `cost` 後に値が届く (操作ウィンドウは促しを受けて検出する)。
    /// `cost` が `None` の促しでは値が届かない。`tick` が返す待ちは外から測った待ちと等しいことを確かめる。
    fn simulate(
        rate: TickRate,
        start: Instant,
        until: Instant,
        cost: impl FnMut(Instant) -> Option<Duration>,
    ) -> Run {
        simulate_with(Pacer::new(rate, start), start, until, cost, || {
            Duration::ZERO
        })
    }

    /// `simulate` で、促しと刻みのたびに、スリープが予定より `oversleep` だけ遅れて起きる (OS のスリープの粒度を模す。T0864)。
    fn simulate_with(
        mut pacer: Pacer,
        start: Instant,
        until: Instant,
        mut cost: impl FnMut(Instant) -> Option<Duration>,
        mut oversleep: impl FnMut() -> Duration,
    ) -> Run {
        let mut run = Run {
            pacer: pacer.clone(),
            ticks: Vec::new(),
            deadlines: Vec::new(),
            waits: Vec::new(),
            used: Vec::new(),
            values: Vec::new(),
            max_margin: pacer.margin,
            meter: WaitMeter::default(),
        };
        // 届く前の値 (届く時刻と促しの番号。届く時刻の順)。
        let mut pending: VecDeque<(Instant, u32)> = VecDeque::new();
        let mut now = start;
        for _ in 0..1_000_000 {
            let (at, wake) = pacer.wake(now);
            if at > now {
                now = at + oversleep();
            }
            if now >= until {
                break;
            }
            match wake {
                Wake::Pulse => {
                    let seq = pacer.pulse(now);
                    if let Some(cost) = cost(now) {
                        let arrival = now + cost;
                        let i = pending.partition_point(|(a, _)| *a <= arrival);
                        pending.insert(i, (arrival, seq));
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
                        run.waits.push(now - arrived);
                        run.used.push((now, now - arrived));
                    }
                    run.deadlines.push(pacer.deadline());
                    let ticked = pacer.tick(now);
                    assert_eq!(ticked.wait, used.map(|a| now - a));
                    // 余裕は寄せているときだけ報告する。
                    if let Some(margin) = ticked.margin {
                        assert_eq!(margin, pacer.margin);
                    }
                    run.meter.record(now, ticked);
                    run.values.push((now, ticked.values, ticked.late));
                    run.ticks.push(now);
                    run.max_margin = run.max_margin.max(pacer.margin);
                }
            }
        }
        run.pacer = pacer;
        run
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
                let run = simulate(rate, start, until, |_| Some(cost));
                let (ticks, waits) = (&run.ticks, &run.waits);
                // 刻みは格子のまま (速さは設定を超えず、`dt` も揺れない)。
                for pair in ticks.windows(2) {
                    assert_eq!(pair[1] - pair[0], period, "{rate:?} {cost_ms} ms");
                }
                assert!(ticks.len() as u32 <= rate.hz() * 3, "{rate:?}");
                // 寄せた後は、値は刻みの余裕 (揺れなければ下限のまま) 前に届く。
                let settled = &waits[waits.len() / 2..];
                assert!(!settled.is_empty());
                for wait in settled {
                    assert!(
                        wait.abs_diff(Pacer::MIN_MARGIN) < Duration::from_micros(1),
                        "{rate:?} {cost_ms} ms: {wait:?}"
                    );
                }
                // 揺れなければ、刻みに遅れる値は無い。
                assert_eq!(run.late(), 0, "{rate:?} {cost_ms} ms");
                assert_eq!(run.max_margin, Pacer::MIN_MARGIN);
            }
        }
    }

    #[test]
    fn pulse_returns_after_the_tick_when_tracking_stops() {
        let start = Instant::now();
        let rate = TickRate::Fps60;
        let stop = start + Duration::from_secs(1);
        let until = stop + Pacer::LEAD_WINDOW + Duration::from_millis(100);
        let run = simulate(rate, start, until, |now| {
            (now < stop).then_some(Duration::from_millis(5))
        });
        let last = *run.ticks.last().unwrap();
        assert_eq!(run.pacer.pulse_offset(last), rate.period());
        // 届いている間に寄せた促しは、届かなくなっても刻みの格子を動かさない。
        for pair in run.ticks.windows(2) {
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
        // 届いていなければ何も出さない (促しを寄せないので余裕も出さない)。
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
            margin: Some(Pacer::MIN_MARGIN),
        };
        assert_eq!(pacer.tick(tick), used);
        // 寄せた促し (次の刻みの 7 ms 前) の値が 6 ms で届けば、刻みの 1 ms 前に届いて間に合う。
        let (pulse, wake) = pacer.wake(tick);
        assert_eq!(wake, Wake::Pulse);
        let seq = pacer.pulse(pulse);
        pacer.arrived(pulse + Duration::from_millis(6), seq);
        let tick = tick + period;
        let used = Ticked {
            wait: Some(Pacer::MIN_MARGIN),
            values: 1,
            late: 0,
            margin: Some(Pacer::MIN_MARGIN),
        };
        assert_eq!(pacer.tick(tick), used);
        // 混んで 9 ms かかると、狙った刻みに遅れる。その刻みは新しい値を使わない。
        // (届いた後は、遅れる値の多い寄せ方を避けて 9 ms に合わせ直す。`best_lead`)
        let (pulse, _) = pacer.wake(tick);
        let slow = pacer.pulse(pulse);
        let missed = tick + period;
        assert_eq!(
            pacer.tick(missed),
            Ticked {
                margin: Some(Pacer::MIN_MARGIN),
                ..Ticked::default()
            }
        );
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
            margin: Some(Pacer::MIN_MARGIN),
        };
        assert_eq!(pacer.tick(tick), used);
        // 覚えていない番号の値も、待ちには数える (遅れには数えない)。
        pacer.arrived(tick + Duration::from_millis(3), u32::MAX);
        let next = tick + period;
        let used = Ticked {
            wait: Some(period - Duration::from_millis(3)),
            values: 1,
            late: 0,
            margin: Some(Pacer::MIN_MARGIN),
        };
        assert_eq!(pacer.tick(next), used);
    }

    #[test]
    fn late_values_are_counted_when_detection_sometimes_misses_the_tick() {
        let start = Instant::now();
        let until = start + Duration::from_secs(3);
        let rate = TickRate::Fps60;
        let mut pulses = 0u32;
        // 30 回に 1 回、検出と IPC が 3 ms 長くかかる (余裕の 1 ms を超えて刻みに遅れる)。まれなので、
        // ほかの値を 3 ms 待たせて間に合わせるより、遅れさせる (`best_lead`)。
        let run = simulate(rate, start, until, |_| {
            pulses += 1;
            Some(Duration::from_millis(if pulses.is_multiple_of(30) {
                9
            } else {
                6
            }))
        });
        let slow = (pulses / 30) as usize;
        let late = run.late();
        // 遅い値をどれも遅れに数える。
        assert!(slow - 1 <= late && late <= slow, "{late} / {slow}");
        // 遅れた値は次の値に置き換わるので、使われた値の待ちは余裕のまま。
        let settled = &run.waits[run.waits.len() / 2..];
        assert!(
            settled
                .iter()
                .all(|w| w.abs_diff(Pacer::MIN_MARGIN) < Duration::from_micros(1))
        );
        // 届くまでの時間が見込みより長くて遅れた値では、余裕を増やさない (起きる時刻の揺れではない)。
        assert_eq!(run.max_margin, Pacer::MIN_MARGIN);
    }

    /// 仮の時刻のテストの乱数 (xorshift。同じ種なら同じ列)。
    struct Rng(u64);

    impl Rng {
        /// `0` 以上 `max` 以下。
        fn upto(&mut self, max: Duration) -> Duration {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            max.mul_f64((self.0 >> 11) as f64 / (1u64 << 53) as f64)
        }
    }

    /// 余裕を見直さない (T0864 の前と同じ 1 ms のまま) `Pacer`。比べるために使う。
    fn fixed_margin(rate: TickRate, start: Instant) -> Pacer {
        let mut pacer = Pacer::new(rate, start);
        // 見直しの区切りが始まらない (始まりが遠い先)。
        pacer.judge_start = Some(start + Duration::from_secs(3600));
        pacer
    }

    /// 届くまでの時間を直近の約 1 秒の値だけで見る (T0868 の前と同じ) `Pacer`。比べるために使う。
    fn time_window_only(rate: TickRate, start: Instant) -> Pacer {
        let mut pacer = Pacer::new(rate, start);
        pacer.min_leads = 0;
        pacer
    }

    /// 刻みは格子より早くならず、速さは設定を超えない。
    fn assert_keeps_the_rate(run: &Run, rate: TickRate, seconds: u32, label: &str) {
        for (tick, deadline) in run.ticks.iter().zip(&run.deadlines) {
            assert!(tick >= deadline, "{label}");
        }
        for pair in run.deadlines.windows(2) {
            assert!(pair[1] - pair[0] >= rate.period(), "{label}");
        }
        assert!(run.ticks.len() as u32 <= rate.hz() * seconds, "{label}");
    }

    #[test]
    fn margin_grows_when_sleeps_wake_late() {
        let start = Instant::now();
        let seconds = 20;
        let until = start + Duration::from_secs(u64::from(seconds));
        let settled = start + Duration::from_secs(10);
        for rate in TickRate::ALL {
            // スリープが 0〜2 ms 遅れる機械と、0〜15 ms 遅れる機械 (粒度の粗い OS)。検出と IPC は揺れない。
            for late_ms in [2, 15] {
                let label = format!("{rate:?} {late_ms} ms");
                let max = Duration::from_millis(late_ms);
                let cost = |_| Some(Duration::from_millis(6));
                let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
                let before = simulate_with(fixed_margin(rate, start), start, until, cost, || {
                    rng.upto(max)
                });
                let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
                let after = simulate_with(Pacer::new(rate, start), start, until, cost, || {
                    rng.upto(max)
                });
                let (before_late, after_late) =
                    (before.late_ratio(settled), after.late_ratio(settled));
                println!(
                    "{label}: late {:.1}% -> {:.1}%, margin {:?}",
                    before_late * 100.0,
                    after_late * 100.0,
                    after.pacer.margin
                );
                // 余裕が増えて、遅れた値の割合が下がる。上限は超えない。
                assert!(after.pacer.margin > Pacer::MIN_MARGIN, "{label}");
                // 増えた余裕が負荷の報告に出る (T0867)。
                let reported = after.meter.measured().and_then(|w| w.margin_ms);
                assert!(
                    reported.is_some_and(|m| m > 1.0 && m <= 4.0 + 1e-3),
                    "{label}: {reported:?}"
                );
                assert!(after.max_margin <= Pacer::MAX_MARGIN, "{label}");
                assert!(after_late < before_late, "{label}");
                if late_ms == 2 {
                    // 余裕の上限で収まる揺れなら、遅れはほとんど無くなる。
                    assert!(after_late < 0.05, "{label}");
                } else {
                    // 上限より大きく揺れるなら、余裕は上限で止まる (遅れは減るが残る。刻みの間隔が空いた注意の出る計算機)。
                    assert_eq!(after.max_margin, Pacer::MAX_MARGIN, "{label}");
                }
                assert_keeps_the_rate(&before, rate, seconds, &label);
                assert_keeps_the_rate(&after, rate, seconds, &label);
            }
        }
    }

    #[test]
    fn margin_returns_when_sleeps_stop_waking_late() {
        let start = Instant::now();
        let rate = TickRate::Fps60;
        let calm = start + Duration::from_secs(10);
        let until = calm + Duration::from_secs(40);
        let mut rng = Rng(7);
        let now = std::cell::Cell::new(start);
        // 10 秒の間スリープが 0〜2 ms 遅れ、その後は遅れない。
        let run = simulate_with(
            Pacer::new(rate, start),
            start,
            until,
            |at| {
                now.set(at);
                Some(Duration::from_millis(6))
            },
            || {
                if now.get() < calm {
                    rng.upto(Duration::from_millis(2))
                } else {
                    Duration::ZERO
                }
            },
        );
        assert!(run.max_margin > Pacer::MIN_MARGIN);
        assert_eq!(run.pacer.margin, Pacer::MIN_MARGIN);
        assert_eq!(run.late_ratio(calm + Duration::from_secs(1)), 0.0);
        // 戻った後は、値は刻みの 1 ms 前に届く。
        let last = run.waits[run.waits.len() - 10..].to_vec();
        assert!(
            last.iter()
                .all(|w| w.abs_diff(Pacer::MIN_MARGIN) < Duration::from_micros(1)),
            "{last:?}"
        );
    }

    #[test]
    fn widely_varying_detection_does_not_leave_many_values_late() {
        // T0863 の測定 (混んだ計算機で遅れた値 40%) を模す: 検出と IPC が 6 割は 3 ms、4 割は 14 ms。
        let start = Instant::now();
        let seconds = 10;
        let until = start + Duration::from_secs(u64::from(seconds));
        for rate in TickRate::ALL {
            let label = format!("{rate:?}");
            let mut rng = Rng(42);
            let run = simulate(rate, start, until, |_| {
                Some(Duration::from_millis(
                    if rng.upto(Duration::from_millis(10)) < Duration::from_millis(4) {
                        14
                    } else {
                        3
                    },
                ))
            });
            let late = run.late_ratio(start + Duration::from_secs(2));
            let settled = &run.waits[run.waits.len() / 5..];
            let mean = settled.iter().sum::<Duration>() / settled.len() as u32;
            println!("{label}: late {:.1}%, wait {mean:?}", late * 100.0);
            // 遅い値に合わせて寄せる (速い値は待つが、狙った刻みはどれも新しい値を使う)。
            assert!(late < 0.05, "{label}: {late}");
            // 届くまでの時間の揺れでは余裕を増やさない。
            assert_eq!(run.max_margin, Pacer::MIN_MARGIN, "{label}");
            let reported = run.meter.measured().and_then(|w| w.margin_ms);
            assert!(
                reported.is_some_and(|m| (m - 1.0).abs() < 1e-3),
                "{label}: {reported:?}"
            );
            assert_keeps_the_rate(&run, rate, seconds, &label);
        }
    }

    /// 検出の重い操作ウィンドウ (`pulse.ts` の `Pulse` と、新しい映像のフレームがあるときだけ検出する `FaceTracker`) を模す。
    /// 映像のフレームは `frame` ごと。促しを受けると、前の検出が終わってから前の検出にかかった時間が経つまでは飛ばし、
    /// 新しいフレームがあれば `detect` + 0〜`jitter` かけて検出し、値を送る (T0868)。
    struct Detector {
        start: Instant,
        frame: Duration,
        detect: Duration,
        jitter: Duration,
        rng: Rng,
        /// 前の検出が終わった時刻と、かかった時間。
        last_end: Option<Instant>,
        last_cost: Duration,
        /// 前に検出したフレームの番号。
        last_frame: Option<u128>,
    }

    impl Detector {
        fn new(start: Instant, frame_ms: u64, detect_ms: u64, jitter_ms: u64, seed: u64) -> Self {
            Self {
                start,
                frame: Duration::from_millis(frame_ms),
                detect: Duration::from_millis(detect_ms),
                jitter: Duration::from_millis(jitter_ms),
                rng: Rng(seed),
                last_end: None,
                last_cost: Duration::ZERO,
                last_frame: None,
            }
        }

        /// `now` の促しで送る値の、促しから届くまでの時間 (送らなければ `None`)。
        fn pulse(&mut self, now: Instant) -> Option<Duration> {
            if let Some(end) = self.last_end {
                // 検出の途中に届いた促しは、検出が終わってから扱われる (WebView に溜まる)。
                if now < end || now - end < self.last_cost {
                    return None;
                }
            }
            // 最初のフレームは少しずらす (刻みの格子と揃わないように)。
            let frame = (now - self.start + self.frame / 3).as_nanos() / self.frame.as_nanos();
            if self.last_frame.is_some_and(|last| last >= frame) {
                self.last_end = Some(now);
                self.last_cost = Duration::ZERO;
                return None;
            }
            let cost = self.detect + self.rng.upto(self.jitter);
            self.last_frame = Some(frame);
            self.last_end = Some(now + cost);
            self.last_cost = cost;
            Some(cost)
        }
    }

    #[test]
    fn few_values_from_slow_detection_are_rarely_late() {
        // T0864 の e2e (T0812 の手順。検出 200 ms・カメラ 2 fps) で遅れた値が 33〜50% だったものを模す。
        let start = Instant::now();
        let seconds = 60;
        let until = start + Duration::from_secs(u64::from(seconds));
        let settled = start + Duration::from_secs(10);
        for rate in TickRate::ALL {
            // (フレームの間隔, 検出, 検出の揺れ) ミリ秒。
            for (frame, detect, jitter) in [(500, 200, 8), (500, 200, 20), (200, 80, 20)] {
                let label = format!("{rate:?} frame {frame} detect {detect}+0..{jitter} ms");
                let run = |pacer| {
                    let mut detector = Detector::new(start, frame, detect, jitter, 11);
                    let mut sleep = Rng(5);
                    simulate_with(
                        pacer,
                        start,
                        until,
                        |now| detector.pulse(now),
                        || sleep.upto(Duration::from_micros(500)),
                    )
                };
                let before = run(time_window_only(rate, start));
                let after = run(Pacer::new(rate, start));
                let (before_late, after_late) =
                    (before.late_ratio(settled), after.late_ratio(settled));
                let values: u32 = after.values.iter().map(|(_, v, _)| *v).sum();
                println!(
                    "{label}: {:.1} values/s, late {:.1}% -> {:.1}%",
                    f64::from(values) / f64::from(seconds),
                    before_late * 100.0,
                    after_late * 100.0
                );
                assert!(after_late < before_late, "{label}");
                assert!(after_late < 0.15, "{label}");
                assert_keeps_the_rate(&before, rate, seconds, &label);
                assert_keeps_the_rate(&after, rate, seconds, &label);
            }
        }
    }

    #[test]
    fn old_leads_are_dropped_when_detection_changes() {
        // 10 秒たったところで、検出の重さや値の間隔が変わる (T0868)。
        let start = Instant::now();
        let switch = start + Duration::from_secs(10);
        let seconds = 70;
        let until = switch + Duration::from_secs(60);
        let rate = TickRate::Fps60;
        // (前のフレームの間隔・検出・揺れ, 後のフレームの間隔・検出・揺れ) ミリ秒。
        let cases = [
            // 軽い検出 (60 fps のカメラ・6 ms) から重い検出 (2 fps・200 ms) へ: 値の間隔が急に長くなる。
            ((16, 6, 0), (500, 200, 8)),
            ((100, 30, 8), (500, 205, 8)),
            // 重い検出から軽い検出へ: 値の間隔が急に短くなる。
            ((500, 200, 8), (16, 6, 0)),
            ((500, 205, 8), (200, 53, 8)),
            // 値の間隔は同じ (2 fps) まま、検出が重くなる・軽くなる。
            ((500, 100, 8), (500, 200, 8)),
            ((500, 205, 8), (500, 103, 8)),
        ];
        for (from, to) in cases {
            let label = format!("{from:?} -> {to:?}");
            let run = |pacer: Pacer, until: Instant, switch: Instant| {
                let mut first = Detector::new(start, from.0, from.1, from.2, 3);
                let mut second = Detector::new(start, to.0, to.1, to.2, 3);
                let mut sleep = Rng(9);
                simulate_with(
                    pacer,
                    start,
                    until,
                    |now| {
                        if now < switch {
                            first.pulse(now)
                        } else {
                            second.pulse(now)
                        }
                    },
                    || sleep.upto(Duration::from_micros(500)),
                )
            };
            // 後の値が 1 秒に `MIN_LEADS` 個より多ければ、約 1 秒 (と 1 フレーム) の後には前の値を見ない。
            // 少なくても、古さの上限の後には見ない。
            let dropped = if to.0 <= 100 {
                Pacer::LEAD_WINDOW + Duration::from_millis(to.0)
            } else {
                Pacer::LEAD_MAX_AGE
            };
            let short = run(Pacer::new(rate, start), switch + dropped, switch);
            let oldest = short.pacer.leads.front().map(|(t, _)| *t);
            assert!(oldest.is_some_and(|t| t >= switch), "{label}: {oldest:?}");
            // その後の遅れた値の割合と待ちは、初めから後の検出だったときと同じほど (前の検出に引きずられていない)。
            let changed = run(Pacer::new(rate, start), until, switch);
            let steady = run(Pacer::new(rate, start), until, start);
            let (late, wait) = changed.between(switch + dropped, until);
            let (steady_late, steady_wait) = steady.between(switch + dropped, until);
            // 変わってすぐ (前の値も見ている間) を、T0868 の前 (直近の約 1 秒だけ見る) と比べる (確かめるのは上の 2 つ)。
            let (moving, moving_wait) = changed.between(switch, switch + dropped);
            let (window, window_wait) =
                run(time_window_only(rate, start), until, switch).between(switch, switch + dropped);
            println!(
                "{label}: first {dropped:?}: late {:.1}% wait {moving_wait:.2?} (1 s window: {:.1}% {window_wait:.2?}); \
                 then late {:.1}% wait {wait:.2?} (steady {:.1}% {steady_wait:.2?})",
                moving * 100.0,
                window * 100.0,
                late * 100.0,
                steady_late * 100.0,
            );
            assert!(
                late <= steady_late + 0.05,
                "{label}: {late} / {steady_late}"
            );
            assert!(
                wait <= steady_wait + Duration::from_millis(1),
                "{label}: {wait:?} / {steady_wait:?}"
            );
            assert_keeps_the_rate(&changed, rate, seconds, &label);
        }
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
                    // 余裕は途中で 1 ms から 1.5 ms に増えた。集め終えた刻みの値を出す。
                    margin: Some(Duration::from_micros(if i < 30 { 1000 } else { 1500 })),
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
        assert!(
            measured.margin_ms.is_some_and(|m| (m - 1.5).abs() < 1e-3),
            "{measured:?}"
        );
        // 値は使われたが、促しを寄せていない (番号の無い値) なら余裕は無し。
        for _ in 0..60 {
            t += period;
            meter.record(
                t,
                Ticked {
                    wait: Some(Duration::from_millis(1)),
                    values: 1,
                    late: 0,
                    margin: None,
                },
            );
        }
        let measured = meter.measured().unwrap();
        assert_eq!(measured.margin_ms, None);
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
                margin: None,
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
