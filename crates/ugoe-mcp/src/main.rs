//! `ugoe-mcp`: stdio (既定) か HTTP で動く MCP サーバ。使い方は `crates/ugoe-mcp/README.md`。

use std::io::{BufRead, Write};

use anyhow::Context;
use clap::Parser;
use ugoe_mcp::http::{ENDPOINT, HttpOptions, HttpServer, generate_token};
use ugoe_mcp::{Options, Server};

/// トークンを渡す環境変数 (コマンド行に書くとほかの利用者から見えることがあるので、こちらを勧める)。
const TOKEN_ENV: &str = "UGOE_MCP_TOKEN";

/// ugoe の .ugoeproj を読み、編集を提案・適用する MCP サーバ (stdio か HTTP)。
#[derive(Parser)]
#[command(name = "ugoe-mcp", version)]
struct Cli {
    /// 読むだけにする (適用・Undo・Redo・保存のツールを出さない)
    #[arg(long)]
    read_only: bool,
    /// 描画に使う wgpu のバックエンド (vulkan / metal / dx12 / gl。カンマ区切りで複数)。省略時は WGPU_BACKEND、それも無ければ wgpu の既定
    #[arg(long)]
    backend: Option<String>,
    /// stdio の代わりに HTTP (MCP の Streamable HTTP) で待ち受ける。`127.0.0.1:8765` の形か、ポート番号だけ (ループバック)。エンドポイントは /mcp
    #[arg(long, value_name = "ADDR")]
    http: Option<String>,
    /// HTTP の要求に求めるトークン (32 文字以上)。環境変数 UGOE_MCP_TOKEN でも渡せる。ループバック以外で待ち受けるときと、トンネルを通すときに要る
    #[arg(long, requires = "http")]
    token: Option<String>,
    /// HTTP の要求に求めるトークンを OS の乱数で作り (256 ビット)、標準エラーに書く
    #[arg(long, requires = "http", conflicts_with = "token")]
    generate_token: bool,
    /// ループバックのほかに受け付ける Origin (`https://example.com` の形。繰り返せる)
    #[arg(long = "allow-origin", value_name = "ORIGIN", requires = "http")]
    allow_origins: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let server = Server::new(Options {
        read_only: cli.read_only,
        backend: cli.backend,
    });
    match cli.http {
        Some(addr) => {
            let token = if cli.generate_token {
                Some(generate_token()?)
            } else {
                cli.token
                    .or_else(|| std::env::var(TOKEN_ENV).ok().filter(|t| !t.is_empty()))
            };
            serve_http(&addr, server, token, cli.generate_token, cli.allow_origins)
        }
        None => serve_stdio(server),
    }
}

fn serve_stdio(mut server: Server) -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    // 標準出力には MCP のメッセージだけを書く。ログは標準エラーへ。
    eprintln!("ugoe-mcp {} ready (stdio)", env!("CARGO_PKG_VERSION"));
    for line in stdin.lines() {
        let line = line.context("failed to read stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle_line(&line) {
            writeln!(stdout, "{response}").context("failed to write stdout")?;
            stdout.flush().context("failed to flush stdout")?;
        }
    }
    Ok(())
}

fn serve_http(
    addr: &str,
    server: Server,
    token: Option<String>,
    show_token: bool,
    allowed_origins: Vec<String>,
) -> anyhow::Result<()> {
    let auth = if token.is_some() {
        "token required"
    } else {
        "loopback only, no token"
    };
    let http = HttpServer::bind(
        addr,
        server,
        HttpOptions {
            token: token.clone(),
            allowed_origins,
        },
    )?;
    let local = http
        .local_addr()
        .context("failed to get the local address")?;
    eprintln!(
        "ugoe-mcp {} listening on http://{local}{ENDPOINT} ({auth})",
        env!("CARGO_PKG_VERSION")
    );
    if show_token && let Some(token) = token {
        // 作ったトークンは利用者に伝えるしかない。標準エラーはログに残りうるので、見せる相手に注意する。
        eprintln!("ugoe-mcp: generated token (keep it secret): {token}");
        eprintln!(
            "ugoe-mcp: send it as \"Authorization: Bearer <token>\", or use \
             http://{local}{ENDPOINT}/<token> for clients that cannot send headers"
        );
    }
    http.serve().context("the HTTP server stopped")
}
