//! 描画に使うテクスチャを揃える。

use std::path::PathBuf;

use ugoe_format::{TextureData, TextureFormat};
use ugoe_model::Model;

use crate::image::{self, Rgba8};
use crate::{Error, bail};

/// 描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
pub fn texture_count(model: &Model) -> usize {
    model
        .meshes
        .iter()
        .map(|m| m.texture as usize + 1)
        .max()
        .unwrap_or(0)
        .max(model.textures.len())
}

/// テクスチャが無いときに使う 1x1 の不透明な白。
pub fn placeholder_texture() -> Rgba8 {
    Rgba8 {
        width: 1,
        height: 1,
        pixels: vec![255; 4],
    }
}

/// 描画に使うテクスチャ (乗算済み RGBA8) を番号順に揃える。
///
/// 優先順: `overrides` (`--texture`) の指定 → バンドルの PNG → 不透明な白 1x1 (JSON・フィクスチャ用)。
pub fn resolve_textures(
    model: &Model,
    bundle: &[TextureData],
    overrides: &[(u32, PathBuf)],
    warnings: &mut Vec<String>,
) -> Result<Vec<Rgba8>, Error> {
    let count = texture_count(model);
    if let Some((index, _)) = overrides.iter().find(|(i, _)| *i as usize >= count) {
        bail!("--texture {index}: the model has only {count} texture(s)");
    }
    (0..count)
        .map(|i| {
            if let Some((_, path)) = overrides.iter().rev().find(|(j, _)| *j as usize == i) {
                let bytes = std::fs::read(path)
                    .map_err(Error::io(format!("reading {}", path.display())))?;
                return image::decode_png_premultiplied(&bytes).map_err(|e| Error::Texture {
                    context: format!("decoding texture {i} from {}", path.display()),
                    source: Box::new(e),
                });
            }
            match bundle.get(i) {
                Some(data) => match data.format {
                    TextureFormat::Png => {
                        image::decode_png_premultiplied(&data.bytes).map_err(|e| Error::Texture {
                            context: format!("decoding bundle texture {i}"),
                            source: Box::new(e),
                        })
                    }
                    TextureFormat::Ktx2 => bail!(
                        "bundle texture {i} is KTX2, which `ugoe render` cannot decode yet; \
                         pass --texture {i}=<file.png> instead"
                    ),
                },
                None => {
                    if !bundle.is_empty() {
                        warnings.push(format!(
                            "bundle has no data for texture {i}; using an opaque white placeholder"
                        ));
                    }
                    Ok(placeholder_texture())
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_textures_from_meshes_and_model() {
        let mut model = ugoe_model::fixtures::single_quad();
        assert_eq!(texture_count(&model), 1);
        model.textures.clear();
        assert_eq!(texture_count(&model), 1);
        model.meshes[0].texture = 2;
        assert_eq!(texture_count(&model), 3);
    }

    #[test]
    fn resolves_placeholder_and_rejects_out_of_range_overrides() {
        let model = ugoe_model::fixtures::single_quad();
        let mut warnings = Vec::new();
        let textures = resolve_textures(&model, &[], &[], &mut warnings).unwrap();
        assert_eq!(textures, vec![placeholder_texture()]);
        assert!(warnings.is_empty());

        let err = resolve_textures(&model, &[], &[(1, "a.png".into())], &mut warnings)
            .unwrap_err()
            .to_string();
        assert!(err.contains("only 1 texture"), "{err}");
    }

    #[test]
    fn rejects_ktx2_bundle_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: Vec::new(),
        };
        let err = resolve_textures(&model, &[ktx2], &[], &mut Vec::new())
            .unwrap_err()
            .to_string();
        assert!(err.contains("KTX2"), "{err}");
    }

    #[test]
    fn decodes_bundle_png_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let red = Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255],
        };
        let png = TextureData {
            format: TextureFormat::Png,
            bytes: image::encode_png_unpremultiplied(&red).unwrap(),
        };
        let textures = resolve_textures(&model, &[png], &[], &mut Vec::new()).unwrap();
        assert_eq!(textures, vec![red]);
    }
}
