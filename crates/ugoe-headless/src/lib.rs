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
//!
//! 毎フレーム描き続ける用途 (スタジオの仮想カメラ) には、レンダラとテクスチャを持ち続けて読み戻しを待たない
//! [`Resident`] (常駐の描画の口) を使う。

pub mod animate;
pub mod args;
mod gpu;
pub mod image;
mod resident;
mod textures;
mod view;
pub mod yuv;

use std::path::{Path, PathBuf};

use ugoe::Puppet;
use ugoe_format::TextureData;
use ugoe_model::Model;
use ugoe_render::FrameStats;

pub use animate::Animation;
pub use gpu::{Gpu, stats_json};
pub use image::Rgba8;
pub use resident::{DEFAULT_SLOTS, Readback, Resident, ResidentOptions, Wait};
pub use textures::{placeholder_texture, resolve_normal_maps, resolve_textures, texture_count};
pub use ugoe_render::{Light, Lighting};
pub use view::{Fit, MAX_SIZE, fit_view, output_size};

/// ヘッドレス描画のエラー。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 引数やモデルの誤り (描画を始める前に分かるもの)。
    #[error("{0}")]
    Invalid(String),
    /// ファイルの読み込みの失敗 (通常のファイルでないものも含む)。`context` はどのファイルか。
    #[error("{context}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    /// モーション・表情・物理の JSON の読み込みの失敗。
    #[error("{context}")]
    Format {
        context: String,
        #[source]
        source: ugoe_format::FormatError,
    },
    /// PNG のデコードの失敗。
    #[error("{context}")]
    PngDecode {
        context: &'static str,
        #[source]
        source: png::DecodingError,
    },
    /// PNG のエンコードの失敗。
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
    /// モデルの検査 (コンパイル) の失敗。
    #[error("compiling model")]
    Compile(#[source] ugoe_runtime::CompileError),
    /// アダプタを選べない (GPU もソフトウェアのラスタライザも無い)。
    #[error(
        "no GPU adapter available ({0}). Rendering needs a GPU or a software rasterizer such as \
         Mesa lavapipe (Vulkan) or llvmpipe (GL); try --backend gl or WGPU_BACKEND=vulkan"
    )]
    NoAdapter(wgpu::RequestAdapterError),
    /// デバイスを作れない。
    #[error("requesting GPU device")]
    Device(#[source] wgpu::RequestDeviceError),
    /// 描画中の GPU の失敗 (検証エラー・メモリ不足・読み戻しの失敗など)。
    #[error("{context}: {detail}")]
    Gpu {
        context: &'static str,
        detail: String,
    },
    /// レンダラが受け付けない入力 (テクスチャの大きさと画素の数の食い違いなど)。
    #[error(transparent)]
    Render(#[from] ugoe_render::RenderError),
}

impl Error {
    fn format(context: impl Into<String>) -> impl FnOnce(ugoe_format::FormatError) -> Self {
        let context = context.into();
        move |source| Self::Format { context, source }
    }
}

/// 外から渡されたパスのファイルを読む。通常のファイルでなければ (装置・FIFO・ディレクトリ) 読まずにエラーにする
/// (`/dev/zero` や FIFO を読み続けてメモリを使い果たしたり、止まったりしないように。MCP サーバではエージェントがパスを渡す)。
pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>, Error> {
    let error = |source| Error::Io {
        context: format!("reading {}", path.display()),
        source,
    };
    let metadata = std::fs::metadata(path).map_err(error)?;
    if !metadata.is_file() {
        return Err(error(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        )));
    }
    std::fs::read(path).map_err(error)
}

/// [`read_file`] で読み、UTF-8 の文字列にする。
pub(crate) fn read_text(path: &Path) -> Result<String, Error> {
    String::from_utf8(read_file(path)?).map_err(|e| Error::Io {
        context: format!("reading {}", path.display()),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.utf8_error()),
    })
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
    /// バンドルの法線マップ (添字 = テクスチャの番号。`Bundle::normal_map` で引いたもの)。JSON とフィクスチャでは空。
    pub normal_maps: Vec<Option<TextureData>>,
    /// バンドルの物理・モーション・表情。JSON とフィクスチャでは空。
    pub animation: Animation,
}

impl Scene {
    /// モデルだけ (テクスチャは不透明な白、物理・モーション・表情は無し)。
    pub fn from_model(model: Model) -> Self {
        Self {
            model,
            textures: Vec::new(),
            normal_maps: Vec::new(),
            animation: Animation::default(),
        }
    }

