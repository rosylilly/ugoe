//! 適合テストと単体テストのための、簡単なブロック圧縮 (BC1・BC3・BC7) の書き出し ([`crate::image::encode_ktx2`])。
//!
//! 画質は求めない (読む側の展開を比べるためのもの)。端点はブロックの画素の各チャンネルの最小と最大で、添字は
//! いちばん近い色を選ぶ。BC1 は 4 色のモードのほかに、すべてのブロックを 3 色のモード (4 つ目の添字は黒) で書ける。BC7 はモード 6 (1 つの区画・RGBA の 7bit の端点と p ビット・4bit の添字) だけを使う。
//! ブロックの形式は Microsoft の Direct3D 11 の文書と Khronos Data Format Specification の S3TC・BPTC の節に従う
//! (`docs/adr/0030-compressed-ktx2-decoding.md`)。

use crate::Rgba8;

/// 書くブロック圧縮の形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockFormat {
    /// BC1 (アルファなし。常に 4 色のモード)。
    Bc1,
    /// BC1 (アルファなし。常に 3 色のモード: `color0 <= color1`、添字 3 は黒)。
    Bc1ThreeColor,
    /// BC3 (BC1 の色と、8 段のアルファ)。
    Bc3,
    /// BC7 のモード 6。
    Bc7,
}

impl BlockFormat {
    /// 1 ブロック (4x4 画素) のバイト数。
    pub(crate) const fn block_bytes(self) -> usize {
        match self {
            Self::Bc1 | Self::Bc1ThreeColor => 8,
            Self::Bc3 | Self::Bc7 => 16,
        }
    }
}

/// `image` (ストレートアルファの RGBA8) をブロックに分けて圧縮する。ブロックは行の順。
/// 右端と下端の欠けたブロックは、画像の端の画素を繰り返して埋める。
pub(crate) fn encode(image: &Rgba8, format: BlockFormat) -> Vec<u8> {
    let (width, height) = (image.width as usize, image.height as usize);
    let mut out = Vec::with_capacity(width.div_ceil(4) * height.div_ceil(4) * format.block_bytes());
    for by in 0..height.div_ceil(4) {
        for bx in 0..width.div_ceil(4) {
            let mut block = [[0u8; 4]; 16];
            for (i, texel) in block.iter_mut().enumerate() {
                let x = (bx * 4 + i % 4).min(width - 1);
                let y = (by * 4 + i / 4).min(height - 1);
                let at = (y * width + x) * 4;
                texel.copy_from_slice(&image.pixels[at..at + 4]);
            }
            match format {
                BlockFormat::Bc1 => out.extend(color_block(&block)),
                BlockFormat::Bc1ThreeColor => out.extend(three_color_block(&block)),
                BlockFormat::Bc3 => {
                    out.extend(alpha_block(&block));
                    out.extend(color_block(&block));
                }
                BlockFormat::Bc7 => out.extend(bc7_mode6(&block)),
            }
        }
    }
    out
}

/// ブロックの最初の `N` チャンネルの端点 (`low` から `high` への線分に画素を当てる)。
///
/// 各チャンネルの最小と最大を取り、値の幅がいちばん大きいチャンネルと逆向きに変わるチャンネル (共分散が負) は
/// 最小と最大を入れ替える (色とアルファが逆に変わるグラデーションでも線分が画素に沿うように)。
fn endpoints<const N: usize>(block: &[[u8; 4]; 16]) -> ([u8; N], [u8; N]) {
    let mut low = [255u8; N];
    let mut high = [0u8; N];
    for texel in block {
        for c in 0..N {
            low[c] = low[c].min(texel[c]);
            high[c] = high[c].max(texel[c]);
        }
    }
    let main = (0..N).max_by_key(|&c| high[c] - low[c]).unwrap_or(0);
    let mean = |c: usize| block.iter().map(|t| i32::from(t[c])).sum::<i32>() / 16;
    let main_mean = mean(main);
    for c in 0..N {
        let m = mean(c);
        let covariance: i32 = block
            .iter()
            .map(|t| (i32::from(t[main]) - main_mean) * (i32::from(t[c]) - m))
            .sum();
        if covariance < 0 {
            std::mem::swap(&mut low[c], &mut high[c]);
        }
    }
    (low, high)
}

/// 8bit の RGB を RGB565 にする (切り捨て)。
fn to_565(c: [u8; 3]) -> u16 {
    (u16::from(c[0] >> 3) << 11) | (u16::from(c[1] >> 2) << 5) | u16::from(c[2] >> 3)
}

/// RGB565 を 8bit の RGB に広げる (上位のビットを下に繰り返す)。
fn from_565(c: u16) -> [i32; 3] {
    let (r, g, b) = (
        i32::from(c >> 11),
        i32::from((c >> 5) & 63),
        i32::from(c & 31),
    );
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    ]
}

fn distance(a: [i32; 3], b: [u8; 4]) -> i32 {
    (0..3).map(|c| (a[c] - i32::from(b[c])).pow(2)).sum()
}

