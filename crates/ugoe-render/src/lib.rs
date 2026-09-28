//! ugoe の参照レンダラ (wgpu)。仕様: `docs/spec/rendering.md`。
//!
//! 設計原則:
//! - **デバイスやサーフェスを所有しない。** ホスト (エディタ・ゲームエンジン・ビューア) が
//!   作った `wgpu::Device` / `Queue` と描画先 `TextureView` を受け取る。
//! - 入力は [`ugoe_runtime::ModelInstance`] の評価結果のみ。モデルの意味論には踏み込まない。
//! - ネイティブ (Vulkan/Metal/DX12/GL) と Web (WebGPU/WebGL2) の双方で動くこと。
//!   WebGL2 で使えない機能 (storage buffer・compute 等) には必ずフォールバックを用意する。

mod mask;
mod pipeline;

use std::sync::Arc;

use ugoe_runtime::{CompiledModel, ModelInstance};
use wgpu::util::DeviceExt;

pub use crate::mask::MaskAtlasOptions;
use crate::mask::{CellRequest, MaskContent, MaskSlot, MaskStep, MeshMask};
use crate::pipeline::{MASK_ATLAS_FORMAT, PipelineKey, Pipelines, VEC2_SIZE};

/// 参照シェーダの WGSL ソース。
pub const DRAWABLE_WGSL: &str = include_str!("shaders/drawable.wgsl");

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("texture index {0} is not uploaded")]
    MissingTexture(u32),
    #[error("texture data is {actual} bytes, expected {expected} bytes for {width}x{height} RGBA8")]
    InvalidTextureData {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

/// キャンバス座標から描画先への変換。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// キャンバス座標 → クリップ座標 (列優先 3x3)。
    pub view_proj: [[f32; 3]; 3],
}

impl View {
    /// 裏面カリングで表とみなす三角形の向き (クリップ座標)。
    ///
    /// キャンバス座標 (Y 下向き) で画面上反時計回りの三角形が表 (`docs/spec/rendering.md`)。
    /// 通常のビューは Y 軸を反転する (行列式 < 0) ので、クリップ座標 (Y 上向き) でも反時計回りになる。
    /// Y 軸を反転しないビューでは向きが逆になるので時計回りを表とする。
    fn front_face(&self) -> wgpu::FrontFace {
        let m = &self.view_proj;
        let det = m[0][0] * m[1][1] - m[1][0] * m[0][1];
        if det < 0.0 {
            wgpu::FrontFace::Ccw
        } else {
            wgpu::FrontFace::Cw
        }
    }
}

/// [`Renderer`] の設定。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererOptions {
    /// マスクアトラスの大きさの上限と、マスクを描く解像度 (`docs/spec/rendering.md` の「精度」)。
    pub mask_atlas: MaskAtlasOptions,
}

/// 直前の [`Renderer::encode`] で行った描画の統計 (テストやプロファイル用)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// 本描画で描いた要素の数。
    pub drawables: u32,
    /// マスクアトラスに描いたマスクの内容の数 (同じ内容を使う要素は 1 つと数える)。
    pub mask_sets: u32,
    /// マスクパスでマスク用メッシュを描いた回数 (作業領域の全面描画は数えない)。
    pub mask_draws: u32,
    /// 使ったマスクアトラスの枚数。
    pub mask_atlases: u32,
}

/// シェーダの `Globals` と同じ配置 (mat3x3 の各列は vec4 に詰める)。
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsRaw {
    view_proj: [[f32; 4]; 3],
}

/// シェーダの `DrawUniforms` と同じ配置。
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawUniformsRaw {
    multiply_color: [f32; 4],
    screen_color: [f32; 4],
    mask_rect: [f32; 4],
    mask_channel: [f32; 4],
    mask_params: [f32; 4],
    target_rect: [f32; 4],
    target_channel: [f32; 4],
}

/// シェーダの `mask_rect`・`mask_channel`・`mask_params` (要素自身のマスクの読み方)。
#[derive(Debug, Clone, Copy)]
struct MaskRead {
    rect: [f32; 4],
    channel: [f32; 4],
    params: [f32; 4],
}

impl MaskRead {
    /// マスクを持たない (係数 1)。
    const NONE: Self = Self {
        rect: [0.0, 0.0, 1.0, 1.0],
        channel: [0.0; 4],
        params: [0.0; 4],
    };

