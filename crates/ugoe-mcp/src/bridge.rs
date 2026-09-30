//! エディタに組み込む MCP サーバ (ADR 0023 の 1)。
//!
//! エディタは、開いている文書を貸す持ち主 ([`crate::host::DocumentHost`]) を作り、
//! [`EditorBridge::start`] でループバックの HTTP (T0514 と同じ Streamable HTTP) で待ち受ける。
//! 起動中は発見用のファイル ([`crate::discovery`]) を書き、[`EditorBridge::stop`] か捨てたときに消す。
//! `ugoe-mcp --editor` ([`crate::relay`]) はそのファイルを読んでつなぐ。
//!
//! 守り:
//! - 待ち受けは `127.0.0.1` だけ。トークンは必須 (渡さなければ OS の乱数で作る)。`Origin` は
//!   ループバックだけ受ける (ブラウザのページからの要求を断る)。
//! - パスの制限 (`--root` 相当) は [`BridgeOptions::options`] の `root` で決める。既定は制限しない:
//!   エディタでは、ほかのプロジェクトを開く・閉じるは持ち主が断り、保存 (書き出し先を含む) は利用者の
//!   確認を通すので、読み書きの先は利用者が見る。読むだけのツール (`import_psd`・`get_psd_layers` の PSD、
//!   `render_png` のモーションの JSON) は、トークンを持つクライアントなら利用者の権限で読める
//!   (エージェントは普通、同じ利用者の権限でファイルを読める)。限りたいなら、プロジェクトの
//!   ディレクトリなどを `root` に渡す。

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::discovery::{self, Discovery};
use crate::host::DocumentHost;
use crate::http::{HttpError, HttpOptions, HttpServer, RunningHttpServer, generate_token};
use crate::server::Server;
use crate::tools::Options;

/// エディタの MCP サーバの設定。
#[derive(Debug, Clone, Default)]
pub struct BridgeOptions {
    /// ツールの設定 (読むだけにする・描画のバックエンド・パスの制限)。
    pub options: Options,
    /// 待ち受けるポート (`127.0.0.1`)。0 なら OS が空いているポートを選ぶ。
    pub port: u16,
    /// 要求に求めるトークン (32 文字以上)。`None` なら作る。
    pub token: Option<String>,
    /// 発見用のファイルのパス。`None` なら既定の場所 ([`discovery::default_path`])。
    pub discovery: Option<PathBuf>,
}

/// エディタの MCP サーバを始められない。
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error("failed to start the MCP server: {0}")]
    Io(#[from] io::Error),
    #[error("cannot tell where to write the discovery file (set UGOE_MCP_DISCOVERY)")]
    NoDiscoveryPath,
    #[error("failed to write the discovery file {}: {source}", .path.display())]
    Discovery { path: PathBuf, source: io::Error },
}

/// 待ち受けているエディタの MCP サーバ。捨てると止まり、発見用のファイルを消す。
#[derive(Debug)]
pub struct EditorBridge {
    server: Option<RunningHttpServer>,
    discovery: Discovery,
    discovery_path: PathBuf,
}

impl EditorBridge {
    /// `host` の文書を扱う MCP サーバを別のスレッドで始め、発見用のファイルを書く。
    pub fn start(host: Arc<dyn DocumentHost>, options: BridgeOptions) -> Result<Self, BridgeError> {
        let token = match options.token {
            Some(token) => token,
            None => generate_token()?,
        };
        let discovery_path = match options.discovery {
            Some(path) => path,
            None => discovery::default_path().ok_or(BridgeError::NoDiscoveryPath)?,
        };
        let server = Server::with_host(options.options, host);
        let http = HttpServer::bind(
            &format!("127.0.0.1:{}", options.port),
            server,
            HttpOptions {
                token: Some(token.clone()),
                allowed_origins: Vec::new(),
            },
        )?;
        let port = http.local_addr()?.port();
        let discovery = Discovery::new(port, token);
        let running = http.spawn()?;
        if let Err(source) = discovery::write(&discovery_path, &discovery) {
            running.stop();
            return Err(BridgeError::Discovery {
                path: discovery_path,
                source,
            });
        }
        Ok(Self {
            server: Some(running),
            discovery,
            discovery_path,
        })
    }

    /// 待ち受けているアドレス。
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.server.as_ref().map(RunningHttpServer::local_addr)
    }

    /// MCP の URL (`http://127.0.0.1:<port>/mcp`)。
    pub fn url(&self) -> String {
        self.discovery.url()
    }

    /// 要求に求めるトークン (画面でクライアントの登録を示すため)。
    pub fn token(&self) -> &str {
        &self.discovery.token
    }

    /// 書いた発見用のファイル。
    pub fn discovery_path(&self) -> &Path {
        &self.discovery_path
    }

    /// 止めて、発見用のファイルを消す (ほかのエディタが書き換えていれば残す)。処理中の要求は待たない。
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop();
            if let Err(e) = discovery::remove_if_ours(&self.discovery_path, &self.discovery) {
                eprintln!(
                    "ugoe-mcp: failed to remove {}: {e}",
                    self.discovery_path.display()
                );
            }
        }
    }
}

impl Drop for EditorBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}
