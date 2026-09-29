//! サーバを子プロセスとして起動し、stdio で JSON-RPC のメッセージを流して確かめる
//! (MCP クライアントがつないだときと同じ経路)。
//! 簡単な編集 (パラメータの作成とキーの追加) の提案・適用・Undo・保存までを通す。

#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;

use serde_json::{Value, json};
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Project, ProjectMeta, Textures};
use ugoe_model::fixtures;

/// テスト用の一時ディレクトリ。破棄時に消す。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("ugoe-mcp-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

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

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Client {
    fn spawn(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad line {line:?}: {e}"))
    }

    /// 要求を送り、応答 (同じ ID) を返す。
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let response = self.recv();
        assert_eq!(response["id"], id, "{response}");
        response
    }

    /// 旧来の握手をする。
    fn initialize(&mut self) -> Value {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" },
            }),
        );
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        response
    }

    /// ツールを呼び、`CallToolResult` を返す。
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        response["result"].clone()
    }

    /// 成功するはずのツールを呼び、構造化した結果を返す。
    fn call_ok(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], false, "{name}: {result}");
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, result["structuredContent"]);
        result["structuredContent"].clone()
    }

    /// 失敗するはずのツールを呼び、エラーの文を返す。
    fn call_err(&mut self, name: &str, arguments: Value) -> String {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], true, "{name}: {result}");
        result["content"][0]["text"].as_str().unwrap().to_owned()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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

#[test]
fn legacy_handshake_and_tools_list() {
    let mut client = Client::spawn(&[]);
    let init = client.initialize();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "ugoe-mcp");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let list = client.request("tools/list", json!({}));
    let names = tool_names(&list);
    for name in [
        "open_project",
        "close_project",
        "get_model_summary",
        "get_element",
        "propose_edit",
        "apply_proposal",
        "undo",
        "redo",
        "save_project",
        "render_png",
    ] {
        assert!(names.iter().any(|n| n == name), "missing {name}");
    }
    for tool in list["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
    }
    // 提案の入力の型は Command の JSON Schema から作る。
    let propose = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "propose_edit")
        .unwrap();
    let schema = &propose["inputSchema"];
    assert_eq!(
        schema["properties"]["commands"]["items"]["$ref"],
        "#/$defs/Command"
    );
    assert!(schema["$defs"]["Command"]["oneOf"].is_array());
    assert!(schema["$defs"]["KeyformTarget"].is_object());

    // 未知のバージョンには、応じられる最新の版を返す。
    let init = client.request("initialize", json!({ "protocolVersion": "1999-01-01" }));
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
}

#[test]
fn modern_requests_carry_meta() {
    let mut client = Client::spawn(&[]);
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    });
    let discover = client.request("server/discover", json!({ "_meta": meta }));
    assert!(
        discover["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );

    let list = client.request("tools/list", json!({ "_meta": meta }));
    assert_eq!(list["result"]["resultType"], "complete");
    assert_eq!(
        list["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "ugoe-mcp"
    );
    assert!(!tool_names(&list).is_empty());

    let bad = client.request(
        "tools/list",
        json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": "1900-01-01" } }),
    );
    assert_eq!(bad["error"]["code"], -32022);
    assert_eq!(bad["error"]["data"]["requested"], "1900-01-01");

    let missing = client.request(
        "tools/list",
        json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }),
    );
    assert_eq!(missing["error"]["code"], -32602);
}

#[test]
fn protocol_errors() {
    let mut client = Client::spawn(&[]);
    client.initialize();
    client.send(&json!("not an object"));
    assert_eq!(client.recv()["error"]["code"], -32600);
    writeln!(client.stdin, "{{ broken").unwrap();
    client.stdin.flush().unwrap();
    let parse = client.recv();
    assert_eq!(parse["error"]["code"], -32700);
    assert_eq!(parse["id"], Value::Null);
    let unknown = client.request("resources/list", json!({}));
    assert_eq!(unknown["error"]["code"], -32601);
    let tool = client.request("tools/call", json!({ "name": "nope", "arguments": {} }));
    assert_eq!(tool["error"]["code"], -32602);
    let ping = client.request("ping", json!({}));
    assert_eq!(ping["result"], json!({}));
}

