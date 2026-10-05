//! JS へ公開する [`WebPuppet`]。ブラウザの `<canvas>` に wgpu のサーフェスを作って描画する。
//!
//! - WebGPU が使えればそれを、使えなければ WebGL2 を使う (`docs/adr/0003-editor-viewport-in-webview.md`)。
//! - クリアはホスト (ここ) の責務。レンダラは上に重ねるだけ (`docs/spec/rendering.md`)。
//! - 画面座標は canvas の描画バッファの画素 (CSS ピクセル × `devicePixelRatio`)。
//! - 物理・モーション・表情・エフェクトの操作は [`animation`] に分けてある。

mod animation;
mod physics;

use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ugoe::Puppet;
use ugoe::core::Vec2;
use ugoe::render::{RenderError, Renderer};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::animation::Library;
use crate::dress_up::{Wardrobe, WardrobeError, attached_names};
use crate::frame::FrameError;
use crate::load::{self, ErrorInfo, LoadError, LoadWarning, LoadedModel, Rgba8};
use crate::view::{Transform, ViewState};
use crate::{gizmo, hit, view};

// フェイストラッキングの API (T0703)。別のファイルに分けて、ほかの API の追加と衝突しにくくする。
mod tracking;
// ライティングの API (T0210)。
mod lighting;
// 着せ替えの API (T0123)。
mod dress_up;
// 描画結果の読み戻しと評価の結果の取り出し (T0418)。
mod readback;
// 受け手の口: 届いた値を基準値として書き、物理だけを進める (T0133)。
mod follow;

fn js_error(e: impl Display) -> JsError {
    JsError::new(&e.to_string())
}

