//! 指揮役が受け手に配るメッセージの形 (`docs/studio.md#フレームの配信-内部の形`、ADR 0028 の 3.)。
//!
//! スタジオの中だけで使う形で、版を上げて変えてよい (受け手のページもスタジオが配るので、いつも同じ版で揃う)。
//! フロントエンドの読み取りは `packages/player/src/follower.ts` の `decodeFrame`。両方のテストが同じバイト列を使う。

use serde::Serialize;
use ugoe::LipSyncInput;

/// フレームのバイト列の先頭の種類の値。
pub const FRAME_KIND: u8 = 1;
/// フラグ: このフレームを適用する前に物理をリセットする (着せ替えなど、モデルを読み直さずに形が変わるとき)。
pub const FLAG_RESET: u8 = 1;
/// フラグ: リップシンクを当てる (音量と強さが入っている)。
pub const FLAG_LIP_SYNC: u8 = 2;
/// フレームの頭の大きさ (バイト)。`values` は 4 の倍数の位置から始まる (JS で `Float32Array` をそのまま重ねられる)。
pub const FRAME_HEADER: usize = 32;

/// 1 刻み分のフレーム。受け手は `seq` の順に 1 つずつ `Puppet::follow(values, lip_sync, dt)` で適用する。
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// 刻みの通し番号。シーンが変わっても続けて数える。
    pub seq: u64,
    /// どのシーン ([`Scene::revision`]) の値か。
    pub revision: u32,
    /// この刻みで進める時間 (秒)。
    pub dt: f32,
    /// 適用する前に物理をリセットするか。
    pub reset: bool,
    /// 指揮役の物理の段に入る直前のパラメータの値 (シーンの `parameter_ids` の順。`Puppet::values_before_physics`)。
    pub values: Vec<f32>,
    /// 指揮役のリップシンクの音量と強さ (`Puppet::lip_sync_applied`)。当てなければ `None`。
    pub lip_sync: Option<LipSyncInput>,
}

/// フレームのバイト列を読めない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("the frame is too short ({0} bytes)")]
    TooShort(usize),
    #[error("not a frame (kind {0})")]
    Kind(u8),
    #[error("the frame has {expected} values but {actual} bytes of values")]
    Length { expected: usize, actual: usize },
}

impl Frame {
    /// バイト列にする (リトルエンディアン)。
    ///
    /// | 位置 | 型 | 中身 |
    /// | --- | --- | --- |
    /// | 0 | u8 | 種類 ([`FRAME_KIND`]) |
    /// | 1 | u8 | フラグ ([`FLAG_RESET`]・[`FLAG_LIP_SYNC`]) |
    /// | 2 | u16 | 予約 (0) |
    /// | 4 | u32 | `revision` |
    /// | 8 | u64 | `seq` |
    /// | 16 | f32 | `dt` |
    /// | 20 | f32 | リップシンクの音量 (無ければ 0) |
    /// | 24 | f32 | リップシンクの強さ (無ければ 0) |
    /// | 28 | u32 | 値の数 |
    /// | 32 | f32 × 数 | `values` |
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_HEADER + self.values.len() * 4);
        out.push(FRAME_KIND);
        let mut flags = 0;
        if self.reset {
            flags |= FLAG_RESET;
        }
        if self.lip_sync.is_some() {
            flags |= FLAG_LIP_SYNC;
        }
        out.push(flags);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&self.revision.to_le_bytes());
        out.extend_from_slice(&self.seq.to_le_bytes());
        out.extend_from_slice(&self.dt.to_le_bytes());
        let lip_sync = self.lip_sync.map_or([0.0; 2], |l| [l.level, l.strength]);
        for v in lip_sync {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&(self.values.len() as u32).to_le_bytes());
        for v in &self.values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// [`Frame::encode`] の逆。
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let header = bytes
            .get(..FRAME_HEADER)
            .ok_or(FrameError::TooShort(bytes.len()))?;
        if header[0] != FRAME_KIND {
            return Err(FrameError::Kind(header[0]));
        }
        let u32_at =
            |i: usize| u32::from_le_bytes([header[i], header[i + 1], header[i + 2], header[i + 3]]);
        let mut seq = [0u8; 8];
        seq.copy_from_slice(&header[8..16]);
        let count = u32_at(28) as usize;
        let rest = &bytes[FRAME_HEADER..];
        if rest.len() != count * 4 {
            return Err(FrameError::Length {
                expected: count,
                actual: rest.len(),
            });
        }
        Ok(Self {
            seq: u64::from_le_bytes(seq),
            revision: u32_at(4),
            dt: f32::from_bits(u32_at(16)),
            reset: header[1] & FLAG_RESET != 0,
            lip_sync: (header[1] & FLAG_LIP_SYNC != 0).then(|| LipSyncInput {
                level: f32::from_bits(u32_at(20)),
                strength: f32::from_bits(u32_at(24)),
            }),
            values: rest
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect(),
        })
    }
}

