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
use ugoe_editor_core::{
    Document, Message, Project, ProjectMeta, Proposals, Textures, export_bundle,
};
use ugoe_mcp::discovery::{self, DISCOVERY_ENV, Discovery};
use ugoe_mcp::host::{Action, Approvals, Change, HostError, Lent, Opening, export_replace_failed};
use ugoe_mcp::{BridgeOptions, DocumentHost, EditorBridge};
use ugoe_model::fixtures;

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
        data.save(project).map_err(Message::from)?;
        document.mark_saved();
        if let Some(bundle) = bundle {
            fs::write(bundle, export_bundle(&data).unwrap())
                .map_err(|e| export_replace_failed(bundle, &e))?;
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
    /// `None` なら閉じた ([`Relay::close_stdin`])。
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Relay {
    fn spawn(discovery: &Path, cwd: &Path) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"));
        command
            .args(["--editor", "--discovery"])
            .arg(discovery)
            .current_dir(cwd);
        Self::spawn_command(command)
    }

    /// `command` (中継を起動するコマンド) を stdio のサーバとして起動する。
    fn spawn_command(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            next_id: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// 標準入力を閉じる (クライアントが終わった)。
    fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// 要求を送り、答えを待たずに id を返す。
    fn send_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// 標準出力の次の 1 行。
    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad line {line:?}: {e}"))
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send_request(method, params);
        let response = self.read();
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
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.path.join("state").join("editor-mcp.json");
    let (editor, _asked) = FakeEditor::open(&project);
    // 中継はエディタより先に起動してよい (要求のたびに発見用のファイルを探す)。
    let mut relay = Relay::spawn(&discovery_file, &tmp.path);

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
    fs::create_dir_all(tmp.path.join("other")).unwrap();
    let other = save_fixture_project(&tmp.path.join("other"));
    let err = relay.call_err("open_project", json!({ "path": other }));
    assert!(err.contains("cannot open other projects"), "{err}");
    let err = relay.call_err("get_model_summary", json!({ "path": other }));
    assert!(err.contains("cannot open other projects"), "{err}");
    let err = relay.call_err("close_project", json!({ "path": path }));
    assert!(err.contains("not allowed"), "{err}");
    let err = relay.call_err(
        "open_project",
        json!({ "path": tmp.path.join("new.ugoeproj"), "create": true }),
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
    let declined = relay.call("apply_proposal", json!({ "path": path, "proposal": id }));
    assert_eq!(declined["isError"], true, "{declined}");
    let err = declined["content"][0]["text"].as_str().unwrap();
    assert!(err.contains("declined"), "{err}");
    assert_eq!(declined["structuredContent"]["code"], "host.denied");
    editor.set_policy(Policy::Ignore);
    let ignored = relay.call("apply_proposal", json!({ "path": path, "proposal": id }));
    let err = ignored["content"][0]["text"].as_str().unwrap();
    assert!(err.contains("did not answer"), "{err}");
    assert_eq!(ignored["structuredContent"]["code"], "host.timed_out");
    assert_eq!(editor.parameter_ids(), ["X"]);
    // 適用できない提案では確認を出さない。
    let err = relay.call_err("apply_proposal", json!({ "path": path, "proposal": "p99" }));
    assert!(err.contains("not found"), "{err}");

    // 承認すれば、エディタの文書に適用する。
    editor.set_policy(Policy::Approve);
    let applied = relay.call_ok("apply_proposal", json!({ "path": path, "proposal": id }));
    assert_eq!(applied["changes"]["parameters"]["added"], json!(["AngleX"]));
    assert_eq!(applied["project"]["modified"], true);
    // 戻せるか・進められるかはエディタの文書の履歴から。
    assert_eq!(applied["project"]["can_undo"], true);
    assert_eq!(applied["project"]["can_redo"], false);
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);
    let undone = relay.call_ok("undo", json!({ "path": path }));
    assert_eq!(undone["undone"], true);
    assert_eq!(undone["can_undo"], false);
    assert_eq!(undone["can_redo"], true);
    assert_eq!(editor.parameter_ids(), ["X"]);
    let redone = relay.call_ok("redo", json!({ "path": path }));
    assert_eq!(redone["can_undo"], true);
    assert_eq!(redone["can_redo"], false);
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);

    let bundle = tmp.path.join("quad.ugoe");
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
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.path.join("editor-mcp.json");
    let (editor, asked) = FakeEditor::open(&project);
    let editor = {
        // 確認を長く待てるようにする。
        let mut editor = Arc::try_unwrap(editor).ok().unwrap();
        editor.timeout = Duration::from_secs(30);
        Arc::new(editor)
    };
    editor.set_policy(Policy::Manual);
    let _bridge = start(&editor, &discovery_file);

    let mut agent = Relay::spawn(&discovery_file, &tmp.path);
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
    let mut other = Relay::spawn(&discovery_file, &tmp.path);
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

/// 確認を長く待つ偽のエディタにつないだ中継と、置いた提案 ([`waiting_editor`])。
struct Waiting {
    editor: Arc<FakeEditor>,
    asked: Receiver<(u64, String)>,
    bridge: EditorBridge,
    agent: Relay,
    path: String,
    proposal: String,
}

/// 確認を長く待つ偽のエディタ (画面の答えはテストが出す) を、中継につないで提案を 1 つ置く。
fn waiting_editor(tmp: &TempDir) -> Waiting {
    let project = save_fixture_project(&tmp.path);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.path.join("editor-mcp.json");
    let (editor, asked) = FakeEditor::open(&project);
    let editor = {
        let mut editor = Arc::try_unwrap(editor).ok().unwrap();
        editor.timeout = Duration::from_secs(30);
        Arc::new(editor)
    };
    editor.set_policy(Policy::Manual);
    let bridge = start(&editor, &discovery_file);
    let mut agent = Relay::spawn(&discovery_file, &tmp.path);
    agent.initialize();
    let proposal = agent.call_ok("propose_edit", add_angle(&path))["proposal"]
        .as_str()
        .unwrap()
        .to_owned();
    Waiting {
        editor,
        asked,
        bridge,
        agent,
        path,
        proposal,
    }
}

/// `done` が真になるまで待つ (10 秒で諦める)。
fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(std::time::Instant::now() < deadline, "timed out: {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_relay_answers_other_requests_while_an_approval_waits() {
    let tmp = TempDir::new();
    let Waiting {
        editor,
        asked,
        bridge: _bridge,
        mut agent,
        path,
        proposal,
    } = waiting_editor(&tmp);
    let apply = agent.send_request(
        "tools/call",
        json!({ "name": "apply_proposal", "arguments": { "path": path, "proposal": proposal } }),
    );
    let (question, _) = asked.recv_timeout(Duration::from_secs(10)).unwrap();

    // 同じ中継 (同じクライアント) のほかの要求が、確認を待つ適用より先に返る。
    let ping = agent.request("ping", json!({}));
    assert_eq!(ping["result"], json!({}), "{ping}");
    let listed = agent.call_ok("list_proposals", json!({ "path": path }));
    assert_eq!(listed["proposals"][0]["proposal"], proposal);
    // 並行の要求でも、標準出力の行は混ざらない (1 行ずつ JSON として読める)。
    let ids: Vec<u64> = (0..8)
        .map(|_| {
            agent.send_request(
                "tools/call",
                json!({ "name": "list_proposals", "arguments": { "path": path } }),
            )
        })
        .collect();
    let mut answered: Vec<u64> = (0..8)
        .map(|_| agent.read()["id"].as_u64().unwrap())
        .collect();
    answered.sort_unstable();
    assert_eq!(answered, ids);
    assert_eq!(editor.parameter_ids(), ["X"]);

    assert!(editor.approvals.answer(question, true));
    let applied = agent.read();
    assert_eq!(applied["id"], apply, "{applied}");
    assert_eq!(applied["result"]["isError"], false, "{applied}");
    assert_eq!(editor.parameter_ids(), ["X", "AngleX"]);
}

#[test]
fn a_cancelled_apply_is_declined_in_the_editor() {
    let tmp = TempDir::new();
    let Waiting {
        editor,
        asked,
        bridge: _bridge,
        mut agent,
        path,
        proposal,
    } = waiting_editor(&tmp);
    let apply = agent.send_request(
        "tools/call",
        json!({ "name": "apply_proposal", "arguments": { "path": path, "proposal": proposal } }),
    );
    let (question, _) = asked.recv_timeout(Duration::from_secs(10)).unwrap();
    agent.send(&json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": apply, "reason": "timed out" },
    }));
    // エディタの確認が閉じ、後から承認しても適用されない。
    wait_until("the approval closes", || {
        editor.approvals.pending().is_empty()
    });
    assert!(!editor.approvals.answer(question, true));
    assert_eq!(editor.parameter_ids(), ["X"]);
    // 取り消した要求には答えない (次の行は次の要求の答え)。
    let ping = agent.request("ping", json!({}));
    assert_eq!(ping["result"], json!({}), "{ping}");
    // 提案は残る (もう一度適用を頼める)。
    let listed = agent.call_ok("list_proposals", json!({ "path": path }));
    assert_eq!(listed["proposals"][0]["proposal"], proposal);
}

