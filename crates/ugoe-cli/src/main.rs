//! `ugoe` コマンド。人間と AI エージェントの双方が、GUI 無しでモデルを検査・変換・確認するための道具。

mod fixtures;
mod physics;
mod sample;

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
use ugoe_editor_core::AtlasOptions;
use ugoe_format::{TextureData, TextureFormat};
use ugoe_headless::{Fit, Light, Lighting, RenderOptions, Scene, animate, args};
use ugoe_model::Model;

/// `INPUT` の書き方 (validate / info / render で共通)。
const INPUT_HELP: &str =
    "Model JSON (.json), bundle (.ugoe), or built-in fixture as `fixture:<name>`";
/// `validate` / `info` の `INPUT` の書き方。モジュールも受け付ける。
const MODULE_INPUT_HELP: &str = "Model JSON (.json), bundle (.ugoe), dress-up module (.ugoemod), \
     or built-in fixture as `fixture:<name>`";

#[derive(Debug, Parser)]
#[command(name = "ugoe", version, about = "ugoe toolchain CLI")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// モデルか着せ替えのモジュール (`.ugoemod`) を検証する。
    Validate {
        #[arg(help = MODULE_INPUT_HELP)]
        input: String,
        /// 構造の検証に加えて、プロファイルへの準拠を検査する。
        #[arg(long, value_enum)]
        profile: Option<Profile>,
        /// 物理の設定の JSON。バンドルの物理の代わりに検証する (JSON のモデルの物理を調べるときに使う)。
        #[arg(long, value_name = "PATH")]
        physics: Option<PathBuf>,
        /// 本体に付ける着せ替えのモジュール (`.ugoemod`、複数指定可。入力は `.ugoe` バンドル)。
        /// 付けた結果のモデルと物理を検証する。
        #[arg(long = "module", value_name = "PATH")]
        modules: Vec<PathBuf>,
    },
    /// モデルの概要 (パラメータ・パーツ・メッシュ数など) か、着せ替えのモジュールの概要
    /// (スロット・条件・要素の数・テクスチャ) を表示する。
    Info {
        #[arg(help = MODULE_INPUT_HELP)]
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
    /// `.ugoeproj` プロジェクトか `.ugoe` バンドルから、配布用の `.ugoe` バンドルを書き出す。
    /// `.ugoemod` モジュールはモジュールとして書き直す。
    Build(BuildArgs),
    /// 見本のモデルを生成する。
    #[command(subcommand)]
    Sample(SampleCmd),
}

#[derive(Debug, Subcommand)]
enum SampleCmd {
    /// アバタープロファイル (レベル 3) に準拠した見本のモデルを、PSD からコマンドの列で組み立てる。
    Avatar(SampleAvatarArgs),
    /// 着せ替えの見本 (スロットを持つ本体と衣装のモジュール) を書き出す。
    DressUp(SampleDressUpArgs),
}

#[derive(Debug, Args)]
struct SampleDressUpArgs {
    /// 書き出すディレクトリ (無ければ作る)。本体 `dress-up.ugoe` とモジュールの `.ugoemod` を置く。
    #[arg(short, long)]
    output: PathBuf,
    /// エディタで開くプロジェクトも書き出すディレクトリ。本体 `dress-up.ugoeproj` と、それを参照する
    /// モジュールのプロジェクト (`long-hair.ugoeproj` など) を並べて置く。
    #[arg(long, value_name = "DIR")]
    project: Option<PathBuf>,
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
    /// 法線マップを付けてライティングを有効にした版を書き出す (`assets/samples/avatar-lit.ugoe`)。
    #[arg(long)]
    lit: bool,
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
struct BuildArgs {
    /// 入力の `.ugoeproj` ディレクトリか `.ugoe` バンドル (`.ugoemod` モジュールも可)。
    input: PathBuf,
    /// 書き出す `.ugoe` バンドル (入力がモジュールなら `.ugoemod`)。
    #[arg(short, long)]
    output: PathBuf,
    /// テクスチャをアトラスにまとめる (`docs/spec/file-formats.md#テクスチャのアトラス化`)。
    /// モジュールはモジュールのテクスチャだけをまとめる。
    #[arg(long)]
    atlas: bool,
    /// アトラス 1 枚の幅と高さの上限 (px)。
    #[arg(long, value_name = "PX", requires = "atlas",
          default_value_t = AtlasOptions::default().max_size)]
    atlas_max_size: u32,
    /// アトラスで各画像の周りに複製する端の画素の幅 (px)。
    #[arg(long, value_name = "PX", requires = "atlas",
          default_value_t = AtlasOptions::default().padding)]
    atlas_padding: u32,
    /// 画像以外のチャンクを zstd で圧縮して書く (`docs/spec/file-formats.md`)。既定は無圧縮。
    #[arg(long)]
    compress: bool,
}

