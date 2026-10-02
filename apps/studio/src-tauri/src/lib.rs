//! スタジオのバックエンド (`apps/studio/README.md`、ADR 0028)。
//!
//! - 指揮役 ([`conductor::Conductor`]) を刻みのスレッドで回し、刻みごとのフレームを出力ウィンドウへ Tauri の `Channel` で配る。
//! - 操作ウィンドウへは、シーン (パラメータの定義) と、間引いた値と負荷 (毎秒 10 回) を配る。
//! - 出力ウィンドウ (枠なし・背景の透過) の表示・最前面・大きさを操作する。
//! - 刻みの速さ (30 / 60 / 120) を変え、実際の速さと、出力ウィンドウから届く描画の負荷を操作ウィンドウに配る (T0804)。
//!
//! 描画はしない (出力ウィンドウの WebView の wasm が受け手として描く)。

pub mod conductor;
pub mod follower;
pub mod pacing;
pub mod stream;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{AppHandle, Manager, PhysicalSize, WebviewWindow, WindowEvent};

use crate::conductor::Conductor;
use crate::pacing::{Measured, Pacer, RateMeter, TickRate};
use crate::stream::{Frame, Scene};

/// 操作ウィンドウのラベル (tauri.conf.json)。
const CONTROL: &str = "main";
/// 出力ウィンドウのラベル (tauri.conf.json)。
const OUTPUT: &str = "output";
/// 出力ウィンドウからの描画の負荷の報告が、これより古ければ「止まっている」とみなす (隠れて描いていないなど)。
const RENDER_STATS_STALE: Duration = Duration::from_secs(2);
/// 読み込むファイルの大きさの上限 (誤って巨大なファイルを選んだときにメモリを使い果たさないため)。
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

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
    #[serde(rename = "scene")]
    Scene {
        scene: &'a Scene,
        parameters: Vec<ParameterInfo>,
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
}

/// 今のシーンのモデル (受け手が `scene_model` で取る)。
#[derive(Debug)]
struct SceneModel {
    revision: u32,
    bytes: Arc<Vec<u8>>,
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
    conductor: Mutex<Conductor>,
    model: Mutex<Option<SceneModel>>,
    /// フレームを受ける出力。
    outputs: Subscribers,
    /// 値と負荷を受ける操作ウィンドウ。
    monitors: Subscribers,
    /// 直近の 1 刻みにかかった時間 (ナノ秒)。
    tick_nanos: AtomicU64,
    /// 刻みの速さ (回/秒。0 は既定)。刻みのスレッドが刻みごとに読む。
    tick_rate: AtomicU32,
    /// 実際の刻みの速さと揺れ (刻みのスレッドが書く)。
    measured: Mutex<Option<Measured>>,
    /// 出力ごとの描画の負荷と、報告を受けた時刻。
    renders: Mutex<HashMap<String, (RenderStats, Instant)>>,
}