/// `code` と補足の値をプロパティに持つ JS の `Error` (`name` は `name`)。補足の値の数は数、ほかは文字列にする。
fn coded_error(name: &str, message: &str, info: ErrorInfo) -> JsValue {
    let js = js_sys::Error::new(message);
    js.set_name(name);
    // 普通のオブジェクトへの設定なので失敗しない。
    let _ = js_sys::Reflect::set(&js, &"code".into(), &info.code.into());
    for (key, value) in info.fields {
        let value = match value {
            serde_json::Value::Number(n) => JsValue::from_f64(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(s) => JsValue::from_str(&s),
            other => JsValue::from_str(&other.to_string()),
        };
        let _ = js_sys::Reflect::set(&js, &key.into(), &value);
    }
    js.into()
}

/// 読み込みの失敗を JS の値にする: `name` が `"LoadError"` で、`code` と補足の値 ([`LoadError::info`]) を持つ `Error`。
fn load_error(e: LoadError) -> JsValue {
    coded_error("LoadError", &e.to_string(), e.info())
}

/// 描画・読み戻しの失敗を JS の値にする: `name` が `"RenderError"` で、`code` ([`FrameError::info`]) を持つ `Error` (T0465)。
fn frame_error(e: FrameError) -> JsValue {
    coded_error(FrameError::NAME, &e.to_string(), e.info())
}

/// `limits_json` を読む (`load_bundle` / `load_module`)。形の誤りは例外。
fn load_limits(json: Option<String>) -> Result<load::LoadLimits, JsError> {
    load::LoadLimits::from_json(json.as_deref())
        .map_err(|e| JsError::new(&format!("invalid limits: {e}")))
}

/// モデル 1 体と、それを描く canvas。
///
/// パラメータの設定 (`set_parameter`) は評価を遅らせ、次の `render` や問い合わせでまとめて評価する。
/// 描画は呼ばれたときだけ行う (ループはホストが `requestAnimationFrame` で回す)。
#[wasm_bindgen]
#[derive(Debug)]
pub struct WebPuppet {
    canvas: HtmlCanvasElement,
    // サーフェスより長く生かしておく。
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// 描画に使うビューの形式 (リニア化しないため sRGB ではない形式)。
    view_format: wgpu::TextureFormat,
    backend: wgpu::Backend,
    renderer: Renderer,
    puppet: Option<Puppet>,
    /// バンドルのモーションと表情 (名前で再生する)。
    library: Library,
    view: ViewState,
    /// 乗算済みアルファの背景色。
    background: wgpu::Color,
    /// パラメータが変わり、評価し直す必要がある。
    dirty: bool,
    /// デバイスを失った (GPU プロセスのクラッシュ・WebGL2 のコンテキストの喪失など)。ホストは作り直す必要がある。
    lost: Arc<AtomicBool>,
    /// WebGL2 で作ったときの、canvas の `webglcontextlost` のリスナー (`lost` を立てる。破棄のときに外す。T0932)。
    context_lost: Option<Closure<dyn FnMut()>>,
    /// 読み込んだ着せ替えのモジュールと、レンダラに登録したテクスチャの番号の範囲
    /// (モデルを読み直したときや付け外しのときに、使わなくなった画像を捨てるため)。
    wardrobe: Wardrobe,
    /// 直前の `load_json` / `load_bundle` の警告のうち、外したモジュール (`wardrobe.detached()`) 以外のもの (T0125)。
    load_warnings: Vec<LoadWarning>,
    /// トラッキング中のモーションの優先 (`set_motion_over_tracking`。T0721)。読み直した `Puppet` に渡し直す。
    motion_over_tracking: bool,
    /// モーションを優先しても、トラッキングのままにするパラメータの ID (`set_tracking_kept`。T0722)。
    tracking_kept: Vec<String>,
    /// 使っている表情の名前 (`active_expressions`。T0721)。
    active_expressions: crate::animation::ActiveExpressions,
}

#[wasm_bindgen]
impl WebPuppet {
    /// `canvas` に描画する `WebPuppet` を作る。
    ///
    /// `backend` は `"auto"` (既定: WebGPU、無ければ WebGL2)・`"webgpu"`・`"webgl2"`。
    /// 1 つの canvas は WebGPU と WebGL2 のどちらか一方にしか使えないので、失敗したら別の canvas で作り直すこと。
    pub async fn create(
        canvas: HtmlCanvasElement,
        backend: Option<String>,
    ) -> Result<WebPuppet, JsError> {
        console_error_panic_hook::set_once();
        let mut backends = match backend.as_deref().unwrap_or("auto") {
            "auto" => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
            "webgl2" => wgpu::Backends::GL,
            other => {
                return Err(JsError::new(&format!(
                    "unknown backend `{other}` (expected auto, webgpu or webgl2)"
                )));
            }
        };
        // navigator.gpu があってもアダプタを作れないブラウザがあるので、実際に試して判定する。
        if backends.contains(wgpu::Backends::BROWSER_WEBGPU)
            && !wgpu::util::is_browser_webgpu_supported().await
        {
            backends.remove(wgpu::Backends::BROWSER_WEBGPU);
        }
        if backends.is_empty() {
            return Err(JsError::new("WebGPU is not available in this browser"));
        }
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        // レンダラは WebGL2 で使える機能だけを使う。テクスチャの大きさだけはアダプタの上限まで使う。
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ugoe web"),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        // canvas がデバイスの扱える大きさを超えていれば、`resize` と同じく上限に収める
        // (超えたままではサーフェスを作れない)。
        let (width, height) = view::surface_size(
            canvas.width(),
            canvas.height(),
            device.limits().max_texture_dimension_2d,
        );
        if (width, height) != (canvas.width(), canvas.height()) {
            canvas.set_width(width);
            canvas.set_height(height);
        }
        // 既定の処理はパニックするので、コンソールに出すだけにする。
        device.on_uncaptured_error(Arc::new(|e| {
            web_sys::console::error_1(&format!("ugoe: GPU error: {e}").into());
        }));
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            flag.store(true, Ordering::Relaxed);
            web_sys::console::error_1(
                &format!("ugoe: GPU device lost ({reason:?}): {message}").into(),
            );
        });

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| JsError::new("the canvas surface supports no texture format"))?;
        let view_format = format.remove_srgb_suffix();
        // 乗算済みアルファで合成できれば、透明な背景でページの上に重ねられる。
        // WebGPU の canvas は常に `premultiplied` に対応するが、wgpu は対応を `Opaque` だけと報告するので
        // (wgpu 30 の backend/webgpu.rs)、WebGPU では報告によらず乗算済みにする (しないと背景が黒になる。T0404)。
        let alpha_mode = if adapter.get_info().backend == wgpu::Backend::BrowserWebGpu
            || caps
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            wgpu::CompositeAlphaMode::Auto
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: if view_format == format {
                Vec::new()
            } else {
                vec![view_format]
            },
        };
        surface.configure(&device, &config);
        let renderer = Renderer::new(&device, view_format);
        let backend = adapter.get_info().backend;
        // wgpu の GL のバックエンド (wgpu-hal 30) は WebGL2 のコンテキストの喪失をデバイスの喪失として知らせず、
        // 描画も失敗しない (何も描かれない) ので、canvas のイベントで `lost` を立てる (T0932)。
        let context_lost = (backend == wgpu::Backend::Gl)
            .then(|| listen_context_lost(&canvas, lost.clone()))
            .transpose()?;

        Ok(Self {
            canvas,
            _instance: instance,
            surface,
            backend,
            device,
            queue,
            config,
            view_format,
            renderer,
            puppet: None,
            library: Library::default(),
            view: ViewState::default(),
            background: wgpu::Color::TRANSPARENT,
            dirty: false,
            lost,
            context_lost,
            wardrobe: Wardrobe::default(),
            load_warnings: Vec::new(),
            motion_over_tracking: false,
            tracking_kept: Vec::new(),
            active_expressions: Default::default(),
        })
    }

    /// デバイスを失ったか。`true` なら以後の描画は無効 (`render` は例外を投げる) なので、新しい canvas で作り直すこと。
    ///
    /// WebGPU はデバイスの喪失 (GPU プロセスのクラッシュなど)、WebGL2 は canvas の `webglcontextlost`
    /// (GPU のリセット・ドライバの更新・タブを裏に回したときの回収など) で `true` になる (T0932)。
    /// どちらも知らせのイベントは出さないので、ホストは描画のたびか毎フレーム確かめる。
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }

    /// 使っているバックエンド (`"webgpu"` または `"webgl2"`)。
    pub fn backend(&self) -> String {
        match self.backend {
            wgpu::Backend::BrowserWebGpu => "webgpu".to_owned(),
            wgpu::Backend::Gl => "webgl2".to_owned(),
            other => other.to_string(),
        }
    }

    // --- モデル ---

    /// JSON のモデルを読む。テクスチャは白のプレースホルダになる。戻り値は警告の一覧。
    ///
    /// すでにモデルがあれば置き換え、同じ ID のパラメータの値は引き継ぐ。
    /// 物理の設定・モーションと表情の一覧・段の有効無効も引き継ぐ (新しいモデルと合わない物理は外して警告する)。
    /// 付けていた着せ替えのモジュールは付け直す (付けられないものは外して警告する)。
    ///
    /// 失敗は `name` が `"LoadError"` の `Error` で、`code` と補足の値を持つ (README の「読み込みの失敗」)。何も変えない。
    pub fn load_json(&mut self, json: &str) -> Result<Vec<String>, JsValue> {
        load::from_json(json)
            .and_then(|loaded| self.load(loaded))
            .map_err(load_error)
    }

    /// `.ugoe` バンドルを読む (PNG テクスチャをデコードする)。戻り値は警告の一覧。
    ///
    /// すでにモデルがあれば置き換え、同じ ID のパラメータの値と段の有効無効は引き継ぐ。
    /// 物理・モーション・表情はバンドルのものに置き換える (物理は `validate_physics` で検査済み)。
    /// 付けていた着せ替えのモジュールは付け直す (付けられないものは外して警告する)。
    ///
    /// `limits_json` は読み込みの上限 `{"max_chunk_decompressed"?, "max_total_decompressed"?, "max_image_side"?}`
    /// (省くと既定値。T0152)。超えたら `code` が `"read_limit_exceeded"` か `"image_limit_exceeded"` の失敗
    /// (補足の値は README の「読み込みの上限」)。
    ///
    /// 失敗は `name` が `"LoadError"` の `Error` で、`code` と補足の値を持つ (README の「読み込みの失敗」)。何も変えない。
    pub fn load_bundle(
        &mut self,
        bytes: &[u8],
        limits_json: Option<String>,
    ) -> Result<Vec<String>, JsValue> {
        let limits = load_limits(limits_json)?;
        load::from_bundle_with(bytes, limits)
            .and_then(|loaded| self.load(loaded))
            .map_err(load_error)
    }

    /// 直前の `load_json` / `load_bundle` の警告の JSON (T0125)。戻り値の文の警告と同じ順・同じ数で、
    /// 各要素は `{"code", "warning", ...}`。`warning` は戻り値の文と同じ文字列、`code` は
    /// `"texture_ktx2"` | `"texture_missing"` | `"normal_map_ktx2"` (補足は `texture`: テクスチャの番号)、
    /// `"physics_removed"` (`reason`: 英語の理由)、`"module_detached"` (`detached_modules_json` の要素と同じ項目)。
    /// 一覧は次の読み込みが成功するまで変わらない (警告が無ければ `[]`)。
    pub fn load_warnings_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&load::warnings_json(
            &self.load_warnings,
            self.wardrobe.detached(),
        ))
        .map_err(|e| JsError::new(&e.to_string()))
    }

    /// モデルを外す (背景だけを描くようになる)。モーションと表情の一覧も空にする。
    pub fn unload(&mut self) {
        self.puppet = None;
        self.library = Library::default();
        self.active_expressions.clear();
    }

    /// モデルを読み込んでいるか。
    pub fn has_model(&self) -> bool {
        self.puppet.is_some()
    }

    /// モデルのキャンバスの大きさ `[width, height]` (キャンバス座標)。モデルが無ければ空。
    pub fn canvas_size(&self) -> Vec<f32> {
        self.canvas_extent().map(Vec::from).unwrap_or_default()
    }

    // --- パラメータ ---

    /// パラメータの ID (モデルの定義順)。
    pub fn parameter_ids(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(|p| {
                let model = p.instance().compiled().model();
                model.parameters.iter().map(|p| p.id.to_string()).collect()
            })
            .unwrap_or_default()
    }

    /// パラメータとグループの定義の JSON: `{ "parameters": [...], "groups": [...] }`。
    /// 各要素はモデルの JSON 形式と同じ (id・name・min・max・default・kind・mirror・group)。
    /// モデルが無ければ両方とも空。
    pub fn parameters_json(&self) -> Result<String, JsError> {
        match &self.puppet {
            Some(p) => load::parameters_json(p.instance().compiled().model()).map_err(js_error),
            None => Ok(r#"{"parameters":[],"groups":[]}"#.to_owned()),
        }
    }

    /// パラメータを設定する (値域にクランプされる。NaN は書かない)。ID が無ければ `false`。
    pub fn set_parameter(&mut self, id: &str, value: f32) -> bool {
        let Some(puppet) = &mut self.puppet else {
            return false;
        };
        if !load::set_parameter(puppet.instance_mut(), id, value) {
            return false;
        }
        self.dirty = true;
        true
    }

    /// パラメータの現在値。ID が無ければ `undefined`。
    pub fn parameter(&self, id: &str) -> Option<f32> {
        let params = self.puppet.as_ref()?.instance().parameters();
        params.index_of(id).map(|i| params.get(i))
    }

    /// すべてのパラメータを既定値に戻す。
    pub fn reset_parameters(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.instance_mut().parameters_mut().reset();
            self.dirty = true;
        }
    }

    /// 物理・モーションを `dt` 秒進めて評価する。静止画の表示だけなら呼ばなくてよい。
    /// 負・NaN・無限大の `dt` は 0 とみなす (`Puppet::update`)。
    pub fn update(&mut self, dt: f32) {
        if let Some(puppet) = &mut self.puppet {
            puppet.update(dt);
            self.dirty = false;
        }
    }

    // --- 描画 ---

    /// 描画先の大きさ (画素) を変える。canvas 要素の `width` / `height` も書き換える。
    pub fn resize(&mut self, width: u32, height: u32) {
        let max = self.device.limits().max_texture_dimension_2d;
        let (width, height) = view::surface_size(width, height, max);
        if (width, height) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// 描画先の幅 (画素)。
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// 描画先の高さ (画素)。
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// 背景色 (ストレートアルファ、各成分 0..1。範囲の外はクランプする)。既定は透明。NaN を含めば何も変えない。
    pub fn set_background(&mut self, r: f64, g: f64, b: f64, a: f64) {
        if let Some(color) = view::background_color(r, g, b, a) {
            self.background = color;
        }
    }

    /// 現在の状態を canvas に描く。GPU を失った (`is_lost()`) 後は、`code` が `device_lost` の `RenderError` を投げる (T0465)。
    pub fn render(&mut self) -> Result<(), JsValue> {
        if self.is_lost() {
            return Err(frame_error(FrameError::DeviceLost));
        }
        self.evaluate();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                // 作り直して次のフレームで描く。
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(JsError::new("failed to acquire the canvas texture").into());
            }
        };
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.view_format),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ugoe web frame"),
            });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let result = match (&self.puppet, self.transform()) {
            (Some(puppet), Some(transform)) => self.renderer.encode(
                &self.device,
                &self.queue,
                &mut encoder,
                &target,
                &transform.to_view([self.config.width, self.config.height]),
                puppet.instance(),
            ),
            _ => Ok(()),
        };
        // 描画に失敗しても、クリアした画面は出す。
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        result.map_err(|e| js_error(e).into())
    }

    // --- ビュー (docs/editor.md#ビューポート) ---

    /// キャンバス全体が収まるように戻す (ズーム 1・パン 0)。
    pub fn fit(&mut self) {
        self.view = ViewState::default();
    }

    /// ズーム (フィットに対する倍率) とパン (フィットの位置からのずれ、画面座標の画素) を設定する。
    pub fn set_view(&mut self, zoom: f32, pan_x: f32, pan_y: f32) {
        if zoom.is_finite() && pan_x.is_finite() && pan_y.is_finite() {
            self.view = ViewState {
                zoom: zoom.clamp(view::MIN_ZOOM, view::MAX_ZOOM),
                pan: [pan_x, pan_y],
            };
        }
    }

    /// ズーム (フィットに対する倍率)。
    pub fn zoom(&self) -> f32 {
        self.view.zoom
    }

    /// パンの x (フィットの位置からのずれ、画面座標の画素)。
    pub fn pan_x(&self) -> f32 {
        self.view.pan[0]
    }

    /// パンの y (フィットの位置からのずれ、画面座標の画素)。
    pub fn pan_y(&self) -> f32 {
        self.view.pan[1]
    }

    /// 画面座標 `(x, y)` を中心にズームを `factor` 倍する。`factor` が正の有限値でないか、位置が有限でなければ何も変えない。
    pub fn zoom_at(&mut self, factor: f32, x: f32, y: f32) {
        if let Some(canvas) = self.canvas_extent() {
            let surface = [self.config.width, self.config.height];
            self.view.zoom_at(factor, [x, y], canvas, surface);
        }
    }

    /// 画面座標で `(dx, dy)` 画素だけずらす。
    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        if dx.is_finite() && dy.is_finite() {
            self.view.pan[0] += dx;
            self.view.pan[1] += dy;
        }
    }

    /// キャンバス座標 → 画面座標の変換 `[scale, offset_x, offset_y]` (`screen = offset + scale * canvas`)。
    /// ギズモなどのオーバーレイを同じ変換で描くために使う。モデルが無ければ空。
    pub fn view_transform(&self) -> Vec<f32> {
        self.transform()
            .map(|t| vec![t.scale, t.offset[0], t.offset[1]])
            .unwrap_or_default()
    }

    /// キャンバス座標 → 画面座標 `[x, y]`。モデルが無ければ空。
    pub fn canvas_to_screen(&self, x: f32, y: f32) -> Vec<f32> {
        self.transform()
            .map(|t| Vec::from(t.canvas_to_screen([x, y])))
            .unwrap_or_default()
    }

    /// 画面座標 → キャンバス座標 `[x, y]`。モデルが無ければ空。
    pub fn screen_to_canvas(&self, x: f32, y: f32) -> Vec<f32> {
        self.transform()
            .map(|t| Vec::from(t.screen_to_canvas([x, y])))
            .unwrap_or_default()
    }

    // --- 評価結果の問い合わせ (ヒットテスト・ギズモ用、座標はキャンバス座標) ---

    /// 描画可能要素 (メッシュ) の ID。以下の添字はこの順。
    pub fn drawable_ids(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(|p| {
                let model = p.instance().compiled().model();
                model.meshes.iter().map(|m| m.id.to_string()).collect()
            })
            .unwrap_or_default()
    }

    /// 要素ごとの外接矩形 `[min_x, min_y, max_x, max_y, ...]`。
    pub fn drawable_bounds(&mut self) -> Vec<f32> {
        self.evaluate();
        self.puppet
            .as_ref()
            .map(|p| hit::drawable_bounds(p.instance()))
            .unwrap_or_default()
    }

    /// 要素 `index` の評価済みの頂点位置 `[x0, y0, x1, y1, ...]`。
    pub fn drawable_positions(&mut self, index: u32) -> Option<Vec<f32>> {
        self.evaluate();
        let d = self
            .puppet
            .as_ref()?
            .instance()
            .drawables()
            .get(index as usize)?;
        Some(d.positions.iter().flat_map(|p| [p.x, p.y]).collect())
    }

    /// 描画順 (奥から手前へ、要素の添字)。
    pub fn render_order(&mut self) -> Vec<u32> {
        self.evaluate();
        self.puppet
            .as_ref()
            .map(|p| p.instance().render_order().to_vec())
            .unwrap_or_default()
    }

    /// キャンバス座標 `(x, y)` にある、いちばん手前の表示中の要素の添字。
    pub fn hit_test(&mut self, x: f32, y: f32) -> Option<u32> {
        self.evaluate();
        hit::hit_test(self.puppet.as_ref()?.instance(), [x, y])
    }

    /// キャンバス座標 `(x, y)` にある表示中の要素の添字のすべて (手前から奥へ)。
    pub fn hit_test_all(&mut self, x: f32, y: f32) -> Vec<u32> {
        self.evaluate();
        self.puppet
            .as_ref()
            .map(|p| hit::hit_test_all(p.instance(), [x, y]))
            .unwrap_or_default()
    }

    // --- 当たり判定 (`Model::hit_areas`、T0308、座標はキャンバス座標) ---

    /// 当たり判定の ID (モデルの定義順)。
    pub fn hit_area_ids(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(|p| {
                let model = p.instance().compiled().model();
                model.hit_areas.iter().map(|h| h.id.to_string()).collect()
            })
            .unwrap_or_default()
    }

    /// キャンバス座標 `(x, y)` を含む当たり判定の ID (手前から奥へ)。
    /// 不透明度 0 や非表示のメッシュでも当たる (`docs/spec/evaluation.md#9-当たり判定`)。
    pub fn hit_areas_at(&mut self, x: f32, y: f32) -> Vec<String> {
        self.evaluate();
        self.puppet
            .as_ref()
            .map(|p| {
                let hits = p.hit_areas_at(Vec2::new(x, y));
                hits.into_iter().map(str::to_string).collect()
            })
            .unwrap_or_default()
    }

    /// 当たり判定 `id` がキャンバス座標 `(x, y)` を含むか。未知の ID なら `false`。
    pub fn hit_area_test(&mut self, id: &str, x: f32, y: f32) -> bool {
        self.evaluate();
        self.puppet
            .as_ref()
            .is_some_and(|p| p.hit_test(id, Vec2::new(x, y)))
    }

    // --- デフォーマのギズモ (T0607、座標はキャンバス座標) ---

    /// ワープと回転デフォーマの ID (モデルの定義順)。
    pub fn deformer_ids(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(|p| gizmo::ids(p.instance().compiled().model()))
            .unwrap_or_default()
    }

    /// ワープ `id` の今のパラメータ値での制御点 `[x0, y0, ...]` (行優先、x が速い)。
    /// ワープでなければ `undefined`。
    pub fn warp_points(&mut self, id: &str) -> Option<Vec<f32>> {
        self.evaluate();
        gizmo::warp_points(self.puppet.as_ref()?.instance(), id)
    }

    /// 回転デフォーマ `id` の今のパラメータ値でのワールド変換 `[ox, oy, ax, ay, bx, by]`
    /// (原点と、局所座標の x 軸・y 軸の単位ベクトルの像)。回転デフォーマでなければ `undefined`。
    pub fn rotation_frame(&mut self, id: &str) -> Option<Vec<f32>> {
        self.evaluate();
        gizmo::rotation_frame(self.puppet.as_ref()?.instance(), id)
    }
}

