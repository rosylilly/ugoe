//! PNG の読み書きと、乗算済みアルファとの変換。
//!
//! 色の値はリニア化しない (`docs/spec/rendering.md` の「前提」)。PNG の 8bit 値をそのまま使う。

use std::io::Cursor;

use crate::{Error, bail};

/// RGBA8 の画像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// PNG をデコードして、乗算済みアルファの RGBA8 にする。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。
pub fn decode_png_premultiplied(bytes: &[u8]) -> Result<Rgba8, Error> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(png_decode("reading PNG header"))?;
    let Some(size) = reader.output_buffer_size() else {
        bail!("PNG image is too large");
    };
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(png_decode("decoding PNG"))?;
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
        png::ColorType::Indexed => bail!("indexed PNG was not expanded"),
    }
    premultiply(&mut pixels);
    Ok(Rgba8 {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// 乗算済みアルファの RGBA8 を、ストレートアルファの PNG にエンコードする。
pub fn encode_png_unpremultiplied(image: &Rgba8) -> Result<Vec<u8>, Error> {
    let mut pixels = image.pixels.clone();
    unpremultiply(&mut pixels);
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(png_encode("writing PNG header"))?;
    writer
        .write_image_data(&pixels)
        .map_err(png_encode("writing PNG data"))?;
    writer.finish().map_err(png_encode("finishing PNG"))?;
    Ok(out)
}

/// ストレートアルファを乗算済みにする。
pub fn premultiply(rgba: &mut [u8]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

/// 乗算済みから戻す。アルファ 0 の画素は黒にする。
pub fn unpremultiply(rgba: &mut [u8]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = (u32::from(*c) * 255 + a / 2)
                .checked_div(a)
                .map_or(0, |v| v.min(255) as u8);
        }
    }
}

fn png_decode(context: &'static str) -> impl FnOnce(png::DecodingError) -> Error {
    move |source| Error::PngDecode { context, source }
}

fn png_encode(context: &'static str) -> impl FnOnce(png::EncodingError) -> Error {
    move |source| Error::PngEncode { context, source }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiply_round_trip() {
        let mut px = vec![200, 100, 50, 128, 10, 20, 30, 0, 255, 255, 255, 255];
        premultiply(&mut px);
        assert_eq!(px, vec![100, 50, 25, 128, 0, 0, 0, 0, 255, 255, 255, 255]);
        unpremultiply(&mut px);
        assert_eq!(px, vec![199, 100, 50, 128, 0, 0, 0, 0, 255, 255, 255, 255]);
    }

    #[test]
    fn unpremultiply_clamps_invalid_values() {
        // 乗算済みとして不正な値 (色 > アルファ) でも溢れない。
        let mut px = vec![255, 0, 0, 10];
        unpremultiply(&mut px);
        assert_eq!(px, vec![255, 0, 0, 10]);
    }

    #[test]
    fn png_round_trip_premultiplies() {
        let image = Rgba8 {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 255, 0, 0, 128, 128],
        };
        let bytes = encode_png_unpremultiplied(&image).unwrap();
        let decoded = decode_png_premultiplied(&bytes).unwrap();
        assert_eq!(decoded, image);
    }

    #[test]
    fn decodes_rgb_png_as_opaque() {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 1, 1);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[1, 2, 3]).unwrap();
        writer.finish().unwrap();
        let decoded = decode_png_premultiplied(&out).unwrap();
        assert_eq!(decoded.pixels, vec![1, 2, 3, 255]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_png_premultiplied(b"not a png").is_err());
    }
}
