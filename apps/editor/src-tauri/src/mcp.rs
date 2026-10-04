//! エディタの中の MCP サーバ (ブリッジ) と、その IPC コマンド。方針: ADR 0023・`docs/editor.md#エージェントをつなぐ`。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。
//!
//! - エディタの開いている文書と提案を持ち主 ([`EditorHost`]。`ugoe_mcp::DocumentHost`) として貸し、
//!   `ugoe_mcp::EditorBridge` でループバックの HTTP (トークン必須) で待ち受ける。既定は無効で、
//!   設定の「Connect agents」で有効にする (アプリの設定のディレクトリの `mcp-bridge.json` に残す)。
//! - エージェントの提案は、AI タブの提案の一覧に置く (`Proposals::set_source` でエージェントの名前を記す)。
//! - エージェントの適用・Undo・Redo・保存は、画面に確認のダイアログを出し ([`BridgeEvent::Approval`])、
//!   利用者が承認したときだけ行う (拒否・時間切れはツールのエラー)。画面から採用するときは確認しない。
//! - ほかのプロジェクトを開く・作る・閉じる・PSD から作るはブリッジからはできない。未保存のプロジェクトは
//!   パスで指せないので、保存を促す。
//! - Claude Desktop には、同梱の `ugoe-mcp` を `--editor` で動かす MCP Bundle (`.mcpb`) を、その場で作って渡す
//!   ([`open_claude_desktop_bundle`]・[`save_claude_desktop_bundle`]。`ugoe_mcp::mcpb`)。
//! - ブリッジの編集はエディタの文書に入るので、外での書き換え (`docs/editor.md#外での書き換え`) にはならない。
//!   保存はエディタの保存と同じ処理 ([`crate::project::ProjectSession::save_to`]) で、指紋も取り直す。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;
use ugoe_editor_core::{Coded, Document, Message, export_bundle_with_settings};
use ugoe_mcp::host::{Action, Approvals, Change, HostError, Lent, Opening};
use ugoe_mcp::http::generate_token;
use ugoe_mcp::mcpb::{Bundle, Platform};
use ugoe_mcp::wsl::WslPaths;
use ugoe_mcp::{BridgeOptions, DocumentHost, EditorBridge};

use crate::AppState;
use crate::error::{EditorError, IpcResult};
use crate::project::ProjectSession;

/// 設定のファイル (アプリの設定のディレクトリの下)。トークンを含むので Unix では 0600 で書く。
const SETTINGS_FILE: &str = "mcp-bridge.json";
/// 画面へ送るイベントの名前 (payload は [`BridgeEvent`])。
pub const EVENT: &str = "mcp-bridge";
/// 利用者の確認を待つ時間。過ぎるとエージェントにはツールのエラー (時間切れ) が返る。
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);
/// 覚えておくクライアントの数。
const MAX_CLIENTS: usize = 8;

/// 拒否したときにエージェントへ返す文。
const DECLINED: &str = "the user declined this in the ugoe editor; ask them what to change instead";

/// 残す設定 (`mcp-bridge.json`)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct StoredSettings {
    enabled: bool,
    /// 待ち受けるポート (`127.0.0.1`)。`None` なら起動のたびに空いているポート。
    port: Option<u16>,
    /// 要求に求めるトークン。無ければ作って残す (再起動しても HTTP で直接つなぐクライアントの設定が使えるように)。
    token: Option<String>,
}

/// エージェントが求めた、利用者の確認が要る操作の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    Apply,
    Undo,
    Redo,
    Save,
}

/// 確認のダイアログに出す、エージェントの求め。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ApprovalRequest {
    /// 答えるときの番号 (`answer_mcp_approval`)。
    #[ts(type = "number")]
    pub id: u64,
    pub action: ApprovalAction,
    /// 最後に握手したクライアントの名前 (複数のクライアントが同時につながると区別できない)。
    pub client: Option<String>,
    /// 適用する提案の番号と題名 (`apply`)。
    pub proposal: Option<u32>,
    pub title: Option<String>,
    /// 外すコマンドの番号 (`apply`)。
    pub excluded: Vec<usize>,
    /// 保存と一緒に書き出す `.ugoe` (`save`)。
    pub bundle: Option<String>,
    /// 答えを待つ秒数。
    pub timeout_secs: u64,
}

