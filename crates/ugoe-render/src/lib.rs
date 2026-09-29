//! ugoe の参照レンダラ (wgpu)。仕様: `docs/spec/rendering.md`。
//!
//! 設計原則:
//! - **デバイスやサーフェスを所有しない。** ホスト (エディタ・ゲームエンジン・ビューア) が
//!   作った `wgpu::Device` / `Queue` と描画先 `TextureView` を受け取る。
//! - 入力は [`ugoe_runtime::ModelInstance`] の評価結果のみ。モデルの意味論には踏み込まない。
//! - ネイティブ (Vulkan/Metal/DX12/GL) と Web (WebGPU/WebGL2) の双方で動くこと。
//!   WebGL2 で使えない機能 (storage buffer・compute 等) には必ずフォールバックを用意する。

mod lighting;
mod mask;
mod pipeline;
mod upload;

use std::sync::Arc;

use ugoe_runtime::{CompiledModel, ModelInstance};
use wgpu::util::DeviceExt;

use crate::lighting::LightingRaw;
pub use crate::lighting::{Light, Lighting, MAX_LIGHTS};
pub use crate::mask::MaskAtlasOptions;
use crate::mask::{AtlasPool, CellRequest, MaskContent, MaskSlot, MaskStep, MeshMask};
use crate::pipeline::{MASK_ATLAS_FORMAT, PipelineKey, Pipelines, VEC2_SIZE};
use crate::upload::{IndexUploads, PositionUploads, SlotUploads};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererOptions {
    /// マスクアトラスの大きさの上限と、マスクを描く解像度 (`docs/spec/rendering.md` の「精度」)。
    pub mask_atlas: MaskAtlasOptions,
    /// 描画順で隣り合い、描画の状態が同じ要素を 1 回の描画にまとめるか (既定 `true`)。
    /// 結果は変わらない (`docs/spec/rendering.md#描画の最適化`)。`false` は比較や調査用。
    pub batch_draws: bool,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            mask_atlas: MaskAtlasOptions::default(),
            batch_draws: true,
        }
    }
}

/// 直前の [`Renderer::encode`] で行った描画の統計 (テストやプロファイル用)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// 本描画で描いた要素の数。
    pub drawables: u32,
    /// 本描画の描画コマンドの数 (まとめた後。`docs/spec/rendering.md#描画の最適化`)。
    pub draw_calls: u32,
    /// 本描画で、描画の状態 (合成モードとカリング・テクスチャ・マスクアトラス) を前の描画から
    /// 切り替えた回数。最初の描画は数えない。
    pub state_changes: u32,
    /// マスクアトラスに描いたマスクの内容の数 (同じ内容を使う要素は 1 つと数える)。
    pub mask_sets: u32,
    /// マスクパスでマスク用メッシュを描いた回数 (作業領域の全面描画は数えない)。
    pub mask_draws: u32,
    /// 使ったマスクアトラスの枚数。
    pub mask_atlases: u32,
    /// フレームの終わりに持っているマスクアトラスの合計のバイト数 (このフレームで使わなかったが、
    /// 縮小・解放を待っているアトラスも含む。`MaskAtlasOptions::shrink_after_frames`)。
    pub mask_atlas_bytes: u64,
    /// マスクアトラスの内容を前のフレームから使い回したか (マスクパスを省いた。`mask_draws` は 0)。
    pub masks_reused: bool,
    /// 転送した頂点位置のバイト数 (変わっていない要素は転送しない)。
    pub position_bytes: u64,
    /// 転送したユニフォームのバイト数 (要素ごとの値・マスクパス用・ビュー・光源。変わっていないものは転送しない。
    /// 光源はライティングを受ける要素を描くフレームだけ転送する)。
    pub uniform_bytes: u64,
    /// 転送した本描画のインデックスのバイト数 (描画順や表示が変わったときだけ転送する)。
    pub index_bytes: u64,
    /// 転送したバイト数の合計 (`position_bytes + uniform_bytes + index_bytes`)。
    pub uploaded_bytes: u64,
}

/// シェーダの `Globals` と同じ配置 (mat3x3 の各列は vec4 に詰める)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsRaw {
    view_proj: [[f32; 4]; 3],
}

/// 本描画の要素 1 つ分の値。シェーダの `element_data` の texel の並びと同じ。
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ElementRaw {
    multiply_color: [f32; 4],
    screen_color: [f32; 4],
    mask_rect: [f32; 4],
    mask_channel: [f32; 4],
    mask_params: [f32; 4],
}

