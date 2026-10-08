//! 仮想カメラの受け手 (ADR 0028 の 3.・5.・8.、`docs/studio.md#仮想カメラ`)。
//!
//! 指揮役のフレームを同じプロセスのチャネルで受け、ネイティブの受け手 ([`Follower`]) で物理を回し、`ugoe-headless` の
//! 常駐の描画の口 ([`Resident`]) で描いて OS の画素の形 (Windows は NV12、Linux は YUY2) で読み戻し、`ugoe-studio-vcam` に渡す。
//!
//! - 刻み (30 / 60 / 120) と仮想カメラのフレームレート (30 / 60) は別。フレームは届いたものをすべて順に適用し、描くのは
//!   仮想カメラのフレームレートだけ (ADR 0028 の 3. の「描画の頻度に依らない」)。
//! - 背景は不透明にする (透過のシーンは設定した単色で埋める)。画像の背景は出力ウィンドウと同じく出力を覆うように置き
//!   (透過のところは設定の色で埋める)、モデルが無い間は背景だけを出す。
//! - 光源はシーンの光源を出力ウィンドウと同じ規則でレンダラに渡し ([`crate::lighting`])、モデルとモジュールの法線マップも渡す (T0808)。
//! - 止めている間は、このスレッドも受け手も作らない (GPU を使わない)。GPU (`Gpu`) は一度作ったら破棄しない
//!   (`crates/ugoe-headless/README.md#gpu`)。GPU を待つ時間の上限を超えたら (止めるときの待ち・動いている間に読み戻しが
//!   [`STALL_LIMIT`] 来ない)、その GPU を手放して次の開始で作り直す (T0832)。描画中に GPU がエラーを返したときも
//!   (`gpuError`。デバイスの喪失など) 手放す (T0836)。
//! - GPU は始めるときの設定の希望 (「使う GPU」。T0860、`docs/studio.md#使う-gpu`) で作る。希望が変わっていれば、次の開始で
//!   前の GPU を (破棄せずに) 手放して作り直す。同じ希望なら作り直さない。
//! - シーンのモデル・モジュール・背景の画像を読めなくても止まらず、出力ウィンドウの受け手と同じ報告 ([`Event::LoadFailures`]) で
//!   読めなかったものをすべて知らせる (T0836・T0840、`docs/studio.md#受け手が読めなかったとき`)。
//! - OS の部品 (`VirtualCamera`) は、作ったスレッドで使って落とす (Windows の COM)。

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ugoe::format::TextureData;
use ugoe::format::image::ImageError;
use ugoe::model::module::Module;
use ugoe::render::View;
use ugoe::{Puppet, TextureSource};
use ugoe_headless::image::{
    self, decode_image, decode_image_premultiplied, decode_png_premultiplied,
};
use ugoe_headless::yuv::{self, PixelFormat};
use ugoe_headless::{
    DEFAULT_SLOTS, Gpu, PowerPreference, Readback, Resident, ResidentOptions, Rgba8,
    Scene as HeadlessScene, Shading, Wait, placeholder_texture, texture_count,
};
use ugoe_studio_vcam::{Config, Delivery, PixelLayout, VirtualCamera};

use crate::conductor;
use crate::follower::{self, Follower, Received};
use crate::lighting::scene_lighting;
use crate::load_failure::{LoadFailure, LoadFailureCode};
use crate::profile::{Background, Transform};
use crate::stage::Asset;
use crate::stream::{Frame, Look, Scene as StreamScene};

/// 負荷の報告に使う、この出力のラベル (`report_render_stats` の出力ウィンドウの `output` と並ぶ)。
pub const LABEL: &str = "virtualCamera";
/// 既定の背景 (透過のシーンを埋める色)。
pub const DEFAULT_BACKGROUND: [u8; 3] = [0xf0, 0xf0, 0xf0];
/// 実際のフレームレートが設定のこの割合を下回ったら「遅れた」。
const LATE_SHARE: f32 = 0.95;
/// 負荷と状態をまとめる長さ。
const REPORT_INTERVAL: Duration = Duration::from_secs(1);
/// 動いている間に、待っている読み戻しがこの長さ来なければ GPU が止まったとみなす (`gpuTimeout` で止める)。
/// ふつうの GPU は 1 枚を数十ミリ秒で描くので、追いつかない (「遅れています」) と取り違えない長さにしてある。
pub const STALL_LIMIT: Duration = Duration::from_secs(10);

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
    /// 止まった (始められなかった・途中で失敗した)。`code` は `ugoe_studio_vcam::Error::code` か
    /// `gpu` (GPU が無い)・`gpuError` (描画中に GPU がエラーを返した)・`gpuTimeout` (GPU が止まった)・
    /// `model` (フレームをモデルに当てられない)・`os`。シーンのファイルを読めなかったことは止まる理由にせず、
    /// [`Event::LoadFailures`] で知らせる。
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
    /// 付けるモジュールのバイト列 (合成の順。`scene.modules` と同じ順。受け取れないものは `None`)。
    pub modules: Vec<Option<Arc<Vec<u8>>>>,
    /// 背景の画像のバイト列 (画像の背景のシーンだけ。`scene_background`)。
    pub background: Option<Arc<Vec<u8>>>,
}

/// 仮想カメラのスレッドからの知らせ。
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Status(Status),
    /// 直近の約 1 秒の描画の負荷 (回/秒・1 回の時間の平均と最大 (ミリ秒)・捨てた数の合計) と、描いている GPU
    /// (wgpu のバックエンドの名前とアダプタの名前。T0860)。
    Stats {
        fps: f32,
        render_ms: f32,
        render_max_ms: f32,
        dropped: u64,
        backend: &'static str,
        adapter: String,
    },
    /// 今のシーンのファイルを読めなかった (`load_failure.rs`。受け手のラベルは [`LABEL`])。1 つのシーンで読めなかったものすべて
    /// (空にはしない)。
    LoadFailures(Vec<LoadFailure>),
}

/// 受け手と描画のエラー (仮想カメラを止める理由)。
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    /// 描画の準備・描画・読み戻しの失敗 (検証エラー・デバイスの喪失・待つ時間の上限を超えたなど)。
    #[error(transparent)]
    Gpu(#[from] ugoe_headless::Error),
    /// フレームをモデルに当てられない (値の並びが違うなど)。
    #[error("{0}")]
    Model(String),
}