    /// `.ugoe` バンドルの中身。
    pub fn from_bundle(bundle: ugoe_format::Bundle) -> Self {
        let normal_maps = ugoe::lighting::bundle_normal_maps(&bundle)
            .into_iter()
            .map(|t| t.cloned())
            .collect();
        Self {
            model: bundle.model,
            textures: bundle.textures,
            normal_maps,
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
    /// キャンバスを出力にどう収めるか (`--fit`)。
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
    /// 光源 (`--ambient`・`--light`)。`None` ならレンダラに渡さない (環境光が白で光源なし。
    /// ライティングを受けるメッシュも受けない場合と同じ色になる)。
    pub lighting: Option<Lighting>,
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
            lighting: None,
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

/// ライティングの入力 (`docs/spec/rendering.md#ライティング`)。既定値は法線マップなしで、光源を渡さない。
#[derive(Debug, Clone, Default)]
pub struct Shading {
    /// 法線マップ (添字 = テクスチャの番号。RGBA8 でアルファを掛けない)。`None` の番号は法線 `(0, 0, 1)`。
    pub normal_maps: Vec<Option<Rgba8>>,
    /// 光源。`None` ならレンダラに渡さない。
    pub lighting: Option<Lighting>,
}

/// GPU で描く直前の状態 ([`prepare`] の結果)。
#[derive(Debug)]
pub struct Prepared {
    /// パラメータを書き込み、必要なら時間を進めたモデル。
    pub puppet: Puppet,
    /// 描画に使うテクスチャ (乗算済み RGBA8、番号順)。
    pub textures: Vec<Rgba8>,
    /// 法線マップと光源。
    pub shading: Shading,
    /// 出力の大きさ。
    pub size: (u32, u32),
    /// キャンバスを出力に収めるビュー ([`fit_view`])。
    pub view: ugoe_render::View,
    /// 背景色 (ストレートアルファの RGBA8)。
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
        normal_maps,
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
    // モデルの検査 (コンパイル) を先に行う。テクスチャの枚数はメッシュが参照する番号から決まるので、
    // 検査の前に揃えると範囲の外の番号 (`u32::MAX` など) の数だけ仮のテクスチャを作ってしまう。
    let (mut puppet, param_warnings) = animate::puppet(model, &options.params)?;
    let model = puppet.instance().compiled().model();
    let mut warnings = Vec::new();
    let textures = resolve_textures(model, &textures, &options.texture_overrides, &mut warnings)?;
    let shading = Shading {
        normal_maps: resolve_normal_maps(&normal_maps, &mut warnings)?,
        lighting: options.lighting.clone(),
    };
    warnings.extend(param_warnings);
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
        shading,
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
            &self.shading,
            self.size,
            &self.view,
            self.background,
        )
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    /// モデルの検査 (コンパイル) をテクスチャを揃える前に行う。範囲の外のテクスチャの番号
    /// (`u32::MAX` なら 40 億枚の仮のテクスチャ) を作り始めないように。
    #[test]
    fn rejects_invalid_models_before_resolving_textures() {
        let mut model = fixtures::single_quad();
        model.meshes[0].texture = 1;
        let options = RenderOptions {
            texture_overrides: vec![(1, "does-not-exist.png".into())],
            ..Default::default()
        };
        let err = prepare(Scene::from_model(model.clone()), &options).unwrap_err();
        assert!(matches!(err, Error::Compile(_)), "{err:?}");

        model.meshes[0].texture = u32::MAX;
        let err = prepare(Scene::from_model(model), &RenderOptions::default()).unwrap_err();
        assert!(matches!(err, Error::Compile(_)), "{err:?}");
    }

    #[test]
    fn prepares_without_advancing_time_by_default() {
        let prepared = prepare(
            Scene::from_model(fixtures::single_quad()),
            &RenderOptions {
                params: vec![("X".into(), 2.0)],
                width: Some(64),
                no_physics: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(prepared.size, (64, 64));
        assert_eq!(prepared.textures, vec![placeholder_texture()]);
        assert_eq!(prepared.advanced, None);
        // クランプの警告と、時間を進めないときの `--no-physics` の警告。
        assert_eq!(prepared.warnings.len(), 2, "{:?}", prepared.warnings);
        assert!(
            prepared.warnings[0].contains("clamped"),
            "{:?}",
            prepared.warnings
        );
        assert!(
            prepared.warnings[1].contains("only apply"),
            "{:?}",
            prepared.warnings
        );
    }

    #[test]
    fn advances_time_when_asked() {
        let prepared = prepare(
            Scene::from_model(fixtures::single_quad()),
            &RenderOptions {
                time: Some(0.5),
                fps: 10.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            prepared.advanced,
            Some(Advanced {
                time: 0.5,
                updates: 5
            })
        );
        assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);

        // fps は時間を進めるときだけ確かめる。
        let err = prepare(
            Scene::from_model(fixtures::single_quad()),
            &RenderOptions {
                time: Some(1.0),
                fps: 0.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("fps"), "{err}");
    }
}
