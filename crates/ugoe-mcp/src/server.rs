//! MCP の JSON-RPC の処理 (トランスポートに依らない部分)。
//!
//! 1 つのメッセージ (JSON-RPC の要求・通知・応答) を受け取り、返す応答を作る。
//! 旧来の `initialize` の握手をする版 (2025-11-25 以前) と、要求ごとに `_meta` で版を示す版
//! (2026-07-28) の両方に応じる。仕様: https://modelcontextprotocol.io/specification/

use serde_json::{Map, Value, json};

use crate::tools::{Options, Tools};

/// 要求ごとに `_meta` で版を示す版 (握手をしない)。
pub const MODERN_VERSION: &str = "2026-07-28";
/// `initialize` の握手をする版。先頭が最新。
pub const LEGACY_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub(crate) const META_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

// JSON-RPC のエラーコード。
pub(crate) const PARSE_ERROR: i64 = -32700;
pub(crate) const INVALID_REQUEST: i64 = -32600;
pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
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
分類と値はあなたが画像を見て決める (サーバは AI を呼ばない)。";

/// サーバの状態。1 つのプロセスにつき 1 つ (stdio の接続、または HTTP のすべての要求で共有する)。
#[derive(Debug)]
pub struct Server {
    tools: Tools,
}

/// 要求の版。
enum Era {
    /// `initialize` の握手をする版 (または版を示さない要求)。
    Legacy,
    /// 要求ごとに `_meta` で版を示す版。結果に `resultType` と `serverInfo` を付ける。
    Modern,
}

impl Server {
    pub fn new(options: Options) -> Self {
        Self {
            tools: Tools::new(options),
        }
    }

    /// 1 行 (1 メッセージ) を処理し、返す行を作る。通知と応答には何も返さない。
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
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
    pub fn handle(&mut self, message: Value) -> Option<Value> {
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
            return None;
        };
        Some(self.request(id, method, &params))
    }

    fn request(&mut self, id: Value, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => return success(id, self.initialize(params)),
            "server/discover" => return success(id, self.discover()),
            _ => {}
        }
        let era = match era(params) {
            Ok(era) => era,
            Err((code, message, data)) => return error_with_data(id, code, &message, data),
        };
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

    fn initialize(&self, params: &Value) -> Value {
        // 求められた版に応じられればその版、でなければ応じられる最新の版を返す (クライアントが判断する)。
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = requested
            .and_then(|r| LEGACY_VERSIONS.iter().find(|v| **v == r))
            .unwrap_or(&LEGACY_VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": capabilities(),
            "serverInfo": server_info(),
            "instructions": INSTRUCTIONS,
        })
    }

    fn discover(&self) -> Value {
        json!({
            "resultType": "complete",
            "supportedVersions": supported_versions(),
            "capabilities": capabilities(),
            "_meta": { META_SERVER_INFO: server_info() },
            "instructions": INSTRUCTIONS,
        })
    }

    fn call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((INVALID_PARAMS, "missing tool name".to_owned()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        self.tools
            .call(name, arguments)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown tool: {name}")))
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
