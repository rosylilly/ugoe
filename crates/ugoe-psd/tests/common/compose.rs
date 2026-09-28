//! テスト用 PSD の合成画像 (画像データのセクション) を、レイヤーから計算する。
//!
//! Photoshop の合成のうち、フィクスチャが使うものだけを扱う:
//! - ブレンドモードは通常 (`norm`) と乗算 (`mul `)。ほかは panic する
//! - レイヤーの不透明度と塗りの不透明度 (効果がないので両者の積)
//! - 非表示のレイヤー・グループは描かない
//! - クリッピング: 直下のクリッピングでないレイヤー (土台) の画素の不透明度で切り抜く。
//!   土台は不透明度 100% の通常のレイヤーに限る
//! - 通過 (`pass`) のグループは子をそのまま背景に描き、不透明度で元の背景と混ぜる。
//!   それ以外のグループは子を透明な画像に描いてから、通常のブレンドで背景に重ねる
//! - ユーザーマスク (ライターが書く 4x4 の不透明な矩形の外を透明にする)
//! - 調整レイヤー (画素を持たないレイヤー) は何もしない (ライターが書くのは何も変えない補正だけ)
//!
//! 色は非乗算の sRGB の値をそのまま線形に混ぜる (Photoshop の 8 ビット RGB の既定と同じ)。
//! 最後に白い背景に重ねて、アルファのない RGB8 にする。

use super::Layer;

/// 乗算済みの RGBA (0..=1)。
type Canvas = Vec<[f32; 4]>;

enum Node<'a> {
    Layer(&'a Layer),
    Group(&'a Layer, Vec<Node<'a>>),
}

/// ファイル内の順 (下から上) のレイヤーを木にする。
fn tree(layers: &[Layer]) -> Vec<Node<'_>> {
    let mut stack: Vec<Vec<Node<'_>>> = vec![Vec::new()];
    for layer in layers {
        match layer.section {
            Some((3, _)) => stack.push(Vec::new()),
            Some((1 | 2, _)) => {
                let children = stack.pop().expect("group end before group");
                stack
                    .last_mut()
                    .expect("unbalanced groups")
                    .push(Node::Group(layer, children));
            }
            _ => stack
                .last_mut()
                .expect("unbalanced groups")
                .push(Node::Layer(layer)),
        }
    }
    assert_eq!(stack.len(), 1, "unbalanced groups");
    stack.pop().unwrap_or_default()
}

/// レイヤーを合成し、白い背景に重ねた RGB8 を返す。
pub fn flatten(width: u32, height: u32, layers: &[Layer]) -> Vec<u8> {
    let rgba = compose(width, height, layers);
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            // 非乗算の値を白に重ねる。
            let over_white = |c: u8| {
                let (c, a) = (f32::from(c), f32::from(a) / 255.0);
                (c * a + 255.0 * (1.0 - a)).round() as u8
            };
            [over_white(r), over_white(g), over_white(b)]
        })
        .collect()
}

/// レイヤーを透明な画像に合成し、非乗算の RGBA8 を返す。
pub fn compose(width: u32, height: u32, layers: &[Layer]) -> Vec<u8> {
    let mut canvas = vec![[0.0; 4]; width as usize * height as usize];
    draw(&tree(layers), width, height, &mut canvas);
    canvas
        .iter()
        .flat_map(|&[r, g, b, a]| {
            let un = |c: f32| {
                if a <= 0.0 {
                    0
                } else {
                    (c / a * 255.0).round().clamp(0.0, 255.0) as u8
                }
            };
            [un(r), un(g), un(b), (a * 255.0).round() as u8]
        })
        .collect()
}

fn unit(v: u8) -> f32 {
    f32::from(v) / 255.0
}

