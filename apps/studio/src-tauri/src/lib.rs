//! スタジオのバックエンド (`apps/studio/README.md`、ADR 0028)。
//!
//! - 指揮役 ([`conductor::Conductor`]) を刻みのスレッドで回し、刻みごとのフレームを出力ウィンドウへ Tauri の `Channel` で配る。
//! - 操作ウィンドウへは、シーン (パラメータの定義) と、間引いた値と負荷 (毎秒 10 回) を配る。
//! - 出力ウィンドウ (枠なし・背景の透過) の表示・最前面・大きさを操作する。
//! - 刻みの速さ (30 / 60 / 120) を変え、実際の速さと、出力ウィンドウから届く描画の負荷を操作ウィンドウに配る (T0804)。
//! - プロファイル (シーンの一覧) を読み書きし、シーンを切り替える ([`stage::Stage`])。
//! - 入力 (T0803): 操作ウィンドウが取ったトラッキングとマイクの音量を指揮役に入れ、モーション・表情を操作する。
//!   グローバルなホットキーを登録し、押されたら操作を行う。操作ウィンドウの入力の処理を刻みで動かす (`subscribe_pulse`)。
//! - 仮想カメラ ([`virtual_camera`]、T0807) を始めると、同じプロセスのチャネルでシーンとフレームを渡す。仮想カメラの受け手は
//!   ネイティブの受け手と `ugoe-headless` の常駐の描画で描き、`ugoe-studio-vcam` で OS に渡す。
//! - 操作 API (T0805): ループバックの WebSocket で外のツールから操作させる ([`api`])。
//!
//! - ブラウザソース (T0808): 操作 API と同じ待ち受けで、OBS のブラウザソース用のページ・ファイル・フレームを出す
//!   ([`api::source`])。シーンの通知とフレームを、出力ウィンドウと同じ順にページの WebSocket へ配る。
//!
//! 出力ウィンドウとブラウザソースの描画はしない (出力ウィンドウの WebView の wasm が受け手として描く)。

mod access;
// コマンドの許可の表 (build.rs が読む)。ここではその検査だけを行う (T0820)。
#[cfg(test)]
mod acl;
pub mod animation;
pub mod api;
pub mod conductor;
mod error;
pub mod follower;
pub mod hotkeys;
pub mod input;
pub mod lighting;
pub mod load_failure;
pub mod pacing;
pub mod profile;
pub mod stage;
pub mod stream;
pub mod virtual_camera;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::{AppHandle, Manager, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::animation::{AnimationError, AnimationView};
use crate::conductor::Conductor;
use crate::error::CommandError;
use crate::hotkeys::{ExpressionMode, Hotkey, HotkeyAction, HotkeyProblem, MotionPriority};
use crate::input::{InputSettings, MotionSettings, TrackingFrame};
use crate::pacing::{Measured, Pacer, RateMeter, TickRate};
use crate::profile::{Profile, SceneSettings, VirtualCameraSettings};
use crate::stage::{Finish, Loading, Pending, ProfileView, SceneError, Stage};
use crate::stream::{Frame, Scene};
use crate::virtual_camera::{
    Command as CameraCommand, Event as CameraEvent, SceneFiles, Settings as CameraSettings,
};

/// 操作ウィンドウのラベル (tauri.conf.json)。
const CONTROL: &str = "main";
/// 出力ウィンドウのラベル (tauri.conf.json)。
const OUTPUT: &str = "output";
/// 出力ウィンドウからの描画の負荷の報告が、これより古ければ「止まっている」とみなす (隠れて描いていないなど)。
const RENDER_STATS_STALE: Duration = Duration::from_secs(2);
/// 最近のプロファイルを覚えるファイル (アプリのデータの場所)。
const RECENT_PROFILES_FILE: &str = "recent-profiles.json";

/// 毒された `Mutex` も使い続ける (刻みのスレッドが止まらないように)。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 操作ウィンドウに出すパラメータの定義。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterInfo {
    id: String,
    name: String,
    min: f32,
    max: f32,
    default: f32,
    /// 中継用のパラメータ (一覧では既定で隠す)。
    internal: bool,
}

/// 操作ウィンドウへの通知 (JSON。`kind` で分ける)。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum Monitor<'a> {
    /// シーンが変わった (接続したときも送る)。
    #[serde(rename = "scene", rename_all = "camelCase")]
    Scene {
        scene: &'a Scene,
        parameters: Vec<ParameterInfo>,
        /// モデル (と付けているモジュール) にライティングを受けるメッシュがあるか。
        uses_lighting: bool,
    },
    /// 間引いた今の値と負荷。
    #[serde(rename = "status", rename_all = "camelCase")]
    Status {
        seq: u64,
        values: &'a [f32],
        /// 直近の 1 刻みにかかった時間 (ミリ秒)。
        tick_ms: f32,
        /// フレームを受けている出力の数。
        outputs: usize,
        /// 設定した刻みの速さ (回/秒)。
        tick_rate: u32,
        /// 実際の刻みの速さ (回/秒。測り終えるまでは無し)。
        measured_rate: Option<f32>,
        /// 直近 1 秒の刻みの間隔の最大 (ミリ秒)。
        max_interval_ms: Option<f32>,
        /// 出力ごとの描画の負荷。
        renderers: Vec<RendererLoad>,
    },
    /// 出力ウィンドウの状態が変わった。
    #[serde(rename = "output")]
    Output(OutputState),
    /// プロファイル (シーンの一覧・保存先・出しているシーン・最近のプロファイル) が変わった (接続したときも送る)。
    #[serde(rename = "profile")]
    Profile(ProfileView<'a>),
    /// モーション・表情・入力の様子が変わった (接続したときも送る。T0803)。
    #[serde(rename = "animation")]
    Animation(AnimationView),
    /// ホットキーを登録し直した: 登録できなかった割り当てと理由 (接続したときも送る。T0803)。
    #[serde(rename = "hotkeys", rename_all = "camelCase")]
    Hotkeys {
        problems: &'a [(usize, HotkeyProblem)],
        /// 割り当ての画面でキーを押している間、登録を止めている。
        suspended: bool,
        /// Wayland のセッション (アプリが前面に無いとホットキーを受けられない。ADR 0028 の 9.)。
        wayland: bool,
    },
    /// ホットキーが押された (キャリブレーションは操作ウィンドウが行う)。行えなければ `error`。
    #[serde(rename = "hotkey")]
    Hotkey {
        action: &'a HotkeyAction,
        error: Option<CommandError>,
    },
    /// 仮想カメラの状態が変わった (T0807)。
    #[serde(rename = "virtualCamera")]
    VirtualCamera(&'a virtual_camera::Status),
    /// 操作 API の様子 (待ち受け・承認したクライアント・承認の待ち。接続したときも送る。T0805)。
    #[serde(rename = "api")]
    Api(api::ApiView),
    /// 受け手が今のシーンのファイルを読めなかった (受け手ごと。無ければ空。接続したときも送る。T0825)。
    #[serde(rename = "loadFailures")]
    LoadFailures {
        failures: Vec<load_failure::ReceiverFailure>,
    },
    /// 舞台のロックの外で読んでいる切り替えのうち最後に始めたもののシーンの ID。終われば `None` (`Stage::loading`。
    /// ホットキー・外部 API が始めたものも含む。変わったときと接続したときに送る。T0841)。
    #[serde(rename = "sceneLoading")]
    SceneLoading { scene: Option<&'a str> },
}

/// ファイルを読む・スレッドを待つ処理を、メインのスレッド (イベントループ) と非同期の実行器のワーカーの外で行う
/// (配信中に両方のウィンドウのイベントとフレームを止めない。docs/studio.md#隠れても止まらない)。
/// ロックは `action` の中で取って放す (`.await` をまたいで握らない)。
async fn blocking<T: Send + 'static>(
    studio: &Arc<Studio>,
    action: impl FnOnce(&Arc<Studio>) -> T + Send + 'static,
) -> Result<T, tauri::Error> {
    let studio = Arc::clone(studio);
    tauri::async_runtime::spawn_blocking(move || action(&studio)).await
}

/// 出力ウィンドウの状態。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputState {
    visible: bool,
    on_top: bool,
    /// 中身の大きさ (物理ピクセル。配信ソフトが取り込む大きさ)。
    width: u32,
    height: u32,
}

/// 出力ウィンドウが報告する描画の負荷 (直近の約 1 秒)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenderStats {
    /// 描いた回数 (回/秒)。
    fps: f32,
    /// 1 回の描画 (届いたフレームの適用と描画) の CPU の時間の平均 (ミリ秒)。
    render_ms: f32,
    /// 1 回の描画の CPU の時間の最大 (ミリ秒)。
    render_max_ms: f32,
    /// 溢れて捨てたフレームの数 (受け手を作ってからの合計)。
    dropped: u64,
}

impl RenderStats {
    fn is_valid(&self) -> bool {
        [self.fps, self.render_ms, self.render_max_ms]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
    }
}

/// 操作ウィンドウに出す、出力ごとの描画の負荷。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RendererLoad {
    /// 出力のラベル (`output` は出力ウィンドウ)。
    label: String,
    #[serde(flatten)]
    stats: RenderStats,
    /// 報告が途絶えている (隠れていて描いていない・閉じた)。
    stale: bool,
    /// この出力の目標の描く回数 (回/秒)。無ければ刻みの速さ (出力ウィンドウ)。仮想カメラはそのフレームレート。
    target_fps: Option<u32>,
}

