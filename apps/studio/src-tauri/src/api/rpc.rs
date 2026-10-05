//! JSON-RPC 2.0 のメッセージ (要求・応答・通知) の読み書き (`docs/studio-api.md`)。
//!
//! 1 つの WebSocket のテキストのメッセージに 1 つの JSON-RPC のメッセージを載せる。まとめた要求 (配列) は受けない。

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::CommandError;

/// JSON として読めない。
pub const PARSE_ERROR: i64 = -32700;
/// JSON-RPC の要求の形でない。
pub const INVALID_REQUEST: i64 = -32600;
/// 知らないメソッド。
pub const METHOD_NOT_FOUND: i64 = -32601;
/// 引数の形が違う。
pub const INVALID_PARAMS: i64 = -32602;
/// スタジオの中の誤り。
pub const INTERNAL_ERROR: i64 = -32603;
/// まだ `session.hello` を済ませていない (または承認を待っている)。
pub const NOT_AUTHENTICATED: i64 = -32000;
/// 与えられた権限では呼べない (読むだけのクライアントが操作した)。
pub const FORBIDDEN: i64 = -32001;
/// トークンか鍵が違う、または利用者が承認しなかった。
pub const ACCESS_DENIED: i64 = -32002;
/// 操作を行えなかった (`data.code` に理由。モデルが無い・知らない名前など)。
pub const OPERATION_FAILED: i64 = -32010;

/// 要求の `id` (数か文字列。通知なら無し)。
pub type Id = Value;

/// 読んだ要求。
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// 通知 (`id` が無い) なら `None`。応答を返さない。
    pub id: Option<Id>,
    pub method: String,
    /// 引数 (省けば空のオブジェクト)。
    pub params: Map<String, Value>,
}

/// 要求の誤り。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    /// 操作を行えなかった。`error` は `code` を持つ形 (`SceneError`・`AnimationError` など)。
    ///
    /// `data` は `error` をそのまま書いた形 (camelCase の `code` と平らなフィールド) に、コマンドのエラーと同じ
    /// ADR 0029 のコード (`messageCode`) と引数 (`args`) を足したもの (T0839)。元のフィールドは公開の約束なので変えない。
    pub(crate) fn failed<E>(error: &E) -> Self
    where
        E: Serialize + std::fmt::Display + Clone + Into<CommandError>,
    {
        let mut data = serde_json::to_value(error).unwrap_or(Value::Null);
        if let Value::Object(fields) = &mut data {
            let CommandError { code, args, .. } = error.clone().into();
            fields.insert("messageCode".into(), Value::from(code));
            fields.insert(
                "args".into(),
                Value::Object(
                    args.into_iter()
                        .map(|(name, value)| (name.to_owned(), value))
                        .collect(),
                ),
            );
        }
        Self::new(OPERATION_FAILED, error.to_string()).with_data(data)
    }
}

/// テキストのメッセージを要求として読む。読めなければ、送り返す誤りの応答 (`id` は分かれば付ける)。
pub fn parse(text: &str) -> Result<Request, (Option<Id>, RpcError)> {
    let value: Value = serde_json::from_str(text).map_err(|e| {
        (
            None,
            RpcError::new(PARSE_ERROR, format!("parse error: {e}")),
        )
    })?;
    let Value::Object(mut object) = value else {
        let message = if value.is_array() {
            "batch requests are not supported"
        } else {
            "the request must be a JSON object"
        };
        return Err((None, RpcError::new(INVALID_REQUEST, message)));
    };
    let id = match object.remove("id") {
        None => None,
        Some(id @ (Value::Number(_) | Value::String(_))) => Some(id),
        Some(_) => {
            return Err((
                None,
                RpcError::new(INVALID_REQUEST, "`id` must be a number or a string"),
            ));
        }
    };
    let invalid = |message: &str| (id.clone(), RpcError::new(INVALID_REQUEST, message));
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid("`jsonrpc` must be \"2.0\""));
    }
    let Some(Value::String(method)) = object.remove("method") else {
        return Err(invalid("`method` must be a string"));
    };
    let params = match object.remove("params") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(params)) => params,
        Some(_) => {
            return Err((
                id,
                RpcError::invalid_params("`params` must be an object (named parameters)"),
            ));
        }
    };
    Ok(Request { id, method, params })
}

