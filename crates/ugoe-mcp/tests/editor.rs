//! 偽のエディタ (テストの中で文書の持ち主を実装し、`EditorBridge` で HTTP で待ち受ける) に、
//! `ugoe-mcp --editor` (子プロセスの中継) でつなぎ、stdio のクライアントからツールを一通り動かす
//! (ADR 0023。実際のエディタ (T0642) の持ち主と同じ形: 文書と提案を錠の中に持ち、適用・保存は
//! 利用者の確認を待ち、ほかのプロジェクトは開かない)。

#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Document, Project, ProjectMeta, Proposals, Textures, export_bundle};
use ugoe_mcp::discovery::{self, Discovery};
use ugoe_mcp::host::{Action, Approvals, Change, HostError, Lent, Opening};
use ugoe_mcp::{BridgeOptions, DocumentHost, EditorBridge};
use ugoe_model::fixtures;

/// テスト用の一時ディレクトリ。破棄時に消す。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("ugoe-mcp-editor-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
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

/// 利用者が確認にどう答えるか。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Policy {
    Approve,
    Decline,
    /// 答えない (時間切れ)。
    Ignore,
    /// テストが [`FakeEditor::asked`] で受けて、自分で答える。
    Manual,
}

/// 偽のエディタ。エディタの `AppState` と同じく、文書とプロジェクトの情報を別々の錠に持つ
/// (錠の順序は document → session)。
struct FakeEditor {
    path: PathBuf,
    document: Mutex<Document>,
    session: Mutex<Session>,
    approvals: Arc<Approvals>,
    policy: Mutex<Policy>,
    timeout: Duration,
    /// 確認を出した (画面に出したダイアログ): 確認の番号と操作の説明。
    asked: Mutex<Sender<(u64, String)>>,
    /// 知らされた出来事。
    events: Mutex<Vec<String>>,
}

struct Session {
    meta: ProjectMeta,
    proposals: Proposals,
}

impl FakeEditor {
    fn open(path: &Path) -> (Arc<Self>, Receiver<(u64, String)>) {
        let project = Project::load(path).unwrap();
        let (sender, receiver) = mpsc::channel();
        let editor = Arc::new(Self {
            path: path.to_path_buf(),
            document: Mutex::new(project.document()),
            session: Mutex::new(Session {
                meta: project.meta,
                proposals: Proposals::new(),
            }),
            approvals: Arc::new(Approvals::new()),
            policy: Mutex::new(Policy::Approve),
            timeout: Duration::from_millis(300),
            asked: Mutex::new(sender),
            events: Mutex::new(Vec::new()),
        });
        (editor, receiver)
    }

    fn set_policy(&self, policy: Policy) {
        *self.policy.lock().unwrap() = policy;
    }

    fn parameter_ids(&self) -> Vec<String> {
        let document = self.document.lock().unwrap();
        document
            .model()
            .parameters
            .iter()
            .map(|p| p.id.as_str().to_owned())
            .collect()
    }

    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn check_project(&self, project: &Path) -> Result<(), HostError> {
        if project == self.path {
            Ok(())
        } else {
            Err(HostError::Denied(format!(
                "the editor has `{}` open; agents connected to the editor cannot open other projects",
                self.path.display()
            )))
        }
    }

    /// 画面に確認を出し、答えを待つ。
    fn confirm(&self, what: String) -> Result<(), HostError> {
        let policy = *self.policy.lock().unwrap();
        let approvals = Arc::clone(&self.approvals);
        let asked = self.asked.lock().unwrap().clone();
        self.approvals
            .request(self.timeout, "the user declined in the editor", move |id| {
                match policy {
                    Policy::Approve | Policy::Decline => {
                        // 画面のスレッドから答える。
                        std::thread::spawn(move || {
                            approvals.answer(id, policy == Policy::Approve);
                        });
                    }
                    Policy::Ignore => {}
                    Policy::Manual => asked.send((id, what)).unwrap(),
                }
            })
    }
}

