//! webview に読み書きを許したパス (T1616。ビューアの `FileAccess` (T0727)・スタジオの `access.rs` (T0838) と同じ考え)。
//!
//! webview で思わぬスクリプトが動いても、利用者が選んでいないファイルを読み書きできないようにする。ファイルのダイアログは
//! バックエンドが開き (`crate::dialog`)、選ばれたパスを覚える。コマンドの一覧と例外は `docs/editor.md#読み書きできるパス`。
//!
//! - 読める: 開くダイアログで選んだパス (ファイルとフォルダー)・ウィンドウにドロップしたパス・開いているプロジェクトの
//!   フォルダーの中・開いているプロジェクトが指すファイル (モジュールの本体・プレビューの着せ替えの一覧・提案の元の PSD)・
//!   選んだモジュールのプロジェクトが指す本体 (`module_open_problem` が返したもの)・`open_project` で閉じたモジュールの
//!   プロジェクトのフォルダー (本体を直した後に戻るため。T1622)。どれもこのセッションの間だけ覚える。
//! - 書ける: 保存のダイアログで選んだパス。1 回書いたら忘れる。今の保存先への保存 (`save_project`) はバックエンドが
//!   保存先を持つので、選び直さずに書ける。

use std::path::{Component, Path, PathBuf};

use ugoe_editor_core::Document;

use crate::AppState;
use crate::error::{EditorError, IpcResult};
use crate::project::{ProjectSession, absolute_path, lock_both};

/// webview に読み書きを許したパス。ダイアログで選んだパスの扱いはビューア・スタジオと同じ `ugoe_fs::FileAccess` で、
/// ここではエディタの規則 (開いているプロジェクトの中とプロジェクトが指すファイルも読める) とエラーのコードを足す。
#[derive(Debug, Default)]
pub struct FileAccess(ugoe_fs::FileAccess);

impl FileAccess {
    /// 開くダイアログで選んだ・ドロップしたパスを、読めるパスに加える。
    pub(crate) fn grant_readable(&self, paths: impl IntoIterator<Item = PathBuf>) {
        self.0.grant_readable(paths);
    }

    /// 保存のダイアログで選んだパスを、1 回だけ書けるパスにする。
    pub(crate) fn grant_writable(&self, path: PathBuf) {
        self.0.grant_writable(path);
    }

    /// 書けるパスなら、書けるパスから除いて `Ok` を返す (同じパスにもう 1 回書くには、またダイアログで選ぶ)。
    pub(crate) fn take_writable(&self, path: &Path) -> IpcResult<()> {
        if self.0.take_writable(path) {
            Ok(())
        } else {
            Err(EditorError::not_chosen_for_save(path).into())
        }
    }

    /// `path` を読んでよいか (ダイアログで選んだ・ドロップした・開いているプロジェクトの中・プロジェクトが指す)。
    pub(crate) fn check_readable(
        &self,
        path: &Path,
        document: &Document,
        session: &ProjectSession,
    ) -> IpcResult<()> {
        let allowed = self.0.is_chosen(path)
            || session.path().is_some_and(|dir| inside(path, dir))
            || project_references(document, session).any(|p| p == path);
        if allowed {
            Ok(())
        } else {
            Err(EditorError::not_chosen(path).into())
        }
    }
}

/// `path` を読んでよいか ([`FileAccess::check_readable`])。ドキュメントとプロジェクトのロックを取るので、
/// 握っていないときに呼ぶ。
pub(crate) fn check_readable(state: &AppState, path: &Path) -> IpcResult<()> {
    let (document, session) = lock_both(state)?;
    state.files.check_readable(path, &document, &session)
}

/// webview から受けた読むパス: 絶対パスにし (`project::absolute_path`)、読んでよいか確かめる ([`check_readable`])。
pub(crate) fn readable_path(state: &AppState, path: &str) -> IpcResult<PathBuf> {
    let path = absolute_path(path)?;
    check_readable(state, &path)?;
    Ok(path)
}

/// webview から受けた書くパス: 絶対パスにし、保存のダイアログで選んだものなら書けるパスから除いて返す
/// ([`FileAccess::take_writable`])。
pub(crate) fn writable_path(state: &AppState, path: &str) -> IpcResult<PathBuf> {
    let path = absolute_path(path)?;
    state.files.take_writable(&path)?;
    Ok(path)
}

/// ウィンドウにドロップされたファイル (プレビューの着せ替えの `.ugoemod`) を読めるパスに加える。Tauri はこのハンドラーを
/// ドロップのイベント (`tauri://drag-drop`) を webview へ送った直後に、同じメインスレッドで呼ぶ。webview がそのイベントを
/// 受けて `read_module_file` を呼ぶのはその後になる (IPC もメインスレッドで受ける) ので、読むときには加えてある (ビューアと同じ)。
pub(crate) fn on_webview_event(webview: &tauri::Webview, event: &tauri::WebviewEvent) {
    use tauri::{DragDropEvent, Manager, WebviewEvent};
    if let WebviewEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
        // 画面はパスを文字列で受けて返してくるので、許すパスも文字列から作る (`crate::dialog` と同じ)
        let paths = paths
            .iter()
            .map(|p| PathBuf::from(p.to_string_lossy().into_owned()));
        webview.state::<AppState>().files.grant_readable(paths);
    }
}

