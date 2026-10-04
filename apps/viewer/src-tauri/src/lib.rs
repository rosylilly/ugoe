//! ビューアのバックエンド。モデルの描画とパラメータの操作はすべて webview の wasm (`ugoe-wasm`) で行い、
//! ここはファイルの読み書きとカメラの許可など、webview だけではできないことに限る (`apps/viewer/README.md`)。

mod atomic_write;
mod error;

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tauri::ipc::Response;
use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::{DragDropEvent, Manager, State, WebviewEvent, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use atomic_write::write_atomic;
use error::CommandError;

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
    task: impl FnOnce() -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| CommandError::internal(e.to_string()))?
}

/// モデルかモジュールのファイルを読み、バイト列をそのまま返す (JS では `ArrayBuffer`)。
/// 形式の判別と読み込みはフロントエンドの wasm が行う。読めるのは、利用者が渡したパス
/// (ダイアログ・ドロップ・起動時の引数。前に渡したものも覚えている) だけ (`FileAccess`)。
#[tauri::command]
async fn read_model_file(
    access: State<'_, FileAccess>,
    path: String,
) -> Result<Response, CommandError> {
    let path = PathBuf::from(path);
    access.check_readable(&path)?;
    blocking(move || read_model(&path, MAX_FILE_BYTES))
        .await
        .map(Response::new)
}

/// `read_model_file` の中身。`max` バイトより大きいファイルは読まない。
fn read_model(path: &Path, max: u64) -> Result<Vec<u8>, CommandError> {
    if !is_readable_path(path) {
        return Err(CommandError::not_model(path));
    }
    let too_large = |len: u64| CommandError::too_large(path, len, max);
    let read_failed = |e: std::io::Error| CommandError::read_failed(path, &e);
    let file = std::fs::File::open(path).map_err(read_failed)?;
    let len = file.metadata().map_err(read_failed)?.len();
    if len > max {
        return Err(too_large(len));
    }
    // 大きさを見てから読むまでの間にファイルが大きくなっても上限を超えて読まないよう、上限 + 1 バイトまでに限って読む。
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(read_failed)?;
    if bytes.len() as u64 > max {
        return Err(too_large(bytes.len() as u64));
    }
    Ok(bytes)
}

/// JSON のファイル (トラッキングの記録など) を書く。パスは `save_file_dialog` で利用者が選んだもの (1 回だけ書ける)。
/// 一時ファイルを経て置き換えるので、失敗しても前のファイルは残る (`atomic_write`)。
#[tauri::command]
async fn write_json_file(
    access: State<'_, FileAccess>,
    path: String,
    contents: String,
) -> Result<(), CommandError> {
    let path = PathBuf::from(path);
    access.take_writable(&path)?;
    blocking(move || write_json(&path, &contents)).await
}

fn write_json(path: &Path, contents: &str) -> Result<(), CommandError> {
    if !is_json_path(path) {
        return Err(CommandError::not_json(path));
    }
    write_atomic(path, contents.as_bytes()).map_err(|e| CommandError::write_failed(path, &e))
}

fn is_json_path(path: &Path) -> bool {
    has_extension(path, &["json"])
}

/// `write_png_file` に保存先のパスを渡すヘッダー (`packages/player/src/screenshot.ts` の `PNG_PATH_HEADER` と同じ)。
/// バイト列は本文でそのまま受け取るので、パスはヘッダーに URI の形 (`encodeURIComponent`) で入れる。
const PNG_PATH_HEADER: &str = "x-ugoe-path";

/// PNG のファイルの先頭 8 バイト。
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// スクリーンショット (T0420) の PNG を書く。本文は PNG のバイト列、パスはヘッダー `x-ugoe-path`
/// (`save_file_dialog` で利用者が選んだもの。1 回だけ書ける)。拡張子が `.png` で、中身が PNG のものだけを書く。
/// 一時ファイルを経て置き換えるので、失敗しても前のファイルは残る (`atomic_write`)。
#[tauri::command]
async fn write_png_file(
    access: State<'_, FileAccess>,
    request: tauri::ipc::Request<'_>,
) -> Result<(), CommandError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(CommandError::not_png_data());
    };
    let header = request
        .headers()
        .get(PNG_PATH_HEADER)
        .ok_or_else(|| CommandError::missing_path(PNG_PATH_HEADER))?
        .to_str()
        .map_err(|e| CommandError::invalid_path(format!("{PNG_PATH_HEADER}: {e}")))?;
    let path = PathBuf::from(percent_decode(header).map_err(CommandError::invalid_path)?);
    access.take_writable(&path)?;
    // 要求は借りものなので、別のスレッドへはバイト列の写しを渡す。
    let bytes = bytes.clone();
    blocking(move || write_png(&path, &bytes)).await
}

