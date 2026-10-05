//! オフスクリーンに描画して画素を読み戻す描画テスト。`just test-gpu` で実行する。
//!
//! GPU (またはソフトウェアラスタライザの lavapipe など) が見つからない場合は、
//! 黙って飛ばさずに失敗する (feature を明示的に有効にしたときだけ実行されるため)。
#![cfg(feature = "gpu-tests")]

use std::sync::OnceLock;

use ugoe_core::{Rgb, Vec2};
use ugoe_model::mesh::Mesh;
use ugoe_model::{BlendMode, MaskRef, Model, fixtures};
use ugoe_render::{
    FrameStats, Light, Lighting, MAX_TEXTURES, MaskAtlasOptions, RenderError, Renderer,
    RendererOptions, View,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

/// 描画先の既定の一辺 (画素)。
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
    /// 描画先の一辺 (画素、64 の倍数)。ビューはキャンバス全体をこれに写す。
    size: u32,
    /// 描画前に設定するパラメータ。
    params: Vec<(&'static str, f32)>,
    /// 光源。`None` なら設定しない (既定の光源)。
    lighting: Option<Lighting>,
    /// 法線マップ `(テクスチャの番号, 一辺, RGBA8)`。
    normal_maps: Vec<(u32, u32, Vec<u8>)>,
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
            size: SIZE,
            params: Vec::new(),
            lighting: None,
            normal_maps: Vec::new(),
        }
    }

    fn render(self) -> Image {
        self.try_render().expect("render")
    }

    fn try_render(self) -> Result<Image, RenderError> {
        let (mut renderer, instance) = self.prepare()?;
        render_frame(&mut renderer, &instance, &self.view, self.size, self.clear)
    }

    /// レンダラ (テクスチャ登録済み) とインスタンス (パラメータ設定・更新済み) を作る。
    fn prepare(&self) -> Result<(Renderer, ModelInstance), RenderError> {
        let Gpu { device, queue, .. } = gpu();
        let compiled = CompiledModel::compile(self.model.clone()).expect("compile");
        let mut instance = ModelInstance::new(compiled);
        for &(id, value) in &self.params {
            let index = instance.parameters().index_of(id).expect("parameter");
            instance.parameters_mut().set(index, value);
        }
        instance.update();
        let mut renderer = Renderer::with_options(device, FORMAT, self.options);
        if self.texture != [0; 4] {
            renderer.set_texture(device, queue, 0, 1, 1, &self.texture)?;
        }
        for (i, texture) in self.extra_textures.iter().enumerate() {
            renderer.set_texture(device, queue, i as u32 + 1, 1, 1, texture)?;
        }
        for (index, size, rgba) in &self.normal_maps {
            renderer.set_normal_map(device, queue, *index, *size, *size, rgba)?;
        }
        if let Some(lighting) = &self.lighting {
            renderer.set_lighting(lighting);
        }
        Ok((renderer, instance))
    }
}

/// 一辺 `size` の描画先を `clear` でクリアし、`renderer` で 1 フレーム描いて読み戻す。
fn render_frame(
    renderer: &mut Renderer,
    instance: &ModelInstance,
    view: &View,
    size: u32,
    clear: wgpu::Color,
) -> Result<Image, RenderError> {
    let Gpu { device, queue, .. } = gpu();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
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

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    // クリアはホストの責務。
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    renderer.encode(device, queue, &mut encoder, &target_view, view, instance)?;

    let bytes_per_row = size * 4; // 256 (COPY_BYTES_PER_ROW_ALIGNMENT) の倍数
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * size),
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
                rows_per_image: Some(size),
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
    dump_image(&pixels, size);
    Ok(Image {
        pixels,
        size,
        stats: renderer.last_frame_stats(),
    })
}

/// 環境変数 `UGOE_GPU_TEST_DUMP` にディレクトリが指定されていれば、描いた画像を
/// `<テスト名>-<番号>.png` として書き出す (目で確かめる・実装の前後で比べるため)。
fn dump_image(pixels: &[u8], size: u32) {
    thread_local! {
        // テストは 1 つずつ別のスレッドで走るので、番号はテストの中での順番になる。
        static COUNTER: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    let Some(dir) = std::env::var_os("UGOE_GPU_TEST_DUMP") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("create dump dir");
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("unknown").replace("::", "-");
    let n = COUNTER.replace(COUNTER.get() + 1);
    let path = dir.join(format!("{test}-{n:04}.png"));
    let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("create dump file"));
    let mut encoder = png::Encoder::new(file, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(pixels).expect("png data");
}

struct Image {
    pixels: Vec<u8>,
    size: u32,
    stats: FrameStats,
}

impl Image {
    fn at(&self, (x, y): (u32, u32)) -> [u8; 4] {
        let i = ((y * self.size + x) * 4) as usize;
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

    fn count(&self, color: [u8; 4]) -> usize {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|&&p| p == color)
            .count()
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

/// 一辺 `size` のマスクアトラス 1 枚のバイト数 (RGBA8)。
fn atlas_bytes(size: u64) -> u64 {
    size * size * 4
}

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

/// 不透明な赤の四角形。`masks` のメッシュでクリッピングする (`invert` なら全参照を反転マスクにする)。
fn red_quad(id: &str, rect: [f32; 4], masks: &[&str], invert: bool) -> Mesh {
    let mut mesh = quad(id, rect);
    mesh.masks = masks
        .iter()
        .map(|&m| MaskRef {
            mesh: m.into(),
            invert,
        })
        .collect();
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
            mask_atlas_bytes: atlas_bytes(256),
            ..image.stats
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
fn overlapping_masks_combine_as_union() {
    // 和集合は 1 − Π(1 − c): テクスチャのアルファ 0.75 と 0.75 で 1 − 0.25² = 0.9375
    // (T0202 の飽和加算 min(1, Σ) では 1 だった)。
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
    let c = 1.0 - (1.0 - 191.0 / 255.0f32).powi(2);
    image.assert_pixel((32, 32), [c, 0.0, 0.0, c]);
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
            mask_atlas_bytes: atlas_bytes(256),
            ..image.stats
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
            mask_atlas_bytes: atlas_bytes(256),
            ..image.stats
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
    // 一辺 16 のアトラスには 1 チャンネルに 1 セル (10x16 に縮小) しか入らないので、
    // 3 チャンネル分を超えた 4 つ目以降の集合は 2 枚目に回る。
    scene.options.mask_atlas = MaskAtlasOptions {
        max_size: 16,
        ..MaskAtlasOptions::default()
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
            mask_atlas_bytes: 0,
            ..image.stats
        }
    );
}

// ---- マスクの意味論 (ADR 0009、T0205) ----

const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const WHITE: [f32; 4] = [1.0; 4];

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model.meshes.iter_mut().find(|m| m.id == id).expect("mesh")
}

/// メッシュ `id` のすべてのキーフォームを書き換える。
fn edit_keyforms(model: &mut Model, id: &str, f: impl Fn(&mut ugoe_model::mesh::MeshKeyform)) {
    for kf in &mut mesh_mut(model, id).keyforms.keyforms {
        f(kf);
    }
}

fn tint(model: &mut Model, id: &str, [r, g, b]: [f32; 3]) {
    edit_keyforms(model, id, |k| k.multiply_color = Rgb::new(r, g, b));
}

/// 1x1 のテクスチャ `[0, 0, 0, alpha]` を足し、メッシュ `ids` に割り当てる。
fn use_alpha_texture(scene: &mut Scene, ids: &[&str], alpha: u8) {
    let texture = scene.model.textures[0].clone();
    scene.model.textures.push(texture);
    let index = scene.model.textures.len() as u32 - 1;
    for id in ids {
        mesh_mut(&mut scene.model, id).texture = index;
    }
    scene.extra_textures.push([0, 0, 0, alpha]);
}

/// `c` 倍した赤 (乗算済み)。
fn red(c: f32) -> [f32; 4] {
    [c, 0.0, 0.0, c]
}

#[test]
fn union_of_translucent_masks_is_one_minus_product() {
    // A: 画素 x 16..32 (α = a)、B: 画素 x 24..48 (α = b)。赤い四角形は画素 16..48。
    let mut scene = Scene::new(model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["A", "B"], false),
        mask_quad("A", [64.0, 0.0, 128.0, 256.0], 1.0),
        mask_quad("B", [96.0, 0.0, 192.0, 256.0], 1.0),
    ]));
    use_alpha_texture(&mut scene, &["A"], 128);
    use_alpha_texture(&mut scene, &["B"], 64);
    let image = scene.render();
    let (a, b) = (128.0 / 255.0, 64.0 / 255.0);
    image.assert_pixel((20, 32), red(a));
    image.assert_pixel((28, 32), red(1.0 - (1.0 - a) * (1.0 - b)));
    image.assert_pixel((40, 32), red(b));
    image.assert_pixel((52, 32), NONE);
}

