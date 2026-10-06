//! ブラウザソース (T0808、`docs/studio.md#ブラウザソース`): OBS のブラウザソースに URL を入れるだけでアバターを取り込めるよう、
//! 操作 API と同じ待ち受け ([`super::server`]) に `/source/<トークン>/…` の経路を足す (ADR 0028 の 5.・6.)。
//!
//! | 経路 (`/source/<トークン>/` の下) | 中身 |
//! | --- | --- |
//! | (空) | 受け手のページ (`source.html`。出力ウィンドウと同じ `Output.svelte` を WebSocket で動かす) |
//! | `assets/…` | ページのスクリプト・スタイル・wasm (ビルドした `dist/assets/`) |
//! | `frames` | WebSocket: シーンの通知 (テキストの JSON) とフレーム (バイナリ) を配る。ページは描画の負荷と読めなかったこと (テキストの JSON) を返す |
//! | `scene/<revision>/model`・`scene/<revision>/module/<index>`・`scene/<revision>/background` | 今のシーンのファイル |
//!
//! - **スイッチとトークンは操作 API とは別** (`studio-api.json` の `source`。既定は切)。切っているときとトークンが違うときは、
//!   どちらも `404` にする (URL が正しいかを当てさせない)。トークンは定数時間で比べる。
//! - **読むだけ。** ページとファイルとフレームを受け取れるだけで、操作はできない。ファイルは今のシーンが使うものだけを出す。
//! - Host・転送ヘッダ・Origin の検査は経路に依らず [`super::http::check_request`] が先に行う。ページの WebSocket の `Origin` は
//!   スタジオ自身の `http://127.0.0.1:<port>` なので通る (ほかの Web ページからは接続できない)。
//! - 応答には `Referrer-Policy: no-referrer` (URL のトークンを外へ出さない) と、外のものを読まない CSP を付ける。

use std::collections::HashMap;
use std::io::{BufReader, ErrorKind, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use tungstenite::Bytes;
use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{Message, Role, WebSocket};

use super::http::{self, Reject, RequestHead};
use super::server::{Keepalive, Liveness};
use super::{store, ws};
use crate::error::CommandError;
use crate::load_failure::LoadFailure;
use crate::{RenderStats, Reported, Studio, lock};

/// 経路の頭。
pub const PREFIX: &str = "/source/";
/// ページの HTML (ビルドした `dist/` の中の名前。`apps/studio/vite.config.ts` の入口)。
pub const PAGE: &str = "source.html";
/// 同時に保てるページの接続の数の上限。
pub const MAX_CONNECTIONS: usize = 8;
/// 1 つの接続に送らずに溜めておけるメッセージの数 (60 刻みで約 4 秒)。溢れたら接続を閉じる (ページはつなぎ直す)。
pub const QUEUE: usize = 256;
/// 負荷の表示 (`load_panel`) に使うラベルの頭 (2 つ目からは `browserSource:2` のように番号を付ける)。
pub const LABEL: &str = "browserSource";
/// WebSocket の close の符号: ブラウザソースを切った・トークンを作り直した (1001 Going Away)。
pub const CLOSE_GOING_AWAY: u16 = 1001;

/// 送るものが無いときに、届いたものを見に行く間隔。
const POLL: Duration = Duration::from_millis(50);

/// ページの応答に付ける CSP。ページは自分のスクリプト・wasm・スタイルと、背景の画像の blob URL だけを使う。
const CSP: &str = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; \
                   img-src 'self' blob: data:; connect-src 'self' ws://127.0.0.1:* ws://localhost:*; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// ページの接続へ送るもの。
#[derive(Debug, Clone)]
pub enum Push {
    /// シーンの通知 (JSON) かフレームのバイト列。
    Message(Message),
    /// 閉じる (WebSocket の close の符号と理由)。
    Close(u16, &'static str),
}

/// ビルドしたページのファイルを読む口 (Tauri の `AssetResolver`。テストでは置き換える)。パスは `dist/` からの相対。
type AssetReader = Box<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>;

/// ブラウザソースのページの接続 (`Studio` が持つ)。
#[derive(Default)]
pub struct Sources {
    state: Mutex<SourcesState>,
    assets: OnceLock<AssetReader>,
}

#[derive(Debug, Default)]
struct SourcesState {
    connections: HashMap<u64, SourceConnection>,
    next: u64,
}

#[derive(Debug)]
struct SourceConnection {
    sender: SyncSender<Push>,
    /// 負荷の表示のラベル。
    label: String,
}

impl std::fmt::Debug for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field("state", &self.state)
            .field("assets", &self.assets.get().is_some())
            .finish()
    }
}

