//! `ugoe-mcp --editor`: stdio のクライアントと、実行中のエディタの MCP サーバの中継 (ADR 0023 の 3・4)。
//!
//! 標準入力の 1 行 (1 メッセージ) を、発見用のファイル ([`crate::discovery`]) が示すエディタへ HTTP で
//! 送り、答えを標準出力に 1 行で書く。メッセージの中身はほぼそのまま通すが、次の 2 つは中継が行う:
//!
//! - **パス:** ツールの引数のパス (`path`・`psd`・`bundle`、パスの形の `motion`・`expression`) のうち
//!   相対パスは、中継の作業ディレクトリ (= エージェントの作業ディレクトリ) から絶対パスにする
//!   (エディタの作業ディレクトリは意味を持たないので)。WSL と Windows のあいだなら、引数を Windows の
//!   パスに、結果 (`path`・`psd`・`bundle`・`closed`、エラーの文の `` `…` `` で囲んだパス) を WSL の
//!   パスに戻す ([`crate::wsl`])。
//! - **エラー:** エディタが見つからない・つながらない・版が合わないときは、ツールの呼び出しにはツールの
//!   エラー (`isError`) で、ほかの要求には JSON-RPC のエラーで、直し方を答える。発見用のファイルは
//!   要求のたびに読み直すので、中継を起動した後にエディタを起動してもよい。
//!
//! # どのエディタにつなぐか
//!
//! 1. `--discovery <FILE>` か `UGOE_MCP_DISCOVERY` があれば、そのファイルだけ。
//! 2. このプラットフォームの既定の場所 ([`crate::discovery::default_path`])。
//! 3. WSL の中なら、Windows の `%LOCALAPPDATA%` の下 ([`crate::wsl::windows_discovery_candidates`])。
//!    こちらを使うときはパスを変換する。Windows のループバックへは、WSL のネットワークが
//!    「ミラー」のときだけ届く (NAT なら、つながらないときに案内する)。
//!
//! 2 と 3 の両方にあれば、新しく書かれたほうを使う。
//!
//! # WSL から Windows 版を使う (推奨)
//!
//! WSL から Windows 版の `ugoe-mcp.exe --editor` を登録すると、Windows の中のループバックで完結する。
//! エージェントが渡す WSL のパスは、`--wsl-distro <名前>` (または作業ディレクトリが
//! `\\wsl.localhost\<名前>\…` のとき) で Windows のパスに変える。

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use crate::discovery::{self, Discovery, DiscoveryError};
use crate::server::META_PROTOCOL_VERSION;
use crate::wsl::{self, WslPaths};

/// ツールの引数のうちパスのもの (いつもパス)。
const PATH_ARGUMENTS: &[&str] = &["path", "psd", "bundle"];
/// ツールの引数のうち、名前かパスのもの (パスの形のときだけ変える)。
const NAME_OR_PATH_ARGUMENTS: &[&str] = &["motion", "expression"];
/// ツールの結果のうちパスのもの。
const PATH_RESULTS: &[&str] = &["path", "psd", "bundle", "closed"];
/// 中継のエラーの JSON-RPC のコード (実装が定めるサーバのエラー)。
const RELAY_ERROR: i64 = -32000;
/// エディタへの接続を待つ時間 (ループバックなのですぐ答えるはず。WSL の NAT では届かず待つ)。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// 答えの大きさの上限 (画像を含むツールの結果が収まる大きさ)。
const MAX_RESPONSE: u64 = 256 * 1024 * 1024;

/// 中継の設定。
#[derive(Debug, Clone, Default)]
pub struct RelayOptions {
    /// 発見用のファイル (`--discovery`)。`None` なら探す (モジュールの説明)。
    pub discovery: Option<PathBuf>,
    /// エージェントが WSL の中にいるときのディストリビューションの名前 (`--wsl-distro`。Windows 版で使う)。
    pub wsl_distro: Option<String>,
}

