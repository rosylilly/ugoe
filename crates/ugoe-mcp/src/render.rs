//! PNG の描画。`ugoe render` (ugoe-cli) と同じ `ugoe_headless` の関数を呼ぶので、描画の規則
//! (大きさ・背景・パラメータのクランプ・テクスチャの扱い・時間の進め方) は CLI と同じになる。
//!
//! 長く動くサーバなので、GPU のドライバの不調でプロセスが落ちないように次の手当てをする。
//!
//! - GPU ([`Gpu`]) は最初の描画で用意し、以後使い回す。破棄はしない (WSL2 の dzn はデバイスの
//!   破棄時に SIGSEGV を起こすことがある。`Gpu::leak` と同じ理由)。
//! - アダプタやデバイスを用意できなければツールのエラーにし、次の呼び出しでもう一度試す。
//! - 描画中の wgpu のエラー (検証・メモリ不足) は `ugoe_headless` がエラースコープで受けてエラーにする。
//! - それでも panic したら受け止めてツールのエラーにし、その GPU を捨てて (破棄せずに手放して)、
//!   次の呼び出しで作り直す。
//! - 出力の一辺は [`MAX_SIZE`] (4096) まで、進める時間は [`MAX_TIME`] (3600 秒) までに限る。描いた PNG は
//!   base64 でエージェントに返すので、CLI (`ugoe_headless::MAX_SIZE` = 16384) より小さくする。
//! - 読み戻しは `Gpu::wait_timeout` (既定 30 秒) までしか待たない。ドライバが止まって上限を超えたら、
//!   panic のときと同じくツールのエラーにして GPU を捨てる (止まったデバイスを使い回さない。描画の錠も手放される)。
//!
//! ドライバの中の SIGSEGV のように、プロセスごと落ちる不調は防げない。そのときは MCP クライアントが
//! サーバを起動し直す (開いていたプロジェクトの未保存の変更は失われる)。

use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ugoe_editor_core::{Project, ProjectError, export_bundle};
use ugoe_headless::{Gpu, RenderOptions, Scene};

/// `render_png` で明示できる出力の一辺の上限 (`width`・`height`)。ツールのスキーマの `maximum` もこの値。
///
/// 1 回の描画で読み戻す画素 (4096 × 4096 × 4 = 64 MiB) と、base64 で返す PNG を抑えるため、CLI の上限
/// ([`ugoe_headless::MAX_SIZE`]) より小さくする。大きさを指定しないとき (キャンバスの大きさ) には効かない。
pub const MAX_SIZE: u32 = 4096;

/// `render_png` の `time` の上限 (秒)。時間は 60 fps で進めるので、`ugoe_headless::animate::MAX_UPDATES`
/// (60 fps で 1 時間) と同じ。ツールのスキーマの `maximum` もこの値。
pub const MAX_TIME: f32 = (ugoe_headless::animate::MAX_UPDATES / FPS) as f32;

/// `render_png` で時間を進めるときの 1 秒あたりの `update` の回数 (`ugoe render` の `--fps` の既定と同じ)。
const FPS: u32 = 60;

/// 描画の指定。値は `ugoe render` の同名のオプションと同じ意味。
#[derive(Debug, Clone, Default)]
pub struct RenderRequest {
    /// パラメータの ID と値。
    pub params: Vec<(String, f32)>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// `transparent` (既定)・`white`・`black`・`#RRGGBB`・`#RRGGBBAA`。
    pub background: Option<String>,
    /// モーションの名前か `.motion.json` のパス。
    pub motion: Option<String>,
    /// 表情の名前か `.expression.json` のパス。
    pub expression: Option<String>,
    /// この秒数だけ時間を進めてから描く。
    pub time: Option<f32>,
}

impl RenderRequest {
    /// `width`・`height` が 1..=[`MAX_SIZE`]、`time` が 0..=[`MAX_TIME`] に入っているか。
    fn check(&self) -> Result<(), RenderError> {
        for (name, value) in [("width", self.width), ("height", self.height)] {
            if let Some(n) = value
                && !(1..=MAX_SIZE).contains(&n)
            {
                return Err(RenderError::Invalid(format!(
                    "{name} must be in 1..={MAX_SIZE}, got {n}"
                )));
            }
        }
        if let Some(t) = self.time
            && !(0.0..=MAX_TIME).contains(&t)
        {
            return Err(RenderError::Invalid(format!(
                "time must be in 0..={MAX_TIME} seconds, got {t}"
            )));
        }
        Ok(())
    }
}

/// 描画の結果。
#[derive(Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    /// 警告 (`warning: ` で始まる行)・時間を進めた回数・出力の大きさとアダプタ。
    pub log: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("{0}")]
    Invalid(String),
    #[error("invalid background: {0}")]
    Background(String),
    #[error("failed to export the project as a bundle: {0}")]
    Export(#[from] ProjectError),
    #[error("failed to read the exported bundle: {0}")]
    Bundle(ugoe_format::FormatError),
    #[error("{}", error_chain(.0))]
    Headless(#[from] ugoe_headless::Error),
    #[error(
        "rendering panicked ({0}); the GPU device was discarded and will be recreated on the \
         next call"
    )]
    Panicked(String),
    #[error(
        "{}; the GPU device was discarded and will be recreated on the next call",
        error_chain(.0)
    )]
    TimedOut(ugoe_headless::Error),
}