impl DocumentHost for FakeEditor {
    fn with_project(&self, project: &Path, f: &mut dyn FnMut(Lent<'_>)) -> Result<(), HostError> {
        self.check_project(project)?;
        let mut document = self.document.lock().unwrap();
        let mut session = self.session.lock().unwrap();
        let session = &mut *session;
        f(Lent {
            meta: &session.meta,
            document: &mut document,
            proposals: &mut session.proposals,
        });
        Ok(())
    }

    fn open(&self, project: &Path, opening: Opening, _: bool) -> Result<(), HostError> {
        self.check_project(project)?;
        match opening {
            // 利用者の文書をディスクの内容で置き換えない。
            Opening::Load => Ok(()),
            Opening::New { .. } => Err(HostError::Denied("cannot create projects".to_owned())),
        }
    }

    fn close(&self, _: &Path, _: bool) -> Result<(), HostError> {
        Err(HostError::Denied(
            "agents cannot close the project open in the editor".to_owned(),
        ))
    }

    fn save(&self, project: &Path, bundle: Option<&Path>) -> Result<(), HostError> {
        self.check_project(project)?;
        let mut document = self.document.lock().unwrap();
        let session = self.session.lock().unwrap();
        let data = Project::from_document(session.meta.clone(), &document);
        data.save(project).map_err(|e| e.to_string())?;
        document.mark_saved();
        if let Some(bundle) = bundle {
            fs::write(bundle, export_bundle(&data).unwrap()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn authorize(&self, action: &Action<'_>) -> Result<(), HostError> {
        match *action {
            Action::Open {
                project,
                create: false,
                ..
            } => self.check_project(project),
            Action::Open { .. } | Action::Import { .. } | Action::Close { .. } => Err(
                HostError::Denied("not allowed from agents connected to the editor".to_owned()),
            ),
            Action::Apply {
                proposal, title, ..
            } => self.confirm(format!("apply p{proposal} {title}")),
            Action::Save { .. } => self.confirm("save".to_owned()),
            Action::Undo { .. } => self.confirm("undo".to_owned()),
            Action::Redo { .. } => self.confirm("redo".to_owned()),
        }
    }

    fn changed(&self, change: &Change<'_>) {
        let event = match change {
            Change::ClientConnected { name, .. } => format!("client {name}"),
            Change::Proposed {
                proposal, client, ..
            } => format!("proposed p{proposal} by {}", client.unwrap_or("?")),
            Change::Edited { .. } => "edited".to_owned(),
            Change::Saved { .. } => "saved".to_owned(),
            Change::Opened { .. } => "opened".to_owned(),
            Change::Closed { .. } => "closed".to_owned(),
        };
        self.events.lock().unwrap().push(event);
    }

    fn instructions(&self) -> Option<String> {
        Some(format!(
            "This server is the running ugoe editor. The open project is {}.",
            self.path.display()
        ))
    }
}

/// `ugoe-mcp --editor` を子プロセスで起動した stdio のクライアント。
struct Relay {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Relay {
    fn spawn(discovery: &Path, cwd: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
            .args(["--editor", "--discovery"])
            .arg(discovery)
            .current_dir(cwd)
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

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let response: Value =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad line {line:?}: {e}"));
        assert_eq!(response["id"], id, "{response}");
        response
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test-agent", "version": "1" },
            }),
        );
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        response
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )["result"]
            .clone()
    }

    fn call_ok(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], false, "{name}: {result}");
        result["structuredContent"].clone()
    }

    fn call_err(&mut self, name: &str, arguments: Value) -> String {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], true, "{name}: {result}");
        result["content"][0]["text"].as_str().unwrap().to_owned()
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start(editor: &Arc<FakeEditor>, discovery: &Path) -> EditorBridge {
    let host: Arc<dyn DocumentHost> = editor.clone();
    EditorBridge::start(
        host,
        BridgeOptions {
            discovery: Some(discovery.to_path_buf()),
            ..BridgeOptions::default()
        },
    )
    .unwrap()
}

fn add_angle(path: &str) -> Value {
    json!({
        "path": path,
        "title": "Add AngleX",
        "rationale": "face turns",
        "commands": [
            {
                "op": "add_parameter",
                "parameter": { "id": "AngleX", "name": "Angle X", "min": -30.0, "default": 0.0, "max": 30.0 },
            },
        ],
    })
}

