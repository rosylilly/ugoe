//! ヘッドレスの wgpu でモデルを描いて読み戻す。
//!
//! - 描画先は `Rgba8Unorm`・乗算済みアルファ。PNG に書くときにストレートアルファへ戻す
//!   ([`crate::image::encode_png_unpremultiplied`])。
//! - 描画中の wgpu のエラー (検証・メモリ不足・内部) はエラースコープで受け取り、[`Error::Gpu`] にする
//!   (wgpu の既定では panic になる)。

use ugoe_render::{FrameStats, Renderer, View};
use ugoe_runtime::ModelInstance;

use crate::image::Rgba8;
use crate::{Error, Shading, bail};

/// 描画先の形式。レンダラの前提 (リニア化しない) に合わせて sRGB 形式にしない。
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// ヘッドレスの GPU。
#[derive(Debug)]
pub struct Gpu {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// 選ばれたアダプタ。
    pub adapter_info: wgpu::AdapterInfo,
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
        })
    }

    /// `名前 (バックエンド)` の形のアダプタの説明。
    pub fn adapter_description(&self) -> String {
        format!(
            "{} ({:?})",
            self.adapter_info.name, self.adapter_info.backend
        )
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
        // スコープは積んだのと逆の順に外す (変数は宣言と逆の順に破棄されるので、途中で戻っても崩れない)。
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let out_of_memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = self.render_unscoped(instance, textures, shading, size, view, background);
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
        let max = device.limits().max_texture_dimension_2d;
        if width > max || height > max {
            bail!("output size {width}x{height} exceeds the device limit {max}");
        }
        for (i, t) in textures.iter().enumerate() {
            if t.width > max || t.height > max {
                bail!(
                    "texture {i} is {}x{}, which exceeds the device limit {max}",
                    t.width,
                    t.height
                );
            }
        }
        for (i, t) in shading.normal_maps.iter().enumerate() {
            if let Some(t) = t
                && (t.width > max || t.height > max)
            {
                bail!(
                    "the normal map of texture {i} is {}x{}, which exceeds the device limit {max}",
                    t.width,
                    t.height
                );
            }
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

        let target = device.create_texture(&wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ugoe render"),
        });
        // クリアはホストの責務 (レンダラは LoadOp::Load で重ねる)。
        let a = f64::from(background[3]) / 255.0;
        let premul = |c: u8| f64::from(c) / 255.0 * a;
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
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
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(gpu_error("waiting for the GPU"))?;
        rx.recv()
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

fn gpu_error<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> Error {
    move |e| Error::Gpu {
        context,
        detail: e.to_string(),
    }
}