/// シーンの通知 (JSON。`kind` は `"scene"`)。接続したときと、シーンが変わったときに配る。
///
/// 受け手はモデルのバイト列を `scene_model` のコマンドで `revision` を付けて取る (任意のパスは読ませない)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", rename = "scene")]
pub struct Scene {
    /// シーンの版。モデルを読むたびに上がる。
    pub revision: u32,
    /// モデルの表示名 (ファイル名)。
    pub model_name: String,
    /// フレームの `values` の並び (モデルのパラメータの定義順)。
    pub parameter_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `packages/player/src/follower.test.ts` の `GOLDEN` と同じバイト列。
    const GOLDEN: [u8; 40] = [
        1, 3, 0, 0, // 種類・フラグ (リセットとリップシンク)・予約
        7, 0, 0, 0, // revision = 7
        0x2a, 0x01, 0, 0, 0, 0, 0, 0, // seq = 298
        0x89, 0x88, 0x88, 0x3c, // dt = 1/60
        0, 0, 0, 0x3f, // リップシンクの音量 = 0.5
        0, 0, 0x40, 0x3f, // リップシンクの強さ = 0.75
        2, 0, 0, 0, // 値の数 = 2
        0, 0, 0xc0, 0x3f, // 1.5
        0, 0, 0x20, 0xc1, // -10
    ];

    fn golden() -> Frame {
        Frame {
            seq: 298,
            revision: 7,
            dt: 1.0 / 60.0,
            reset: true,
            values: vec![1.5, -10.0],
            lip_sync: Some(LipSyncInput {
                level: 0.5,
                strength: 0.75,
            }),
        }
    }

    #[test]
    fn encodes_the_documented_layout() {
        assert_eq!(golden().encode(), GOLDEN);
        assert_eq!(Frame::decode(&GOLDEN).unwrap(), golden());
    }

    #[test]
    fn round_trips() {
        let frame = Frame {
            seq: u64::MAX - 1,
            revision: 0,
            dt: 0.1,
            reset: false,
            values: (0..100).map(|i| i as f32 * 0.25 - 3.0).collect(),
            lip_sync: None,
        };
        assert_eq!(Frame::decode(&frame.encode()).unwrap(), frame);
    }

    #[test]
    fn rejects_broken_bytes() {
        assert_eq!(Frame::decode(&GOLDEN[..10]), Err(FrameError::TooShort(10)));
        let mut kind = GOLDEN;
        kind[0] = 2;
        assert_eq!(Frame::decode(&kind), Err(FrameError::Kind(2)));
        assert_eq!(
            Frame::decode(&GOLDEN[..38]),
            Err(FrameError::Length {
                expected: 2,
                actual: 6
            })
        );
    }

    #[test]
    fn scene_is_tagged_json() {
        let scene = Scene {
            revision: 3,
            model_name: "avatar.ugoe".into(),
            parameter_ids: vec!["AngleX".into()],
        };
        assert_eq!(
            serde_json::to_string(&scene).unwrap(),
            r#"{"kind":"scene","revision":3,"modelName":"avatar.ugoe","parameterIds":["AngleX"]}"#
        );
    }
}
