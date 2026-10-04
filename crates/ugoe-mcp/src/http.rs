//! HTTP のトランスポート (MCP の Streamable HTTP。T0514)。
//!
//! 仕様: https://modelcontextprotocol.io/specification/draft/basic/transports/streamable-http
//! (2026-07-28) と https://modelcontextprotocol.io/specification/2025-11-25/basic/transports
//! (`initialize` の握手をする版)。
//!
//! - エンドポイントは 1 つ (`/mcp`)。要求は 1 つずつ POST で受け、応答は JSON 1 つで返す
//!   (`application/json`)。ツールは同期で進み、途中の通知を送らないので SSE は使わない。
//! - 取り消し: 答えを返す前にクライアントが接続を閉じたら、その要求を取り消す (2026-07-28 の
//!   Streamable HTTP の取り消し。確認を待つ適用などが拒否で終わる。`crate::host` の「取り消し」)。
//!   旧来の版は接続の切断を取り消しとみなさないよう勧めるが、このサーバは SSE で答えを送り直さない
//!   ので、去ったクライアントの答えは誰にも届かない。`notifications/cancelled` の POST も受ける
//!   ([`Server`] が処理中の要求の id で探す)。
//! - 通知と応答の POST には `202 Accepted` を返す。GET (サーバからの SSE) と DELETE (セッションの
//!   終了) には `405` を返す。セッション (`Mcp-Session-Id`) は発行しない。状態 (開いている
//!   プロジェクトと提案) はプロセスに 1 つで、すべての要求で共有する (stdio と同じ)。
//! - 要求ごとに `_meta` で版を示す版では、`MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name` の
//!   ヘッダが本文と一致することを確かめる (違えば `400` と `HeaderMismatch`)。
//! - 守り: 既定はループバックだけに束ねる。ループバック以外に束ねるにはトークンが要る。
//!   `Origin` が付いていれば、ループバックか許したものに限る (DNS リバインディング対策。違えば `403`)。
//!   トークンがなければ、`Host` がループバックの名前で、プロキシやトンネルの転送ヘッダが無い要求だけを
//!   受ける (トンネルで認証なしに外へ公開されないように)。トークンは `Authorization: Bearer` か、
//!   ヘッダを送れないクライアントのために URL のパス (`/mcp/<token>`) で受ける。
//!
//! HTTP/1.1 の処理は std の `TcpListener` の上に自分で書いた (接続ごとのスレッド・keep-alive・
//! `Content-Length` と `chunked` の本文)。非同期ランタイムを持ち込まないため。外から届く口なので、
//! 本文の大きさ・ヘッダの数と大きさ・読み込みの時間・同時の接続の数に上限を設け ([`Limits`])、
//! 超えたら `413`・`431`・`408`・`503` で答えて接続を閉じる。要求を読み終える前の誤りの答え (`408`・`413`・
//! `431`・`400` など) の後は、書く側を閉じてから相手の残りを上限まで読み捨てて閉じる (lingering close。
//! 読み残しのまま閉じると RST になり、Windows のクライアントが答えを失う。T0556)。

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::{Value, json};

use crate::server::{
    self, INVALID_REQUEST, LEGACY_VERSIONS, META_PROTOCOL_VERSION, METHOD_NOT_FOUND,
    MODERN_VERSION, PARSE_ERROR, Server, UNSUPPORTED_PROTOCOL_VERSION,
};

/// MCP のエンドポイントのパス。
pub const ENDPOINT: &str = "/mcp";
/// トークンの最短の長さ。16 進数で 128 ビット (推測されにくいように)。
pub const MIN_TOKEN_LEN: usize = 32;
/// [`generate_token`] が作るトークンの乱数のバイト数 (256 ビット)。
const GENERATED_TOKEN_BYTES: usize = 32;
/// `Host` の検査を外す許可がないときに、転送ヘッダとみなすもの (プロキシやトンネルが付ける)。
const FORWARDING_HEADERS: &[&str] = &[
    "forwarded",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-real-ip",
    "cf-connecting-ip",
    "true-client-ip",
];
/// `HeaderMismatch` のエラーコード (2026-07-28)。
const HEADER_MISMATCH: i64 = -32020;

/// 自前の HTTP サーバの上限。超えたら答えて接続を閉じる。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// 本文の上限 (超えたら `413`)。`propose_edit` のコマンドの列 (キーフォームの頂点を含む) が
    /// 収まる大きさにする。
    pub(crate) max_body: usize,
    /// 要求の行とヘッダの合計の大きさの上限 (超えたら `431`)。
    pub(crate) max_header_bytes: usize,
    /// ヘッダの数の上限 (超えたら `431`)。
    pub(crate) max_headers: usize,
    /// 同時に開いておく接続の上限 (超えたら `503`)。
    pub(crate) max_connections: usize,
    /// 1 つの要求を読み終えるまでの時間の上限 (keep-alive で次の要求を待つ時間を含む。超えたら
    /// `408`)。少しずつ送り続けて接続を握り続ける相手 (slowloris) も、この時間で切る。
    pub(crate) request_timeout: Duration,
    /// 誤りの答えの後に、相手が送り続ける残りを読み捨てる時間の上限 (lingering close)。
    pub(crate) linger_timeout: Duration,
    /// 誤りの答えの後に読み捨てる量の上限。
    pub(crate) max_linger_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_body: 32 * 1024 * 1024,
            max_header_bytes: 64 * 1024,
            max_headers: 100,
            max_connections: 32,
            request_timeout: Duration::from_secs(60),
            linger_timeout: Duration::from_secs(2),
            max_linger_bytes: 1024 * 1024,
        }
    }
}

/// HTTP のトランスポートの設定。
#[derive(Debug, Clone, Default)]
pub struct HttpOptions {
    /// 要求に求めるトークン。`None` ならループバックからの要求だけを受ける。
    pub token: Option<String>,
    /// ループバックのほかに受け付ける `Origin` (`https://example.com` の形)。
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("invalid address {0:?} (expected HOST:PORT or PORT)")]
    Address(String),
    #[error("failed to listen on {addr}: {source}")]
    Bind { addr: String, source: io::Error },
    #[error(
        "refusing to listen on {0} without a token: only loopback addresses may be used without \
         authentication (pass --token or set UGOE_MCP_TOKEN)"
    )]
    NotLoopback(SocketAddr),
    #[error("the token must be at least {MIN_TOKEN_LEN} characters long")]
    WeakToken,
    #[error("the token must consist of visible ASCII characters other than '/', '?' and '#'")]
    TokenCharacters,
    #[error("failed to generate a token: {0}")]
    Random(String),
}

