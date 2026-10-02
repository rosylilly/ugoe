//! 仮想カメラの受け手 (ADR 0028 の 3.・5.・8.、`docs/studio.md#仮想カメラ`)。
//!
//! 指揮役のフレームを同じプロセスのチャネルで受け、ネイティブの受け手 ([`Follower`]) で物理を回し、`ugoe-headless` の
//! 常駐の描画の口 ([`Resident`]) で描いて OS の画素の形 (Windows は NV12、Linux は YUY2) で読み戻し、`ugoe-studio-vcam` に渡す。
//!
//! - 刻み (30 / 60 / 120) と仮想カメラのフレームレート (30 / 60) は別。フレームは届いたものをすべて順に適用し、描くのは
//!   仮想カメラのフレームレートだけ (ADR 0028 の 3. の「描画の頻度に依らない」)。
//! - 背景は不透明にする (透過のシーンは設定した単色で埋める)。モデルが無い間は単色だけを出す。
//! - 止めている間は、このスレッドも受け手も作らない (GPU を使わない)。GPU (`Gpu`) は一度作ったら破棄しない
//!   (`crates/ugoe-headless/README.md#gpu`)。
//! - OS の部品 (`VirtualCamera`) は、作ったスレッドで使って落とす (Windows の COM)。

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ugoe::format::TextureFormat;
use ugoe::model::module::Module;
use ugoe::render::View;
use ugoe::{Puppet, TextureSource};
use ugoe_headless::image::decode_png_premultiplied;
use ugoe_headless::yuv::{self, PixelFormat};
use ugoe_headless::{
    DEFAULT_SLOTS, Gpu, Readback, Resident, ResidentOptions, Rgba8, Scene as HeadlessScene,
    Shading, Wait, placeholder_texture, resolve_textures,
};
use ugoe_studio_vcam::{Config, Delivery, PixelLayout, VirtualCamera};

use crate::conductor;
use crate::follower::{Follower, Received};
use crate::profile::{Background, Transform};
use crate::stream::{Frame, Look, Scene as StreamScene};

/// 負荷の報告に使う、この出力のラベル (`report_render_stats` の出力ウィンドウの `output` と並ぶ)。
pub const LABEL: &str = "virtualCamera";
/// 既定の背景 (透過のシーンを埋める色)。
pub const DEFAULT_BACKGROUND: [u8; 3] = [0xf0, 0xf0, 0xf0];
/// 実際のフレームレートが設定のこの割合を下回ったら「遅れた」。
const LATE_SHARE: f32 = 0.95;
/// 負荷と状態をまとめる長さ。
const REPORT_INTERVAL: Duration = Duration::from_secs(1);

/// 操作ウィンドウで選ぶ設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// 透過のところを埋める色 (RGB)。
    pub background: [u8; 3],
}

impl Default for Settings {
    fn default() -> Self {
        let Config { width, height, fps } = Config::default();
        Self {
            width,
            height,
            fps,
            background: DEFAULT_BACKGROUND,
        }
    }
}

impl Settings {
    pub fn config(&self) -> Config {
        Config {
            width: self.width,
            height: self.height,
            fps: self.fps,
        }
    }
}

/// 仮想カメラの状態 (操作ウィンドウへの通知。`state` で分ける)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum Status {
    /// 止めている。`unavailable` は始められない理由 (Windows 11 でない・機器が無いなど。`ugoe_studio_vcam::Error::code`)。
    #[serde(rename = "stopped", rename_all = "camelCase")]
    Stopped { unavailable: Option<String> },
    /// 始めている途中。
    #[serde(rename = "starting")]
    Starting { settings: Settings },
    /// 動いている。
    #[serde(rename = "running", rename_all = "camelCase")]
    Running {
        settings: Settings,
        /// 書いている先 (Linux は `/dev/videoN`、Windows はカメラの名前)。
        device: String,
        /// 受け取るアプリがいる (Linux では分からないので、いつも `true`)。
        consumer: bool,
        /// 描画が仮想カメラのフレームレートに追いついていない。
        late: bool,
    },
    /// 止まった (始められなかった・途中で失敗した)。`code` は `ugoe_studio_vcam::Error::code` か `gpu`・`model`。
    #[serde(rename = "failed", rename_all = "camelCase")]
    Failed {
        settings: Settings,
        code: String,
        detail: String,
    },
}