impl Sources {
    /// ページのファイルを読む口を置く (起動時に 1 回)。
    pub fn set_assets(&self, reader: impl Fn(&str) -> Option<Vec<u8>> + Send + Sync + 'static) {
        let _ = self.assets.set(Box::new(reader));
    }

    /// いまの接続の数。
    pub fn len(&self) -> usize {
        lock(&self.state).connections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// すべての接続へ送る。溜まりすぎた (読まれていない) 接続は外す (送る口を落とすので、溜まった分を送ってから閉じる)。
    /// 外した接続の負荷の表示のラベルを返す。
    fn send_all(&self, push: &Push) -> Vec<String> {
        let mut removed = Vec::new();
        lock(&self.state)
            .connections
            .retain(|_, conn| match conn.sender.try_send(push.clone()) {
                Ok(()) => true,
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    removed.push(conn.label.clone());
                    false
                }
            });
        removed
    }
}

/// ページが返すもの (`apps/studio/src/lib/feed.ts` の `sourceFeed`)。
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Incoming {
    /// 描画の負荷。
    #[serde(rename = "renderStats")]
    RenderStats(RenderStats),
    /// 今のシーンのファイルを読めなかった (T0825)。
    #[serde(rename = "loadFailure")]
    LoadFailure(LoadFailure),
}

impl Studio {
    // --- 配る ---

    /// フレームのバイト列を配る (刻みのスレッドが指揮役を握ったまま呼ぶ。シーンの通知より後に届くように)。
    pub(crate) fn source_frame(&self, bytes: Vec<u8>) {
        self.source_push(&Push::Message(Message::Binary(Bytes::from(bytes))));
    }

    /// シーンの通知 (JSON) を配る。
    pub(crate) fn source_scene(&self, json: &str) {
        self.source_push(&Push::Message(Message::text(json)));
    }

    fn source_push(&self, push: &Push) {
        if self.sources.is_empty() {
            return;
        }
        let removed = self.sources.send_all(push);
        if !removed.is_empty() {
            let mut renders = lock(&self.renders);
            for label in &removed {
                renders.remove(label);
            }
            drop(renders);
            for label in &removed {
                self.forget_load_failure(label);
            }
            self.send_api_view();
        }
    }

    /// ページの接続をすべて閉じる (ブラウザソースを切った・トークンを作り直した・ポートを変えた)。
    pub(crate) fn close_sources(&self, reason: &'static str) {
        let closed: Vec<SourceConnection> = lock(&self.sources.state)
            .connections
            .drain()
            .map(|(_, c)| c)
            .collect();
        if closed.is_empty() {
            return;
        }
        let mut renders = lock(&self.renders);
        for conn in &closed {
            // 溢れていれば close は届かないが、送る口が落ちるので接続は閉じる。
            let _ = conn.sender.try_send(Push::Close(CLOSE_GOING_AWAY, reason));
            renders.remove(&conn.label);
        }
        drop(renders);
        for conn in &closed {
            self.forget_load_failure(&conn.label);
        }
        drop(closed);
        self.send_api_view();
    }

    // --- 接続 ---

