//! 描画パイプラインの生成とキャッシュ。合成モードの BlendState は `docs/spec/rendering.md` の表に従う。

use ugoe_model::BlendMode;

/// パイプラインを区別するキー。
///
/// TODO(T0202): マスクの有無・反転の有無をキーに加える。
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
}

/// 頂点 1 つ分の位置 (スロット 0) と UV (スロット 1) の大きさ。どちらも `vec2<f32>`。
pub(crate) const VEC2_SIZE: u64 = 8;
const POSITION_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
const UV_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![1 => Float32x2];

impl Pipelines {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        layouts: &[&wgpu::BindGroupLayout],
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
        Self {
            module,
            layout,
            target_format,
            cache: Vec::new(),
        }
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
            label: Some("ugoe drawable pipeline"),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &self.module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: key.cull_front_face.unwrap_or(wgpu::FrontFace::Ccw),
                cull_mode: key.cull_front_face.map(|_| wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.target_format,
                    blend: Some(blend_state(key.blend)),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
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