/// 握手したクライアント。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct BridgeClient {
    pub name: String,
    pub version: Option<String>,
    /// 最後に握手した時刻 (UNIX 時刻のミリ秒)。
    #[ts(type = "number")]
    pub connected_at: u64,
}

/// 同梱の `ugoe-mcp` (登録のコマンドに埋める)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Sidecar {
    /// エディタの実行ファイルの隣の `ugoe-mcp` (`.exe`)。開発中は同じ `target/` のビルド結果。
    /// 見つからなければ `null` (画面はビルドの方法を案内する)。
    pub path: Option<String>,
    /// Windows で、`path` を WSL から指すパス (`/mnt/c/...`)。
    pub wsl_path: Option<String>,
    /// `windows`・`macos`・`linux`。
    pub platform: String,
    /// エディタ自身が WSL の中で動いている (Linux 版を WSLg で動かしている)。
    pub in_wsl: bool,
}

/// 「Connect agents」の画面に出す状態。トークンは含めない (`get_mcp_bridge_token`)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct McpBridgeStatus {
    pub enabled: bool,
    /// 待ち受けている。
    pub running: bool,
    /// MCP の URL (`http://127.0.0.1:<port>/mcp`)。
    pub url: Option<String>,
    /// 決めたポート。`null` なら起動のたびに空いているポート。
    pub port: Option<u16>,
    /// 発見用のファイル (`ugoe-mcp --editor` が読む)。
    pub discovery: Option<String>,
    /// 始められなかった理由。
    pub error: Option<Message>,
    pub clients: Vec<BridgeClient>,
    /// 答えを待っている確認 (画面を開き直したときに出し直す)。
    pub pending: Vec<ApprovalRequest>,
    /// エージェントがツールに渡すプロジェクトのパス。未保存なら `null` (パスで指せない)。
    pub project: Option<String>,
    pub sidecar: Sidecar,
}

/// 画面へ送るイベント。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BridgeEvent {
    /// 状態が変わった (始めた・止めた・クライアントが握手した)。
    Status,
    /// 確認を求める。
    Approval { request: ApprovalRequest },
    /// 確認が終わった (答えた・時間切れ・止めた)。
    ApprovalClosed {
        #[ts(type = "number")]
        id: u64,
    },
    /// エージェントが文書を変えた・保存した (読み直す)。
    Document,
    /// エージェントが提案を置いた。
    Proposals,
}

/// 持ち主が使うエディタの側 (Tauri のアプリ。テストでは差し替える)。
pub(crate) trait EditorApp: Send + Sync + 'static {
    fn state(&self) -> &AppState;
    /// 画面へイベントを送る。待たない。
    fn emit(&self, event: BridgeEvent);
}

struct TauriApp(AppHandle);

impl EditorApp for TauriApp {
    fn state(&self) -> &AppState {
        self.0.state::<AppState>().inner()
    }

    fn emit(&self, event: BridgeEvent) {
        if let Err(e) = self.0.emit(EVENT, event) {
            eprintln!("could not notify the window about the MCP bridge: {e}");
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// 持ち主とブリッジの状態が共有するもの。
#[derive(Debug)]
struct Shared {
    approvals: Approvals,
    pending: Mutex<Vec<ApprovalRequest>>,
    clients: Mutex<Vec<BridgeClient>>,
    timeout: Duration,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            approvals: Approvals::new(),
            pending: Mutex::default(),
            clients: Mutex::default(),
            timeout: APPROVAL_TIMEOUT,
        }
    }
}

impl Shared {
    fn last_client(&self) -> Option<String> {
        lock(&self.clients)
            .iter()
            .max_by_key(|c| c.connected_at)
            .map(|c| c.name.clone())
    }
}

/// ブリッジ (AppState が持つ)。
#[derive(Debug, Default)]
pub struct McpBridge {
    settings_path: OnceLock<PathBuf>,
    settings: Mutex<StoredSettings>,
    running: Mutex<Option<EditorBridge>>,
    error: Mutex<Option<Message>>,
    shared: Arc<Shared>,
    /// 発見用のファイルの場所 (テストで変える。`None` なら `ugoe_mcp::discovery` の既定)。
    discovery: Option<PathBuf>,
}

impl McpBridge {
    /// 設定を `dir` (アプリの設定のディレクトリ) から読む。なければ既定 (無効) のまま。
    pub fn load(&self, dir: PathBuf) -> Result<(), EditorError> {
        let path = dir.join(SETTINGS_FILE);
        let _ = self.settings_path.set(path.clone());
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(EditorError::io(&path, e)),
        };
        let settings: StoredSettings =
            serde_json::from_str(&text).map_err(|e| EditorError::io(&path, e))?;
        *lock(&self.settings) = settings;
        Ok(())
    }