#[derive(Debug, Args)]
struct RenderArgs {
    #[arg(help = INPUT_HELP)]
    input: String,
    /// 出力する PNG のパス。
    #[arg(short, long)]
    output: PathBuf,
    /// パラメータの値 (複数指定可)。値域の外はクランプされる。
    #[arg(long = "param", value_name = "ID=VALUE", value_parser = args::parse_param)]
    params: Vec<(String, f32)>,
    /// 出力の大きさ。
    #[arg(long, value_name = "WxH", value_parser = args::parse_size,
          conflicts_with_all = ["width", "height", "scale"])]
    size: Option<(u32, u32)>,
    /// 出力の幅。`--height` が無ければ高さはキャンバスの縦横比から決める。
    #[arg(long, value_parser = args::parse_dimension, conflicts_with = "scale")]
    width: Option<u32>,
    /// 出力の高さ。`--width` が無ければ幅はキャンバスの縦横比から決める。
    #[arg(long, value_parser = args::parse_dimension, conflicts_with = "scale")]
    height: Option<u32>,
    /// 大きさを指定しないときの、キャンバスに対する倍率。
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
    /// キャンバスを出力にどう収めるか。
    #[arg(long, value_enum, default_value_t = Fit::Contain)]
    fit: Fit,
    /// 背景色。`transparent` (既定)、`white`、`black`、`#RRGGBB`、`#RRGGBBAA`。
    #[arg(long, value_name = "COLOR", default_value = "transparent",
          value_parser = args::parse_color)]
    background: [u8; 4],
    /// テクスチャを PNG ファイルで差し替える (複数指定可)。
    #[arg(long = "texture", value_name = "INDEX=PATH", value_parser = args::parse_texture)]
    textures: Vec<(u32, PathBuf)>,
    /// この秒数だけ `ugoe::Puppet` で時間を進めてから描く (物理・モーション・表情・目パチ・呼吸・ポーズ)。
    /// `--motion` / `--expression` だけを指定すると 0 秒。どれも無ければ時間を進めずにパラメータだけで描く。
    #[arg(long, value_name = "SECONDS", value_parser = args::parse_time)]
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
    #[arg(long, default_value = "60", value_parser = args::parse_fps)]
    fps: f32,
    /// 環境光の色: `V` (灰色) か `R,G,B` (1 を超えてもよい)。既定は 1 (白)。
    /// `--ambient` も `--light` も無ければ光源を渡さない (ライティングを受けるメッシュも受けない場合と同じ色)。
    #[arg(long, value_name = "COLOR", value_parser = args::parse_light_color)]
    ambient: Option<ugoe::core::Rgb>,
    /// 光源 (複数指定可): `directional:DX,DY,DZ[:COLOR]` (DX,DY,DZ は光が進む向き) か
    /// `point:X,Y,Z[:COLOR[:RANGE]]`。座標はキャンバス座標 + 奥行き (手前が +z)。
    #[arg(long = "light", value_name = "LIGHT", value_parser = args::parse_light)]
    lights: Vec<Light>,
    /// 本体に付ける着せ替えのモジュール (`.ugoemod`、複数指定可。入力は `.ugoe` バンドル)。
    #[arg(long = "module", value_name = "PATH")]
    modules: Vec<PathBuf>,
    /// wgpu のバックエンド (vulkan / metal / dx12 / gl。カンマ区切りで複数)。
    /// 省略時は環境変数 `WGPU_BACKEND`、それも無ければ wgpu の既定。
    #[arg(long)]
    backend: Option<String>,
    /// 描画の統計 (描画回数など。`ugoe_render::FrameStats`) を標準出力に `stats: <JSON>` の 1 行で書く。
    #[arg(long)]
    stats: bool,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Cmd::Validate {
            input,
            profile,
            physics,
            modules,
        } => {
            let mut scene = match load(&input)? {
                Input::Scene(scene) => *scene,
                Input::Module(_) => {
                    if profile.is_some() || physics.is_some() || !modules.is_empty() {
                        bail!(
                            "--profile, --physics and --module need a model; {input} is a dress-up module"
                        );
                    }
                    // 読み込みがモジュール単体の検査 (`validate_module`) をしている。
                    // 本体との整合は `ugoe validate <base>.ugoe --module <module>.ugoemod` で調べる。
                    println!("ok");
                    return Ok(());
                }
            };
            // `--physics` はバンドルの物理の代わり。モジュールのリグはそれに加わる。
            if let Some(path) = physics {
                scene.animation.physics = Some(animate::read_physics(&path)?);
            }
            if !modules.is_empty() {
                scene = attach_modules(scene, read_modules(&modules)?)?;
            }
            let Scene {
                model, animation, ..
            } = scene;
            let settings = animation.physics;
            let mut errors = ugoe_model::validate::validate(&model);
            if let Some(settings) = &settings {
                errors.extend(ugoe_model::validate::validate_physics(settings, &model));
            }
            for e in &errors {
                eprintln!("error: {e}");
            }
            // 物理の警告はモデルを評価して調べるので、エラーがないときだけ。
            if let Some(settings) = settings.as_ref()
                && errors.is_empty()
            {
                for w in physics::warnings(&model, settings)? {
                    eprintln!("warning: {w}");
                }
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
            let m = match load(&input)? {
                Input::Scene(scene) => scene.model,
                Input::Module(file) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&module_info_json(&file))?
                        );
                    } else {
                        print_module_info(&file);
                    }
                    return Ok(());
                }
            };
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
            use ugoe_model::standard::{ALL, BODY_EXTENDED, EXTENDED, VOWELS};
            let mut printed = Vec::new();
            for (id, min, default, max) in ALL
                .iter()
                .chain(EXTENDED)
                .chain(BODY_EXTENDED)
                .chain(VOWELS)
            {
                if !printed.contains(id) {
                    println!("{id:<12} {min:>6} {default:>6} {max:>6}");
                    printed.push(id);
                }
            }
        }
        Cmd::Render(args) => run_render(args)?,
        Cmd::ImportPsd(args) => run_import_psd(args)?,
        Cmd::Build(args) => run_build(args)?,
        Cmd::Sample(SampleCmd::Avatar(args)) => run_sample_avatar(args)?,
        Cmd::Sample(SampleCmd::DressUp(args)) => run_sample_dress_up(args)?,
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

