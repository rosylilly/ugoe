//! MCP の JSON-RPC の処理 (トランスポートに依らない部分)。
//!
//! 1 つのメッセージ (JSON-RPC の要求・通知・応答) を受け取り、返す応答を作る。
//! 旧来の `initialize` の握手をする版 (2025-11-25 以前) と、要求ごとに `_meta` で版を示す版
//! (2026-07-28) の両方に応じる。仕様: <https://modelcontextprotocol.io/specification/>

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value, json};

use crate::host::{Cancellation, DocumentHost};
use crate::render::panic_message;
use crate::tools::{Options, Tools};

/// 要求ごとに `_meta` で版を示す版 (握手をしない)。
pub const MODERN_VERSION: &str = "2026-07-28";
/// `initialize` の握手をする版。先頭が最新。
pub const LEGACY_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub(crate) const META_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
const META_CLIENT_INFO: &str = "io.modelcontextprotocol/clientInfo";
const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

// JSON-RPC のエラーコード。
pub(crate) const PARSE_ERROR: i64 = -32700;
pub(crate) const INVALID_REQUEST: i64 = -32600;
pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
pub(crate) const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// サーバがエージェントに示す使い方。
const INSTRUCTIONS: &str = "\
ugoe の .ugoeproj プロジェクト (2D リギングのモデル) を読み、編集を提案・適用する。\n\
手順: open_project でプロジェクトを開く → get_model_summary / get_element で構成を読む → \
propose_edit で Command の列を提案する (検証と差分だけを返し、モデルは変えない) → \
ユーザーの承認を得て apply_proposal で適用する → render_png で見た目を確かめる → save_project で保存する。\n\
適用は 1 回の undo で取り消せる。save_project を呼ぶまでディスクには書かない。\n\
PSD からの自動リギング: import_psd → get_psd_layers (画像を見て分類する) → set_layer_labels → \
get_rig_guide (画像を見て値を選ぶ) → propose_rig → ユーザーの承認を得て apply_proposal。\
着せ替えのモジュール (import_psd に base と slot を渡して作る) では、get_rig_guide と propose_rig の代わりに \
get_module_rig_guide と propose_module_rig (パラメータを足さない。set_layer_labels の variant_sets はポーズになる)。\
分類と値はあなたが画像を見て決める (サーバは AI を呼ばない)。";

/// サーバの状態。1 つのプロセスにつき 1 つ (stdio の接続、または HTTP のすべての要求で共有する)。
/// `&self` で処理でき、HTTP の接続ごとのスレッドから同時に呼んでよい。
pub struct Server {
    tools: Tools,
    /// 処理中の要求の取り消しの印。キーは要求の id の JSON の文 (`1` と `"1"` を区別する)。
    /// HTTP はセッションを持たないので、別のクライアントの要求が同じ id を持つことがある。
    in_flight: Mutex<HashMap<String, Vec<Arc<Cancellation>>>>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("tools", &self.tools)
            .finish_non_exhaustive()
    }
}

/// 処理中の要求の登録 (捨てると外す)。
struct InFlight<'a> {
    server: &'a Server,
    key: String,
    cancellation: Arc<Cancellation>,
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        let mut in_flight = self.server.lock_in_flight();
        if let Some(list) = in_flight.get_mut(&self.key) {
            list.retain(|c| !Arc::ptr_eq(c, &self.cancellation));
            if list.is_empty() {
                in_flight.remove(&self.key);
            }
        }
    }
}

/// 要求の版。
enum Era {
    /// `initialize` の握手をする版 (または版を示さない要求)。
    Legacy,
    /// 要求ごとに `_meta` で版を示す版。結果に `resultType` と `serverInfo` を付ける。
    Modern,
}

impl Server {
    /// ファイルを自分で開くサーバ (`crate::host::FileHost`)。
    pub fn new(options: Options) -> Self {
        Self {
            tools: Tools::new(options),
            in_flight: Mutex::default(),
        }
    }

    /// 外から渡した持ち主 (エディタなど) の文書を扱うサーバ。
    pub fn with_host(options: Options, host: Arc<dyn DocumentHost>) -> Self {
        Self {
            tools: Tools::with_host(options, host),
            in_flight: Mutex::default(),
        }
    }

