//! `ugoe` コマンド。人間と AI エージェントの双方が、GUI 無しでモデルを検査・変換・確認するための道具。

mod fixtures;
mod json_source;
mod physics;
mod sample;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
use json_source::JsonSource;
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
    Render(Box<RenderArgs>),
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
    /// テクスチャを既定の設定でアトラスにまとめた版を書き出す (`assets/samples/avatar-atlas.ugoe`。
    /// 描画をまとめるエンジンの見本)。法線マップのあるテクスチャはまとめられないので `--lit` とは使えない。
    #[arg(long, conflicts_with = "lit")]
    atlas: bool,
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
    /// モデル・モジュールの JSON も可 (画像は JSON の隣の `textures/<name>.png`)。
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
    /// `META` に書くメタデータ (作者やライセンスなど。自由な JSON) のパス。入力のバンドル・モジュールの
    /// `META` の代わりにする。
    #[arg(long, value_name = "PATH")]
    meta: Option<PathBuf>,
    /// 物理の設定の JSON (`*.physics.json`) のパス。入力がモデルの JSON のときだけ使える。
    #[arg(long, value_name = "PATH")]
    physics: Option<PathBuf>,
    /// モーションの JSON (`*.motion.json`) のパス (複数指定可。並べた順に書く)。入力がモデルの JSON のときだけ使える。
    #[arg(long = "motion", value_name = "PATH")]
    motions: Vec<PathBuf>,
    /// 表情の JSON (`*.expression.json`) のパス (複数指定可。並べた順に書く)。入力がモデルの JSON のときだけ使える。
    #[arg(long = "expression", value_name = "PATH")]
    expressions: Vec<PathBuf>,
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
    /// ビュー変換を 6 数 `A,B,C,D,E,F` で直接与える (`--fit` の代わり)。キャンバスの点 `(x, y)` を出力の
    /// `(A*x + C*y + E, B*x + D*y + F)` (出力の画素、左上が原点・Y 下向き) に写す。鏡映や回転もできる。
    /// 公開仕様の描画の例の `view` と同じ意味 (`spec/examples/README.md#rendering-examples`)。
    #[arg(long, value_name = "A,B,C,D,E,F", value_parser = parse_view,
          allow_hyphen_values = true, conflicts_with = "fit")]
    view: Option<[f32; 6]>,
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
    /// GPU を待つ時間の上限 (秒)。超えたらエラーで終わる (ドライバが止まっても戻るように)。
    /// 遅いソフトウェアの GPU (lavapipe・llvmpipe) で大きな画を描くときは延ばす。`0` は上限なし。
    #[arg(long, value_name = "SECONDS", default_value = "30",
          value_parser = args::parse_gpu_timeout)]
    gpu_timeout: Duration,
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
            // モーションの数値は読み込みで検査した。時刻が減る点はエラーにせず警告する。
            for motion in &animation.motions {
                for w in ugoe_model::validate::motion_warnings(motion) {
                    eprintln!("warning: motion `{}`: {w}", motion.name);
                }
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
        Cmd::Render(args) => run_render(*args)?,
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

    // 通常のファイルだけを、PSD のファイルの上限まで読む (装置・FIFO・ディレクトリや巨大なファイルで
    // 止まったりメモリを使い果たしたりしないように。エディタ・MCP サーバと同じ `ugoe_fs::read_regular_file`)。
    let bytes = ugoe_fs::read_regular_file(&args.input, ugoe_psd::MAX_PSD_FILE_BYTES)
        .with_context(|| format!("reading {}", args.input.display()))?;
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
/// モデル・モジュールの JSON からも書ける ([`read_json_source`])。
fn run_build(args: BuildArgs) -> anyhow::Result<()> {
    use ugoe_editor_core::Project;

    // 入力を読む前に、壊れた `--meta` で止める。
    let new_meta = args
        .meta
        .as_deref()
        .map(json_source::read_meta)
        .transpose()?;
    let (project, meta) = if args.input.is_dir() {
        reject_animation_files(&args, "a project")?;
        let project = Project::load(&args.input)
            .with_context(|| format!("loading {}", args.input.display()))?;
        (project, None)
    } else {
        // 利用者が渡すパスなので、`load` と同じく FIFO (`/dev/stdin` など) も読む。
        let bytes = std::fs::read(&args.input)
            .with_context(|| format!("reading {}", args.input.display()))?;
        if bytes.starts_with(&ugoe_format::MAGIC) {
            match read_container(&args.input, &bytes)? {
                Container::Bundle(bundle) => {
                    reject_animation_files(&args, "a bundle")?;
                    project_from_bundle(*bundle)?
                }
                Container::Module(mut file) => {
                    reject_animation_files(&args, "a dress-up module")?;
                    file.meta = new_meta.or(file.meta);
                    return rebuild_module(&args, &file);
                }
            }
        } else {
            match read_json_source(&args.input, bytes)? {
                JsonSource::Model(model) => {
                    let mut project = project_from_json(&args.input, *model)?;
                    add_animation_files(&args, &mut project)?;
                    (project, None)
                }
                JsonSource::Module(module) => {
                    reject_animation_files(&args, "a module JSON")?;
                    let mut file = module_file_from_json(&args.input, *module)?;
                    file.meta = new_meta;
                    return rebuild_module(&args, &file);
                }
            }
        }
    };
    let meta = new_meta.or(meta);

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

/// `ugoe build` の入力の JSON (モデルかモジュール) を読む ([`json_source::parse`])。検証に通らなければ、
/// バンドルと同じ形 (各エラーを `error: ...`、最後に件数) で失敗する。
fn read_json_source(path: &Path, bytes: Vec<u8>) -> anyhow::Result<JsonSource> {
    let json = String::from_utf8(bytes).with_context(|| {
        format!(
            "{} is neither a .ugoe bundle, a .ugoemod module nor UTF-8 JSON",
            path.display()
        )
    })?;
    match json_source::parse(path, &json)? {
        Ok(source) => Ok(source),
        Err(error) => Err(validation_failure(path, &error).unwrap_or_else(|| error.into())),
    }
}

/// モデルの JSON を、`ugoe build` が扱うプロジェクトにする。画像は JSON の隣の `textures/<name>.png`
/// ([`json_source::images`])。
fn project_from_json(path: &Path, model: Model) -> anyhow::Result<ugoe_editor_core::Project> {
    use ugoe_editor_core::{Project, ProjectMeta};

    let textures = json_source::images(path, &model.textures)?;
    Ok(Project::new(
        ProjectMeta::new(model.meta.name.clone()),
        model,
        textures,
    ))
}

/// `--physics`・`--motion`・`--expression` はモデルの JSON の入力にだけ使える (ほかの入力は物理などを
/// 自分で持つか、持てない)。指定されていれば、入力の種類 `what` を挙げて失敗する。
fn reject_animation_files(args: &BuildArgs, what: &str) -> anyhow::Result<()> {
    if args.physics.is_some() || !args.motions.is_empty() || !args.expressions.is_empty() {
        bail!(
            "--physics, --motion and --expression need a model JSON input; {} is {what}",
            args.input.display()
        );
    }
    Ok(())
}

/// `--physics`・`--motion`・`--expression` のファイルを読み、モデルの JSON から作ったプロジェクトに添える。
/// 物理はモデルとの参照を検証し、モーションと表情は数値を検証する。通らなければ、そのファイルについて
/// バンドルと同じ形 (各エラーを `error: ...`、最後に件数) で失敗する。
fn add_animation_files(
    args: &BuildArgs,
    project: &mut ugoe_editor_core::Project,
) -> anyhow::Result<()> {
    let invalid = |path: &Path, error: ugoe_format::FormatError| -> anyhow::Error {
        validation_failure(path, &error).unwrap_or_else(|| {
            anyhow::Error::new(error).context(format!("parsing {}", path.display()))
        })
    };
    if let Some(path) = &args.physics {
        let physics = ugoe_format::physics_from_json(&json_source::read_json(path)?)
            .map_err(|e| invalid(path, e))?;
        let errors = ugoe_model::validate::validate_physics(&physics, &project.model);
        if !errors.is_empty() {
            return Err(invalid(
                path,
                ugoe_format::FormatError::InvalidPhysics(errors),
            ));
        }
        project.physics = Some(physics);
    }
    for path in &args.motions {
        let motion = ugoe_format::motion_from_json(&json_source::read_json(path)?)
            .map_err(|e| invalid(path, e))?;
        project.motions.push(motion);
    }
    for path in &args.expressions {
        let expression = ugoe_format::expression_from_json(&json_source::read_json(path)?)
            .map_err(|e| invalid(path, e))?;
        project.expressions.push(expression);
    }
    Ok(())
}

/// モジュールの JSON を、画像 (JSON の隣の `textures/<name>.png`) を付けたモジュールのファイルにする。
fn module_file_from_json(
    path: &Path,
    module: ugoe_model::module::Module,
) -> anyhow::Result<ugoe_format::ModuleFile> {
    let images = json_source::images(path, &module.textures)?;
    let image = |name: &str| TextureData {
        format: TextureFormat::Png,
        bytes: images.get(name).map(|png| png.to_vec()).unwrap_or_default(),
    };
    let textures = module.textures.iter().map(|t| image(&t.name)).collect();
    // 法線マップは `normal_map` を持つテクスチャごとに 1 つずつ (同じ名前でも並べる)。
    let normal_maps = module
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref())
        .map(image)
        .collect();
    let mut file = ugoe_format::ModuleFile::new(module);
    file.textures = textures;
    file.normal_maps = normal_maps;
    Ok(file)
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
    // `import-psd` と同じく、通常のファイルだけを PSD のファイルの上限まで読む。
    let psd =
        ugoe_fs::read_regular_file(&args.psd, ugoe_psd::MAX_PSD_FILE_BYTES).with_context(|| {
            format!(
                "reading {} (run from the repository root or pass --psd)",
                args.psd.display()
            )
        })?;
    let mut project = sample::build_avatar(&psd)?;
    if args.lit {
        project = sample::lit::light_avatar(&project, &psd)?;
    }
    if args.atlas {
        let (atlased, report) = project.with_atlas(&AtlasOptions::default())?;
        print_atlas_report(&report);
        project = atlased;
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
    let mut model = ugoe_runtime::compose(&scene.model, &modules)?;
    let mut animation = scene.animation;
    let has_rigs = modules.iter().any(|m| !m.physics.rigs.is_empty());
    match animation.physics.take() {
        Some(settings) => {
            animation.physics = Some(ugoe_runtime::compose_physics(
                &scene.model,
                &settings,
                &modules,
            )?);
        }
        // 本体に物理が無ければ、スロットが公開する衝突体は何も指さない (`docs/spec/modules.md#合成`)。
        None if has_rigs => {
            animation.physics = Some(ugoe_runtime::compose_physics_without_base(
                &scene.model,
                &mut model,
                &ugoe_editor_core::physics_edit::default_settings(),
                &modules,
            )?);
        }
        None => {}
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
/// モデル・モジュール・バンドルの物理・モーション・表情が検証に通らなければ、各エラーを標準エラーに `error: ...` で書いてから失敗する。
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
        && let Some(format_error) = e.downcast_ref::<FormatError>()
        && let Some(failure) = validation_failure(path, format_error)
    {
        return Err(failure);
    }
    result
}

/// 検証エラーの列を持つ `error` なら、各エラーを標準エラーに `error: ...` で書き、件数で失敗するエラーを返す。
/// ほかの種類は `None`。
fn validation_failure(path: &Path, error: &ugoe_format::FormatError) -> Option<anyhow::Error> {
    use ugoe_format::FormatError;

    let errors = error.validation_errors()?;
    // モーションと表情のエラーは名前を前に付ける。
    let prefix = match error {
        FormatError::InvalidMotion { name, .. } | FormatError::InvalidExpression { name, .. } => {
            format!("{name}: ")
        }
        _ => String::new(),
    };
    for error in errors {
        eprintln!("error: {prefix}{error}");
    }
    // 各エラーは上で上限なしに書いたので、最後の文は件数だけにする (`validate` のモデルの検査と同じ形)。
    Some(anyhow::anyhow!(
        "{}: {} validation error(s)",
        path.display(),
        errors.len()
    ))
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
    let prepared = prepare_render(&args)?;
    if let Some(advanced) = prepared.advanced {
        println!(
            "advanced {}s in {} update(s)",
            advanced.time, advanced.updates
        );
    }
    for w in &prepared.warnings {
        eprintln!("warning: {w}");
    }

    let mut gpu = ugoe_headless::Gpu::new(args.backend.as_deref())?;
    gpu.set_wait_timeout(args.gpu_timeout);
    let result = prepared.render(&gpu);
    let adapter = gpu.adapter_description();
    gpu.leak();
    let (image, stats) = result.map_err(|e| {
        let timed_out = e.is_gpu_wait_timeout();
        let e = anyhow::Error::new(e);
        if timed_out {
            e.context("rendering did not finish within --gpu-timeout (raise it, or 0 waits without a limit)")
        } else {
            e
        }
    })?;

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

/// `ugoe render` の引数から、GPU を使わずに描画の準備をする (`ugoe_headless::prepare`)。
/// GPU を使う前に入力をすべて検査する (GPU の無い環境でも引数の誤りを報告できるように)。
fn prepare_render(args: &RenderArgs) -> anyhow::Result<ugoe_headless::Prepared> {
    let mut scene = load_input(&args.input)?;
    let mut physics = args.physics.clone();
    if !args.modules.is_empty() {
        // `--physics` は本体の物理の代わりにし、モジュールのリグはそれに加える (`validate` と同じ)。
        // 時間を進めないなら物理は使わないので、`prepare` に渡したまま (使わない旨の警告を出させる)。
        let animating = args.time.is_some() || args.motion.is_some() || args.expression.is_some();
        if animating && let Some(path) = physics.take() {
            scene.animation.physics = Some(animate::read_physics(&path)?);
        }
        scene = attach_modules(scene, read_modules(&args.modules)?)?;
    }
    let lighting = (args.ambient.is_some() || !args.lights.is_empty()).then(|| Lighting {
        ambient: args.ambient.unwrap_or(Lighting::default().ambient),
        lights: args.lights.clone(),
    });
    let options = RenderOptions {
        params: args.params.clone(),
        size: args.size,
        width: args.width,
        height: args.height,
        scale: args.scale,
        // 大きさを指定しないときは、上限に収まるよう縮める (`render_png` と同じ規則。上限は 16384)。
        shrink_to: Some(ugoe_headless::MAX_SIZE),
        fit: args.fit,
        background: args.background,
        texture_overrides: args.textures.clone(),
        time: args.time,
        motion: args.motion.clone(),
        expression: args.expression.clone(),
        physics,
        no_physics: args.no_physics,
        no_effects: args.no_effects,
        fps: args.fps,
        lighting,
    };
    let mut prepared = ugoe_headless::prepare(scene, &options)?;
    if let Some(view) = args.view {
        prepared.view = target_view(view, prepared.size);
    }
    Ok(prepared)
}

/// `--view` の 6 数 `A,B,C,D,E,F` を読む。どれも有限の数であること。
fn parse_view(s: &str) -> Result<[f32; 6], String> {
    let values = s
        .split(',')
        .map(|v| {
            v.trim()
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("invalid number `{v}` in `{s}`"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    values
        .try_into()
        .map_err(|_| format!("expected six numbers A,B,C,D,E,F for a view, got `{s}`"))
}

/// キャンバスの px を出力の px に写す `[a, b, c, d, e, f]` に、出力の px からクリップ座標への変換
/// (Y 下向きから上向きへ) を合成したビュー。
///
/// 逆変換の無いビュー (`a*d - b*c = 0`) も受け付ける。三角形は面積を持たず、何も描かれない
/// (`docs/spec/rendering.md#ビュー変換`)。
fn target_view([a, b, c, d, e, f]: [f32; 6], (width, height): (u32, u32)) -> ugoe::render::View {
    let (sx, sy) = (2.0 / width as f32, -2.0 / height as f32);
    ugoe::render::View {
        view_proj: [
            [a * sx, b * sy, 0.0],
            [c * sx, d * sy, 0.0],
            [e * sx - 1.0, f * sy + 1.0, 1.0],
        ],
    }
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

    /// コミットした `assets/samples/avatar-atlas.ugoe` が、`avatar.ugoe` を既定の設定でアトラスにまとめたものと同じか。
    /// 違えば `just cli sample avatar --atlas -o assets/samples/avatar-atlas.ugoe` で作り直す。
    #[test]
    fn committed_atlas_sample_is_up_to_date() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |path: &str| {
            ugoe_format::read_bundle(&std::fs::read(root.join(path)).unwrap()).unwrap()
        };
        let committed = read("assets/samples/avatar-atlas.ugoe");
        let (project, _) = project_from_bundle(read("assets/samples/avatar.ugoe")).unwrap();
        let (fresh, report) = project.with_atlas(&AtlasOptions::default()).unwrap();
        assert!(report.kept.is_empty(), "{report:?}");
        assert_eq!(report.atlases.len(), 1, "{report:?}");
        let hint = "regenerate assets/samples/avatar-atlas.ugoe";
        assert_eq!(committed.model, fresh.model, "{hint}");
        assert_eq!(
            (
                &committed.physics,
                &committed.motions,
                &committed.expressions
            ),
            (&fresh.physics, &fresh.motions, &fresh.expressions),
            "{hint}"
        );
        let committed_textures: Vec<_> = committed.textures.iter().map(|t| &t.bytes[..]).collect();
        let fresh_textures: Vec<_> = fresh
            .model
            .textures
            .iter()
            .map(|t| &fresh.textures.get(&t.name).unwrap()[..])
            .collect();
        assert_eq!(committed_textures, fresh_textures, "{hint}");
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
        assert!(scene.model.meshes.iter().any(|m| m.id == "LongHair/Back"));
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

    /// `render --module --physics`: `--physics` は本体の物理の代わりで、モジュールの物理のリグはそれに加わる
    /// (`validate` と同じ。差し替えでモジュールのリグが消えない)。
    #[test]
    fn render_adds_module_rigs_to_the_physics_file() {
        let sample = sample::dress_up::build_dress_up().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let path = |name: &str| tmp.path().join(name).to_str().unwrap().to_owned();
        let physics = sample.base.physics.clone().unwrap();
        std::fs::write(
            path("base.physics.json"),
            ugoe_format::physics_to_json(&physics).unwrap(),
        )
        .unwrap();
        for (name, bytes) in sample.files().unwrap() {
            std::fs::write(path(&name), bytes).unwrap();
        }
        let cli = parse(&[
            "render",
            &path("dress-up.ugoe"),
            "-o",
            &path("never.png"),
            "--module",
            &path("long-hair.ugoemod"),
            "--physics",
            &path("base.physics.json"),
            "--time",
            "0.1",
        ])
        .unwrap();
        let Cmd::Render(args) = cli.command else {
            panic!("not render");
        };
        let prepared = prepare_render(&args).unwrap();
        let rigs: Vec<&str> = prepared
            .puppet
            .physics()
            .unwrap()
            .settings()
            .rigs
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert!(rigs.contains(&"HairSway"), "{rigs:?}");
        assert!(rigs.contains(&"LongHair/Sway"), "{rigs:?}");
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
        assert_eq!(args.gpu_timeout, ugoe_headless::DEFAULT_WAIT_TIMEOUT);
    }

    #[test]
    fn parses_the_gpu_timeout() {
        let timeout = |value: &str| {
            parse(&["render", "m.json", "-o", "o.png", "--gpu-timeout", value]).map(|cli| {
                let Cmd::Render(args) = cli.command else {
                    panic!("not render");
                };
                args.gpu_timeout
            })
        };
        assert_eq!(timeout("120").unwrap(), Duration::from_secs(120));
        assert_eq!(timeout("0").unwrap(), Duration::MAX);
        for bad in ["-1", "inf", "abc"] {
            assert!(timeout(bad).is_err(), "{bad}");
        }
    }

    /// 大きさを指定しないとき、上限 (16384) を超えるキャンバスは縮めて描く (`render_png` と同じ規則)。
    #[test]
    fn render_shrinks_a_canvas_larger_than_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let mut model = ugoe_model::fixtures::single_quad();
        model.meta.canvas.width = 40000.0;
        model.meta.canvas.height = 10000.0;
        let path = tmp.path().join("big.json");
        std::fs::write(&path, ugoe_format::model_to_json(&model).unwrap()).unwrap();
        let path = path.to_str().unwrap();

        let render = |extra: &[&str]| {
            let mut argv = vec!["render", path, "-o", "o.png"];
            argv.extend(extra);
            let Cmd::Render(args) = parse(&argv).unwrap().command else {
                panic!("not render");
            };
            prepare_render(&args)
        };
        let prepared = render(&[]).unwrap();
        assert_eq!(prepared.size, (16384, 4096));
        assert!(
            prepared.warnings[0].contains("rendered at 16384x4096"),
            "{:?}",
            prepared.warnings
        );
        // 大きさを指定すれば縮めない (警告も出さない)。
        let prepared = render(&["--width", "100"]).unwrap();
        assert_eq!(prepared.size, (100, 25));
        assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);
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
    fn parses_view_args() {
        let cli = parse(&[
            "render",
            "m.ugoe",
            "-o",
            "o.png",
            "--size",
            "4x2",
            "--view",
            "-1,0,0,1,4,0",
        ])
        .unwrap();
        let Cmd::Render(args) = cli.command else {
            panic!("not render");
        };
        assert_eq!(args.view, Some([-1.0, 0.0, 0.0, 1.0, 4.0, 0.0]));
        for bad in ["1,0,0,1,0", "1,0,0,1,0,0,0", "1,0,0,1,0,nan", "1,0,0,x,0,0"] {
            assert!(
                parse(&["render", "m.ugoe", "-o", "o.png", "--view", bad]).is_err(),
                "{bad}"
            );
        }
        // `--fit` とは同時に使えない。
        assert!(
            parse(&[
                "render",
                "m.ugoe",
                "-o",
                "o.png",
                "--fit",
                "stretch",
                "--view",
                "1,0,0,1,0,0"
            ])
            .is_err()
        );
    }

    /// `--view` はキャンバスの px を出力の px に写し、出力の px をクリップ座標 (Y 上向き) にする。
    #[test]
    fn target_view_maps_canvas_pixels_to_the_output() {
        let apply = |view: &ugoe::render::View, (x, y): (f32, f32)| {
            let m = view.view_proj;
            (
                m[0][0] * x + m[1][0] * y + m[2][0],
                m[0][1] * x + m[1][1] * y + m[2][1],
            )
        };
        // 左右の鏡映: キャンバスの (0, 0) は出力の右上 (4, 0)、(4, 2) は左下 (0, 2)。
        let view = target_view([-1.0, 0.0, 0.0, 1.0, 4.0, 0.0], (4, 2));
        assert_eq!(apply(&view, (0.0, 0.0)), (1.0, 1.0));
        assert_eq!(apply(&view, (4.0, 2.0)), (-1.0, -1.0));
        // 引き伸ばし (`--fit stretch`) と同じビュー。
        let canvas = (256.0, 128.0);
        let stretch = ugoe_headless::fit_view(canvas, (64, 32), Fit::Stretch);
        let view = target_view([0.25, 0.0, 0.0, 0.25, 0.0, 0.0], (64, 32));
        for point in [(0.0, 0.0), (256.0, 128.0), (100.0, 30.0)] {
            let (x0, y0) = apply(&stretch, point);
            let (x1, y1) = apply(&view, point);
            assert!(
                (x0 - x1).abs() < 1e-6 && (y0 - y1).abs() < 1e-6,
                "{point:?}"
            );
        }
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