    fn new(active: &ActiveMask, mask: MeshMask, strength: f32) -> Self {
        let one_hot = active.slot.channel_mask();
        let (offset, channel) = if mask.complement {
            (1.0, one_hot.map(|v| -v))
        } else {
            (0.0, one_hot)
        };
        Self {
            rect: active.transform,
            channel,
            params: [offset, strength, 0.0, 0.0],
        }
    }
}

const DRAW_UNIFORMS_SIZE: u64 = std::mem::size_of::<DrawUniformsRaw>() as u64;

/// モデルの静的な頂点データ (UV・インデックス) と、フレームごとに書き換えるバッファ。
#[derive(Debug)]
struct Geometry {
    /// どのモデルのデータか (インスタンスが別モデルに変わったら作り直す)。
    model: Arc<CompiledModel>,
    positions: wgpu::Buffer,
    uvs: wgpu::Buffer,
    indices: wgpu::Buffer,
    meshes: Vec<MeshRange>,
    /// マスクの内容。モデルの静的なデータなので、ロード時に一度だけ求める。
    masks: Vec<MaskContent>,
    /// マスクの内容のインデックスを、レベルの大きい順に並べたもの (外接矩形の伝播に使う)。
    masks_by_level_desc: Vec<u32>,
    /// 内容 `c` の `Draw` の段のユニフォームを置く最初のスロット。
    mask_slot_base: Vec<u32>,
    /// 描画可能要素ごとのユニフォーム。要素 `i` はオフセット `i * draw_stride`。
    /// その後ろに、マスクパスでマスク用メッシュを描く分を内容ごとに並べる。
    draws: wgpu::Buffer,
    draws_bind_group: wgpu::BindGroup,
    draw_stride: u64,
}

/// 共有バッファ内での 1 メッシュ分の範囲 (バイト単位)。
#[derive(Debug, Clone, Copy)]
struct MeshRange {
    vertex_offset: u64,
    vertex_size: u64,
    index_offset: u64,
    index_count: u32,
    texture: u32,
    culling: bool,
    /// 自身のマスクとして読む内容。マスクを持たないなら `None`。
    mask: Option<MeshMask>,
}

/// マスクアトラス 1 枚。フレームをまたいで使い回す。
#[derive(Debug)]
struct MaskAtlas {
    // ビューとバインドグループが参照するので保持しておく。
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

/// 1 フレームで使うマスクの内容の情報。
#[derive(Debug, Clone, Copy)]
struct ActiveMask {
    /// `Geometry::masks` のインデックス。
    content: u32,
    slot: MaskSlot,
    /// 画面 UV → アトラス UV の変換 (`mask_rect` ユニフォーム)。
    transform: [f32; 4],
}

#[derive(Debug)]
struct TextureSlot {
    // バインドグループが参照するので保持しておく。
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// モデル描画器。モデル (テクスチャ集合) 1 つにつき 1 つ作る。
///
/// `encode` はバッファを `Queue::write_buffer` で書き換えるので、
/// 同じ `Renderer` の `encode` は 1 回の `Queue::submit` につき 1 回だけ呼ぶこと。
#[derive(Debug)]
pub struct Renderer {
    target_format: wgpu::TextureFormat,
    pipelines: Pipelines,
    draw_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// マスクを使わない要素に束縛する 1x1 のダミー (`mask_strength = 0` なので値は効かない)。
    no_mask_bind_group: wgpu::BindGroup,
    mask_options: MaskAtlasOptions,
    /// アトラスの一辺の上限 (設定とデバイスの上限の小さい方)。
    mask_atlas_max_size: u32,
    /// 今のアトラスの一辺。必要になれば大きくし、小さくはしない (0 はまだ作っていない)。
    mask_atlas_size: u32,
    mask_atlases: Vec<MaskAtlas>,
    stats: FrameStats,
    textures: Vec<Option<TextureSlot>>,
    geometry: Option<Geometry>,
    // フレームごとの書き込み用の作業領域。
    positions_scratch: Vec<[f32; 2]>,
    draws_scratch: Vec<u8>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        Self::with_options(device, target_format, RendererOptions::default())
    }

