//! `WebPuppet` の受け手の口: 届いたパラメータの値を基準値として書き、物理より後の段だけを進める
//! (`Puppet::set_base_values`・`Puppet::follow`。`docs/spec/motion.md#受け手として進める`、T0133・T0134)。
//!
//! 値は `Float32Array` (パラメータの定義順。`parameter_ids()` と同じ並び) でまとめて渡す。

use ugoe::LipSyncInput;
use wasm_bindgen::prelude::*;

use super::{WebPuppet, js_error};

#[wasm_bindgen]
impl WebPuppet {
    /// すべてのパラメータの現在値 (`parameter_ids()` の順)。モデルが無ければ空。
    pub fn parameter_values(&self) -> Vec<f32> {
        self.puppet
            .as_ref()
            .map(|p| p.instance().parameters().as_slice().to_vec())
            .unwrap_or_default()
    }

    /// パラメータの値 (`parameter_ids()` の順) をまとめて **基準値として** 書く (値域にクランプする)。
    ///
    /// `set_parameter` の値は前のフレームの出力と同じなら基準値にならないが、これは同じでも基準値にする。
    /// 評価は次の `render` や問い合わせまで遅らせる。モデルが無い・数が違う・NaN を含むときは、何も変えずに例外を投げる。
    pub fn set_base_values(&mut self, values: &[f32]) -> Result<(), JsError> {
        let puppet = self
            .puppet
            .as_mut()
            .ok_or_else(|| JsError::new("no model is loaded"))?;
        puppet.set_base_values(values).map_err(js_error)?;
        self.dirty = true;
        Ok(())
    }

    /// 受け手として 1 フレーム進める: `values` を基準値として書き、物理 → リップシンク → ポーズ → 評価の順に `dt` 秒進める
    /// (`Puppet::follow`。`update` の後半と同じ順)。
    ///
    /// `values` は指揮役の物理の段に入る直前の値、`lip_sync_level` と `lip_sync_strength` は指揮役のリップシンクの音量と強さ
    /// (`Puppet::values_before_physics`・`Puppet::lip_sync_applied`)。`lip_sync_level` を省くとリップシンクを当てない。
    /// `lip_sync_strength` を省くと 1。リップシンクはこの `WebPuppet` のモデルの対象に当て、`set_lip_sync_level` の音量は使わない。
    ///
    /// モーション・トラッキング・表情・目パチ・呼吸は通さない (値を作った指揮役で済んでいるため)。
    /// 同じ列を与えれば、物理を含めて同じ形になる (ネイティブの `Puppet::follow` とも許容誤差内で同じ)。
    /// 描くのは最後のフレームだけでよい。モデルが無い・数が違う・NaN を含むときは、何も変えずに例外を投げる。
    pub fn follow(
        &mut self,
        values: &[f32],
        dt: f32,
        lip_sync_level: Option<f32>,
        lip_sync_strength: Option<f32>,
    ) -> Result<(), JsError> {
        let puppet = self
            .puppet
            .as_mut()
            .ok_or_else(|| JsError::new("no model is loaded"))?;
        let lip_sync = lip_sync_level.map(|level| LipSyncInput {
            level,
            strength: lip_sync_strength.unwrap_or(1.0),
        });
        puppet.follow(values, lip_sync, dt).map_err(js_error)?;
        self.dirty = false;
        Ok(())
    }
}
