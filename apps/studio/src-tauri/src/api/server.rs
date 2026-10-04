//! 待ち受け: ループバック (`127.0.0.1` と `::1`) にだけ束ね、接続ごとにスレッドで要求を読む (ADR 0028 の 6.)。
//!
//! 1 つの待ち受けで HTTP と WebSocket を扱う。経路は `/api` (操作 API の WebSocket) と `/source/<トークン>/…`
//! (ブラウザソース、[`super::source`]、T0808) で (`route`)、どちらも有効にしたときだけ答える (切っていれば `404`)。
//! 待ち受けは、どちらかが有効な間だけ動かす (`Studio::sync_listener`)。
//! どの経路も、先に [`http::check_request`] (Host・転送ヘッダ・Origin) を通す。
//!
//! WebSocket のフレームは tungstenite に任せる ([`ws`])。送るものは [`Outgoing`] の列で渡すので、ほかのスレッド
//! (刻みのスレッドの通知・操作ウィンドウの承認) が接続を待たずに送れる。

use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{Message, Role, WebSocket};

use super::Outgoing;
use super::http::{self, Reject};
use super::{source, ws};
use crate::{Studio, lock};

/// 要求の頭を読み終えるまでの時間の上限 (接続してから。1 回の読み取りごとではなく全体)。
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
/// 要求の頭を読んでいる途中の接続の数の上限 (超えた接続は読まずに閉じる)。
///
/// 頭を読み終えて経路が決まるまでは、経路ごとの接続の上限 (操作 API の 32・ブラウザソースの 8) が効かないので、ここで抑える。
const MAX_READING_HEADS: usize = 64;
/// WebSocket をつないでから `session.hello` をするまでの時間の上限 (承認待ちになれば外す)。
const HELLO_TIMEOUT: Duration = Duration::from_secs(30);
/// 読むのを区切る時間 (この間隔で、ほかのスレッドからの通知を送る)。
const POLL: Duration = Duration::from_millis(20);
/// 何も届かなければ ping を送る間隔。
const PING_INTERVAL: Duration = Duration::from_secs(20);
/// これだけ何も届かなければ (pong も) 切る。
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// 接続が生きているかの確かめ (操作 API とブラウザソースで共通)。
///
/// 何も届かないまま [`PING_INTERVAL`] が過ぎたら ping を送り ([`PING_INTERVAL`] ごと)、[`IDLE_TIMEOUT`] が過ぎたら切る。
/// 送ったものではなく届いたもので数える (通知やフレームを送り続けていても、読むだけの相手には ping を送って pong を待つ)。
#[derive(Debug, Clone, Copy)]
pub(super) struct Keepalive {
    received: Instant,
    pinged: Instant,
}

/// [`Keepalive::check`] の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Liveness {
    Alive,
    /// ping を送る。
    Ping,
    /// 切る。
    Dead,
}

impl Keepalive {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            received: now,
            pinged: now,
        }
    }

    /// 何か (メッセージ・pong・ping) が届いた。
    pub(super) fn received(&mut self, now: Instant) {
        self.received = now;
    }

    /// 今どうするか。`Ping` を返したら、送ったものとして数える。
    pub(super) fn check(&mut self, now: Instant) -> Liveness {
        let quiet = now.saturating_duration_since(self.received);
        if quiet > IDLE_TIMEOUT {
            Liveness::Dead
        } else if quiet > PING_INTERVAL
            && now.saturating_duration_since(self.pinged) > PING_INTERVAL
        {
            self.pinged = now;
            Liveness::Ping
        } else {
            Liveness::Alive
        }
    }
}

/// 名乗るまでの期限 (操作 API)。名乗らずに ping に答え続ける接続が、接続の上限を埋めないようにする。
#[derive(Debug, Clone, Copy)]
pub(super) struct HelloDeadline {
    /// 期限 (名乗った・承認待ちになったと分かれば `None`)。
    until: Option<Instant>,
}