/// 推測されにくいトークン (OS の乱数で 256 ビット、16 進数で 64 文字) を作る。
pub fn generate_token() -> Result<String, HttpError> {
    let mut bytes = [0u8; GENERATED_TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| HttpError::Random(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// HTTP で待ち受けるサーバ。
#[derive(Debug)]
pub struct HttpServer {
    listener: TcpListener,
    endpoint: Arc<Endpoint>,
    connections: Arc<AtomicUsize>,
    limits: Limits,
    /// 止めるように頼まれた ([`RunningHttpServer::stop`])。
    stopping: Arc<AtomicBool>,
}

/// 別のスレッドで待ち受けているサーバ ([`HttpServer::spawn`])。止める (または捨てる) と、新しい接続を
/// 受けなくなり、開いている接続への要求にも `503` で答える。
#[derive(Debug)]
pub struct RunningHttpServer {
    addr: SocketAddr,
    endpoint: Arc<Endpoint>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<io::Result<()>>>,
}

impl RunningHttpServer {
    /// 束ねたアドレス。
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// 止め、待ち受けのスレッドが終わるのを待つ。処理中の要求 (確認を待つ適用など) は待たない
    /// (持ち主が確認を拒否して終わらせる。`crate::host::Approvals::deny_all`)。
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.endpoint.closed.store(true, Ordering::SeqCst);
        self.stopping.store(true, Ordering::SeqCst);
        let Some(thread) = self.thread.take() else {
            return;
        };
        // accept で止まっているスレッドを起こす (すべてのアドレスに束ねたならループバックへつなぐ)。
        let mut wake = self.addr;
        if wake.ip().is_unspecified() {
            wake.set_ip(match wake {
                SocketAddr::V4(_) => IpAddr::from([127, 0, 0, 1]),
                SocketAddr::V6(_) => IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]),
            });
        }
        let _ = TcpStream::connect_timeout(&wake, Duration::from_secs(1));
        let _ = thread.join();
    }
}

impl Drop for RunningHttpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl HttpServer {
    /// `addr` (`127.0.0.1:8765` の形か、ポート番号だけ。ポート番号だけならループバック) に束ねる。
    /// ループバック以外に束ねるにはトークンが要る。
    pub fn bind(addr: &str, server: Server, options: HttpOptions) -> Result<Self, HttpError> {
        if let Some(token) = &options.token {
            validate_token(token)?;
        }
        let addr = if addr.parse::<u16>().is_ok() {
            format!("127.0.0.1:{addr}")
        } else if addr.contains(':') {
            addr.to_owned()
        } else {
            return Err(HttpError::Address(addr.to_owned()));
        };
        let listener = TcpListener::bind(&addr).map_err(|source| HttpError::Bind {
            addr: addr.clone(),
            source,
        })?;
        let local = listener.local_addr().map_err(|source| HttpError::Bind {
            addr: addr.clone(),
            source,
        })?;
        if !local.ip().is_loopback() && options.token.is_none() {
            return Err(HttpError::NotLoopback(local));
        }
        Ok(Self {
            listener,
            endpoint: Arc::new(Endpoint::new(server, options)),
            connections: Arc::new(AtomicUsize::new(0)),
            limits: Limits::default(),
            stopping: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 上限を変える (テストで小さくするため)。
    #[cfg(test)]
    pub(crate) fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// 束ねたアドレス (ポート 0 で束ねたときに実際のポートを知るため)。
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// 別のスレッドで要求を受け続け、止める手段を返す (エディタに組み込むとき)。
    pub fn spawn(self) -> io::Result<RunningHttpServer> {
        let addr = self.local_addr()?;
        let endpoint = Arc::clone(&self.endpoint);
        let stopping = Arc::clone(&self.stopping);
        let thread = std::thread::Builder::new()
            .name("ugoe-mcp-http".to_owned())
            .spawn(move || self.serve())?;
        Ok(RunningHttpServer {
            addr,
            endpoint,
            stopping,
            thread: Some(thread),
        })
    }

    /// 要求を受け続ける。接続ごとにスレッドを立てる。MCP のメッセージの処理は接続のスレッドで同時に
    /// 進む ([`Server`] は `&self` で処理できる)。
    pub fn serve(self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            if self.stopping.load(Ordering::SeqCst) {
                break;
            }
            let mut stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("ugoe-mcp: failed to accept a connection: {e}");
                    // ファイル記述子が尽きたときなどに空回りしないように少し待つ。
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            if self.connections.fetch_add(1, Ordering::SeqCst) >= self.limits.max_connections {
                self.connections.fetch_sub(1, Ordering::SeqCst);
                let response = Response::text(503, "too many connections");
                let _ = write_response(&mut stream, &response, false);
                continue;
            }
            let endpoint = Arc::clone(&self.endpoint);
            let slot = ConnectionSlot(Arc::clone(&self.connections));
            let limits = self.limits;
            std::thread::spawn(move || {
                // 接続のスレッドが panic しても枠を返すように、捨てるときに数を戻す。
                let _slot = slot;
                serve_connection(&endpoint, stream, &limits);
            });
        }
        Ok(())
    }
}

/// 同時の接続の枠 1 つ。捨てると (スレッドが panic で終わっても) 数を戻す。
struct ConnectionSlot(Arc<AtomicUsize>);

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn validate_token(token: &str) -> Result<(), HttpError> {
    if token.len() < MIN_TOKEN_LEN {
        return Err(HttpError::WeakToken);
    }
    // パスに入れても区切られず、ヘッダにもそのまま書ける文字だけにする。
    if !token
        .bytes()
        .all(|b| (0x21..=0x7e).contains(&b) && !matches!(b, b'/' | b'?' | b'#'))
    {
        return Err(HttpError::TokenCharacters);
    }
    Ok(())
}

/// 読み込みに期限を付けた接続。期限は要求ごとに決め直す。
struct TimedStream {
    stream: TcpStream,
    deadline: Instant,
}

impl Read for TimedStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(remaining))?;
        self.stream.read(buf)
    }
}

/// 1 つの接続で要求を読み、答える。keep-alive なら続けて読む。
fn serve_connection(endpoint: &Endpoint, stream: TcpStream, limits: &Limits) {
    let _ = stream.set_nodelay(true);
    // 応答を読まない相手に書き込みで止められないように。
    let _ = stream.set_write_timeout(Some(limits.request_timeout));
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(TimedStream {
        stream,
        deadline: Instant::now(),
    });
    loop {
        reader.get_mut().deadline = Instant::now() + limits.request_timeout;
        match read_request(&mut reader, &mut writer, limits) {
            Ok(Some((request, keep_alive))) => {
                let response = endpoint.respond_cancellable(&request, disconnect_probe(&writer));
                if write_response(&mut writer, &response, keep_alive).is_err() || !keep_alive {
                    break;
                }
            }
            Ok(None) | Err(ReadError::Io) => break,
            Err(ReadError::Timeout) => {
                let response = Response::text(408, "request timeout");
                if write_response(&mut writer, &response, false).is_ok() {
                    linger(&mut reader, &writer, limits);
                }
                break;
            }
            Err(ReadError::Bad(status, message)) => {
                let response = Response::text(status, message);
                if write_response(&mut writer, &response, false).is_ok() {
                    linger(&mut reader, &writer, limits);
                }
                break;
            }
        }
    }
}

/// 要求を読み終える前に誤りの答えを書いたあと、すぐには閉じずに書く側だけを閉じ、相手が送り続ける残りを
/// 時間と量の上限まで読み捨てる (lingering close)。読み残しのあるまま閉じると RST が送られ、Windows の
/// クライアントは受け取り済みの答えを捨てる。相手が書く側を閉じるか、上限に達したら戻る (呼び手が閉じる)。
fn linger(reader: &mut BufReader<TimedStream>, writer: &TcpStream, limits: &Limits) {
    if writer.shutdown(Shutdown::Write).is_err() {
        return;
    }
    reader.get_mut().deadline = Instant::now() + limits.linger_timeout;
    let mut discarded = 0;
    let mut buf = [0u8; 8192];
    while discarded < limits.max_linger_bytes {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => discarded += n,
        }
    }
}