/// フレームや通知を受ける WebView (ラベル → Channel)。開き直したページは前の Channel を置き換える。
#[derive(Default)]
struct Subscribers(Mutex<HashMap<String, Channel<InvokeResponseBody>>>);

impl std::fmt::Debug for Subscribers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(lock(&self.0).keys()).finish()
    }
}

impl Subscribers {
    fn insert(&self, label: &str, channel: Channel<InvokeResponseBody>) {
        lock(&self.0).insert(label.to_owned(), channel);
    }

    fn len(&self) -> usize {
        lock(&self.0).len()
    }

    /// `body` をすべてへ送る。送れなかった (閉じた) ものは外す。
    fn send_all(&self, body: &InvokeResponseBody) {
        lock(&self.0).retain(|_, channel| channel.send(body.clone()).is_ok());
    }
}

/// 刻みのスレッドとコマンドが共有するもの。
#[derive(Debug, Default)]
struct Studio {
    /// プロファイルと今のシーンのファイル。`conductor` と両方を握るときは、こちらを先に握る。
    stage: Mutex<Stage>,
    conductor: Mutex<Conductor>,
    /// 起動して最初に、最後のプロファイルを開いたか (`restore_profile`)。
    restored: std::sync::atomic::AtomicBool,
    /// フレームを受ける出力。
    outputs: Subscribers,
    /// 値と負荷を受ける操作ウィンドウ。
    monitors: Subscribers,
    /// 直近の 1 刻みにかかった時間 (ナノ秒)。
    tick_nanos: AtomicU64,
    /// 刻みの速さ (回/秒。0 は既定)。刻みのスレッドが刻みごとに読む。プロファイルの `output.tickRate` の写し (`with_stage` で合わせる)。
    tick_rate: AtomicU32,
    /// 実際の刻みの速さと揺れ (刻みのスレッドが書く)。
    measured: Mutex<Option<Measured>>,
    /// 出力ごとの描画の負荷 (ラベル → 報告)。
    renders: Mutex<HashMap<String, Reported>>,
    /// 受け手が今のシーンで読めなかったこと (ラベル → 報告。T0825)。ほかのロックを握ったまま取ってよい (最後に取る)。
    load_failures: Mutex<load_failure::LoadFailures>,
    /// 刻みごとに入力の処理を促す操作ウィンドウ (`subscribe_pulse`。T0803)。
    pulses: Subscribers,
    /// グローバルなホットキー (T0803)。
    hotkeys: Mutex<Hotkeys>,
    /// ホットキーの登録に使う (`setup` で入れる。テストでは無い)。
    app: OnceLock<AppHandle>,
    /// 動いている仮想カメラ (止めている間は `None`。スレッドも受け手も作らない)。
    camera: Mutex<Option<CameraThread>>,
    /// 仮想カメラの最新の状態。
    camera_status: Mutex<Option<virtual_camera::Status>>,
    /// 操作 API (T0805)。
    api: api::Api,
    /// ブラウザソースのページの接続 (T0808。待ち受けは操作 API と共有する)。
    sources: api::source::Sources,
    /// `dressup.list` が読んだ付けていないモジュールの概要 (T0833)。ほかのロックを握らずに取る。
    module_summaries: Mutex<api::ModuleSummaries>,
    /// webview に読み書きを許したパス (T0838)。舞台のロックを握ったまま取ってよい (ほかのロックを握らない)。
    files: access::FileAccess,
}

/// 登録しているホットキー。
#[derive(Debug, Default)]
struct Hotkeys {
    /// 登録のもとにした割り当て (プロファイルの値。同じなら登録し直さない)。
    source: Option<Vec<Hotkey>>,
    /// 登録したキーの ID → 操作。
    bindings: HashMap<u32, HotkeyAction>,
    /// 登録できなかった割り当ての位置と理由。
    problems: Vec<(usize, HotkeyProblem)>,
    /// 割り当ての画面でキーを押している間は登録を外す。
    suspended: bool,
}

/// Wayland のセッションか (グローバルなホットキーは X11 の窓が前面にあるときだけ届く)。
fn wayland_session() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland")))
}

/// 出力から届いた描画の負荷。
#[derive(Debug, Clone, Copy)]
struct Reported {
    stats: RenderStats,
    /// 目標の描く回数 (回/秒)。`None` は刻みの速さ。
    target_fps: Option<u32>,
    /// 報告を受けた時刻。
    at: Instant,
}

/// 動いている仮想カメラのスレッド。
#[derive(Debug)]
struct CameraThread {
    commands: Sender<CameraCommand>,
    thread: JoinHandle<()>,
    /// 今の設定 (プロファイルの設定と比べて、変わったら効かせる。`sync_camera`)。
    settings: CameraSettings,
}

impl Studio {
    /// webview から受けたパスを読んでよいか (`access::FileAccess::check_readable`)。
    fn check_readable(&self, path: &Path) -> Result<(), CommandError> {
        self.files.check_readable(path, &lock(&self.stage))
    }

    fn json(value: &impl Serialize) -> Option<InvokeResponseBody> {
        serde_json::to_string(value)
            .ok()
            .map(InvokeResponseBody::Json)
    }

    fn scene_message(conductor: &Conductor) -> Option<InvokeResponseBody> {
        conductor.scene().and_then(Self::json)
    }

    fn monitor_scene_message(conductor: &Conductor) -> Option<InvokeResponseBody> {
        let scene = conductor.scene()?;
        let parameters = conductor
            .parameters()
            .iter()
            .map(|p| ParameterInfo {
                id: p.id.to_string(),
                name: p.name.clone(),
                min: p.min,
                max: p.max,
                default: p.default,
                internal: p.internal,
            })
            .collect();
        Self::json(&Monitor::Scene {
            scene,
            parameters,
            uses_lighting: conductor.uses_lighting(),
        })
    }

    fn tick_rate(&self) -> TickRate {
        TickRate::from_hz(self.tick_rate.load(Ordering::Relaxed)).unwrap_or(TickRate::DEFAULT)
    }

    /// 出力ごとの描画の負荷。報告の古いものは `stale` にする。
    fn renderer_loads(&self, now: Instant) -> Vec<RendererLoad> {
        let mut loads: Vec<_> = lock(&self.renders)
            .iter()
            .map(|(label, reported)| RendererLoad {
                label: label.clone(),
                stats: reported.stats,
                stale: now.saturating_duration_since(reported.at) > RENDER_STATS_STALE,
                target_fps: reported.target_fps,
            })
            .collect();
        loads.sort_by(|a, b| a.label.cmp(&b.label));
        loads
    }

    /// 1 刻み進めてフレームを配る。`status` なら操作ウィンドウにも値を送る。
    fn tick(&self, dt: f32, status: bool) {
        let started = Instant::now();
        let mut conductor = lock(&self.conductor);
        let frame = conductor.tick(dt);
        let view = conductor
            .take_animation_changed()
            .then(|| conductor.animation_view());
        let fired = conductor.take_motion_events();
        if view.is_some() || !fired.is_empty() {
            self.api_animation(view.as_ref(), &fired);
        }
        if let Some(view) = view
            && let Some(body) = Self::json(&Monitor::Animation(view))
        {
            self.monitors.send_all(&body);
        }
        let Some(frame) = frame else {
            return;
        };
        let bytes = frame.encode();
        self.source_frame(bytes.clone());
        self.outputs.send_all(&InvokeResponseBody::Raw(bytes));
        if let Some(camera) = &*lock(&self.camera) {
            let _ = camera.commands.send(CameraCommand::Frame(frame.clone()));
        }
        drop(conductor);
        let nanos = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.tick_nanos.store(nanos, Ordering::Relaxed);
        if status {
            self.send_status(&frame);
        }
    }

    fn send_status(&self, frame: &Frame) {
        let outputs = self.outputs.len();
        let measured = *lock(&self.measured);
        let message = Monitor::Status {
            seq: frame.seq,
            values: &frame.values,
            tick_ms: self.tick_nanos.load(Ordering::Relaxed) as f32 / 1e6,
            outputs,
            tick_rate: self.tick_rate().hz(),
            measured_rate: measured.map(|m| m.rate),
            max_interval_ms: measured.map(|m| m.max_interval_ms),
            renderers: self.renderer_loads(Instant::now()),
        };
        if let Some(body) = Self::json(&message) {
            self.monitors.send_all(&body);
        }
    }

    /// シーンが変わった: 受け手と操作ウィンドウに配る。次の刻みのフレームより先に届くよう、指揮役を握ったまま呼ぶ。
    fn send_scene(&self, stage: &Stage, conductor: &Conductor) {
        self.scene_changed_load_failures(conductor.scene().map(|s| s.revision));
        if let Some(body) = Self::scene_message(conductor) {
            if let InvokeResponseBody::Json(json) = &body {
                self.source_scene(json);
            }
            self.outputs.send_all(&body);
        }
        if let Some(camera) = &*lock(&self.camera)
            && let Some(files) = Self::camera_scene(stage, conductor)
        {
            let _ = camera.commands.send(CameraCommand::Scene(Box::new(files)));
        }
        if let Some(body) = Self::monitor_scene_message(conductor) {
            self.monitors.send_all(&body);
        }
    }

    fn scene_loading_message(stage: &Stage) -> Option<InvokeResponseBody> {
        Self::json(&Monitor::SceneLoading {
            scene: stage.loading(),
        })
    }

