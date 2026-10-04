//! 常駐の描画の口 (ADR 0028 の 3.、仮想カメラの受け手が使う)。
//!
//! [`Gpu::render`] は 1 枚を描くたびにレンダラ・テクスチャ・描画先を作り直して読み戻しを待つ。[`Resident`] はそれらを
//! 持ち続け、毎フレームは描いて読み戻しを **頼むだけ** にする (待たない)。読み戻し用のバッファを数枚回し、
//! [`Resident::poll`] で終わったものから最新の 1 枚を受け取る。
//!
//! - 画素の形は [`PixelFormat`]。NV12・YUY2 は GPU の計算シェーダ (`yuv.wgsl`) で変換してから読み戻す
//!   (読み戻す量は RGBA の半分〜3/8)。計算シェーダの無いアダプタでは RGBA を読み戻して CPU で変換する ([`crate::yuv::convert`])。
//! - クリアの色 (`background`) はストレートアルファ。仮想カメラでは不透明な色を渡す (透過は単色で埋める)。
//!   背景の画像 ([`Resident::set_backdrop`]) を置くと、クリアの代わりに毎フレームその画を描画先に写してから描く (GPU の中の写し)。
//! - 描画中の wgpu のエラーは [`Error::Gpu`] にする ([`Gpu::render`] と同じ)。
//! - [`Wait::All`] で待つのは [`Gpu::wait_timeout`] まで ([`Gpu::render`] と同じ)。

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use ugoe_render::{FrameStats, Lighting, Renderer, View};
use ugoe_runtime::ModelInstance;

use crate::gpu::{clear, gpu_error, new_renderer, target_texture};
use crate::image::Rgba8;
use crate::yuv::{self, PixelFormat};
use crate::{Error, Gpu, Shading, bail};

/// 読み戻し用のバッファの既定の枚数 (GPU が 2 フレーム遅れても描き続けられる)。
pub const DEFAULT_SLOTS: usize = 3;

const IDLE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

/// 計算シェーダの 1 つのワークグループの一辺 (`yuv.wgsl` の `@workgroup_size(8, 8)`)。
const WORKGROUP: u32 = 8;

/// 常駐の描画の設定。
#[derive(Debug, Clone)]
pub struct ResidentOptions {
    /// 出力の大きさ (画素)。
    pub size: (u32, u32),
    /// 読み戻す画素の形。
    pub format: PixelFormat,
    /// クリアの色 (ストレートアルファの RGBA8)。
    pub background: [u8; 4],
    /// キャンバスを出力に写すビュー ([`crate::fit_view`])。
    pub view: View,
    /// 読み戻し用のバッファの枚数 (1 以上)。
    pub slots: usize,
    /// `true` なら、計算シェーダのあるアダプタでも NV12・YUY2 への変換を CPU で行う
    /// (RGBA を読み戻して [`crate::yuv::convert`])。計算シェーダの無いアダプタと同じ道をテストで通すためのもので、ふつうは `false`。
    pub cpu_yuv: bool,
}

/// 読み戻した 1 枚。
#[derive(Debug, Clone, PartialEq)]
pub struct Readback {
    /// [`Resident::submit`] に渡した印 (受け手のフレームの `seq` など)。
    pub tag: u64,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    /// [`PixelFormat::frame_bytes`] の大きさの画素。
    pub bytes: Vec<u8>,
    /// この 1 枚を描いたときの統計。
    pub stats: FrameStats,
}

/// 読み戻しを待つか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// 終わっているものだけを受け取る (待たない)。
    No,
    /// 頼んだものがすべて終わるまで待つ (テストと、止めるときの後片付け)。待つのは [`Gpu::wait_timeout`] までで、
    /// 超えたら [`Error::Gpu`] を返す ([`Error::is_gpu_wait_timeout`])。そのときは `Gpu` とこの `Resident` を使い回さない。
    All,
}

