//! `WebPuppet` のフェイストラッキングの API (`crate::tracking` を JS に公開する)。
//!
//! 変換は `Puppet::set_tracking` を通して Rust の対応付け (`ugoe_motion::tracking`) で行う。
//! 入力は次の `update(dt)` から毎フレーム、モーションの直後に基準値へ書き込まれる
//! (`docs/spec/motion.md#フレーム内の適用順`)。

use ugoe::motion::{BodyInput, Rotation};
use wasm_bindgen::prelude::*;

use super::WebPuppet;
use crate::tracking;

#[wasm_bindgen]
impl WebPuppet {
    /// フェイストラッキングの入力を設定する。モデルが無ければ何もしない。
    ///
    /// `names` と `scores` は ARKit のブレンドシェイプの名前と値 (0〜1)。MediaPipe Face Landmarker の
    /// カテゴリ名をそのまま渡せる (知らない名前は無視し、渡さなかったものは 0)。`Left` / `Right` は
    /// トラッキングされた人自身の左右。`yaw` / `pitch` / `roll` は頭の回転 (度) で、符号はアバタープロファイルの
    /// 向きの約束に合わせる (`docs/spec/avatar-profile.md#向きと左右の約束`)。平滑化などはホストが行ってから渡す。
    ///
    /// 残りは体のトラッキングの入力で、どれも省略できる (`undefined` / `null`): 体の回転 (度、頭と同じ符号。
    /// 省略した軸は頭の回転 × 0.3) と、左右の肩の上下 (0 が中立、1 ですくめた状態。省略すると書き込まない)。
    #[allow(clippy::too_many_arguments)]
    pub fn set_tracking(
        &mut self,
        names: Vec<String>,
        scores: &[f32],
        yaw: f32,
        pitch: f32,
        roll: f32,
        body_yaw: Option<f32>,
        body_pitch: Option<f32>,
        body_roll: Option<f32>,
        shoulder_left: Option<f32>,
        shoulder_right: Option<f32>,
    ) {
        if let Some(puppet) = &mut self.puppet {
            let head = Rotation { yaw, pitch, roll };
            let body = BodyInput {
                yaw: body_yaw,
                pitch: body_pitch,
                roll: body_roll,
                shoulder_left,
                shoulder_right,
            };
            puppet.set_tracking(Some(tracking::tracking_input(&names, scores, head, body)));
        }
    }

    /// トラッキングの入力を外す。最後に書き込んだ値はパラメータに残る (既定値に戻すのはホスト)。
    pub fn clear_tracking(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.set_tracking(None);
        }
    }

    /// トラッキングの入力が設定されているか。
    pub fn is_tracking(&self) -> bool {
        self.puppet.as_ref().is_some_and(|p| p.tracking().is_some())
    }

    /// トラッキングが書き込むパラメータの ID (モデルの定義順。`avatar.tracking` の上書きを反映する)。
    pub fn tracking_parameter_ids(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(tracking::tracking_targets)
            .unwrap_or_default()
    }
}
