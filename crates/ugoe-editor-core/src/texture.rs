//! ドキュメントが持つテクスチャ画像 (レイヤー単位の PNG)。設計: `docs/editor.md#テクスチャ`。
//!
//! 画像はデコードせず、PNG ファイルのバイト列のまま持つ。`Model::textures` の `name` で引く。
//! プロジェクトでは `textures/<name>.png` に保存するため、名前はファイル名に使える形に限る。

use std::collections::BTreeMap;
use std::sync::Arc;

/// テクスチャ名 → PNG ファイルのバイト列。
///
/// Undo のスナップショットで複製しても画像のバイト列はコピーしないように `Arc` で共有する。
pub type Textures = BTreeMap<String, Arc<[u8]>>;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// テクスチャ名の長さの上限 (バイト)。
pub const MAX_TEXTURE_NAME_LEN: usize = 128;

/// テクスチャ名がファイル名として安全か: `[A-Za-z0-9_.-]` だけからなり、`.` で始まらず、
/// 空でなく、[`MAX_TEXTURE_NAME_LEN`] 以下。
pub fn is_valid_texture_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_TEXTURE_NAME_LEN
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}

/// PNG の幅と高さを IHDR から読む (画像はデコードしない)。PNG でなければ `None`。
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&PNG_SIGNATURE) || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let be = |range: std::ops::Range<usize>| -> Option<u32> {
        Some(u32::from_be_bytes(bytes.get(range)?.try_into().ok()?))
    };
    let (width, height) = (be(16..20)?, be(20..24)?);
    (width > 0 && height > 0).then_some((width, height))
}

/// 単色の RGBA8 PNG を作る (新規プロジェクトの既定テクスチャやテスト用)。
pub fn solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let pixels: Vec<u8> = (0..u64::from(width) * u64::from(height))
        .flat_map(|_| rgba)
        .collect();
    encode_rgba_png(width, height, &pixels)
}

/// 非乗算 RGBA8 の画素を PNG にする。`rgba` の長さは `width * height * 4` であること。
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // メモリ上への書き込みは、寸法とデータ長が合っていれば失敗しない。
    let written = encoder.write_header().and_then(|mut writer| {
        writer.write_image_data(rgba)?;
        writer.finish()
    });
    debug_assert!(written.is_ok(), "{written:?}");
    out
}

/// PNG を非乗算 RGBA8 の画素 (幅・高さ・画素) にデコードする。読めなければ `None`。
///
/// グレースケール・RGB・パレット・16 ビットの PNG も RGBA8 に直す。
pub fn decode_rgba_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        // `normalize_to_color8` はパレットを展開するので、ここには来ない。
        png::ColorType::Indexed => return None,
    };
    let expected = info.width as usize * info.height as usize * 4;
    (rgba.len() == expected).then_some((info.width, info.height, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_names() {
        assert!(is_valid_texture_name("hair_front-1.v2"));
        for bad in ["", ".hidden", "a/b", "a\\b", "..", "髪", "a b"] {
            assert!(!is_valid_texture_name(bad), "{bad:?}");
        }
        assert!(!is_valid_texture_name(
            &"a".repeat(MAX_TEXTURE_NAME_LEN + 1)
        ));
    }

    #[test]
    fn png_size_reads_ihdr() {
        assert_eq!(png_size(&solid_png(3, 2, [255; 4])), Some((3, 2)));
        assert_eq!(png_size(b"not a png"), None);
        assert_eq!(png_size(&PNG_SIGNATURE), None);
    }

    #[test]
    fn png_roundtrip() {
        let pixels: Vec<u8> = (0..2 * 3 * 4).map(|i| i as u8 * 10).collect();
        let png = encode_rgba_png(2, 3, &pixels);
        assert_eq!(png_size(&png), Some((2, 3)));
        assert_eq!(decode_rgba_png(&png), Some((2, 3, pixels)));
        assert_eq!(decode_rgba_png(b"not a png"), None);
    }
}
