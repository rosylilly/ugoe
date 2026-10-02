//! 待ち受け: ループバック (`127.0.0.1` と `::1`) にだけ束ね、接続ごとにスレッドで要求を読む (ADR 0028 の 6.)。
//!
//! 1 つの待ち受けで HTTP と WebSocket を扱う。経路は今は `/api` (操作 API の WebSocket) だけで、ブラウザソース (T0808) は
//! 同じ待ち受けに経路を足す ([`route`])。どの経路も、先に [`http::check_request`] (Host・転送ヘッダ・Origin) を通す。
//!
//! WebSocket のフレームは tungstenite に任せる ([`ws`])。送るものは [`Outgoing`] の列で渡すので、ほかのスレッド
//! (刻みのスレッドの通知・操作ウィンドウの承認) が接続を待たずに送れる。

use std::io::{BufReader, ErrorKind, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{Message, Role, WebSocket};

use super::Outgoing;
use super::http::{self, Reject};
use super::ws;
use crate::Studio;

/// 要求の頭を読み終えるまでの時間の上限。
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
/// 読むのを区切る時間 (この間隔で、ほかのスレッドからの通知を送る)。
const POLL: Duration = Duration::from_millis(20);
/// 何も送っていなければ ping を送る間隔。
const PING_INTERVAL: Duration = Duration::from_secs(20);
/// これだけ何も届かなければ (pong も) 切る。
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// 送るのに待てる時間 (クライアントが読まずに詰まったとき)。
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// 待ち受けている。止めるには [`Server::stop`]。
#[derive(Debug)]
pub struct Server {
    stopping: Arc<AtomicBool>,
    /// 束ねたアドレス (止めるときに自分に接続して accept を起こす)。
    addrs: Vec<SocketAddr>,
}

impl Server {
    /// `port` で待ち受け始める。`127.0.0.1` に束ねられなければ誤り (`::1` は束ねられなくてもよい)。
    pub(crate) fn start(studio: Arc<Studio>, port: u16) -> Result<Self, String> {
        let v4 = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .map_err(|e| format!("cannot listen on 127.0.0.1:{port}: {e}"))?;
        // `port` が 0 (テスト) なら OS が選んだポートを `::1` にも使う。
        let port = v4
            .local_addr()
            .map_err(|e| format!("cannot listen on port {port}: {e}"))?
            .port();
        let v6 = TcpListener::bind((Ipv6Addr::LOCALHOST, port)).ok();
        let stopping = Arc::new(AtomicBool::new(false));
        let mut addrs = Vec::new();
        for listener in std::iter::once(v4).chain(v6) {
            let addr = listener
                .local_addr()
                .map_err(|e| format!("cannot listen on port {port}: {e}"))?;
            addrs.push(addr);
            let studio = Arc::clone(&studio);
            let stopping = Arc::clone(&stopping);
            std::thread::Builder::new()
                .name(format!("ugoe-studio-api-{addr}"))
                .spawn(move || accept_loop(&listener, &studio, &stopping, port))
                .map_err(|e| e.to_string())?;
        }
        Ok(Self { stopping, addrs })
    }

    /// 束ねたアドレス。
    pub fn addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    /// 待ち受けをやめる (今の接続は [`super::Studio::restart_api`] が閉じる)。
    pub fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        for addr in &self.addrs {
            // accept で待っているスレッドを起こす。
            let _ = TcpStream::connect_timeout(addr, Duration::from_millis(200));
        }
    }
}

fn accept_loop(listener: &TcpListener, studio: &Arc<Studio>, stopping: &AtomicBool, port: u16) {
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else {
            continue;
        };
        let studio = Arc::clone(studio);
        let _ = std::thread::Builder::new()
            .name("ugoe-studio-api-conn".into())
            .spawn(move || serve(&studio, stream, port));
    }
}

