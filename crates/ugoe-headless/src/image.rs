//! PNG の読み書き・KTX2 の読み込みと、乗算済みアルファとの変換。
//!
//! 色の値はリニア化しない (`docs/spec/rendering.md` の「前提」)。PNG と KTX2 の 8bit 値をそのまま使う。

use ugoe_format::TextureFormat;

use crate::{Error, bc};

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

/// 形式 (PNG・KTX2) に合わせて画像をデコードし、乗算済みアルファの RGBA8 にする (`ugoe_format::image`)。
///
/// KTX2 は圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7 を読み、ほかの形式 (ETC2・ASTC・Basis Universal など) は
/// [`ugoe_format::image::ImageError::UnsupportedKtx2`] ([`Error::ImageDecode`])。上限は [`decode_png_premultiplied`] と同じ。
pub fn decode_image_premultiplied(format: TextureFormat, bytes: &[u8]) -> Result<Rgba8, Error> {
    Ok(ugoe_format::image::decode_image_premultiplied(
        format,
        bytes,
        ImageLimits::default(),
    )?)
}

/// 形式 (PNG・KTX2) に合わせて画像をデコードし、アルファを掛けない RGBA8 にする (法線マップ用)。
///
/// 読む形式と上限は [`decode_image_premultiplied`] と同じ。
pub fn decode_image(format: TextureFormat, bytes: &[u8]) -> Result<Rgba8, Error> {
    Ok(ugoe_format::image::decode_image(
        format,
        bytes,
        ImageLimits::default(),
    )?)
}

/// [`encode_ktx2`] の書き方。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ktx2Options {
    /// 転送関数 sRGB の形式 (`VK_FORMAT_*_SRGB*`) で書く。偽ならリニアの形式 (`*_UNORM*`)。
    /// 画素の値はどちらも同じに書く (読む側は値を保存されたまま使う。`docs/spec/rendering.md#前提`)。
    pub srgb: bool,
    /// 各段を zstd で超圧縮する (`supercompressionScheme` = 2)。
    pub zstd: bool,
    /// ブロック圧縮。既定は圧縮しない RGBA8。
    pub compression: Ktx2Compression,
}

/// [`encode_ktx2`] のブロック圧縮 (`docs/adr/0030-compressed-ktx2-decoding.md`)。
///
/// 適合テストで読む側の展開を比べるための簡単な圧縮で、画質は求めない (端点は各チャンネルの最小と最大)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Ktx2Compression {
    /// 圧縮しない (`VK_FORMAT_R8G8B8A8_*`)。
    #[default]
    None,
    /// BC1 のアルファなし (`VK_FORMAT_BC1_RGB_*_BLOCK`)。アルファは捨てる。
    Bc1,
    /// BC1 のアルファなしで、すべてのブロックを 3 色のモード (`color0 <= color1`。4 つ目の色は黒) で書く。
    /// アルファのない BC1 では 4 つ目の色も不透明に読むことを確かめるためのもの。アルファは捨てる。
    Bc1ThreeColor,
    /// BC3 (`VK_FORMAT_BC3_*_BLOCK`)。
    Bc3,
    /// BC7 (`VK_FORMAT_BC7_*_BLOCK`。モード 6 だけを使う)。
    Bc7,
}

impl Ktx2Compression {
    /// `vkFormat` (リニアと sRGB) と、データ形式の記述の色のモデル。
    fn vk_format_and_model(self) -> ((u32, u32), u8) {
        match self {
            Self::None => ((37, 43), DF_MODEL_RGBSDA), // VK_FORMAT_R8G8B8A8_UNORM / _SRGB
            Self::Bc1 | Self::Bc1ThreeColor => ((131, 132), 128), // VK_FORMAT_BC1_RGB_*_BLOCK、KHR_DF_MODEL_BC1A
            Self::Bc3 => ((137, 138), 130), // VK_FORMAT_BC3_*_BLOCK、KHR_DF_MODEL_BC3
            Self::Bc7 => ((145, 146), 134), // VK_FORMAT_BC7_*_BLOCK、KHR_DF_MODEL_BC7
        }
    }

    fn block(self) -> Option<bc::BlockFormat> {
        match self {
            Self::None => None,
            Self::Bc1 => Some(bc::BlockFormat::Bc1),
            Self::Bc1ThreeColor => Some(bc::BlockFormat::Bc1ThreeColor),
            Self::Bc3 => Some(bc::BlockFormat::Bc3),
            Self::Bc7 => Some(bc::BlockFormat::Bc7),
        }
    }
}

/// `KHR_DF_MODEL_RGBSDA`・`KHR_DF_PRIMARIES_BT709`・`KHR_DF_TRANSFER_LINEAR` / `_SRGB`。
const DF_MODEL_RGBSDA: u8 = 1;
const DF_PRIMARIES_BT709: u8 = 1;
const DF_TRANSFER_LINEAR: u8 = 1;
const DF_TRANSFER_SRGB: u8 = 2;