impl FeedError {
    /// 止まった理由のコード ([`Status::Failed`] の `code`)。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Gpu(e) if e.is_gpu_wait_timeout() => "gpuTimeout",
            Self::Gpu(_) => "gpuError",
            Self::Model(_) => "model",
        }
    }
}

/// 仮想カメラが受け取る画素の形。
pub fn pixel_format() -> PixelFormat {
    match ugoe_studio_vcam::pixel_layout() {
        PixelLayout::Nv12 => PixelFormat::Nv12,
        PixelLayout::Yuy2 => PixelFormat::Yuy2,
    }
}

/// 描画に使う GPU。
static GPU: GpuCell<Gpu> = GpuCell::new();

/// 描画に使う GPU の置き場。作った GPU は破棄しない (`Box::leak`。`crates/ugoe-headless/README.md#gpu`)。作れなかったことも覚える。
/// 待つ時間の上限を超えた GPU は [`GpuCell::discard`] で手放し、次の [`GpuCell::get`] で作り直す。
/// 作ったときの電力の希望も覚え、違う希望で求められたら前の GPU を手放して作り直す (作れなかったことも、違う希望なら試し直す)。
#[derive(Debug)]
struct GpuCell<T: 'static> {
    slot: Mutex<Option<MadeGpu<T>>>,
}