/// BC1 / BC3 の色のブロック (8 バイト)。`color0 > color1` の 4 色のモードで書く (同じなら添字はすべて 0)。
fn color_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let (low, high) = endpoints::<3>(block);
    let (mut c0, mut c1) = (to_565(high), to_565(low));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let mut indices = 0u32;
    if c0 != c1 {
        let (e0, e1) = (from_565(c0), from_565(c1));
        let palette = [
            e0,
            e1,
            [0, 1, 2].map(|c| (2 * e0[c] + e1[c]) / 3),
            [0, 1, 2].map(|c| (e0[c] + 2 * e1[c]) / 3),
        ];
        for (i, texel) in block.iter().enumerate() {
            let best = (0..4)
                .min_by_key(|&k| distance(palette[k], *texel))
                .unwrap_or(0);
            indices |= (best as u32) << (2 * i);
        }
    }
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..].copy_from_slice(&indices.to_le_bytes());
    out
}

/// BC1 の 3 色のモードの色のブロック (8 バイト)。`color0 <= color1` で書き、色は端点・その中点・黒の 4 つ
/// (添字 3 の黒は、アルファのある BC1 では透明、アルファのない BC1 では不透明。Khronos Data Format Specification の S3TC の節)。
/// 端点は黒を除いた画素から取る (黒い画素は添字 3 で表す)。
fn three_color_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let colored: Vec<[u8; 4]> = block
        .iter()
        .copied()
        .filter(|t| t[..3] != [0, 0, 0])
        .collect();
    let (low, high) = if colored.is_empty() {
        ([0; 3], [0; 3])
    } else {
        let mut padded = [colored[0]; 16];
        padded[..colored.len()].copy_from_slice(&colored);
        endpoints::<3>(&padded)
    };
    let (mut c0, mut c1) = (to_565(low), to_565(high));
    if c0 > c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let (e0, e1) = (from_565(c0), from_565(c1));
    let palette = [e0, e1, [0, 1, 2].map(|c| (e0[c] + e1[c]) / 2), [0, 0, 0]];
    let mut indices = 0u32;
    for (i, texel) in block.iter().enumerate() {
        let best = (0..4)
            .min_by_key(|&k| distance(palette[k], *texel))
            .unwrap_or(0);
        indices |= (best as u32) << (2 * i);
    }
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..].copy_from_slice(&indices.to_le_bytes());
    out
}

/// BC3 のアルファのブロック (8 バイト)。`alpha0 > alpha1` の 8 段のモードで書く (同じなら添字はすべて 0)。
fn alpha_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let a0 = block.iter().map(|t| t[3]).max().unwrap_or(0);
    let a1 = block.iter().map(|t| t[3]).min().unwrap_or(0);
    let mut indices = 0u64;
    if a0 != a1 {
        let (e0, e1) = (i32::from(a0), i32::from(a1));
        let mut palette = [e0, e1, 0, 0, 0, 0, 0, 0];
        for (k, value) in palette.iter_mut().enumerate().skip(2) {
            let w = k as i32 - 1;
            *value = ((7 - w) * e0 + w * e1) / 7;
        }
        for (i, texel) in block.iter().enumerate() {
            let a = i32::from(texel[3]);
            let best = (0..8).min_by_key(|&k| (palette[k] - a).abs()).unwrap_or(0);
            indices |= (best as u64) << (3 * i);
        }
    }
    let mut out = [0u8; 8];
    out[0] = a0;
    out[1] = a1;
    out[2..].copy_from_slice(&indices.to_le_bytes()[..6]);
    out
}

/// BC7 の 4bit の添字の重み (`/ 64`)。
const BC7_WEIGHTS: [i32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

/// BC7 のモード 6 のブロック (16 バイト)。端点は最小 (p ビット 0) と最大 (p ビット 1)。
fn bc7_mode6(block: &[[u8; 4]; 16]) -> [u8; 16] {
    let (low, high) = endpoints::<4>(block);
    // 7bit の端点と p ビット。8bit の値は `(c << 1) | p`。
    let mut ends = [(low.map(|v| v >> 1), 0u8), (high.map(|v| v >> 1), 1u8)];
    let value = |(c, p): ([u8; 4], u8)| c.map(|v| i32::from((v << 1) | p));
    let pick = |ends: &[([u8; 4], u8); 2]| -> [usize; 16] {
        let (e0, e1) = (value(ends[0]), value(ends[1]));
        block.map(|texel| {
            (0..16)
                .min_by_key(|&k| {
                    let w = BC7_WEIGHTS[k];
                    (0..4)
                        .map(|c| (((64 - w) * e0[c] + w * e1[c] + 32) >> 6) - i32::from(texel[c]))
                        .map(|d| d * d)
                        .sum::<i32>()
                })
                .unwrap_or(0)
        })
    };
    let mut indices = pick(&ends);
    // 最初の画素の添字は 3bit しか書けない (最上位は 0)。8 以上なら端点を入れ替えて添字を反転する (重みは対称)。
    if indices[0] >= 8 {
        ends.swap(0, 1);
        indices = indices.map(|k| 15 - k);
    }
    let mut bits: u128 = 1 << 6; // モード 6
    let mut at = 7;
    let mut put = |value: u128, len: u32| {
        bits |= value << at;
        at += len;
    };
    for c in 0..4 {
        put(u128::from(ends[0].0[c]), 7);
        put(u128::from(ends[1].0[c]), 7);
    }
    put(u128::from(ends[0].1), 1);
    put(u128::from(ends[1].1), 1);
    for (i, &k) in indices.iter().enumerate() {
        put(k as u128, if i == 0 { 3 } else { 4 });
    }
    bits.to_le_bytes()
}
