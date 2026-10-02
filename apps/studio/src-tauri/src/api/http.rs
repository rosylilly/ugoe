//! 待ち受けの HTTP の部分: 要求の頭 (要求の行とヘッダ) を読み、ブラウザやトンネルからの悪用を防ぐ検査をする
//! (ADR 0028 の 6.、`crates/ugoe-mcp/src/http.rs` と同じ考え方)。
//!
//! 操作 API (`/api` の WebSocket) とブラウザソース (T0808) は 1 つの待ち受けを共有する。ここは経路に依らない検査と、
//! 応答を書く手順だけを持つ。

use std::io::{self, BufRead, Read, Write};
use std::net::IpAddr;

/// 要求の頭の大きさの上限 (バイト)。
const MAX_HEAD_BYTES: usize = 16 * 1024;
/// ヘッダの数の上限。
const MAX_HEADERS: usize = 64;

/// プロキシやトンネルが付ける転送のヘッダ。付いた要求は受けない (外から転送された要求とみなす)。
const FORWARDED_HEADERS: &[&str] = &[
    "forwarded",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-real-ip",
    "cf-connecting-ip",
    "true-client-ip",
];

/// 要求の頭。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RequestHead {
    pub method: String,
    /// 要求の対象 (`/api` など。クエリを含む)。
    pub target: String,
    /// ヘッダ (名前は小文字にする)。
    pub headers: Vec<(String, String)>,
}

impl RequestHead {
    /// ヘッダの値 (名前は小文字で渡す)。同じ名前が複数あれば最初のもの。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// 同じ名前のヘッダの数。
    pub fn header_count(&self, name: &str) -> usize {
        self.headers.iter().filter(|(n, _)| n == name).count()
    }

    /// クエリを除いた経路。
    pub fn path(&self) -> &str {
        self.target
            .split_once('?')
            .map_or(self.target.as_str(), |(p, _)| p)
    }

    /// カンマで区切ったヘッダの値に `token` があるか (大文字小文字を区別しない。`Connection: keep-alive, Upgrade` など)。
    pub fn header_has_token(&self, name: &str, token: &str) -> bool {
        self.headers
            .iter()
            .filter(|(n, _)| n == name)
            .flat_map(|(_, v)| v.split(','))
            .any(|t| t.trim().eq_ignore_ascii_case(token))
    }
}

/// 要求を受けない理由 (応答の状態と短い文)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reject {
    pub status: u16,
    pub message: &'static str,
}

impl Reject {
    pub const fn new(status: u16, message: &'static str) -> Self {
        Self { status, message }
    }
}

/// 読めなかった。
#[derive(Debug)]
pub enum ReadError {
    /// 要求の前に接続が閉じた・読めなかった (応答しない)。
    Io(io::Error),
    /// 形が違う (応答してから閉じる)。
    Bad(Reject),
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// 1 行を読む (`\r\n` か `\n` で終わる)。`budget` を超えたら 431。
fn read_line(reader: &mut impl BufRead, budget: usize) -> Result<Option<Vec<u8>>, ReadError> {
    let mut line = Vec::new();
    let n = reader
        .take(budget as u64 + 1)
        .read_until(b'\n', &mut line)?;
    if n == 0 {
        return Ok(None);
    }
    if !line.ends_with(b"\n") {
        return Err(ReadError::Bad(if n > budget {
            Reject::new(431, "request header too large")
        } else {
            Reject::new(400, "unexpected end of request")
        }));
    }
    line.pop();
    if line.ends_with(b"\r") {
        line.pop();
    }
    Ok(Some(line))
}

/// 要求の頭を読む。接続が要求の前で閉じていれば `None`。本文は読まない (`GET` だけを扱う)。
pub fn read_head(reader: &mut impl BufRead) -> Result<Option<RequestHead>, ReadError> {
    const BAD: ReadError = ReadError::Bad(Reject::new(400, "malformed request head"));
    let mut budget = MAX_HEAD_BYTES;
    let request_line = loop {
        let Some(line) = read_line(reader, budget)? else {
            return Ok(None);
        };
        budget = budget.saturating_sub(line.len() + 2);
        if !line.is_empty() {
            break String::from_utf8(line).map_err(|_| BAD)?;
        }
    };
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(BAD);
    };
    if version != "HTTP/1.1" {
        return Err(ReadError::Bad(Reject::new(
            505,
            "HTTP version not supported",
        )));
    }
    if !target.starts_with('/') {
        return Err(BAD);
    }
    let mut head = RequestHead {
        method: method.to_owned(),
        target: target.to_owned(),
        headers: Vec::new(),
    };
    loop {
        let line = read_line(reader, budget)?.ok_or(ReadError::Bad(Reject::new(
            400,
            "unexpected end of request",
        )))?;
        budget = budget.saturating_sub(line.len() + 2);
        if line.is_empty() {
            break;
        }
        if head.headers.len() >= MAX_HEADERS {
            return Err(ReadError::Bad(Reject::new(431, "too many header fields")));
        }
        let line = String::from_utf8(line).map_err(|_| BAD)?;
        // 折り返し (obs-fold) は受けない (RFC 9112 5.2)。
        let (name, value) = line.split_once(':').ok_or(BAD)?;
        if name.is_empty() || name.contains([' ', '\t']) {
            return Err(BAD);
        }
        head.headers.push((
            name.to_ascii_lowercase(),
            value.trim_matches([' ', '\t']).to_owned(),
        ));
    }
    Ok(Some(head))
}

