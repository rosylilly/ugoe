//! `ugoe render`: ヘッドレスの wgpu でモデルを描画して PNG に書き出す。
//!
//! - 描画先は `Rgba8Unorm`・乗算済みアルファ。PNG に書くときにストレートアルファへ戻す。
//! - キャンバス座標 (Y 下向き) をクリップ座標 (Y 上向き) へ写すビューを作る (`docs/spec/rendering.md`)。

use std::path::PathBuf;

use anyhow::{Context, bail};
use ugoe_model::Model;
use ugoe_render::{Renderer, View};
use ugoe_runtime::ModelInstance;

use crate::image::Rgba8;

/// 描画先の形式。レンダラの前提 (リニア化しない) に合わせて sRGB 形式にしない。
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// 出力の一辺の上限 (デバイスの上限がこれより小さければそちらが優先される)。
pub const MAX_SIZE: u32 = 16384;

/// キャンバスを出力画像にどう収めるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Fit {
    /// 縦横比を保って全体が収まるように拡大縮小し、中央に置く。
    #[default]
    Contain,
    /// 縦横比を無視して出力全体に引き伸ばす。
    Stretch,
}

/// `ID=VALUE` を読む。
pub fn parse_param(s: &str) -> Result<(String, f32), String> {
    let (id, value) = s
        .split_once('=')
        .ok_or_else(|| format!("expected ID=VALUE, got `{s}`"))?;
    let id = id.trim();
    if id.is_empty() {
        return Err(format!("empty parameter ID in `{s}`"));
    }
    let value: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{value}` in `{s}`"))?;
    if !value.is_finite() {
        return Err(format!("parameter value must be finite, got `{value}`"));
    }
    Ok((id.to_owned(), value))
}

/// `WxH` を読む。
pub fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("expected WxH, got `{s}`"))?;
    let parse = |v: &str| -> Result<u32, String> {
        let n: u32 = v
            .trim()
            .parse()
            .map_err(|_| format!("invalid size `{s}`"))?;
        check_dimension(n)?;
        Ok(n)
    };
    Ok((parse(w)?, parse(h)?))
}

/// `--width` / `--height` の値。
pub fn parse_dimension(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| format!("invalid size `{s}`"))?;
    check_dimension(n)?;
    Ok(n)
}

fn check_dimension(n: u32) -> Result<(), String> {
    if n == 0 || n > MAX_SIZE {
        return Err(format!("size must be in 1..={MAX_SIZE}, got {n}"));
    }
    Ok(())
}

/// `transparent` / `white` / `black` / `#RRGGBB` / `#RRGGBBAA` (`#` は省略可) を、ストレートアルファの RGBA8 に読む。
pub fn parse_color(s: &str) -> Result<[u8; 4], String> {
    for (name, color) in [
        ("transparent", [0; 4]),
        ("white", [255; 4]),
        ("black", [0, 0, 0, 255]),
    ] {
        if s.eq_ignore_ascii_case(name) {
            return Ok(color);
        }
    }
    let hex = s.strip_prefix('#').unwrap_or(s);
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "expected transparent, white, black, #RRGGBB or #RRGGBBAA, got `{s}`"
        ));
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string());
    let a = if hex.len() == 8 { byte(6)? } else { 255 };
    Ok([byte(0)?, byte(2)?, byte(4)?, a])
}

/// `INDEX=PATH` を読む (`--texture`)。
pub fn parse_texture(s: &str) -> Result<(u32, PathBuf), String> {
    let (index, path) = s
        .split_once('=')
        .ok_or_else(|| format!("expected INDEX=PATH, got `{s}`"))?;
    let index = index
        .trim()
        .parse()
        .map_err(|_| format!("invalid texture index `{index}` in `{s}`"))?;
    if path.is_empty() {
        return Err(format!("empty path in `{s}`"));
    }
    Ok((index, PathBuf::from(path)))
}