    fn send_scene_loading(&self, stage: &Stage) {
        if let Some(body) = Self::scene_loading_message(stage) {
            self.monitors.send_all(&body);
        }
    }

    fn send_profile(&self, stage: &Stage) {
        if let Some(body) = Self::json(&Monitor::Profile(stage.view())) {
            self.monitors.send_all(&body);
        }
    }

    /// 舞台と指揮役を握って `action` を行い、プロファイルを配る。シーンが変わっていれば、それも配る。
    fn with_stage<T>(&self, action: impl FnOnce(&mut Stage, &mut Conductor) -> T) -> T {
        let mut stage = lock(&self.stage);
        let mut conductor = lock(&self.conductor);
        let before = conductor.scene().map(|s| s.revision);
        let loading = stage.loading().map(str::to_owned);
        let snapshot = api::StageSnapshot::of(&stage);
        let result = action(&mut stage, &mut conductor);
        if conductor.scene().map(|s| s.revision) != before {
            self.send_scene(&stage, &conductor);
        }
        drop(conductor);
        self.api_stage_changed(&snapshot, &api::StageSnapshot::of(&stage));
        self.send_profile(&stage);
        if stage.loading() != loading.as_deref() {
            self.send_scene_loading(&stage);
        }
        self.sync_hotkeys(&stage.profile().hotkeys, false);
        self.tick_rate
            .store(stage.profile().output.tick_rate().hz(), Ordering::Relaxed);
        result
    }

    /// シーンを切り替える (T0834。docs/studio.md#シーンの切り替えと舞台のロック): 舞台を握って `begin` で始め、ファイルを読んでパペットを
    /// 組み立てる (T0837) のは舞台のロックの外で行い、握り直して差し込む (その間もほかの操作と外部 API の要求に答える)。
    /// 読むファイルが無ければ握ったまま差し込む。
    /// 読んでいる間に後から始めた切り替えがあれば、読んだものは捨てて `Ok` を返す (差し込むのは最後の切り替えだけ)。
    fn switch_scene(
        &self,
        begin: impl FnOnce(&mut Stage, &mut Conductor) -> Result<Option<Pending>, SceneError>,
    ) -> Result<(), SceneError> {
        self.switch_scene_with(begin, stage::LOADING)
    }

    /// [`Studio::switch_scene`] を、`loading` で読んで組み立てて行う。
    fn switch_scene_with(
        &self,
        begin: impl FnOnce(&mut Stage, &mut Conductor) -> Result<Option<Pending>, SceneError>,
        loading: Loading<'_>,
    ) -> Result<(), SceneError> {
        let pending = self.with_stage(|stage, conductor| {
            let Some(pending) = begin(stage, conductor)? else {
                return Ok(None);
            };
            if pending.needs_reading() {
                return Ok(Some(pending));
            }
            // 見せ方だけの編集など: 握ったまま差し込む (通知を 1 回にまとめる)。
            stage.settle(pending, conductor, loading).map(|()| None)
        })?;
        match pending {
            Some(pending) => self.present_with(pending, loading),
            None => Ok(()),
        }
    }

    /// 始めた切り替えを、舞台のロックの外で読んで組み立て、差し込む。
    fn present_with(&self, mut pending: Pending, loading: Loading<'_>) -> Result<(), SceneError> {
        loop {
            let loaded = pending.load_with(loading);
            match self.with_stage(|stage, conductor| stage.finish(loaded, conductor))? {
                Finish::Again(next) => pending = *next,
                Finish::Done | Finish::Superseded => return Ok(()),
            }
        }
    }

    fn hotkeys_message(hotkeys: &Hotkeys) -> Option<InvokeResponseBody> {
        Self::json(&Monitor::Hotkeys {
            problems: &hotkeys.problems,
            suspended: hotkeys.suspended,
            wayland: wayland_session(),
        })
    }

    /// プロファイルの割り当てが変わっていればホットキーを登録し直し、結果を操作ウィンドウに配る。`force` なら変わらなくても登録し直す。
    fn sync_hotkeys(&self, list: &[Hotkey], force: bool) {
        let mut hotkeys = lock(&self.hotkeys);
        if !force && hotkeys.source.as_deref() == Some(list) {
            return;
        }
        hotkeys.source = Some(list.to_vec());
        hotkeys.bindings.clear();
        let shortcuts = self.app.get().map(|app| app.global_shortcut());
        if let Some(shortcuts) = shortcuts {
            // 前の登録を外せなくても、新しい登録は試す。
            let _ = shortcuts.unregister_all();
        }
        let plan = hotkeys::plan(list);
        let mut problems = plan.problems;
        if !hotkeys.suspended {
            for (index, shortcut) in plan.register {
                if let Some(shortcuts) = shortcuts
                    && let Err(e) = shortcuts.register(shortcut)
                {
                    problems.push((
                        index,
                        HotkeyProblem::RegisterFailed {
                            message: e.to_string(),
                        },
                    ));
                    continue;
                }
                hotkeys
                    .bindings
                    .insert(shortcut.id(), list[index].action.clone());
            }
        }
        problems.sort_by_key(|(index, _)| *index);
        hotkeys.problems = problems;
        if let Some(body) = Self::hotkeys_message(&hotkeys) {
            self.monitors.send_all(&body);
        }
    }

    /// 押されたホットキーの操作を行い、操作ウィンドウに知らせる。
    fn run_hotkey(&self, id: u32) {
        let Some(action) = lock(&self.hotkeys).bindings.get(&id).cloned() else {
            return;
        };
        let error = self.perform_hotkey(&action);
        if let Some(body) = Self::json(&Monitor::Hotkey {
            action: &action,
            error,
        }) {
            self.monitors.send_all(&body);
        }
    }

    fn perform_hotkey(&self, action: &HotkeyAction) -> Option<CommandError> {
        let animate = |f: &dyn Fn(
            &mut crate::animation::Animation,
            &mut ugoe::Puppet,
        ) -> Result<bool, AnimationError>| {
            lock(&self.conductor).perform(|animation, puppet| match puppet {
                Some(puppet) => f(animation, puppet),
                None => Err(AnimationError::NoModel),
            })
        };
        let result = match action {
            HotkeyAction::Scene { scene } => {
                return self
                    .switch_scene(|stage, conductor| {
                        stage.begin_activate(scene, conductor).map(Some)
                    })
                    .err()
                    .map(CommandError::from);
            }
            HotkeyAction::DressUp { slot, module } => {
                return self
                    .switch_scene(|stage, conductor| {
                        stage.begin_dress_up(slot, module.as_deref(), conductor)
                    })
                    .err()
                    .map(CommandError::from);
            }
            HotkeyAction::Expression { expression, mode } => animate(&|animation, puppet| {
                match mode {
                    ExpressionMode::Set => animation.set_expression(puppet, expression),
                    ExpressionMode::Toggle => animation.toggle_expression(puppet, expression),
                }
                .map(|()| true)
            }),
            HotkeyAction::ClearExpressions => animate(&|animation, puppet| {
                animation.clear_expressions(puppet);
                Ok(true)
            }),
            HotkeyAction::Motion { motion, priority } => {
                animate(&|animation, puppet| animation.play(puppet, motion, (*priority).into()))
            }
            HotkeyAction::StopMotions => animate(&|animation, puppet| {
                animation.stop(puppet);
                Ok(true)
            }),
            HotkeyAction::TrackingPause => {
                let mut conductor = lock(&self.conductor);
                conductor.perform(|animation, puppet| {
                    let paused = !animation.tracking_paused();
                    animation.set_tracking_paused(puppet, paused);
                });
                Ok(true)
            }
            HotkeyAction::LipSyncPause => {
                let mut conductor = lock(&self.conductor);
                conductor.perform(|animation, puppet| {
                    let paused = !animation.lip_sync_paused();
                    animation.set_lip_sync_paused(puppet, paused);
                });
                Ok(true)
            }
            // キャリブレーションは操作ウィンドウのトラッカーが行う (知らせを受けて)。
            HotkeyAction::TrackingCalibrate | HotkeyAction::Unknown => Ok(true),
        };
        match result {
            Ok(true) => None,
            Ok(false) => Some(CommandError::motion_rejected()),
            Err(e) => Some(e.into()),
        }
    }

    fn send_output_state(&self, state: OutputState) {
        if let Some(body) = Self::json(&Monitor::Output(state)) {
            self.monitors.send_all(&body);
        }
    }

    /// 今のシーン `revision` のファイルを `pick` で選ぶ。今のシーンでなければエラー (出力ウィンドウとブラウザソースの受け手が取る)。
    fn scene_bytes(
        &self,
        revision: u32,
        pick: impl FnOnce(&Stage) -> Option<Arc<Vec<u8>>>,
    ) -> Result<Arc<Vec<u8>>, CommandError> {
        let stage = lock(&self.stage);
        let current = lock(&self.conductor).scene().map(|s| s.revision);
        if current != Some(revision) {
            return Err(CommandError::scene_not_current(revision));
        }
        pick(&stage).ok_or_else(|| CommandError::scene_no_file(revision))
    }

    /// 仮想カメラに渡す今のシーン (シーンの通知と、受け手が `scene_model` などで取るファイル)。
    fn camera_scene(stage: &Stage, conductor: &Conductor) -> Option<SceneFiles> {
        let scene = conductor.scene()?.clone();
        let modules = (0..scene.modules.len())
            .map(|i| stage.module_bytes(i))
            .collect();
        Some(SceneFiles {
            model: stage.model_bytes(),
            modules,
            background: stage.background_image(),
            scene,
        })
    }

