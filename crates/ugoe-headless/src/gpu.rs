//! ヘッドレスの wgpu でモデルを描いて読み戻す。
//!
//! - 描画先は `Rgba8Unorm`・乗算済みアルファ。PNG に書くときにストレートアルファへ戻す
//!   ([`crate::image::encode_png_unpremultiplied`])。
//! - 描画中の wgpu のエラー (検証・メモリ不足・内部) はエラースコープで受け取り、[`Error::Gpu`] にする
//!   (wgpu の既定では panic になる)。
//! - 読み戻しは時間の上限 ([`Gpu::wait_timeout`]) まで待ち、超えたら [`Error::Gpu`] を返す
//!   ([`Error::is_gpu_wait_timeout`])。ドライバが止まっても呼び出したスレッドが戻らなくならないように。

use std::time::{Duration, Instant};

use ugoe_render::{FrameStats, Renderer, View};
use ugoe_runtime::ModelInstance;

use crate::image::Rgba8;
use crate::{Error, Shading, bail};

/// 描画先の形式。レンダラの前提 (リニア化しない) に合わせて sRGB 形式にしない。
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// GPU を待つ時間の上限の既定 (30 秒)。
///
/// 1 枚の描画 ([`Gpu::render`]) と [`crate::Resident::poll`] の [`crate::Wait::All`] が、1 回の呼び出しで
/// 読み戻しを待つ時間の合計の上限。ソフトウェアのラスタライザ (lavapipe・llvmpipe) でも 1 枚は数秒かからないので、
/// 十分に余裕を持たせた長さにしてある (ふつうの描画が上限に当たらないように)。変えるときは [`Gpu::set_wait_timeout`]。
pub const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

/// 1 回の `Device::poll` で待つ長さの上限。上限 ([`Gpu::wait_timeout`]) までこの刻みで待ち直す。
///
/// wgpu の GL のバックエンドは 1 回の待ちを約 2.1 秒 (`i32::MAX` ナノ秒) で打ち切るので、
/// 刻んで待ち直さないと、どのバックエンドでも同じ上限にならない。
const WAIT_STEP: Duration = Duration::from_secs(1);

/// [`Error::is_gpu_wait_timeout`] で見分ける、待つ時間の上限を超えたときの `context`。
pub(crate) const WAIT_TIMED_OUT: &str = "the GPU did not finish in time";

/// ヘッドレスの GPU。
#[derive(Debug)]
pub struct Gpu {
    _instance: wgpu::Instance,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    /// 選ばれたアダプタ。
    pub adapter_info: wgpu::AdapterInfo,
    /// アダプタが WebGPU の全機能を持たないときに、使える機能 (計算シェーダの有無など)。
    downlevel: wgpu::DownlevelCapabilities,
    /// 読み戻しを待つ時間の上限 ([`Gpu::wait_timeout`])。
    wait_timeout: Duration,
}