/// 原因をたどって `: ` でつなぐ (`anyhow` の `{:#}` と同じ形。エージェントが原因まで読めるように)。
fn error_chain(error: &ugoe_headless::Error) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(e) = source {
        text.push_str(": ");
        text.push_str(&e.to_string());
        source = e.source();
    }
    text
}

/// 描画に使う GPU を持つ。
#[derive(Debug, Default)]
pub struct Renderer {
    /// wgpu のバックエンド (`vulkan`・`gl` など)。`None` なら `WGPU_BACKEND` か wgpu の既定。
    backend: Option<String>,
    /// 用意した GPU。破棄しないように `ManuallyDrop` に入れる。
    gpu: Option<ManuallyDrop<Gpu>>,
}

impl Renderer {
    pub fn new(backend: Option<String>) -> Self {
        Self { backend, gpu: None }
    }

    /// `project` の今のモデルを描く。
    pub fn render(
        &mut self,
        project: &Project,
        req: &RenderRequest,
    ) -> Result<Rendered, RenderError> {
        // GPU を使う前に引数とモデルを検査する (GPU が無くても誤りを返せるように)。
        req.check()?;
        let background = match &req.background {
            Some(bg) => ugoe_headless::args::parse_color(bg).map_err(RenderError::Background)?,
            None => [0; 4],
        };
        let options = RenderOptions {
            params: req.params.clone(),
            width: req.width,
            height: req.height,
            background,
            time: req.time,
            fps: FPS as f32,
            motion: req.motion.clone(),
            expression: req.expression.clone(),
            ..RenderOptions::default()
        };
        let scene = scene(project)?;
        match catch_unwind(AssertUnwindSafe(|| self.render_scene(scene, &options))) {
            Ok(result) => self.discard_hung_gpu(result),
            Err(payload) => {
                // 不調のデバイスは使い回さない。破棄もしない (ManuallyDrop のまま手放す)。
                self.gpu = None;
                Err(RenderError::Panicked(panic_message(payload.as_ref())))
            }
        }
    }

    /// GPU を待つ時間の上限を超えたら、その GPU を使い回さない (破棄もしない。ManuallyDrop のまま手放す)。
    fn discard_hung_gpu(
        &mut self,
        result: Result<Rendered, RenderError>,
    ) -> Result<Rendered, RenderError> {
        match result {
            Err(RenderError::Headless(e)) if e.is_gpu_wait_timeout() => {
                self.gpu = None;
                Err(RenderError::TimedOut(e))
            }
            result => result,
        }
    }

    fn render_scene(
        &mut self,
        scene: Scene,
        options: &RenderOptions,
    ) -> Result<Rendered, RenderError> {
        let prepared = ugoe_headless::prepare(scene, options)?;
        let mut log: Vec<String> = prepared
            .warnings
            .iter()
            .map(|w| format!("warning: {w}"))
            .collect();
        if let Some(advanced) = prepared.advanced {
            log.push(format!(
                "advanced {}s in {} update(s)",
                advanced.time, advanced.updates
            ));
        }
        let gpu = match self.gpu.take() {
            Some(gpu) => gpu,
            None => ManuallyDrop::new(Gpu::new(self.backend.as_deref())?),
        };
        let gpu = self.gpu.insert(gpu);
        let (image, _stats) = prepared.render(gpu)?;
        let png = ugoe_headless::image::encode_png_unpremultiplied(&image)?;
        log.push(format!(
            "rendered {}x{} (adapter: {})",
            prepared.size.0,
            prepared.size.1,
            gpu.adapter_description()
        ));
        Ok(Rendered {
            png,
            log: log.join("\n"),
        })
    }
}

/// プロジェクトを描くものにする。`export_bundle` で書き出したバンドルをメモリの上で読み直すので、
/// 書き出した `.ugoe` を `ugoe render` で描いたのと同じ中身になる。
fn scene(project: &Project) -> Result<Scene, RenderError> {
    let bytes = export_bundle(project)?;
    let bundle = ugoe_format::read_bundle(&bytes).map_err(RenderError::Bundle)?;
    Ok(Scene::from_bundle(bundle))
}

