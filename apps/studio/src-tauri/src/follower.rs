//! ネイティブの受け手 (ADR 0028 の 3.)。届いたフレームを `seq` の順に 1 つずつ `Puppet::follow` で適用し、物理を回す。
//!
//! 一致の検査 (ADR 0028 の 10.、`tests/follow_match.rs`) と、仮想カメラの受け手 (`virtual_camera.rs`、T0807) が使う。
//! wasm の受け手は `packages/player/src/follower.ts` で、規則は同じ。

use ugoe::{ParameterValuesError, PartOpacityInput, Puppet};

use crate::stream::Frame;

/// フレームを 1 つ適用する。`part_opacities` はフレームのものを、このモデルのパーツの番号に引き直したもの
/// ([`Follower`] が作る。T0853)。ネイティブで `Puppet::follow` を呼ぶのはここだけにする
/// (T0134・T0135 で `follow` の引数が増えたときに合わせる場所を 1 つにするため)。
pub fn apply_frame(
    puppet: &mut Puppet,
    frame: &Frame,
    part_opacities: &[PartOpacityInput],
) -> Result<(), ParameterValuesError> {
    if frame.reset {
        puppet.reset_physics();
    }
    puppet.follow(&frame.values, frame.lip_sync, part_opacities, frame.dt)
}

/// シーンの通知のポーズの要素 `parts` (`Look::pose`) を、それぞれのグループの表示する要素にする (T0842)。
///
/// 受け手はモデルの読み直し・モジュールの付け直しの後に呼ぶ。その直後 (`immediately`) はフェードせずに表示し、
/// 同じモデルのままの切り替えは次のフレームからフェードで切り替える (指揮役と同じ。`docs/studio.md#ポーズの要素`、T0844)。
/// モデルに無い要素は何もしない。
pub fn show_pose(puppet: &mut Puppet, parts: &[String], immediately: bool) {
    for part in parts {
        let part = part.as_str().into();
        if immediately {
            puppet.pose_mut().show_immediately(&part);
        } else {
            puppet.pose_mut().show(&part);
        }
    }
}

/// フレームを受け取った結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Received {
    /// 適用した。
    Applied,
    /// `seq` が飛んでいたので、物理をリセットしてから適用した (接続が切れていたなど)。
    AppliedAfterGap,
    /// 前のシーンのフレームなので捨てた。
    Stale,
    /// まだ知らないシーンのフレーム。シーン ([`Follower::set_scene`]・[`Follower::continue_scene`]) を待つ必要がある。
    Ahead,
}

/// 受け手。モデルが変わるシーンごとにモデル (`Puppet`) を作り直す。
///
/// 適用するフレームは `revision` が `from` (モデルを読んだシーン) から今のシーンまでのもの。同じモデルのまま次のシーンに
/// 移ったとき ([`Follower::continue_scene`]。見せ方や着せ替えだけが変わった) は、前のシーンのフレームも捨てずに適用し、
/// `seq` の続きも保つ (`packages/player/src/follower.ts` の `FrameQueue` と同じ規則)。
///
/// フレームのパーツの番号は、そのフレームのシーンの `part_ids` で ID に直し、今のモデルの番号に引き直して当てる
/// (着せ替えの直後に前のシーンのフレームを当ててもモジュールのパーツがずれないように。T0853)。引き直しの表はシーンを
/// 受けたときに作り、フレームごとには確保しない。
#[derive(Debug, Default)]
pub struct Follower {
    puppet: Option<Puppet>,
    from: Option<u32>,
    revision: Option<u32>,
    last_seq: Option<u64>,
    /// 受けたシーンのパーツの並び (`revision` の順。並びが前と同じシーンは足さない)。
    parts: Vec<ScenePartMap>,
    /// 引き直したパーツの不透明度 (フレームごとに使い回す)。
    remapped: Vec<PartOpacityInput>,
}

/// 1 つのシーンのパーツの並び (`Scene::part_ids`) と、その番号から受け手のモデルの番号への表 (T0853)。
#[derive(Debug)]
struct ScenePartMap {
    revision: u32,
    part_ids: Vec<String>,
    /// シーンの番号ごとの、受け手のモデルの番号 (モデルに無い ID は `None`)。並びがモデルと同じなら `None` (引き直さない)。
    table: Option<Vec<Option<u32>>>,
}

