//! `ugoe-mcp`: stdio (既定) か HTTP で動く MCP サーバ、または実行中のエディタへの中継 (`--editor`)。
//! 使い方は `crates/ugoe-mcp/README.md`。

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use ugoe_mcp::http::{ENDPOINT, HttpOptions, HttpServer, generate_token};
use ugoe_mcp::mcpb::{Bundle, Platform};
use ugoe_mcp::relay::{Relay, RelayOptions};
use ugoe_mcp::verbatim::plain_path;
use ugoe_mcp::{Options, Root, Server};

/// トークンを渡す環境変数 (コマンド行に書くとほかの利用者から見えることがあるので、こちらを勧める)。
const TOKEN_ENV: &str = "UGOE_MCP_TOKEN";

/// ugoe の .ugoeproj を読み、編集を提案・適用する MCP サーバ (stdio か HTTP)。
#[derive(Parser)]
#[command(name = "ugoe-mcp", version)]
struct Cli {
    /// 実行中の ugoe エディタにつなぐ: stdio のメッセージを、エディタの MCP サーバへ中継する (エディタの設定で「エージェントをつなぐ」を有効にしておく)
    #[arg(long, conflicts_with_all = ["read_only", "backend", "gpu_timeout", "root", "allow_any_path", "http"])]
    editor: bool,
    /// --editor で読む発見用のファイル (既定は OS ごとの場所。環境変数 UGOE_MCP_DISCOVERY でも変えられる)
    #[arg(long, value_name = "FILE", requires = "editor")]
    discovery: Option<PathBuf>,
    /// --editor で、エージェントが動く WSL のディストリビューションの名前 (Windows 版を WSL から使うとき。WSL のパスを Windows のパスに変える)
    #[arg(long, value_name = "NAME", requires = "editor")]
    wsl_distro: Option<String>,
    /// 読むだけにする (適用・Undo・Redo・保存のツールを出さない)
    #[arg(long)]
    read_only: bool,
    /// 描画に使う wgpu のバックエンド (vulkan / metal / dx12 / gl。カンマ区切りで複数)。省略時は WGPU_BACKEND、それも無ければ wgpu の既定
    #[arg(long)]
    backend: Option<String>,
    /// 描画 (render_png) で GPU を待つ時間の上限の秒数 (既定 30)。超えたらツールのエラーにして GPU を作り直す。遅いソフトウェアの GPU (lavapipe・llvmpipe) で大きな画を描くときは延ばす。0 は上限なし
    #[arg(long, value_name = "SECONDS", value_parser = ugoe_headless::args::parse_gpu_timeout)]
    gpu_timeout: Option<Duration>,
    /// ツールが読み書きするパスをこのディレクトリの下に限る (相対パスもここから解決する)。--http では、これか --allow-any-path が要る
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    /// --http で、--root を付けずに任意のパスを読み書きさせる (利用者の権限でどのファイルにも届くので、勧めない)
    #[arg(long, requires = "http", conflicts_with = "root")]
    allow_any_path: bool,
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
    /// サーバを動かさず、この実行ファイルの `--editor` を包む Claude Desktop 用の MCP Bundle (.mcpb) を書く
    #[arg(long, value_name = "OUT", conflicts_with_all = ["editor", "read_only", "backend", "gpu_timeout", "root", "http"])]
    mcpb: Option<PathBuf>,
    /// --mcpb で束に入れるアイコン (PNG。512×512 を勧める)
    #[arg(long, value_name = "PNG", requires = "mcpb")]
    mcpb_icon: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(out) = cli.mcpb {
        return pack_mcpb(&out, cli.mcpb_icon.as_deref());
    }
    if cli.editor {
        return relay(RelayOptions {
            discovery: cli.discovery,
            wsl_distro: cli.wsl_distro,
        });
    }
    // HTTP ではほかのプログラムやリモートのクライアントがツールを呼べるので、パスを限るかを明示させる
    // (--read-only でも任意の PSD やプロジェクトを読んで画像で返せるので、それだけでは足りない)。
    if cli.http.is_some() && cli.root.is_none() && !cli.allow_any_path {
        anyhow::bail!(
            "--http needs --root <DIR> to confine the paths the tools can read and write \
             (or --allow-any-path to let them reach any path you can)"
        );
    }
    let root = cli.root.as_deref().map(Root::new).transpose()?;
    let paths = match &root {
        Some(root) => format!(
            "paths confined to {}",
            plain_path(&root.dir().to_string_lossy())
        ),
        None => "any path".to_owned(),
    };
    let server = Server::new(Options {
        read_only: cli.read_only,
        backend: cli.backend,
        root,
        gpu_timeout: cli.gpu_timeout,
    });
    match cli.http {
        Some(addr) => {
            let token = if cli.generate_token {
                Some(generate_token()?)
            } else {
                cli.token
                    .or_else(|| std::env::var(TOKEN_ENV).ok().filter(|t| !t.is_empty()))
            };
            serve_http(
                &addr,
                server,
                token,
                cli.generate_token,
                cli.allow_origins,
                &paths,
            )
        }
        None => serve_stdio(server, &paths),
    }
}

