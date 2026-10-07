//! 優先度付きのモーションマネージャ (クロスフェード)。
//! 仕様: `docs/spec/motion.md#モーションマネージャ`。

use alloc::string::String;
use alloc::vec::Vec;
use ugoe_model::PartId;
use ugoe_model::motion::{CurveTarget, Motion, MotionEvent};
use ugoe_runtime::ParameterValues;

use crate::MotionPlayer;

/// モーションの優先度。`Idle < Normal < Force`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// 待機モーション。ほかのモーションが再生中なら開始しない。
    Idle,
    /// アプリの操作などで再生する普通のモーション。
    Normal,
    /// 常に開始する。
    Force,
}

/// 発火したイベントを発火したモーション ([`MotionManager::fired_event_sources`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSource {
    /// モーションを始めた番号 ([`MotionManager::motion_numbers`])。
    pub number: u64,
    /// モーションの名前 ([`Motion::name`])。
    pub name: String,
}

/// 優先度とクロスフェードを持つモーションの再生管理。
///
/// 新しいモーションを開始すると、再生中のモーションは [`MotionPlayer::stop`] で自身の `fade_out`
/// をかけてフェードアウトし、新しいモーションは自身の `fade_in` をかけてフェードインする。
/// 適用は開始した順 (古いものが先) に `lerp` で重ねるので、重なっている間はクロスフェードになる。
#[derive(Debug, Default)]
pub struct MotionManager {
    /// 再生中のプレイヤー (フェードアウト中のものを含む)。開始した順。
    players: Vec<MotionPlayer>,
    /// `players` の各モーションを始めた番号 (`players` と同じ並び)。
    numbers: Vec<u64>,
    /// これまでに始めたモーションの数 (最後に始めたモーションの番号)。
    started: u64,
    /// 現在のモーション (最後に開始し、止めていないもの) の優先度。なければ `None`。
    priority: Option<Priority>,
    /// 直前の [`update`](Self::update) で発火したイベント。
    events: Vec<MotionEvent>,
    /// `events` の各イベントを発火したモーション (`events` と同じ並び)。
    sources: Vec<EventSource>,
}

impl MotionManager {
    /// モーションを再生していないマネージャを作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// `priority` でモーションを開始する。現在のモーションより低い優先度なら開始せず `false` を返す。
    pub fn start(&mut self, motion: Motion, priority: Priority) -> bool {
        if self.priority.is_some_and(|current| priority < current) {
            return false;
        }
        for player in &mut self.players {
            player.stop();
        }
        self.players.push(MotionPlayer::new(motion));
        self.started += 1;
        self.numbers.push(self.started);
        self.priority = Some(priority);
        true
    }

    /// すべてのモーションをフェードアウトさせる。以降はどの優先度でも開始できる。
    pub fn stop_all(&mut self) {
        for player in &mut self.players {
            player.stop();
        }
        self.priority = None;
    }

    /// 現在のモーションの優先度。現在のモーションが終わるか止めると `None` になる。
    pub fn current_priority(&self) -> Option<Priority> {
        self.priority
    }

    /// 現在のモーション (止めていないもの) が再生中か。
    pub fn is_playing(&self) -> bool {
        self.priority.is_some()
    }

    /// 再生中のプレイヤー (フェードアウト中のものを含む)。開始した順。
    pub fn players(&self) -> &[MotionPlayer] {
        &self.players
    }

    /// [`players`](Self::players) の各モーションを始めた番号 (同じ並び)。
    ///
    /// 番号は [`start`](Self::start) で始めるたびに 1 から 1 ずつ上がる (始めなかった要求では上がらない)。
    /// 同じモーションを始め直したことや、イベントを発火したモーションを見分けるのに使う。
    pub fn motion_numbers(&self) -> &[u64] {
        &self.numbers
    }

    /// 現在のモーション (最後に開始し、止めていないもの) を始めた番号。なければ `None`。
    pub fn current_motion_number(&self) -> Option<u64> {
        self.priority.and(self.numbers.last().copied())
    }