impl HelloDeadline {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            until: Some(now + HELLO_TIMEOUT),
        }
    }

    /// 期限を過ぎてもまだ名乗っていなければ `true` (閉じる)。
    ///
    /// `introduced` (名乗ったか。承認待ちも含む) は期限を過ぎたときだけ呼ぶ (名簿の錠を刻みごとに取らない)。
    /// 名乗っていれば期限を外す (名乗った接続が名乗る前に戻ることは無い)。
    pub(super) fn expired(&mut self, now: Instant, introduced: impl FnOnce() -> bool) -> bool {
        match self.until {
            Some(until) if now >= until => {
                if introduced() {
                    self.until = None;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

/// 要求の頭を読んでいる途中の接続の枠。落とすと返す。
#[derive(Debug)]
struct HeadSlot(Arc<AtomicUsize>);

impl HeadSlot {
    /// 枠を取る。`MAX_READING_HEADS` 個すべて使っていれば `None`。
    fn take(reading: &Arc<AtomicUsize>) -> Option<Self> {
        reading
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < MAX_READING_HEADS).then_some(n + 1)
            })
            .ok()
            .map(|_| Self(Arc::clone(reading)))
    }
}

impl Drop for HeadSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// 期限までに読む ([`http::read_head`] に渡す)。
///
/// 読み取りのたびに、ソケットの読み取りの時間の上限を期限までの残りにする (少しずつ送る相手も期限で切る)。
/// 期限を過ぎていれば読まずに `TimedOut`。
struct Deadlined<'a> {
    reader: &'a mut BufReader<TcpStream>,
    deadline: Instant,
}

impl Deadlined<'_> {
    fn arm(&self) -> io::Result<()> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                "request head took too long",
            ));
        }
        self.reader.get_ref().set_read_timeout(Some(left))
    }
}

impl Read for Deadlined<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.reader.buffer().is_empty() {
            self.arm()?;
        }
        self.reader.read(buf)
    }
}

impl BufRead for Deadlined<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.reader.buffer().is_empty() {
            self.arm()?;
        }
        self.reader.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.reader.consume(amount);
    }
}

/// 要求の頭を `deadline` までに読む。
fn read_head_until(
    reader: &mut BufReader<TcpStream>,
    deadline: Instant,
) -> Result<Option<http::RequestHead>, http::ReadError> {
    http::read_head(&mut Deadlined { reader, deadline })
}

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
        let reading = Arc::new(AtomicUsize::new(0));
        let mut addrs = Vec::new();
        for listener in std::iter::once(v4).chain(v6) {
            let addr = listener
                .local_addr()
                .map_err(|e| format!("cannot listen on port {port}: {e}"))?;
            addrs.push(addr);
            let studio = Arc::clone(&studio);
            let stopping = Arc::clone(&stopping);
            let reading = Arc::clone(&reading);
            std::thread::Builder::new()
                .name(format!("ugoe-studio-api-{addr}"))
                .spawn(move || accept_loop(&listener, &studio, &stopping, &reading, port))
                .map_err(|e| e.to_string())?;
        }
        Ok(Self { stopping, addrs })
    }

    /// 束ねたアドレス。
    pub fn addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    /// 待ち受けをやめる (今の接続は `Studio::sync_listener` が閉じる)。
    pub fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        for addr in &self.addrs {
            // accept で待っているスレッドを起こす。
            let _ = TcpStream::connect_timeout(addr, Duration::from_millis(200));
        }
    }
}

fn accept_loop(
    listener: &TcpListener,
    studio: &Arc<Studio>,
    stopping: &AtomicBool,
    reading: &Arc<AtomicUsize>,
    port: u16,
) {
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else {
            continue;
        };
        // 頭を読んでいる途中の接続が多すぎれば、スレッドを作らずに閉じる。
        let Some(slot) = HeadSlot::take(reading) else {
            continue;
        };
        let studio = Arc::clone(studio);
        let _ = std::thread::Builder::new()
            .name("ugoe-studio-api-conn".into())
            .spawn(move || serve(&studio, stream, slot, port));
    }
}

