//! 文字列の引数を読む (`ugoe render` のオプションと、MCP サーバの `render_png` の引数で共通)。
//!
//! どれも誤りを `String` で返すので、clap の `value_parser` にそのまま渡せる。

use std::path::PathBuf;
use std::time::Duration;

use ugoe::core::Rgb;
use ugoe_render::Light;

use crate::view::MAX_SIZE;

/// `ID=VALUE` を読む。
pub fn parse_param(s: &str) -> Result<(String, f32), String> {
    let (id, value) = s
        .split_once('=')
        .ok_or_else(|| format!("expected ID=VALUE, got `{s}`"))?;
    let id = id.trim();
    if id.is_empty() {
        return Err(format!("empty parameter ID in `{s}`"));
    }
    let value: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{value}` in `{s}`"))?;
    if !value.is_finite() {
        return Err(format!("parameter value must be finite, got `{value}`"));
    }
    Ok((id.to_owned(), value))
}

/// `WxH` を読む。
pub fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("expected WxH, got `{s}`"))?;
    let parse = |v: &str| -> Result<u32, String> {
        let n: u32 = v
            .trim()
            .parse()
            .map_err(|_| format!("invalid size `{s}`"))?;
        check_dimension(n)?;
        Ok(n)
    };
    Ok((parse(w)?, parse(h)?))
}

/// `--width` / `--height` の値。
pub fn parse_dimension(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| format!("invalid size `{s}`"))?;
    check_dimension(n)?;
    Ok(n)
}

/// 出力の一辺が 1..=[`MAX_SIZE`] に入っているか。
pub fn check_dimension(n: u32) -> Result<(), String> {
    if n == 0 || n > MAX_SIZE {
        return Err(format!("size must be in 1..={MAX_SIZE}, got {n}"));
    }
    Ok(())
}

/// `transparent` / `white` / `black` / `#RRGGBB` / `#RRGGBBAA` (`#` は省略可) を、ストレートアルファの RGBA8 に読む。
pub fn parse_color(s: &str) -> Result<[u8; 4], String> {
    for (name, color) in [
        ("transparent", [0; 4]),
        ("white", [255; 4]),
        ("black", [0, 0, 0, 255]),
    ] {
        if s.eq_ignore_ascii_case(name) {
            return Ok(color);
        }
    }
    let hex = s.strip_prefix('#').unwrap_or(s);
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "expected transparent, white, black, #RRGGBB or #RRGGBBAA, got `{s}`"
        ));
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string());
    let a = if hex.len() == 8 { byte(6)? } else { 255 };
    Ok([byte(0)?, byte(2)?, byte(4)?, a])
}

/// `INDEX=PATH` を読む (`--texture`)。
pub fn parse_texture(s: &str) -> Result<(u32, PathBuf), String> {
    let (index, path) = s
        .split_once('=')
        .ok_or_else(|| format!("expected INDEX=PATH, got `{s}`"))?;
    let index = index
        .trim()
        .parse()
        .map_err(|_| format!("invalid texture index `{index}` in `{s}`"))?;
    if path.is_empty() {
        return Err(format!("empty path in `{s}`"));
    }
    Ok((index, PathBuf::from(path)))
}

/// `--time` を読む (0 以上の有限の秒数)。
pub fn parse_time(s: &str) -> Result<f32, String> {
    let t: f32 = s
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{s}`"))?;
    check_time(t).map_err(|_| format!("time must be a finite number >= 0, got `{s}`"))
}

/// `--fps` を読む (0 より大きい有限の値)。
pub fn parse_fps(s: &str) -> Result<f32, String> {
    let fps: f32 = s
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{s}`"))?;
    check_fps(fps).map_err(|_| format!("fps must be a finite number > 0, got `{s}`"))
}