#[test]
fn intersection_keeps_common_area() {
    let mut model = fixtures::intersection_masks();
    tint(&mut model, "Face", [0.0, 0.0, 1.0]);
    tint(&mut model, "Tongue", [1.0, 0.0, 0.0]);
    let image = Scene::new(model).render();
    // Tongue の見える範囲は (96,160)〜(192,192) = 画素 (24,40)〜(48,48)。
    image.assert_pixel((24, 40), RED);
    image.assert_pixel((47, 47), RED);
    // 口の外 (顔の内側) と、顔の外 (口の範囲の外でもある) には出ない。
    image.assert_pixel((40, 50), BLUE);
    image.assert_pixel((52, 44), BLUE);
    image.assert_pixel((58, 58), NONE);
    assert_eq!(image.count([255, 0, 0, 255]), 24 * 8);
    assert_eq!(image.stats.mask_sets, 1);
    assert_eq!(image.stats.mask_draws, 2);
}

#[test]
fn intersection_multiplies_translucent_coverage() {
    // 共通部分は Π c。
    let mut scene = Scene::new(model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["A", "B"], false),
        mask_quad("A", [64.0, 0.0, 128.0, 256.0], 1.0),
        mask_quad("B", [96.0, 0.0, 192.0, 256.0], 1.0),
    ]));
    mesh_mut(&mut scene.model, "Content").mask_mode = ugoe_model::MaskMode::Intersection;
    use_alpha_texture(&mut scene, &["A"], 128);
    use_alpha_texture(&mut scene, &["B"], 64);
    let image = scene.render();
    let (a, b) = (128.0 / 255.0, 64.0 / 255.0);
    image.assert_pixel((20, 32), NONE);
    image.assert_pixel((28, 32), red(a * b));
    image.assert_pixel((40, 32), NONE);
}

/// mixed_masks の Face を青、Brow を赤にし、Hair は色としては描かない (不透明度 0)。
fn mixed_scene(strength: f32) -> Scene {
    let mut model = fixtures::mixed_masks();
    tint(&mut model, "Face", [0.0, 0.0, 1.0]);
    tint(&mut model, "Brow", [1.0, 0.0, 0.0]);
    edit_keyforms(&mut model, "Hair", |k| k.opacity = 0.0);
    let mut scene = Scene::new(model);
    scene.params = vec![("Strength", strength)];
    scene
}

#[test]
fn normal_and_inverted_masks_combine_on_one_mesh() {
    let image = mixed_scene(1.0).render();
    // Brow (画素 16..48 × 16..32) は顔の内側かつ髪 (画素 y 8..24) の外側 = y 24..32 だけ見える。
    image.assert_pixel((20, 20), BLUE);
    image.assert_pixel((20, 23), BLUE);
    image.assert_pixel((20, 24), RED);
    image.assert_pixel((47, 31), RED);
    image.assert_pixel((32, 36), BLUE);
    image.assert_pixel((4, 4), NONE);
    assert_eq!(image.stats.mask_sets, 1);
    assert_eq!(image.stats.mask_draws, 2);
}

#[test]
fn mask_strength_lets_masked_area_show_through() {
    // Strength 0.5 → mask_strength = 0.5 (fixtures::mixed_masks)。髪の下 (反転マスクの内側) も
    // 1 − s = 0.5 だけ見える。k = 1 − s × (1 − m)。
    let image = mixed_scene(0.5).render();
    let over_blue = |k: f32| [k, 0.0, 1.0 - k, 1.0];
    image.assert_pixel((20, 20), over_blue(0.5));
    image.assert_pixel((20, 28), RED);
    // 0 なら マスクは効かない (全体が見える)。
    let image = mixed_scene(0.0).render();
    image.assert_pixel((20, 20), RED);
    image.assert_pixel((20, 28), RED);

    // 通常マスクの外側も 1 − s だけ見える。
    let mut scene = clipped_scene(false, 1.0);
    edit_keyforms(&mut scene.model, "Content", |k| k.mask_strength = 0.25);
    let image = scene.render();
    image.assert_pixel((24, 32), red(0.75));
    image.assert_pixel((40, 32), RED);
}

#[test]
fn nested_masks_inherit_clipping() {
    let mut model = fixtures::nested_masks();
    tint(&mut model, "Iris", [0.0, 0.0, 1.0]);
    tint(&mut model, "Pupil", [1.0, 0.0, 0.0]);
    let image = Scene::new(model).render();
    // Pupil (画素 28..36 × 10..30) は Iris で切り抜かれ、Iris は Sclera (画素 y 24..40) で
    // 切り抜かれるので、Pupil は白目の外 (y < 24) に出ない。
    image.assert_pixel((32, 12), NONE);
    image.assert_pixel((32, 23), NONE);
    image.assert_pixel((32, 24), RED);
    image.assert_pixel((32, 29), RED);
    image.assert_pixel((32, 30), BLUE);
    image.assert_pixel((26, 26), BLUE);
    image.assert_pixel((20, 32), WHITE);
    assert_eq!(image.covered_pixels([0; 4]), 32 * 16);
    // Iris と Pupil の 2 つの内容を、レベルごとに別のアトラスに描く。
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 3,
            mask_sets: 2,
            mask_draws: 2,
            mask_atlases: 2,
            mask_atlas_bytes: 2 * atlas_bytes(256),
            ..image.stats
        }
    );
}

#[test]
fn nested_mask_strength_is_inherited() {
    // Iris の mask_strength を 0.5 にすると、白目の外 (画素 y 16..24) の Iris は 0.5 だけ見え、
    // Iris をマスクにする Pupil のカバレッジも 0.5 になる (入れ子の係数は mask_strength を含む)。
    let mut model = fixtures::nested_masks();
    tint(&mut model, "Iris", [0.0, 0.0, 1.0]);
    tint(&mut model, "Pupil", [1.0, 0.0, 0.0]);
    edit_keyforms(&mut model, "Iris", |k| k.mask_strength = 0.5);
    let image = Scene::new(model).render();
    image.assert_pixel((26, 20), [0.0, 0.0, 0.5, 0.5]);
    // 0.5 の赤を 0.5 の青の上に normal 合成。
    image.assert_pixel((32, 20), [0.5, 0.0, 0.25, 0.75]);
    image.assert_pixel((32, 12), NONE);
    // 白目の内側は変わらない。
    image.assert_pixel((32, 26), RED);
}

#[test]
fn mask_only_mesh_is_not_drawn_but_masks() {
    let mut model = fixtures::intersection_masks();
    tint(&mut model, "Face", [0.0, 0.0, 1.0]);
    let image = Scene::new(model.clone()).render();
    // MouthMask (画素 16..48 × 32..48、白) は描かれない。
    image.assert_pixel((18, 34), BLUE);
    assert_eq!(image.stats.drawables, 2);

    // 普通のマスク用メッシュでも、mask_only にすれば描かれずにマスクとして働く。
    let mut scene = clipped_scene(false, 1.0);
    let mask = mesh_mut(&mut scene.model, "Mask");
    mask.mask_only = true;
    mask.blend = BlendMode::Normal;
    let image = scene.render();
    image.assert_pixel((40, 32), RED);
    image.assert_pixel((24, 32), NONE);
    image.assert_pixel((52, 32), NONE);
    assert_eq!(image.covered_pixels([0; 4]), 16 * 32);
    assert_eq!(image.stats.drawables, 1);
    assert_eq!(image.stats.mask_draws, 1);

    // 反転マスクとしても働く。
    let mut scene = clipped_scene(true, 1.0);
    mesh_mut(&mut scene.model, "Mask").mask_only = true;
    let image = scene.render();
    image.assert_pixel((24, 32), RED);
    image.assert_pixel((40, 32), NONE);
}