/// 1 つの接続: 要求の頭を読み、検査して経路に渡す。
fn serve(studio: &Arc<Studio>, stream: TcpStream, slot: HeadSlot, port: u16) {
    let deadline = Instant::now() + HEAD_TIMEOUT;
    // ループバックにだけ束ねているので、ほかの相手からは届かないはず (念のため)。
    if !stream.peer_addr().is_ok_and(|a| a.ip().is_loopback()) {
        return;
    }
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let _ = stream.set_nodelay(true);
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut writer = stream;
    let head = read_head_until(&mut reader, deadline);
    drop(slot);
    let head = match head {
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

/// 経路ごとの処理。
fn route(
    studio: &Arc<Studio>,
    head: &http::RequestHead,
    reader: BufReader<TcpStream>,
    mut writer: TcpStream,
) {
    let path = head.path();
    if path.starts_with(source::PREFIX) {
        source::serve(studio, head, reader, writer);
        return;
    }
    match path {
        "/api" if lock(&studio.api.0).settings.enabled => match ws::handshake(head) {
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
    let mut keepalive = Keepalive::new(Instant::now());
    let mut hello = HelloDeadline::new(Instant::now());
    'serve: loop {
        if hello.expired(Instant::now(), || studio.api_introduced(connection)) {
            close(
                &mut socket,
                super::CLOSE_HELLO_TIMEOUT,
                "session.hello timed out",
            );
            break;
        }
        // 送るもの (応答・通知・閉じる)。
        loop {
            match receiver.try_recv() {
                Ok(Outgoing::Text(text)) => {
                    if socket.send(Message::text(text)).is_err() {
                        break 'serve;
                    }
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
                keepalive.received(Instant::now());
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
                keepalive.received(Instant::now());
            }
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                match keepalive.check(Instant::now()) {
                    Liveness::Alive => {}
                    Liveness::Ping => {
                        if socket.send(Message::Ping(Vec::new().into())).is_err() {
                            break;
                        }
                    }
                    Liveness::Dead => break,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 送り続けていても、何も届かなければ ping を送り、それでも届かなければ切る (読むだけの相手を pong で確かめる)。
    #[test]
    fn keepalive_counts_what_arrives() {
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        let mut keepalive = Keepalive::new(start);
        assert_eq!(keepalive.check(at(10)), Liveness::Alive);
        assert_eq!(keepalive.check(at(21)), Liveness::Ping);
        // ping は間隔を空けて送る。
        assert_eq!(keepalive.check(at(30)), Liveness::Alive);
        assert_eq!(keepalive.check(at(42)), Liveness::Ping);
        // pong が届けば数え直す。
        keepalive.received(at(43));
        assert_eq!(keepalive.check(at(60)), Liveness::Alive);
        assert_eq!(keepalive.check(at(64)), Liveness::Ping);
        assert_eq!(keepalive.check(at(85)), Liveness::Ping);
        assert_eq!(keepalive.check(at(104)), Liveness::Dead);
    }

    /// 期限までに名乗らなければ閉じる。名乗った (承認待ちを含む) と分かれば、それ以後は確かめない。
    #[test]
    fn hello_deadline_closes_only_silent_connections() {
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        let unexpected = || -> bool { panic!("asked before the deadline") };

        let mut silent = HelloDeadline::new(start);
        assert!(!silent.expired(at(29), unexpected));
        assert!(silent.expired(at(30), || false));

        let mut introduced = HelloDeadline::new(start);
        assert!(!introduced.expired(at(31), || true));
        assert!(!introduced.expired(at(600), || panic!("asked again")));
    }

    /// 頭を読んでいる途中の接続の枠は上限まで。落とせば返る。
    #[test]
    fn head_slots_are_limited() {
        let reading = Arc::new(AtomicUsize::new(0));
        let mut slots: Vec<_> = (0..MAX_READING_HEADS)
            .map(|_| HeadSlot::take(&reading).unwrap())
            .collect();
        assert!(HeadSlot::take(&reading).is_none());
        slots.pop();
        assert!(HeadSlot::take(&reading).is_some());
        drop(slots);
        assert_eq!(reading.load(Ordering::SeqCst), 0);
    }

    /// 1 バイトずつ送り続けても、頭を読む全体の期限で切る (1 回の読み取りごとの上限だけでは切れない)。
    #[test]
    fn a_slow_request_head_hits_the_overall_deadline() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let sender = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            // 期限 (下の 300 ms) より長く、少しずつ送る。読む側が閉じたら書けなくなって終わる。
            for byte in b"GET /api HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Slow: "
                .iter()
                .cycle()
                .take(200)
            {
                if stream.write_all(&[*byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let start = Instant::now();
        let result = read_head_until(&mut reader, start + Duration::from_millis(300));
        let took = start.elapsed();
        assert!(
            matches!(&result, Err(http::ReadError::Io(e)) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)),
            "{result:?}"
        );
        assert!(took >= Duration::from_millis(300), "{took:?}");
        assert!(took < Duration::from_secs(1), "{took:?}");
        drop(reader);
        sender.join().unwrap();
    }

    /// 期限までに届いた頭は読める。
    #[test]
    fn a_request_head_within_the_deadline_is_read() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let sender = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            for part in [&b"GET /api HTTP/1.1\r\n"[..], b"Host: 127.0.0.1\r\n\r\n"] {
                stream.write_all(part).unwrap();
                std::thread::sleep(Duration::from_millis(20));
            }
            stream
        });
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let head = read_head_until(&mut reader, Instant::now() + Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(head.path(), "/api");
        drop(sender.join().unwrap());
    }
}
