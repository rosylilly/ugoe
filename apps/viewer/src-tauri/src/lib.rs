//! ビューアのバックエンド。モデルの描画とパラメータの操作はすべて webview の wasm (`ugoe-wasm`) で行い、
//! ここはファイルの読み込みなど、webview だけではできないことに限る (`apps/viewer/README.md`)。

use std::path::Path;

use tauri::ipc::Response;

/// 読み込むファイルの大きさの上限 (誤って巨大なファイルを選んだときにメモリを使い果たさないため)。
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// ビューアが開けるファイルか (拡張子で判断する)。`.ugoe` バンドルと JSON のモデル。
fn is_model_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ugoe") || e.eq_ignore_ascii_case("json"))
}

/// モデルのファイルを読み、バイト列をそのまま返す (JS では `ArrayBuffer`)。
/// 形式の判別と読み込みはフロントエンドの wasm が行う。
#[tauri::command]
fn read_model_file(path: String) -> Result<Response, String> {
    let path = Path::new(&path);
    if !is_model_path(path) {
        return Err(format!(
            "not a model file (.ugoe or .json): {}",
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
        .invoke_handler(tauri::generate_handler![read_model_file, initial_file])
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
    }
}