/// 作ったときの電力の希望と、作った GPU (か作れなかった理由)。
type MadeGpu<T> = (Option<PowerPreference>, Result<&'static T, String>);

impl<T> GpuCell<T> {
    const fn new() -> Self {
        Self {
            slot: Mutex::new(None),
        }
    }

    /// 希望 `power` で作った今の GPU。無ければ (違う希望で作ったものしか無ければ) `make` で作る。
    fn get(
        &self,
        power: Option<PowerPreference>,
        make: impl FnOnce() -> Result<T, String>,
    ) -> Result<&'static T, String> {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            Some((made, result)) if *made == power => result.clone(),
            _ => {
                // 違う希望で作った GPU は手放すだけで破棄しない (`Box::leak` したまま)。
                let result = make().map(|gpu| &*Box::leak(Box::new(gpu)));
                *slot = Some((power, result.clone()));
                result
            }
        }
    }

    /// `gpu` を手放す (破棄はしない)。すでに作り直していれば何もしない。
    fn discard(&self, gpu: &T) {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*slot, Some((_, Ok(current))) if std::ptr::eq(current, gpu)) {
            *slot = None;
        }
    }
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
    /// 付けたモジュールの版 (読めなかったモジュールがあれば `None`。次のシーンの通知で付け直す)。
    modules_revision: Option<u32>,
    /// 今のシーンの見せ方。
    look: Look,
    /// デコードした背景の画像 (パスと、乗算済みの画)。読めなかった画像は `None` の画 (次のシーンの通知で読み直す)。
    image: Option<(PathBuf, Option<Rgba8>)>,
    /// 出力を覆うように置いて透過のところを埋めた背景の画像 (画像の背景で、読めたときだけ)。
    backdrop: Option<Rgba8>,
    /// モデルの無い間に出す背景だけの画。
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
    /// 本体の法線マップ (乗算しない。`Model::textures` の順。無いものは `None`)。
    normal_maps: Vec<Option<Rgba8>>,
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
            modules_revision: None,
            look: Look::default(),
            image: None,
            backdrop: None,
            blank: None,
            last_seq: None,
            skipped: 0,
        }
    }

    /// シーンの通知を受ける。今のシーンのモデル・モジュール・背景の画像で読めなかったものを、すべて返す
    /// (出力ウィンドウの受け手と同じ扱い。`docs/studio.md#受け手が読めなかったとき`)。並びはモジュール (番号の順)・背景の画像の順。
    ///
    /// - モデルを読めなければ前のモデルを残し、このシーンのフレームを当てない (見せ方も変えない)。次のシーンの通知で読み直す。
    ///   モジュールと背景の画像はこのシーンでは読まないので、返すのはモデルのことだけ。
    /// - モジュールを読めなければ、モデルと読めたモジュールで出す。次のシーンの通知で付け直す。
    /// - 背景の画像を読めなければ設定の色で埋める。次のシーンの通知で読み直す。
    pub fn set_scene(
        &mut self,
        gpu: &Gpu,
        files: &SceneFiles,
    ) -> Result<Vec<LoadFailure>, FeedError> {
        let scene = &files.scene;
        let failure = |asset, index, code| LoadFailure {
            revision: scene.revision,
            asset,
            index,
            code,
        };
        let module_failures = |skipped: Vec<(usize, LoadFailureCode)>| {
            skipped
                .into_iter()
                .map(|(index, code)| failure(Asset::Module, Some(index), code))
                .collect::<Vec<_>>()
        };
        let mut unread = Vec::new();
        // モデルを読み直したか、モジュールを付け直したか (ポーズの要素をフェードせずに表示する)。
        let mut reloaded = false;
        if scene.model_revision == 0 {
            self.follower.clear();
            self.resident = None;
            self.model = None;
            self.modules_revision = Some(scene.modules_revision);
        } else if self.model.as_ref().map(|m| m.revision) != Some(scene.model_revision) {
            let Some(bytes) = files.model.as_deref() else {
                return Ok(vec![failure(
                    Asset::Model,
                    None,
                    LoadFailureCode::Unavailable,
                )]);
            };
            let Some((mut puppet, model)) = read_model(bytes, scene.model_revision) else {
                return Ok(vec![failure(
                    Asset::Model,
                    None,
                    LoadFailureCode::Unreadable,
                )]);
            };
            let (modules, skipped) = attach_modules(&mut puppet, &files.modules);
            unread = module_failures(skipped);
            self.look = scene.look.clone();
            self.resident = Some(self.resident_for(gpu, &puppet, &model, &modules)?);
            self.model = Some(model);
            self.follower
                .set_scene(scene.revision, puppet, &scene.part_ids);
            self.last_seq = None;
            reloaded = true;
            self.modules_revision = unread.is_empty().then_some(scene.modules_revision);
        } else {
            if self.modules_revision != Some(scene.modules_revision) {
                let puppet = self
                    .follower
                    .puppet_mut()
                    .ok_or_else(|| FeedError::Model("no model is loaded".to_owned()))?;
                let (modules, skipped) = attach_modules(puppet, &files.modules);
                unread = module_failures(skipped);
                self.look = scene.look.clone();
                // 物理のリセットは、指揮役がフレームの印で揃える。テクスチャの並びが変わるので描画は作り直す。
                let (Some(puppet), Some(model)) = (self.follower.puppet(), self.model.as_ref())
                else {
                    return Err(FeedError::Model("no model is loaded".to_owned()));
                };
                let resident = self.resident_for(gpu, puppet, model, &modules)?;
                self.resident = Some(resident);
                self.modules_revision = unread.is_empty().then_some(scene.modules_revision);
                reloaded = true;
            }
            self.follower
                .continue_scene(scene.revision, &scene.part_ids);
        }
        self.look = scene.look.clone();
        // ポーズの要素は読み直し・付け直しの後に表示させる (受け手のポーズは受け手が進める。T0842)。
        // 読み直し・付け直しの直後はフェードせずに表示する (T0844)。
        if let Some(puppet) = self.follower.puppet_mut() {
            follower::show_pose(puppet, &scene.look.pose, reloaded);
        }
        if let Some(code) = self.read_image(files.background.as_deref()) {
            unread.push(failure(Asset::Background, None, code));
        }
        self.apply_look(gpu)?;
        Ok(unread)
    }

    /// 画像の背景をデコードする (同じパスで読めたものは読み直さない)。読めない画像は設定の色で埋め、その理由を返す。
    fn read_image(&mut self, bytes: Option<&Vec<u8>>) -> Option<LoadFailureCode> {
        let Background::Image { path } = &self.look.background else {
            self.image = None;
            return None;
        };
        if self
            .image
            .as_ref()
            .is_some_and(|(p, image)| p == path && image.is_some())
        {
            return None;
        }
        let (decoded, code) = match bytes {
            None => (None, Some(LoadFailureCode::Unavailable)),
            Some(bytes) => match decode_background(bytes) {
                Ok(image) => (Some(image), None),
                Err(_) => (None, Some(LoadFailureCode::Unreadable)),
            },
        };
        self.image = Some((path.clone(), decoded));
        code
    }

    /// 合成したモデルのテクスチャを揃えて、常駐の描画を作る。
    fn resident_for(
        &self,
        gpu: &Gpu,
        puppet: &Puppet,
        model: &LoadedModel,
        modules: &[LoadedModule],
    ) -> Result<Resident, ugoe_headless::Error> {
        let (textures, normal_maps): (Vec<_>, Vec<_>) = puppet
            .texture_sources()
            .into_iter()
            .map(|source| {
                let (texture, normal_map) = match source {
                    TextureSource::Base(i) => (
                        model.textures.get(i as usize),
                        model.normal_maps.get(i as usize).and_then(Option::as_ref),
                    ),
                    TextureSource::Module { module, index } => {
                        let m = modules.iter().find(|m| m.module.name == module);
                        (
                            m.and_then(|m| m.textures.get(index as usize)),
                            m.and_then(|m| m.normal_maps.get(index as usize))
                                .and_then(Option::as_ref),
                        )
                    }
                };
                (
                    texture.cloned().unwrap_or_else(placeholder_texture),
                    normal_map.cloned(),
                )
            })
            .unzip();
        let shading = Shading {
            normal_maps,
            lighting: scene_lighting(self.look.lighting.as_ref(), model.canvas),
        };
        Resident::new(
            gpu,
            &textures,
            &shading,
            ResidentOptions {
                size: self.size,
                format: self.format,
                background: self.clear_color(),
                view: placement_view(model.canvas, self.size, &self.look.transform),
                slots: DEFAULT_SLOTS,
                cpu_yuv: false,
            },
        )
    }

    /// フレームを 1 つ適用する。まだ知らないシーンのフレームは捨てる (シーンの知らせが先に届くので、ふつうは起きない)。
    pub fn receive(&mut self, frame: &Frame) -> Result<Received, FeedError> {
        let received = self
            .follower
            .receive(frame)
            .map_err(|e| FeedError::Model(e.to_string()))?;
        if matches!(received, Received::Applied | Received::AppliedAfterGap) {
            self.last_seq = Some(frame.seq);
        }
        Ok(received)
    }

    /// 透過のところを埋める色を変える (次の描画から)。
    pub fn set_fill(&mut self, gpu: &Gpu, rgb: [u8; 3]) -> Result<(), ugoe_headless::Error> {
        self.fill = rgb;
        self.apply_look(gpu)
    }

    /// 見せ方 (背景・配置・光源) を描画に当てる。
    fn apply_look(&mut self, gpu: &Gpu) -> Result<(), ugoe_headless::Error> {
        self.blank = None;
        self.backdrop = match &self.image {
            Some((_, Some(image))) => {
                let mut covered = image::cover(image, self.size);
                image::over_color(&mut covered, self.fill);
                Some(covered)
            }
            _ => None,
        };
        let background = self.clear_color();
        let Some(model) = &self.model else {
            return Ok(());
        };
        let view = placement_view(model.canvas, self.size, &self.look.transform);
        let lighting = scene_lighting(self.look.lighting.as_ref(), model.canvas);
        if let Some(resident) = &mut self.resident {
            resident.set_background(background);
            resident.set_view(view);
            resident.set_lighting(lighting.as_ref());
            resident.set_backdrop(gpu, self.backdrop.as_ref())?;
        }
        Ok(())
    }

    /// シーンの背景を不透明な色にする。透過と画像 (の透過のところ・読めなかった画像) は設定の色で埋める。
    fn clear_color(&self) -> [u8; 4] {
        let [r, g, b] = match &self.look.background {
            Background::Color { color } | Background::Chroma { color } => {
                parse_hex(color).unwrap_or(self.fill)
            }
            Background::Transparent | Background::Image { .. } => self.fill,
        };
        [r, g, b, 255]
    }

    /// 今の形を描いて読み戻しを頼む (待たない)。GPU が追いついていなければ描かずに `false`。
    /// モデルが無ければ何もせず `true`。
    pub fn render(&mut self, gpu: &Gpu) -> Result<bool, ugoe_headless::Error> {
        let (Some(resident), Some(puppet)) = (&mut self.resident, self.follower.puppet()) else {
            return Ok(true);
        };
        let submitted = resident.submit(gpu, puppet.instance(), self.last_seq.unwrap_or(0))?;
        if !submitted {
            self.skipped += 1;
        }
        Ok(submitted)
    }

    /// 読み戻しの終わった最新の画。モデルが無ければ背景の色だけの画。
    ///
    /// [`Wait::All`] で GPU を待つ時間の上限を超えたら、そのエラー ([`ugoe_headless::Error::is_gpu_wait_timeout`]) を返す。
    pub fn poll(&mut self, gpu: &Gpu, wait: Wait) -> Result<Option<Vec<u8>>, ugoe_headless::Error> {
        match &mut self.resident {
            Some(resident) => Ok(resident
                .poll(gpu, wait)?
                .map(|Readback { bytes, .. }| bytes)),
            None => self.blank_frame().map(Some),
        }
    }

    /// 読み戻しを待っているものがあるか。
    pub fn waiting(&self) -> bool {
        self.resident.as_ref().is_some_and(|r| r.in_flight() > 0)
    }

    /// 描かずに捨てた数と、読み戻したが新しい画があったので使わなかった数の合計。
    pub fn dropped(&self) -> u64 {
        self.skipped + self.resident.as_ref().map_or(0, Resident::superseded)
    }

    fn blank_frame(&mut self) -> Result<Vec<u8>, ugoe_headless::Error> {
        if self.blank.is_none() {
            let (width, height) = self.size;
            let rgba = self.backdrop.clone().unwrap_or_else(|| Rgba8 {
                width,
                height,
                pixels: self.clear_color().repeat(width as usize * height as usize),
            });
            self.blank = Some(yuv::convert(&rgba, self.format)?);
        }
        Ok(self.blank.clone().unwrap_or_default())
    }
}