/// `--gpu-timeout` を読む: GPU を待つ時間の上限の秒数 ([`crate::Gpu::set_wait_timeout`])。
///
/// 0 より大きい有限の秒数か、上限なしの `0`。負・NaN・無限大と、`Duration` に収まらない値はエラー。
pub fn parse_gpu_timeout(s: &str) -> Result<Duration, String> {
    let secs: f64 = s
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{s}`"))?;
    let invalid = || format!("GPU timeout must be a finite number of seconds >= 0, got `{s}`");
    if !secs.is_finite() || secs < 0.0 {
        return Err(invalid());
    }
    if secs == 0.0 {
        return Ok(Duration::MAX);
    }
    Duration::try_from_secs_f64(secs).map_err(|_| invalid())
}

/// 光の色 (`--ambient` と `--light` の色) を読む: `V` (灰色) か `R,G,B`。0 以上の有限の値で、1 を超えてもよい (強さを兼ねる)。
pub fn parse_light_color(s: &str) -> Result<Rgb, String> {
    let values = parse_floats(s)?;
    if values.iter().any(|v| *v < 0.0) {
        return Err(format!("light color must not be negative, got `{s}`"));
    }
    match values[..] {
        [v] => Ok(Rgb::new(v, v, v)),
        [r, g, b] => Ok(Rgb::new(r, g, b)),
        _ => Err(format!("expected V or R,G,B for a light color, got `{s}`")),
    }
}

/// 光源 (`--light`) を読む (`docs/spec/rendering.md#ライティング`)。座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z)。
///
/// - `directional:DX,DY,DZ[:COLOR]`: 平行光源。`DX,DY,DZ` は光が進む向き (長さは問わない)。
/// - `point:X,Y,Z[:COLOR[:RANGE]]`: 点光源。`RANGE` は減衰して 0 になる距離 (既定は `inf` で減衰しない)。
///
/// `COLOR` は [`parse_light_color`] の形で、既定は白 (1)。
pub fn parse_light(s: &str) -> Result<Light, String> {
    let mut fields = s.split(':');
    let kind = fields.next().unwrap_or_default().trim();
    let vector = fields
        .next()
        .ok_or_else(|| format!("expected directional:DX,DY,DZ or point:X,Y,Z, got `{s}`"))?;
    let vector: [f32; 3] = parse_floats(vector)?
        .try_into()
        .map_err(|_| format!("expected 3 numbers in `{vector}` (in `{s}`)"))?;
    let color = fields
        .next()
        .map(parse_light_color)
        .transpose()?
        .unwrap_or(Rgb::WHITE);
    let light = match kind {
        "directional" => {
            if vector == [0.0; 3] {
                return Err(format!("the direction of `{s}` is zero"));
            }
            Light::Directional {
                direction: vector,
                color,
            }
        }
        "point" => {
            let range = match fields.next().map(str::trim) {
                None | Some("inf") => f32::INFINITY,
                Some(range) => range
                    .parse::<f32>()
                    .ok()
                    .filter(|r| r.is_finite() && *r > 0.0)
                    .ok_or_else(|| {
                        format!("range must be a positive number or inf, got `{range}`")
                    })?,
            };
            Light::Point {
                position: vector,
                color,
                range,
            }
        }
        _ => {
            return Err(format!(
                "unknown light `{kind}` (expected directional or point)"
            ));
        }
    };
    if let Some(rest) = fields.next() {
        return Err(format!("unexpected `:{rest}` in `{s}`"));
    }
    Ok(light)
}

/// `a,b,c` のようにカンマで区切った有限の数を読む。
fn parse_floats(s: &str) -> Result<Vec<f32>, String> {
    s.split(',')
        .map(|v| {
            v.trim()
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("invalid number `{v}` in `{s}`"))
        })
        .collect()
}

/// 進める秒数が 0 以上の有限の値か。
pub fn check_time(t: f32) -> Result<f32, crate::Error> {
    if !t.is_finite() || t < 0.0 {
        crate::bail!("time must be a finite number >= 0, got {t}");
    }
    Ok(t)
}