/// 読み戻し用のバッファ 1 枚。
#[derive(Debug)]
struct Slot {
    buffer: wgpu::Buffer,
    /// `IDLE` → (submit) `PENDING` → (map の完了) `READY` / `FAILED` → (poll) `IDLE`。
    state: Arc<AtomicU8>,
    tag: u64,
    stats: FrameStats,
}

/// RGBA から YUV への計算シェーダ。
#[derive(Debug)]
struct Converter {
    passes: Vec<(wgpu::ComputePipeline, (u32, u32))>,
    bind_group: wgpu::BindGroup,
    output: wgpu::Buffer,
}

/// 常駐の描画。レンダラ・テクスチャ・描画先・読み戻しのバッファを持ち続ける。
#[derive(Debug)]
pub struct Resident {
    renderer: Renderer,
    target: wgpu::Texture,
    target_view: wgpu::TextureView,
    size: (u32, u32),
    format: PixelFormat,
    background: [u8; 4],
    /// 背景の画像 (描画先と同じ大きさ・形。乗算済み)。あればクリアの代わりに写す。
    backdrop: Option<wgpu::Texture>,
    view: View,
    converter: Option<Converter>,
    slots: Vec<Slot>,
    /// RGBA を読み戻すときの 1 行のバイト数 (`COPY_BYTES_PER_ROW_ALIGNMENT` に揃えたもの)。
    padded_row_bytes: u32,
    /// 読み戻しは終わったが、より新しい 1 枚があったので渡さなかった数。
    superseded: u64,
}