/// 成功の応答。
pub fn result(id: &Id, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

/// 誤りの応答 (`id` が分からなければ `null`)。
pub fn error(id: Option<&Id>, error: &RpcError) -> String {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(&Value::Null), "error": error }).to_string()
}

/// 通知 (スタジオからクライアントへ)。
pub fn notification(method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_requests_and_notifications() {
        let r = parse(r#"{"jsonrpc":"2.0","id":1,"method":"scene.list"}"#).unwrap();
        assert_eq!(r.id, Some(json!(1)));
        assert_eq!(r.method, "scene.list");
        assert!(r.params.is_empty());
        let r = parse(r#"{"jsonrpc":"2.0","method":"motion.stop","params":{"a":1}}"#).unwrap();
        assert_eq!(r.id, None);
        assert_eq!(r.params["a"], 1);
        let r = parse(r#"{"jsonrpc":"2.0","id":"x","method":"m","params":null}"#).unwrap();
        assert_eq!(r.id, Some(json!("x")));
    }

    #[test]
    fn reports_malformed_messages() {
        let code = |text: &str| parse(text).unwrap_err();
        assert_eq!(code("{").1.code, PARSE_ERROR);
        assert_eq!(code("[]").1.code, INVALID_REQUEST);
        assert_eq!(code("1").1.code, INVALID_REQUEST);
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":{},"method":"m"}"#).1.code,
            INVALID_REQUEST
        );
        let (id, e) = code(r#"{"jsonrpc":"1.0","id":7,"method":"m"}"#);
        assert_eq!((id, e.code), (Some(json!(7)), INVALID_REQUEST));
        assert_eq!(code(r#"{"jsonrpc":"2.0","id":7}"#).1.code, INVALID_REQUEST);
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":7,"method":"m","params":[1]}"#)
                .1
                .code,
            INVALID_PARAMS
        );
    }

    /// `-32010` の `data` は元のフィールドをそのまま持ち、ADR 0029 のコードと引数を足す (T0839)。
    #[test]
    fn failed_keeps_the_fields_and_adds_the_message_code_and_args() {
        use crate::animation::AnimationError;
        use crate::stage::{Asset, SceneError};
        use std::path::PathBuf;

        let error = RpcError::failed(&SceneError::Missing {
            asset: Asset::Module,
            path: PathBuf::from("/m/hat.ugoemod"),
        });
        assert_eq!(error.code, OPERATION_FAILED);
        assert_eq!(error.message, "/m/hat.ugoemod: not found");
        assert_eq!(
            error.data,
            Some(json!({
                "code": "missing",
                "asset": "module",
                "path": "/m/hat.ugoemod",
                "messageCode": "scene.module_missing",
                "args": { "asset": "module", "path": "/m/hat.ugoemod" },
            }))
        );
        assert_eq!(
            RpcError::failed(&SceneError::UnknownScene { id: "s".into() }).data,
            Some(json!({
                "code": "unknownScene",
                "id": "s",
                "messageCode": "scene.unknown",
                "args": { "id": "s" },
            }))
        );
        // 引数の無いエラーも `args` を (空で) 持つ。
        assert_eq!(
            RpcError::failed(&AnimationError::NoModel).data,
            Some(json!({ "code": "noModel", "messageCode": "animation.no_model", "args": {} }))
        );
        assert_eq!(
            RpcError::failed(&AnimationError::UnknownMotion {
                name: "Wave".into()
            })
            .data,
            Some(json!({
                "code": "unknownMotion",
                "name": "Wave",
                "messageCode": "animation.unknown_motion",
                "args": { "name": "Wave" },
            }))
        );
    }

    #[test]
    fn writes_responses() {
        let read = |text: String| serde_json::from_str::<Value>(&text).unwrap();
        assert_eq!(
            read(result(&json!(1), json!({"ok":true}))),
            json!({"jsonrpc":"2.0","id":1,"result":{"ok":true}})
        );
        assert_eq!(
            read(error(None, &RpcError::new(PARSE_ERROR, "x"))),
            json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"x"}})
        );
        assert_eq!(
            read(notification("scene.changed", json!({}))),
            json!({"jsonrpc":"2.0","method":"scene.changed","params":{}})
        );
    }
}