    /// 仮想カメラの今の状態 (動かしたことが無ければ、始められるかを調べる)。
    fn camera_status(&self) -> virtual_camera::Status {
        lock(&self.camera_status)
            .get_or_insert_with(|| virtual_camera::Status::Stopped {
                unavailable: virtual_camera::unavailable(),
            })
            .clone()
    }

    /// 仮想カメラのスレッドからの知らせを受ける。
    fn camera_event(&self, event: CameraEvent, fps: u32) {
        match event {
            CameraEvent::Status(status) => {
                if !matches!(status, virtual_camera::Status::Running { .. }) {
                    lock(&self.renders).remove(virtual_camera::LABEL);
                }
                // 止まった (止めた・失敗した) 受け手の報告は捨てる。始め直したら今のシーンを読み直して知らせる。
                if matches!(
                    status,
                    virtual_camera::Status::Stopped { .. } | virtual_camera::Status::Failed { .. }
                ) {
                    self.forget_load_failure(virtual_camera::LABEL);
                }
                if let Some(body) = Self::json(&Monitor::VirtualCamera(&status)) {
                    self.monitors.send_all(&body);
                }
                *lock(&self.camera_status) = Some(status);
            }
            CameraEvent::LoadFailures(failures) => {
                self.report_load_failures(virtual_camera::LABEL, &failures);
            }
            CameraEvent::Stats {
                fps: measured,
                render_ms,
                render_max_ms,
                dropped,
            } => {
                let stats = RenderStats {
                    fps: measured,
                    render_ms,
                    render_max_ms,
                    dropped,
                };
                lock(&self.renders).insert(
                    virtual_camera::LABEL.to_owned(),
                    Reported {
                        stats,
                        target_fps: Some(fps),
                        at: Instant::now(),
                    },
                );
            }
        }
    }

    /// 仮想カメラを止め、スレッドが終わるのを待つ。
    fn stop_camera(&self) {
        // 仮想カメラのスレッドは知らせ (`report_load_failure`) で指揮役を握ることがあり、指揮役を握ったまま `camera` を
        // 握る経路 (`send_scene`・刻み) があるので、`camera` を放してから終わりを待つ。
        let camera = lock(&self.camera).take();
        if let Some(camera) = camera {
            let _ = camera.commands.send(CameraCommand::Stop);
            let _ = camera.thread.join();
        }
    }
}

/// 刻みのスレッド。設定した速さで `dt` を測って指揮役を進める (WebView の抑制を受けない)。
/// 遅れたときは取り戻そうとせず、次の刻みを今から数え直す (`dt` は実測なので時間は失わない。`pacing.rs`)。
fn run_ticks(studio: Arc<Studio>) {
    let mut rate = studio.tick_rate();
    let mut last = Instant::now();
    let mut pacer = Pacer::new(rate, last);
    let mut meter = RateMeter::default();
    let mut count = 0u32;
    loop {
        let now = Instant::now();
        let deadline = pacer.deadline();
        if deadline > now {
            std::thread::sleep(deadline - now);
        }
        let now = Instant::now();
        pacer.tick(now);
        meter.record(now);
        if meter.measured() != *lock(&studio.measured) {
            *lock(&studio.measured) = meter.measured();
        }
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        count = (count + 1) % rate.status_every();
        studio.tick(dt, count == 0);
        // 操作ウィンドウの入力の処理を促す (隠れていても Channel のメッセージは届く。docs/studio.md#隠れても止まらない)。
        studio
            .pulses
            .send_all(&InvokeResponseBody::Json("0".to_owned()));
        let wanted = studio.tick_rate();
        if wanted != rate {
            rate = wanted;
            pacer.set_rate(rate, Instant::now());
            meter.reset();
            *lock(&studio.measured) = None;
            count = 0;
        }
    }
}

/// 起動して最初の 1 回だけ、最後に使ったプロファイルを開く (無ければ何もしない)。開けなければエラー (新しいプロファイルのまま)。
#[tauri::command]
async fn restore_profile(studio: tauri::State<'_, Arc<Studio>>) -> Result<(), CommandError> {
    blocking(&studio, |studio| {
        if studio.restored.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let Some(path) = lock(&studio.stage).recent().first().cloned() else {
            return Ok(());
        };
        open_profile_at(studio, &path)
    })
    .await?
}