    /// ページの接続を名簿に入れ、今のシーンを最初に送る。多すぎれば `None`。
    ///
    /// 指揮役を握ったまま入れるので、シーンより先にそのシーンのフレームが届くことは無い (`subscribe_frames` と同じ)。
    pub(super) fn source_connect(&self, sender: SyncSender<Push>) -> Option<u64> {
        let conductor = lock(&self.conductor);
        let mut state = lock(&self.sources.state);
        if state.connections.len() >= MAX_CONNECTIONS {
            return None;
        }
        // ポーズの要素は指揮役が今表示しているもの (モーションが切り替えた後に開いたページも揃える。T0848)。
        if let Some(scene) = conductor.joining_scene()
            && let Ok(json) = serde_json::to_string(&scene)
        {
            let _ = sender.try_send(Push::Message(Message::text(json)));
        }
        let id = state.next;
        state.next += 1;
        // 負荷の表示のラベルは、空いている番号の小さいものから使う。
        let label = (1..)
            .map(|n| {
                if n == 1 {
                    LABEL.to_owned()
                } else {
                    format!("{LABEL}:{n}")
                }
            })
            .find(|l| state.connections.values().all(|c| &c.label != l))
            .unwrap_or_else(|| LABEL.to_owned());
        state
            .connections
            .insert(id, SourceConnection { sender, label });
        drop(state);
        drop(conductor);
        self.send_api_view();
        Some(id)
    }

    fn source_disconnect(&self, id: u64) {
        let removed = lock(&self.sources.state).connections.remove(&id);
        if let Some(conn) = removed {
            lock(&self.renders).remove(&conn.label);
            self.forget_load_failure(&conn.label);
            self.send_api_view();
        }
    }

    /// ページから届いたテキスト (描画の負荷・読めなかったこと)。読めないものは捨てる。
    fn source_receive(&self, id: u64, text: &str) {
        let Ok(incoming) = serde_json::from_str::<Incoming>(text) else {
            return;
        };
        let Some(label) = lock(&self.sources.state)
            .connections
            .get(&id)
            .map(|c| c.label.clone())
        else {
            return;
        };
        let stats = match incoming {
            Incoming::RenderStats(stats) if stats.is_valid() => stats,
            Incoming::RenderStats(_) => return,
            Incoming::LoadFailure(failure) => {
                self.report_load_failure(&label, failure);
                return;
            }
        };
        lock(&self.renders).insert(
            label,
            Reported {
                stats,
                target_fps: None,
                at: Instant::now(),
            },
        );
    }

    // --- 設定 ---

    /// ブラウザソースを出す・やめる (覚える。やめるとページの接続はすべて閉じる)。
    pub(crate) fn set_source_enabled(self: &Arc<Self>, enabled: bool) {
        {
            let mut state = lock(&self.api.0);
            if state.settings.source.enabled == enabled && (!enabled || state.server.is_some()) {
                return;
            }
            state.settings.source.enabled = enabled;
            state.save();
        }
        self.sync_listener(false);
    }

    /// OBS に入れる URL (`http://127.0.0.1:<port>/source/<トークン>/`)。
    pub(crate) fn source_url(&self) -> String {
        let state = lock(&self.api.0);
        source_url(state.settings.port, &state.settings.source.token)
    }

    /// トークンを作り直す (古い URL のページは閉じ、以後は `404`)。新しい URL を返す。
    pub(crate) fn regenerate_source_token(&self) -> Result<String, CommandError> {
        let token = store::generate_secret().map_err(CommandError::random_failed)?;
        let url = {
            let mut state = lock(&self.api.0);
            state.settings.source.token.clone_from(&token);
            state.save();
            source_url(state.settings.port, &token)
        };
        self.close_sources("the browser source URL was changed");
        Ok(url)
    }

    /// `token` がブラウザソースを出しているときの正しいトークンか (定数時間で比べる)。
    fn source_allows(&self, token: &str) -> bool {
        let state = lock(&self.api.0);
        let source = &state.settings.source;
        source.enabled && !source.token.is_empty() && store::constant_time_eq(token, &source.token)
    }
}

/// OBS に入れる URL。
pub fn source_url(port: u16, token: &str) -> String {
    format!("http://127.0.0.1:{port}{PREFIX}{token}/")
}

