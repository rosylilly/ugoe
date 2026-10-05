//! エディタの中の MCP サーバ (`crate::mcp`) のテスト。本物の `AppState` を持ち主にしてブリッジを始め、
//! `ugoe_mcp::relay::Relay` (`ugoe-mcp --editor` と同じ中継。ここではプロセスの中で動かす) からツールを呼ぶ。
//! 入力は `ugoe_model::fixtures::single_quad` を保存したプロジェクト。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Project, ProjectMeta, Textures};
use ugoe_mcp::relay::{Relay, RelayOptions};
use ugoe_model::fixtures;

use super::*;
use crate::project::ProjectSession;

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

/// 画面の代わり。確認にどう答えるかを決め、届いたイベントを覚える。
struct TestApp {
    state: AppState,
    /// 確認への答え (`None` なら答えない = 時間切れ)。
    answer: Mutex<Option<bool>>,
    events: Mutex<Vec<BridgeEvent>>,
}

impl EditorApp for TestApp {
    fn state(&self) -> &AppState {
        &self.state
    }

    fn emit(&self, event: BridgeEvent) {
        if let BridgeEvent::Approval { request } = &event
            && let Some(approved) = *self.answer.lock().unwrap()
        {
            // 確認の番号は出す前に登録されているので、ここで答えてよい (答えは待たない)。
            assert!(self.state.mcp.answer(request.id, approved));
        }
        self.events.lock().unwrap().push(event);
    }
}

impl TestApp {
    fn open(project: Option<&Path>, discovery: &Path) -> Arc<Self> {
        Self::open_with_timeout(project, discovery, Duration::from_millis(300))
    }

    fn open_with_timeout(project: Option<&Path>, discovery: &Path, timeout: Duration) -> Arc<Self> {
        let (session, document) = match project {
            Some(dir) => {
                let (project, disk) = Project::load_tracked(dir).unwrap();
                ProjectSession::from_project(project, Some((dir.to_path_buf(), disk)))
            }
            None => crate::project::initial_state(),
        };
        let mut state = crate::ai::tests::state(None);
        state.document = Mutex::new(document);
        state.project = Mutex::new(session);
        state.mcp = McpBridge {
            discovery: Some(discovery.to_path_buf()),
            shared: Arc::new(Shared {
                timeout,
                ..Shared::default()
            }),
            ..McpBridge::default()
        };
        Arc::new(Self {
            state,
            answer: Mutex::new(Some(true)),
            events: Mutex::new(Vec::new()),
        })
    }

    fn enable(self: &Arc<Self>) {
        let app: Arc<dyn EditorApp> = self.clone();
        self.state.mcp.set_enabled(app, true).unwrap();
        let status = self.status();
        assert!(status.running, "{status:?}");
    }

    fn status(&self) -> McpBridgeStatus {
        self.state.mcp.status(&self.state).unwrap()
    }

    fn set_answer(&self, answer: Option<bool>) {
        *self.answer.lock().unwrap() = answer;
    }

    fn take_events(&self) -> Vec<BridgeEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }

    fn parameter_ids(&self) -> Vec<String> {
        let document = self.state.document.lock().unwrap();
        document
            .model()
            .parameters
            .iter()
            .map(|p| p.id.as_str().to_owned())
            .collect()
    }
}

/// 中継 (プロセスの中) でつなぐクライアント。
struct Client {
    relay: Relay,
    next_id: u64,
}

impl Client {
    fn new(discovery: &Path) -> Self {
        Self {
            relay: Relay::new(RelayOptions {
                discovery: Some(discovery.to_path_buf()),
                wsl_distro: None,
            }),
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let out = self.relay.handle_line(&line.to_string()).unwrap();
        serde_json::from_str(&out).unwrap()
    }

    fn initialize(&mut self) -> Value {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test-agent", "version": "1.2" },
            }),
        )
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )["result"]
            .clone()
    }

    fn ok(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], false, "{name}: {result}");
        result["structuredContent"].clone()
    }

    fn err(&mut self, name: &str, arguments: Value) -> String {
        self.coded(name, arguments).0
    }

    /// 失敗するはずのツールを呼び、文と `structuredContent` (コードと引数。`message` は文と同じ) を返す。
    fn coded(&mut self, name: &str, arguments: Value) -> (String, Value) {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], true, "{name}: {result}");
        let text = result["content"][0]["text"].as_str().unwrap().to_owned();
        let structured = result["structuredContent"].clone();
        assert_eq!(structured["message"], text, "{name}: {result}");
        (text, structured)
    }
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

