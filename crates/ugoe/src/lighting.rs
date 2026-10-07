//! ホストがライティングを使うための補助 (`docs/spec/rendering.md#ホストでのライティング`)。
//!
//! 光源 ([`Lighting`]・[`Light`]・[`MAX_LIGHTS`]) はホストの状態で、参照レンダラなら `Renderer::set_lighting` に渡す。
//! 光源の型は `ugoe-core` の再エクスポートで、`render` feature を外しても使える (T0166)。
//! `ugoe::render` にも同じ型がある。
//! 法線マップは [`bundle_normal_maps`] で引いてデコードし、`Renderer::set_normal_map` で登録する
//! (PNG は **アルファを掛けずに** デコードする。デコードはホストの仕事)。

use alloc::vec::Vec;

pub use ugoe_core::lighting::{Light, Lighting, MAX_LIGHTS};
use ugoe_format::{Bundle, TextureData};
use ugoe_model::Model;

use crate::Puppet;

/// モデルにライティングを受けるメッシュがあるか (`Model::lighting` と `Mesh::lit` の両方が `true`)。
/// ホストが光源の操作を出すかを決めるために使う。
pub fn uses_lighting(model: &Model) -> bool {
    model.lighting && model.meshes.iter().any(|m| m.lit)
}

/// バンドルの法線マップを、テクスチャの番号ごとに並べる (添字 = テクスチャの番号、長さ = `model.textures` の数)。
/// 法線マップの無い番号は `None`。
pub fn bundle_normal_maps(bundle: &Bundle) -> Vec<Option<&TextureData>> {
    (0..bundle.model.textures.len())
        .map(|i| bundle.normal_map(i))
        .collect()
}

impl Puppet {
    /// このモデルにライティングを受けるメッシュがあるか ([`uses_lighting`])。
    pub fn uses_lighting(&self) -> bool {
        uses_lighting(self.instance().compiled().model())
    }
}

#[cfg(test)]
mod tests {
    use ugoe_format::TextureFormat;
    use ugoe_model::fixtures;

    use super::*;

    #[test]
    fn lighting_needs_both_the_model_and_a_mesh() {
        let mut model = fixtures::single_quad();
        model.meshes[0].lit = true;
        assert!(!uses_lighting(&model));
        model.lighting = true;
        assert!(uses_lighting(&model));
        assert!(Puppet::new(model.clone()).unwrap().uses_lighting());
        model.meshes[0].lit = false;
        assert!(!uses_lighting(&model));
    }

    #[test]
    fn normal_maps_are_indexed_by_texture() {
        let mut model = fixtures::single_quad();
        let mut second = model.textures[0].clone();
        second.normal_map = Some("n".into());
        model.textures.push(second);
        let mut bundle = Bundle::new(model);
        let normal = TextureData {
            format: TextureFormat::Png,
            bytes: vec![1, 2, 3],
        };
        bundle.normal_maps = vec![normal.clone()];
        assert_eq!(bundle_normal_maps(&bundle), vec![None, Some(&normal)]);
    }

    /// `ugoe::lighting` と `ugoe::render` の光源は同じ型 (再エクスポート)。
    #[cfg(feature = "render")]
    #[test]
    fn render_reexports_the_same_types() {
        let lighting: crate::render::Lighting = Lighting::default();
        let _: Light = crate::render::Light::Directional {
            direction: [0.0, 0.0, -1.0],
            color: lighting.ambient,
        };
        assert_eq!(crate::render::MAX_LIGHTS, MAX_LIGHTS);
    }
}
