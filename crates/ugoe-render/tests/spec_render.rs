//! 公開仕様の描画の例 (`spec/examples/render-*/render.json`) を、参照レンダラが満たすことの検査。
//! 例の形式と許容誤差は `spec/examples/README.md#rendering-examples`。`just test-gpu` で実行する。
//!
//! `render.json` の期待値は仕様の式から導いた値。このテストの側で値を合わせてはならない。
//! 参照レンダラと食い違う画素は [`KNOWN_MISMATCHES`] に理由を付けて挙げる。
#![cfg(feature = "gpu-tests")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;
use ugoe_core::Rgb;
use ugoe_format::image::{ImageLimits, decode_png};
use ugoe_format::model_from_json;
use ugoe_model::validate::validate;
use ugoe_render::{Light, Lighting, Renderer, View};
use ugoe_runtime::{CompiledModel, ModelInstance};

/// 描画先の形式。仕様は合成のたびに `[0, 1]` に丸める (`spec/rendering.md#conformance`) ので、
/// 8 ビット unorm の丸めがそれに当たる。乗算済みの色が alpha を超える画素や、alpha 0 で色を持つ
/// 画素も unorm はそのまま保つ (チャンネルどうしを比べて丸めない)。量子化の誤差は合成ごとに
/// 高々 0.5/255 で、許容誤差 3 に収まる。
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// 各チャンネルの許容誤差 (乗算済み、0..255)。`spec/examples/README.md#rendering-tolerances`。
const TOLERANCE: f32 = 3.0;

/// 参照レンダラが満たさない画素 `(例, case, [列, 行])`。理由をコメントに書く。
const KNOWN_MISMATCHES: &[(&str, usize, [u32; 2])] = &[];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderExample {
    model: String,
    #[allow(dead_code)]
    summary: String,
    textures: Vec<Image>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    width: u32,
    height: u32,
    /// 乗算していない 0..255 の RGBA。`png` があればそれを読んだ結果。
    rgba: Vec<u8>,
    /// 画像の PNG の中身 (`spec/examples/README.md#file-examples` の `png`)。
    png: Option<PngSpec>,
    normal_map: Option<Box<Image>>,
}

/// PNG の中身の記述。描画の例が使うもの (インターレースなし・色のチャンクなし) だけを受け付ける。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PngSpec {
    color_type: u8,
    bit_depth: u8,
    palette: Option<Vec<[u8; 3]>>,
    transparency: Option<Vec<u16>>,
    /// 画素ごと・行ごとのサンプル (ビット深度のまま)。
    samples: Vec<u16>,
}

impl PngSpec {
    /// 記述の PNG を `png` クレートで書く (走査線のフィルタは任意。読んだ画素は同じ)。
    fn encode(&self, width: u32, height: u32) -> Vec<u8> {
        let color = png::ColorType::from_u8(self.color_type).expect("color type");
        let depth = png::BitDepth::from_u8(self.bit_depth).expect("bit depth");
        let channels = color.samples();
        let bits = usize::from(self.bit_depth);
        let stride = (width as usize * channels * bits).div_ceil(8);
        let mut data = vec![0u8; stride * height as usize];
        for (row, samples) in self.samples.chunks(width as usize * channels).enumerate() {
            let line = &mut data[row * stride..(row + 1) * stride];
            for (i, &v) in samples.iter().enumerate() {
                let bit = i * bits;
                match bits {
                    16 => line[bit / 8..bit / 8 + 2].copy_from_slice(&v.to_be_bytes()),
                    8 => line[bit / 8] = v as u8,
                    _ => line[bit / 8] |= (v as u8) << (8 - bits - bit % 8),
                }
            }
        }
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some(palette) = &self.palette {
            encoder.set_palette(palette.as_flattened().to_vec());
        }
        if let Some(t) = &self.transparency {
            let bytes: Vec<u8> = if color == png::ColorType::Indexed {
                t.iter().map(|&a| a as u8).collect()
            } else {
                t.iter().flat_map(|v| v.to_be_bytes()).collect()
            };
            encoder.set_trns(bytes);
        }
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&data).expect("PNG data");
        writer.finish().expect("PNG");
        out
    }
}

