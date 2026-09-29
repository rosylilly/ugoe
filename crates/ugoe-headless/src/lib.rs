//! ヘッドレス描画。`ugoe render` (ugoe-cli) と MCP サーバの `render_png` (ugoe-mcp) が同じ関数を呼ぶ。
//!
//! 流れは 2 段に分かれる。
//!
//! 1. [`prepare`]: GPU を使わずに、出力の大きさと `fit` のビューを決め、テクスチャを揃え、パラメータを
//!    書き込み、必要なら時間を進める (引数の誤りは GPU の無い環境でも報告できる)。
//! 2. [`Prepared::render`]: ヘッドレスの GPU ([`Gpu`]) で描いて読み戻す。結果は乗算済みアルファの
//!    [`Rgba8`] で、[`image::encode_png_unpremultiplied`] で PNG にする。
//!
//! 描画の規則は `crates/ugoe-cli/README.md` の `ugoe render` の節。

pub mod animate;
pub mod args;
mod gpu;
pub mod image;
mod textures;
mod view;

use std::path::PathBuf;

use ugoe::Puppet;
use ugoe_format::TextureData;
use ugoe_model::Model;
use ugoe_render::FrameStats;

pub use animate::Animation;
pub use gpu::{Gpu, stats_json};
pub use image::Rgba8;
pub use textures::{placeholder_texture, resolve_textures, texture_count};
pub use view::{Fit, MAX_SIZE, fit_view, output_size};

/// ヘッドレス描画のエラー。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 引数やモデルの誤り (描画を始める前に分かるもの)。
    #[error("{0}")]
    Invalid(String),
    #[error("{context}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{context}")]
    Format {
        context: String,
        #[source]
        source: ugoe_format::FormatError,
    },
    #[error("{context}")]
    PngDecode {
        context: &'static str,
        #[source]
        source: png::DecodingError,
    },
    #[error("{context}")]
    PngEncode {
        context: &'static str,
        #[source]
        source: png::EncodingError,
    },
    /// テクスチャのデコードの失敗。`context` はどのテクスチャか。
    #[error("{context}")]
    Texture {
        context: String,
        #[source]
        source: Box<Error>,
    },
    #[error("compiling model")]
    Compile(#[source] ugoe_runtime::CompileError),
    #[error(
        "no GPU adapter available ({0}). Rendering needs a GPU or a software rasterizer such as \
         Mesa lavapipe (Vulkan) or llvmpipe (GL); try --backend gl or WGPU_BACKEND=vulkan"
    )]
    NoAdapter(wgpu::RequestAdapterError),
    #[error("requesting GPU device")]
    Device(#[source] wgpu::RequestDeviceError),
    /// 描画中の GPU の失敗 (検証エラー・メモリ不足・読み戻しの失敗など)。
    #[error("{context}: {detail}")]
    Gpu {
        context: &'static str,
        detail: String,
    },
    #[error(transparent)]
    Render(#[from] ugoe_render::RenderError),
}

impl Error {
    fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }

    fn format(context: impl Into<String>) -> impl FnOnce(ugoe_format::FormatError) -> Self {
        let context = context.into();
        move |source| Self::Format { context, source }
    }
}

/// `Invalid` のエラーを返す。
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::Error::Invalid(format!($($arg)*)))
    };
}
pub(crate) use bail;

/// 描くもの: モデルと、バンドルに入っているテクスチャ・物理・モーション・表情。
#[derive(Debug)]
pub struct Scene {
    pub model: Model,
    /// バンドルのテクスチャ (`model.textures` と同じ順)。JSON とフィクスチャでは空。
    pub textures: Vec<TextureData>,
    /// バンドルの物理・モーション・表情。JSON とフィクスチャでは空。
    pub animation: Animation,
}

impl Scene {
    /// モデルだけ (テクスチャは不透明な白、物理・モーション・表情は無し)。
    pub fn from_model(model: Model) -> Self {
        Self {
            model,
            textures: Vec::new(),
            animation: Animation::default(),
        }
    }

    /// `.ugoe` バンドルの中身。
    pub fn from_bundle(bundle: ugoe_format::Bundle) -> Self {
        Self {
            model: bundle.model,
            textures: bundle.textures,
            animation: Animation {
                physics: bundle.physics,
                motions: bundle.motions,
                expressions: bundle.expressions,
            },
        }
    }
}