/// `path` が `dir` の中 (`dir` そのものも) か。`..` を含むパスは外に出られるので中とみなさない。
fn inside(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir) && !path.components().any(|c| c == Component::ParentDir)
}

/// 開いているプロジェクトが指すファイル (モジュールの本体・プレビューの着せ替えの一覧・提案の元の PSD)。
fn project_references<'a>(
    document: &'a Document,
    session: &'a ProjectSession,
) -> impl Iterator<Item = PathBuf> + 'a {
    let base = document.module().map(|m| m.base().path.clone());
    let modules = crate::dress_up::modules(session)
        .into_iter()
        .map(|m| PathBuf::from(m.path));
    let psd = session.proposals.psd().map(Path::to_path_buf);
    base.into_iter().chain(modules).chain(psd)
}

#[cfg(test)]
mod tests {
    use ugoe_editor_core::Project;

    use super::*;
    use crate::dress_up::{PreviewModule, set_modules};
    use crate::project::initial_state;

    #[test]
    fn reads_only_chosen_paths() {
        let access = FileAccess::default();
        let (session, document) = ProjectSession::from_project(Project::empty("a"), None);
        let psd = PathBuf::from("/home/u/a.psd");
        let err = access
            .check_readable(&psd, &document, &session)
            .unwrap_err();
        assert_eq!(err.code, "file.not_chosen");
        assert_eq!(err.args["path"], "/home/u/a.psd".into());
        access.grant_readable([psd.clone()]);
        access.check_readable(&psd, &document, &session).unwrap();
        // 同じフォルダーのほかのファイルは読めない
        assert!(
            access
                .check_readable(Path::new("/home/u/b.psd"), &document, &session)
                .is_err()
        );
    }

    #[test]
    fn reads_inside_the_open_project_and_what_it_refers_to() {
        let access = FileAccess::default();
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path();
        let (mut session, mut document) = initial_state();
        session
            .save_to(&mut document, work.join("a.ugoeproj"))
            .unwrap();
        for path in [
            "a.ugoeproj",
            "a.ugoeproj/project.json",
            "a.ugoeproj/x/b.png",
        ] {
            access
                .check_readable(&work.join(path), &document, &session)
                .unwrap();
        }
        for path in [
            "a.ugoeproj/../secret.json",
            "b.ugoeproj",
            "a.ugoeproj2/project.json",
        ] {
            let err = access
                .check_readable(&work.join(path), &document, &session)
                .unwrap_err();
            assert_eq!(err.code, "file.not_chosen", "{path}");
        }
        // プレビューの着せ替えの一覧と、提案の元の PSD
        let hat = Path::new("/mods/hat.ugoemod");
        let psd = Path::new("/art/a.psd");
        assert!(access.check_readable(hat, &document, &session).is_err());
        assert!(access.check_readable(psd, &document, &session).is_err());
        set_modules(
            &mut session,
            &[PreviewModule {
                path: hat.display().to_string(),
                attached: true,
            }],
        );
        session.proposals.set_psd(psd.to_path_buf());
        access.check_readable(hat, &document, &session).unwrap();
        access.check_readable(psd, &document, &session).unwrap();
    }

    /// コマンドが webview から受けたパスに使う確かめ: 選んでいないパスはファイルに触る前に断る。
    #[test]
    fn commands_refuse_paths_that_were_not_chosen() {
        let state = crate::ai::tests::state(None);
        let tmp = tempfile::tempdir().unwrap();
        let psd = tmp.path().join("a.psd");
        let psd_text = psd.display().to_string();
        let err = readable_path(&state, &psd_text).unwrap_err();
        assert_eq!(err.code, "file.not_chosen");
        // 相対パスは今まで通り先に断る
        assert_eq!(
            readable_path(&state, "a.psd").unwrap_err().code,
            "editor.relative_path"
        );
        state.files.grant_readable([psd.clone()]);
        assert_eq!(readable_path(&state, &psd_text).unwrap(), psd);

        let out = tmp.path().join("a.ugoe");
        let out_text = out.display().to_string();
        assert_eq!(
            writable_path(&state, &out_text).unwrap_err().code,
            "file.not_chosen_for_save"
        );
        // 開くダイアログで選んだだけのパスには書けない
        state.files.grant_readable([out.clone()]);
        assert!(writable_path(&state, &out_text).is_err());
        state.files.grant_writable(out.clone());
        assert_eq!(writable_path(&state, &out_text).unwrap(), out);
        assert!(writable_path(&state, &out_text).is_err());
    }

    #[test]
    fn writes_only_once_to_the_path_chosen_in_the_save_dialog() {
        let access = FileAccess::default();
        let shot = PathBuf::from("/home/u/shot.png");
        let err = access.take_writable(&shot).unwrap_err();
        assert_eq!(err.code, "file.not_chosen_for_save");
        access.grant_writable(shot.clone());
        // 開くダイアログで選んだパスとは別に数える
        assert!(
            access
                .take_writable(Path::new("/home/u/other.png"))
                .is_err()
        );
        access.take_writable(&shot).unwrap();
        assert!(access.take_writable(&shot).is_err());
    }
}
