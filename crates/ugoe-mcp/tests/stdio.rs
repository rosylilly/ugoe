//! サーバを子プロセスとして起動し、stdio で JSON-RPC のメッセージを流して確かめる
//! (MCP クライアントがつないだときと同じ経路)。
//! 簡単な編集 (パラメータの作成とキーの追加) の提案・適用・Undo・保存までを通す。

#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
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
    /// [`Client::shutdown`] で閉じる。
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Client {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_with(Command::new(env!("CARGO_BIN_EXE_ugoe-mcp")).args(args))
    }

    fn spawn_with(command: &mut Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
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

impl Client {
    /// 標準入力を閉じ (サーバは読み終えて終了する)、終了を待つ。
    #[cfg_attr(not(feature = "gpu-tests"), allow(dead_code))]
    fn shutdown(&mut self) -> ExitStatus {
        drop(self.stdin.take());
        self.child.wait().unwrap()
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
    let stdin = client.stdin.as_mut().unwrap();
    writeln!(stdin, "{{ broken").unwrap();
    stdin.flush().unwrap();
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

    // 同じ提案は 2 度適用できない (適用でモデルが変わったので)。
    let err = client.call_err("apply_proposal", json!({ "path": path, "proposal": id }));
    assert!(err.contains("the model changed"), "{err}");
    let err = client.call_err("apply_proposal", json!({ "path": path, "proposal": "p99" }));
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
fn render_reports_bad_arguments_before_touching_the_gpu() {
    let tmp = TempDir::new("render");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap();
    let mut client = Client::spawn(&[]);
    client.initialize();
    let err = client.call_err(
        "render_png",
        json!({ "path": path, "params": { "Nope": 1 } }),
    );
    assert!(err.contains("unknown parameter `Nope` (known: X)"), "{err}");
    let err = client.call_err("render_png", json!({ "path": path, "background": "#fff" }));
    assert!(err.contains("invalid background"), "{err}");
}

/// 胸から上の実際の PSD (`assets/fixtures/psd-real/`)。
fn bustup_psd() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd")
}

/// MCP の image コンテンツの PNG の幅と高さ (IHDR)。
fn png_size(content: &Value) -> (u32, u32) {
    use base64::Engine;
    assert_eq!(content["type"], "image", "{content}");
    assert_eq!(content["mimeType"], "image/png");
    let png = base64::engine::general_purpose::STANDARD
        .decode(content["data"].as_str().unwrap())
        .unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

/// 自動リギングの判断をエージェントに任せる流れ: PSD の取り込み → レイヤーと画像 → 分類を渡す →
/// 値を渡して提案 (範囲の外なら直して呼び直す) → 適用 → 保存。
#[test]
fn agent_driven_auto_rig_from_psd() {
    let tmp = TempDir::new("auto-rig");
    let project = tmp.0.join("bustup.ugoeproj");
    let path = project.to_str().unwrap();
    let psd = bustup_psd();
    let mut client = Client::spawn(&[]);
    client.initialize();

    // 分類の語彙と値の形は ugoe_ai の JSON Schema から作る。
    let list = client.request("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let schema_of =
        |name: &str| tools.iter().find(|t| t["name"] == name).unwrap()["inputSchema"].clone();
    let labels = schema_of("set_layer_labels");
    let expected = ugoe_ai::classify::output_schema();
    let item = &expected["properties"]["layers"]["items"]["properties"];
    let got = &labels["properties"]["layers"]["items"]["properties"];
    assert_eq!(got["label"], item["label"]);
    assert_eq!(got["side"], item["side"]);
    let rig = schema_of("propose_rig");
    assert_eq!(
        rig["properties"]["params"]["properties"],
        ugoe_ai::rig::rig_output_schema()["properties"]
    );

    // PSD から新しいプロジェクトを作る (ディスクにはまだ書かない)。
    let imported = client.call_ok(
        "import_psd",
        json!({ "path": path, "psd": psd.to_str().unwrap() }),
    );
    assert_eq!(imported["on_disk"], false);
    assert_eq!(imported["modified"], true);
    let total = imported["layers"].as_u64().unwrap() as usize;
    assert!(total > 10, "{imported}");
    assert!(imported["meshes"].as_u64().unwrap() > 10);

    // 最初のページ: 一覧・指示・全体の画像・サムネイル。
    let result = client.call("get_psd_layers", json!({ "path": path, "count": 4 }));
    assert_eq!(result["isError"], false, "{result}");
    let page = &result["structuredContent"];
    assert_eq!(page["total"], total);
    assert_eq!(page["next_start"], 4);
    let instructions = page["instructions"].as_str().unwrap();
    assert!(
        instructions.contains("character's OWN left"),
        "{instructions}"
    );
    assert!(instructions.contains("eye_white"), "{instructions}");
    let content = result["content"].as_array().unwrap();
    assert!(content[1]["text"].as_str().unwrap().starts_with("overview"));
    let (w, h) = png_size(&content[2]);
    assert!(w.max(h) <= 1024 && w.max(h) >= 256, "{w}x{h}");
    // 画素のあるレイヤーごとに `layer N: 名前` の文と画像。
    let layers = page["layers"].as_array().unwrap();
    let with_pixels: Vec<&Value> = layers.iter().filter(|l| l["has_pixels"] == true).collect();
    assert_eq!(content.len(), 3 + 2 * with_pixels.len());
    for (k, layer) in with_pixels.iter().enumerate() {
        let caption = content[3 + 2 * k]["text"].as_str().unwrap();
        assert!(
            caption.starts_with(&format!("layer {}: ", layer["index"])),
            "{caption}"
        );
        let (w, h) = png_size(&content[4 + 2 * k]);
        assert!(w.max(h) <= 256, "{w}x{h}");
        assert!(layer["mesh"].is_string(), "{layer}");
        assert_eq!(layer["set_by"], "rules");
    }
    // 画像を小さくできる。indices で特定のレイヤーを見る。
    let result = client.call(
        "get_psd_layers",
        json!({ "path": path, "indices": [with_pixels[0]["index"]], "thumbnail_size": 64 }),
    );
    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 3, "no overview for indices");
    let (w, h) = png_size(&content[2]);
    assert!(w.max(h) <= 64, "{w}x{h}");
    client.call_err(
        "get_psd_layers",
        json!({ "path": path, "indices": [total] }),
    );

    // 一覧だけを全部読む (エージェントが名前の規則の案を見て判断する代わりに、その案をそのまま渡す)。
    let mut all = Vec::new();
    let mut start = Some(0);
    while let Some(s) = start {
        let result = client.call(
            "get_psd_layers",
            json!({ "path": path, "start": s, "count": 32, "images": false, "overview": false }),
        );
        assert_eq!(result["content"].as_array().unwrap().len(), 1);
        let page = &result["structuredContent"];
        all.extend(page["layers"].as_array().unwrap().iter().cloned());
        start = page["next_start"].as_u64();
    }
    assert_eq!(all.len(), total);

    // 誤った分類はツールのエラーで返り、分類は変わらない。
    let err = client.call_err(
        "set_layer_labels",
        json!({ "path": path, "layers": [{ "index": total, "label": "face", "side": "center", "confidence": 1 }] }),
    );
    assert!(err.contains("does not exist"), "{err}");
    let err = client.call_err(
        "set_layer_labels",
        json!({ "path": path, "layers": [{ "index": 0, "label": "halo", "side": "center", "confidence": 1 }] }),
    );
    assert!(err.contains("does not match the schema"), "{err}");

    let answer: Vec<Value> = all
        .iter()
        .map(|l| json!({ "index": l["index"], "label": l["label"], "side": l["side"], "confidence": 0.9 }))
        .collect();
    let summary = client.call_ok(
        "set_layer_labels",
        json!({ "path": path, "layers": answer }),
    );
    assert_eq!(summary["updated"], total);
    assert_eq!(summary["set_by_agent"], total);
    assert_eq!(summary["rig"]["ready"], true, "{summary}");
    assert_eq!(summary["needs_review"], json!([]));
    assert!(summary["meshes"].as_array().unwrap().len() > 10);
    assert_eq!(summary["variant_sets"], json!([]));

    // 差分の組 (同じ部品の別の絵) だけを渡す。分類は変わらず、組は一覧にも出る。
    let sets = schema_of("set_layer_labels")["properties"]["variant_sets"].clone();
    assert_eq!(
        sets["items"],
        ugoe_ai::classify::output_schema()["properties"]["variant_sets"]["items"]
    );
    let err = client.call_err("set_layer_labels", json!({ "path": path }));
    assert!(err.contains("variant_sets"), "{err}");
    let err = client.call_err(
        "set_layer_labels",
        json!({ "path": path, "variant_sets": [[1, 2], [2, 3]] }),
    );
    assert!(err.contains("more than one variant set"), "{err}");
    let summary = client.call_ok(
        "set_layer_labels",
        json!({ "path": path, "variant_sets": [[3, 1], [5]] }),
    );
    assert_eq!(summary["updated"], 0);
    assert_eq!(summary["set_by_agent"], total);
    assert_eq!(summary["variant_sets"], json!([[1, 3]]));
    let page = client.call_ok(
        "get_psd_layers",
        json!({ "path": path, "count": 4, "images": false, "overview": false }),
    );
    let variants: Vec<&Value> = page["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| &l["variant"])
        .collect();
    assert_eq!(variants, [&json!(null), &json!(1), &json!(null), &json!(1)]);
    // 空の組で消える。
    let summary = client.call_ok(
        "set_layer_labels",
        json!({ "path": path, "variant_sets": [] }),
    );
    assert_eq!(summary["variant_sets"], json!([]));

    // 値の説明・位置・原画。
    let result = client.call("get_rig_guide", json!({ "path": path }));
    assert_eq!(result["isError"], false, "{result}");
    let guide = &result["structuredContent"];
    assert!(
        guide["instructions"]
            .as_str()
            .unwrap()
            .contains("turn_angle_scale")
    );
    assert!(guide["layout"].as_str().unwrap().contains("- face:"));
    assert_eq!(guide["defaults"]["turn_angle_scale"], 0.7_f32 as f64);
    png_size(&result["content"][2]);

    // 範囲の外の値はエラーを読んで直す。形の違う値もエラー。
    let mut params = json!({
        "turn_angle_scale": 2.5,
        "eye_close_line": 0.72,
        "eye_close_bend": 0.05,
        "mouth_open_height": 0.4,
        "reason": "large anime eyes; keep the turn small",
    });
    let err = client.call_err("propose_rig", json!({ "path": path, "params": params }));
    assert!(err.contains("turn_angle_scale is 2.5"), "{err}");
    let err = client.call_err(
        "propose_rig",
        json!({ "path": path, "params": { "turn_angle_scale": 0.6 } }),
    );
    assert!(err.contains("does not match the schema"), "{err}");
    params["turn_angle_scale"] = json!(0.6);
    let proposal = client.call_ok("propose_rig", json!({ "path": path, "params": params }));
    let id = proposal["proposal"].as_str().unwrap().to_owned();
    assert!(proposal["title"].as_str().unwrap().ends_with("agent"));
    assert!(
        proposal["rationale"]
            .as_str()
            .unwrap()
            .contains("large anime eyes")
    );
    assert_eq!(proposal["params"]["turn_angle_scale"], 0.6_f32 as f64);
    assert_eq!(proposal["report"]["warnings"], json!([]));
    let commands = proposal["commands"].as_array().unwrap();
    assert!(commands.len() > 20, "{}", commands.len());
    let added = proposal["changes"]["parameters"]["added"]
        .as_array()
        .unwrap();
    for p in [
        "AngleX",
        "AngleY",
        "AngleZ",
        "EyeLOpen",
        "EyeROpen",
        "MouthOpenY",
    ] {
        assert!(added.contains(&json!(p)), "{p} not in {added:?}");
    }
    // 提案だけではモデルは変わらない。
    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert!(parameter_ids(&summary).is_empty());
    let listed = client.call_ok("list_proposals", json!({ "path": path }));
    assert_eq!(listed["proposals"][0]["proposal"], id.as_str());
    assert_eq!(listed["proposals"][0]["stale"], false);

    // 外すコマンドの番号の誤りはエラー。承認されたら適用する。
    let err = client.call_err(
        "apply_proposal",
        json!({ "path": path, "proposal": id, "exclude": [commands.len()] }),
    );
    assert!(err.contains("does not exist"), "{err}");
    let applied = client.call_ok("apply_proposal", json!({ "path": path, "proposal": id }));
    assert_eq!(applied["changes"], proposal["changes"]);
    let summary = client.call_ok("get_model_summary", json!({ "path": path }));
    assert!(parameter_ids(&summary).contains(&"EyeLOpen".to_owned()));

    let bundle = tmp.0.join("bustup.ugoe");
    let saved = client.call_ok(
        "save_project",
        json!({ "path": path, "bundle": bundle.to_str().unwrap() }),
    );
    assert_eq!(saved["on_disk"], true);
    assert!(fs::read(&bundle).unwrap().starts_with(b"UGOE"));
    let on_disk = Project::load(&project).unwrap();
    assert!(ugoe_model::validate::validate(&on_disk.model).is_empty());
    assert_eq!(on_disk.model.parameters.len(), 7);

    // 保存したプロジェクトを開き直すと PSD は覚えていないので、get_psd_layers に渡す。
    client.call_ok("close_project", json!({ "path": path }));
    let err = client.call_err("get_rig_guide", json!({ "path": path }));
    assert!(err.contains("no PSD"), "{err}");
    let err = client.call_err(
        "import_psd",
        json!({ "path": path, "psd": psd.to_str().unwrap() }),
    );
    assert!(err.contains("already a project"), "{err}");
    let page = client.call_ok(
        "get_psd_layers",
        json!({ "path": path, "psd": psd.to_str().unwrap(), "images": false, "count": 1 }),
    );
    assert_eq!(page["layers"][0]["set_by"], "rules");
}

/// `--root` で、ツールが触れるパスを 1 つのディレクトリの下に限る。相対パスは root から解決し、
/// `..`・root の外の絶対パス・外を指すシンボリックリンクは拒否する (読む・作る・書き出す・描く)。
#[test]
fn root_confines_tool_paths() {
    let tmp = TempDir::new("root");
    let root = tmp.0.join("root");
    let outside = tmp.0.join("outside");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    save_fixture_project(&root);
    let other = save_fixture_project(&outside);
    fs::write(outside.join("secret.psd"), b"8BPS").unwrap();
    fs::write(outside.join("idle.motion.json"), b"{}").unwrap();
    let root_arg = root.to_str().unwrap();
    let mut client = Client::spawn(&["--root", root_arg]);
    client.initialize();

    // 相対パスは root から解決する (サーバの作業ディレクトリではない)。
    let opened = client.call_ok("open_project", json!({ "path": "quad.ugoeproj" }));
    let canonical_root = root.canonicalize().unwrap();
    assert_eq!(opened["path"], json!(canonical_root.join("quad.ugoeproj")));
    // root の下を指す絶対パスも使える。
    let absolute = canonical_root.join("quad.ugoeproj");
    client.call_ok(
        "get_model_summary",
        json!({ "path": absolute.to_str().unwrap() }),
    );

    let outside_abs = other.to_str().unwrap().to_owned();
    for (tool, arguments) in [
        (
            "open_project",
            json!({ "path": "../outside/quad.ugoeproj" }),
        ),
        ("open_project", json!({ "path": outside_abs })),
        ("get_model_summary", json!({ "path": outside_abs })),
        (
            "open_project",
            json!({ "path": "../outside/new.ugoeproj", "create": true }),
        ),
        (
            "import_psd",
            json!({ "path": "new.ugoeproj", "psd": "../outside/secret.psd" }),
        ),
        (
            "get_psd_layers",
            json!({ "path": "quad.ugoeproj", "psd": outside.join("secret.psd").to_str().unwrap() }),
        ),
        (
            "save_project",
            json!({ "path": "quad.ugoeproj", "bundle": "../outside/quad.ugoe" }),
        ),
        (
            "save_project",
            json!({ "path": "quad.ugoeproj", "bundle": "sub/../../outside/quad.ugoe" }),
        ),
    ] {
        let err = client.call_err(tool, arguments.clone());
        assert!(
            err.contains("outside the root"),
            "{tool} {arguments}: {err}"
        );
    }
    assert!(!outside.join("new.ugoeproj").exists());
    assert!(!outside.join("quad.ugoe").exists());
    // 描くモーションや表情の JSON のパスも限る (GPU に触る前に断る)。
    let err = client.call_err(
        "render_png",
        json!({ "path": "quad.ugoeproj", "motion": "../outside/idle.motion.json" }),
    );
    assert!(err.contains("outside the root"), "{err}");
    // 名前でもファイルでもなければ、作業ディレクトリから読もうとせずに断る。
    let err = client.call_err(
        "render_png",
        json!({ "path": "quad.ugoeproj", "expression": "smile.expression.json" }),
    );
    assert!(err.contains("no expression named"), "{err}");
    assert!(err.contains("is not a file"), "{err}");

    // root の下なら書き出せる。
    let saved = client.call_ok(
        "save_project",
        json!({ "path": "quad.ugoeproj", "bundle": "quad.ugoe" }),
    );
    assert_eq!(saved["bundle"], json!(canonical_root.join("quad.ugoe")));
    assert!(root.join("quad.ugoe").is_file());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        // root の外を指すリンク (プロジェクト・PSD・書き出し先)。
        symlink(&other, root.join("link.ugoeproj")).unwrap();
        symlink(outside.join("secret.psd"), root.join("secret.psd")).unwrap();
        symlink(outside.join("planted.ugoe"), root.join("planted.ugoe")).unwrap();
        let err = client.call_err("open_project", json!({ "path": "link.ugoeproj" }));
        assert!(err.contains("outside the root"), "{err}");
        let err = client.call_err(
            "import_psd",
            json!({ "path": "new.ugoeproj", "psd": "secret.psd" }),
        );
        assert!(err.contains("outside the root"), "{err}");
        let err = client.call_err(
            "save_project",
            json!({ "path": "quad.ugoeproj", "bundle": "planted.ugoe" }),
        );
        assert!(err.contains("symbolic link"), "{err}");
        assert!(!outside.join("planted.ugoe").exists());

        // プロジェクトのディレクトリは root の中でも、中のファイルが外を指すリンク。
        let leaky = root.join("leaky.ugoeproj");
        fs::create_dir_all(&leaky).unwrap();
        for name in ["project.json", "model.json", "textures"] {
            symlink(other.join(name), leaky.join(name)).unwrap();
        }
        let err = client.call_err("open_project", json!({ "path": "leaky.ugoeproj" }));
        assert!(err.contains("leads outside the root"), "{err}");
    }
}

/// `render_png` が `ugoe` (ugoe-cli) の実行ファイルなしで描けること。GPU (または lavapipe などの
/// ソフトウェアラスタライザ) が要る。`cargo test -p ugoe-mcp --features gpu-tests` で実行する。
#[cfg(feature = "gpu-tests")]
#[test]
fn renders_png_without_the_cli() {
    use base64::Engine;

    let tmp = TempDir::new("render-gpu");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap();
    // PATH を空にして、子プロセスの `ugoe` を探せないようにする。
    let mut client = Client::spawn_with(
        Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
            .env("PATH", "")
            .env_remove("UGOE_BIN"),
    );
    client.initialize();
    let mut render = |x: f32| {
        let result = client.call(
            "render_png",
            json!({ "path": path, "params": { "X": x }, "width": 64, "background": "black" }),
        );
        assert_eq!(result["isError"], false, "{result}");
        let log = result["structuredContent"]["log"]
            .as_str()
            .unwrap()
            .to_owned();
        let image = &result["content"][1];
        assert_eq!(image["mimeType"], "image/png", "{result}");
        let png = base64::engine::general_purpose::STANDARD
            .decode(image["data"].as_str().unwrap())
            .unwrap();
        (
            ugoe_headless::image::decode_png_premultiplied(&png).unwrap(),
            log,
        )
    };
    // 2 回目は同じ GPU を使い回す。
    let (left, log) = render(0.0);
    let (right, _) = render(1.0);
    println!("render_png: {log}");
    assert!(log.contains("rendered 64x64"), "{log}");
    assert_eq!((left.width, left.height), (64, 64));
    let white = |image: &ugoe_headless::Rgba8| {
        image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 200 && p[3] == 255)
            .count()
    };
    assert!(white(&left) > 0 && white(&left) < 64 * 64);
    assert_ne!(left.pixels, right.pixels, "X should move the quad");

    // 終了時に GPU を破棄しないので、ドライバが終了時に落ちない (WSL2 の dzn)。
    let status = client.shutdown();
    assert!(status.success(), "{status}");
}
