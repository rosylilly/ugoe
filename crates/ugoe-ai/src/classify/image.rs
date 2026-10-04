//! 分類のためにモデルへ送る画像 (全体の合成画像とレイヤーのサムネイル) を作る。
//!
//! 送る画像は小さくする (大きな画像はトークンを多く使い、分類の精度は上がらない)。
//! 透明な部分は明るい灰色で塗る (透明のままだとモデルからは黒に見え、黒い線が消えるため)。

use ugoe_psd::{Layer, LayerNode, LayerTree};

use crate::AiError;

/// 透明な部分を塗る色。
const MATTE: [u8; 3] = [224, 224, 224];

/// 8 ビット RGB の画像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Rgb {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgb {
    /// PNG にする。
    pub fn to_png(&self) -> Result<Vec<u8>, AiError> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let err = |e: png::EncodingError| AiError::InvalidRequest(format!("png encoding: {e}"));
        let mut writer = encoder.write_header().map_err(err)?;
        writer.write_image_data(&self.data).map_err(err)?;
        writer.finish().map_err(err)?;
        Ok(out)
    }
}

/// 見えているレイヤーを通常の合成で重ねた、キャンバス全体の非乗算 RGBA (背景は透明)。
///
/// ブレンドモード・クリッピングは無視する (分類に使う目安の画像なので)。
fn composite_rgba(tree: &LayerTree) -> Vec<f32> {
    let (w, h) = (tree.width as usize, tree.height as usize);
    // 乗算済みの RGBA で重ねる
    let mut acc = vec![0.0f32; w * h * 4];
    fn walk(nodes: &[LayerNode], opacity: f32, acc: &mut [f32], w: usize, h: usize) {
        for node in nodes {
            match node {
                LayerNode::Group {
                    visible,
                    children,
                    opacity: o,
                    ..
                } => {
                    if *visible {
                        walk(children, opacity * o, acc, w, h);
                    }
                }
                LayerNode::Layer(layer) => {
                    if layer.visible {
                        let a = opacity * layer.opacity * layer.fill_opacity;
                        blend_layer(layer, a, acc, w, h);
                    }
                }
            }
        }
    }
    walk(&tree.root, 1.0, &mut acc, w, h);
    acc
}

fn blend_layer(layer: &Layer, opacity: f32, acc: &mut [f32], w: usize, h: usize) {
    let (ox, oy) = (layer.offset.x.round() as i64, layer.offset.y.round() as i64);
    for ly in 0..layer.height as usize {
        let y = oy + ly as i64;
        if y < 0 || y >= h as i64 {
            continue;
        }
        for lx in 0..layer.width as usize {
            let x = ox + lx as i64;
            if x < 0 || x >= w as i64 {
                continue;
            }
            let s = (ly * layer.width as usize + lx) * 4;
            let Some(px) = layer.rgba.get(s..s + 4) else {
                return;
            };
            let a = f32::from(px[3]) / 255.0 * opacity;
            if a <= 0.0 {
                continue;
            }
            let d = (y as usize * w + x as usize) * 4;
            for c in 0..3 {
                acc[d + c] = f32::from(px[c]) / 255.0 * a + acc[d + c] * (1.0 - a);
            }
            acc[d + 3] = a + acc[d + 3] * (1.0 - a);
        }
    }
}

/// 全体の合成画像を、長い辺が `max_side` 以下になるように縮めたもの。
///
/// 返す倍率 (縮めた画像の 1 画素 = キャンバスの `1 / scale` 画素) で、レイヤーの位置を
/// 縮めた画像の座標に直す。
pub(super) fn overview(tree: &LayerTree, max_side: u32) -> (Rgb, f32) {
    let acc = composite_rgba(tree);
    let scale = fit_scale(tree.width, tree.height, max_side);
    let img = resample_premultiplied(&acc, tree.width, tree.height, scale);
    (img, scale)
}

/// 1 枚のレイヤーのサムネイル。長い辺が `max_side` を超えれば縮め、`min_side` に満たなければ
/// 整数倍に拡大する (小さな目のパーツでもモデルが形を見られるように)。
pub(super) fn thumbnail(layer: &Layer, max_side: u32, min_side: u32) -> Option<Rgb> {
    // 画素の数は usize で数える (u32 ではあふれる大きさのレイヤーもある)。
    let texels = layer.width as usize * layer.height as usize;
    if texels == 0 || layer.rgba.len() != texels * 4 {
        return None;
    }
    let premul: Vec<f32> = layer
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|px| {
            let a = f32::from(px[3]) / 255.0;
            [
                f32::from(px[0]) / 255.0 * a,
                f32::from(px[1]) / 255.0 * a,
                f32::from(px[2]) / 255.0 * a,
                a,
            ]
        })
        .collect();
    let long = layer.width.max(layer.height);
    let scale = if long < min_side {
        (min_side.div_ceil(long)) as f32
    } else {
        fit_scale(layer.width, layer.height, max_side)
    };
    Some(resample_premultiplied(
        &premul,
        layer.width,
        layer.height,
        scale,
    ))
}