/// `ugoe build`: プロジェクトかバンドルを読み、必要ならテクスチャをアトラスにまとめてバンドルに書く。
///
/// バンドルから作るときは、物理・モーション・表情・法線マップ・`META` をそのまま引き継ぐ。
/// 入力が着せ替えのモジュールなら、モジュールとして書き直す ([`rebuild_module`])。
fn run_build(args: BuildArgs) -> anyhow::Result<()> {
    use ugoe_editor_core::Project;

    let (project, meta) = if args.input.is_dir() {
        let project = Project::load(&args.input)
            .with_context(|| format!("loading {}", args.input.display()))?;
        (project, None)
    } else {
        let bytes = std::fs::read(&args.input)
            .with_context(|| format!("reading {}", args.input.display()))?;
        match read_container(&args.input, &bytes)? {
            Container::Bundle(bundle) => project_from_bundle(*bundle)?,
            Container::Module(file) => return rebuild_module(&args, &file),
        }
    };

    let project = if args.atlas {
        let (atlased, report) = project.with_atlas(&atlas_options(&args))?;
        print_atlas_report(&report);
        atlased
    } else {
        project
    };

    let textures = project
        .model
        .textures
        .iter()
        .map(|t| {
            let png = project
                .textures
                .get(&t.name)
                .with_context(|| format!("texture `{}` has no image data", t.name))?;
            Ok(TextureData {
                format: TextureFormat::Png,
                bytes: png.to_vec(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let normal_maps = project
        .model
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref())
        .map(|name| {
            let png = project
                .textures
                .get(name)
                .with_context(|| format!("normal map `{name}` has no image data"))?;
            Ok(TextureData {
                format: TextureFormat::Png,
                bytes: png.to_vec(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let bytes = ugoe_format::write_bundle_with(
        &ugoe_format::Bundle {
            model: project.model,
            physics: project.physics,
            motions: project.motions,
            expressions: project.expressions,
            textures,
            normal_maps,
            meta,
        },
        write_options(&args),
    )?;
    std::fs::write(&args.output, &bytes)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!("wrote {} ({} bytes)", args.output.display(), bytes.len());
    Ok(())
}

/// バンドルを、`ugoe build` が扱うプロジェクトと `META` にする。テクスチャと法線マップの画像は名前で
/// `Project::textures` に入れる (プロジェクトと同じく、法線マップも `TextureRef::normal_map` の名前で引く)。
fn project_from_bundle(
    bundle: ugoe_format::Bundle,
) -> anyhow::Result<(ugoe_editor_core::Project, Option<serde_json::Value>)> {
    use ugoe_editor_core::{Project, ProjectMeta};

    let mut textures = ugoe_editor_core::Textures::new();
    for (i, (texture, data)) in bundle
        .model
        .textures
        .iter()
        .zip(&bundle.textures)
        .enumerate()
    {
        if data.format != TextureFormat::Png {
            bail!(
                "bundle texture {i} (`{}`) is not PNG; `ugoe build` needs PNG textures",
                texture.name
            );
        }
        textures.insert(texture.name.clone(), data.bytes.clone().into());
    }
    let normal_names = bundle
        .model
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref());
    for (name, data) in normal_names.zip(&bundle.normal_maps) {
        if data.format != TextureFormat::Png {
            bail!("bundle normal map `{name}` is not PNG; `ugoe build` needs PNG normal maps");
        }
        textures.insert(name.to_owned(), data.bytes.clone().into());
    }
    let mut project = Project::new(
        ProjectMeta::new(bundle.model.meta.name.clone()),
        bundle.model,
        textures,
    );
    project.physics = bundle.physics;
    project.motions = bundle.motions;
    project.expressions = bundle.expressions;
    Ok((project, bundle.meta))
}

/// `ugoe build <module>.ugoemod`: 着せ替えのモジュールを読み直して書き出す。`--atlas` ではモジュールの
/// テクスチャだけをモジュールの単位でアトラスにまとめる (`docs/spec/file-formats.md#テクスチャのアトラス化`)。
fn rebuild_module(args: &BuildArgs, file: &ugoe_format::ModuleFile) -> anyhow::Result<()> {
    let atlased;
    let file = if args.atlas {
        atlased = atlas_module(file, &atlas_options(args))?;
        &atlased
    } else {
        file
    };
    let bytes = ugoe_format::write_module_with(file, write_options(args))?;
    std::fs::write(&args.output, &bytes)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!("wrote {} ({} bytes)", args.output.display(), bytes.len());
    Ok(())
}

/// モジュールのテクスチャをアトラスにまとめる (`ugoe_editor_core::atlas::build_module_atlas`)。
/// テクスチャと法線マップの画像は、プロジェクトと同じく名前で引く。
fn atlas_module(
    file: &ugoe_format::ModuleFile,
    options: &AtlasOptions,
) -> anyhow::Result<ugoe_format::ModuleFile> {
    let mut images = ugoe_editor_core::Textures::new();
    let mut add = |name: &str, data: &TextureData, what: &str| {
        if data.format != TextureFormat::Png {
            bail!("module {what} `{name}` is not PNG; `ugoe build --atlas` needs PNG images");
        }
        if images
            .insert(name.to_owned(), data.bytes.clone().into())
            .is_some()
        {
            bail!(
                "module image name `{name}` is used twice; `ugoe build --atlas` needs unique names"
            );
        }
        Ok(())
    };
    for (i, (texture, data)) in file.module.textures.iter().zip(&file.textures).enumerate() {
        add(&texture.name, data, "texture")?;
        if let (Some(name), Some(data)) = (&texture.normal_map, file.normal_map(i)) {
            add(name, data, "normal map")?;
        }
    }
    let atlased = ugoe_editor_core::atlas::build_module_atlas(&file.module, &images, options)?;
    print_atlas_report(&atlased.report);
    let image = |name: &str| -> anyhow::Result<TextureData> {
        let png = atlased
            .textures
            .get(name)
            .with_context(|| format!("image `{name}` has no data"))?;
        Ok(TextureData {
            format: TextureFormat::Png,
            bytes: png.to_vec(),
        })
    };
    let textures = atlased
        .module
        .textures
        .iter()
        .map(|t| image(&t.name))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let normal_maps = atlased
        .module
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref())
        .map(image)
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(ugoe_format::ModuleFile {
        module: atlased.module,
        textures,
        normal_maps,
        meta: file.meta.clone(),
    })
}

fn atlas_options(args: &BuildArgs) -> AtlasOptions {
    AtlasOptions {
        max_size: args.atlas_max_size,
        padding: args.atlas_padding,
    }
}

/// `--atlas` の要約を標準出力に、まとめずに残したテクスチャを標準エラーに書く。
fn print_atlas_report(report: &ugoe_editor_core::AtlasReport) {
    for (name, reason) in &report.kept {
        eprintln!(
            "warning: texture `{name}` was not packed ({})",
            kept_reason(*reason)
        );
    }
    let sizes: Vec<String> = report
        .atlases
        .iter()
        .map(|(w, h)| format!("{w}x{h}"))
        .collect();
    println!(
        "atlas: packed {} texture(s) into {} atlas(es) [{}], kept {}, removed {} unused",
        report.packed,
        report.atlases.len(),
        sizes.join(", "),
        report.kept.len(),
        report.removed.len()
    );
}

fn write_options(args: &BuildArgs) -> ugoe_format::WriteOptions {
    ugoe_format::WriteOptions {
        compress: args.compress,
    }
}

fn kept_reason(reason: ugoe_editor_core::atlas::KeptReason) -> &'static str {
    use ugoe_editor_core::atlas::KeptReason;
    match reason {
        KeptReason::TooLarge => "larger than --atlas-max-size",
        KeptReason::UvOutOfRange => "its meshes use UVs outside 0..1",
        KeptReason::HasNormalMap => "it has a normal map",
    }
}

/// `ugoe sample avatar`: 見本のモデルを組み立てて書き出す (`sample::build_avatar`)。
fn run_sample_avatar(args: SampleAvatarArgs) -> anyhow::Result<()> {
    let psd = std::fs::read(&args.psd).with_context(|| {
        format!(
            "reading {} (run from the repository root or pass --psd)",
            args.psd.display()
        )
    })?;
    let mut project = sample::build_avatar(&psd)?;
    if args.lit {
        project = sample::lit::light_avatar(&project, &psd)?;
    }
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

/// `ugoe sample dress-up`: 着せ替えの見本を書き出す (`sample::dress_up`)。
fn run_sample_dress_up(args: SampleDressUpArgs) -> anyhow::Result<()> {
    let sample = sample::dress_up::build_dress_up()?;
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("creating {}", args.output.display()))?;
    for (name, bytes) in sample.files()? {
        let path = args.output.join(name);
        std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
    if let Some(dir) = args.project {
        for path in sample.save_projects(&dir)? {
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}

/// `--module` の `.ugoemod` を読む。
fn read_modules(paths: &[PathBuf]) -> anyhow::Result<Vec<ugoe_format::ModuleFile>> {
    paths
        .iter()
        .map(|path| {
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            ugoe_format::read_module(&bytes)
                .with_context(|| format!("reading module {}", path.display()))
        })
        .collect()
}

/// `--module`: モジュールを本体に合成したシーンにする (`docs/spec/modules.md#合成`)。
/// モジュールのテクスチャは本体のテクスチャの後ろに、モジュールの順に並ぶ (合成がメッシュの番号をずらすのと同じ)。
fn attach_modules(scene: Scene, files: Vec<ugoe_format::ModuleFile>) -> anyhow::Result<Scene> {
    if scene.textures.len() != scene.model.textures.len() {
        bail!("--module needs a .ugoe bundle (with textures) as the input");
    }
    let modules: Vec<&ugoe_model::Module> = files.iter().map(|f| &f.module).collect();
    let model = ugoe_runtime::compose(&scene.model, &modules)?;
    let mut animation = scene.animation;
    let has_rigs = modules.iter().any(|m| !m.physics.rigs.is_empty());
    if animation.physics.is_some() || has_rigs {
        let settings = animation
            .physics
            .take()
            .unwrap_or_else(ugoe_editor_core::physics_edit::default_settings);
        animation.physics = Some(ugoe_runtime::compose_physics(
            &scene.model,
            &settings,
            &modules,
        )?);
    }
    let mut textures = scene.textures;
    let mut normal_maps = scene.normal_maps;
    normal_maps.resize(scene.model.textures.len(), None);
    for file in files {
        textures.extend(file.textures);
        // モジュールの法線マップは、`normal_map` を持つテクスチャの順に並ぶ。
        let mut maps = file.normal_maps.into_iter();
        for texture in &file.module.textures {
            normal_maps.push(texture.normal_map.as_ref().and_then(|_| maps.next()));
        }
    }
    Ok(Scene {
        model,
        textures,
        normal_maps,
        animation,
    })
}

/// 入力として読んだもの: 描けるシーン (モデル JSON・バンドル・フィクスチャ) か、着せ替えのモジュール
/// (読み込みで単体の検査 `validate_module` を済ませたもの)。
enum Input {
    Scene(Box<Scene>),
    Module(Box<ugoe_format::ModuleFile>),
}

/// 先頭のマジック `UGOE` を持つファイルの中身。
enum Container {
    Bundle(Box<ugoe_format::Bundle>),
    Module(Box<ugoe_format::ModuleFile>),
}

/// `.ugoe` バンドルか `.ugoemod` モジュールを読む。どちらも同じコンテナなので、拡張子ではなく中身で判別する
/// (バンドルとして読んで `MODU` チャンクがあればモジュールとして読み直す)。
/// モデルやモジュールが検証に通らなければ、各エラーを標準エラーに `error: ...` で書いてから失敗する。
fn read_container(path: &Path, bytes: &[u8]) -> anyhow::Result<Container> {
    use ugoe_format::{ChunkTag, FormatError};

    let result = match ugoe_format::read_bundle(bytes) {
        Ok(bundle) => return Ok(Container::Bundle(Box::new(bundle))),
        Err(FormatError::UnexpectedChunk(ChunkTag::MODU)) => ugoe_format::read_module(bytes)
            .map(|file| Container::Module(Box::new(file)))
            .with_context(|| format!("reading module {}", path.display())),
        Err(e) => Err(e).with_context(|| format!("reading bundle {}", path.display())),
    };
    if let Err(e) = &result
        && let Some(FormatError::Invalid(errors) | FormatError::InvalidModule(errors)) =
            e.downcast_ref::<FormatError>()
    {
        for error in errors {
            eprintln!("error: {error}");
        }
    }
    result
}

/// `fixture:<name>`、`.ugoe` バンドル・`.ugoemod` モジュール (先頭のマジックで判別)、モデル JSON のいずれかを読む。
fn load(input: &str) -> anyhow::Result<Input> {
    if let Some(name) = input.strip_prefix("fixture:") {
        return Ok(Input::Scene(Box::new(Scene::from_model(
            fixtures::by_name(name)?,
        ))));
    }
    let path = Path::new(input);
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.starts_with(&ugoe_format::MAGIC) {
        return Ok(match read_container(path, &bytes)? {
            Container::Bundle(bundle) => Input::Scene(Box::new(Scene::from_bundle(*bundle))),
            Container::Module(file) => Input::Module(file),
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
    Ok(Input::Scene(Box::new(Scene::from_model(model))))
}

/// 描けるシーンを読む ([`load`])。モジュールは本体に付けないと描けないのでエラーにする。
fn load_input(input: &str) -> anyhow::Result<Scene> {
    match load(input)? {
        Input::Scene(scene) => Ok(*scene),
        Input::Module(_) => bail!(
            "{input} is a dress-up module (.ugoemod), not a model (attach it to a base bundle with `--module`)"
        ),
    }
}

fn info_json(m: &Model) -> serde_json::Value {
    serde_json::json!({
        "kind": "model",
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

/// `ugoe info <module>.ugoemod` の表示。
fn print_module_info(file: &ugoe_format::ModuleFile) {
    let m = &file.module;
    let list = |ids: Vec<&str>| {
        if ids.is_empty() {
            "-".to_owned()
        } else {
            ids.join(", ")
        }
    };
    println!("module:     {}", m.name);
    println!("slot:       {}", m.slot.as_str());
    println!(
        "requires:   avatar level {}; slots {}; parameters {}",
        m.requires
            .avatar_level
            .map_or_else(|| "-".to_owned(), |l| l.to_string()),
        list(m.requires.slots.iter().map(|s| s.as_str()).collect()),
        list(m.requires.parameters.iter().map(|p| p.as_str()).collect())
    );
    let used = m.referenced_parameters();
    println!(
        "parameters: {} used ({})",
        used.len(),
        list(used.iter().map(|p| p.as_str()).collect())
    );
    println!("parts:      {}", m.parts.len());
    println!("deformers:  {}", m.deformers.len());
    println!("meshes:     {}", m.meshes.len());
    println!("glues:      {}", m.glues.len());
    println!(
        "physics:    {} rig(s), {} collider(s)",
        m.physics.rigs.len(),
        m.physics.colliders.len()
    );
    println!(
        "pose:       {} group(s), {} link(s)",
        m.pose.groups.len(),
        m.pose.links.len()
    );
    println!("textures:   {}", m.textures.len());
    for (i, (t, data)) in m.textures.iter().zip(&file.textures).enumerate() {
        let normal = file
            .normal_map(i)
            .zip(t.normal_map.as_deref())
            .map(|(data, name)| format!(", normal map `{name}` ({})", format_name(data.format)))
            .unwrap_or_default();
        println!(
            "  {i}: `{}` {}x{} ({}){normal}",
            t.name,
            t.width,
            t.height,
            format_name(data.format)
        );
    }
}

/// `ugoe info <module>.ugoemod --json`。
fn module_info_json(file: &ugoe_format::ModuleFile) -> serde_json::Value {
    let m = &file.module;
    serde_json::json!({
        "kind": "module",
        "name": m.name,
        "slot": m.slot.as_str(),
        "requires": {
            "avatar_level": m.requires.avatar_level,
            "slots": m.requires.slots.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "parameters": m.requires.parameters.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
        },
        "parameters": m.referenced_parameters().iter().map(|p| p.as_str().to_owned()).collect::<Vec<_>>(),
        "parts": m.parts.len(),
        "deformers": m.deformers.len(),
        "meshes": m.meshes.len(),
        "glues": m.glues.len(),
        "physics": { "rigs": m.physics.rigs.len(), "colliders": m.physics.colliders.len() },
        "pose": { "groups": m.pose.groups.len(), "links": m.pose.links.len() },
        "textures": m.textures.iter().zip(&file.textures).enumerate().map(|(i, (t, data))| serde_json::json!({
            "name": t.name,
            "width": t.width,
            "height": t.height,
            "format": format_name(data.format),
            "normal_map": t.normal_map.as_deref().zip(file.normal_map(i)).map(|(name, data)| serde_json::json!({
                "name": name,
                "format": format_name(data.format),
            })),
        })).collect::<Vec<_>>(),
    })
}

fn format_name(format: TextureFormat) -> &'static str {
    match format {
        TextureFormat::Png => "png",
        TextureFormat::Ktx2 => "ktx2",
    }
}

fn run_render(args: RenderArgs) -> anyhow::Result<()> {
    // GPU を使う前に入力をすべて検査する (GPU の無い環境でも引数の誤りを報告できるように)。
    let mut scene = load_input(&args.input)?;
    if !args.modules.is_empty() {
        scene = attach_modules(scene, read_modules(&args.modules)?)?;
    }
    let lighting = (args.ambient.is_some() || !args.lights.is_empty()).then(|| Lighting {
        ambient: args.ambient.unwrap_or(Lighting::default().ambient),
        lights: args.lights,
    });
    let options = RenderOptions {
        params: args.params,
        size: args.size,
        width: args.width,
        height: args.height,
        scale: args.scale,
        fit: args.fit,
        background: args.background,
        texture_overrides: args.textures,
        time: args.time,
        motion: args.motion,
        expression: args.expression,
        physics: args.physics,
        no_physics: args.no_physics,
        no_effects: args.no_effects,
        fps: args.fps,
        lighting,
    };
    let prepared = ugoe_headless::prepare(scene, &options)?;
    if let Some(advanced) = prepared.advanced {
        println!(
            "advanced {}s in {} update(s)",
            advanced.time, advanced.updates
        );
    }
    for w in &prepared.warnings {
        eprintln!("warning: {w}");
    }

    let gpu = ugoe_headless::Gpu::new(args.backend.as_deref())?;
    let result = prepared.render(&gpu);
    let adapter = gpu.adapter_description();
    gpu.leak();
    let (image, stats) = result?;

    let png = ugoe_headless::image::encode_png_unpremultiplied(&image)?;
    std::fs::write(&args.output, png)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!(
        "wrote {} ({}x{}, adapter: {adapter})",
        args.output.display(),
        prepared.size.0,
        prepared.size.1
    );
    if args.stats {
        println!("stats: {}", ugoe_headless::stats_json(&stats));
    }
    Ok(())
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
    fn render_attaches_modules_after_the_base_textures() {
        let sample = sample::dress_up::build_dress_up().unwrap();
        let base = sample.base.model.clone();
        let files = vec![sample.modules[0].clone(), sample.modules[3].clone()];
        let module_textures: usize = files.iter().map(|f| f.textures.len()).sum();
        let scene = attach_modules(Scene::from_bundle(sample.base), files).unwrap();
        assert_eq!(scene.textures.len(), base.textures.len() + module_textures);
        assert_eq!(scene.textures.len(), scene.model.textures.len());
        assert_eq!(scene.normal_maps.len(), scene.textures.len());
        assert!(
            scene
                .model
                .meshes
                .iter()
                .any(|m| m.id.as_str() == "LongHair/Back")
        );
        // モジュールに物理のリグがあるので、本体に物理が無くても物理の設定ができる。
        assert!(!scene.animation.physics.unwrap().rigs.is_empty());
        // テクスチャの無い入力 (フィクスチャ) には付けられない。
        let fixture = Scene::from_model(ugoe_model::fixtures::dress_up_base());
        let hair = ugoe_format::ModuleFile {
            module: ugoe_model::fixtures::long_hair_module(),
            textures: Vec::new(),
            normal_maps: Vec::new(),
            meta: None,
        };
        assert!(attach_modules(fixture, vec![hair]).is_err());
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
    fn parses_lighting_args() {
        let cli = parse(&[
            "render",
            "m.ugoe",
            "-o",
            "o.png",
            "--ambient",
            "0.2",
            "--light",
            "directional:1,0,-1",
            "--light",
            "point:0,0,100:1,0.5,0:300",
        ])
        .unwrap();
        let Cmd::Render(args) = cli.command else {
            panic!("not render");
        };
        assert_eq!(args.ambient, Some(ugoe::core::Rgb::new(0.2, 0.2, 0.2)));
        assert_eq!(args.lights.len(), 2);
        assert!(parse(&["render", "m.ugoe", "-o", "o.png", "--light", "spot:0,0,1"]).is_err());
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
}