/// 中継が答えを返せない理由 (クライアントへの案内になる)。
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error(
        "the ugoe editor is not running, or its MCP bridge is off (no discovery file at {searched}). \
         Start the editor and turn on Settings > Connect agents"
    )]
    NotFound { searched: String },
    #[error("{0}")]
    Discovery(#[from] DiscoveryError),
    #[error(
        "could not connect to the ugoe editor at {url} (pid {pid}, from {}): {reason}. The editor \
         may have quit or turned the bridge off; start it again{hint}",
        .file.display()
    )]
    Unreachable {
        url: String,
        pid: u32,
        file: PathBuf,
        reason: String,
        hint: &'static str,
    },
    #[error(
        "the ugoe editor rejected the token from {}; the token may have been regenerated, try again",
        .0.display()
    )]
    Unauthorized(PathBuf),
    #[error("the ugoe editor answered HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("failed to talk to the ugoe editor: {0}")]
    Transport(String),
}

/// WSL の中から Windows のエディタにつながらないときの案内。
const WSL_NETWORK_HINT: &str = ". This ugoe-mcp runs inside WSL and the editor on Windows: WSL \
    reaches Windows' loopback only with mirrored networking (networkingMode=mirrored in \
    %UserProfile%\\.wslconfig, then `wsl --shutdown`). Alternatively register the Windows build \
    instead (`ugoe-mcp.exe --editor` from WSL), which needs no network setup";

/// つなぐエディタ。
#[derive(Debug, Clone)]
struct Target {
    discovery: Discovery,
    file: PathBuf,
    /// Windows のエディタの発見用のファイルを WSL の中から読んだ。
    windows_from_wsl: bool,
}

/// パスの変換。
#[derive(Debug, Clone)]
struct PathMap {
    /// WSL のパスと Windows のパスを変える (エージェントが WSL、エディタが Windows)。
    wsl: Option<WslPaths>,
}

impl PathMap {
    /// エージェントが渡したパスを、エディタに渡すパスにする。
    fn outgoing(&self, path: &str) -> String {
        let local = if path.starts_with('/') || Path::new(path).is_absolute() {
            path.to_owned()
        } else {
            match std::path::absolute(path) {
                Ok(absolute) => absolute.to_string_lossy().into_owned(),
                Err(_) => return path.to_owned(),
            }
        };
        match &self.wsl {
            Some(wsl) if local.starts_with('/') => wsl.to_windows(&local).unwrap_or(local),
            _ => local,
        }
    }

    /// エディタが返したパスを、エージェントに見せるパスにする。
    fn incoming(&self, path: &str) -> Option<String> {
        self.wsl.as_ref()?.to_wsl(path)
    }

    /// `tools/call` の引数を変える。
    fn arguments(&self, arguments: &mut Value) {
        let Some(arguments) = arguments.as_object_mut() else {
            return;
        };
        for (key, value) in arguments.iter_mut() {
            let Some(text) = value.as_str() else {
                continue;
            };
            let is_path = PATH_ARGUMENTS.contains(&key.as_str())
                || (NAME_OR_PATH_ARGUMENTS.contains(&key.as_str()) && looks_like_path(text));
            if is_path && !text.is_empty() {
                *value = Value::String(self.outgoing(text));
            }
        }
    }

