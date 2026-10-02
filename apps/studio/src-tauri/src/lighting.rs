//! シーンの光源 (`Look::lighting`。`@ugoe/player` の `LightingSettings` の JSON) をレンダラの光源にする (T0808)。
//!
//! 仮想カメラの受け手が、出力ウィンドウ (`Player.updateLighting` → `toLightingJson` → `set_lighting_json`) と同じ光源で描くため。
//! 規則は `packages/player/src/lighting.ts` の `sanitizeLighting`・`toLightingJson` と同じ (変えたら両方を合わせる):
//! シーンの光源が無ければ既定の組 (`standard`)、足りない・壊れた値は既定値で埋め、範囲の外は収める。切ってあれば光源を渡さない。

use serde_json::Value;
use ugoe::core::Rgb;
use ugoe::render::{Light, Lighting};

/// 範囲 (`lighting.ts` の `LIMITS`)。
const INTENSITY: (f64, f64) = (0.0, 3.0);
const AZIMUTH: (f64, f64) = (-180.0, 180.0);
const ELEVATION: (f64, f64) = (0.0, 90.0);
const POSITION: (f64, f64) = (-0.5, 1.5);
const DEPTH: (f64, f64) = (0.0, 2.0);
const RANGE: (f64, f64) = (0.1, 4.0);

/// 色と強さ。
#[derive(Debug, Clone, PartialEq)]
struct LightColor {
    color: [f32; 3],
    intensity: f64,
}

/// `LightingSettings` (読んで収めたもの)。
#[derive(Debug, Clone, PartialEq)]
struct Settings {
    enabled: bool,
    ambient: LightColor,
    directional: Option<(LightColor, f64, f64)>,
    point: Option<(LightColor, [f64; 3], f64)>,
}

fn number(value: &Value, key: &str, (min, max): (f64, f64), fallback: f64) -> f64 {
    match value.get(key).and_then(Value::as_f64) {
        Some(v) if v.is_finite() => v.clamp(min, max),
        _ => fallback,
    }
}

fn boolean(value: &Value, key: &str, fallback: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

/// `#rrggbb` か `#rgb` を 0..1 の RGB にする (`color.ts` の `parseHexColor`。`#` は必須)。
fn hex(text: &str) -> Option<[f32; 3]> {
    let digits = text.strip_prefix('#')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let expanded: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    let channel = |i: usize| {
        u8::from_str_radix(&expanded[i..i + 2], 16)
            .ok()
            .map(|v| f32::from(v) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

fn light_color(value: &Value, color: &str, intensity: f64) -> LightColor {
    let fallback = hex(color).unwrap_or([1.0; 3]);
    LightColor {
        color: value
            .get("color")
            .and_then(Value::as_str)
            .and_then(hex)
            .unwrap_or(fallback),
        intensity: number(value, "intensity", INTENSITY, intensity),
    }
}

/// 読む (`sanitizeLighting`)。`None` (シーンに光源が無い) は既定の組 (`standard`)。
fn settings(value: Option<&Value>) -> Settings {
    let empty = Value::Null;
    let v = value.unwrap_or(&empty);
    let d = v.get("directional").unwrap_or(&empty);
    let p = v.get("point").unwrap_or(&empty);
    Settings {
        enabled: boolean(v, "enabled", true),
        ambient: light_color(v.get("ambient").unwrap_or(&empty), "#ffffff", 0.55),
        directional: boolean(d, "enabled", true).then(|| {
            (
                light_color(d, "#ffffff", 0.6),
                number(d, "azimuth", AZIMUTH, 135.0),
                number(d, "elevation", ELEVATION, 45.0),
            )
        }),
        point: boolean(p, "enabled", false).then(|| {
            (
                light_color(p, "#ffffff", 1.0),
                [
                    number(p, "x", POSITION, 0.3),
                    number(p, "y", POSITION, 0.3),
                    number(p, "z", DEPTH, 0.4),
                ],
                number(p, "range", RANGE, 1.5),
            )
        }),
    }
}

fn scaled(c: &LightColor) -> Rgb {
    let k = c.intensity as f32;
    Rgb::new(c.color[0] * k, c.color[1] * k, c.color[2] * k)
}

/// シーンの光源をレンダラの光源にする (`toLightingJson`)。`canvas` はモデルのキャンバスの大きさ。切ってあれば `None`。
pub fn scene_lighting(value: Option<&Value>, canvas: (f32, f32)) -> Option<Lighting> {
    let s = settings(value);
    if !s.enabled {
        return None;
    }
    let mut lights = Vec::new();
    if let Some((color, azimuth, elevation)) = &s.directional {
        let (az, el) = (azimuth.to_radians(), elevation.to_radians());
        let flat = el.cos();
        // 光が来る側 (Y 下向き・手前が +z) の逆が、光が進む向き。
        let toward = [flat * az.cos(), -flat * az.sin(), el.sin()];
        lights.push(Light::Directional {
            direction: toward.map(|c| -c as f32),
            color: scaled(color),
        });
    }
    if let Some((color, [x, y, z], range)) = &s.point {
        let (w, h) = (f64::from(canvas.0), f64::from(canvas.1));
        let size = w.max(h).max(1.0);
        lights.push(Light::Point {
            position: [(x * w) as f32, (y * h) as f32, (z * size) as f32],
            color: scaled(color),
            range: (range.max(RANGE.0) * size) as f32,
        });
    }
    Some(Lighting {
        ambient: scaled(&s.ambient),
        lights,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn no_scene_lighting_means_the_standard_preset() {
        let lighting = scene_lighting(None, (100.0, 200.0)).unwrap();
        assert!(close(lighting.ambient.r, 0.55));
        assert_eq!(lighting.lights.len(), 1);
        let Light::Directional { direction, color } = lighting.lights[0] else {
            panic!("{:?}", lighting.lights);
        };
        // 方位 135 度・高さ 45 度から来る光 (左上の手前) は、右下の奥へ進む。
        let h = 0.5f32.sqrt();
        assert!(close(direction[0], h * h), "{direction:?}");
        assert!(close(direction[1], h * h));
        assert!(close(direction[2], -h));
        assert!(close(color.g, 0.6));
    }

    #[test]
    fn reads_and_clamps_like_the_player() {
        let value = json!({
            "enabled": true,
            "ambient": { "color": "#ff0000", "intensity": 9 },
            "directional": { "enabled": false },
            "point": { "enabled": true, "color": "#00f", "intensity": 2, "x": 0.5, "y": 2.0, "z": 0.5, "range": 0.0 }
        });
        let lighting = scene_lighting(Some(&value), (200.0, 100.0)).unwrap();
        assert_eq!(lighting.ambient, Rgb::new(3.0, 0.0, 0.0));
        assert_eq!(
            lighting.lights,
            vec![Light::Point {
                position: [100.0, 150.0, 100.0],
                color: Rgb::new(0.0, 0.0, 2.0),
                range: 20.0,
            }]
        );
        assert!(scene_lighting(Some(&json!({ "enabled": false })), (1.0, 1.0)).is_none());
        // 壊れた値は既定値で埋める。
        let broken = json!({ "ambient": { "color": "red", "intensity": "x" } });
        let lighting = scene_lighting(Some(&broken), (1.0, 1.0)).unwrap();
        assert!(close(lighting.ambient.b, 0.55));
    }
}