#[test]
fn relays_the_tools_to_a_running_editor() {
    let tmp = TempDir::new("flow");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.0.join("state").join("editor-mcp.json");
    let (editor, _asked) = FakeEditor::open(&project);
    // 中継はエディタより先に起動してよい (要求のたびに発見用のファイルを探す)。
    let mut relay = Relay::spawn(&discovery_file, &tmp.0);

    let missing = relay.request("initialize", json!({ "protocolVersion": "2025-11-25" }));
    let message = missing["error"]["message"].as_str().unwrap();
    assert!(message.contains("not running"), "{message}");
    let err = relay.call_err("get_model_summary", json!({ "path": path }));
    assert!(err.contains("not running"), "{err}");

    let bridge = start(&editor, &discovery_file);
    let written = discovery::read(&discovery_file).unwrap();
    assert_eq!(written.token, bridge.token());
    assert_eq!(written.url(), bridge.url());
    assert_eq!(written.pid, std::process::id());

    let init = relay.initialize();
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(
        instructions.contains("running ugoe editor"),
        "{instructions}"
    );
    let list = relay.request("tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for name in [
        "open_project",
        "propose_edit",
        "apply_proposal",
        "save_project",
    ] {
        assert!(names.contains(&name), "{name} in {names:?}");
    }

    // 開いているプロジェクトは開ける (中身は置き換えない)。ほかは開けない・閉じられない。
    let opened = relay.call_ok("open_project", json!({ "path": path }));
    assert_eq!(opened["name"], "quad");
    // 相対パスは中継の作業ディレクトリ (エージェントの作業ディレクトリ) から解決する。
    relay.call_ok("open_project", json!({ "path": "quad.ugoeproj" }));
    fs::create_dir_all(tmp.0.join("other")).unwrap();
    let other = save_fixture_project(&tmp.0.join("other"));
    let err = relay.call_err("open_project", json!({ "path": other }));
    assert!(err.contains("cannot open other projects"), "{err}");
    let err = relay.call_err("get_model_summary", json!({ "path": other }));
    assert!(err.contains("cannot open other projects"), "{err}");
    let err = relay.call_err("close_project", json!({ "path": path }));
    assert!(err.contains("not allowed"), "{err}");
    let err = relay.call_err(
        "open_project",
        json!({ "path": tmp.0.join("new.ugoeproj"), "create": true }),
    );
    assert!(err.contains("not allowed"), "{err}");

    let summary = relay.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(summary["parameters"][0]["id"], "X");

    // 提案はエディタの提案の一覧に置かれ、クライアントの名前が知らされる。
    let proposal = relay.call_ok("propose_edit", add_angle(&path));
    let id = proposal["proposal"].as_str().unwrap().to_owned();
    assert!(!editor.session.lock().unwrap().proposals.is_empty());
    assert!(
        editor
            .events()
            .contains(&format!("proposed {id} by test-agent")),
        "{:?}",
        editor.events()
    );
    let listed = relay.call_ok("list_proposals", json!({ "path": path }));
    assert_eq!(listed["proposals"][0]["proposal"], id);

    // 利用者が拒否・時間切れなら適用しない。
    editor.set_policy(Policy::Decline);
    let err = relay.call_err("apply_proposal", json!({ "path": path, "proposal": id }));
    assert!(err.contains("declined"), "{err}");
    editor.set_policy(Policy::Ignore);
    let err = relay.call_err("apply_proposal", json!({ "path": path, "proposal": id }));
    assert!(err.contains("did not answer"), "{err}");
    assert_eq!(editor.parameter_ids(), ["X"]);
    // 適用できない提案では確認を出さない。
    let err = relay.call_err("apply_proposal", json!({ "path": path, "proposal": "p99" }));
    assert!(err.contains("not found"), "{err}");

    // 承認すれば、エディタの文書に適用する。
    editor.set_policy(Policy::Approve);
    let applied = relay.call_ok("apply_proposal", json!({ "path": path, "proposal": id }));
    assert_eq!(applied["changes"]["parameters"]["added"], json!(["AngleX"]));
    assert_eq!(applied["project"]["modified"], true);
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);
    let undone = relay.call_ok("undo", json!({ "path": path }));
    assert_eq!(undone["undone"], true);
    assert_eq!(editor.parameter_ids(), ["X"]);
    relay.call_ok("redo", json!({ "path": path }));
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);

    let bundle = tmp.0.join("quad.ugoe");
    let saved = relay.call_ok("save_project", json!({ "path": path, "bundle": bundle }));
    assert_eq!(saved["modified"], false);
    assert!(bundle.is_file());
    let on_disk = Project::load(&project).unwrap();
    assert_eq!(on_disk.model.parameters.len(), 2);

    let events = editor.events();
    for expected in ["client test-agent", "edited", "saved"] {
        assert!(
            events.iter().any(|e| e == expected),
            "{expected} in {events:?}"
        );
    }

    // エディタが止まると、発見用のファイルを消し、中継は分かりやすいエラーを返す。
    bridge.stop();
    assert!(!discovery_file.exists());
    let err = relay.call_err("get_model_summary", json!({ "path": path }));
    assert!(err.contains("not running"), "{err}");

    // 別のポートとトークンで起動し直しても、同じ中継でつながる。
    let bridge = start(&editor, &discovery_file);
    let summary = relay.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(summary["parameters"][1]["id"], "AngleX");

    // 発見用のファイルが残っていても、エディタが待ち受けていなければそう示す。
    let stale = discovery::read(&discovery_file).unwrap();
    drop(bridge);
    discovery::write(&discovery_file, &stale).unwrap();
    let err = relay.call_err("get_model_summary", json!({ "path": path }));
    assert!(err.contains("could not connect"), "{err}");
    assert!(err.contains(&format!("pid {}", stale.pid)), "{err}");

    // 知らない版の発見用のファイル。
    fs::write(
        &discovery_file,
        r#"{"format": 99, "version": "9.9.9", "pid": 1, "port": 1, "token": "x", "endpoint": "/mcp"}"#,
    )
    .unwrap();
    let err = relay.call_err("get_model_summary", json!({ "path": path }));
    assert!(err.contains("format 99"), "{err}");
}

