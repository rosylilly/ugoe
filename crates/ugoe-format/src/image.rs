//! テクスチャの PNG と KTX2 のデコード。
//!
//! バンドルの `TEXR` / `NRML` の PNG と KTX2 を、8bit の RGBA に正規化してデコードする。ヘッドレス描画 (`ugoe-headless`) と
//! WebAssembly (`ugoe-wasm`) が同じ関数を使い、画像の大きさの上限と色の展開の規則を 1 か所に保つ。
//!
//! - PNG のデコード (`decode_png`・`decode_png_premultiplied`・`decode_image`・`decode_image_premultiplied`) と
//!   KTX2 のブロック圧縮 (BCn) の展開は、feature `png` (`std` を含む) が要る。feature が無ければ、PNG の関数は無く、
//!   [`decode_ktx2`] は圧縮していない 8bit の RGB / RGBA (超圧縮は無しか zstd) だけを読み、BCn は
//!   [`ImageError::UnsupportedKtx2`] にする (`std` の無い環境では画像のデコードはもともとホストの役目。
//!   `docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md`)。
//!
//! - 色の値はリニア化しない (`docs/spec/rendering.md#前提`)。PNG は `gAMA`・`sRGB`・`iCCP`・`cHRM` を見ずに
//!   保存された値のまま使う。KTX2 も、形式の `_SRGB` や転送関数によらず保存された値のまま使う。
//! - グレースケール・パレット・16bit は 8bit RGBA に展開する (16bit は上位 8bit、`tRNS` はアルファ)。
//! - KTX2 は圧縮していない 8bit の RGB / RGBA と、ブロック圧縮の BC1〜BC5・BC7 ([`decode_ktx2`]) を読む。
//!   ETC2・ASTC・Basis Universal などは [`ImageError::UnsupportedKtx2`] にする (読む側が代用するか誤りにするかを決める。
//!   `docs/adr/0030-compressed-ktx2-decoding.md`)。
//! - 外から来る画像は信用しない。見出し (IHDR) の大きさだけで確保すると、小さなファイルで数 GB を求められるので、
//!   確保の前に一辺を [`ImageLimits::max_side`] と比べる (png の `Limits` はデコーダの中のバッファにしか効かない)。
//!   KTX2 も見出しの大きさを確保の前に比べる。

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
#[cfg(feature = "png")]
use alloc::{string::ToString, vec};
#[cfg(feature = "png")]
use std::io::Cursor;

use ruzstd::io::Read;

use crate::bundle::KTX2_SIGNATURE;
#[cfg(feature = "png")]
use crate::bundle::TextureFormat;

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
/// アルファが乗算済みかどうかは作った関数で決まる (`decode_png`・[`decode_ktx2`] はストレート、
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
    /// 読める KTX2 だが、デコードしない形式 (ETC2・ASTC・BC6H・Basis Universal など、8bit の RGB / RGBA と
    /// BC1〜BC5・BC7 以外・2D でないテクスチャなど)。中身は理由。
    #[error("unsupported KTX2: {0}")]
    UnsupportedKtx2(String),
}

/// 形式 (`format`) に合わせて画像をデコードし、アルファを掛けない RGBA8 にする (法線マップ用)。
///
/// feature `png` (`std` を含む) が要る。無ければ KTX2 は [`decode_ktx2`] で読む。
#[cfg(feature = "png")]
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
///
/// feature `png` (`std` を含む) が要る。無ければ KTX2 は [`decode_ktx2_premultiplied`] で読む。
#[cfg(feature = "png")]
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
/// 画素を確保する前に [`ImageError::TooLarge`]。feature `png` (`std` を含む) が要る。
#[cfg(feature = "png")]
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
/// 展開と上限は `decode_png` と同じ。乗算は [`premultiply`]。feature `png` (`std` を含む) が要る。
#[cfg(feature = "png")]
pub fn decode_png_premultiplied(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    let mut image = decode_png(bytes, limits)?;
    premultiply(&mut image.pixels);
    Ok(image)
}

/// KTX2 ファイルの先頭の識別子 (12 バイト)。
pub const KTX2_IDENTIFIER: [u8; 12] = KTX2_SIGNATURE;

/// KTX2 の画素の形式 (`vkFormat` から決める。[`KTX2_FORMATS`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ktx2Texel {
    /// 圧縮していない 8bit。中身は 1 画素のバイト数 (RGB は 3、RGBA は 4)。
    Uncompressed(usize),
    /// ブロック圧縮 (4x4 画素のブロック)。
    Block(BlockFormat),
}