impl Gpu {
    /// サーフェス無しでアダプタとデバイスを作る。
    ///
    /// `backend` (例: `vulkan`、`gl`、`vulkan,gl`) を省略すると環境変数 `WGPU_BACKEND` に従い、
    /// それも無ければ wgpu の既定。`WGPU_POWER_PREF` も参照する。
    pub fn new(backend: Option<&str>) -> Result<Self, Error> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        if let Some(backend) = backend {
            desc.backends = wgpu::Backends::from_comma_list(backend);
            if desc.backends.is_empty() {
                bail!(
                    "unknown backend `{backend}` (expected vulkan, metal, dx12 or gl; \
                     comma-separated for several)"
                );
            }
        }
        let instance = wgpu::Instance::new(desc);
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::from_env().unwrap_or_default(),
                ..Default::default()
            })) {
                Ok(adapter) => adapter,
                Err(e) => {
                    // 破棄しない理由は [`Gpu::leak`] を参照。
                    std::mem::forget(instance);
                    return Err(Error::NoAdapter(e));
                }
            };
        let adapter_info = adapter.get_info();
        let downlevel = adapter.get_downlevel_capabilities();
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("ugoe render"),
                required_limits: limits,
                ..Default::default()
            })) {
                Ok(pair) => pair,
                Err(e) => {
                    std::mem::forget((instance, adapter));
                    return Err(Error::Device(e));
                }
            };
        Ok(Self {
            _instance: instance,
            device,
            queue,
            adapter_info,
            downlevel,
            wait_timeout: DEFAULT_WAIT_TIMEOUT,
        })
    }

    /// GPU を待つ時間の上限 (既定は [`DEFAULT_WAIT_TIMEOUT`])。
    ///
    /// [`Gpu::render`] と [`crate::Resident::poll`] の [`crate::Wait::All`] は、読み戻しをこの長さまで待ち、
    /// 超えたら [`Error::Gpu`] を返す ([`Error::is_gpu_wait_timeout`] が `true`)。上限を超えたデバイスは
    /// 止まっているかもしれないので、長く動くプロセスはその `Gpu` (と、それで作った `Resident`) を使い回さずに
    /// 手放して作り直す (破棄はしない。[`Gpu::leak`])。
    pub fn wait_timeout(&self) -> Duration {
        self.wait_timeout
    }

    /// GPU を待つ時間の上限を変える ([`Gpu::wait_timeout`])。`Duration::MAX` なら上限なしで待つ。
    pub fn set_wait_timeout(&mut self, timeout: Duration) {
        self.wait_timeout = timeout;
    }

    /// 頼んだ GPU の仕事がすべて終わるまで、上限 ([`Gpu::wait_timeout`]) まで待つ。
    pub(crate) fn wait(&self) -> Result<(), Error> {
        wait_with(self.wait_timeout, |step| {
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(step),
                })
                .map(drop)
        })
    }

    /// `名前 (バックエンド)` の形のアダプタの説明。
    pub fn adapter_description(&self) -> String {
        format!(
            "{} ({:?})",
            self.adapter_info.name, self.adapter_info.backend
        )
    }

    /// 計算シェーダを使えるか (常駐の描画は、使えなければ YUV への変換を CPU で行う)。
    pub fn supports_compute(&self) -> bool {
        self.downlevel
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    }

    /// インスタンスとデバイスを破棄せずに手放す (プロセスの終了でまとめて解放される)。
    ///
    /// WSL2 の dzn (Vulkan on D3D12) ドライバは、wgpu のインスタンスやデバイスの破棄時に
    /// SIGSEGV を起こすことがある。CLI は描画のあとすぐ終了するので、破棄しないほうが安全。
    /// 長く動くプロセス (MCP サーバ) は 1 つの `Gpu` を使い回し、やはり破棄しない。
    pub fn leak(self) {
        std::mem::forget(self);
    }

    /// `instance` を `size` の画像に描いて読み戻す。結果は乗算済みアルファと、描画の統計。
    ///
    /// `background` はストレートアルファの RGBA8。乗算済みに直してからクリアする。
    /// `shading` の法線マップと光源をレンダラに渡す (`docs/spec/rendering.md#ライティング`)。
    pub fn render(
        &self,
        instance: &ModelInstance,
        textures: &[Rgba8],
        shading: &Shading,
        size: (u32, u32),
        view: &View,
        background: [u8; 4],
    ) -> Result<(Rgba8, FrameStats), Error> {
        self.scoped(|| self.render_unscoped(instance, textures, shading, size, view, background))
    }

    /// `f` の間の wgpu のエラー (検証・メモリ不足・内部) を受け取り、[`Error::Gpu`] にする。
    pub(crate) fn scoped<T>(&self, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        // スコープは積んだのと逆の順に外す (変数は宣言と逆の順に破棄されるので、途中で戻っても崩れない)。
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let out_of_memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = f();
        for scope in [validation, out_of_memory, internal] {
            if let Some(e) = pollster::block_on(scope.pop()) {
                return Err(Error::Gpu {
                    context: "the GPU reported an error while rendering",
                    detail: e.to_string(),
                });
            }
        }
        result
    }

    fn render_unscoped(
        &self,
        instance: &ModelInstance,
        textures: &[Rgba8],
        shading: &Shading,
        size: (u32, u32),
        view: &View,
        background: [u8; 4],
    ) -> Result<(Rgba8, FrameStats), Error> {
        let (device, queue) = (&self.device, &self.queue);
        let (width, height) = size;
        let mut renderer = new_renderer(device, queue, textures, shading, size)?;
        let target = target_texture(device, size, wgpu::TextureUsages::empty());
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ugoe render"),
        });
        clear(&mut encoder, &target_view, background);
        renderer.encode(device, queue, &mut encoder, &target_view, view, instance)?;
        let stats = renderer.last_frame_stats();

        // 読み戻しバッファの行は COPY_BYTES_PER_ROW_ALIGNMENT の倍数に揃える必要がある。
        let row_bytes = width * 4;
        let padded_row_bytes = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe render readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);

        let (tx, rx) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        self.wait()?;
        // 待ち終わればコールバックは呼ばれている。`recv` で待つと、呼ばれなかったときに止まり続ける。
        rx.try_recv()
            .map_err(gpu_error("readback callback was not called"))?
            .map_err(gpu_error("mapping the readback buffer"))?;
        let mapped = readback
            .get_mapped_range(..)
            .map_err(gpu_error("reading the readback buffer"))?;
        let mut pixels = Vec::with_capacity(row_bytes as usize * height as usize);
        for row in mapped.chunks_exact(padded_row_bytes as usize) {
            pixels.extend_from_slice(&row[..row_bytes as usize]);
        }
        Ok((
            Rgba8 {
                width,
                height,
                pixels,
            },
            stats,
        ))
    }
}

