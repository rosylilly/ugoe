//! 指揮役が受け手に配るメッセージの形 (`docs/studio.md#フレームの配信-内部の形`、ADR 0028 の 3.)。
//!
//! スタジオの中だけで使う形で、版を上げて変えてよい (受け手のページもスタジオが配るので、いつも同じ版で揃う)。
//! フロントエンドの読み取りは `packages/player/src/follower.ts` の `decodeFrame`。両方のテストが同じバイト列を使う。

use serde::Serialize;
use ugoe::{LipSyncInput, PartOpacityInput};

use crate::profile::{Background, Transform};

/// フレームのバイト列の先頭の種類の値。
pub const FRAME_KIND: u8 = 1;
/// フラグ: このフレームを適用する前に物理をリセットする (着せ替えなど、モデルを読み直さずに形が変わるとき)。
pub const FLAG_RESET: u8 = 1;
/// フラグ: リップシンクを当てる (音量と強さが入っている)。
pub const FLAG_LIP_SYNC: u8 = 2;
/// フレームの頭の大きさ (バイト)。`values` は 4 の倍数の位置から始まる (JS で `Float32Array` をそのまま重ねられる)。
pub const FRAME_HEADER: usize = 32;

/// 1 刻み分のフレーム。受け手は `seq` の順に 1 つずつ `Puppet::follow(values, lip_sync, part_opacities, dt)` で適用する。
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
    /// 指揮役のモーションのパーツの不透明度のカーブ (`Puppet::part_opacities_applied`。カーブのあるパーツだけ。T0135)。
    pub part_opacities: Vec<PartOpacityInput>,
}

/// フレームのバイト列を読めない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("the frame is too short ({0} bytes)")]
    TooShort(usize),
    #[error("not a frame (kind {0})")]
    Kind(u8),
    #[error("the frame has {expected} bytes after the header but {actual} bytes")]
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
    /// | 28 | u32 | 値の数 `n` |
    /// | 32 | f32 × `n` | `values` |
    /// | 32 + 4n | u32 | パーツの不透明度の数 `k` |
    /// | 36 + 4n | u32 × `k` | パーツの番号 |
    /// | 36 + 4n + 4k | f32 × 2`k` | 値と重み (パーツごとに交互) |
    pub fn encode(&self) -> Vec<u8> {
        let k = self.part_opacities.len();
        let mut out = Vec::with_capacity(FRAME_HEADER + self.values.len() * 4 + 4 + k * 12);
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
        out.extend_from_slice(&(k as u32).to_le_bytes());
        for p in &self.part_opacities {
            out.extend_from_slice(&p.part.to_le_bytes());
        }
        for p in &self.part_opacities {
            out.extend_from_slice(&p.value.to_le_bytes());
            out.extend_from_slice(&p.weight.to_le_bytes());
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
        let words: Vec<u32> = rest
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        // 値の後にパーツの不透明度の数が要る。
        let parts = words.get(count).map_or(0, |&k| k as usize);
        let expected = (count + 1 + parts * 3) * 4;
        if rest.len() != expected {
            return Err(FrameError::Length {
                expected,
                actual: rest.len(),
            });
        }
        let (part_ids, pairs) = words[count + 1..].split_at(parts);
        Ok(Self {
            seq: u64::from_le_bytes(seq),
            revision: u32_at(4),
            dt: f32::from_bits(u32_at(16)),
            reset: header[1] & FLAG_RESET != 0,
            lip_sync: (header[1] & FLAG_LIP_SYNC != 0).then(|| LipSyncInput {
                level: f32::from_bits(u32_at(20)),
                strength: f32::from_bits(u32_at(24)),
            }),
            values: words[..count].iter().map(|&w| f32::from_bits(w)).collect(),
            part_opacities: part_ids
                .iter()
                .zip(pairs.as_chunks::<2>().0)
                .map(|(&part, &[value, weight])| PartOpacityInput {
                    part,
                    value: f32::from_bits(value),
                    weight: f32::from_bits(weight),
                })
                .collect(),
        })
    }
}

