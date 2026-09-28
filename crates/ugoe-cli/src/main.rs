//! `ugoe` コマンド。人間と AI エージェントの双方が、GUI 無しでモデルを検査・変換・確認するための道具。

mod animate;
mod fixtures;
mod image;
mod render;
mod sample;

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
use ugoe_format::{TextureData, TextureFormat};
use ugoe_model::Model;

use crate::image::Rgba8;
use crate::render::Fit;

/// `INPUT` の書き方 (validate / info / render で共通)。
const INPUT_HELP: &str =
    "Model JSON (.json), bundle (.ugoe), or built-in fixture as `fixture:<name>`";

#[derive(Debug, Parser)]
#[command(name = "ugoe", version, about = "ugoe toolchain CLI")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// モデルを検証する。
    Validate {
        #[arg(help = INPUT_HELP)]
        input: String,
        /// 構造の検証に加えて、プロファイルへの準拠を検査する。
        #[arg(long, value_enum)]
        profile: Option<Profile>,
    },
    /// モデルの概要 (パラメータ・パーツ・メッシュ数など) を表示する。
    Info {
        #[arg(help = INPUT_HELP)]
        input: String,
        /// 機械で読みやすい JSON で出力する。
        #[arg(long)]
        json: bool,
    },
    /// 組み込みフィクスチャを JSON で標準出力に書き出す。名前を省略するか `list` で一覧を表示する。
    Fixture { name: Option<String> },
    /// 標準パラメータとアバタープロファイルの拡張パラメータの一覧を表示する。
    StandardParams,
    /// ヘッドレスの GPU でモデルを描画して PNG に書き出す。
    Render(RenderArgs),
    /// PSD を取り込んで新しい `.ugoeproj` プロジェクトに保存する。
    ImportPsd(ImportPsdArgs),
    /// 見本のモデルを生成する。
    #[command(subcommand)]
    Sample(SampleCmd),
}

#[derive(Debug, Subcommand)]
enum SampleCmd {
    /// アバタープロファイル (レベル 3) に準拠した見本のモデルを、PSD からコマンドの列で組み立てる。
    Avatar(SampleAvatarArgs),
}

#[derive(Debug, Args)]
struct SampleAvatarArgs {
    /// 書き出す `.ugoe` バンドル。
    #[arg(short, long)]
    output: PathBuf,
    /// 入力の PSD (既定はリポジトリのルートからの相対パス)。
    #[arg(long, default_value = sample::DEFAULT_PSD)]
    psd: PathBuf,
    /// `.ugoeproj` プロジェクトにも保存する (エディタで開く用)。
    #[arg(long, value_name = "DIR")]
    project: Option<PathBuf>,
}

/// `ugoe validate --profile` で検査するプロファイル。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Profile {
    /// アバタープロファイル (`docs/spec/avatar-profile.md`)。
    Avatar,
}

#[derive(Debug, Args)]
struct ImportPsdArgs {
    /// 取り込む PSD ファイル。
    input: PathBuf,
    /// 保存する `.ugoeproj` ディレクトリ。
    #[arg(short, long)]
    output: PathBuf,
    /// `.ugoe` バンドルにも書き出す。
    #[arg(long, value_name = "PATH")]
    bundle: Option<PathBuf>,
    /// プロジェクトと最上位のパーツの名前 (既定は PSD のファイル名)。
    #[arg(long)]
    name: Option<String>,
    /// 自動メッシュ: この値以上のアルファを中身とする。
    #[arg(long)]
    alpha_threshold: Option<u8>,
    /// 自動メッシュ: 輪郭を外に広げる距離 (px)。
    #[arg(long)]
    margin: Option<f32>,
    /// 自動メッシュ: 輪郭上の頂点間隔 (px)。
    #[arg(long)]
    edge_spacing: Option<f32>,
    /// 自動メッシュ: 内部点の間隔 (px。0 なら内部点なし)。
    #[arg(long)]
    interior_spacing: Option<f32>,
}