/// 提案 → エディタの一覧にエージェントの名前付きで出る → 確認 (拒否・時間切れ・承認) → 適用 → エディタの Undo で
/// 提案が再び出る → 保存 (外の変更と取り違えない)。
#[test]
fn agents_propose_and_the_user_approves_in_the_editor() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    let discovery = tmp.path.join("state").join("editor-mcp.json");
    let app = TestApp::open(Some(&project), &discovery);
    let mut client = Client::new(&discovery);

    // 既定は無効: 発見用のファイルは無く、中継は「動いていない」と答える。
    assert!(!app.status().enabled && !app.status().running);
    let err = client.err("get_model_summary", json!({ "path": path }));
    assert!(err.to_string().contains("Connect agents"), "{err}");

    app.enable();
    let status = app.status();
    assert_eq!(status.project.as_deref(), Some(path.as_str()));
    assert_eq!(
        status.discovery.as_deref(),
        Some(discovery.display().to_string().as_str())
    );
    let init = client.initialize();
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(
        instructions.contains(&format!("`{path}`")),
        "{instructions}"
    );
    let clients = app.status().clients;
    assert_eq!(clients.len(), 1);
    assert_eq!(
        (clients[0].name.as_str(), clients[0].version.as_deref()),
        ("test-agent", Some("1.2"))
    );
    app.take_events();

    // 提案はエディタの提案の一覧に、エージェントの名前付きで出る。文書は変わらない。
    let proposed = client.ok("propose_edit", add_angle(&path));
    assert_eq!(proposed["proposal"], "p1");
    {
        let document = app.state.document.lock().unwrap();
        let mut session = app.state.project.lock().unwrap();
        let views = session.lend().1.views(&document);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].title, "Add AngleX");
        assert_eq!(views[0].source.as_deref(), Some("test-agent"));
    }
    assert_eq!(app.take_events(), vec![BridgeEvent::Proposals]);
    assert_eq!(app.parameter_ids(), vec!["X"]);

    // 拒否: エラーが返り、文書は変わらない。確認は画面に出て、閉じた知らせも届く。
    app.set_answer(Some(false));
    let err = client.err("apply_proposal", json!({ "path": path, "proposal": "p1" }));
    assert!(err.to_string().contains("declined"), "{err}");
    let events = app.take_events();
    let BridgeEvent::Approval { request } = &events[0] else {
        panic!("{events:?}")
    };
    assert_eq!(request.action, ApprovalAction::Apply);
    assert_eq!(request.proposal, Some(1));
    assert_eq!(request.title.as_deref(), Some("Add AngleX"));
    assert_eq!(request.client.as_deref(), Some("test-agent"));
    assert_eq!(
        events[1],
        BridgeEvent::ApprovalClosed {
            id: request.id,
            outcome: ApprovalOutcome::Declined,
        }
    );
    assert_eq!(app.parameter_ids(), vec!["X"]);

    // 時間切れ: エラーが返り、待っている確認は残らない。
    app.set_answer(None);
    let err = client.err("apply_proposal", json!({ "path": path, "proposal": "p1" }));
    assert!(err.to_string().contains("did not answer in time"), "{err}");
    assert!(app.status().pending.is_empty());
    assert!(matches!(
        app.take_events().last(),
        Some(BridgeEvent::ApprovalClosed {
            outcome: ApprovalOutcome::TimedOut,
            ..
        })
    ));
    assert_eq!(app.parameter_ids(), vec!["X"]);

    // 承認: 適用され、画面に読み直しを知らせる。
    app.set_answer(Some(true));
    app.take_events();
    client.ok("apply_proposal", json!({ "path": path, "proposal": "p1" }));
    assert_eq!(app.parameter_ids(), vec!["X", "AngleX"]);
    let events = app.take_events();
    assert!(events.contains(&BridgeEvent::Document));
    assert!(events.iter().any(|e| matches!(
        e,
        BridgeEvent::ApprovalClosed {
            outcome: ApprovalOutcome::Approved,
            ..
        }
    )));

    // エディタの Undo (画面の操作) で戻すと、提案が再び一覧に出る。
    {
        let mut document = app.state.document.lock().unwrap();
        assert!(document.undo());
        let mut session = app.state.project.lock().unwrap();
        let views = session.lend().1.views(&document);
        assert_eq!(views.len(), 1);
        assert!(!views[0].stale);
    }
    // エージェントの Redo も確認を通す。
    let redone = client.ok("redo", json!({ "path": path }));
    assert_eq!(redone["redone"], true);
    assert_eq!(app.parameter_ids(), vec!["X", "AngleX"]);

    // 保存と書き出し: エディタの保存と同じく、自分の保存は外の変更と取り違えない。
    let bundle = tmp.path.join("quad.ugoe");
    let saved = client.ok(
        "save_project",
        json!({ "path": path, "bundle": bundle.display().to_string() }),
    );
    assert_eq!(saved["modified"], false);
    assert!(bundle.is_file());
    assert_eq!(Project::load(&project).unwrap().model.parameters.len(), 2);
    {
        let document = app.state.document.lock().unwrap();
        let session = app.state.project.lock().unwrap();
        assert!(!document.is_modified());
        assert_eq!(session.external_change().unwrap(), None);
    }

    // 外で書き換えられた後は、エージェントの保存は断る (利用者がエディタで選ぶ)。
    let meta = project.join("project.json");
    let text = std::fs::read_to_string(&meta).unwrap();
    std::fs::write(&meta, text.replace("quad", "renamed")).unwrap();
    let err = client.err("save_project", json!({ "path": path }));
    assert!(
        err.to_string().contains("changed outside the editor"),
        "{err}"
    );

    // 止めると発見用のファイルが消え、中継は「動いていない」と答える。
    let as_app: Arc<dyn EditorApp> = app.clone();
    app.state.mcp.set_enabled(as_app, false).unwrap();
    assert!(!discovery.exists());
    let err = client.err("get_model_summary", json!({ "path": path }));
    assert!(err.to_string().contains("not running"), "{err}");
}

