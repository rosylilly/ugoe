//! `ugoe-conformance`: 配布物の適合テストの参照の生成と比べ合わせ (tests/conformance/README.md)。
//!
//! ```sh
//! ugoe-conformance reference --out DIR          # DIR/plan.json・plan.txt と DIR/reference/<ケース>.json
//! ugoe-conformance compare --out DIR --binding c [--images]
//! ```
//!
//! 参照の画像 (`DIR/reference/<ケース>.png`) は `ugoe render` で作る (`plan.json` の `render_args`。`run.mjs` が実行する)。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use ugoe_conformance::frame::Frame;
use ugoe_conformance::plan::Plan;
use ugoe_conformance::{compare, reference, repo_root};

#[derive(Parser)]
#[command(
    name = "ugoe-conformance",
    about = "Conformance tests of the ugoe distributions"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Expand the cases into a plan and write the reference evaluation results
    Reference {
        /// Output directory
        #[arg(long)]
        out: PathBuf,
        /// Case list (default: tests/conformance/cases.json)
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Compare the results of a binding (DIR/<binding>/) with the reference (DIR/reference/)
    Compare {
        /// Directory written by `reference`
        #[arg(long)]
        out: PathBuf,
        /// Name of the binding (the subdirectory with its results)
        #[arg(long)]
        binding: String,
        /// Also compare the images (<case>.png) of the cases that have `image: true`
        #[arg(long)]
        images: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<bool> {
    match cli.command {
        Command::Reference { out, cases } => {
            let root = repo_root();
            let cases = cases.unwrap_or_else(|| root.join("tests/conformance/cases.json"));
            let plan = Plan::load(&cases, &root)?;
            let dir = out.join("reference");
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            write(&out.join("plan.json"), &serde_json::to_vec_pretty(&plan)?)?;
            write(&out.join("plan.txt"), plan.to_text().as_bytes())?;
            for case in &plan.cases {
                let frame = reference::evaluate(case)?;
                write(
                    &dir.join(format!("{}.json", case.name)),
                    &serde_json::to_vec(&frame)?,
                )?;
            }
            println!("{} cases: {}", plan.cases.len(), out.display());
            Ok(true)
        }
        Command::Compare {
            out,
            binding,
            images,
        } => compare_binding(&out, &binding, images),
    }
}

fn compare_binding(out: &Path, binding: &str, images: bool) -> anyhow::Result<bool> {
    let plan: Plan = serde_json::from_slice(&read(&out.join("plan.json"))?)?;
    let actual_dir = out.join(binding);
    if !actual_dir.is_dir() {
        bail!("{} does not exist", actual_dir.display());
    }
    let mut failed = 0;
    for case in &plan.cases {
        let mut errors = Vec::new();
        let reference: Frame = serde_json::from_slice(&read(
            &out.join("reference").join(format!("{}.json", case.name)),
        )?)?;
        let path = actual_dir.join(format!("{}.json", case.name));
        match read(&path).and_then(|b| Ok(serde_json::from_slice::<Frame>(&b)?)) {
            Ok(actual) => errors.extend(compare::frames(&reference, &actual)),
            Err(e) => errors.push(format!("{e:#}")),
        }
        let mut note = String::new();
        if images && case.image {
            match compare_image(out, &actual_dir, &case.name) {
                Ok(c) => {
                    note = format!(
                        " (image: {:.3}% matching, worst {}/255)",
                        c.matching * 100.0,
                        c.worst
                    );
                    if !c.passes() {
                        errors.push(format!(
                            "image: only {:.3}% of the pixels are within {}/255 (need {}%)",
                            c.matching * 100.0,
                            compare::PIXEL_TOLERANCE,
                            compare::PIXEL_RATIO * 100.0
                        ));
                    }
                }
                Err(e) => errors.push(format!("image: {e:#}")),
            }
        }
        if errors.is_empty() {
            println!("ok   {}{note}", case.name);
        } else {
            failed += 1;
            println!("FAIL {}{note}", case.name);
            for e in errors.iter().take(10) {
                println!("       {e}");
            }
            if errors.len() > 10 {
                println!("       ... {} more", errors.len() - 10);
            }
        }
    }
    println!(
        "{binding}: {} of {} cases passed",
        plan.cases.len() - failed,
        plan.cases.len()
    );
    Ok(failed == 0)
}

/// `<ケース>.png` を比べる。落ちたら `<ケース>.diff.png` を書く。
fn compare_image(
    out: &Path,
    actual_dir: &Path,
    name: &str,
) -> anyhow::Result<compare::ImageComparison> {
    let decode = |path: &Path| -> anyhow::Result<ugoe_headless::Rgba8> {
        ugoe_headless::image::decode_png(&read(path)?)
            .with_context(|| format!("decoding {}", path.display()))
    };
    let reference = decode(&out.join("reference").join(format!("{name}.png")))?;
    let actual = decode(&actual_dir.join(format!("{name}.png")))?;
    if (reference.width, reference.height) != (actual.width, actual.height) {
        bail!(
            "size is {}x{}, expected {}x{}",
            actual.width,
            actual.height,
            reference.width,
            reference.height
        );
    }
    let result =
        compare::images(&reference.pixels, &actual.pixels).context("images of different sizes")?;
    if !result.passes() {
        let diff = ugoe_headless::Rgba8 {
            width: reference.width,
            height: reference.height,
            pixels: compare::diff_image(&reference.pixels, &actual.pixels),
        };
        let png = ugoe_headless::image::encode_png_unpremultiplied(&diff)?;
        write(&actual_dir.join(format!("{name}.diff.png")), &png)?;
    }
    Ok(result)
}

fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("reading {}", path.display()))
}

fn write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}