/// デコードするブロック圧縮の形式 (`docs/adr/0030-compressed-ktx2-decoding.md`)。展開は bcdec_rs。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockFormat {
    /// BC1 のアルファなし。3 色のモードの 4 つ目も不透明の黒。
    Bc1Rgb,
    /// BC1 の 1bit のアルファ付き。3 色のモードの 4 つ目は透明の黒。
    Bc1Rgba,
    Bc2,
    Bc3,
    /// 1 チャンネル。`(R, 0, 0, 255)` にする (GPU が読むときと同じ)。
    Bc4,
    /// 2 チャンネル。`(R, G, 0, 255)` にする。
    Bc5,
    Bc7,
}

impl BlockFormat {
    /// 1 ブロック (4x4 画素) のバイト数。
    const fn block_bytes(self) -> usize {
        match self {
            Self::Bc1Rgb | Self::Bc1Rgba | Self::Bc4 => 8,
            Self::Bc2 | Self::Bc3 | Self::Bc5 | Self::Bc7 => 16,
        }
    }

    /// 1 ブロックを 4x4 の RGBA8 (行の順、1 行 16 バイト) に展開する。`block` は [`Self::block_bytes`] バイト。
    #[cfg(feature = "png")]
    fn decode(self, block: &[u8], out: &mut [u8; 64]) {
        const PITCH: usize = 16;
        match self {
            Self::Bc1Rgb | Self::Bc1Rgba => {
                bcdec_rs::bc1(block, out, PITCH);
                if self == Self::Bc1Rgb {
                    for p in out.as_chunks_mut::<4>().0 {
                        p[3] = 255;
                    }
                }
            }
            Self::Bc2 => bcdec_rs::bc2(block, out, PITCH),
            Self::Bc3 => bcdec_rs::bc3(block, out, PITCH),
            Self::Bc7 => bcdec_rs::bc7(block, out, PITCH),
            Self::Bc4 => {
                let mut r = [0u8; 16];
                bcdec_rs::bc4(block, &mut r, 4, false);
                for (p, r) in out.as_chunks_mut::<4>().0.iter_mut().zip(r) {
                    *p = [r, 0, 0, 255];
                }
            }
            Self::Bc5 => {
                let mut rg = [0u8; 32];
                bcdec_rs::bc5(block, &mut rg, 8, false);
                for (p, rg) in out
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(rg.as_chunks::<2>().0)
                {
                    *p = [rg[0], rg[1], 0, 255];
                }
            }
        }
    }
}

/// KTX2 の `vkFormat` のうち、デコードする形式。
///
/// `_SRGB` と `_UNORM` は同じに扱う (値を保存されたまま使う。`docs/spec/rendering.md#前提`)。
/// BC4 / BC5 の `_SNORM` と BC6H (HDR) は読まない。
const KTX2_FORMATS: [(u32, Ktx2Texel); 16] = [
    (23, Ktx2Texel::Uncompressed(3)), // VK_FORMAT_R8G8B8_UNORM
    (29, Ktx2Texel::Uncompressed(3)), // VK_FORMAT_R8G8B8_SRGB
    (37, Ktx2Texel::Uncompressed(4)), // VK_FORMAT_R8G8B8A8_UNORM
    (43, Ktx2Texel::Uncompressed(4)), // VK_FORMAT_R8G8B8A8_SRGB
    (131, Ktx2Texel::Block(BlockFormat::Bc1Rgb)), // VK_FORMAT_BC1_RGB_UNORM_BLOCK
    (132, Ktx2Texel::Block(BlockFormat::Bc1Rgb)), // VK_FORMAT_BC1_RGB_SRGB_BLOCK
    (133, Ktx2Texel::Block(BlockFormat::Bc1Rgba)), // VK_FORMAT_BC1_RGBA_UNORM_BLOCK
    (134, Ktx2Texel::Block(BlockFormat::Bc1Rgba)), // VK_FORMAT_BC1_RGBA_SRGB_BLOCK
    (135, Ktx2Texel::Block(BlockFormat::Bc2)), // VK_FORMAT_BC2_UNORM_BLOCK
    (136, Ktx2Texel::Block(BlockFormat::Bc2)), // VK_FORMAT_BC2_SRGB_BLOCK
    (137, Ktx2Texel::Block(BlockFormat::Bc3)), // VK_FORMAT_BC3_UNORM_BLOCK
    (138, Ktx2Texel::Block(BlockFormat::Bc3)), // VK_FORMAT_BC3_SRGB_BLOCK
    (139, Ktx2Texel::Block(BlockFormat::Bc4)), // VK_FORMAT_BC4_UNORM_BLOCK
    (141, Ktx2Texel::Block(BlockFormat::Bc5)), // VK_FORMAT_BC5_UNORM_BLOCK
    (145, Ktx2Texel::Block(BlockFormat::Bc7)), // VK_FORMAT_BC7_UNORM_BLOCK
    (146, Ktx2Texel::Block(BlockFormat::Bc7)), // VK_FORMAT_BC7_SRGB_BLOCK
];
/// KTX2 の見出し (識別子・形式・大きさ・索引) と、レベルの索引の 1 段の大きさ。
const KTX2_HEADER_LEN: usize = 80;
const KTX2_LEVEL_LEN: usize = 24;
/// `supercompressionScheme` の値。
const KTX2_SUPERCOMPRESSION_NONE: u32 = 0;
const KTX2_SUPERCOMPRESSION_ZSTD: u32 = 2;

