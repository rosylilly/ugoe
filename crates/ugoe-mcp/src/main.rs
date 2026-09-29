//! `ugoe-mcp`: stdio で動く MCP サーバ。使い方は `crates/ugoe-mcp/README.md`。

use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use ugoe_mcp::{Options, Server};

/// ugoe の .ugoeproj を読み、編集を提案・適用する MCP サーバ (stdio)。
#[derive(Parser)]
#[command(name = "ugoe-mcp", version)]
struct Cli {
    /// 読むだけにする (適用・Undo・Redo・保存のツールを出さない)
    #[arg(long)]
    read_only: bool,
    /// 描画に使う `ugoe` (ugoe-cli) の実行ファイル。省略時は UGOE_BIN、このサーバの隣、PATH の順に探す
    #[arg(long, value_name = "PATH")]
    ugoe_bin: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut server = Server::new(Options {
        read_only: cli.read_only,
        ugoe_bin: cli.ugoe_bin,
    });
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
