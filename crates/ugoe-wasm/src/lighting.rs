//! 光源の JSON (`WebPuppet::set_lighting_json`) を `ugoe::render::Lighting` にする。GPU には触れない。
//!
//! 形 (`docs/spec/rendering.md#ライティング`。座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z)):
//!
//! ```json
//! {
//!   "ambient": [0.3, 0.3, 0.3],
//!   "lights": [
//!     { "type": "directional", "direction": [1, 1, -1], "color": [1, 1, 1] },
//!     { "type": "point", "position": [512, 256, 300], "color": [1, 0.9, 0.8], "range": 1200 }
//!   ]
//! }
//! ```
//!
//! - `ambient` と各光源の `color` は省略すると白 `[1, 1, 1]`。1 を超えてもよい (強さを兼ねる)。
//! - `direction` は光が進む向き (長さは問わない)。点光源の `range` は省略すると減衰しない (JSON は無限大を書けないため)。
//! - 使えない光源 (向きが 0・`range` が正でない)・負の色は誤りにする (レンダラは黙って捨てるので、先に知らせる)。

use serde::Deserialize;
use ugoe::core::Rgb;
use ugoe::render::{Light, Lighting};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LightingJson {
    #[serde(default = "white")]
    ambient: [f32; 3],
    #[serde(default)]
    lights: Vec<LightJson>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum LightJson {
    Directional {
        direction: [f32; 3],
        #[serde(default = "white")]
        color: [f32; 3],
    },
    Point {
        position: [f32; 3],
        #[serde(default = "white")]
        color: [f32; 3],
        range: Option<f32>,
    },
}

fn white() -> [f32; 3] {
    [1.0; 3]
}

/// 光源の JSON を読む。誤りは人が読める文で返す。
pub fn lighting_from_json(json: &str) -> Result<Lighting, String> {
    let parsed: LightingJson = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let lights = parsed
        .lights
        .into_iter()
        .enumerate()
        .map(|(i, light)| {
            let (light, color) = match light {
                LightJson::Directional { direction, color } => {
                    if direction == [0.0; 3] {
                        return Err(format!("lights[{i}]: the direction is zero"));
                    }
                    (
                        Light::Directional {
                            direction,
                            color: rgb(color),
                        },
                        color,
                    )
                }
                LightJson::Point {
                    position,
                    color,
                    range,
                } => {
                    let range = range.unwrap_or(f32::INFINITY);
                    if range.is_nan() || range <= 0.0 {
                        return Err(format!("lights[{i}]: range must be positive, got {range}"));
                    }
                    (
                        Light::Point {
                            position,
                            color: rgb(color),
                            range,
                        },
                        color,
                    )
                }
            };
            check_color(&format!("lights[{i}].color"), color)?;
            Ok(light)
        })
        .collect::<Result<_, _>>()?;
    check_color("ambient", parsed.ambient)?;
    Ok(Lighting {
        ambient: rgb(parsed.ambient),
        lights,
    })
}

fn check_color(name: &str, color: [f32; 3]) -> Result<(), String> {
    if color.iter().any(|c| *c < 0.0) {
        return Err(format!("{name} must not be negative, got {color:?}"));
    }
    Ok(())
}

fn rgb([r, g, b]: [f32; 3]) -> Rgb {
    Rgb::new(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_lights_with_defaults() {
        let lighting = lighting_from_json(
            r#"{
                "ambient": [0.2, 0.3, 0.4],
                "lights": [
                    { "type": "directional", "direction": [1, 1, -1] },
                    { "type": "point", "position": [10, 20, 30], "color": [2, 1, 0], "range": 50 },
                    { "type": "point", "position": [0, 0, 1] }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(lighting.ambient, Rgb::new(0.2, 0.3, 0.4));
        assert_eq!(
            lighting.lights,
            vec![
                Light::Directional {
                    direction: [1.0, 1.0, -1.0],
                    color: Rgb::WHITE,
                },
                Light::Point {
                    position: [10.0, 20.0, 30.0],
                    color: Rgb::new(2.0, 1.0, 0.0),
                    range: 50.0,
                },
                Light::Point {
                    position: [0.0, 0.0, 1.0],
                    color: Rgb::WHITE,
                    range: f32::INFINITY,
                },
            ]
        );
    }

    #[test]
    fn empty_object_is_the_default_lighting() {
        assert_eq!(lighting_from_json("{}").unwrap(), Lighting::default());
    }

    #[test]
    fn rejects_unusable_lights() {
        for json in [
            r#"{ "lights": [{ "type": "spot", "position": [0, 0, 1] }] }"#,
            r#"{ "lights": [{ "type": "directional", "direction": [0, 0, 0] }] }"#,
            r#"{ "lights": [{ "type": "directional", "direction": [0, 0] }] }"#,
            r#"{ "lights": [{ "type": "point", "position": [0, 0, 1], "range": 0 }] }"#,
            r#"{ "lights": [{ "type": "point", "position": [0, 0, 1], "color": [-1, 0, 0] }] }"#,
            r#"{ "lights": [{ "type": "directional", "direction": [0, 0, 1], "dir": 1 }] }"#,
            r#"{ "ambient": [-0.1, 0, 0] }"#,
            r#"{ "ambient": 1 }"#,
            r#"{ "light": [] }"#,
        ] {
            assert!(lighting_from_json(json).is_err(), "{json}");
        }
    }
}
