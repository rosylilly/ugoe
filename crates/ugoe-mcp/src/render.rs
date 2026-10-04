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
//! - 読み戻しは `Gpu::wait_timeout` (既定 30 秒) までしか待たない。ドライバが止まって上限を超えたら、
//!   panic のときと同じくツールのエラーにして GPU を捨てる (止まったデバイスを使い回さない。描画の錠も手放される)。
//!
//! ドライバの中の SIGSEGV のように、プロセスごと落ちる不調は防げない。そのときは MCP クライアントが
//! サーバを起動し直す (開いていたプロジェクトの未保存の変更は失われる)。

use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ugoe_editor_core::{Project, ProjectError, export_bundle};
use ugoe_headless::{Gpu, RenderOptions, Scene};

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

/// 描画の結果。
#[derive(Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    /// 警告 (`warning: ` で始まる行)・時間を進めた回数・出力の大きさとアダプタ。
    pub log: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
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