    /// `dt` 秒進め、開始した順にパラメータへ適用する。終わったプレイヤーは取り除く。
    pub fn update(&mut self, parameters: &mut ParameterValues, dt: f32) {
        self.events.clear();
        self.sources.clear();
        for (player, &number) in self.players.iter_mut().zip(&self.numbers) {
            player.advance(dt);
            for event in player.fired_events() {
                self.events.push(event.clone());
                self.sources.push(EventSource {
                    number,
                    name: player.motion().name.clone(),
                });
            }
            player.apply(parameters, 1.0);
        }
        if self.players.last().is_some_and(MotionPlayer::is_finished) {
            self.priority = None;
        }
        let mut finished = self.players.iter().map(MotionPlayer::is_finished);
        self.numbers.retain(|_| finished.next() != Some(true));
        self.players.retain(|p| !p.is_finished());
    }

    /// 直前の [`update`](Self::update) で発火したイベント。開始した順のプレイヤーごとに、発火した順に並ぶ。
    pub fn fired_events(&self) -> &[MotionEvent] {
        &self.events
    }

    /// [`fired_events`](Self::fired_events) の各イベントを発火したモーション (同じ並び)。
    ///
    /// フェードアウト中のモーションや、この `update` で終わったモーションのイベントも、そのモーションを指す。
    pub fn fired_event_sources(&self) -> &[EventSource] {
        &self.sources
    }

    /// エフェクトグループ `name` の強さ (`docs/spec/motion.md#エフェクトグループ`)。
    ///
    /// 1 から始め、開始した順に各モーションのカーブの値を `lerp(s, value, w)` で重ねる
    /// (`w` はカーブのフェードの重み)。対象のカーブがなければ 1。
    pub fn effect_group_weight(&self, name: &str) -> f32 {
        let mut s = 1.0;
        let is_group =
            |target: &CurveTarget| matches!(target, CurveTarget::EffectGroup(g) if g == name);
        for player in &self.players {
            for (_, _, value, w) in player.curve_values_where(is_group) {
                s += (value - s) * w;
            }
        }
        s
    }

    /// パーツの不透明度のカーブの値 `(パーツ, 値, 重み)`。パーツごとに 1 つ、最初に現れた順に並ぶ。
    ///
    /// 開始した順に各モーションのカーブを重ねる。値は `lerp(v, value, w)`、重みは
    /// `1 - (1 - W)(1 - w)` で重ねる (どちらも最初は 0)。呼び出し側は、パーツ自身の不透明度 `o` を
    /// `lerp(o, 値, 重み)` で上書きする。
    pub fn part_opacities(&self) -> Vec<(PartId, f32, f32)> {
        self.part_opacities_in(&mut PartOpacityBuffer::default())
            .map(|(part, value, weight)| (part.clone(), value, weight))
            .collect()
    }

    /// [`Self::part_opacities`] と同じ値を、パーツの ID を複製せずに返す。
    ///
    /// 重ねる作業に `buffer` を使う。同じ `buffer` を毎フレーム渡せば、メモリを確保しない。
    pub fn part_opacities_in<'a>(
        &'a self,
        buffer: &'a mut PartOpacityBuffer,
    ) -> impl Iterator<Item = (&'a PartId, f32, f32)> + Clone + 'a {
        let out = &mut buffer.entries;
        out.clear();
        for (player_index, player) in self.players.iter().enumerate() {
            let is_part = |target: &CurveTarget| matches!(target, CurveTarget::PartOpacity(_));
            for (curve, target, value, w) in player.curve_values_where(is_part) {
                let CurveTarget::PartOpacity(part) = target else {
                    continue;
                };
                let i = match out
                    .iter()
                    .position(|e| self.part_of(e.player, e.curve) == Some(part))
                {
                    Some(i) => i,
                    None => {
                        out.push(PartOpacityEntry {
                            player: player_index,
                            curve,
                            value: 0.0,
                            weight: 0.0,
                        });
                        out.len() - 1
                    }
                };
                let PartOpacityEntry {
                    value: v, weight, ..
                } = &mut out[i];
                // 重み W で上書きした上に、さらに w で上書きする: lerp(lerp(o, v, W), value, w)
                // = lerp(o, v', W') となる v' と W' を求める。
                let new_weight = 1.0 - (1.0 - *weight) * (1.0 - w);
                if new_weight > 0.0 {
                    *v = (*v * *weight * (1.0 - w) + value * w) / new_weight;
                }
                *weight = new_weight;
            }
        }
        let buffer: &'a PartOpacityBuffer = buffer;
        buffer
            .entries
            .iter()
            .filter_map(|e| Some((self.part_of(e.player, e.curve)?, e.value, e.weight)))
    }

    /// `player` 番目のプレイヤーの `curve` 番目のカーブが不透明度を書くパーツ。
    fn part_of(&self, player: usize, curve: usize) -> Option<&PartId> {
        let curve = self.players.get(player)?.motion().curves.get(curve)?;
        match &curve.target {
            CurveTarget::PartOpacity(part) => Some(part),
            _ => None,
        }
    }
}

