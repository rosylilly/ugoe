//! 標準パラメータ ID。
//!
//! モーション・表情・物理プリセット・AI 自動リギングがモデル間で再利用できるよう、
//! 意味の決まったパラメータ ID を定義する。値域の慣例は `docs/spec/model.md#標準パラメータ`。

/// 頭の左右の向き (-30〜30 度)。正でキャラクターから見て左 (画面では右) を向く。
pub const ANGLE_X: &str = "AngleX";
/// 頭の上下の向き (-30〜30 度)。正で上を向く。
pub const ANGLE_Y: &str = "AngleY";
/// 頭の傾き (-30〜30 度)。正で時計回りに傾く。
pub const ANGLE_Z: &str = "AngleZ";
/// 体の左右の向き (-10〜10 度)。符号は [`ANGLE_X`] と同じ。
pub const BODY_ANGLE_X: &str = "BodyAngleX";
/// 体の上下の向き (-10〜10 度)。符号は [`ANGLE_Y`] と同じ。
pub const BODY_ANGLE_Y: &str = "BodyAngleY";
/// 体の傾き (-10〜10 度)。符号は [`ANGLE_Z`] と同じ。
pub const BODY_ANGLE_Z: &str = "BodyAngleZ";
/// 呼吸 (0〜1)。常時エフェクトの呼吸が周期的に動かす。
pub const BREATH: &str = "Breath";
/// 左目 (キャラクター自身の左) の開閉。0 で閉じ、1 (既定) で開く。
pub const EYE_L_OPEN: &str = "EyeLOpen";
/// 右目の開閉。0 で閉じ、1 (既定) で開く。
pub const EYE_R_OPEN: &str = "EyeROpen";
/// 左目を細める (笑い目、0〜1)。拡張パラメータ ([`EXTENDED`]) にも含まれる。
pub const EYE_L_SMILE: &str = "EyeLSmile";
/// 右目を細める (笑い目、0〜1)。拡張パラメータ ([`EXTENDED`]) にも含まれる。
pub const EYE_R_SMILE: &str = "EyeRSmile";
/// 視線の左右 (-1〜1、0 が正面)。
pub const EYE_BALL_X: &str = "EyeBallX";
/// 視線の上下 (-1〜1、0 が正面)。
pub const EYE_BALL_Y: &str = "EyeBallY";
/// 左眉の上下 (-1〜1、0 が中立)。
pub const BROW_L_Y: &str = "BrowLY";
/// 右眉の上下 (-1〜1、0 が中立)。
pub const BROW_R_Y: &str = "BrowRY";
/// 口の形 (-1〜1、0 が中立)。
pub const MOUTH_FORM: &str = "MouthForm";
/// 口の開き (0〜1)。0 で閉じる。リップシンクもここに書き込む。
pub const MOUTH_OPEN_Y: &str = "MouthOpenY";
/// 頬の赤み (照れ、0〜1)。
pub const CHEEK: &str = "Cheek";
/// 前髪の揺れ (-1〜1)。物理の出力に使う。
pub const HAIR_FRONT: &str = "HairFront";
/// 横髪の揺れ (-1〜1)。物理の出力に使う。
pub const HAIR_SIDE: &str = "HairSide";
/// 後ろ髪の揺れ (-1〜1)。物理の出力に使う。
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

/// 左目の見開き (0〜1)。
pub const EYE_L_WIDE: &str = "EyeLWide";
/// 右目の見開き (0〜1)。
pub const EYE_R_WIDE: &str = "EyeRWide";
/// 眉の内側を上げる (困り眉、0〜1)。
pub const BROW_INNER_UP: &str = "BrowInnerUp";
/// 左眉の傾き (-1〜1)。正で外側が上がる。
pub const BROW_L_ANGLE: &str = "BrowLAngle";
/// 右眉の傾き (-1〜1)。正で外側が上がる。
pub const BROW_R_ANGLE: &str = "BrowRAngle";
/// 口の左右の位置 (-1〜1)。正でキャラクターの左。
pub const MOUTH_X: &str = "MouthX";
/// あごの左右の位置 (-1〜1)。正でキャラクターの左。
pub const JAW_X: &str = "JawX";
/// 口をすぼめる (0〜1)。
pub const MOUTH_PUCKER: &str = "MouthPucker";
/// 口を丸く突き出す (0〜1)。
pub const MOUTH_FUNNEL: &str = "MouthFunnel";
/// 唇を押し付ける (0〜1)。
pub const MOUTH_PRESS: &str = "MouthPress";
/// 口元を上げる (への字の逆、0〜1)。
pub const MOUTH_SHRUG: &str = "MouthShrug";
/// 頬をふくらませる (0〜1)。
pub const CHEEK_PUFF: &str = "CheekPuff";
/// 舌を出す (0〜1)。
pub const TONGUE_OUT: &str = "TongueOut";
/// 鼻にしわを寄せる (0〜1)。
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

/// 左肩 (キャラクター自身の左) の上下 (-1〜1)。0 が中立、1 で肩をすくめた状態、負で下げた状態。
pub const SHOULDER_L_Y: &str = "ShoulderLY";
/// 右肩の上下 (-1〜1)。0 が中立、1 で肩をすくめた状態、負で下げた状態。
pub const SHOULDER_R_Y: &str = "ShoulderRY";

/// アバタープロファイルの体の拡張パラメータと既定の値域 `(id, min, default, max)`。
pub const BODY_EXTENDED: &[(&str, f32, f32, f32)] = &[
    (SHOULDER_L_Y, -1.0, 0.0, 1.0),
    (SHOULDER_R_Y, -1.0, 0.0, 1.0),
];

// 音声からの母音の推定 (`docs/spec/avatar-profile.md#音声-リップシンク`)。

/// 母音「あ」の口の形 (0〜1)。
pub const MOUTH_VOWEL_A: &str = "MouthVowelA";
/// 母音「い」の口の形 (0〜1)。
pub const MOUTH_VOWEL_I: &str = "MouthVowelI";
/// 母音「う」の口の形 (0〜1)。
pub const MOUTH_VOWEL_U: &str = "MouthVowelU";
/// 母音「え」の口の形 (0〜1)。
pub const MOUTH_VOWEL_E: &str = "MouthVowelE";
/// 母音「お」の口の形 (0〜1)。
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
