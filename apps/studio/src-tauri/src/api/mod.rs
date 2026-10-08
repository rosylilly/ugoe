//! スタジオの操作 API (ugoe Studio API、T0805): ループバックの WebSocket の上の JSON-RPC 2.0 で、外のツール (Stream Deck・
//! チャットのボットなど) からシーン・表情・モーション・パラメータ・着せ替えを操作し、変化の通知を受け取らせる。
//!
//! 安全性の規則は ADR 0028 の 6.、メッセージは `docs/studio-api.md` (英語の API リファレンス) と `docs/studio-api.schema.json` (JSON Schema)。
//!
//! - **既定は切。** 操作ウィンドウで有効にしたときだけ待ち受ける ([`server`])。ループバック (`127.0.0.1` と `::1`) にだけ束ねる。
//! - **トークンと利用者の許可。** 初めてのクライアントは `session.hello` でトークンと名前を示し、操作ウィンドウで利用者が
//!   承認する (権限を選ぶ) まで何もできない。承認したらクライアント用の鍵を返し、以後はその鍵で接続する ([`store`])。
//! - 操作は指揮役と舞台に入れる (ホットキーと同じ口。ADR 0028 の 3.)。パラメータの値の上書きは期限付き
//!   ([`crate::conductor::Conductor::override_parameter`])。
//!
//! 接続ごとの状態 (名乗る前・承認待ち・使える) と送る口はここが持つ。WebSocket の読み書きは [`server`] と [`ws`]。

pub mod commands;
pub mod http;
mod methods;
pub use methods::{MAX_MODULE_SUMMARIES, ModuleSummaries};
pub mod rpc;
pub mod server;
pub mod source;
pub mod store;
pub mod ws;

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Map, Value, json};
use tauri::{Manager, UserAttentionType};

use crate::animation::AnimationView;
use crate::conductor::{Conductor, FiredEvent};
use crate::error::CommandError;
use crate::stage::Stage;
use crate::{CONTROL, Monitor, Studio, attention, lock};
use rpc::{Id, RpcError};
use store::{ApiSettings, ClientRecord, Permission};

/// API の版 (`session.hello` の応答)。互換の無い変更をしたら上げる。
pub const API_VERSION: u32 = 1;
/// 設定と鍵を置くファイル (アプリのデータの場所)。
pub const SETTINGS_FILE: &str = "studio-api.json";
/// 同時に保てる接続の数の上限。
pub const MAX_CONNECTIONS: usize = 32;
/// 承認を待てるクライアントの数の上限 (操作ウィンドウが知らせで埋まらないように)。
const MAX_PENDING: usize = 4;

/// 通知の種類 (`events.subscribe` で選ぶ)。
pub const EVENTS: &[&str] = &[
    "scene.changed",
    "scene.listChanged",
    "expression.changed",
    "motion.started",
    "motion.finished",
    "motion.event",
    "dressup.changed",
    "pose.changed",
    "tracking.changed",
];

/// 接続へ送るもの。送る口のスレッド ([`server`]) が WebSocket のフレームにする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// テキストのメッセージ (JSON-RPC の応答か通知)。
    Text(String),
    /// 閉じる (WebSocket の close の符号と理由)。
    Close(u16, &'static str),
}

/// WebSocket の close の符号: 利用者が鍵を取り消した。
pub const CLOSE_REVOKED: u16 = 4001;
/// WebSocket の close の符号: トークンか鍵が違う、または利用者が承認しなかった。
pub const CLOSE_DENIED: u16 = 4003;
/// WebSocket の close の符号: つないでから決まった時間 (30 秒) のうちに `session.hello` をしなかった (HTTP の 408 に倣う)。
pub const CLOSE_HELLO_TIMEOUT: u16 = 4008;
/// WebSocket の close の符号: 利用者が切断した・API を切った (1001 Going Away)。
pub const CLOSE_GOING_AWAY: u16 = 1001;

