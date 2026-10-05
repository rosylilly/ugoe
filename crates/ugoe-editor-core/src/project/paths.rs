//! 本体の参照のパスの補助 (解決・正規化・相対パス)。

use std::path::{Path, PathBuf};

/// `project.json` に書いた本体の参照を、プロジェクトのディレクトリ `dir` から解決する。
pub(super) fn resolve_base(dir: &Path, base: &str) -> PathBuf {
    let base = Path::new(base);
    if base.is_absolute() {
        normalize_path(base)
    } else {
        normalize_path(&absolute(dir).join(base))
    }
}

/// 絶対パスにする (今の作業ディレクトリから)。取れなければそのまま。
pub(super) fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `.` と `..` を字面の上で取り除く (ファイルシステムは見ない)。
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in absolute(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// `from` (ディレクトリ) から `to` への相対パス (`/` 区切り)。根が違えば `to` の絶対パス。
pub(super) fn relative_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_components)
        .take_while(|(a, b)| a == b)
        .count();
    if common == 0 {
        return to.display().to_string();
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); from.len() - common];
    parts.extend(
        to_components[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}