impl WebPuppet {
    fn load(&mut self, loaded: LoadedModel) -> Result<Vec<String>, LoadError> {
        if self.is_lost() {
            return Err(LoadError::DeviceLost);
        }
        let LoadedModel {
            model,
            animation,
            textures,
            normal_maps,
            warnings: mut load_warnings,
        } = loaded;
        self.check_texture_sizes(&textures, &normal_maps)?;
        let mut puppet = Puppet::new(model)?;
        let library = match animation {
            Some(animation) => {
                if let Some(physics) = animation.physics {
                    puppet = puppet.with_physics(physics);
                }
                Some(animation.library)
            }
            None => None,
        };
        if let Some(old) = &self.puppet {
            load_warnings.extend(crate::animation::carry_over(
                old,
                &mut puppet,
                library.is_none(),
            ));
            load::carry_over_parameters(old.instance(), puppet.instance_mut());
        }
        let attached: Vec<String> = attached_names(self.puppet.as_ref())
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut sink = dress_up::RendererSink {
            renderer: &mut self.renderer,
            device: &self.device,
            queue: &self.queue,
        };
        let mut warnings: Vec<String> = load_warnings.iter().map(ToString::to_string).collect();
        warnings.extend(
            self.wardrobe
                .load_base(&mut puppet, &attached, &textures, &normal_maps, &mut sink)
                .map_err(|e| match e {
                    WardrobeError::Texture(message) => LoadError::Register(message),
                    e => LoadError::Register(e.to_string()),
                })?,
        );
        puppet.set_motion_over_tracking(self.motion_over_tracking);
        puppet.set_tracking_kept(self.tracking_kept.iter().cloned());
        self.puppet = Some(puppet);
        self.load_warnings = load_warnings;
        // 再生の状態は読み直すと初めからになる。
        self.active_expressions.clear();
        if let Some(library) = library {
            self.library = library;
        }
        self.dirty = false;
        Ok(warnings)
    }