/// panic の値の文 (`&str` か `String`。ほかは「unknown panic」)。
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use ugoe_editor_core::ProjectMeta;

    use super::*;

    fn quad_project() -> Project {
        let model = ugoe_model::fixtures::single_quad();
        let mut textures = ugoe_editor_core::Textures::new();
        for t in &model.textures {
            let png = ugoe_headless::image::encode_png_unpremultiplied(
                &ugoe_headless::placeholder_texture(),
            )
            .unwrap();
            textures.insert(t.name.clone(), png.into());
        }
        Project::new(ProjectMeta::new("quad"), model, textures)
    }

    // T0015: 検証に通らないモデルは、検証エラーの列の共通の形 (`docs/spec/model.md#検証エラーの文`) で伝える。
    #[test]
    fn reports_validation_errors_in_the_shared_form() {
        let mut project = quad_project();
        project.model.parameters[0].default = 99.0;
        let errors = ugoe_model::validate::validate(&project.model);
        assert!(!errors.is_empty());
        let mut renderer = Renderer::default();
        let err = renderer
            .render(&project, &RenderRequest::default())
            .unwrap_err()
            .to_string();
        assert!(
            err.ends_with(&ugoe_model::validate::describe_errors("model", &errors)),
            "{err}"
        );
        assert!(renderer.gpu.is_none());
    }

    #[test]
    fn reports_bad_arguments_before_touching_the_gpu() {
        let mut renderer = Renderer::default();
        let project = quad_project();
        let err = renderer
            .render(
                &project,
                &RenderRequest {
                    params: vec![("Nope".into(), 1.0)],
                    ..RenderRequest::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown parameter `Nope` (known: X)"), "{err}");

        let err = renderer
            .render(
                &project,
                &RenderRequest {
                    background: Some("#fff".into()),
                    ..RenderRequest::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid background"), "{err}");

        let err = renderer
            .render(
                &project,
                &RenderRequest {
                    motion: Some("Idle".into()),
                    ..RenderRequest::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("no motion named `Idle`"), "{err}");
        assert!(renderer.gpu.is_none());
    }

    /// 大きさと時間の上限 (ツールのスキーマの `maximum`) を、GPU を使う前に確かめる。
    #[test]
    fn checks_the_size_and_time_limits_before_touching_the_gpu() {
        assert_eq!(MAX_TIME, 3600.0);
        let mut renderer = Renderer::default();
        let project = quad_project();
        let render = |renderer: &mut Renderer, req: RenderRequest| {
            renderer.render(&project, &req).unwrap_err().to_string()
        };
        for (width, height) in [
            (Some(MAX_SIZE + 1), None),
            (None, Some(MAX_SIZE + 1)),
            (Some(0), None),
        ] {
            let err = render(
                &mut renderer,
                RenderRequest {
                    width,
                    height,
                    ..RenderRequest::default()
                },
            );
            assert!(err.contains(&format!("must be in 1..={MAX_SIZE}")), "{err}");
        }
        for time in [MAX_TIME + 1.0, -1.0, f32::NAN] {
            let err = render(
                &mut renderer,
                RenderRequest {
                    time: Some(time),
                    ..RenderRequest::default()
                },
            );
            assert!(err.contains("time must be in 0..=3600 seconds"), "{err}");
        }
        assert!(renderer.gpu.is_none());

        // 上限ちょうどは通る (検査を通った後の誤りで止めて、GPU は使わない)。
        let err = render(
            &mut renderer,
            RenderRequest {
                width: Some(MAX_SIZE),
                height: Some(MAX_SIZE),
                time: Some(MAX_TIME),
                motion: Some("Idle".into()),
                ..RenderRequest::default()
            },
        );
        assert!(err.contains("no motion named `Idle`"), "{err}");
        assert!(renderer.gpu.is_none());
    }

    fn wait_timeout() -> RenderError {
        ugoe_headless::Error::gpu_wait_timeout(std::time::Duration::from_secs(30)).into()
    }

    /// GPU を待つ時間の上限を超えたら、GPU を捨てたことをエラーに書く。ほかのエラーはそのまま。
    #[test]
    fn reports_a_wait_timeout() {
        let mut renderer = Renderer::default();
        let other = ugoe_headless::Error::Invalid("bad".into());
        let err = renderer.discard_hung_gpu(Err(other.into())).unwrap_err();
        assert_eq!(err.to_string(), "bad");

        let err = renderer.discard_hung_gpu(Err(wait_timeout())).unwrap_err();
        assert!(matches!(err, RenderError::TimedOut(_)), "{err:?}");
        let err = err.to_string();
        assert!(err.contains("the GPU did not finish in time"), "{err}");
        assert!(err.contains("will be recreated on the next call"), "{err}");
    }

    /// 上限を超えた GPU は使い回さない。ほかのエラーでは捨てない。
    #[cfg(feature = "gpu-tests")]
    #[test]
    fn discards_the_gpu_after_a_wait_timeout() {
        let mut renderer = Renderer::default();
        let project = quad_project();
        renderer
            .render(&project, &RenderRequest::default())
            .unwrap();
        assert!(renderer.gpu.is_some());

        let other = ugoe_headless::Error::Invalid("bad".into());
        let _ = renderer.discard_hung_gpu(Err(other.into()));
        assert!(renderer.gpu.is_some());

        let _ = renderer.discard_hung_gpu(Err(wait_timeout()));
        assert!(renderer.gpu.is_none());
        // 次の呼び出しで作り直す。
        renderer
            .render(&project, &RenderRequest::default())
            .unwrap();
        assert!(renderer.gpu.is_some());
    }

    #[test]
    fn scene_carries_the_project_textures() {
        let project = quad_project();
        let scene = scene(&project).unwrap();
        assert_eq!(scene.textures.len(), project.model.textures.len());
        assert_eq!(scene.model, project.model);
    }
}
