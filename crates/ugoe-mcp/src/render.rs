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
//!   大きさを指定しないときは、キャンバスが大きければ [`MAX_SIZE`] に収まるよう縮めて描く。`width` か
//!   `height` の片方だけを指定して、縦横比から決めた他方が [`MAX_SIZE`] を超えるならエラーにする (`check_derived_size`)。
//! - 読み戻しは `Gpu::wait_timeout` (既定 30 秒。`ugoe-mcp --gpu-timeout` で変える) までしか待たない。ドライバが止まって上限を超えたら、
//!   panic のときと同じくツールのエラーにして GPU を捨てる (止まったデバイスを使い回さない。描画の錠も手放される)。
//!
//! ドライバの中の SIGSEGV のように、プロセスごと落ちる不調は防げない。そのときは MCP クライアントが
//! サーバを起動し直す (開いていたプロジェクトの未保存の変更は失われる)。

use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use ugoe_editor_core::{Project, ProjectError, export_bundle};
use ugoe_headless::{Gpu, RenderOptions, Scene};

/// `render_png` で明示できる出力の一辺の上限 (`width`・`height`)。ツールのスキーマの `maximum` もこの値。
///
/// 1 回の描画で読み戻す画素 (4096 × 4096 × 4 = 64 MiB) と、base64 で返す PNG を抑えるため、CLI の上限
/// ([`ugoe_headless::MAX_SIZE`]) より小さくする。大きさを指定しないときは、キャンバスの長い辺がこれを超えるなら
/// 縦横比を保ってこれに収まるよう縮めて描き、縮めたことを `log` に警告で書く (`RenderOptions::shrink_to`)。
pub const MAX_SIZE: u32 = 4096;

/// `render_png` の `width`・`height` の説明 (ツールのスキーマ)。
pub const SIZE_NOTE: &str = "出力の幅・高さ (画素)。片方だけなら他方はキャンバスの縦横比から決め、それが 4096 を超えるならエラー \
     (縮めない)。どちらも省くとキャンバスの大きさで描き、長い辺が 4096 を超えるなら縦横比を保って 4096 に収まるよう縮める \
     (縮めたことは log に書く)";

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

    /// 検査して `ugoe_headless` の描き方の指定にする。
    fn options(&self) -> Result<RenderOptions, RenderError> {
        self.check()?;
        let background = match &self.background {
            Some(bg) => ugoe_headless::args::parse_color(bg).map_err(RenderError::Background)?,
            None => [0; 4],
        };
        Ok(RenderOptions {
            params: self.params.clone(),
            width: self.width,
            height: self.height,
            background,
            time: self.time,
            fps: FPS as f32,
            motion: self.motion.clone(),
            expression: self.expression.clone(),
            // 大きさを指定しないときも、キャンバスが大きければ明示できる上限に収まるよう縮める (警告を `log` に書く)。
            shrink_to: Some(MAX_SIZE),
            ..RenderOptions::default()
        })
    }
}

/// `width` か `height` の片方だけを指定したとき、キャンバスの縦横比から決めた他方が [`MAX_SIZE`] に収まるか。
///
/// 明示した大きさは縮めない (`ugoe render` と同じ規則) ので、収まらなければエラーにし、収まる指定の値を示す。
/// 決めた大きさは `ugoe_headless::output_size` と同じ丸め (四捨五入) で求める。キャンバスが不正なら何もしない
/// (誤りは `output_size` が報告する)。
fn check_derived_size(canvas: (f32, f32), req: &RenderRequest) -> Result<(), RenderError> {
    let (cw, ch) = canvas;
    if !(cw > 0.0 && ch > 0.0) {
        return Ok(());
    }
    // 他方 = n × num / den (`output_size` と同じ順で計算する)。
    let (given, other, n, num, den) = match (req.width, req.height) {
        (Some(w), None) => ("width", "height", w, ch, cw),
        (None, Some(h)) => ("height", "width", h, cw, ch),
        _ => return Ok(()),
    };
    let derive = |n: u32| (n as f32 * num / den).round();
    let derived = derive(n);
    if derived <= MAX_SIZE as f32 {
        return Ok(());
    }
    // 他方が上限に収まる最大の値。丸めの境目を合わせるため、近くを順に試す。極端な縦横比では 1 でも収まらない。
    let mut max = ((MAX_SIZE as f32 + 0.5) * den / num).floor() as u32 + 1;
    while max > 0 && derive(max) > MAX_SIZE as f32 {
        max -= 1;
    }
    let hint = if max == 0 {
        format!("give the {other} instead")
    } else {
        format!("give a {given} of at most {max}, or the {other} instead")
    };
    Err(RenderError::Invalid(format!(
        "{given} {n} gives a {other} of {derived} for the {cw}x{ch} canvas, larger than \
         {MAX_SIZE}; {hint}"
    )))
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
        "{}; the GPU device was discarded and will be recreated on the next call (the server's \
         --gpu-timeout sets the limit)",
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
    /// GPU を待つ時間の上限 (`--gpu-timeout`)。`None` なら `ugoe_headless::DEFAULT_WAIT_TIMEOUT`。
    wait_timeout: Option<Duration>,
    /// 用意した GPU。破棄しないように `ManuallyDrop` に入れる。
    gpu: Option<ManuallyDrop<Gpu>>,
}

impl Renderer {
    /// `wait_timeout` は GPU を待つ時間の上限 (`Gpu::set_wait_timeout`。`Duration::MAX` なら上限なし)。
    /// `None` なら既定 (`ugoe_headless::DEFAULT_WAIT_TIMEOUT`)。
    pub fn new(backend: Option<String>, wait_timeout: Option<Duration>) -> Self {
        Self {
            backend,
            wait_timeout,
            gpu: None,
        }
    }