/// 背景の画像の一辺の上限 (画素)。これを超える画像は読まない (巨大な画像でデコードが固まらないように)。
pub const MAX_BACKGROUND_SIDE: u32 = 8192;
/// 背景の画像の画素の数の上限 (RGBA で 128 MiB)。
pub const MAX_BACKGROUND_PIXELS: u64 = 32 * 1024 * 1024;

/// 背景の画像を読めない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackgroundError {
    #[error("the background image is not a PNG, JPEG or WebP image")]
    Format,
    #[error("the background image is {width}x{height}, which is too large")]
    TooLarge { width: u32, height: u32 },
    #[error("cannot decode the background image: {0}")]
    Decode(String),
}

/// 大きさが上限の内か。
fn check_background_size(width: u32, height: u32) -> Result<(), BackgroundError> {
    if width == 0
        || height == 0
        || width > MAX_BACKGROUND_SIDE
        || height > MAX_BACKGROUND_SIDE
        || u64::from(width) * u64::from(height) > MAX_BACKGROUND_PIXELS
    {
        return Err(BackgroundError::TooLarge { width, height });
    }
    Ok(())
}

/// 背景の画像 (PNG・JPEG・WebP。中身の先頭で見分ける) を乗算済みの RGBA8 にする。デコードの前に大きさを確かめ、
/// 上限 ([`MAX_BACKGROUND_SIDE`]・[`MAX_BACKGROUND_PIXELS`]) を超えるものは読まない。
pub fn decode_background(bytes: &[u8]) -> Result<Rgba8, BackgroundError> {
    let decode = |e: &dyn std::fmt::Display| BackgroundError::Decode(e.to_string());
    let mut image = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        // IHDR の幅と高さ (先頭から 16 バイト目から、ビッグエンディアンの u32 が 2 つ)。
        let size = |i: usize| {
            bytes
                .get(i..i + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or(BackgroundError::Format)
        };
        check_background_size(size(16)?, size(20)?)?;
        return decode_png_premultiplied(bytes).map_err(|e| decode(&e));
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        use zune_jpeg::zune_core::colorspace::ColorSpace;
        use zune_jpeg::zune_core::options::DecoderOptions;
        let options = DecoderOptions::default()
            .set_max_width(MAX_BACKGROUND_SIDE as usize)
            .set_max_height(MAX_BACKGROUND_SIDE as usize)
            .jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut decoder =
            zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(bytes), options);
        decoder.decode_headers().map_err(|e| decode(&e))?;
        let info = decoder.info().ok_or(BackgroundError::Format)?;
        let (width, height) = (u32::from(info.width), u32::from(info.height));
        check_background_size(width, height)?;
        let pixels = decoder.decode().map_err(|e| decode(&e))?;
        if decoder.output_colorspace() != Some(ColorSpace::RGBA)
            || pixels.len() != width as usize * height as usize * 4
        {
            return Err(BackgroundError::Decode(
                "unexpected JPEG output layout".to_owned(),
            ));
        }
        Rgba8 {
            width,
            height,
            pixels,
        }
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        let mut decoder =
            image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).map_err(|e| decode(&e))?;
        let (width, height) = decoder.dimensions();
        check_background_size(width, height)?;
        let channels = if decoder.has_alpha() { 4 } else { 3 };
        let mut buf = vec![0u8; width as usize * height as usize * channels];
        decoder.read_image(&mut buf).map_err(|e| decode(&e))?;
        let pixels = if channels == 4 {
            buf
        } else {
            buf.as_chunks::<3>()
                .0
                .iter()
                .flat_map(|[r, g, b]| [*r, *g, *b, 255])
                .collect()
        };
        Rgba8 {
            width,
            height,
            pixels,
        }
    } else {
        return Err(BackgroundError::Format);
    };
    image::premultiply(&mut image.pixels);
    Ok(image)
}

/// 読んだモジュール。
#[derive(Debug)]
struct LoadedModule {
    module: Arc<Module>,
    /// テクスチャ (乗算済み。`Module::textures` の順)。
    textures: Vec<Rgba8>,
    /// 法線マップ (乗算しない。`Module::textures` の順。無いもの・読めないものは `None`)。
    normal_maps: Vec<Option<Rgba8>>,
}

/// モデル (`.ugoe`・JSON) を読み、物理を有効にした `Puppet` と、デコードしたテクスチャ・法線マップを返す。読めなければ `None`。
fn read_model(bytes: &[u8], revision: u32) -> Option<(Puppet, LoadedModel)> {
    let bundle = conductor::read_model(bytes).ok()?;
    let physics = bundle.physics.clone();
    let headless = HeadlessScene::from_bundle(bundle);
    let count = texture_count(&headless.model);
    let normal_maps = headless
        .normal_maps
        .iter()
        .map(Option::as_ref)
        .chain(std::iter::repeat(None));
    let (textures, normal_maps) = decode_textures(headless.textures.iter().map(Some), normal_maps)
        .take(count)
        .collect::<Option<(Vec<_>, Vec<_>)>>()?;
    let canvas = (
        headless.model.meta.canvas.width,
        headless.model.meta.canvas.height,
    );
    let mut puppet = Puppet::new(headless.model).ok()?;
    if let Some(physics) = physics {
        puppet = puppet.with_physics(physics);
    }
    Some((
        puppet,
        LoadedModel {
            revision,
            canvas,
            textures,
            normal_maps,
        },
    ))
}