    pub fn with_options(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        options: RendererOptions,
    ) -> Self {
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ugoe globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<GlobalsRaw>() as u64
                    ),
                },
                count: None,
            }],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ugoe draw uniforms layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // マスクパスでは頂点シェーダもアトラスへの変換を読む。
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(DRAW_UNIFORMS_SIZE),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ugoe texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        // マスク (group 3) もテクスチャ + サンプラなので同じレイアウトを使う。
        let pipelines = Pipelines::new(
            device,
            target_format,
            &[
                &globals_layout,
                &draw_layout,
                &texture_layout,
                &texture_layout,
            ],
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ugoe texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe globals"),
            size: std::mem::size_of::<GlobalsRaw>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ugoe globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });

        // 内容は使われない (mask_strength = 0)。wgpu は未書き込みのテクスチャをゼロで初期化する。
        let no_mask_texture = device.create_texture(&texture_descriptor("ugoe no mask", 1, 1));
        let no_mask_bind_group =
            texture_bind_group(device, &texture_layout, &no_mask_texture, &sampler);
        let mask_atlas_max_size =
            mask::max_atlas_size(options.mask_atlas, device.limits().max_texture_dimension_2d);

        Self {
            target_format,
            pipelines,
            draw_layout,
            texture_layout,
            sampler,
            globals,
            globals_bind_group,
            no_mask_bind_group,
            mask_options: options.mask_atlas,
            mask_atlas_max_size,
            mask_atlas_size: 0,
            mask_atlases: Vec::new(),
            stats: FrameStats::default(),
            textures: Vec::new(),
            geometry: None,
            positions_scratch: Vec::new(),
            draws_scratch: Vec::new(),
        }
    }

    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// 直前に成功した [`Renderer::encode`] の統計。
    pub fn last_frame_stats(&self) -> FrameStats {
        self.stats
    }

    /// 乗算済みアルファの RGBA8 テクスチャを登録する。同じ `index` に再登録すると置き換える。
    ///
    /// 値はリニア化せずにそのまま合成する (`docs/spec/rendering.md` の「前提」) ので、
    /// テクスチャ形式は `Rgba8Unorm` (sRGB 形式ではない)。
    pub fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: u32,
        width: u32,
        height: u32,
        rgba_premultiplied: &[u8],
    ) -> Result<(), RenderError> {
        let expected = width as usize * height as usize * 4;
        if width == 0 || height == 0 || rgba_premultiplied.len() != expected {
            return Err(RenderError::InvalidTextureData {
                width,
                height,
                expected,
                actual: rgba_premultiplied.len(),
            });
        }
        let texture = device.create_texture_with_data(
            queue,
            &texture_descriptor("ugoe model texture", width, height),
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba_premultiplied,
        );
        let bind_group = texture_bind_group(device, &self.texture_layout, &texture, &self.sampler);
        let index = index as usize;
        if self.textures.len() <= index {
            self.textures.resize_with(index + 1, || None);
        }
        self.textures[index] = Some(TextureSlot {
            _texture: texture,
            bind_group,
        });
        Ok(())
    }

    /// 現在のインスタンス状態を描画するコマンドを `encoder` に積む。
    ///
    /// マスクを使う要素があれば、先にマスクパスでマスクアトラスを描く。
    /// 描画先は読み込んだうえで上に重ねる (`LoadOp::Load`)。クリアはホストが行う。
    /// 非表示の要素は描画しない。描画する要素 (とそのマスク用メッシュ) のテクスチャが未登録なら
    /// エラーを返し、何も積まない。
    ///
    /// マスクの解像度は `target` のテクスチャの大きさ (画素) から決める (`docs/spec/rendering.md` の「精度」)。
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        view: &View,
        instance: &ModelInstance,
    ) -> Result<(), RenderError> {
        if self
            .geometry
            .as_ref()
            .is_none_or(|g| !Arc::ptr_eq(&g.model, instance.compiled()))
        {
            self.geometry = Some(Geometry::new(
                device,
                &self.draw_layout,
                instance.compiled(),
            ));
        }
        let Some(geometry) = self.geometry.as_ref() else {
            return Ok(());
        };
        let drawables = instance.drawables();
        let m = view.view_proj;
        let target_size = target.texture().size();
        let target_pixels = [
            target_size.width.max(1) as f32,
            target_size.height.max(1) as f32,
        ];
        let mesh_bbox = |i: usize| screen_bbox(&m, &drawables[i].positions);

        // 描画する要素を決め、必要なテクスチャとパイプラインを先に揃える。
        // マスクの内容ごとに、それを読む要素の画面上の外接矩形 (画面 UV) も求める。
        let front_face = view.front_face();
        let mut draws = Vec::with_capacity(drawables.len());
        let mut bboxes: Vec<Option<[f32; 4]>> = vec![None; geometry.masks.len()];
        for &i in instance.render_order() {
            let (state, mesh) = (&drawables[i as usize], geometry.meshes[i as usize]);
            if !state.visible || mesh.index_count == 0 {
                continue;
            }
            if self.texture(mesh.texture).is_none() {
                return Err(RenderError::MissingTexture(mesh.texture));
            }
            if let Some(mask) = mesh.mask {
                union_bbox(&mut bboxes[mask.content as usize], mesh_bbox(i as usize));
            }
            let key = PipelineKey {
                blend: state.blend,
                cull_front_face: mesh.culling.then_some(front_face),
            };
            self.pipelines.prepare(device, key);
            draws.push((i as usize, key));
        }

        // 入れ子の継承: マスク用メッシュ自身のマスクは、そのメッシュが内容を描く範囲
        // (メッシュの外接矩形と、描き込む内容の外接矩形の共通部分) で要る。
        // 内容のレベルは、そのマスク用メッシュ自身の内容より必ず大きいので、大きい順に伝播する。
        for &c in &geometry.masks_by_level_desc {
            let Some(outer) = bboxes[c as usize].and_then(clip_bbox) else {
                continue;
            };
            for &member in &geometry.masks[c as usize].members {
                let Some(own) = geometry.meshes[member as usize].mask else {
                    continue;
                };
                if let Some(b) = mesh_bbox(member as usize).and_then(|b| intersect_bbox(b, outer)) {
                    union_bbox(&mut bboxes[own.content as usize], Some(b));
                }
            }
        }

        // 使う内容にアトラスの場所を割り当てる。外接矩形が画面外か面積 0 の内容は
        // 読む要素が画素を持たないので、描かない。外接矩形は画素の境界に広げて、
        // 既定の解像度ではアトラスの texel と画面の画素が一致するようにする。
        let mut active_of: Vec<Option<usize>> = vec![None; geometry.masks.len()];
        let mut used = Vec::new();
        let mut requests = Vec::new();
        for (c, bbox) in bboxes.iter().enumerate() {
            let Some(b) = bbox.and_then(|b| snap_bbox(b, target_pixels)) else {
                continue;
            };
            let content = &geometry.masks[c];
            // マスク用メッシュは自分が非表示でも描くので、テクスチャが要る。
            for &member in &content.members {
                let mesh = geometry.meshes[member as usize];
                if mesh.index_count > 0 && self.texture(mesh.texture).is_none() {
                    return Err(RenderError::MissingTexture(mesh.texture));
                }
            }
            let [width, height] = mask::cell_size(
                [
                    (b[2] - b[0]) * target_pixels[0],
                    (b[3] - b[1]) * target_pixels[1],
                ],
                self.mask_options.downscale,
                self.mask_atlas_max_size,
            );
            active_of[c] = Some(used.len());
            used.push((c as u32, b));
            requests.push(CellRequest {
                level: content.level,
                width,
                height,
            });
        }
        let (atlas_size, slots) = if requests.is_empty() {
            (self.mask_atlas_size, Vec::new())
        } else {
            mask::allocate(&requests, self.mask_atlas_size, self.mask_atlas_max_size)
        };
        let active: Vec<ActiveMask> = slots
            .into_iter()
            .zip(used)
            .map(|(slot, (content, bbox))| ActiveMask {
                content,
                slot,
                transform: slot.transform(atlas_size, bbox),
            })
            .collect();
        draws.retain(|&(i, _)| {
            geometry.meshes[i]
                .mask
                .is_none_or(|mask| active_of[mask.content as usize].is_some())
        });
        let atlas_count = active
            .iter()
            .map(|a| a.slot.atlas as usize + 1)
            .max()
            .unwrap_or(0);
        if atlas_size != self.mask_atlas_size {
            self.mask_atlases.clear();
            self.mask_atlas_size = atlas_size;
        }
        while self.mask_atlases.len() < atlas_count {
            self.mask_atlases.push(MaskAtlas::new(
                device,
                &self.texture_layout,
                &self.sampler,
                atlas_size,
            ));
        }

        // 頂点位置 (毎フレーム)。
        self.positions_scratch.clear();
        for state in drawables {
            self.positions_scratch
                .extend(state.positions.iter().map(|p| [p.x, p.y]));
        }
        if !self.positions_scratch.is_empty() {
            queue.write_buffer(
                &geometry.positions,
                0,
                bytemuck::cast_slice(&self.positions_scratch),
            );
        }

        // 要素 (またはマスク用メッシュ) `i` 自身のマスクの読み方。
        let mask_read = |i: usize| {
            geometry.meshes[i]
                .mask
                .and_then(|mask| {
                    let a = &active[active_of[mask.content as usize]?];
                    Some(MaskRead::new(a, mask, drawables[i].mask_strength))
                })
                .unwrap_or(MaskRead::NONE)
        };

        // 要素ごとのユニフォームと、マスクパスで描くマスク用メッシュのユニフォーム。
        let stride = geometry.draw_stride as usize;
        self.draws_scratch.clear();
        self.draws_scratch.resize(geometry.draws.size() as usize, 0);
        let mut write_draw = |slot: usize, raw: DrawUniformsRaw| {
            let start = slot * stride;
            self.draws_scratch[start..start + DRAW_UNIFORMS_SIZE as usize]
                .copy_from_slice(bytemuck::bytes_of(&raw));
        };
        for (i, state) in drawables.iter().enumerate() {
            let read = mask_read(i);
            write_draw(
                i,
                DrawUniformsRaw {
                    multiply_color: [
                        state.multiply_color.r,
                        state.multiply_color.g,
                        state.multiply_color.b,
                        state.opacity,
                    ],
                    screen_color: [
                        state.screen_color.r,
                        state.screen_color.g,
                        state.screen_color.b,
                        0.0,
                    ],
                    mask_rect: read.rect,
                    mask_channel: read.channel,
                    mask_params: read.params,
                    target_rect: [0.0; 4],
                    target_channel: [0.0; 4],
                },
            );
        }
        for a in &active {
            let base = geometry.mask_slot_base[a.content as usize] as usize;
            let draw_steps = geometry.masks[a.content as usize]
                .steps
                .iter()
                .filter_map(|step| match *step {
                    MaskStep::Draw { mesh, scratch } => Some((mesh, scratch)),
                    _ => None,
                });
            for (j, (mesh, scratch)) in draw_steps.enumerate() {
                // カバレッジはテクスチャのアルファと自身のマスク係数だけで決まる (不透明度・色は使わない)。
                let read = mask_read(mesh as usize);
                write_draw(
                    base + j,
                    DrawUniformsRaw {
                        multiply_color: [1.0; 4],
                        screen_color: [0.0; 4],
                        mask_rect: read.rect,
                        mask_channel: read.channel,
                        mask_params: read.params,
                        target_rect: a.transform,
                        target_channel: if scratch {
                            [0.0, 0.0, 0.0, 1.0]
                        } else {
                            a.slot.channel_mask()
                        },
                    },
                );
            }
        }
        if !self.draws_scratch.is_empty() {
            queue.write_buffer(&geometry.draws, 0, &self.draws_scratch);
        }

        let globals = GlobalsRaw {
            view_proj: m.map(|c| [c[0], c[1], c[2], 0.0]),
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        // マスクパス: アトラスごとに 1 パス (レベルの小さいアトラスから)。各内容はシザー矩形で
        // 自分のセルだけに描く。アトラスは 1 でクリアし、乗算合成で値を作る。
        let pipelines = self.pipelines.mask();
        let mut mask_draws = 0;
        for (index, atlas) in self.mask_atlases[..atlas_count].iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ugoe mask pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &atlas.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for a in active.iter().filter(|a| a.slot.atlas as usize == index) {
                let [x, y, w, h] = a.slot.cell;
                pass.set_scissor_rect(x, y, w, h);
                let base = geometry.mask_slot_base[a.content as usize] as usize;
                let mut j = 0;
                for step in &geometry.masks[a.content as usize].steps {
                    match *step {
                        MaskStep::FillScratch => {
                            pass.set_pipeline(&pipelines.fill_scratch);
                            pass.draw(0..3, 0..1);
                        }
                        MaskStep::Resolve => {
                            pass.set_pipeline(&pipelines.resolve[a.slot.channel as usize]);
                            pass.draw(0..3, 0..1);
                        }
                        MaskStep::Draw { mesh: member, .. } => {
                            let slot = base + j;
                            j += 1;
                            let mesh = geometry.meshes[member as usize];
                            let Some(texture) = self.texture(mesh.texture) else {
                                continue;
                            };
                            // 自身のマスクの内容が描かれていなければ、このメッシュは
                            // 内容のセルの範囲に画素を持たない (カバレッジ 0 と同じなので描かない)。
                            let own_mask = match mesh.mask {
                                None => &self.no_mask_bind_group,
                                Some(own) => match active_of[own.content as usize] {
                                    Some(o) => {
                                        &self.mask_atlases[active[o].slot.atlas as usize].bind_group
                                    }
                                    None => continue,
                                },
                            };
                            if mesh.index_count == 0 {
                                continue;
                            }
                            pass.set_pipeline(&pipelines.draw);
                            pass.set_bind_group(0, &self.globals_bind_group, &[]);
                            pass.set_bind_group(
                                1,
                                &geometry.draws_bind_group,
                                &[(slot * stride) as u32],
                            );
                            pass.set_bind_group(2, &texture.bind_group, &[]);
                            pass.set_bind_group(3, own_mask, &[]);
                            geometry.draw_mesh(&mut pass, mesh);
                            mask_draws += 1;
                        }
                    }
                }
            }
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ugoe main pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        let mut drawn = 0;
        for &(i, key) in &draws {
            let mesh = geometry.meshes[i];
            let (Some(pipeline), Some(texture)) =
                (self.pipelines.get(key), self.texture(mesh.texture))
            else {
                continue;
            };
            let mask_bind_group = match mesh.mask.and_then(|m| active_of[m.content as usize]) {
                Some(a) => &self.mask_atlases[active[a].slot.atlas as usize].bind_group,
                None => &self.no_mask_bind_group,
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(1, &geometry.draws_bind_group, &[(i * stride) as u32]);
            pass.set_bind_group(2, &texture.bind_group, &[]);
            pass.set_bind_group(3, mask_bind_group, &[]);
            geometry.draw_mesh(&mut pass, mesh);
            drawn += 1;
        }

        self.stats = FrameStats {
            drawables: drawn,
            mask_sets: active.len() as u32,
            mask_draws,
            mask_atlases: atlas_count as u32,
        };
        Ok(())
    }

    fn texture(&self, index: u32) -> Option<&TextureSlot> {
        self.textures.get(index as usize).and_then(Option::as_ref)
    }
}

impl Geometry {
    fn new(
        device: &wgpu::Device,
        draw_layout: &wgpu::BindGroupLayout,
        compiled: &Arc<CompiledModel>,
    ) -> Self {
        let model = compiled.model();
        let (mesh_masks, masks) = mask::build(compiled);
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for (mesh, &mask) in model.meshes.iter().zip(&mesh_masks) {
            let vertex_offset = uvs.len() as u64 * VEC2_SIZE;
            let index_offset = indices.len() as u64 * 2;
            uvs.extend(mesh.uvs.iter().map(|uv| [uv.x, uv.y]));
            indices.extend_from_slice(&mesh.indices);
            // 空のスライスは束縛できないので、頂点のないメッシュは描かない。
            let index_count = if mesh.uvs.is_empty() {
                0
            } else {
                mesh.indices.len() as u32
            };
            meshes.push(MeshRange {
                vertex_offset,
                vertex_size: mesh.uvs.len() as u64 * VEC2_SIZE,
                index_offset,
                index_count,
                texture: mesh.texture,
                culling: mesh.culling,
                mask,
            });
        }

        let positions = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe positions"),
            size: uvs.len() as u64 * VEC2_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uvs = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ugoe uvs"),
            contents: bytemuck::cast_slice(&uvs),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ugoe indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let draw_stride = DRAW_UNIFORMS_SIZE.div_ceil(alignment) * alignment;
        let mut mask_slot_base = Vec::with_capacity(masks.len());
        let mut slots = meshes.len() as u32;
        for content in &masks {
            mask_slot_base.push(slots);
            slots += content.draw_count() as u32;
        }
        let mut masks_by_level_desc: Vec<u32> = (0..masks.len() as u32).collect();
        masks_by_level_desc.sort_by_key(|&c| std::cmp::Reverse(masks[c as usize].level));
        let draws = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe draw uniforms"),
            size: draw_stride * u64::from(slots.max(1)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let draws_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ugoe draw uniforms"),
            layout: draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &draws,
                    offset: 0,
                    size: wgpu::BufferSize::new(DRAW_UNIFORMS_SIZE),
                }),
            }],
        });

        Self {
            model: compiled.clone(),
            positions,
            uvs,
            indices,
            meshes,
            masks,
            masks_by_level_desc,
            mask_slot_base,
            draws,
            draws_bind_group,
            draw_stride,
        }
    }

    /// 頂点・インデックスを束縛して 1 メッシュを描く。
    fn draw_mesh(&self, pass: &mut wgpu::RenderPass<'_>, mesh: MeshRange) {
        // WebGL2 は base_vertex を使えないので、バッファのスライスで要素を切り替える。
        let vertices = mesh.vertex_offset..mesh.vertex_offset + mesh.vertex_size;
        pass.set_vertex_buffer(0, self.positions.slice(vertices.clone()));
        pass.set_vertex_buffer(1, self.uvs.slice(vertices));
        let index_size = u64::from(mesh.index_count) * 2;
        pass.set_index_buffer(
            self.indices
                .slice(mesh.index_offset..mesh.index_offset + index_size),
            wgpu::IndexFormat::Uint16,
        );
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

impl MaskAtlas {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            format: MASK_ATLAS_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..texture_descriptor("ugoe mask atlas", size, size)
        });
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            bind_group: texture_bind_group(device, layout, &texture, sampler),
            _texture: texture,
        }
    }
}