/// 描画先 512 画素 (キャンバスの 2 倍) に、赤い四角形 20 個 (各 80x80 画素) を並べ、
/// それぞれ別のマスクで左側 (縁は画素の途中 x0 + 26.6) だけ残す。
fn many_sharp_masks() -> Scene {
    let mut meshes = Vec::new();
    for k in 0..20 {
        let (x, y) = (8.0 + 48.0 * (k % 5) as f32, 8.0 + 48.0 * (k / 5) as f32);
        let mask = format!("M{k}");
        meshes.push(red_quad(
            &format!("C{k}"),
            [x, y, x + 40.0, y + 40.0],
            &[&mask],
            false,
        ));
        meshes.push(mask_quad(
            &mask,
            [x - 4.0, y - 4.0, x + 13.3, y + 44.0],
            1.0,
        ));
    }
    let mut scene = Scene::new(model_of(meshes));
    scene.size = 512;
    scene
}

/// 赤の画素ごとの値 (0..255)。
fn red_values(image: &Image) -> Vec<u8> {
    image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[0])
        .collect()
}

#[test]
fn masks_are_as_sharp_as_the_screen_by_default() {
    let image = many_sharp_masks().render();
    // 既定の精度では、マスクは画面の画素と同じ解像度で描かれる。縁の画素も 0 か 1 になり、
    // 残るのは中心が縁より左の画素 (各四角形で 27 列 × 80 行)。
    let values = red_values(&image);
    let partial = values.iter().filter(|&&v| v > 2 && v < 253).count();
    assert_eq!(partial, 0);
    assert_eq!(image.covered_pixels([0; 4]), 20 * 27 * 80);
    assert_eq!(image.stats.mask_sets, 20);
    assert_eq!(image.stats.mask_atlases, 1);
}

#[test]
fn downscale_option_lowers_mask_resolution() {
    let mut scene = many_sharp_masks();
    scene.options.mask_atlas = MaskAtlasOptions {
        downscale: 4,
        ..MaskAtlasOptions::default()
    };
    let image = scene.render();
    // 縁はぼやける (中間の値の画素がある) が、縁から離れた所は同じ。
    let values = red_values(&image);
    assert!(values.iter().any(|&v| v > 2 && v < 253));
    let (x0, y0) = (16, 16);
    image.assert_pixel((x0 + 4, y0 + 40), RED);
    image.assert_pixel((x0 + 20, y0 + 40), RED);
    image.assert_pixel((x0 + 34, y0 + 40), NONE);
    image.assert_pixel((x0 + 70, y0 + 40), NONE);
}

// ---- マスクアトラスの縮小 (T0206) ----

/// 縮小を待つフレーム数 (テストを短くするため既定の 120 より小さくする)。
const SHRINK_AFTER: u32 = 4;

fn shrinking_options(max_size: u32) -> RendererOptions {
    RendererOptions {
        mask_atlas: MaskAtlasOptions {
            max_size,
            shrink_after_frames: SHRINK_AFTER,
            ..MaskAtlasOptions::default()
        },
        ..RendererOptions::default()
    }
}

/// `clipped_scene` を一辺 `size` の描画先に描いた結果が、64 画素のときと同じ形か確かめる。
#[track_caller]
fn assert_clipped(image: &Image) {
    let s = image.size / 64;
    image.assert_pixel((24 * s, 32 * s), NONE);
    image.assert_pixel((31 * s, 32 * s), NONE);
    image.assert_pixel((32 * s, 32 * s), RED);
    image.assert_pixel((47 * s, 47 * s), RED);
    image.assert_pixel((52 * s, 32 * s), NONE);
    assert_eq!(image.covered_pixels([0; 4]) as u32, 16 * 32 * s * s);
}

#[test]
fn mask_atlas_shrinks_after_zooming_out() {
    let mut scene = clipped_scene(false, 1.0);
    scene.options = shrinking_options(4096);
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    let view = normal_view();
    let clear = wgpu::Color::TRANSPARENT;
    // 拡大: マスクの外接矩形は 256x256 画素なので、セル (縁込み 258) が入る 512 のアトラス。
    let image = render_frame(&mut renderer, &instance, &view, 512, clear).expect("render");
    assert_clipped(&image);
    assert_eq!(image.stats.mask_atlas_bytes, atlas_bytes(512));
    // 縮小: 256 で足りるが、SHRINK_AFTER フレーム続くまでは作り直さない。
    for frame in 1..=SHRINK_AFTER {
        let image = render_frame(&mut renderer, &instance, &view, SIZE, clear).expect("render");
        // 大きいアトラスのままでも、縮めた後でも、マスクは正しく効く。
        assert_clipped(&image);
        let expected = if frame < SHRINK_AFTER { 512 } else { 256 };
        assert_eq!(
            image.stats.mask_atlas_bytes,
            atlas_bytes(expected),
            "frame {frame}"
        );
    }
    // 縮めた後に拡大し直しても正しく描ける。
    let image = render_frame(&mut renderer, &instance, &view, 512, clear).expect("render");
    assert_clipped(&image);
    assert_eq!(image.stats.mask_atlas_bytes, atlas_bytes(512));
}

#[test]
fn repeated_zoom_cycles_do_not_grow_memory() {
    let mut scene = clipped_scene(false, 1.0);
    scene.options = shrinking_options(4096);
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    let view = normal_view();
    let clear = wgpu::Color::TRANSPARENT;
    let mut peak = 0;
    for cycle in 0..5 {
        let image = render_frame(&mut renderer, &instance, &view, 1024, clear).expect("render");
        assert_clipped(&image);
        peak = peak.max(image.stats.mask_atlas_bytes);
        let mut last = image;
        for _ in 0..SHRINK_AFTER {
            last = render_frame(&mut renderer, &instance, &view, SIZE, clear).expect("render");
            assert_clipped(&last);
            peak = peak.max(last.stats.mask_atlas_bytes);
        }
        assert_eq!(
            last.stats.mask_atlas_bytes,
            atlas_bytes(256),
            "cycle {cycle}"
        );
    }
    assert_eq!(peak, atlas_bytes(1024));
}

/// `canvas_view` を右へ `shift` (クリップ座標) だけずらしたビュー。
fn shifted_view(shift: f32) -> View {
    let mut view = normal_view();
    view.view_proj[2][0] += shift;
    view
}

#[test]
fn unused_overflow_atlases_are_released() {
    let mut scene = five_mask_sets();
    // 一辺 16 では 1 枚に 3 つの内容しか入らない (`mask_sets_overflow_into_more_atlases`)。
    scene.options = shrinking_options(16);
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    let clear = wgpu::Color::TRANSPARENT;
    let image =
        render_frame(&mut renderer, &instance, &normal_view(), SIZE, clear).expect("render");
    assert_five_mask_sets(&image, false);
    assert_eq!(image.stats.mask_atlases, 2);
    assert_eq!(image.stats.mask_atlas_bytes, 2 * atlas_bytes(16));

    // 右へ半分ずらすと、画面に入る内容は左の 3 つ (キャンバス x < 128) だけになる。
    let view = shifted_view(1.0);
    for frame in 1..=SHRINK_AFTER {
        let image = render_frame(&mut renderer, &instance, &view, SIZE, clear).expect("render");
        assert_eq!(image.stats.mask_sets, 3);
        assert_eq!(image.stats.mask_atlases, 1);
        // 左端の 2 本は、キャンバス x 16..48 → 画素 36..44 と 64..96 → 画素 48..56。
        image.assert_pixel((40, 20), RED);
        image.assert_pixel((40, 44), NONE);
        image.assert_pixel((52, 20), RED);
        image.assert_pixel((52, 44), NONE);
        let expected = if frame < SHRINK_AFTER { 2 } else { 1 };
        assert_eq!(
            image.stats.mask_atlas_bytes,
            expected * atlas_bytes(16),
            "frame {frame}"
        );
    }

    // 画面外に出してマスクを使わなくなったら、すべて解放する。
    let view = shifted_view(4.0);
    for frame in 1..=SHRINK_AFTER {
        let image = render_frame(&mut renderer, &instance, &view, SIZE, clear).expect("render");
        assert_eq!(image.stats.mask_atlases, 0);
        let expected = if frame < SHRINK_AFTER {
            atlas_bytes(16)
        } else {
            0
        };
        assert_eq!(image.stats.mask_atlas_bytes, expected, "frame {frame}");
    }

    // 戻せば作り直して正しく描く。
    let image =
        render_frame(&mut renderer, &instance, &normal_view(), SIZE, clear).expect("render");
    assert_five_mask_sets(&image, false);
    assert_eq!(image.stats.mask_atlas_bytes, 2 * atlas_bytes(16));
}