/// KTX2 をデコードして、アルファを掛けない (ストレートの) RGBA8 にする。法線マップのようにアルファを使わない画像用。
///
/// 読むのは圧縮していない `VK_FORMAT_R8G8B8_*` / `VK_FORMAT_R8G8B8A8_*` と、ブロック圧縮の BC1〜BC5・BC7
/// (`_UNORM`・`_SRGB`。BC4 / BC5 は `_UNORM` だけ) の 2D テクスチャで、超圧縮は無しか zstd。ブロック圧縮は展開して
/// RGBA8 にする (BC4 は `(R, 0, 0, 255)`、BC5 は `(R, G, 0, 255)`。`docs/adr/0030-compressed-ktx2-decoding.md`)。値は形式や転送関数によらず保存されたまま使い、ミップマップは最初の段 (レベル 0) だけを読む。
/// データ形式の記述 (`KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を含む) は見ない (`docs/spec/rendering.md#前提`)。
///
/// - 一辺が `limits.max_side` を超える画像は、画素を確保する前に [`ImageError::TooLarge`]。
/// - ほかの形式 (ETC2・ASTC・BC6H・Basis Universal など)・配列・キューブマップ・3D は [`ImageError::UnsupportedKtx2`]。
/// - 見出しやレベルの範囲が壊れていれば [`ImageError::InvalidKtx2`]。
/// - ブロック圧縮の展開には feature `png` が要る。無ければ BCn も [`ImageError::UnsupportedKtx2`]。
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
            "vkFormat {vk_format} (only uncompressed 8-bit RGB and RGBA, BC1-BC5 and BC7 are decoded)"
        ));
    };
    #[cfg(not(feature = "png"))]
    if let Ktx2Texel::Block(_) = texel {
        return Err(block_compression_needs_png(vk_format));
    }
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
    let rgba_size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(ImageError::TooLarge { width, height, max })?;
    // レベル 0 の (超圧縮を外した) バイト数。ブロック圧縮は端の欠けたブロックも 1 つに数える。
    let size = match texel {
        Ktx2Texel::Uncompressed(bytes) => (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(bytes)),
        Ktx2Texel::Block(bc) => (width.div_ceil(4) as usize)
            .checked_mul(height.div_ceil(4) as usize)
            .and_then(|n| n.checked_mul(bc.block_bytes())),
    }
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
    let pixels = match texel {
        Ktx2Texel::Uncompressed(4) => data.to_vec(),
        Ktx2Texel::Uncompressed(_) => {
            let mut pixels = Vec::with_capacity(rgba_size);
            for p in data.as_chunks::<3>().0 {
                pixels.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
            pixels
        }
        #[cfg(feature = "png")]
        Ktx2Texel::Block(bc) => decode_blocks(bc, data, width as usize, height as usize, rgba_size),
        #[cfg(not(feature = "png"))]
        Ktx2Texel::Block(_) => return Err(block_compression_needs_png(vk_format)),
    };
    Ok(Rgba8 {
        width,
        height,
        pixels,
    })
}

/// feature `png` が無いときの、ブロック圧縮の KTX2 のエラー。
#[cfg(not(feature = "png"))]
fn block_compression_needs_png(vk_format: u32) -> ImageError {
    ImageError::UnsupportedKtx2(format!(
        "vkFormat {vk_format} (decoding block-compressed formats needs the `png` feature)"
    ))
}