/// 出力サイズを決める。
///
/// `size` があればそれ。`width` / `height` の片方だけならキャンバスの縦横比から他方を決める。
/// どちらも無ければキャンバスの大きさ × `scale`。
pub fn output_size(
    canvas: (f32, f32),
    size: Option<(u32, u32)>,
    width: Option<u32>,
    height: Option<u32>,
    scale: f32,
) -> anyhow::Result<(u32, u32)> {
    let (cw, ch) = canvas;
    if !(cw > 0.0 && ch > 0.0) {
        bail!("model canvas has no area ({cw}x{ch})");
    }
    let dim = |v: f32| -> anyhow::Result<u32> {
        let n = v.round();
        if !(1.0..=MAX_SIZE as f32).contains(&n) {
            bail!("output size {v} is out of range 1..={MAX_SIZE}");
        }
        Ok(n as u32)
    };
    Ok(match (size, width, height) {
        (Some(s), _, _) => s,
        (None, Some(w), Some(h)) => (w, h),
        (None, Some(w), None) => (w, dim(w as f32 * ch / cw)?),
        (None, None, Some(h)) => (dim(h as f32 * cw / ch)?, h),
        (None, None, None) => {
            if !(scale > 0.0 && scale.is_finite()) {
                bail!("--scale must be positive, got {scale}");
            }
            (dim(cw * scale)?, dim(ch * scale)?)
        }
    })
}

/// キャンバス (Y 下向き) を出力画像に収めるビュー。クリップ座標は Y 上向き。
pub fn fit_view(canvas: (f32, f32), output: (u32, u32), fit: Fit) -> View {
    let (cw, ch) = canvas;
    let (w, h) = (output.0 as f32, output.1 as f32);
    // キャンバス座標 → 出力画素座標 (px = ox + sx * x, py = oy + sy * y)。
    let (sx, sy, ox, oy) = match fit {
        Fit::Contain => {
            let s = (w / cw).min(h / ch);
            (s, s, (w - cw * s) / 2.0, (h - ch * s) / 2.0)
        }
        Fit::Stretch => (w / cw, h / ch, 0.0, 0.0),
    };
    // 画素座標 → クリップ座標 (x: 0..w → -1..1、y: 0..h → 1..-1)。
    View {
        view_proj: [
            [2.0 * sx / w, 0.0, 0.0],
            [0.0, -2.0 * sy / h, 0.0],
            [2.0 * ox / w - 1.0, 1.0 - 2.0 * oy / h, 1.0],
        ],
    }
}

/// `--param` をインスタンスに設定し、`update` する。
///
/// 値は値域にクランプされる (`ParameterValues::set`)。クランプされたものは警告文を返す。
pub fn apply_params(
    instance: &mut ModelInstance,
    params: &[(String, f32)],
) -> anyhow::Result<Vec<String>> {
    let mut warnings = Vec::new();
    for (id, value) in params {
        let parameters = instance.parameters_mut();
        let Some(index) = parameters.index_of(id) else {
            let known: Vec<&str> = instance
                .compiled()
                .model()
                .parameters
                .iter()
                .map(|p| p.id.as_str())
                .collect();
            let known = if known.is_empty() {
                "none".to_owned()
            } else {
                known.join(", ")
            };
            bail!("unknown parameter `{id}` (known: {known})");
        };
        parameters.set(index, *value);
        let actual = parameters.get(index);
        if actual != *value {
            let p = &instance.compiled().model().parameters[index.0 as usize];
            warnings.push(format!(
                "parameter {id}={value} clamped to {actual} (range {}..{})",
                p.min, p.max
            ));
        }
    }
    instance.update();
    Ok(warnings)
}

/// 描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
pub fn texture_count(model: &Model) -> usize {
    model
        .meshes
        .iter()
        .map(|m| m.texture as usize + 1)
        .max()
        .unwrap_or(0)
        .max(model.textures.len())
}

/// テクスチャが無いときに使う 1x1 の不透明な白。
pub fn placeholder_texture() -> Rgba8 {
    Rgba8 {
        width: 1,
        height: 1,
        pixels: vec![255; 4],
    }
}