    /// 1 行 (1 メッセージ) を処理し、返す行を作る。通知と応答には何も返さない。
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let response = match serde_json::from_str::<Value>(line) {
            Ok(message) => self.handle(message),
            Err(e) => Some(error(
                Value::Null,
                PARSE_ERROR,
                &format!("parse error: {e}"),
            )),
        };
        // `Value` の文字列化は失敗しない。改行は文字列の中でもエスケープされるので、1 行になる。
        response.map(|r| r.to_string())
    }

    /// 1 つのメッセージを処理する。
    pub fn handle(&self, message: Value) -> Option<Value> {
        self.handle_cancellable(message, None)
    }

    /// 1 つのメッセージを処理する。`disconnected` はクライアントが答えを待たずに去ったかを答える
    /// (HTTP の接続。去ったら要求を取り消す)。
    pub(crate) fn handle_cancellable(
        &self,
        message: Value,
        disconnected: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    ) -> Option<Value> {
        let Value::Object(message) = message else {
            // JSON-RPC のバッチ (配列) は 2025-06-18 で取り除かれたので受け付けない。
            return Some(error(
                Value::Null,
                INVALID_REQUEST,
                "expected a JSON-RPC object",
            ));
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // クライアントからの応答 (このサーバは要求を送らないので来ないはず) は無視する。
            if message.contains_key("result") || message.contains_key("error") {
                return None;
            }
            let id = message.get("id").cloned().unwrap_or(Value::Null);
            return Some(error(id, INVALID_REQUEST, "missing method"));
        };
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(id) = message.get("id").cloned() else {
            // 通知 (`notifications/initialized`・`notifications/cancelled` など)。応答しない。
            if method == "notifications/cancelled" {
                self.cancel(&params);
            }
            return None;
        };
        // 処理の間、取り消しの印を登録し、このスレッドに付ける (`crate::host` の「取り消し」)。
        let registered = InFlight {
            server: self,
            key: id.to_string(),
            cancellation: Arc::new(Cancellation::new(disconnected)),
        };
        self.lock_in_flight()
            .entry(registered.key.clone())
            .or_default()
            .push(Arc::clone(&registered.cancellation));
        let reply = registered
            .cancellation
            .scope(|| self.request(id, method, &params));
        drop(registered);
        Some(reply)
    }

    fn lock_in_flight(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<Arc<Cancellation>>>> {
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// `notifications/cancelled` の `requestId` の要求を取り消す。処理中の要求のうち、その id を持つものが
    /// 1 つだけのときに限る (HTTP で複数のクライアントが同じ id を使っていれば、どれのことか分からない
    /// ので無視する。`--editor` の中継は id を重ならないものに変えて送る)。知らない id・終わった要求の
    /// 取り消しも無視する (仕様のとおり)。
    fn cancel(&self, params: &Value) {
        let Some(request) = params.get("requestId") else {
            return;
        };
        let in_flight = self.lock_in_flight();
        if let Some([cancellation]) = in_flight.get(&request.to_string()).map(Vec::as_slice) {
            cancellation.cancel();
            let reason = params.get("reason").and_then(Value::as_str);
            eprintln!(
                "ugoe-mcp: the client cancelled request {request}{}",
                reason.map(|r| format!(": {r}")).unwrap_or_default()
            );
        }
    }

    fn request(&self, id: Value, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => return success(id, self.initialize(params)),
            "server/discover" => return success(id, self.discover()),
            _ => {}
        }
        let era = match era(params) {
            Ok(era) => era,
            Err((code, message, data)) => return error_with_data(id, code, &message, data),
        };
        if let Some(info) = params.get("_meta").and_then(|m| m.get(META_CLIENT_INFO)) {
            self.client_info(info, false);
        }
        let result = match method {
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.tools.list() })),
            "tools/call" => self.call(params),
            _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
        };
        match result {
            Ok(mut result) => {
                if matches!(era, Era::Modern)
                    && let Value::Object(map) = &mut result
                {
                    map.insert("resultType".to_owned(), json!("complete"));
                    map.insert(
                        "_meta".to_owned(),
                        json!({ META_SERVER_INFO: server_info() }),
                    );
                }
                success(id, result)
            }
            Err((code, message)) => error(id, code, &message),
        }
    }

    /// クライアントの名前と版 (`Implementation`) を覚えて持ち主に知らせる。
    fn client_info(&self, info: &Value, handshake: bool) {
        if let Some(name) = info.get("name").and_then(Value::as_str) {
            let version = info.get("version").and_then(Value::as_str);
            self.tools.client_connected(name, version, handshake);
        }
    }

    fn instructions(&self) -> String {
        match self.tools.host_instructions() {
            Some(extra) => format!("{INSTRUCTIONS}\n{extra}"),
            None => INSTRUCTIONS.to_owned(),
        }
    }

    fn initialize(&self, params: &Value) -> Value {
        if let Some(info) = params.get("clientInfo") {
            self.client_info(info, true);
        }
        // 求められた版に応じられればその版、でなければ応じられる最新の版を返す (クライアントが判断する)。
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = requested
            .and_then(|r| LEGACY_VERSIONS.iter().find(|v| **v == r))
            .unwrap_or(&LEGACY_VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": capabilities(),
            "serverInfo": server_info(),
            "instructions": self.instructions(),
        })
    }

    fn discover(&self) -> Value {
        json!({
            "resultType": "complete",
            "supportedVersions": supported_versions(),
            "capabilities": capabilities(),
            "_meta": { META_SERVER_INFO: server_info() },
            "instructions": self.instructions(),
        })
    }

    fn call(&self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((INVALID_PARAMS, "missing tool name".to_owned()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        // ツールや持ち主の不具合で panic しても、サーバ (stdio では唯一のスレッド) ごと落とさずにエラーで
        // 答える (開いている未保存の変更を失わないように)。錠は毒されても使い続ける (`tools::lock`)。
        let called = catch_unwind(AssertUnwindSafe(|| self.tools.call(name, arguments))).map_err(
            |payload| {
                (
                    INTERNAL_ERROR,
                    format!(
                        "internal error in tool `{name}`: {}",
                        panic_message(payload.as_ref())
                    ),
                )
            },
        )?;
        called.ok_or_else(|| (INVALID_PARAMS, format!("unknown tool: {name}")))
    }
}

/// 要求の `_meta` から版を決める。版を示していれば、応じられる版で、必須の項目があること。
fn era(params: &Value) -> Result<Era, (i64, String, Option<Value>)> {
    let meta = params.get("_meta").and_then(Value::as_object);
    let Some(version) = meta.and_then(|m| m.get(META_PROTOCOL_VERSION)) else {
        return Ok(Era::Legacy);
    };
    if version.as_str() != Some(MODERN_VERSION) {
        return Err((
            UNSUPPORTED_PROTOCOL_VERSION,
            "Unsupported protocol version".to_owned(),
            Some(json!({ "supported": supported_versions(), "requested": version })),
        ));
    }
    if !meta.is_some_and(|m| m.contains_key(META_CLIENT_CAPABILITIES)) {
        return Err((
            INVALID_PARAMS,
            format!("missing _meta[\"{META_CLIENT_CAPABILITIES}\"]"),
            None,
        ));
    }
    Ok(Era::Modern)
}

/// 応じられる版の一覧 (先頭が最新)。
pub(crate) fn supported_versions() -> Vec<&'static str> {
    let mut versions = vec![MODERN_VERSION];
    versions.extend_from_slice(LEGACY_VERSIONS);
    versions
}