/// シーンの通知 (JSON。`kind` は `"scene"`)。接続したときと、シーンの状態が変わったときに配る。
///
/// 受け手はモデルのバイト列を `scene_model` のコマンドで `revision` を付けて取る (任意のパスは読ませない)。
/// モジュールと背景の画像も同じく `scene_module`・`scene_background` で取る。
///
/// 受け手は版の番号を比べて、することを決める (`docs/studio.md#シーンの切り替えと受け手`)。
/// - `model_revision` が変わった: モデルを読み直し、`revision` のフレームを最初から適用する (物理は新しく始まる)。
/// - `modules_revision` だけが変わった: モジュールを付け直す。物理のリセットはフレームの印で揃える。
/// - どちらも同じ: 見せ方 (ライティング・配置・背景・ポーズの要素) だけを変え、前のシーンのフレームも続けて適用する。
///
/// どの場合も、読み直し・付け直しの後に `pose` の要素を表示する要素にする (受け手のポーズは受け手が進めるため。T0842)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", rename = "scene")]
pub struct Scene {
    /// シーンの版。シーンの状態 (モデル・着せ替え・見せ方) が変わるたびに上がる。
    pub revision: u32,
    /// モデルを読んだときの `revision`。モデルが無ければ 0。
    pub model_revision: u32,
    /// 付けるモジュールの組を変えたときの `revision`。
    pub modules_revision: u32,
    /// モデルの表示名 (ファイル名)。モデルが無ければ空。
    pub model_name: String,
    /// フレームの `values` の並び (モデルのパラメータの定義順)。
    pub parameter_ids: Vec<String>,
    /// フレームの `part_opacities` のパーツの番号の並び (合成したモデルの `parts` の ID の順。モジュールのパーツは
    /// `<モジュール>/<パーツ>`)。受け手は番号をこの ID に直し、自分のモデルの番号に引き直して当てる (T0853)。
    pub part_ids: Vec<String>,
    /// 付けているモジュールの名前 (合成の順。`scene_module` の添字と同じ順)。
    pub modules: Vec<String>,
    /// 見せ方。
    #[serde(flatten)]
    pub look: Look,
}

/// シーンの見せ方 (物理に関わらないもの)。
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    /// 光源の設定 (`@ugoe/player` の `LightingSettings`)。`None` は既定の光源。
    pub lighting: Option<serde_json::Value>,
    pub transform: Transform,
    pub background: Background,
    /// ポーズのグループごとに表示する要素のパーツ (合成したモデルのポーズのグループの順。T0842)。
    /// 指揮役が、シーンの設定のうちモデルにある要素と、挙がっていないグループの先頭の要素で埋めて配る。
    pub pose: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `packages/player/src/follower.test.ts` の `GOLDEN` と同じバイト列。
    const GOLDEN: [u8; 56] = [
        1, 3, 0, 0, // 種類・フラグ (リセットとリップシンク)・予約
        7, 0, 0, 0, // revision = 7
        0x2a, 0x01, 0, 0, 0, 0, 0, 0, // seq = 298
        0x89, 0x88, 0x88, 0x3c, // dt = 1/60
        0, 0, 0, 0x3f, // リップシンクの音量 = 0.5
        0, 0, 0x40, 0x3f, // リップシンクの強さ = 0.75
        2, 0, 0, 0, // 値の数 = 2
        0, 0, 0xc0, 0x3f, // 1.5
        0, 0, 0x20, 0xc1, // -10
        1, 0, 0, 0, // パーツの不透明度の数 = 1
        3, 0, 0, 0, // パーツの番号 = 3
        0, 0, 0x80, 0x3e, // 値 = 0.25
        0, 0, 0, 0x3f, // 重み = 0.5
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
            part_opacities: vec![PartOpacityInput {
                part: 3,
                value: 0.25,
                weight: 0.5,
            }],
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
            part_opacities: (0..5)
                .map(|i| PartOpacityInput {
                    part: i * 2,
                    value: i as f32 * 0.2,
                    weight: 1.0 - i as f32 * 0.1,
                })
                .collect(),
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
            Frame::decode(&GOLDEN[..54]),
            Err(FrameError::Length {
                expected: 24,
                actual: 22
            })
        );
        // パーツの不透明度の数が無い (T0134 までの形)。
        assert_eq!(
            Frame::decode(&GOLDEN[..40]),
            Err(FrameError::Length {
                expected: 12,
                actual: 8
            })
        );
    }

    #[test]
    fn scene_is_tagged_json() {
        let scene = Scene {
            revision: 3,
            model_revision: 2,
            modules_revision: 2,
            model_name: "avatar.ugoe".into(),
            parameter_ids: vec!["AngleX".into()],
            part_ids: vec!["Root".into(), "cat-ears/Ears".into()],
            modules: vec!["cat-ears".into()],
            look: Look::default(),
        };
        assert_eq!(
            serde_json::to_value(&scene).unwrap(),
            serde_json::json!({
                "kind": "scene",
                "revision": 3,
                "modelRevision": 2,
                "modulesRevision": 2,
                "modelName": "avatar.ugoe",
                "parameterIds": ["AngleX"],
                "partIds": ["Root", "cat-ears/Ears"],
                "modules": ["cat-ears"],
                "lighting": null,
                "transform": { "x": 0.0, "y": 0.0, "scale": 1.0, "rotation": 0.0 },
                "background": { "kind": "transparent" },
                "pose": []
            })
        );
    }
}