impl Image {
    /// 乗算していない画素。`png` があれば、その PNG を参照実装の読み込み (`decode_png`) で読んだもので、
    /// それが `rgba` と同じことも確かめる。
    fn decoded(&self, label: &str) -> Vec<u8> {
        let Some(spec) = &self.png else {
            return self.rgba.clone();
        };
        let bytes = spec.encode(self.width, self.height);
        let decoded = decode_png(&bytes, ImageLimits::default()).expect("decode PNG");
        assert_eq!(decoded.pixels, self.rgba, "{label}: decoded PNG");
        decoded.pixels
    }

    /// 描画に渡す乗算済みの画素。PNG の画像は参照実装の `premultiply` で乗算済みにする。
    fn premultiplied(&self, label: &str) -> Vec<u8> {
        let mut pixels = self.decoded(label);
        if self.png.is_none() {
            return premultiply(&pixels);
        }
        ugoe_format::image::premultiply(&mut pixels);
        pixels
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    #[allow(dead_code)]
    summary: String,
    #[serde(default)]
    parameters: BTreeMap<String, f32>,
    target: [u32; 2],
    /// キャンバスの `(x, y)` を描画先の px の `(a·x + c·y + e, b·x + d·y + f)` に写す `[a, b, c, d, e, f]`。
    view: Option<[f32; 6]>,
    lights: Option<Lights>,
    pixels: Vec<Pixel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lights {
    #[serde(default = "white")]
    ambient: [f32; 3],
    #[serde(default)]
    directional: Vec<Directional>,
    #[serde(default)]
    point: Vec<Point>,
}

fn white() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Directional {
    direction: [f32; 3],
    color: [f32; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    position: [f32; 3],
    color: [f32; 3],
    /// 省略すると範囲は無限。
    range: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pixel {
    at: [u32; 2],
    /// 乗算済みの 0..255 (実数)。
    rgba: [f32; 4],
}

fn rgb([r, g, b]: [f32; 3]) -> Rgb {
    Rgb::new(r, g, b)
}

impl Lights {
    fn to_lighting(&self) -> Lighting {
        let directional = self.directional.iter().map(|l| Light::Directional {
            direction: l.direction,
            color: rgb(l.color),
        });
        let point = self.point.iter().map(|l| Light::Point {
            position: l.position,
            color: rgb(l.color),
            range: l.range.unwrap_or(f32::INFINITY),
        });
        Lighting {
            ambient: rgb(self.ambient),
            lights: directional.chain(point).collect(),
        }
    }
}

struct Gpu {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// テスト全体で 1 つのデバイスを共有する (破棄しない理由は `tests/gpu.rs` の `gpu()`)。
fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
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
            "ugoe-render spec examples: using adapter {:?}",
            adapter.get_info()
        );
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ugoe-render spec examples"),
            required_limits: limits,
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

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/examples")
}

/// 乗算していない RGBA8 を乗算済みにする (`spec/rendering.md` はテクスチャを乗算済みとして合成する)。
fn premultiply(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let a = f32::from(p[3]) / 255.0;
            let c = |v: u8| (f32::from(v) * a).round() as u8;
            [c(p[0]), c(p[1]), c(p[2]), p[3]]
        })
        .collect()
}

/// キャンバスの px を描画先の px に写す `[a, b, c, d, e, f]` (`spec/examples/README.md#files`) に、
/// 描画先の px から NDC への変換 (Y 下向きから上向きへ) を合成したビュー。
fn target_view(view: [f32; 6], [width, height]: [u32; 2]) -> View {
    let [a, b, c, d, e, f] = view;
    let (sx, sy) = (2.0 / width as f32, -2.0 / height as f32);
    View {
        view_proj: [
            [a * sx, b * sy, 0.0],
            [c * sx, d * sy, 0.0],
            [e * sx - 1.0, f * sy + 1.0, 1.0],
        ],
    }
}

/// `case.view` を省略したときのビュー。キャンバス全体を描画先全体に引き伸ばす。
fn stretch_view(canvas_width: f32, canvas_height: f32, [width, height]: [u32; 2]) -> [f32; 6] {
    [
        width as f32 / canvas_width,
        0.0,
        0.0,
        height as f32 / canvas_height,
        0.0,
        0.0,
    ]
}

#[test]
fn reference_renderer_meets_the_rendering_examples() {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .expect("spec/examples")
        .map(|entry| entry.expect("spec/examples").path())
        .filter(|path| path.join("render.json").is_file())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty());