/// テクスチャと法線マップを番号順にデコードする (PNG と、圧縮していない KTX2。`ugoe_headless::image::decode_image*`)。
/// 無いテクスチャと読めない形式の KTX2 (ブロック圧縮など) は不透明な白、法線マップは無しで描く (wasm の受け手と同じ)。
/// 壊れた画像があれば、その番号で `None`。
fn decode_textures<'a>(
    textures: impl Iterator<Item = Option<&'a TextureData>>,
    normal_maps: impl Iterator<Item = Option<&'a TextureData>>,
) -> impl Iterator<Item = Option<(Rgba8, Option<Rgba8>)>> {
    let unsupported = |e: &ugoe_headless::Error| {
        matches!(
            e,
            ugoe_headless::Error::ImageDecode(ImageError::UnsupportedKtx2(_))
        )
    };
    textures
        .chain(std::iter::repeat(None))
        .zip(normal_maps)
        .map(move |(texture, normal_map)| {
            let texture = match texture.map(|d| decode_image_premultiplied(d.format, &d.bytes)) {
                None => placeholder_texture(),
                Some(Ok(image)) => image,
                Some(Err(e)) if unsupported(&e) => placeholder_texture(),
                Some(Err(_)) => return None,
            };
            let normal_map = match normal_map.map(|d| decode_image(d.format, &d.bytes)) {
                None => None,
                Some(Ok(image)) => Some(image),
                Some(Err(e)) if unsupported(&e) => None,
                Some(Err(_)) => return None,
            };
            Some((texture, normal_map))
        })
}

/// `.ugoemod` を読み、テクスチャと法線マップをデコードする。読めなければ `None`。
fn read_module(bytes: &[u8]) -> Option<LoadedModule> {
    let file = ugoe::format::read_module(bytes).ok()?;
    let count = file.module.textures.len();
    let (textures, normal_maps) = decode_textures(
        file.textures.iter().map(Some),
        (0..count).map(|i| file.normal_map(i)),
    )
    .take(count)
    .collect::<Option<(Vec<_>, Vec<_>)>>()?;
    Some(LoadedModule {
        module: Arc::new(file.module),
        textures,
        normal_maps,
    })
}

