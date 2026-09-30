//! 標準パラメータ ID。
//!
//! モーション・表情・物理プリセット・AI 自動リギングがモデル間で再利用できるよう、
//! 意味の決まったパラメータ ID を定義する。値域の慣例は `docs/spec/model.md#標準パラメータ`。

pub const ANGLE_X: &str = "AngleX";
pub const ANGLE_Y: &str = "AngleY";
pub const ANGLE_Z: &str = "AngleZ";
pub const BODY_ANGLE_X: &str = "BodyAngleX";
pub const BODY_ANGLE_Y: &str = "BodyAngleY";
pub const BODY_ANGLE_Z: &str = "BodyAngleZ";
pub const BREATH: &str = "Breath";
pub const EYE_L_OPEN: &str = "EyeLOpen";
pub const EYE_R_OPEN: &str = "EyeROpen";
pub const EYE_L_SMILE: &str = "EyeLSmile";
pub const EYE_R_SMILE: &str = "EyeRSmile";
pub const EYE_BALL_X: &str = "EyeBallX";
pub const EYE_BALL_Y: &str = "EyeBallY";
pub const BROW_L_Y: &str = "BrowLY";
pub const BROW_R_Y: &str = "BrowRY";
pub const MOUTH_FORM: &str = "MouthForm";
pub const MOUTH_OPEN_Y: &str = "MouthOpenY";
pub const CHEEK: &str = "Cheek";
pub const HAIR_FRONT: &str = "HairFront";
pub const HAIR_SIDE: &str = "HairSide";
pub const HAIR_BACK: &str = "HairBack";

/// 全標準パラメータと既定の値域 `(id, min, default, max)`。
pub const ALL: &[(&str, f32, f32, f32)] = &[
    (ANGLE_X, -30.0, 0.0, 30.0),
    (ANGLE_Y, -30.0, 0.0, 30.0),
    (ANGLE_Z, -30.0, 0.0, 30.0),
    (BODY_ANGLE_X, -10.0, 0.0, 10.0),
    (BODY_ANGLE_Y, -10.0, 0.0, 10.0),
    (BODY_ANGLE_Z, -10.0, 0.0, 10.0),
    (BREATH, 0.0, 0.0, 1.0),
    (EYE_L_OPEN, 0.0, 1.0, 1.0),
    (EYE_R_OPEN, 0.0, 1.0, 1.0),
    (EYE_L_SMILE, 0.0, 0.0, 1.0),
    (EYE_R_SMILE, 0.0, 0.0, 1.0),
    (EYE_BALL_X, -1.0, 0.0, 1.0),
    (EYE_BALL_Y, -1.0, 0.0, 1.0),
    (BROW_L_Y, -1.0, 0.0, 1.0),
    (BROW_R_Y, -1.0, 0.0, 1.0),
    (MOUTH_FORM, -1.0, 0.0, 1.0),
    (MOUTH_OPEN_Y, 0.0, 0.0, 1.0),
    (CHEEK, 0.0, 0.0, 1.0),
    (HAIR_FRONT, -1.0, 0.0, 1.0),
    (HAIR_SIDE, -1.0, 0.0, 1.0),
    (HAIR_BACK, -1.0, 0.0, 1.0),
];

// アバタープロファイルの拡張パラメータ (`docs/spec/avatar-profile.md#拡張パラメータ-レベル-3`)。
// `EYE_L_SMILE` / `EYE_R_SMILE` は上の標準パラメータと共通。

pub const EYE_L_WIDE: &str = "EyeLWide";
pub const EYE_R_WIDE: &str = "EyeRWide";
pub const BROW_INNER_UP: &str = "BrowInnerUp";
pub const BROW_L_ANGLE: &str = "BrowLAngle";
pub const BROW_R_ANGLE: &str = "BrowRAngle";
pub const MOUTH_X: &str = "MouthX";
pub const JAW_X: &str = "JawX";
pub const MOUTH_PUCKER: &str = "MouthPucker";
pub const MOUTH_FUNNEL: &str = "MouthFunnel";
pub const MOUTH_PRESS: &str = "MouthPress";
pub const MOUTH_SHRUG: &str = "MouthShrug";
pub const CHEEK_PUFF: &str = "CheekPuff";
pub const TONGUE_OUT: &str = "TongueOut";
pub const NOSE_SNEER: &str = "NoseSneer";

