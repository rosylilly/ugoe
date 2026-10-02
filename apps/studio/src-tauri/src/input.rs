//! 入力とモーションの設定 (プロファイルの `input`・`motion`。`docs/studio.md#入力`、T0803)。
//!
//! カメラとマイクは操作ウィンドウの WebView で取る (ADR 0028 の 4.) ので、ここは選んだ機器と調整の値を持つだけ。
//! 値は画面が使い、バックエンドは保存と収め直しだけをする (モーションの設定は指揮役が使う)。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ugoe::motion::{BodyInput, Rotation, TrackingInput};

/// 入力の機器 (WebView の `MediaDeviceInfo`)。ID は計算機やブラウザの状態で変わることがあるので、名前でも探す。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Device {
    pub id: String,
    pub label: String,
}

/// 入力の設定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InputSettings {
    /// トラッキングに使うカメラ。`None` は最初に見つかったもの。
    pub camera: Option<Device>,
    /// トラッキングの平滑化の強さ (0〜1)。
    pub smoothing: f64,
    /// 左右を入れ替える (モデルを鏡に映った自分のように動かす)。
    pub swap_sides: bool,
    /// 体 (肩) もトラッキングする。
    pub track_body: bool,
    /// トラッキング中はモデルの自動の目パチを止める。
    pub stop_eye_blink: bool,
    /// 口パクに使うマイク。`None` は既定の機器。
    pub microphone: Option<Device>,
    /// マイクの感度 (0〜1。0.5 で等倍)。
    pub sensitivity: f64,
    /// ノイズの閾値 (0〜1。これより小さな音では口を開かない)。
    pub noise_gate: f64,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            camera: None,
            smoothing: 0.5,
            swap_sides: false,
            track_body: true,
            stop_eye_blink: true,
            microphone: None,
            sensitivity: 0.5,
            noise_gate: 0.3,
        }
    }
}

/// 機器の名前と ID の長さの上限 (文字)。
const MAX_DEVICE_CHARS: usize = 512;

fn unit(v: f64, default: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        default
    }
}

fn sanitize_device(device: &mut Option<Device>) {
    if let Some(d) = device {
        if d.id.is_empty() && d.label.is_empty() {
            *device = None;
            return;
        }
        for s in [&mut d.id, &mut d.label] {
            if s.chars().count() > MAX_DEVICE_CHARS {
                *s = s.chars().take(MAX_DEVICE_CHARS).collect();
            }
        }
    }
}

impl InputSettings {
    /// 値を範囲に収める (数でない値は既定に戻す)。
    pub fn sanitize(&mut self) {
        let default = Self::default();
        self.smoothing = unit(self.smoothing, default.smoothing);
        self.sensitivity = unit(self.sensitivity, default.sensitivity);
        self.noise_gate = unit(self.noise_gate, default.noise_gate);
        sanitize_device(&mut self.camera);
        sanitize_device(&mut self.microphone);
    }
}

/// モーションの設定 (`docs/spec/motion.md#トラッキング中のモーションの優先`、T0721・T0722 のビューアと同じ考え方)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MotionSettings {
    /// 何も再生していないときに待機モーションを流す。
    pub auto_idle: bool,
    /// 待機モーションの名前。`None` は名前で選ぶ (`idle`、無ければ `idle` で始まるもの)。モデルに無ければ名前で選ぶ。
    pub idle_motion: Option<String>,
    /// トラッキング中も、再生中のモーション (待機を除く) が動かすパラメータではモーションを優先する。
    pub over_tracking: bool,
    /// モーションを優先しても、トラッキングのままにするパラメータの ID。
    pub tracking_kept: Vec<String>,
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            auto_idle: true,
            idle_motion: None,
            over_tracking: true,
            tracking_kept: Vec::new(),
        }
    }
}

impl MotionSettings {
    /// 重なった ID を除く。
    pub fn sanitize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.tracking_kept.retain(|id| seen.insert(id.clone()));
        if self.idle_motion.as_deref() == Some("") {
            self.idle_motion = None;
        }
    }
}