    /// `project` の今のモデルを描く。
    pub fn render(
        &mut self,
        project: &Project,
        req: &RenderRequest,
    ) -> Result<Rendered, RenderError> {
        // GPU を使う前に引数とモデルを検査する (GPU が無くても誤りを返せるように)。
        let options = req.options()?;
        let scene = scene(project)?;
        let canvas = &scene.model.meta.canvas;
        check_derived_size((canvas.width, canvas.height), req)?;
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
            None => {
                let mut gpu = Gpu::new(self.backend.as_deref())?;
                if let Some(timeout) = self.wait_timeout {
                    gpu.set_wait_timeout(timeout);
                }
                ManuallyDrop::new(gpu)
            }
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
        assert!(SIZE_NOTE.contains(&MAX_SIZE.to_string()), "{SIZE_NOTE}");
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

    /// 大きさを指定しないとき、キャンバスの長い辺が [`MAX_SIZE`] を超えるなら縦横比を保って縮め、`log` に書く警告を出す。
    #[test]
    fn shrinks_a_large_canvas_to_the_size_limit() {
        let mut project = quad_project();
        project.model.meta.canvas.width = 6000.0;
        project.model.meta.canvas.height = 8000.0;
        let prepare = |req: RenderRequest| {
            ugoe_headless::prepare(scene(&project).unwrap(), &req.options().unwrap()).unwrap()
        };
        let prepared = prepare(RenderRequest::default());
        assert_eq!(prepared.size, (3072, MAX_SIZE));
        assert_eq!(prepared.warnings.len(), 1, "{:?}", prepared.warnings);
        assert!(
            prepared.warnings[0]
                .contains("larger than 4096 pixels on a side; rendered at 3072x4096"),
            "{:?}",
            prepared.warnings
        );
        // 明示した大きさは縮めない。
        let prepared = prepare(RenderRequest {
            width: Some(300),
            ..RenderRequest::default()
        });
        assert_eq!(prepared.size, (300, 400));
        assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);
    }

    /// `width` か `height` の片方だけを指定して、縦横比から決めた他方が [`MAX_SIZE`] を超えるなら、縮めずにエラーにする。
    #[test]
    fn rejects_a_derived_side_over_the_size_limit() {
        let req = |width, height| RenderRequest {
            width,
            height,
            ..RenderRequest::default()
        };
        let check = |canvas, width, height| check_derived_size(canvas, &req(width, height));
        let message = |canvas, width, height| check(canvas, width, height).unwrap_err().to_string();

        // 縦長のキャンバス (1:2) で幅を指定すると、高さが上限を超える。
        let tall = (3000.0, 6000.0);
        assert_eq!(
            message(tall, Some(MAX_SIZE), None),
            "width 4096 gives a height of 8192 for the 3000x6000 canvas, larger than 4096; \
             give a width of at most 2048, or the height instead"
        );
        check(tall, Some(2048), None).unwrap();
        assert!(check(tall, Some(2049), None).is_err());
        // 高さを指定するなら、幅は上限に収まる。両方を指定すれば縦横比に関わらない。
        check(tall, None, Some(MAX_SIZE)).unwrap();
        check(tall, Some(MAX_SIZE), Some(MAX_SIZE)).unwrap();

        // 横長のキャンバス (3:1) で高さを指定すると、幅が上限を超える。
        let wide = (6000.0, 2000.0);
        assert_eq!(
            message(wide, None, Some(2000)),
            "height 2000 gives a width of 6000 for the 6000x2000 canvas, larger than 4096; \
             give a height of at most 1365, or the width instead"
        );
        check(wide, None, Some(1365)).unwrap();
        check(wide, Some(MAX_SIZE), None).unwrap();

        // 示す値は丸めの境目でも上限に収まる最大の値。
        for canvas in [
            (1000.0, 3001.0),
            (70.0, 99999.0),
            (4095.0, 4097.0),
            (1.0, 3.0),
        ] {
            let err = message(canvas, Some(MAX_SIZE), None);
            let max: u32 = err
                .split("at most ")
                .nth(1)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| panic!("{err}"));
            check(canvas, Some(max), None).unwrap_or_else(|e| panic!("{canvas:?} {max}: {e}"));
            assert!(
                check(canvas, Some(max + 1), None).is_err(),
                "{canvas:?} {max}"
            );
        }

        // 1 でも収まらない縦横比では、他方を指定させる。
        let err = message((1.0, 9000.0), Some(1), None);
        assert!(
            err.ends_with("larger than 4096; give the height instead"),
            "{err}"
        );

        // 大きさを指定しないとき・不正なキャンバスは調べない (縮めるか、`output_size` が報告する)。
        check(tall, None, None).unwrap();
        check((0.0, 1.0), Some(MAX_SIZE), None).unwrap();
    }

    /// サーバを通しても、GPU を使う前にエラーにする。決めた他方が収まれば `ugoe_headless` と同じ大きさで描く。
    #[test]
    fn checks_the_derived_side_before_touching_the_gpu() {
        let mut project = quad_project();
        project.model.meta.canvas.width = 1000.0;
        project.model.meta.canvas.height = 3000.0;
        let mut renderer = Renderer::default();
        let err = renderer
            .render(
                &project,
                &RenderRequest {
                    width: Some(2000),
                    ..RenderRequest::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("gives a height of 6000"), "{err}");
        assert!(renderer.gpu.is_none());

        let req = RenderRequest {
            width: Some(1365),
            ..RenderRequest::default()
        };
        let prepared =
            ugoe_headless::prepare(scene(&project).unwrap(), &req.options().unwrap()).unwrap();
        assert_eq!(prepared.size, (1365, 4095));
        assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);
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
