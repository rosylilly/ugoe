//! `WebPuppet` の受け手の口: 届いたパラメータの値を基準値として書き、物理だけを進める
//! (`Puppet::set_base_values`・`Puppet::follow`。`docs/spec/motion.md#受け手として進める`、T0133)。
//!
//! 値は `Float32Array` (パラメータの定義順。`parameter_ids()` と同じ並び) でまとめて渡す。

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

    /// 受け手として 1 フレーム進める: `values` を基準値として書き、物理とポーズの段だけを `dt` 秒進めて評価する。
    ///
    /// モーション・トラッキング・表情・目パチ・呼吸・リップシンクは通さない (値を作った指揮役で済んでいるため)。
    /// 同じ `values` と `dt` の列を与えれば、物理を含めて同じ形になる (ネイティブの `Puppet::follow` とも許容誤差内で同じ)。
    /// 描くのは最後のフレームだけでよい。モデルが無い・数が違う・NaN を含むときは、何も変えずに例外を投げる。
    pub fn follow(&mut self, values: &[f32], dt: f32) -> Result<(), JsError> {
        let puppet = self
            .puppet
            .as_mut()
            .ok_or_else(|| JsError::new("no model is loaded"))?;
        puppet.follow(values, dt).map_err(js_error)?;
        self.dirty = false;
        Ok(())
    }
}
