//! 生成した法線マップを参照レンダラ (`ugoe-render`) で描くテスト。`just test-gpu` で実行する。
//!
//! 環境変数 `UGOE_NORMAL_RENDER_DUMP` にディレクトリを指定すると、描いた画像を PNG で書き出す (目視確認用)。
#![cfg(feature = "gpu-tests")]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ugoe_ai::depth::DepthMap;
use ugoe_ai::normal::{NormalMapOptions, plan_depth_normal_maps, propose_bulge_normal_maps};
use ugoe_core::{Rgb, Vec2};
use ugoe_editor_core::texture::{decode_rgba_png, encode_rgba_png};
use ugoe_editor_core::{Command, Document, Project, PsdImportOptions, import_psd};
use ugoe_model::{TextureRef, fixtures};
use ugoe_render::{Light, Lighting, Renderer, View};
use ugoe_runtime::{CompiledModel, ModelInstance};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Gpu {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// テスト全体で 1 つのデバイスを共有し、破棄しない (WSL2 の dzn が破棄時に落ちるため。ugoe-render のテストと同じ)。
fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap_or_else(|e| panic!("no wgpu adapter available ({e})"));
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ugoe-ai normal map test"),
            required_limits:
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .expect("request device");
        Gpu {
            _instance: instance,
            device,
            queue,
        }
    })
}

/// 左上の手前から照らす光 (環境光は弱め)。
fn side_light() -> Lighting {
    Lighting {
        ambient: Rgb::new(0.35, 0.35, 0.35),
        lights: vec![Light::Directional {
            direction: [1.0, 0.6, -0.8],
            color: Rgb::new(0.9, 0.9, 0.9),
        }],
    }
}

/// 描いた画像 (乗算済みアルファの RGBA8)。
struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    fn luma(&self, x: u32, y: u32) -> f32 {
        let i = ((y * self.width + x) * 4) as usize;
        let p = &self.pixels[i..i + 3];
        (f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2])) / 3.0
    }

    /// `UGOE_NORMAL_RENDER_DUMP` があれば `<name>.png` に書き出す (ストレートアルファに直す)。
    fn dump(&self, name: &str) {
        let Some(dir) = std::env::var_os("UGOE_NORMAL_RENDER_DUMP") else {
            return;
        };
        let mut rgba = self.pixels.clone();
        for p in rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(p[3]);
            if a > 0 {
                for c in &mut p[..3] {
                    *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        let path = PathBuf::from(dir).join(format!("{name}.png"));
        std::fs::write(&path, encode_rgba_png(self.width, self.height, &rgba)).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

/// ドキュメントのモデルを、テクスチャと法線マップを登録して描く。幅 `width` (64 の倍数) でキャンバス全体を写す。
fn render(doc: &Document, lighting: &Lighting, width: u32) -> Image {
    let Gpu { device, queue, .. } = gpu();
    let model = doc.model();
    let canvas = Vec2::new(model.meta.canvas.width, model.meta.canvas.height);
    let height = ((width as f32 * canvas.y / canvas.x).round() as u32).max(1);
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    instance.update();

    let mut renderer = Renderer::new(device, FORMAT);
    for (i, texture) in model.textures.iter().enumerate() {
        let (w, h, mut rgba) = decode_rgba_png(doc.texture(&texture.name).unwrap()).unwrap();
        for p in rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(p[3]);
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * a + 127) / 255) as u8;
            }
        }
        renderer
            .set_texture(device, queue, i as u32, w, h, &rgba)
            .unwrap();
        if let Some(name) = &texture.normal_map {
            let (w, h, rgba) = decode_rgba_png(doc.texture(name).unwrap()).unwrap();
            renderer
                .set_normal_map(device, queue, i as u32, w, h, &rgba)
                .unwrap();
        }
    }
    renderer.set_lighting(lighting);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
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
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    // キャンバス (Y 下向き) をクリップ座標全体に写す。
    let view = View {
        view_proj: [
            [2.0 / canvas.x, 0.0, 0.0],
            [0.0, -2.0 / canvas.y, 0.0],
            [-1.0, 1.0, 1.0],
        ],
    };
    renderer
        .encode(device, queue, &mut encoder, &target_view, &view, &instance)
        .unwrap();

    let bytes_per_row = width * 4;
    assert_eq!(bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * height),
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
                rows_per_image: Some(height),
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
    Image {
        width,
        height,
        pixels,
    }
}

fn apply_bulge(doc: &mut Document) {
    let proposal = propose_bulge_normal_maps(
        doc.model(),
        doc.textures(),
        &BTreeMap::new(),
        &NormalMapOptions::default(),
    )
    .unwrap();
    doc.apply(Command::Batch {
        label: proposal.title,
        commands: proposal.commands,
    })
    .unwrap();
}

