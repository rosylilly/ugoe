//! Godot の `XRFaceTracker` (Unified Expressions) から、トラッキングの入力 (ARKit の 52 ブレンドシェイプ) への対応表。
//!
//! `XRFaceTracker.get_blend_shapes()` は `BlendShapeEntry` の順 (`FT_*`、143 個) の値 (0〜1) を返す。
//! ここではそれを `docs/spec/avatar-profile.md#arkit-のブレンドシェイプ` の入力 (ARKit の名前) に直す。
//! 出典: Godot の `XRFaceTracker` (https://docs.godotengine.org/en/stable/classes/class_xrfacetracker.html)、
//! Unified Expressions (https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/unified-blendshapes)。
//!
//! 対応の決め方:
//!
//! - 左右は Unified Expressions も ARKit もトラッキングされた人の左右なので、入れ替えない。
//! - Unified Expressions が細かく分けている形 (唇の上下と左右・口角の引きと傾き・眉の下げと寄せ) は、
//!   分けた形の平均か最大を ARKit の 1 つの形に当てる。平均は「同時に動く部分を合わせた量」(唇のすぼめ・巻き込みなど)、
//!   最大は「どちらか一方でその表情になる」(笑いの口角・眉の下げ) の形に使う。
//! - Unified Expressions の合成の形 (`FT_EYE_CLOSED` 以降の左右・上下をまとめたもの) は、トラッカーが書くとは限らない
//!   (多くは基本の形だけを書く)。書かれていれば使えるように、基本の形から求めた値との最大を取る。
//! - 対応する ARKit の形がない Unified Expressions の形 (瞳孔・鼻孔・舌の細かい動き・喉・首など) は使わない。