/// クライアントが接続を閉じたかを確かめる関数 (要求を処理するスレッドで、答えを書く前に呼ぶ)。
fn disconnect_probe(stream: &TcpStream) -> Option<Box<dyn Fn() -> bool + Send + Sync>> {
    let stream = stream.try_clone().ok()?;
    Some(Box::new(move || peer_closed(&stream)))
}

/// 相手が接続を閉じたか。読まずに覗くので、次の要求 (パイプライン) の本文は消さない。読み込みの期限は
/// 次に読むときに [`TimedStream`] が決め直す。
fn peer_closed(stream: &TcpStream) -> bool {
    if stream
        .set_read_timeout(Some(Duration::from_millis(1)))
        .is_err()
    {
        return false;
    }
    let mut byte = [0u8; 1];
    match stream.peek(&mut byte) {
        Ok(0) => true,
        Ok(_) => false,
        Err(e) => !matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
        ),
    }
}

/// 読んだ HTTP の要求。
#[derive(Debug, Clone, Default)]
pub(crate) struct Request {
    pub(crate) method: String,
    /// 要求の対象 (`/mcp?x=y` のようなパスと問い合わせ)。
    pub(crate) target: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Request {
    /// ヘッダの値 (名前は大文字と小文字を区別しない。同じ名前が複数あれば最初のもの)。
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// 返す HTTP の応答。
#[derive(Debug)]
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(&'static str, String)>,
    pub(crate) body: Vec<u8>,
}

impl Response {
    fn empty(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn text(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "text/plain; charset=utf-8".to_owned())],
            body: format!("{message}\n").into_bytes(),
        }
    }

    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "application/json".to_owned())],
            body: value.to_string().into_bytes(),
        }
    }

    /// ID の無い JSON-RPC のエラーを本文にした応答 (要求を読む前に拒むとき)。
    fn rpc_error(status: u16, code: i64, message: &str) -> Self {
        Self::json(status, &server::error(Value::Null, code, message))
    }
}

enum ReadError {
    /// 読み書きの失敗 (接続が切れたなど)。答えずに接続を閉じる。
    Io,
    /// 要求を読み終える前に時間の上限を過ぎた。`408` で答えて接続を閉じる。
    Timeout,
    /// 不正な要求。この状態コードで答えて接続を閉じる。
    Bad(u16, &'static str),
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Self::Timeout,
            _ => Self::Io,
        }
    }
}

/// 1 行を読む (行末の CRLF を除く)。接続が閉じていれば `None`。
fn read_line(reader: &mut impl BufRead, limit: usize) -> Result<Option<Vec<u8>>, ReadError> {
    let mut line = Vec::new();
    let n = reader.take(limit as u64 + 1).read_until(b'\n', &mut line)?;
    if n == 0 {
        return Ok(None);
    }
    if !line.ends_with(b"\n") {
        return Err(if n > limit {
            ReadError::Bad(431, "request header too large")
        } else {
            ReadError::Bad(400, "unexpected end of request")
        });
    }
    line.pop();
    if line.ends_with(b"\r") {
        line.pop();
    }
    Ok(Some(line))
}

/// 要求を 1 つ読む。接続が要求の前で閉じていれば `None`。2 つ目の値は keep-alive か。
fn read_request(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    limits: &Limits,
) -> Result<Option<(Request, bool)>, ReadError> {
    const BAD_HEAD: ReadError = ReadError::Bad(400, "malformed request head");
    // 要求の行 (前に空行があれば読み飛ばす)。
    let mut budget = limits.max_header_bytes;
    let request_line = loop {
        let Some(line) = read_line(reader, budget)? else {
            return Ok(None);
        };
        budget = budget.saturating_sub(line.len() + 2);
        if !line.is_empty() {
            break String::from_utf8(line).map_err(|_| BAD_HEAD)?;
        }
    };
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(BAD_HEAD);
    };
    let http10 = match version {
        "HTTP/1.1" => false,
        "HTTP/1.0" => true,
        _ => return Err(ReadError::Bad(505, "HTTP version not supported")),
    };
    let mut request = Request {
        method: method.to_owned(),
        target: target.to_owned(),
        ..Request::default()
    };
    loop {
        let line =
            read_line(reader, budget)?.ok_or(ReadError::Bad(400, "unexpected end of request"))?;
        budget = budget.saturating_sub(line.len() + 2);
        if line.is_empty() {
            break;
        }
        if request.headers.len() >= limits.max_headers {
            return Err(ReadError::Bad(431, "too many header fields"));
        }
        let line = String::from_utf8(line).map_err(|_| BAD_HEAD)?;
        // 折り返し (obs-fold) は受けない (RFC 9112 5.2)。
        let (name, value) = line.split_once(':').ok_or(BAD_HEAD)?;
        if name.is_empty() || name.contains([' ', '\t']) {
            return Err(BAD_HEAD);
        }
        request
            .headers
            .push((name.to_owned(), value.trim_matches([' ', '\t']).to_owned()));
    }

    let connection = request
        .header("connection")
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut keep_alive = if http10 {
        connection.contains("keep-alive")
    } else {
        !connection.contains("close")
    };

    if let Some(encoding) = request.header("transfer-encoding") {
        // 解けるのは `chunked` だけ (`gzip, chunked` などは本文を解けない)。
        if !encoding.trim().eq_ignore_ascii_case("chunked") {
            return Err(ReadError::Bad(501, "unsupported transfer encoding"));
        }
        // `Content-Length` もあれば `chunked` で読み、答えたら閉じる (RFC 9112 6.1)。
        if request.header("content-length").is_some() {
            keep_alive = false;
        }
        continue_if_expected(&request, writer)?;
        request.body = read_chunked(reader, limits)?;
    } else if let Some(length) = request.header("content-length") {
        let length: usize = length
            .parse()
            .map_err(|_| ReadError::Bad(400, "invalid content-length"))?;
        if length > limits.max_body {
            return Err(ReadError::Bad(413, "request body too large"));
        }
        if length > 0 {
            continue_if_expected(&request, writer)?;
        }
        request.body = vec![0; length];
        reader.read_exact(&mut request.body)?;
    }
    Ok(Some((request, keep_alive)))
}

/// `Expect: 100-continue` なら、本文を送ってよいと伝える (curl は大きな本文でこれを待つ)。
fn continue_if_expected(request: &Request, writer: &mut impl Write) -> Result<(), ReadError> {
    if request
        .header("expect")
        .is_some_and(|e| e.eq_ignore_ascii_case("100-continue"))
    {
        writer.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
        writer.flush()?;
    }
    Ok(())
}

