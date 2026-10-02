//! `ugoe-conformance`: 配布物の適合テストの参照の生成と比べ合わせ (tests/conformance/README.md)。
//!
//! ```sh
//! ugoe-conformance reference --out DIR [--only CASE,...]   # DIR/plan.json・plan.txt・DIR/reference/<ケース>.json・DIR/models/
//! ugoe-conformance compare --out DIR --binding c [--images | --images-only]
//! ```
//!
//! 実行器は、扱えないケースを `DIR/<バインディング>/skipped.json` (`{"<ケース>": "理由"}`) に書いてよい。
//! 比べ合わせはそのケースを飛ばしたと表示し、失敗には数えない。
//!
//! 参照の画像 (`DIR/reference/<ケース>.png`) は `ugoe render` で作る (`plan.json` の `render_args`。`run.mjs` が実行する)。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use ugoe_conformance::frame::Frame;
use ugoe_conformance::plan::Plan;
use ugoe_conformance::{bundles, compare, reference, repo_root};

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
        /// Keep only these cases (comma-separated names)
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
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
        /// Compare only the images (for bindings that cannot read the evaluation results back);
        /// cases without `image: true` are skipped
        #[arg(long, conflicts_with = "images")]
        images_only: bool,
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
        Command::Reference { out, cases, only } => {
            let root = repo_root();
            let cases = cases.unwrap_or_else(|| root.join("tests/conformance/cases.json"));
            let models = out.join("models");
            bundles::write_all(&models)?;
            let mut plan = Plan::load(&cases, &root, &models)?;
            if !only.is_empty() {
                if let Some(unknown) = only
                    .iter()
                    .find(|n| !plan.cases.iter().any(|c| &c.name == *n))
                {
                    bail!("unknown case `{unknown}`");
                }
                plan.cases.retain(|c| only.contains(&c.name));
            }
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
            images_only,
        } => {
            let mode = match (images, images_only) {
                (_, true) => Mode::ImagesOnly,
                (true, false) => Mode::FramesAndImages,
                (false, false) => Mode::Frames,
            };
            compare_binding(&out, &binding, mode)
        }
    }
}

/// 受け手として進める口 (`Puppet::follow`) を実行器から呼べるバインディング (T0133)。
/// Unity と Godot はまだ口を出していないので、`follow` のケースを飛ばす。
const FOLLOW_BINDINGS: [&str; 3] = ["c", "wasm", "wasm-webgpu"];

/// 何を比べるか。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Frames,
    FramesAndImages,
    ImagesOnly,
}

fn compare_binding(out: &Path, binding: &str, mode: Mode) -> anyhow::Result<bool> {
    let plan: Plan = serde_json::from_slice(&read(&out.join("plan.json"))?)?;
    let actual_dir = out.join(binding);
    if !actual_dir.is_dir() {
        bail!("{} does not exist", actual_dir.display());
    }
    // 実行器が扱えないとしたケース (`{"<ケース>": "理由"}`)。
    let skipped_path = actual_dir.join("skipped.json");
    let mut skipped: BTreeMap<String, String> = if skipped_path.exists() {
        serde_json::from_slice(&read(&skipped_path)?)
            .with_context(|| format!("parsing {}", skipped_path.display()))?
    } else {
        BTreeMap::new()
    };
    // 受け手として進める口 (`follow`。T0133) を持つバインディング。ほかはそのケースを飛ばす。
    if !FOLLOW_BINDINGS.contains(&binding) {
        for case in plan.cases.iter().filter(|c| !c.follow.is_empty()) {
            skipped
                .entry(case.name.clone())
                .or_insert_with(|| "the binding does not expose follow yet".to_owned());
        }
    }
    if mode == Mode::ImagesOnly {
        for case in plan.cases.iter().filter(|c| !c.image) {
            skipped
                .entry(case.name.clone())
                .or_insert_with(|| "the case has no image".to_owned());
        }
    }
    let mut failed = 0;
    let mut compared = 0;
    for case in &plan.cases {
        if let Some(reason) = skipped.get(&case.name) {
            println!("skip {} ({reason})", case.name);
            continue;
        }
        compared += 1;
        let mut errors = Vec::new();
        if mode != Mode::ImagesOnly {
            let reference: Frame = serde_json::from_slice(&read(
                &out.join("reference").join(format!("{}.json", case.name)),
            )?)?;
            let path = actual_dir.join(format!("{}.json", case.name));
            match read(&path).and_then(|b| Ok(serde_json::from_slice::<Frame>(&b)?)) {
                Ok(actual) => errors.extend(compare::frames(&reference, &actual)),
                Err(e) => errors.push(format!("{e:#}")),
            }
        }
        let mut note = String::new();
        if mode != Mode::Frames && case.image {
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
    let skips = plan.cases.len() - compared;
    println!(
        "{binding}: {} of {compared} cases passed{}",
        compared - failed,
        if skips > 0 {
            format!(" ({skips} skipped)")
        } else {
            String::new()
        }
    );
    if compared == 0 {
        bail!("{binding}: no case was compared");
    }
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