fn open_profile_at(studio: &Arc<Studio>, path: &Path) -> Result<(), CommandError> {
    // プロファイルとシーンのファイルは舞台のロックの外で読む (T0834。パペットの組み立ても、T0837)。
    let read = Profile::read(path);
    let error = studio
        .with_stage(|stage, conductor| stage.begin_open_profile(path, read, conductor))?
        .and_then(|pending| match pending {
            Some(pending) => studio.present_with(pending, stage::LOADING),
            None => Ok(()),
        })
        .err();
    // 仮想カメラの設定もプロファイルのものにする (仮想カメラを始められなくても、プロファイルは開いている)。
    let _ = sync_camera(studio);
    match error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

/// 新しいプロファイルにする (空のシーンを 1 つ)。
#[tauri::command]
async fn new_profile(studio: tauri::State<'_, Arc<Studio>>) -> Result<(), CommandError> {
    // 仮想カメラを始め直すことがある (スレッドの終わりを待つ) ので、メインのスレッドの外で行う。
    blocking(&studio, |studio| {
        studio.with_stage(Stage::new_profile);
        let _ = sync_camera(studio);
    })
    .await?;
    Ok(())
}

/// プロファイルを開き、最後に使ったシーンにする。シーンを出せなくてもプロファイルは開く (エラーは返す)。
/// 開けるのは、開くダイアログで選んだパスと最近のプロファイル (`access::FileAccess`)。
#[tauri::command]
async fn open_profile(
    studio: tauri::State<'_, Arc<Studio>>,
    path: PathBuf,
) -> Result<(), CommandError> {
    studio.check_readable(&path)?;
    blocking(&studio, move |studio| {
        studio.restored.store(true, Ordering::SeqCst);
        open_profile_at(studio, &path)
    })
    .await?
}

/// 保存する。`path` を省くと今の保存先 (無ければ何もせず `null`)。保存先を返す。
/// `path` は `save_file_dialog` で利用者が選んだもの (1 回だけ書ける)。
#[tauri::command]
async fn save_profile(
    studio: tauri::State<'_, Arc<Studio>>,
    path: Option<PathBuf>,
) -> Result<Option<PathBuf>, CommandError> {
    if let Some(path) = &path {
        studio.files.take_writable(path)?;
    }
    blocking(&studio, move |studio| {
        Ok(studio.with_stage(|stage, _| stage.save_profile(path.as_deref()))?)
    })
    .await?
}

/// 最近のプロファイルの一覧から外す。
#[tauri::command]
fn forget_recent_profile(studio: tauri::State<'_, Arc<Studio>>, path: PathBuf) {
    studio.with_stage(|stage, _| stage.forget_recent(&path));
}

/// モデルの無いシーンを加え、その ID を返す。
#[tauri::command]
fn add_scene(studio: tauri::State<'_, Arc<Studio>>, name: String) -> String {
    studio.with_stage(|stage, _| stage.add_scene(&name))
}

/// シーンを複製してすぐ後ろに置き、その ID を返す。
#[tauri::command]
fn duplicate_scene(
    studio: tauri::State<'_, Arc<Studio>>,
    id: String,
    name: String,
) -> Result<String, CommandError> {
    Ok(studio.with_stage(|stage, _| stage.duplicate_scene(&id, &name))?)
}

/// シーンを `index` 番目に動かす。
#[tauri::command]
fn move_scene(
    studio: tauri::State<'_, Arc<Studio>>,
    id: String,
    index: usize,
) -> Result<(), CommandError> {
    Ok(studio.with_stage(|stage, _| stage.move_scene(&id, index))?)
}

/// シーンを消す。
#[tauri::command]
fn remove_scene(studio: tauri::State<'_, Arc<Studio>>, id: String) -> Result<(), CommandError> {
    Ok(studio.with_stage(|stage, _| stage.remove_scene(&id))?)
}

/// シーンの設定を置き換える。出しているシーンなら出し直す (出せなければ元に戻してエラー)。
/// シーンが指すファイルは、開くダイアログで選んだものか今のプロファイルにあるものだけ (`access::FileAccess`)。
#[tauri::command]
async fn update_scene(
    studio: tauri::State<'_, Arc<Studio>>,
    scene: SceneSettings,
) -> Result<(), CommandError> {
    studio.files.check_scene(&scene, &lock(&studio.stage))?;
    blocking(&studio, |studio| {
        Ok(studio.switch_scene(|stage, conductor| stage.begin_update_scene(scene, conductor))?)
    })
    .await?
}

/// シーンに切り替える (出せなければ前のシーンのままエラー)。
#[tauri::command]
async fn activate_scene(
    studio: tauri::State<'_, Arc<Studio>>,
    id: String,
) -> Result<(), CommandError> {
    blocking(&studio, move |studio| {
        Ok(studio
            .switch_scene(|stage, conductor| stage.begin_activate(&id, conductor).map(Some))?)
    })
    .await?
}

/// 出力ウィンドウの大きさのプリセットをプロファイルに覚える。
#[tauri::command]
fn set_output_preset(studio: tauri::State<'_, Arc<Studio>>, preset: String) {
    studio.with_stage(|stage, _| stage.set_output_size(&preset));
}

/// 出力のフレームを受け取り始める。シーンがあれば最初に送る (以後は刻みごとのフレーム)。
#[tauri::command]
fn subscribe_frames(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    channel: Channel<InvokeResponseBody>,
) {
    let conductor = lock(&studio.conductor);
    if let Some(body) = Studio::scene_message(&conductor) {
        let _ = channel.send(body);
    }
    studio.outputs.insert(webview.label(), channel);
}

/// 今のシーン `revision` のファイルを `pick` で選んで返す。今のシーンでなければエラー (受け手は次のシーンを待つ)。
/// 大きなバイト列を写すので、メインのスレッドの外で行う。
async fn scene_file(
    studio: &Arc<Studio>,
    revision: u32,
    pick: impl FnOnce(&Stage) -> Option<Arc<Vec<u8>>> + Send + 'static,
) -> Result<Response, CommandError> {
    blocking(studio, move |studio| {
        studio
            .scene_bytes(revision, pick)
            .map(|bytes| Response::new(bytes.to_vec()))
    })
    .await?
}

/// シーン `revision` のモデルのバイト列。
#[tauri::command]
async fn scene_model(
    studio: tauri::State<'_, Arc<Studio>>,
    revision: u32,
) -> Result<Response, CommandError> {
    scene_file(&studio, revision, Stage::model_bytes).await
}

/// シーン `revision` の `index` 番目 (合成の順) のモジュールのバイト列。
#[tauri::command]
async fn scene_module(
    studio: tauri::State<'_, Arc<Studio>>,
    revision: u32,
    index: usize,
) -> Result<Response, CommandError> {
    scene_file(&studio, revision, move |stage| stage.module_bytes(index)).await
}

/// シーン `revision` の背景の画像のバイト列。
#[tauri::command]
async fn scene_background(
    studio: tauri::State<'_, Arc<Studio>>,
    revision: u32,
) -> Result<Response, CommandError> {
    scene_file(&studio, revision, Stage::background_image).await
}

/// 操作ウィンドウが値と負荷を受け取り始める。シーンがあれば最初に送る。
#[tauri::command]
fn subscribe_status(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    channel: Channel<InvokeResponseBody>,
) {
    let stage = lock(&studio.stage);
    if let Some(body) = Studio::json(&Monitor::Profile(stage.view())) {
        let _ = channel.send(body);
    }
    if let Some(body) = Studio::scene_loading_message(&stage) {
        let _ = channel.send(body);
    }
    let conductor = lock(&studio.conductor);
    if let Some(body) = Studio::monitor_scene_message(&conductor) {
        let _ = channel.send(body);
    }
    if let Some(body) = Studio::json(&Monitor::Animation(conductor.animation_view())) {
        let _ = channel.send(body);
    }
    if let Some(body) = Studio::hotkeys_message(&lock(&studio.hotkeys)) {
        let _ = channel.send(body);
    }
    drop(conductor);
    drop(stage);
    if let Some(body) = Studio::json(&Monitor::Api(studio.api_view())) {
        let _ = channel.send(body);
    }
    if let Some(body) = studio.load_failures_message() {
        let _ = channel.send(body);
    }
    studio.monitors.insert(webview.label(), channel);
}

/// 刻みごとに入力の処理を促すメッセージを受け取り始める (操作ウィンドウのカメラとマイク。`requestAnimationFrame` に頼らない)。
#[tauri::command]
fn subscribe_pulse(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    channel: Channel<InvokeResponseBody>,
) {
    studio.pulses.insert(webview.label(), channel);
}

/// トラッキングの入力 (キャリブレーションと平滑化の後)。`null` で外す (トラッキングが書き込んでいたパラメータは既定値に戻る)。
#[tauri::command]
fn set_tracking(studio: tauri::State<'_, Arc<Studio>>, frame: Option<TrackingFrame>) {
    let input = frame.as_ref().map(TrackingFrame::to_input);
    lock(&studio.conductor).perform(|animation, puppet| animation.set_tracking(puppet, input));
}

/// トラッキングを一時停止する・再開する。
#[tauri::command]
fn set_tracking_paused(studio: tauri::State<'_, Arc<Studio>>, paused: bool) {
    lock(&studio.conductor)
        .perform(|animation, puppet| animation.set_tracking_paused(puppet, paused));
}

/// モデルの自動の目パチを動かすか (トラッキング中は止める)。
#[tauri::command]
fn set_auto_eye_blink(studio: tauri::State<'_, Arc<Studio>>, enabled: bool) {
    lock(&studio.conductor)
        .perform(|animation, puppet| animation.set_auto_eye_blink(puppet, enabled));
}

/// マイクの音量 (0〜1)。`null` で口パクを止める。
#[tauri::command]
fn set_lip_sync_level(studio: tauri::State<'_, Arc<Studio>>, level: Option<f32>) {
    lock(&studio.conductor).set_lip_sync_level(level);
}

/// 口パクを一時停止する・再開する。
#[tauri::command]
fn set_lip_sync_paused(studio: tauri::State<'_, Arc<Studio>>, paused: bool) {
    lock(&studio.conductor)
        .perform(|animation, puppet| animation.set_lip_sync_paused(puppet, paused));
}

/// 指揮役の演技を `action` で触る (モデルが無ければエラー)。
fn animate<T>(
    studio: &Studio,
    action: impl FnOnce(
        &mut crate::animation::Animation,
        &mut ugoe::Puppet,
    ) -> Result<T, AnimationError>,
) -> Result<T, CommandError> {
    lock(&studio.conductor)
        .perform(|animation, puppet| match puppet {
            Some(puppet) => action(animation, puppet),
            None => Err(AnimationError::NoModel),
        })
        .map_err(CommandError::from)
}

/// モーションを優先度付きで始める。再生中のモーションより優先度が低くて始まらなければ `false`。
#[tauri::command]
fn play_motion(
    studio: tauri::State<'_, Arc<Studio>>,
    name: String,
    priority: MotionPriority,
) -> Result<bool, CommandError> {
    animate(&studio, |animation, puppet| {
        animation.play(puppet, &name, priority.into())
    })
}

/// 再生中のモーションを止める (待機モーションに戻る)。
#[tauri::command]
fn stop_motions(studio: tauri::State<'_, Arc<Studio>>) -> Result<(), CommandError> {
    animate(&studio, |animation, puppet| {
        animation.stop(puppet);
        Ok(())
    })
}

/// 表情を `name` だけにする。
#[tauri::command]
fn set_expression(studio: tauri::State<'_, Arc<Studio>>, name: String) -> Result<(), CommandError> {
    animate(&studio, |animation, puppet| {
        animation.set_expression(puppet, &name)
    })
}

/// 表情 `name` を重ねる。
#[tauri::command]
fn add_expression(studio: tauri::State<'_, Arc<Studio>>, name: String) -> Result<(), CommandError> {
    animate(&studio, |animation, puppet| {
        animation.add_expression(puppet, &name)
    })
}

/// 表情 `name` を外す。
#[tauri::command]
fn remove_expression(
    studio: tauri::State<'_, Arc<Studio>>,
    name: String,
) -> Result<(), CommandError> {
    animate(&studio, |animation, puppet| {
        animation.remove_expression(puppet, &name);
        Ok(())
    })
}

/// 表情をすべて外す。
#[tauri::command]
fn clear_expressions(studio: tauri::State<'_, Arc<Studio>>) -> Result<(), CommandError> {
    animate(&studio, |animation, puppet| {
        animation.clear_expressions(puppet);
        Ok(())
    })
}

/// 待機モーションとトラッキング中のモーションの優先の設定 (プロファイルに持つ)。
#[tauri::command]
fn set_motion_settings(studio: tauri::State<'_, Arc<Studio>>, settings: MotionSettings) {
    studio.with_stage(|stage, conductor| stage.set_motion_settings(settings, conductor));
}

/// 入力の機器と調整 (プロファイルに持つ)。
#[tauri::command]
fn set_input_settings(studio: tauri::State<'_, Arc<Studio>>, settings: InputSettings) {
    studio.with_stage(|stage, _| stage.set_input_settings(settings));
}

/// ホットキーの割り当て (プロファイルに持ち、登録し直す)。着せ替えのモジュールは、開くダイアログで選んだものか
/// 今のプロファイルにあるものだけ (`access::FileAccess`)。
#[tauri::command]
fn set_hotkeys(
    studio: tauri::State<'_, Arc<Studio>>,
    hotkeys: Vec<Hotkey>,
) -> Result<(), CommandError> {
    studio.files.check_hotkeys(&hotkeys, &lock(&studio.stage))?;
    studio.with_stage(|stage, _| stage.set_hotkeys(hotkeys));
    Ok(())
}

/// 割り当ての画面でキーを押している間、ホットキーの登録を外す (押したキーが操作を行わないように)。
#[tauri::command]
fn suspend_hotkeys(studio: tauri::State<'_, Arc<Studio>>, suspended: bool) {
    let list = {
        let mut hotkeys = lock(&studio.hotkeys);
        if hotkeys.suspended == suspended {
            return;
        }
        hotkeys.suspended = suspended;
        hotkeys.source.clone().unwrap_or_default()
    };
    studio.sync_hotkeys(&list, true);
}

/// モジュール (`.ugoemod`) が付くスロットの名前 (ホットキーの着せ替えの割り当て)。読めるのは、開くダイアログで選んだものか
/// 今のプロファイルにあるものだけ (`access::FileAccess`)。
#[tauri::command]
async fn module_slot(
    studio: tauri::State<'_, Arc<Studio>>,
    path: PathBuf,
) -> Result<String, CommandError> {
    studio.check_readable(&path)?;
    tauri::async_runtime::spawn_blocking(move || read_module_slot(&path)).await?
}

/// 開くダイアログを開き、選ばれたファイルのパスを返す (取り消したら `null`)。選ばれたパスは読めるパスに加える
/// (`access::FileAccess`)。拡張子のフィルタと名前は画面が渡す。ダイアログをバックエンドで開くのは、利用者が選んだパスを
/// バックエンドが知るため (T0838。ビューアの `open_file_dialog` と同じ。複数は選ばない)。
#[tauri::command]
async fn open_file_dialog(
    window: WebviewWindow,
    studio: tauri::State<'_, Arc<Studio>>,
    filter_name: String,
    extensions: Vec<String>,
) -> Result<Option<String>, CommandError> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let extensions: Vec<&str> = extensions.iter().map(String::as_str).collect();
        let dialog = window.dialog().file().add_filter(filter_name, &extensions);
        // tauri-plugin-dialog の `open` と同じく、親のウィンドウは Windows と macOS でだけ付ける。
        #[cfg(any(windows, target_os = "macos"))]
        let dialog = dialog.set_parent(&window);
        dialog
            .blocking_pick_file()
            .map(|p| p.into_path().map_err(CommandError::dialog_failed))
            .transpose()
    })
    .await??;
    let Some(path) = picked.map(path_string) else {
        return Ok(None);
    };
    studio.files.grant_readable([PathBuf::from(&path)]);
    Ok(Some(path))
}