// ---- 転送の省略 (T0207) ----

/// 頂点 4 つのメッシュ 1 つ分の頂点位置のバイト数。
const QUAD_BYTES: u64 = 4 * 8;

fn set_param(instance: &mut ModelInstance, id: &str, value: f32) {
    let index = instance.parameters().index_of(id).expect("parameter");
    instance.parameters_mut().set(index, value);
}

fn frame(renderer: &mut Renderer, instance: &ModelInstance) -> Image {
    render_frame(
        renderer,
        instance,
        &normal_view(),
        SIZE,
        wgpu::Color::TRANSPARENT,
    )
    .expect("render")
}

/// 赤い `Content` (画素 16..48) を、パラメータ X で動く `Mask` で切り抜く。
/// `Mask` は single_quad の四角形で、X = 0 なら画素 16..48、X = 1 なら画素 32..64 を覆う。
fn moving_mask_model() -> Model {
    let mut mask = fixtures::single_quad().meshes.remove(0);
    mask.id = "Mask".into();
    mask.name = "Mask".into();
    mask.blend = BlendMode::Multiply;
    for kf in &mut mask.keyforms.keyforms {
        kf.draw_order = 400.0;
    }
    model_of(vec![
        red_quad("Content", [64.0, 64.0, 192.0, 192.0], &["Mask"], false),
        mask,
    ])
}

/// `moving_mask_model` を X = `x` (0 か 1) で描いた結果か確かめる。
#[track_caller]
fn assert_mask_at(image: &Image, x: f32) {
    image.assert_pixel((24, 32), if x == 0.0 { RED } else { NONE });
    image.assert_pixel((40, 32), RED);
    image.assert_pixel((52, 32), NONE);
}

/// single_quad (白) を X = `x` (0 か 1) で描いた結果か確かめる。
#[track_caller]
fn assert_quad_at(image: &Image, x: f32) {
    image.assert_pixel((20, 32), if x == 0.0 { WHITE } else { NONE });
    image.assert_pixel((40, 32), WHITE);
    image.assert_pixel((52, 32), if x == 0.0 { NONE } else { WHITE });
}

#[test]
fn static_frames_upload_nothing_after_the_first() {
    let (mut renderer, mut instance) = clipped_scene(false, 1.0).prepare().expect("prepare");
    let first = frame(&mut renderer, &instance);
    assert_clipped(&first);
    assert_eq!(first.stats.position_bytes, 2 * QUAD_BYTES);
    assert!(first.stats.uniform_bytes > 0);
    assert!(!first.stats.masks_reused);
    assert_eq!(first.stats.mask_draws, 1);

    // update を呼ばない (出力が同じ) フレームと、呼んでも何も変わらないフレーム。
    for update in [false, true, true] {
        if update {
            instance.update();
        }
        let image = frame(&mut renderer, &instance);
        assert_clipped(&image);
        assert_eq!(
            image.stats,
            FrameStats {
                masks_reused: true,
                mask_draws: 0,
                position_bytes: 0,
                uniform_bytes: 0,
                index_bytes: 0,
                uploaded_bytes: 0,
                ..first.stats
            },
            "update: {update}"
        );
    }

    // ビューだけが変わればビューのユニフォームとマスクを描き直す (頂点位置は転送しない)。
    let image = render_frame(
        &mut renderer,
        &instance,
        &shifted_view(0.0625),
        SIZE,
        wgpu::Color::TRANSPARENT,
    )
    .expect("render");
    assert_eq!(image.stats.position_bytes, 0);
    assert!(image.stats.uniform_bytes > 0);
    assert!(!image.stats.masks_reused);
}

#[test]
fn moving_one_mesh_uploads_only_that_mesh() {
    let mut model = fixtures::single_quad();
    model
        .meshes
        .push(red_quad("Still", [0.0, 0.0, 32.0, 32.0], &[], false));
    let (mut renderer, mut instance) = Scene::new(model).prepare().expect("prepare");
    let image = frame(&mut renderer, &instance);
    assert_quad_at(&image, 0.0);
    assert_eq!(image.stats.position_bytes, 2 * QUAD_BYTES);

    set_param(&mut instance, "X", 1.0);
    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_quad_at(&image, 1.0);
    image.assert_pixel((4, 4), RED);
    assert_eq!(image.stats.position_bytes, QUAD_BYTES);
    // 色や不透明度は変わらず、マスクもないのでユニフォームは転送しない。
    assert_eq!(image.stats.uniform_bytes, 0);
}

#[test]
fn moving_mask_redraws_the_atlas() {
    let (mut renderer, mut instance) = Scene::new(moving_mask_model()).prepare().expect("prepare");
    assert_mask_at(&frame(&mut renderer, &instance), 0.0);

    set_param(&mut instance, "X", 1.0);
    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_mask_at(&image, 1.0);
    assert!(!image.stats.masks_reused);
    assert_eq!(image.stats.mask_draws, 1);

    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_mask_at(&image, 1.0);
    assert!(image.stats.masks_reused);
}

#[test]
fn two_instances_alternate_on_one_renderer() {
    let scene = Scene::new(moving_mask_model());
    let (mut renderer, mut a) = scene.prepare().expect("prepare");
    // 同じモデル (同じ CompiledModel) の 2 つ目のインスタンス。
    let mut b = ModelInstance::new(a.compiled().clone());
    set_param(&mut b, "X", 1.0);
    b.update();
    for round in 0..3 {
        // どちらも毎回 update する (変更フラグは「変化なし」だが、バッファは相手の内容)。
        a.update();
        b.update();
        let image = frame(&mut renderer, &a);
        assert_mask_at(&image, 0.0);
        assert!(!image.stats.masks_reused, "round {round}");
        let image = frame(&mut renderer, &b);
        assert_mask_at(&image, 1.0);
        // 動くのはマスク用メッシュだけなので、比べて違う要素だけを転送する。
        if round > 0 {
            assert_eq!(image.stats.position_bytes, QUAD_BYTES, "round {round}");
        }
    }
}

#[test]
fn several_updates_between_encodes_are_not_lost() {
    let (mut renderer, mut instance) = Scene::new(fixtures::single_quad())
        .prepare()
        .expect("prepare");
    assert_quad_at(&frame(&mut renderer, &instance), 0.0);
    // 2 回目の update の変更フラグは「変化なし」だが、描いたときからは動いている。
    set_param(&mut instance, "X", 1.0);
    instance.update();
    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_quad_at(&image, 1.0);
    assert_eq!(image.stats.position_bytes, QUAD_BYTES);
}

#[test]
fn switching_models_reuploads_everything() {
    let (mut renderer, quad) = Scene::new(fixtures::single_quad())
        .prepare()
        .expect("prepare");
    let compiled = CompiledModel::compile(moving_mask_model()).expect("compile");
    let mut masked = ModelInstance::new(compiled);
    masked.update();
    for _ in 0..2 {
        let image = frame(&mut renderer, &quad);
        assert_quad_at(&image, 0.0);
        assert_eq!(image.stats.position_bytes, QUAD_BYTES);
        let image = frame(&mut renderer, &masked);
        assert_mask_at(&image, 0.0);
        assert_eq!(image.stats.position_bytes, 2 * QUAD_BYTES);
        assert!(!image.stats.masks_reused);
    }
}

#[test]
fn mark_all_changed_reuploads_positions() {
    let (mut renderer, mut instance) = clipped_scene(false, 1.0).prepare().expect("prepare");
    frame(&mut renderer, &instance);
    instance.mark_all_changed();
    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_clipped(&image);
    assert_eq!(image.stats.position_bytes, 2 * QUAD_BYTES);
    // マスク用メッシュの頂点位置を書き直したので、アトラスも描き直す。
    assert!(!image.stats.masks_reused);
}

