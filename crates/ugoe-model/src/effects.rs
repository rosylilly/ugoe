//! 目パチ・呼吸・リップシンクの設定データ。再生は `ugoe-motion`、仕様は `docs/spec/motion.md#エフェクト`。
//!
//! `Default` は「対象パラメータなし (何もしない)」で、時間などの値だけ仕様の既定値を持つ。
//! 標準パラメータ (`crate::standard`) を対象にした推奨設定は `standard()` で作る。

use serde::{Deserialize, Serialize};

use crate::ParamId;
use crate::standard;

/// 目パチのエフェクトグループ名 (`motion::CurveTarget::EffectGroup`)。
pub const EYE_BLINK_GROUP: &str = "EyeBlink";
/// リップシンクのエフェクトグループ名 (`motion::CurveTarget::EffectGroup`)。
pub const LIP_SYNC_GROUP: &str = "LipSync";

/// エフェクト設定一式。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Effects {
    pub eye_blink: EyeBlink,
    pub breath: Breath,
    pub lip_sync: LipSync,
}

impl Effects {
    /// 標準パラメータを対象にした推奨設定。
    pub fn standard() -> Self {
        Self {
            eye_blink: EyeBlink::standard(),
            breath: Breath::standard(),
            lip_sync: LipSync::standard(),
        }
    }
}

/// 目パチ。対象パラメータを 1 → 0 → 1 と動かす。時間はすべて秒。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EyeBlink {
    /// エフェクトグループ `EyeBlink` の対象。
    pub parameters: Vec<ParamId>,
    /// まばたきの平均間隔。0 より大きい。
    pub interval: f32,
    /// 閉じるのにかける時間。0 以上。
    pub closing: f32,
    /// 閉じたまま保つ時間。0 以上。
    pub closed: f32,
    /// 開くのにかける時間。0 以上。
    pub opening: f32,
    /// 間隔を決める決定的な擬似乱数のシード。
    pub seed: u64,
}

impl Default for EyeBlink {
    fn default() -> Self {
        Self {
            parameters: Vec::new(),
            interval: 4.0,
            closing: 0.1,
            closed: 0.05,
            opening: 0.15,
            seed: 0,
        }
    }
}

impl EyeBlink {
    /// `EyeLOpen` と `EyeROpen` を対象にした既定の目パチ。
    pub fn standard() -> Self {
        Self {
            parameters: vec![standard::EYE_L_OPEN.into(), standard::EYE_R_OPEN.into()],
            ..Self::default()
        }
    }
}

/// 呼吸。各パラメータに `weight * (offset + peak * sin(2π t / cycle))` を加算する。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Breath {
    pub parameters: Vec<BreathParameter>,
}

impl Breath {
    /// 標準パラメータを対象にした既定の呼吸。周期は互いに割り切れない値にして、動きの繰り返しを目立たなくする。
    pub fn standard() -> Self {
        let p = |id: &str, offset, peak, cycle, weight| BreathParameter {
            parameter: id.into(),
            offset,
            peak,
            cycle,
            weight,
        };
        Self {
            parameters: vec![
                p(standard::ANGLE_X, 0.0, 15.0, 6.5345, 0.5),
                p(standard::ANGLE_Y, 0.0, 8.0, 3.5345, 0.5),
                p(standard::ANGLE_Z, 0.0, 10.0, 5.5345, 0.5),
                p(standard::BODY_ANGLE_X, 0.0, 4.0, 15.5345, 0.5),
                p(standard::BREATH, 0.5, 0.5, 3.2345, 1.0),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreathParameter {
    pub parameter: ParamId,
    /// 波の中心値。
    pub offset: f32,
    /// 波の振幅。
    pub peak: f32,
    /// 周期 (秒)。0 より大きい。
    pub cycle: f32,
    /// 加算するときに掛ける重み。
    pub weight: f32,
}

/// リップシンク。外部から与えた音量 (0〜1) を対象パラメータに書き込む。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LipSync {
    /// エフェクトグループ `LipSync` の対象。
    pub parameters: Vec<ParamId>,
}

impl LipSync {
    /// `MouthOpenY` を対象にした既定のリップシンク。
    pub fn standard() -> Self {
        Self {
            parameters: vec![standard::MOUTH_OPEN_Y.into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_roundtrips_json() {
        let effects = Effects::standard();
        let json = serde_json::to_string(&effects).unwrap();
        let back: Effects = serde_json::from_str(&json).unwrap();
        assert_eq!(effects, back);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let effects: Effects =
            serde_json::from_str(r#"{"eye_blink":{"parameters":["EyeLOpen"],"seed":7}}"#).unwrap();
        assert_eq!(
            effects,
            Effects {
                eye_blink: EyeBlink {
                    parameters: vec!["EyeLOpen".into()],
                    seed: 7,
                    ..EyeBlink::default()
                },
                ..Effects::default()
            }
        );
        assert_eq!(effects.eye_blink.interval, 4.0);
    }

    #[test]
    fn standard_targets_are_standard_parameters() {
        let known = |id: &ParamId| standard::ALL.iter().any(|(s, ..)| id.as_str() == *s);
        let effects = Effects::standard();
        assert!(effects.eye_blink.parameters.iter().all(known));
        assert!(effects.lip_sync.parameters.iter().all(known));
        assert!(
            effects
                .breath
                .parameters
                .iter()
                .all(|b| known(&b.parameter))
        );
    }
}