    fn save_settings(&self, settings: &StoredSettings) -> Result<(), EditorError> {
        let Some(path) = self.settings_path.get() else {
            return Ok(());
        };
        let mut text = serde_json::to_string_pretty(settings)
            .map_err(|e| EditorError::Internal(e.to_string()))?;
        text.push('\n');
        write_private(path, &text).map_err(|e| EditorError::io(path, e))
    }

    fn update(&self, f: impl FnOnce(&mut StoredSettings)) -> Result<StoredSettings, EditorError> {
        let mut settings = lock(&self.settings);
        let mut next = settings.clone();
        f(&mut next);
        self.save_settings(&next)?;
        *settings = next.clone();
        Ok(next)
    }

    /// 設定で有効なら始める (起動時)。
    pub(crate) fn start_if_enabled(&self, app: Arc<dyn EditorApp>) {
        if lock(&self.settings).enabled {
            self.start(app);
        }
    }

    /// 始める (動いていれば止めてから)。失敗は状態の `error` に出す。
    fn start(&self, app: Arc<dyn EditorApp>) {
        self.stop();
        let result = self.token().and_then(|token| {
            let host = Arc::new(EditorHost {
                app: Arc::clone(&app),
                shared: Arc::clone(&self.shared),
            });
            EditorBridge::start(
                host,
                BridgeOptions {
                    port: lock(&self.settings).port.unwrap_or(0),
                    token: Some(token),
                    discovery: self.discovery.clone(),
                    ..BridgeOptions::default()
                },
            )
            .map_err(|e| EditorError::BridgeStart(e.to_string()))
        });
        match result {
            Ok(bridge) => {
                *lock(&self.running) = Some(bridge);
                *lock(&self.error) = None;
            }
            Err(e) => *lock(&self.error) = Some(e.to_message()),
        }
        app.emit(BridgeEvent::Status);
    }

    /// 止める。待っている確認はすべて拒否し、発見用のファイルを消す。
    pub fn stop(&self) {
        self.shared.approvals.deny_all();
        lock(&self.shared.pending).clear();
        lock(&self.shared.clients).clear();
        if let Some(bridge) = lock(&self.running).take() {
            bridge.stop();
        }
    }

    /// 残したトークン。無ければ作って残す。
    pub(crate) fn token(&self) -> Result<String, EditorError> {
        if let Some(token) = lock(&self.settings).token.clone() {
            return Ok(token);
        }
        let token = generate_token().map_err(|e| EditorError::Internal(e.to_string()))?;
        self.update(|s| s.token = Some(token.clone()))?;
        Ok(token)
    }

    pub(crate) fn set_enabled(
        &self,
        app: Arc<dyn EditorApp>,
        enabled: bool,
    ) -> Result<(), EditorError> {
        self.update(|s| s.enabled = enabled)?;
        if enabled {
            self.start(app);
        } else {
            self.stop();
            *lock(&self.error) = None;
            app.emit(BridgeEvent::Status);
        }
        Ok(())
    }

    pub(crate) fn set_port(
        &self,
        app: Arc<dyn EditorApp>,
        port: Option<u16>,
    ) -> Result<(), EditorError> {
        if port == Some(0) {
            return Err(EditorError::InvalidPort);
        }
        let settings = self.update(|s| s.port = port)?;
        if settings.enabled {
            self.start(app);
        }
        Ok(())
    }

    /// トークンを作り直す (前のトークンを知るクライアントは使えなくなる)。動いていれば始め直す。
    pub(crate) fn regenerate_token(&self, app: Arc<dyn EditorApp>) -> Result<(), EditorError> {
        let token = generate_token().map_err(|e| EditorError::Internal(e.to_string()))?;
        let settings = self.update(|s| s.token = Some(token))?;
        if settings.enabled {
            self.start(app);
        }
        Ok(())
    }

