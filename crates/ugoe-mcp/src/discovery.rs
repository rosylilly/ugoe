//! 発見用のファイル: 実行中のエディタが MCP を待ち受けるポートとトークン (ADR 0023)。
//!
//! エディタ ([`crate::bridge::EditorBridge`]) が起動中に書き、終了時に消す。`ugoe-mcp --editor`
//! ([`crate::relay`]) が読んでエディタにつなぐ。トークンを含むので、利用者だけが読める場所と権限にする。
//!
//! 置き場所 (`<dir>/app.ugoe.editor/editor-mcp.json`。環境変数 `UGOE_MCP_DISCOVERY` でファイルのパスを
//! 変えられる)。ディレクトリの名前はエディタの識別子 (`tauri.conf.json` の `identifier`) にする
//! (Windows の `%LOCALAPPDATA%\ugoe` はエディタのインストール先なので避ける):
//!
//! | OS | `<dir>` |
//! | --- | --- |
//! | Windows | `%LOCALAPPDATA%` (利用者ごとのアクセス権が付いている) |
//! | macOS | `~/Library/Application Support` |
//! | Linux ほか | `$XDG_STATE_HOME` (絶対パスのとき)、無ければ `~/.local/state` |
//!
//! Unix では、ディレクトリを 0700 で作り、ファイルを 0600 で書く (一時ファイルに書いてから名前を変える
//! ので、読む側が書きかけを読まない)。WSL の中の `--editor` が Windows のエディタのファイルを探す方法は
//! [`crate::wsl`]。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 発見用のファイルの形の版。読む側が知らない版なら、つながずに更新を促す。
pub const DISCOVERY_FORMAT: u32 = 1;
/// 発見用のファイルのパスを変える環境変数 (エディタと `--editor` の両方が見る)。
pub const DISCOVERY_ENV: &str = "UGOE_MCP_DISCOVERY";
/// ファイル名。
pub const FILE_NAME: &str = "editor-mcp.json";
/// アプリのデータのディレクトリの下のディレクトリ名 (エディタの識別子)。
pub const APP_DIR: &str = "app.ugoe.editor";

/// 発見用のファイルの中身。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    /// 形の版 ([`DISCOVERY_FORMAT`])。
    pub format: u32,
    /// 書いたエディタの ugoe の版 (`ugoe-mcp` のクレートの版)。
    pub version: String,
    /// エディタのプロセス ID (利用者への案内に使う。生きているかは接続して確かめる)。
    pub pid: u32,
    /// 待ち受けるループバックのポート (`127.0.0.1`)。
    pub port: u16,
    /// 要求に求めるトークン (`Authorization: Bearer`)。
    pub token: String,
    /// MCP のエンドポイントのパス (`/mcp`)。
    pub endpoint: String,
}

impl Discovery {
    /// このプロセスが `port` で待ち受けることを示す中身。
    pub fn new(port: u16, token: String) -> Self {
        Self {
            format: DISCOVERY_FORMAT,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            pid: std::process::id(),
            port,
            token,
            endpoint: crate::http::ENDPOINT.to_owned(),
        }
    }

    /// エディタの MCP の URL (ループバック)。
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}{}", self.port, self.endpoint)
    }
}

/// 発見用のファイルを読めない。
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("failed to read {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("{} is not a valid discovery file: {reason}", .path.display())]
    Invalid { path: PathBuf, reason: String },
    #[error(
        "{} was written by ugoe {version} in discovery format {found}, but this ugoe-mcp {ours} \
         reads format {DISCOVERY_FORMAT}; use the ugoe-mcp that ships with the editor (or update \
         both to the same version)",
        .path.display(),
        ours = env!("CARGO_PKG_VERSION")
    )]
    Format {
        path: PathBuf,
        found: u64,
        version: String,
    },
}

/// このプラットフォームでの発見用のファイルの既定のパス (`UGOE_MCP_DISCOVERY` があればそれ)。
/// 場所を決められない (ホームのディレクトリが分からないなど) なら `None`。
pub fn default_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(DISCOVERY_ENV).filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    Some(data_dir()?.join(APP_DIR).join(FILE_NAME))
}

/// OS のアプリのデータのディレクトリ (モジュールの説明の表)。
fn data_dir() -> Option<PathBuf> {
    let env_dir = |name: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    if cfg!(windows) {
        env_dir("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        Some(env_dir("HOME")?.join("Library").join("Application Support"))
    } else {
        env_dir("XDG_STATE_HOME").or_else(|| Some(env_dir("HOME")?.join(".local").join("state")))
    }
}

/// 発見用のファイルを書く (利用者だけが読める権限で、置き換えは不可分に)。
pub fn write(path: &Path, discovery: &Discovery) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(discovery).map_err(io::Error::other)?;
    ugoe_fs::write_atomic_private(path, &bytes)
}