    /// テクスチャと法線マップを、何も変える前にレンダラが受け付けるか (デバイスの扱える大きさに収まるかなど)。
    fn check_texture_sizes(
        &self,
        textures: &[Rgba8],
        normal_maps: &[Option<Rgba8>],
    ) -> Result<(), RenderError> {
        for (i, t) in textures.iter().enumerate() {
            self.renderer.check_texture(i as u32, t.width, t.height)?;
        }
        for (i, t) in normal_maps.iter().enumerate() {
            if let Some(t) = t {
                self.renderer
                    .check_normal_map(i as u32, t.width, t.height)?;
            }
        }
        Ok(())
    }

    fn evaluate(&mut self) {
        if self.dirty {
            if let Some(puppet) = &mut self.puppet {
                puppet.instance_mut().update();
            }
            self.dirty = false;
        }
    }

    fn canvas_extent(&self) -> Option<[f32; 2]> {
        let canvas = &self
            .puppet
            .as_ref()?
            .instance()
            .compiled()
            .model()
            .meta
            .canvas;
        Some([canvas.width, canvas.height])
    }

    fn transform(&self) -> Option<Transform> {
        let canvas = self.canvas_extent()?;
        Some(
            self.view
                .transform(canvas, [self.config.width, self.config.height]),
        )
    }
}

