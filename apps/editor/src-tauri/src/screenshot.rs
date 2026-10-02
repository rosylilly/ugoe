//! ビューポートのスクリーンショット (T0420) の保存。画像は webview の中で `WebPuppet.read_pixels()` から取り、
//! PNG に変換してから送ってくる (`src/lib/viewport/saveScreenshot.ts`)。ここはファイルに書くだけ。
//! 設計: `docs/editor.md#スクリーンショット`。

use std::path::Path;

use crate::project::absolute_path;

/// 保存先のパスを渡すヘッダー (`src/lib/viewport/screenshot.ts` の `PNG_PATH_HEADER` と同じ)。
/// バイト列は本文でそのまま受け取るので、パスはヘッダーに URI の形 (`encodeURIComponent`) で入れる。
const PATH_HEADER: &str = "x-ugoe-path";

/// PNG のファイルの先頭 8 バイト。
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// PNG の画像を書く。本文は PNG のバイト列、パスはヘッダー `x-ugoe-path` (保存のダイアログで利用者が選んだ絶対パス)。
/// 拡張子が `.png` で、中身が PNG のものだけを書く。
#[tauri::command]
pub fn write_png_file(request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the bytes of a PNG image".into());
    };
    let header = request
        .headers()
        .get(PATH_HEADER)
        .ok_or_else(|| format!("missing the {PATH_HEADER} header"))?
        .to_str()
        .map_err(|e| format!("{PATH_HEADER}: {e}"))?;
    write_png(&percent_decode(header)?, bytes)
}

/// [`write_png_file`] の中身 (テストから呼ぶ)。
fn write_png(path: &str, bytes: &[u8]) -> Result<(), String> {
    let path = absolute_path(path)?;
    if !is_png_path(&path) {
        return Err(format!("not a PNG file: {}", path.display()));
    }
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("the data is not a PNG image".into());
    }
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn is_png_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

/// `encodeURIComponent` で符号化した文字列を戻す (`%XX` を 1 バイトに。結果は UTF-8)。
fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let byte = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("invalid percent-encoding: {text}"))?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| format!("invalid percent-encoding: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_encoded_paths() {
        assert_eq!(
            percent_decode("%2Fhome%2F%E3%81%82%20b%2Fshot.png").unwrap(),
            "/home/あ b/shot.png"
        );
        assert_eq!(percent_decode("C%3A%5Cshot.png").unwrap(), "C:\\shot.png");
        assert!(percent_decode("%2").is_err());
        assert!(percent_decode("%zz").is_err());
        assert!(percent_decode("%FF").is_err());
    }

    #[test]
    fn writes_png_files_only() {
        let dir = std::env::temp_dir().join(format!("ugoe-editor-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = [PNG_SIGNATURE, b"rest"].concat();
        let path = dir.join("shot.PNG");
        write_png(path.to_str().unwrap(), &png).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), png);
        assert!(write_png(dir.join("shot.sh").to_str().unwrap(), &png).is_err());
        assert!(write_png(dir.join("other.png").to_str().unwrap(), b"not a png").is_err());
        assert!(!dir.join("other.png").exists());
        let err = write_png("shot.png", &png).unwrap_err();
        assert!(err.contains("must be absolute"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