/// 1 つの接続: 要求の頭を読み、検査して経路に渡す。
fn serve(studio: &Arc<Studio>, stream: TcpStream, port: u16) {
    // ループバックにだけ束ねているので、ほかの相手からは届かないはず (念のため)。
    if !stream.peer_addr().is_ok_and(|a| a.ip().is_loopback()) {
        return;
    }
    let _ = stream.set_read_timeout(Some(HEAD_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let _ = stream.set_nodelay(true);
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut writer = stream;
    let head = match http::read_head(&mut reader) {
        Ok(Some(head)) => head,
        Ok(None) | Err(http::ReadError::Io(_)) => return,
        Err(http::ReadError::Bad(reject)) => {
            let _ = http::write_reject(&mut writer, reject, &[]);
            return;
        }
    };
    if let Err(reject) = http::check_request(&head, port) {
        let _ = http::write_reject(&mut writer, reject, &[]);
        return;
    }
    route(studio, &head, reader, writer);
}

/// 経路ごとの処理。T0808 (ブラウザソース) はここに `/source/<token>/...` を足す。
fn route(
    studio: &Arc<Studio>,
    head: &http::RequestHead,
    reader: BufReader<TcpStream>,
    mut writer: TcpStream,
) {
    match head.path() {
        "/api" => match ws::handshake(head) {
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
        },
        _ => {
            let _ = http::write_reject(&mut writer, Reject::new(404, "not found"), &[]);
        }
    }
}

/// WebSocket の接続を名簿に入れ、閉じるまで回す。
///
/// tungstenite の `WebSocket` は読みと書きを分けられないので、1 つのスレッドで回す: 読むのを短い時間 ([`POLL`]) で区切り、
/// その合間に列 ([`Outgoing`]) のものを送る。ping への pong と、クライアントの close への応答は tungstenite が返す。
fn websocket(studio: &Arc<Studio>, reader: BufReader<TcpStream>) {
    // 握手の要求の後に読み込んでしまったバイト (普通は無い) も渡す。
    let part = reader.buffer().to_vec();
    let stream = reader.into_inner();
    let _ = stream.set_read_timeout(Some(POLL));
    let mut socket = WebSocket::from_partially_read(stream, part, Role::Server, Some(ws::config()));
    let (sender, receiver) = mpsc::channel::<Outgoing>();
    let Some(connection) = studio.api_connect(sender) else {
        close(
            &mut socket,
            ws::close::TRY_AGAIN_LATER,
            "too many connections",
        );
        return;
    };
    let mut received = Instant::now();
    let mut sent = Instant::now();
    'serve: loop {
        // 送るもの (応答・通知・閉じる)。
        loop {
            match receiver.try_recv() {
                Ok(Outgoing::Text(text)) => {
                    if socket.send(Message::text(text)).is_err() {
                        break 'serve;
                    }
                    sent = Instant::now();
                }
                Ok(Outgoing::Close(code, reason)) => {
                    close(&mut socket, code, reason);
                    break 'serve;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'serve,
            }
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                received = Instant::now();
                studio.api_receive(connection, text.as_str());
            }
            Ok(Message::Binary(_)) => {
                close(
                    &mut socket,
                    ws::close::UNSUPPORTED_DATA,
                    "binary messages are not supported",
                );
                break;
            }
            // ping への pong は tungstenite が返す。close への応答も返し、次の read で閉じたと分かる。
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_)) => {
                received = Instant::now();
            }
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                let now = Instant::now();
                if now.duration_since(received) > IDLE_TIMEOUT {
                    break;
                }
                if now.duration_since(sent) > PING_INTERVAL {
                    if socket.send(Message::Ping(Vec::new().into())).is_err() {
                        break;
                    }
                    sent = now;
                }
            }
            // 決まりに反するフレーム・大きすぎるメッセージ・UTF-8 でないテキストは、理由を付けて閉じる。
            Err(tungstenite::Error::Capacity(_)) => {
                close(&mut socket, ws::close::TOO_BIG, "message too large");
                break;
            }
            Err(tungstenite::Error::Protocol(_)) => {
                close(&mut socket, ws::close::PROTOCOL_ERROR, "protocol error");
                break;
            }
            Err(tungstenite::Error::Utf8(_)) => {
                close(
                    &mut socket,
                    ws::close::INVALID_DATA,
                    "text messages must be UTF-8",
                );
                break;
            }
            Err(_) => break,
        }
    }
    studio.api_disconnect(connection);
    let _ = socket.get_ref().shutdown(Shutdown::Both);
}

/// close を送る (送れなくても続ける)。
fn close(socket: &mut WebSocket<TcpStream>, code: u16, reason: &str) {
    let frame = CloseFrame {
        code: CloseCode::from(code),
        reason: reason.to_owned().into(),
    };
    let _ = socket.close(Some(frame));
    let _ = socket.flush();
}
