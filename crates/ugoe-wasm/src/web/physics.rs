//! `WebPuppet` のエディタ向けの物理の表示の問い合わせ (T0629)。処理の本体は [`crate::physics_view`]。

use ugoe::core::Vec2;
use wasm_bindgen::prelude::*;

use super::WebPuppet;
use crate::physics_view;

#[wasm_bindgen]
impl WebPuppet {
    /// デフォーマ `id` の局所座標の点 `(x, y)` の、今のパラメータ値でのキャンバス座標 `[x, y]`。
    /// デフォーマがなければ `undefined`。
    ///
    /// @internal エディタ向け。予告なく変わる (公開の API の外。crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」)。
    pub fn deformer_point(&mut self, id: &str, x: f32, y: f32) -> Option<Vec<f32>> {
        self.evaluate();
        physics_view::deformer_point(self.puppet.as_ref()?.instance(), id, Vec2::new(x, y))
    }

    /// デフォーマ `id` のワールド変換の、局所座標の点 `(x, y)` でのヤコビ行列 `[a, b, c, d]`
    /// (列優先)。デフォーマがなければ `undefined`。
    ///
    /// @internal エディタ向け。予告なく変わる (公開の API の外。crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」)。
    pub fn deformer_jacobian(&mut self, id: &str, x: f32, y: f32) -> Option<Vec<f32>> {
        self.evaluate();
        physics_view::deformer_jacobian(self.puppet.as_ref()?.instance(), id, Vec2::new(x, y))
    }

    /// リグ `rig` (`rigs` の添字) の粒子の今の位置 `[x0, y0, x1, y1, ...]` (物理空間。動的物理では
    /// キャンバス座標)。最後の `update` の後の位置で、`reset_physics` の後や `update` の前は初期位置。
    /// リグや物理が無ければ空。
    ///
    /// @internal エディタ向け。予告なく変わる (公開の API の外。crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」)。
    pub fn physics_particles(&self, rig: u32) -> Vec<f32> {
        physics_view::particles(self.puppet.as_ref(), rig)
    }
}