/// アバタープロファイルの拡張パラメータと既定の値域 `(id, min, default, max)`。
/// 仕様書の表と同じ順。`EyeLSmile` / `EyeRSmile` は [`ALL`] にも含まれる。
pub const EXTENDED: &[(&str, f32, f32, f32)] = &[
    (EYE_L_WIDE, 0.0, 0.0, 1.0),
    (EYE_R_WIDE, 0.0, 0.0, 1.0),
    (EYE_L_SMILE, 0.0, 0.0, 1.0),
    (EYE_R_SMILE, 0.0, 0.0, 1.0),
    (BROW_INNER_UP, 0.0, 0.0, 1.0),
    (BROW_L_ANGLE, -1.0, 0.0, 1.0),
    (BROW_R_ANGLE, -1.0, 0.0, 1.0),
    (MOUTH_X, -1.0, 0.0, 1.0),
    (JAW_X, -1.0, 0.0, 1.0),
    (MOUTH_PUCKER, 0.0, 0.0, 1.0),
    (MOUTH_FUNNEL, 0.0, 0.0, 1.0),
    (MOUTH_PRESS, 0.0, 0.0, 1.0),
    (MOUTH_SHRUG, 0.0, 0.0, 1.0),
    (CHEEK_PUFF, 0.0, 0.0, 1.0),
    (TONGUE_OUT, 0.0, 0.0, 1.0),
    (NOSE_SNEER, 0.0, 0.0, 1.0),
];

// アバタープロファイルの体の拡張パラメータ (`docs/spec/avatar-profile.md#体の拡張パラメータ`)。
// 体のトラッキング (Pose Landmarker など) で動かす。レベル 3 の拡張パラメータの数には数えない。

pub const SHOULDER_L_Y: &str = "ShoulderLY";
pub const SHOULDER_R_Y: &str = "ShoulderRY";

/// アバタープロファイルの体の拡張パラメータと既定の値域 `(id, min, default, max)`。
pub const BODY_EXTENDED: &[(&str, f32, f32, f32)] = &[
    (SHOULDER_L_Y, -1.0, 0.0, 1.0),
    (SHOULDER_R_Y, -1.0, 0.0, 1.0),
];

// 音声からの母音の推定 (`docs/spec/avatar-profile.md#音声-リップシンク`)。

pub const MOUTH_VOWEL_A: &str = "MouthVowelA";
pub const MOUTH_VOWEL_I: &str = "MouthVowelI";
pub const MOUTH_VOWEL_U: &str = "MouthVowelU";
pub const MOUTH_VOWEL_E: &str = "MouthVowelE";
pub const MOUTH_VOWEL_O: &str = "MouthVowelO";

/// 母音のパラメータと既定の値域 `(id, min, default, max)`。
pub const VOWELS: &[(&str, f32, f32, f32)] = &[
    (MOUTH_VOWEL_A, 0.0, 0.0, 1.0),
    (MOUTH_VOWEL_I, 0.0, 0.0, 1.0),
    (MOUTH_VOWEL_U, 0.0, 0.0, 1.0),
    (MOUTH_VOWEL_E, 0.0, 0.0, 1.0),
    (MOUTH_VOWEL_O, 0.0, 0.0, 1.0),
];

/// ID から、[`ALL`]・[`EXTENDED`]・[`BODY_EXTENDED`]・[`VOWELS`] のいずれかにある既定の値域
/// `(min, default, max)` を返す。
pub fn range(id: &str) -> Option<(f32, f32, f32)> {
    ALL.iter()
        .chain(EXTENDED)
        .chain(BODY_EXTENDED)
        .chain(VOWELS)
        .find(|(s, ..)| *s == id)
        .map(|&(_, min, default, max)| (min, default, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_except_shared_smiles() {
        let mut ids: Vec<&str> = ALL
            .iter()
            .chain(EXTENDED)
            .chain(BODY_EXTENDED)
            .chain(VOWELS)
            .map(|p| p.0)
            .collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        // EyeLSmile と EyeRSmile だけが ALL と EXTENDED の両方にある。
        assert_eq!(before - ids.len(), 2);
    }

    #[test]
    fn ranges_contain_their_defaults() {
        for &(id, min, default, max) in ALL
            .iter()
            .chain(EXTENDED)
            .chain(BODY_EXTENDED)
            .chain(VOWELS)
        {
            assert!(min <= default && default <= max, "{id}");
            assert_eq!(range(id), Some((min, default, max)));
        }
        assert_eq!(range("Nope"), None);
    }
}