fn draw(nodes: &[Node<'_>], width: u32, height: u32, canvas: &mut Canvas) {
    // クリッピングの土台の画素の不透明度 (キャンバスの大きさ)。
    let mut base: Option<Vec<f32>> = None;
    for node in nodes {
        match node {
            Node::Layer(layer) => {
                let coverage = coverage(layer, width, height);
                let clip = if layer.clipping {
                    Some(
                        base.as_deref()
                            .expect("clipping layer without a base layer"),
                    )
                } else {
                    None
                };
                if !layer.hidden && layer.rgba.is_some() {
                    paint(layer, &coverage, clip, width, canvas);
                }
                if !layer.clipping {
                    assert!(
                        layer.opacity == 255 && layer.blend == *b"norm" || !is_base(nodes, layer),
                        "clipping base {} must be opaque and normal",
                        layer.name
                    );
                    base = Some(if layer.hidden {
                        vec![0.0; coverage.len()]
                    } else {
                        coverage
                    });
                }
            }
            Node::Group(group, children) => {
                assert!(!group.clipping, "clipped groups are not supported");
                base = None;
                if group.hidden {
                    continue;
                }
                let opacity = unit(group.opacity);
                match &group.group_blend() {
                    b"pass" => {
                        let mut inner = canvas.clone();
                        draw(children, width, height, &mut inner);
                        for (dst, src) in canvas.iter_mut().zip(&inner) {
                            for c in 0..4 {
                                dst[c] += (src[c] - dst[c]) * opacity;
                            }
                        }
                    }
                    b"norm" => {
                        let mut inner = vec![[0.0; 4]; canvas.len()];
                        draw(children, width, height, &mut inner);
                        for (dst, src) in canvas.iter_mut().zip(&inner) {
                            let src = src.map(|c| c * opacity);
                            for c in 0..4 {
                                dst[c] = src[c] + dst[c] * (1.0 - src[3]);
                            }
                        }
                    }
                    other => panic!("group blend {:?} is not supported", other),
                }
            }
        }
    }
}

/// `layer` の直上がクリッピングのレイヤーか (= `layer` がクリッピングの土台か)。
fn is_base(nodes: &[Node<'_>], layer: &Layer) -> bool {
    let index = nodes
        .iter()
        .position(|n| matches!(n, Node::Layer(l) if std::ptr::eq(*l, layer)))
        .expect("layer in nodes");
    matches!(nodes.get(index + 1), Some(Node::Layer(l)) if l.clipping)
}

/// レイヤーの各画素の不透明度 (0..=1) をキャンバスの大きさで返す。マスクを掛けてある。
fn coverage(layer: &Layer, width: u32, height: u32) -> Vec<f32> {
    let mut out = vec![0.0; width as usize * height as usize];
    let Some(rgba) = &layer.rgba else {
        return out;
    };
    for_each_pixel(layer, width, height, |i, j| {
        let mut a = unit(rgba[j * 4 + 3]);
        if layer.mask {
            let (x, y) = (
                (j as u32 % layer.width) as i32,
                (j as u32 / layer.width) as i32,
            );
            if x >= 4 || y >= 4 {
                a = 0.0;
            }
        }
        out[i] = a;
    });
    out
}

/// キャンバスの中に入るレイヤーの画素ごとに、(キャンバスの添字, レイヤーの添字) を渡す。
fn for_each_pixel(layer: &Layer, width: u32, height: u32, mut f: impl FnMut(usize, usize)) {
    for ly in 0..layer.height as i32 {
        for lx in 0..layer.width as i32 {
            let (x, y) = (layer.left + lx, layer.top + ly);
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            f(
                y as usize * width as usize + x as usize,
                ly as usize * layer.width as usize + lx as usize,
            );
        }
    }
}

fn paint(layer: &Layer, coverage: &[f32], clip: Option<&[f32]>, width: u32, canvas: &mut Canvas) {
    let Some(rgba) = &layer.rgba else {
        return;
    };
    let height = (canvas.len() / width as usize) as u32;
    let opacity = unit(layer.opacity) * layer.fill_opacity.map_or(1.0, unit);
    for_each_pixel(layer, width, height, |i, j| {
        let a = coverage[i] * opacity * clip.map_or(1.0, |c| c[i]);
        if a <= 0.0 {
            return;
        }
        let src = [0, 1, 2].map(|c| unit(rgba[j * 4 + c]));
        let dst = &mut canvas[i];
        let ab = dst[3];
        // 背景の非乗算の色。
        let cb = [0, 1, 2].map(|c| if ab > 0.0 { dst[c] / ab } else { 0.0 });
        for c in 0..3 {
            // ブレンドした色は、背景の不透明度に応じて元の色と混ぜる (W3C Compositing の式)。
            let blended = match &layer.blend {
                b"norm" => src[c],
                b"mul " => src[c] * cb[c],
                other => panic!("blend {:?} is not supported", other),
            };
            let cs = (1.0 - ab) * src[c] + ab * blended;
            dst[c] = a * cs + (1.0 - a) * dst[c];
        }
        dst[3] = a + ab * (1.0 - a);
    });
}
