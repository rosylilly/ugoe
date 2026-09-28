//! オフスクリーンに描画して画素を読み戻す描画テスト。`just test-gpu` で実行する。
//!
//! GPU (またはソフトウェアラスタライザの lavapipe など) が見つからない場合は、
//! 黙って飛ばさずに失敗する (feature を明示的に有効にしたときだけ実行されるため)。
#![cfg(feature = "gpu-tests")]

use std::sync::OnceLock;

use ugoe_core::{Rgb, Vec2};
use ugoe_model::mesh::Mesh;
use ugoe_model::{BlendMode, Model, fixtures};
use ugoe_render::{FrameStats, MaskAtlasOptions, RenderError, Renderer, RendererOptions, View};
use ugoe_runtime::{CompiledModel, ModelInstance};

const SIZE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// 8bit 量子化とラスタライザの差を吸収する許容誤差 (0..255)。
const TOLERANCE: i32 = 2;

/// 画素 (16..48 が single_quad の四角形の内側) の代表点。
const INSIDE: (u32, u32) = (32, 32);
const OUTSIDE: (u32, u32) = (4, 4);

struct Gpu {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// テスト全体で 1 つのデバイスを共有する。
///
/// static に置いて破棄しないのは、WSL2 の dzn (Vulkan on D3D12) ドライバが
/// インスタンスの破棄時に SIGSEGV を起こすため (テスト自体は通る)。
fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(create_gpu)
}

fn create_gpu() -> Gpu {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    )
    .unwrap_or_else(|e| {
        panic!(
            "no wgpu adapter available ({e}). GPU tests need a GPU or a software rasterizer \
                 such as Mesa lavapipe (Vulkan) or llvmpipe (GL)"
        )
    });
    eprintln!(
        "ugoe-render gpu tests: using adapter {:?}",
        adapter.get_info()
    );
    // WebGL2 の制限の範囲で動くことも確かめる。
    let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ugoe-render gpu test"),
        required_limits: limits,
        ..Default::default()
    }))
    .expect("request device");
    Gpu {
        _instance: instance,
        device,
        queue,
    }
}

/// キャンバス 256x256 (single_quad) をクリップ座標全体に写すビュー。
fn canvas_view(flip_y: bool) -> View {
    let (sy, ty) = if flip_y { (-1.0, 1.0) } else { (1.0, -1.0) };
    View {
        view_proj: [
            [1.0 / 128.0, 0.0, 0.0],
            [0.0, sy / 128.0, 0.0],
            [-1.0, ty, 1.0],
        ],
    }
}

fn normal_view() -> View {
    canvas_view(true)
}

struct Scene {
    model: Model,
    /// テクスチャ 0 (1x1)。`[0; 4]` なら登録しない。
    texture: [u8; 4],
    /// テクスチャ 1 以降 (1x1)。
    extra_textures: Vec<[u8; 4]>,
    clear: wgpu::Color,
    view: View,
    options: RendererOptions,
}

impl Scene {
    fn new(model: Model) -> Self {
        Self {
            model,
            texture: [255; 4],
            extra_textures: Vec::new(),
            clear: wgpu::Color::TRANSPARENT,
            view: normal_view(),
            options: RendererOptions::default(),
        }
    }

    fn render(self) -> Image {
        self.try_render().expect("render")
    }

