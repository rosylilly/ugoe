//! ポーズ (排他的なパーツの切り替え)。仕様: `docs/spec/motion.md#ポーズ`。

use ugoe_model::PartId;
use ugoe_model::pose::{Pose, PoseGroup};

/// ポーズグループ 1 つ分の状態。
#[derive(Debug, Clone)]
struct GroupState {
    /// 表示中の要素の添字。
    visible: usize,
    /// 要素ごとの不透明度。
    opacities: Vec<f32>,
}

/// ポーズの再生状態。
///
/// 各グループは表示中の要素を 1 つ持つ。最初は先頭の要素を表示する (不透明度 1、ほかは 0)。
/// 表示する要素が変わると、表示中の要素の不透明度を `1 / fade_duration` の速さで 1 まで上げ、
/// ほかの要素の不透明度を `1 - (表示中の要素の不透明度)` 以下に抑える。
#[derive(Debug, Clone)]
pub struct PosePlayer {
    pose: Pose,
    groups: Vec<GroupState>,
}

impl PosePlayer {
    /// 各グループの先頭の要素を表示した状態 (先頭の不透明度 1、ほかは 0) で作る。
    pub fn new(pose: Pose) -> Self {
        let groups = pose
            .groups
            .iter()
            .map(|g| GroupState {
                visible: 0,
                opacities: (0..g.entries.len())
                    .map(|i| if i == 0 { 1.0 } else { 0.0 })
                    .collect(),
            })
            .collect();
        Self { pose, groups }
    }

    /// ポーズの設定。
    pub fn pose(&self) -> &Pose {
        &self.pose
    }

    /// `part` がいずれかのグループの要素 (リンクは含めない) か。
    pub fn contains(&self, part: &PartId) -> bool {
        self.pose
            .groups
            .iter()
            .any(|g| g.entries.iter().any(|e| &e.part == part))
    }

    /// `part` を含むグループで、`part` を表示する要素にする (フェードは次の `update` から)。
    /// どのグループの要素でもなければ `false`。
    pub fn show(&mut self, part: &PartId) -> bool {
        let mut found = false;
        for (group, state) in self.pose.groups.iter().zip(&mut self.groups) {
            if let Some(i) = group.entries.iter().position(|e| &e.part == part) {
                state.visible = i;
                found = true;
            }
        }
        found
    }

    /// `previous` の再生状態 (表示中の要素と不透明度) を引き継ぐ。
    ///
    /// 要素のパーツの並びが同じグループの状態を写す (リンクは違ってよい)。着せ替えの組み替え
    /// (`docs/spec/modules.md#puppet-での着せ替え`) でポーズを作り直したとき、表示中の要素を保つために使う。
    /// 対応するグループが無いグループは、先頭の要素の表示のまま。
    pub fn carry_over(&mut self, previous: &PosePlayer) {
        let same = |a: &PoseGroup, b: &PoseGroup| {
            a.entries.len() == b.entries.len()
                && a.entries
                    .iter()
                    .zip(&b.entries)
                    .all(|(x, y)| x.part == y.part)
        };
        for (group, state) in self.pose.groups.iter().zip(&mut self.groups) {
            let found = previous
                .pose
                .groups
                .iter()
                .zip(&previous.groups)
                .find(|(g, _)| same(g, group));
            if let Some((_, old)) = found {
                state.clone_from(old);
            }
        }
    }

    /// 表示中のパーツ (グループの順)。
    pub fn visible_parts(&self) -> impl Iterator<Item = &PartId> {
        self.pose
            .groups
            .iter()
            .zip(&self.groups)
            .filter_map(|(g, s)| g.entries.get(s.visible).map(|e| &e.part))
    }

    /// `dt` 秒進める。
    ///
    /// `requests` はパーツごとの表示の要求 (モーションのパーツの不透明度のカーブの値)。
    /// 各グループで、要求の値が 0 より大きい要素のうち最大のもの (同じなら先の要素) を表示する。
    /// 要求がなければ表示中の要素を保つ。
    pub fn update(&mut self, dt: f32, requests: &[(PartId, f32)]) {
        self.update_by(dt, |part| {
            requests.iter().find(|(p, _)| p == part).map(|&(_, v)| v)
        });
    }