    /// 確認に答える。待っている確認が無ければ (時間切れの後など) `false`。
    pub(crate) fn answer(&self, id: u64, approved: bool) -> bool {
        self.shared.approvals.answer(id, approved)
    }

    pub(crate) fn status(&self, state: &AppState) -> IpcResult<McpBridgeStatus> {
        let settings = lock(&self.settings).clone();
        let running = lock(&self.running);
        let project = crate::ai::lock_project(state)?
            .path()
            .map(|p| p.display().to_string());
        Ok(McpBridgeStatus {
            enabled: settings.enabled,
            running: running.is_some(),
            url: running.as_ref().map(EditorBridge::url),
            port: settings.port,
            discovery: running
                .as_ref()
                .map(|b| b.discovery_path().display().to_string()),
            error: lock(&self.error).clone(),
            clients: lock(&self.shared.clients).clone(),
            pending: lock(&self.shared.pending).clone(),
            project,
            sidecar: sidecar(),
        })
    }
}

/// トークンを含むファイルを、利用者だけが読める権限で書く。
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        // 前からあるファイルは mode が効かないので直す
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    options.open(path)?.write_all(text.as_bytes())
}

/// `ugoe-mcp` を探す場所 (`dir` はエディタの実行ファイルのディレクトリ)。インストールしたものは隣にある。
/// 開発中 (`just editor`。実行ファイルは `target/debug/`) は同梱しないので、同じ `target/` のビルド結果
/// (`cargo build -p ugoe-mcp` の `target/debug/`、`--release` の `target/release/`) も探す。
fn sidecar_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![dir.join(name)];
    if let (Some(profile), Some(target)) = (dir.file_name(), dir.parent())
        && (profile == "debug" || profile == "release")
    {
        for other in ["release", "debug"] {
            if profile != other {
                candidates.push(target.join(other).join(name));
            }
        }
    }
    candidates
}

/// 同梱の `ugoe-mcp` のパス (Tauri のサイドカー。エディタの実行ファイルと同じディレクトリに置かれる)。
fn sidecar_path() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ugoe-mcp.exe"
    } else {
        "ugoe-mcp"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| sidecar_candidates(dir, name)))
        .and_then(|candidates| candidates.into_iter().find(|p| p.is_file()))
}

/// 同梱の `ugoe-mcp` (登録のコマンドに埋める)。
fn sidecar() -> Sidecar {
    let path = sidecar_path();
    let wsl_path = if cfg!(windows) {
        path.as_ref()
            .and_then(|p| WslPaths::new("", "/mnt/").to_wsl(&p.display().to_string()))
    } else {
        None
    };
    let platform = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    Sidecar {
        path: path.map(|p| p.display().to_string()),
        wsl_path,
        platform: platform.into(),
        in_wsl: cfg!(target_os = "linux") && std::env::var_os("WSL_DISTRO_NAME").is_some(),
    }
}

// ---- Claude Desktop (MCP Bundle) ----

/// `.mcpb` に入れるアイコン (エディタのアイコン。512×512)。
const BUNDLE_ICON: &[u8] = include_bytes!("../icons/icon.png");
/// 「Install in Claude Desktop」で作る `.mcpb` の名前 (アプリのキャッシュのディレクトリの下)。
const BUNDLE_FILE: &str = "ugoe-editor.mcpb";

/// 同梱の `ugoe-mcp` を `--editor` で動かす `.mcpb` を `out` に書く。束の OS はエディタの OS。
/// 同梱の `ugoe-mcp` をその場で包むので、インストーラに `.mcpb` を別に入れない (同じ実行ファイルを 2 つ
/// 持たずに済み、エディタと版が必ずそろう)。
fn write_claude_desktop_bundle(binary: Option<PathBuf>, out: &Path) -> Result<(), EditorError> {
    let binary = binary.ok_or(EditorError::NoMcpBinary)?;
    let platform = Platform::current().ok_or(EditorError::UnsupportedPlatform)?;
    let mut bundle = Bundle::new(binary, platform);
    bundle.icon = Some(BUNDLE_ICON.to_vec());
    bundle
        .write(out)
        .map_err(|e| EditorError::BundleWrite(e.to_string()))
}

