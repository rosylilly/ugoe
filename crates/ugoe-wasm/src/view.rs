//! ビューポートの表示状態 (フィット・ズーム・パン) と座標変換。GPU には触れない。
//!
//! 座標系:
//! - **キャンバス座標**: モデルの座標 (左上原点・ピクセル単位・Y 下向き。`docs/spec/model.md`)。
//! - **画面座標**: 描画先 (canvas 要素の描画バッファ) の画素。左上原点・Y 下向き。
//!   CSS ピクセルではない (CSS ピクセル × `devicePixelRatio`)。
//!
//! ズーム 1・パン 0 は「キャンバス全体が収まる最大の大きさで中央に置く」(フィット) 状態。

use ugoe::render::View;

/// ズームの範囲 (フィットに対する倍率)。
pub const MIN_ZOOM: f32 = 1.0 / 64.0;
pub const MAX_ZOOM: f32 = 256.0;

/// ビューポートの表示状態。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewState {
    /// フィットに対する倍率。
    pub zoom: f32,
    /// フィットの位置からのずれ (画面座標の画素)。
    pub pan: [f32; 2],
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
        }
    }
}

/// キャンバス座標 → 画面座標の相似変換 (`screen = offset + scale * canvas`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub scale: f32,
    pub offset: [f32; 2],
}

impl Transform {
    pub fn canvas_to_screen(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        [
            self.offset[0] + self.scale * x,
            self.offset[1] + self.scale * y,
        ]
    }

    pub fn screen_to_canvas(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        [
            (x - self.offset[0]) / self.scale,
            (y - self.offset[1]) / self.scale,
        ]
    }

    /// レンダラに渡すビュー (画面座標 `0..w` × `0..h` をクリップ座標 `-1..1` × `1..-1` へ)。
    pub fn to_view(&self, surface: [u32; 2]) -> View {
        let (w, h) = (surface[0].max(1) as f32, surface[1].max(1) as f32);
        let [ox, oy] = self.offset;
        let s = self.scale;
        View {
            view_proj: [
                [2.0 * s / w, 0.0, 0.0],
                [0.0, -2.0 * s / h, 0.0],
                [2.0 * ox / w - 1.0, 1.0 - 2.0 * oy / h, 1.0],
            ],
        }
    }
}

impl ViewState {
    /// `canvas` (モデルのキャンバスの大きさ) を `surface` (画面の画素数) に表示する変換。
    pub fn transform(&self, canvas: [f32; 2], surface: [u32; 2]) -> Transform {
        let (w, h) = (surface[0] as f32, surface[1] as f32);
        let [cw, ch] = canvas;
        let fit = if cw > 0.0 && ch > 0.0 && w > 0.0 && h > 0.0 {
            (w / cw).min(h / ch)
        } else {
            1.0
        };
        let scale = fit * self.zoom;
        Transform {
            scale,
            offset: [
                w / 2.0 + self.pan[0] - cw / 2.0 * scale,
                h / 2.0 + self.pan[1] - ch / 2.0 * scale,
            ],
        }
    }

