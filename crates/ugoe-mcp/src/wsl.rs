//! WSL と Windows のあいだ (ADR 0023 の 4)。
//!
//! - WSL の中の Linux 版の `ugoe-mcp --editor` が、Windows で動くエディタの発見用のファイルを探す
//!   ([`windows_discovery_candidates`])。
//! - エージェントが渡す WSL のパスと、Windows のエディタが扱うパスを相互に変える ([`WslPaths`])。
//!   中継 ([`crate::relay`]) が、ツールの引数 (WSL → Windows) と結果 (Windows → WSL) に使う。
//!
//! # WSL の中か
//!
//! `WSL_DISTRO_NAME` (WSL が設定する。ディストリビューションの名前) があれば WSL の中とみなす。
//! 無ければ `/proc/sys/fs/binfmt_misc/WSLInterop` か `/proc/version` の `microsoft` で判定し、名前は
//! `wslpath -w /` (`\\wsl.localhost\<名前>\`) から読む。
//!
//! # Windows の利用者のディレクトリ (`%LOCALAPPDATA%`)
//!
//! 1. 相互運用 (interop) で `cmd.exe /c echo %LOCALAPPDATA%` を実行し、答えを WSL のパスに変える。
//!    `cmd.exe` は `PATH` (appendWindowsPath) か `<mount>/c/Windows/System32/cmd.exe` から探す。作業
//!    ディレクトリを `<mount>/c` にする (Linux のディレクトリだと UNC の警告が出るので)。
//! 2. 相互運用が無効 (`/etc/wsl.conf` の `[interop] enabled=false`) か失敗したら、
//!    `<mount>/<ドライブ>/Users/*/AppData/Local/app.ugoe.editor/editor-mcp.json` を探す。
//!
//! `<mount>` は `/etc/wsl.conf` の `[automount] root` (既定 `/mnt/`)。
//!
//! # パスの変換の規則
//!
//! | WSL | Windows |
//! | --- | --- |
//! | `/mnt/c/Users/a` | `C:\Users\a` |
//! | `/mnt/c` | `C:\` |
//! | `/home/a` (ほかの絶対パス) | `\\wsl.localhost\<distro>\home\a` |
//!
//! 逆向きは、`C:\…`・`C:/…`・`\\?\C:\…`・`\\wsl.localhost\<distro>\…`・`\\wsl$\<distro>\…`・
//! `\\?\UNC\wsl.localhost\<distro>\…` を受ける。ほかのディストリビューションや、ほかの UNC のパスは
//! 変えない (WSL から指せる形が無いので)。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::discovery::{APP_DIR, FILE_NAME};

/// 自動マウントの根の既定。
const DEFAULT_MOUNT_ROOT: &str = "/mnt/";

/// WSL のパスと Windows のパスの変換。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslPaths {
    /// ディストリビューションの名前 (`\\wsl.localhost\<distro>` の部分)。
    distro: String,
    /// ドライブを自動マウントする根 (`/mnt/`。末尾は `/`)。
    mount_root: String,
}

impl WslPaths {
    /// `mount_root` は `/mnt/` の形 (末尾の `/` は無くてもよい)。
    pub fn new(distro: impl Into<String>, mount_root: &str) -> Self {
        let mut mount_root = mount_root.to_owned();
        if !mount_root.ends_with('/') {
            mount_root.push('/');
        }
        if !mount_root.starts_with('/') {
            mount_root.insert(0, '/');
        }
        Self {
            distro: distro.into(),
            mount_root,
        }
    }

