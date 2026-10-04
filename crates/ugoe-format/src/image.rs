//! テクスチャの PNG のデコード (feature `png`)。
//!
//! バンドルの `TEXR` / `NRML` の PNG を、8bit の RGBA に正規化してデコードする。ヘッドレス描画 (`ugoe-headless`) と
//! WebAssembly (`ugoe-wasm`) が同じ関数を使い、画像の大きさの上限と色の展開の規則を 1 か所に保つ。
//!
//! - 色の値はリニア化しない (`docs/spec/rendering.md` の「前提」)。PNG の 8bit 値をそのまま使う。
//! - グレースケール・パレット・16bit は 8bit RGBA に展開する (16bit は上位 8bit)。
//! - 外から来る画像は信用しない。見出し (IHDR) の大きさだけで確保すると、小さなファイルで数 GB を求められるので、
//!   確保の前に一辺を [`ImageLimits::max_side`] と比べる (png の `Limits` はデコーダの中のバッファにしか効かない)。

use std::io::Cursor;

/// [`ImageLimits::default`] の、デコードする画像の一辺の上限 (16384)。
///
/// これより大きな 2D テクスチャを扱えるデバイスは少ない。RGBA8 で 1 GiB。
pub const DEFAULT_MAX_IMAGE_SIDE: u32 = 16384;

/// PNG のデコードの上限 (`docs/spec/file-formats.md#読み込みの上限`)。
///
/// 上限はファイル形式の決まりではなく、読む側の方針 ([`crate::ReadLimits`] と同じ考え方)。
/// 超える画像は、画素を確保する前に [`ImageError::TooLarge`] にする。
///
/// 将来ほかの上限を足せるように `#[non_exhaustive]` にしている。値を変えるときは
/// `ImageLimits::default().with_max_side(n)` のように既定値から始める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImageLimits {
    /// 画像の幅と高さそれぞれの上限 (画素)。既定は [`DEFAULT_MAX_IMAGE_SIDE`]。
    pub max_side: u32,
}

impl Default for ImageLimits {
    fn default() -> Self {
        Self {
            max_side: DEFAULT_MAX_IMAGE_SIDE,
        }
    }
}

impl ImageLimits {
    /// 一辺の上限を `pixels` にしたもの。
    pub const fn with_max_side(self, pixels: u32) -> Self {
        Self {
            max_side: pixels,
            ..self
        }
    }
}

/// 8bit RGBA の画像。`pixels` は行の順に `width * height * 4` バイト。
///
/// アルファが乗算済みかどうかは作った関数で決まる ([`decode_png`] はストレート、[`decode_png_premultiplied`] は乗算済み)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    /// 幅 (画素)。
    pub width: u32,
    /// 高さ (画素)。
    pub height: u32,
    /// 画素。1 画素 4 バイト (R・G・B・A)、左上から行の順。
    pub pixels: Vec<u8>,
}

/// PNG のデコードの失敗。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// 一辺が上限 ([`ImageLimits::max_side`]) を超える。画素は確保していない。
    #[error("PNG image is {width}x{height}, larger than the limit {max}x{max}")]
    TooLarge {
        /// 画像の幅 (画素)。
        width: u32,
        /// 画像の高さ (画素)。
        height: u32,
        /// 一辺の上限 (画素)。
        max: u32,
    },
    /// PNG として読めない (壊れている・切り詰め・対応しない形)。中身は png クレートのメッセージ。
    #[error("decoding PNG: {0}")]
    Decode(String),
}

