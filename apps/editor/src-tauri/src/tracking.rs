//! プレビューモードのフェイストラッキング (T0636) のためのこと: カメラの許可と、記録の保存。
//! 検出とパラメータへの変換は webview の中 (`@ugoe/face-tracking` と `ugoe-wasm`) で行う。

use std::path::Path;

use tauri::webview::{PermissionKind, PermissionResponse};

use crate::error::{EditorError, IpcResult};
use crate::project::absolute_path;

/// トラッキングの記録 (JSON) を書く。パスは保存のダイアログで利用者が選んだ絶対パス。`.json` だけを書く。
#[tauri::command]
pub fn write_json_file(path: String, contents: String) -> IpcResult<()> {
    let path = absolute_path(&path)?;
    if !is_json_path(&path) {
        return Err(EditorError::NotJsonFile(path.display().to_string()).into());
    }
    std::fs::write(&path, contents).map_err(|e| EditorError::io(&path, e).into())
}

fn is_json_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// webview からの許可の要求への応答。フェイストラッキングのカメラだけを許す (ビューアと同じ)。
/// ページはアプリに同梱したものだけなので、カメラを使うのは利用者がトラッキングを始めたときに限られる。
/// macOS では、これとは別に OS の許可 (`Info.plist` の `NSCameraUsageDescription`) が要る。
pub fn permission_response(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Camera => PermissionResponse::Allow,
        _ => PermissionResponse::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_json_files_only() {
        assert!(is_json_path(Path::new("tracking.JSON")));
        assert!(!is_json_path(Path::new("tracking.txt")));
        let dir = std::env::temp_dir().join(format!("ugoe-editor-test-{}", std::process::id()));
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
        // 相対パスは作業ディレクトリに書いてしまうので断る (`docs/editor.md#プロジェクト`)。
        let err = write_json_file("ugoe-editor-rec.json".into(), "{}".into()).unwrap_err();
        assert_eq!(err.code, "editor.relative_path");
        assert!(!Path::new("ugoe-editor-rec.json").exists());
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