    fn try_render(self) -> Result<Image, RenderError> {
        let Gpu { device, queue, .. } = gpu();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
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

        let instance = ModelInstance::new(CompiledModel::compile(self.model).expect("compile"));
        let mut renderer = Renderer::with_options(device, FORMAT, self.options);
        if self.texture != [0; 4] {
            renderer.set_texture(device, queue, 0, 1, 1, &self.texture)?;
        }
        for (i, texture) in self.extra_textures.iter().enumerate() {
            renderer.set_texture(device, queue, i as u32 + 1, 1, 1, texture)?;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // クリアはホストの責務。
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.encode(
            device,
            queue,
            &mut encoder,
            &target_view,
            &self.view,
            &instance,
        )?;

        let bytes_per_row = SIZE * 4; // 256 = COPY_BYTES_PER_ROW_ALIGNMENT
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * SIZE),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(SIZE),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("map readback"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let pixels = readback
            .get_mapped_range(..)
            .expect("mapped range")
            .to_vec();
        Ok(Image {
            pixels,
            stats: renderer.last_frame_stats(),
        })
    }
}

struct Image {
    pixels: Vec<u8>,
    stats: FrameStats,
}

impl Image {
    fn at(&self, (x, y): (u32, u32)) -> [u8; 4] {
        let i = ((y * SIZE + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().expect("4 bytes")
    }

    /// `expected` は 0..1 の乗算済み RGBA。
    #[track_caller]
    fn assert_pixel(&self, at: (u32, u32), expected: [f32; 4]) {
        let actual = self.at(at);
        let expected = expected.map(|v| (v * 255.0).round() as i32);
        let ok = actual
            .iter()
            .zip(expected)
            .all(|(&a, e)| (i32::from(a) - e).abs() <= TOLERANCE);
        assert!(
            ok,
            "pixel {at:?}: expected {expected:?}, got {actual:?} (tolerance {TOLERANCE})"
        );
    }

    fn covered_pixels(&self, background: [u8; 4]) -> usize {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|&&p| p != background)
            .count()
    }
}

fn with_keyforms(mut model: Model, f: impl Fn(&mut ugoe_model::mesh::MeshKeyform)) -> Model {
    for kf in &mut model.meshes[0].keyforms.keyforms {
        f(kf);
    }
    model
}

fn clear_color([r, g, b, a]: [f64; 4]) -> wgpu::Color {
    wgpu::Color { r, g, b, a }
}

#[test]
fn normal_blend_covers_quad() {
    let image = Scene::new(fixtures::single_quad()).render();
    image.assert_pixel(INSIDE, [1.0; 4]);
    image.assert_pixel(OUTSIDE, [0.0; 4]);
    // キャンバス 64..192 → 画素 16..48 の 32x32。
    assert_eq!(image.covered_pixels([0; 4]), 32 * 32);
    image.assert_pixel((16, 16), [1.0; 4]);
    image.assert_pixel((47, 47), [1.0; 4]);
    image.assert_pixel((15, 16), [0.0; 4]);
    image.assert_pixel((48, 47), [0.0; 4]);
}

#[test]
fn opacity_scales_premultiplied_output() {
    let model = with_keyforms(fixtures::single_quad(), |k| k.opacity = 0.5);
    let mut scene = Scene::new(model);
    scene.clear = clear_color([0.0, 0.0, 1.0, 1.0]);
    let image = scene.render();
    // 0.5 の白を青の上に normal 合成。
    image.assert_pixel(INSIDE, [0.5, 0.5, 1.0, 1.0]);
    image.assert_pixel(OUTSIDE, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn invisible_drawable_is_skipped() {
    let model = with_keyforms(fixtures::single_quad(), |k| k.opacity = 0.0);
    let mut scene = Scene::new(model);
    // テクスチャが未登録でも、描かない要素ならエラーにならない。
    scene.texture = [0; 4];
    let image = scene.render();
    assert_eq!(image.covered_pixels([0; 4]), 0);
}

#[test]
fn missing_texture_is_an_error() {
    let mut scene = Scene::new(fixtures::single_quad());
    scene.texture = [0; 4];
    assert!(matches!(
        scene.try_render(),
        Err(RenderError::MissingTexture(0))
    ));
}

#[test]
fn multiply_color_tints_texture() {
    let model = with_keyforms(fixtures::single_quad(), |k| {
        k.multiply_color = Rgb::new(1.0, 0.5, 0.0);
    });
    let image = Scene::new(model).render();
    image.assert_pixel(INSIDE, [1.0, 0.5, 0.0, 1.0]);
}

#[test]
fn screen_color_brightens_texture() {
    let model = with_keyforms(fixtures::single_quad(), |k| {
        k.screen_color = Rgb::new(1.0, 0.5, 0.0);
    });
    let mut scene = Scene::new(model);
    // 不透明な 50% グレー: rgb = 0.5 + S - 0.5 * S。
    scene.texture = [128, 128, 128, 255];
    let image = scene.render();
    let c = 128.0 / 255.0;
    let screen = |s: f32| c + s - c * s;
    image.assert_pixel(INSIDE, [screen(1.0), screen(0.5), screen(0.0), 1.0]);
}

#[test]
fn screen_color_respects_premultiplied_alpha() {
    let model = with_keyforms(fixtures::single_quad(), |k| {
        k.screen_color = Rgb::new(1.0, 0.0, 0.0);
    });
    let mut scene = Scene::new(model);
    // 50% 不透明の黒 (乗算済み)。スクリーン色は c.a で重み付けされる。
    scene.texture = [0, 0, 0, 128];
    let image = scene.render();
    let a = 128.0 / 255.0;
    image.assert_pixel(INSIDE, [a, 0.0, 0.0, a]);
}

#[test]
fn additive_blend_adds_color_and_keeps_dst_alpha() {
    let mut model = with_keyforms(fixtures::single_quad(), |k| {
        k.multiply_color = Rgb::new(0.5, 0.25, 0.0);
    });
    model.meshes[0].blend = BlendMode::Additive;
    let mut scene = Scene::new(model);
    scene.clear = clear_color([0.25, 0.25, 0.25, 0.5]);
    let image = scene.render();
    image.assert_pixel(INSIDE, [0.75, 0.5, 0.25, 0.5]);
    image.assert_pixel(OUTSIDE, [0.25, 0.25, 0.25, 0.5]);
}

#[test]
fn multiply_blend_multiplies_dst_and_keeps_dst_alpha() {
    let mut model = with_keyforms(fixtures::single_quad(), |k| {
        k.multiply_color = Rgb::new(0.5, 0.5, 1.0);
    });
    model.meshes[0].blend = BlendMode::Multiply;
    let mut scene = Scene::new(model);
    scene.clear = clear_color([0.5, 1.0, 0.25, 1.0]);
    let image = scene.render();
    // src.a = 1 なので dst * (1 - src.a) は 0。
    image.assert_pixel(INSIDE, [0.25, 0.5, 0.25, 1.0]);
    image.assert_pixel(OUTSIDE, [0.5, 1.0, 0.25, 1.0]);
}

#[test]
fn multiply_blend_with_translucent_source() {
    let mut model = with_keyforms(fixtures::single_quad(), |k| {
        k.multiply_color = Rgb::new(0.0, 0.0, 0.0);
        k.opacity = 0.5;
    });
    model.meshes[0].blend = BlendMode::Multiply;
    let mut scene = Scene::new(model);
    scene.clear = clear_color([1.0, 0.5, 0.25, 1.0]);
    let image = scene.render();
    // src = 0 (黒, a = 0.5): dst * (1 - 0.5)。
    image.assert_pixel(INSIDE, [0.5, 0.25, 0.125, 1.0]);
}

/// single_quad の三角形 (0,1,2)・(0,2,3) はキャンバス上で時計回り (= 裏)。
fn culled_quad(front_facing: bool) -> Model {
    let mut model = fixtures::single_quad();
    let mesh = &mut model.meshes[0];
    mesh.culling = true;
    if front_facing {
        mesh.indices = vec![0, 2, 1, 0, 3, 2];
    }
    model
}

#[test]
fn culling_drops_back_faces() {
    let image = Scene::new(culled_quad(false)).render();
    assert_eq!(image.covered_pixels([0; 4]), 0);

    let image = Scene::new(culled_quad(true)).render();
    assert_eq!(image.covered_pixels([0; 4]), 32 * 32);
}

#[test]
fn culling_without_flag_draws_both_faces() {
    let mut model = culled_quad(false);
    model.meshes[0].culling = false;
    let image = Scene::new(model).render();
    assert_eq!(image.covered_pixels([0; 4]), 32 * 32);
}

#[test]
fn culling_follows_view_flip() {
    // Y 軸を反転しないビューでは、画面上の向きが逆になる。表裏の判定はキャンバス座標で行うので結果は同じ。
    for front_facing in [false, true] {
        let mut scene = Scene::new(culled_quad(front_facing));
        scene.view = canvas_view(false);
        let image = scene.render();
        let expected = if front_facing { 32 * 32 } else { 0 };
        assert_eq!(
            image.covered_pixels([0; 4]),
            expected,
            "front_facing = {front_facing}"
        );
    }
}

#[test]
fn render_order_follows_draw_order() {
    let mut model = fixtures::single_quad();
    let mut top = model.meshes[0].clone();
    top.id = "Top".into();
    top.name = "Top".into();
    // 宣言は後だが描画順の値が小さい (400 < 500) ので奥に描かれる。
    for kf in &mut top.keyforms.keyforms {
        kf.draw_order = 400.0;
        kf.multiply_color = Rgb::new(0.0, 1.0, 0.0);
    }
    for kf in &mut model.meshes[0].keyforms.keyforms {
        kf.multiply_color = Rgb::new(1.0, 0.0, 0.0);
    }
    model.meshes.push(top);
    let image = Scene::new(model).render();
    // 描画順 500 の赤が最後に描かれる。
    image.assert_pixel(INSIDE, [1.0, 0.0, 0.0, 1.0]);
}

// ---- マスク ----
//
// キャンバス 256 → 画素 64 なので、画素座標はキャンバス座標の 1/4。

const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const NONE: [f32; 4] = [0.0; 4];

/// キャンバス上の矩形 `[x0, y0, x1, y1]` を覆う白い四角形 (single_quad のメッシュを複製)。
fn quad(id: &str, [x0, y0, x1, y1]: [f32; 4]) -> Mesh {
    let mut mesh = fixtures::single_quad().meshes.remove(0);
    mesh.id = id.into();
    mesh.name = id.into();
    for kf in &mut mesh.keyforms.keyforms {
        kf.positions = vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ];
    }
    mesh
}

/// 不透明な赤の四角形。`masks` のメッシュでクリッピングする。
fn red_quad(id: &str, rect: [f32; 4], masks: &[&str], invert: bool) -> Mesh {
    let mut mesh = quad(id, rect);
    mesh.masks = masks.iter().map(|&m| m.into()).collect();
    mesh.invert_mask = invert;
    for kf in &mut mesh.keyforms.keyforms {
        kf.multiply_color = Rgb::new(1.0, 0.0, 0.0);
    }
    mesh
}

/// マスク用メッシュ。白の乗算合成は描画先を変えないので、本描画の結果には現れない
/// (マスクパスは合成モードと色を使わない)。描画順は赤い要素より奥。
fn mask_quad(id: &str, rect: [f32; 4], opacity: f32) -> Mesh {
    let mut mesh = quad(id, rect);
    mesh.blend = BlendMode::Multiply;
    for kf in &mut mesh.keyforms.keyforms {
        kf.opacity = opacity;
        kf.draw_order = 400.0;
    }
    mesh
}

fn model_of(meshes: Vec<Mesh>) -> Model {
    let mut model = fixtures::single_quad();
    model.meshes = meshes;
    model
}

/// 赤い四角形 (画素 16..48) を、右側にずれたマスク (画素 x 32..56, y 8..56) で切り抜く。
fn clipped_scene(invert: bool, mask_opacity: f32) -> Scene {
    Scene::new(model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["Mask"], invert),
        mask_quad("Mask", [128.0, 32.0, 224.0, 224.0], mask_opacity),
    ]))
}