/// PNG をデコードして、アルファを掛けない (ストレートの) RGBA8 にする。法線マップのようにアルファを使わない画像用。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。一辺が `limits.max_side` を超える画像は
/// 画素を確保する前に [`ImageError::TooLarge`]。
pub fn decode_png(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(decode_error)?;
    let (width, height) = (reader.info().width, reader.info().height);
    let max = limits.max_side;
    if width > max || height > max {
        return Err(ImageError::TooLarge { width, height, max });
    }
    // 上限を大きくした 32bit の環境 (wasm32) では、出力のバイト数が usize に収まらないことがある。
    let size = reader
        .output_buffer_size()
        .ok_or(ImageError::TooLarge { width, height, max })?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(decode_error)?;
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
        // `normalize_to_color8` がパレットを展開するので来ない。
        png::ColorType::Indexed => {
            return Err(ImageError::Decode(
                "indexed PNG was not expanded".to_owned(),
            ));
        }
    }
    Ok(Rgba8 {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// PNG をデコードして、乗算済みアルファの RGBA8 にする (描画するテクスチャ用)。
///
/// 展開と上限は [`decode_png`] と同じ。乗算は [`premultiply`]。
pub fn decode_png_premultiplied(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    let mut image = decode_png(bytes, limits)?;
    premultiply(&mut image.pixels);
    Ok(image)
}

/// ストレートアルファの RGBA8 を乗算済みにする (`c * a / 255` を四捨五入)。末尾の 4 バイトに満たない分は触らない。
pub fn premultiply(rgba: &mut [u8]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

fn decode_error(error: png::DecodingError) -> ImageError {
    ImageError::Decode(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        (width, height): (u32, u32),
        color: png::ColorType,
        depth: png::BitDepth,
        palette: Option<&[u8]>,
        data: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some(palette) = palette {
            encoder.set_palette(palette.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
        out
    }

    fn rgba8(width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        encode(
            (width, height),
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            None,
            data,
        )
    }

    fn straight(bytes: &[u8]) -> Vec<u8> {
        decode_png(bytes, ImageLimits::default()).unwrap().pixels
    }

    #[test]
    fn rgba_is_kept_and_premultiplied_on_request() {
        let bytes = rgba8(2, 1, &[255, 0, 0, 255, 128, 200, 255, 0]);
        let image = decode_png(&bytes, ImageLimits::default()).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        // ストレートのデコードはアルファ 0 の下の色も残す (法線マップ用)。
        assert_eq!(image.pixels, vec![255, 0, 0, 255, 128, 200, 255, 0]);
        let image = decode_png_premultiplied(&bytes, ImageLimits::default()).unwrap();
        assert_eq!(image.pixels, vec![255, 0, 0, 255, 0, 0, 0, 0]);
        let half = rgba8(1, 1, &[255, 255, 255, 128]);
        assert_eq!(
            decode_png_premultiplied(&half, ImageLimits::default())
                .unwrap()
                .pixels,
            vec![128, 128, 128, 128]
        );
    }

    #[test]
    fn rgb_is_opaque() {
        let bytes = encode(
            (1, 1),
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            None,
            &[1, 2, 3],
        );
        assert_eq!(straight(&bytes), vec![1, 2, 3, 255]);
    }

    #[test]
    fn grayscale_is_expanded_to_rgba() {
        let gray = encode(
            (2, 1),
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            None,
            &[0, 200],
        );
        assert_eq!(straight(&gray), vec![0, 0, 0, 255, 200, 200, 200, 255]);
        let gray_alpha = encode(
            (1, 1),
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Eight,
            None,
            &[100, 50],
        );
        assert_eq!(straight(&gray_alpha), vec![100, 100, 100, 50]);
        // 1bit のグレースケールも 8bit に広げる (0 → 0、1 → 255)。
        let bits = encode(
            (2, 1),
            png::ColorType::Grayscale,
            png::BitDepth::One,
            None,
            &[0b0100_0000],
        );
        assert_eq!(straight(&bits), vec![0, 0, 0, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn palette_is_expanded_to_rgba() {
        let palette = [10, 20, 30, 40, 50, 60];
        let bytes = encode(
            (2, 1),
            png::ColorType::Indexed,
            png::BitDepth::Eight,
            Some(&palette),
            &[1, 0],
        );
        assert_eq!(straight(&bytes), vec![40, 50, 60, 255, 10, 20, 30, 255]);
    }

    #[test]
    fn sixteen_bit_is_reduced_to_eight_bit() {
        // 16bit はビッグエンディアン。上位の 8bit を使う。
        let bytes = encode(
            (1, 1),
            png::ColorType::Rgba,
            png::BitDepth::Sixteen,
            None,
            &[0x12, 0x34, 0xab, 0xcd, 0xff, 0x00, 0x80, 0x01],
        );
        assert_eq!(straight(&bytes), vec![0x12, 0xab, 0xff, 0x80]);
    }

    /// IHDR が 65535x65535 で、画素のデータが zlib の見出しだけの小さな PNG。
    fn huge_png_header() -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 65535, 65535);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::IDAT, &[0x78, 0x9c]).unwrap();
        // 残りの画素は書かない (`finish` も呼ばない)。
        std::mem::forget(writer);
        out
    }

    /// 見出しの大きさを確かめてから確保する (外から来た PNG で約 17GB を確保しようとして落ちないように)。
    #[test]
    fn rejects_images_larger_than_the_limit_before_allocating() {
        let err = decode_png(&huge_png_header(), ImageLimits::default()).unwrap_err();
        assert!(
            matches!(
                err,
                ImageError::TooLarge {
                    width: 65535,
                    height: 65535,
                    max: DEFAULT_MAX_IMAGE_SIDE,
                }
            ),
            "{err}"
        );
        assert_eq!(
            err.to_string(),
            "PNG image is 65535x65535, larger than the limit 16384x16384"
        );
        assert!(decode_png_premultiplied(&huge_png_header(), ImageLimits::default()).is_err());
    }

    /// 上限は呼び出し側が変えられる。ちょうど上限の一辺は読め、1 つ超えるとエラー。
    #[test]
    fn the_limit_can_be_changed_by_the_caller() {
        let bytes = rgba8(3, 2, &[7; 3 * 2 * 4]);
        assert_eq!(ImageLimits::default().with_max_side(3).max_side, 3);
        let image = decode_png(&bytes, ImageLimits::default().with_max_side(3)).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        let err = decode_png(&bytes, ImageLimits::default().with_max_side(2)).unwrap_err();
        assert!(
            matches!(
                err,
                ImageError::TooLarge {
                    width: 3,
                    height: 2,
                    max: 2,
                }
            ),
            "{err}"
        );
        // 高さだけが超えるときも同じ。
        let tall = rgba8(1, 3, &[0; 3 * 4]);
        assert!(decode_png(&tall, ImageLimits::default().with_max_side(2)).is_err());
    }

    #[test]
    fn broken_input_is_an_error() {
        let err = decode_png(b"not a png", ImageLimits::default()).unwrap_err();
        assert!(matches!(err, ImageError::Decode(_)), "{err}");
        // 見出しは正しいが画素が足りない。
        let mut truncated = rgba8(4, 4, &[1; 4 * 4 * 4]);
        truncated.truncate(truncated.len() - 20);
        assert!(decode_png(&truncated, ImageLimits::default()).is_err());
    }

    #[test]
    fn premultiply_rounds_to_nearest() {
        let mut px = vec![200, 100, 50, 128, 10, 20, 30, 0, 255, 255, 255, 255, 9];
        premultiply(&mut px);
        assert_eq!(
            px,
            vec![100, 50, 25, 128, 0, 0, 0, 0, 255, 255, 255, 255, 9]
        );
    }
}