/// `XRFaceTracker.BlendShapeEntry` の値 (Godot 4.3 以降。Godot の API の値で、版を通して変わらない)。
#[allow(dead_code)]
mod ft {
    pub const EYE_LOOK_OUT_RIGHT: usize = 0;
    pub const EYE_LOOK_IN_RIGHT: usize = 1;
    pub const EYE_LOOK_UP_RIGHT: usize = 2;
    pub const EYE_LOOK_DOWN_RIGHT: usize = 3;
    pub const EYE_LOOK_OUT_LEFT: usize = 4;
    pub const EYE_LOOK_IN_LEFT: usize = 5;
    pub const EYE_LOOK_UP_LEFT: usize = 6;
    pub const EYE_LOOK_DOWN_LEFT: usize = 7;
    pub const EYE_CLOSED_RIGHT: usize = 8;
    pub const EYE_CLOSED_LEFT: usize = 9;
    pub const EYE_SQUINT_RIGHT: usize = 10;
    pub const EYE_SQUINT_LEFT: usize = 11;
    pub const EYE_WIDE_RIGHT: usize = 12;
    pub const EYE_WIDE_LEFT: usize = 13;
    pub const BROW_PINCH_RIGHT: usize = 18;
    pub const BROW_PINCH_LEFT: usize = 19;
    pub const BROW_LOWERER_RIGHT: usize = 20;
    pub const BROW_LOWERER_LEFT: usize = 21;
    pub const BROW_INNER_UP_RIGHT: usize = 22;
    pub const BROW_INNER_UP_LEFT: usize = 23;
    pub const BROW_OUTER_UP_RIGHT: usize = 24;
    pub const BROW_OUTER_UP_LEFT: usize = 25;
    pub const NOSE_SNEER_RIGHT: usize = 26;
    pub const NOSE_SNEER_LEFT: usize = 27;
    pub const CHEEK_SQUINT_RIGHT: usize = 32;
    pub const CHEEK_SQUINT_LEFT: usize = 33;
    pub const CHEEK_PUFF_RIGHT: usize = 34;
    pub const CHEEK_PUFF_LEFT: usize = 35;
    pub const JAW_OPEN: usize = 38;
    pub const MOUTH_CLOSED: usize = 39;
    pub const JAW_RIGHT: usize = 40;
    pub const JAW_LEFT: usize = 41;
    pub const JAW_FORWARD: usize = 42;
    pub const LIP_SUCK_UPPER_RIGHT: usize = 46;
    pub const LIP_SUCK_UPPER_LEFT: usize = 47;
    pub const LIP_SUCK_LOWER_RIGHT: usize = 48;
    pub const LIP_SUCK_LOWER_LEFT: usize = 49;
    pub const LIP_FUNNEL_UPPER_RIGHT: usize = 52;
    pub const LIP_FUNNEL_UPPER_LEFT: usize = 53;
    pub const LIP_FUNNEL_LOWER_RIGHT: usize = 54;
    pub const LIP_FUNNEL_LOWER_LEFT: usize = 55;
    pub const LIP_PUCKER_UPPER_RIGHT: usize = 56;
    pub const LIP_PUCKER_UPPER_LEFT: usize = 57;
    pub const LIP_PUCKER_LOWER_RIGHT: usize = 58;
    pub const LIP_PUCKER_LOWER_LEFT: usize = 59;
    pub const MOUTH_UPPER_UP_RIGHT: usize = 60;
    pub const MOUTH_UPPER_UP_LEFT: usize = 61;
    pub const MOUTH_LOWER_DOWN_RIGHT: usize = 62;
    pub const MOUTH_LOWER_DOWN_LEFT: usize = 63;
    pub const MOUTH_UPPER_RIGHT: usize = 66;
    pub const MOUTH_UPPER_LEFT: usize = 67;
    pub const MOUTH_LOWER_RIGHT: usize = 68;
    pub const MOUTH_LOWER_LEFT: usize = 69;
    pub const MOUTH_CORNER_PULL_RIGHT: usize = 70;
    pub const MOUTH_CORNER_PULL_LEFT: usize = 71;
    pub const MOUTH_CORNER_SLANT_RIGHT: usize = 72;
    pub const MOUTH_CORNER_SLANT_LEFT: usize = 73;
    pub const MOUTH_FROWN_RIGHT: usize = 74;
    pub const MOUTH_FROWN_LEFT: usize = 75;
    pub const MOUTH_STRETCH_RIGHT: usize = 76;
    pub const MOUTH_STRETCH_LEFT: usize = 77;
    pub const MOUTH_DIMPLE_RIGHT: usize = 78;
    pub const MOUTH_DIMPLE_LEFT: usize = 79;
    pub const MOUTH_RAISER_UPPER: usize = 80;
    pub const MOUTH_RAISER_LOWER: usize = 81;
    pub const MOUTH_PRESS_RIGHT: usize = 82;
    pub const MOUTH_PRESS_LEFT: usize = 83;
    pub const TONGUE_OUT: usize = 86;
    // 合成の形
    pub const BROW_DOWN_RIGHT: usize = 107;
    pub const BROW_DOWN_LEFT: usize = 108;
    pub const CHEEK_PUFF: usize = 116;
    pub const LIP_SUCK_UPPER: usize = 119;
    pub const LIP_SUCK_LOWER: usize = 120;
    pub const LIP_FUNNEL: usize = 124;
    pub const LIP_PUCKER: usize = 127;
    pub const MOUTH_RIGHT: usize = 131;
    pub const MOUTH_LEFT: usize = 132;
    pub const MOUTH_SMILE_RIGHT: usize = 133;
    pub const MOUTH_SMILE_LEFT: usize = 134;
    /// `FT_MAX` (形の数)。
    pub const COUNT: usize = 143;
}