#[test]
fn mask_clips_drawable() {
    let image = clipped_scene(false, 1.0).render();
    image.assert_pixel((24, 32), NONE);
    image.assert_pixel((31, 32), NONE);
    image.assert_pixel((32, 32), RED);
    image.assert_pixel((40, 32), RED);
    image.assert_pixel((47, 47), RED);
    // マスクだけの範囲には何も描かれない。
    image.assert_pixel((52, 32), NONE);
    assert_eq!(image.covered_pixels([0; 4]), 16 * 32);
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 2,
            mask_sets: 1,
            mask_draws: 1,
            mask_atlases: 1,
        }
    );
}

#[test]
fn inverted_mask_keeps_outside() {
    let image = clipped_scene(true, 1.0).render();
    image.assert_pixel((16, 32), RED);
    image.assert_pixel((24, 32), RED);
    image.assert_pixel((31, 32), RED);
    image.assert_pixel((32, 32), NONE);
    image.assert_pixel((40, 32), NONE);
    image.assert_pixel((52, 32), NONE);
    assert_eq!(image.covered_pixels([0; 4]), 16 * 32);
}

#[test]
fn multiple_masks_are_united() {
    let image = Scene::new(model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["L", "R"], false),
        mask_quad("L", [64.0, 0.0, 96.0, 256.0], 1.0),
        mask_quad("R", [160.0, 0.0, 192.0, 256.0], 1.0),
    ]))
    .render();
    image.assert_pixel((20, 32), RED);
    image.assert_pixel((32, 32), NONE);
    image.assert_pixel((44, 32), RED);
    assert_eq!(image.covered_pixels([0; 4]), 2 * 8 * 32);
    assert_eq!(image.stats.mask_sets, 1);
    assert_eq!(image.stats.mask_draws, 2);
}