/// 発見用のファイルを読む。形の版が違えば [`DiscoveryError::Format`]。
pub fn read(path: &Path) -> Result<Discovery, DiscoveryError> {
    let bytes = fs::read(path).map_err(|source| DiscoveryError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let invalid = |reason: String| DiscoveryError::Invalid {
        path: path.to_path_buf(),
        reason,
    };
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
    let format = value
        .get("format")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("missing \"format\"".to_owned()))?;
    if format != u64::from(DISCOVERY_FORMAT) {
        return Err(DiscoveryError::Format {
            path: path.to_path_buf(),
            found: format,
            version: value
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_owned(),
        });
    }
    let discovery: Discovery = serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
    if discovery.port == 0 {
        return Err(invalid("\"port\" must not be 0".to_owned()));
    }
    // `url` はループバックの URL にパスを足すだけなので、`@` (利用者情報になり、ほかのホストへつなぐ)・
    // `//`・`?`・`#` などを含まない、`/` で始まるパスに限る。
    let endpoint_is_path = discovery.endpoint.starts_with('/')
        && !discovery.endpoint.starts_with("//")
        && discovery
            .endpoint
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~'));
    if !endpoint_is_path {
        return Err(invalid(format!(
            "\"endpoint\" {:?} is not a URL path",
            discovery.endpoint
        )));
    }
    Ok(discovery)
}

/// 発見用のファイルが `ours` のまま (ほかのエディタが上書きしていない) なら消す。消したら `true`。
pub fn remove_if_ours(path: &Path, ours: &Discovery) -> io::Result<bool> {
    match read(path) {
        Ok(found) if found == *ours => {
            fs::remove_file(path)?;
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(DiscoveryError::Read { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            Ok(false)
        }
        // 壊れたファイルや知らない版は、ほかのエディタのものかもしれないので残す。
        Err(_) => Ok(false),
    }
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    // Windows の %LOCALAPPDATA% の下は、利用者ごとのアクセス権を受け継ぐ。
    fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_reads_and_removes() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let path = dir.join("nested").join(FILE_NAME);
        let ours = Discovery::new(40000, "t".repeat(64));
        write(&path, &ours).unwrap();
        assert_eq!(read(&path).unwrap(), ours);
        assert_eq!(ours.url(), "http://127.0.0.1:40000/mcp");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&path), 0o600);
            assert_eq!(mode(path.parent().unwrap()), 0o700);
        }
        // 置き換えても一時ファイルを残さない。
        let newer = Discovery::new(40001, "u".repeat(64));
        write(&path, &newer).unwrap();
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        // ほかのエディタが書いたものは消さない。
        assert!(!remove_if_ours(&path, &ours).unwrap());
        assert!(path.exists());
        assert!(remove_if_ours(&path, &newer).unwrap());
        assert!(!path.exists());
        assert!(!remove_if_ours(&path, &newer).unwrap());
    }

    #[test]
    fn rejects_other_formats_and_broken_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let path = dir.join(FILE_NAME);
        fs::write(&path, r#"{"format": 2, "version": "9.0.0", "port": 1}"#).unwrap();
        let err = read(&path).unwrap_err();
        assert!(
            matches!(err, DiscoveryError::Format { found: 2, .. }),
            "{err}"
        );
        assert!(err.to_string().contains("9.0.0"), "{err}");
        fs::write(&path, "{").unwrap();
        assert!(matches!(read(&path), Err(DiscoveryError::Invalid { .. })));
        fs::write(&path, r#"{"format": 1}"#).unwrap();
        assert!(matches!(read(&path), Err(DiscoveryError::Invalid { .. })));
        assert!(matches!(
            read(&dir.join("missing.json")),
            Err(DiscoveryError::Read { .. })
        ));
    }

    /// エンドポイントはループバックの URL のパスにしかならない (`@` で URL の利用者情報にして、トークンを
    /// ほかのホストへ送らせない)。
    #[test]
    fn rejects_endpoints_that_leave_the_loopback() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let path = dir.join(FILE_NAME);
        for endpoint in [
            "@evil.example/mcp",
            "mcp",
            "/mcp#x",
            "/mcp?x",
            "//evil.example/mcp",
            "/m cp",
            "/mcp\\x",
        ] {
            let mut discovery = Discovery::new(40000, "t".repeat(64));
            discovery.endpoint = endpoint.to_owned();
            write(&path, &discovery).unwrap();
            let err = read(&path).unwrap_err();
            assert!(
                matches!(err, DiscoveryError::Invalid { .. }),
                "{endpoint}: {err}"
            );
        }
        let mut zero = Discovery::new(0, "t".repeat(64));
        write(&path, &zero).unwrap();
        assert!(matches!(read(&path), Err(DiscoveryError::Invalid { .. })));
        zero.port = 1;
        zero.endpoint = "/mcp/sub-path_1.x~".to_owned();
        write(&path, &zero).unwrap();
        assert_eq!(read(&path).unwrap(), zero);
    }
}