/// `host[:port]` のホストがループバック (`localhost`・`127.0.0.0/8`・`::1`) か。
pub fn is_loopback_host(authority: &str) -> bool {
    if let Ok(ip) = authority.parse::<IpAddr>() {
        return ip.is_loopback();
    }
    let host = if let Some(rest) = authority.strip_prefix('[') {
        // `[::1]:28451`
        rest.split_once(']').map_or("", |(host, _)| host)
    } else {
        authority
            .split_once(':')
            .map_or(authority, |(host, _)| host)
    };
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// スタジオが出すページ自身の `Origin` (`http://127.0.0.1:<port>`・`http://localhost:<port>`・`http://[::1]:<port>`) か。
pub fn is_own_origin(origin: &str, port: u16) -> bool {
    let Some(authority) = origin.strip_prefix("http://") else {
        return false;
    };
    ["127.0.0.1", "localhost", "[::1]"]
        .iter()
        .any(|host| authority.eq_ignore_ascii_case(&format!("{host}:{port}")))
}

/// 経路に依らない検査 (ADR 0028 の 6.):
///
/// - `Host` が 1 つで、ループバックの名前であること (DNS リバインディングで別の名前から届いた要求を断る)。
/// - プロキシ・トンネルの転送ヘッダが付いていないこと。
/// - `Origin` が付いていれば、スタジオが出すページ自身の `Origin` であること (開いた Web ページからの接続を断る)。
///   `Origin: null` (ファイルや sandbox のページ) も断る。
pub fn check_request(head: &RequestHead, port: u16) -> Result<(), Reject> {
    const FORBIDDEN_HOST: Reject =
        Reject::new(403, "the Host header must name the loopback interface");
    if head.header_count("host") != 1 {
        return Err(FORBIDDEN_HOST);
    }
    if !head.header("host").is_some_and(is_loopback_host) {
        return Err(FORBIDDEN_HOST);
    }
    if FORWARDED_HEADERS
        .iter()
        .any(|name| head.header(name).is_some())
    {
        return Err(Reject::new(403, "forwarded requests are not accepted"));
    }
    if head.header_count("origin") > 1 {
        return Err(Reject::new(403, "origin not allowed"));
    }
    if let Some(origin) = head.header("origin")
        && !is_own_origin(origin, port)
    {
        return Err(Reject::new(403, "origin not allowed"));
    }
    Ok(())
}

/// 状態の文。
fn reason(status: u16) -> &'static str {
    match status {
        101 => "Switching Protocols",
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        426 => "Upgrade Required",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "Error",
    }
}

