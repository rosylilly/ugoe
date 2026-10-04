//! テクスチャの PNG と KTX2 のデコード (feature `png`)。
//!
//! バンドルの `TEXR` / `NRML` の PNG と KTX2 を、8bit の RGBA に正規化してデコードする。ヘッドレス描画 (`ugoe-headless`) と
//! WebAssembly (`ugoe-wasm`) が同じ関数を使い、画像の大きさの上限と色の展開の規則を 1 か所に保つ。
//!
//! - 色の値はリニア化しない (`docs/spec/rendering.md` の「前提」)。PNG の 8bit 値をそのまま使う。
//!   KTX2 も、形式の `_SRGB` や転送関数によらず保存された値のまま使う。
//! - グレースケール・パレット・16bit は 8bit RGBA に展開する (16bit は上位 8bit)。
//! - KTX2 は圧縮していない 8bit の RGB / RGBA ([`decode_ktx2`]) だけを読む。ブロック圧縮などは
//!   [`ImageError::UnsupportedKtx2`] にする (読む側が代用するか誤りにするかを決める)。
//! - 外から来る画像は信用しない。見出し (IHDR) の大きさだけで確保すると、小さなファイルで数 GB を求められるので、
//!   確保の前に一辺を [`ImageLimits::max_side`] と比べる (png の `Limits` はデコーダの中のバッファにしか効かない)。
//!   KTX2 も見出しの大きさを確保の前に比べる。

use std::io::{Cursor, Read};

use crate::bundle::{KTX2_SIGNATURE, TextureFormat};

/// [`ImageLimits::default`] の、デコードする画像の一辺の上限 (16384)。
///
/// これより大きな 2D テクスチャを扱えるデバイスは少ない。RGBA8 で 1 GiB。
pub const DEFAULT_MAX_IMAGE_SIDE: u32 = 16384;

/// 画像 (PNG・KTX2) のデコードの上限 (`docs/spec/file-formats.md#読み込みの上限`)。
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
/// アルファが乗算済みかどうかは作った関数で決まる ([`decode_png`]・[`decode_ktx2`] はストレート、
/// `*_premultiplied` は乗算済み)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    /// 幅 (画素)。
    pub width: u32,
    /// 高さ (画素)。
    pub height: u32,
    /// 画素。1 画素 4 バイト (R・G・B・A)、左上から行の順。
    pub pixels: Vec<u8>,
}

/// 画像 (PNG・KTX2) のデコードの失敗。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// 一辺が上限 ([`ImageLimits::max_side`]) を超える。画素は確保していない。
    #[error("image is {width}x{height}, larger than the limit {max}x{max}")]
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
    /// KTX2 として読めない (見出しやレベルの範囲が壊れている・切り詰め・zstd の展開の失敗)。
    #[error("decoding KTX2: {0}")]
    InvalidKtx2(String),
    /// 読める KTX2 だが、デコードしない形式 (ブロック圧縮・Basis Universal・8bit の RGB / RGBA 以外・
    /// 2D でないテクスチャなど)。中身は理由。
    #[error("unsupported KTX2: {0}")]
    UnsupportedKtx2(String),
}

/// 形式 (`format`) に合わせて画像をデコードし、アルファを掛けない RGBA8 にする (法線マップ用)。
pub fn decode_image(
    format: TextureFormat,
    bytes: &[u8],
    limits: ImageLimits,
) -> Result<Rgba8, ImageError> {
    match format {
        TextureFormat::Png => decode_png(bytes, limits),
        TextureFormat::Ktx2 => decode_ktx2(bytes, limits),
    }
}