/// [`MotionManager::part_opacities_in`] の作業領域。使い回せば毎フレームのメモリの確保がない。
#[derive(Debug, Clone, Default)]
pub struct PartOpacityBuffer {
    /// パーツごとの値と重み。最初に現れた順。
    entries: Vec<PartOpacityEntry>,
}

/// パーツ 1 つ分の重ねた値と重み。パーツは最初に現れたカーブで指す (ID を複製しない)。
#[derive(Debug, Clone, Copy)]
struct PartOpacityEntry {
    player: usize,
    curve: usize,
    value: f32,
    weight: f32,
}

#[cfg(test)]
mod tests {
    use ugoe_model::effects::EYE_BLINK_GROUP;
    use ugoe_model::fixtures;
    use ugoe_model::motion::{Curve, Point, Segment};
    use ugoe_runtime::ParamIndex;

    use super::*;
    use crate::easing;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    fn constant(target: CurveTarget, v: f32) -> Curve {
        Curve {
            target,
            segments: vec![Segment::Start {
                p: Point { t: 0.0, v },
            }],
            fade_in: None,
            fade_out: None,
        }
    }

    fn motion(name: &str, value: f32, fade: f32) -> Motion {
        Motion {
            name: name.into(),
            duration: 10.0,
            looping: true,
            fade_in: fade,
            fade_out: fade,
            curves: vec![constant(CurveTarget::Parameter("X".into()), value)],
            events: vec![],
        }
    }

    fn x(p: &ParameterValues) -> f32 {
        p.get(ParamIndex(0))
    }

    fn params() -> ParameterValues {
        ParameterValues::from_model(&fixtures::single_quad())
    }

    #[test]
    fn lower_priority_request_is_ignored() {
        let mut m = MotionManager::new();
        assert!(m.start(motion("a", 1.0, 0.0), Priority::Normal));
        assert!(!m.start(motion("b", 1.0, 0.0), Priority::Idle));
        assert_eq!(m.players().len(), 1);
        // 同じ優先度と高い優先度は開始できる。
        assert!(m.start(motion("c", 1.0, 0.0), Priority::Normal));
        assert!(m.start(motion("d", 1.0, 0.0), Priority::Force));
        assert_eq!(m.current_priority(), Some(Priority::Force));
        m.stop_all();
        assert!(!m.is_playing());
        assert!(m.start(motion("e", 1.0, 0.0), Priority::Idle));
    }

    /// 現在のモーションが終わると、低い優先度のモーションも開始できる。
    #[test]
    fn priority_clears_when_current_motion_finishes() {
        let mut m = MotionManager::new();
        let mut once = motion("once", 1.0, 0.0);
        once.looping = false;
        once.duration = 1.0;
        m.start(once, Priority::Normal);
        let mut p = params();
        m.update(&mut p, 0.5);
        assert_eq!(m.current_priority(), Some(Priority::Normal));
        m.update(&mut p, 0.6);
        assert_eq!(m.current_priority(), None);
        assert!(m.players().is_empty());
        assert!(m.start(motion("idle", 1.0, 0.0), Priority::Idle));
    }