/// 長い辺を `max_side` 以下にする倍率 (1 以下)。
fn fit_scale(width: u32, height: u32, max_side: u32) -> f32 {
    let long = width.max(height).max(1);
    if long <= max_side {
        1.0
    } else {
        max_side as f32 / long as f32
    }
}

/// 乗算済み RGBA (0..=1) を `scale` 倍にし、灰色の上に置いた RGB にする。
///
/// 縮小は面積平均 (出力の 1 画素が覆う入力の画素の平均)、拡大は最近傍。
fn resample_premultiplied(src: &[f32], width: u32, height: u32, scale: f32) -> Rgb {
    let ow = ((width as f32 * scale).round() as u32).max(1);
    let oh = ((height as f32 * scale).round() as u32).max(1);
    let (w, h) = (width as usize, height as usize);
    let mut data = Vec::with_capacity(ow as usize * oh as usize * 3);
    for oy in 0..oh as usize {
        let y0 = oy * h / oh as usize;
        let y1 = ((oy + 1) * h / oh as usize).max(y0 + 1).min(h);
        for ox in 0..ow as usize {
            let x0 = ox * w / ow as usize;
            let x1 = ((ox + 1) * w / ow as usize).max(x0 + 1).min(w);
            let mut sum = [0.0f32; 4];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    for c in 0..4 {
                        sum[c] += src[i + c];
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f32;
            let a = sum[3] / n;
            for c in 0..3 {
                let v = sum[c] / n + f32::from(MATTE[c]) / 255.0 * (1.0 - a);
                data.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
    }
    Rgb {
        width: ow,
        height: oh,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe_core::Vec2;

    fn layer(w: u32, h: u32, rgba: [u8; 4]) -> Layer {
        Layer {
            name: "l".into(),
            visible: true,
            opacity: 1.0,
            offset: Vec2::new(1.0, 1.0),
            width: w,
            height: h,
            rgba: rgba.repeat((w * h) as usize),
            blend_mode: "norm".into(),
            id: None,
            fill_opacity: 1.0,
            clipping: false,
        }
    }

    #[test]
    fn thumbnail_fits_and_enlarges() {
        let big = thumbnail(&layer(400, 100, [255, 0, 0, 255]), 200, 64).unwrap();
        assert_eq!((big.width, big.height), (200, 50));
        assert_eq!(&big.data[..3], &[255, 0, 0]);
        let small = thumbnail(&layer(15, 9, [0, 0, 255, 255]), 200, 64).unwrap();
        assert_eq!((small.width, small.height), (75, 45));
        assert!(thumbnail(&layer(0, 0, [0; 4]), 200, 64).is_none());
    }

    #[test]
    fn huge_layers_without_pixels_have_no_thumbnail() {
        // 画素の数を u32 で数えるとあふれる大きさ (PSD のレイヤーは一辺 60000 まで)。
        let mut huge = layer(0, 0, [0; 4]);
        (huge.width, huge.height) = (60_000, 60_000);
        assert!(thumbnail(&huge, 200, 64).is_none());
    }

    #[test]
    fn transparent_pixels_become_matte() {
        let t = thumbnail(&layer(2, 2, [0, 0, 0, 0]), 200, 1).unwrap();
        assert_eq!(&t.data[..3], &MATTE);
    }

    #[test]
    fn overview_composites_visible_layers_and_scales() {
        let mut hidden = layer(2, 2, [0, 255, 0, 255]);
        hidden.visible = false;
        let tree = LayerTree {
            width: 4,
            height: 4,
            root: vec![
                LayerNode::Layer(layer(2, 2, [255, 0, 0, 255])),
                LayerNode::Layer(hidden),
            ],
            warnings: Vec::new(),
        };
        let (img, scale) = overview(&tree, 2);
        assert_eq!(scale, 0.5);
        assert_eq!((img.width, img.height), (2, 2));
        // 右下の 2x2 のうち左上の 1 画素だけが赤: 赤 1/4 + 灰 3/4
        let px = &img.data[9..12];
        assert_eq!(px[0], ((255.0 + 3.0 * 224.0) / 4.0f32).round() as u8);
        assert_eq!(px[1], ((3.0 * 224.0) / 4.0f32).round() as u8);
        let png = img.to_png().unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