#[test]
fn closing_stdin_cancels_the_waiting_approvals() {
    let tmp = TempDir::new();
    let Waiting {
        editor,
        asked,
        bridge: _bridge,
        mut agent,
        path,
        proposal,
    } = waiting_editor(&tmp);
    agent.send_request(
        "tools/call",
        json!({ "name": "apply_proposal", "arguments": { "path": path, "proposal": proposal } }),
    );
    asked.recv_timeout(Duration::from_secs(10)).unwrap();
    agent.close_stdin();
    wait_until("the approval closes", || {
        editor.approvals.pending().is_empty()
    });
    wait_until("the relay exits", || {
        agent.child.try_wait().unwrap().is_some()
    });
    assert_eq!(editor.parameter_ids(), ["X"]);
}

#[test]
fn an_http_client_disconnecting_cancels_its_approval() {
    let tmp = TempDir::new();
    let Waiting {
        editor,
        asked,
        bridge,
        agent: _agent,
        path,
        proposal,
    } = waiting_editor(&tmp);
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "apply_proposal", "arguments": { "path": path, "proposal": proposal } },
    })
    .to_string();
    let addr = bridge.local_addr().unwrap();
    let mut stream = std::net::TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\
         Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
         Content-Length: {}\r\n\r\n{body}",
        addr.port(),
        bridge.token(),
        body.len()
    )
    .unwrap();
    let (question, _) = asked.recv_timeout(Duration::from_secs(10)).unwrap();
    // 答えを待たずに接続を閉じた (Streamable HTTP の取り消し)。
    drop(stream);
    wait_until("the approval closes", || {
        editor.approvals.pending().is_empty()
    });
    assert!(!editor.approvals.answer(question, true));
    assert_eq!(editor.parameter_ids(), ["X"]);
}