/// エージェントが要求を取り消すと (`notifications/cancelled`)、確認は閉じ、取り消しとして画面に知らせる (T1603)。
#[test]
fn cancelled_requests_close_the_approval_as_cancelled() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    let discovery = tmp.path.join("state").join("editor-mcp.json");
    // 取り消しが時間切れより先に届くように、時間は長くする。
    let app = TestApp::open_with_timeout(Some(&project), &discovery, Duration::from_secs(30));
    app.enable();
    let mut client = Client::new(&discovery);
    client.initialize();
    client.ok("propose_edit", add_angle(&path));
    app.set_answer(None);
    app.take_events();

    let relay = &client.relay;
    let call = json!({
        "jsonrpc": "2.0",
        "id": "apply",
        "method": "tools/call",
        "params": { "name": "apply_proposal", "arguments": { "path": path, "proposal": "p1" } },
    });
    let reply = std::thread::scope(|scope| {
        let waiting = scope.spawn(|| relay.handle_line(&call.to_string()));
        // 確認が画面に出てから取り消す (時間切れより前に)。
        while !app
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, BridgeEvent::Approval { .. }))
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        let cancel = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": "apply" },
        });
        assert_eq!(relay.handle_line(&cancel.to_string()), None);
        waiting.join().unwrap()
    });
    // 取り消した要求には答えない。
    assert_eq!(reply, None);
    let events = app.take_events();
    let BridgeEvent::Approval { request } = &events[0] else {
        panic!("{events:?}")
    };
    assert_eq!(
        events[1],
        BridgeEvent::ApprovalClosed {
            id: request.id,
            outcome: ApprovalOutcome::Cancelled,
        }
    );
    assert!(app.status().pending.is_empty());
    assert_eq!(app.parameter_ids(), vec!["X"]);
}

/// 確認の終わり方は `HostError` の種類で見分ける。拒否の文が何であっても取り消しにはならない (T0557)。
#[test]
fn approval_outcomes_follow_the_error_kind() {
    assert_eq!(ApprovalOutcome::of(&Ok(())), ApprovalOutcome::Approved);
    for denied in [DECLINED, "the client cancelled the request", "stopped"] {
        assert_eq!(
            ApprovalOutcome::of(&Err(HostError::Denied(denied.to_owned()))),
            ApprovalOutcome::Declined,
            "{denied}"
        );
    }
    assert_eq!(
        ApprovalOutcome::of(&Err(failed(
            EditorError::Io {
                path: "a".to_owned(),
                detail: "io".to_owned(),
            }
            .into()
        ))),
        ApprovalOutcome::Declined
    );
    assert_eq!(
        ApprovalOutcome::of(&Err(HostError::TimedOut)),
        ApprovalOutcome::TimedOut
    );
    assert_eq!(
        ApprovalOutcome::of(&Err(HostError::Cancelled)),
        ApprovalOutcome::Cancelled
    );
}