/// 描き方の指定。各項目は `ugoe render` のオプションに対応する。
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// パラメータの ID と値 (`--param`)。値域の外はクランプされ、警告になる。
    pub params: Vec<(String, f32)>,
    /// 出力の大きさ (`--size`)。
    pub size: Option<(u32, u32)>,
    /// 出力の幅 (`--width`)。`height` が無ければ高さはキャンバスの縦横比から決める。
    pub width: Option<u32>,
    /// 出力の高さ (`--height`)。
    pub height: Option<u32>,
    /// 大きさを指定しないときの、キャンバスに対する倍率 (`--scale`)。
    pub scale: f32,
    pub fit: Fit,
    /// 背景色 (ストレートアルファの RGBA8。`--background`)。
    pub background: [u8; 4],
    /// テクスチャを PNG ファイルで差し替える (`--texture`)。
    pub texture_overrides: Vec<(u32, PathBuf)>,
    /// この秒数だけ時間を進めてから描く (`--time`)。
    pub time: Option<f32>,
    /// 再生するモーション: バンドルのモーションの名前か、`.motion.json` のパス (`--motion`)。
    pub motion: Option<String>,
    /// 使う表情: バンドルの表情の名前か、`.expression.json` のパス (`--expression`)。
    pub expression: Option<String>,
    /// 物理の設定 (JSON) のパス (`--physics`)。省略するとバンドルの物理を使う。
    pub physics: Option<PathBuf>,
    /// 時間を進めるときに物理を使わない (`--no-physics`)。
    pub no_physics: bool,
    /// 時間を進めるときに目パチと呼吸を止める (`--no-effects`)。
    pub no_effects: bool,
    /// 時間を進めるときの 1 秒あたりの `update` の回数 (`--fps`)。
    pub fps: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            params: Vec::new(),
            size: None,
            width: None,
            height: None,
            scale: 1.0,
            fit: Fit::Contain,
            background: [0; 4],
            texture_overrides: Vec::new(),
            time: None,
            motion: None,
            expression: None,
            physics: None,
            no_physics: false,
            no_effects: false,
            fps: 60.0,
        }
    }
}

/// 時間を進めた結果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Advanced {
    /// 進めた秒数。
    pub time: f32,
    /// `update` の回数。
    pub updates: u32,
}

/// GPU で描く直前の状態 ([`prepare`] の結果)。
#[derive(Debug)]
pub struct Prepared {
    pub puppet: Puppet,
    /// 描画に使うテクスチャ (乗算済み RGBA8、番号順)。
    pub textures: Vec<Rgba8>,
    /// 出力の大きさ。
    pub size: (u32, u32),
    pub view: ugoe_render::View,
    pub background: [u8; 4],
    /// 時間を進めたなら、その秒数と `update` の回数。
    pub advanced: Option<Advanced>,
    /// 警告 (パラメータのクランプなど)。`warning: ` は付けない。
    pub warnings: Vec<String>,
}

/// GPU を使わずに描画の準備をする。引数の誤りはここで報告する。
pub fn prepare(scene: Scene, options: &RenderOptions) -> Result<Prepared, Error> {
    let Scene {
        model,
        textures,
        animation,
    } = scene;
    let canvas = (model.meta.canvas.width, model.meta.canvas.height);
    let size = output_size(
        canvas,
        options.size,
        options.width,
        options.height,
        options.scale,
    )?;
    let view = fit_view(canvas, size, options.fit);
    let mut warnings = Vec::new();
    let textures = resolve_textures(&model, &textures, &options.texture_overrides, &mut warnings)?;
    let (mut puppet, w) = animate::puppet(model, &options.params)?;
    warnings.extend(w);
    let mut advanced = None;
    if options.time.is_some() || options.motion.is_some() || options.expression.is_some() {
        let physics = match (&options.physics, options.no_physics) {
            (Some(path), _) => Some(animate::read_physics(path)?),
            (None, false) => animation.physics,
            (None, true) => None,
        };
        let playback = animate::Playback {
            time: args::check_time(options.time.unwrap_or(0.0))?,
            fps: args::check_fps(options.fps)?,
            motion: options
                .motion
                .as_deref()
                .map(|m| animate::resolve_motion(m, &animation.motions))
                .transpose()?,
            expression: options
                .expression
                .as_deref()
                .map(|e| animate::resolve_expression(e, &animation.expressions))
                .transpose()?,
            physics,
            effects: !options.no_effects,
        };
        let time = playback.time;
        let updates = animate::advance(&mut puppet, playback)?;
        advanced = Some(Advanced { time, updates });
    } else if options.physics.is_some() || options.no_physics || options.no_effects {
        warnings.push(
            "--physics, --no-physics and --no-effects only apply with --time, --motion \
             or --expression"
                .to_owned(),
        );
    }
    Ok(Prepared {
        puppet,
        textures,
        size,
        view,
        background: options.background,
        advanced,
        warnings,
    })
}

impl Prepared {
    /// `gpu` で描いて読み戻す。結果は乗算済みアルファの画像と、描画の統計。
    pub fn render(&self, gpu: &Gpu) -> Result<(Rgba8, FrameStats), Error> {
        gpu.render(
            self.puppet.instance(),
            &self.textures,
            self.size,
            &self.view,
            self.background,
        )
    }
}