#[test]
fn bulged_disc_is_lit_from_the_light_side() {
    // キャンバス 256 の中央 (64..192) の四角形に、円のテクスチャを貼る。
    let mut model = fixtures::single_quad();
    model.textures = vec![TextureRef {
        name: "disc".into(),
        width: 64,
        height: 64,
        normal_map: None,
    }];
    let rgba: Vec<u8> = (0..64 * 64)
        .flat_map(|i| {
            let (x, y) = ((i % 64) as f32 - 31.5, (i / 64) as f32 - 31.5);
            [230, 230, 230, if x.hypot(y) < 30.0 { 255 } else { 0 }]
        })
        .collect();
    let textures = [("disc".to_owned(), Arc::from(encode_rgba_png(64, 64, &rgba)))].into();
    let mut doc = Document::with_textures(model, textures);
    apply_bulge(&mut doc);
    let image = render(&doc, &side_light(), 256);
    image.dump("disc-lit");
    let q = doc.model().meshes[0].keyforms.keyforms[1].positions[0];
    let (x0, y0) = (q.x as u32, q.y as u32);
    // 円の左上の側は右下の側より明るく、中心はその間。
    let upper_left = image.luma(x0 + 32, y0 + 40);
    let lower_right = image.luma(x0 + 96, y0 + 88);
    let center = image.luma(x0 + 64, y0 + 64);
    assert!(
        upper_left > center + 10.0 && center > lower_right + 10.0,
        "{upper_left} {center} {lower_right}"
    );
}

/// 胸から上の PSD を取り込み、ライティングを有効にしたドキュメント。
fn bustup() -> Document {
    let name = "comfyui-see-through-bustup-parts.psd";
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures/psd-real")
        .join(name);
    let bytes = std::fs::read(path).unwrap();
    let mut doc = Document::new(Project::empty(name).model);
    let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(name)).unwrap();
    doc.apply(import.command).unwrap();
    doc.apply(Command::SetLighting { enabled: true }).unwrap();
    doc
}

/// 描いた 2 枚で、8 を超えて違う値の数。
fn changed(a: &Image, b: &Image) -> usize {
    a.pixels
        .iter()
        .zip(&b.pixels)
        .filter(|(a, b)| a.abs_diff(**b) > 8)
        .count()
}

#[test]
fn bustup_with_generated_normal_maps() {
    let mut doc = bustup();

    let flat = render(&doc, &side_light(), 512);
    flat.dump("bustup-flat");
    apply_bulge(&mut doc);
    let lit = render(&doc, &side_light(), 512);
    lit.dump("bustup-bulge");
    let unlit = render(&doc, &Lighting::default(), 512);
    unlit.dump("bustup-unlit");

    // 法線マップで陰影が付く (平らな法線のときと画素が変わる)。光源がなければ変わらない
    // (環境光が白なので、ライティングを受けない場合と同じ色)。
    let n = changed(&flat, &lit);
    assert!(n > flat.pixels.len() / 50, "{n}");
    doc.apply(Command::SetLighting { enabled: false }).unwrap();
    let off = render(&doc, &Lighting::default(), 512);
    assert!(
        off.pixels
            .iter()
            .zip(&unlit.pixels)
            .all(|(a, b)| a.abs_diff(*b) <= 2)
    );
}

#[test]
fn bustup_with_depth_normal_maps() {
    // 深度推定の代わりに、人物を覆う楕円体の深度マップ (中央が手前) を使う。
    let n = 64u32;
    let values = (0..n * n)
        .map(|i| {
            let u = ((i % n) as f32 + 0.5) / n as f32;
            let v = ((i / n) as f32 + 0.5) / n as f32;
            let r2 = ((u - 0.5) / 0.45).powi(2) + ((v - 0.55) / 0.5).powi(2);
            (1.0 - r2).max(0.0).sqrt()
        })
        .collect();
    let map = DepthMap::new(n, n, values).unwrap();
    let mut doc = bustup();
    let flat = render(&doc, &side_light(), 512);
    let proposal = plan_depth_normal_maps(
        doc.model(),
        doc.textures(),
        &map,
        &BTreeMap::new(),
        &NormalMapOptions::default(),
    )
    .unwrap();
    doc.apply(Command::Batch {
        label: proposal.title,
        commands: proposal.commands,
    })
    .unwrap();
    let lit = render(&doc, &side_light(), 512);
    lit.dump("bustup-depth");
    let n = changed(&flat, &lit);
    assert!(n > flat.pixels.len() / 50, "{n}");
}