/// プロファイルの保存のダイアログを開き、選ばれたパスを返す (取り消したら `null`)。拡張子 `.json` が無ければ付ける
/// (Linux のダイアログはフィルタの拡張子を付けずに返すことがある。T0011)。返したパスは `save_profile` で 1 回だけ書ける。
#[tauri::command]
async fn save_file_dialog(
    window: WebviewWindow,
    studio: tauri::State<'_, Arc<Studio>>,
    filter_name: String,
    default_name: String,
) -> Result<Option<String>, CommandError> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_file_name(default_name)
            .add_filter(filter_name, &[PROFILE_EXTENSION])
            .set_parent(&window)
            .blocking_save_file()
            .map(|p| p.into_path().map_err(CommandError::dialog_failed))
            .transpose()
    })
    .await??;
    let Some(path) = picked.map(|p| path_string(ugoe_fs::with_extension(p, PROFILE_EXTENSION)))
    else {
        return Ok(None);
    };
    studio.files.grant_writable(PathBuf::from(&path));
    Ok(Some(path))
}

/// プロファイルの拡張子。
const PROFILE_EXTENSION: &str = "json";

/// 画面に渡すパスの文字列。画面はこの文字列のまま返してくるので、許したパスもこの文字列から作る。
fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn read_module_slot(path: &Path) -> Result<String, CommandError> {
    let bytes = stage::read_asset(
        stage::Asset::Module,
        path,
        stage::MODULE_EXTENSIONS,
        stage::MAX_MODEL_BYTES,
    )?;
    let file = ugoe::format::read_module(&bytes).map_err(|e| SceneError::Unreadable {
        asset: stage::Asset::Module,
        path: path.to_owned(),
        message: e.to_string(),
    })?;
    Ok(file.module.slot.as_str().to_owned())
}

/// パラメータを書き換える (次の刻みから基準値になる)。ID が無ければ `false`。
#[tauri::command]
fn set_parameter(studio: tauri::State<'_, Arc<Studio>>, id: String, value: f32) -> bool {
    value.is_finite() && lock(&studio.conductor).set_parameter(&id, value)
}

/// すべてのパラメータを既定値に戻す。
#[tauri::command]
fn reset_parameters(studio: tauri::State<'_, Arc<Studio>>) {
    lock(&studio.conductor).reset_parameters();
}

/// 刻みの速さ (回/秒)。
#[tauri::command]
fn tick_rate(studio: tauri::State<'_, Arc<Studio>>) -> u32 {
    studio.tick_rate().hz()
}

/// 刻みの速さを 30 / 60 / 120 から選ぶ (プロファイルに持つ。次の刻みから)。
#[tauri::command]
fn set_tick_rate(studio: tauri::State<'_, Arc<Studio>>, rate: u32) -> Result<u32, CommandError> {
    let rate = TickRate::from_hz(rate).ok_or(CommandError::unsupported_tick_rate(rate))?;
    studio.with_stage(|stage, _| stage.set_tick_rate(rate));
    Ok(rate.hz())
}

/// 出力が描画の負荷を報告する (操作ウィンドウに出す。出力ウィンドウには何も重ねない)。
#[tauri::command]
fn report_render_stats(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    stats: RenderStats,
) -> Result<(), CommandError> {
    if !stats.is_valid() {
        return Err(CommandError::invalid_render_stats());
    }
    lock(&studio.renders).insert(
        webview.label().to_owned(),
        Reported {
            stats,
            target_fps: None,
            at: Instant::now(),
        },
    );
    Ok(())
}

/// 受け手が今のシーンのファイルを読めなかったことを報告する (操作ウィンドウに出す。出力ウィンドウには何も重ねない。
/// 今のシーンの報告でなければ捨てる。docs/studio.md#受け手が読めなかったとき)。
#[tauri::command]
fn report_load_failure(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    failure: load_failure::LoadFailure,
) {
    studio.report_load_failure(webview.label(), failure);
}

/// 仮想カメラの今の状態 (止めているなら、始められない理由も)。
#[tauri::command]
fn virtual_camera_state(studio: tauri::State<'_, Arc<Studio>>) -> virtual_camera::Status {
    studio.camera_status()
}

/// 仮想カメラを、プロファイルの設定で始める (動いていれば止めてから始め直す)。状態の変化は操作ウィンドウへの通知で届く。
/// `settings` を渡すと、先にプロファイルの設定にする。
#[tauri::command]
async fn start_virtual_camera(
    studio: tauri::State<'_, Arc<Studio>>,
    settings: Option<CameraSettings>,
) -> Result<(), CommandError> {
    // 動いている仮想カメラのスレッドの終わりを待つので、メインのスレッドの外で行う。
    blocking(&studio, move |studio| {
        if let Some(settings) = settings {
            validate_camera(settings)?;
            studio.with_stage(|stage, _| {
                stage.set_virtual_camera(VirtualCameraSettings::from_settings(settings));
            });
        }
        let settings = lock(&studio.stage)
            .profile()
            .output
            .virtual_camera
            .settings();
        start_camera(studio, settings)
    })
    .await?
}

/// 仮想カメラの設定を変える (プロファイルに持つ)。動いていれば効かせる (`sync_camera`)。
#[tauri::command]
async fn set_virtual_camera_settings(
    studio: tauri::State<'_, Arc<Studio>>,
    settings: VirtualCameraSettings,
) -> Result<(), CommandError> {
    // 仮想カメラを始め直すことがある (スレッドの終わりを待つ) ので、メインのスレッドの外で行う。
    blocking(&studio, move |studio| {
        studio.with_stage(|stage, _| stage.set_virtual_camera(settings));
        sync_camera(studio)
    })
    .await?
}

/// プロファイルの仮想カメラの設定を、動いている仮想カメラに効かせる。解像度かフレームレートが変われば始め直し
/// (OS のカメラの形が変わるため)、色だけなら次の画から変える。止まった (失敗した) 仮想カメラは始め直さない。
fn sync_camera(studio: &Arc<Studio>) -> Result<(), CommandError> {
    let wanted = lock(&studio.stage)
        .profile()
        .output
        .virtual_camera
        .settings();
    let active = matches!(
        studio.camera_status(),
        virtual_camera::Status::Starting { .. } | virtual_camera::Status::Running { .. }
    );
    {
        let mut camera = lock(&studio.camera);
        let Some(running) = camera.as_mut() else {
            return Ok(());
        };
        if running.settings == wanted {
            return Ok(());
        }
        if running.settings.config() == wanted.config() || !active {
            let _ = running
                .commands
                .send(CameraCommand::Background(wanted.background));
            running.settings.background = wanted.background;
            return Ok(());
        }
    }
    start_camera(studio, wanted)
}

/// 選べない解像度・フレームレートなら `virtual_camera.invalid_config`。
fn validate_camera(settings: CameraSettings) -> Result<(), CommandError> {
    let config = settings.config();
    config
        .validate()
        .map_err(|_| CommandError::invalid_camera_config(config))
}

fn start_camera(studio: &Arc<Studio>, settings: CameraSettings) -> Result<(), CommandError> {
    validate_camera(settings)?;
    studio.stop_camera();
    let (tx, rx) = std::sync::mpsc::channel();
    // 刻みのスレッドより先にシーンが届くよう、舞台と指揮役を (この順に) 握ったままシーンを送り、送り先を置く。
    let stage = lock(&studio.stage);
    let conductor = lock(&studio.conductor);
    if let Some(files) = Studio::camera_scene(&stage, &conductor) {
        let _ = tx.send(CameraCommand::Scene(Box::new(files)));
    }
    let events = Arc::clone(studio);
    let thread = std::thread::Builder::new()
        .name("ugoe-studio-virtual-camera".into())
        .spawn(move || {
            virtual_camera::run(settings, rx, |event| {
                events.camera_event(event, settings.fps)
            });
        })
        .map_err(CommandError::internal)?;
    *lock(&studio.camera) = Some(CameraThread {
        commands: tx,
        thread,
        settings,
    });
    drop(conductor);
    drop(stage);
    Ok(())
}