/// 束を作る (数秒かかるので、画面のスレッドの外で)。
async fn build_bundle(out: PathBuf) -> IpcResult<String> {
    let display = out.display().to_string();
    tauri::async_runtime::spawn_blocking(move || write_claude_desktop_bundle(sidecar_path(), &out))
        .await
        .map_err(|e| EditorError::Task(e.to_string()))??;
    Ok(display)
}

/// Claude Desktop 用の `.mcpb` を作り、OS に開かせる (Claude Desktop がインストールの画面を出す)。
/// 作ったファイルのパスを返す (開けなかったときに、Claude Desktop から選ばせるため)。
#[tauri::command]
pub async fn open_claude_desktop_bundle(app: AppHandle) -> IpcResult<String> {
    use tauri_plugin_opener::OpenerExt;
    let out = app
        .path()
        .app_cache_dir()
        .map_err(|e| EditorError::Internal(e.to_string()))?
        .join(BUNDLE_FILE);
    let path = build_bundle(out).await?;
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|e| EditorError::OpenFailed {
            path: path.clone(),
            detail: e.to_string(),
        })?;
    Ok(path)
}

/// Claude Desktop 用の `.mcpb` を `path` (保存のダイアログで選んだ絶対パス) に書く。
#[tauri::command]
pub async fn save_claude_desktop_bundle(path: String) -> IpcResult<String> {
    let out = crate::project::absolute_path(&path)?;
    if out.extension().is_none_or(|e| e != "mcpb") {
        return Err(EditorError::NotMcpbFile.into());
    }
    build_bundle(out).await
}

// ---- 持ち主 ----

/// エディタの文書を貸す持ち主。メソッドは HTTP の接続のスレッドで呼ばれる (`ugoe_mcp::host` の「スレッドと錠」)。
struct EditorHost {
    app: Arc<dyn EditorApp>,
    shared: Arc<Shared>,
}