/// `/source/…` の要求 (`server.rs` の `route` から)。
pub(crate) fn serve(
    studio: &Arc<Studio>,
    head: &RequestHead,
    reader: BufReader<TcpStream>,
    mut writer: TcpStream,
) {
    let not_found = Reject::new(404, "not found");
    let rest = head.path().strip_prefix(PREFIX).unwrap_or_default();
    let (token, rest) = match rest.split_once('/') {
        Some((token, rest)) => (token, Some(rest)),
        None => (rest, None),
    };
    if !studio.source_allows(token) {
        let _ = http::write_reject(&mut writer, not_found, &[]);
        return;
    }
    let Some(rest) = rest else {
        // `/source/<トークン>` (末尾の `/` なし): ページの相対のパスが合うように `/` を付けた URL へ送る。
        let location = format!("{PREFIX}{token}/");
        let _ = write_redirect(&mut writer, &location);
        return;
    };
    if head.method != "GET" {
        let _ = http::write_reject(&mut writer, Reject::new(405, "method not allowed"), &[]);
        return;
    }
    if rest == "frames" {
        match ws::handshake(head) {
            Ok(response) => {
                if writer.write_all(response.as_bytes()).is_ok() && writer.flush().is_ok() {
                    websocket(studio, reader);
                }
            }
            Err(reject) => {
                let extra: &[(&str, &str)] = if reject.status == 426 {
                    &[("Sec-WebSocket-Version", "13"), ("Upgrade", "websocket")]
                } else {
                    &[]
                };
                let _ = http::write_reject(&mut writer, reject, extra);
            }
        }
        return;
    }
    let body = match file(studio, rest) {
        Ok(body) => body,
        Err(reject) => {
            let _ = http::write_reject(&mut writer, reject, &[]);
            return;
        }
    };
    let _ = write_body(&mut writer, &body.0, body.1);
}

/// 経路 `rest` (トークンの後) のファイルと、その `Content-Type`。
fn file(studio: &Studio, rest: &str) -> Result<(Arc<Vec<u8>>, &'static str), Reject> {
    const NOT_FOUND: Reject = Reject::new(404, "not found");
    if let Some(scene) = rest.strip_prefix("scene/") {
        let mut parts = scene.split('/');
        let revision: u32 = parts.next().and_then(|r| r.parse().ok()).ok_or(NOT_FOUND)?;
        let bytes = match (parts.next(), parts.next(), parts.next()) {
            (Some("model"), None, None) => {
                studio.scene_bytes(revision, crate::stage::Stage::model_bytes)
            }
            (Some("module"), Some(index), None) => {
                let index: usize = index.parse().map_err(|_| NOT_FOUND)?;
                studio.scene_bytes(revision, |stage| stage.module_bytes(index))
            }
            (Some("background"), None, None) => {
                studio.scene_bytes(revision, crate::stage::Stage::background_image)
            }
            _ => return Err(NOT_FOUND),
        };
        // 今のシーンでなければ (もう次のシーンになった) 404。ページは次のシーンの通知を待つ。
        return bytes
            .map(|b| (b, "application/octet-stream"))
            .map_err(|_| NOT_FOUND);
    }
    let path = if rest.is_empty() {
        PAGE
    } else if is_asset_path(rest) {
        rest
    } else {
        return Err(NOT_FOUND);
    };
    let reader = studio.sources.assets.get().ok_or(NOT_FOUND)?;
    let bytes = reader(path).ok_or(NOT_FOUND)?;
    Ok((Arc::new(bytes), content_type(path)))
}

/// ページのファイルとして出してよいパス (`assets/` の下の、`..` などを含まない名前)。
fn is_asset_path(path: &str) -> bool {
    let Some(name) = path.strip_prefix("assets/") else {
        return false;
    };
    !name.is_empty()
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}

/// 拡張子から `Content-Type` を決める (ビルドしたページが使う種類だけ)。
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json" | "map") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// 共通のヘッダ。
fn common_headers() -> String {
    format!(
        "Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n\
         Content-Security-Policy: {CSP}\r\nConnection: close\r\n"
    )
}

fn write_body(writer: &mut impl Write, body: &[u8], content_type: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{}\r\n",
        body.len(),
        common_headers()
    );
    writer.write_all(head.as_bytes())?;
    writer.write_all(body)?;
    writer.flush()
}

fn write_redirect(writer: &mut impl Write, location: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Length: 0\r\n{}\r\n",
        common_headers()
    );
    writer.write_all(head.as_bytes())?;
    writer.flush()
}