/// 接続の段階。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Session {
    /// まだ `session.hello` をしていない。
    New,
    /// 利用者の承認を待っている (`session.hello` の応答は承認のときに返す)。
    Pending { request: Id, pending: String },
    /// 使える。
    Ready {
        client: String,
        permission: Permission,
        events: BTreeSet<String>,
    },
}

#[derive(Debug)]
struct Connection {
    sender: Sender<Outgoing>,
    session: Session,
}

/// 承認を待つクライアント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingClient {
    /// 承認・拒否で指す ID。
    pub id: String,
    pub name: String,
    /// クライアントが求めた権限 (画面の初めの選択)。
    pub permission: Permission,
    #[serde(skip)]
    connection: u64,
}

/// 操作ウィンドウに出す、承認したクライアント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientView {
    pub id: String,
    pub name: String,
    pub permission: Permission,
    pub approved_at: u64,
    pub last_seen: Option<u64>,
    /// いま接続している数。
    pub connections: usize,
}

/// 操作ウィンドウに出す API の様子 (`lib.rs` の `Monitor::Api`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiView {
    pub enabled: bool,
    /// 操作 API を受け付けている (有効でも、ポートを使えなければ `false`)。
    pub listening: bool,
    /// 待ち受けるポート (ブラウザソースと共有)。
    pub port: u16,
    /// 待ち受けられない・設定を読めない・書けない理由。
    pub error: Option<String>,
    pub clients: Vec<ClientView>,
    pub pending: Vec<PendingClient>,
    /// 名乗る前と承認待ちも含めた接続の数。
    pub connections: usize,
    /// ブラウザソース (T0808。待ち受けとポートを共有する)。
    pub source: SourceView,
}

/// 操作ウィンドウに出すブラウザソースの様子 (URL はトークンを含むので `source_url` で別に取る)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    pub enabled: bool,
    /// ページを出している (有効でも、ポートを使えなければ `false`)。
    pub listening: bool,
    /// つないでいるページの数。
    pub connections: usize,
}

/// 操作 API の状態 (`Studio` が持つ)。
#[derive(Debug, Default)]
pub struct Api(Mutex<ApiState>);

#[derive(Debug, Default)]
struct ApiState {
    /// 設定を書く場所。`None` なら覚えない (テスト)。
    path: Option<PathBuf>,
    settings: ApiSettings,
    /// 待ち受けられない・設定を書けない理由。
    error: Option<String>,
    /// 起動したときに設定のファイルを読めなかった理由 (次に書けたら消す)。
    file_error: Option<String>,
    server: Option<server::Server>,
    connections: HashMap<u64, Connection>,
    next_connection: u64,
    pending: Vec<PendingClient>,
    /// 直前に見た演技の様子 (通知の差分を取る)。
    animation: Option<AnimationView>,
}

impl ApiState {
    fn save(&mut self) {
        if let Some(path) = &self.path {
            match self.settings.write(path) {
                Ok(()) => self.file_error = None,
                Err(e) => self.error = Some(format!("{}: {e}", path.display())),
            }
        }
    }

    fn view(&self) -> ApiView {
        let clients = self
            .settings
            .clients
            .iter()
            .map(|c| ClientView {
                id: c.id.clone(),
                name: c.name.clone(),
                permission: c.permission,
                approved_at: c.approved_at,
                last_seen: c.last_seen,
                connections: self
                    .connections
                    .values()
                    .filter(|conn| matches!(&conn.session, Session::Ready { client, .. } if *client == c.id))
                    .count(),
            })
            .collect();
        ApiView {
            enabled: self.settings.enabled,
            listening: self.server.is_some() && self.settings.enabled,
            port: self.settings.port,
            error: self.error.clone().or_else(|| self.file_error.clone()),
            clients,
            pending: self.pending.clone(),
            connections: self.connections.len(),
            source: SourceView {
                enabled: self.settings.source.enabled,
                listening: self.server.is_some() && self.settings.source.enabled,
                connections: 0,
            },
        }
    }

