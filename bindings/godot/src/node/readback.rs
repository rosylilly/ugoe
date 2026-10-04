//! `UgoePuppet2D` の、モデルの JSON の読み込みと評価の結果の読み戻し (T0420)。
//!
//! 配布物の適合テスト (`tests/conformance/README.md`) で、Godot の中の評価の結果を C ABI・wasm と同じ値で比べるためのもの。
//! 値は直前の評価 (`advance` か毎フレームの更新) の結果で、`docs/spec/evaluation.md#8-出力` の出力と同じ。
//! ノードの既存の機能から分けて、別の `#[godot_api]` のブロックに置く。

use godot::prelude::*;
use ugoe::format::Bundle;

use super::UgoePuppet2D;
use crate::model::{LoadError, LoadedModel};

#[godot_api(secondary)]
impl UgoePuppet2D {
    /// モデルの JSON (`docs/spec/file-formats.md`) の文字列から読み込む。成功したら true。
    ///
    /// テクスチャは無いので、すべて不透明な白で描く (`ugoe render` でモデルの JSON を描くときと同じ)。
    /// 物理・モーション・表情も無い。`model` は null になる (シーンには保存されない)。
    #[func]
    pub fn load_model_from_json(&mut self, json: GString) -> bool {
        self.model = None;
        match load_json(&json.to_string()) {
            Ok(loaded) => {
                self.install(loaded);
                true
            }
            Err(e) => {
                self.fail_load(&e, "(model JSON)");
                false
            }
        }
    }

    /// 描画可能要素 (メッシュ) の ID をモデルの順に並べる。`get_drawable_*` の番号はこの順。
    #[func]
    pub fn get_drawable_ids(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.model()
                    .meshes
                    .iter()
                    .map(|m| GString::from(m.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `index` 番の描画可能要素の頂点位置 (**キャンバス座標**。ノードの座標にするには `get_canvas_origin()` を引く)。
    /// 番号が範囲の外なら空。
    #[func]
    pub fn get_drawable_positions(&self, index: i32) -> PackedVector2Array {
        self.drawable_state(index)
            .map(|d| d.positions.iter().map(|p| Vector2::new(p.x, p.y)).collect())
            .unwrap_or_default()
    }

    /// `index` 番の描画可能要素の評価の結果: `{ "opacity", "draw_order", "multiply_color" (Color), "screen_color" (Color),
    /// "mask_strength", "visible" }`。番号が範囲の外なら空。
    #[func]
    pub fn get_drawable_state(&self, index: i32) -> VarDictionary {
        let mut dict = VarDictionary::new();
        if let Some(d) = self.drawable_state(index) {
            let color = |c: ugoe::core::Rgb| Color::from_rgb(c.r, c.g, c.b);
            dict.set("opacity", d.opacity);
            dict.set("draw_order", d.draw_order);
            dict.set("multiply_color", color(d.multiply_color));
            dict.set("screen_color", color(d.screen_color));
            dict.set("mask_strength", d.mask_strength);
            dict.set("visible", d.visible);
        }
        dict
    }

    /// 描く順番 (奥から手前へ並べた描画可能要素の番号)。
    #[func]
    pub fn get_render_order(&self) -> PackedInt32Array {
        self.loaded
            .as_ref()
            .map(|l| {
                l.puppet
                    .instance()
                    .render_order()
                    .iter()
                    .map(|&i| i as i32)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl UgoePuppet2D {
    fn drawable_state(&self, index: i32) -> Option<&ugoe::runtime::DrawableState> {
        let index = usize::try_from(index).ok()?;
        self.loaded
            .as_ref()?
            .puppet
            .instance()
            .drawables()
            .get(index)
    }
}

/// モデルの JSON を、テクスチャの無いバンドルとして読み込む。
fn load_json(json: &str) -> Result<LoadedModel, LoadError> {
    let model = ugoe::format::model_from_json(json)?;
    let mut loaded = LoadedModel::from_bundle(Bundle::new(model))?;
    // テクスチャが無いのは前提なので、画像の無いテクスチャの警告は出さない。
    loaded.warnings.clear();
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe::model::fixtures;

    #[test]
    fn loads_a_model_json_without_textures() {
        let json = ugoe::format::model_to_json(&fixtures::single_quad()).expect("json");
        let mut loaded = load_json(&json).expect("load");
        assert_eq!(loaded.drawables.len(), 1);
        assert!(loaded.texture(0).is_none());
        assert!(loaded.warnings.is_empty());
        loaded.puppet.update(0.0);
        assert_eq!(loaded.puppet.instance().render_order(), [0]);
    }

    #[test]
    fn rejects_broken_json() {
        assert!(load_json("{").is_err());
    }
}