impl ScenePartMap {
    fn new(revision: u32, part_ids: &[String], puppet: &Puppet) -> Self {
        let mut map = Self {
            revision,
            part_ids: part_ids.to_vec(),
            table: None,
        };
        map.rebuild(puppet);
        map
    }

    /// `puppet` のモデルの番号への表を作り直す (モジュールを付け直したとき)。
    fn rebuild(&mut self, puppet: &Puppet) {
        let compiled = puppet.instance().compiled();
        let parts = &compiled.model().parts;
        let same = parts.len() == self.part_ids.len()
            && parts
                .iter()
                .zip(&self.part_ids)
                .all(|(part, id)| part.id.as_str() == id);
        self.table = (!same).then(|| {
            self.part_ids
                .iter()
                .map(|id| compiled.part_index(id).and_then(|i| u32::try_from(i).ok()))
                .collect()
        });
    }
}

impl Follower {
    pub fn new() -> Self {
        Self::default()
    }

    /// 新しいシーン `revision` のモデルにする。`puppet` は同じモデルから物理を有効にして作ったもの、`part_ids` はシーンの
    /// パーツの並び (`Scene::part_ids`)。
    pub fn set_scene(&mut self, revision: u32, puppet: Puppet, part_ids: &[String]) {
        self.parts = vec![ScenePartMap::new(revision, part_ids, &puppet)];
        self.puppet = Some(puppet);
        self.from = Some(revision);
        self.revision = Some(revision);
        self.last_seq = None;
    }

    /// 同じモデルのまま次のシーン `revision` (パーツの並びは `part_ids`) に移る (前のシーンのフレームも続けて適用する)。
    /// モジュールを付け直したなら、付け直した後に呼ぶ (前のシーンのパーツの番号を今のモデルに引き直すため)。
    /// モデルが無ければ何もしない。
    pub fn continue_scene(&mut self, revision: u32, part_ids: &[String]) {
        let Some(puppet) = &self.puppet else {
            return;
        };
        for map in &mut self.parts {
            map.rebuild(puppet);
        }
        let newer = self
            .parts
            .last()
            .is_none_or(|last| revision > last.revision);
        let same = self
            .parts
            .last()
            .is_some_and(|last| last.part_ids == part_ids);
        if newer && !same {
            self.parts
                .push(ScenePartMap::new(revision, part_ids, puppet));
        }
        self.revision = Some(self.revision.map_or(revision, |r| r.max(revision)));
    }

    /// モデルの無いシーンにする (フレームはすべて先のものとして捨てる)。
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn puppet(&self) -> Option<&Puppet> {
        self.puppet.as_ref()
    }

    /// 着せ替え (モジュールの付け直し) に使う。
    pub fn puppet_mut(&mut self) -> Option<&mut Puppet> {
        self.puppet.as_mut()
    }

    /// フレームを 1 つ受け取る。
    pub fn receive(&mut self, frame: &Frame) -> Result<Received, ParameterValuesError> {
        let Self {
            puppet: Some(puppet),
            from: Some(from),
            revision: Some(revision),
            last_seq,
            parts,
            remapped,
        } = self
        else {
            return Ok(Received::Ahead);
        };
        if frame.revision < *from {
            return Ok(Received::Stale);
        }
        if frame.revision > *revision {
            return Ok(Received::Ahead);
        }
        let gap = last_seq.is_some_and(|last| frame.seq != last.wrapping_add(1));
        if gap {
            puppet.reset_physics();
        }
        // フレームのシーン (受けていなければその前の、最も新しいシーン) の並びで引き直す。フレームは順に届くので、
        // それより前のシーンの並びは捨てる。
        let at = parts
            .iter()
            .rposition(|map| map.revision <= frame.revision)
            .unwrap_or(0);
        parts.drain(..at);
        let part_opacities = match parts.first().and_then(|map| map.table.as_ref()) {
            None => frame.part_opacities.as_slice(),
            Some(table) => {
                remapped.clear();
                remapped.extend(frame.part_opacities.iter().filter_map(|input| {
                    let part = (*table.get(input.part as usize)?)?;
                    Some(PartOpacityInput { part, ..*input })
                }));
                remapped.as_slice()
            }
        };
        apply_frame(puppet, frame, part_opacities)?;
        *last_seq = Some(frame.seq);
        Ok(if gap {
            Received::AppliedAfterGap
        } else {
            Received::Applied
        })
    }
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;