#[test]
fn replacing_a_mask_texture_redraws_the_atlas() {
    let mut scene = clipped_scene(false, 1.0);
    use_alpha_texture(&mut scene, &["Mask"], 255);
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    assert_clipped(&frame(&mut renderer, &instance));
    // マスクのテクスチャを透明にすると、カバレッジは 0 になる。
    let Gpu { device, queue, .. } = gpu();
    renderer
        .set_texture(device, queue, 1, 1, 1, &[0; 4])
        .expect("texture");
    let image = frame(&mut renderer, &instance);
    assert!(!image.stats.masks_reused);
    image.assert_pixel((40, 32), NONE);
    assert_eq!(image.covered_pixels([0; 4]), 0);
}

// ---- 描画の統合 (T0208) ----

/// `scene` を描画の統合あり・なしで描き、結果が画素単位で一致することを確かめる。
/// (統合あり, 統合なし) の順に返す。
#[track_caller]
fn render_with_and_without_batching(scene: impl Fn() -> Scene) -> (Image, Image) {
    let batched = scene().render();
    let mut unbatched = scene();
    unbatched.options.batch_draws = false;
    let unbatched = unbatched.render();
    let differing = batched
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(unbatched.pixels.as_chunks::<4>().0)
        .enumerate()
        .find(|(_, (a, b))| a != b);
    if let Some((i, (a, b))) = differing {
        let (x, y) = (i as u32 % batched.size, i as u32 / batched.size);
        panic!("pixel ({x}, {y}): batched {a:?}, per-drawable {b:?}");
    }
    assert!(batched.covered_pixels([0; 4]) > 0, "scene draws something");
    assert_eq!(batched.stats.drawables, unbatched.stats.drawables);
    assert_eq!(batched.stats.state_changes, unbatched.stats.state_changes);
    assert_eq!(unbatched.stats.draw_calls, unbatched.stats.drawables);
    (batched, unbatched)
}

/// 重なり合う半透明の四角形 6 枚。色・不透明度・スクリーン色は要素ごとに違うが、描画の状態は同じ。
fn overlapping_tinted_quads() -> Scene {
    let mut meshes = Vec::new();
    for k in 0..6 {
        let x = 32.0 + 24.0 * k as f32;
        let mut mesh = quad(
            &format!("Q{k}"),
            [x, 48.0 + 8.0 * k as f32, x + 96.0, 208.0],
        );
        let t = k as f32 / 5.0;
        for kf in &mut mesh.keyforms.keyforms {
            kf.multiply_color = Rgb::new(1.0 - t, t, 0.5);
            kf.screen_color = Rgb::new(0.0, 0.0, t * 0.5);
            kf.opacity = 0.4 + 0.1 * k as f32;
            // 宣言と逆の順に描く (描画順で並べ替えたインデックスを使う)。
            kf.draw_order = 600.0 - k as f32;
        }
        meshes.push(mesh);
    }
    Scene::new(model_of(meshes))
}

#[test]
fn batching_matches_per_drawable_drawing() {
    let (batched, unbatched) = render_with_and_without_batching(overlapping_tinted_quads);
    assert_eq!(batched.stats.draw_calls, 1);
    assert_eq!(batched.stats.state_changes, 0);
    assert_eq!(unbatched.stats.draw_calls, 6);

    // マスクを読む要素と読まない要素、別の合成モード、入れ子、共通部分、複数のアトラス。
    render_with_and_without_batching(|| {
        let band = |i: f32| [64.0, 64.0 + 32.0 * i, 192.0, 96.0 + 32.0 * i];
        let mut scene = Scene::new(model_of(vec![
            red_quad("A", band(0.0), &["M1"], false),
            red_quad("B", band(1.0), &["M1"], true),
            red_quad("Free", [32.0, 32.0, 224.0, 224.0], &[], false),
            red_quad("C", band(2.0), &["M1", "M2"], false),
            red_quad("D", band(3.0), &["M2"], false),
            mask_quad("M1", [64.0, 0.0, 128.0, 256.0], 1.0),
            mask_quad("M2", [160.0, 0.0, 192.0, 256.0], 1.0),
        ]));
        edit_keyforms(&mut scene.model, "Free", |k| k.opacity = 0.3);
        tint(&mut scene.model, "C", [0.0, 1.0, 0.0]);
        scene
    });
    render_with_and_without_batching(|| mixed_scene(0.5));
    render_with_and_without_batching(|| {
        let mut model = fixtures::nested_masks();
        tint(&mut model, "Iris", [0.0, 0.0, 1.0]);
        tint(&mut model, "Pupil", [1.0, 0.0, 0.0]);
        Scene::new(model)
    });
    render_with_and_without_batching(|| {
        let mut model = fixtures::intersection_masks();
        tint(&mut model, "Face", [0.0, 0.0, 1.0]);
        tint(&mut model, "Tongue", [1.0, 0.0, 0.0]);
        Scene::new(model)
    });
    let (batched, _) = render_with_and_without_batching(|| {
        let mut scene = five_mask_sets();
        scene.options = shrinking_options(16);
        scene
    });
    assert!(batched.stats.mask_atlases > 1);
}

#[test]
fn unmasked_drawable_between_atlases_counts_the_atlas_change() {
    // C0..C2 はアトラス 0、C3・C4 はアトラス 1 を読む (一辺 16)。その間にマスクを読まない要素を挟む。
    // まとめるかどうかによらず、アトラス 0 → 1 の切り替えを 1 回と数える。
    let scene = || {
        let mut scene = five_mask_sets();
        scene.options = shrinking_options(16);
        for k in 0..5 {
            edit_keyforms(&mut scene.model, &format!("C{k}"), |kf| {
                kf.draw_order = 500.0 + k as f32;
            });
        }
        let mut free = quad("Free", [0.0, 224.0, 32.0, 256.0]);
        for kf in &mut free.keyforms.keyforms {
            kf.draw_order = 502.5;
        }
        scene.model.meshes.push(free);
        scene
    };
    let (batched, _) = render_with_and_without_batching(scene);
    assert_eq!(batched.stats.mask_atlases, 2);
    // M0..M4 (乗算合成) → C0..C2 と Free (アトラス 0) → C3・C4 (アトラス 1)。
    assert_eq!(batched.stats.draw_calls, 3);
    assert_eq!(batched.stats.state_changes, 2);
}

#[test]
fn adjacent_drawables_with_the_same_state_share_one_draw() {
    // 描画順: M1, M2 (乗算合成) → A..D (通常の合成。マスクは同じアトラス)。
    let band = |i: f32| [64.0, 64.0 + 32.0 * i, 192.0, 96.0 + 32.0 * i];
    let image = Scene::new(model_of(vec![
        red_quad("A", band(0.0), &["M1"], false),
        red_quad("B", band(1.0), &["M1"], true),
        red_quad("C", band(2.0), &["M1", "M2"], false),
        red_quad("D", band(3.0), &[], false),
        mask_quad("M1", [64.0, 0.0, 128.0, 256.0], 1.0),
        mask_quad("M2", [160.0, 0.0, 192.0, 256.0], 1.0),
    ]))
    .render();
    image.assert_pixel((20, 20), RED);
    image.assert_pixel((36, 20), NONE);
    image.assert_pixel((20, 44), RED);
    image.assert_pixel((36, 44), RED);
    assert_eq!(
        image.stats,
        FrameStats {
            drawables: 6,
            draw_calls: 2,
            state_changes: 1,
            ..image.stats
        }
    );
}

