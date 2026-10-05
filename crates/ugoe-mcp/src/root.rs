//! ツールが触れるパスを 1 つのディレクトリ (`--root`) の下に限る。
//!
//! ツールが受け取るパス (プロジェクト・PSD・書き出す `.ugoe`・描くモーションや表情の JSON) は、
//! [`Root::resolve`] で正規化してから root の下にあるかを確かめる。相対パスは root から解決する。
//!
//! - 存在する部分は `canonicalize` で正規化する (シンボリックリンクと `..` を解いた実際の場所で判定する)。
//! - まだ無い末尾 (保存先など) は、存在する最も深い祖先を正規化して結合する。無い部分に `..` や
//!   `.` があれば拒否する (リンクを解く前の `..` の意味は決められないので)。無い部分の途中に、
//!   行き先の無いシンボリックリンクがあっても拒否する (書くとリンクの先に作られるので)。
//! - 絶対パスは root の下を指すときだけ受ける。Windows のドライブ (`C:\`・`C:foo`)・UNC (`\\server\share`)・
//!   `\\?\` の形も、正規化した結果を root と部品ごとに比べるので、root の外なら拒否する。
//!
//! 判定と実際の読み書きの間にパスが変わる競合 (TOCTOU) は防がない: 判定の後、読み書きの前に、ほかの
//! プロセスが root の中のディレクトリをシンボリックリンクに差し替えると、その先を読み書きしうる。
//! ツールはシンボリックリンクを作れないので、MCP のクライアントだけではこの競合を起こせない。
//! root の中を、信頼しないほかのプロセスに書かせないこと (README の「パスを限る」)。
//! 判定はツールを呼ぶたびにし直すので、呼び出しをまたいだ差し替えは次の呼び出しで拒否する。

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use ugoe_editor_core::message::Message;
use ugoe_editor_core::message_args;

use crate::host::mcp_message;

/// 使えないパス (`mcp.invalid_path`)。`path` はツールが受け取ったパスか、確かめたパス。
fn invalid(path: &str, message: String) -> Message {
    mcp_message(
        "mcp.invalid_path",
        message_args! { "path" => path },
        message,
    )
}

/// ツールが触れるパスの根。正規化した既存のディレクトリ。
#[derive(Debug, Clone)]
pub struct Root {
    dir: PathBuf,
}

/// root を用意できない。
#[derive(Debug, thiserror::Error)]
#[error("invalid --root `{}`: {reason}", .path.display())]
pub struct RootError {
    path: PathBuf,
    reason: String,
}

impl Root {
    /// `dir` (既存のディレクトリ) を root にする。
    pub fn new(dir: &Path) -> Result<Self, RootError> {
        let error = |reason: String| RootError {
            path: dir.to_path_buf(),
            reason,
        };
        let canonical = dir.canonicalize().map_err(|e| error(e.to_string()))?;
        if !canonical.is_dir() {
            return Err(error("not a directory".to_owned()));
        }
        Ok(Self { dir: canonical })
    }

    /// 正規化した root のディレクトリ。
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// ツールが受け取ったパスを、root の下の正規化した絶対パスにする。外を指せばエラー。
    /// パスはまだ無くてもよい (保存先)。
    /// エラーは `mcp.invalid_path` か `mcp.outside_root` (README の「ツールのエラー」)。
    pub fn resolve(&self, path: &str) -> Result<PathBuf, Message> {
        if path.is_empty() {
            return Err(invalid(path, "the path is empty".to_owned()));
        }
        let joined = self.dir.join(path);
        let resolved = canonicalize_lenient(&joined)
            .map_err(|e| invalid(path, format!("`{path}`: {}", e.message)))?;
        if !resolved.starts_with(&self.dir) {
            return Err(self.outside(
                path,
                format!(
                    "`{path}` is outside the root directory `{}` (the server runs with --root; use a \
                     path under it, relative paths are resolved from it)",
                    self.dir.display()
                ),
            ));
        }
        Ok(resolved)
    }

    /// root の外を指す (`mcp.outside_root`)。
    fn outside(&self, path: &str, message: String) -> Message {
        mcp_message(
            "mcp.outside_root",
            message_args! { "path" => path, "root" => self.dir.display().to_string() },
            message,
        )
    }

    /// モジュールのプロジェクトが参照する本体 (`project.json` の `module.base` を絶対パスにしたもの) を、読む前に
    /// 確かめる (T0524。[`ugoe_editor_core::project::BaseCheck`])。正規化して root の下にあり、`.ugoeproj` なら
    /// その中のシンボリックリンクも外を指さないときに、正規化したパス (読むパス) を返す。本体がまだ無くても、
    /// root の下なら受ける (見つからないことは読む側が伝える)。
    pub(crate) fn check_base(&self, path: &Path) -> Result<PathBuf, Message> {
        // エラーは本体のパスを前に付けて伝わる (`ProjectError::BaseRefused`)。
        let resolved = canonicalize_lenient(path)?;
        if !resolved.starts_with(&self.dir) {
            return Err(self.outside(
                &path.display().to_string(),
                format!(
                    "outside the root directory `{}` (the server runs with --root; the module project's \
                     `module.base` in project.json must point under it)",
                    self.dir.display()
                ),
            ));
        }
        if resolved.is_dir() {
            self.check_project_dir(&resolved)?;
        }
        Ok(resolved)
    }

