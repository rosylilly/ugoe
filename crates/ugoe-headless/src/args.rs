//! 文字列の引数を読む (`ugoe render` のオプションと、MCP サーバの `render_png` の引数で共通)。
//!
//! どれも誤りを `String` で返すので、clap の `value_parser` にそのまま渡せる。

use std::path::PathBuf;

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
    fn parses_time_and_fps() {
        assert_eq!(parse_time("1.5"), Ok(1.5));
        assert!(parse_time("-1").is_err());
        assert!(parse_time("inf").is_err());
        assert_eq!(parse_fps("30"), Ok(30.0));
        assert!(parse_fps("0").is_err());
        assert!(check_time(f32::NAN).is_err());
        assert!(check_fps(-1.0).is_err());
    }
}