#[test]
fn texture_and_blend_changes_split_draws_in_order() {
    // 描画順 A (テクスチャ 0), B (0), C (1), D (0, 加算), E (0)。重なりの上は後の要素。
    let rect = [64.0, 64.0, 192.0, 192.0];
    let mut meshes = vec![
        red_quad("A", rect, &[], false),
        quad("B", rect),
        quad("C", rect),
        quad("D", rect),
        quad("E", [64.0, 64.0, 128.0, 192.0]),
    ];
    for (k, mesh) in meshes.iter_mut().enumerate() {
        for kf in &mut mesh.keyforms.keyforms {
            kf.draw_order = 500.0 + k as f32;
        }
    }
    meshes[3].blend = BlendMode::Additive;
    let mut scene = Scene::new(model_of(meshes));
    tint(&mut scene.model, "B", [0.0, 0.0, 1.0]);
    edit_keyforms(&mut scene.model, "C", |k| k.opacity = 0.5);
    tint(&mut scene.model, "D", [0.0, 0.25, 0.0]);
    tint(&mut scene.model, "E", [1.0, 0.0, 0.0]);
    let texture = scene.model.textures[0].clone();
    scene.model.textures.push(texture);
    mesh_mut(&mut scene.model, "C").texture = 1;
    scene.extra_textures.push([0, 255, 0, 255]);
    let image = scene.render();
    // 青の上に 0.5 の緑、その上に緑 0.25 を加算。左半分は最後に赤。
    image.assert_pixel((40, 32), [0.0, 0.75, 0.5, 1.0]);
    image.assert_pixel((20, 32), RED);
    assert_eq!(image.stats.draw_calls, 4, "{:?}", image.stats);
    assert_eq!(image.stats.state_changes, 3);
}

#[test]
fn draw_order_changes_reupload_indices() {
    // Top は X = 0 で奥 (400)、X = 1 で手前 (600)。
    let mut model = fixtures::single_quad();
    let mut top = model.meshes[0].clone();
    top.id = "Top".into();
    top.name = "Top".into();
    for (k, kf) in top.keyforms.keyforms.iter_mut().enumerate() {
        kf.draw_order = if k == 2 { 600.0 } else { 400.0 };
        kf.multiply_color = Rgb::new(0.0, 1.0, 0.0);
    }
    tint(&mut model, "Quad", [1.0, 0.0, 0.0]);
    model.meshes.push(top);
    let (mut renderer, mut instance) = Scene::new(model).prepare().expect("prepare");
    let image = frame(&mut renderer, &instance);
    image.assert_pixel(INSIDE, RED);
    // 四角形 2 枚 × 6 インデックス × 4 バイト。
    assert_eq!(image.stats.index_bytes, 2 * 6 * 4);
    assert_eq!(image.stats.draw_calls, 1);
    let s = image.stats;
    assert_eq!(
        s.uploaded_bytes,
        s.position_bytes + s.uniform_bytes + s.index_bytes
    );

    // 動いても並びが同じならインデックスは転送しない。
    set_param(&mut instance, "X", 0.25);
    instance.update();
    let image = frame(&mut renderer, &instance);
    assert_eq!(image.stats.index_bytes, 0);
    assert!(image.stats.position_bytes > 0);

    set_param(&mut instance, "X", 1.0);
    instance.update();
    let image = frame(&mut renderer, &instance);
    image.assert_pixel((52, 32), [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(image.stats.index_bytes, 2 * 6 * 4);
    assert_eq!(image.stats.draw_calls, 1);
}

#[test]
fn many_drawables_read_their_own_values() {
    // 80 枚 (要素ごとの値のテクスチャは 1 行 64 要素なので 2 行目にまたがる)。
    // 10 × 8 のマス (1 マス = キャンバス 24 = 画素 6) に並べ、赤の強さを番号で変える。
    const COUNT: usize = 80;
    let scene = || {
        let meshes = (0..COUNT)
            .map(|k| {
                let (x, y) = ((k % 10) as f32 * 24.0, (k / 10) as f32 * 24.0);
                let mut mesh = quad(&format!("Q{k}"), [x, y, x + 24.0, y + 24.0]);
                for kf in &mut mesh.keyforms.keyforms {
                    kf.multiply_color = Rgb::new(k as f32 / (COUNT - 1) as f32, 0.0, 1.0);
                }
                mesh
            })
            .collect();
        Scene::new(model_of(meshes))
    };
    let (image, _) = render_with_and_without_batching(scene);
    assert_eq!(image.stats.draw_calls, 1);
    for k in [0, 63, 64, 79] {
        let (x, y) = ((k % 10) as u32 * 6 + 3, (k / 10) as u32 * 6 + 3);
        image.assert_pixel((x, y), [k as f32 / (COUNT - 1) as f32, 0.0, 1.0, 1.0]);
    }
}

// --- ライティング (docs/spec/rendering.md#ライティング) ---

/// 法線 `[x, y, z]` (y は画像の上向き) を法線マップの texel にする (`c = (n + 1) / 2`)。
fn normal_texel(n: [f32; 3]) -> [u8; 4] {
    let [x, y, z] = n.map(|v| ((v + 1.0) * 0.5 * 255.0).round() as u8);
    [x, y, z, 255]
}

/// 画像の右 (+u) を向いた法線。
const NORMAL_RIGHT: [f32; 3] = [0.6, 0.0, 0.8];
/// 画像の上 (−v) を向いた法線。
const NORMAL_UP: [f32; 3] = [0.0, 0.6, 0.8];

fn gray(v: f32) -> [f32; 4] {
    [v, v, v, 1.0]
}

fn lights(ambient: f32, lights: Vec<Light>) -> Lighting {
    Lighting {
        ambient: Rgb::new(ambient, ambient, ambient),
        lights,
    }
}

/// 光源へ向かう向き `l` (キャンバス座標) から照らす白の平行光源 (`direction` は光が進む向きなので `−l`)。
fn light_from([x, y, z]: [f32; 3]) -> Light {
    Light::Directional {
        direction: [-x, -y, -z],
        color: Rgb::WHITE,
    }
}

/// single_quad の四角形 (キャンバス 64..192) の中心 (128, 128) のまわりに、`map` で四隅を写したモデル。
/// ライティングを受ける。四隅は UV の (0,0), (1,0), (1,1), (0,1) の順。
fn lit_quad(map: impl Fn([f32; 2]) -> [f32; 2]) -> Model {
    let mut model = fixtures::single_quad();
    model.lighting = true;
    let corners = [[-64.0, -64.0], [64.0, -64.0], [64.0, 64.0], [-64.0, 64.0]].map(|c| {
        let [x, y] = map(c);
        Vec2::new(128.0 + x, 128.0 + y)
    });
    for kf in &mut model.meshes[0].keyforms.keyforms {
        kf.positions = corners.to_vec();
    }
    model
}

fn identity([x, y]: [f32; 2]) -> [f32; 2] {
    [x, y]
}

/// 法線 1 つの法線マップ (1x1) を持つ、ライティングを受ける四角形。
fn normal_mapped_scene(model: Model, normal: [f32; 3], lighting: Lighting) -> Scene {
    let mut scene = Scene::new(model);
    scene
        .normal_maps
        .push((0, 1, normal_texel(normal).to_vec()));
    scene.lighting = Some(lighting);
    scene
}

#[track_caller]
fn assert_same_pixels(a: &Image, b: &Image) {
    let differing = a
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.pixels.as_chunks::<4>().0)
        .enumerate()
        .find(|(_, (a, b))| a != b);
    if let Some((i, (pa, pb))) = differing {
        let (x, y) = (i as u32 % a.size, i as u32 / a.size);
        panic!("pixel ({x}, {y}): {pa:?} != {pb:?}");
    }
}

#[test]
fn lighting_scales_lit_drawables() {
    // 法線マップなし (法線 (0, 0, 1))。正面の光は dot = 1、真横の光は dot = 0。
    let mut scene = Scene::new(lit_quad(identity));
    scene.clear = clear_color([0.0, 0.0, 1.0, 1.0]);
    let mut half_front = light_from([0.0, 0.0, 1.0]);
    if let Light::Directional { color, .. } = &mut half_front {
        *color = Rgb::new(0.5, 0.25, 0.5);
    }
    scene.lighting = Some(lights(0.25, vec![half_front, light_from([1.0, 0.0, 0.0])]));
    let image = scene.render();
    image.assert_pixel(INSIDE, [0.75, 0.5, 0.75, 1.0]);
    image.assert_pixel(OUTSIDE, [0.0, 0.0, 1.0, 1.0]);

    // 乗算色・スクリーン色の後、不透明度の前に掛かる (乗算済みのまま)。
    let mut model = lit_quad(identity);
    edit_keyforms(&mut model, "Quad", |k| {
        k.multiply_color = Rgb::new(1.0, 0.5, 0.0);
        k.screen_color = Rgb::new(0.0, 0.0, 0.5);
        k.opacity = 0.5;
    });
    let mut scene = Scene::new(model);
    scene.lighting = Some(lights(0.5, vec![]));
    scene
        .render()
        .assert_pixel(INSIDE, [0.25, 0.125, 0.125, 0.5]);
}

#[test]
fn unlit_drawables_ignore_lighting() {
    let strong = || {
        lights(
            0.1,
            vec![
                light_from([1.0, -1.0, 0.5]),
                Light::Point {
                    position: [100.0, 100.0, 20.0],
                    color: Rgb::new(3.0, 0.0, 0.0),
                    range: 100.0,
                },
            ],
        )
    };
    let reference = Scene::new(fixtures::single_quad()).render();

    // モデルがライティングしない (法線マップがあっても)。
    let mut scene = normal_mapped_scene(fixtures::single_quad(), NORMAL_RIGHT, strong());
    scene.model.lighting = false;
    assert_same_pixels(&scene.render(), &reference);

    // メッシュがライティングを受けない。
    let mut scene = normal_mapped_scene(lit_quad(identity), NORMAL_RIGHT, strong());
    scene.model.meshes[0].lit = false;
    assert_same_pixels(&scene.render(), &reference);

    // 既定の光源 (環境光が白で光源なし) は、ライティングを受けても同じ色になる。
    let mut scene = Scene::new(lit_quad(identity));
    scene
        .normal_maps
        .push((0, 1, normal_texel(NORMAL_RIGHT).to_vec()));
    assert_same_pixels(&scene.render(), &reference);
}

#[test]
fn normal_map_tilts_the_shading() {
    // 画像の右を向いた法線: 右からの光で 0.6、左・上からの光で 0。
    let render = |normal, l| {
        normal_mapped_scene(lit_quad(identity), normal, lights(0.0, vec![light_from(l)])).render()
    };
    render(NORMAL_RIGHT, [1.0, 0.0, 0.0]).assert_pixel(INSIDE, gray(0.6));
    render(NORMAL_RIGHT, [-1.0, 0.0, 0.0]).assert_pixel(INSIDE, gray(0.0));
    render(NORMAL_RIGHT, [0.0, -1.0, 0.0]).assert_pixel(INSIDE, gray(0.0));
    // G は画像の上向き: キャンバスの上 (−y) からの光で明るくなる。
    render(NORMAL_UP, [0.0, -1.0, 0.0]).assert_pixel(INSIDE, gray(0.6));
    render(NORMAL_UP, [0.0, 1.0, 0.0]).assert_pixel(INSIDE, gray(0.0));
    // 正面の光は z 成分の 0.8。
    render(NORMAL_RIGHT, [0.0, 0.0, 1.0]).assert_pixel(INSIDE, gray(0.8));
}

/// 四角形の変形 (UV → キャンバスの写像) の線形部分。
const TRANSFORMS: [(&str, [[f32; 2]; 2]); 6] = [
    // 列優先: [写した u 軸, 写した v 軸]
    ("identity", [[1.0, 0.0], [0.0, 1.0]]),
    ("rotate 90", [[0.0, 1.0], [-1.0, 0.0]]),
    ("rotate 180", [[-1.0, 0.0], [0.0, -1.0]]),
    ("mirror x", [[-1.0, 0.0], [0.0, 1.0]]),
    ("mirror y", [[1.0, 0.0], [0.0, -1.0]]),
    ("transpose", [[0.0, 1.0], [1.0, 0.0]]),
];

fn apply(m: [[f32; 2]; 2], [x, y]: [f32; 2]) -> [f32; 2] {
    [m[0][0] * x + m[1][0] * y, m[0][1] * x + m[1][1] * y]
}

#[test]
fn normals_follow_rotation_and_reflection() {
    // 法線は変形と一緒に回り、反転すれば鏡に映る: 画面での向きは、UV 空間での向き
    // (画像の上は −v) を変形の線形部分で写したもの。その向きから照らしたときだけ明るい。
    let axes = [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]];
    for (name, m) in TRANSFORMS {
        for (normal, uv_direction) in [(NORMAL_RIGHT, [1.0, 0.0]), (NORMAL_UP, [0.0, -1.0])] {
            let expected = apply(m, uv_direction);
            for [lx, ly] in axes {
                let image = normal_mapped_scene(
                    lit_quad(|c| apply(m, c)),
                    normal,
                    lights(0.0, vec![light_from([lx, ly, 0.0])]),
                )
                .render();
                let facing = expected == [lx, ly];
                let actual = image.at(INSIDE);
                assert!(
                    (i32::from(actual[0]) - if facing { 153 } else { 0 }).abs() <= TOLERANCE,
                    "{name}: normal {normal:?} lit from {:?}: got {actual:?}",
                    [lx, ly]
                );
            }
        }
    }
}