/// 始められない理由 (`None` なら始められる)。
pub fn unavailable() -> Option<String> {
    ugoe_studio_vcam::availability()
        .err()
        .map(|e| e.code().to_owned())
}

/// 仮想カメラのスレッドへの知らせ。
#[derive(Debug)]
pub enum Command {
    /// シーンが変わった (シーンの通知と、受け手が取るファイル)。
    Scene(Box<SceneFiles>),
    /// 指揮役の 1 刻み分のフレーム。
    Frame(Frame),
    /// 背景の色を変える。
    Background([u8; 3]),
    /// 止める。
    Stop,
}

/// シーンの通知と、出力ウィンドウの受け手が `scene_model`・`scene_module` で取るファイル。
#[derive(Debug, Clone)]
pub struct SceneFiles {
    pub scene: StreamScene,
    /// モデルのバイト列 (モデルの無いシーンでは `None`)。
    pub model: Option<Arc<Vec<u8>>>,
    /// 付けるモジュールのバイト列 (合成の順。`scene.modules` と同じ順)。
    pub modules: Vec<Arc<Vec<u8>>>,
}

/// 仮想カメラのスレッドからの知らせ。
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Status(Status),
    /// 直近の約 1 秒の描画の負荷 (回/秒・1 回の時間の平均と最大 (ミリ秒)・捨てた数の合計)。
    Stats {
        fps: f32,
        render_ms: f32,
        render_max_ms: f32,
        dropped: u64,
    },
}

/// 仮想カメラが受け取る画素の形。
pub fn pixel_format() -> PixelFormat {
    match ugoe_studio_vcam::pixel_layout() {
        PixelLayout::Nv12 => PixelFormat::Nv12,
        PixelLayout::Yuy2 => PixelFormat::Yuy2,
    }
}

/// 描画に使う GPU。一度作ったら破棄しない (失敗も覚える)。
fn gpu() -> Result<&'static Gpu, String> {
    static GPU: OnceLock<Result<Gpu, String>> = OnceLock::new();
    GPU.get_or_init(|| Gpu::new(None).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(Clone::clone)
}

/// 受け手と常駐の描画 (OS の部品を除いた部分。GPU のテストからも使う)。
///
/// シーンの通知を受けたら、出力ウィンドウの受け手と同じ規則で、することを決める (`docs/studio.md#シーンの切り替えと受け手`):
/// モデルの版が変わったら読み直す、モジュールの版だけが変わったら付け直す、どちらも同じなら見せ方 (配置・背景) だけを変える。
#[derive(Debug)]
pub struct Feed {
    size: (u32, u32),
    format: PixelFormat,
    /// 透過のところを埋める色 (設定)。
    fill: [u8; 3],
    follower: Follower,
    resident: Option<Resident>,
    /// 読んだモデル (`model_revision`・キャンバスの大きさ・本体のテクスチャ)。
    model: Option<LoadedModel>,
    /// 付けたモジュールの版。
    modules_revision: u32,
    /// 今のシーンの見せ方。
    look: Look,
    /// モデルの無い間に出す単色の画。
    blank: Option<Vec<u8>>,
    /// 最後に適用したフレームの `seq`。
    last_seq: Option<u64>,
    /// 描かずに捨てた数 (GPU が追いついていない)。
    skipped: u64,
}

/// 読んだモデル。
#[derive(Debug)]
struct LoadedModel {
    revision: u32,
    canvas: (f32, f32),
    /// 本体のテクスチャ (乗算済み。`Model::textures` の順)。
    textures: Vec<Rgba8>,
}

impl Feed {
    pub fn new(settings: &Settings, format: PixelFormat) -> Self {
        Self {
            size: (settings.width, settings.height),
            format,
            fill: settings.background,
            follower: Follower::new(),
            resident: None,
            model: None,
            modules_revision: 0,
            look: Look::default(),
            blank: None,
            last_seq: None,
            skipped: 0,
        }
    }

