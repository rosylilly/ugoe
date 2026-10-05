//! サーバを `--http` で子プロセスとして起動し、HTTP (MCP の Streamable HTTP) で確かめる
//! (ChatGPT などの HTTP の MCP クライアントがつないだときと同じ経路)。
//! 握手 → ツールの一覧 → ツールの呼び出し (提案・適用・保存) と、トークン・Origin・Host の守り。

#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use serde_json::{Value, json};
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Project, ProjectMeta, Textures};
use ugoe_model::fixtures;

const TOKEN: &str = "test-token-0123456789abcdef0123456789abcdef";
const MODERN: &str = "2026-07-28";

/// `single_quad` (メッシュ `Quad`、パラメータ `X`) のプロジェクトを保存する。
fn save_fixture_project(dir: &Path) -> PathBuf {
    let model = fixtures::single_quad();
    let mut textures = Textures::new();
    for t in &model.textures {
        let png: Arc<[u8]> = solid_png(t.width, t.height, [255, 255, 255, 255]).into();
        textures.insert(t.name.clone(), png);
    }
    let path = dir.join("quad.ugoeproj");
    Project::new(ProjectMeta::new("quad"), model, textures)
        .save(&path)
        .unwrap();
    path
}

/// `--http 127.0.0.1:0` で起動したサーバ。標準エラーの最初の行から URL を読む。
struct HttpServer {
    child: Child,
    /// `http://127.0.0.1:PORT/mcp`
    url: String,
    /// `127.0.0.1:PORT`
    authority: String,
}

impl HttpServer {
    fn spawn(args: &[&str], token: Option<&str>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"));
        command
            .args(["--http", "127.0.0.1:0"])
            .args(args)
            .env_remove("UGOE_MCP_TOKEN")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(token) = token {
            command.env("UGOE_MCP_TOKEN", token);
        }
        let mut child = command.spawn().unwrap();
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        stderr.read_line(&mut line).unwrap();
        let url = line
            .split_whitespace()
            .find(|w| w.starts_with("http://"))
            .unwrap_or_else(|| panic!("no URL in {line:?}"))
            .to_owned();
        let authority = url
            .trim_start_matches("http://")
            .trim_end_matches("/mcp")
            .to_owned();
        // 標準エラー (拒否のログ) を読み捨てて、パイプが詰まらないようにする。
        std::thread::spawn(move || std::io::copy(&mut stderr, &mut std::io::sink()));
        Self {
            child,
            url,
            authority,
        }
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// HTTP の MCP クライアント (ureq)。
struct Client {
    agent: ureq::Agent,
    url: String,
    headers: Vec<(String, String)>,
    next_id: u64,
}

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("{e}: {:?}", self.body))
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

impl Client {
    fn new(url: &str) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .new_agent(),
            url: url.to_owned(),
            headers: Vec::new(),
            next_id: 1,
        }
    }

    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    fn post(&self, message: &Value, headers: &[(&str, &str)]) -> Reply {
        let mut request = self
            .agent
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send(message.to_string()).unwrap();
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_owned()))
            .collect();
        let body = response.into_body().read_to_string().unwrap();
        Reply {
            status,
            headers,
            body,
        }
    }

    fn request(&mut self, method: &str, params: Value, headers: &[(&str, &str)]) -> Reply {
        let id = self.next_id;
        self.next_id += 1;
        let reply = self.post(
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
            headers,
        );
        if reply.status == 200 {
            assert_eq!(reply.json()["id"], id, "{}", reply.body);
        }
        reply
    }

    /// 旧来の版 (2025-11-25) でツールを呼び、構造化した結果を返す。
    fn call_ok(&mut self, name: &str, arguments: Value) -> Value {
        let reply = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
            &[("MCP-Protocol-Version", "2025-11-25")],
        );
        assert_eq!(reply.status, 200, "{name}: {}", reply.body);
        let result = reply.json()["result"].clone();
        assert_eq!(result["isError"], false, "{name}: {result}");
        result["structuredContent"].clone()
    }
}

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn tool_names(list: &Value) -> Vec<String> {
    list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}