/// 要素 1 つ分の texel の数 (シェーダの `ELEMENT_TEXELS`)。
const ELEMENT_TEXELS: u32 = 5;
const ELEMENT_BYTES: usize = std::mem::size_of::<ElementRaw>();
/// 要素ごとの値のテクスチャの 1 行に並べる要素の数の上限。
const ELEMENTS_PER_ROW: u32 = 64;
const ELEMENT_DATA_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// シェーダの `DrawUniforms` (マスクパス) と同じ配置。
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
const LIGHTING_SIZE: u64 = std::mem::size_of::<LightingRaw>() as u64;

/// 要素ごとのライティングの種類 (`mask_params.z`。シェーダの `MaskRead::params` の z)。
const LIGHT_NONE: f32 = 0.0;
/// 法線を `(0, 0, 1)` とする (法線マップが登録されていない)。
const LIGHT_FLAT: f32 = 1.0;
/// 法線マップを読む。
const LIGHT_NORMAL_MAP: f32 = 2.0;

/// モデルの静的な頂点データ (UV・要素の番号・インデックス) と、フレームごとに書き換えるバッファ。
#[derive(Debug)]
struct Geometry {
    /// どのモデルのデータか (インスタンスが別モデルに変わったら作り直す)。
    model: Arc<CompiledModel>,
    positions: wgpu::Buffer,
    uvs: wgpu::Buffer,
    /// 頂点ごとの要素の番号 (本描画で要素ごとの値を引く)。
    elements: wgpu::Buffer,
    /// 全要素のインデックスを宣言順に並べたもの (全体の頂点番号)。マスクパスで使う。
    indices: wgpu::Buffer,
    /// `indices` と同じ内容 (本描画のインデックスを組み立てる)。
    index_data: Vec<u32>,
    /// 本描画のインデックス。描く要素のインデックスを描画順に並べ、変わったときだけ書き換える。
    draw_indices: wgpu::Buffer,
    meshes: Vec<MeshRange>,
    /// 全要素の頂点数の合計。
    vertex_count: usize,
    /// 要素ごとの値 (`ElementRaw`) を並べた `Rgba32Float` テクスチャ。
    element_data: wgpu::Texture,
    /// ビュー (binding 0) と要素ごとの値 (binding 1)。
    globals_bind_group: wgpu::BindGroup,
    /// マスクの内容。モデルの静的なデータなので、ロード時に一度だけ求める。
    masks: Vec<MaskContent>,
    /// マスクの内容のインデックスを、レベルの大きい順に並べたもの (外接矩形の伝播に使う)。
    masks_by_level_desc: Vec<u32>,
    /// 内容 `c` の `Draw` の段のユニフォームを置く最初のスロット。
    mask_slot_base: Vec<u32>,
    /// マスクパスでマスク用メッシュを描く分のユニフォームを、内容ごとに並べたもの。
    /// スロット `i` はオフセット `i * draw_stride`。
    draws: wgpu::Buffer,
    draws_bind_group: wgpu::BindGroup,
    draw_stride: u64,
}

/// 共有バッファ内での 1 メッシュ分の範囲。
#[derive(Debug, Clone, Copy)]
struct MeshRange {
    /// `Geometry::indices` での最初のインデックスの位置。
    first_index: u32,
    index_count: u32,
    texture: u32,
    culling: bool,
    /// ライティングを受けるか (`Model::lighting` と `Mesh::lit` の両方が `true`)。
    lit: bool,
    /// 自身のマスクとして読む内容。マスクを持たないなら `None`。
    mask: Option<MeshMask>,
}

impl MeshRange {
    /// `Geometry::indices` でのインデックスの範囲。
    fn indices(&self) -> std::ops::Range<u32> {
        self.first_index..self.first_index + self.index_count
    }

    fn indices_usize(&self) -> std::ops::Range<usize> {
        self.first_index as usize..(self.first_index + self.index_count) as usize
    }
}

/// 本描画の描画の状態 (`docs/spec/rendering.md#描画の最適化`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrawState {
    /// 合成モードとカリング。
    key: PipelineKey,
    texture: u32,
    /// 読むマスクアトラス。マスクを読まない要素は `None` で、どのアトラスとも両立する
    /// (マスクの強さが 0 なので、アトラスの値は結果に効かない)。
    mask_atlas: Option<u32>,
}

impl DrawState {
    /// `self` の描画に続けて `next` を同じ描画で描けるなら、まとめた状態を返す。
    fn merge(self, next: Self) -> Option<Self> {
        let mask_atlas = match (self.mask_atlas, next.mask_atlas) {
            (Some(a), Some(b)) if a != b => return None,
            (a, b) => a.or(b),
        };
        (self.key == next.key && self.texture == next.texture)
            .then_some(Self { mask_atlas, ..self })
    }
}