    /// この WSL (の中で動くプロセス) の変換。WSL の中でなければ `None`。
    pub fn detect() -> Option<Self> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        let distro = match std::env::var("WSL_DISTRO_NAME") {
            Ok(name) if !name.is_empty() => name,
            _ => {
                if !is_wsl_kernel() {
                    return None;
                }
                distro_from_wslpath()?
            }
        };
        Some(Self::new(distro, &mount_root()))
    }

    pub fn distro(&self) -> &str {
        &self.distro
    }

    pub fn mount_root(&self) -> &str {
        &self.mount_root
    }

    /// WSL の絶対パス (`/` で始まる) を Windows のパスにする。絶対パスでなければ `None`。
    pub fn to_windows(&self, path: &str) -> Option<String> {
        if !path.starts_with('/') {
            return None;
        }
        if let Some(rest) = path.strip_prefix(&self.mount_root) {
            let (drive, tail) = rest.split_once('/').unwrap_or((rest, ""));
            let mut letters = drive.chars();
            if let (Some(letter), None) = (letters.next(), letters.next())
                && letter.is_ascii_alphabetic()
            {
                return Some(format!(
                    "{}:\\{}",
                    letter.to_ascii_uppercase(),
                    tail.replace('/', "\\")
                ));
            }
        }
        Some(format!(
            "\\\\wsl.localhost\\{}{}",
            self.distro,
            path.replace('/', "\\")
        ))
    }

    /// Windows の絶対パスを WSL のパスにする。WSL から指せない (ほかのディストリビューション・ほかの
    /// UNC) か、Windows の絶対パスでなければ `None`。
    pub fn to_wsl(&self, path: &str) -> Option<String> {
        let path = path.replace('/', "\\");
        // `\\?\` (正規化したパス) を外す。`\\?\UNC\server\share` は `\\server\share`。
        let path = match path.strip_prefix("\\\\?\\") {
            Some(rest) => match rest.strip_prefix("UNC\\") {
                Some(unc) => format!("\\\\{unc}"),
                None => rest.to_owned(),
            },
            None => path,
        };
        if let Some(unc) = path.strip_prefix("\\\\") {
            let mut parts = unc.splitn(3, '\\');
            let server = parts.next()?;
            let share = parts.next()?;
            let rest = parts.next().unwrap_or("");
            let is_wsl =
                server.eq_ignore_ascii_case("wsl.localhost") || server.eq_ignore_ascii_case("wsl$");
            if !is_wsl || !share.eq_ignore_ascii_case(&self.distro) {
                return None;
            }
            return Some(format!("/{}", rest.replace('\\', "/")));
        }
        let mut chars = path.chars();
        let letter = chars.next()?;
        if !letter.is_ascii_alphabetic() || chars.next() != Some(':') {
            return None;
        }
        let rest = chars.as_str();
        // `C:foo` (ドライブの中の相対パス) は受けない。
        if !rest.is_empty() && !rest.starts_with('\\') {
            return None;
        }
        let rest = rest.trim_start_matches('\\').replace('\\', "/");
        let mut wsl = format!("{}{}", self.mount_root, letter.to_ascii_lowercase());
        if !rest.is_empty() {
            wsl.push('/');
            wsl.push_str(&rest);
        }
        Some(wsl)
    }
}

/// カーネルが WSL のものか (`WSL_DISTRO_NAME` が無いとき。`sudo` の下など)。
fn is_wsl_kernel() -> bool {
    Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists()
        || std::fs::read_to_string("/proc/version")
            .is_ok_and(|v| v.to_ascii_lowercase().contains("microsoft"))
}