#[test]
fn other_requests_proceed_while_an_approval_waits() {
    let tmp = TempDir::new("concurrent");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.0.join("editor-mcp.json");
    let (editor, asked) = FakeEditor::open(&project);
    let editor = {
        // 確認を長く待てるようにする。
        let mut editor = Arc::try_unwrap(editor).ok().unwrap();
        editor.timeout = Duration::from_secs(30);
        Arc::new(editor)
    };
    editor.set_policy(Policy::Manual);
    let _bridge = start(&editor, &discovery_file);

    let mut agent = Relay::spawn(&discovery_file, &tmp.0);
    agent.initialize();
    let proposal = agent.call_ok("propose_edit", add_angle(&path));
    let id = proposal["proposal"].as_str().unwrap().to_owned();

    // 適用の確認を待つ間に、ほかのクライアントの要求にも、エディタ自身の編集の錠にも答えられる。
    let waiting = std::thread::spawn({
        let path = path.clone();
        let id = id.clone();
        move || agent.call("apply_proposal", json!({ "path": path, "proposal": id }))
    });
    let (question, what) = asked.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(what, format!("apply {id} Add AngleX"));
    let mut other = Relay::spawn(&discovery_file, &tmp.0);
    other.initialize();
    let listed = other.call_ok("list_proposals", json!({ "path": path }));
    assert_eq!(listed["proposals"][0]["proposal"], id);
    assert_eq!(editor.parameter_ids(), ["X"]);
    assert_eq!(editor.approvals.pending(), vec![question]);

    assert!(editor.approvals.answer(question, true));
    let applied = waiting.join().unwrap();
    assert_eq!(applied["isError"], false, "{applied}");
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);
}

#[test]
fn a_changed_model_during_the_approval_rejects_the_stale_proposal() {
    let tmp = TempDir::new("stale");
    let project = save_fixture_project(&tmp.0);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.0.join("editor-mcp.json");
    let (editor, asked) = FakeEditor::open(&project);
    let editor = {
        let mut editor = Arc::try_unwrap(editor).ok().unwrap();
        editor.timeout = Duration::from_secs(30);
        Arc::new(editor)
    };
    editor.set_policy(Policy::Manual);
    let _bridge = start(&editor, &discovery_file);
    let mut agent = Relay::spawn(&discovery_file, &tmp.0);
    agent.initialize();
    let id = agent.call_ok("propose_edit", add_angle(&path))["proposal"]
        .as_str()
        .unwrap()
        .to_owned();
    let waiting = std::thread::spawn({
        let path = path.clone();
        move || agent.call("apply_proposal", json!({ "path": path, "proposal": id }))
    });
    let (question, _) = asked.recv_timeout(Duration::from_secs(10)).unwrap();
    // 利用者が確認の間にエディタで編集した。
    editor
        .document
        .lock()
        .unwrap()
        .apply(
            serde_json::from_value(json!({
                "op": "add_parameter",
                "parameter": { "id": "Other", "name": "Other", "min": 0.0, "default": 0.0, "max": 1.0 },
            }))
            .unwrap(),
        )
        .unwrap();
    editor.approvals.answer(question, true);
    let result = waiting.join().unwrap();
    assert_eq!(result["isError"], true, "{result}");
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("model changed"),
        "{result}"
    );
    assert_eq!(editor.parameter_ids(), ["X", "Other"]);
}

#[test]
fn the_bridge_requires_the_token_and_loopback() {
    let tmp = TempDir::new("token");
    let project = save_fixture_project(&tmp.0);
    let discovery_file = tmp.0.join("editor-mcp.json");
    let (editor, _asked) = FakeEditor::open(&project);
    let bridge = start(&editor, &discovery_file);
    assert!(bridge.local_addr().unwrap().ip().is_loopback());
    assert!(bridge.token().len() >= 32);

    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent();
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }).to_string();
    let without = agent
        .post(&bridge.url())
        .header("Content-Type", "application/json")
        .send(&body)
        .unwrap();
    assert_eq!(without.status().as_u16(), 401);
    let with = agent
        .post(&bridge.url())
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", bridge.token()))
        .send(&body)
        .unwrap();
    assert_eq!(with.status().as_u16(), 200);

    // ほかのエディタが発見用のファイルを書き換えていれば、止めても消さない。
    let theirs = Discovery::new(1, "o".repeat(64));
    discovery::write(&discovery_file, &theirs).unwrap();
    drop(bridge);
    assert_eq!(discovery::read(&discovery_file).unwrap(), theirs);
}