/// 本描画の 1 回の描画。`first_index` は `Geometry::draw_indices` での位置。
#[derive(Debug, Clone, Copy)]
struct DrawCall {
    state: DrawState,
    first_index: u32,
    index_count: u32,
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
#[derive(Debug, Clone, Copy, PartialEq)]
struct ActiveMask {
    /// `Geometry::masks` のインデックス。
    content: u32,
    slot: MaskSlot,
    /// 画面 UV → アトラス UV の変換 (`mask_rect` ユニフォーム)。
    transform: [f32; 4],
}

/// マスクアトラスに今描かれている内容の記録 (変わらなければマスクパスを省く)。
///
/// ここにない入力 (マスク用メッシュの頂点位置とマスクパスのユニフォーム) は、
/// そのフレームで転送したかどうかで変化を判断する。
#[derive(Debug, Default)]
struct MaskFrame {
    atlas_size: u32,
    view_proj: [[f32; 3]; 3],
    texture_generation: u64,
    active: Vec<ActiveMask>,
}

#[derive(Debug)]
struct TextureSlot {
    /// 法線マップを登録し直したときに、バインドグループを作り直すのに使う。
    texture: wgpu::Texture,
    /// テクスチャ・サンプラ・法線マップ (なければダミー)。
    bind_group: wgpu::BindGroup,
}

/// モデル描画器。モデル (テクスチャ集合) 1 つにつき 1 つ作る。
///
/// `encode` はバッファを `Queue::write_buffer` で書き換えるので、
/// 同じ `Renderer` の `encode` は 1 回の `Queue::submit` につき 1 回だけ呼ぶこと。
///
/// 変わっていない頂点位置・ユニフォーム・マスクアトラスは転送し直さない (`docs/spec/rendering.md` の
/// 「転送の省略」)。1 つの `Renderer` で同じモデルの複数のインスタンスを交互に描いてもよく、
/// `encode` の間に `update` を何度呼んでもよい。
#[derive(Debug)]
pub struct Renderer {
    target_format: wgpu::TextureFormat,
    pipelines: Pipelines,
    globals_layout: wgpu::BindGroupLayout,
    draw_layout: wgpu::BindGroupLayout,
    /// モデルのテクスチャ (テクスチャ・サンプラ・法線マップ。group 2)。
    model_texture_layout: wgpu::BindGroupLayout,
    /// マスクアトラス (テクスチャ・サンプラ。group 3)。
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    /// 光源 (`LightingRaw`) のユニフォーム。
    lighting_buffer: wgpu::Buffer,
    /// `set_lighting` で渡された光源。
    lighting: LightingRaw,
    lighting_uploaded: Option<LightingRaw>,
    /// 法線マップのないテクスチャに束縛する 1x1 のダミー (シェーダは値を使わない)。
    flat_normal: wgpu::Texture,
    /// テクスチャの番号ごとの法線マップ。
    normal_maps: Vec<Option<wgpu::Texture>>,
    /// マスクを使わない要素に束縛する 1x1 のダミー (`mask_strength = 0` なので値は効かない)。
    no_mask_bind_group: wgpu::BindGroup,
    mask_options: MaskAtlasOptions,
    batch_draws: bool,
    /// アトラスの一辺の上限 (設定とデバイスの上限の小さい方)。
    mask_atlas_max_size: u32,
    /// アトラスの一辺と枚数 (すぐに大きくし、ヒステリシスを持たせて小さくする)。
    mask_atlas_pool: AtlasPool,
    mask_atlases: Vec<MaskAtlas>,
    stats: FrameStats,
    textures: Vec<Option<TextureSlot>>,
    /// `set_texture` のたびに増やす (マスクアトラスの使い回しの判断に使う)。
    texture_generation: u64,
    geometry: Option<Geometry>,
    /// `geometry` のバッファに転送済みの内容の控え (`docs/spec/rendering.md` の「転送の省略」)。
    position_uploads: PositionUploads,
    element_uploads: SlotUploads,
    draw_uploads: SlotUploads,
    index_uploads: IndexUploads,
    globals_uploaded: Option<GlobalsRaw>,
    /// マスクアトラスの今の内容。`None` なら内容は当てにならない。
    mask_frame: Option<MaskFrame>,
    // フレームごとの書き込み用の作業領域。
    elements_scratch: Vec<u8>,
    draws_scratch: Vec<u8>,
    indices_scratch: Vec<u32>,
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
            entries: &[
                wgpu::BindGroupLayoutEntry {
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
                },
                // 光源 (`docs/spec/rendering.md#ライティング`)。
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(LIGHTING_SIZE),
                    },
                    count: None,
                },
                // 本描画の要素ごとの値 (`textureLoad` で読むのでフィルタしない)。
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
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
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ugoe texture layout"),
            entries: &[texture_entry(0), sampler_entry],
        });
        // モデルのテクスチャは法線マップ (binding 2) を同じサンプラで読む。
        let model_texture_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ugoe model texture layout"),
                entries: &[texture_entry(0), sampler_entry, texture_entry(2)],
            });
        let pipelines = Pipelines::new(
            device,
            target_format,
            &[
                &globals_layout,
                &draw_layout,
                &model_texture_layout,
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
        let lighting = LightingRaw::new(&Lighting::default());
        let lighting_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe lighting"),
            size: LIGHTING_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let flat_normal = device.create_texture(&texture_descriptor("ugoe flat normal", 1, 1));
        // 内容は使われない (mask_strength = 0)。wgpu は未書き込みのテクスチャをゼロで初期化する。
        let no_mask_texture = device.create_texture(&texture_descriptor("ugoe no mask", 1, 1));
        let no_mask_bind_group =
            texture_bind_group(device, &texture_layout, &no_mask_texture, &sampler);
        let mask_atlas_max_size =
            mask::max_atlas_size(options.mask_atlas, device.limits().max_texture_dimension_2d);

        Self {
            target_format,
            pipelines,
            globals_layout,
            draw_layout,
            model_texture_layout,
            texture_layout,
            sampler,
            globals,
            lighting_buffer,
            lighting,
            lighting_uploaded: None,
            flat_normal,
            normal_maps: Vec::new(),
            no_mask_bind_group,
            mask_options: options.mask_atlas,
            batch_draws: options.batch_draws,
            mask_atlas_max_size,
            mask_atlas_pool: AtlasPool::default(),
            mask_atlases: Vec::new(),
            stats: FrameStats::default(),
            textures: Vec::new(),
            texture_generation: 0,
            geometry: None,
            position_uploads: PositionUploads::default(),
            element_uploads: SlotUploads::default(),
            draw_uploads: SlotUploads::default(),
            index_uploads: IndexUploads::default(),
            globals_uploaded: None,
            mask_frame: None,
            elements_scratch: Vec::new(),
            draws_scratch: Vec::new(),
            indices_scratch: Vec::new(),
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
        let bind_group = self.model_texture_bind_group(device, index, &texture);
        let index = index as usize;
        if self.textures.len() <= index {
            self.textures.resize_with(index + 1, || None);
        }
        self.textures[index] = Some(TextureSlot {
            texture,
            bind_group,
        });
        self.texture_generation += 1;
        Ok(())
    }

    /// テクスチャ `index` の法線マップ (RGBA8、アルファは使わない) を登録する。同じ `index` に再登録すると置き換える。
    ///
    /// 値は接空間法線 `n = 2c − 1` で、G は画像の上向き (`docs/spec/rendering.md#ライティング`)。
    /// 大きさはテクスチャと違ってもよい (同じ UV 空間で読む)。テクスチャより先に登録してもよい。
    pub fn set_normal_map(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), RenderError> {
        let expected = width as usize * height as usize * 4;
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err(RenderError::InvalidTextureData {
                width,
                height,
                expected,
                actual: rgba.len(),
            });
        }
        let normal_map = device.create_texture_with_data(
            queue,
            &texture_descriptor("ugoe normal map", width, height),
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba,
        );
        let slot = index as usize;
        if self.normal_maps.len() <= slot {
            self.normal_maps.resize_with(slot + 1, || None);
        }
        self.normal_maps[slot] = Some(normal_map);
        self.rebind_texture(device, index);
        Ok(())
    }

    /// テクスチャ `index` の法線マップを外す (法線は `(0, 0, 1)` になる)。登録されていなければ何もしない。
    pub fn remove_normal_map(&mut self, device: &wgpu::Device, index: u32) {
        if let Some(normal_map) = self.normal_maps.get_mut(index as usize)
            && normal_map.take().is_some()
        {
            self.rebind_texture(device, index);
        }
    }

    /// 光源を設定する (`docs/spec/rendering.md#ライティング`)。次の [`Renderer::encode`] から使う。
    ///
    /// ライティングを受けるのは `Model::lighting` と `Mesh::lit` の両方が `true` のメッシュだけ。
    /// 設定しなければ [`Lighting::default`] (環境光が白で光源なし。ライティングしない場合と同じ色になる)。
    pub fn set_lighting(&mut self, lighting: &Lighting) {
        self.lighting = LightingRaw::new(lighting);
    }

    /// テクスチャ `index` のバインドグループ (テクスチャ・サンプラ・法線マップ) を作る。
    fn model_texture_bind_group(
        &self,
        device: &wgpu::Device,
        index: u32,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let normal_map = self.normal_map(index).unwrap_or(&self.flat_normal);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_view = normal_map.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ugoe model texture"),
            layout: &self.model_texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
            ],
        })
    }

    /// 法線マップが変わったテクスチャ `index` のバインドグループを作り直す (テクスチャが未登録なら何もしない)。
    fn rebind_texture(&mut self, device: &wgpu::Device, index: u32) {
        let Some(slot) = self.texture(index) else {
            return;
        };
        let bind_group = self.model_texture_bind_group(device, index, &slot.texture);
        if let Some(Some(slot)) = self.textures.get_mut(index as usize) {
            slot.bind_group = bind_group;
        }
    }

    fn normal_map(&self, index: u32) -> Option<&wgpu::Texture> {
        self.normal_maps
            .get(index as usize)
            .and_then(Option::as_ref)
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
            let layouts = GeometryLayouts {
                globals: &self.globals_layout,
                globals_buffer: &self.globals,
                lighting_buffer: &self.lighting_buffer,
                draw: &self.draw_layout,
            };
            let geometry = Geometry::new(device, &layouts, instance.compiled());
            self.position_uploads.reset(geometry.vertex_count);
            self.element_uploads.reset();
            self.draw_uploads.reset();
            self.index_uploads.reset();
            self.mask_frame = None;
            self.geometry = Some(geometry);
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
        // このフレームだけで要る一辺を求め、縮小のヒステリシスを通して詰める一辺を決める。
        let shrink_after = self.mask_options.shrink_after_frames;
        let (needed, slots) = if requests.is_empty() {
            (0, Vec::new())
        } else {
            mask::allocate(&requests, 0, self.mask_atlas_max_size)
        };
        let frame_size = self.mask_atlas_pool.frame_size(needed, shrink_after);
        let (atlas_size, slots) = if frame_size == needed || requests.is_empty() {
            (frame_size, slots)
        } else {
            mask::allocate(&requests, frame_size, self.mask_atlas_max_size)
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
        // 作り直したアトラスの内容は当てにならない。
        let mut atlases_changed = false;
        if atlas_size != self.mask_atlas_pool.size() {
            self.mask_atlases.clear();
            atlases_changed = true;
        }
        let keep = self
            .mask_atlas_pool
            .commit(atlas_size, atlas_count as u32, shrink_after) as usize;
        self.mask_atlases.truncate(keep);
        while self.mask_atlases.len() < atlas_count {
            self.mask_atlases.push(MaskAtlas::new(
                device,
                &self.texture_layout,
                &self.sampler,
                atlas_size,
            ));
            atlases_changed = true;
        }

        // 頂点位置 (変わった要素だけ)。
        let position_bytes = self.position_uploads.upload(instance, |offset, data| {
            queue.write_buffer(&geometry.positions, offset, data);
        });

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

        // 本描画の要素ごとの値 (頂点の要素の番号で引くので、描画の状態の違いにならない)。
        // ライティングの種類は `mask_params.z` (シェーダの `MaskRead::params`) に置く。
        self.elements_scratch.clear();
        for (i, state) in drawables.iter().enumerate() {
            let mut read = mask_read(i);
            let mesh = geometry.meshes[i];
            read.params[2] = match (mesh.lit, self.normal_map(mesh.texture)) {
                (false, _) => LIGHT_NONE,
                (true, None) => LIGHT_FLAT,
                (true, Some(_)) => LIGHT_NORMAL_MAP,
            };
            let raw = ElementRaw {
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
            };
            self.elements_scratch
                .extend_from_slice(bytemuck::bytes_of(&raw));
        }
        let element_writes =
            self.element_uploads
                .upload(&self.elements_scratch, ELEMENT_BYTES, |offset, data| {
                    geometry.write_elements(queue, offset as usize / ELEMENT_BYTES, data);
                });

        // マスクパスで描くマスク用メッシュのユニフォーム。
        let stride = geometry.draw_stride as usize;
        self.draws_scratch.clear();
        self.draws_scratch.resize(geometry.draws.size() as usize, 0);
        let mut write_draw = |slot: usize, raw: DrawUniformsRaw| {
            let start = slot * stride;
            self.draws_scratch[start..start + DRAW_UNIFORMS_SIZE as usize]
                .copy_from_slice(bytemuck::bytes_of(&raw));
        };
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
        let draw_writes = self
            .draw_uploads
            .upload(&self.draws_scratch, stride, |offset, data| {
                queue.write_buffer(&geometry.draws, offset, data);
            });
        let mut uniform_bytes = element_writes.bytes + draw_writes.bytes;

        let globals = GlobalsRaw {
            view_proj: m.map(|c| [c[0], c[1], c[2], 0.0]),
        };
        if self.globals_uploaded != Some(globals) {
            queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
            self.globals_uploaded = Some(globals);
            uniform_bytes += std::mem::size_of::<GlobalsRaw>() as u64;
        }
        // 光源は、ライティングを受ける要素を描くときだけ要る。
        let lit = draws.iter().any(|&(i, _)| geometry.meshes[i].lit);
        if lit && self.lighting_uploaded != Some(self.lighting) {
            queue.write_buffer(&self.lighting_buffer, 0, bytemuck::bytes_of(&self.lighting));
            self.lighting_uploaded = Some(self.lighting);
            uniform_bytes += LIGHTING_SIZE;
        }

        // マスクアトラスの内容の使い回し: アトラス・割り当て・ビュー・テクスチャが前と同じで、
        // マスクパスのユニフォームとマスク用メッシュの頂点位置を転送していなければ、描く内容も同じ。
        let mask_uniforms_written = draw_writes.last_slot.is_some();
        let members_written = active.iter().any(|a| {
            geometry.masks[a.content as usize]
                .members
                .iter()
                .any(|&member| self.position_uploads.written(member as usize))
        });
        let masks_reused = !active.is_empty()
            && !atlases_changed
            && !mask_uniforms_written
            && !members_written
            && self.mask_frame.as_ref().is_some_and(|f| {
                f.atlas_size == atlas_size
                    && f.view_proj == m
                    && f.texture_generation == self.texture_generation
                    && f.active == active
            });
        let mask_frame = self.mask_frame.get_or_insert_with(MaskFrame::default);
        mask_frame.atlas_size = atlas_size;
        mask_frame.view_proj = m;
        mask_frame.texture_generation = self.texture_generation;
        mask_frame.active.clear();
        mask_frame.active.extend_from_slice(&active);
        let mask_atlases = if masks_reused {
            &[][..]
        } else {
            &self.mask_atlases[..atlas_count]
        };

        // マスクパス: アトラスごとに 1 パス (レベルの小さいアトラスから)。各内容はシザー矩形で
        // 自分のセルだけに描く。アトラスは 1 でクリアし、乗算合成で値を作る。
        let pipelines = self.pipelines.mask();
        let mut mask_draws = 0;
        for (index, atlas) in mask_atlases.iter().enumerate() {
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
            // マスク用メッシュは宣言順のインデックスで 1 つずつ描く。
            pass.set_vertex_buffer(0, geometry.positions.slice(..));
            pass.set_vertex_buffer(1, geometry.uvs.slice(..));
            pass.set_index_buffer(geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
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
                            pass.set_bind_group(0, &geometry.globals_bind_group, &[]);
                            pass.set_bind_group(
                                1,
                                &geometry.draws_bind_group,
                                &[(slot * stride) as u32],
                            );
                            pass.set_bind_group(2, &texture.bind_group, &[]);
                            pass.set_bind_group(3, own_mask, &[]);
                            pass.draw_indexed(mesh.indices(), 0, 0..1);
                            mask_draws += 1;
                        }
                    }
                }
            }
        }

        // 本描画: 描画順で隣り合い、描画の状態が同じ要素を 1 回の描画にまとめる
        // (`docs/spec/rendering.md#描画の最適化`)。インデックスは描く要素のものを描画順に並べる。
        self.indices_scratch.clear();
        let mut calls: Vec<DrawCall> = Vec::new();
        let mut state_changes = 0;
        for &(i, key) in &draws {
            let mesh = geometry.meshes[i];
            let state = DrawState {
                key,
                texture: mesh.texture,
                mask_atlas: mesh
                    .mask
                    .and_then(|m| active_of[m.content as usize])
                    .map(|a| active[a].slot.atlas),
            };
            let first_index = self.indices_scratch.len() as u32;
            self.indices_scratch
                .extend_from_slice(&geometry.index_data[mesh.indices_usize()]);
            let merged = calls.last().and_then(|last| last.state.merge(state));
            if let (Some(last), Some(merged), true) = (calls.last_mut(), merged, self.batch_draws) {
                last.state = merged;
                last.index_count += mesh.index_count;
                continue;
            }
            if merged.is_none() && !calls.is_empty() {
                state_changes += 1;
            }
            calls.push(DrawCall {
                state,
                first_index,
                index_count: mesh.index_count,
            });
        }
        let index_bytes = self
            .index_uploads
            .upload(&self.indices_scratch, |offset, data| {
                queue.write_buffer(&geometry.draw_indices, offset, data);
            });

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
        if !calls.is_empty() {
            pass.set_bind_group(0, &geometry.globals_bind_group, &[]);
            // 本描画のシェーダはユニフォームを読まないが、レイアウトを共有しているので束縛しておく。
            pass.set_bind_group(1, &geometry.draws_bind_group, &[0]);
            pass.set_vertex_buffer(0, geometry.positions.slice(..));
            pass.set_vertex_buffer(1, geometry.uvs.slice(..));
            pass.set_vertex_buffer(2, geometry.elements.slice(..));
            pass.set_index_buffer(geometry.draw_indices.slice(..), wgpu::IndexFormat::Uint32);
        }
        // 束縛中の状態 (`mask_atlas` は束縛中のアトラス。`None` はダミー)。変わったものだけ束縛し直す。
        let mut bound: Option<DrawState> = None;
        for call in &calls {
            let state = call.state;
            let (Some(pipeline), Some(texture)) =
                (self.pipelines.get(state.key), self.texture(state.texture))
            else {
                continue;
            };
            if bound.is_none_or(|b| b.key != state.key) {
                pass.set_pipeline(pipeline);
            }
            if bound.is_none_or(|b| b.texture != state.texture) {
                pass.set_bind_group(2, &texture.bind_group, &[]);
            }
            // マスクを読まない描画 (`None`) は、どのアトラスが束縛されていてもよい。
            let mask_atlas = match bound {
                Some(b) if state.mask_atlas.is_none() || state.mask_atlas == b.mask_atlas => {
                    b.mask_atlas
                }
                _ => {
                    let mask_bind_group = match state.mask_atlas {
                        Some(atlas) => &self.mask_atlases[atlas as usize].bind_group,
                        None => &self.no_mask_bind_group,
                    };
                    pass.set_bind_group(3, mask_bind_group, &[]);
                    state.mask_atlas
                }
            };
            bound = Some(DrawState {
                mask_atlas,
                ..state
            });
            pass.draw_indexed(
                call.first_index..call.first_index + call.index_count,
                0,
                0..1,
            );
        }

        self.stats = FrameStats {
            drawables: draws.len() as u32,
            draw_calls: calls.len() as u32,
            state_changes,
            mask_sets: active.len() as u32,
            mask_draws,
            mask_atlases: atlas_count as u32,
            mask_atlas_bytes: self.mask_atlas_pool.bytes(),
            masks_reused,
            position_bytes,
            uniform_bytes,
            index_bytes,
            uploaded_bytes: position_bytes + uniform_bytes + index_bytes,
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
        layouts: &GeometryLayouts<'_>,
        compiled: &Arc<CompiledModel>,
    ) -> Self {
        let model = compiled.model();
        let (mesh_masks, masks) = mask::build(compiled);
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut elements: Vec<u32> = Vec::new();
        let mut index_data: Vec<u32> = Vec::new();
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for (i, (mesh, &mask)) in model.meshes.iter().zip(&mesh_masks).enumerate() {
            let base = uvs.len() as u32;
            let first_index = index_data.len() as u32;
            uvs.extend(mesh.uvs.iter().map(|uv| [uv.x, uv.y]));
            elements.resize(uvs.len(), i as u32);
            // 頂点のないメッシュは描かない (インデックスが指す頂点がない)。
            let index_count = if mesh.uvs.is_empty() {
                0
            } else {
                index_data.extend(mesh.indices.iter().map(|&k| base + u32::from(k)));
                mesh.indices.len() as u32
            };
            meshes.push(MeshRange {
                first_index,
                index_count,
                texture: mesh.texture,
                culling: mesh.culling,
                lit: model.lighting && mesh.lit,
                mask,
            });
        }

        // 空のバッファは束縛できないので、最低 1 つ分の大きさにする (中身は使わない)。
        let vertex_count = uvs.len();
        let positions = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe positions"),
            size: vertex_count.max(1) as u64 * VEC2_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let static_buffer = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        uvs.resize(vertex_count.max(1), [0.0; 2]);
        elements.resize(vertex_count.max(1), 0);
        let uvs = static_buffer(
            "ugoe uvs",
            bytemuck::cast_slice(&uvs),
            wgpu::BufferUsages::VERTEX,
        );
        let elements = static_buffer(
            "ugoe element indices",
            bytemuck::cast_slice(&elements),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = static_buffer(
            "ugoe indices",
            bytemuck::cast_slice(if index_data.is_empty() {
                &[0]
            } else {
                &index_data[..]
            }),
            wgpu::BufferUsages::INDEX,
        );
        // 各要素は 1 フレームに高々 1 回描くので、全要素のインデックスの数で足りる。
        let draw_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe draw indices"),
            size: indices.size(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let element_count = model.meshes.len().max(1) as u32;
        let per_row = element_count.min(ELEMENTS_PER_ROW);
        let element_data = device.create_texture(&wgpu::TextureDescriptor {
            format: ELEMENT_DATA_FORMAT,
            ..texture_descriptor(
                "ugoe element data",
                per_row * ELEMENT_TEXELS,
                element_count.div_ceil(per_row),
            )
        });
        let element_view = element_data.create_view(&wgpu::TextureViewDescriptor::default());
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ugoe globals"),
            layout: layouts.globals,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: layouts.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&element_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: layouts.lighting_buffer.as_entire_binding(),
                },
            ],
        });

        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let draw_stride = DRAW_UNIFORMS_SIZE.div_ceil(alignment) * alignment;
        let mut mask_slot_base = Vec::with_capacity(masks.len());
        let mut slots = 0;
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
            layout: layouts.draw,
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
            elements,
            indices,
            index_data,
            draw_indices,
            meshes,
            vertex_count,
            element_data,
            globals_bind_group,
            masks,
            masks_by_level_desc,
            mask_slot_base,
            draws,
            draws_bind_group,
            draw_stride,
        }
    }

    /// 要素ごとの値のテクスチャの 1 行の要素数。
    fn elements_per_row(&self) -> u32 {
        self.element_data.width() / ELEMENT_TEXELS
    }

    /// 要素ごとの値 (`ElementRaw` の列) のうち、要素 `first` から始まる `data` をテクスチャに書く。
    fn write_elements(&self, queue: &wgpu::Queue, first: usize, data: &[u8]) {
        let per_row = self.elements_per_row() as usize;
        let mut element = first;
        let mut rest = data;
        while !rest.is_empty() {
            let (row, column) = (element / per_row, element % per_row);
            let count = (per_row - column).min(rest.len() / ELEMENT_BYTES);
            let (head, tail) = rest.split_at(count * ELEMENT_BYTES);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.element_data,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: column as u32 * ELEMENT_TEXELS,
                        y: row as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                head,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(head.len() as u32),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: count as u32 * ELEMENT_TEXELS,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            element += count;
            rest = tail;
        }
    }
}

