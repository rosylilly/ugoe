//! カメラの API が求める画素の形 (NV12・YUY2) と、RGBA からの変換 (CPU の参照実装)。
//!
//! 常駐の描画 ([`crate::Resident`]) は同じ変換を GPU の計算シェーダ (`yuv.wgsl`) で行う。ここの関数は、
//! 計算シェーダを使えないアダプタでの代わりと、GPU の結果を確かめるテストの参照に使う。
//!
//! 色の変換は BT.709 の限定範囲 (Y は 16〜235、Cb・Cr は 16〜240)。描画先の値 (sRGB で符号化されたまま) を
//! そのまま R'G'B' とみなす。色差は隣り合う画素の R'G'B' を平均してから変換する (NV12 は 2×2、YUY2 は横の 2 画素)。
//! アルファは見ない (仮想カメラは不透明な背景の上に描くので、アルファはいつも 255 になる)。

use crate::image::Rgba8;
use crate::{Error, bail};

/// 読み戻す画素の形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// 乗算済みアルファの RGBA8 (行の詰め物なし)。
    Rgba8,
    /// Y の面 (幅 × 高さ) の後に、Cb と Cr を交互に並べた面 (幅 × 高さ / 2)。Windows の仮想カメラが使う。
    Nv12,
    /// 横の 2 画素ごとに Y0 Cb Y1 Cr。v4l2loopback (Linux) が使う (V4L2 の `YUYV`)。
    Yuy2,
}

impl PixelFormat {
    /// 1 枚のバイト数。
    pub fn frame_bytes(self, width: u32, height: u32) -> usize {
        let pixels = width as usize * height as usize;
        match self {
            Self::Rgba8 => pixels * 4,
            Self::Nv12 => pixels * 3 / 2,
            Self::Yuy2 => pixels * 2,
        }
    }

    /// この形で使える大きさか。YUV の形は幅を 4 の倍数、高さを 2 の倍数にする
    /// (計算シェーダが 4 バイトずつ書くため。1920×1080・1280×720 は満たす)。
    pub fn check_size(self, width: u32, height: u32) -> Result<(), Error> {
        if width == 0 || height == 0 {
            bail!("frame size {width}x{height} has no area");
        }
        if self != Self::Rgba8 && (!width.is_multiple_of(4) || !height.is_multiple_of(2)) {
            bail!(
                "{self:?} frames need a width divisible by 4 and an even height, got {width}x{height}"
            );
        }
        Ok(())
    }
}

/// BT.709 の限定範囲の Y (R'G'B' は 0〜1)。
fn luma(r: f32, g: f32, b: f32) -> f32 {
    16.0 + 219.0 * (0.2126 * r + 0.7152 * g + 0.0722 * b)
}

/// BT.709 の限定範囲の Cb と Cr (R'G'B' は 0〜1)。
fn chroma(r: f32, g: f32, b: f32) -> (f32, f32) {
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    (
        128.0 + 224.0 * (b - y) / 1.8556,
        128.0 + 224.0 * (r - y) / 1.5748,
    )
}

/// 0〜255 に丸める (`yuv.wgsl` の `to_byte` と同じ)。
fn to_byte(v: f32) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

fn rgb(image: &Rgba8, x: u32, y: u32) -> [f32; 3] {
    let i = (y as usize * image.width as usize + x as usize) * 4;
    let p = &image.pixels[i..i + 3];
    [p[0], p[1], p[2]].map(|c| f32::from(c) / 255.0)
}

fn average(pixels: &[[f32; 3]]) -> [f32; 3] {
    let n = pixels.len() as f32;
    let sum = pixels
        .iter()
        .fold([0.0; 3], |a, p| [a[0] + p[0], a[1] + p[1], a[2] + p[2]]);
    sum.map(|c| c / n)
}