#[test]
fn a_changed_model_during_the_approval_rejects_the_stale_proposal() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.to_str().unwrap().to_owned();
    let discovery_file = tmp.path.join("editor-mcp.json");
    let (editor, asked) = FakeEditor::open(&project);
    let editor = {
        let mut editor = Arc::try_unwrap(editor).ok().unwrap();
        editor.timeout = Duration::from_secs(30);
        Arc::new(editor)
    };
    editor.set_policy(Policy::Manual);
    let _bridge = start(&editor, &discovery_file);
    let mut agent = Relay::spawn(&discovery_file, &tmp.path);
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
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let discovery_file = tmp.path.join("editor-mcp.json");
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

// ---- MCP Bundle (Claude Desktop) ----

/// zip の 1 つの項目 (名前・Unix の mode・汎用のフラグ・展開した中身)。
struct ZipEntry {
    name: String,
    mode: u32,
    flags: usize,
    data: Vec<u8>,
}

fn le16(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// `.mcpb` (zip) を中央ディレクトリから読む (テスト用の最小の読み手。deflate と無圧縮だけ)。
fn read_zip(bytes: &[u8]) -> Vec<ZipEntry> {
    let end = bytes.len() - 22;
    assert_eq!(le32(bytes, end), 0x0605_4b50, "end of central directory");
    let count = le16(bytes, end + 10);
    let mut at = le32(bytes, end + 16) as usize;
    let mut entries = Vec::new();
    for _ in 0..count {
        assert_eq!(le32(bytes, at), 0x0201_4b50, "central directory entry");
        assert_eq!(le16(bytes, at + 4) >> 8, 3, "made on Unix (keeps the mode)");
        let flags = le16(bytes, at + 8);
        let method = le16(bytes, at + 10);
        let compressed = le32(bytes, at + 20) as usize;
        let size = le32(bytes, at + 24) as usize;
        let name_len = le16(bytes, at + 28);
        let extra_len = le16(bytes, at + 30);
        let comment_len = le16(bytes, at + 32);
        let mode = le32(bytes, at + 38) >> 16;
        let local = le32(bytes, at + 42) as usize;
        let name = String::from_utf8(bytes[at + 46..at + 46 + name_len].to_vec()).unwrap();
        assert_eq!(le32(bytes, local), 0x0403_4b50, "local header of {name}");
        assert_eq!(
            le16(bytes, local + 6),
            flags,
            "local and central flags of {name}"
        );
        assert_eq!(
            le32(bytes, local + 14),
            le32(bytes, at + 16),
            "local CRC of {name}"
        );
        let start = local + 30 + le16(bytes, local + 26) + le16(bytes, local + 28);
        let raw = &bytes[start..start + compressed];
        let data = match method {
            0 => raw.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec(raw).unwrap(),
            other => panic!("unknown compression {other} for {name}"),
        };
        assert_eq!(data.len(), size, "{name}");
        entries.push(ZipEntry {
            name,
            mode,
            flags,
            data,
        });
        at += 46 + name_len + extra_len + comment_len;
    }
    entries
}

/// manifest を MCPB の manifest 0.3 の規則 (公式の schema と `mcpb validate` の検査) で確かめる。
/// <https://github.com/modelcontextprotocol/mcpb/blob/main/schemas/mcpb-manifest-v0.3.schema.json>
fn check_manifest(manifest: &Value, files: &[&str]) {
    const TOP: &[&str] = &[
        "$schema",
        "dxt_version",
        "manifest_version",
        "name",
        "display_name",
        "version",
        "description",
        "long_description",
        "author",
        "repository",
        "homepage",
        "documentation",
        "support",
        "icon",
        "icons",
        "screenshots",
        "server",
        "tools",
        "tools_generated",
        "prompts",
        "prompts_generated",
        "keywords",
        "license",
        "privacy_policies",
        "compatibility",
        "user_config",
        "_meta",
        "localization",
    ];
    let object = manifest.as_object().unwrap();
    for key in object.keys() {
        assert!(TOP.contains(&key.as_str()), "unknown top-level field {key}");
    }
    assert_eq!(manifest["manifest_version"], "0.3");
    for key in ["name", "version", "description"] {
        assert!(
            manifest[key].as_str().is_some_and(|s| !s.is_empty()),
            "{key}"
        );
    }
    assert!(manifest["author"]["name"].is_string());
    let version = manifest["version"].as_str().unwrap();
    assert_eq!(
        version
            .split('.')
            .filter(|p| p.parse::<u64>().is_ok())
            .count(),
        3,
        "{version}"
    );

    let server = &manifest["server"];
    assert_eq!(server["type"], "binary");
    let entry = server["entry_point"].as_str().unwrap();
    assert!(files.contains(&entry), "entry point {entry} in {files:?}");
    let config = server["mcp_config"].as_object().unwrap();
    for key in config.keys() {
        assert!(
            ["command", "args", "env", "platform_overrides"].contains(&key.as_str()),
            "{key}"
        );
    }
    assert_eq!(config["command"], format!("${{__dirname}}/{entry}"));
    // `mcpb validate` が受ける変数だけ
    let mut strings = vec![config["command"].as_str().unwrap()];
    strings.extend(
        config["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap()),
    );
    for s in strings {
        let mut rest = s;
        while let Some(start) = rest.find("${") {
            let end = rest[start..].find('}').unwrap() + start;
            let var = &rest[start + 2..end];
            assert!(
                ["__dirname", "pathSeparator", "/"].contains(&var)
                    || var.starts_with("user_config."),
                "variable {var} in {s}"
            );
            rest = &rest[end..];
        }
    }

    for tool in manifest["tools"].as_array().unwrap() {
        let keys: Vec<&String> = tool.as_object().unwrap().keys().collect();
        assert!(
            keys.iter().all(|k| *k == "name" || *k == "description"),
            "{tool}"
        );
        assert!(tool["name"].is_string());
    }
    let platforms = manifest["compatibility"]["platforms"].as_array().unwrap();
    for p in platforms {
        assert!(
            ["darwin", "win32", "linux"].contains(&p.as_str().unwrap()),
            "{p}"
        );
    }
    if let Some(icon) = manifest["icon"].as_str() {
        assert!(files.contains(&icon), "icon {icon}");
    }
}

/// Claude Desktop が `${__dirname}` に入れる形の、展開したディレクトリのパス (`dir` は絶対パス)。
///
/// Windows の `canonicalize` ([`TempDir`]) は `\\?\C:\…` (verbatim) のパスを返す。verbatim のパスは `/` を
/// 区切りとして受け付けない (Windows は `\\?\` の後を正規化しない) ので、manifest のコマンド
/// `${__dirname}/server/ugoe-mcp.exe` をそのままつなぐと起動できない (`InvalidFilename`)。Claude Desktop が
/// 渡す展開先は `C:\Users\…` の通常の形なので、`\\?\` を外してそれに合わせる。
fn desktop_dirname(dir: &str) -> String {
    if let Some(rest) = dir.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = dir.strip_prefix(r"\\?\") {
        rest.to_owned()
    } else {
        dir.to_owned()
    }
}

/// manifest の `mcp_config` の文字列の `${__dirname}` を `dirname` で置き換える。
fn expand_dirname(template: &str, dirname: &str) -> String {
    template.replace("${__dirname}", dirname)
}

#[test]
fn the_bundle_dirname_is_the_plain_form_of_the_directory() {
    // Windows の canonicalize の形 (verbatim) は通常の形にする。
    assert_eq!(
        desktop_dirname(r"\\?\C:\Users\a\AppData\Local\Temp\x\installed"),
        r"C:\Users\a\AppData\Local\Temp\x\installed"
    );
    assert_eq!(
        desktop_dirname(r"\\?\UNC\server\share\installed"),
        r"\\server\share\installed"
    );
    // 通常の形はそのまま。
    for dir in [r"C:\Users\a\installed", "/tmp/x/installed"] {
        assert_eq!(desktop_dirname(dir), dir);
    }
    // 置き換えたコマンドは verbatim でない絶対パスに `/` 区切りの残りが続く形 (Windows の通常のパスは
    // `/` も区切りとして受け付ける)。
    let command = expand_dirname(
        "${__dirname}/server/ugoe-mcp.exe",
        &desktop_dirname(r"\\?\C:\x\installed"),
    );
    assert_eq!(command, r"C:\x\installed/server/ugoe-mcp.exe");
    assert!(!command.starts_with(r"\\?\"), "{command}");
}

#[test]
fn the_mcp_bundle_relays_to_a_running_editor() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.to_str().unwrap().to_owned();
    let icon = tmp.path.join("icon.png");
    fs::write(&icon, solid_png(512, 512, [40, 120, 220, 255])).unwrap();

    // `ugoe-mcp --mcpb` がこの実行ファイルを包む。
    let bundle = tmp.path.join("ugoe-editor.mcpb");
    let status = Command::new(env!("CARGO_BIN_EXE_ugoe-mcp"))
        .arg("--mcpb")
        .arg(&bundle)
        .arg("--mcpb-icon")
        .arg(&icon)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let entries = read_zip(&fs::read(&bundle).unwrap());
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    let manifest: Value = serde_json::from_slice(&entries[0].data).unwrap();
    assert_eq!(names[0], "manifest.json");
    check_manifest(&manifest, &names);
    let platform = ugoe_mcp::mcpb::Platform::current().unwrap();
    assert_eq!(
        manifest["compatibility"]["platforms"],
        json!([platform.as_str()])
    );
    assert_eq!(
        manifest["server"]["mcp_config"]["args"],
        json!(["--editor"])
    );
    assert_eq!(manifest["icon"], "icon.png");
    let entry = manifest["server"]["entry_point"].as_str().unwrap();
    let binary = entries.iter().find(|e| e.name == entry).unwrap();
    assert_eq!(binary.mode & 0o777, 0o755, "the server must be executable");
    assert_eq!(binary.mode & 0o170_000, 0o100_000, "a regular file");
    for e in &entries {
        assert_eq!(e.flags & 0x0800, 0x0800, "UTF-8 name: {}", e.name);
        assert_eq!(e.flags & 0x0008, 0, "no data descriptor: {}", e.name);
    }
    // 中身が同じか (失敗したときに数百 MB のバイト列を出さないよう、長さだけを示す)
    let exe = fs::read(env!("CARGO_BIN_EXE_ugoe-mcp")).unwrap();
    assert!(
        binary.data == exe,
        "the packed server differs from ugoe-mcp ({} and {} bytes)",
        binary.data.len(),
        exe.len()
    );
    let packed_icon = entries.iter().find(|e| e.name == "icon.png").unwrap();
    assert_eq!(packed_icon.data, fs::read(&icon).unwrap());

    // Claude Desktop のように展開し、manifest のコマンドを起動する (`${__dirname}` は展開したディレクトリ)。
    let dir = tmp.path.join("installed");
    for e in &entries {
        let file = dir.join(&e.name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, &e.data).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(e.mode & 0o777)).unwrap();
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.join(entry)).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "the extracted server is executable ({mode:o})"
        );
    }
    let config = &manifest["server"]["mcp_config"];
    let dirname = desktop_dirname(dir.to_str().unwrap());
    let mut command = Command::new(expand_dirname(
        config["command"].as_str().unwrap(),
        &dirname,
    ));
    for arg in config["args"].as_array().unwrap() {
        command.arg(expand_dirname(arg.as_str().unwrap(), &dirname));
    }
    // 発見用のファイルは既定の場所の代わりにテストのディレクトリ (manifest には書かない)。
    let discovery_file = tmp.path.join("state").join("editor-mcp.json");
    command
        .env(DISCOVERY_ENV, &discovery_file)
        .current_dir(&tmp.path);

    let (editor, _asked) = FakeEditor::open(&project);
    let _bridge = start(&editor, &discovery_file);
    let mut relay = Relay::spawn_command(command);
    let init = relay.initialize();
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(
        instructions.contains("running ugoe editor"),
        "{instructions}"
    );

    // manifest の tools は、中継から見えるツールと同じ。
    let list = relay.request("tools/list", json!({}));
    let served: Vec<&Value> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["name"])
        .collect();
    let declared: Vec<&Value> = manifest["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["name"])
        .collect();
    assert_eq!(served, declared);

    let summary = relay.call_ok("get_model_summary", json!({ "path": path }));
    assert_eq!(summary["parameters"][0]["id"], "X");
    relay.call_ok("propose_edit", add_angle(&path));
    assert!(
        editor
            .events()
            .iter()
            .any(|e| e.starts_with("proposed") && e.ends_with("by test-agent")),
        "{:?}",
        editor.events()
    );
}
