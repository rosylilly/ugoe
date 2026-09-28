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
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let pixels: Vec<u8> = (0..u64::from(width) * u64::from(height))
        .flat_map(|_| rgba)
        .collect();
    // メモリ上への書き込みは、寸法とデータ長が合っていれば失敗しない。
    let written = encoder.write_header().and_then(|mut writer| {
        writer.write_image_data(&pixels)?;
        writer.finish()
    });
    debug_assert!(written.is_ok(), "{written:?}");
    out
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
}