/// 保存と一緒の `.ugoe` の書き出しは一時ファイルを経る: 書けなければ前のファイルを残し、一時ファイルも残さない (T0687)。
#[test]
fn saving_with_a_bundle_keeps_the_previous_bundle_when_it_fails() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(Some(&project), &discovery);
    let mut client = Client::new(&discovery);
    app.enable();
    client.initialize();
    let names = |dir: &std::path::Path| {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };

    // 書き出した後に書き直しても、行き先には一時ファイルを残さない。
    let out = tmp.path.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let bundle = out.join("quad.ugoe");
    for _ in 0..2 {
        client.ok(
            "save_project",
            json!({ "path": path, "bundle": bundle.display().to_string() }),
        );
    }
    assert_eq!(names(&out), ["quad.ugoe"]);

    // 一時ファイルを作れなければ、何も保存しない。
    let missing = out.join("missing").join("quad.ugoe");
    let (err, detail) = client.coded(
        "save_project",
        json!({ "path": path, "bundle": missing.display().to_string() }),
    );
    assert!(err.contains("nothing was saved"), "{err}");
    assert_eq!(detail["code"], "mcp.export_write_failed");
    assert_eq!(detail["args"]["path"], missing.display().to_string());

    // 置き換えられなければ (行き先が中身のあるディレクトリ)、前のものを残し、一時ファイルを消す。
    let blocked = out.join("blocked.ugoe");
    std::fs::create_dir_all(blocked.join("inside")).unwrap();
    let (err, detail) = client.coded(
        "save_project",
        json!({ "path": path, "bundle": blocked.display().to_string() }),
    );
    assert!(err.contains("previous file is left"), "{err}");
    assert_eq!(detail["code"], "mcp.export_replace_failed");
    assert!(blocked.join("inside").is_dir());
    assert_eq!(names(&out), ["blocked.ugoe", "quad.ugoe"]);
}

/// エディタがプロジェクトを保存できなければ、エディタのエラーのコード (`project.*`) と引数がツールの結果の
/// `structuredContent` に載る (`HostError::Failed` がコードを持つ。T0563)。
#[test]
fn a_failed_save_reports_the_editor_error_code() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(Some(&project), &discovery);
    app.enable();
    let mut client = Client::new(&discovery);
    client.initialize();
    client.ok("propose_edit", add_angle(&path));
    client.ok("apply_proposal", json!({ "path": path, "proposal": "p1" }));

    // 外の変更の検査と写しの後、書く前に止め、テクスチャのディレクトリをファイルに置き換える: 保存は
    // テクスチャを書けずに失敗する。
    let (reached_tx, reached) = std::sync::mpsc::channel();
    let (resume, resume_rx) = std::sync::mpsc::channel::<()>();
    *app.state.mcp.shared.pause_saving.lock().unwrap() = Some((reached_tx, resume_rx));
    let textures = project.join("textures");
    let (err, detail) = std::thread::scope(|scope| {
        let saving = scope.spawn(|| client.coded("save_project", json!({ "path": path })));
        reached.recv_timeout(Duration::from_secs(10)).unwrap();
        std::fs::remove_dir_all(&textures).unwrap();
        std::fs::write(&textures, b"not a directory").unwrap();
        drop(resume);
        saving.join().unwrap()
    });
    assert_eq!(detail["code"], "project.io", "{err}");
    assert!(
        detail["args"]["path"]
            .as_str()
            .unwrap()
            .starts_with(&textures.display().to_string()),
        "{detail}"
    );
    assert!(detail["args"]["detail"].is_string(), "{detail}");
}

