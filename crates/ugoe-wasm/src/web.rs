//! JS へ公開する [`WebPuppet`]。ブラウザの `<canvas>` に wgpu のサーフェスを作って描画する。
//!
//! - WebGPU が使えればそれを、使えなければ WebGL2 を使う (`docs/adr/0003-editor-viewport-in-webview.md`)。
//! - クリアはホスト (ここ) の責務。レンダラは上に重ねるだけ (`docs/spec/rendering.md`)。
//! - 画面座標は canvas の描画バッファの画素 (CSS ピクセル × `devicePixelRatio`)。

use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ugoe::Puppet;
use ugoe::core::Vec2;
use ugoe::render::Renderer;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::load::{self, LoadError, LoadedModel};
use crate::view::{Transform, ViewState};
use crate::{gizmo, hit, view};

fn js_error(e: impl Display) -> JsError {
    JsError::new(&e.to_string())
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
    view: ViewState,
    /// 乗算済みアルファの背景色。
    background: wgpu::Color,
    /// パラメータが変わり、評価し直す必要がある。
    dirty: bool,
    /// デバイスを失った (GPU プロセスのクラッシュなど)。ホストは作り直す必要がある。
    lost: Arc<AtomicBool>,
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

        let (width, height) = (canvas.width().max(1), canvas.height().max(1));
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
        let alpha_mode = if caps
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

        Ok(Self {
            canvas,
            _instance: instance,
            surface,
            backend: adapter.get_info().backend,
            device,
            queue,
            config,
            view_format,
            renderer,
            puppet: None,
            view: ViewState::default(),
            background: wgpu::Color::TRANSPARENT,
            dirty: false,
            lost,
        })
    }

    /// デバイスを失ったか。`true` なら以後の描画は無効なので、新しい canvas で作り直すこと。
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
    pub fn load_json(&mut self, json: &str) -> Result<Vec<String>, JsError> {
        self.load(load::from_json(json).map_err(js_error)?)
    }

    /// `.ugoe` バンドルを読む (PNG テクスチャをデコードする)。戻り値は警告の一覧。
    ///
    /// すでにモデルがあれば置き換え、同じ ID のパラメータの値は引き継ぐ。
    pub fn load_bundle(&mut self, bytes: &[u8]) -> Result<Vec<String>, JsError> {
        self.load(load::from_bundle(bytes).map_err(js_error)?)
    }

    /// モデルを外す (背景だけを描くようになる)。
    pub fn unload(&mut self) {
        self.puppet = None;
    }

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

    /// パラメータを設定する (値域にクランプされる)。ID が無ければ `false`。
    pub fn set_parameter(&mut self, id: &str, value: f32) -> bool {
        let Some(puppet) = &mut self.puppet else {
            return false;
        };
        let params = puppet.instance_mut().parameters_mut();
        let Some(index) = params.index_of(id) else {
            return false;
        };
        params.set(index, value);
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
        let (width, height) = (width.clamp(1, max), height.clamp(1, max));
        if (width, height) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// 背景色 (ストレートアルファ、各成分 0..1)。既定は透明。
    pub fn set_background(&mut self, r: f64, g: f64, b: f64, a: f64) {
        let a = a.clamp(0.0, 1.0);
        self.background = wgpu::Color {
            r: r.clamp(0.0, 1.0) * a,
            g: g.clamp(0.0, 1.0) * a,
            b: b.clamp(0.0, 1.0) * a,
            a,
        };
    }

    /// 現在の状態を canvas に描く。
    pub fn render(&mut self) -> Result<(), JsError> {
        if self.is_lost() {
            return Err(JsError::new("the GPU device was lost"));
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
                return Err(JsError::new("failed to acquire the canvas texture"));
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
        result.map_err(js_error)
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

    pub fn zoom(&self) -> f32 {
        self.view.zoom
    }

    pub fn pan_x(&self) -> f32 {
        self.view.pan[0]
    }

    pub fn pan_y(&self) -> f32 {
        self.view.pan[1]
    }

    /// 画面座標 `(x, y)` を中心にズームを `factor` 倍する。
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
                model.hit_areas.iter().map(|h| h.id.clone()).collect()
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
    fn load(&mut self, loaded: LoadedModel) -> Result<Vec<String>, JsError> {
        if self.is_lost() {
            return Err(JsError::new("the GPU device was lost"));
        }
        let LoadedModel {
            model,
            physics,
            textures,
            warnings,
        } = loaded;
        let max = self.device.limits().max_texture_dimension_2d;
        if let Some((i, t)) = textures
            .iter()
            .enumerate()
            .find(|(_, t)| t.width > max || t.height > max)
        {
            return Err(JsError::new(&format!(
                "texture {i} is {}x{}, which exceeds the device limit {max}",
                t.width, t.height
            )));
        }
        let mut puppet = Puppet::new(model)
            .map_err(LoadError::from)
            .map_err(js_error)?;
        if let Some(physics) = physics {
            puppet = puppet.with_physics(physics);
        }
        if let Some(old) = &self.puppet {
            load::carry_over_parameters(old.instance(), puppet.instance_mut());
        }
        for (i, t) in textures.iter().enumerate() {
            self.renderer
                .set_texture(
                    &self.device,
                    &self.queue,
                    i as u32,
                    t.width,
                    t.height,
                    &t.pixels,
                )
                .map_err(js_error)?;
        }
        self.puppet = Some(puppet);
        self.dirty = false;
        Ok(warnings)
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