/// `wslpath -w /` (`\\wsl.localhost\Ubuntu\`) からディストリビューションの名前を読む。
fn distro_from_wslpath() -> Option<String> {
    let output = Command::new("wslpath")
        .args(["-w", "/"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let unc = text.trim().strip_prefix("\\\\")?;
    let mut parts = unc.split('\\');
    let _server = parts.next()?;
    parts.next().filter(|s| !s.is_empty()).map(str::to_owned)
}

/// `/etc/wsl.conf` の `[automount] root` (無ければ `/mnt/`)。
pub fn mount_root() -> String {
    std::fs::read_to_string("/etc/wsl.conf")
        .ok()
        .and_then(|conf| automount_root(&conf))
        .unwrap_or_else(|| DEFAULT_MOUNT_ROOT.to_owned())
}

/// `wsl.conf` (INI) の `[automount]` の `root`。
fn automount_root(conf: &str) -> Option<String> {
    let mut in_automount = false;
    for line in conf.lines() {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_automount = section.trim().eq_ignore_ascii_case("automount");
            continue;
        }
        if !in_automount {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim().eq_ignore_ascii_case("root")
        {
            let value = value.trim().trim_matches('"').trim();
            if value.starts_with('/') {
                return Some(value.to_owned());
            }
        }
    }
    None
}

/// Windows の `%LOCALAPPDATA%` を、相互運用の `cmd.exe` で尋ねて WSL のパスで返す。
/// 相互運用が無効・失敗なら `None`。
pub fn windows_local_app_data(paths: &WslPaths) -> Option<PathBuf> {
    let system_cmd = format!("{}c/Windows/System32/cmd.exe", paths.mount_root);
    let drive_c = format!("{}c", paths.mount_root);
    for cmd in ["cmd.exe", system_cmd.as_str()] {
        let mut command = Command::new(cmd);
        command
            .args(["/d", "/c", "echo %LOCALAPPDATA%"])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        if Path::new(&drive_c).is_dir() {
            command.current_dir(&drive_c);
        }
        let Ok(output) = command.output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let dir = text.trim();
        // 展開されなければ `%LOCALAPPDATA%` のまま返る。
        if dir.is_empty() || dir.contains('%') {
            continue;
        }
        if let Some(wsl) = paths.to_wsl(dir) {
            return Some(PathBuf::from(wsl));
        }
    }
    None
}

/// Windows のエディタの発見用のファイルの候補 (WSL のパス)。`%LOCALAPPDATA%` が分かればその下だけ、
/// 分からなければ各ドライブの `Users/*/AppData/Local` の下の、あるもの。
pub fn windows_discovery_candidates(paths: &WslPaths) -> Vec<PathBuf> {
    if let Some(dir) = windows_local_app_data(paths) {
        return vec![dir.join(APP_DIR).join(FILE_NAME)];
    }
    let mut found = Vec::new();
    let Ok(drives) = std::fs::read_dir(&paths.mount_root) else {
        return found;
    };
    for drive in drives.flatten() {
        let Ok(users) = std::fs::read_dir(drive.path().join("Users")) else {
            continue;
        };
        for user in users.flatten() {
            let candidate = user
                .path()
                .join("AppData")
                .join("Local")
                .join(APP_DIR)
                .join(FILE_NAME);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ubuntu() -> WslPaths {
        WslPaths::new("Ubuntu", "/mnt/")
    }

    #[test]
    fn wsl_paths_become_windows_paths() {
        let p = ubuntu();
        let cases = [
            (
                "/mnt/c/Users/a/model.ugoeproj",
                "C:\\Users\\a\\model.ugoeproj",
            ),
            ("/mnt/d/work", "D:\\work"),
            ("/mnt/c", "C:\\"),
            ("/mnt/c/", "C:\\"),
            (
                "/home/a/model.ugoeproj",
                "\\\\wsl.localhost\\Ubuntu\\home\\a\\model.ugoeproj",
            ),
            ("/", "\\\\wsl.localhost\\Ubuntu\\"),
            // ドライブの名前でないマウントは Linux のパスとして扱う。
            ("/mnt/wsl/x", "\\\\wsl.localhost\\Ubuntu\\mnt\\wsl\\x"),
            ("/mnt", "\\\\wsl.localhost\\Ubuntu\\mnt"),
        ];
        for (wsl, windows) in cases {
            assert_eq!(p.to_windows(wsl).as_deref(), Some(windows), "{wsl}");
        }
        assert_eq!(p.to_windows("relative/path"), None);
        assert_eq!(p.to_windows("C:\\already"), None);
    }

    #[test]
    fn windows_paths_become_wsl_paths() {
        let p = ubuntu();
        let cases = [
            (
                "C:\\Users\\a\\model.ugoeproj",
                "/mnt/c/Users/a/model.ugoeproj",
            ),
            ("C:/Users/a", "/mnt/c/Users/a"),
            ("c:\\", "/mnt/c"),
            ("D:", "/mnt/d"),
            ("\\\\?\\C:\\Users\\a", "/mnt/c/Users/a"),
            ("\\\\wsl.localhost\\Ubuntu\\home\\a", "/home/a"),
            ("\\\\wsl$\\ubuntu\\home\\a", "/home/a"),
            ("\\\\?\\UNC\\wsl.localhost\\Ubuntu\\home\\a", "/home/a"),
            ("\\\\wsl.localhost\\Ubuntu", "/"),
        ];
        for (windows, wsl) in cases {
            assert_eq!(p.to_wsl(windows).as_deref(), Some(wsl), "{windows}");
        }
        for other in [
            "\\\\wsl.localhost\\Debian\\home\\a",
            "\\\\server\\share\\x",
            "C:relative",
            "relative\\x",
            "/home/a",
            "",
        ] {
            assert_eq!(p.to_wsl(other), None, "{other}");
        }
    }

    #[test]
    fn conversions_round_trip() {
        for mount in ["/mnt/", "/"] {
            let p = WslPaths::new("Ubuntu-24.04", mount);
            for wsl in [
                format!("{mount}c/Users/a/b.ugoeproj"),
                "/home/a/b c/d.psd".to_owned(),
            ] {
                let windows = p.to_windows(&wsl).unwrap();
                assert_eq!(
                    p.to_wsl(&windows).as_deref(),
                    Some(wsl.as_str()),
                    "{windows}"
                );
            }
        }
    }

    #[test]
    fn custom_mount_root() {
        let p = WslPaths::new("Ubuntu", "/");
        assert_eq!(p.to_windows("/c/x").as_deref(), Some("C:\\x"));
        assert_eq!(p.to_wsl("C:\\x").as_deref(), Some("/c/x"));
        assert_eq!(
            p.to_windows("/home/a").as_deref(),
            Some("\\\\wsl.localhost\\Ubuntu\\home\\a")
        );
        assert_eq!(WslPaths::new("U", "/windir").mount_root(), "/windir/");
    }

    #[test]
    fn reads_the_automount_root() {
        let conf = "[boot]\nsystemd=true\n\n[automount]\nenabled = true\nroot = /win/ # comment\n";
        assert_eq!(automount_root(conf).as_deref(), Some("/win/"));
        assert_eq!(
            automount_root("[automount]\nroot=\"/x\"").as_deref(),
            Some("/x")
        );
        assert_eq!(automount_root("[boot]\nroot=/x\n"), None);
        assert_eq!(automount_root(""), None);
    }
}