/// ブロック圧縮の段 (`data` はブロックが行の順に `ceil(w / 4) * ceil(h / 4)` 個。呼ぶ側が長さを確かめている) を
/// RGBA8 (`rgba_size` = `w * h * 4` バイト) に展開する。右端と下端の欠けたブロックは、画像に入る画素だけを使う。
#[cfg(feature = "png")]
fn decode_blocks(
    bc: BlockFormat,
    data: &[u8],
    width: usize,
    height: usize,
    rgba_size: usize,
) -> Vec<u8> {
    let mut pixels = vec![0u8; rgba_size];
    let mut block = [0u8; 64];
    let blocks_x = width.div_ceil(4);
    for (i, compressed) in data.chunks_exact(bc.block_bytes()).enumerate() {
        bc.decode(compressed, &mut block);
        let (x, y) = ((i % blocks_x) * 4, (i / blocks_x) * 4);
        let columns = (width - x).min(4);
        for (row, line) in block
            .as_chunks::<16>()
            .0
            .iter()
            .take((height - y).min(4))
            .enumerate()
        {
            let at = ((y + row) * width + x) * 4;
            pixels[at..at + columns * 4].copy_from_slice(&line[..columns * 4]);
        }
    }
    pixels
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

#[cfg(feature = "png")]
fn decode_error(error: png::DecodingError) -> ImageError {
    ImageError::Decode(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "png")]
    fn encode(
        size: (u32, u32),
        color: png::ColorType,
        depth: png::BitDepth,
        palette: Option<&[u8]>,
        data: &[u8],
    ) -> Vec<u8> {
        encode_with(size, color, depth, data, |encoder| {
            if let Some(palette) = palette {
                encoder.set_palette(palette.to_vec());
            }
        })
    }

    /// `setup` で見出しの後のチャンク (`PLTE`・`tRNS`・`gAMA` など) を足して PNG を作る。
    #[cfg(feature = "png")]
    fn encode_with(
        (width, height): (u32, u32),
        color: png::ColorType,
        depth: png::BitDepth,
        data: &[u8],
        setup: impl FnOnce(&mut png::Encoder<'_, &mut Vec<u8>>),
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        setup(&mut encoder);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
        out
    }

    #[cfg(feature = "png")]
    fn rgba8(width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        encode(
            (width, height),
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            None,
            data,
        )
    }

    #[cfg(feature = "png")]
    fn straight(bytes: &[u8]) -> Vec<u8> {
        decode_png(bytes, ImageLimits::default()).unwrap().pixels
    }

    #[cfg(feature = "png")]
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

    #[cfg(feature = "png")]
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

    #[cfg(feature = "png")]
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

    #[cfg(feature = "png")]
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

    #[cfg(feature = "png")]
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

    /// パレットの `tRNS` はアルファになる。`tRNS` より後ろの番号は不透明 (`docs/spec/rendering.md#前提`)。
    #[cfg(feature = "png")]
    #[test]
    fn palette_transparency_becomes_alpha() {
        let bytes = encode_with(
            (3, 1),
            png::ColorType::Indexed,
            png::BitDepth::Eight,
            &[0, 1, 2],
            |encoder| {
                encoder.set_palette(vec![10, 20, 30, 40, 50, 60, 70, 80, 90]);
                encoder.set_trns(vec![0, 128]);
            },
        );
        assert_eq!(
            straight(&bytes),
            vec![10, 20, 30, 0, 40, 50, 60, 128, 70, 80, 90, 255]
        );
    }

    /// グレースケールと RGB の `tRNS` (透明にする 1 色) は、一致する画素のアルファを 0、ほかを 255 にする。
    #[cfg(feature = "png")]
    #[test]
    fn color_key_transparency_becomes_alpha() {
        let gray = encode_with(
            (2, 1),
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            &[200, 201],
            |encoder| encoder.set_trns(vec![0, 200]),
        );
        assert_eq!(straight(&gray), vec![200, 200, 200, 0, 201, 201, 201, 255]);
        let rgb = encode_with(
            (2, 1),
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &[1, 2, 3, 1, 2, 4],
            |encoder| encoder.set_trns(vec![0, 1, 0, 2, 0, 3]),
        );
        assert_eq!(straight(&rgb), vec![1, 2, 3, 0, 1, 2, 4, 255]);
    }

    /// 16bit の `tRNS` は、上位 8bit にする前の 16bit の値で比べる (下位のバイトだけが違う画素は不透明)。
    #[cfg(feature = "png")]
    #[test]
    fn sixteen_bit_color_key_is_compared_before_reduction() {
        let bytes = encode_with(
            (2, 1),
            png::ColorType::Rgb,
            png::BitDepth::Sixteen,
            &[
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, // 透明にする色と同じ
                0x12, 0x35, 0x56, 0x78, 0x9a, 0xbc, // R の下位だけが違う
            ],
            |encoder| encoder.set_trns(vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]),
        );
        assert_eq!(
            straight(&bytes),
            vec![0x12, 0x56, 0x9a, 0, 0x12, 0x56, 0x9a, 255]
        );
        let gray = encode(
            (1, 1),
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Sixteen,
            None,
            &[0xab, 0xff, 0x7f, 0xff],
        );
        assert_eq!(straight(&gray), vec![0xab, 0xab, 0xab, 0x7f]);
    }

    /// 8bit 未満のグレースケールは `v × 255 / (2^depth − 1)` に広げる。パレットの番号は広げずに引く。
    #[cfg(feature = "png")]
    #[test]
    fn low_bit_depths_are_scaled_or_looked_up() {
        let two = encode(
            (4, 1),
            png::ColorType::Grayscale,
            png::BitDepth::Two,
            None,
            &[0b00_01_10_11],
        );
        assert_eq!(
            straight(&two),
            vec![
                0, 0, 0, 255, 85, 85, 85, 255, 170, 170, 170, 255, 255, 255, 255, 255
            ]
        );
        let four = encode(
            (2, 1),
            png::ColorType::Grayscale,
            png::BitDepth::Four,
            None,
            &[0x1f],
        );
        assert_eq!(straight(&four), vec![17, 17, 17, 255, 255, 255, 255, 255]);
        let palette = encode(
            (2, 1),
            png::ColorType::Indexed,
            png::BitDepth::Four,
            Some(&[10, 20, 30, 40, 50, 60]),
            &[0x10],
        );
        assert_eq!(straight(&palette), vec![40, 50, 60, 255, 10, 20, 30, 255]);
    }

    /// zlib の無圧縮のブロック 1 つで `data` を包む (テストで PNG を手で組むため)。
    #[cfg(feature = "png")]
    fn zlib_stored(data: &[u8]) -> Vec<u8> {
        let len = u16::try_from(data.len()).unwrap();
        let mut out = vec![0x78, 0x01, 0x01];
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(data);
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in data {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        out.extend_from_slice(&((b << 16) | a).to_be_bytes());
        out
    }

    /// チャンクの型と中身から、長さと CRC を付けたチャンクを作る。
    #[cfg(feature = "png")]
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut crc = !0u32;
        for &byte in kind.iter().chain(data) {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&(!crc).to_be_bytes());
        out
    }

    /// `IHDR` の後に `extra` のチャンクを挟んだ、8bit RGBA の PNG を手で組む。
    #[cfg(feature = "png")]
    fn handmade_png(
        (width, height): (u32, u32),
        interlace: u8,
        extra: &[Vec<u8>],
        raw: &[u8],
    ) -> Vec<u8> {
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut ihdr = width.to_be_bytes().to_vec();
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, interlace]);
        out.extend(chunk(b"IHDR", &ihdr));
        for c in extra {
            out.extend_from_slice(c);
        }
        out.extend(chunk(b"IDAT", &zlib_stored(raw)));
        out.extend(chunk(b"IEND", &[]));
        out
    }

    /// `gAMA`・`sRGB`・`cHRM`・`iCCP` は値を変えない (保存された値のまま。`docs/spec/rendering.md#前提`)。
    #[cfg(feature = "png")]
    #[test]
    fn gamma_and_color_chunks_are_ignored() {
        let raw = [0, 10, 128, 250, 200];
        let plain = handmade_png((1, 1), 0, &[], &raw);
        let mut iccp = b"profile\0\0".to_vec();
        iccp.extend(zlib_stored(&[0; 16]));
        let chunks = [
            // ガンマ 1.0 と 0.2 (sRGB の値として読むなら大きく変わる)。
            chunk(b"gAMA", &100_000u32.to_be_bytes()),
            chunk(b"gAMA", &20_000u32.to_be_bytes()),
            chunk(b"sRGB", &[0]),
            chunk(b"cHRM", &[0x42; 32]),
            chunk(b"iCCP", &iccp),
        ];
        assert_eq!(straight(&plain), vec![10, 128, 250, 200]);
        for c in &chunks {
            let tagged = handmade_png((1, 1), 0, std::slice::from_ref(c), &raw);
            assert_eq!(straight(&tagged), straight(&plain));
        }
        let all = handmade_png((1, 1), 0, &chunks[1..], &raw);
        assert_eq!(straight(&all), straight(&plain));
    }

    /// Adam7 のインターレースは、行の順の画像に戻す。
    #[cfg(feature = "png")]
    #[test]
    fn interlaced_png_is_deinterlaced() {
        // 2x2 の Adam7: 1 回目が (0, 0)、6 回目が (1, 0)、7 回目が 2 行目。各行は先頭にフィルタの種類 (0)。
        let raw = [
            0, 1, 1, 1, 255, // (0, 0)
            0, 2, 2, 2, 255, // (1, 0)
            0, 3, 3, 3, 255, 4, 4, 4, 255, // (0, 1)・(1, 1)
        ];
        let bytes = handmade_png((2, 2), 1, &[], &raw);
        assert_eq!(
            straight(&bytes),
            vec![1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 255]
        );
    }

    /// APNG は既定の画像 (`IDAT`) だけを読む。アニメーションの枠 (`fdAT`) は読まない。
    #[cfg(feature = "png")]
    #[test]
    fn apng_uses_only_the_default_image() {
        for separate_default in [false, true] {
            let mut out = Vec::new();
            let mut encoder = png::Encoder::new(&mut out, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(2, 0).unwrap();
            encoder.set_sep_def_img(separate_default).unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1, 2, 3, 4]).unwrap();
            writer.write_image_data(&[5, 6, 7, 8]).unwrap();
            if separate_default {
                writer.write_image_data(&[9, 10, 11, 12]).unwrap();
            }
            writer.finish().unwrap();
            assert_eq!(straight(&out), vec![1, 2, 3, 4], "{separate_default}");
        }
    }

    /// IHDR が 65535x65535 で、画素のデータが zlib の見出しだけの小さな PNG。
    #[cfg(feature = "png")]
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
    #[cfg(feature = "png")]
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
    #[cfg(feature = "png")]
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

    #[cfg(feature = "png")]
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
    const BC1_RGB_UNORM_BLOCK: u32 = 131;
    const BC1_RGBA_SRGB_BLOCK: u32 = 134;
    const BC2_UNORM_BLOCK: u32 = 135;
    const BC3_UNORM_BLOCK: u32 = 137;
    const BC3_SRGB_BLOCK: u32 = 138;
    const BC4_UNORM_BLOCK: u32 = 139;
    const BC4_SNORM_BLOCK: u32 = 140;
    const BC5_UNORM_BLOCK: u32 = 141;
    const BC6H_UFLOAT_BLOCK: u32 = 143;
    const BC7_UNORM_BLOCK: u32 = 145;
    const BC7_SRGB_BLOCK: u32 = 146;
    const ETC2_R8G8B8A8_UNORM_BLOCK: u32 = 151;

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

    /// 読まない圧縮形式 (ETC2・BC6H・BC4 の SNORM)・超圧縮 (Basis)・配列・キューブマップ・3D は、読めるが対応しない KTX2。
    #[test]
    fn ktx2_unsupported_kinds_are_reported_as_such() {
        let unsupported = |ktx2: Ktx2| {
            let err = ktx2_straight(&ktx2.bytes()).unwrap_err();
            assert!(matches!(err, ImageError::UnsupportedKtx2(_)), "{err}");
            err.to_string()
        };
        let etc2 = unsupported(Ktx2::rgba8(
            ETC2_R8G8B8A8_UNORM_BLOCK,
            (4, 4),
            vec![vec![0; 16]],
        ));
        assert_eq!(
            etc2,
            "unsupported KTX2: vkFormat 151 (only uncompressed 8-bit RGB and RGBA, BC1-BC5 and BC7 are decoded)"
        );
        unsupported(Ktx2::rgba8(BC6H_UFLOAT_BLOCK, (4, 4), vec![vec![0; 16]]));
        unsupported(Ktx2::rgba8(BC4_SNORM_BLOCK, (4, 4), vec![vec![0; 8]]));
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

    /// feature `png` が無ければ、ブロック圧縮は展開せずに対応しない形式として報告する。
    #[cfg(not(feature = "png"))]
    #[test]
    fn ktx2_block_compression_needs_the_png_feature() {
        for (format, bytes) in [
            (BC1_RGB_UNORM_BLOCK, 8),
            (BC1_RGBA_SRGB_BLOCK, 8),
            (BC2_UNORM_BLOCK, 16),
            (BC3_UNORM_BLOCK, 16),
            (BC3_SRGB_BLOCK, 16),
            (BC4_UNORM_BLOCK, 8),
            (BC5_UNORM_BLOCK, 16),
            (BC7_UNORM_BLOCK, 16),
            (BC7_SRGB_BLOCK, 16),
        ] {
            let ktx2 = Ktx2::rgba8(format, (4, 4), vec![vec![0; bytes]]).bytes();
            let message = unsupported_bytes(&ktx2);
            assert!(message.contains("`png` feature"), "{message}");
        }
    }

    /// RGB565 の色。
    #[cfg(feature = "png")]
    const RED: u16 = 0xf800;
    #[cfg(feature = "png")]
    const BLUE: u16 = 0x001f;

    /// BC1 の色のブロック (端点 2 つと、画素ごとの 2bit の添字。添字は左上から行の順)。
    #[cfg(feature = "png")]
    fn bc1_block(c0: u16, c1: u16, indices: [u8; 16]) -> Vec<u8> {
        let bits = indices
            .iter()
            .enumerate()
            .fold(0u32, |bits, (i, &index)| bits | u32::from(index) << (2 * i));
        [c0.to_le_bytes(), c1.to_le_bytes()]
            .concat()
            .into_iter()
            .chain(bits.to_le_bytes())
            .collect()
    }

    /// BC3 / BC4 のアルファ (1 チャンネル) のブロック。添字はすべて 0 (値は `a0`)。
    #[cfg(feature = "png")]
    fn bc4_block(a0: u8, a1: u8) -> Vec<u8> {
        vec![a0, a1, 0, 0, 0, 0, 0, 0]
    }

    /// BC7 のモード 6 の、全画素が同じ色のブロック。端点はどちらも `rgba` (7bit) で、p ビットは 1
    /// (8bit の値は `(c << 1) | 1`)。添字はすべて 0。
    #[cfg(feature = "png")]
    fn bc7_solid(rgba: [u8; 4]) -> Vec<u8> {
        let mut bits: u128 = 1 << 6; // モード 6
        let mut at = 7;
        for c in rgba {
            for _ in 0..2 {
                bits |= u128::from(c & 0x7f) << at;
                at += 7;
            }
        }
        bits |= 0b11 << at; // p ビット
        bits.to_le_bytes().to_vec()
    }

    #[cfg(feature = "png")]
    fn pixel(image: &Rgba8, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * image.width + x) * 4) as usize;
        image.pixels[at..at + 4].try_into().unwrap()
    }

    /// BC1: 4 色のモードは端点の色、3 色のモードの 4 つ目は、アルファ付きの形式なら透明の黒、アルファなしなら不透明の黒。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_bc1_is_decoded() {
        let mut indices = [0; 16];
        indices[1] = 1;
        let four = bc1_block(RED, BLUE, indices);
        let image =
            ktx2_straight(&Ktx2::rgba8(BC1_RGB_UNORM_BLOCK, (4, 4), vec![four]).bytes()).unwrap();
        assert_eq!((image.width, image.height), (4, 4));
        assert_eq!(pixel(&image, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 1, 0), [0, 0, 255, 255]);
        let three = bc1_block(BLUE, RED, [3; 16]);
        for (format, alpha) in [(BC1_RGB_UNORM_BLOCK, 255), (BC1_RGBA_SRGB_BLOCK, 0)] {
            let image =
                ktx2_straight(&Ktx2::rgba8(format, (4, 4), vec![three.clone()]).bytes()).unwrap();
            assert_eq!(image.pixels, [0, 0, 0, alpha].repeat(16), "{format}");
        }
    }

    /// BC3 の色のブロックは、端点の大小によらず 4 色のモードで読む (Direct3D 10 以降の決まり。
    /// `docs/adr/0030-compressed-ktx2-decoding.md`)。`color0 == color1` で添字 3 は端点の色で、黒ではない。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_bc3_colors_always_use_four_color_mode() {
        let block = [bc4_block(128, 0), bc1_block(RED, RED, [3; 16])].concat();
        for format in [BC3_UNORM_BLOCK, BC3_SRGB_BLOCK] {
            let bytes = Ktx2::rgba8(format, (4, 4), vec![block.clone()]).bytes();
            assert_eq!(
                ktx2_straight(&bytes).unwrap().pixels,
                [255, 0, 0, 128].repeat(16)
            );
            // 乗算は展開した後 (`docs/spec/rendering.md#前提`)。
            assert_eq!(
                decode_ktx2_premultiplied(&bytes, ImageLimits::default())
                    .unwrap()
                    .pixels,
                [128, 0, 0, 128].repeat(16)
            );
        }
        // BC2 のアルファは 4bit をそのまま広げる (0x8 → 0x88)。
        let bc2 = [vec![0x88; 8], bc1_block(BLUE, BLUE, [0; 16])].concat();
        assert_eq!(
            ktx2_straight(&Ktx2::rgba8(BC2_UNORM_BLOCK, (4, 4), vec![bc2]).bytes())
                .unwrap()
                .pixels,
            [0, 0, 255, 0x88].repeat(16)
        );
    }

    /// BC4 は `(R, 0, 0, 255)`、BC5 は `(R, G, 0, 255)`、BC7 は RGBA。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_bc4_bc5_and_bc7_are_decoded() {
        let decode = |format: u32, block: Vec<u8>| {
            ktx2_straight(&Ktx2::rgba8(format, (4, 4), vec![block]).bytes())
                .unwrap()
                .pixels
        };
        assert_eq!(
            decode(BC4_UNORM_BLOCK, bc4_block(200, 0)),
            [200, 0, 0, 255].repeat(16)
        );
        assert_eq!(
            decode(
                BC5_UNORM_BLOCK,
                [bc4_block(10, 0), bc4_block(20, 0)].concat()
            ),
            [10, 20, 0, 255].repeat(16)
        );
        for format in [BC7_UNORM_BLOCK, BC7_SRGB_BLOCK] {
            assert_eq!(
                decode(format, bc7_solid([100, 50, 0, 63])),
                [201, 101, 1, 127].repeat(16)
            );
        }
    }

    /// 幅・高さが 4 の倍数でなければ、端のブロックの画像に入る画素だけを使う。ミップマップは最初の段だけ、zstd も外す。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_partial_blocks_levels_and_zstd() {
        // 5x3 は 2x1 ブロック。左のブロックは赤、右は青。
        let level0 = [bc1_block(RED, RED, [0; 16]), bc1_block(BLUE, BLUE, [0; 16])].concat();
        let level1 = bc1_block(0xffff, 0xffff, [0; 16]); // 2x1 (白)
        let level2 = bc1_block(0xffff, 0xffff, [0; 16]); // 1x1
        for zstd in [false, true] {
            let mut ktx2 = Ktx2::rgba8(
                BC1_RGB_UNORM_BLOCK,
                (5, 3),
                vec![level0.clone(), level1.clone(), level2.clone()],
            );
            ktx2.zstd = zstd;
            let image = ktx2_straight(&ktx2.bytes()).unwrap();
            assert_eq!((image.width, image.height), (5, 3));
            assert_eq!(image.pixels.len(), 5 * 3 * 4);
            for y in 0..3 {
                for x in 0..4 {
                    assert_eq!(pixel(&image, x, y), [255, 0, 0, 255]);
                }
                assert_eq!(pixel(&image, 4, y), [0, 0, 255, 255]);
            }
        }
    }

    /// ブロック圧縮の段の大きさは、ブロックの数とブロックのバイト数の積と合わなければ壊れている。上限は展開する前に確かめる。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_block_level_size_must_match() {
        let short = Ktx2::rgba8(BC3_UNORM_BLOCK, (4, 4), vec![vec![0; 8]]).bytes();
        let err = ktx2_straight(&short).unwrap_err();
        assert_eq!(
            err.to_string(),
            "decoding KTX2: level 0 has 8 stored bytes, but a 4x4 image needs 16"
        );
        // 5x5 は 2x2 ブロック (BC7 で 64 バイト)。
        let bytes = Ktx2::rgba8(BC7_UNORM_BLOCK, (5, 5), vec![vec![0; 48]]).bytes();
        assert!(matches!(
            ktx2_straight(&bytes).unwrap_err(),
            ImageError::InvalidKtx2(_)
        ));
        let huge = Ktx2::rgba8(BC1_RGB_UNORM_BLOCK, (65535, 65535), vec![vec![0; 8]]).bytes();
        assert!(matches!(
            ktx2_straight(&huge).unwrap_err(),
            ImageError::TooLarge { .. }
        ));
    }

    /// 乱れたブロックでもパニックせず、大きさどおりの画素を返す (外から来るデータ)。
    #[cfg(feature = "png")]
    #[test]
    fn ktx2_arbitrary_blocks_do_not_panic() {
        let mut seed = 0x2545_f491_u32;
        let mut noise = |n: usize| {
            (0..n)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    seed as u8
                })
                .collect::<Vec<u8>>()
        };
        for (format, block) in [
            (BC1_RGB_UNORM_BLOCK, 8),
            (BC1_RGBA_SRGB_BLOCK, 8),
            (BC2_UNORM_BLOCK, 16),
            (BC3_UNORM_BLOCK, 16),
            (BC4_UNORM_BLOCK, 8),
            (BC5_UNORM_BLOCK, 16),
            (BC7_UNORM_BLOCK, 16),
        ] {
            let bytes = Ktx2::rgba8(format, (7, 9), vec![noise(2 * 3 * block)]).bytes();
            assert_eq!(ktx2_straight(&bytes).unwrap().pixels.len(), 7 * 9 * 4);
        }
    }

    #[cfg(feature = "png")]
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