    /// シーンの通知を受ける。
    pub fn set_scene(&mut self, gpu: &Gpu, files: &SceneFiles) -> Result<(), String> {
        let scene = &files.scene;
        if scene.model_revision == 0 {
            self.follower.clear();
            self.resident = None;
            self.model = None;
        } else if self.model.as_ref().map(|m| m.revision) != Some(scene.model_revision) {
            let bytes = files
                .model
                .as_deref()
                .ok_or("the scene has no model file")?;
            let bundle = conductor::read_model(bytes).map_err(|e| e.to_string())?;
            let physics = bundle.physics.clone();
            let headless = HeadlessScene::from_bundle(bundle);
            let mut warnings = Vec::new();
            let textures =
                resolve_textures(&headless.model, &headless.textures, &[], &mut warnings)
                    .map_err(|e| e.to_string())?;
            let canvas = (
                headless.model.meta.canvas.width,
                headless.model.meta.canvas.height,
            );
            let mut puppet = Puppet::new(headless.model).map_err(|e| e.to_string())?;
            if let Some(physics) = physics {
                puppet = puppet.with_physics(physics);
            }
            let modules = read_modules(&files.modules)?;
            if !modules.is_empty() {
                puppet
                    .set_modules(modules.iter().map(|(m, _)| Arc::clone(m)).collect())
                    .map_err(|e| e.to_string())?;
            }
            let model = LoadedModel {
                revision: scene.model_revision,
                canvas,
                textures,
            };
            self.resident = Some(self.resident_for(gpu, &puppet, &model, &modules)?);
            self.model = Some(model);
            self.follower.set_scene(scene.revision, puppet);
            self.last_seq = None;
        } else {
            if self.modules_revision != scene.modules_revision {
                let modules = read_modules(&files.modules)?;
                self.follower
                    .puppet_mut()
                    .ok_or("no model is loaded")?
                    .set_modules(modules.iter().map(|(m, _)| Arc::clone(m)).collect())
                    .map_err(|e| e.to_string())?;
                // 物理のリセットは、指揮役がフレームの印で揃える。テクスチャの並びが変わるので描画は作り直す。
                let puppet = self.follower.puppet().ok_or("no model is loaded")?;
                let model = self.model.as_ref().ok_or("no model is loaded")?;
                let resident = self.resident_for(gpu, puppet, model, &modules)?;
                self.resident = Some(resident);
            }
            self.follower.continue_scene(scene.revision);
        }
        self.modules_revision = scene.modules_revision;
        self.look = scene.look.clone();
        self.apply_look();
        Ok(())
    }

    /// 合成したモデルのテクスチャを揃えて、常駐の描画を作る。
    fn resident_for(
        &self,
        gpu: &Gpu,
        puppet: &Puppet,
        model: &LoadedModel,
        modules: &[LoadedModule],
    ) -> Result<Resident, String> {
        let textures = puppet
            .texture_sources()
            .into_iter()
            .map(|source| {
                let found = match source {
                    TextureSource::Base(i) => model.textures.get(i as usize),
                    TextureSource::Module { module, index } => modules
                        .iter()
                        .find(|(m, _)| m.name == module)
                        .and_then(|(_, textures)| textures.get(index as usize)),
                };
                found.cloned().unwrap_or_else(placeholder_texture)
            })
            .collect::<Vec<_>>();
        Resident::new(
            gpu,
            &textures,
            // TODO(T0808): シーンのライティング (`Look::lighting`) と法線マップを渡す (出力ウィンドウの画と比べるときに揃える)。
            &Shading::default(),
            ResidentOptions {
                size: self.size,
                format: self.format,
                background: self.clear_color(),
                view: placement_view(model.canvas, self.size, &self.look.transform),
                slots: DEFAULT_SLOTS,
            },
        )
        .map_err(|e| e.to_string())
    }

    /// フレームを 1 つ適用する。まだ知らないシーンのフレームは捨てる (シーンの知らせが先に届くので、ふつうは起きない)。
    pub fn receive(&mut self, frame: &Frame) -> Result<Received, String> {
        let received = self.follower.receive(frame).map_err(|e| e.to_string())?;
        if matches!(received, Received::Applied | Received::AppliedAfterGap) {
            self.last_seq = Some(frame.seq);
        }
        Ok(received)
    }

    /// 透過のところを埋める色を変える (次の描画から)。
    pub fn set_fill(&mut self, rgb: [u8; 3]) {
        self.fill = rgb;
        self.apply_look();
    }

    /// 見せ方 (背景・配置) を描画に当てる。
    fn apply_look(&mut self) {
        self.blank = None;
        let background = self.clear_color();
        let view = self
            .model
            .as_ref()
            .map(|m| placement_view(m.canvas, self.size, &self.look.transform));
        if let (Some(resident), Some(view)) = (&mut self.resident, view) {
            resident.set_background(background);
            resident.set_view(view);
        }
    }