impl Resident {
    /// レンダラを作り、テクスチャ・法線マップ・光源を上げる。
    pub fn new(
        gpu: &Gpu,
        textures: &[Rgba8],
        shading: &Shading,
        options: ResidentOptions,
    ) -> Result<Self, Error> {
        let ResidentOptions {
            size,
            format,
            background,
            view,
            slots,
            cpu_yuv,
        } = options;
        format.check_size(size.0, size.1)?;
        if slots == 0 {
            bail!("the resident renderer needs at least one readback buffer");
        }
        gpu.scoped(|| {
            let device = &gpu.device;
            let renderer = new_renderer(device, &gpu.queue, textures, shading, size)?;
            let compute = format != PixelFormat::Rgba8 && !cpu_yuv && gpu.supports_compute();
            // 背景の画像 (`set_backdrop`) を写せるよう、描画先は写し先にもする。
            let usage = wgpu::TextureUsages::COPY_DST
                | if compute {
                    wgpu::TextureUsages::TEXTURE_BINDING
                } else {
                    wgpu::TextureUsages::empty()
                };
            let target = target_texture(device, size, usage);
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let converter = compute.then(|| Converter::new(gpu, &target_view, size, format));
            let padded_row_bytes =
                (size.0 * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
            let download = match &converter {
                Some(_) => format.frame_bytes(size.0, size.1) as u64,
                None => u64::from(padded_row_bytes) * u64::from(size.1),
            };
            let slots = (0..slots)
                .map(|_| Slot {
                    buffer: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("ugoe resident readback"),
                        size: download,
                        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    state: Arc::new(AtomicU8::new(IDLE)),
                    tag: 0,
                    stats: FrameStats::default(),
                })
                .collect();
            Ok(Self {
                renderer,
                target,
                target_view,
                size,
                format,
                background,
                backdrop: None,
                view,
                converter,
                slots,
                padded_row_bytes,
                superseded: 0,
            })
        })
    }

    /// 出力の大きさ (画素)。
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// 読み戻す画素の形。
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// YUV への変換を GPU で行うか (`false` なら CPU で行う)。
    pub fn converts_on_gpu(&self) -> bool {
        self.converter.is_some()
    }

    /// クリアの色を変える (次の [`Resident::submit`] から)。
    pub fn set_background(&mut self, background: [u8; 4]) {
        self.background = background;
    }

    /// 光源を変える (次の [`Resident::submit`] から)。`None` は光源を渡さないのと同じ (環境光が白で光源なし)。
    pub fn set_lighting(&mut self, lighting: Option<&Lighting>) {
        self.renderer
            .set_lighting(lighting.unwrap_or(&Lighting::default()));
    }

    /// 背景の画像を置く・外す (次の [`Resident::submit`] から)。画像は出力と同じ大きさの乗算済みの RGBA8 で、
    /// 毎フレーム、クリアの代わりに描画先へ写してからモデルを描く。外すとクリアの色に戻る。
    pub fn set_backdrop(&mut self, gpu: &Gpu, image: Option<&Rgba8>) -> Result<(), Error> {
        let Some(image) = image else {
            self.backdrop = None;
            return Ok(());
        };
        if (image.width, image.height) != self.size
            || image.pixels.len() != self.size.0 as usize * self.size.1 as usize * 4
        {
            bail!(
                "the backdrop is {}x{} but the output is {}x{}",
                image.width,
                image.height,
                self.size.0,
                self.size.1
            );
        }
        let texture = gpu.scoped(|| {
            let texture = target_texture(&gpu.device, self.size, wgpu::TextureUsages::COPY_DST);
            gpu.queue.write_texture(
                texture.as_image_copy(),
                &image.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.size.0 * 4),
                    rows_per_image: Some(self.size.1),
                },
                texture.size(),
            );
            Ok(texture)
        })?;
        self.backdrop = Some(texture);
        Ok(())
    }

    /// ビューを変える (次の [`Resident::submit`] から)。
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// 読み戻しを頼んだまま終わっていない数。
    pub fn in_flight(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.state.load(Ordering::Acquire) != IDLE)
            .count()
    }

    /// 読み戻しは終わったが、より新しい 1 枚があったので渡さなかった数 (作ってからの合計)。
    pub fn superseded(&self) -> u64 {
        self.superseded
    }

    /// `instance` を描いて読み戻しを頼む (待たない)。空いたバッファが無ければ描かずに `false` を返す
    /// (GPU が追いついていない)。`tag` は [`Readback::tag`] に戻る。
    pub fn submit(&mut self, gpu: &Gpu, instance: &ModelInstance, tag: u64) -> Result<bool, Error> {
        let Some(index) = self
            .slots
            .iter()
            .position(|s| s.state.load(Ordering::Acquire) == IDLE)
        else {
            return Ok(false);
        };
        let (device, queue) = (&gpu.device, &gpu.queue);
        let stats = gpu.scoped(|| {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ugoe resident render"),
            });
            match &self.backdrop {
                Some(backdrop) => encoder.copy_texture_to_texture(
                    backdrop.as_image_copy(),
                    self.target.as_image_copy(),
                    self.target.size(),
                ),
                None => clear(&mut encoder, &self.target_view, self.background),
            }
            self.renderer.encode(
                device,
                queue,
                &mut encoder,
                &self.target_view,
                &self.view,
                instance,
            )?;
            let slot = &self.slots[index];
            match &self.converter {
                Some(converter) => {
                    converter.encode(&mut encoder);
                    encoder.copy_buffer_to_buffer(
                        &converter.output,
                        0,
                        &slot.buffer,
                        0,
                        slot.buffer.size(),
                    );
                }
                None => encoder.copy_texture_to_buffer(
                    self.target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &slot.buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(self.padded_row_bytes),
                            rows_per_image: Some(self.size.1),
                        },
                    },
                    self.target.size(),
                ),
            }
            queue.submit([encoder.finish()]);
            Ok(self.renderer.last_frame_stats())
        })?;
        let slot = &mut self.slots[index];
        slot.tag = tag;
        slot.stats = stats;
        slot.state.store(PENDING, Ordering::Release);
        let state = Arc::clone(&slot.state);
        slot.buffer.map_async(wgpu::MapMode::Read, .., move |r| {
            state.store(if r.is_ok() { READY } else { FAILED }, Ordering::Release);
        });
        Ok(true)
    }

    /// 読み戻しの終わったものから、最も新しい 1 枚を返す (古いものは捨てて [`Resident::superseded`] に数える)。
    /// 終わったものが無ければ `None`。
    pub fn poll(&mut self, gpu: &Gpu, wait: Wait) -> Result<Option<Readback>, Error> {
        match wait {
            Wait::No => {
                gpu.device
                    .poll(wgpu::PollType::Poll)
                    .map_err(gpu_error("waiting for the GPU"))?;
            }
            Wait::All => gpu.wait()?,
        }
        let mut newest: Option<usize> = None;
        let mut failed = false;
        for (i, slot) in self.slots.iter().enumerate() {
            match slot.state.load(Ordering::Acquire) {
                READY => {
                    if let Some(n) = newest {
                        if self.slots[n].tag < slot.tag {
                            newest = Some(i);
                        }
                    } else {
                        newest = Some(i);
                    }
                }
                FAILED => failed = true,
                _ => {}
            }
        }
        let result = match newest {
            Some(i) => Some(self.read(i)?),
            None => None,
        };
        for (i, slot) in self.slots.iter().enumerate() {
            match slot.state.load(Ordering::Acquire) {
                READY => {
                    if Some(i) != newest {
                        self.superseded += 1;
                    }
                    slot.buffer.unmap();
                    slot.state.store(IDLE, Ordering::Release);
                }
                FAILED => slot.state.store(IDLE, Ordering::Release),
                _ => {}
            }
        }
        if failed {
            return Err(Error::Gpu {
                context: "mapping the readback buffer",
                detail: "the GPU could not map a readback buffer".to_owned(),
            });
        }
        Ok(result)
    }

    fn read(&self, index: usize) -> Result<Readback, Error> {
        let slot = &self.slots[index];
        let mapped = slot
            .buffer
            .get_mapped_range(..)
            .map_err(gpu_error("reading the readback buffer"))?;
        let (width, height) = self.size;
        let bytes = if self.converter.is_some() {
            mapped.to_vec()
        } else {
            let row_bytes = width as usize * 4;
            let mut pixels = Vec::with_capacity(row_bytes * height as usize);
            for row in mapped.chunks_exact(self.padded_row_bytes as usize) {
                pixels.extend_from_slice(&row[..row_bytes]);
            }
            if self.format == PixelFormat::Rgba8 {
                pixels
            } else {
                yuv::convert(
                    &Rgba8 {
                        width,
                        height,
                        pixels,
                    },
                    self.format,
                )?
            }
        };
        Ok(Readback {
            tag: slot.tag,
            width,
            height,
            format: self.format,
            bytes,
            stats: slot.stats,
        })
    }
}