    /// `tools/call` の結果 (`CallToolResult`) のパスを戻す。
    fn result(&self, result: &mut Value) {
        if self.wsl.is_none() {
            return;
        }
        if let Some(structured) = result.get_mut("structuredContent") {
            self.value(structured);
        }
        let is_error = result.get("isError").and_then(Value::as_bool) == Some(true);
        let Some(content) = result.get_mut("content").and_then(Value::as_array_mut) else {
            return;
        };
        for item in content {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }
            let Some(text) = item.get_mut("text") else {
                continue;
            };
            let Some(original) = text.as_str() else {
                continue;
            };
            let mapped = match serde_json::from_str::<Value>(original) {
                // 構造化した値と同じ JSON の文。
                Ok(mut value) if !is_error && value.is_object() => {
                    self.value(&mut value);
                    value.to_string()
                }
                _ => self.quoted(original),
            };
            *text = Value::String(mapped);
        }
    }

    /// JSON の中の、パスの名前の項目の文字列を戻す。
    fn value(&self, value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, item) in map.iter_mut() {
                    if PATH_RESULTS.contains(&key.as_str())
                        && let Some(mapped) = item.as_str().and_then(|p| self.incoming(p))
                    {
                        *item = Value::String(mapped);
                    } else {
                        self.value(item);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| self.value(item)),
            _ => {}
        }
    }

    /// 文の中の `` `…` `` で囲んだパスを戻す (エラーの文はパスをこの形で示す)。
    fn quoted(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut parts = text.split('`');
        if let Some(first) = parts.next() {
            out.push_str(first);
        }
        for (i, part) in parts.enumerate() {
            out.push('`');
            // 奇数番目 (0 始まりの i が偶数) が囲まれた部分。
            match self.incoming(part) {
                Some(mapped) if i % 2 == 0 => out.push_str(&mapped),
                _ => out.push_str(part),
            }
        }
        out
    }
}

/// 名前かパスの引数がパスの形か (区切りを含むか、`.json` で終わる)。
fn looks_like_path(text: &str) -> bool {
    text.contains('/') || text.contains('\\') || text.ends_with(".json")
}

/// 中継。
pub struct Relay {
    options: RelayOptions,
    agent: ureq::Agent,
    /// Windows 版で、エージェントが WSL にいるときの変換 (`--wsl-distro` か作業ディレクトリから)。
    local_wsl: Option<WslPaths>,
    /// 最後に使ったエディタ (版の違いの警告を 1 回だけ出すため)。
    last: Option<Target>,
}

impl std::fmt::Debug for Relay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relay")
            .field("options", &self.options)
            .field("local_wsl", &self.local_wsl)
            .finish_non_exhaustive()
    }
}