/// 短い文の応答を書いて、接続を閉じることを伝える。
pub fn write_reject(
    writer: &mut impl Write,
    reject: Reject,
    extra: &[(&str, &str)],
) -> io::Result<()> {
    let body = format!("{}\n", reject.message);
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
         Connection: close\r\nCache-Control: no-store\r\n",
        reject.status,
        reason(reject.status),
        body.len()
    );
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    writer.write_all(head.as_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn head(text: &str) -> RequestHead {
        read_head(&mut text.as_bytes()).unwrap().unwrap()
    }

    #[test]
    fn reads_the_request_head() {
        let h = head(
            "\r\nGET /api?x=1 HTTP/1.1\r\nHost: 127.0.0.1:28451\r\nConnection: keep-alive, Upgrade\r\n\
             Upgrade: websocket\r\n\r\nignored body",
        );
        assert_eq!(h.method, "GET");
        assert_eq!(h.path(), "/api");
        assert_eq!(h.header("host"), Some("127.0.0.1:28451"));
        assert!(h.header_has_token("connection", "upgrade"));
        assert!(!h.header_has_token("connection", "close"));
        assert!(read_head(&mut "".as_bytes()).unwrap().is_none());
    }

    #[test]
    fn rejects_malformed_heads() {
        for text in [
            "GET /api HTTP/1.0\r\n\r\n",
            "GET /api\r\n\r\n",
            "GET api HTTP/1.1\r\n\r\n",
            "GET /api HTTP/1.1\r\nBad Header: x\r\n\r\n",
            "GET /api HTTP/1.1\r\n folded\r\n\r\n",
            "GET /api HTTP/1.1\r\nHost: a\r\n",
        ] {
            assert!(
                matches!(read_head(&mut text.as_bytes()), Err(ReadError::Bad(_))),
                "{text:?}"
            );
        }
        let huge = format!(
            "GET /api HTTP/1.1\r\nX: {}\r\n\r\n",
            "a".repeat(MAX_HEAD_BYTES)
        );
        assert!(matches!(
            read_head(&mut huge.as_bytes()),
            Err(ReadError::Bad(Reject { status: 431, .. }))
        ));
    }

    #[test]
    fn only_loopback_hosts_and_own_origins_pass() {
        let ok =
            |extra: &str| check_request(&head(&format!("GET /api HTTP/1.1\r\n{extra}\r\n")), 28451);
        assert!(ok("Host: 127.0.0.1:28451\r\n").is_ok());
        assert!(ok("Host: localhost:28451\r\n").is_ok());
        assert!(ok("Host: [::1]:28451\r\n").is_ok());
        assert!(ok("Host: 127.0.0.1:28451\r\nOrigin: http://127.0.0.1:28451\r\n").is_ok());
        assert!(ok("Host: localhost:28451\r\nOrigin: http://LOCALHOST:28451\r\n").is_ok());
        for bad in [
            "",
            "Host: evil.example:28451\r\n",
            "Host: 127.0.0.1.evil.example\r\n",
            "Host: 127.0.0.1:28451\r\nHost: evil.example\r\n",
            "Host: 127.0.0.1:28451\r\nX-Forwarded-For: 1.2.3.4\r\n",
            "Host: 127.0.0.1:28451\r\nForwarded: for=1.2.3.4\r\n",
            "Host: 127.0.0.1:28451\r\nOrigin: https://evil.example\r\n",
            "Host: 127.0.0.1:28451\r\nOrigin: http://127.0.0.1:9999\r\n",
            "Host: 127.0.0.1:28451\r\nOrigin: null\r\n",
            "Host: 127.0.0.1:28451\r\nOrigin: http://127.0.0.1:28451\r\nOrigin: https://evil.example\r\n",
        ] {
            assert_eq!(ok(bad).map_err(|r| r.status), Err(403), "{bad:?}");
        }
    }

    #[test]
    fn writes_a_closing_response() {
        let mut out = Vec::new();
        write_reject(
            &mut out,
            Reject::new(403, "nope"),
            &[("Sec-WebSocket-Version", "13")],
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.contains("Sec-WebSocket-Version: 13\r\n"));
        assert!(text.ends_with("\r\n\r\nnope\n"));
    }
}