impl Converter {
    fn new(
        gpu: &Gpu,
        source: &wgpu::TextureView,
        (width, height): (u32, u32),
        format: PixelFormat,
    ) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ugoe yuv"),
            source: wgpu::ShaderSource::Wgsl(include_str!("yuv.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ugoe yuv layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ugoe yuv pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe yuv output"),
            size: format.frame_bytes(width, height) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe yuv params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut raw = [0u8; 16];
        raw[..4].copy_from_slice(&width.to_le_bytes());
        raw[4..8].copy_from_slice(&height.to_le_bytes());
        gpu.queue.write_buffer(&params, 0, &raw);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ugoe yuv bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
            ],
        });
        // 入口ごとに、1 語を書く呼び出しの数 (横・縦)。
        let entries: &[(&str, (u32, u32))] = match format {
            PixelFormat::Nv12 => &[
                ("nv12_luma", (width / 4, height)),
                ("nv12_chroma", (width / 4, height / 2)),
            ],
            PixelFormat::Yuy2 => &[("yuy2", (width / 2, height))],
            PixelFormat::Rgba8 => &[],
        };
        let passes = entries
            .iter()
            .map(|&(entry, words)| {
                let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                });
                (pipeline, words)
            })
            .collect();
        Self {
            passes,
            bind_group,
            output,
        }
    }

    fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ugoe yuv"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        for (pipeline, (x, y)) in &self.passes {
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(x.div_ceil(WORKGROUP), y.div_ceil(WORKGROUP), 1);
        }
    }
}