fn same_path(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

/// エディタのエラーを MCP の持ち主のエラーにする (英語の文をそのまま使う)。
fn failed(error: Message) -> HostError {
    HostError::Failed(error.message)
}

/// 保存していないプロジェクトを指せないときの文。
const UNSAVED: &str = "the project open in the ugoe editor has not been saved yet, so tools cannot refer \
     to it by path; ask the user to save it in the editor (File > Save As…) and use that path";

impl EditorHost {
    fn lock_both(
        &self,
    ) -> Result<(MutexGuard<'_, Document>, MutexGuard<'_, ProjectSession>), HostError> {
        let state = self.app.state();
        let document = state
            .document
            .lock()
            .map_err(|_| HostError::Failed("document lock poisoned".into()))?;
        let session = state
            .project
            .lock()
            .map_err(|_| HostError::Failed("project lock poisoned".into()))?;
        Ok((document, session))
    }

    /// `project` がエディタで開いているプロジェクトか。
    fn check(session: &ProjectSession, project: &Path) -> Result<PathBuf, HostError> {
        let Some(open) = session.path() else {
            return Err(HostError::Denied(UNSAVED.into()));
        };
        if same_path(open, project) {
            Ok(open.to_path_buf())
        } else {
            Err(HostError::Denied(format!(
                "the ugoe editor has `{}` open; agents connected to the editor can only use that \
                 project (ask the user to open `{}` in the editor)",
                open.display(),
                project.display()
            )))
        }
    }

    fn check_current(&self, project: &Path) -> Result<(), HostError> {
        let session = crate::ai::lock_project(self.app.state()).map_err(failed)?;
        Self::check(&session, project).map(drop)
    }

    /// 画面に確認を出し、答えか時間切れまで待つ。
    fn confirm(&self, request: ApprovalRequest) -> Result<(), HostError> {
        let mut asked = None;
        let result = self
            .shared
            .approvals
            .request(self.shared.timeout, DECLINED, |id| {
                asked = Some(id);
                let request = ApprovalRequest { id, ..request };
                lock(&self.shared.pending).push(request.clone());
                self.app.emit(BridgeEvent::Approval { request });
            });
        if let Some(id) = asked {
            lock(&self.shared.pending).retain(|r| r.id != id);
            self.app.emit(BridgeEvent::ApprovalClosed { id });
        }
        result
    }

    fn request(&self, action: ApprovalAction) -> ApprovalRequest {
        ApprovalRequest {
            id: 0,
            action,
            client: self.shared.last_client(),
            proposal: None,
            title: None,
            excluded: Vec::new(),
            bundle: None,
            timeout_secs: self.shared.timeout.as_secs(),
        }
    }
}

impl DocumentHost for EditorHost {
    fn with_project(&self, project: &Path, f: &mut dyn FnMut(Lent<'_>)) -> Result<(), HostError> {
        let (mut document, mut session) = self.lock_both()?;
        Self::check(&session, project)?;
        let (meta, proposals) = session.lend();
        f(Lent {
            meta,
            document: &mut document,
            proposals,
        });
        Ok(())
    }

    fn open(&self, project: &Path, opening: Opening, _: bool) -> Result<(), HostError> {
        self.check_current(project)?;
        match opening {
            // 利用者の文書をディスクの内容で置き換えない。
            Opening::Load => Ok(()),
            Opening::New { .. } => Err(HostError::Denied(
                "agents connected to the ugoe editor cannot create projects".into(),
            )),
        }
    }

    fn close(&self, _: &Path, _: bool) -> Result<(), HostError> {
        Err(HostError::Denied(
            "agents connected to the ugoe editor cannot close the project; the user closes it in the editor"
                .into(),
        ))
    }

    fn save(&self, project: &Path, bundle: Option<&Path>) -> Result<(), HostError> {
        let (mut document, mut session) = self.lock_both()?;
        let dir = Self::check(&session, project)?;
        if session.external_change().map_err(failed)?.is_some() {
            return Err(HostError::Denied(
                "the project files were changed outside the editor after it was opened; ask the \
                 user to reload or save it from the editor"
                    .into(),
            ));
        }
        session.save_to(&mut document, dir).map_err(failed)?;
        if let Some(bundle) = bundle {
            let (bytes, _) = export_bundle_with_settings(
                &session.to_project(&document),
                &session.export_settings(),
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(bundle, bytes)
                .map_err(|e| format!("failed to write {}: {e}", bundle.display()))?;
        }
        Ok(())
    }

    fn authorize(&self, action: &Action<'_>) -> Result<(), HostError> {
        match *action {
            Action::Open {
                project,
                create: false,
                ..
            } => self.check_current(project),
            Action::Open { .. } => Err(HostError::Denied(
                "agents connected to the ugoe editor cannot create projects; the user opens \
                 projects in the editor"
                    .into(),
            )),
            Action::Import { .. } => Err(HostError::Denied(
                "agents connected to the ugoe editor cannot create projects from a PSD; ask the \
                 user to use File > New from PSD… in the editor, save it, and then use \
                 get_psd_layers with `psd`"
                    .into(),
            )),
            Action::Close { .. } => Err(HostError::Denied(
                "agents connected to the ugoe editor cannot close the project".into(),
            )),
            Action::Apply {
                project,
                proposal,
                title,
                excluded,
            } => {
                self.check_current(project)?;
                self.confirm(ApprovalRequest {
                    proposal: Some(proposal),
                    title: Some(title.to_owned()),
                    excluded: excluded.to_vec(),
                    ..self.request(ApprovalAction::Apply)
                })
            }
            Action::Undo { project } => {
                self.check_current(project)?;
                self.confirm(self.request(ApprovalAction::Undo))
            }
            Action::Redo { project } => {
                self.check_current(project)?;
                self.confirm(self.request(ApprovalAction::Redo))
            }
            Action::Save { project, bundle } => {
                self.check_current(project)?;
                self.confirm(ApprovalRequest {
                    bundle: bundle.map(|b| b.display().to_string()),
                    ..self.request(ApprovalAction::Save)
                })
            }
        }
    }

    fn changed(&self, change: &Change<'_>) {
        match *change {
            Change::ClientConnected { name, version } => {
                let mut clients = lock(&self.shared.clients);
                clients.retain(|c| c.name != name);
                clients.push(BridgeClient {
                    name: name.to_owned(),
                    version: version.map(str::to_owned),
                    connected_at: now_ms(),
                });
                let excess = clients.len().saturating_sub(MAX_CLIENTS);
                clients.drain(..excess);
                drop(clients);
                self.app.emit(BridgeEvent::Status);
            }
            Change::Proposed {
                proposal, client, ..
            } => {
                if let Ok((_document, mut session)) = self.lock_both() {
                    session
                        .lend()
                        .1
                        .set_source(proposal, client.unwrap_or("an agent"));
                }
                self.app.emit(BridgeEvent::Proposals);
            }
            Change::Edited { .. } | Change::Saved { .. } => self.app.emit(BridgeEvent::Document),
            Change::Opened { .. } | Change::Closed { .. } => {}
        }
    }

    fn instructions(&self) -> Option<String> {
        let session = crate::ai::lock_project(self.app.state()).ok()?;
        let project = match session.path() {
            Some(path) => format!(
                "The project open in the editor is `{}`; pass this path to every tool.",
                path.display()
            ),
            None => "The project open in the editor has not been saved yet, so tools cannot refer \
                     to it; ask the user to save it in the editor first."
                .to_owned(),
        };
        Some(format!(
            "You are connected to the running ugoe editor, not to files on disk. {project} \
             Proposals (propose_edit, propose_rig) appear in the editor's AI tab, where the user can \
             preview and adopt them. apply_proposal, undo, redo and save_project ask the user in the \
             editor first and fail if they decline or do not answer within {} seconds. You cannot \
             open other projects, create, import or close projects through the editor.",
            self.shared.timeout.as_secs()
        ))
    }
}

// ---- IPC ----

fn app(handle: AppHandle) -> Arc<dyn EditorApp> {
    Arc::new(TauriApp(handle))
}

/// 「Connect agents」の画面の状態。
#[tauri::command]
pub fn mcp_bridge_status(app: AppHandle) -> IpcResult<McpBridgeStatus> {
    let state = app.state::<AppState>();
    state.mcp.status(&state)
}

/// ブリッジを有効・無効にし (設定に残す)、始める・止める。
#[tauri::command]
pub fn set_mcp_bridge_enabled(app: AppHandle, enabled: bool) -> IpcResult<McpBridgeStatus> {
    let state = app.state::<AppState>();
    state.mcp.set_enabled(self::app(app.clone()), enabled)?;
    state.mcp.status(&state)
}

/// 待ち受けるポートを決める (`null` なら起動のたびに空いているポート)。動いていれば始め直す。
#[tauri::command]
pub fn set_mcp_bridge_port(app: AppHandle, port: Option<u16>) -> IpcResult<McpBridgeStatus> {
    let state = app.state::<AppState>();
    state.mcp.set_port(self::app(app.clone()), port)?;
    state.mcp.status(&state)
}

/// トークンを作り直す。動いていれば始め直す (中継は発見用のファイルを読み直すので登録は変えなくてよい)。
#[tauri::command]
pub fn regenerate_mcp_bridge_token(app: AppHandle) -> IpcResult<McpBridgeStatus> {
    let state = app.state::<AppState>();
    state.mcp.regenerate_token(self::app(app.clone()))?;
    state.mcp.status(&state)
}

/// トークン (HTTP で直接つなぐクライアントの設定に写すため。画面は求められたときだけ読む)。
#[tauri::command]
pub fn get_mcp_bridge_token(app: AppHandle) -> IpcResult<String> {
    Ok(app.state::<AppState>().mcp.token()?)
}

/// 確認に答える。待っている確認が無ければ (時間切れの後など) `false`。
#[tauri::command]
pub fn answer_mcp_approval(app: AppHandle, id: u64, approved: bool) -> bool {
    app.state::<AppState>().mcp.answer(id, approved)
}

/// 起動時: 設定を読み、有効なら始める。
pub fn setup(app: &AppHandle) {
    let state = app.state::<AppState>();
    let loaded = match app.path().app_config_dir() {
        Ok(dir) => state.mcp.load(dir),
        Err(e) => Err(EditorError::Internal(e.to_string())),
    };
    if let Err(e) = loaded {
        eprintln!("could not read the MCP bridge settings: {e}");
    }
    state.mcp.start_if_enabled(self::app(app.clone()));
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
