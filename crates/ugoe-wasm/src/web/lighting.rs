//! `WebPuppet` のライティングの API (`docs/spec/rendering.md#ライティング`)。
//!
//! 法線マップは `load_bundle` がバンドルから (着せ替えのモジュールの分は付け外しで) 登録する。
//! 光源はホストがここで渡す (JSON の形は [`crate::lighting`])。
//! ライティングを受けるのは、`Model::lighting` と `Mesh::lit` の両方が `true` のメッシュだけ。

use ugoe::render::Lighting;
use wasm_bindgen::prelude::*;

use super::WebPuppet;
use crate::lighting;

#[wasm_bindgen]
impl WebPuppet {
    /// 光源を設定する。次の `render` から使う。`undefined` で既定 (環境光が白で光源なし。
    /// ライティングを受けるメッシュも受けない場合と同じ色) に戻す。モデルを読み直しても保つ。
    ///
    /// 形は `{"ambient": [r, g, b], "lights": [{"type": "directional", "direction": [x, y, z], "color": [r, g, b]},
    /// {"type": "point", "position": [x, y, z], "color": [r, g, b], "range": 800}]}`。座標はキャンバス座標
    /// (ピクセル・Y 下向き) + 奥行き (手前が +z)。`direction` は光が進む向き。色は省略すると白、`range` は省略すると減衰しない。
    pub fn set_lighting_json(&mut self, json: Option<String>) -> Result<(), JsError> {
        let lighting = match json {
            Some(json) => lighting::lighting_from_json(&json).map_err(|e| JsError::new(&e))?,
            None => Lighting::default(),
        };
        self.renderer.set_lighting(&lighting);
        Ok(())
    }

    /// 読み込んだモデルにライティングを受けるメッシュがあるか (`Model::lighting` と `Mesh::lit` の両方が `true`)。
    /// ビューアが光源の操作を出すかを決めるために使う。
    pub fn uses_lighting(&self) -> bool {
        self.puppet.as_ref().is_some_and(|p| p.uses_lighting())
    }
}