#[derive(Debug, Args)]
struct RenderArgs {
    #[arg(help = INPUT_HELP)]
    input: String,
    /// 出力する PNG のパス。
    #[arg(short, long)]
    output: PathBuf,
    /// パラメータの値 (複数指定可)。値域の外はクランプされる。
    #[arg(long = "param", value_name = "ID=VALUE", value_parser = render::parse_param)]
    params: Vec<(String, f32)>,
    /// 出力の大きさ。
    #[arg(long, value_name = "WxH", value_parser = render::parse_size,
          conflicts_with_all = ["width", "height", "scale"])]
    size: Option<(u32, u32)>,
    /// 出力の幅。`--height` が無ければ高さはキャンバスの縦横比から決める。
    #[arg(long, value_parser = render::parse_dimension, conflicts_with = "scale")]
    width: Option<u32>,
    /// 出力の高さ。`--width` が無ければ幅はキャンバスの縦横比から決める。
    #[arg(long, value_parser = render::parse_dimension, conflicts_with = "scale")]
    height: Option<u32>,
    /// 大きさを指定しないときの、キャンバスに対する倍率。
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
    /// キャンバスを出力にどう収めるか。
    #[arg(long, value_enum, default_value_t = Fit::Contain)]
    fit: Fit,
    /// 背景色。`transparent` (既定)、`white`、`black`、`#RRGGBB`、`#RRGGBBAA`。
    #[arg(long, value_name = "COLOR", default_value = "transparent",
          value_parser = render::parse_color)]
    background: [u8; 4],
    /// テクスチャを PNG ファイルで差し替える (複数指定可)。
    #[arg(long = "texture", value_name = "INDEX=PATH", value_parser = render::parse_texture)]
    textures: Vec<(u32, PathBuf)>,
    /// この秒数だけ `ugoe::Puppet` で時間を進めてから描く (物理・モーション・表情・目パチ・呼吸・ポーズ)。
    /// `--motion` / `--expression` だけを指定すると 0 秒。どれも無ければ時間を進めずにパラメータだけで描く。
    #[arg(long, value_name = "SECONDS", value_parser = animate::parse_time)]
    time: Option<f32>,
    /// 再生するモーション: バンドルのモーションの名前か、`.motion.json` のパス。
    #[arg(long, value_name = "NAME|PATH")]
    motion: Option<String>,
    /// 使う表情: バンドルの表情の名前か、`.expression.json` のパス。
    #[arg(long, value_name = "NAME|PATH")]
    expression: Option<String>,
    /// 物理の設定 (JSON) のパス。省略するとバンドルの物理を使う。
    #[arg(long, value_name = "PATH", conflicts_with = "no_physics")]
    physics: Option<PathBuf>,
    /// 時間を進めるときに物理を使わない。
    #[arg(long)]
    no_physics: bool,
    /// 時間を進めるときに目パチと呼吸を止める。
    #[arg(long)]
    no_effects: bool,
    /// 時間を進めるときの 1 秒あたりの `update` の回数。
    #[arg(long, default_value = "60", value_parser = animate::parse_fps)]
    fps: f32,
    /// wgpu のバックエンド (vulkan / metal / dx12 / gl。カンマ区切りで複数)。
    /// 省略時は環境変数 `WGPU_BACKEND`、それも無ければ wgpu の既定。
    #[arg(long)]
    backend: Option<String>,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Cmd::Validate { input, profile } => {
            let model = load_input(&input)?.model;
            let errors = ugoe_model::validate::validate(&model);
            for e in &errors {
                eprintln!("error: {e}");
            }
            let profile_errors = match profile {
                Some(Profile::Avatar) => check_avatar_profile(&model),
                None => 0,
            };
            if !errors.is_empty() {
                bail!("{} validation error(s)", errors.len());
            }
            if profile_errors > 0 {
                bail!("{profile_errors} avatar profile error(s)");
            }
            println!("ok");
        }
        Cmd::Info { input, json } => {
            let m = load_input(&input)?.model;
            if json {
                println!("{}", serde_json::to_string_pretty(&info_json(&m))?);
            } else {
                println!("name:       {}", m.meta.name);
                println!(
                    "canvas:     {}x{}",
                    m.meta.canvas.width, m.meta.canvas.height
                );
                println!("parameters: {}", m.parameters.len());
                println!("parts:      {}", m.parts.len());
                println!("deformers:  {}", m.deformers.len());
                println!("meshes:     {}", m.meshes.len());
                println!("textures:   {}", m.textures.len());
            }
        }
        Cmd::Fixture { name } => match name.as_deref() {
            None | Some("list") => {
                for name in fixtures::names() {
                    println!("{name}");
                }
            }
            Some(name) => {
                let model = fixtures::by_name(name)?;
                println!("{}", ugoe_format::model_to_json(&model)?);
            }
        },
        Cmd::StandardParams => {
            use ugoe_model::standard::{ALL, EXTENDED, VOWELS};
            let mut printed = Vec::new();
            for (id, min, default, max) in ALL.iter().chain(EXTENDED).chain(VOWELS) {
                if !printed.contains(id) {
                    println!("{id:<12} {min:>6} {default:>6} {max:>6}");
                    printed.push(id);
                }
            }
        }
        Cmd::Render(args) => run_render(args)?,
        Cmd::ImportPsd(args) => run_import_psd(args)?,
        Cmd::Sample(SampleCmd::Avatar(args)) => run_sample_avatar(args)?,
    }
    Ok(())
}