impl Relay {
    pub fn new(options: RelayOptions) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            // 適用はエディタで利用者の確認を待つので、答えの時間は限らない (エディタが時間切れを決める)。
            .timeout_global(None)
            .http_status_as_error(false)
            // ループバックへ直接つなぐ (HTTP_PROXY などを使わない)。
            .proxy(None)
            // 要求ごとに新しい接続にする (エディタが閉じた古い接続で送って失敗しないように)。
            .max_idle_connections(0)
            .build()
            .new_agent();
        let local_wsl = local_wsl(&options);
        Self {
            options,
            agent,
            local_wsl,
            last: None,
        }
    }

    /// 起動時の案内 (標準エラーへ): どのエディタにつなぐか、見つからないか。
    pub fn describe(&self) -> String {
        match self.locate() {
            Ok(target) => format!(
                "relaying to the ugoe editor at {} (pid {}, from {}){}",
                target.discovery.url(),
                target.discovery.pid,
                target.file.display(),
                match self.path_map(&target).wsl {
                    Some(wsl) => format!(
                        "; converting paths between WSL ({}) and Windows",
                        wsl.distro()
                    ),
                    None => String::new(),
                }
            ),
            Err(e) => format!("{e} (the relay keeps running and looks again on each request)"),
        }
    }

    /// 標準入力が閉じるまで中継する。
    pub fn run(&mut self, input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle_line(&line) {
                writeln!(output, "{reply}")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    /// 1 行 (1 メッセージ) を中継し、返す行を作る。
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let message = match serde_json::from_str::<Value>(line) {
            Ok(message) => message,
            // 壊れた行はエディタに解かせる (同じエラーを返す)。
            Err(_) => Value::String(line.to_owned()),
        };
        let reply = match &message {
            Value::String(raw) => self.post_raw(raw.clone()),
            _ => self.forward(message.clone()),
        };
        match reply {
            Ok(reply) => reply.map(|r| r.to_string()),
            Err(e) => {
                eprintln!("ugoe-mcp: {e}");
                error_reply(&message, &e).map(|r| r.to_string())
            }
        }
    }

    fn forward(&mut self, mut message: Value) -> Result<Option<Value>, RelayError> {
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let target = self.locate()?;
        let paths = self.path_map(&target);
        let is_call = method.as_deref() == Some("tools/call");
        if is_call && let Some(arguments) = message.pointer_mut("/params/arguments") {
            paths.arguments(arguments);
        }
        let headers = headers(&message);
        let mut reply = self.post(&target, &message, &headers)?;
        if is_call && let Some(result) = reply.as_mut().and_then(|r| r.get_mut("result")) {
            paths.result(result);
        }
        Ok(reply)
    }

    /// JSON として読めない行をそのまま送る。
    fn post_raw(&mut self, raw: String) -> Result<Option<Value>, RelayError> {
        let target = self.locate()?;
        self.send(&target, raw.into_bytes(), &[])
    }

    /// 送る。つながらない・トークンが違うときは、発見用のファイルを読み直して 1 回だけ送り直す
    /// (エディタが別のポート・トークンで起動し直したとき)。送り直すのは、要求が届いていないと
    /// 分かるときだけ。
    fn post(
        &mut self,
        target: &Target,
        message: &Value,
        headers: &[(&'static str, String)],
    ) -> Result<Option<Value>, RelayError> {
        let body = message.to_string().into_bytes();
        match self.send(target, body.clone(), headers) {
            Err(RelayError::Unreachable { .. } | RelayError::Unauthorized(_)) => {
                let retry = self.locate()?;
                self.send(&retry, body, headers)
            }
            result => result,
        }
    }

    fn send(
        &mut self,
        target: &Target,
        body: Vec<u8>,
        headers: &[(&'static str, String)],
    ) -> Result<Option<Value>, RelayError> {
        self.warn_version(target);
        let url = target.discovery.url();
        let mut request = self
            .agent
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header(
                "Authorization",
                &format!("Bearer {}", target.discovery.token),
            );
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        let mut response = request.send(&body[..]).map_err(|e| {
            if is_connect_error(&e) {
                RelayError::Unreachable {
                    url: url.clone(),
                    pid: target.discovery.pid,
                    file: target.file.clone(),
                    reason: e.to_string(),
                    hint: if target.windows_from_wsl {
                        WSL_NETWORK_HINT
                    } else {
                        ""
                    },
                }
            } else {
                RelayError::Transport(e.to_string())
            }
        })?;
        let status = response.status().as_u16();
        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE)
            .read_to_vec()
            .map_err(|e| RelayError::Transport(e.to_string()))?;
        match status {
            202 => Ok(None),
            401 => Err(RelayError::Unauthorized(target.file.clone())),
            _ => match serde_json::from_slice::<Value>(&bytes) {
                // JSON-RPC の答え (エラーの答えを 4xx で返すこともある)。
                Ok(reply) if reply.get("jsonrpc").is_some() => Ok(Some(reply)),
                _ => Err(RelayError::Http {
                    status,
                    body: String::from_utf8_lossy(&bytes).trim().to_owned(),
                }),
            },
        }
    }

    fn warn_version(&mut self, target: &Target) {
        let same = self
            .last
            .as_ref()
            .is_some_and(|last| last.file == target.file && last.discovery == target.discovery);
        if same {
            return;
        }
        if target.discovery.version != env!("CARGO_PKG_VERSION") {
            eprintln!(
                "ugoe-mcp: warning: the editor runs ugoe {} but this relay is {}; the tools are the \
                 editor's, but prefer the ugoe-mcp that ships with the editor",
                target.discovery.version,
                env!("CARGO_PKG_VERSION")
            );
        }
        self.last = Some(target.clone());
    }

    /// このエディタとのパスの変換。
    fn path_map(&self, target: &Target) -> PathMap {
        if target.windows_from_wsl {
            return PathMap {
                wsl: WslPaths::detect(),
            };
        }
        PathMap {
            wsl: self.local_wsl.clone(),
        }
    }

    /// つなぐエディタを探す (モジュールの説明の「どのエディタにつなぐか」)。
    fn locate(&self) -> Result<Target, RelayError> {
        if let Some(file) = &self.options.discovery {
            return self.read_target(file, false);
        }
        let mut candidates: Vec<(PathBuf, bool)> = Vec::new();
        if let Some(native) = discovery::default_path() {
            candidates.push((native, false));
        }
        // 既定の場所を環境変数で変えたなら、ほかは探さない。
        if std::env::var_os(discovery::DISCOVERY_ENV).is_none_or(|v| v.is_empty())
            && let Some(paths) = WslPaths::detect()
        {
            candidates.extend(
                wsl::windows_discovery_candidates(&paths)
                    .into_iter()
                    .map(|p| (p, true)),
            );
        }
        let newest = candidates
            .iter()
            .filter_map(|(path, windows)| {
                let modified = std::fs::metadata(path).ok()?.modified().ok()?;
                Some((modified, path, *windows))
            })
            .max_by_key(|(modified, _, _)| *modified);
        match newest {
            Some((_, path, windows)) => self.read_target(path, windows),
            None => Err(RelayError::NotFound {
                searched: if candidates.is_empty() {
                    "(no location: set UGOE_MCP_DISCOVERY)".to_owned()
                } else {
                    candidates
                        .iter()
                        .map(|(p, _)| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            }),
        }
    }

    fn read_target(&self, file: &Path, windows_from_wsl: bool) -> Result<Target, RelayError> {
        let discovery = match discovery::read(file) {
            Ok(discovery) => discovery,
            Err(DiscoveryError::Read { source, .. })
                if source.kind() == io::ErrorKind::NotFound =>
            {
                return Err(RelayError::NotFound {
                    searched: file.display().to_string(),
                });
            }
            Err(e) => return Err(e.into()),
        };
        Ok(Target {
            discovery,
            file: file.to_path_buf(),
            windows_from_wsl,
        })
    }
}

/// Windows 版で、エージェントが WSL にいるときの変換。`--wsl-distro` か、作業ディレクトリが
/// `\\wsl.localhost\<名前>\…` (WSL の Linux のディレクトリから .exe を起動したとき) から決める。
fn local_wsl(options: &RelayOptions) -> Option<WslPaths> {
    if !cfg!(windows) {
        return None;
    }
    let distro = options.wsl_distro.clone().or_else(|| {
        let cwd = std::env::current_dir().ok()?;
        distro_from_unc(&cwd.to_string_lossy())
    })?;
    Some(WslPaths::new(distro, "/mnt/"))
}

/// `\\wsl.localhost\<名前>\…`・`\\wsl$\<名前>\…`・`\\?\UNC\wsl.localhost\<名前>\…` の名前。
fn distro_from_unc(path: &str) -> Option<String> {
    let path = path.replace('/', "\\");
    let unc = path
        .strip_prefix("\\\\?\\UNC\\")
        .or_else(|| path.strip_prefix("\\\\"))?;
    let mut parts = unc.split('\\');
    let server = parts.next()?;
    if !(server.eq_ignore_ascii_case("wsl.localhost") || server.eq_ignore_ascii_case("wsl$")) {
        return None;
    }
    parts.next().filter(|s| !s.is_empty()).map(str::to_owned)
}

/// ureq のエラーが、つなぐ前 (要求が届いていない) のものか。
fn is_connect_error(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::Io(e) => matches!(
            e.kind(),
            io::ErrorKind::ConnectionRefused
                | io::ErrorKind::AddrNotAvailable
                | io::ErrorKind::NetworkUnreachable
                | io::ErrorKind::HostUnreachable
        ),
        ureq::Error::Timeout(timeout) => matches!(timeout, ureq::Timeout::Connect),
        ureq::Error::ConnectionFailed => true,
        _ => false,
    }
}

/// 要求の版に合わせたヘッダ (要求ごとに `_meta` で版を示す版では `MCP-Protocol-Version`・
/// `Mcp-Method`・`Mcp-Name` が要る。`crate::http` が本文と一致を確かめる)。
fn headers(message: &Value) -> Vec<(&'static str, String)> {
    let mut headers = Vec::new();
    let params = message.get("params");
    let Some(version) = params
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get(META_PROTOCOL_VERSION))
        .and_then(Value::as_str)
    else {
        return headers;
    };
    headers.push(("MCP-Protocol-Version", version.to_owned()));
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return headers;
    };
    headers.push(("Mcp-Method", method.to_owned()));
    let name = match method {
        "tools/call" | "prompts/get" => params.and_then(|p| p.get("name")),
        "resources/read" => params.and_then(|p| p.get("uri")),
        _ => None,
    };
    if let Some(name) = name.and_then(Value::as_str) {
        headers.push(("Mcp-Name", header_value(name)));
    }
    headers
}

/// ヘッダに書ける値 (ASCII の見える文字と空白でなければ `=?base64?…?=`)。
fn header_value(value: &str) -> String {
    use base64::Engine;
    if value.bytes().all(|b| (0x20..0x7f).contains(&b)) {
        value.to_owned()
    } else {
        format!(
            "=?base64?{}?=",
            base64::engine::general_purpose::STANDARD.encode(value)
        )
    }
}

/// 中継できなかった要求への答え。ツールの呼び出しにはツールのエラー (エージェントが読める)、
/// ほかの要求には JSON-RPC のエラー。通知には答えない。
fn error_reply(message: &Value, error: &RelayError) -> Option<Value> {
    let id = message.get("id")?.clone();
    let text = error.to_string();
    if message.get("method").and_then(Value::as_str) == Some("tools/call") {
        return Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": text }],
                "isError": true,
            },
        }));
    }
    Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": RELAY_ERROR, "message": text },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wsl_map() -> PathMap {
        PathMap {
            wsl: Some(WslPaths::new("Ubuntu", "/mnt/")),
        }
    }

    #[test]
    fn arguments_become_windows_paths() {
        let map = wsl_map();
        let mut args = json!({
            "path": "/mnt/c/work/a.ugoeproj",
            "psd": "/home/u/art.psd",
            "bundle": "/mnt/d/out.ugoe",
            "motion": "idle",
            "expression": "/home/u/smile.expression.json",
            "title": "/not/a/path/argument",
            "params": { "path": "/x" },
        });
        map.arguments(&mut args);
        assert_eq!(args["path"], "C:\\work\\a.ugoeproj");
        assert_eq!(args["psd"], "\\\\wsl.localhost\\Ubuntu\\home\\u\\art.psd");
        assert_eq!(args["bundle"], "D:\\out.ugoe");
        assert_eq!(args["motion"], "idle");
        assert_eq!(
            args["expression"],
            "\\\\wsl.localhost\\Ubuntu\\home\\u\\smile.expression.json"
        );
        assert_eq!(args["title"], "/not/a/path/argument");
        assert_eq!(args["params"]["path"], "/x");
    }

    #[test]
    fn relative_arguments_resolve_from_the_relay() {
        let cwd = std::env::current_dir().unwrap();
        let native = PathMap { wsl: None };
        // `motions/walk.motion.json` の区切りは、Windows では `\\` に正規化される。
        let mut args = json!({ "path": "a.ugoeproj", "motion": "walk.motion.json" });
        native.arguments(&mut args);
        assert_eq!(
            args["path"],
            cwd.join("a.ugoeproj").to_string_lossy().as_ref()
        );
        assert_eq!(
            args["motion"],
            cwd.join("walk.motion.json").to_string_lossy().as_ref()
        );
        if cfg!(unix) {
            let mut args = json!({ "path": "a.ugoeproj" });
            wsl_map().arguments(&mut args);
            let expected = wsl_map()
                .wsl
                .unwrap()
                .to_windows(&cwd.join("a.ugoeproj").to_string_lossy())
                .unwrap();
            assert_eq!(args["path"], expected);
        }
    }

    #[test]
    fn results_become_wsl_paths() {
        let map = wsl_map();
        let structured = json!({
            "path": "\\\\?\\C:\\work\\a.ugoeproj",
            "project": { "path": "\\\\wsl.localhost\\Ubuntu\\home\\u\\b.ugoeproj", "revision": 1 },
            "psd": "C:\\art.psd",
            "layers": [{ "path": ["Head", "Eye"] }],
            "name": "C:\\not-a-path-key",
        });
        let mut result = json!({
            "content": [
                { "type": "text", "text": structured.to_string() },
                { "type": "image", "data": "AAAA", "mimeType": "image/png" },
            ],
            "structuredContent": structured,
            "isError": false,
        });
        map.result(&mut result);
        let expected = json!({
            "path": "/mnt/c/work/a.ugoeproj",
            "project": { "path": "/home/u/b.ugoeproj", "revision": 1 },
            "psd": "/mnt/c/art.psd",
            "layers": [{ "path": ["Head", "Eye"] }],
            "name": "C:\\not-a-path-key",
        });
        assert_eq!(result["structuredContent"], expected);
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, expected);
        assert_eq!(result["content"][1]["data"], "AAAA");

        let mut error = json!({
            "content": [{ "type": "text", "text": "`C:\\work\\a.ugoeproj` has unsaved changes; `x` stays" }],
            "isError": true,
        });
        map.result(&mut error);
        assert_eq!(
            error["content"][0]["text"],
            "`/mnt/c/work/a.ugoeproj` has unsaved changes; `x` stays"
        );
    }

    #[test]
    fn native_results_are_untouched() {
        let mut result = json!({
            "content": [{ "type": "text", "text": "{\"path\":\"C:\\\\a\"}" }],
            "structuredContent": { "path": "C:\\a" },
        });
        let before = result.clone();
        PathMap { wsl: None }.result(&mut result);
        assert_eq!(result, before);
    }

    #[test]
    fn modern_requests_get_headers() {
        let modern = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "get_model_summary", "_meta": { META_PROTOCOL_VERSION: "2026-07-28" } },
        });
        assert_eq!(
            headers(&modern),
            vec![
                ("MCP-Protocol-Version", "2026-07-28".to_owned()),
                ("Mcp-Method", "tools/call".to_owned()),
                ("Mcp-Name", "get_model_summary".to_owned()),
            ]
        );
        let legacy = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        assert!(headers(&legacy).is_empty());
        assert_eq!(header_value("名前"), "=?base64?5ZCN5YmN?=");
    }

    #[test]
    fn errors_answer_tool_calls_as_tool_errors() {
        let error = RelayError::NotFound {
            searched: "/x/editor-mcp.json".to_owned(),
        };
        let call = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {} });
        let reply = error_reply(&call, &error).unwrap();
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not running")
        );
        let list = json!({ "jsonrpc": "2.0", "id": "a", "method": "tools/list" });
        let reply = error_reply(&list, &error).unwrap();
        assert_eq!(reply["error"]["code"], RELAY_ERROR);
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(error_reply(&note, &error).is_none());
    }

    #[test]
    fn distro_names_from_unc_working_directories() {
        assert_eq!(
            distro_from_unc("\\\\wsl.localhost\\Ubuntu\\home\\u").as_deref(),
            Some("Ubuntu")
        );
        assert_eq!(
            distro_from_unc("\\\\wsl$\\Debian").as_deref(),
            Some("Debian")
        );
        assert_eq!(
            distro_from_unc("\\\\?\\UNC\\wsl.localhost\\Arch\\x").as_deref(),
            Some("Arch")
        );
        assert_eq!(distro_from_unc("\\\\server\\share"), None);
        assert_eq!(distro_from_unc("C:\\Users"), None);
    }

    #[test]
    fn locates_the_named_discovery_file() {
        let dir = std::env::temp_dir().join(format!("ugoe-mcp-relay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("editor-mcp.json");
        let relay = Relay::new(RelayOptions {
            discovery: Some(file.clone()),
            wsl_distro: None,
        });
        assert!(matches!(relay.locate(), Err(RelayError::NotFound { .. })));
        let discovery = Discovery::new(1, "t".repeat(64));
        discovery::write(&file, &discovery).unwrap();
        let target = relay.locate().unwrap();
        assert_eq!(target.discovery, discovery);
        assert!(!target.windows_from_wsl);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
