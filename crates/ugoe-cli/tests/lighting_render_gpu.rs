//! `ugoe render` のライティング (T0210): 法線マップ付きのバンドルを `--ambient`・`--light` で描き、
//! 光源の向きで陰影が変わることと、光源を指定しなければ法線マップが無いのと同じ PNG になることを確かめる。
//! `cargo test -p ugoe-cli --features gpu-tests` で実行する (`render_gpu.rs` と同じく GPU が必要)。
//! 絵は一時ディレクトリの `ugoe-lighting-render/` に残るので、目で確かめられる (`docs/testing.md`)。
#![cfg(feature = "gpu-tests")]

use std::path::{Path, PathBuf};
use std::process::Command;

use ugoe_format::{Bundle, TextureData, TextureFormat};

/// 法線マップの一辺。
const NORMAL_SIZE: u32 = 64;

fn out_dir() -> PathBuf {
    // 目で確かめられるように残す (次の実行で上書きする)。
    let dir = std::env::temp_dir().join("ugoe-lighting-render");
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(rgba).expect("png data");
    writer.finish().expect("png finish");
    out
}

/// 半球のふくらみの法線マップ (G が上向き。円の外は `(0, 0, 1)`)。
fn dome_normal_map() -> Vec<u8> {
    let mut rgba = Vec::new();
    for y in 0..NORMAL_SIZE {
        for x in 0..NORMAL_SIZE {
            let u = (x as f32 + 0.5) / NORMAL_SIZE as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / NORMAL_SIZE as f32 * 2.0 - 1.0;
            let r2 = u * u + v * v;
            // 画像の下向き (v) を、法線の上向き (G) に直す。
            let n = if r2 < 1.0 {
                [u, -v, (1.0 - r2).sqrt()]
            } else {
                [0.0, 0.0, 1.0]
            };
            let c = |n: f32| ((n * 0.5 + 0.5) * 255.0).round() as u8;
            rgba.extend_from_slice(&[c(n[0]), c(n[1]), c(n[2]), 255]);
        }
    }
    rgba
}

/// single_quad (白い四角、キャンバス 256x256 の 64..192) のバンドル。`lit` ならライティングを受け、法線マップを持つ。
fn write_quad_bundle(path: &Path, lit: bool) {
    let mut model = ugoe_model::fixtures::single_quad();
    let mut bundle_normal_maps = Vec::new();
    if lit {
        model.lighting = true;
        model.meshes[0].lit = true;
        model.textures[0].normal_map = Some("dome".into());
        bundle_normal_maps.push(TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(NORMAL_SIZE, NORMAL_SIZE, &dome_normal_map()),
        });
    }
    let mut bundle = Bundle::new(model);
    bundle.textures = vec![TextureData {
        format: TextureFormat::Png,
        bytes: encode_png(1, 1, &[255; 4]),
    }];
    bundle.normal_maps = bundle_normal_maps;
    let bytes = ugoe_format::write_bundle(&bundle).expect("write bundle");
    std::fs::write(path, bytes).expect("write bundle file");
}

struct Png {
    width: u32,
    pixels: Vec<u8>,
}

impl Png {
    fn read(bytes: &[u8]) -> Self {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("png header");
        let mut pixels = vec![0; reader.output_buffer_size().expect("buffer size")];
        let info = reader.next_frame(&mut pixels).expect("png data");
        assert_eq!(info.color_type, png::ColorType::Rgba);
        Self {
            width: info.width,
            pixels,
        }
    }

    /// 画素の明るさ (R。テクスチャも光も白なので RGB は同じ)。
    fn luma(&self, (x, y): (u32, u32)) -> u8 {
        self.pixels[((y * self.width + x) * 4) as usize]
    }
}

/// `ugoe render <bundle> --background black <args> -o <out>` を実行し、PNG のバイト列を返す。
fn render(bundle: &Path, out: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_ugoe"))
        .arg("render")
        .arg(bundle)
        .args(["--background", "black"])
        .args(args)
        .arg("-o")
        .arg(out)
        .output()
        .expect("run ugoe");
    assert!(
        output.status.success(),
        "ugoe render failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read(out).expect("read output")
}

// ふくらみの左・右・上・下 (中心 (128, 128) から半径の半分)。
const LEFT: (u32, u32) = (96, 128);
const RIGHT: (u32, u32) = (160, 128);
const TOP: (u32, u32) = (128, 96);
const BOTTOM: (u32, u32) = (128, 160);

#[test]
fn light_direction_changes_the_shading() {
    let dir = out_dir();
    let bundle = dir.join("dome.ugoe");
    write_quad_bundle(&bundle, true);
    let frame = |name: &str, args: &[&str]| Png::read(&render(&bundle, &dir.join(name), args));

    // 左の手前から照らす (光は右奥へ進む): 左側が明るい。
    let from_left = frame(
        "from-left.png",
        &["--ambient", "0.1", "--light", "directional:1,0,-1"],
    );
    assert!(
        from_left.luma(LEFT) > from_left.luma(RIGHT) + 60,
        "left {} / right {}",
        from_left.luma(LEFT),
        from_left.luma(RIGHT)
    );
    // 右から照らすと逆になる。
    let from_right = frame(
        "from-right.png",
        &["--ambient", "0.1", "--light", "directional:-1,0,-1"],
    );
    assert!(
        from_right.luma(RIGHT) > from_right.luma(LEFT) + 60,
        "left {} / right {}",
        from_right.luma(LEFT),
        from_right.luma(RIGHT)
    );
    // 上から照らす (光は下へ進む。キャンバスは Y 下向き): 上側が明るい (法線マップの G は上向き)。
    let from_top = frame(
        "from-top.png",
        &["--ambient", "0.1", "--light", "directional:0,1,-1"],
    );
    assert!(
        from_top.luma(TOP) > from_top.luma(BOTTOM) + 60,
        "top {} / bottom {}",
        from_top.luma(TOP),
        from_top.luma(BOTTOM)
    );
    // 左端の近くに置いた点光源: 左側が明るい。
    let point = frame(
        "point-left.png",
        &["--ambient", "0.1", "--light", "point:64,128,48:1.2:400"],
    );
    assert!(
        point.luma(LEFT) > point.luma(RIGHT) + 30,
        "left {} / right {}",
        point.luma(LEFT),
        point.luma(RIGHT)
    );
}

#[test]
fn without_lights_the_normal_map_changes_nothing() {
    let dir = out_dir();
    let lit = dir.join("dome-for-compare.ugoe");
    let plain = dir.join("plain.ugoe");
    write_quad_bundle(&lit, true);
    write_quad_bundle(&plain, false);

    let reference = render(&plain, &dir.join("plain.png"), &[]);
    // 光源を指定しなければ、法線マップとライティングの指定があっても 1 画素も変わらない。
    let unlit = render(&lit, &dir.join("dome-no-lights.png"), &[]);
    assert!(
        unlit == reference,
        "the normal map changed the unlit render"
    );
    // 環境光 1 だけ (既定と同じ光) でも変わらない。
    let ambient = render(&lit, &dir.join("dome-ambient-1.png"), &["--ambient", "1"]);
    assert!(ambient == reference, "ambient 1 changed the render");
    // ライティングを受けないモデルは、光源を指定しても変わらない。
    let plain_lit = render(
        &plain,
        &dir.join("plain-with-light.png"),
        &["--ambient", "0.1", "--light", "directional:1,0,-1"],
    );
    assert!(plain_lit == reference, "lights changed an unlit model");
}