    /// プロジェクトのディレクトリの中の (読み書きする) シンボリックリンクが root の外を指していないことを
    /// 確かめる。直下と `motions/`・`expressions/`・`textures/` の中を見る (`Project::load`・`Project::save` が
    /// 触れる場所)。
    pub(crate) fn check_project_dir(&self, dir: &Path) -> Result<(), Message> {
        let io = |path: &Path, e: io::Error| {
            let path = path.display().to_string();
            invalid(&path, format!("`{path}`: {e}"))
        };
        for sub in ["", "motions", "expressions", "textures"] {
            let Ok(entries) = fs::read_dir(dir.join(sub)) else {
                continue;
            };
            for entry in entries {
                let entry = entry.map_err(|e| io(dir, e))?;
                let is_link = entry
                    .file_type()
                    .map_err(|e| io(&entry.path(), e))?
                    .is_symlink();
                if is_link {
                    self.contains_existing(&entry.path())?;
                }
            }
        }
        Ok(())
    }

    /// 既存のパスを正規化し、root の下にあることを確かめる (プロジェクトの中のシンボリックリンクなど)。
    pub(crate) fn contains_existing(&self, path: &Path) -> Result<(), Message> {
        let shown = path.display().to_string();
        let resolved = path
            .canonicalize()
            .map_err(|e| invalid(&shown, format!("`{shown}`: {e}")))?;
        if resolved.starts_with(&self.dir) {
            Ok(())
        } else {
            Err(self.outside(
                &shown,
                format!(
                    "`{shown}` leads outside the root directory `{}` (a symbolic link)",
                    self.dir.display()
                ),
            ))
        }
    }
}