    fn send(&self, connection: u64, message: Outgoing) {
        if let Some(conn) = self.connections.get(&connection) {
            // 送る口が閉じていれば、読む側が外す。
            let _ = conn.sender.send(message);
        }
    }

    /// 接続を閉じる (送る口に close を送り、名簿から外す)。
    fn close(&mut self, connection: u64, code: u16, reason: &'static str) {
        if let Some(conn) = self.connections.remove(&connection) {
            let _ = conn.sender.send(Outgoing::Close(code, reason));
        }
        self.pending.retain(|p| p.connection != connection);
    }

    fn close_all(&mut self, code: u16, reason: &'static str) {
        let ids: Vec<u64> = self.connections.keys().copied().collect();
        for id in ids {
            self.close(id, code, reason);
        }
    }
}

/// 今の時刻 (Unix 時間の秒)。
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `session.hello` の引数。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Hello {
    name: String,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    permission: Option<Permission>,
}

/// `events.subscribe` / `events.unsubscribe` の引数。
#[derive(Debug, serde::Deserialize)]
struct EventsParams {
    events: Vec<String>,
}

fn params<T: serde::de::DeserializeOwned>(params: &Map<String, Value>) -> Result<T, RpcError> {
    serde_json::from_value(Value::Object(params.clone()))
        .map_err(|e| RpcError::invalid_params(e.to_string()))
}

impl Studio {
    // --- 状態 ---

    /// 設定を読む (起動時)。有効になっていれば待ち受ける。`path` は `None` なら覚えない。
    pub(crate) fn load_api(self: &Arc<Self>, path: Option<PathBuf>) {
        {
            let mut state = lock(&self.api.0);
            let (settings, error) = match &path {
                Some(path) => ApiSettings::read(path),
                None => (ApiSettings::default(), None),
            };
            state.settings = settings;
            state.path = path;
            // 作ったトークンを覚える (読み直すたびに変わらないように)。読めなかったファイル (壊れたもの・新しい版の
            // スタジオが書いたもの) は、起動しただけでは書き換えない (承認したクライアントの鍵を黙って失わない)。
            // 利用者が設定を変えたときに書く。
            state.file_error = error;
            if state.file_error.is_none() {
                state.save();
            }
        }
        self.sync_listener(false);
    }

    /// 操作ウィンドウに出す様子。
    pub(crate) fn api_view(&self) -> ApiView {
        let mut view = lock(&self.api.0).view();
        view.source.connections = self.sources.len();
        view
    }

    pub(crate) fn send_api_view(&self) {
        let view = self.api_view();
        let request =
            lock(&self.approval_attention).pending(view.pending.iter().map(|p| p.id.as_str()));
        self.request_control_attention(request);
        if let Some(body) = Self::json(&Monitor::Api(view)) {
            self.monitors.send_all(&body);
        }
    }

    /// 操作ウィンドウが前面にあるかが変わった (承認を待つクライアントを OS の側で知らせるか。T0856)。
    pub(crate) fn control_focused(&self, focused: bool) {
        let request = lock(&self.approval_attention).focus(focused);
        self.request_control_attention(request);
    }

    /// 操作ウィンドウの注意を引く・止める (Windows はタスクバーの点滅、macOS は Dock のアイコンの跳ね、Linux は
    /// ウィンドウマネージャの「注意を要する」の印。docs/studio.md#外部-api)。ロックを握らずに呼ぶ。テストでは何もしない。
    fn request_control_attention(&self, request: Option<attention::Request>) {
        let Some(request) = request else { return };
        let Some(window) = self
            .app
            .get()
            .and_then(|app| app.get_webview_window(CONTROL))
        else {
            return;
        };
        let kind = match request {
            attention::Request::Start => Some(UserAttentionType::Critical),
            attention::Request::Stop => None,
        };
        // 引けなくても承認の知らせは操作ウィンドウに出ているので、誤りは捨てる。
        let _ = window.request_user_attention(kind);
    }