/// 形式 (`format`) に合わせて画像をデコードし、乗算済みアルファの RGBA8 にする (描画するテクスチャ用)。
pub fn decode_image_premultiplied(
    format: TextureFormat,
    bytes: &[u8],
    limits: ImageLimits,
) -> Result<Rgba8, ImageError> {
    let mut image = decode_image(format, bytes, limits)?;
    premultiply(&mut image.pixels);
    Ok(image)
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

/// KTX2 ファイルの先頭の識別子 (12 バイト)。
pub const KTX2_IDENTIFIER: [u8; 12] = KTX2_SIGNATURE;

/// KTX2 の `vkFormat` のうち、デコードする形式 (圧縮していない 8bit の RGB / RGBA) と 1 画素のバイト数。
///
/// `_SRGB` と `_UNORM` は同じに扱う (値を保存されたまま使う。`docs/spec/rendering.md#前提`)。
const KTX2_FORMATS: [(u32, usize); 4] = [
    (23, 3), // VK_FORMAT_R8G8B8_UNORM
    (29, 3), // VK_FORMAT_R8G8B8_SRGB
    (37, 4), // VK_FORMAT_R8G8B8A8_UNORM
    (43, 4), // VK_FORMAT_R8G8B8A8_SRGB
];
/// KTX2 の見出し (識別子・形式・大きさ・索引) と、レベルの索引の 1 段の大きさ。
const KTX2_HEADER_LEN: usize = 80;
const KTX2_LEVEL_LEN: usize = 24;
/// `supercompressionScheme` の値。
const KTX2_SUPERCOMPRESSION_NONE: u32 = 0;
const KTX2_SUPERCOMPRESSION_ZSTD: u32 = 2;

/// KTX2 をデコードして、アルファを掛けない (ストレートの) RGBA8 にする。法線マップのようにアルファを使わない画像用。
///
/// 読むのは圧縮していない `VK_FORMAT_R8G8B8_*` / `VK_FORMAT_R8G8B8A8_*` (`_UNORM`・`_SRGB`) の 2D テクスチャで、
/// 超圧縮は無しか zstd。値は形式や転送関数によらず保存されたまま使い、ミップマップは最初の段 (レベル 0) だけを読む。
/// データ形式の記述 (`KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を含む) は見ない (`docs/spec/rendering.md#前提`)。
///
/// - 一辺が `limits.max_side` を超える画像は、画素を確保する前に [`ImageError::TooLarge`]。
/// - ほかの形式 (ブロック圧縮・Basis Universal など)・配列・キューブマップ・3D は [`ImageError::UnsupportedKtx2`]。
/// - 見出しやレベルの範囲が壊れていれば [`ImageError::InvalidKtx2`]。
pub fn decode_ktx2(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    let invalid = |message: &str| ImageError::InvalidKtx2(message.to_owned());
    let header = bytes
        .get(..KTX2_HEADER_LEN)
        .ok_or_else(|| invalid("shorter than the header"))?;
    if !header.starts_with(&KTX2_SIGNATURE) {
        return Err(invalid("no KTX2 identifier"));
    }
    let u32_at = |at: usize| {
        u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    let (vk_format, width, height) = (u32_at(12), u32_at(20), u32_at(24));
    let (depth, layers, faces, scheme) = (u32_at(28), u32_at(32), u32_at(36), u32_at(44));
    let unsupported = |message: String| Err(ImageError::UnsupportedKtx2(message));
    let Some(&(_, texel)) = KTX2_FORMATS.iter().find(|(f, _)| *f == vk_format) else {
        return unsupported(format!(
            "vkFormat {vk_format} (only uncompressed 8-bit RGB and RGBA are decoded)"
        ));
    };
    if width == 0 || height == 0 {
        return Err(invalid("not a 2D texture (width or height is 0)"));
    }
    if depth != 0 || layers > 1 || faces != 1 {
        return unsupported(format!(
            "not a single 2D image (depth {depth}, {layers} layers, {faces} faces)"
        ));
    }
    if scheme != KTX2_SUPERCOMPRESSION_NONE && scheme != KTX2_SUPERCOMPRESSION_ZSTD {
        return unsupported(format!("supercompression scheme {scheme}"));
    }
    let max = limits.max_side;
    if width > max || height > max {
        return Err(ImageError::TooLarge { width, height, max });
    }
    // レベルの索引の最初 (レベル 0 = いちばん大きい段)。`levelCount` が 0 (読む側でミップを作る) でも 1 段はある。
    let level = bytes
        .get(KTX2_HEADER_LEN..KTX2_HEADER_LEN + KTX2_LEVEL_LEN)
        .ok_or_else(|| invalid("no level index"))?;
    let u64_at = |at: usize| {
        let mut b = [0; 8];
        b.copy_from_slice(&level[at..at + 8]);
        u64::from_le_bytes(b)
    };
    let (offset, length, uncompressed) = (u64_at(0), u64_at(8), u64_at(16));
    let data = usize::try_from(offset)
        .ok()
        .zip(usize::try_from(length).ok())
        .and_then(|(offset, length)| bytes.get(offset..offset.checked_add(length)?))
        .ok_or_else(|| invalid("level 0 is outside the file"))?;
    // 上限を大きくした 32bit の環境 (wasm32) では、画素のバイト数が usize に収まらないことがある。
    let size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(texel))
        .ok_or(ImageError::TooLarge { width, height, max })?;
    let expected = |actual: u64, what: &str| {
        if actual == size as u64 {
            Ok(())
        } else {
            Err(ImageError::InvalidKtx2(format!(
                "level 0 has {actual} {what} bytes, but a {width}x{height} image needs {size}"
            )))
        }
    };
    let decompressed;
    let data = if scheme == KTX2_SUPERCOMPRESSION_ZSTD {
        expected(uncompressed, "uncompressed")?;
        let decoder = ruzstd::decoding::StreamingDecoder::new(data)
            .map_err(|e| ImageError::InvalidKtx2(format!("zstd: {e}")))?;
        let mut out = Vec::with_capacity(size);
        decoder
            .take(size as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|e| ImageError::InvalidKtx2(format!("zstd: {e}")))?;
        expected(out.len() as u64, "decompressed")?;
        decompressed = out;
        decompressed.as_slice()
    } else {
        expected(length, "stored")?;
        data
    };
    let pixels = if texel == 4 {
        data.to_vec()
    } else {
        let mut pixels = Vec::with_capacity(size / 3 * 4);
        for p in data.as_chunks::<3>().0 {
            pixels.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        pixels
    };
    Ok(Rgba8 {
        width,
        height,
        pixels,
    })
}

/// KTX2 をデコードして、乗算済みアルファの RGBA8 にする (描画するテクスチャ用)。
///
/// 読む形式と上限は [`decode_ktx2`] と同じ。乗算は [`premultiply`]。
pub fn decode_ktx2_premultiplied(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    let mut image = decode_ktx2(bytes, limits)?;
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
            "image is 65535x65535, larger than the limit 16384x16384"
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

    const R8G8B8A8_UNORM: u32 = 37;
    const R8G8B8A8_SRGB: u32 = 43;
    const R8G8B8_SRGB: u32 = 29;
    const BC3_UNORM_BLOCK: u32 = 137;

    /// KTX2 の見出し (`vkFormat`・大きさ・`layerCount`・`faceCount`・超圧縮) とレベルの列 (レベル 0 から) で KTX2 を作る。
    /// データ形式の記述は付けない (デコードは見ない)。`zstd` ならレベルを zstd で圧縮する。
    struct Ktx2 {
        vk_format: u32,
        size: (u32, u32),
        layers: u32,
        faces: u32,
        zstd: bool,
        levels: Vec<Vec<u8>>,
    }

    impl Ktx2 {
        fn rgba8(vk_format: u32, size: (u32, u32), levels: Vec<Vec<u8>>) -> Self {
            Self {
                vk_format,
                size,
                layers: 0,
                faces: 1,
                zstd: false,
                levels,
            }
        }

        fn bytes(&self) -> Vec<u8> {
            let mut out = KTX2_SIGNATURE.to_vec();
            let scheme = if self.zstd { 2 } else { 0 };
            for v in [
                self.vk_format,
                1,
                self.size.0,
                self.size.1,
                0,
                self.layers,
                self.faces,
                self.levels.len() as u32,
                scheme,
            ] {
                out.extend(v.to_le_bytes());
            }
            out.extend([0; 32]); // DFD・KVD・SGD なし
            let stored: Vec<Vec<u8>> = self
                .levels
                .iter()
                .map(|l| {
                    if self.zstd {
                        ruzstd::encoding::compress_to_vec(
                            l.as_slice(),
                            ruzstd::encoding::CompressionLevel::Fastest,
                        )
                    } else {
                        l.clone()
                    }
                })
                .collect();
            // 小さい段から順にファイルに置く (KTX2 の決まり)。
            let mut offset = out.len() + self.levels.len() * KTX2_LEVEL_LEN;
            let mut offsets = vec![0; stored.len()];
            for (i, level) in stored.iter().enumerate().rev() {
                offsets[i] = offset;
                offset += level.len();
            }
            for (i, level) in stored.iter().enumerate() {
                out.extend((offsets[i] as u64).to_le_bytes());
                out.extend((level.len() as u64).to_le_bytes());
                out.extend((self.levels[i].len() as u64).to_le_bytes());
            }
            for level in stored.iter().rev() {
                out.extend(level);
            }
            out
        }
    }

    fn ktx2_straight(bytes: &[u8]) -> Result<Rgba8, ImageError> {
        decode_ktx2(bytes, ImageLimits::default())
    }

    /// 値は `_SRGB` でも `_UNORM` でも保存されたまま。アルファは乗算していないとみなし、求めれば乗算する。
    #[test]
    fn ktx2_rgba8_is_kept_as_stored_and_premultiplied_on_request() {
        let texels = vec![
            255, 0, 0, 255, 128, 200, 255, 0, 255, 255, 255, 128, 10, 20, 30, 40,
        ];
        for format in [R8G8B8A8_SRGB, R8G8B8A8_UNORM] {
            let bytes = Ktx2::rgba8(format, (2, 2), vec![texels.clone()]).bytes();
            let image = ktx2_straight(&bytes).unwrap();
            assert_eq!((image.width, image.height), (2, 2));
            assert_eq!(image.pixels, texels);
            let image = decode_ktx2_premultiplied(&bytes, ImageLimits::default()).unwrap();
            assert_eq!(
                image.pixels,
                vec![255, 0, 0, 255, 0, 0, 0, 0, 128, 128, 128, 128, 2, 3, 5, 40]
            );
        }
    }

    #[test]
    fn ktx2_rgb8_is_opaque() {
        let bytes = Ktx2::rgba8(R8G8B8_SRGB, (2, 1), vec![vec![1, 2, 3, 4, 5, 6]]).bytes();
        assert_eq!(
            ktx2_straight(&bytes).unwrap().pixels,
            vec![1, 2, 3, 255, 4, 5, 6, 255]
        );
    }

    /// ミップマップは最初の段 (いちばん大きいもの) だけを使う。
    #[test]
    fn ktx2_uses_only_the_base_level() {
        let base = vec![9; 4 * 2 * 2];
        let bytes = Ktx2::rgba8(
            R8G8B8A8_SRGB,
            (4, 2),
            vec![
                [base.clone(), base.clone()].concat(),
                vec![1; 8],
                vec![2; 4],
            ],
        )
        .bytes();
        let image = ktx2_straight(&bytes).unwrap();
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(image.pixels, vec![9; 4 * 4 * 2]);
    }

    #[test]
    fn ktx2_zstd_levels_are_decompressed() {
        let texels: Vec<u8> = (0..3 * 2 * 4).map(|i| i as u8).collect();
        let mut ktx2 = Ktx2::rgba8(R8G8B8A8_UNORM, (3, 2), vec![texels.clone(), vec![7; 4]]);
        ktx2.zstd = true;
        assert_eq!(ktx2_straight(&ktx2.bytes()).unwrap().pixels, texels);
    }

    /// 見出しの大きさを確かめてから確保する (PNG と同じ)。上限は呼び出し側が変えられる。
    #[test]
    fn ktx2_larger_than_the_limit_is_rejected_before_allocating() {
        let huge = Ktx2::rgba8(R8G8B8A8_SRGB, (65535, 65535), vec![vec![0; 4]]).bytes();
        let err = ktx2_straight(&huge).unwrap_err();
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
            "image is 65535x65535, larger than the limit 16384x16384"
        );
        let bytes = Ktx2::rgba8(R8G8B8A8_SRGB, (3, 2), vec![vec![0; 3 * 2 * 4]]).bytes();
        assert!(decode_ktx2(&bytes, ImageLimits::default().with_max_side(3)).is_ok());
        let err =
            decode_ktx2_premultiplied(&bytes, ImageLimits::default().with_max_side(2)).unwrap_err();
        assert!(
            matches!(
                err,
                ImageError::TooLarge {
                    width: 3,
                    height: 2,
                    max: 2
                }
            ),
            "{err}"
        );
    }

    /// 圧縮した形式・超圧縮 (Basis)・配列・キューブマップ・3D は、読めるが対応しない KTX2。
    #[test]
    fn ktx2_unsupported_kinds_are_reported_as_such() {
        let unsupported = |ktx2: Ktx2| {
            let err = ktx2_straight(&ktx2.bytes()).unwrap_err();
            assert!(matches!(err, ImageError::UnsupportedKtx2(_)), "{err}");
            err.to_string()
        };
        let bc3 = unsupported(Ktx2::rgba8(BC3_UNORM_BLOCK, (4, 4), vec![vec![0; 16]]));
        assert_eq!(
            bc3,
            "unsupported KTX2: vkFormat 137 (only uncompressed 8-bit RGB and RGBA are decoded)"
        );
        // Basis Universal (`vkFormat` が 0 で、超圧縮が BasisLZ)。
        unsupported(Ktx2::rgba8(0, (4, 4), vec![vec![0; 16]]));
        let mut array = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![0; 8]]);
        array.layers = 2;
        unsupported(array);
        let mut cube = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![0; 24]]);
        cube.faces = 6;
        unsupported(cube);
        let mut basis_lz = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![0; 4]]).bytes();
        basis_lz[44] = 1;
        assert!(unsupported_bytes(&basis_lz).contains("supercompression scheme 1"));
        let mut volume = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![0; 4]]).bytes();
        volume[28] = 2;
        unsupported_bytes(&volume);
    }

    fn unsupported_bytes(bytes: &[u8]) -> String {
        let err = ktx2_straight(bytes).unwrap_err();
        assert!(matches!(err, ImageError::UnsupportedKtx2(_)), "{err}");
        err.to_string()
    }

    #[test]
    fn broken_ktx2_is_an_error() {
        let invalid = |bytes: &[u8]| {
            let err = ktx2_straight(bytes).unwrap_err();
            assert!(matches!(err, ImageError::InvalidKtx2(_)), "{err}");
            err.to_string()
        };
        let good = Ktx2::rgba8(R8G8B8A8_SRGB, (2, 1), vec![vec![1; 8]]).bytes();
        assert!(ktx2_straight(&good).is_ok());
        // 識別子だけ・見出しの途中まで。
        invalid(&KTX2_SIGNATURE);
        invalid(&good[..KTX2_HEADER_LEN + 4]);
        invalid(
            b"not a ktx2 file, but long enough to hold a whole KTX2 header of eighty bytes.....",
        );
        // 画素の途中で切れている。
        invalid(&good[..good.len() - 1]);
        // レベル 0 の位置がファイルの外・長さの足し算があふれる。
        let mut outside = good.clone();
        outside[80..88].copy_from_slice(&u64::MAX.to_le_bytes());
        invalid(&outside);
        let mut overflow = good.clone();
        overflow[88..96].copy_from_slice(&u64::MAX.to_le_bytes());
        invalid(&overflow);
        // レベル 0 の大きさが画像と合わない。
        let short = Ktx2::rgba8(R8G8B8A8_SRGB, (2, 1), vec![vec![1; 4]]).bytes();
        assert_eq!(
            invalid(&short),
            "decoding KTX2: level 0 has 4 stored bytes, but a 2x1 image needs 8"
        );
        // 幅か高さが 0 (1D のテクスチャ)。
        invalid(&Ktx2::rgba8(R8G8B8A8_SRGB, (2, 0), vec![vec![1; 8]]).bytes());
        // zstd: 壊れたフレーム・展開した大きさが合わない。
        let mut zstd = Ktx2::rgba8(R8G8B8A8_SRGB, (2, 1), vec![vec![1; 8]]);
        zstd.zstd = true;
        let mut garbage = zstd.bytes();
        let end = garbage.len();
        garbage[end - 6..].fill(0xff);
        invalid(&garbage);
        let mut wrong = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![1; 8]]);
        wrong.zstd = true;
        invalid(&wrong.bytes());
    }

    #[test]
    fn decode_image_follows_the_format() {
        let png = rgba8(1, 1, &[200, 100, 50, 128]);
        let ktx2 = Ktx2::rgba8(R8G8B8A8_SRGB, (1, 1), vec![vec![200, 100, 50, 128]]).bytes();
        for (format, bytes) in [(TextureFormat::Png, &png), (TextureFormat::Ktx2, &ktx2)] {
            let limits = ImageLimits::default();
            assert_eq!(
                decode_image(format, bytes, limits).unwrap().pixels,
                vec![200, 100, 50, 128]
            );
            assert_eq!(
                decode_image_premultiplied(format, bytes, limits)
                    .unwrap()
                    .pixels,
                vec![100, 50, 25, 128]
            );
        }
        assert!(decode_image(TextureFormat::Png, &ktx2, ImageLimits::default()).is_err());
        assert!(decode_image(TextureFormat::Ktx2, &png, ImageLimits::default()).is_err());
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