impl Studio {
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
        Self::json(&Monitor::Scene { scene, parameters })
    }

    fn tick_rate(&self) -> TickRate {
        TickRate::from_hz(self.tick_rate.load(Ordering::Relaxed)).unwrap_or(TickRate::DEFAULT)
    }

    /// 出力ごとの描画の負荷。報告の古いものは `stale` にする。
    fn renderer_loads(&self, now: Instant) -> Vec<RendererLoad> {
        let mut loads: Vec<_> = lock(&self.renders)
            .iter()
            .map(|(label, (stats, at))| RendererLoad {
                label: label.clone(),
                stats: *stats,
                stale: now.saturating_duration_since(*at) > RENDER_STATS_STALE,
            })
            .collect();
        loads.sort_by(|a, b| a.label.cmp(&b.label));
        loads
    }

    /// 1 刻み進めてフレームを配る。`status` なら操作ウィンドウにも値を送る。
    fn tick(&self, dt: f32, status: bool) {
        let started = Instant::now();
        let mut conductor = lock(&self.conductor);
        let Some(frame) = conductor.tick(dt) else {
            return;
        };
        self.outputs
            .send_all(&InvokeResponseBody::Raw(frame.encode()));
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

    fn send_output_state(&self, state: OutputState) {
        if let Some(body) = Self::json(&Monitor::Output(state)) {
            self.monitors.send_all(&body);
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

/// モデルのファイルか (拡張子で判断する)。`.ugoe` バンドルと JSON のモデル。
fn is_model_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ugoe") || e.eq_ignore_ascii_case("json"))
}

fn read_model_bytes(path: &Path) -> Result<Vec<u8>, String> {
    if !is_model_path(path) {
        return Err(format!(
            "not a model file (.ugoe or .json): {}",
            path.display()
        ));
    }
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if len > MAX_FILE_BYTES {
        return Err(format!(
            "{}: file is too large ({len} bytes)",
            path.display()
        ));
    }
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// モデルを開いて新しいシーンにし、受け手と操作ウィンドウにシーンを配る。
#[tauri::command]
fn open_model(studio: tauri::State<'_, Arc<Studio>>, path: String) -> Result<(), String> {
    let path = Path::new(&path);
    let bytes = read_model_bytes(path)?;
    let bundle = conductor::read_model(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut conductor = lock(&studio.conductor);
    let revision = conductor
        .load(&name, bundle)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .revision;
    *lock(&studio.model) = Some(SceneModel {
        revision,
        bytes: Arc::new(bytes),
    });
    // 次の刻みのフレームより先に届くよう、指揮役を握ったまま配る。
    if let Some(body) = Studio::scene_message(&conductor) {
        studio.outputs.send_all(&body);
    }
    if let Some(body) = Studio::monitor_scene_message(&conductor) {
        studio.monitors.send_all(&body);
    }
    Ok(())
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

/// シーン `revision` のモデルのバイト列。今のシーンでなければエラー (受け手は次のシーンを待つ)。
#[tauri::command]
fn scene_model(studio: tauri::State<'_, Arc<Studio>>, revision: u32) -> Result<Response, String> {
    match &*lock(&studio.model) {
        Some(model) if model.revision == revision => Ok(Response::new(model.bytes.to_vec())),
        _ => Err(format!("scene {revision} is not the current scene")),
    }
}

/// 操作ウィンドウが値と負荷を受け取り始める。シーンがあれば最初に送る。
#[tauri::command]
fn subscribe_status(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    channel: Channel<InvokeResponseBody>,
) {
    let conductor = lock(&studio.conductor);
    if let Some(body) = Studio::monitor_scene_message(&conductor) {
        let _ = channel.send(body);
    }
    studio.monitors.insert(webview.label(), channel);
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

/// 刻みの速さを 30 / 60 / 120 から選ぶ (次の刻みから)。
#[tauri::command]
fn set_tick_rate(studio: tauri::State<'_, Arc<Studio>>, rate: u32) -> Result<u32, String> {
    let rate = TickRate::from_hz(rate).ok_or_else(|| format!("unsupported tick rate: {rate}"))?;
    studio.tick_rate.store(rate.hz(), Ordering::Relaxed);
    Ok(rate.hz())
}

/// 出力が描画の負荷を報告する (操作ウィンドウに出す。出力ウィンドウには何も重ねない)。
#[tauri::command]
fn report_render_stats(
    studio: tauri::State<'_, Arc<Studio>>,
    webview: tauri::Webview,
    stats: RenderStats,
) -> Result<(), String> {
    if !stats.is_valid() {
        return Err("invalid render stats".to_owned());
    }
    lock(&studio.renders).insert(webview.label().to_owned(), (stats, Instant::now()));
    Ok(())
}

fn output_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(OUTPUT)
        .ok_or_else(|| "the output window is missing".to_owned())
}

fn read_output_state(window: &WebviewWindow) -> Result<OutputState, String> {
    let size = window.inner_size().map_err(|e| e.to_string())?;
    Ok(OutputState {
        visible: window.is_visible().map_err(|e| e.to_string())?,
        on_top: window.is_always_on_top().map_err(|e| e.to_string())?,
        width: size.width,
        height: size.height,
    })
}

/// 出力ウィンドウの状態。
#[tauri::command]
fn output_state(app: AppHandle) -> Result<OutputState, String> {
    read_output_state(&output_window(&app)?)
}

/// 出力ウィンドウを表示する・隠す (隠しても受け手は動き続ける)。
#[tauri::command]
fn set_output_visible(app: AppHandle, visible: bool) -> Result<OutputState, String> {
    let window = output_window(&app)?;
    if visible {
        window.show().map_err(|e| e.to_string())?;
    } else {
        window.hide().map_err(|e| e.to_string())?;
    }
    read_output_state(&window)
}

/// 出力ウィンドウを最前面にする・戻す。
#[tauri::command]
fn set_output_on_top(app: AppHandle, on_top: bool) -> Result<OutputState, String> {
    let window = output_window(&app)?;
    window
        .set_always_on_top(on_top)
        .map_err(|e| e.to_string())?;
    read_output_state(&window)
}

/// 出力ウィンドウの中身の大きさを物理ピクセルで決める (配信ソフトが取り込む大きさ。1920×1080 など)。
#[tauri::command]
fn set_output_size(app: AppHandle, width: u32, height: u32) -> Result<OutputState, String> {
    if !(16..=8192).contains(&width) || !(16..=8192).contains(&height) {
        return Err(format!("unsupported output size: {width}x{height}"));
    }
    let window = output_window(&app)?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    read_output_state(&window)
}

pub fn run() {
    let studio = Arc::new(Studio::default());
    let ticking = Arc::clone(&studio);
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .manage(studio)
        .setup(move |_| {
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
            // 操作ウィンドウを閉じたらアプリを終える (隠れた出力ウィンドウを残さない)。
            (CONTROL, WindowEvent::Destroyed) => window.app_handle().exit(0),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            open_model,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ugoe studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bundles_and_json_only() {
        assert!(is_model_path(Path::new("a/b/model.UGOE")));
        assert!(is_model_path(Path::new("model.json")));
        assert!(!is_model_path(Path::new("hair.ugoemod")));
        assert!(!is_model_path(Path::new("model")));
        assert!(read_model_bytes(Path::new("profile.toml")).is_err());
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
            }],
        };
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            concat!(
                r#"{"kind":"status","seq":3,"values":[0.5],"tickMs":0.25,"outputs":1,"tickRate":60,"#,
                r#""measuredRate":59.5,"maxIntervalMs":null,"renderers":[{"label":"output","fps":60.0,"#,
                r#""renderMs":1.5,"renderMaxMs":2.0,"dropped":0,"stale":false}]}"#
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

    #[test]
    fn tick_rate_defaults_to_60_and_accepts_the_presets_only() {
        let studio = Studio::default();
        assert_eq!(studio.tick_rate(), TickRate::Fps60);
        studio.tick_rate.store(120, Ordering::Relaxed);
        assert_eq!(studio.tick_rate(), TickRate::Fps120);
        studio.tick_rate.store(75, Ordering::Relaxed);
        assert_eq!(studio.tick_rate(), TickRate::DEFAULT);
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
        lock(&studio.renders).insert("output".into(), (stats, at));
        assert!(!studio.renderer_loads(at + Duration::from_secs(1))[0].stale);
        assert!(studio.renderer_loads(at + Duration::from_secs(3))[0].stale);
    }
}