    /// 設定に合わせて待ち受けを止める・始める。操作 API とブラウザソースのどちらかが有効なら待ち受ける (ADR 0028 の 6.)。
    /// `restart` なら (ポートを変えた) 待ち受けていても始め直す。切った方の接続 (と始め直すときはすべての接続) を閉じる。
    pub(crate) fn sync_listener(self: &Arc<Self>, restart: bool) {
        let mut state = lock(&self.api.0);
        let wanted = state.settings.enabled || state.settings.source.enabled;
        if (restart || !wanted)
            && let Some(server) = state.server.take()
        {
            server.stop();
        }
        if restart {
            state.close_all(CLOSE_GOING_AWAY, "the port was changed");
        } else if !state.settings.enabled {
            state.close_all(CLOSE_GOING_AWAY, "the API was turned off");
        }
        let close_sources = restart || !state.settings.source.enabled;
        if wanted && state.server.is_none() {
            match server::Server::start(Arc::clone(self), state.settings.port) {
                Ok(server) => {
                    state.server = Some(server);
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
        } else if !wanted {
            state.error = None;
        }
        drop(state);
        if close_sources {
            self.close_sources(if restart {
                "the port was changed"
            } else {
                "the browser source was turned off"
            });
        }
        self.send_api_view();
    }

    pub(crate) fn set_api_enabled(self: &Arc<Self>, enabled: bool) {
        {
            let mut state = lock(&self.api.0);
            if state.settings.enabled == enabled && (!enabled || state.server.is_some()) {
                return;
            }
            state.settings.enabled = enabled;
            state.save();
        }
        self.sync_listener(false);
    }

    pub(crate) fn set_api_port(self: &Arc<Self>, port: u16) -> Result<(), CommandError> {
        if port < store::MIN_PORT {
            return Err(CommandError::unsupported_api_port(port, store::MIN_PORT));
        }
        {
            let mut state = lock(&self.api.0);
            if state.settings.port == port {
                return Ok(());
            }
            state.settings.port = port;
            state.save();
        }
        self.sync_listener(true);
        Ok(())
    }

    pub(crate) fn api_token(&self) -> String {
        lock(&self.api.0).settings.token.clone()
    }

    /// トークンを作り直す (古いトークンでは初めての接続ができなくなる。承認したクライアントの鍵はそのまま)。
    pub(crate) fn regenerate_api_token(&self) -> Result<String, CommandError> {
        let token = store::generate_secret().map_err(CommandError::random_failed)?;
        let mut state = lock(&self.api.0);
        state.settings.token.clone_from(&token);
        state.save();
        Ok(token)
    }

    /// 承認を待つクライアントを承認する: 鍵を作って覚え、`session.hello` に応答する。
    /// 承認したクライアントの ID (承認を待つ間の ID とは別に作る。[`ClientView::id`]) を返す。
    pub(crate) fn approve_api_client(
        &self,
        id: &str,
        permission: Permission,
    ) -> Result<String, CommandError> {
        let mut state = lock(&self.api.0);
        let index = state
            .pending
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| CommandError::unknown_pending_client(id))?;
        if state.settings.clients.len() >= store::MAX_CLIENTS {
            return Err(CommandError::too_many_api_clients(store::MAX_CLIENTS));
        }
        let key = store::generate_secret().map_err(CommandError::random_failed)?;
        let client = store::generate_id().map_err(CommandError::random_failed)?;
        let pending = state.pending.remove(index);
        let now = now_secs();
        state.settings.clients.push(ClientRecord {
            id: client.clone(),
            name: pending.name.clone(),
            key: key.clone(),
            permission,
            approved_at: now,
            last_seen: Some(now),
        });
        state.save();
        if let Some(conn) = state.connections.get_mut(&pending.connection)
            && let Session::Pending { request, .. } = &conn.session
        {
            let request = request.clone();
            conn.session = Session::Ready {
                client: client.clone(),
                permission,
                events: BTreeSet::new(),
            };
            let result = hello_result(&client, permission, Some(&key));
            let _ = conn
                .sender
                .send(Outgoing::Text(rpc::result(&request, result)));
        }
        drop(state);
        self.send_api_view();
        Ok(client)
    }

    /// 承認を待つクライアントを断る (誤りを返して閉じる)。
    pub(crate) fn deny_api_client(&self, id: &str) {
        let mut state = lock(&self.api.0);
        if let Some(index) = state.pending.iter().position(|p| p.id == id) {
            let pending = state.pending.remove(index);
            if let Some(Connection {
                session: Session::Pending { request, .. },
                ..
            }) = state.connections.get(&pending.connection)
            {
                let error =
                    RpcError::new(rpc::ACCESS_DENIED, "the user did not approve this client");
                let message = rpc::error(Some(request), &error);
                state.send(pending.connection, Outgoing::Text(message));
            }
            state.close(pending.connection, CLOSE_DENIED, "not approved");
        }
        drop(state);
        self.send_api_view();
    }

    /// 承認したクライアントの鍵を取り消す (つないでいれば切る)。
    pub(crate) fn revoke_api_client(&self, id: &str) {
        let mut state = lock(&self.api.0);
        state.settings.clients.retain(|c| c.id != id);
        state.save();
        for connection in connections_of(&state, id) {
            state.close(connection, CLOSE_REVOKED, "revoked");
        }
        drop(state);
        self.send_api_view();
    }

    /// 承認したクライアントの今の接続を切る (鍵はそのまま。また接続できる)。
    pub(crate) fn disconnect_api_client(&self, id: &str) {
        let mut state = lock(&self.api.0);
        for connection in connections_of(&state, id) {
            state.close(connection, CLOSE_GOING_AWAY, "disconnected by the user");
        }
        drop(state);
        self.send_api_view();
    }

    // --- 接続 ---

    /// WebSocket の接続を名簿に入れる。多すぎれば `None`。
    pub(crate) fn api_connect(&self, sender: Sender<Outgoing>) -> Option<u64> {
        let mut state = lock(&self.api.0);
        if state.connections.len() >= MAX_CONNECTIONS {
            return None;
        }
        let id = state.next_connection;
        state.next_connection += 1;
        state.connections.insert(
            id,
            Connection {
                sender,
                session: Session::New,
            },
        );
        drop(state);
        self.send_api_view();
        Some(id)
    }

    /// 接続が閉じた (承認待ちなら取り下げる)。
    pub(crate) fn api_disconnect(&self, connection: u64) {
        let mut state = lock(&self.api.0);
        let known = state.connections.remove(&connection).is_some();
        state.pending.retain(|p| p.connection != connection);
        drop(state);
        if known {
            self.send_api_view();
        }
    }

    /// 接続が名乗ったか (`session.hello` を受けた。承認待ちも含む)。名簿に無ければ `true` (もう閉じている)。
    pub(crate) fn api_introduced(&self, connection: u64) -> bool {
        lock(&self.api.0)
            .connections
            .get(&connection)
            .is_none_or(|conn| conn.session != Session::New)
    }

    /// 接続に送る。
    fn api_send(&self, connection: u64, message: Outgoing) {
        lock(&self.api.0).send(connection, message);
    }

    /// 届いたテキストのメッセージを 1 つ処理し、応答を送る (承認を待つ `session.hello` は承認のときに応答する)。
    pub(crate) fn api_receive(&self, connection: u64, text: &str) {
        let request = match rpc::parse(text) {
            Ok(request) => request,
            Err((id, error)) => {
                self.api_send(connection, Outgoing::Text(rpc::error(id.as_ref(), &error)));
                return;
            }
        };
        let outcome = self.api_call(connection, &request);
        let Some(id) = &request.id else {
            return;
        };
        let message = match outcome {
            Some(Ok(result)) => rpc::result(id, result),
            Some(Err(error)) => rpc::error(Some(id), &error),
            // 承認を待つ (応答は後で)。
            None => return,
        };
        self.api_send(connection, Outgoing::Text(message));
    }

    /// 要求を 1 つ行う。応答を後で返すなら `None`。
    fn api_call(&self, connection: u64, request: &rpc::Request) -> Option<Result<Value, RpcError>> {
        let session = lock(&self.api.0)
            .connections
            .get(&connection)
            .map(|c| c.session.clone())?;
        if request.method == "session.hello" {
            return self.api_hello(connection, request);
        }
        let Session::Ready {
            permission, client, ..
        } = session
        else {
            let message = match session {
                Session::Pending { .. } => "waiting for the user to approve this client",
                _ => "call session.hello first",
            };
            return Some(Err(RpcError::new(rpc::NOT_AUTHENTICATED, message)));
        };
        let Some(required) = methods::permission(&request.method) else {
            return Some(Err(RpcError::new(
                rpc::METHOD_NOT_FOUND,
                format!("unknown method `{}`", request.method),
            )));
        };
        if required == Permission::Control && permission != Permission::Control {
            return Some(Err(RpcError::new(
                rpc::FORBIDDEN,
                format!("`{}` needs the control permission", request.method),
            )));
        }
        // 取り消された鍵の接続は、閉じる前に届いた要求も断る。
        if !lock(&self.api.0)
            .settings
            .clients
            .iter()
            .any(|c| c.id == client)
        {
            return Some(Err(RpcError::new(
                rpc::ACCESS_DENIED,
                "the key was revoked",
            )));
        }
        Some(match request.method.as_str() {
            "events.subscribe" => self.api_subscribe(connection, &request.params, true),
            "events.unsubscribe" => self.api_subscribe(connection, &request.params, false),
            method => methods::call(self, method, &request.params),
        })
    }

    fn api_hello(
        &self,
        connection: u64,
        request: &rpc::Request,
    ) -> Option<Result<Value, RpcError>> {
        let hello: Hello = match params(&request.params) {
            Ok(hello) => hello,
            Err(e) => return Some(Err(e)),
        };
        let name = store::clean_name(&hello.name);
        if name.is_empty() {
            return Some(Err(RpcError::invalid_params("`name` must not be empty")));
        }
        let mut state = lock(&self.api.0);
        let session = state.connections.get(&connection)?.session.clone();
        if session != Session::New {
            return Some(Err(RpcError::new(
                rpc::INVALID_REQUEST,
                "session.hello was already called",
            )));
        }
        // 承認したクライアントの鍵。
        if let Some(key) = hello.key.as_deref() {
            let found = state
                .settings
                .client_by_key(key)
                .map(|c| (c.id.clone(), c.permission));
            if let Some((client, permission)) = found {
                if let Some(record) = state.settings.clients.iter_mut().find(|c| c.id == client) {
                    record.last_seen = Some(now_secs());
                }
                state.save();
                if let Some(conn) = state.connections.get_mut(&connection) {
                    conn.session = Session::Ready {
                        client: client.clone(),
                        permission,
                        events: BTreeSet::new(),
                    };
                }
                drop(state);
                self.send_api_view();
                return Some(Ok(hello_result(&client, permission, None)));
            }
        }
        let token_ok = hello.token.as_deref().is_some_and(|token| {
            !state.settings.token.is_empty()
                && store::constant_time_eq(token, &state.settings.token)
        });
        if !token_ok {
            let error = RpcError::new(rpc::ACCESS_DENIED, "unknown key or wrong token");
            if let Some(id) = &request.id {
                state.send(connection, Outgoing::Text(rpc::error(Some(id), &error)));
            }
            state.close(connection, CLOSE_DENIED, "access denied");
            drop(state);
            self.send_api_view();
            return None;
        }
        let Some(id) = request.id.clone() else {
            return Some(Err(RpcError::new(
                rpc::INVALID_REQUEST,
                "session.hello needs an id",
            )));
        };
        if state.pending.len() >= MAX_PENDING {
            return Some(Err(RpcError::new(
                rpc::NOT_AUTHENTICATED,
                "too many clients are waiting for approval; try again later",
            )));
        }
        let pending = match store::generate_id() {
            Ok(pending) => pending,
            Err(e) => return Some(Err(RpcError::new(rpc::INTERNAL_ERROR, e))),
        };
        state.pending.push(PendingClient {
            id: pending.clone(),
            name,
            permission: hello.permission.unwrap_or_default(),
            connection,
        });
        if let Some(conn) = state.connections.get_mut(&connection) {
            conn.session = Session::Pending {
                request: id,
                pending,
            };
        }
        drop(state);
        self.send_api_view();
        None
    }

    fn api_subscribe(
        &self,
        connection: u64,
        params_: &Map<String, Value>,
        subscribe: bool,
    ) -> Result<Value, RpcError> {
        let EventsParams { events } = params(params_)?;
        if let Some(unknown) = events.iter().find(|e| !EVENTS.contains(&e.as_str())) {
            return Err(RpcError::invalid_params(format!(
                "unknown event `{unknown}`"
            )));
        }
        let mut state = lock(&self.api.0);
        let Some(Connection {
            session: Session::Ready {
                events: current, ..
            },
            ..
        }) = state.connections.get_mut(&connection)
        else {
            return Err(RpcError::new(
                rpc::NOT_AUTHENTICATED,
                "call session.hello first",
            ));
        };
        for event in events {
            if subscribe {
                current.insert(event);
            } else {
                current.remove(&event);
            }
        }
        Ok(json!({ "events": current.iter().collect::<Vec<_>>() }))
    }

    // --- 通知 ---

    /// 通知 `event` を、受け取ると選んだ接続へ送る。
    pub(crate) fn api_notify(&self, event: &str, params: Value) {
        let state = lock(&self.api.0);
        let mut message = None;
        for conn in state.connections.values() {
            if let Session::Ready { events, .. } = &conn.session
                && events.contains(event)
            {
                let text = message.get_or_insert_with(|| rpc::notification(event, params.clone()));
                let _ = conn.sender.send(Outgoing::Text(text.clone()));
            }
        }
    }

    /// 通知を受ける接続があるか (通知の中身を作る前に見る)。
    fn api_listening_for(&self, event: &str) -> bool {
        lock(&self.api.0)
            .connections
            .values()
            .any(|c| matches!(&c.session, Session::Ready { events, .. } if events.contains(event)))
    }

    /// 刻みの後: 演技の様子の変化 (表情・モーション・トラッキング。変わっていなければ `view` は `None`) と、
    /// 発火したモーションのイベントを通知する。
    ///
    /// 順序: モーションのイベントは、そのモーションの `motion.started` の後に送る。この刻みで終わった (または替わった)
    /// モーションのイベントは、その `motion.finished` の前に送る (終わりで発火したイベントが、終わりの知らせより後に届かないように)。
    pub(crate) fn api_animation(&self, view: Option<&AnimationView>, fired: &[FiredEvent]) {
        let mut events = match view {
            Some(view) => {
                let previous = lock(&self.api.0).animation.replace(view.clone());
                animation_events(previous.as_ref(), view)
            }
            None => Vec::new(),
        };
        if !fired.is_empty() && self.api_listening_for("motion.event") {
            // 終わった (替えられた) モーションのイベントは、その終わりの前に送る。
            let finished = events.iter().position(|(e, _)| *e == "motion.finished");
            let mut ended = Vec::new();
            for f in fired {
                let params =
                    json!({ "motion": f.motion, "value": f.event.value, "time": f.event.t });
                if !f.current {
                    ended.push(("motion.event", params));
                } else {
                    events.push(("motion.event", params));
                }
            }
            let at = finished.unwrap_or(0);
            events.splice(at..at, ended);
        }
        for (event, params) in events {
            self.api_notify(event, params);
        }
    }

    /// ポーズの表示中の要素か選択が変わった: `pose.changed` を通知する (中身は `pose.list` と同じ。T0848)。
    pub(crate) fn api_pose_changed(&self, conductor: &Conductor) {
        if self.api_listening_for("pose.changed") {
            self.api_notify("pose.changed", methods::pose_value(conductor));
        }
    }

    /// 舞台が変わった: 出しているシーン・着せ替え・シーンの一覧の変化を通知する ([`StageSnapshot`] を前後で比べる)。
    pub(crate) fn api_stage_changed(&self, before: &StageSnapshot, after: &StageSnapshot) {
        if before.live != after.live {
            self.api_notify("scene.changed", json!({ "scene": after.live_scene() }));
        }
        // シーンを切り替えて付けているモジュールが変わったときも送る (着せ替えの組だけを見ているクライアントのため)。
        if before.modules != after.modules {
            self.api_notify("dressup.changed", json!({ "modules": after.modules }));
        }
        if before.scenes != after.scenes {
            self.api_notify("scene.listChanged", json!({ "scenes": after.scenes }));
        }
    }
}

/// 承認したクライアント `id` の接続。
fn connections_of(state: &ApiState, id: &str) -> Vec<u64> {
    state
        .connections
        .iter()
        .filter(|(_, c)| matches!(&c.session, Session::Ready { client, .. } if client == id))
        .map(|(k, _)| *k)
        .collect()
}

/// `session.hello` の応答。
fn hello_result(client: &str, permission: Permission, key: Option<&str>) -> Value {
    let mut result = json!({
        "studioVersion": env!("CARGO_PKG_VERSION"),
        "apiVersion": API_VERSION,
        "client": client,
        "permission": permission,
        "events": EVENTS,
    });
    if let Some(key) = key {
        result["key"] = json!(key);
    }
    result
}

/// 演技の様子の差分から通知を作る。
fn animation_events(
    previous: Option<&AnimationView>,
    view: &AnimationView,
) -> Vec<(&'static str, Value)> {
    let mut events = Vec::new();
    let empty = AnimationView {
        motions: Vec::new(),
        expressions: Vec::new(),
        current: None,
        active_expressions: Vec::new(),
        idle_motion: None,
        tracking: false,
        tracking_paused: false,
        lip_sync_paused: false,
        tracking_parameters: Vec::new(),
        slots: Vec::new(),
        motion_start: 0,
    };
    let previous = previous.unwrap_or(&empty);
    if previous.active_expressions != view.active_expressions {
        events.push((
            "expression.changed",
            json!({ "active": view.active_expressions }),
        ));
    }
    // 同じモーションを始め直したときも、終わりと始まりを知らせる。
    if (&previous.current, previous.motion_start) != (&view.current, view.motion_start) {
        if let Some(motion) = &previous.current {
            events.push(("motion.finished", json!({ "motion": motion.name })));
        }
        if let Some(motion) = &view.current {
            events.push((
                "motion.started",
                json!({ "motion": motion.name, "priority": motion.priority }),
            ));
        }
    }
    if (
        previous.tracking,
        previous.tracking_paused,
        previous.lip_sync_paused,
    ) != (view.tracking, view.tracking_paused, view.lip_sync_paused)
    {
        events.push(("tracking.changed", methods::tracking_state(view)));
    }
    events
}

/// 通知の差分を取るための舞台の様子 ([`Studio::with_stage`] の前後で取る)。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageSnapshot {
    live: Option<Value>,
    modules: Vec<Value>,
    scenes: Vec<Value>,
}

impl StageSnapshot {
    pub(crate) fn of(stage: &Stage) -> Self {
        let scenes = methods::scene_list(stage);
        let live = stage
            .live()
            .and_then(|id| scenes.iter().find(|s| s["id"] == id).cloned());
        Self {
            live,
            modules: methods::live_modules(stage),
            scenes,
        }
    }

    fn live_scene(&self) -> Value {
        self.live.clone().unwrap_or(Value::Null)
    }
}

#[cfg(all(test, feature = "gpu-tests"))]
mod source_check;
#[cfg(test)]
mod tests;