/// 操作ウィンドウから届くトラッキングの 1 フレーム (`@ugoe/face-tracking` の `TrackingFrame`。キャリブレーションと平滑化の後)。
/// パラメータへの変換は Rust の対応付け (`ugoe_motion::tracking`) が行う。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingFrame {
    pub head: HeadPose,
    /// ARKit のブレンドシェイプの名前 (MediaPipe のカテゴリ名) → 0〜1。知らない名前は無視する。
    #[serde(default)]
    pub blend_shapes: BTreeMap<String, f32>,
    /// 体の値。省くと体の回転は頭 × 0.3。
    #[serde(default)]
    pub body: Option<BodyPose>,
}

/// 頭の回転 (度。`docs/spec/avatar-profile.md#向きと左右の約束`)。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HeadPose {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// 体の値。無い値は `null`。
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BodyPose {
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub roll: Option<f32>,
    pub shoulder_left: Option<f32>,
    pub shoulder_right: Option<f32>,
}

impl TrackingFrame {
    /// `Puppet::set_tracking` に渡す入力。数でない値は 0 (体は無し) にする。
    pub fn to_input(&self) -> TrackingInput {
        let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
        let opt = |v: Option<f32>| v.filter(|v| v.is_finite());
        let body = self.body.unwrap_or_default();
        let mut input = TrackingInput {
            head: Rotation {
                yaw: finite(self.head.yaw),
                pitch: finite(self.head.pitch),
                roll: finite(self.head.roll),
            },
            body: BodyInput {
                yaw: opt(body.yaw),
                pitch: opt(body.pitch),
                roll: opt(body.roll),
                shoulder_left: opt(body.shoulder_left),
                shoulder_right: opt(body.shoulder_right),
            },
            ..TrackingInput::default()
        };
        input.set_blend_shapes(
            self.blend_shapes
                .iter()
                .map(|(name, value)| (name.as_str(), finite(*value))),
        );
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_frames_become_puppet_input() {
        let frame: TrackingFrame = serde_json::from_str(
            r#"{
              "head": { "yaw": 12.5, "pitch": -3, "roll": 1 },
              "blendShapes": { "_neutral": 1, "eyeBlinkLeft": 0.75, "jawOpen": 0.5 },
              "body": { "yaw": null, "pitch": 2, "roll": null, "shoulderLeft": 0.25, "shoulderRight": null }
            }"#,
        )
        .unwrap();
        let input = frame.to_input();
        assert_eq!(input.head.yaw, 12.5);
        assert_eq!(input.blend_shape("eyeBlinkLeft"), Some(0.75));
        assert_eq!(input.blend_shape("jawOpen"), Some(0.5));
        assert_eq!(input.body.pitch, Some(2.0));
        assert_eq!(input.body.yaw, None);
        assert_eq!(input.body.shoulder_left, Some(0.25));
        // 体を省いたフレーム。
        let frame: TrackingFrame = serde_json::from_str(
            r#"{ "head": { "yaw": 0, "pitch": 0, "roll": 0 }, "blendShapes": {} }"#,
        )
        .unwrap();
        assert_eq!(frame.to_input().body, BodyInput::default());
    }

    #[test]
    fn missing_fields_take_the_defaults() {
        let input: InputSettings = serde_json::from_str(r#"{ "sensitivity": 0.8 }"#).unwrap();
        assert_eq!(
            input,
            InputSettings {
                sensitivity: 0.8,
                ..InputSettings::default()
            }
        );
        let motion: MotionSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(motion, MotionSettings::default());
    }

    #[test]
    fn values_are_kept_in_range() {
        let mut input = InputSettings {
            smoothing: 3.0,
            sensitivity: f64::NAN,
            noise_gate: -1.0,
            camera: Some(Device::default()),
            microphone: Some(Device {
                id: "x".repeat(600),
                label: "mic".into(),
            }),
            ..InputSettings::default()
        };
        input.sanitize();
        assert_eq!(
            (input.smoothing, input.sensitivity, input.noise_gate),
            (1.0, 0.5, 0.0)
        );
        assert_eq!(input.camera, None);
        assert_eq!(input.microphone.unwrap().id.len(), MAX_DEVICE_CHARS);

        let mut motion = MotionSettings {
            idle_motion: Some(String::new()),
            tracking_kept: vec!["EyeLOpen".into(), "EyeROpen".into(), "EyeLOpen".into()],
            ..MotionSettings::default()
        };
        motion.sanitize();
        assert_eq!(motion.idle_motion, None);
        assert_eq!(motion.tracking_kept, ["EyeLOpen", "EyeROpen"]);
    }
}