/// 仮想カメラを止める (受け手と GPU の描画も止める)。
#[tauri::command]
async fn stop_virtual_camera(studio: tauri::State<'_, Arc<Studio>>) -> Result<(), CommandError> {
    // 仮想カメラのスレッドの終わりを待つので、メインのスレッドの外で行う。
    Ok(blocking(&studio, |studio| studio.stop_camera()).await?)
}

fn output_window(app: &AppHandle) -> Result<WebviewWindow, CommandError> {
    app.get_webview_window(OUTPUT)
        .ok_or_else(CommandError::output_window_missing)
}

fn read_output_state(window: &WebviewWindow) -> Result<OutputState, CommandError> {
    let size = window
        .inner_size()
        .map_err(CommandError::output_window_failed)?;
    Ok(OutputState {
        visible: window
            .is_visible()
            .map_err(CommandError::output_window_failed)?,
        on_top: window
            .is_always_on_top()
            .map_err(CommandError::output_window_failed)?,
        width: size.width,
        height: size.height,
    })
}

/// 出力ウィンドウの状態。
#[tauri::command]
fn output_state(app: AppHandle) -> Result<OutputState, CommandError> {
    read_output_state(&output_window(&app)?)
}

/// 出力ウィンドウを表示する・隠す (隠しても受け手は動き続ける)。
#[tauri::command]
fn set_output_visible(app: AppHandle, visible: bool) -> Result<OutputState, CommandError> {
    let window = output_window(&app)?;
    if visible {
        window.show().map_err(CommandError::output_window_failed)?;
    } else {
        window.hide().map_err(CommandError::output_window_failed)?;
    }
    // Linux (tao) はウィンドウの操作をイベントループへ送ってから行うので、直後の `is_visible` はまだ前の値を返す
    // (操作ウィンドウのボタンが切り替わらなかった。T0006 の実機のテストで見つけた)。頼んだ値を返す。
    Ok(OutputState {
        visible,
        ..read_output_state(&window)?
    })
}

/// 出力ウィンドウを最前面にする・戻す。
#[tauri::command]
fn set_output_on_top(app: AppHandle, on_top: bool) -> Result<OutputState, CommandError> {
    let window = output_window(&app)?;
    window
        .set_always_on_top(on_top)
        .map_err(CommandError::output_window_failed)?;
    // `set_output_visible` と同じく、直後に読むと前の値のことがあるので、頼んだ値を返す。
    Ok(OutputState {
        on_top,
        ..read_output_state(&window)?
    })
}

/// 出力ウィンドウの中身の大きさを物理ピクセルで決める (配信ソフトが取り込む大きさ。1920×1080 など)。
#[tauri::command]
fn set_output_size(app: AppHandle, width: u32, height: u32) -> Result<OutputState, CommandError> {
    if !(16..=8192).contains(&width) || !(16..=8192).contains(&height) {
        return Err(CommandError::unsupported_output_size(width, height));
    }
    let window = output_window(&app)?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(CommandError::output_window_failed)?;
    read_output_state(&window)
}

/// WebView からの許可の要求への応答。操作ウィンドウのカメラ (フェイストラッキング) とマイク (口パク) だけを許す (T0803)。
/// ページはアプリに同梱したものだけなので、使うのは利用者が入力を始めたときに限られる。
/// macOS では、これとは別に OS の許可 (`Info.plist` の `NSCameraUsageDescription`・`NSMicrophoneUsageDescription`) が要る。
fn permission_response(label: &str, kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Camera | PermissionKind::Microphone if label == CONTROL => {
            PermissionResponse::Allow
        }
        _ => PermissionResponse::Default,
    }
}

