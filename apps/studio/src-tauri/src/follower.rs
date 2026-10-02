//! ネイティブの受け手 (ADR 0028 の 3.)。届いたフレームを `seq` の順に 1 つずつ `Puppet::follow` で適用し、物理を回す。
//!
//! いまは一致の検査 (ADR 0028 の 10.、`tests/follow_match.rs`) で使う。仮想カメラ (T0807) の受け手もこれを使う。
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
    /// まだ知らないシーンのフレーム。シーン ([`Follower::set_scene`]) を待つ必要がある。
    Ahead,
}

/// 受け手。シーンごとにモデル (`Puppet`) を作り直す。
#[derive(Debug, Default)]
pub struct Follower {
    puppet: Option<Puppet>,
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
        self.revision = Some(revision);
        self.last_seq = None;
    }

    pub fn puppet(&self) -> Option<&Puppet> {
        self.puppet.as_ref()
    }

    /// フレームを 1 つ受け取る。
    pub fn receive(&mut self, frame: &Frame) -> Result<Received, ParameterValuesError> {
        let (Some(puppet), Some(revision)) = (self.puppet.as_mut(), self.revision) else {
            return Ok(Received::Ahead);
        };
        if frame.revision < revision {
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
}