/// シーンのモジュール (合成の順。受け取れなかったものは `None`) を読んで `puppet` に付ける。読めない・付けられない
/// ものは飛ばして、残りを付ける (出力ウィンドウの受け手と同じ)。付けたモジュールと、飛ばしたものすべての番号と理由
/// (番号の順) を返す。
fn attach_modules(
    puppet: &mut Puppet,
    files: &[Option<Arc<Vec<u8>>>],
) -> (Vec<LoadedModule>, Vec<(usize, LoadFailureCode)>) {
    let mut skipped = Vec::new();
    let mut loaded = Vec::new();
    for (i, bytes) in files.iter().enumerate() {
        match bytes.as_deref().map(|b| read_module(b)) {
            None => skipped.push((i, LoadFailureCode::Unavailable)),
            Some(None) => skipped.push((i, LoadFailureCode::Unreadable)),
            Some(Some(module)) => loaded.push((i, module)),
        }
    }
    let modules = |loaded: &[(usize, LoadedModule)]| {
        loaded
            .iter()
            .map(|(_, m)| Arc::clone(&m.module))
            .collect::<Vec<_>>()
    };
    // ふつうはすべて付けられる。付けられなければ、1 つずつ足して付けられないもの (合成できない組) を見つける。
    if puppet.set_modules(modules(&loaded)).is_err() {
        let mut attached: Vec<(usize, LoadedModule)> = Vec::new();
        for (i, module) in loaded {
            let mut trial = modules(&attached);
            trial.push(Arc::clone(&module.module));
            if puppet.set_modules(trial).is_ok() {
                attached.push((i, module));
            } else {
                skipped.push((i, LoadFailureCode::Unreadable));
            }
        }
        loaded = attached;
        // 最後に足せなかったときは、付けられた組のまま (失敗した `set_modules` は何も変えない)。
        if loaded.is_empty() {
            let _ = puppet.set_modules(Vec::new());
        }
    }
    skipped.sort_unstable_by_key(|(i, _)| *i);
    (loaded.into_iter().map(|(_, m)| m).collect(), skipped)
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

/// 動いている間に、待っている読み戻しが来ない長さを見る (GPU が止まったか)。
#[derive(Debug, Default)]
struct Stall {
    /// 待っているのに読み戻しが来なくなった時刻。
    since: Option<Instant>,
}

impl Stall {
    /// 描いて読み戻しを見るたびに呼ぶ。`waiting` は読み戻しを待っているものがあるか、`received` はこの回に読み戻しが来たか。
    /// 待っているのに来ない長さが `limit` に達したら `true`。
    fn check(&mut self, now: Instant, waiting: bool, received: bool, limit: Duration) -> bool {
        if received || !waiting {
            self.since = None;
            return false;
        }
        now.duration_since(*self.since.get_or_insert(now)) >= limit
    }
}

/// 止めるときに、頼んだ読み戻しを待ち終える (`wait`)。時間切れならログに書いて GPU を手放し (次の開始で作り直す)、`true` を返す。
fn drain<T>(
    cell: &GpuCell<T>,
    gpu: &T,
    wait: impl FnOnce() -> Result<(), ugoe_headless::Error>,
) -> bool {
    match wait() {
        Ok(()) => false,
        Err(e) if e.is_gpu_wait_timeout() => {
            eprintln!("virtual camera: {e}; the GPU will be recreated on the next start");
            cell.discard(gpu);
            true
        }
        Err(e) => {
            eprintln!("virtual camera: waiting for the last readbacks failed: {e}");
            false
        }
    }
}

/// 知らせを 1 つ適用する。止めるなら `Ok(false)`。今のシーンのファイルを読めなければ、読めなかったものを `report` で知らせる。
fn apply(
    feed: &mut Feed,
    gpu: &Gpu,
    command: Command,
    report: impl FnOnce(Vec<LoadFailure>),
) -> Result<bool, FeedError> {
    match command {
        Command::Frame(frame) => feed.receive(&frame).map(|_| true),
        Command::Scene(files) => {
            let failures = feed.set_scene(gpu, &files)?;
            if !failures.is_empty() {
                report(failures);
            }
            Ok(true)
        }
        Command::Background(rgb) => Ok(feed.set_fill(gpu, rgb).map(|()| true)?),
        Command::Stop => Ok(false),
    }
}

/// 仮想カメラのスレッドの本体。`Command::Stop` かチャネルが閉じるまで回り、止まったら [`Status::Stopped`] を知らせる。
/// GPU は電力の希望 `power` で作る (`None` は希望なしで、環境変数 `WGPU_POWER_PREF` に従う)。
pub fn run(
    settings: Settings,
    power: Option<PowerPreference>,
    commands: Receiver<Command>,
    events: impl Fn(Event),
) {
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
    let gpu = match GPU.get(power, || {
        Gpu::with_power_preference(None, power).map_err(|e| e.to_string())
    }) {
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
    let mut stall = Stall::default();
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
            match apply(&mut feed, gpu, command, |f| events(Event::LoadFailures(f))) {
                Ok(true) => {}
                Ok(false) => break 'outer,
                Err(e) => {
                    if matches!(e, FeedError::Gpu(_)) {
                        // デバイスを失ったなら使い回せないので、次の開始で作り直す (手放すだけで破棄はしない)。
                        GPU.discard(gpu);
                    }
                    return failed(e.code(), e.to_string());
                }
            }
        }
        if Instant::now() < next {
            continue;
        }
        let started = Instant::now();
        next = (next + interval).max(started);
        // GPU のエラー (描画・読み戻し。検証エラー・デバイスの喪失など) は `gpuError`、OS の部品のエラーは `os`。
        let sent = feed
            .render(gpu)
            .and_then(|_| feed.poll(gpu, Wait::No))
            .map_err(|e| ("gpuError", e.to_string()))
            .and_then(|frame| match frame {
                Some(frame) => camera
                    .send(&frame)
                    .map(Some)
                    .map_err(|e| ("os", e.to_string())),
                None => Ok(None),
            });
        let received = matches!(sent, Ok(Some(_)));
        if stall.check(Instant::now(), feed.waiting(), received, STALL_LIMIT) {
            // 止まったかもしれない GPU で作ったものは使い回さず、破棄もしない (`Gpu::wait_timeout`)。
            GPU.discard(gpu);
            std::mem::forget(feed);
            return failed(
                "gpuTimeout",
                format!(
                    "no frame came back from the GPU for {STALL_LIMIT:?}; the device may be hung"
                ),
            );
        }
        match sent {
            Ok(Some(delivery)) => {
                consumer = delivery == Delivery::Delivered;
                meter.record(started.elapsed());
            }
            Ok(None) => {}
            Err((code, detail)) => {
                if code == "gpuError" {
                    GPU.discard(gpu);
                }
                return failed(code, detail);
            }
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
                backend: gpu.adapter_info.backend.to_str(),
                adapter: gpu.adapter_info.name.clone(),
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
    if drain(&GPU, gpu, || feed.poll(gpu, Wait::All).map(drop)) {
        std::mem::forget(feed);
    }
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

    /// 背景の画像の画素 (乗算済み) の `(x, y)` の RGBA。
    fn pixel(image: &Rgba8, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * image.width + x) * 4) as usize;
        [
            image.pixels[i],
            image.pixels[i + 1],
            image.pixels[i + 2],
            image.pixels[i + 3],
        ]
    }

    fn near(a: [u8; 4], b: [u8; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 24)
    }

    fn webp(pixels: &[u8], width: u32, height: u32, color: image_webp::ColorType) -> Vec<u8> {
        let mut out = Vec::new();
        image_webp::WebPEncoder::new(&mut out)
            .encode(pixels, width, height, color)
            .unwrap();
        out
    }

    #[test]
    fn decodes_png_jpeg_and_webp_backgrounds() {
        // JPEG (左が赤・右が青の 16x8。Chrome で書いたもの)。
        let jpeg = decode_background(include_bytes!("../tests/fixtures/red-blue.jpg")).unwrap();
        assert_eq!((jpeg.width, jpeg.height), (16, 8));
        assert!(
            near(pixel(&jpeg, 2, 4), [255, 0, 0, 255]),
            "{:?}",
            pixel(&jpeg, 2, 4)
        );
        assert!(near(pixel(&jpeg, 13, 4), [0, 0, 255, 255]));
        // WebP (透過あり・なし)。透過ありは乗算済みにする。
        let rgba = [200, 100, 50, 128, 10, 20, 30, 255];
        let alpha = decode_background(&webp(&rgba, 2, 1, image_webp::ColorType::Rgba8)).unwrap();
        assert_eq!(alpha.pixels, vec![100, 50, 25, 128, 10, 20, 30, 255]);
        let rgb = decode_background(&webp(
            &[1, 2, 3, 4, 5, 6],
            2,
            1,
            image_webp::ColorType::Rgb8,
        ))
        .unwrap();
        assert_eq!(rgb.pixels, vec![1, 2, 3, 255, 4, 5, 6, 255]);
        // PNG。
        let png = ugoe_headless::image::encode_png_unpremultiplied(&Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![9, 8, 7, 255],
        })
        .unwrap();
        assert_eq!(decode_background(&png).unwrap().pixels, vec![9, 8, 7, 255]);
        assert_eq!(decode_background(b"GIF89a"), Err(BackgroundError::Format));
        assert!(matches!(
            decode_background(&[0xff, 0xd8, 0xff, 0]),
            Err(BackgroundError::Decode(_))
        ));
    }

    #[test]
    fn huge_backgrounds_are_refused_before_decoding() {
        let side = MAX_BACKGROUND_SIDE + 1;
        // PNG は IHDR の大きさだけを見て断る (本体が無くても)。
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend_from_slice(&side.to_be_bytes());
        png.extend_from_slice(&1u32.to_be_bytes());
        assert_eq!(
            decode_background(&png),
            Err(BackgroundError::TooLarge {
                width: side,
                height: 1
            })
        );
        let wide = webp(
            &vec![0; side as usize * 3],
            side,
            1,
            image_webp::ColorType::Rgb8,
        );
        assert!(matches!(
            decode_background(&wide),
            Err(BackgroundError::TooLarge { .. })
        ));
        // 一辺は収まっても、画素の数で断る。
        assert!(check_background_size(MAX_BACKGROUND_SIDE, MAX_BACKGROUND_SIDE).is_err());
        assert!(check_background_size(1920, 1080).is_ok());
        assert!(check_background_size(0, 1).is_err());
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

    #[test]
    fn a_hung_gpu_is_recreated_on_the_next_start() {
        let cell = GpuCell::<u32>::new();
        let first = cell.get(None, || Ok(1)).unwrap();
        // 待ち終えたら手放さない。
        assert!(!drain(&cell, first, || Ok(())));
        assert!(std::ptr::eq(cell.get(None, || Ok(2)).unwrap(), first));
        // 時間切れでないエラーでも手放さない。
        let other = ugoe_headless::Error::Gpu {
            context: "mapping the readback buffer",
            detail: String::new(),
        };
        assert!(!drain(&cell, first, || Err(other)));
        assert_eq!(*cell.get(None, || Ok(2)).unwrap(), 1);
        // 時間切れなら手放し、次は作り直す。
        assert!(drain(&cell, first, || Err(
            ugoe_headless::Error::gpu_wait_timeout(Duration::ZERO)
        )));
        let second = cell.get(None, || Ok(2)).unwrap();
        assert_eq!(*second, 2);
        // 前の GPU を手放しても、作り直したものはそのまま。
        cell.discard(first);
        assert!(std::ptr::eq(cell.get(None, || Ok(3)).unwrap(), second));
    }

    #[test]
    fn a_gpu_is_recreated_only_when_the_power_preference_changes() {
        let cell = GpuCell::<u32>::new();
        let high = Some(PowerPreference::HighPerformance);
        let first = cell.get(None, || Ok(1)).unwrap();
        // 同じ希望なら作り直さない
        assert!(std::ptr::eq(cell.get(None, || Ok(2)).unwrap(), first));
        // 違う希望なら作り直す (前のものは手放すだけ)
        let second = cell.get(high, || Ok(2)).unwrap();
        assert_eq!(*second, 2);
        assert!(std::ptr::eq(cell.get(high, || Ok(3)).unwrap(), second));
        // 前の GPU を手放しても、今のものはそのまま
        cell.discard(first);
        assert!(std::ptr::eq(cell.get(high, || Ok(3)).unwrap(), second));
        // 作れなかったことは同じ希望なら覚え、違う希望なら試し直す
        let low = Some(PowerPreference::LowPower);
        assert_eq!(
            cell.get(low, || Err("no adapter".into())),
            Err("no adapter".into())
        );
        assert_eq!(cell.get(low, || Ok(4)), Err("no adapter".into()));
        assert_eq!(cell.get(None, || Ok(5)).copied(), Ok(5));
    }

    #[test]
    fn a_failed_gpu_is_remembered() {
        let cell = GpuCell::<u32>::new();
        assert_eq!(
            cell.get(None, || Err("no adapter".into())),
            Err("no adapter".into())
        );
        assert_eq!(cell.get(None, || Ok(1)), Err("no adapter".into()));
    }

    #[test]
    fn a_stall_is_readbacks_not_coming_back_for_the_limit() {
        let limit = Duration::from_secs(10);
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_secs(s);
        let mut stall = Stall::default();
        // 待っているものが無ければ、来なくても止まっていない。
        assert!(!stall.check(at(0), false, false, limit));
        assert!(!stall.check(at(20), false, false, limit));
        // 待っているのに来ない。上限の手前までは止まっていない。
        assert!(!stall.check(at(21), true, false, limit));
        assert!(!stall.check(at(30), true, false, limit));
        // 来たら数え直す (遅いだけの GPU)。
        assert!(!stall.check(at(30), true, true, limit));
        assert!(!stall.check(at(31), true, false, limit));
        assert!(!stall.check(at(40), true, false, limit));
        assert!(stall.check(at(41), true, false, limit));
    }

    /// 不透明な画 (乗算済みとストレートが同じなので、PNG と KTX2 の書き方の違いを気にしなくてよい)。
    fn rgba() -> Rgba8 {
        Rgba8 {
            width: 2,
            height: 1,
            pixels: vec![200, 100, 50, 255, 10, 20, 30, 255],
        }
    }

    fn png_texture() -> TextureData {
        TextureData {
            format: ugoe::format::TextureFormat::Png,
            bytes: image::encode_png_unpremultiplied(&rgba()).unwrap(),
        }
    }

    fn ktx2_texture() -> TextureData {
        TextureData {
            format: ugoe::format::TextureFormat::Ktx2,
            bytes: image::encode_ktx2(&[rgba()], image::Ktx2Options::default()).unwrap(),
        }
    }

    /// 読めない形式の KTX2 (`vkFormat` を ETC2 (151) に書き換えたもの。BC1〜BC5・BC7 は読める。ADR 0030)。
    fn etc2_texture() -> TextureData {
        let mut texture = ktx2_texture();
        texture.bytes[12..16].copy_from_slice(&151u32.to_le_bytes());
        texture
    }

    fn broken_texture() -> TextureData {
        TextureData {
            format: ugoe::format::TextureFormat::Png,
            bytes: b"\x89PNG\r\n\x1a\nbroken".to_vec(),
        }
    }

    fn module_bytes(module: ugoe::model::module::Module, texture: TextureData) -> Arc<Vec<u8>> {
        let count = module.textures.len();
        Arc::new(
            ugoe::format::write_module(&ugoe::format::ModuleFile {
                module,
                textures: vec![texture; count],
                normal_maps: Vec::new(),
                meta: None,
            })
            .unwrap(),
        )
    }

    fn model_bytes(texture: TextureData) -> Vec<u8> {
        let mut bundle = ugoe::format::Bundle::new(ugoe::model::fixtures::dress_up_base());
        bundle.textures = vec![texture; texture_count(&bundle.model)];
        ugoe::format::write_bundle(&bundle).unwrap()
    }

    /// モデルとモジュールのテクスチャは PNG と KTX2 (圧縮していないものと BC1〜BC5・BC7) を読む (T0836)。読めない形式の KTX2 は白で描き、
    /// 壊れた画像のあるモデル・モジュールは読めない。
    #[test]
    fn models_and_modules_read_png_and_ktx2_textures() {
        let premultiplied = decode_png_premultiplied(&png_texture().bytes).unwrap();
        assert_eq!(premultiplied, rgba());

        let (_, model) = read_model(&model_bytes(ktx2_texture()), 3).unwrap();
        assert_eq!(model.revision, 3);
        assert!(!model.textures.is_empty());
        assert!(model.textures.iter().all(|t| *t == premultiplied));
        let (_, model) = read_model(&model_bytes(etc2_texture()), 3).unwrap();
        assert!(model.textures.iter().all(|t| *t == placeholder_texture()));
        assert!(read_model(&model_bytes(broken_texture()), 3).is_none());
        assert!(read_model(b"not a model", 3).is_none());

        let hair = ugoe::model::fixtures::long_hair_module;
        let module = read_module(&module_bytes(hair(), ktx2_texture())).unwrap();
        assert!(!module.textures.is_empty());
        assert!(module.textures.iter().all(|t| *t == premultiplied));
        let module = read_module(&module_bytes(hair(), png_texture())).unwrap();
        assert!(module.textures.iter().all(|t| *t == premultiplied));
        let module = read_module(&module_bytes(hair(), etc2_texture())).unwrap();
        assert!(module.textures.iter().all(|t| *t == placeholder_texture()));
        assert!(read_module(&module_bytes(hair(), broken_texture())).is_none());
    }

    /// 受け取れない・読めない・付けられないモジュールは飛ばして、残りを付ける。飛ばしたものはすべて番号の順に知らせる
    /// (T0836・T0840)。
    #[test]
    fn unreadable_modules_are_skipped_and_all_are_reported() {
        use ugoe::model::fixtures;
        let hair = module_bytes(fixtures::long_hair_module(), png_texture());
        let dress = module_bytes(fixtures::dress_module(), png_texture());
        let garbage = Arc::new(b"not a module".to_vec());
        let base = || Puppet::new(fixtures::dress_up_base()).unwrap();
        let names = |puppet: &Puppet| {
            puppet
                .modules()
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<_>>()
        };

        let mut puppet = base();
        let (attached, failure) = attach_modules(
            &mut puppet,
            &[
                Some(Arc::clone(&hair)),
                Some(Arc::clone(&garbage)),
                None,
                Some(Arc::clone(&dress)),
            ],
        );
        assert_eq!(
            failure,
            [
                (1, LoadFailureCode::Unreadable),
                (2, LoadFailureCode::Unavailable)
            ]
        );
        assert_eq!(attached.len(), 2);
        assert_eq!(names(&puppet).len(), 2);

        let mut puppet = base();
        let (attached, failure) = attach_modules(&mut puppet, &[None, Some(Arc::clone(&garbage))]);
        assert_eq!(
            failure,
            [
                (0, LoadFailureCode::Unavailable),
                (1, LoadFailureCode::Unreadable)
            ]
        );
        assert!(attached.is_empty());

        // 本体にスロットが無いモジュールは付けられない (読めないものとして知らせる)。前に付けていた組は外す。
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let (attached, failure) = attach_modules(&mut puppet, &[Some(Arc::clone(&hair))]);
        assert_eq!(failure, [(0, LoadFailureCode::Unreadable)]);
        assert!(attached.is_empty());
        // 付けられないものは後から見つかるが、番号の順に並べる。
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let (_, failure) = attach_modules(&mut puppet, &[Some(Arc::clone(&hair)), None]);
        assert_eq!(
            failure,
            [
                (0, LoadFailureCode::Unreadable),
                (1, LoadFailureCode::Unavailable)
            ]
        );
        let mut puppet = base();
        attach_modules(&mut puppet, &[Some(Arc::clone(&hair))]);
        assert_eq!(names(&puppet).len(), 1);
        let (_, failure) = attach_modules(&mut puppet, &[Some(Arc::clone(&garbage))]);
        assert_eq!(failure, [(0, LoadFailureCode::Unreadable)]);
        assert!(names(&puppet).is_empty());
    }

    /// 背景の画像を受け取れない・読めないことを返す。読めなかった画像は次のシーンの通知で読み直す (T0836)。
    #[test]
    fn unreadable_backgrounds_are_reported_until_they_are_read() {
        let mut feed = Feed::new(&Settings::default(), PixelFormat::Rgba8);
        feed.look.background = Background::Image {
            path: PathBuf::from("/bg.png"),
        };
        let garbage = b"not an image".to_vec();
        let png = image::encode_png_unpremultiplied(&rgba()).unwrap();
        assert_eq!(feed.read_image(None), Some(LoadFailureCode::Unavailable));
        assert_eq!(
            feed.read_image(Some(&garbage)),
            Some(LoadFailureCode::Unreadable)
        );
        assert_eq!(
            feed.read_image(Some(&garbage)),
            Some(LoadFailureCode::Unreadable)
        );
        assert_eq!(feed.read_image(Some(&png)), None);
        // 読めた画像は、同じパスなら読み直さない。
        assert_eq!(feed.read_image(None), None);
        assert!(matches!(&feed.image, Some((_, Some(_)))));
        feed.look.background = Background::Transparent;
        assert_eq!(feed.read_image(None), None);
        assert!(feed.image.is_none());
    }

    /// 描画中の GPU のエラーは `os` でなく `gpuError` (止まったときは `gpuTimeout`) で止まる (T0836)。
    #[test]
    fn gpu_errors_have_their_own_codes() {
        let gpu = FeedError::from(ugoe_headless::Error::Gpu {
            context: "submitting",
            detail: "device lost".into(),
        });
        assert_eq!(gpu.code(), "gpuError");
        let timeout = FeedError::from(ugoe_headless::Error::gpu_wait_timeout(Duration::ZERO));
        assert_eq!(timeout.code(), "gpuTimeout");
        assert_eq!(FeedError::Model("values".into()).code(), "model");
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
        run(Settings::default(), None, rx, |e| {
            events.lock().unwrap().push(e)
        });
        let events = events.into_inner().unwrap();
        assert!(matches!(events[0], Event::Status(Status::Starting { .. })));
        assert!(
            matches!(&events[1], Event::Status(Status::Failed { code, .. }) if !code.is_empty()),
            "{events:?}"
        );
    }
}
