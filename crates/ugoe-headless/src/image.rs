//! PNG の読み書きと、乗算済みアルファとの変換。
//!
//! 色の値はリニア化しない (`docs/spec/rendering.md` の「前提」)。PNG の 8bit 値をそのまま使う。

use crate::Error;

pub use ugoe_format::image::{ImageLimits, Rgba8, premultiply};

/// PNG をデコードして、乗算済みアルファの RGBA8 にする。
///
/// デコードは `ugoe_format::image` (ugoe-wasm と共有)。グレースケール・パレット・16bit は 8bit RGBA に展開する。
/// 一辺が既定の上限 ([`ugoe_format::image::DEFAULT_MAX_IMAGE_SIDE`] = 出力の [`crate::MAX_SIZE`] と同じ) を超える画像はエラー。
pub fn decode_png_premultiplied(bytes: &[u8]) -> Result<Rgba8, Error> {
    Ok(ugoe_format::image::decode_png_premultiplied(
        bytes,
        ImageLimits::default(),
    )?)
}

/// PNG をデコードして、アルファを掛けない RGBA8 にする (法線マップ用。アルファは使わない)。
///
/// 展開と上限は [`decode_png_premultiplied`] と同じ。
pub fn decode_png(bytes: &[u8]) -> Result<Rgba8, Error> {
    Ok(ugoe_format::image::decode_png(
        bytes,
        ImageLimits::default(),
    )?)
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

/// 乗算済みの `image` を、`size` の出力を覆うように拡大・縮小して中央に置く (CSS の `background-size: cover` と
/// `background-position: center`)。はみ出した分は切り、標本はバイリニアで取る (端は端の画素を伸ばす)。
/// 結果は乗算済みの RGBA8。スタジオの仮想カメラが、シーンの背景の画像を出力ウィンドウと同じ置き方で描くのに使う。
pub fn cover(image: &Rgba8, (width, height): (u32, u32)) -> Rgba8 {
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let (iw, ih) = (image.width as usize, image.height as usize);
    if iw == 0 || ih == 0 || image.pixels.len() < iw * ih * 4 {
        return Rgba8 {
            width,
            height,
            pixels,
        };
    }
    let scale = (width as f32 / iw as f32).max(height as f32 / ih as f32);
    // 出力の画素の中心 → 画像の座標 (画素の中心が整数 + 0.5)。
    let ox = (iw as f32 * scale - width as f32) / 2.0;
    let oy = (ih as f32 * scale - height as f32) / 2.0;
    let texel = |x: usize, y: usize| {
        let i = (y * iw + x) * 4;
        [
            f32::from(image.pixels[i]),
            f32::from(image.pixels[i + 1]),
            f32::from(image.pixels[i + 2]),
            f32::from(image.pixels[i + 3]),
        ]
    };
    let axis = |p: f32, n: usize| {
        let p = (p - 0.5).clamp(0.0, (n - 1) as f32);
        let i = p.floor() as usize;
        (i, (i + 1).min(n - 1), p - i as f32)
    };
    for y in 0..height as usize {
        let (y0, y1, fy) = axis((y as f32 + 0.5 + oy) / scale, ih);
        for x in 0..width as usize {
            let (x0, x1, fx) = axis((x as f32 + 0.5 + ox) / scale, iw);
            let (a, b, c, d) = (texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1));
            let out = &mut pixels[(y * width as usize + x) * 4..][..4];
            for k in 0..4 {
                let top = a[k] + (b[k] - a[k]) * fx;
                let bottom = c[k] + (d[k] - c[k]) * fx;
                out[k] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Rgba8 {
        width,
        height,
        pixels,
    }
}

/// 乗算済みの `image` を、不透明な色 `rgb` の上に重ねる (結果は不透明)。
pub fn over_color(image: &mut Rgba8, rgb: [u8; 3]) {
    for p in image.pixels.as_chunks_mut::<4>().0 {
        let rest = 255 - u32::from(p[3]);
        for (c, bg) in p[..3].iter_mut().zip(rgb) {
            *c = (u32::from(*c) + (u32::from(bg) * rest + 127) / 255).min(255) as u8;
        }
        p[3] = 255;
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

fn png_encode(context: &'static str) -> impl FnOnce(png::EncodingError) -> Error {
    move |source| Error::PngEncode { context, source }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_fills_the_output_and_crops_the_long_side() {
        // 左半分が赤・右半分が青の 4x2 の画像を、2x2 に覆うように置く (縦に合わせて横の両端を切る)。
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        let row = [red, red, blue, blue].concat();
        let image = Rgba8 {
            width: 4,
            height: 2,
            pixels: [row.clone(), row].concat(),
        };
        let out = cover(&image, (2, 2));
        assert_eq!((out.width, out.height), (2, 2));
        assert_eq!(&out.pixels[..4], &red);
        assert_eq!(&out.pixels[4..8], &blue);
        // 2 倍に拡大すると、同じ色の中は同じ色のまま、境目はなめらかに変わる。
        let big = cover(&image, (8, 4));
        assert_eq!(&big.pixels[..4], &red);
        assert_eq!(&big.pixels[7 * 4..8 * 4], &blue);
        let middle = &big.pixels[3 * 4..4 * 4];
        assert!(middle[0] > 0 && middle[2] > 0, "{middle:?}");
        // 空の画像は透過になる。
        let empty = cover(
            &Rgba8 {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
            (2, 1),
        );
        assert_eq!(empty.pixels, vec![0; 8]);
    }

    #[test]
    fn over_color_fills_the_transparent_part() {
        let mut image = Rgba8 {
            width: 3,
            height: 1,
            pixels: vec![10, 20, 30, 255, 0, 0, 0, 0, 64, 0, 0, 128],
        };
        over_color(&mut image, [200, 100, 50]);
        assert_eq!(
            image.pixels,
            vec![10, 20, 30, 255, 200, 100, 50, 255, 64 + 100, 50, 25, 255]
        );
    }

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
    fn straight_decode_keeps_color_under_zero_alpha() {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[128, 200, 255, 0]).unwrap();
        writer.finish().unwrap();
        assert_eq!(decode_png(&out).unwrap().pixels, vec![128, 200, 255, 0]);
        assert_eq!(decode_png_premultiplied(&out).unwrap().pixels, vec![0; 4]);
    }

    /// 上限と展開の規則は `ugoe_format::image` のテストが確かめる。ここでは既定の上限が出力の上限と同じことと、
    /// エラーがそのまま伝わることだけを確かめる。
    #[test]
    fn uses_the_shared_decoder_with_the_output_limit() {
        assert_eq!(ugoe_format::image::DEFAULT_MAX_IMAGE_SIDE, crate::MAX_SIZE);
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, crate::MAX_SIZE + 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::IDAT, &[0x78, 0x9c]).unwrap();
        // 残りの画素は書かない (見出しで止まる)。
        std::mem::forget(writer);
        let err = decode_png(&out).unwrap_err().to_string();
        assert!(err.contains("16385x1, larger than the limit"), "{err}");
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_png_premultiplied(b"not a png").is_err());
    }
}