    /// シーンの背景を不透明な色にする。透過と画像は設定の色で埋める。
    fn clear_color(&self) -> [u8; 4] {
        let [r, g, b] = match &self.look.background {
            Background::Color { color } | Background::Chroma { color } => {
                parse_hex(color).unwrap_or(self.fill)
            }
            // TODO(T0808): 背景の画像を描く (いまは設定の色で埋める)。
            Background::Transparent | Background::Image { .. } => self.fill,
        };
        [r, g, b, 255]
    }

    /// 今の形を描いて読み戻しを頼む (待たない)。GPU が追いついていなければ描かずに `false`。
    /// モデルが無ければ何もせず `true`。
    pub fn render(&mut self, gpu: &Gpu) -> Result<bool, String> {
        let (Some(resident), Some(puppet)) = (&mut self.resident, self.follower.puppet()) else {
            return Ok(true);
        };
        let submitted = resident
            .submit(gpu, puppet.instance(), self.last_seq.unwrap_or(0))
            .map_err(|e| e.to_string())?;
        if !submitted {
            self.skipped += 1;
        }
        Ok(submitted)
    }

    /// 読み戻しの終わった最新の画。モデルが無ければ背景の色だけの画。
    pub fn poll(&mut self, gpu: &Gpu, wait: Wait) -> Result<Option<Vec<u8>>, String> {
        match &mut self.resident {
            Some(resident) => Ok(resident
                .poll(gpu, wait)
                .map_err(|e| e.to_string())?
                .map(|Readback { bytes, .. }| bytes)),
            None => self.blank_frame().map(Some),
        }
    }

    /// 描かずに捨てた数と、読み戻したが新しい画があったので使わなかった数の合計。
    pub fn dropped(&self) -> u64 {
        self.skipped + self.resident.as_ref().map_or(0, Resident::superseded)
    }

    fn blank_frame(&mut self) -> Result<Vec<u8>, String> {
        if self.blank.is_none() {
            let (width, height) = self.size;
            let rgba = Rgba8 {
                width,
                height,
                pixels: self.clear_color().repeat(width as usize * height as usize),
            };
            self.blank = Some(yuv::convert(&rgba, self.format).map_err(|e| e.to_string())?);
        }
        Ok(self.blank.clone().unwrap_or_default())
    }
}

/// 読んだモジュールと、そのテクスチャ (乗算済み。`Module::textures` の順)。
type LoadedModule = (Arc<Module>, Vec<Rgba8>);

/// `.ugoemod` を読み、テクスチャをデコードする (合成の順)。
fn read_modules(files: &[Arc<Vec<u8>>]) -> Result<Vec<LoadedModule>, String> {
    files
        .iter()
        .map(|bytes| {
            let file = ugoe::format::read_module(bytes).map_err(|e| e.to_string())?;
            let textures = (0..file.module.textures.len())
                .map(|i| match file.textures.get(i) {
                    Some(data) if data.format == TextureFormat::Png => {
                        decode_png_premultiplied(&data.bytes)
                            .unwrap_or_else(|_| placeholder_texture())
                    }
                    _ => placeholder_texture(),
                })
                .collect();
            Ok((Arc::new(file.module), textures))
        })
        .collect()
}

/// `#rrggbb` を RGB にする。
fn parse_hex(color: &str) -> Option<[u8; 3]> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// シーンの配置 (位置・大きさ・回転) で、キャンバスを `size` の出力に写すビュー。出力ウィンドウ
/// (`apps/studio/src/lib/outputLayout.ts` の `stageLayout`・`zoomFactor` とパン) と同じ置き方: キャンバスを出力に収め
/// (contain)、大きさを掛け、キャンバスの中心を出力の中心から (x × 幅, y × 高さ) ずらした位置に置き、そこを中心に時計回りに回す。
pub fn placement_view(canvas: (f32, f32), size: (u32, u32), transform: &Transform) -> View {
    let (cw, ch) = canvas;
    let (w, h) = (size.0 as f32, size.1 as f32);
    let fit = if cw > 0.0 && ch > 0.0 {
        (w / cw).min(h / ch)
    } else {
        1.0
    };
    let s = fit * transform.scale as f32;
    let (sin, cos) = (transform.rotation as f32).to_radians().sin_cos();
    // 画素座標: p = 中心 + ずれ + R(θ) · s · (キャンバス座標 − キャンバスの中心)
    let px = w / 2.0 + transform.x as f32 * w;
    let py = h / 2.0 + transform.y as f32 * h;
    let (cx, cy) = (cw / 2.0, ch / 2.0);
    let ox = px - s * (cos * cx - sin * cy);
    let oy = py - s * (sin * cx + cos * cy);
    // 画素座標 → クリップ座標 (x: 0..w → -1..1、y: 0..h → 1..-1)。
    View {
        view_proj: [
            [2.0 * s * cos / w, -2.0 * s * sin / h, 0.0],
            [-2.0 * s * sin / w, -2.0 * s * cos / h, 0.0],
            [2.0 * ox / w - 1.0, 1.0 - 2.0 * oy / h, 1.0],
        ],
    }
}