/// パラメータを作り、メッシュの格子にその軸のキーを加える。提案 → 適用 → Undo/Redo → 保存。
#[test]
fn propose_apply_undo_and_save() {
    let tmp = TempDir::new("edit");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap();
    let mut client = Client::spawn(&[]);
    client.initialize();

    let opened = client.call_ok("open_project", json!({ "path": path }));
    assert_eq!(opened["counts"]["meshes"], 1);
    assert_eq!(opened["on_disk"], true);

    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(parameter_ids(&summary), ["X"]);
    assert_eq!(summary["meshes"][0]["id"], "Quad");

    let quad = client.call_ok(
        "get_element",
        json!({ "path": path, "kind": "mesh", "id": "Quad" }),
    );
    assert_eq!(quad["id"], "Quad");
    client.call_err(
        "get_element",
        json!({ "path": path, "kind": "mesh", "id": "Nope" }),
    );

    let commands = json!([
        {
            "op": "add_parameter",
            "parameter": { "id": "AngleX", "name": "Angle X", "min": -30.0, "default": 0.0, "max": 30.0 },
        },
        { "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Quad" }, "parameter": "AngleX", "key": -30.0 },
        { "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Quad" }, "parameter": "AngleX", "key": 30.0 },
    ]);
    let proposal = client.call_ok(
        "propose_edit",
        json!({
            "path": path,
            "title": "Add AngleX",
            "rationale": "face turns left and right",
            "commands": commands,
        }),
    );
    let id = proposal["proposal"].as_str().unwrap().to_owned();
    assert_eq!(
        proposal["changes"]["parameters"]["added"],
        json!(["AngleX"])
    );
    assert_eq!(proposal["changes"]["meshes"]["changed"], json!(["Quad"]));
    assert_eq!(proposal["report"]["warnings"], json!([]));
    // 提案だけではモデルは変わらない。
    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(parameter_ids(&summary), ["X"]);
    assert_eq!(summary["project"]["modified"], false);

    let applied = client.call_ok("apply_proposal", json!({ "path": path, "proposal": id }));
    assert_eq!(applied["project"]["modified"], true);
    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(parameter_ids(&summary), ["X", "AngleX"]);
    let axes = summary["meshes"][0]["keyforms"]["axes"].as_array().unwrap();
    let angle = axes.iter().find(|a| a["parameter"] == "AngleX").unwrap();
    assert_eq!(angle["keys"], json!([-30.0, 30.0]));

    // 同じ提案は 2 度適用できない。
    let err = client.call_err("apply_proposal", json!({ "path": path, "proposal": id }));
    assert!(err.contains("not found"), "{err}");

    // 検証を通らない提案は拒否され、ID を返さない。
    let err = client.call_err(
        "propose_edit",
        json!({
            "path": path,
            "title": "bad",
            "commands": [{ "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Nope" }, "parameter": "AngleX", "key": 1.0 }],
        }),
    );
    assert!(err.contains("rejected"), "{err}");
    let err = client.call_err(
        "propose_edit",
        json!({ "path": path, "title": "bad", "commands": [{ "op": "no_such_op" }] }),
    );
    assert!(err.contains("invalid arguments"), "{err}");

    // 提案の後にモデルが変わったら、その提案は適用できない。
    let stale = client.call_ok(
        "propose_edit",
        json!({
            "path": path,
            "title": "rename",
            "commands": [{ "op": "rename_parameter", "id": "AngleX", "name": "Angle" }],
        }),
    );
    let undone = client.call_ok("undo", json!({ "path": path }));
    assert_eq!(undone["undone"], true);
    assert_eq!(undone["modified"], false);
    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(parameter_ids(&summary), ["X"]);
    client.call_err(
        "apply_proposal",
        json!({ "path": path, "proposal": stale["proposal"] }),
    );
    let redone = client.call_ok("redo", json!({ "path": path }));
    assert_eq!(redone["redone"], true);

    // 未保存の変更があれば閉じられない。
    client.call_err("close_project", json!({ "path": path }));

    let bundle = tmp.0.join("quad.ugoe");
    let saved = client.call_ok(
        "save_project",
        json!({ "path": path, "bundle": bundle.to_str().unwrap() }),
    );
    assert_eq!(saved["modified"], false);
    assert!(fs::read(&bundle).unwrap().starts_with(b"UGOE"));
    let on_disk = Project::load(&project).unwrap();
    assert!(
        on_disk
            .model
            .parameters
            .iter()
            .any(|p| p.id.to_string() == "AngleX")
    );

    client.call_ok("close_project", json!({ "path": path }));
}

#[test]
fn create_new_project() {
    let tmp = TempDir::new("create");
    let project = tmp.0.join("new.ugoeproj");
    let path = project.to_str().unwrap();
    let mut client = Client::spawn(&[]);
    client.initialize();

    let err = client.call_err("get_model_summary", json!({ "path": path }));
    assert!(err.contains("project"), "{err}");
    let opened = client.call_ok("open_project", json!({ "path": path, "create": true }));
    assert_eq!(opened["name"], "new");
    assert_eq!(opened["on_disk"], false);
    let proposal = client.call_ok(
        "propose_edit",
        json!({
            "path": path,
            "title": "Add a parameter",
            "commands": [{
                "op": "add_parameter",
                "parameter": { "id": "Breath", "name": "Breath", "min": 0.0, "default": 0.0, "max": 1.0 },
            }],
        }),
    );
    client.call_ok(
        "apply_proposal",
        json!({ "path": path, "proposal": proposal["proposal"] }),
    );
    let saved = client.call_ok("save_project", json!({ "path": path }));
    assert_eq!(saved["on_disk"], true);
    let err = client.call_err("open_project", json!({ "path": path, "create": true }));
    assert!(err.contains("already a project"), "{err}");
    assert_eq!(
        Project::load(&project).unwrap().model.parameters[0]
            .id
            .to_string(),
        "Breath"
    );
}

#[test]
fn read_only_hides_edit_tools() {
    let tmp = TempDir::new("read-only");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap();
    let mut client = Client::spawn(&["--read-only"]);
    client.initialize();

    let names = tool_names(&client.request("tools/list", json!({})));
    assert!(names.iter().any(|n| n == "propose_edit"));
    for name in ["apply_proposal", "undo", "redo", "save_project"] {
        assert!(!names.iter().any(|n| n == name), "{name} should be hidden");
    }
    let proposal = client.call_ok(
        "propose_edit",
        json!({
            "path": path,
            "title": "rename",
            "commands": [{ "op": "rename_parameter", "id": "X", "name": "Renamed" }],
        }),
    );
    let err = client.call_err(
        "apply_proposal",
        json!({ "path": path, "proposal": proposal["proposal"] }),
    );
    assert!(err.contains("--read-only"), "{err}");
}

#[test]
fn render_reports_a_missing_cli() {
    let tmp = TempDir::new("render");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap();
    let missing = tmp.0.join("no-such-ugoe");
    let mut client = Client::spawn(&["--ugoe-bin", missing.to_str().unwrap()]);
    client.initialize();
    let err = client.call_err("render_png", json!({ "path": path }));
    assert!(err.contains("no-such-ugoe"), "{err}");
    assert!(err.contains("--ugoe-bin"), "{err}");
}
