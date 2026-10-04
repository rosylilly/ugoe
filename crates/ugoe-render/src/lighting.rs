//! 光源 (`docs/spec/rendering.md#ライティング`)。
//!
//! 光源はモデルのデータではなく、ホストがフレームごとに [`crate::Renderer::set_lighting`] で渡す。

use ugoe_core::Rgb;

/// 参照レンダラが 1 フレームに使う光源の数の上限。使える光源のうち、これより後ろのものは使わない。
pub const MAX_LIGHTS: usize = 8;

/// ホストが渡す光源の組 (`docs/spec/rendering.md#ライティング`)。
///
/// 既定値は環境光が白 (1, 1, 1) で光源なし。ライティングを受ける要素も、受けない場合と同じ色で描かれる。
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    /// 環境光。法線によらず掛かる。
    pub ambient: Rgb,
    /// 光源。使える光源 ([`Light`]) のうち先頭の [`MAX_LIGHTS`] 個だけを使う。
    pub lights: Vec<Light>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient: Rgb::WHITE,
            lights: Vec::new(),
        }
    }
}

/// 光源 1 つ。位置と向きはキャンバス座標 (ピクセル単位・Y 下向き) + 奥行き (z、キャンバスから手前向き)。
///
/// 色は 1 を超えてもよい (強さを兼ねる)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    /// 平行光源。`direction` は光が進む向き (長さは問わない)。
    /// 例: 左上の手前から照らすなら `[1.0, 1.0, -1.0]`。長さが 0 か有限でなければ使わない。
    Directional {
        /// 光が進む向き (x, y, z)。
        direction: [f32; 3],
        /// 光の色 (強さを兼ねる)。
        color: Rgb,
    },
    /// 点光源。`position` の位置から全方向を照らし、距離 `range` で 0 になるように減衰する
    /// (`(1 − d / range)²`)。`range` が `f32::INFINITY` なら減衰しない。
    /// `range` が正でない (NaN を含む) か、位置が有限でなければ使わない。
    Point {
        /// 光源の位置 (x, y, z)。
        position: [f32; 3],
        /// 光の色 (強さを兼ねる)。
        color: Rgb,
        /// 光が届く距離 (キャンバスのピクセル単位)。
        range: f32,
    },
}

/// シェーダの `Light` と同じ配置。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct LightRaw {
    /// 平行光源: xyz = 光源へ向かう単位ベクトル, w = 0。点光源: xyz = 位置, w = 1。
    vector: [f32; 4],
    /// rgb = 色, a = 点光源の `1 / range` (平行光源は 0)。
    color: [f32; 4],
}

/// シェーダの `Lighting` と同じ配置。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightingRaw {
    ambient: [f32; 4],
    /// x = 使う光源の数。
    count: [u32; 4],
    lights: [LightRaw; MAX_LIGHTS],
}

impl LightingRaw {
    pub(crate) fn new(lighting: &Lighting) -> Self {
        let mut raw = Self {
            ambient: rgba(lighting.ambient, 0.0),
            count: [0; 4],
            lights: [LightRaw {
                vector: [0.0; 4],
                color: [0.0; 4],
            }; MAX_LIGHTS],
        };
        let lights = lighting.lights.iter().filter_map(LightRaw::new);
        for (slot, light) in raw.lights.iter_mut().zip(lights) {
            *slot = light;
            raw.count[0] += 1;
        }
        raw
    }
}

impl LightRaw {
    /// 使えない光源 (向きが 0・有限でない値・`range` が正でない) なら `None`。
    fn new(light: &Light) -> Option<Self> {
        match *light {
            Light::Directional { direction, color } => {
                // 最大の成分で割ってから長さを求める (2 乗があふれたり 0 に潰れたりしないように)。
                let scale = direction.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                if !(scale.is_finite() && scale > 0.0) || direction.iter().any(|v| v.is_nan()) {
                    return None;
                }
                let [x, y, z] = direction.map(|v| v / scale);
                let length = (x * x + y * y + z * z).sqrt();
                Some(Self {
                    vector: [-x / length, -y / length, -z / length, 0.0],
                    color: rgba(color, 0.0),
                })
            }
            Light::Point {
                position,
                color,
                range,
            } => (position.iter().all(|v| v.is_finite()) && range > 0.0).then(|| {
                let [x, y, z] = position;
                Self {
                    vector: [x, y, z, 1.0],
                    color: rgba(color, 1.0 / range),
                }
            }),
        }
    }
}

fn rgba(c: Rgb, a: f32) -> [f32; 4] {
    [c.r, c.g, c.b, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_light_points_toward_the_light() {
        let raw = LightingRaw::new(&Lighting {
            ambient: Rgb::new(0.1, 0.2, 0.3),
            lights: vec![Light::Directional {
                direction: [0.0, 0.0, -2.0],
                color: Rgb::WHITE,
            }],
        });
        assert_eq!(raw.ambient, [0.1, 0.2, 0.3, 0.0]);
        assert_eq!(raw.count[0], 1);
        assert_eq!(raw.lights[0].vector, [0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn extreme_finite_directions_are_usable() {
        // 有限で 0 でない向きは、成分の 2 乗が f32 であふれても (または 0 に潰れても) 使う。
        let raw = LightingRaw::new(&Lighting {
            ambient: Rgb::BLACK,
            lights: vec![
                Light::Directional {
                    direction: [3e37, 0.0, -4e37],
                    color: Rgb::WHITE,
                },
                Light::Directional {
                    direction: [0.0, 1e-30, 0.0],
                    color: Rgb::WHITE,
                },
            ],
        });
        assert_eq!(raw.count[0], 2);
        let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(close(raw.lights[0].vector, [-0.6, 0.0, 0.8, 0.0]));
        assert!(close(raw.lights[1].vector, [0.0, -1.0, 0.0, 0.0]));
    }

    #[test]
    fn point_light_stores_inverse_range() {
        let raw = LightingRaw::new(&Lighting {
            ambient: Rgb::BLACK,
            lights: vec![
                Light::Point {
                    position: [1.0, 2.0, 3.0],
                    color: Rgb::WHITE,
                    range: 4.0,
                },
                Light::Point {
                    position: [0.0; 3],
                    color: Rgb::WHITE,
                    range: f32::INFINITY,
                },
            ],
        });
        assert_eq!(raw.count[0], 2);
        assert_eq!(raw.lights[0].vector, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(raw.lights[0].color[3], 0.25);
        assert_eq!(raw.lights[1].color[3], 0.0);
    }

    #[test]
    fn unusable_and_extra_lights_are_dropped() {
        let white = Rgb::WHITE;
        let mut lights = vec![
            Light::Directional {
                direction: [0.0; 3],
                color: white,
            },
            Light::Directional {
                direction: [f32::NAN, 0.0, 1.0],
                color: white,
            },
            Light::Point {
                position: [0.0; 3],
                color: white,
                range: 0.0,
            },
            Light::Point {
                position: [0.0; 3],
                color: white,
                range: f32::NAN,
            },
        ];
        assert_eq!(
            LightingRaw::new(&Lighting {
                ambient: white,
                lights: lights.clone(),
            })
            .count[0],
            0
        );
        // 使えない光源は上限の数に数えない。
        lights.extend(std::iter::repeat_n(
            Light::Directional {
                direction: [0.0, 0.0, -1.0],
                color: white,
            },
            MAX_LIGHTS + 1,
        ));
        let raw = LightingRaw::new(&Lighting {
            ambient: white,
            lights,
        });
        assert_eq!(raw.count[0], MAX_LIGHTS as u32);
    }
}