/// RGBA8 を `format` に変換する。`Rgba8` ならそのまま写す。
pub fn convert(image: &Rgba8, format: PixelFormat) -> Result<Vec<u8>, Error> {
    let (w, h) = (image.width, image.height);
    format.check_size(w, h)?;
    let mut out = Vec::with_capacity(format.frame_bytes(w, h));
    match format {
        PixelFormat::Rgba8 => out.extend_from_slice(&image.pixels),
        PixelFormat::Nv12 => {
            for y in 0..h {
                for x in 0..w {
                    let [r, g, b] = rgb(image, x, y);
                    out.push(to_byte(luma(r, g, b)));
                }
            }
            for y in (0..h).step_by(2) {
                for x in (0..w).step_by(2) {
                    let [r, g, b] = average(&[
                        rgb(image, x, y),
                        rgb(image, x + 1, y),
                        rgb(image, x, y + 1),
                        rgb(image, x + 1, y + 1),
                    ]);
                    let (cb, cr) = chroma(r, g, b);
                    out.extend([to_byte(cb), to_byte(cr)]);
                }
            }
        }
        PixelFormat::Yuy2 => {
            for y in 0..h {
                for x in (0..w).step_by(2) {
                    let p0 = rgb(image, x, y);
                    let p1 = rgb(image, x + 1, y);
                    let [r, g, b] = average(&[p0, p1]);
                    let (cb, cr) = chroma(r, g, b);
                    out.extend([
                        to_byte(luma(p0[0], p0[1], p0[2])),
                        to_byte(cb),
                        to_byte(luma(p1[0], p1[1], p1[2])),
                        to_byte(cr),
                    ]);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Rgba8 {
        Rgba8 {
            width: w,
            height: h,
            pixels: rgba.repeat(w as usize * h as usize),
        }
    }

    #[test]
    fn sizes_and_limits() {
        assert_eq!(PixelFormat::Nv12.frame_bytes(1920, 1080), 3_110_400);
        assert_eq!(PixelFormat::Yuy2.frame_bytes(1280, 720), 1_843_200);
        assert_eq!(PixelFormat::Rgba8.frame_bytes(2, 3), 24);
        assert!(PixelFormat::Nv12.check_size(1920, 1080).is_ok());
        assert!(PixelFormat::Nv12.check_size(6, 4).is_err());
        assert!(PixelFormat::Yuy2.check_size(8, 3).is_err());
        assert!(PixelFormat::Rgba8.check_size(3, 3).is_ok());
        assert!(PixelFormat::Rgba8.check_size(0, 3).is_err());
    }

    #[test]
    fn black_white_and_primaries_use_the_limited_range() {
        let black = convert(&solid(4, 2, [0, 0, 0, 255]), PixelFormat::Nv12).unwrap();
        assert_eq!(&black[..8], &[16; 8]);
        assert_eq!(&black[8..], &[128, 128, 128, 128]);
        let white = convert(&solid(4, 2, [255; 4]), PixelFormat::Yuy2).unwrap();
        assert_eq!(&white[..4], &[235, 128, 235, 128]);
        // BT.709 の赤: Y = 63、Cb = 102、Cr = 240。
        let red = convert(&solid(4, 2, [255, 0, 0, 255]), PixelFormat::Nv12).unwrap();
        assert_eq!((red[0], red[8], red[9]), (63, 102, 240));
    }

    #[test]
    fn chroma_averages_neighbours() {
        // 左の列が黒、右の列が白: NV12 の色差は灰色 (128)、Y は画素ごと。
        let mut image = solid(4, 2, [0, 0, 0, 255]);
        for y in 0..2 {
            for x in [1, 3] {
                let i = (y * 4 + x) * 4;
                image.pixels[i..i + 3].copy_from_slice(&[255; 3]);
            }
        }
        let nv12 = convert(&image, PixelFormat::Nv12).unwrap();
        assert_eq!(&nv12[..4], &[16, 235, 16, 235]);
        assert_eq!(&nv12[8..], &[128; 4]);
        let yuy2 = convert(&image, PixelFormat::Yuy2).unwrap();
        assert_eq!(&yuy2[..8], &[16, 128, 235, 128, 16, 128, 235, 128]);
    }
}