/// ヘッドレスの GPU。
#[derive(Debug)]
pub struct Gpu {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    /// サーフェス無しでアダプタとデバイスを作る。
    ///
    /// `backend` (例: `vulkan`、`gl`、`vulkan,gl`) を省略すると環境変数 `WGPU_BACKEND` に従い、
    /// それも無ければ wgpu の既定。`WGPU_POWER_PREF` も参照する。
    pub fn new(backend: Option<&str>) -> anyhow::Result<Self> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        if let Some(backend) = backend {
            desc.backends = wgpu::Backends::from_comma_list(backend);
            if desc.backends.is_empty() {
                bail!(
                    "unknown backend `{backend}` (expected vulkan, metal, dx12 or gl; \
                     comma-separated for several)"
                );
            }
        }
        let instance = wgpu::Instance::new(desc);
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::from_env().unwrap_or_default(),
                ..Default::default()
            })) {
                Ok(adapter) => adapter,
                Err(e) => {
                    // 破棄しない理由は [`Gpu::leak`] を参照。
                    std::mem::forget(instance);
                    bail!(
                        "no GPU adapter available ({e}). Rendering needs a GPU or a software \
                     rasterizer such as Mesa lavapipe (Vulkan) or llvmpipe (GL); try \
                     --backend gl or WGPU_BACKEND=vulkan"
                    );
                }
            };
        let adapter_info = adapter.get_info();
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("ugoe render"),
                required_limits: limits,
                ..Default::default()
            })) {
                Ok(pair) => pair,
                Err(e) => {
                    std::mem::forget((instance, adapter));
                    return Err(e).context("requesting GPU device");
                }
            };
        Ok(Self {
            _instance: instance,
            device,
            queue,
            adapter_info,
        })
    }

    /// インスタンスとデバイスを破棄せずに手放す (プロセスの終了でまとめて解放される)。
    ///
    /// WSL2 の dzn (Vulkan on D3D12) ドライバは、wgpu のインスタンスやデバイスの破棄時に
    /// SIGSEGV を起こすことがある。CLI は描画のあとすぐ終了するので、破棄しないほうが安全。
    pub fn leak(self) {
        std::mem::forget(self);
    }

    /// `instance` を `size` の画像に描いて読み戻す。結果は乗算済みアルファ。
    ///
    /// `background` はストレートアルファの RGBA8。乗算済みに直してからクリアする。
    pub fn render(
        &self,
        instance: &ModelInstance,
        textures: &[Rgba8],
        size: (u32, u32),
        view: &View,
        background: [u8; 4],
    ) -> anyhow::Result<Rgba8> {
        let (device, queue) = (&self.device, &self.queue);
        let (width, height) = size;
        let max = device.limits().max_texture_dimension_2d;
        if width > max || height > max {
            bail!("output size {width}x{height} exceeds the device limit {max}");
        }
        for (i, t) in textures.iter().enumerate() {
            if t.width > max || t.height > max {
                bail!(
                    "texture {i} is {}x{}, which exceeds the device limit {max}",
                    t.width,
                    t.height
                );
            }
        }

        let mut renderer = Renderer::new(device, FORMAT);
        for (i, t) in textures.iter().enumerate() {
            renderer.set_texture(device, queue, i as u32, t.width, t.height, &t.pixels)?;
        }

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ugoe render target"),
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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ugoe render"),
        });
        // クリアはホストの責務 (レンダラは LoadOp::Load で重ねる)。
        let a = f64::from(background[3]) / 255.0;
        let premul = |c: u8| f64::from(c) / 255.0 * a;
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: premul(background[0]),
                        g: premul(background[1]),
                        b: premul(background[2]),
                        a,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.encode(device, queue, &mut encoder, &target_view, view, instance)?;

        // 読み戻しバッファの行は COPY_BYTES_PER_ROW_ALIGNMENT の倍数に揃える必要がある。
        let row_bytes = width * 4;
        let padded_row_bytes = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe render readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);

        let (tx, rx) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("waiting for the GPU")?;
        rx.recv()
            .context("readback callback was not called")?
            .context("mapping the readback buffer")?;
        let mapped = readback
            .get_mapped_range(..)
            .context("reading the readback buffer")?;
        let mut pixels = Vec::with_capacity(row_bytes as usize * height as usize);
        for row in mapped.chunks_exact(padded_row_bytes as usize) {
            pixels.extend_from_slice(&row[..row_bytes as usize]);
        }
        Ok(Rgba8 {
            width,
            height,
            pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use ugoe_runtime::CompiledModel;

    use super::*;

    #[test]
    fn parses_params() {
        assert_eq!(parse_param("X=0.5"), Ok(("X".into(), 0.5)));
        assert_eq!(parse_param("AngleX=-30"), Ok(("AngleX".into(), -30.0)));
        assert!(parse_param("X").is_err());
        assert!(parse_param("=1").is_err());
        assert!(parse_param("X=abc").is_err());
        assert!(parse_param("X=NaN").is_err());
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("640x480"), Ok((640, 480)));
        assert_eq!(parse_size("1X2"), Ok((1, 2)));
        assert!(parse_size("640").is_err());
        assert!(parse_size("0x10").is_err());
        assert!(parse_size("99999x10").is_err());
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("transparent"), Ok([0, 0, 0, 0]));
        assert_eq!(parse_color("White"), Ok([255; 4]));
        assert_eq!(parse_color("#ff8000"), Ok([255, 128, 0, 255]));
        assert_eq!(parse_color("FF800080"), Ok([255, 128, 0, 128]));
        assert!(parse_color("#fff").is_err());
        assert!(parse_color("#gg0000").is_err());
    }

    #[test]
    fn parses_texture_overrides() {
        assert_eq!(parse_texture("1=a.png"), Ok((1, PathBuf::from("a.png"))));
        assert!(parse_texture("a.png").is_err());
        assert!(parse_texture("x=a.png").is_err());
        assert!(parse_texture("0=").is_err());
    }

    #[test]
    fn output_size_rules() {
        let c = (256.0, 128.0);
        assert_eq!(output_size(c, None, None, None, 1.0).unwrap(), (256, 128));
        assert_eq!(output_size(c, None, None, None, 0.5).unwrap(), (128, 64));
        assert_eq!(output_size(c, None, Some(64), None, 1.0).unwrap(), (64, 32));
        assert_eq!(
            output_size(c, None, None, Some(64), 1.0).unwrap(),
            (128, 64)
        );
        assert_eq!(
            output_size(c, None, Some(10), Some(20), 1.0).unwrap(),
            (10, 20)
        );
        assert_eq!(
            output_size(c, Some((5, 6)), None, None, 1.0).unwrap(),
            (5, 6)
        );
        assert!(output_size(c, None, None, None, 0.0).is_err());
        assert!(output_size(c, None, None, None, 1000.0).is_err());
        assert!(output_size((0.0, 1.0), None, None, None, 1.0).is_err());
    }

    fn apply(view: &View, (x, y): (f32, f32)) -> (f32, f32) {
        let m = view.view_proj;
        (
            m[0][0] * x + m[1][0] * y + m[2][0],
            m[0][1] * x + m[1][1] * y + m[2][1],
        )
    }

    #[test]
    fn contain_fit_centers_canvas() {
        // 256x128 のキャンバスを 100x100 に: 縮尺 100/256、上下に余白。
        let view = fit_view((256.0, 128.0), (100, 100), Fit::Contain);
        let (x0, y0) = apply(&view, (0.0, 0.0));
        let (x1, y1) = apply(&view, (256.0, 128.0));
        assert!((x0 + 1.0).abs() < 1e-6 && (x1 - 1.0).abs() < 1e-6);
        assert!((y0 - 0.5).abs() < 1e-6, "{y0}");
        assert!((y1 + 0.5).abs() < 1e-6, "{y1}");
    }

    #[test]
    fn stretch_fit_fills_output() {
        let view = fit_view((256.0, 128.0), (100, 100), Fit::Stretch);
        assert_eq!(apply(&view, (0.0, 0.0)), (-1.0, 1.0));
        assert_eq!(apply(&view, (256.0, 128.0)), (1.0, -1.0));
    }

    fn single_quad_instance() -> ModelInstance {
        let compiled = CompiledModel::compile(ugoe_model::fixtures::single_quad()).unwrap();
        ModelInstance::new(compiled)
    }

    #[test]
    fn applies_and_clamps_params() {
        let mut instance = single_quad_instance();
        let warnings = apply_params(&mut instance, &[("X".into(), 0.5)]).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(instance.parameters().as_slice(), &[0.5]);
        // 四角形は 32px 右へ動く。
        assert_eq!(instance.drawables()[0].positions[0].x, 96.0);

        let warnings = apply_params(&mut instance, &[("X".into(), 5.0)]).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("clamped to 1"), "{}", warnings[0]);
        assert_eq!(instance.parameters().as_slice(), &[1.0]);
    }

    #[test]
    fn unknown_param_lists_known_ids() {
        let mut instance = single_quad_instance();
        let err = apply_params(&mut instance, &[("Nope".into(), 0.0)])
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown parameter `Nope`"), "{err}");
        assert!(err.contains("known: X"), "{err}");
    }

    #[test]
    fn counts_textures_from_meshes_and_model() {
        let mut model = ugoe_model::fixtures::single_quad();
        assert_eq!(texture_count(&model), 1);
        model.textures.clear();
        assert_eq!(texture_count(&model), 1);
        model.meshes[0].texture = 2;
        assert_eq!(texture_count(&model), 3);
    }
}