    /// 古いモーションは自身の fade_out で、新しいモーションは fade_in でクロスフェードする。
    #[test]
    fn new_motion_cross_fades_with_old() {
        let mut m = MotionManager::new();
        m.start(motion("a", -1.0, 1.0), Priority::Normal);
        let mut p = params();
        m.update(&mut p, 2.0);
        assert!(close(x(&p), -1.0));

        m.start(motion("b", 1.0, 1.0), Priority::Normal);
        let mut p = params();
        m.update(&mut p, 0.25);
        // a を重み easing(0.75) で、その上に b を easing(0.25) で重ねる。
        let a = -easing(0.75);
        let expected = a + (1.0 - a) * easing(0.25);
        assert!(close(x(&p), expected), "{} != {expected}", x(&p));
        assert_eq!(m.players().len(), 2);

        let mut p = params();
        m.update(&mut p, 1.0);
        assert_eq!(m.players().len(), 1);
        assert!(close(x(&p), 1.0));
    }

    #[test]
    fn collects_events_from_all_players() {
        let mut a = motion("a", 0.0, 1.0);
        a.events = vec![MotionEvent {
            t: 0.5,
            value: "a".into(),
        }];
        let mut b = a.clone();
        b.events[0].value = "b".into();
        let mut m = MotionManager::new();
        m.start(a, Priority::Normal);
        m.start(b, Priority::Normal);
        m.update(&mut params(), 0.5);
        let names: Vec<_> = m.fired_events().iter().map(|e| e.value.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
        m.update(&mut params(), 0.1);
        assert!(m.fired_events().is_empty());
    }

    /// フェードアウト中のモーションのイベントは、新しいモーションではなく発火したモーションを指す。
    #[test]
    fn event_sources_name_the_motion_that_fired() {
        let event = |value: &str| MotionEvent {
            t: 0.5,
            value: value.into(),
        };
        let mut a = motion("a", 0.0, 1.0);
        a.events = vec![event("from a")];
        let mut b = motion("b", 0.0, 1.0);
        b.events = vec![event("from b")];
        let mut m = MotionManager::new();
        assert!(m.start(a, Priority::Normal));
        m.update(&mut params(), 0.1);
        // b を始めて a をフェードアウトさせる。次の update で、フェードアウト中の a と b のイベントが発火する。
        assert!(m.start(b, Priority::Normal));
        assert_eq!(m.motion_numbers(), [1, 2]);
        assert_eq!(m.current_motion_number(), Some(2));
        m.update(&mut params(), 0.5);
        let fired: Vec<_> = m
            .fired_event_sources()
            .iter()
            .zip(m.fired_events())
            .map(|(s, e)| (s.number, s.name.as_str(), e.value.as_str()))
            .collect();
        assert_eq!(fired, [(1, "a", "from a"), (2, "b", "from b")]);
        m.update(&mut params(), 0.1);
        assert!(m.fired_event_sources().is_empty());
    }

    /// 終わったモーションの番号は取り除き、同じモーションを始め直すと番号が上がる。
    #[test]
    fn motion_numbers_follow_the_players() {
        let mut once = motion("once", 0.0, 0.0);
        once.looping = false;
        once.duration = 1.0;
        once.events = vec![MotionEvent {
            t: 1.0,
            value: "end".into(),
        }];
        let mut m = MotionManager::new();
        assert_eq!(m.current_motion_number(), None);
        m.start(once.clone(), Priority::Normal);
        assert!(!m.start(once.clone(), Priority::Idle));
        m.start(once.clone(), Priority::Normal);
        assert_eq!(m.motion_numbers(), [1, 2]);
        // 1 は止めた (fade_out 0) ので次の update で終わる。2 は終わりのイベントを発火して終わる。
        m.update(&mut params(), 1.0);
        assert!(m.motion_numbers().is_empty());
        assert_eq!(m.current_motion_number(), None);
        assert_eq!(
            m.fired_event_sources(),
            [EventSource {
                number: 2,
                name: "once".into()
            }]
        );
        m.start(once, Priority::Normal);
        assert_eq!(m.motion_numbers(), [3]);
        m.stop_all();
        assert_eq!(m.current_motion_number(), None);
        assert_eq!(m.motion_numbers(), [3]);
    }

    #[test]
    fn effect_group_weight_blends_curves() {
        let mut m = MotionManager::new();
        assert_eq!(m.effect_group_weight(EYE_BLINK_GROUP), 1.0);
        let mut a = motion("a", 0.0, 0.0);
        a.fade_in = 1.0;
        a.curves.push(constant(
            CurveTarget::EffectGroup(EYE_BLINK_GROUP.into()),
            0.0,
        ));
        m.start(a, Priority::Normal);
        m.update(&mut params(), 0.5);
        assert!(close(m.effect_group_weight(EYE_BLINK_GROUP), 0.5));
        assert_eq!(m.effect_group_weight("Other"), 1.0);
    }

    #[test]
    fn part_opacities_combine_weights() {
        let target = CurveTarget::PartOpacity("P".into());
        let mut a = motion("a", 0.0, 0.0);
        a.curves.push(constant(target.clone(), 0.2));
        let mut b = motion("b", 0.0, 1.0);
        b.curves.push(constant(target, 0.8));
        let mut m = MotionManager::new();
        m.start(a, Priority::Normal);
        m.update(&mut params(), 1.0);
        let out = m.part_opacities();
        assert_eq!(out.len(), 1);
        assert!(close(out[0].1, 0.2) && close(out[0].2, 1.0));

        // a は止めた直後 (fade_out 0 なので次の advance で終わる)。b を半分フェードインさせる。
        m.players[0].stop();
        m.players.push(MotionPlayer::new(b));
        m.players[1].advance(0.5);
        let out = m.part_opacities();
        // a の重みは 0 (終わった)、b は easing(0.5) = 0.5。
        assert!(close(out[0].1, 0.8) && close(out[0].2, 0.5), "{out:?}");

        // 重ねた結果は lerp(lerp(o, 0.2, W1), 0.8, w2) と一致する。
        let (v1, w1, v2, w2) = (0.2, 0.6, 0.8, 0.5);
        let w = 1.0 - (1.0 - w1) * (1.0 - w2);
        let v = (v1 * w1 * (1.0 - w2) + v2 * w2) / w;
        for o in [0.0, 0.3, 1.0] {
            let direct = {
                let a = o + (v1 - o) * w1;
                a + (v2 - a) * w2
            };
            assert!(close(o + (v - o) * w, direct));
        }
    }

    /// `part_opacities_in` は `part_opacities` と同じ値を返し、作業領域を使い回す。
    #[test]
    fn part_opacities_in_matches_part_opacities() {
        let (p, q) = (
            CurveTarget::PartOpacity("P".into()),
            CurveTarget::PartOpacity("Q".into()),
        );
        let mut a = motion("a", 0.0, 1.0);
        a.curves.push(constant(p.clone(), 0.2));
        a.curves.push(constant(
            CurveTarget::EffectGroup(EYE_BLINK_GROUP.into()),
            0.5,
        ));
        let mut b = motion("b", 0.0, 1.0);
        b.curves.push(constant(q, 0.4));
        b.curves.push(constant(p, 0.8));
        let mut m = MotionManager::new();
        m.start(a, Priority::Normal);
        m.update(&mut params(), 0.5);
        m.start(b, Priority::Normal);
        m.update(&mut params(), 0.25);

        let mut buffer = PartOpacityBuffer::default();
        for _ in 0..2 {
            let borrowed: Vec<_> = m
                .part_opacities_in(&mut buffer)
                .map(|(part, v, w)| (part.clone(), v, w))
                .collect();
            assert_eq!(borrowed, m.part_opacities());
            assert_eq!(
                borrowed
                    .iter()
                    .map(|(p, ..)| p.as_str())
                    .collect::<Vec<_>>(),
                ["P", "Q"]
            );
        }
        let capacity = buffer.entries.capacity();
        m.part_opacities_in(&mut buffer).count();
        assert_eq!(buffer.entries.capacity(), capacity);
    }
}
