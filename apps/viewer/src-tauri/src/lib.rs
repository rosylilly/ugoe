//! ビューアのバックエンド。モデルの描画とパラメータの操作はすべて webview の wasm (`ugoe-wasm`) で行い、
//! ここはファイルの読み書きとカメラの許可など、webview だけではできないことに限る (`apps/viewer/README.md`)。

use std::io::Read;
use std::path::Path;

use tauri::ipc::Response;
use tauri::webview::{PermissionKind, PermissionResponse};

/// 読み込むファイルの大きさの上限 (誤って巨大なファイルを選んだときにメモリを使い果たさないため)。
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// モデルのファイルか (拡張子で判断する)。`.ugoe` バンドルと JSON のモデル。
fn is_model_path(path: &Path) -> bool {
    has_extension(path, &["ugoe", "json"])
}

/// ビューアが読めるファイルか。モデルと、着せ替えのモジュール (`.ugoemod`、T0706)。
fn is_readable_path(path: &Path) -> bool {
    is_model_path(path) || has_extension(path, &["ugoemod"])
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// ファイルの読み書きを、Tauri のメインスレッド (イベントのループ) ではなく読み書き用のスレッドで行う。
/// コマンドは `async` にして、読み書きはここに渡す (同期のコマンドはメインスレッドで動き、
/// 大きなファイルやネットワークのドライブを読む間ウィンドウが止まるため。T0726)。
async fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| format!("the file task failed: {e}"))?
}

/// モデルかモジュールのファイルを読み、バイト列をそのまま返す (JS では `ArrayBuffer`)。
/// 形式の判別と読み込みはフロントエンドの wasm が行う。
#[tauri::command]
async fn read_model_file(path: String) -> Result<Response, String> {
    blocking(move || read_model(Path::new(&path), MAX_FILE_BYTES))
        .await
        .map(Response::new)
}

/// `read_model_file` の中身。`max` バイトより大きいファイルは読まない。
fn read_model(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    if !is_readable_path(path) {
        return Err(format!(
            "not a model or module file (.ugoe, .json or .ugoemod): {}",
            path.display()
        ));
    }
    let too_large = |len: u64| format!("{}: file is too large ({len} bytes)", path.display());
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = file
        .metadata()
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if len > max {
        return Err(too_large(len));
    }
    // 大きさを見てから読むまでの間にファイルが大きくなっても上限を超えて読まないよう、上限 + 1 バイトまでに限って読む。
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() as u64 > max {
        return Err(too_large(bytes.len() as u64));
    }
    Ok(bytes)
}

/// JSON のファイル (トラッキングの記録など) を書く。パスは保存のダイアログで利用者が選んだもの。
#[tauri::command]
async fn write_json_file(path: String, contents: String) -> Result<(), String> {
    blocking(move || write_json(Path::new(&path), &contents)).await
}

fn write_json(path: &Path, contents: &str) -> Result<(), String> {
    if !is_json_path(path) {
        return Err(format!("not a JSON file: {}", path.display()));
    }
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn is_json_path(path: &Path) -> bool {
    has_extension(path, &["json"])
}

/// `write_png_file` に保存先のパスを渡すヘッダー (`src/lib/screenshot.ts` の `PNG_PATH_HEADER` と同じ)。
/// バイト列は本文でそのまま受け取るので、パスはヘッダーに URI の形 (`encodeURIComponent`) で入れる。
const PNG_PATH_HEADER: &str = "x-ugoe-path";

/// PNG のファイルの先頭 8 バイト。
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// スクリーンショット (T0420) の PNG を書く。本文は PNG のバイト列、パスはヘッダー `x-ugoe-path`
/// (保存のダイアログで利用者が選んだもの)。拡張子が `.png` で、中身が PNG のものだけを書く。
#[tauri::command]
async fn write_png_file(request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the bytes of a PNG image".into());
    };
    let header = request
        .headers()
        .get(PNG_PATH_HEADER)
        .ok_or_else(|| format!("missing the {PNG_PATH_HEADER} header"))?
        .to_str()
        .map_err(|e| format!("{PNG_PATH_HEADER}: {e}"))?;
    let path = percent_decode(header)?;
    // 要求は借りものなので、別のスレッドへはバイト列の写しを渡す。
    let bytes = bytes.clone();
    blocking(move || write_png(Path::new(&path), &bytes)).await
}