/// [`Geometry::new`] がバインドグループを作るのに使うレイアウトとバッファ。
struct GeometryLayouts<'a> {
    globals: &'a wgpu::BindGroupLayout,
    globals_buffer: &'a wgpu::Buffer,
    lighting_buffer: &'a wgpu::Buffer,
    draw: &'a wgpu::BindGroupLayout,
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

    /// WebGL2 (GLSL ES 3.00) に変換できる (ストレージバッファ・compute などを使っていない)。
    #[test]
    fn reference_shader_translates_to_webgl2() {
        use naga::back::glsl;
        let module = naga::front::wgsl::parse_str(super::DRAWABLE_WGSL).expect("parse");
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("validate");
        let options = glsl::Options {
            version: glsl::Version::Embedded {
                version: 300,
                is_webgl: true,
            },
            ..Default::default()
        };
        for (stage, entry_point) in [
            (naga::ShaderStage::Vertex, "vs_main"),
            (naga::ShaderStage::Fragment, "fs_main"),
            (naga::ShaderStage::Vertex, "vs_mask"),
            (naga::ShaderStage::Fragment, "fs_mask"),
            (naga::ShaderStage::Vertex, "vs_fill"),
            (naga::ShaderStage::Fragment, "fs_fill"),
        ] {
            let pipeline_options = glsl::PipelineOptions {
                shader_stage: stage,
                entry_point: entry_point.into(),
                multiview: None,
            };
            let mut out = String::new();
            glsl::Writer::new(
                &mut out,
                &module,
                &info,
                &options,
                &pipeline_options,
                naga::proc::BoundsCheckPolicies::default(),
            )
            .and_then(|mut writer| writer.write())
            .unwrap_or_else(|e| panic!("{entry_point}: {e}"));
        }
    }
}
