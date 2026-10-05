//! webview に読み書きを許したパスと、保存のダイアログが返したパスの拡張子の補い (T0037)。
//!
//! エディタ・ビューア・スタジオは、webview で思わぬスクリプトが動いても利用者が選んでいないファイルを読み書きできないように、
//! ファイルのダイアログをバックエンドで開き、選ばれたパスだけを読み書きさせる (T0727・T0838・T1616)。3 つのアプリに
//! 写しで並んでいた共通の部分をここに置く。
//!
//! - [`FileAccess`] は、開くダイアログで選んだパス (読める。このセッションの間) と、保存のダイアログで選んだパス (1 回だけ書ける) を持つ。
//! - [`WritablePaths`] は、1 回だけ書けるパスだけを持つ (読めるパスの持ち方が違うビューアが使う)。
//! - [`with_extension`] は、保存のダイアログが拡張子を付けずに返したパスに付け足す。
//!
//! アプリごとの規則 (開いているプロジェクト・プロファイルにあるパスも読める、読めるパスを覚えておく、断るときのエラーのコード)
//! と、Tauri に依存するダイアログの組み立ては各アプリに残す。

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// webview に読み書きを許したパス。読める: 開くダイアログで選んだ・ウィンドウにドロップしたパス (覚えるのはこのセッションの間)。
/// 書ける: 保存のダイアログで選んだパス (1 回書いたら忘れる)。書けるパスは読めるパスにならない。
#[derive(Debug, Default)]
pub struct FileAccess {
    /// 開くダイアログで選んだ・ドロップしたパス。
    chosen: Mutex<HashSet<PathBuf>>,
    /// 保存のダイアログで選び、まだ書いていないパス。
    writable: WritablePaths,
}

impl FileAccess {
    /// 開くダイアログで選んだ・ドロップしたパスを、読めるパスに加える。
    pub fn grant_readable(&self, paths: impl IntoIterator<Item = PathBuf>) {
        lock(&self.chosen).extend(paths);
    }

    /// 開くダイアログで選んだ・ドロップしたパスか。パスは文字列のまま比べる (アプリは画面に渡した文字列から作ったパスを
    /// 加え、画面が返した文字列から作ったパスで確かめる)。
    pub fn is_chosen(&self, path: &Path) -> bool {
        lock(&self.chosen).contains(path)
    }

    /// 保存のダイアログで選んだパスを、1 回だけ書けるパスにする ([`WritablePaths::grant`])。
    pub fn grant_writable(&self, path: PathBuf) {
        self.writable.grant(path);
    }

    /// 書けるパスなら、書けるパスから除いて `true` を返す ([`WritablePaths::take`])。
    pub fn take_writable(&self, path: &Path) -> bool {
        self.writable.take(path)
    }
}

/// 保存のダイアログで選び、まだ書いていないパス。1 回書いたら忘れる (同じパスにもう 1 回書くには、またダイアログで選ぶ)。
#[derive(Debug, Default)]
pub struct WritablePaths(Mutex<HashSet<PathBuf>>);

impl WritablePaths {
    /// 保存のダイアログで選んだパスを、1 回だけ書けるパスにする。
    pub fn grant(&self, path: PathBuf) {
        lock(&self.0).insert(path);
    }

    /// 書けるパスなら、書けるパスから除いて `true` を返す。
    pub fn take(&self, path: &Path) -> bool {
        lock(&self.0).remove(path)
    }
}

/// `.` と拡張子 (ASCII の大文字小文字は区別しない) で終わっていなければ付け足す (Linux の保存のダイアログはフィルタの
/// 拡張子を付けずに返すことがある)。エディタの画面の `withExtension` (`apps/editor/src/lib/filePaths.ts`) と同じく
/// パスの末尾の文字列で見るので、名前が拡張子だけ (`.json`) でも付けない (画面が付け直しても同じパスになり、書けるパスと食い違わない)。
pub fn with_extension(path: PathBuf, extension: &str) -> PathBuf {
    let bytes = path.as_os_str().as_encoded_bytes();
    let suffix = extension.as_bytes();
    let has = bytes.len() > suffix.len()
        && bytes[bytes.len() - suffix.len() - 1] == b'.'
        && bytes[bytes.len() - suffix.len()..].eq_ignore_ascii_case(suffix);
    if has {
        return path;
    }
    let mut name = OsString::from(path);
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}

/// ロックを取る。ほかのスレッドがロック中に panic していても、中身 (パスの一覧) はそのまま使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_chosen_paths() {
        let access = FileAccess::default();
        let psd = PathBuf::from("/home/u/a.psd");
        assert!(!access.is_chosen(&psd));
        access.grant_readable([psd.clone()]);
        assert!(access.is_chosen(&psd));
        // 同じフォルダーのほかのファイルは読めない
        assert!(!access.is_chosen(Path::new("/home/u/b.psd")));
    }

    #[test]
    fn writes_only_once_to_the_path_chosen_in_the_save_dialog() {
        let access = FileAccess::default();
        let shot = PathBuf::from("/home/u/shot.png");
        assert!(!access.take_writable(&shot));
        access.grant_writable(shot.clone());
        assert!(!access.take_writable(Path::new("/home/u/other.png")));
        // 書けるパスは読めるパスにならない
        assert!(!access.is_chosen(&shot));
        assert!(access.take_writable(&shot));
        assert!(!access.take_writable(&shot));
        // 開くダイアログで選んだだけのパスには書けない
        access.grant_readable([shot.clone()]);
        assert!(!access.take_writable(&shot));
    }

    #[test]
    fn adds_the_extension_the_save_dialog_left_out() {
        let ext = |path: &str, extension| with_extension(PathBuf::from(path), extension);
        assert_eq!(ext("/a/shot", "png"), Path::new("/a/shot.png"));
        assert_eq!(ext("/a/shot.PNG", "png"), Path::new("/a/shot.PNG"));
        assert_eq!(ext("/a/stream.JSON", "json"), Path::new("/a/stream.JSON"));
        assert_eq!(
            ext("/a/b.ugoe", "ugoeproj"),
            Path::new("/a/b.ugoe.ugoeproj")
        );
        assert_eq!(
            ext("/a/shot.png.txt", "png"),
            Path::new("/a/shot.png.txt.png")
        );
        // 拡張子の前に `.` が要る
        assert_eq!(ext("/a/xpng", "png"), Path::new("/a/xpng.png"));
        // 名前が拡張子だけでも、画面の `withExtension` と同じく付けない
        assert_eq!(ext("/a/.json", "json"), Path::new("/a/.json"));
    }
}