#[test]
fn lighting_is_in_canvas_space_regardless_of_the_view() {
    // Y を反転しないビューでは画像が上下逆に写るが、光はキャンバス座標なので同じ向きから当たる。
    let render = |flip_y| {
        let mut scene = normal_mapped_scene(
            lit_quad(identity),
            NORMAL_UP,
            lights(0.0, vec![light_from([0.0, -1.0, 0.0])]),
        );
        scene.view = canvas_view(flip_y);
        scene.render()
    };
    render(true).assert_pixel(INSIDE, gray(0.6));
    render(false).assert_pixel(INSIDE, gray(0.6));
}

#[test]
fn point_light_attenuates_with_distance() {
    let point = |z: f32, range: f32| Light::Point {
        position: [128.0, 128.0, z],
        color: Rgb::WHITE,
        range,
    };
    let render = |light| {
        let mut scene = Scene::new(lit_quad(identity));
        scene.lighting = Some(lights(0.0, vec![light]));
        scene.render()
    };
    // 真上 (距離 64) から: dot ≈ 1、減衰 (1 − 64 / 128)² = 0.25。
    let image = render(point(64.0, 128.0));
    image.assert_pixel(INSIDE, gray(0.25));
    // 四角形の角 (距離 ≈ 110) は暗い。
    let corner = image.at((17, 17))[0];
    assert!(corner < 10, "corner {corner}");
    // 減衰しない点光源は、真上で 1。
    render(point(64.0, f32::INFINITY)).assert_pixel(INSIDE, gray(1.0));
    // 範囲の外は照らさない。
    render(point(200.0, 150.0)).assert_pixel(INSIDE, gray(0.0));
}