/// 1 秒あたりの `update` の回数が 0 より大きい有限の値か。
pub fn check_fps(fps: f32) -> Result<f32, crate::Error> {
    if !fps.is_finite() || fps <= 0.0 {
        crate::bail!("fps must be a finite number > 0, got {fps}");
    }
    Ok(fps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_params() {
        assert_eq!(parse_param("X=0.5"), Ok(("X".into(), 0.5)));
        assert_eq!(parse_param("AngleX=-30"), Ok(("AngleX".into(), -30.0)));
        assert!(parse_param("X").is_err());
        assert!(parse_param("=1").is_err());
        assert!(parse_param("X=abc").is_err());
        assert!(parse_param("X=NaN").is_err());
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("640x480"), Ok((640, 480)));
        assert_eq!(parse_size("1X2"), Ok((1, 2)));
        assert!(parse_size("640").is_err());
        assert!(parse_size("0x10").is_err());
        assert!(parse_size("99999x10").is_err());
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("transparent"), Ok([0, 0, 0, 0]));
        assert_eq!(parse_color("White"), Ok([255; 4]));
        assert_eq!(parse_color("#ff8000"), Ok([255, 128, 0, 255]));
        assert_eq!(parse_color("FF800080"), Ok([255, 128, 0, 128]));
        assert!(parse_color("#fff").is_err());
        assert!(parse_color("#gg0000").is_err());
    }

    #[test]
    fn parses_texture_overrides() {
        assert_eq!(parse_texture("1=a.png"), Ok((1, PathBuf::from("a.png"))));
        assert!(parse_texture("a.png").is_err());
        assert!(parse_texture("x=a.png").is_err());
        assert!(parse_texture("0=").is_err());
    }

    #[test]
    fn parses_light_colors() {
        assert_eq!(parse_light_color("0.5"), Ok(Rgb::new(0.5, 0.5, 0.5)));
        assert_eq!(parse_light_color("1, 2,0"), Ok(Rgb::new(1.0, 2.0, 0.0)));
        assert!(parse_light_color("1,2").is_err());
        assert!(parse_light_color("-1").is_err());
        assert!(parse_light_color("inf").is_err());
        assert!(parse_light_color("").is_err());
    }

    #[test]
    fn parses_lights() {
        assert_eq!(
            parse_light("directional:1,1,-1"),
            Ok(Light::Directional {
                direction: [1.0, 1.0, -1.0],
                color: Rgb::WHITE,
            })
        );
        assert_eq!(
            parse_light("directional:0,0,-1:0.5"),
            Ok(Light::Directional {
                direction: [0.0, 0.0, -1.0],
                color: Rgb::new(0.5, 0.5, 0.5),
            })
        );
        assert_eq!(
            parse_light("point:10,20,30"),
            Ok(Light::Point {
                position: [10.0, 20.0, 30.0],
                color: Rgb::WHITE,
                range: f32::INFINITY,
            })
        );
        assert_eq!(
            parse_light("point:10,20,30:1,0.5,0:400"),
            Ok(Light::Point {
                position: [10.0, 20.0, 30.0],
                color: Rgb::new(1.0, 0.5, 0.0),
                range: 400.0,
            })
        );
        assert!(parse_light("spot:0,0,1").is_err());
        assert!(parse_light("directional").is_err());
        assert!(parse_light("directional:0,0,0").is_err());
        assert!(parse_light("directional:1,1").is_err());
        assert!(parse_light("directional:1,1,1:1:2").is_err());
        assert!(parse_light("point:0,0,1:1:0").is_err());
        assert!(parse_light("point:0,0,1:1:-5").is_err());
        assert!(parse_light("point:0,0,1:1:10:x").is_err());
    }

    #[test]
    fn parses_time_and_fps() {
        assert_eq!(parse_time("1.5"), Ok(1.5));
        assert!(parse_time("-1").is_err());
        assert!(parse_time("inf").is_err());
        assert_eq!(parse_fps("30"), Ok(30.0));
        assert!(parse_fps("0").is_err());
        assert!(check_time(f32::NAN).is_err());
        assert!(check_fps(-1.0).is_err());
    }

    #[test]
    fn parses_gpu_timeouts() {
        assert_eq!(parse_gpu_timeout("30"), Ok(Duration::from_secs(30)));
        assert_eq!(parse_gpu_timeout("0.5"), Ok(Duration::from_millis(500)));
        // 0 は上限なし。
        assert_eq!(parse_gpu_timeout("0"), Ok(Duration::MAX));
        for bad in ["-1", "inf", "NaN", "1e300", "x", ""] {
            assert!(parse_gpu_timeout(bad).is_err(), "{bad}");
        }
    }
}