    let mut failures = Vec::new();
    let mut known = Vec::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let example: RenderExample =
            serde_json::from_str(&std::fs::read_to_string(dir.join("render.json")).unwrap())
                .unwrap_or_else(|e| panic!("{name}/render.json: {e}"));
        let model = model_from_json(&std::fs::read_to_string(dir.join(&example.model)).unwrap())
            .unwrap_or_else(|e| panic!("{name}/{}: {e}", example.model));
        let errors = validate(&model);
        assert!(errors.is_empty(), "{name}: invalid model: {errors:?}");
        assert_eq!(
            model.textures.len(),
            example.textures.len(),
            "{name}: textures"
        );
        let canvas = model.meta.canvas;
        let compiled = CompiledModel::compile(model).unwrap();

        for (i, case) in example.cases.iter().enumerate() {
            let mut instance = ModelInstance::new(compiled.clone());
            for (id, &value) in &case.parameters {
                let index = instance
                    .parameters()
                    .index_of(id)
                    .unwrap_or_else(|| panic!("{name} case {i}: unknown parameter {id}"));
                instance.parameters_mut().set(index, value);
            }
            instance.update();
            let image = render(
                &name,
                &example.textures,
                case,
                &instance,
                target_view(
                    case.view
                        .unwrap_or_else(|| stretch_view(canvas.width, canvas.height, case.target)),
                    case.target,
                ),
            );
            for pixel in &case.pixels {
                let [x, y] = pixel.at;
                let actual = image.at(x, y);
                let ok = actual
                    .iter()
                    .zip(pixel.rgba)
                    .all(|(&a, e)| (f32::from(a) - e).abs() <= TOLERANCE);
                let known_mismatch = KNOWN_MISMATCHES.contains(&(name.as_str(), i, pixel.at));
                let line = format!(
                    "{name} case {i} [{x}, {y}]: expected {:?}, got {actual:?}",
                    pixel.rgba
                );
                match (ok, known_mismatch) {
                    (true, false) => {}
                    (false, false) => failures.push(line),
                    (false, true) => known.push(line),
                    (true, true) => failures.push(format!("{line} (listed as a known mismatch)")),
                }
            }
        }
    }
    for line in &known {
        eprintln!("known mismatch: {line}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

struct Target {
    pixels: Vec<u8>,
    width: u32,
}

impl Target {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().expect("4 bytes")
    }
}

/// `case.target` の大きさの描画先を透明な黒でクリアし、1 フレーム描いて読み戻す。
fn render(
    name: &str,
    textures: &[Image],
    case: &Case,
    instance: &ModelInstance,
    view: View,
) -> Target {
    let Gpu { device, queue, .. } = gpu();
    let mut renderer = Renderer::new(device, FORMAT);
    for (index, texture) in (0u32..).zip(textures) {
        renderer
            .set_texture(
                device,
                queue,
                index,
                texture.width,
                texture.height,
                &texture.premultiplied(&format!("{name}: texture {index}")),
            )
            .expect("texture");
        if let Some(normal) = &texture.normal_map {
            renderer
                .set_normal_map(
                    device,
                    queue,
                    index,
                    normal.width,
                    normal.height,
                    &normal.decoded(&format!("{name}: normal map {index}")),
                )
                .expect("normal map");
        }
    }
    if let Some(lights) = &case.lights {
        renderer.set_lighting(&lights.to_lighting());
    }

    let [width, height] = case.target;
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
    renderer
        .encode(device, queue, &mut encoder, &target_view, &view, instance)
        .expect("encode");

    let row = width * 4;
    let padded_row = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded_row * height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
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
    let mapped = readback.get_mapped_range(..).expect("mapped range");
    let pixels = mapped
        .chunks_exact(padded_row as usize)
        .flat_map(|r| &r[..row as usize])
        .copied()
        .collect();
    Target { pixels, width }
}