/// この実行ファイルを包む `.mcpb` を書く (`ugoe_mcp::mcpb`)。
fn pack_mcpb(out: &Path, icon: Option<&Path>) -> anyhow::Result<()> {
    let platform =
        Platform::current().context("MCP Bundles run only on macOS, Windows and Linux")?;
    let exe = std::env::current_exe().context("failed to find this executable")?;
    let mut bundle = Bundle::new(exe, platform);
    if let Some(icon) = icon {
        bundle.icon = Some(
            std::fs::read(icon).with_context(|| format!("failed to read {}", icon.display()))?,
        );
    }
    bundle.write(out)?;
    eprintln!(
        "ugoe-mcp: wrote {} ({}; runs `ugoe-mcp --editor`)",
        out.display(),
        platform.as_str()
    );
    Ok(())
}

fn relay(options: RelayOptions) -> anyhow::Result<()> {
    let relay = Relay::new(options);
    // 標準出力には MCP のメッセージだけを書く。ログは標準エラーへ。
    eprintln!(
        "ugoe-mcp {} --editor: {}",
        env!("CARGO_PKG_VERSION"),
        relay.describe()
    );
    relay
        .run(std::io::stdin().lock(), std::io::stdout())
        .context("failed to relay stdio")
}

fn serve_stdio(server: Server, paths: &str) -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    // 標準出力には MCP のメッセージだけを書く。ログは標準エラーへ。
    eprintln!(
        "ugoe-mcp {} ready (stdio; {paths})",
        env!("CARGO_PKG_VERSION")
    );
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
    paths: &str,
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
        "ugoe-mcp {} listening on http://{local}{ENDPOINT} ({auth}; {paths})",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu_timeout(args: &[&str]) -> Result<Option<Duration>, clap::Error> {
        Cli::try_parse_from(std::iter::once("ugoe-mcp").chain(args.iter().copied()))
            .map(|cli| cli.gpu_timeout)
    }

    #[test]
    fn parses_the_gpu_timeout() {
        assert_eq!(gpu_timeout(&[]).unwrap(), None);
        assert_eq!(
            gpu_timeout(&["--gpu-timeout", "120"]).unwrap(),
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            gpu_timeout(&["--gpu-timeout", "0.5"]).unwrap(),
            Some(Duration::from_millis(500))
        );
        // 0 は上限なし。
        assert_eq!(
            gpu_timeout(&["--gpu-timeout", "0"]).unwrap(),
            Some(Duration::MAX)
        );
        for bad in ["-1", "NaN", "inf", "1e300", "abc", ""] {
            let arg = format!("--gpu-timeout={bad}");
            let err = gpu_timeout(&[&arg]).unwrap_err();
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ValueValidation,
                "{bad}: {err}"
            );
        }
        // 描画しない形 (中継・束の書き出し) とは一緒に使えない。
        for args in [
            &["--editor", "--gpu-timeout", "10"][..],
            &["--mcpb", "x.mcpb", "--gpu-timeout", "10"],
        ] {
            let err = gpu_timeout(args).unwrap_err();
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{args:?}: {err}"
            );
        }
    }
}
