//! `WebPuppet` の受け手の口: 届いたパラメータの値を基準値として書き、物理より後の段だけを進める
//! (`Puppet::set_base_values`・`Puppet::follow`。`docs/spec/motion.md#受け手として進める`、T0133・T0134)。
//!
//! 値は `Float32Array` (パラメータの定義順。`parameter_ids()` と同じ並び) でまとめて渡す。
//! モーションのパーツの不透明度のカーブ (T0135) は、パーツの番号の `Uint32Array` と、値と重みを交互に並べた `Float32Array` で渡す。
//!
//! 指揮役の側の口 (`values_before_physics`・`lip_sync_applied`・`part_opacities_applied_*`。T0454) は、直前の `update` (か `follow`)
//! が受け手へ配る値を、`follow` の引数と同じ形で返す。

use ugoe::{LipSyncInput, PartOpacityInput};
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

    /// 受け手として 1 フレーム進める: `values` を基準値として書き、物理 → リップシンク → パーツの不透明度 (モーションのカーブと
    /// ポーズ) → 評価の順に `dt` 秒進める (`Puppet::follow`。`update` の後半と同じ順)。
    ///
    /// `values` は指揮役の物理の段に入る直前の値、`lip_sync_level` と `lip_sync_strength` は指揮役のリップシンクの音量と強さ
    /// (`Puppet::values_before_physics`・`Puppet::lip_sync_applied`)。`lip_sync_level` を省くとリップシンクを当てない。
    /// `lip_sync_strength` を省くと 1。リップシンクはこの `WebPuppet` のモデルの対象に当て、`set_lip_sync_level` の音量は使わない。
    /// `part_opacity_parts` と `part_opacity_values` は指揮役のモーションのパーツの不透明度のカーブ (`Puppet::part_opacities_applied`)
    /// で、パーツの番号 (モデルの `parts` の順) と、値と重みを交互に並べたもの (番号の 2 倍の数)。省くとカーブなし。
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
        part_opacity_parts: Option<Vec<u32>>,
        part_opacity_values: Option<Vec<f32>>,
    ) -> Result<(), JsError> {
        let puppet = self
            .puppet
            .as_mut()
            .ok_or_else(|| JsError::new("no model is loaded"))?;
        let lip_sync = lip_sync_level.map(|level| LipSyncInput {
            level,
            strength: lip_sync_strength.unwrap_or(1.0),
        });
        let parts = part_opacity_parts.unwrap_or_default();
        let pairs = part_opacity_values.unwrap_or_default();
        if pairs.len() != parts.len().saturating_mul(2) {
            return Err(JsError::new(&format!(
                "expected {} part opacity values and weights, got {}",
                parts.len().saturating_mul(2),
                pairs.len()
            )));
        }
        let part_opacities: Vec<PartOpacityInput> = parts
            .iter()
            .zip(pairs.as_chunks::<2>().0)
            .map(|(&part, &[value, weight])| PartOpacityInput {
                part,
                value,
                weight,
            })
            .collect();
        puppet
            .follow(values, lip_sync, &part_opacities, dt)
            .map_err(js_error)?;
        self.dirty = false;
        Ok(())
    }

    /// 指揮役の口: 直前の `update` (か `follow`) で、物理の段に入る直前のパラメータの値 (`parameter_ids()` の順。
    /// `Puppet::values_before_physics`)。受け手の `follow` の `values` にそのまま渡す。物理の段を止めていても同じ位置の値。
    /// `update` の前は読み込んだときの値。モデルが無ければ空。
    pub fn values_before_physics(&self) -> Vec<f32> {
        self.puppet
            .as_ref()
            .map(|p| p.values_before_physics().to_vec())
            .unwrap_or_default()
    }

    /// 指揮役の口: 直前の `update` (か `follow`) でリップシンクの段が当てた `[音量, 強さ]` (`Puppet::lip_sync_applied`)。
    /// 段を止めていたか音量が無ければ (当てていなければ) `undefined`。受け手の `follow` の `lip_sync_level` と `lip_sync_strength` に
    /// `applied?.[0]`・`applied?.[1]` を渡す。モデルが無ければ `undefined`。
    pub fn lip_sync_applied(&self) -> Option<Vec<f32>> {
        let applied = self.puppet.as_ref()?.lip_sync_applied()?;
        Some(vec![applied.level, applied.strength])
    }

    /// 指揮役の口: 直前の `update` (か `follow`) で、モーションのパーツの不透明度のカーブが値を求めたパーツの番号
    /// (モデルの `parts` の順。カーブのあるパーツだけ。`Puppet::part_opacities_applied`)。受け手の `follow` の `part_opacity_parts`。
    /// 値と重みは同じ順で `part_opacities_applied_values()`。モデルが無ければ空。
    pub fn part_opacities_applied_parts(&self) -> Vec<u32> {
        self.puppet
            .as_ref()
            .map(|p| p.part_opacities_applied().iter().map(|o| o.part).collect())
            .unwrap_or_default()
    }

    /// 指揮役の口: `part_opacities_applied_parts()` の各パーツのカーブの値と重みを交互に並べたもの (番号の 2 倍の数)。
    /// 受け手の `follow` の `part_opacity_values`。モデルが無ければ空。
    pub fn part_opacities_applied_values(&self) -> Vec<f32> {
        self.puppet
            .as_ref()
            .map(|p| {
                p.part_opacities_applied()
                    .iter()
                    .flat_map(|o| [o.value, o.weight])
                    .collect()
            })
            .unwrap_or_default()
    }
}