#[test]
fn overlapping_masks_saturate_at_full_coverage() {
    // テクスチャのアルファ 0.75 + 0.75 は 1 に飽和する (min(1, Σ))。
    let mut scene = Scene::new(model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["A", "B"], false),
        mask_quad("A", [0.0, 0.0, 256.0, 256.0], 1.0),
        mask_quad("B", [0.0, 0.0, 256.0, 256.0], 1.0),
    ]));
    let texture = scene.model.textures[0].clone();
    scene.model.textures.push(texture);
    for mesh in &mut scene.model.meshes[1..] {
        mesh.texture = 1;
    }
    scene.extra_textures = vec![[0, 0, 0, 191]];
    let image = scene.render();
    image.assert_pixel((32, 32), RED);
}

#[test]
fn mask_opacity_does_not_affect_coverage() {
    // カバレッジはテクスチャのアルファだけで決まる (docs/spec/rendering.md)。
    let reference = clipped_scene(false, 1.0).render();
    let inverted_reference = clipped_scene(true, 1.0).render();
    for opacity in [0.5, 0.0] {
        let image = clipped_scene(false, opacity).render();
        image.assert_pixel((40, 32), RED);
        image.assert_pixel((24, 32), NONE);
        assert!(image.pixels == reference.pixels, "opacity = {opacity}");

        let image = clipped_scene(true, opacity).render();
        image.assert_pixel((40, 32), NONE);
        image.assert_pixel((24, 32), RED);
        assert!(
            image.pixels == inverted_reference.pixels,
            "opacity = {opacity}"
        );
    }
}