/// ストレートアルファの RGBA8 の段 (`levels[0]` がレベル 0。続く段は幅と高さを半分 (切り捨て、最小 1) にしたもの) を、
/// KTX2 (圧縮していない RGBA8 か、`options.compression` のブロック圧縮) にする。適合テストと単体テストで KTX2 の
/// バンドルを作るためのもの。
///
/// データ形式の記述の `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` は立てない (アルファは乗算していない。`docs/spec/rendering.md#前提`)。
/// 段が無い・画素の数が大きさと合わない・段の大きさが半分ずつでなければ [`Error::Invalid`]。
pub fn encode_ktx2(levels: &[Rgba8], options: Ktx2Options) -> Result<Vec<u8>, Error> {
    let Some(base) = levels.first() else {
        return Err(Error::Invalid("KTX2 needs at least one level".to_owned()));
    };
    for (i, level) in levels.iter().enumerate() {
        let size = ((base.width >> i).max(1), (base.height >> i).max(1));
        if (level.width, level.height) != size
            || level.pixels.len() != level.width as usize * level.height as usize * 4
        {
            return Err(Error::Invalid(format!(
                "KTX2 level {i} must be {}x{} with 4 bytes per pixel",
                size.0, size.1
            )));
        }
    }
    let ((linear, srgb), model) = options.compression.vk_format_and_model();
    let (vk_format, transfer) = if options.srgb {
        (srgb, DF_TRANSFER_SRGB)
    } else {
        (linear, DF_TRANSFER_LINEAR)
    };
    let block = options.compression.block();
    // データ形式の記述: dfdTotalSize と基本の記述ブロック (KDF 1.3) 1 つ。
    // 標本は、RGBA8 なら R・G・B・A の 8bit、ブロック圧縮ならブロックの中の色 (とアルファ) のビットの範囲。
    let samples: Vec<(u16, u8, u8, u32)> = match options.compression {
        // (bitOffset、bitLength - 1、channelType、sampleUpper)。sRGB のアルファはリニア (KHR_DF_SAMPLE_DATATYPE_LINEAR)。
        Ktx2Compression::None => {
            let alpha = if options.srgb { 15 | 0x10 } else { 15 };
            [0u8, 1, 2, alpha]
                .into_iter()
                .enumerate()
                .map(|(i, channel)| (i as u16 * 8, 7, channel, 255))
                .collect()
        }
        Ktx2Compression::Bc1 | Ktx2Compression::Bc1ThreeColor => vec![(0, 63, 0, u32::MAX)],
        Ktx2Compression::Bc7 => vec![(0, 127, 0, u32::MAX)],
        Ktx2Compression::Bc3 => vec![(0, 63, 15, u32::MAX), (64, 63, 0, u32::MAX)],
    };
    let mut dfd = Vec::new();
    let block_size = 24 + 16 * samples.len() as u32;
    dfd.extend((4 + block_size).to_le_bytes());
    dfd.extend(0u32.to_le_bytes()); // vendorId = Khronos、descriptorType = basic
    dfd.extend((2 | (block_size << 16)).to_le_bytes()); // versionNumber = 1.3
    dfd.extend([model, DF_PRIMARIES_BT709, transfer, 0]); // flags = 0 (乗算していない)
    let (dimension, bytes_plane) = match block {
        Some(block) => ([3, 3, 0, 0], block.block_bytes() as u8),
        None => ([0; 4], 4),
    };
    dfd.extend(dimension); // texelBlockDimension (4x4 は 3)
    dfd.extend([bytes_plane, 0, 0, 0, 0, 0, 0, 0]); // bytesPlane
    for (offset, length, channel, upper) in samples {
        dfd.extend(offset.to_le_bytes());
        dfd.extend([length, channel]);
        dfd.extend(0u32.to_le_bytes()); // samplePosition
        dfd.extend(0u32.to_le_bytes()); // sampleLower
        dfd.extend(upper.to_le_bytes()); // sampleUpper
    }
    let raw: Vec<Vec<u8>> = levels
        .iter()
        .map(|level| match block {
            Some(block) => bc::encode(level, block),
            None => level.pixels.clone(),
        })
        .collect();
    let stored: Vec<Vec<u8>> = raw
        .iter()
        .map(|level| {
            if options.zstd {
                ruzstd::encoding::compress_to_vec(
                    level.as_slice(),
                    ruzstd::encoding::CompressionLevel::Fastest,
                )
            } else {
                level.clone()
            }
        })
        .collect();
    let level_index = 80;
    let dfd_offset = level_index + 24 * levels.len();
    let mut out = ugoe_format::image::KTX2_IDENTIFIER.to_vec();
    let header = [
        vk_format,
        1, // typeSize
        base.width,
        base.height,
        0, // pixelDepth
        0, // layerCount
        1, // faceCount
        levels.len() as u32,
        if options.zstd { 2 } else { 0 },
        dfd_offset as u32,
        dfd.len() as u32,
        0, // kvdByteOffset
        0, // kvdByteLength
    ];
    for value in header {
        out.extend(value.to_le_bytes());
    }
    out.extend([0; 16]); // sgdByteOffset・sgdByteLength
    // 段のデータは小さい段から順に置く。超圧縮しないときは画素 (ブロック) の大きさと 4 の公倍数の境界に揃える。
    let align = if options.zstd {
        1
    } else {
        block.map_or(4, |b| b.block_bytes())
    };
    let mut cursor = dfd_offset + dfd.len();
    let mut offsets = vec![0; levels.len()];
    for (i, data) in stored.iter().enumerate().rev() {
        cursor = cursor.next_multiple_of(align);
        offsets[i] = cursor;
        cursor += data.len();
    }
    for (i, data) in stored.iter().enumerate() {
        out.extend((offsets[i] as u64).to_le_bytes());
        out.extend((data.len() as u64).to_le_bytes());
        out.extend((raw[i].len() as u64).to_le_bytes());
    }
    out.extend(dfd);
    for (i, data) in stored.iter().enumerate().rev() {
        out.resize(offsets[i], 0);
        out.extend(data);
    }
    Ok(out)
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

    fn level(width: u32, height: u32, value: u8) -> Rgba8 {
        Rgba8 {
            width,
            height,
            pixels: vec![value; width as usize * height as usize * 4],
        }
    }

    /// 書いた KTX2 は `ugoe_format::image` で読め、レベル 0 の値が保存されたまま戻る。乗算済みの印は立てない。
    #[test]
    fn ktx2_round_trip_keeps_values_and_does_not_flag_premultiplied() {
        let base = Rgba8 {
            width: 2,
            height: 2,
            pixels: vec![
                255, 0, 0, 255, 128, 200, 255, 0, 255, 255, 255, 128, 10, 20, 30, 40,
            ],
        };
        for srgb in [false, true] {
            for zstd in [false, true] {
                let options = Ktx2Options {
                    srgb,
                    zstd,
                    ..Default::default()
                };
                let bytes = encode_ktx2(&[base.clone(), level(1, 1, 7)], options).unwrap();
                assert_eq!(
                    ugoe_format::TextureFormat::detect(&bytes),
                    Some(TextureFormat::Ktx2)
                );
                // データ形式の記述 (dfdByteOffset から) の転送関数と flags。
                // flags の KHR_DF_FLAG_ALPHA_PREMULTIPLIED (bit 0) は立てない。
                let dfd = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
                assert_eq!(bytes[dfd + 4 + 8 + 2], if srgb { 2 } else { 1 }, "transfer");
                assert_eq!(bytes[dfd + 4 + 8 + 3] & 1, 0);
                assert_eq!(
                    decode_image(TextureFormat::Ktx2, &bytes).unwrap(),
                    base,
                    "{options:?}"
                );
                let mut premultiplied = base.clone();
                premultiply(&mut premultiplied.pixels);
                assert_eq!(
                    decode_image_premultiplied(TextureFormat::Ktx2, &bytes).unwrap(),
                    premultiplied
                );
            }
        }
    }

    /// 色とアルファが位置で滑らかに変わる、4 の倍数でない大きさの画像 (ストレートアルファ)。
    fn gradient(width: u32, height: u32) -> Rgba8 {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend([
                    (x * 255 / (width - 1)) as u8,
                    (y * 255 / (height - 1)) as u8,
                    ((x + y) * 255 / (width + height - 2)) as u8,
                    (255 - y * 200 / (height - 1)) as u8,
                ]);
            }
        }
        Rgba8 {
            width,
            height,
            pixels,
        }
    }

    /// ブロック圧縮の KTX2 は `ugoe_format::image` で読め、元の画素に近い (BC1 はアルファを捨てる)。
    /// `vkFormat` と、データ形式の記述の色のモデル・ブロックの大きさも確かめる。
    #[test]
    fn ktx2_block_compression_round_trips_approximately() {
        let base = gradient(37, 30);
        let mip = gradient(18, 15);
        for (compression, formats, model, tolerance) in [
            (Ktx2Compression::Bc1, (131, 132), 128, [24, 24, 24, 255]),
            (
                Ktx2Compression::Bc1ThreeColor,
                (131, 132),
                128,
                [32, 32, 32, 255],
            ),
            (Ktx2Compression::Bc3, (137, 138), 130, [24, 24, 24, 10]),
            (Ktx2Compression::Bc7, (145, 146), 134, [24, 24, 24, 24]),
        ] {
            for (srgb, zstd) in [(false, false), (true, true)] {
                let options = Ktx2Options {
                    srgb,
                    zstd,
                    compression,
                };
                let bytes = encode_ktx2(&[base.clone(), mip.clone()], options).unwrap();
                let vk_format = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
                assert_eq!(vk_format, if srgb { formats.1 } else { formats.0 });
                let dfd = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
                assert_eq!(bytes[dfd + 12], model, "{options:?}");
                assert_eq!(bytes[dfd + 16], 3, "texelBlockDimension0 {options:?}");
                let image = decode_image(TextureFormat::Ktx2, &bytes).unwrap();
                assert_eq!((image.width, image.height), (37, 30));
                for (a, b) in image.pixels.chunks(4).zip(base.pixels.chunks(4)) {
                    for c in 0..4 {
                        let diff = a[c].abs_diff(b[c]);
                        assert!(diff <= tolerance[c], "{options:?}: {a:?} vs {b:?}");
                    }
                }
                if matches!(
                    compression,
                    Ktx2Compression::Bc1 | Ktx2Compression::Bc1ThreeColor
                ) {
                    assert!(image.pixels.chunks(4).all(|p| p[3] == 255));
                }
            }
        }
    }

    /// 一色のブロックは、BC3 のアルファでは元の値のまま、BC7 では 1 以内で戻る (p ビットは RGBA で共有するので、
    /// 偶奇の違うチャンネルを同時にちょうどにはできない)。
    #[test]
    fn ktx2_block_compression_keeps_solid_colors() {
        let solid = Rgba8 {
            width: 4,
            height: 4,
            pixels: [201, 100, 3, 77].repeat(16),
        };
        let decode = |compression| {
            let options = Ktx2Options {
                compression,
                ..Default::default()
            };
            let bytes = encode_ktx2(std::slice::from_ref(&solid), options).unwrap();
            decode_image(TextureFormat::Ktx2, &bytes).unwrap().pixels
        };
        let bc7 = decode(Ktx2Compression::Bc7);
        assert!(
            bc7.iter()
                .zip(&solid.pixels)
                .all(|(a, b)| a.abs_diff(*b) <= 1),
            "{bc7:?}"
        );
        assert!(decode(Ktx2Compression::Bc3).chunks(4).all(|p| p[3] == 77));
    }

    /// `Bc1ThreeColor` はすべてのブロックを 3 色のモード (`color0 <= color1`) で書き、黒い画素は添字 3 (黒) にする。
    /// アルファのない BC1 なので、添字 3 も不透明な黒に読む。
    #[test]
    fn ktx2_bc1_three_color_mode_uses_opaque_black() {
        let (width, height) = (8, 4);
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend(if (x + y) % 3 == 0 {
                    [0, 0, 0, 0]
                } else {
                    [200, 120 + 10 * x as u8, 40, 255]
                });
            }
        }
        let image = Rgba8 {
            width,
            height,
            pixels,
        };
        let options = Ktx2Options {
            compression: Ktx2Compression::Bc1ThreeColor,
            ..Default::default()
        };
        let bytes = encode_ktx2(std::slice::from_ref(&image), options).unwrap();
        // 段は 1 つで、ファイルの終わりの 2 ブロック。
        let blocks = &bytes[bytes.len() - 16..];
        let mut three = 0;
        for block in blocks.chunks(8) {
            let c0 = u16::from_le_bytes([block[0], block[1]]);
            let c1 = u16::from_le_bytes([block[2], block[3]]);
            assert!(c0 <= c1, "{block:?}");
            let indices = u32::from_le_bytes(block[4..].try_into().unwrap());
            three += (0..16).filter(|i| (indices >> (2 * i)) & 3 == 3).count();
        }
        let decoded = decode_image(TextureFormat::Ktx2, &bytes).unwrap();
        let black = image.pixels.chunks(4).filter(|p| p[3] == 0).count();
        assert_eq!(three, black);
        for (a, b) in decoded.pixels.chunks(4).zip(image.pixels.chunks(4)) {
            if b[3] == 0 {
                assert_eq!(a, [0, 0, 0, 255]);
            } else {
                assert_eq!(a[3], 255);
                assert!((0..3).all(|c| a[c].abs_diff(b[c]) <= 24), "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn ktx2_levels_must_halve() {
        let options = Ktx2Options::default();
        assert!(encode_ktx2(&[], options).is_err());
        assert!(encode_ktx2(&[level(4, 1, 0), level(2, 1, 0), level(1, 1, 0)], options).is_ok());
        assert!(encode_ktx2(&[level(4, 1, 0), level(1, 1, 0)], options).is_err());
        let mut short = level(2, 2, 0);
        short.pixels.pop();
        assert!(encode_ktx2(&[short], options).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_png_premultiplied(b"not a png").is_err());
    }
}