/// 存在する最も深い祖先を正規化し、無い部分を結合する。エラーは `mcp.invalid_path` (`path` は `path`)。
pub(crate) fn canonicalize_lenient(path: &Path) -> Result<PathBuf, Message> {
    let invalid = |message: String| invalid(&path.display().to_string(), message);
    let mut existing = path.to_path_buf();
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(canonical) => {
                let mut resolved = canonical;
                for name in missing.iter().rev() {
                    resolved.push(name);
                }
                return Ok(resolved);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // 行き先の無いシンボリックリンクは、無いものとして扱わない (書くとリンクの先に作られる)。
                if fs::symlink_metadata(&existing).is_ok() {
                    return Err(invalid(format!(
                        "`{}` is a symbolic link whose target does not exist",
                        existing.display()
                    )));
                }
                match existing.components().next_back() {
                    Some(Component::Normal(name)) => missing.push(name.to_owned()),
                    Some(Component::ParentDir | Component::CurDir) => {
                        return Err(invalid(
                            "`..` and `.` are only allowed in the part of the path that exists"
                                .to_owned(),
                        ));
                    }
                    // 根 (`/`・`C:\`) が無い。
                    _ => return Err(invalid(e.to_string())),
                }
                if !existing.pop() {
                    return Err(invalid(e.to_string()));
                }
            }
            Err(e) => return Err(invalid(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の一時ディレクトリ。`path` は正規化したパスで、破棄時に消す。
    struct TempDir {
        path: PathBuf,
        _dir: tempfile::TempDir,
    }

    impl TempDir {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().canonicalize().unwrap();
            Self { path, _dir: dir }
        }
    }

    /// `<tmp>/root/{a.ugoeproj/, sub/}` と、root の外の `<tmp>/outside/`。
    fn layout() -> (TempDir, Root) {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.path.join("root/a.ugoeproj")).unwrap();
        fs::create_dir_all(tmp.path.join("root/sub")).unwrap();
        fs::create_dir_all(tmp.path.join("outside")).unwrap();
        fs::write(tmp.path.join("outside/secret.psd"), b"x").unwrap();
        let root = Root::new(&tmp.path.join("root")).unwrap();
        (tmp, root)
    }

    #[test]
    fn resolves_paths_under_the_root() {
        let (tmp, root) = layout();
        let dir = tmp.path.join("root");
        assert_eq!(root.resolve("a.ugoeproj").unwrap(), dir.join("a.ugoeproj"));
        assert_eq!(
            root.resolve("./sub/../a.ugoeproj").unwrap(),
            dir.join("a.ugoeproj")
        );
        // まだ無いパス (保存先) も、root の下なら受ける。
        assert_eq!(
            root.resolve("sub/new.ugoeproj").unwrap(),
            dir.join("sub/new.ugoeproj")
        );
        assert_eq!(
            root.resolve("sub/x/y/out.ugoe").unwrap(),
            dir.join("sub/x/y/out.ugoe")
        );
        // root の下を指す絶対パスも受ける。
        let absolute = dir.join("a.ugoeproj");
        assert_eq!(root.resolve(absolute.to_str().unwrap()).unwrap(), absolute);
        assert_eq!(root.resolve(".").unwrap(), dir);
    }

    #[test]
    fn rejects_paths_outside_the_root() {
        let (tmp, root) = layout();
        for path in [
            "..",
            "../outside/secret.psd",
            "sub/../../outside/secret.psd",
            "../outside/new.ugoeproj",
            "../../../../../../../../etc/passwd",
        ] {
            let err = root.resolve(path).unwrap_err();
            assert!(err.message.contains("outside the root"), "{path}: {err}");
        }
        let absolute = tmp.path.join("outside/secret.psd");
        let err = root.resolve(absolute.to_str().unwrap()).unwrap_err();
        assert!(err.message.contains("outside the root"), "{err}");
        // 名前が root で始まるだけの隣のディレクトリ (`root-other`) は外。
        fs::create_dir_all(tmp.path.join("root-other")).unwrap();
        let sibling = tmp.path.join("root-other");
        assert!(root.resolve(sibling.to_str().unwrap()).is_err());
        assert!(root.resolve("../root-other").is_err());
        assert!(root.resolve("").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_dot_dot_in_the_missing_part() {
        let (_tmp, root) = layout();
        // `nope` は無いので、その後の `..` の意味を決められない。
        let err = root.resolve("nope/../a.ugoeproj").unwrap_err();
        assert!(err.message.contains("`..`"), "{err}");
        let err = root.resolve("nope/../../outside").unwrap_err();
        assert!(err.message.contains("`..`"), "{err}");
    }

    /// Windows はパスを開く前に `..` を字面で解く (無い部分の `..` も) ので、正規化の結果も字面で解いた
    /// 場所になる。その場所で root の中かを判定する。
    #[cfg(windows)]
    #[test]
    fn resolves_dot_dot_in_the_missing_part_lexically() {
        let (_tmp, root) = layout();
        let inside = root.resolve("nope/../a.ugoeproj").unwrap();
        assert_eq!(inside, root.dir().join("a.ugoeproj"));
        let err = root.resolve("nope/../../outside").unwrap_err();
        assert!(err.message.contains("outside the root"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn follows_symbolic_links_before_judging() {
        use std::os::unix::fs::symlink;

        let (tmp, root) = layout();
        let dir = tmp.path.join("root");
        // root の外を指すリンク (ディレクトリ・ファイル)。
        symlink(tmp.path.join("outside"), dir.join("escape")).unwrap();
        symlink(tmp.path.join("outside/secret.psd"), dir.join("secret.psd")).unwrap();
        // root の中を指すリンクは受ける。
        symlink(dir.join("a.ugoeproj"), dir.join("alias.ugoeproj")).unwrap();
        // 行き先の無いリンク (書くと外に作られる)。
        symlink(
            tmp.path.join("outside/planted.ugoe"),
            dir.join("dangling.ugoe"),
        )
        .unwrap();
        symlink(tmp.path.join("outside/nodir"), dir.join("dangling-dir")).unwrap();

        for path in [
            "escape",
            "escape/secret.psd",
            "escape/new.ugoeproj",
            "secret.psd",
        ] {
            let err = root.resolve(path).unwrap_err();
            assert!(err.message.contains("outside the root"), "{path}: {err}");
        }
        for path in ["dangling.ugoe", "dangling-dir/x.ugoe"] {
            let err = root.resolve(path).unwrap_err();
            assert!(err.message.contains("symbolic link"), "{path}: {err}");
        }
        assert_eq!(
            root.resolve("alias.ugoeproj").unwrap(),
            dir.join("a.ugoeproj")
        );
        assert!(root.contains_existing(&dir.join("alias.ugoeproj")).is_ok());
        assert!(root.contains_existing(&dir.join("escape")).is_err());

        // root 自体がリンクでも、正規化した先で判定する。
        symlink(&dir, tmp.path.join("root-link")).unwrap();
        let linked = Root::new(&tmp.path.join("root-link")).unwrap();
        assert_eq!(linked.dir(), dir);
        assert!(linked.resolve("a.ugoeproj").is_ok());
        assert!(linked.resolve("../outside").is_err());
    }

    #[test]
    fn root_must_be_an_existing_directory() {
        let (tmp, _root) = layout();
        assert!(Root::new(&tmp.path.join("missing")).is_err());
        let err = Root::new(&tmp.path.join("outside/secret.psd")).unwrap_err();
        assert!(err.to_string().contains("not a directory"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn rejects_other_drives_and_unc_paths() {
        let (_tmp, root) = layout();
        for path in [
            r"C:\Windows\win.ini",
            r"C:Windows",
            r"\Windows",
            r"\\localhost\C$\Windows",
            r"\\?\C:\Windows",
        ] {
            assert!(root.resolve(path).is_err(), "{path}");
        }
    }
}