/// 保存はドキュメントのロックの外で書く: 書いている間もほかのクライアントの読むだけのツールが答え、
/// 書いている間の編集は未保存のまま残る (T0025)。
#[test]
fn saving_writes_the_files_outside_the_document_lock() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(Some(&project), &discovery);
    app.enable();
    let mut client = Client::new(&discovery);
    client.initialize();
    client.ok("propose_edit", add_angle(&path));
    client.ok("apply_proposal", json!({ "path": path, "proposal": "p1" }));

    let saved = std::thread::scope(|scope| {
        // 写しを取った後で止める。止めている間に失敗しても、`resume` を捨てれば保存は続く。
        let (reached_tx, reached) = std::sync::mpsc::channel();
        let (resume, resume_rx) = std::sync::mpsc::channel();
        *app.state.mcp.shared.pause_saving.lock().unwrap() = Some((reached_tx, resume_rx));
        let saving = scope.spawn(|| client.ok("save_project", json!({ "path": path })));
        reached.recv_timeout(Duration::from_secs(10)).unwrap();

        let (answered_tx, answered) = std::sync::mpsc::channel();
        let (discovery, path) = (&discovery, &path);
        scope.spawn(move || {
            let mut other = Client::new(discovery);
            other.initialize();
            let summary = other.ok("get_model_summary", json!({ "path": path }));
            answered_tx.send(summary).unwrap();
        });
        let summary = answered
            .recv_timeout(Duration::from_secs(10))
            .expect("a read-only tool waited for the save to finish writing");
        assert!(!summary.is_null());
        // 書いている間の画面の編集。
        app.state
            .document
            .lock()
            .unwrap()
            .apply(ugoe_editor_core::Command::AddParameter {
                parameter: ugoe_model::Parameter {
                    id: "AngleY".into(),
                    ..fixtures::single_quad().parameters[0].clone()
                },
            })
            .unwrap();

        resume.send(()).unwrap();
        saving.join().unwrap()
    });
    // 書いたのは写しを取ったときの姿で、その後の編集は未保存のまま。
    assert_eq!(saved["modified"], true);
    assert_eq!(Project::load(&project).unwrap().model.parameters.len(), 2);
    assert_eq!(app.parameter_ids(), vec!["X", "AngleX", "AngleY"]);
    let mut document = app.state.document.lock().unwrap();
    let session = app.state.project.lock().unwrap();
    assert!(document.is_modified());
    assert_eq!(session.external_change().unwrap(), None);
    // 保存した状態に Undo で戻れば、変更なしになる。
    assert!(document.undo());
    assert!(!document.is_modified());
}

/// ほかのプロジェクト・作る・閉じる・PSD から作るはできない。開いているプロジェクトの open_project は状態を返すだけ。
#[test]
fn agents_cannot_switch_the_editor_project() {
    let tmp = TempDir::new();
    let project = save_fixture_project(&tmp.path);
    let path = project.display().to_string();
    std::fs::create_dir_all(tmp.path.join("other")).unwrap();
    let other = save_fixture_project(&tmp.path.join("other"));
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(Some(&project), &discovery);
    app.enable();
    let mut client = Client::new(&discovery);
    client.initialize();

    let opened = client.ok("open_project", json!({ "path": path }));
    assert_eq!(opened["name"], "quad");
    for (tool, args, expected) in [
        (
            "open_project",
            json!({ "path": other }),
            "can only use that project",
        ),
        (
            "get_model_summary",
            json!({ "path": other }),
            "can only use that project",
        ),
        (
            "open_project",
            json!({ "path": tmp.path.join("new.ugoeproj"), "create": true }),
            "cannot create projects",
        ),
        ("close_project", json!({ "path": path }), "cannot close"),
        (
            "import_psd",
            json!({ "path": tmp.path.join("psd.ugoeproj"), "psd": tmp.path.join("a.psd") }),
            "New from PSD",
        ),
    ] {
        let err = client.err(tool, args);
        assert!(err.to_string().contains(expected), "{tool}: {err}");
    }
    assert!(app.status().pending.is_empty());
}

/// 未保存のプロジェクトはパスで指せないので、保存を促す。
#[test]
fn unsaved_projects_ask_the_user_to_save() {
    let tmp = TempDir::new();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(None, &discovery);
    app.enable();
    assert_eq!(app.status().project, None);
    let mut client = Client::new(&discovery);
    let init = client.initialize();
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("not been saved"), "{instructions}");
    let err = client.err(
        "get_model_summary",
        json!({ "path": tmp.path.join("x.ugoeproj") }),
    );
    assert!(err.to_string().contains("Save As"), "{err}");
}

