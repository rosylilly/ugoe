//! `ugoe` コマンド。人間と AI エージェントの双方が、GUI 無しでモデルを検査・変換・確認するための道具。

use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ugoe", version, about = "ugoe toolchain CLI")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// モデル JSON を検証する。
    Validate { path: PathBuf },
    /// モデル JSON の概要 (パラメータ・パーツ・メッシュ数など) を表示する。
    Info { path: PathBuf },
    /// 組み込みフィクスチャを JSON で標準出力に書き出す。
    Fixture {
        #[arg(value_parser = ["single-quad"])]
        name: String,
    },
    /// 標準パラメータ一覧を表示する。
    StandardParams,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Cmd::Validate { path } => {
            let model = load(&path)?;
            let errors = ugoe_model::validate::validate(&model);
            if errors.is_empty() {
                println!("ok");
            } else {
                for e in &errors {
                    eprintln!("error: {e}");
                }
                bail!("{} validation error(s)", errors.len());
            }
        }
        Cmd::Info { path } => {
            let m = load(&path)?;
            println!("name:       {}", m.meta.name);
            println!(
                "canvas:     {}x{}",
                m.meta.canvas.width, m.meta.canvas.height
            );
            println!("parameters: {}", m.parameters.len());
            println!("parts:      {}", m.parts.len());
            println!("deformers:  {}", m.deformers.len());
            println!("meshes:     {}", m.meshes.len());
        }
        Cmd::Fixture { name } => {
            let model = match name.as_str() {
                "single-quad" => ugoe_model::fixtures::single_quad(),
                _ => unreachable!(),
            };
            println!("{}", ugoe_format::model_to_json(&model)?);
        }
        Cmd::StandardParams => {
            for (id, min, default, max) in ugoe_model::standard::ALL {
                println!("{id:<12} {min:>6} {default:>6} {max:>6}");
            }
        }
    }
    Ok(())
}

fn load(path: &PathBuf) -> anyhow::Result<ugoe_model::Model> {
    let json =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(ugoe_format::model_from_json(&json)?)
}