fn write_png(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    if !has_extension(path, &["png"]) {
        return Err(CommandError::not_png(path));
    }
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err(CommandError::not_png_data());
    }
    write_atomic(path, bytes).map_err(|e| CommandError::write_failed(path, &e))
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

/// 起動時の引数で渡されたモデルのパス (`ugoe-viewer model.ugoe`、OS の「このアプリで開く」)。読めるパスに加える。
#[tauri::command]
fn initial_file(access: State<'_, FileAccess>) -> Option<String> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && is_model_path(Path::new(a)))?;
    access.grant_readable([PathBuf::from(&path)]);
    Some(path)
}

/// 開くダイアログを開き、選ばれたファイルのパスを返す (取り消したら空)。選ばれたパスは読めるパスに加える。
/// 文言 (`title`・`filter_name`) と拡張子のフィルタは画面が渡す。ダイアログをバックエンドで開くのは、
/// 利用者が選んだパスをバックエンドが知るため (T0727)。
#[tauri::command]
async fn open_file_dialog(
    window: WebviewWindow,
    access: State<'_, FileAccess>,
    title: String,
    filter_name: String,
    extensions: Vec<String>,
    multiple: bool,
) -> Result<Vec<String>, CommandError> {
    let paths = blocking(move || {
        let extensions: Vec<&str> = extensions.iter().map(String::as_str).collect();
        let dialog = window
            .dialog()
            .file()
            .set_title(title)
            .add_filter(filter_name, &extensions);
        // tauri-plugin-dialog の `open` と同じく、親のウィンドウは Windows と macOS でだけ付ける。
        #[cfg(any(windows, target_os = "macos"))]
        let dialog = dialog.set_parent(&window);
        let picked = if multiple {
            dialog.blocking_pick_files().unwrap_or_default()
        } else {
            dialog.blocking_pick_file().into_iter().collect()
        };
        picked
            .into_iter()
            .map(|p| {
                p.into_path()
                    .map(path_string)
                    .map_err(|e| CommandError::dialog_failed(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;
    access.grant_readable(paths.iter().map(PathBuf::from));
    Ok(paths)
}

/// 保存のダイアログを開き、選ばれたパスを返す (取り消したら `null`)。拡張子が無ければ `extension` を付ける
/// (Linux のダイアログはフィルタの拡張子を付けずに返すことがある)。返したパスは 1 回だけ書ける
/// (`write_json_file`・`write_png_file`)。書ける形式は `json` と `png` だけ。
#[tauri::command]
async fn save_file_dialog(
    window: WebviewWindow,
    access: State<'_, FileAccess>,
    title: String,
    default_name: String,
    filter_name: String,
    extension: String,
) -> Result<Option<String>, CommandError> {
    if !WRITABLE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(CommandError::unsupported_save_type(&extension));
    }
    let picked = blocking(move || {
        window
            .dialog()
            .file()
            .set_title(title)
            .set_file_name(default_name)
            .add_filter(filter_name, &[extension.as_str()])
            .set_parent(&window)
            .blocking_save_file()
            .map(|p| {
                p.into_path()
                    .map_err(|e| CommandError::dialog_failed(e.to_string()))
            })
            .transpose()
            .map(|p| p.map(|p| with_extension(p, &extension)))
    })
    .await?;
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path_string(path);
    access.grant_writable(PathBuf::from(&path));
    Ok(Some(path))
}

/// 保存のダイアログで選べる (書ける) 拡張子。
const WRITABLE_EXTENSIONS: &[&str] = &["json", "png"];

/// 拡張子 (大文字小文字は区別しない) が無ければ付け足す。
fn with_extension(path: PathBuf, extension: &str) -> PathBuf {
    if has_extension(&path, &[extension]) {
        return path;
    }
    let mut name = path.into_os_string();
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}

/// 画面に渡すパスの文字列。画面はこの文字列のまま返してくるので、許したパスもこの文字列から作る。
fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

/// 覚えておく読めるパスの数の上限 (古いものから忘れる)。
const MAX_READABLE_PATHS: usize = 1000;

/// webview に読み書きを許したパス (T0727)。webview で思わぬスクリプトが動いても、利用者が選んでいない
/// ファイルを読み書きできないようにする。
///
/// - 読める: 開くダイアログで選んだ・ウィンドウにドロップした・起動時の引数のファイル。覚えた場所 (`path:`) から
///   読み直せるよう、アプリのデータのディレクトリの `readable-files.json` に覚える (新しいものから
///   `MAX_READABLE_PATHS` 件)。
/// - 書ける: 保存のダイアログで選んだパス。1 回書いたら忘れる (覚えない)。
#[derive(Default)]
struct FileAccess {
    readable: Mutex<ReadableFiles>,
    writable: Mutex<HashSet<PathBuf>>,
}

/// 読めるパスの一覧 (古い順) と、それを覚えるファイル。
#[derive(Default)]
struct ReadableFiles {
    paths: Vec<PathBuf>,
    store: Option<PathBuf>,
}

impl FileAccess {
    /// `store` に覚えた読めるパスを読み込む (無い・壊れていれば空から始める)。
    fn load(store: Option<PathBuf>) -> Self {
        let paths = store
            .as_deref()
            .and_then(|s| std::fs::read(s).ok())
            .and_then(|bytes| serde_json::from_slice::<Vec<PathBuf>>(&bytes).ok())
            .unwrap_or_default();
        Self {
            readable: Mutex::new(ReadableFiles { paths, store }),
            writable: Mutex::default(),
        }
    }

    fn check_readable(&self, path: &Path) -> Result<(), CommandError> {
        if lock(&self.readable).paths.iter().any(|p| p == path) {
            Ok(())
        } else {
            Err(CommandError::not_chosen(path))
        }
    }

    /// 読めるパスに加え (すでにあれば新しくし)、覚える。覚えるファイルに書けなくても、このセッションでは読める。
    fn grant_readable(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut readable = lock(&self.readable);
        for path in paths {
            readable.paths.retain(|p| p != &path);
            readable.paths.push(path);
        }
        let excess = readable.paths.len().saturating_sub(MAX_READABLE_PATHS);
        readable.paths.drain(..excess);
        if let Some(store) = &readable.store {
            let _ = save_paths(store, &readable.paths);
        }
    }

    fn grant_writable(&self, path: PathBuf) {
        lock(&self.writable).insert(path);
    }

    /// 書けるパスなら、書けるパスから除いて `Ok` を返す (同じパスにもう 1 回書くには、またダイアログで選ぶ)。
    fn take_writable(&self, path: &Path) -> Result<(), CommandError> {
        if lock(&self.writable).remove(path) {
            Ok(())
        } else {
            Err(CommandError::not_chosen_for_save(path))
        }
    }
}

fn save_paths(store: &Path, paths: &[PathBuf]) -> Result<(), String> {
    if let Some(dir) = store.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec(paths).map_err(|e| e.to_string())?;
    write_atomic(store, &json).map_err(|e| e.to_string())
}

/// ロックを取る。ほかのスレッドがロック中に panic していても、中身 (パスの一覧) はそのまま使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// ウィンドウにドロップされたファイルを読めるパスに加える。Tauri はこのハンドラーをドロップのイベント
/// (`tauri://drag-drop`) を webview へ送った直後に、同じメインスレッドで呼ぶ。webview がそのイベントを受けて
/// `read_model_file` を呼ぶのはその後になる (IPC もメインスレッドで受ける) ので、読むときには加えてある。
fn on_webview_event(webview: &tauri::Webview, event: &WebviewEvent) {
    if let WebviewEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
        webview
            .state::<FileAccess>()
            .grant_readable(paths.iter().cloned().map(path_string).map(PathBuf::from));
    }
}

/// 読めるパスを覚えるファイル (アプリのデータのディレクトリ)。
const READABLE_FILES_STORE: &str = "readable-files.json";

pub fn run() {
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let store = app
                .path()
                .app_data_dir()
                .ok()
                .map(|d| d.join(READABLE_FILES_STORE));
            app.manage(FileAccess::load(store));
            Ok(())
        })
        .on_permission_request(|_, kind| permission_response(kind))
        .on_webview_event(on_webview_event)
        .invoke_handler(tauri::generate_handler![
            initial_file,
            open_file_dialog,
            read_model_file,
            save_file_dialog,
            write_json_file,
            write_png_file
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
        write_json(&path, "{}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
        assert_eq!(
            write_json(&dir.join("rec.sh"), "x").unwrap_err().code,
            "file.not_json"
        );
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
        let too_large = read_model(&path, 7).unwrap_err();
        assert_eq!(too_large.code, "file.too_large");
        assert_eq!(too_large.args["size"], 8);
        assert_eq!(too_large.args["max"], 7);
        assert_eq!(too_large.args["path"], path.display().to_string());
        let other = dir.join("model.txt");
        std::fs::write(&other, b"{}").unwrap();
        assert_eq!(read_model(&other, 8).unwrap_err().code, "file.not_model");
        let missing = read_model(&dir.join("missing.ugoe"), 8).unwrap_err();
        assert_eq!(missing.code, "file.not_found");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn runs_file_tasks_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker =
            tauri::async_runtime::block_on(blocking(|| Ok(std::thread::current().id()))).unwrap();
        assert_ne!(worker, caller);
        let err: Result<(), CommandError> =
            tauri::async_runtime::block_on(blocking(|| Err(CommandError::not_png_data())));
        assert_eq!(err.unwrap_err().code, "request.not_png_data");
        // 中でパニックしたら `internal` のコードのエラーにする (スタジオと同じ。T0819)。
        let panicked: Result<(), CommandError> = tauri::async_runtime::block_on(blocking(|| {
            std::panic::resume_unwind(Box::new("boom"))
        }));
        assert_eq!(panicked.unwrap_err().code, "internal");
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
        assert_eq!(
            write_png(&dir.join("shot.sh"), &png).unwrap_err().code,
            "file.not_png"
        );
        assert_eq!(
            write_png(&dir.join("other.png"), b"not a png")
                .unwrap_err()
                .code,
            "request.not_png_data"
        );
        assert!(!dir.join("other.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_only_the_files_the_user_chose() {
        let access = FileAccess::load(None);
        let model = PathBuf::from("/home/u/model.ugoe");
        assert!(access.check_readable(&model).is_err());
        access.grant_readable([model.clone(), PathBuf::from("/home/u/hair.ugoemod")]);
        assert!(access.check_readable(&model).is_ok());
        assert!(
            access
                .check_readable(Path::new("/home/u/hair.ugoemod"))
                .is_ok()
        );
        // 同じディレクトリのほかのファイル・ほかのアプリの設定は読めない。
        assert!(
            access
                .check_readable(Path::new("/home/u/other.ugoe"))
                .is_err()
        );
        assert!(
            access
                .check_readable(Path::new("/home/u/.config/app/settings.json"))
                .unwrap_err()
                .code
                == "file.not_chosen"
        );
    }

    #[test]
    fn remembers_readable_files_across_sessions() {
        let dir = std::env::temp_dir().join(format!("ugoe-viewer-grants-{}", std::process::id()));
        let store = dir.join("data").join(READABLE_FILES_STORE);
        let access = FileAccess::load(Some(store.clone()));
        access.grant_readable([PathBuf::from("/a.ugoe"), PathBuf::from("/b.ugoemod")]);
        // 読み直したアプリ (次のセッション) でも読める。
        let next = FileAccess::load(Some(store.clone()));
        assert!(next.check_readable(Path::new("/a.ugoe")).is_ok());
        assert!(next.check_readable(Path::new("/b.ugoemod")).is_ok());
        // 壊れたファイルなら空から始める。
        std::fs::write(&store, b"not json").unwrap();
        let broken = FileAccess::load(Some(store));
        assert!(broken.check_readable(Path::new("/a.ugoe")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn forgets_the_oldest_readable_files() {
        let access = FileAccess::load(None);
        access.grant_readable((0..MAX_READABLE_PATHS).map(|i| PathBuf::from(format!("/{i}.ugoe"))));
        // 前に許したパスをもう 1 回許すと新しくなり、先に忘れられない。
        access.grant_readable([PathBuf::from("/0.ugoe")]);
        access.grant_readable([PathBuf::from("/new.ugoe")]);
        assert_eq!(lock(&access.readable).paths.len(), MAX_READABLE_PATHS);
        assert!(access.check_readable(Path::new("/0.ugoe")).is_ok());
        assert!(access.check_readable(Path::new("/new.ugoe")).is_ok());
        assert!(access.check_readable(Path::new("/1.ugoe")).is_err());
        assert!(access.check_readable(Path::new("/2.ugoe")).is_ok());
    }

    #[test]
    fn writes_only_once_to_the_path_chosen_in_the_save_dialog() {
        let access = FileAccess::load(None);
        let shot = PathBuf::from("/home/u/shot.png");
        assert!(access.take_writable(&shot).unwrap_err().code == "file.not_chosen_for_save");
        access.grant_writable(shot.clone());
        assert!(
            access
                .take_writable(Path::new("/home/u/.bashrc.png"))
                .is_err()
        );
        assert!(access.take_writable(&shot).is_ok());
        assert!(access.take_writable(&shot).is_err());
        // 書けるパスは読めるパスにならない。
        assert!(access.check_readable(&shot).is_err());
    }

    #[test]
    fn adds_the_extension_of_the_save_dialog() {
        assert_eq!(
            with_extension(PathBuf::from("/home/u/rec"), "json"),
            PathBuf::from("/home/u/rec.json")
        );
        assert_eq!(
            with_extension(PathBuf::from("/home/u/shot.PNG"), "png"),
            PathBuf::from("/home/u/shot.PNG")
        );
        assert_eq!(
            with_extension(PathBuf::from("/a/shot.png.txt"), "png"),
            PathBuf::from("/a/shot.png.txt.png")
        );
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
