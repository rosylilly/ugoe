//! ビューアのバックエンド。モデルの描画とパラメータの操作はすべて webview の wasm (`ugoe-wasm`) で行い、
//! ここはファイルの読み書きとカメラの許可など、webview だけではできないことに限る (`apps/viewer/README.md`)。

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

/// モデルかモジュールのファイルを読み、バイト列をそのまま返す (JS では `ArrayBuffer`)。
/// 形式の判別と読み込みはフロントエンドの wasm が行う。
#[tauri::command]
fn read_model_file(path: String) -> Result<Response, String> {
    let path = Path::new(&path);
    if !is_readable_path(path) {
        return Err(format!(
            "not a model or module file (.ugoe, .json or .ugoemod): {}",
            path.display()
        ));
    }
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if len > MAX_FILE_BYTES {
        return Err(format!(
            "{}: file is too large ({len} bytes)",
            path.display()
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Response::new(bytes))
}

/// JSON のファイル (トラッキングの記録など) を書く。パスは保存のダイアログで利用者が選んだもの。
#[tauri::command]
fn write_json_file(path: String, contents: String) -> Result<(), String> {
    let path = Path::new(&path);
    if !is_json_path(path) {
        return Err(format!("not a JSON file: {}", path.display()));
    }
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn is_json_path(path: &Path) -> bool {
    has_extension(path, &["json"])
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
        .on_permission_request(|_, kind| permission_response(kind))
        .invoke_handler(tauri::generate_handler![
            read_model_file,
            write_json_file,
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
        write_json_file(path.to_string_lossy().into_owned(), "{}".into()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
        assert!(
            write_json_file(
                dir.join("rec.sh").to_string_lossy().into_owned(),
                "x".into()
            )
            .is_err()
        );
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