    /// [`Self::update`] と同じだが、表示の要求を `request` で引く (要素のパーツを渡し、要求がなければ `None`)。
    /// 要求の列を作らずに済む (`Puppet` が毎フレーム使う)。
    pub fn update_by(&mut self, dt: f32, mut request: impl FnMut(&PartId) -> Option<f32>) {
        let rate = if self.pose.fade_duration > 0.0 {
            dt.max(0.0) / self.pose.fade_duration
        } else {
            f32::INFINITY
        };
        for (group, state) in self.pose.groups.iter().zip(&mut self.groups) {
            let mut best: Option<(usize, f32)> = None;
            for (i, entry) in group.entries.iter().enumerate() {
                if let Some(v) = request(&entry.part)
                    && v > 0.0
                    && best.is_none_or(|(_, b)| v > b)
                {
                    best = Some((i, v));
                }
            }
            if let Some((i, _)) = best {
                state.visible = i;
            }
            let Some(shown) = state.opacities.get_mut(state.visible) else {
                continue;
            };
            *shown = (*shown + rate).min(1.0);
            let limit = 1.0 - *shown;
            for (i, o) in state.opacities.iter_mut().enumerate() {
                if i != state.visible {
                    *o = o.min(limit);
                }
            }
        }
    }

    /// 各要素とそのリンクのパーツの不透明度 `(パーツ, 不透明度)`。グループの順、要素の順に並ぶ。
    pub fn opacities(&self) -> impl Iterator<Item = (&PartId, f32)> {
        self.pose
            .groups
            .iter()
            .zip(&self.groups)
            .flat_map(|(g, s)| {
                g.entries.iter().zip(&s.opacities).flat_map(|(e, &o)| {
                    std::iter::once(&e.part)
                        .chain(e.links.iter())
                        .map(move |p| (p, o))
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    fn player() -> PosePlayer {
        PosePlayer::new(fixtures::pose_and_effects().pose)
    }

    fn opacity(p: &PosePlayer, id: &str) -> f32 {
        p.opacities()
            .find(|(part, _)| part.as_str() == id)
            .map(|(_, o)| o)
            .unwrap()
    }

    #[test]
    fn first_entry_is_visible_initially_and_links_follow() {
        let mut p = player();
        p.update(0.1, &[]);
        assert_eq!(opacity(&p, "ArmA"), 1.0);
        assert_eq!(opacity(&p, "SleeveA"), 1.0);
        assert_eq!(opacity(&p, "ArmB"), 0.0);
        assert!(p.contains(&"ArmB".into()));
        assert!(!p.contains(&"SleeveA".into()));
    }

    /// 切り替えは fade_duration (0.5 秒) でクロスフェードする。
    #[test]
    fn switching_cross_fades() {
        let mut p = player();
        assert!(p.show(&"ArmB".into()));
        p.update(0.125, &[]);
        assert!(close(opacity(&p, "ArmB"), 0.25));
        assert!(close(opacity(&p, "ArmA"), 0.75));
        assert!(close(opacity(&p, "SleeveA"), 0.75));
        p.update(0.5, &[]);
        assert_eq!(opacity(&p, "ArmB"), 1.0);
        assert_eq!(opacity(&p, "ArmA"), 0.0);
        assert_eq!(
            p.visible_parts().map(|p| p.as_str()).collect::<Vec<_>>(),
            ["ArmB"]
        );
    }

    #[test]
    fn requests_pick_largest_positive_value() {
        let mut p = player();
        p.update(1.0, &[("ArmA".into(), 0.2), ("ArmB".into(), 0.9)]);
        assert_eq!(opacity(&p, "ArmB"), 1.0);
        // 要求がすべて 0 なら表示中の要素を保つ。
        p.update(1.0, &[("ArmA".into(), 0.0), ("ArmB".into(), 0.0)]);
        assert_eq!(opacity(&p, "ArmB"), 1.0);
        assert!(!p.show(&"Missing".into()));
    }

    #[test]
    fn zero_fade_duration_switches_immediately() {
        let mut pose = fixtures::pose_and_effects().pose;
        pose.fade_duration = 0.0;
        let mut p = PosePlayer::new(pose);
        p.show(&"ArmB".into());
        p.update(0.0, &[]);
        assert_eq!(opacity(&p, "ArmB"), 1.0);
        assert_eq!(opacity(&p, "ArmA"), 0.0);
    }

    /// 要素のパーツの並びが同じグループは、リンクが違っても途中の状態を引き継ぐ。
    #[test]
    fn carry_over_keeps_the_visible_entry_of_matching_groups() {
        let mut old = player();
        assert!(old.show(&"ArmB".into()));
        old.update(0.125, &[]);

        let mut pose = old.pose().clone();
        pose.groups[0].entries[1].links.push("Extra".into());
        pose.groups.push(PoseGroup {
            entries: vec![ugoe_model::pose::PoseEntry {
                part: "HatA".into(),
                links: vec![],
            }],
        });
        let mut p = PosePlayer::new(pose);
        p.carry_over(&old);
        assert!(close(opacity(&p, "ArmB"), 0.25));
        assert!(close(opacity(&p, "Extra"), 0.25));
        assert!(close(opacity(&p, "ArmA"), 0.75));
        assert_eq!(opacity(&p, "HatA"), 1.0);
        assert_eq!(p.visible_parts().next().unwrap().as_str(), "ArmB");
    }
}