#[test]
fn mask_texture_alpha_scales_coverage() {
    let mut scene = clipped_scene(false, 1.0);
    let texture = scene.model.textures[0].clone();
    scene.model.textures.push(texture);
    scene.model.meshes[1].texture = 1;
    // 50% 不透明の黒 (乗算済み)。マスクの色は使わず、アルファだけが効く。
    scene.extra_textures = vec![[0, 0, 0, 128]];
    let image = scene.render();
    let a = 128.0 / 255.0;
    image.assert_pixel((40, 32), [a, 0.0, 0.0, a]);
    image.assert_pixel((24, 32), NONE);
}

#[test]
fn invisible_mask_mesh_is_still_drawn_as_mask() {
    // 不透明度 0 のマスク用メッシュは要素としては描かれないが、マスクパスでは同じように描く。
    let image = clipped_scene(false, 0.0).render();
    image.assert_pixel((24, 32), NONE);
    image.assert_pixel((40, 32), RED);
    assert_eq!(image.covered_pixels([0; 4]), 16 * 32);
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 1,
            mask_sets: 1,
            mask_draws: 1,
            mask_atlases: 1,
        }
    );

    let image = clipped_scene(true, 0.0).render();
    image.assert_pixel((24, 32), RED);
    image.assert_pixel((40, 32), NONE);
    assert_eq!(image.covered_pixels([0; 4]), 16 * 32);
}

#[test]
fn missing_mask_texture_is_an_error() {
    let mut scene = clipped_scene(false, 1.0);
    let texture = scene.model.textures[0].clone();
    scene.model.textures.push(texture);
    scene.model.meshes[1].texture = 1;
    // マスク用メッシュが非表示でも、マスクとして描くのでテクスチャが要る。
    for kf in &mut scene.model.meshes[1].keyforms.keyforms {
        kf.opacity = 0.0;
    }
    assert!(matches!(
        scene.try_render(),
        Err(RenderError::MissingTexture(1))
    ));
}