/// 1 秒ごとの負荷の集計。
#[derive(Debug)]
struct Meter {
    start: Instant,
    count: u32,
    total: Duration,
    max: Duration,
}

impl Meter {
    fn new(now: Instant) -> Self {
        Self {
            start: now,
            count: 0,
            total: Duration::ZERO,
            max: Duration::ZERO,
        }
    }

    fn record(&mut self, took: Duration) {
        self.count += 1;
        self.total += took;
        self.max = self.max.max(took);
    }
}

/// 知らせを 1 つ適用する。止めるなら `Ok(false)`。
fn apply(feed: &mut Feed, gpu: &Gpu, command: Command) -> Result<bool, (&'static str, String)> {
    match command {
        Command::Frame(frame) => feed.receive(&frame).map(|_| true).map_err(|e| ("model", e)),
        Command::Scene(files) => feed
            .set_scene(gpu, &files)
            .map(|()| true)
            .map_err(|e| ("model", e)),
        Command::Background(rgb) => {
            feed.set_fill(rgb);
            Ok(true)
        }
        Command::Stop => Ok(false),
    }
}

/// 仮想カメラのスレッドの本体。`Command::Stop` かチャネルが閉じるまで回り、止まったら [`Status::Stopped`] を知らせる。
pub fn run(settings: Settings, commands: Receiver<Command>, events: impl Fn(Event)) {
    events(Event::Status(Status::Starting { settings }));
    let failed = |code: &str, detail: String| {
        events(Event::Status(Status::Failed {
            settings,
            code: code.to_owned(),
            detail,
        }));
    };
    let mut camera = match VirtualCamera::start(settings.config()) {
        Ok(camera) => camera,
        Err(e) => return failed(e.code(), e.to_string()),
    };
    let gpu = match gpu() {
        Ok(gpu) => gpu,
        Err(e) => return failed("gpu", e),
    };
    let device = camera.device();
    let mut feed = Feed::new(&settings, pixel_format());
    let interval = Duration::from_secs(1) / settings.fps;
    let mut next = Instant::now();
    let mut meter = Meter::new(next);
    let mut consumer = true;
    let mut late = false;
    let mut reported: Option<Status> = None;
    let report = |status: Status, reported: &mut Option<Status>| {
        if reported.as_ref() != Some(&status) {
            events(Event::Status(status.clone()));
            *reported = Some(status);
        }
    };
    report(
        Status::Running {
            settings,
            device: device.clone(),
            consumer,
            late,
        },
        &mut reported,
    );
    'outer: loop {
        // 次に描く時刻まで、届いたものを順に適用する (届いていたものはまとめて)。
        let wait = next.saturating_duration_since(Instant::now());
        let first = match commands.recv_timeout(wait) {
            Ok(command) => Some(command),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        for command in first.into_iter().chain(commands.try_iter()) {
            match apply(&mut feed, gpu, command) {
                Ok(true) => {}
                Ok(false) => break 'outer,
                Err((code, detail)) => return failed(code, detail),
            }
        }
        if Instant::now() < next {
            continue;
        }
        let started = Instant::now();
        next = (next + interval).max(started);
        let sent = feed
            .render(gpu)
            .and_then(|_| feed.poll(gpu, Wait::No))
            .and_then(|frame| match frame {
                Some(frame) => camera.send(&frame).map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            });
        match sent {
            Ok(Some(delivery)) => {
                consumer = delivery == Delivery::Delivered;
                meter.record(started.elapsed());
            }
            Ok(None) => {}
            Err(e) => return failed("os", e),
        }
        let now = Instant::now();
        let elapsed = now.duration_since(meter.start);
        if elapsed >= REPORT_INTERVAL {
            let fps = meter.count as f32 / elapsed.as_secs_f32();
            late = fps < settings.fps as f32 * LATE_SHARE;
            let count = meter.count.max(1);
            events(Event::Stats {
                fps,
                render_ms: meter.total.as_secs_f32() * 1000.0 / count as f32,
                render_max_ms: meter.max.as_secs_f32() * 1000.0,
                dropped: feed.dropped(),
            });
            meter = Meter::new(now);
        }
        report(
            Status::Running {
                settings,
                device: device.clone(),
                consumer,
                late,
            },
            &mut reported,
        );
    }
    // 読み戻しを待ち終えてから落とす (バッファを使ったまま捨てない)。
    let _ = feed.poll(gpu, Wait::All);
    drop(camera);
    events(Event::Status(Status::Stopped {
        unavailable: unavailable(),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_tagged_json() {
        let settings = Settings::default();
        assert_eq!(
            serde_json::to_string(&Status::Running {
                settings,
                device: "/dev/video10".into(),
                consumer: true,
                late: false,
            })
            .unwrap(),
            concat!(
                r#"{"state":"running","settings":{"width":1280,"height":720,"fps":30,"#,
                r#""background":[240,240,240]},"device":"/dev/video10","consumer":true,"late":false}"#
            )
        );
        assert_eq!(
            serde_json::to_string(&Status::Stopped {
                unavailable: Some("noDevice".into())
            })
            .unwrap(),
            r#"{"state":"stopped","unavailable":"noDevice"}"#
        );
    }

    #[test]
    fn settings_round_trip() {
        let json = r#"{"width":1920,"height":1080,"fps":60,"background":[0,255,0]}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.config().fps, 60);
        assert_eq!(settings.background, [0, 255, 0]);
    }

    fn apply(view: &View, (x, y): (f32, f32), size: (u32, u32)) -> (f32, f32) {
        let m = view.view_proj;
        let clip = (
            m[0][0] * x + m[1][0] * y + m[2][0],
            m[0][1] * x + m[1][1] * y + m[2][1],
        );
        // クリップ座標 → 画素座標。
        (
            (clip.0 + 1.0) / 2.0 * size.0 as f32,
            (1.0 - clip.1) / 2.0 * size.1 as f32,
        )
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn the_default_placement_fits_the_canvas_like_the_output_window() {
        let canvas = (200.0, 100.0);
        let size = (400, 400);
        let view = placement_view(canvas, size, &Transform::default());
        assert_eq!(
            view,
            ugoe_headless::fit_view(canvas, size, ugoe_headless::Fit::Contain)
        );
    }

    #[test]
    fn placement_moves_scales_and_rotates_around_the_placed_center() {
        let canvas = (200.0, 100.0);
        let size = (400, 200);
        let transform = Transform {
            x: 0.25,
            y: -0.1,
            scale: 0.5,
            rotation: 90.0,
        };
        let view = placement_view(canvas, size, &transform);
        // キャンバスの中心は、出力の中心から (0.25 × 400, -0.1 × 200) = (100, -20) ずれた位置。
        let center = (300.0, 80.0);
        assert!(close(apply(&view, (100.0, 50.0), size), center));
        // 収める倍率は 2、大きさ 0.5 で 1。右へ 10 の点は、時計回りに 90 度回って下へ 10。
        assert!(close(apply(&view, (110.0, 50.0), size), (300.0, 90.0)));
        assert!(close(apply(&view, (100.0, 40.0), size), (310.0, 80.0)));
    }

    #[test]
    fn scene_colors_are_parsed() {
        assert_eq!(parse_hex("#00ff7f"), Some([0, 255, 127]));
        assert_eq!(parse_hex("00ff7f"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("#gg0000"), None);
    }

    #[test]
    fn the_layout_matches_the_os() {
        let expected = if cfg!(windows) {
            PixelFormat::Nv12
        } else {
            PixelFormat::Yuy2
        };
        assert_eq!(pixel_format(), expected);
    }

    /// 仮想カメラが始められない環境 (CI・WSL) では、失敗を知らせて止まる (GPU を使わない)。
    #[test]
    fn a_missing_device_is_reported_without_rendering() {
        if ugoe_studio_vcam::availability().is_ok() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        drop(tx);
        let events = std::sync::Mutex::new(Vec::new());
        run(Settings::default(), rx, |e| events.lock().unwrap().push(e));
        let events = events.into_inner().unwrap();
        assert!(matches!(events[0], Event::Status(Status::Starting { .. })));
        assert!(
            matches!(&events[1], Event::Status(Status::Failed { code, .. }) if !code.is_empty()),
            "{events:?}"
        );
    }
}