fn parameter_ids(summary: &Value) -> Vec<String> {
    summary["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .collect()
}

/// 1 つの TCP 接続に生の HTTP を書き、応答を 1 つ読む (ヘッダを自由に書くため)。
fn raw_request(stream: &mut TcpStream, head: &str, body: &str) -> (u16, String) {
    write!(stream, "{head}Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    let status: u16 = status_line.split(' ').nth(1).unwrap().parse().unwrap();
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    (status, String::from_utf8(body).unwrap())
}

/// 旧来の握手 (initialize) → ツールの一覧 → 提案・適用・保存。版を示す要求も同じ状態を見る。
#[test]
fn legacy_and_modern_clients_call_tools_over_http() {
    let tmp = tempfile::tempdir().unwrap();
    let project = save_fixture_project(tmp.path());
    // ツールが触れるパスを一時ディレクトリの下に限る。相対パスはそこから解決する。
    let path = "quad.ugoeproj";
    let server = HttpServer::spawn(&["--root", tmp.path().to_str().unwrap()], None);
    let mut client = Client::new(&server.url);

    let init = client.request(
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" },
        }),
        &[],
    );
    assert_eq!(init.status, 200);
    assert!(
        init.header("content-type")
            .unwrap()
            .starts_with("application/json")
    );
    // セッションは発行しない。
    assert!(init.header("mcp-session-id").is_none());
    assert_eq!(init.json()["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(init.json()["result"]["serverInfo"]["name"], "ugoe-mcp");

    let initialized = client.post(
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        &[("MCP-Protocol-Version", "2025-11-25")],
    );
    assert_eq!(initialized.status, 202);
    assert!(initialized.body.is_empty());

    let list = client.request(
        "tools/list",
        json!({}),
        &[("MCP-Protocol-Version", "2025-11-25")],
    );
    assert_eq!(list.status, 200);
    let names = tool_names(&list.json());
    for name in [
        "open_project",
        "propose_edit",
        "apply_proposal",
        "save_project",
        "render_png",
    ] {
        assert!(names.iter().any(|n| n == name), "missing {name}");
    }

    let opened = client.call_ok("open_project", json!({ "path": path }));
    assert_eq!(opened["counts"]["meshes"], 1);
    // root の外は開けない。
    let reply = client.request(
        "tools/call",
        json!({ "name": "open_project", "arguments": { "path": "../elsewhere.ugoeproj", "create": true } }),
        &[("MCP-Protocol-Version", "2025-11-25")],
    );
    let result = &reply.json()["result"];
    assert_eq!(result["isError"], true, "{result}");
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("outside the root"),
        "{result}"
    );
    let proposal = client.call_ok(
        "propose_edit",
        json!({
            "path": path,
            "title": "Add AngleX",
            "commands": [
                { "op": "add_parameter", "parameter": { "id": "AngleX", "name": "Angle X", "min": -30.0, "default": 0.0, "max": 30.0 } },
                { "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Quad" }, "parameter": "AngleX", "key": 30.0 },
            ],
        }),
    );
    assert_eq!(
        proposal["changes"]["parameters"]["added"],
        json!(["AngleX"])
    );
    let applied = client.call_ok(
        "apply_proposal",
        json!({ "path": path, "proposal": proposal["proposal"] }),
    );
    assert_eq!(applied["project"]["modified"], true);

    // 別のクライアント (要求ごとに版を示す版) も同じサーバの状態を見る。
    let mut modern = Client::new(&server.url);
    let reply = modern.request(
        "tools/call",
        json!({ "name": "get_model_summary", "arguments": { "path": path }, "_meta": modern_meta() }),
        &[
            ("MCP-Protocol-Version", MODERN),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "get_model_summary"),
        ],
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    let result = &reply.json()["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(parameter_ids(&result["structuredContent"]), ["X", "AngleX"]);

    // ヘッダと本文が食い違えば 400 と HeaderMismatch。
    let reply = modern.request(
        "tools/call",
        json!({ "name": "get_model_summary", "arguments": { "path": path }, "_meta": modern_meta() }),
        &[
            ("MCP-Protocol-Version", MODERN),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "save_project"),
        ],
    );
    assert_eq!(reply.status, 400);
    assert_eq!(reply.json()["error"]["code"], -32020);

    let saved = client.call_ok("save_project", json!({ "path": path }));
    assert_eq!(saved["modified"], false);
    let on_disk = Project::load(&project).unwrap();
    assert!(on_disk.model.parameters.iter().any(|p| p.id == "AngleX"));
}

/// 生の HTTP: GET と DELETE は 405、keep-alive で続けて送れる、Host と Origin の守り。
#[test]
fn raw_http_behaviour_and_guards() {
    let server = HttpServer::spawn(&["--allow-any-path"], None);
    let host = &server.authority;
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let post = |extra: &str| {
        format!(
            "POST /mcp HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
             Accept: application/json, text/event-stream\r\n{extra}"
        )
    };

    let mut stream = TcpStream::connect(host).unwrap();
    let (status, _) = raw_request(
        &mut stream,
        &format!("GET /mcp HTTP/1.1\r\nHost: {host}\r\nAccept: text/event-stream\r\n"),
        "",
    );
    assert_eq!(status, 405);
    // 同じ接続で続けて送れる (keep-alive)。
    for _ in 0..2 {
        let (status, body) = raw_request(&mut stream, &post(""), ping);
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["result"],
            json!({})
        );
    }
    let (status, _) = raw_request(
        &mut stream,
        &format!("DELETE /mcp HTTP/1.1\r\nHost: {host}\r\nMcp-Session-Id: x\r\n"),
        "",
    );
    assert_eq!(status, 405);

    for (extra, expected) in [
        ("Origin: https://evil.example\r\n", 403),
        ("Origin: http://localhost:5173\r\n", 200),
        ("X-Forwarded-For: 203.0.113.9\r\n", 403),
    ] {
        let mut stream = TcpStream::connect(host).unwrap();
        let (status, body) = raw_request(&mut stream, &post(extra), ping);
        assert_eq!(status, expected, "{extra}: {body}");
    }
    // DNS リバインディング (攻撃者の名前がこのアドレスに向いている) やトンネルの公開名。
    let mut stream = TcpStream::connect(host).unwrap();
    let (status, _) = raw_request(
        &mut stream,
        "POST /mcp HTTP/1.1\r\nHost: rebind.evil.example\r\nContent-Type: application/json\r\n",
        ping,
    );
    assert_eq!(status, 403);
}