#[test]
fn drawables_with_same_mask_set_share_mask() {
    // 横帯 4 本 (画素 y 16..24, 24..32, 32..40, 40..48)。
    // M1 は画素 x 16..32、M2 は画素 x 40..48。
    let band = |i: f32| [64.0, 64.0 + 32.0 * i, 192.0, 96.0 + 32.0 * i];
    let image = Scene::new(model_of(vec![
        red_quad("A", band(0.0), &["M1"], false),
        red_quad("B", band(1.0), &["M1"], true),
        red_quad("C", band(2.0), &["M1", "M2"], false),
        // 並びと重複は区別しないので C と同じ集合。
        red_quad("D", band(3.0), &["M2", "M1", "M2"], false),
        mask_quad("M1", [64.0, 0.0, 128.0, 256.0], 1.0),
        mask_quad("M2", [160.0, 0.0, 192.0, 256.0], 1.0),
    ]))
    .render();
    let rows = [
        (20, [RED, NONE, NONE]),
        (28, [NONE, RED, RED]),
        (36, [RED, NONE, RED]),
        (44, [RED, NONE, RED]),
    ];
    for (y, expected) in rows {
        for (x, e) in [20, 36, 44].into_iter().zip(expected) {
            image.assert_pixel((x, y), e);
        }
    }
    // 集合は {M1} と {M1, M2} の 2 つ。マスク用メッシュの描画は 1 + 2 回。
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 6,
            mask_sets: 2,
            mask_draws: 3,
            mask_atlases: 1,
        }
    );
}

/// 縦長の赤い四角形 5 本 (画素 x 4+12k..12+12k, y 16..48) を、それぞれ別のマスクで上半分だけ残す。
fn five_mask_sets() -> Scene {
    let mut meshes = Vec::new();
    for k in 0..5 {
        let x = 16.0 + 48.0 * k as f32;
        let mask = format!("M{k}");
        meshes.push(red_quad(
            &format!("C{k}"),
            [x, 64.0, x + 32.0, 192.0],
            &[&mask],
            false,
        ));
        meshes.push(mask_quad(&mask, [x, 64.0, x + 32.0, 128.0], 1.0));
    }
    Scene::new(model_of(meshes))
}

/// `exact_edges` が偽なら、マスクの縁の画素は検査しない
/// (小さいアトラスでは縁がバイリニア補間でぼける)。
fn assert_five_mask_sets(image: &Image, exact_edges: bool) {
    for k in 0..5 {
        let x = 8 + 12 * k;
        image.assert_pixel((x, 20), RED);
        image.assert_pixel((x, 28), RED);
        image.assert_pixel((x, 36), NONE);
        image.assert_pixel((x, 44), NONE);
        if exact_edges {
            image.assert_pixel((x, 31), RED);
            image.assert_pixel((x, 32), NONE);
        }
    }
    if exact_edges {
        assert_eq!(image.covered_pixels([0; 4]), 5 * 8 * 16);
    }
}

#[test]
fn mask_sets_overflow_into_more_atlases() {
    let mut scene = five_mask_sets();
    // 1 枚に 4 チャンネル × 1 セルしか入らないので、5 つ目の集合は 2 枚目に回る。
    scene.options.mask_atlas = MaskAtlasOptions {
        size: 32,
        min_cell_size: 32,
    };
    let image = scene.render();
    assert_five_mask_sets(&image, false);
    assert_eq!(image.stats.mask_sets, 5);
    assert_eq!(image.stats.mask_draws, 5);
    assert_eq!(image.stats.mask_atlases, 2);

    // 既定の設定なら 1 枚に収まる。
    let image = five_mask_sets().render();
    assert_five_mask_sets(&image, true);
    assert_eq!(image.stats.mask_atlases, 1);
}

#[test]
fn unmasked_frame_uses_no_atlas() {
    let image = Scene::new(fixtures::single_quad()).render();
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 1,
            mask_sets: 0,
            mask_draws: 0,
            mask_atlases: 0,
        }
    );
}