    use super::*;

    fn frame(seq: u64, revision: u32, n: usize) -> Frame {
        Frame {
            seq,
            revision,
            dt: 1.0 / 60.0,
            reset: false,
            values: vec![0.0; n],
            lip_sync: None,
            part_opacities: Vec::new(),
        }
    }

    #[test]
    fn orders_frames_by_scene_and_seq() {
        let puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let n = puppet.instance().parameters().len();
        let mut follower = Follower::new();
        assert_eq!(follower.receive(&frame(0, 1, n)), Ok(Received::Ahead));
        follower.set_scene(2, puppet, &[]);
        assert_eq!(follower.receive(&frame(1, 1, n)), Ok(Received::Stale));
        assert_eq!(follower.receive(&frame(2, 3, n)), Ok(Received::Ahead));
        assert_eq!(follower.receive(&frame(5, 2, n)), Ok(Received::Applied));
        assert_eq!(follower.receive(&frame(6, 2, n)), Ok(Received::Applied));
        assert_eq!(
            follower.receive(&frame(9, 2, n)),
            Ok(Received::AppliedAfterGap)
        );
        assert!(follower.receive(&frame(10, 2, n + 1)).is_err());
    }

    #[test]
    fn continuing_a_scene_keeps_the_previous_frames_and_seq() {
        let puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let n = puppet.instance().parameters().len();
        let mut follower = Follower::new();
        follower.continue_scene(4, &[]);
        assert_eq!(follower.receive(&frame(0, 4, n)), Ok(Received::Ahead));
        follower.set_scene(2, puppet, &[]);
        assert_eq!(follower.receive(&frame(1, 2, n)), Ok(Received::Applied));
        follower.continue_scene(3, &[]);
        // 前のシーンのフレームも適用し、seq の続きを保つ。
        assert_eq!(follower.receive(&frame(2, 2, n)), Ok(Received::Applied));
        assert_eq!(follower.receive(&frame(3, 3, n)), Ok(Received::Applied));
        assert_eq!(follower.receive(&frame(4, 4, n)), Ok(Received::Ahead));
        assert_eq!(follower.receive(&frame(5, 1, n)), Ok(Received::Stale));
        follower.clear();
        assert!(follower.puppet().is_none());
        assert_eq!(follower.receive(&frame(6, 3, n)), Ok(Received::Ahead));
    }

    #[test]
    fn part_numbers_are_remapped_by_the_scene_part_ids() {
        let puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        let n = puppet.instance().parameters().len();
        let ids = crate::conductor::part_ids(&puppet);
        let index = |id: &str| puppet.instance().compiled().part_index(id).unwrap();
        let (arm_a, arm_b) = (index("ArmA"), index("ArmB"));
        // シーンの並び: ArmB・(モデルに無いパーツ)・ArmA。
        let scene = vec!["ArmB".to_owned(), "Missing".to_owned(), "ArmA".to_owned()];
        let mut follower = Follower::new();
        follower.set_scene(1, puppet, &scene);
        let input = |part: u32, value: f32| PartOpacityInput {
            part,
            value,
            weight: 1.0,
        };
        let mut sent = frame(0, 1, n);
        sent.part_opacities = vec![input(0, 0.25), input(1, 0.5), input(2, 0.75)];
        assert_eq!(follower.receive(&sent), Ok(Received::Applied));
        let applied = follower.puppet().unwrap().part_opacities_applied().to_vec();
        assert_eq!(
            applied,
            [input(arm_b as u32, 0.25), input(arm_a as u32, 0.75)]
        );
        // モデルと同じ並びのシーンに移れば、番号はそのまま。
        follower.continue_scene(2, &ids);
        sent.seq = 1;
        sent.revision = 2;
        sent.part_opacities = vec![input(arm_a as u32, 0.5)];
        assert_eq!(follower.receive(&sent), Ok(Received::Applied));
        assert_eq!(
            follower.puppet().unwrap().part_opacities_applied(),
            [input(arm_a as u32, 0.5)]
        );
    }
}