/// トークンを求めるサーバ: ヘッダ (Bearer) かパス (/mcp/<token>) で渡す。
#[test]
fn token_is_required_when_set() {
    let server = HttpServer::spawn(&["--read-only", "--allow-any-path"], Some(TOKEN));
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });

    let reply = Client::new(&server.url).post(&ping, &[]);
    assert_eq!(reply.status, 401);
    assert!(
        reply
            .header("www-authenticate")
            .unwrap()
            .starts_with("Bearer")
    );
    let reply = Client::new(&server.url)
        .with_header("Authorization", "Bearer not-the-token-at-all")
        .post(&ping, &[]);
    assert_eq!(reply.status, 401);

    let mut bearer =
        Client::new(&server.url).with_header("Authorization", &format!("Bearer {TOKEN}"));
    let list = bearer.request("tools/list", json!({}), &[]);
    assert_eq!(list.status, 200);
    // --read-only なので適用のツールは出さない。
    assert!(
        !tool_names(&list.json())
            .iter()
            .any(|n| n == "apply_proposal")
    );

    // ヘッダを送れないクライアント (ChatGPT の認証なしのコネクタ) のために、パスのトークンも受ける。
    // トンネルを通った要求 (公開名の Host・転送ヘッダ) でも、トークンがあれば受ける。
    let mut path = Client::new(&format!("{}/{TOKEN}", server.url))
        .with_header("X-Forwarded-For", "203.0.113.9");
    assert_eq!(path.request("ping", json!({}), &[]).status, 200);
    let reply = Client::new(&format!("{}/wrong", server.url)).post(&ping, &[]);
    assert_eq!(reply.status, 401);
}

/// ループバック以外に束ねるには、トークンが要る。
#[test]
fn refuses_to_listen_off_loopback_without_a_token() {
    let output = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
        .args(["--http", "0.0.0.0:0", "--allow-any-path"])
        .env_remove("UGOE_MCP_TOKEN")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("without a token"), "{stderr}");

    let output = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
        .args([
            "--http",
            "127.0.0.1:0",
            "--allow-any-path",
            "--token",
            "short",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at least 32"), "{stderr}");
}

/// HTTP では、パスを限るか (`--root`) 限らないか (`--allow-any-path`) を明示しないと起動しない
/// (`--read-only` だけでは足りない)。`--root` は既存のディレクトリ。
#[test]
fn http_requires_root_or_allow_any_path() {
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
            .args(["--http", "127.0.0.1:0"])
            .args(args)
            .env_remove("UGOE_MCP_TOKEN")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    for args in [&[][..], &["--read-only"][..]] {
        let output = run(args);
        assert!(!output.status.success(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--http needs --root"), "{args:?}: {stderr}");
    }
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("missing");
    let output = run(&["--root", missing.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid --root"), "{stderr}");
    // 両方は付けられない。
    let output = run(&["--root", tmp.path().to_str().unwrap(), "--allow-any-path"]);
    assert!(!output.status.success());

    // --root を付ければ起動する (最初の行に限る先を書く)。
    let server = HttpServer::spawn(&["--root", tmp.path().to_str().unwrap()], None);
    let mut client = Client::new(&server.url);
    assert_eq!(client.request("ping", json!({}), &[]).status, 200);
}

/// `--generate-token` で作ったトークン (標準エラーに書く) を求める。
#[test]
fn generated_token_is_printed_and_required() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
        .args([
            "--http",
            "127.0.0.1:0",
            "--allow-any-path",
            "--generate-token",
        ])
        .env_remove("UGOE_MCP_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut lines = Vec::new();
    for _ in 0..3 {
        let mut line = String::new();
        stderr.read_line(&mut line).unwrap();
        lines.push(line);
    }
    let url = lines[0]
        .split_whitespace()
        .find(|w| w.starts_with("http://"))
        .unwrap()
        .to_owned();
    assert!(lines[0].contains("token required"), "{lines:?}");
    let token = lines[1].trim().rsplit(' ').next().unwrap().to_owned();
    assert_eq!(token.len(), 64, "{lines:?}");

    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    assert_eq!(Client::new(&url).post(&ping, &[]).status, 401);
    let bearer = Client::new(&url).with_header("Authorization", &format!("Bearer {token}"));
    assert_eq!(bearer.post(&ping, &[]).status, 200);
    let _ = child.kill();
    let _ = child.wait();
}