/// `ugoe validate --profile avatar`: アバタープロファイルへの準拠を検査し、問題を標準エラーに、
/// 主張したレベルと満たしているレベルを標準出力に書く。エラーの数を返す (警告は数えない)。
fn check_avatar_profile(model: &Model) -> usize {
    use ugoe_model::avatar::check::{Severity, check_profile};

    let report = check_profile(model);
    for issue in &report.issues {
        match issue.severity() {
            Severity::Error => eprintln!("error: {issue}"),
            Severity::Warning => eprintln!("warning: {issue}"),
        }
    }
    if let Some(claimed) = report.claimed {
        println!(
            "avatar profile: claims level {claimed}, satisfies level {}",
            report.satisfied
        );
    }
    report.errors().count()
}

/// `ugoe import-psd`: 空のプロジェクトに PSD を取り込み、保存する。取り込みの規則は
/// `ugoe_editor_core::import` (`docs/editor.md#psd-の取り込み`)。
fn run_import_psd(args: ImportPsdArgs) -> anyhow::Result<()> {
    use ugoe_editor_core::{AutoMeshOptions, Project, PsdImportOptions};

    let bytes =
        std::fs::read(&args.input).with_context(|| format!("reading {}", args.input.display()))?;
    let name = args.name.unwrap_or_else(|| {
        args.input
            .file_stem()
            .map_or_else(|| "PSD".to_owned(), |s| s.to_string_lossy().into_owned())
    });
    let defaults = AutoMeshOptions::default();
    let options = PsdImportOptions {
        name: name.clone(),
        mesh: AutoMeshOptions {
            alpha_threshold: args.alpha_threshold.unwrap_or(defaults.alpha_threshold),
            margin: args.margin.unwrap_or(defaults.margin),
            edge_spacing: args.edge_spacing.unwrap_or(defaults.edge_spacing),
            interior_spacing: args.interior_spacing.unwrap_or(defaults.interior_spacing),
            min_island_area: defaults.min_island_area,
        },
    };
    let empty = Project::empty(name);
    let mut document = empty.document();
    let import = ugoe_editor_core::import::import_psd(document.model(), &bytes, &options)
        .with_context(|| format!("importing {}", args.input.display()))?;
    for warning in &import.warnings {
        eprintln!("warning: {warning}");
    }
    document.apply(import.command)?;
    let project = Project::from_document(empty.meta, &document);
    project
        .save(&args.output)
        .with_context(|| format!("saving {}", args.output.display()))?;
    let model = document.model();
    println!(
        "wrote {} ({} parts, {} meshes, {}x{} canvas)",
        args.output.display(),
        model.parts.len(),
        model.meshes.len(),
        model.meta.canvas.width,
        model.meta.canvas.height
    );
    if let Some(path) = args.bundle {
        let bytes = ugoe_editor_core::export_bundle(&project)?;
        std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// `ugoe sample avatar`: 見本のモデルを組み立てて書き出す (`sample::build_avatar`)。
fn run_sample_avatar(args: SampleAvatarArgs) -> anyhow::Result<()> {
    let psd = std::fs::read(&args.psd).with_context(|| {
        format!(
            "reading {} (run from the repository root or pass --psd)",
            args.psd.display()
        )
    })?;
    let project = sample::build_avatar(&psd)?;
    let bytes = ugoe_editor_core::export_bundle(&project)?;
    std::fs::write(&args.output, &bytes)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!("wrote {} ({} bytes)", args.output.display(), bytes.len());
    if let Some(dir) = args.project {
        project
            .save(&dir)
            .with_context(|| format!("saving {}", dir.display()))?;
        println!("wrote {}", dir.display());
    }
    Ok(())
}

/// 読み込んだ入力。
struct Input {
    model: Model,
    /// バンドルのテクスチャ (`model.textures` と同じ順)。JSON とフィクスチャでは空。
    textures: Vec<TextureData>,
    /// バンドルの物理・モーション・表情。JSON とフィクスチャでは空。
    animation: animate::Animation,
}

/// `fixture:<name>`、`.ugoe` バンドル (先頭のマジックで判別)、モデル JSON のいずれかを読む。
fn load_input(input: &str) -> anyhow::Result<Input> {
    if let Some(name) = input.strip_prefix("fixture:") {
        return Ok(Input {
            model: fixtures::by_name(name)?,
            textures: Vec::new(),
            animation: animate::Animation::default(),
        });
    }
    let path = Path::new(input);
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.starts_with(&ugoe_format::MAGIC) {
        let bundle = ugoe_format::read_bundle(&bytes)
            .with_context(|| format!("reading bundle {}", path.display()))?;
        return Ok(Input {
            model: bundle.model,
            textures: bundle.textures,
            animation: animate::Animation {
                physics: bundle.physics,
                motions: bundle.motions,
                expressions: bundle.expressions,
            },
        });
    }
    let json = String::from_utf8(bytes).with_context(|| {
        format!(
            "{} is neither a .ugoe bundle nor UTF-8 JSON",
            path.display()
        )
    })?;
    let model = ugoe_format::model_from_json(&json)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(Input {
        model,
        textures: Vec::new(),
        animation: animate::Animation::default(),
    })
}

fn info_json(m: &Model) -> serde_json::Value {
    serde_json::json!({
        "name": m.meta.name,
        "canvas": { "width": m.meta.canvas.width, "height": m.meta.canvas.height },
        "parameters": m.parameters.iter().map(|p| serde_json::json!({
            "id": p.id.as_str(),
            "min": p.min,
            "default": p.default,
            "max": p.max,
        })).collect::<Vec<_>>(),
        "parts": m.parts.len(),
        "deformers": m.deformers.len(),
        "meshes": m.meshes.len(),
        "textures": m.textures.iter().map(|t| serde_json::json!({
            "name": t.name,
            "width": t.width,
            "height": t.height,
        })).collect::<Vec<_>>(),
    })
}

fn run_render(args: RenderArgs) -> anyhow::Result<()> {
    // GPU を使う前に入力をすべて検査する (GPU の無い環境でも引数の誤りを報告できるように)。
    let Input {
        model,
        textures,
        animation,
    } = load_input(&args.input)?;
    let canvas = (model.meta.canvas.width, model.meta.canvas.height);
    let size = render::output_size(canvas, args.size, args.width, args.height, args.scale)?;
    let view = render::fit_view(canvas, size, args.fit);
    let mut warnings = Vec::new();
    let textures = resolve_textures(&model, &textures, &args.textures, &mut warnings)?;
    let (mut puppet, w) = animate::puppet(model, &args.params)?;
    warnings.extend(w);
    if args.time.is_some() || args.motion.is_some() || args.expression.is_some() {
        let physics = match (&args.physics, args.no_physics) {
            (Some(path), _) => Some(animate::read_physics(path)?),
            (None, false) => animation.physics,
            (None, true) => None,
        };
        let playback = animate::Playback {
            time: args.time.unwrap_or(0.0),
            fps: args.fps,
            motion: args
                .motion
                .as_deref()
                .map(|m| animate::resolve_motion(m, &animation.motions))
                .transpose()?,
            expression: args
                .expression
                .as_deref()
                .map(|e| animate::resolve_expression(e, &animation.expressions))
                .transpose()?,
            physics,
            effects: !args.no_effects,
        };
        let time = playback.time;
        let updates = animate::advance(&mut puppet, playback)?;
        println!("advanced {time}s in {updates} update(s)");
    } else if args.physics.is_some() || args.no_physics || args.no_effects {
        warnings.push(
            "--physics, --no-physics and --no-effects only apply with --time, --motion \
             or --expression"
                .to_owned(),
        );
    }
    for w in &warnings {
        eprintln!("warning: {w}");
    }

    let gpu = render::Gpu::new(args.backend.as_deref())?;
    let result = gpu.render(puppet.instance(), &textures, size, &view, args.background);
    let adapter = format!("{} ({:?})", gpu.adapter_info.name, gpu.adapter_info.backend);
    gpu.leak();
    let image = result?;

    let png = image::encode_png_unpremultiplied(&image)?;
    std::fs::write(&args.output, png)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!(
        "wrote {} ({}x{}, adapter: {adapter})",
        args.output.display(),
        size.0,
        size.1
    );
    Ok(())
}

/// 描画に使うテクスチャ (乗算済み RGBA8) を番号順に揃える。
///
/// 優先順: `--texture` の指定 → バンドルの PNG → 不透明な白 1x1 (JSON・フィクスチャ用)。
fn resolve_textures(
    model: &Model,
    bundle: &[TextureData],
    overrides: &[(u32, PathBuf)],
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<Rgba8>> {
    let count = render::texture_count(model);
    if let Some((index, _)) = overrides.iter().find(|(i, _)| *i as usize >= count) {
        bail!("--texture {index}: the model has only {count} texture(s)");
    }
    (0..count)
        .map(|i| {
            if let Some((_, path)) = overrides.iter().rev().find(|(j, _)| *j as usize == i) {
                let bytes =
                    std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
                return image::decode_png_premultiplied(&bytes)
                    .with_context(|| format!("decoding texture {i} from {}", path.display()));
            }
            match bundle.get(i) {
                Some(data) => match data.format {
                    TextureFormat::Png => image::decode_png_premultiplied(&data.bytes)
                        .with_context(|| format!("decoding bundle texture {i}")),
                    TextureFormat::Ktx2 => bail!(
                        "bundle texture {i} is KTX2, which `ugoe render` cannot decode yet; \
                         pass --texture {i}=<file.png> instead"
                    ),
                },
                None => {
                    if !bundle.is_empty() {
                        warnings.push(format!(
                            "bundle has no data for texture {i}; using an opaque white placeholder"
                        ));
                    }
                    Ok(render::placeholder_texture())
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("ugoe").chain(args.iter().copied()))
    }

    #[test]
    fn parses_render_args() {
        let cli = parse(&[
            "render",
            "fixture:single_quad",
            "-o",
            "out.png",
            "--param",
            "X=0.5",
            "--param",
            "Y=-1",
            "--size",
            "64x32",
            "--background",
            "#ffffff",
            "--texture",
            "0=a.png",
        ])
        .unwrap();
        let Cmd::Render(args) = cli.command else {
            panic!("not render");
        };
        assert_eq!(args.params, vec![("X".into(), 0.5), ("Y".into(), -1.0)]);
        assert_eq!(args.size, Some((64, 32)));
        assert_eq!(args.background, [255, 255, 255, 255]);
        assert_eq!(args.textures, vec![(0, PathBuf::from("a.png"))]);
        assert_eq!(args.fit, Fit::Contain);
        assert_eq!(args.scale, 1.0);
    }

    #[test]
    fn render_defaults_to_transparent_background() {
        let cli = parse(&["render", "fixture:single_quad", "-o", "o.png"]).unwrap();
        let Cmd::Render(args) = cli.command else {
            panic!("not render");
        };
        assert_eq!(args.background, [0; 4]);
    }

    #[test]
    fn size_conflicts_with_width_and_scale() {
        assert!(
            parse(&[
                "render", "m.json", "-o", "o.png", "--size", "1x1", "--width", "2"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "render", "m.json", "-o", "o.png", "--width", "2", "--scale", "2"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "render", "m.json", "-o", "o.png", "--width", "2", "--height", "3"
            ])
            .is_ok()
        );
    }

    #[test]
    fn loads_fixture_inputs() {
        let input = load_input("fixture:single-quad").unwrap();
        assert_eq!(input.model.meta.name, "single_quad");
        assert!(load_input("fixture:nope").is_err());
    }

    #[test]
    fn info_json_lists_parameters() {
        let v = info_json(&ugoe_model::fixtures::single_quad());
        assert_eq!(v["parameters"][0]["id"], "X");
        assert_eq!(v["canvas"]["width"], 256.0);
        assert_eq!(v["meshes"], 1);
    }

    #[test]
    fn resolves_placeholder_and_rejects_out_of_range_overrides() {
        let model = ugoe_model::fixtures::single_quad();
        let mut warnings = Vec::new();
        let textures = resolve_textures(&model, &[], &[], &mut warnings).unwrap();
        assert_eq!(textures, vec![render::placeholder_texture()]);
        assert!(warnings.is_empty());

        let err = resolve_textures(&model, &[], &[(1, "a.png".into())], &mut warnings)
            .unwrap_err()
            .to_string();
        assert!(err.contains("only 1 texture"), "{err}");
    }

    #[test]
    fn rejects_ktx2_bundle_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: Vec::new(),
        };
        let err = resolve_textures(&model, &[ktx2], &[], &mut Vec::new())
            .unwrap_err()
            .to_string();
        assert!(err.contains("KTX2"), "{err}");
    }

    #[test]
    fn decodes_bundle_png_textures() {
        let model = ugoe_model::fixtures::single_quad();
        let red = Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255],
        };
        let png = TextureData {
            format: TextureFormat::Png,
            bytes: image::encode_png_unpremultiplied(&red).unwrap(),
        };
        let textures = resolve_textures(&model, &[png], &[], &mut Vec::new()).unwrap();
        assert_eq!(textures, vec![red]);
    }
}