/// ページの WebSocket を名簿に入れ、閉じるまで回す。
///
/// 送るもの (シーンとフレーム) は刻みのたびに届くので、列を待つのを主にし、その合間に届いたもの (描画の負荷・pong・close) を
/// 非ブロッキングで読む (`server.rs` の操作 API は読むのを主にしている。こちらは送る量が多く、遅れを小さくしたい)。
fn websocket(studio: &Arc<Studio>, reader: BufReader<TcpStream>) {
    let part = reader.buffer().to_vec();
    let stream = reader.into_inner();
    let mut socket = WebSocket::from_partially_read(stream, part, Role::Server, Some(ws::config()));
    let (sender, receiver) = mpsc::sync_channel::<Push>(QUEUE);
    let Some(id) = studio.source_connect(sender) else {
        close(
            &mut socket,
            ws::close::TRY_AGAIN_LATER,
            "too many connections",
        );
        return;
    };
    // 何も届かなければ ping を送り、pong も届かなければ切る (フレームを送り続けていても。描画の負荷は隠れると届かない)。
    let mut keepalive = Keepalive::new(Instant::now());
    'serve: loop {
        let first = match receiver.recv_timeout(POLL) {
            Ok(push) => Some(push),
            Err(RecvTimeoutError::Timeout) => None,
            // 名簿から外された (溢れた)。
            Err(RecvTimeoutError::Disconnected) => break,
        };
        for push in first
            .into_iter()
            .chain(std::iter::from_fn(|| receiver.try_recv().ok()))
        {
            match push {
                Push::Message(message) => {
                    if socket.write(message).is_err() {
                        break 'serve;
                    }
                }
                Push::Close(code, reason) => {
                    close(&mut socket, code, reason);
                    break 'serve;
                }
            }
        }
        if socket.flush().is_err() {
            break;
        }
        // 届いたものを読む (待たない)。
        if socket.get_ref().set_nonblocking(true).is_err() {
            break;
        }
        let read = loop {
            match socket.read() {
                Ok(Message::Text(text)) => {
                    keepalive.received(Instant::now());
                    studio.source_receive(id, text.as_str());
                }
                Ok(Message::Binary(_)) => {
                    close(
                        &mut socket,
                        ws::close::UNSUPPORTED_DATA,
                        "binary messages are not accepted",
                    );
                    break false;
                }
                Ok(_) => keepalive.received(Instant::now()),
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => break true,
                Err(_) => break false,
            }
        };
        if !read || socket.get_ref().set_nonblocking(false).is_err() {
            break;
        }
        match keepalive.check(Instant::now()) {
            Liveness::Alive => {}
            Liveness::Ping => {
                if socket.send(Message::Ping(Bytes::new())).is_err() {
                    break;
                }
            }
            Liveness::Dead => break,
        }
    }
    studio.source_disconnect(id);
    let _ = socket.get_ref().shutdown(Shutdown::Both);
}

/// close を送る (送れなくても続ける)。
fn close(socket: &mut WebSocket<TcpStream>, code: u16, reason: &str) {
    let _ = socket.get_ref().set_nonblocking(false);
    let frame = CloseFrame {
        code: CloseCode::from(code),
        reason: reason.to_owned().into(),
    };
    let _ = socket.close(Some(frame));
    let _ = socket.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_asset_paths_are_served() {
        assert!(is_asset_path("assets/source-AbC_1.js"));
        assert!(is_asset_path("assets/ugoe_wasm_bg-1a2b.wasm"));
        for bad in [
            "assets/",
            "assets/../index.html",
            "assets/./x.js",
            "assets//x.js",
            "assets/x%2e.js",
            "index.html",
            "output.html",
            "../assets/x.js",
            "assets/x\\..\\y.js",
        ] {
            assert!(!is_asset_path(bad), "{bad}");
        }
    }

    #[test]
    fn content_types_follow_the_extension() {
        assert_eq!(content_type("source.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("assets/a.wasm"), "application/wasm");
        assert_eq!(
            content_type("assets/a.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("assets/a"), "application/octet-stream");
    }

    #[test]
    fn the_url_names_the_loopback_address_and_the_token() {
        assert_eq!(
            source_url(28451, "abc"),
            "http://127.0.0.1:28451/source/abc/"
        );
    }
}
