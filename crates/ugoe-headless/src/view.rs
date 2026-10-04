//! 出力の大きさと、キャンバス座標 (Y 下向き) をクリップ座標 (Y 上向き) へ写すビュー
//! (`docs/spec/rendering.md`)。

use ugoe_render::View;

use crate::{Error, bail};

/// 出力の一辺の上限 (デバイスの上限がこれより小さければそちらが優先される)。
pub const MAX_SIZE: u32 = 16384;

/// キャンバスを出力画像にどう収めるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Fit {
    /// 縦横比を保って全体が収まるように拡大縮小し、中央に置く。
    #[default]
    Contain,
    /// 縦横比を無視して出力全体に引き伸ばす。
    Stretch,
}

/// 出力サイズを決める。
///
/// `size` があればそれ。`width` / `height` の片方だけならキャンバスの縦横比から他方を決める。
/// どちらも無ければキャンバスの大きさ × `scale`。どの一辺も 1..=[`MAX_SIZE`] でなければエラー。
pub fn output_size(
    canvas: (f32, f32),
    size: Option<(u32, u32)>,
    width: Option<u32>,
    height: Option<u32>,
    scale: f32,
) -> Result<(u32, u32), Error> {
    let (cw, ch) = canvas;
    if !(cw > 0.0 && ch > 0.0) {
        bail!("model canvas has no area ({cw}x{ch})");
    }
    let dim = |v: f32| -> Result<u32, Error> {
        let n = v.round();
        if !(1.0..=MAX_SIZE as f32).contains(&n) {
            bail!("output size {v} is out of range 1..={MAX_SIZE}");
        }
        Ok(n as u32)
    };
    // 明示した大きさも確かめる (`args` を通さずに値を渡す呼び出し側がある。MCP サーバの `render_png`)。
    let given = |n: u32| -> Result<u32, Error> {
        if n == 0 || n > MAX_SIZE {
            bail!("output size {n} is out of range 1..={MAX_SIZE}");
        }
        Ok(n)
    };
    Ok(match (size, width, height) {
        (Some((w, h)), _, _) => (given(w)?, given(h)?),
        (None, Some(w), Some(h)) => (given(w)?, given(h)?),
        (None, Some(w), None) => (given(w)?, dim(w as f32 * ch / cw)?),
        (None, None, Some(h)) => (dim(h as f32 * cw / ch)?, given(h)?),
        (None, None, None) => {
            if !(scale > 0.0 && scale.is_finite()) {
                bail!("--scale must be positive, got {scale}");
            }
            (dim(cw * scale)?, dim(ch * scale)?)
        }
    })
}

/// キャンバス (Y 下向き) を出力画像に収めるビュー。クリップ座標は Y 上向き。
pub fn fit_view(canvas: (f32, f32), output: (u32, u32), fit: Fit) -> View {
    let (cw, ch) = canvas;
    let (w, h) = (output.0 as f32, output.1 as f32);
    // キャンバス座標 → 出力画素座標 (px = ox + sx * x, py = oy + sy * y)。
    let (sx, sy, ox, oy) = match fit {
        Fit::Contain => {
            let s = (w / cw).min(h / ch);
            (s, s, (w - cw * s) / 2.0, (h - ch * s) / 2.0)
        }
        Fit::Stretch => (w / cw, h / ch, 0.0, 0.0),
    };
    // 画素座標 → クリップ座標 (x: 0..w → -1..1、y: 0..h → 1..-1)。
    View {
        view_proj: [
            [2.0 * sx / w, 0.0, 0.0],
            [0.0, -2.0 * sy / h, 0.0],
            [2.0 * ox / w - 1.0, 1.0 - 2.0 * oy / h, 1.0],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_size_rules() {
        let c = (256.0, 128.0);
        assert_eq!(output_size(c, None, None, None, 1.0).unwrap(), (256, 128));
        assert_eq!(output_size(c, None, None, None, 0.5).unwrap(), (128, 64));
        assert_eq!(output_size(c, None, Some(64), None, 1.0).unwrap(), (64, 32));
        assert_eq!(
            output_size(c, None, None, Some(64), 1.0).unwrap(),
            (128, 64)
        );
        assert_eq!(
            output_size(c, None, Some(10), Some(20), 1.0).unwrap(),
            (10, 20)
        );
        assert_eq!(
            output_size(c, Some((5, 6)), None, None, 1.0).unwrap(),
            (5, 6)
        );
        assert!(output_size(c, None, None, None, 0.0).is_err());
        assert!(output_size(c, None, None, None, 1000.0).is_err());
        assert!(output_size((0.0, 1.0), None, None, None, 1.0).is_err());
    }

    /// 明示した大きさも 1..=MAX_SIZE に入っているかを確かめる (MCP サーバは `args` を通さずに値を渡す)。
    #[test]
    fn explicit_sizes_are_checked() {
        let c = (256.0, 128.0);
        assert!(output_size(c, Some((0, 6)), None, None, 1.0).is_err());
        assert!(output_size(c, Some((5, MAX_SIZE + 1)), None, None, 1.0).is_err());
        assert!(output_size(c, None, Some(0), Some(20), 1.0).is_err());
        assert!(output_size(c, None, Some(10), Some(MAX_SIZE + 1), 1.0).is_err());
        assert!(output_size(c, None, Some(0), None, 1.0).is_err());
        assert!(output_size(c, None, None, Some(u32::MAX), 1.0).is_err());
        assert_eq!(
            output_size(c, Some((MAX_SIZE, 1)), None, None, 1.0).unwrap(),
            (MAX_SIZE, 1)
        );
    }

    fn apply(view: &View, (x, y): (f32, f32)) -> (f32, f32) {
        let m = view.view_proj;
        (
            m[0][0] * x + m[1][0] * y + m[2][0],
            m[0][1] * x + m[1][1] * y + m[2][1],
        )
    }

    #[test]
    fn contain_fit_centers_canvas() {
        // 256x128 のキャンバスを 100x100 に: 縮尺 100/256、上下に余白。
        let view = fit_view((256.0, 128.0), (100, 100), Fit::Contain);
        let (x0, y0) = apply(&view, (0.0, 0.0));
        let (x1, y1) = apply(&view, (256.0, 128.0));
        assert!((x0 + 1.0).abs() < 1e-6 && (x1 - 1.0).abs() < 1e-6);
        assert!((y0 - 0.5).abs() < 1e-6, "{y0}");
        assert!((y1 + 0.5).abs() < 1e-6, "{y1}");
    }

    #[test]
    fn stretch_fit_fills_output() {
        let view = fit_view((256.0, 128.0), (100, 100), Fit::Stretch);
        assert_eq!(apply(&view, (0.0, 0.0)), (-1.0, 1.0));
        assert_eq!(apply(&view, (256.0, 128.0)), (1.0, -1.0));
    }
}