/// `size` の描画先と、`textures`・`shading` を上げたレンダラを作る。大きさがデバイスの上限を超えればエラー
/// (テクスチャと法線マップはレンダラが確かめる)。
pub(crate) fn new_renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &[Rgba8],
    shading: &Shading,
    size: (u32, u32),
) -> Result<Renderer, Error> {
    let (width, height) = size;
    let max = device.limits().max_texture_dimension_2d;
    if width > max || height > max {
        bail!("output size {width}x{height} exceeds the device limit {max}");
    }
    let mut renderer = Renderer::new(device, FORMAT);
    for (i, t) in textures.iter().enumerate() {
        renderer.set_texture(device, queue, i as u32, t.width, t.height, &t.pixels)?;
    }
    for (i, t) in shading.normal_maps.iter().enumerate() {
        if let Some(t) = t {
            renderer.set_normal_map(device, queue, i as u32, t.width, t.height, &t.pixels)?;
        }
    }
    if let Some(lighting) = &shading.lighting {
        renderer.set_lighting(lighting);
    }
    Ok(renderer)
}

/// 描画先のテクスチャ (`RENDER_ATTACHMENT | COPY_SRC` に `usage` を足す)。
pub(crate) fn target_texture(
    device: &wgpu::Device,
    (width, height): (u32, u32),
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ugoe render target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | usage,
        view_formats: &[],
    })
}

/// 描画先を `background` (ストレートアルファの RGBA8) でクリアする。クリアはホストの責務
/// (レンダラは `LoadOp::Load` で重ねる)。
pub(crate) fn clear(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    background: [u8; 4],
) {
    let a = f64::from(background[3]) / 255.0;
    let premul = |c: u8| f64::from(c) / 255.0 * a;
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: premul(background[0]),
                    g: premul(background[1]),
                    b: premul(background[2]),
                    a,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}

/// `ugoe render --stats` で出す描画の統計 (`ugoe_render::FrameStats`) の JSON。
pub fn stats_json(stats: &FrameStats) -> serde_json::Value {
    serde_json::json!({
        "drawables": stats.drawables,
        "draw_calls": stats.draw_calls,
        "state_changes": stats.state_changes,
        "mask_sets": stats.mask_sets,
        "mask_draws": stats.mask_draws,
        "mask_atlases": stats.mask_atlases,
        "mask_atlas_bytes": stats.mask_atlas_bytes,
        "masks_reused": stats.masks_reused,
        "position_bytes": stats.position_bytes,
        "uniform_bytes": stats.uniform_bytes,
        "index_bytes": stats.index_bytes,
        "uploaded_bytes": stats.uploaded_bytes,
    })
}

