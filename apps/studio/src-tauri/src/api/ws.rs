//! WebSocket (RFC 6455) のサーバの側。
//!
//! HTTP の要求の読み取りと検査 (Host・転送ヘッダ・Origin) と経路は自前 ([`super::http`]・[`super::server`]) で行い、
//! Upgrade の要求のときだけ、握手の応答の鍵とフレームの読み書きを tungstenite (同期版) に任せる
//! (`derive_accept_key` と `WebSocket::from_raw_socket`。利用者の決定、2026-10-02)。
//! 拡張 (圧縮など) とサブプロトコルは取り決めない。メッセージとフレームの大きさは [`config`] で小さく抑える。

use tungstenite::handshake::derive_accept_key;
use tungstenite::protocol::WebSocketConfig;

use super::http::{Reject, RequestHead};

/// 1 つのメッセージの大きさの上限 (バイト。分割されたものは合わせた大きさ)。
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
/// 1 つのフレームの大きさの上限 (バイト)。
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// close の符号 (RFC 6455 7.4.1)。
pub mod close {
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const INVALID_DATA: u16 = 1007;
    pub const TOO_BIG: u16 = 1009;
    pub const TRY_AGAIN_LATER: u16 = 1013;
}

/// 接続の設定 (大きさの上限)。
pub fn config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_FRAME_BYTES))
}

/// 握手の要求を確かめ、`101` の応答を返す。違えば断る理由 (`426` なら `Sec-WebSocket-Version: 13` を付けて返す)。
pub fn handshake(head: &RequestHead) -> Result<String, Reject> {
    if head.method != "GET" {
        return Err(Reject::new(
            405,
            "the API accepts WebSocket upgrades (GET) only",
        ));
    }
    if !head.header_has_token("upgrade", "websocket")
        || !head.header_has_token("connection", "upgrade")
    {
        return Err(Reject::new(426, "connect with a WebSocket client"));
    }
    if head.header("sec-websocket-version") != Some("13") {
        return Err(Reject::new(426, "unsupported WebSocket version (use 13)"));
    }
    let key = head
        .header("sec-websocket-key")
        .ok_or(Reject::new(400, "missing Sec-WebSocket-Key"))?;
    // 16 バイトを base64 にしたもの (24 文字で `==` で終わる。RFC 6455 4.1)。
    let base64 = |b: u8| b.is_ascii_alphanumeric() || b == b'+' || b == b'/';
    let bytes = key.as_bytes();
    if bytes.len() != 24 || !bytes[..22].iter().all(|b| base64(*b)) || !key.ends_with("==") {
        return Err(Reject::new(400, "malformed Sec-WebSocket-Key"));
    }
    Ok(format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        derive_accept_key(key.as_bytes())
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::api::http::read_head;

    /// RFC 6455 1.3 の例。
    #[test]
    fn accepts_the_rfc_example_handshake() {
        let head = read_head(
            &mut "GET /api HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                  Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
                .as_bytes(),
        )
        .unwrap()
        .unwrap();
        let response = handshake(&head).unwrap();
        assert!(response.starts_with("HTTP/1.1 101 Switching Protocols\r\n"));
        assert!(response.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n"));
    }

    #[test]
    fn rejects_bad_handshakes() {
        let status = |text: &str| {
            let head = read_head(&mut text.as_bytes()).unwrap().unwrap();
            handshake(&head).map(|_| ()).map_err(|r| r.status)
        };
        let ok = "Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n";
        assert_eq!(
            status(&format!(
                "POST /api HTTP/1.1\r\n{ok}Sec-WebSocket-Version: 13\r\n\r\n"
            )),
            Err(405)
        );
        assert_eq!(status("GET /api HTTP/1.1\r\n\r\n"), Err(426));
        assert_eq!(
            status(&format!(
                "GET /api HTTP/1.1\r\n{ok}Sec-WebSocket-Version: 8\r\n\r\n"
            )),
            Err(426)
        );
        for key in ["c2hvcnQ=", "dGhlIHNhbXBsZSBub25jZQ!!", ""] {
            assert_eq!(
                status(&format!(
                    "GET /api HTTP/1.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                     Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
                )),
                Err(400),
                "{key}"
            );
        }
    }

    #[test]
    fn limits_are_small() {
        let config = config();
        assert_eq!(config.max_message_size, Some(MAX_MESSAGE_BYTES));
        assert_eq!(config.max_frame_size, Some(MAX_FRAME_BYTES));
    }
}
