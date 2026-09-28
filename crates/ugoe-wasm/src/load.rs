//! モデルの読み込み (JSON / `.ugoe` バンドル) とテクスチャのデコード。GPU には触れない。
//!
//! テクスチャは Rust 側で PNG をデコードし、乗算済みアルファの RGBA8 にする
//! (`docs/spec/rendering.md` の「前提」: 色の値はリニア化しない)。
//! ブラウザのデコーダは色空間の変換やアルファの扱いが実装ごとに異なるので使わない。

use std::io::Cursor;

use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::Model;
use ugoe::model::physics::PhysicsSettings;
use ugoe::runtime::ModelInstance;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error("model does not compile: {0}")]
    Compile(#[from] ugoe::runtime::CompileError),
    #[error("decoding texture {index}: {message}")]
    Texture { index: usize, message: String },
}

/// 乗算済みアルファの RGBA8 画像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba8 {
    /// テクスチャが無いときに使う 1x1 の不透明な白 (`ugoe render` と同じ)。
    pub fn placeholder() -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        }
    }
}

/// 読み込んだモデル一式。
#[derive(Debug)]
pub struct LoadedModel {
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    /// 描画に使うテクスチャ (添字 = テクスチャ番号)。足りない分はプレースホルダ。
    pub textures: Vec<Rgba8>,
    /// 読み込めたが注意が要ること (プレースホルダで代用したテクスチャなど)。
    pub warnings: Vec<String>,
}

/// JSON のモデルを読む。テクスチャは持たないので、すべて白のプレースホルダにする。
pub fn from_json(json: &str) -> Result<LoadedModel, LoadError> {
    let model = ugoe::format::model_from_json(json)?;
    let textures = vec![Rgba8::placeholder(); texture_count(&model)];
    Ok(LoadedModel {
        model,
        physics: None,
        textures,
        warnings: Vec::new(),
    })
}

/// `.ugoe` バンドルを読み、PNG のテクスチャをデコードする。
pub fn from_bundle(bytes: &[u8]) -> Result<LoadedModel, LoadError> {
    let Bundle {
        model,
        physics,
        textures: data,
        ..
    } = ugoe::format::read_bundle(bytes)?;
    let mut warnings = Vec::new();
    let textures = (0..texture_count(&model))
        .map(|index| match data.get(index) {
            Some(TextureData {
                format: TextureFormat::Png,
                bytes,
            }) => decode_png_premultiplied(bytes)
                .map_err(|message| LoadError::Texture { index, message }),
            Some(TextureData {
                format: TextureFormat::Ktx2,
                ..
            }) => {
                // KTX2 のデコードは未対応 (`ugoe render` と同じ)。表示は止めずに白で代用する。
                warnings.push(format!(
                    "texture {index} is KTX2, which is not supported yet; using a white placeholder"
                ));
                Ok(Rgba8::placeholder())
            }
            // `read_bundle` がテクスチャの数を検証するので、通常は起きない。
            None => {
                warnings.push(format!(
                    "bundle has no data for texture {index}; using a white placeholder"
                ));
                Ok(Rgba8::placeholder())
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(LoadedModel {
        model,
        physics,
        textures,
        warnings,
    })
}

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

/// `from` のパラメータ値のうち、同じ ID が `to` にもあるものを `to` へ写す (値域でクランプされる)。
///
/// エディタでモデルを読み直したとき (編集・Undo のたび) に、スライダーで決めた表示の状態を保つため。
pub fn carry_over_parameters(from: &ModelInstance, to: &mut ModelInstance) {
    let values = from.parameters();
    let ids = from
        .compiled()
        .model()
        .parameters
        .iter()
        .map(|p| p.id.as_str());
    for (id, &value) in ids.zip(values.as_slice()) {
        let params = to.parameters_mut();
        if let Some(index) = params.index_of(id) {
            params.set(index, value);
        }
    }
    to.update();
}

/// PNG をデコードして、乗算済みアルファの RGBA8 にする。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。
pub fn decode_png_premultiplied(bytes: &[u8]) -> Result<Rgba8, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "PNG image is too large".to_owned())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let pixel_count = info.width as usize * info.height as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    match info.color_type {
        png::ColorType::Rgba => pixels = buf,
        png::ColorType::Rgb => {
            for p in buf.as_chunks::<3>().0 {
                pixels.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for p in buf.as_chunks::<2>().0 {
                pixels.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in &buf {
                pixels.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".to_owned()),
    }
    for p in pixels.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
    Ok(Rgba8 {
        width: info.width,
        height: info.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use ugoe::format::write_bundle;
    use ugoe::model::fixtures;
    use ugoe::runtime::CompiledModel;

    use super::*;

    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(rgba).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn json_models_get_white_placeholders() {
        let json = ugoe::format::model_to_json(&fixtures::single_quad()).unwrap();
        let loaded = from_json(&json).unwrap();
        assert_eq!(loaded.textures, vec![Rgba8::placeholder()]);
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn bundle_png_is_premultiplied() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(2, 1, &[255, 0, 0, 255, 255, 255, 255, 128]),
        }];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(loaded.textures.len(), 1);
        let t = &loaded.textures[0];
        assert_eq!((t.width, t.height), (2, 1));
        assert_eq!(t.pixels, vec![255, 0, 0, 255, 128, 128, 128, 128]);
    }

    #[test]
    fn broken_png_is_an_error() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0],
        }];
        let bytes = write_bundle(&bundle).unwrap();
        assert!(matches!(
            from_bundle(&bytes),
            Err(LoadError::Texture { index: 0, .. })
        ));
    }

    #[test]
    fn carries_over_parameters_by_id() {
        let compiled = CompiledModel::compile(fixtures::single_quad()).unwrap();
        let mut old = ModelInstance::new(compiled);
        let x = old.parameters().index_of("X").unwrap();
        old.parameters_mut().set(x, 0.5);

        let mut model = fixtures::single_quad();
        model.parameters[0].max = 0.25;
        let mut new = ModelInstance::new(CompiledModel::compile(model).unwrap());
        carry_over_parameters(&old, &mut new);
        // 新しい値域でクランプされ、評価し直されている。
        assert_eq!(new.parameters().as_slice(), &[0.25]);
        assert_eq!(new.drawables()[0].positions[0].x, 64.0 + 16.0);
    }
}