/// `chunked` の本文を読む。
fn read_chunked(reader: &mut impl BufRead, limits: &Limits) -> Result<Vec<u8>, ReadError> {
    const BAD_CHUNK: ReadError = ReadError::Bad(400, "malformed chunked body");
    let mut body = Vec::new();
    loop {
        let line = read_line(reader, 1024)?.ok_or(BAD_CHUNK)?;
        let line = std::str::from_utf8(&line).map_err(|_| BAD_CHUNK)?;
        let size = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size, 16).map_err(|_| BAD_CHUNK)?;
        if size == 0 {
            // トレイラーを読み飛ばす。
            let mut budget = limits.max_header_bytes;
            while let Some(line) = read_line(reader, budget)? {
                if line.is_empty() {
                    break;
                }
                budget = budget.saturating_sub(line.len() + 2);
            }
            return Ok(body);
        }
        if body.len().saturating_add(size) > limits.max_body {
            return Err(ReadError::Bad(413, "request body too large"));
        }
        let start = body.len();
        body.resize(start + size, 0);
        reader.read_exact(&mut body[start..])?;
        if read_line(reader, 2)? != Some(Vec::new()) {
            return Err(BAD_CHUNK);
        }
    }
}

fn write_response(
    writer: &mut impl Write,
    response: &Response,
    keep_alive: bool,
) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason(response.status)
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    head.push_str("Cache-Control: no-store\r\n");
    head.push_str(if keep_alive {
        "Connection: keep-alive\r\n\r\n"
    } else {
        "Connection: close\r\n\r\n"
    });
    writer.write_all(head.as_bytes())?;
    writer.write_all(&response.body)?;
    writer.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        408 => "Request Timeout",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

/// MCP のエンドポイント。HTTP の要求を MCP のメッセージとして [`Server`] に渡す
/// (ソケットに依らない部分)。
#[derive(Debug)]
pub(crate) struct Endpoint {
    server: Server,
    options: HttpOptions,
    /// 止めた ([`RunningHttpServer::stop`])。開いている接続への要求にも `503` で答える。
    closed: AtomicBool,
}