    /// ズームを `factor` 倍する。画面座標 `anchor` の下にあるキャンバスの点は動かさない
    /// (マウスホイールでカーソルの位置を中心に拡大縮小する用)。
    pub fn zoom_at(&mut self, factor: f32, anchor: [f32; 2], canvas: [f32; 2], surface: [u32; 2]) {
        if !(factor.is_finite() && factor > 0.0 && anchor.iter().all(|v| v.is_finite())) {
            return;
        }
        let point = self.transform(canvas, surface).screen_to_canvas(anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // `point` が再び `anchor` に写るように、新しい倍率でのずれを求める。
        let t = Self {
            pan: [0.0, 0.0],
            ..*self
        }
        .transform(canvas, surface);
        let moved = t.canvas_to_screen(point);
        self.pan = [anchor[0] - moved[0], anchor[1] - moved[1]];
    }
}

/// 描画先の大きさ (画素)。どちらの辺も `1..=max` (デバイスのテクスチャの上限) に収める。
pub fn surface_size(width: u32, height: u32, max: u32) -> (u32, u32) {
    let max = max.max(1);
    (width.clamp(1, max), height.clamp(1, max))
}

/// 背景色 (ストレートアルファ、各成分 0..1) を乗算済みアルファのクリア色にする。範囲の外はクランプする。
/// NaN を含めば `None` (呼び出し側は前の色のままにする。NaN のクリア色は描画の結果が決まらない)。
pub fn background_color(r: f64, g: f64, b: f64, a: f64) -> Option<wgpu::Color> {
    if [r, g, b, a].iter().any(|v| v.is_nan()) {
        return None;
    }
    let a = a.clamp(0.0, 1.0);
    Some(wgpu::Color {
        r: r.clamp(0.0, 1.0) * a,
        g: g.clamp(0.0, 1.0) * a,
        b: b.clamp(0.0, 1.0) * a,
        a,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
    }

    #[test]
    fn fit_centers_the_canvas() {
        let t = ViewState::default().transform([256.0, 128.0], [200, 200]);
        assert!(close(t.canvas_to_screen([0.0, 0.0]), [0.0, 50.0]));
        assert!(close(t.canvas_to_screen([256.0, 128.0]), [200.0, 150.0]));
    }

    #[test]
    fn pan_moves_in_screen_pixels() {
        let v = ViewState {
            zoom: 2.0,
            pan: [10.0, -5.0],
        };
        let t = v.transform([100.0, 100.0], [100, 100]);
        assert!(close(t.canvas_to_screen([50.0, 50.0]), [60.0, 45.0]));
        assert!(close(t.screen_to_canvas([60.0, 45.0]), [50.0, 50.0]));
    }

    #[test]
    fn zoom_at_keeps_the_anchor_fixed() {
        let (canvas, surface) = ([256.0, 256.0], [300, 200]);
        let mut v = ViewState::default();
        let anchor = [40.0, 170.0];
        let before = v.transform(canvas, surface).screen_to_canvas(anchor);
        v.zoom_at(3.0, anchor, canvas, surface);
        assert_eq!(v.zoom, 3.0);
        let after = v.transform(canvas, surface).screen_to_canvas(anchor);
        assert!(close(before, after), "{before:?} {after:?}");

        v.zoom_at(0.0, anchor, canvas, surface);
        assert_eq!(v.zoom, 3.0);
        v.zoom_at(1e9, anchor, canvas, surface);
        assert_eq!(v.zoom, MAX_ZOOM);
    }

    /// 有限でない位置を中心にしたズームは何も変えない (パンが NaN になって戻らなくなっていた)。
    #[test]
    fn zoom_at_ignores_non_finite_anchors() {
        let (canvas, surface) = ([256.0, 256.0], [300, 200]);
        let mut v = ViewState {
            zoom: 2.0,
            pan: [5.0, -3.0],
        };
        let before = v;
        for anchor in [[f32::NAN, 10.0], [10.0, f32::INFINITY]] {
            v.zoom_at(2.0, anchor, canvas, surface);
            assert_eq!(v, before, "{anchor:?}");
        }
    }

    #[test]
    fn surface_size_stays_within_the_device_limit() {
        assert_eq!(surface_size(0, 0, 8192), (1, 1));
        assert_eq!(surface_size(640, 480, 8192), (640, 480));
        assert_eq!(surface_size(10_000, 300, 8192), (8192, 300));
        assert_eq!(surface_size(10, 10, 0), (1, 1));
    }

    #[test]
    fn background_color_is_premultiplied_and_ignores_nan() {
        let c = background_color(1.0, 0.5, 2.0, 0.5).unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0.5, 0.25, 0.5, 0.5));
        let c = background_color(-1.0, 0.0, 0.0, f64::INFINITY).unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0.0, 0.0, 0.0, 1.0));
        assert!(background_color(f64::NAN, 0.0, 0.0, 1.0).is_none());
        assert!(background_color(0.0, 0.0, 0.0, f64::NAN).is_none());
    }

    #[test]
    fn view_matches_screen_transform() {
        let surface = [200, 100];
        let t = ViewState {
            zoom: 1.5,
            pan: [3.0, 4.0],
        }
        .transform([64.0, 64.0], surface);
        let m = t.to_view(surface).view_proj;
        let p = [10.0, 20.0];
        let clip = [
            m[0][0] * p[0] + m[1][0] * p[1] + m[2][0],
            m[0][1] * p[0] + m[1][1] * p[1] + m[2][1],
        ];
        let s = t.canvas_to_screen(p);
        let expected = [s[0] / 100.0 - 1.0, 1.0 - s[1] / 50.0];
        assert!(close(clip, expected), "{clip:?} {expected:?}");
    }
}
