//! ugoe の参照レンダラ (wgpu)。仕様: `docs/spec/rendering.md`。
//!
//! 設計原則:
//! - **デバイスやサーフェスを所有しない。** ホスト (エディタ・ゲームエンジン・ビューア) が
//!   作った `wgpu::Device` / `Queue` と描画先 `TextureView` を受け取る。
//! - 入力は [`ugoe_runtime::ModelInstance`] の評価結果のみ。モデルの意味論には踏み込まない。
//! - ネイティブ (Vulkan/Metal/DX12/GL) と Web (WebGPU/WebGL2) の双方で動くこと。
//!   WebGL2 で使えない機能 (storage buffer・compute 等) には必ずフォールバックを用意する。

mod pipeline;

use std::sync::Arc;

use ugoe_runtime::{CompiledModel, ModelInstance};
use wgpu::util::DeviceExt;

use crate::pipeline::{PipelineKey, Pipelines, VEC2_SIZE};

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
    mask_flags: [f32; 4],
    mask_rect: [f32; 4],
    mask_channel: [f32; 4],
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
    /// 描画可能要素ごとのユニフォーム。要素 `i` はオフセット `i * draw_stride`。
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
    // TODO(T0202): マスクアトラスに置き換える。今は常に「マスクなし」の 1x1 ダミー。
    mask_bind_group: wgpu::BindGroup,
    textures: Vec<Option<TextureSlot>>,
    geometry: Option<Geometry>,
    // フレームごとの書き込み用の作業領域。
    positions_scratch: Vec<[f32; 2]>,
    draws_scratch: Vec<u8>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
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
                visibility: wgpu::ShaderStages::FRAGMENT,
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

        // 内容は使われない (mask_flags.x = 0)。wgpu は未書き込みのテクスチャをゼロで初期化する。
        let mask_texture = device.create_texture(&texture_descriptor("ugoe dummy mask", 1, 1));
        let mask_bind_group = texture_bind_group(device, &texture_layout, &mask_texture, &sampler);

        Self {
            target_format,
            pipelines,
            draw_layout,
            texture_layout,
            sampler,
            globals,
            globals_bind_group,
            mask_bind_group,
            textures: Vec::new(),
            geometry: None,
            positions_scratch: Vec::new(),
            draws_scratch: Vec::new(),
        }
    }

    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
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
    /// 描画先は読み込んだうえで上に重ねる (`LoadOp::Load`)。クリアはホストが行う。
    /// 非表示の要素は描画しない。描画する要素のテクスチャが未登録ならエラーを返し、何も積まない。
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

        // 描画する要素を決め、必要なテクスチャとパイプラインを先に揃える。
        let front_face = view.front_face();
        let mut draws = Vec::with_capacity(drawables.len());
        for &i in instance.render_order() {
            let (state, mesh) = (&drawables[i as usize], geometry.meshes[i as usize]);
            if !state.visible || mesh.index_count == 0 {
                continue;
            }
            // TODO(T0202): マスク付きの要素はマスクアトラスを参照する。
            if self.texture(mesh.texture).is_none() {
                return Err(RenderError::MissingTexture(mesh.texture));
            }
            let key = PipelineKey {
                blend: state.blend,
                cull_front_face: mesh.culling.then_some(front_face),
            };
            self.pipelines.prepare(device, key);
            draws.push((i as usize, key));
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

        // 要素ごとのユニフォーム。
        self.draws_scratch.clear();
        self.draws_scratch
            .resize(geometry.draw_stride as usize * drawables.len(), 0);
        for (i, state) in drawables.iter().enumerate() {
            let raw = DrawUniformsRaw {
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
                // TODO(T0202): マスクを使う要素ではフラグ・矩形・チャンネルを設定する。
                mask_flags: [0.0; 4],
                mask_rect: [0.0, 0.0, 1.0, 1.0],
                mask_channel: [0.0; 4],
            };
            let start = i * geometry.draw_stride as usize;
            self.draws_scratch[start..start + DRAW_UNIFORMS_SIZE as usize]
                .copy_from_slice(bytemuck::bytes_of(&raw));
        }
        if !self.draws_scratch.is_empty() {
            queue.write_buffer(&geometry.draws, 0, &self.draws_scratch);
        }

        let m = view.view_proj;
        let globals = GlobalsRaw {
            view_proj: m.map(|c| [c[0], c[1], c[2], 0.0]),
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        // TODO(T0202): 本描画の前にマスクパスでマスクアトラスを描く。
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
        pass.set_bind_group(3, &self.mask_bind_group, &[]);
        for (i, key) in draws {
            let mesh = geometry.meshes[i];
            let (Some(pipeline), Some(texture)) =
                (self.pipelines.get(key), self.texture(mesh.texture))
            else {
                continue;
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(
                1,
                &geometry.draws_bind_group,
                &[(i as u64 * geometry.draw_stride) as u32],
            );
            pass.set_bind_group(2, &texture.bind_group, &[]);
            // WebGL2 は base_vertex を使えないので、バッファのスライスで要素を切り替える。
            let vertices = mesh.vertex_offset..mesh.vertex_offset + mesh.vertex_size;
            pass.set_vertex_buffer(0, geometry.positions.slice(vertices.clone()));
            pass.set_vertex_buffer(1, geometry.uvs.slice(vertices));
            let index_size = u64::from(mesh.index_count) * 2;
            pass.set_index_buffer(
                geometry
                    .indices
                    .slice(mesh.index_offset..mesh.index_offset + index_size),
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
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
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for mesh in &model.meshes {
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
        let draws = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe draw uniforms"),
            size: draw_stride * meshes.len().max(1) as u64,
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
            draws,
            draws_bind_group,
            draw_stride,
        }
    }
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
