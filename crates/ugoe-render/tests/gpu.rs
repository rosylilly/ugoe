//! オフスクリーンに描画して画素を読み戻す描画テスト。`just test-gpu` で実行する。
//!
//! GPU (またはソフトウェアラスタライザの lavapipe など) が見つからない場合は、
//! 黙って飛ばさずに失敗する (feature を明示的に有効にしたときだけ実行されるため)。
#![cfg(feature = "gpu-tests")]

use std::sync::OnceLock;

use ugoe_core::Rgb;
use ugoe_model::{BlendMode, Model, fixtures};
use ugoe_render::{RenderError, Renderer, View};
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
    texture: [u8; 4],
    clear: wgpu::Color,
    view: View,
}

impl Scene {
    fn new(model: Model) -> Self {
        Self {
            model,
            texture: [255; 4],
            clear: wgpu::Color::TRANSPARENT,
            view: normal_view(),
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
        let mut renderer = Renderer::new(device, FORMAT);
        if self.texture != [0; 4] {
            renderer.set_texture(device, queue, 0, 1, 1, &self.texture)?;
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
        Ok(Image { pixels })
    }
}

struct Image {
    pixels: Vec<u8>,
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
