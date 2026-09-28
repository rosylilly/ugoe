//! 描画パイプラインの生成とキャッシュ。合成モードの BlendState は `docs/spec/rendering.md` の表に従う。

use ugoe_model::BlendMode;

/// 本描画のパイプラインを区別するキー。
///
/// マスクの有無と反転はユニフォーム (`mask_flags`) で切り替えるので、キーには含めない
/// (`docs/spec/rendering.md` の「GPU リソースの方針」)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PipelineKey {
    pub blend: BlendMode,
    /// 裏面カリングを行う場合の表の向き (クリップ座標で判定)。`None` ならカリングしない。
    pub cull_front_face: Option<wgpu::FrontFace>,
}

/// 共有のバインドグループレイアウトと、キーごとに作ったパイプライン。
#[derive(Debug)]
pub(crate) struct Pipelines {
    module: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    // 組み合わせは高々数十なので線形探索で十分。
    cache: Vec<(PipelineKey, wgpu::RenderPipeline)>,
    /// マスクパス用 (マスクアトラスへの加算合成)。1 種類だけなので最初に作る。
    mask: wgpu::RenderPipeline,
}

/// マスクアトラスのテクスチャ形式。
pub(crate) const MASK_ATLAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// 頂点 1 つ分の位置 (スロット 0) と UV (スロット 1) の大きさ。どちらも `vec2<f32>`。
pub(crate) const VEC2_SIZE: u64 = 8;
const POSITION_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
const UV_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![1 => Float32x2];

impl Pipelines {
    /// `layouts` は本描画のバインドグループレイアウト (group 0..4)。
    /// マスクパスは先頭の 3 つ (globals・要素ごとのユニフォーム・テクスチャ) だけを使う。
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        layouts: &[&wgpu::BindGroupLayout; 4],
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ugoe drawable shader"),
            source: wgpu::ShaderSource::Wgsl(crate::DRAWABLE_WGSL.into()),
        });
        let layouts: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().copied().map(Some).collect();
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ugoe drawable pipeline layout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let mask_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ugoe mask pipeline layout"),
            bind_group_layouts: &layouts[..3],
            immediate_size: 0,
        });
        // 加算合成。RGBA8 に書くので和は 1 に飽和する (和集合 min(1, Σ))。
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let mask = create_pipeline(
            device,
            &PipelineDesc {
                label: "ugoe mask pipeline",
                layout: &mask_layout,
                module: &module,
                vs: "vs_mask",
                fs: "fs_mask",
                format: MASK_ATLAS_FORMAT,
                blend: wgpu::BlendState {
                    color: add,
                    alpha: add,
                },
                // マスクはカリングしない (マスク用メッシュの `culling` は本描画にだけ効く)。
                cull_front_face: None,
            },
        );
        Self {
            module,
            layout,
            target_format,
            cache: Vec::new(),
            mask,
        }
    }

    pub fn mask(&self) -> &wgpu::RenderPipeline {
        &self.mask
    }

    /// キーに対応するパイプラインを (なければ作って) 用意する。
    pub fn prepare(&mut self, device: &wgpu::Device, key: PipelineKey) {
        if self.get(key).is_none() {
            let pipeline = self.create(device, key);
            self.cache.push((key, pipeline));
        }
    }

    pub fn get(&self, key: PipelineKey) -> Option<&wgpu::RenderPipeline> {
        self.cache.iter().find(|(k, _)| *k == key).map(|(_, p)| p)
    }

    fn create(&self, device: &wgpu::Device, key: PipelineKey) -> wgpu::RenderPipeline {
        create_pipeline(
            device,
            &PipelineDesc {
                label: "ugoe drawable pipeline",
                layout: &self.layout,
                module: &self.module,
                vs: "vs_main",
                fs: "fs_main",
                format: self.target_format,
                blend: blend_state(key.blend),
                cull_front_face: key.cull_front_face,
            },
        )
    }
}

struct PipelineDesc<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    module: &'a wgpu::ShaderModule,
    vs: &'a str,
    fs: &'a str,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
    cull_front_face: Option<wgpu::FrontFace>,
}

fn create_pipeline(device: &wgpu::Device, desc: &PipelineDesc<'_>) -> wgpu::RenderPipeline {
    let vertex_layout = |attributes| wgpu::VertexBufferLayout {
        array_stride: VEC2_SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    };
    let buffers = [
        Some(vertex_layout(&POSITION_ATTRIBUTES)),
        Some(vertex_layout(&UV_ATTRIBUTES)),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(desc.label),
        layout: Some(desc.layout),
        vertex: wgpu::VertexState {
            module: desc.module,
            entry_point: Some(desc.vs),
            compilation_options: Default::default(),
            buffers: &buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: desc.cull_front_face.unwrap_or(wgpu::FrontFace::Ccw),
            cull_mode: desc.cull_front_face.map(|_| wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: desc.module,
            entry_point: Some(desc.fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: desc.format,
                blend: Some(desc.blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// 合成モードに対応する BlendState (`docs/spec/rendering.md` の「合成モード」)。
pub(crate) fn blend_state(mode: BlendMode) -> wgpu::BlendState {
    use wgpu::{BlendComponent, BlendFactor, BlendOperation};
    let component = |src_factor, dst_factor| BlendComponent {
        src_factor,
        dst_factor,
        operation: BlendOperation::Add,
    };
    // 描画先のアルファを保つ (additive / multiply)。
    let keep_dst_alpha = component(BlendFactor::Zero, BlendFactor::One);
    match mode {
        BlendMode::Normal => wgpu::BlendState {
            color: component(BlendFactor::One, BlendFactor::OneMinusSrcAlpha),
            alpha: component(BlendFactor::One, BlendFactor::OneMinusSrcAlpha),
        },
        BlendMode::Additive => wgpu::BlendState {
            color: component(BlendFactor::One, BlendFactor::One),
            alpha: keep_dst_alpha,
        },
        BlendMode::Multiply => wgpu::BlendState {
            color: component(BlendFactor::Dst, BlendFactor::OneMinusSrcAlpha),
            alpha: keep_dst_alpha,
        },
    }
}