/// `poll` を [`WAIT_STEP`] 以下の刻みで呼び、`timeout` を超えたら [`Error::Gpu`] を返す。
/// `poll` は渡された長さまで待ち、終われば `Ok`、終わらなければ `PollError::Timeout` を返す。
fn wait_with(
    timeout: Duration,
    mut poll: impl FnMut(Duration) -> Result<(), wgpu::PollError>,
) -> Result<(), Error> {
    // 上限が大きすぎて時刻にできなければ、上限なしで待つ。
    let deadline = Instant::now().checked_add(timeout);
    loop {
        let remaining = deadline.map_or(WAIT_STEP, |d| d.saturating_duration_since(Instant::now()));
        match poll(remaining.min(WAIT_STEP)) {
            Ok(()) => return Ok(()),
            // 残りがあれば待ち直す。残りが 0 の最後の確かめでも終わっていなければ上限を超えた。
            Err(wgpu::PollError::Timeout) if !remaining.is_zero() => {}
            Err(wgpu::PollError::Timeout) => return Err(Error::gpu_wait_timeout(timeout)),
            Err(e) => return Err(gpu_error("waiting for the GPU")(e)),
        }
    }
}

pub(crate) fn gpu_error<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> Error {
    move |e| Error::Gpu {
        context,
        detail: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 知らないバックエンドの名前は、インスタンスを作る前にエラーにする (GPU が無くても確かめられる)。
    #[test]
    fn rejects_unknown_backends() {
        let err = Gpu::new(Some("nope")).unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{err:?}");
        assert!(err.to_string().contains("unknown backend `nope`"), "{err}");
    }

    /// GPU が終わらなければ、上限を超えたところでエラーを返す (止まり続けない)。1 回の待ちは刻みを超えない。
    #[test]
    fn wait_gives_up_after_the_timeout() {
        let timeout = Duration::from_millis(30);
        let started = Instant::now();
        let mut steps = Vec::new();
        let err = wait_with(timeout, |step| {
            steps.push(step);
            std::thread::sleep(step);
            Err(wgpu::PollError::Timeout)
        })
        .unwrap_err();
        assert!(err.is_gpu_wait_timeout(), "{err:?}");
        assert!(matches!(err, Error::Gpu { .. }), "{err:?}");
        assert!(err.to_string().contains("30ms"), "{err}");
        assert!(started.elapsed() >= timeout);
        assert!(steps.iter().all(|&s| s <= WAIT_STEP), "{steps:?}");
        // 最後は残りが 0 の確かめ。
        assert_eq!(steps.last(), Some(&Duration::ZERO));
    }

    /// 1 回の待ちが打ち切られても (GL のバックエンド)、上限の中なら待ち直す。
    #[test]
    fn wait_retries_within_the_timeout() {
        let mut calls = 0;
        wait_with(DEFAULT_WAIT_TIMEOUT, |step| {
            assert!(step <= WAIT_STEP && !step.is_zero(), "{step:?}");
            calls += 1;
            if calls < 3 {
                Err(wgpu::PollError::Timeout)
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(calls, 3);
    }

    /// 上限なし (`Duration::MAX`) でも刻んで待つ。上限を超えたのでない失敗は、上限のエラーにしない。
    #[test]
    fn wait_without_a_limit_and_other_errors() {
        let mut calls = 0;
        let err = wait_with(Duration::MAX, |step| {
            assert_eq!(step, WAIT_STEP);
            calls += 1;
            if calls < 2 {
                Err(wgpu::PollError::Timeout)
            } else {
                Err(wgpu::PollError::WrongSubmissionIndex(2, 1))
            }
        })
        .unwrap_err();
        assert!(!err.is_gpu_wait_timeout(), "{err:?}");
        assert!(err.to_string().starts_with("waiting for the GPU"), "{err}");
    }
}
