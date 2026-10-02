//! 操作 API の設定・トークン・承認したクライアントの鍵と、ブラウザソースのスイッチとトークン (ADR 0028 の 6.、`docs/studio.md#外部-api`)。
//!
//! **プロファイルに入れない** (プロファイルを人に渡しても漏れない)。アプリのデータの場所の `studio-api.json` に置き、
//! 書くときは一時ファイルに書いてから置き換える。Unix では持ち主だけが読めるようにする (0600)。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// `format` の値。
pub const API_FORMAT: &str = "ugoe-studio-api";
/// 書き出す版。読めるのはこれ以下の版。
pub const API_VERSION: u32 = 1;
/// 既定のポート (他製品の既定のポートと同じ番号にしない。Linux と Windows の一時的なポートの範囲の外)。
pub const DEFAULT_PORT: u16 = 28451;
/// 承認したクライアントの数の上限 (ファイルを手で書き換えたとき)。
pub const MAX_CLIENTS: usize = 64;
/// クライアントの名前の長さの上限 (文字)。
pub const MAX_NAME_CHARS: usize = 64;
/// トークンと鍵のバイト数 (OS の乱数で 256 ビット。16 進数で 64 文字)。
const SECRET_BYTES: usize = 32;

/// クライアントに与える権限。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    /// 一覧と様子を読み、通知を受けるだけ。
    #[default]
    Read,
    /// 読むことに加えて、切り替え・再生・値の設定・着せ替えをする。
    Control,
}

/// 承認したクライアント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRecord {
    /// 一覧の中で一意な ID (秘密ではない。画面と取り消しに使う)。
    pub id: String,
    /// クライアントが名乗った名前。
    pub name: String,
    /// クライアント用の鍵 (秘密。次からはこれで接続する)。
    pub key: String,
    pub permission: Permission,
    /// 承認した時刻 (Unix 時間の秒)。
    pub approved_at: u64,
    /// 最後に接続した時刻 (Unix 時間の秒)。
    #[serde(default)]
    pub last_seen: Option<u64>,
}

/// `studio-api.json` の中身。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSettings {
    pub format: String,
    pub version: u32,
    /// 待ち受けるか (既定は切)。
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    /// 初めて接続するクライアントが示すトークン。空なら次に読んだときに作る。
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub clients: Vec<ClientRecord>,
    /// ブラウザソース (T0808)。待ち受けとポートは操作 API と共有し、スイッチとトークンは別に持つ。
    #[serde(default)]
    pub source: SourceSettings,
}

/// ブラウザソースの設定 (`studio-api.json` の `source`)。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSettings {
    /// ページを出すか (既定は切)。
    #[serde(default)]
    pub enabled: bool,
    /// URL のパスに入れるトークン (読むだけ。空なら次に読んだときに作る)。
    #[serde(default)]
    pub token: String,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            format: API_FORMAT.to_owned(),
            version: API_VERSION,
            enabled: false,
            port: DEFAULT_PORT,
            token: String::new(),
            clients: Vec::new(),
            source: SourceSettings::default(),
        }
    }
}