fn capabilities() -> Value {
    json!({ "tools": { "listChanged": false } })
}

fn server_info() -> Value {
    json!({ "name": "ugoe-mcp", "title": "ugoe", "version": env!("CARGO_PKG_VERSION") })
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub(crate) fn error(id: Value, code: i64, message: &str) -> Value {
    error_with_data(id, code, message, None)
}

pub(crate) fn error_with_data(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut err = Map::new();
    err.insert("code".to_owned(), json!(code));
    err.insert("message".to_owned(), json!(message));
    if let Some(data) = data {
        err.insert("data".to_owned(), data);
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": err })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::host::PanickingHost;

    #[test]
    fn cancellation_reaches_only_an_unambiguous_request() {
        let server = Server::new(Options::default());
        let add = |id: Value| {
            let cancellation = Arc::new(Cancellation::new(None));
            server
                .lock_in_flight()
                .entry(id.to_string())
                .or_default()
                .push(Arc::clone(&cancellation));
            cancellation
        };
        let one = add(json!(1));
        let text_one = add(json!("1"));
        let shared_a = add(json!(2));
        let shared_b = add(json!(2));
        let cancel = |id: Value| {
            server.handle(json!({
                "jsonrpc": "2.0", "method": "notifications/cancelled",
                "params": { "requestId": id },
            }))
        };
        assert!(cancel(json!(1)).is_none());
        assert!(cancel(json!(2)).is_none());
        assert!(cancel(json!(3)).is_none());
        assert!(one.is_cancelled());
        assert!(!text_one.is_cancelled(), "1 and \"1\" are different ids");
        assert!(
            !shared_a.is_cancelled() && !shared_b.is_cancelled(),
            "ambiguous ids are ignored"
        );
    }

    #[test]
    fn requests_leave_the_in_flight_table() {
        let server = Server::new(Options::default());
        let ping = server
            .handle(json!({ "jsonrpc": "2.0", "id": 9, "method": "ping" }))
            .unwrap();
        assert_eq!(ping["result"], json!({}));
        assert!(server.lock_in_flight().is_empty());
    }

    #[test]
    fn a_panicking_tool_answers_with_an_internal_error() {
        let server = Server::with_host(Options::default(), Arc::new(PanickingHost));
        let call = json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "get_model_summary", "arguments": { "path": "x.ugoeproj" } },
        });
        // stdio のサーバ (要求を順に処理する 1 つのスレッド) ごと落ちずに、エラーで答える。
        let reply = server.handle(call).unwrap();
        assert_eq!(reply["id"], 3);
        assert_eq!(reply["error"]["code"], INTERNAL_ERROR);
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains("get_model_summary"), "{message}");
        assert!(message.contains("the host is broken"), "{message}");
        let ping = server
            .handle(json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }))
            .unwrap();
        assert_eq!(ping["result"], json!({}));
    }
}