impl Drop for WebPuppet {
    /// `free` で canvas の `webglcontextlost` のリスナーを外す (canvas を使い回しても古い `WebPuppet` の印を立てない)。
    fn drop(&mut self) {
        if let Some(listener) = self.context_lost.take() {
            // 外せなくても、印を立てるだけのリスナーなので害はない。
            let _ = self.canvas.remove_event_listener_with_callback(
                "webglcontextlost",
                listener.as_ref().unchecked_ref(),
            );
        }
    }
}

/// canvas の `webglcontextlost` で `lost` を立てるリスナーを付ける。既定の動作 (復元しない) は止めない
/// (作り直しは新しい canvas で行うので、復元を待たない)。
fn listen_context_lost(
    canvas: &HtmlCanvasElement,
    lost: Arc<AtomicBool>,
) -> Result<Closure<dyn FnMut()>, JsError> {
    let listener = Closure::<dyn FnMut()>::new(move || {
        if !lost.swap(true, Ordering::Relaxed) {
            web_sys::console::error_1(&"ugoe: WebGL context lost".into());
        }
    });
    canvas
        .add_event_listener_with_callback("webglcontextlost", listener.as_ref().unchecked_ref())
        .map_err(|e| JsError::new(&format!("failed to listen for webglcontextlost: {e:?}")))?;
    Ok(listener)
}