pub fn run() {
    let studio = Arc::new(Studio::default());
    let ticking = Arc::clone(&studio);
    let staging = Arc::clone(&studio);
    let pressing = Arc::clone(&studio);
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |_, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        // シーンの切り替えはファイルを読むので、イベントのスレッド (メインのスレッド) を塞がない。
                        let studio = Arc::clone(&pressing);
                        let id = shortcut.id();
                        tauri::async_runtime::spawn_blocking(move || studio.run_hotkey(id));
                    }
                })
                .build(),
        )
        .on_permission_request(|webview, kind| permission_response(webview.label(), kind))
        .manage(studio)
        .setup(move |app| {
            // 最近のプロファイルはアプリのデータの場所に覚える (取れなければ覚えない)。
            let recent = app
                .path()
                .app_config_dir()
                .ok()
                .map(|dir| dir.join(RECENT_PROFILES_FILE));
            *lock(&staging.stage) = Stage::new(recent);
            let _ = staging.app.set(app.handle().clone());
            // 操作 API の設定と鍵はアプリのデータの場所に置く (プロファイルには入れない。有効なら待ち受ける)。
            let api_settings = app
                .path()
                .app_data_dir()
                .ok()
                .map(|dir| dir.join(api::SETTINGS_FILE));
            // ブラウザソースのページは、アプリに入れたフロントエンド (dist/) から出す (開発モードでは dist/ を直に読む)。
            // 無いファイルを Tauri は index.html で答えるので、HTML でないはずのものが HTML なら無いものとする。
            let resolver = app.asset_resolver();
            staging.sources.set_assets(move |path| {
                let asset = resolver.get(path.to_owned())?;
                let html = asset.mime_type.starts_with("text/html");
                (html == path.ends_with(".html")).then_some(asset.bytes)
            });
            staging.load_api(api_settings);
            std::thread::Builder::new()
                .name("ugoe-studio-conductor".into())
                .spawn(move || run_ticks(ticking))?;
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // 出力ウィンドウは閉じずに隠す (受け手を作り直さない。表示は操作ウィンドウから戻す)。
            (OUTPUT, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
                if let Some(output) = window.app_handle().get_webview_window(OUTPUT)
                    && let Ok(state) = read_output_state(&output)
                {
                    window.state::<Arc<Studio>>().send_output_state(state);
                }
            }
            // 操作ウィンドウを閉じたらアプリを終える (隠れた出力ウィンドウを残さない)。仮想カメラは止めてから
            // (Windows のカメラの機器を消し、Linux の機器を閉じる)。
            (CONTROL, WindowEvent::Destroyed) => {
                window.state::<Arc<Studio>>().stop_camera();
                window.app_handle().exit(0);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            restore_profile,
            new_profile,
            open_profile,
            save_profile,
            forget_recent_profile,
            add_scene,
            duplicate_scene,
            move_scene,
            remove_scene,
            update_scene,
            activate_scene,
            set_output_preset,
            scene_module,
            scene_background,
            subscribe_frames,
            scene_model,
            subscribe_status,
            set_parameter,
            reset_parameters,
            output_state,
            set_output_visible,
            set_output_on_top,
            set_output_size,
            tick_rate,
            set_tick_rate,
            report_render_stats,
            report_load_failure,
            subscribe_pulse,
            set_tracking,
            set_tracking_paused,
            set_auto_eye_blink,
            set_lip_sync_level,
            set_lip_sync_paused,
            play_motion,
            stop_motions,
            set_expression,
            add_expression,
            remove_expression,
            clear_expressions,
            set_motion_settings,
            set_input_settings,
            set_hotkeys,
            suspend_hotkeys,
            module_slot,
            open_file_dialog,
            save_file_dialog,
            virtual_camera_state,
            start_virtual_camera,
            stop_virtual_camera,
            set_virtual_camera_settings,
            api::commands::api_state,
            api::commands::set_api_enabled,
            api::commands::set_api_port,
            api::commands::api_token,
            api::commands::regenerate_api_token,
            api::commands::approve_api_client,
            api::commands::deny_api_client,
            api::commands::revoke_api_client,
            api::commands::disconnect_api_client,
            api::commands::set_source_enabled,
            api::commands::source_url,
            api::commands::regenerate_source_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ugoe studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// メインのスレッドの外で行った処理の結果を返す。中でパニックしたら、`internal` のコードのエラーにする (T0819)。
    #[test]
    fn blocking_work_returns_its_result_or_an_internal_error() {
        let studio = Arc::new(Studio::default());
        let tick_rate = tauri::async_runtime::block_on(blocking(&studio, |s| s.tick_rate()));
        assert_eq!(tick_rate.unwrap(), studio.tick_rate());
        let error: CommandError = tauri::async_runtime::block_on(blocking(&studio, |_| {
            std::panic::resume_unwind(Box::new("boom"))
        }))
        .unwrap_err()
        .into();
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "internal");
        assert!(json["message"].is_string());
    }

    #[test]
    fn monitor_messages_are_tagged() {
        let state = OutputState {
            visible: true,
            on_top: false,
            width: 1920,
            height: 1080,
        };
        assert_eq!(
            serde_json::to_string(&Monitor::Output(state)).unwrap(),
            r#"{"kind":"output","visible":true,"onTop":false,"width":1920,"height":1080}"#
        );
        assert_eq!(
            serde_json::to_string(&Monitor::SceneLoading {
                scene: Some("scene-2")
            })
            .unwrap(),
            r#"{"kind":"sceneLoading","scene":"scene-2"}"#
        );
        assert_eq!(
            serde_json::to_string(&Monitor::SceneLoading { scene: None }).unwrap(),
            r#"{"kind":"sceneLoading","scene":null}"#
        );
        let status = Monitor::Status {
            seq: 3,
            values: &[0.5],
            tick_ms: 0.25,
            outputs: 1,
            tick_rate: 60,
            measured_rate: Some(59.5),
            max_interval_ms: None,
            renderers: vec![RendererLoad {
                label: "output".into(),
                stats: RenderStats {
                    fps: 60.0,
                    render_ms: 1.5,
                    render_max_ms: 2.0,
                    dropped: 0,
                },
                stale: false,
                target_fps: None,
            }],
        };
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            concat!(
                r#"{"kind":"status","seq":3,"values":[0.5],"tickMs":0.25,"outputs":1,"tickRate":60,"#,
                r#""measuredRate":59.5,"maxIntervalMs":null,"renderers":[{"label":"output","fps":60.0,"#,
                r#""renderMs":1.5,"renderMaxMs":2.0,"dropped":0,"stale":false,"targetFps":null}]}"#
            )
        );
    }

    /// 隠れても最小化されても止まらないように、両方のウィンドウでバックグラウンドの抑制を切る (T0804)。
    /// WebView2 は同じデータの場所を使う WebView の引数が食い違うと作れないので、両方を同じにする。
    #[test]
    fn both_windows_disable_background_throttling() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let windows = config["app"]["windows"].as_array().unwrap();
        assert_eq!(windows.len(), 2);
        let args = windows[0]["additionalBrowserArgs"].as_str().unwrap();
        for window in windows {
            assert_eq!(window["backgroundThrottling"], "disabled", "{window}");
            assert_eq!(window["additionalBrowserArgs"], args, "{window}");
        }
        for flag in [
            // wry の既定の引数 (上書きすると消えるので書き写す)
            "msWebOOUI,msPdfOOUI,msSmartScreenProtection",
            "CalculateNativeWinOcclusion",
            "IntensiveWakeUpThrottling",
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
        ] {
            assert!(args.contains(flag), "{flag}");
        }
    }

    /// カメラとマイクは操作ウィンドウだけに許す (T0803)。
    #[test]
    fn only_the_control_window_may_use_the_camera_and_the_microphone() {
        for kind in [PermissionKind::Camera, PermissionKind::Microphone] {
            assert!(matches!(
                permission_response(CONTROL, kind),
                PermissionResponse::Allow
            ));
            assert!(matches!(
                permission_response(OUTPUT, kind),
                PermissionResponse::Default
            ));
        }
        assert!(matches!(
            permission_response(CONTROL, PermissionKind::Notifications),
            PermissionResponse::Default
        ));
    }

    #[test]
    fn hotkey_messages_are_tagged() {
        let action = HotkeyAction::TrackingCalibrate;
        assert_eq!(
            serde_json::to_value(Monitor::Hotkey {
                action: &action,
                error: Some(CommandError::motion_rejected()),
            })
            .unwrap(),
            serde_json::json!({
                "kind": "hotkey",
                "action": { "kind": "trackingCalibrate" },
                "error": {
                    "code": "animation.motion_rejected",
                    "args": {},
                    "message": "the motion did not start because a motion with a higher priority is playing",
                }
            })
        );
        let problems = [(1, HotkeyProblem::Unassigned)];
        assert_eq!(
            serde_json::to_value(Monitor::Hotkeys {
                problems: &problems,
                suspended: false,
                wayland: false,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "hotkeys",
                "problems": [[1, { "code": "unassigned" }]],
                "suspended": false,
                "wayland": false
            })
        );
        let view = Conductor::new().animation_view();
        let json = serde_json::to_value(Monitor::Animation(view)).unwrap();
        assert_eq!(json["kind"], "animation");
        assert_eq!(json["trackingPaused"], false);
    }

    /// OS に登録できない環境 (テスト) でも、割り当ての問題は判断して覚える。
    #[test]
    fn hotkeys_are_planned_without_the_app() {
        let studio = Studio::default();
        let list = vec![
            Hotkey {
                keys: "Ctrl+KeyA".into(),
                action: HotkeyAction::StopMotions,
            },
            Hotkey {
                keys: "Ctrl+KeyA".into(),
                action: HotkeyAction::ClearExpressions,
            },
        ];
        studio.sync_hotkeys(&list, false);
        {
            let hotkeys = lock(&studio.hotkeys);
            assert_eq!(hotkeys.bindings.len(), 1);
            assert_eq!(
                hotkeys.problems,
                [(1, HotkeyProblem::Duplicate { other: 0 })]
            );
        }
        // 割り当ての画面でキーを押している間は、登録しない。
        lock(&studio.hotkeys).suspended = true;
        studio.sync_hotkeys(&list, true);
        assert!(lock(&studio.hotkeys).bindings.is_empty());
        // 押されたホットキーの操作: モデルが無ければ行えない。
        lock(&studio.hotkeys).suspended = false;
        studio.sync_hotkeys(&list, true);
        let id = hotkeys::parse_keys("Ctrl+KeyA").unwrap().id();
        assert!(lock(&studio.hotkeys).bindings.contains_key(&id));
        assert_eq!(
            studio.perform_hotkey(&HotkeyAction::StopMotions),
            Some(AnimationError::NoModel.into())
        );
        assert!(studio.perform_hotkey(&HotkeyAction::LipSyncPause).is_none());
        assert!(lock(&studio.conductor).animation_view().lip_sync_paused);
    }

    #[test]
    fn tick_rate_defaults_to_60_and_accepts_the_presets_only() {
        let studio = Studio::default();
        assert_eq!(studio.tick_rate(), TickRate::Fps60);
        studio.tick_rate.store(120, Ordering::Relaxed);
        assert_eq!(studio.tick_rate(), TickRate::Fps120);
        studio.tick_rate.store(75, Ordering::Relaxed);
        assert_eq!(studio.tick_rate(), TickRate::DEFAULT);
    }

    /// 刻みの速さはプロファイルに持ち、プロファイルを変えると刻みのスレッドが読む値も変わる (T0827)。
    #[test]
    fn the_tick_rate_follows_the_profile() {
        let studio = Studio::default();
        studio.with_stage(|stage, _| stage.set_tick_rate(TickRate::Fps120));
        assert_eq!(studio.tick_rate(), TickRate::Fps120);
        assert!(lock(&studio.stage).dirty());
        studio.with_stage(Stage::new_profile);
        assert_eq!(studio.tick_rate(), TickRate::DEFAULT);
    }

    /// プロファイルの仮想カメラの設定を、動いている仮想カメラに効かせる (色はその場で。T0827)。
    #[test]
    fn camera_settings_reach_the_running_camera() {
        let studio = Arc::new(Studio::default());
        // 止めている間は何もしない。
        studio.with_stage(|stage, _| {
            stage.set_virtual_camera(profile::VirtualCameraSettings {
                background: "#00ff00".into(),
                ..Default::default()
            });
        });
        assert_eq!(sync_camera(&studio), Ok(()));

        let (tx, rx) = std::sync::mpsc::channel();
        let settings = CameraSettings::default();
        *lock(&studio.camera) = Some(CameraThread {
            commands: tx,
            thread: std::thread::spawn(|| {}),
            settings,
        });
        *lock(&studio.camera_status) = Some(virtual_camera::Status::Running {
            settings,
            device: "/dev/video10".into(),
            consumer: true,
            late: false,
        });
        assert_eq!(sync_camera(&studio), Ok(()));
        assert!(matches!(
            rx.try_recv(),
            Ok(CameraCommand::Background([0, 255, 0]))
        ));
        assert_eq!(
            lock(&studio.camera).as_ref().unwrap().settings.background,
            [0, 255, 0]
        );
        // 変わっていなければ送らない。
        assert_eq!(sync_camera(&studio), Ok(()));
        assert!(rx.try_recv().is_err());

        // 止まった (失敗した) 仮想カメラは、解像度が変わっても始め直さない (色だけ渡す)。
        *lock(&studio.camera_status) = Some(virtual_camera::Status::Failed {
            settings,
            code: "noDevice".into(),
            detail: String::new(),
        });
        studio.with_stage(|stage, _| {
            stage.set_virtual_camera(profile::VirtualCameraSettings {
                width: 1920,
                height: 1080,
                background: "#0000ff".into(),
                ..Default::default()
            });
        });
        assert_eq!(sync_camera(&studio), Ok(()));
        assert!(matches!(
            rx.try_recv(),
            Ok(CameraCommand::Background([0, 0, 255]))
        ));
        assert_eq!(
            lock(&studio.camera).as_ref().unwrap().settings.config(),
            settings.config()
        );
    }

    #[test]
    fn stale_render_stats_are_flagged() {
        let studio = Studio::default();
        let stats = RenderStats {
            fps: 60.0,
            render_ms: 1.0,
            render_max_ms: 1.0,
            dropped: 0,
        };
        assert!(stats.is_valid());
        assert!(
            !RenderStats {
                render_ms: f32::NAN,
                ..stats
            }
            .is_valid()
        );
        let at = Instant::now();
        lock(&studio.renders).insert(
            "output".into(),
            Reported {
                stats,
                target_fps: None,
                at,
            },
        );
        assert!(!studio.renderer_loads(at + Duration::from_secs(1))[0].stale);
        assert!(studio.renderer_loads(at + Duration::from_secs(3))[0].stale);
    }
}