/// ARKit の形の値の求め方 (Unified Expressions の形の番号の組)。
#[derive(Debug, Clone, Copy)]
enum Rule {
    /// その形の値。
    One(usize),
    /// 形の値の平均。
    Mean(&'static [usize]),
    /// 形の値の最大。
    Max(&'static [usize]),
    /// 基本の形の平均と、合成の形の大きい方。
    MeanOr(&'static [usize], usize),
}

/// ARKit の 52 ブレンドシェイプ (`ugoe::model::avatar::ARKIT_BLEND_SHAPES` の順) ごとの求め方 (README の対応表と同じ)。
const RULES: [(&str, Rule); 52] = {
    use Rule::*;
    use ft::*;
    [
        ("eyeBlinkLeft", One(EYE_CLOSED_LEFT)),
        ("eyeLookDownLeft", One(EYE_LOOK_DOWN_LEFT)),
        ("eyeLookInLeft", One(EYE_LOOK_IN_LEFT)),
        ("eyeLookOutLeft", One(EYE_LOOK_OUT_LEFT)),
        ("eyeLookUpLeft", One(EYE_LOOK_UP_LEFT)),
        ("eyeSquintLeft", One(EYE_SQUINT_LEFT)),
        ("eyeWideLeft", One(EYE_WIDE_LEFT)),
        ("eyeBlinkRight", One(EYE_CLOSED_RIGHT)),
        ("eyeLookDownRight", One(EYE_LOOK_DOWN_RIGHT)),
        ("eyeLookInRight", One(EYE_LOOK_IN_RIGHT)),
        ("eyeLookOutRight", One(EYE_LOOK_OUT_RIGHT)),
        ("eyeLookUpRight", One(EYE_LOOK_UP_RIGHT)),
        ("eyeSquintRight", One(EYE_SQUINT_RIGHT)),
        ("eyeWideRight", One(EYE_WIDE_RIGHT)),
        ("jawForward", One(JAW_FORWARD)),
        ("jawLeft", One(JAW_LEFT)),
        ("jawRight", One(JAW_RIGHT)),
        ("jawOpen", One(JAW_OPEN)),
        ("mouthClose", One(MOUTH_CLOSED)),
        (
            "mouthFunnel",
            MeanOr(
                &[
                    LIP_FUNNEL_UPPER_RIGHT,
                    LIP_FUNNEL_UPPER_LEFT,
                    LIP_FUNNEL_LOWER_RIGHT,
                    LIP_FUNNEL_LOWER_LEFT,
                ],
                LIP_FUNNEL,
            ),
        ),
        (
            "mouthPucker",
            MeanOr(
                &[
                    LIP_PUCKER_UPPER_RIGHT,
                    LIP_PUCKER_UPPER_LEFT,
                    LIP_PUCKER_LOWER_RIGHT,
                    LIP_PUCKER_LOWER_LEFT,
                ],
                LIP_PUCKER,
            ),
        ),
        (
            "mouthLeft",
            MeanOr(&[MOUTH_UPPER_LEFT, MOUTH_LOWER_LEFT], MOUTH_LEFT),
        ),
        (
            "mouthRight",
            MeanOr(&[MOUTH_UPPER_RIGHT, MOUTH_LOWER_RIGHT], MOUTH_RIGHT),
        ),
        (
            "mouthSmileLeft",
            Max(&[
                MOUTH_CORNER_PULL_LEFT,
                MOUTH_CORNER_SLANT_LEFT,
                MOUTH_SMILE_LEFT,
            ]),
        ),
        (
            "mouthSmileRight",
            Max(&[
                MOUTH_CORNER_PULL_RIGHT,
                MOUTH_CORNER_SLANT_RIGHT,
                MOUTH_SMILE_RIGHT,
            ]),
        ),
        ("mouthFrownLeft", One(MOUTH_FROWN_LEFT)),
        ("mouthFrownRight", One(MOUTH_FROWN_RIGHT)),
        ("mouthDimpleLeft", One(MOUTH_DIMPLE_LEFT)),
        ("mouthDimpleRight", One(MOUTH_DIMPLE_RIGHT)),
        ("mouthStretchLeft", One(MOUTH_STRETCH_LEFT)),
        ("mouthStretchRight", One(MOUTH_STRETCH_RIGHT)),
        (
            "mouthRollLower",
            MeanOr(&[LIP_SUCK_LOWER_RIGHT, LIP_SUCK_LOWER_LEFT], LIP_SUCK_LOWER),
        ),
        (
            "mouthRollUpper",
            MeanOr(&[LIP_SUCK_UPPER_RIGHT, LIP_SUCK_UPPER_LEFT], LIP_SUCK_UPPER),
        ),
        ("mouthShrugLower", One(MOUTH_RAISER_LOWER)),
        ("mouthShrugUpper", One(MOUTH_RAISER_UPPER)),
        ("mouthPressLeft", One(MOUTH_PRESS_LEFT)),
        ("mouthPressRight", One(MOUTH_PRESS_RIGHT)),
        ("mouthLowerDownLeft", One(MOUTH_LOWER_DOWN_LEFT)),
        ("mouthLowerDownRight", One(MOUTH_LOWER_DOWN_RIGHT)),
        ("mouthUpperUpLeft", One(MOUTH_UPPER_UP_LEFT)),
        ("mouthUpperUpRight", One(MOUTH_UPPER_UP_RIGHT)),
        (
            "browDownLeft",
            Max(&[BROW_LOWERER_LEFT, BROW_PINCH_LEFT, BROW_DOWN_LEFT]),
        ),
        (
            "browDownRight",
            Max(&[BROW_LOWERER_RIGHT, BROW_PINCH_RIGHT, BROW_DOWN_RIGHT]),
        ),
        (
            "browInnerUp",
            Mean(&[BROW_INNER_UP_RIGHT, BROW_INNER_UP_LEFT]),
        ),
        ("browOuterUpLeft", One(BROW_OUTER_UP_LEFT)),
        ("browOuterUpRight", One(BROW_OUTER_UP_RIGHT)),
        (
            "cheekPuff",
            MeanOr(&[CHEEK_PUFF_RIGHT, CHEEK_PUFF_LEFT], CHEEK_PUFF),
        ),
        ("cheekSquintLeft", One(CHEEK_SQUINT_LEFT)),
        ("cheekSquintRight", One(CHEEK_SQUINT_RIGHT)),
        ("noseSneerLeft", One(NOSE_SNEER_LEFT)),
        ("noseSneerRight", One(NOSE_SNEER_RIGHT)),
        ("tongueOut", One(TONGUE_OUT)),
    ]
};

/// `XRFaceTracker.get_blend_shapes()` の値から、ARKit の 52 ブレンドシェイプの値を求める
/// (`ugoe::model::avatar::ARKIT_BLEND_SHAPES` の順)。足りない形 (古い版や短い配列) は 0、結果は 0〜1 にクランプする。
pub fn arkit_from_unified(values: &[f32]) -> [(&'static str, f32); 52] {
    let at = |i: usize| {
        values
            .get(i)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
    };
    let mean = |is: &[usize]| is.iter().map(|&i| at(i)).sum::<f32>() / is.len() as f32;
    RULES.map(|(name, rule)| {
        let value = match rule {
            Rule::One(i) => at(i),
            Rule::Mean(is) => mean(is),
            Rule::Max(is) => is.iter().map(|&i| at(i)).fold(0.0, f32::max),
            Rule::MeanOr(is, combined) => mean(is).max(at(combined)),
        };
        (name, value.clamp(0.0, 1.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe::model::avatar::ARKIT_BLEND_SHAPES;

    fn value(out: &[(&str, f32); 52], name: &str) -> f32 {
        out.iter().find(|(n, _)| *n == name).expect("name").1
    }

    #[test]
    fn rules_cover_the_arkit_names_in_order() {
        let names: Vec<&str> = RULES.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ARKIT_BLEND_SHAPES);
    }

    #[test]
    fn rules_use_valid_entries() {
        for (name, rule) in RULES {
            let used: Vec<usize> = match rule {
                Rule::One(i) => vec![i],
                Rule::Mean(is) | Rule::Max(is) => is.to_vec(),
                Rule::MeanOr(is, c) => [is, &[c]].concat(),
            };
            assert!(used.iter().all(|&i| i < ft::COUNT), "{name}");
        }
    }

    #[test]
    fn neutral_face_is_all_zero() {
        let out = arkit_from_unified(&[0.0; ft::COUNT]);
        assert!(out.iter().all(|(_, v)| *v == 0.0));
        // 空の配列 (トラッカーがまだ値を書いていない) も 0。
        assert!(arkit_from_unified(&[]).iter().all(|(_, v)| *v == 0.0));
    }

    #[test]
    fn maps_each_side_without_swapping() {
        let mut v = [0.0; ft::COUNT];
        v[ft::EYE_CLOSED_LEFT] = 1.0;
        v[ft::EYE_LOOK_OUT_RIGHT] = 0.5;
        v[ft::JAW_OPEN] = 0.7;
        v[ft::MOUTH_CLOSED] = 0.2;
        v[ft::TONGUE_OUT] = 0.4;
        let out = arkit_from_unified(&v);
        assert_eq!(value(&out, "eyeBlinkLeft"), 1.0);
        assert_eq!(value(&out, "eyeBlinkRight"), 0.0);
        assert_eq!(value(&out, "eyeLookOutRight"), 0.5);
        assert_eq!(value(&out, "eyeLookOutLeft"), 0.0);
        assert_eq!(value(&out, "jawOpen"), 0.7);
        assert_eq!(value(&out, "mouthClose"), 0.2);
        assert_eq!(value(&out, "tongueOut"), 0.4);
    }

    #[test]
    fn split_shapes_are_merged() {
        let mut v = [0.0; ft::COUNT];
        // 唇のすぼめは 4 つの平均
        v[ft::LIP_PUCKER_UPPER_RIGHT] = 1.0;
        v[ft::LIP_PUCKER_UPPER_LEFT] = 1.0;
        // 口角は引きか傾きの大きい方
        v[ft::MOUTH_CORNER_PULL_LEFT] = 0.3;
        v[ft::MOUTH_CORNER_SLANT_LEFT] = 0.6;
        // 眉の下げは下げか寄せの大きい方
        v[ft::BROW_PINCH_RIGHT] = 0.8;
        v[ft::BROW_INNER_UP_LEFT] = 0.4;
        v[ft::CHEEK_PUFF_LEFT] = 1.0;
        v[ft::MOUTH_UPPER_LEFT] = 0.5;
        v[ft::MOUTH_LOWER_LEFT] = 0.3;
        let out = arkit_from_unified(&v);
        assert!((value(&out, "mouthPucker") - 0.5).abs() < 1e-6);
        assert!((value(&out, "mouthSmileLeft") - 0.6).abs() < 1e-6);
        assert_eq!(value(&out, "mouthSmileRight"), 0.0);
        assert!((value(&out, "browDownRight") - 0.8).abs() < 1e-6);
        assert!((value(&out, "browInnerUp") - 0.2).abs() < 1e-6);
        assert!((value(&out, "cheekPuff") - 0.5).abs() < 1e-6);
        assert!((value(&out, "mouthLeft") - 0.4).abs() < 1e-6);
    }

    #[test]
    fn combined_shapes_are_used_when_written() {
        let mut v = [0.0; ft::COUNT];
        v[ft::LIP_FUNNEL] = 0.9;
        v[ft::MOUTH_SMILE_RIGHT] = 0.7;
        v[ft::BROW_DOWN_LEFT] = 0.6;
        let out = arkit_from_unified(&v);
        assert!((value(&out, "mouthFunnel") - 0.9).abs() < 1e-6);
        assert!((value(&out, "mouthSmileRight") - 0.7).abs() < 1e-6);
        assert!((value(&out, "browDownLeft") - 0.6).abs() < 1e-6);
    }

    #[test]
    fn values_are_clamped() {
        let mut v = [0.0; ft::COUNT];
        v[ft::JAW_OPEN] = 1.5;
        v[ft::EYE_WIDE_LEFT] = -0.5;
        v[ft::EYE_WIDE_RIGHT] = f32::NAN;
        let out = arkit_from_unified(&v);
        assert_eq!(value(&out, "jawOpen"), 1.0);
        assert_eq!(value(&out, "eyeWideLeft"), 0.0);
        assert_eq!(value(&out, "eyeWideRight"), 0.0);
    }
}