/// OS の乱数で作った秘密 (16 進数で 64 文字)。
pub fn generate_secret() -> Result<String, String> {
    let mut bytes = [0u8; SECRET_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(hex(&bytes))
}

/// OS の乱数で作った、秘密でない ID (16 進数で 16 文字)。
pub fn generate_id() -> Result<String, String> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(hex(&bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 長さ以外で時間の差が出ない比較 (トークンや鍵を 1 文字ずつ当てられないように)。
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// 秘密の形 (16 進数で 64 文字) か。手で書き換えた短い値は使わずに作り直す。
fn is_secret(value: &str) -> bool {
    value.len() == SECRET_BYTES * 2 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 名前を収める (前後の空白を除き、制御文字を除き、長さを切る)。
pub fn clean_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect()
}

impl ApiSettings {
    /// `path` から読む。無い・読めない・知らない形なら既定 (切) で始める (読めなかったことは返す)。
    /// トークンが無いか形が違えば作る。
    pub fn read(path: &Path) -> (Self, Option<String>) {
        let (mut settings, problem) = match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(s) if s.format == API_FORMAT && (1..=API_VERSION).contains(&s.version) => {
                    (s, None)
                }
                Ok(s) => (
                    Self::default(),
                    Some(format!(
                        "{}: unsupported format {:?} version {}",
                        path.display(),
                        s.format,
                        s.version
                    )),
                ),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
        };
        settings.sanitize();
        (settings, problem)
    }

    /// 値を収める。トークンが無ければ作る (作れなければ空のまま。空のトークンは誰も通さない)。
    pub fn sanitize(&mut self) {
        self.format = API_FORMAT.to_owned();
        self.version = API_VERSION;
        if self.port == 0 {
            self.port = DEFAULT_PORT;
        }
        if !is_secret(&self.token) {
            self.token = generate_secret().unwrap_or_default();
        }
        if !is_secret(&self.source.token) {
            self.source.token = generate_secret().unwrap_or_default();
        }
        let mut seen = std::collections::HashSet::new();
        self.clients
            .retain(|c| is_secret(&c.key) && !c.id.is_empty() && seen.insert(c.id.clone()));
        self.clients.truncate(MAX_CLIENTS);
        for client in &mut self.clients {
            client.name = clean_name(&client.name);
        }
    }

    /// 書き出す (一時ファイルに書いてから置き換える。Unix では 0600)。
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut temp = path.as_os_str().to_owned();
        temp.push(".tmp");
        let temp = PathBuf::from(temp);
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        write_private(&temp, json.as_bytes())?;
        std::fs::rename(&temp, path)
    }

    /// 鍵 `key` を持つクライアント (定数時間で比べる)。
    pub fn client_by_key(&self, key: &str) -> Option<&ClientRecord> {
        // 見つかっても途中で止めない (どれが当たったかで時間が変わらないように)。
        let mut found = None;
        for client in &self.clients {
            if constant_time_eq(&client.key, key) && found.is_none() {
                found = Some(client);
            }
        }
        found
    }
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // Windows ではアプリのデータの場所 (利用者のプロファイルの下) が既に持ち主だけのもの。
    std::fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ugoe-studio-api-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn starts_disabled_with_a_fresh_token() {
        let dir = temp_dir("fresh");
        let (settings, problem) = ApiSettings::read(&dir.join("studio-api.json"));
        assert!(problem.is_none());
        assert!(!settings.enabled);
        assert_eq!(settings.port, DEFAULT_PORT);
        assert!(is_secret(&settings.token));
        assert!(!settings.source.enabled);
        assert!(is_secret(&settings.source.token));
        assert_ne!(settings.token, settings.source.token, "separate tokens");
        let (other, _) = ApiSettings::read(&dir.join("studio-api.json"));
        assert_ne!(settings.token, other.token, "a fresh token per read");
    }

    #[test]
    fn writes_and_reads_back() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("nested").join("studio-api.json");
        let (mut settings, _) = ApiSettings::read(&path);
        settings.enabled = true;
        settings.port = 30000;
        settings.source.enabled = true;
        settings.clients.push(ClientRecord {
            id: "c1".into(),
            name: " Deck\u{7} ".into(),
            key: generate_secret().unwrap(),
            permission: Permission::Control,
            approved_at: 1,
            last_seen: None,
        });
        settings.sanitize();
        settings.write(&path).unwrap();
        let (read, problem) = ApiSettings::read(&path);
        assert!(problem.is_none());
        assert_eq!(read, settings);
        assert_eq!(read.clients[0].name, "Deck");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn broken_files_and_weak_secrets_are_replaced() {
        let dir = temp_dir("broken");
        let path = dir.join("studio-api.json");
        std::fs::write(&path, "{").unwrap();
        let (settings, problem) = ApiSettings::read(&path);
        assert!(problem.is_some());
        assert!(!settings.enabled);
        std::fs::write(
            &path,
            r#"{"format":"ugoe-studio-api","version":1,"enabled":true,"token":"short",
               "clients":[{"id":"a","name":"x","key":"weak","permission":"control","approvedAt":0}]}"#,
        )
        .unwrap();
        let (settings, problem) = ApiSettings::read(&path);
        assert!(problem.is_none());
        assert!(settings.enabled);
        assert!(is_secret(&settings.token));
        assert!(
            settings.clients.is_empty(),
            "a client with a weak key is dropped"
        );
        std::fs::write(&path, r#"{"format":"ugoe-studio-api","version":99}"#).unwrap();
        let (settings, problem) = ApiSettings::read(&path);
        assert!(problem.is_some());
        assert!(!settings.enabled);
    }

    #[test]
    fn keys_are_compared_in_full() {
        let mut settings = ApiSettings::default();
        let key = generate_secret().unwrap();
        settings.clients.push(ClientRecord {
            id: "a".into(),
            name: "a".into(),
            key: key.clone(),
            permission: Permission::Read,
            approved_at: 0,
            last_seen: None,
        });
        assert_eq!(settings.client_by_key(&key).unwrap().id, "a");
        assert!(settings.client_by_key(&key[..63]).is_none());
        assert!(settings.client_by_key("").is_none());
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
    }
}