/// 読めない設定は、起動しただけでは書き換えない (始めもしない)。利用者が設定を変えたときに初めて書く (T0011)。
#[test]
fn an_unreadable_settings_file_is_not_overwritten_at_startup() {
    let tmp = TempDir::new();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(None, &discovery);
    let config = tmp.path.join("config");
    std::fs::create_dir_all(&config).unwrap();
    let file = config.join(SETTINGS_FILE);
    std::fs::write(&file, "{").unwrap();
    assert!(app.state.mcp.load(config).is_err());
    let as_app: Arc<dyn EditorApp> = app.clone();
    app.state.mcp.start_if_enabled(as_app);
    assert!(!app.status().running);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{");
}

/// 設定はファイルに残り (トークンを含むので利用者だけが読める)、トークンを作り直すと始め直す。
#[test]
fn settings_are_kept_and_the_token_can_be_regenerated() {
    let tmp = TempDir::new();
    let discovery = tmp.path.join("editor-mcp.json");
    let app = TestApp::open(None, &discovery);
    app.state.mcp.load(tmp.path.join("config")).unwrap();
    app.enable();
    let file = tmp.path.join("config").join(SETTINGS_FILE);
    let stored: StoredSettings =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(stored.enabled);
    let token = stored.token.clone().unwrap();
    assert_eq!(app.state.mcp.token().unwrap(), token);
    assert_eq!(ugoe_mcp::discovery::read(&discovery).unwrap().token, token);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let as_app: Arc<dyn EditorApp> = app.clone();
    app.state.mcp.regenerate_token(as_app.clone()).unwrap();
    let renewed = app.state.mcp.token().unwrap();
    assert_ne!(renewed, token);
    assert_eq!(
        ugoe_mcp::discovery::read(&discovery).unwrap().token,
        renewed
    );

    // 読み直すと同じ設定 (有効・トークン)。
    let reloaded = McpBridge::default();
    reloaded.load(tmp.path.join("config")).unwrap();
    assert!(lock(&reloaded.settings).enabled);
    assert_eq!(reloaded.token().unwrap(), renewed);

    assert!(app.state.mcp.set_port(as_app.clone(), Some(0)).is_err());
    app.state.mcp.set_enabled(as_app, false).unwrap();
    assert!(!discovery.exists());
}

/// 開発中は同じ target/ のビルド結果も探す (インストールしたものは隣だけ)。
#[test]
fn sidecar_is_looked_up_next_to_the_editor_and_in_the_target_dir() {
    let debug = Path::new("/w/target/debug");
    assert_eq!(
        sidecar_candidates(debug, "ugoe-mcp"),
        vec![
            debug.join("ugoe-mcp"),
            PathBuf::from("/w/target/release/ugoe-mcp")
        ]
    );
    let installed = Path::new("/usr/bin");
    assert_eq!(
        sidecar_candidates(installed, "ugoe-mcp"),
        vec![installed.join("ugoe-mcp")]
    );
}

#[test]
fn the_claude_desktop_bundle_wraps_the_sidecar_with_the_editor_icon() {
    let tmp = TempDir::new();
    let binary = tmp.path.join("ugoe-mcp");
    std::fs::write(&binary, b"relay").unwrap();
    let out = tmp.path.join("nested").join("ugoe-editor.mcpb");
    write_claude_desktop_bundle(Some(binary), &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"));
    let manifest = Bundle::new("ugoe-mcp", Platform::current().unwrap()).manifest();
    assert_eq!(manifest["name"], "ugoe-editor");
    // 束の中身は ugoe-mcp のテスト (crates/ugoe-mcp/tests/editor.rs) で展開して確かめる。ここはアイコンを入れたことだけ。
    let icon_name = b"icon.png";
    assert!(bytes.windows(icon_name.len()).any(|w| w == icon_name));
    assert!(BUNDLE_ICON.starts_with(b"\x89PNG"));

    // 保存先は絶対パスだけ (相対パスは作業ディレクトリに書いてしまう)。
    let err =
        tauri::async_runtime::block_on(save_claude_desktop_bundle("x.mcpb".into())).unwrap_err();
    assert_eq!(err.code, "editor.relative_path");

    let err = write_claude_desktop_bundle(None, &tmp.path.join("x.mcpb")).unwrap_err();
    assert!(
        err.to_string()
            .contains("cargo build --release -p ugoe-mcp"),
        "{err}"
    );
}