impl Endpoint {
    pub(crate) fn new(server: Server, options: HttpOptions) -> Self {
        Self {
            server,
            options,
            closed: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn respond(&self, request: &Request) -> Response {
        self.respond_cancellable(request, None)
    }

    /// `disconnected` はクライアントが接続を閉じたかを答える (閉じたら要求を取り消す)。
    fn respond_cancellable(
        &self,
        request: &Request,
        disconnected: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    ) -> Response {
        if self.closed.load(Ordering::SeqCst) {
            return Response::text(503, "the server is stopping");
        }
        let path = request
            .target
            .split_once('?')
            .map_or(request.target.as_str(), |(path, _)| path);
        let path_token = match path.strip_prefix(ENDPOINT) {
            Some("" | "/") => None,
            Some(rest)
                if self.options.token.is_some() && rest.len() > 1 && rest.starts_with('/') =>
            {
                rest.strip_prefix('/')
            }
            _ => {
                return Response::text(404, &format!("not found: the MCP endpoint is {ENDPOINT}"));
            }
        };
        if let Err(response) = self
            .check_origin(request)
            .and_then(|()| self.check_host(request))
            .and_then(|()| self.check_token(request, path_token))
        {
            return response;
        }
        match request.method.as_str() {
            "POST" => self.post(request, disconnected),
            _ => {
                // GET (サーバからの SSE)・DELETE (セッションの終了) には応じない。
                let mut response = Response::text(405, "method not allowed: use POST");
                response.headers.push(("Allow", "POST".to_owned()));
                response
            }
        }
    }

    /// DNS リバインディング対策: `Origin` が付いていれば、ループバックか許したものに限る。
    fn check_origin(&self, request: &Request) -> Result<(), Response> {
        let Some(origin) = request.header("origin") else {
            return Ok(());
        };
        let origin = origin.trim_end_matches('/');
        let allowed = origin.split_once("://").is_some_and(|(scheme, authority)| {
            matches!(scheme, "http" | "https") && is_loopback_host(authority)
        }) || self
            .options
            .allowed_origins
            .iter()
            .any(|o| o.trim_end_matches('/').eq_ignore_ascii_case(origin));
        if allowed {
            return Ok(());
        }
        eprintln!("ugoe-mcp: rejected a request from origin {origin:?} (see --allow-origin)");
        Err(Response::rpc_error(
            403,
            INVALID_REQUEST,
            &format!(
                "origin {origin:?} is not allowed (start the server with --allow-origin to allow it)"
            ),
        ))
    }

    /// トークンがなければ、ループバックの名前に宛てた、転送されていない要求だけを受ける
    /// (トンネルやプロキシで認証なしに外へ公開されないように。DNS リバインディング対策も兼ねる)。
    fn check_host(&self, request: &Request) -> Result<(), Response> {
        if self.options.token.is_some() {
            return Ok(());
        }
        let host = request.header("host").unwrap_or("");
        if !is_loopback_host(host) {
            eprintln!("ugoe-mcp: rejected a request for host {host:?} (no token is set)");
            return Err(Response::rpc_error(
                403,
                INVALID_REQUEST,
                &format!(
                    "host {host:?} is not a loopback name; set a token (--token or UGOE_MCP_TOKEN) \
                     to accept requests through a tunnel or proxy"
                ),
            ));
        }
        if let Some(name) = FORWARDING_HEADERS
            .iter()
            .find(|name| request.header(name).is_some())
        {
            eprintln!("ugoe-mcp: rejected a forwarded request ({name}; no token is set)");
            return Err(Response::rpc_error(
                403,
                INVALID_REQUEST,
                &format!(
                    "the request was forwarded by a proxy or tunnel ({name}); set a token \
                     (--token or UGOE_MCP_TOKEN) to accept such requests"
                ),
            ));
        }
        Ok(())
    }

    /// トークンを確かめる (`Authorization: Bearer <token>` か、パスの `/mcp/<token>`)。
    fn check_token(&self, request: &Request, path_token: Option<&str>) -> Result<(), Response> {
        let Some(token) = &self.options.token else {
            return Ok(());
        };
        let bearer = request.header("authorization").and_then(|value| {
            let (scheme, credentials) = value.split_once(' ')?;
            scheme
                .eq_ignore_ascii_case("bearer")
                .then(|| credentials.trim())
        });
        if [bearer, path_token]
            .into_iter()
            .flatten()
            .any(|given| constant_time_eq(given.as_bytes(), token.as_bytes()))
        {
            return Ok(());
        }
        eprintln!("ugoe-mcp: rejected a request without a valid token");
        let mut response = Response::rpc_error(401, INVALID_REQUEST, "missing or invalid token");
        response
            .headers
            .push(("WWW-Authenticate", "Bearer realm=\"ugoe-mcp\"".to_owned()));
        Err(response)
    }

    fn post(
        &self,
        request: &Request,
        disconnected: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    ) -> Response {
        if let Some(content_type) = request.header("content-type")
            && !media_type(content_type).eq_ignore_ascii_case("application/json")
        {
            return Response::text(415, "the body must be application/json");
        }
        if let Some(accept) = request.header("accept")
            && !accept.split(',').map(media_type).any(|m| {
                ["application/json", "application/*", "*/*"]
                    .iter()
                    .any(|a| m.eq_ignore_ascii_case(a))
            })
        {
            return Response::text(406, "this server responds with application/json");
        }
        let message: Value = match serde_json::from_slice(&request.body) {
            Ok(message) => message,
            Err(e) => {
                return Response::rpc_error(400, PARSE_ERROR, &format!("parse error: {e}"));
            }
        };
        let Some(object) = message.as_object() else {
            return Response::rpc_error(400, INVALID_REQUEST, "expected a single JSON-RPC object");
        };
        let method = object.get("method").and_then(Value::as_str);
        let id = object.get("id").cloned();
        let (Some(method), Some(id)) = (method, id) else {
            // 通知 (method があり id が無い) と、クライアントからの応答。受け取ったら 202 を返す。
            if method.is_some() || object.contains_key("result") || object.contains_key("error") {
                self.server.handle(message);
                return Response::empty(202);
            }
            return Response::rpc_error(400, INVALID_REQUEST, "missing method");
        };
        let params = object.get("params").cloned().unwrap_or_else(|| json!({}));
        let modern = params
            .get("_meta")
            .and_then(|m| m.get(META_PROTOCOL_VERSION))
            .is_some();
        if let Err(response) = validate_headers(request, &id, method, &params) {
            return response;
        }
        let Some(reply) = self.server.handle_cancellable(message, disconnected) else {
            return Response::empty(202);
        };
        let status = match reply
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_i64)
        {
            Some(UNSUPPORTED_PROTOCOL_VERSION) => 400,
            // 要求ごとに版を示す版では、知らないメソッドに 404 を返す (旧来の HTTP+SSE の 404 と
            // 本文で区別できる)。
            Some(METHOD_NOT_FOUND) if modern => 404,
            _ => 200,
        };
        Response::json(status, &reply)
    }
}

/// 本文と対応するヘッダ (`MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name`) を確かめる。
///
/// 要求ごとに `_meta` で版を示す版 (2026-07-28) では 3 つとも必須で、本文と一致しなければならない。
/// 握手をする版では `MCP-Protocol-Version` だけで (無ければ 2025-03-26 とみなす)、応じられる版で
/// なければならない。どちらの版でも、付いているヘッダは本文と一致しなければならない。
fn validate_headers(
    request: &Request,
    id: &Value,
    method: &str,
    params: &Value,
) -> Result<(), Response> {
    let mismatch = |message: String| {
        Response::json(
            400,
            &server::error(
                id.clone(),
                HEADER_MISMATCH,
                &format!("Header mismatch: {message}"),
            ),
        )
    };
    let body_version = params
        .get("_meta")
        .and_then(|m| m.get(META_PROTOCOL_VERSION));
    let header_version = request.header("mcp-protocol-version");
    let header_method = request.header("mcp-method");
    let header_name = request.header("mcp-name");
    let body_name = match method {
        "tools/call" | "prompts/get" => params.get("name").and_then(Value::as_str),
        "resources/read" => params.get("uri").and_then(Value::as_str),
        _ => None,
    };

    if let Some(body_version) = body_version {
        let Some(header_version) = header_version else {
            return Err(mismatch("missing MCP-Protocol-Version header".to_owned()));
        };
        if body_version.as_str() != Some(header_version) {
            return Err(mismatch(format!(
                "MCP-Protocol-Version header value {header_version:?} does not match body value \
                 {body_version}"
            )));
        }
        if header_method.is_none() {
            return Err(mismatch("missing Mcp-Method header".to_owned()));
        }
        if body_name.is_some() && header_name.is_none() {
            return Err(mismatch("missing Mcp-Name header".to_owned()));
        }
    } else if let Some(header_version) = header_version
        && method != "initialize"
    {
        if header_version == MODERN_VERSION {
            return Err(mismatch(format!(
                "MCP-Protocol-Version header is {MODERN_VERSION}, but the body has no \
                 _meta[\"{META_PROTOCOL_VERSION}\"]"
            )));
        }
        if !LEGACY_VERSIONS.contains(&header_version) {
            return Err(Response::json(
                400,
                &server::error_with_data(
                    id.clone(),
                    UNSUPPORTED_PROTOCOL_VERSION,
                    "Unsupported protocol version",
                    Some(
                        json!({ "supported": server::supported_versions(), "requested": header_version }),
                    ),
                ),
            ));
        }
    }

    if let Some(header_method) = header_method
        && header_method != method
    {
        return Err(mismatch(format!(
            "Mcp-Method header value {header_method:?} does not match body value {method:?}"
        )));
    }
    if let Some(header_name) = header_name {
        let decoded = decode_header_value(header_name);
        if decoded.is_none() || decoded.as_deref() != body_name {
            return Err(mismatch(format!(
                "Mcp-Name header value {header_name:?} does not match body value {body_name:?}"
            )));
        }
    }
    Ok(())
}

/// ヘッダの値を読む。`=?base64?...?=` の形なら base64 を解く (2026-07-28 の Value Encoding)。
fn decode_header_value(value: &str) -> Option<String> {
    let Some(encoded) = value
        .strip_prefix("=?base64?")
        .and_then(|v| v.strip_suffix("?="))
    else {
        return Some(value.to_owned());
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// `type/subtype; params` の `type/subtype`。
fn media_type(value: &str) -> &str {
    value.split(';').next().unwrap_or("").trim()
}

/// `host[:port]` のホストがループバック (`localhost`・`127.0.0.0/8`・`::1`) か。
fn is_loopback_host(authority: &str) -> bool {
    if let Ok(ip) = authority.parse::<IpAddr>() {
        return ip.is_loopback();
    }
    let host = if let Some(rest) = authority.strip_prefix('[') {
        // `[::1]:8765`
        rest.split_once(']').map_or("", |(host, _)| host)
    } else {
        authority
            .split_once(':')
            .map_or(authority, |(host, _)| host)
    };
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// 長さ以外で時間の差が出ない比較 (トークンを 1 文字ずつ当てられないように)。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::Options;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef-token";

    fn endpoint(token: Option<&str>) -> Endpoint {
        Endpoint::new(
            Server::new(Options::default()),
            HttpOptions {
                token: token.map(str::to_owned),
                allowed_origins: vec!["https://chatgpt.com".to_owned()],
            },
        )
    }

    fn post(target: &str, headers: &[(&str, &str)], body: &Value) -> Request {
        let mut all = vec![
            ("Host".to_owned(), "127.0.0.1:8765".to_owned()),
            ("Content-Type".to_owned(), "application/json".to_owned()),
            (
                "Accept".to_owned(),
                "application/json, text/event-stream".to_owned(),
            ),
        ];
        for (name, value) in headers {
            all.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
            all.push(((*name).to_owned(), (*value).to_owned()));
        }
        Request {
            method: "POST".to_owned(),
            target: target.to_owned(),
            headers: all,
            body: body.to_string().into_bytes(),
        }
    }

    fn ping() -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" })
    }

    fn body(response: &Response) -> Value {
        serde_json::from_slice(&response.body).unwrap()
    }

    fn modern_call(name: &str) -> Value {
        json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {
                "name": name, "arguments": { "path": "missing.ugoeproj" },
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": MODERN_VERSION,
                    "io.modelcontextprotocol/clientCapabilities": {},
                },
            },
        })
    }

    #[test]
    fn legacy_requests_and_notifications() {
        let e = endpoint(None);
        let init = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } },
        });
        let r = e.respond(&post("/mcp", &[], &init));
        assert_eq!(r.status, 200);
        assert_eq!(body(&r)["result"]["protocolVersion"], "2025-11-25");
        assert!(
            !r.headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("mcp-session-id"))
        );

        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let r = e.respond(&post(
            "/mcp",
            &[("MCP-Protocol-Version", "2025-11-25")],
            &note,
        ));
        assert_eq!(r.status, 202);
        assert!(r.body.is_empty());

        // ヘッダが無ければ 2025-03-26 とみなして受ける。
        assert_eq!(e.respond(&post("/mcp/", &[], &ping())).status, 200);
        let r = e.respond(&post(
            "/mcp",
            &[("MCP-Protocol-Version", "1999-01-01")],
            &ping(),
        ));
        assert_eq!(r.status, 400);
        assert_eq!(body(&r)["error"]["code"], UNSUPPORTED_PROTOCOL_VERSION);
        // 旧来の版では知らないメソッドは 200 と JSON-RPC のエラー。
        let r = e.respond(&post(
            "/mcp",
            &[],
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "nope" }),
        ));
        assert_eq!(r.status, 200);
        assert_eq!(body(&r)["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn modern_headers_must_match_the_body() {
        let e = endpoint(None);
        let good = [
            ("MCP-Protocol-Version", MODERN_VERSION),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "get_model_summary"),
        ];
        let r = e.respond(&post("/mcp", &good, &modern_call("get_model_summary")));
        assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
        assert_eq!(body(&r)["result"]["resultType"], "complete");

        // base64 の形の Mcp-Name。
        let encoded = format!(
            "=?base64?{}?=",
            base64::engine::general_purpose::STANDARD.encode("get_model_summary")
        );
        let mut headers = good;
        headers[2].1 = &encoded;
        assert_eq!(
            e.respond(&post("/mcp", &headers, &modern_call("get_model_summary")))
                .status,
            200
        );

        for (i, value) in [(0, "2025-11-25"), (1, "tools/list"), (2, "render_png")] {
            let mut headers = good;
            headers[i].1 = value;
            let r = e.respond(&post("/mcp", &headers, &modern_call("get_model_summary")));
            assert_eq!(r.status, 400, "{value}");
            assert_eq!(body(&r)["error"]["code"], HEADER_MISMATCH);
            assert_eq!(body(&r)["id"], 7);
        }
        for missing in 0..3 {
            let headers: Vec<_> = good
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != missing)
                .map(|(_, h)| *h)
                .collect();
            let r = e.respond(&post("/mcp", &headers, &modern_call("get_model_summary")));
            assert_eq!(r.status, 400);
            assert_eq!(body(&r)["error"]["code"], HEADER_MISMATCH);
        }

        // 知らないメソッドは 404 と -32601。
        let mut unknown = modern_call("x");
        unknown["method"] = json!("nope/nope");
        let r = e.respond(&post(
            "/mcp",
            &[
                ("MCP-Protocol-Version", MODERN_VERSION),
                ("Mcp-Method", "nope/nope"),
            ],
            &unknown,
        ));
        assert_eq!(r.status, 404);
        assert_eq!(body(&r)["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn get_delete_and_bad_bodies() {
        let e = endpoint(None);
        let mut get = post("/mcp", &[("Accept", "text/event-stream")], &json!(null));
        get.method = "GET".to_owned();
        get.body.clear();
        let r = e.respond(&get);
        assert_eq!(r.status, 405);
        assert!(r.headers.contains(&("Allow", "POST".to_owned())));
        get.method = "DELETE".to_owned();
        assert_eq!(e.respond(&get).status, 405);

        assert_eq!(e.respond(&post("/other", &[], &ping())).status, 404);
        assert_eq!(e.respond(&post("/mcp/extra", &[], &ping())).status, 404);
        assert_eq!(
            e.respond(&post("/mcp", &[("Content-Type", "text/plain")], &ping()))
                .status,
            415
        );
        assert_eq!(
            e.respond(&post("/mcp", &[("Accept", "text/event-stream")], &ping()))
                .status,
            406
        );
        assert_eq!(e.respond(&post("/mcp", &[], &json!([ping()]))).status, 400);
        let mut garbage = post("/mcp", &[], &ping());
        garbage.body = b"{".to_vec();
        let r = e.respond(&garbage);
        assert_eq!(r.status, 400);
        assert_eq!(body(&r)["error"]["code"], PARSE_ERROR);
    }

    #[test]
    fn origin_host_and_forwarding_without_a_token() {
        let e = endpoint(None);
        for origin in [
            "http://localhost:3000",
            "http://127.0.0.1:8765",
            "http://[::1]:8765",
            "https://chatgpt.com",
        ] {
            assert_eq!(
                e.respond(&post("/mcp", &[("Origin", origin)], &ping()))
                    .status,
                200,
                "{origin}"
            );
        }
        for origin in [
            "https://evil.example",
            "null",
            "http://localhost.evil.example",
        ] {
            let r = e.respond(&post("/mcp", &[("Origin", origin)], &ping()));
            assert_eq!(r.status, 403, "{origin}");
            assert!(body(&r)["id"].is_null());
        }
        // DNS リバインディング: 攻撃者の名前がループバックに向いている。
        assert_eq!(
            e.respond(&post("/mcp", &[("Host", "evil.example:8765")], &ping()))
                .status,
            403
        );
        // トンネル (ngrok・cloudflared など) の転送。
        assert_eq!(
            e.respond(&post("/mcp", &[("Host", "abc.ngrok.app")], &ping()))
                .status,
            403
        );
        assert_eq!(
            e.respond(&post(
                "/mcp",
                &[("X-Forwarded-For", "203.0.113.1")],
                &ping()
            ))
            .status,
            403
        );
        assert_eq!(
            e.respond(&post("/mcp", &[("Host", "localhost:8765")], &ping()))
                .status,
            200
        );
        // トークンが無ければパスのトークンの形も無い。
        assert_eq!(
            e.respond(&post(&format!("/mcp/{TOKEN}"), &[], &ping()))
                .status,
            404
        );
    }

    #[test]
    fn token_in_header_or_path() {
        let e = endpoint(Some(TOKEN));
        let bearer = format!("Bearer {TOKEN}");
        let tunnel = [
            ("Host", "abc.ngrok.app"),
            ("X-Forwarded-For", "203.0.113.1"),
        ];
        let r = e.respond(&post("/mcp", &tunnel, &ping()));
        assert_eq!(r.status, 401);
        assert!(r.headers.iter().any(|(n, _)| *n == "WWW-Authenticate"));
        let r = e.respond(&post(
            "/mcp",
            &[tunnel[0], tunnel[1], ("Authorization", &bearer)],
            &ping(),
        ));
        assert_eq!(r.status, 200);
        assert_eq!(
            e.respond(&post(
                "/mcp",
                &[("authorization", &format!("bearer {TOKEN}"))],
                &ping()
            ))
            .status,
            200
        );
        assert_eq!(
            e.respond(&post(
                "/mcp",
                &[("Authorization", "Bearer wrong-token-wrong-token")],
                &ping()
            ))
            .status,
            401
        );
        assert_eq!(
            e.respond(&post(&format!("/mcp/{TOKEN}"), &tunnel, &ping()))
                .status,
            200
        );
        assert_eq!(
            e.respond(&post(&format!("/mcp/{TOKEN}?x=1"), &tunnel, &ping()))
                .status,
            200
        );
        assert_eq!(e.respond(&post("/mcp/wrong", &tunnel, &ping())).status, 401);
        // `/mcp` で始まるだけの別のパスはエンドポイントでない (トークンがあっても)。
        for path in ["/mcpextra", "/mcpx/y"] {
            let r = e.respond(&post(path, &[("Authorization", &bearer)], &ping()));
            assert_eq!(r.status, 404, "{path}");
        }
        // トークンがあっても Origin は確かめる。
        let r = e.respond(&post(
            "/mcp",
            &[
                ("Authorization", &bearer),
                ("Origin", "https://evil.example"),
            ],
            &ping(),
        ));
        assert_eq!(r.status, 403);
    }

    #[test]
    fn bind_requires_a_token_off_loopback() {
        let server = || Server::new(Options::default());
        let err = HttpServer::bind("0.0.0.0:0", server(), HttpOptions::default()).unwrap_err();
        assert!(matches!(err, HttpError::NotLoopback(_)), "{err}");
        let weak = HttpOptions {
            token: Some("short".to_owned()),
            ..HttpOptions::default()
        };
        assert!(matches!(
            HttpServer::bind("127.0.0.1:0", server(), weak),
            Err(HttpError::WeakToken)
        ));
        let slash = HttpOptions {
            token: Some("0123456789abcdef/0123456789abcdef".to_owned()),
            ..HttpOptions::default()
        };
        assert!(matches!(
            HttpServer::bind("127.0.0.1:0", server(), slash),
            Err(HttpError::TokenCharacters)
        ));
        assert!(matches!(
            HttpServer::bind("localhost", server(), HttpOptions::default()),
            Err(HttpError::Address(_))
        ));
        let bound = HttpServer::bind("0", server(), HttpOptions::default()).unwrap();
        assert!(bound.local_addr().unwrap().ip().is_loopback());
        let with_token = HttpOptions {
            token: Some(TOKEN.to_owned()),
            ..HttpOptions::default()
        };
        assert!(HttpServer::bind("0.0.0.0:0", server(), with_token).is_ok());
    }

    #[test]
    fn reads_requests_from_a_stream() {
        let raw = b"\r\nPOST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}\
                    POST /mcp HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\
                    Expect: 100-continue\r\nConnection: close\r\n\r\n3;x=y\r\nabc\r\n2\r\nde\r\n0\r\n\r\n";
        let mut reader = BufReader::new(&raw[..]);
        let mut written = Vec::new();
        let Ok(Some((first, keep))) = read_request(&mut reader, &mut written, &Limits::default())
        else {
            panic!("first request");
        };
        assert!(keep);
        assert_eq!(
            (first.method.as_str(), first.target.as_str()),
            ("POST", "/mcp")
        );
        assert_eq!(first.header("HOST"), Some("localhost"));
        assert_eq!(first.body, b"{}");
        let Ok(Some((second, keep))) = read_request(&mut reader, &mut written, &Limits::default())
        else {
            panic!("second request");
        };
        assert!(!keep);
        assert_eq!(second.body, b"abcde");
        assert_eq!(written, b"HTTP/1.1 100 Continue\r\n\r\n");
        assert!(matches!(
            read_request(&mut reader, &mut written, &Limits::default()),
            Ok(None)
        ));

        // `Transfer-Encoding` と `Content-Length` の両方があれば `chunked` で読み、答えたら閉じる
        // (RFC 9112 6.1。要求の境界の食い違いを次の要求に持ち越さない)。
        let raw = b"POST /mcp HTTP/1.1\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n\
                    2\r\n{}\r\n0\r\n\r\n";
        let mut reader = BufReader::new(&raw[..]);
        let Ok(Some((both, keep))) = read_request(&mut reader, &mut Vec::new(), &Limits::default())
        else {
            panic!("request with both framings");
        };
        assert_eq!(both.body, b"{}");
        assert!(!keep);
        for (raw, status) in [
            (
                &b"POST /mcp HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\n\r\n"[..],
                501,
            ),
            (&b"GET /mcp HTTP/2\r\n\r\n"[..], 505),
            (b"GET /mcp\r\n\r\n", 400),
            (
                b"POST /mcp HTTP/1.1\r\nContent-Length: 999999999\r\n\r\n",
                413,
            ),
            (
                b"POST /mcp HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n",
                501,
            ),
            (b"POST /mcp HTTP/1.1\r\n folded\r\n\r\n", 400),
        ] {
            let mut reader = BufReader::new(raw);
            match read_request(&mut reader, &mut Vec::new(), &Limits::default()) {
                Err(ReadError::Bad(s, _)) => {
                    assert_eq!(s, status, "{}", String::from_utf8_lossy(raw))
                }
                _ => panic!("expected {status} for {}", String::from_utf8_lossy(raw)),
            }
        }
    }

    #[test]
    fn loopback_hosts() {
        for host in [
            "localhost",
            "LOCALHOST:1",
            "127.0.0.1",
            "127.1.2.3:80",
            "[::1]:8765",
            "::1",
        ] {
            assert!(is_loopback_host(host), "{host}");
        }
        for host in [
            "",
            "example.com",
            "localhost.evil.example",
            "10.0.0.1:80",
            "[::2]:1",
            "0.0.0.0",
        ] {
            assert!(!is_loopback_host(host), "{host}");
        }
    }

    #[test]
    fn generated_tokens_are_long_and_random() {
        let a = generate_token().unwrap();
        let b = generate_token().unwrap();
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert!(validate_token(&a).is_ok());
        // 16 進数で 128 ビットに満たないものは受けない。
        assert!(matches!(
            validate_token(&a[..31]),
            Err(HttpError::WeakToken)
        ));
        assert!(!constant_time_eq(a.as_bytes(), b.as_bytes()));
        assert!(!constant_time_eq(a.as_bytes(), &a.as_bytes()[..63]));
        assert!(constant_time_eq(a.as_bytes(), a.clone().as_bytes()));
    }

    /// 上限を小さくしたサーバを裏で動かし、アドレスを返す。
    fn spawn_limited(limits: Limits) -> SocketAddr {
        let server = HttpServer::bind(
            "127.0.0.1:0",
            Server::new(Options::default()),
            HttpOptions::default(),
        )
        .unwrap()
        .with_limits(limits);
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || server.serve());
        addr
    }

    fn small_limits() -> Limits {
        Limits {
            max_body: 64,
            max_header_bytes: 512,
            max_headers: 8,
            max_connections: 4,
            request_timeout: Duration::from_millis(300),
            linger_timeout: Duration::from_millis(500),
            max_linger_bytes: 4096,
        }
    }

    /// 生の要求を送り、サーバが閉じるまで読んで状態コードを返す。
    fn exchange(addr: SocketAddr, raw: &[u8]) -> u16 {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(raw).unwrap();
        status_of(&mut stream)
    }

    fn status_of(stream: &mut TcpStream) -> u16 {
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        let response = String::from_utf8_lossy(&response);
        response
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status in {response:?}"))
    }

    #[test]
    fn body_over_the_limit_is_413() {
        let addr = spawn_limited(small_limits());
        let body = "x".repeat(65);
        let raw =
            format!("POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 65\r\n\r\n{body}");
        assert_eq!(exchange(addr, raw.as_bytes()), 413);
        let chunked = format!(
            "POST /mcp HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n20\r\n{}\r\n21\r\n{}\r\n0\r\n\r\n",
            "x".repeat(32),
            "x".repeat(33)
        );
        assert_eq!(exchange(addr, chunked.as_bytes()), 413);
    }

    #[test]
    fn headers_over_the_limits_are_431() {
        let addr = spawn_limited(small_limits());
        let large = format!(
            "POST /mcp HTTP/1.1\r\nHost: localhost\r\nX-Large: {}\r\n\r\n",
            "y".repeat(600)
        );
        assert_eq!(exchange(addr, large.as_bytes()), 431);
        let mut many = "POST /mcp HTTP/1.1\r\n".to_owned();
        for i in 0..9 {
            many.push_str(&format!("X-H{i}: 1\r\n"));
        }
        many.push_str("\r\n");
        assert_eq!(exchange(addr, many.as_bytes()), 431);
    }

    #[test]
    fn slow_requests_time_out_with_408() {
        let addr = spawn_limited(small_limits());
        let started = Instant::now();
        // 要求の途中で止まる (残りを送らない)。
        assert_eq!(exchange(addr, b"POST /mcp HTTP/1.1\r\nHost: local"), 408);
        // 少しずつ送り続けても、要求ごとの上限で切る。
        // 送りながら答えを読む (送り終えてから読むと、閉じたサーバへの書き込みで RST が返り、Windows では
        // 受け取り済みの答えが捨てられる)。
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut sender = stream.try_clone().unwrap();
        let sending = std::thread::spawn(move || {
            for byte in b"POST /mcp HTTP/1.1\r\nHost: localhost\r\n" {
                // サーバが切ったら書けなくなる。
                if sender.write_all(&[*byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        assert_eq!(status_of(&mut stream), 408);
        sending.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// 要求の途中で答えを読み、その後も送り続けるクライアント。答えの状態コードと、答えの後に送れたかを返す
    /// (サーバが読み捨てずに閉じれば、送った残りで RST が返り、次の書き込みが失敗する)。
    fn read_early_then_send(addr: SocketAddr, head: &[u8], rest: &[u8]) -> (u16, bool) {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(head).unwrap();
        // サーバは答えを書いて書く側を閉じるので、読み残しを送る前に答えを読み終えられる。
        let status = status_of(&mut stream);
        let mut sent = true;
        for chunk in rest.chunks(rest.len().div_ceil(4).max(1)) {
            std::thread::sleep(Duration::from_millis(30));
            if stream.write_all(chunk).is_err() {
                sent = false;
                break;
            }
        }
        (status, sent)
    }

    #[test]
    fn error_responses_reach_clients_that_are_still_sending() {
        let addr = spawn_limited(small_limits());
        let body = "x".repeat(200);
        // 本文を送る前に 413 を読む。
        let head = "POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 200\r\n\r\n";
        assert_eq!(
            read_early_then_send(addr, head.as_bytes(), body.as_bytes()),
            (413, true)
        );
        // ヘッダを送り終える前に 431 を読む。
        let head = format!(
            "POST /mcp HTTP/1.1\r\nHost: localhost\r\nX-Large: {}",
            "y".repeat(600)
        );
        assert_eq!(
            read_early_then_send(addr, head.as_bytes(), b"yyyyyyyy\r\n\r\n"),
            (431, true)
        );
        // 要求の途中で止まって 408 を読み、その後に残りを送る。
        assert_eq!(
            read_early_then_send(addr, b"POST /mcp HTTP/1.1\r\nHost: local", b"host\r\n\r\n"),
            (408, true)
        );
    }

    #[test]
    fn lingering_close_is_bounded() {
        // 時間: 送り続ける相手でも、読み捨ての上限の時間で閉じる。
        let addr = spawn_limited(small_limits());
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .write_all(b"POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1000\r\n\r\n")
            .unwrap();
        assert_eq!(status_of(&mut stream), 413);
        let started = Instant::now();
        while stream.write_all(b"x").is_ok() {
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(started.elapsed() >= small_limits().linger_timeout / 2);

        // 量: 時間の上限が長くても、読み捨ての上限の量を超えたら閉じる。
        let addr = spawn_limited(Limits {
            linger_timeout: Duration::from_secs(30),
            ..small_limits()
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .write_all(b"POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 99999999\r\n\r\n")
            .unwrap();
        assert_eq!(status_of(&mut stream), 413);
        let started = Instant::now();
        let chunk = [b'x'; 1024];
        while stream.write_all(&chunk).is_ok() {
            assert!(started.elapsed() < Duration::from_secs(10));
        }
    }

    #[test]
    fn connections_over_the_limit_get_503() {
        let limits = Limits {
            max_connections: 2,
            request_timeout: Duration::from_secs(30),
            ..small_limits()
        };
        let addr = spawn_limited(limits);
        let held: Vec<TcpStream> = (0..2).map(|_| TcpStream::connect(addr).unwrap()).collect();
        // 接続を受け付けてスレッドが立つまで待つ。
        let mut status = 0;
        for _ in 0..50 {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .unwrap();
            let mut response = Vec::new();
            let _ = stream.read_to_end(&mut response);
            if response.starts_with(b"HTTP/1.1 503") {
                status = 503;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(status, 503);
        // 閉じれば、また受ける。
        drop(held);
        let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let raw = format!(
            "POST /mcp HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{ping}",
            ping.len()
        );
        let mut ok = false;
        for _ in 0..50 {
            if exchange(addr, raw.as_bytes()) == 200 {
                ok = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(ok);
    }

    #[test]
    fn panics_in_tools_do_not_use_up_the_connections() {
        let server = Server::with_host(Options::default(), Arc::new(crate::host::PanickingHost));
        let server = HttpServer::bind("127.0.0.1:0", server, HttpOptions::default())
            .unwrap()
            .with_limits(Limits {
                max_body: 4096,
                max_connections: 2,
                request_timeout: Duration::from_secs(10),
                ..small_limits()
            });
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || server.serve());
        let request = |body: &str| {
            format!(
                "POST /mcp HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
        };
        let call = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_model_summary","arguments":{"path":"x.ugoeproj"}}}"#;
        // 上限より多く panic させても、接続の枠は戻る (`503` にならない)。
        for _ in 0..4 {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream.write_all(request(call).as_bytes()).unwrap();
            let mut response = Vec::new();
            let _ = stream.read_to_end(&mut response);
        }
        let ping = r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#;
        let mut status = 0;
        for _ in 0..50 {
            status = exchange(addr, request(ping).as_bytes());
            if status == 200 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(status, 200);
    }
}
