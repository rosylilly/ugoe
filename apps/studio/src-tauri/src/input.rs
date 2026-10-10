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
    /// トラッキングの調整 (首振りの感度と遊び・瞼と口の開きの境目。T0877)。
    pub tuning: TrackingTuning,
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
            tuning: TrackingTuning::default(),
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
        self.tuning.sanitize();
        sanitize_device(&mut self.camera);
        sanitize_device(&mut self.microphone);
    }
}

/// 頭の回転の 1 つの軸の調整 (`@ugoe/face-tracking` の `AxisTuning`。packages/face-tracking/src/tuning.ts)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AxisTuning {
    /// 感度 (倍率。0〜4)。遊びを引いた後の角度に掛ける。
    pub gain: f64,
    /// 遊び (度。0〜20)。中立のまわりのこの角度の中は 0 にする。
    pub dead_zone: f64,
}

impl Default for AxisTuning {
    fn default() -> Self {
        Self {
            gain: 1.0,
            dead_zone: 0.0,
        }
    }
}

/// ブレンドシェイプの値の境目 (`@ugoe/face-tracking` の `RangeTuning`)。`low` 以下を 0、`high` 以上を 1 にする。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RangeTuning {
    pub low: f64,
    pub high: f64,
}

impl Default for RangeTuning {
    fn default() -> Self {
        Self {
            low: 0.0,
            high: 1.0,
        }
    }
}

/// トラッキングの調整 (`@ugoe/face-tracking` の `TrackingTuning`)。値は画面 (トラッカー) が使い、ここは保存と収め直しだけをする。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackingTuning {
    pub yaw: AxisTuning,
    pub pitch: AxisTuning,
    pub roll: AxisTuning,
    /// 瞼 (`eyeBlinkLeft`・`eyeBlinkRight` を一緒に)。
    pub eye_blink: RangeTuning,
    /// 口の開き (`jawOpen`)。
    pub jaw_open: RangeTuning,
}

/// 感度の上限 (tuning.ts の `GAIN_MAX`)。
const GAIN_MAX: f64 = 4.0;
/// 遊びの上限 (度。tuning.ts の `DEAD_ZONE_MAX`)。
const DEAD_ZONE_MAX: f64 = 20.0;
/// 境目の間の最小の幅 (tuning.ts の `RANGE_MIN_SPAN`)。
const RANGE_MIN_SPAN: f64 = 0.05;

/// 数なら範囲に寄せ、数でなければ `default`。
fn within(v: f64, min: f64, max: f64, default: f64) -> f64 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

impl AxisTuning {
    fn sanitize(&mut self) {
        let default = Self::default();
        self.gain = within(self.gain, 0.0, GAIN_MAX, default.gain);
        self.dead_zone = within(self.dead_zone, 0.0, DEAD_ZONE_MAX, default.dead_zone);
    }
}

impl RangeTuning {
    /// 0〜1 に収める。間が最小の幅より狭い (下が上以上を含む) なら既定に戻す。
    fn sanitize(&mut self) {
        let default = Self::default();
        self.low = within(self.low, 0.0, 1.0, default.low);
        self.high = within(self.high, 0.0, 1.0, default.high);
        // tuning.ts の `sanitizeRange` と同じく、浮動小数点の誤差の分だけ緩める。
        if self.high - self.low < RANGE_MIN_SPAN - 1e-9 {
            *self = default;
        }
    }
}

impl TrackingTuning {
    /// 値を範囲に収める (数でない値・逆さまの境目は既定に戻す)。
    pub fn sanitize(&mut self) {
        for axis in [&mut self.yaw, &mut self.pitch, &mut self.roll] {
            axis.sanitize();
        }
        for range in [&mut self.eye_blink, &mut self.jaw_open] {
            range.sanitize();
        }
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
    fn tuning_is_read_with_defaults_and_kept_in_range() {
        // 無い項目・知らない項目は既定値。
        let input: InputSettings = serde_json::from_str(
            r#"{ "tuning": { "yaw": { "gain": 2 }, "eyeBlink": { "low": 0.1, "high": 0.6 }, "brow": {} } }"#,
        )
        .unwrap();
        assert_eq!(
            input.tuning,
            TrackingTuning {
                yaw: AxisTuning {
                    gain: 2.0,
                    dead_zone: 0.0
                },
                eye_blink: RangeTuning {
                    low: 0.1,
                    high: 0.6
                },
                ..TrackingTuning::default()
            }
        );
        let mut input = InputSettings {
            tuning: TrackingTuning {
                yaw: AxisTuning {
                    gain: f64::NAN,
                    dead_zone: f64::INFINITY,
                },
                pitch: AxisTuning {
                    gain: 10.0,
                    dead_zone: -1.0,
                },
                eye_blink: RangeTuning {
                    low: 0.7,
                    high: 0.3,
                },
                jaw_open: RangeTuning {
                    low: -1.0,
                    high: 0.5,
                },
                ..TrackingTuning::default()
            },
            ..InputSettings::default()
        };
        input.sanitize();
        assert_eq!(input.tuning.yaw, AxisTuning::default());
        assert_eq!(
            input.tuning.pitch,
            AxisTuning {
                gain: GAIN_MAX,
                dead_zone: 0.0
            }
        );
        assert_eq!(input.tuning.eye_blink, RangeTuning::default());
        assert_eq!(
            input.tuning.jaw_open,
            RangeTuning {
                low: 0.0,
                high: 0.5
            }
        );
        // 最小の幅ちょうどの組は残す (画面で決めた値)。
        let mut range = RangeTuning {
            low: 0.3,
            high: 0.35,
        };
        range.sanitize();
        assert_eq!(range.low, 0.3);
    }

    #[test]
    fn tuning_is_written_in_the_frontend_shape() {
        let json = serde_json::to_value(TrackingTuning::default()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "yaw": { "gain": 1.0, "deadZone": 0.0 },
                "pitch": { "gain": 1.0, "deadZone": 0.0 },
                "roll": { "gain": 1.0, "deadZone": 0.0 },
                "eyeBlink": { "low": 0.0, "high": 1.0 },
                "jawOpen": { "low": 0.0, "high": 1.0 },
            })
        );
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