/// 半球のふくらみの法線マップ (一辺 `size`)。
fn dome_normal_map(size: u32) -> Vec<u8> {
    let mut rgba = Vec::new();
    for j in 0..size {
        for i in 0..size {
            let x = (i as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let y = (j as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let r2 = x * x + y * y;
            // 画像の下向き (+v) の y を、法線マップの上向きに直す。
            let n = if r2 < 1.0 {
                [x, -y, (1.0 - r2).sqrt()]
            } else {
                [0.0, 0.0, 1.0]
            };
            rgba.extend(normal_texel(n));
        }
    }
    rgba
}

#[test]
fn dome_is_lit_from_the_light_side_after_deformation() {
    // 左上の手前から照らす。どう変形しても、画面上でふくらみの左上が明るく右下が暗い。
    // (ふくらみは点対称なので、法線が正しく回っていれば見た目は変形によらない。)
    for (name, m) in TRANSFORMS {
        let mut scene = Scene::new(lit_quad(|c| apply(m, c)));
        scene.size = 128;
        scene.normal_maps.push((0, 32, dome_normal_map(32)));
        scene.lighting = Some(lights(0.1, vec![light_from([-1.0, -1.0, 1.0])]));
        let image = scene.render();
        // 四角形は画素 32..96、ふくらみの中心は 64。
        let upper_left = image.at((48, 48))[0];
        let lower_right = image.at((80, 80))[0];
        let upper_right = image.at((80, 48))[0];
        let lower_left = image.at((48, 80))[0];
        assert!(
            upper_left > 200 && lower_right < 40,
            "{name}: upper left {upper_left}, lower right {lower_right}"
        );
        assert!(
            (i32::from(upper_right) - i32::from(lower_left)).abs() <= 4,
            "{name}: upper right {upper_right}, lower left {lower_left}"
        );
    }
}

#[test]
fn lighting_batches_like_unlit_drawing() {
    // ライティングを受ける要素・受けない要素・法線マップの有無・マスクが混ざっても、
    // 描画の状態 (テクスチャ・合成モード・マスクアトラス) が同じならまとめて描き、結果は要素ごとに描いた場合と一致する。
    let scene = || {
        let mut scene = overlapping_tinted_quads();
        scene.model.lighting = true;
        scene.model.meshes[1].lit = false;
        scene.model.meshes[4].lit = false;
        scene.normal_maps.push((0, 32, dome_normal_map(32)));
        scene.lighting = Some(lights(
            0.2,
            vec![
                light_from([-1.0, -1.0, 1.0]),
                Light::Point {
                    position: [160.0, 160.0, 40.0],
                    color: Rgb::new(0.2, 0.4, 1.0),
                    range: 200.0,
                },
            ],
        ));
        scene
    };
    let (batched, unbatched) = render_with_and_without_batching(scene);
    assert_eq!(batched.stats.draw_calls, 1);
    assert_eq!(unbatched.stats.draw_calls, 6);
    // ライティングしない場合と描画の数は同じ。
    let unlit = overlapping_tinted_quads().render();
    assert_eq!(unlit.stats.draw_calls, batched.stats.draw_calls);
    assert_eq!(unlit.stats.state_changes, batched.stats.state_changes);

    // 法線マップはテクスチャと一緒に描画の状態になる: 法線マップの違うテクスチャの間で描画が分かれる。
    render_with_and_without_batching(|| {
        let mut scene = scene();
        let texture = scene.model.textures[0].clone();
        scene.model.textures.push(texture);
        scene.extra_textures.push([255; 4]);
        for k in [2, 3] {
            scene.model.meshes[k].texture = 1;
        }
        // テクスチャ 1 の法線マップは平ら (テクスチャ 0 はふくらみ)。
        scene
            .normal_maps
            .push((1, 1, normal_texel([0.0, 0.0, 1.0]).to_vec()));
        scene
    });
    render_with_and_without_batching(|| {
        let mut scene = mixed_scene(0.5);
        scene.model.lighting = true;
        scene.normal_maps.push((0, 32, dome_normal_map(32)));
        scene.lighting = Some(lights(0.3, vec![light_from([1.0, 0.0, 1.0])]));
        scene
    });
}

#[test]
fn lighting_is_uploaded_only_when_used_and_changed() {
    let (mut renderer, instance) = Scene::new(lit_quad(identity)).prepare().expect("prepare");
    let first = frame(&mut renderer, &instance);
    let static_frame = frame(&mut renderer, &instance);
    assert_eq!(static_frame.stats.uploaded_bytes, 0);

    renderer.set_lighting(&lights(0.5, vec![]));
    let image = frame(&mut renderer, &instance);
    image.assert_pixel(INSIDE, gray(0.5));
    let lighting_bytes = image.stats.uniform_bytes;
    assert!(lighting_bytes > 0);
    assert_eq!(image.stats.uploaded_bytes, lighting_bytes);
    // 同じ光源をもう一度渡しても転送しない。
    renderer.set_lighting(&lights(0.5, vec![]));
    assert_eq!(frame(&mut renderer, &instance).stats.uploaded_bytes, 0);

    // ライティングを受ける要素がないモデルでは、光源を転送しない (初めのフレームの転送量も変わらない)。
    let (mut renderer, instance) = Scene::new(fixtures::single_quad())
        .prepare()
        .expect("prepare");
    renderer.set_lighting(&lights(0.5, vec![]));
    let unlit = frame(&mut renderer, &instance);
    assert_eq!(
        unlit.stats.uniform_bytes + lighting_bytes,
        first.stats.uniform_bytes
    );
    renderer.set_lighting(&lights(0.25, vec![]));
    assert_eq!(frame(&mut renderer, &instance).stats.uploaded_bytes, 0);
}

#[test]
fn normal_maps_can_be_set_replaced_and_removed() {
    let Gpu { device, queue, .. } = gpu();
    let mut scene = Scene::new(lit_quad(identity));
    scene.lighting = Some(lights(0.0, vec![light_from([1.0, 0.0, 0.0])]));
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    // 法線マップなし: 真横からの光は当たらない。
    frame(&mut renderer, &instance).assert_pixel(INSIDE, gray(0.0));
    let right = normal_texel(NORMAL_RIGHT);
    renderer
        .set_normal_map(device, queue, 0, 1, 1, &right)
        .expect("set normal map");
    frame(&mut renderer, &instance).assert_pixel(INSIDE, gray(0.6));
    // テクスチャを登録し直しても法線マップは残る。
    renderer
        .set_texture(device, queue, 0, 1, 1, &[255; 4])
        .expect("set texture");
    frame(&mut renderer, &instance).assert_pixel(INSIDE, gray(0.6));
    renderer.remove_normal_map(device, 0);
    frame(&mut renderer, &instance).assert_pixel(INSIDE, gray(0.0));
    // 大きさの合わないデータはエラー。
    assert!(matches!(
        renderer.set_normal_map(device, queue, 0, 2, 2, &right),
        Err(RenderError::InvalidTextureData { .. })
    ));

    // テクスチャより先に登録してもよい。
    let mut renderer = Renderer::new(device, FORMAT);
    renderer
        .set_normal_map(device, queue, 0, 1, 1, &right)
        .expect("set normal map");
    renderer
        .set_texture(device, queue, 0, 1, 1, &[255; 4])
        .expect("set texture");
    renderer.set_lighting(&lights(0.0, vec![light_from([1.0, 0.0, 0.0])]));
    frame(&mut renderer, &instance).assert_pixel(INSIDE, gray(0.6));
}

#[test]
fn textures_beyond_the_limits_are_errors() {
    let Gpu { device, queue, .. } = gpu();
    let mut scene = Scene::new(fixtures::single_quad());
    scene.texture = [0; 4];
    let (mut renderer, instance) = scene.prepare().expect("prepare");
    // 一辺がデバイスの上限を超える (wgpu の検証エラーでパニックせず、エラーを返す)。
    let max = device.limits().max_texture_dimension_2d;
    let row = vec![255; (max as usize + 1) * 4];
    assert!(matches!(
        renderer.set_texture(device, queue, 0, max + 1, 1, &row),
        Err(RenderError::TextureTooLarge { index: 0, width, height: 1, max: m })
            if width == max + 1 && m == max
    ));
    assert!(matches!(
        renderer.set_normal_map(device, queue, 0, 1, max + 1, &row),
        Err(RenderError::NormalMapTooLarge { index: 0, width: 1, height, max: m })
            if height == max + 1 && m == max
    ));
    assert!(matches!(
        renderer.check_texture(0, max, max + 1),
        Err(RenderError::TextureTooLarge { .. })
    ));
    assert!(renderer.check_texture(0, max, max).is_ok());
    assert!(renderer.check_normal_map(0, max, max).is_ok());
    // 番号の上限 (表を伸ばしてメモリを使い切らない)。
    for index in [MAX_TEXTURES, u32::MAX] {
        assert!(matches!(
            renderer.set_texture(device, queue, index, 1, 1, &[255; 4]),
            Err(RenderError::TextureIndexOutOfRange(i)) if i == index
        ));
        assert!(matches!(
            renderer.set_normal_map(device, queue, index, 1, 1, &[255; 4]),
            Err(RenderError::TextureIndexOutOfRange(i)) if i == index
        ));
    }
    // 失敗した登録は何も変えない。
    assert!(matches!(
        render_frame(
            &mut renderer,
            &instance,
            &normal_view(),
            SIZE,
            wgpu::Color::TRANSPARENT
        ),
        Err(RenderError::MissingTexture(0))
    ));
    // 上限ちょうどの番号は受け付ける。
    renderer
        .set_texture(device, queue, MAX_TEXTURES - 1, 1, 1, &[255; 4])
        .expect("largest index");
    renderer
        .set_texture(device, queue, 0, 1, 1, &[255; 4])
        .expect("set texture");
    frame(&mut renderer, &instance).assert_pixel(INSIDE, [1.0; 4]);
}