/// 頂点列の画面 UV での外接矩形 `[x0, y0, x1, y1]` (画面で切り詰めない)。頂点がなければ `None`。
fn screen_bbox(m: &[[f32; 3]; 3], positions: &[ugoe_core::Vec2]) -> Option<[f32; 4]> {
    let mut bbox = None;
    for p in positions {
        let [u, v] = screen_uv(m, [p.x, p.y]);
        union_bbox(&mut bbox, Some([u, v, u, v]));
    }
    bbox
}

fn union_bbox(bbox: &mut Option<[f32; 4]>, other: Option<[f32; 4]>) {
    let Some(o) = other else { return };
    let b = bbox.get_or_insert(o);
    *b = [
        b[0].min(o[0]),
        b[1].min(o[1]),
        b[2].max(o[2]),
        b[3].max(o[3]),
    ];
}

/// 面積のある矩形なら `Some`。
fn non_empty(b: [f32; 4]) -> Option<[f32; 4]> {
    (b[2] > b[0] && b[3] > b[1]).then_some(b)
}

fn intersect_bbox(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    non_empty([
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ])
}

/// 画面 (0..1) で切り詰める。面積が 0 なら `None`。
fn clip_bbox(b: [f32; 4]) -> Option<[f32; 4]> {
    non_empty(b.map(|v| v.clamp(0.0, 1.0)))
}

/// 画面で切り詰め、画素の境界まで外側に広げる。面積が 0 なら `None`。
fn snap_bbox(b: [f32; 4], [w, h]: [f32; 2]) -> Option<[f32; 4]> {
    let b = clip_bbox(b)?;
    non_empty([
        (b[0] * w).floor() / w,
        (b[1] * h).floor() / h,
        (b[2] * w).ceil() / w,
        (b[3] * h).ceil() / h,
    ])
}

/// キャンバス座標を画面 UV (0..1、左上原点) に写す。シェーダの `to_screen_uv` と同じ。
fn screen_uv(m: &[[f32; 3]; 3], [x, y]: [f32; 2]) -> [f32; 2] {
    let cx = m[0][0] * x + m[1][0] * y + m[2][0];
    let cy = m[0][1] * x + m[1][1] * y + m[2][1];
    [cx * 0.5 + 0.5, 0.5 - cy * 0.5]
}

fn texture_descriptor(label: &str, width: u32, height: u32) -> wgpu::TextureDescriptor<'_> {
    wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    }
}

fn texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ugoe texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn reference_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(super::DRAWABLE_WGSL).expect("parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("validate");
    }
}
