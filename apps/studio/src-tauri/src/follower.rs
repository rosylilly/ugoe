//! ネイティブの受け手 (ADR 0028 の 3.)。届いたフレームを `seq` の順に 1 つずつ `Puppet::follow` で適用し、物理を回す。
//!
//! 一致の検査 (ADR 0028 の 10.、`tests/follow_match.rs`) と、仮想カメラの受け手 (`virtual_camera.rs`、T0807) が使う。
//! wasm の受け手は `packages/player/src/follower.ts` で、規則は同じ。

use ugoe::{ParameterValuesError, Puppet};

use crate::stream::Frame;

/// フレームを 1 つ適用する。ネイティブで `Puppet::follow` を呼ぶのはここだけにする
/// (T0134 で `follow` の引数が増えたときに合わせる場所を 1 つにするため)。
pub fn apply_frame(puppet: &mut Puppet, frame: &Frame) -> Result<(), ParameterValuesError> {
    if frame.reset {
        puppet.reset_physics();
    }
    puppet.follow(&frame.values, frame.lip_sync, frame.dt)
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
#[derive(Debug, Default)]
pub struct Follower {
    puppet: Option<Puppet>,
    from: Option<u32>,
    revision: Option<u32>,
    last_seq: Option<u64>,
}

impl Follower {
    pub fn new() -> Self {
        Self::default()
    }

    /// 新しいシーン `revision` のモデルにする。`puppet` は同じモデルから物理を有効にして作ったもの。
    pub fn set_scene(&mut self, revision: u32, puppet: Puppet) {
        self.puppet = Some(puppet);
        self.from = Some(revision);
        self.revision = Some(revision);
        self.last_seq = None;
    }

    /// 同じモデルのまま次のシーン `revision` に移る (前のシーンのフレームも続けて適用する)。モデルが無ければ何もしない。
    pub fn continue_scene(&mut self, revision: u32) {
        if self.puppet.is_some() {
            self.revision = Some(self.revision.map_or(revision, |r| r.max(revision)));
        }
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
        let (Some(puppet), Some(from), Some(revision)) =
            (self.puppet.as_mut(), self.from, self.revision)
        else {
            return Ok(Received::Ahead);
        };
        if frame.revision < from {
            return Ok(Received::Stale);
        }
        if frame.revision > revision {
            return Ok(Received::Ahead);
        }
        let gap = self
            .last_seq
            .is_some_and(|last| frame.seq != last.wrapping_add(1));
        if gap {
            puppet.reset_physics();
        }
        apply_frame(puppet, frame)?;
        self.last_seq = Some(frame.seq);
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
        }
    }

    #[test]
    fn orders_frames_by_scene_and_seq() {
        let puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let n = puppet.instance().parameters().len();
        let mut follower = Follower::new();
        assert_eq!(follower.receive(&frame(0, 1, n)), Ok(Received::Ahead));
        follower.set_scene(2, puppet);
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
        follower.continue_scene(4);
        assert_eq!(follower.receive(&frame(0, 4, n)), Ok(Received::Ahead));
        follower.set_scene(2, puppet);
        assert_eq!(follower.receive(&frame(1, 2, n)), Ok(Received::Applied));
        follower.continue_scene(3);
        // 前のシーンのフレームも適用し、seq の続きを保つ。
        assert_eq!(follower.receive(&frame(2, 2, n)), Ok(Received::Applied));
        assert_eq!(follower.receive(&frame(3, 3, n)), Ok(Received::Applied));
        assert_eq!(follower.receive(&frame(4, 4, n)), Ok(Received::Ahead));
        assert_eq!(follower.receive(&frame(5, 1, n)), Ok(Received::Stale));
        follower.clear();
        assert!(follower.puppet().is_none());
        assert_eq!(follower.receive(&frame(6, 3, n)), Ok(Received::Ahead));
    }
}