fn write_png(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if !has_extension(path, &["png"]) {
        return Err(format!("not a PNG file: {}", path.display()));
    }
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("the data is not a PNG image".into());
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// `encodeURIComponent` で符号化した文字列を戻す (`%XX` を 1 バイトに。結果は UTF-8)。
fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("invalid percent-encoding: {text}"))?;
            out.push(hex);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| format!("invalid percent-encoding: {e}"))
}

/// webview からの許可の要求への応答。フェイストラッキング (T0703) のカメラだけを許す。
/// ページはアプリに同梱したものだけなので、カメラを使うのは利用者がトラッキングを始めたときに限られる。
/// macOS では、これとは別に OS の許可 (`Info.plist` の `NSCameraUsageDescription`) が要る。
fn permission_response(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Camera => PermissionResponse::Allow,
        _ => PermissionResponse::Default,
    }
}

/// 起動時の引数で渡されたモデルのパス (`ugoe-viewer model.ugoe`、OS の「このアプリで開く」)。
#[tauri::command]
fn initial_file() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && is_model_path(Path::new(a)))
}

pub fn run() {
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .on_permission_request(|_, kind| permission_response(kind))
        .invoke_handler(tauri::generate_handler![
            read_model_file,
            write_json_file,
            write_png_file,
            initial_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ugoe viewer");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bundles_and_json_only() {
        assert!(is_model_path(Path::new("a/b/model.ugoe")));
        assert!(is_model_path(Path::new("model.JSON")));
        assert!(!is_model_path(Path::new("model.ugoeproj")));
        assert!(!is_model_path(Path::new("model")));
        assert!(!is_model_path(Path::new("character.psd")));
        assert!(!is_model_path(Path::new("hair.ugoemod")));
    }

    #[test]
    fn reads_modules_too() {
        assert!(is_readable_path(Path::new("a/hair.UGOEMOD")));
        assert!(is_readable_path(Path::new("model.ugoe")));
        assert!(!is_readable_path(Path::new("model.ugoeproj")));
    }

    #[test]
    fn writes_json_files_only() {
        assert!(is_json_path(Path::new("tracking.JSON")));
        assert!(!is_json_path(Path::new("tracking.txt")));
        let dir = std::env::temp_dir().join(format!("ugoe-viewer-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rec.json");
        tauri::async_runtime::block_on(write_json_file(
            path.to_string_lossy().into_owned(),
            "{}".into(),
        ))
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
        assert!(write_json(&dir.join("rec.sh"), "x").is_err());
        assert!(!dir.join("rec.sh").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_model_files_within_the_limit() {
        let dir = std::env::temp_dir().join(format!("ugoe-viewer-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.ugoe");
        std::fs::write(&path, b"UGOE1234").unwrap();
        assert_eq!(read_model(&path, 8).unwrap(), b"UGOE1234");
        assert!(read_model(&path, 7).unwrap_err().contains("too large"));
        let other = dir.join("model.txt");
        std::fs::write(&other, b"{}").unwrap();
        assert!(read_model(&other, 8).unwrap_err().contains("not a model"));
        assert!(read_model(&dir.join("missing.ugoe"), 8).is_err());
        // コマンドは別のスレッドで読み、同じ結果を返す。
        assert!(
            tauri::async_runtime::block_on(read_model_file(other.to_string_lossy().into_owned()))
                .is_err()
        );
        assert!(
            tauri::async_runtime::block_on(read_model_file(path.to_string_lossy().into_owned()))
                .is_ok()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn runs_file_tasks_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker =
            tauri::async_runtime::block_on(blocking(|| Ok(std::thread::current().id()))).unwrap();
        assert_ne!(worker, caller);
        let err: Result<(), String> =
            tauri::async_runtime::block_on(blocking(|| Err("failed".into())));
        assert_eq!(err.unwrap_err(), "failed");
    }

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
        let dir = std::env::temp_dir().join(format!("ugoe-viewer-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = [PNG_SIGNATURE, b"rest"].concat();
        let path = dir.join("shot.PNG");
        write_png(&path, &png).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), png);
        assert!(write_png(&dir.join("shot.sh"), &png).is_err());
        assert!(write_png(&dir.join("other.png"), b"not a png").is_err());
        assert!(!dir.join("other.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn allows_only_the_camera() {
        assert!(matches!(
            permission_response(PermissionKind::Camera),
            PermissionResponse::Allow
        ));
        assert!(matches!(
            permission_response(PermissionKind::Microphone),
            PermissionResponse::Default
        ));
    }
}
