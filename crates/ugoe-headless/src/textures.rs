//! 描画に使うテクスチャを揃える。

use std::path::PathBuf;

use ugoe_format::image::ImageError;
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
/// 優先順: `overrides` (`--texture`) の指定 → バンドルの画像 → 不透明な白 1x1 (JSON・フィクスチャ用)。
///
/// 画像は PNG と、圧縮していない 8bit の RGB / RGBA・ブロック圧縮の BC1〜BC5・BC7 の KTX2 を読む (`ugoe_format::image`。
/// 形式は先頭のシグネチャで決める)。読めない形式の KTX2 (ETC2・ASTC・Basis Universal など) はエラーにする
/// (`--texture` で PNG を渡せば描ける)。
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
                let bytes = crate::read_file(path)?;
                // シグネチャで分からなければ PNG として読む (PNG のエラーを出す)。
                let format = TextureFormat::detect(&bytes).unwrap_or(TextureFormat::Png);
                return image::decode_image_premultiplied(format, &bytes).map_err(|e| {
                    Error::Texture {
                        context: format!("decoding texture {i} from {}", path.display()),
                        source: Box::new(e),
                    }
                });
            }
            match bundle.get(i) {
                Some(data) => {
                    image::decode_image_premultiplied(data.format, &data.bytes).map_err(|e| {
                        let context = match &e {
                            Error::ImageDecode(ImageError::UnsupportedKtx2(_)) => format!(
                                "bundle texture {i} is KTX2 in a format that `ugoe render` cannot \
                                 decode; pass --texture {i}=<file.png> instead"
                            ),
                            _ => format!("decoding bundle texture {i}"),
                        };
                        Error::Texture {
                            context,
                            source: Box::new(e),
                        }
                    })
                }
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

/// バンドルの法線マップ (添字 = テクスチャの番号) をデコードする (`docs/spec/rendering.md#ライティング`)。
///
/// 法線マップはアルファを掛けずに読む。読めない形式の KTX2 (ETC2・ASTC・Basis Universal など) は、警告を出して法線マップなし
/// (法線 `(0, 0, 1)`) として描く。`--texture` でテクスチャを差し替えても、法線マップはバンドルのものを使う。
pub fn resolve_normal_maps(
    normal_maps: &[Option<TextureData>],
    warnings: &mut Vec<String>,
) -> Result<Vec<Option<Rgba8>>, Error> {
    normal_maps
        .iter()
        .enumerate()
        .map(|(i, data)| match data {
            None => Ok(None),
            Some(data) => match image::decode_image(data.format, &data.bytes) {
                Ok(map) => Ok(Some(map)),
                Err(Error::ImageDecode(ImageError::UnsupportedKtx2(reason))) => {
                    warnings.push(format!(
                        "the normal map of texture {i} is KTX2 in a format that cannot be decoded \
                         ({reason}); drawing without it"
                    ));
                    Ok(None)
                }
                Err(e) => Err(Error::Texture {
                    context: format!("decoding the normal map of bundle texture {i}"),
                    source: Box::new(e),
                }),
            },
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

    /// `--texture` のパスも通常のファイルだけを読む (`/dev/zero` を読み続けないように)。
    #[cfg(unix)]
    #[test]
    fn refuses_texture_overrides_that_are_not_regular_files() {
        let model = ugoe_model::fixtures::single_quad();
        let err =
            resolve_textures(&model, &[], &[(0, "/dev/null".into())], &mut Vec::new()).unwrap_err();
        let err = format!("{err}: {}", std::error::Error::source(&err).unwrap());
        assert!(err.contains("not a regular file"), "{err}");
    }

    /// 圧縮していない KTX2 (`R8G8B8A8_SRGB` / `_UNORM`) を値のまま読み、乗算する。
    #[test]
    fn decodes_bundle_ktx2_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let straight = Rgba8 {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 255, 255, 255, 255, 128],
        };
        for srgb in [false, true] {
            let ktx2 = TextureData {
                format: TextureFormat::Ktx2,
                bytes: image::encode_ktx2(
                    std::slice::from_ref(&straight),
                    image::Ktx2Options {
                        srgb,
                        ..Default::default()
                    },
                )
                .unwrap(),
            };
            let textures = resolve_textures(&model, &[ktx2], &[], &mut Vec::new()).unwrap();
            assert_eq!(textures[0].pixels, vec![255, 0, 0, 255, 128, 128, 128, 128]);
        }
    }

    /// ブロック圧縮 (BC3) の KTX2 も展開してから乗算する (`docs/adr/0030-compressed-ktx2-decoding.md`)。
    #[test]
    fn decodes_bundle_bc_ktx2_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let straight = Rgba8 {
            width: 4,
            height: 4,
            pixels: [255, 255, 255, 128].repeat(16),
        };
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: image::encode_ktx2(
                std::slice::from_ref(&straight),
                image::Ktx2Options {
                    compression: image::Ktx2Compression::Bc3,
                    ..Default::default()
                },
            )
            .unwrap(),
        };
        let textures = resolve_textures(&model, &[ktx2], &[], &mut Vec::new()).unwrap();
        assert_eq!(textures[0].pixels, [128, 128, 128, 128].repeat(16));
    }

    /// `--texture` に KTX2 のファイルも渡せる (形式はシグネチャで決める)。
    #[test]
    fn texture_overrides_may_be_ktx2() {
        let model = ugoe_model::fixtures::single_quad();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.ktx2");
        let red = Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255],
        };
        std::fs::write(
            &path,
            image::encode_ktx2(std::slice::from_ref(&red), image::Ktx2Options::default()).unwrap(),
        )
        .unwrap();
        let textures = resolve_textures(&model, &[], &[(0, path)], &mut Vec::new()).unwrap();
        assert_eq!(textures, vec![red]);
    }

    /// 読めない形式の KTX2 (ETC2 など) と壊れた KTX2 はエラー。
    #[test]
    fn rejects_unsupported_and_broken_ktx2_bundle_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let error = |bytes: Vec<u8>| {
            let ktx2 = TextureData {
                format: TextureFormat::Ktx2,
                bytes,
            };
            let err = resolve_textures(&model, &[ktx2], &[], &mut Vec::new()).unwrap_err();
            format!("{err}: {}", std::error::Error::source(&err).unwrap())
        };
        let err = error(etc2_ktx2());
        assert!(err.contains("cannot decode; pass --texture 0="), "{err}");
        assert!(err.contains("vkFormat 151"), "{err}");
        let err = error(ugoe_format::image::KTX2_IDENTIFIER.to_vec());
        assert!(
            err.contains("decoding bundle texture 0: decoding KTX2"),
            "{err}"
        );
    }

    /// `VK_FORMAT_ETC2_R8G8B8A8_UNORM_BLOCK` (151) の見出しを持つ KTX2 (読めない形式)。
    fn etc2_ktx2() -> Vec<u8> {
        let mut bytes = image::encode_ktx2(
            &[Rgba8 {
                width: 4,
                height: 4,
                pixels: vec![0; 64],
            }],
            image::Ktx2Options::default(),
        )
        .unwrap();
        bytes[12..16].copy_from_slice(&151u32.to_le_bytes());
        bytes
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

    #[test]
    fn decodes_normal_maps_without_premultiplying() {
        let normal = Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![128, 128, 255, 255],
        };
        let png = TextureData {
            format: TextureFormat::Png,
            bytes: image::encode_png_unpremultiplied(&normal).unwrap(),
        };
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: image::encode_ktx2(std::slice::from_ref(&normal), image::Ktx2Options::default())
                .unwrap(),
        };
        let etc2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: etc2_ktx2(),
        };
        let mut warnings = Vec::new();
        let maps =
            resolve_normal_maps(&[None, Some(png), Some(ktx2), Some(etc2)], &mut warnings).unwrap();
        assert_eq!(maps, vec![None, Some(normal.clone()), Some(normal), None]);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("normal map of texture 3 is KTX2"),
            "{}",
            warnings[0]
        );
        // 壊れた KTX2 は (読めない形式と違って) エラー。
        let broken = TextureData {
            format: TextureFormat::Ktx2,
            bytes: ugoe_format::image::KTX2_IDENTIFIER.to_vec(),
        };
        assert!(resolve_normal_maps(&[Some(broken)], &mut Vec::new()).is_err());
    }
}
