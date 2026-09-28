//! `ugoe render` を実際に実行して PNG を確かめるテスト。
//! `cargo test -p ugoe-cli --features gpu-tests` で実行する。
//!
//! GPU (またはソフトウェアラスタライザの lavapipe など) が見つからない場合は失敗する。
//! 別プロセスで実行するので、終了時のドライバのクラッシュ (WSL2 の dzn) も終了コードで検出できる。
#![cfg(feature = "gpu-tests")]

use std::path::PathBuf;
use std::process::Command;

/// 8bit 量子化とラスタライザの差を吸収する許容誤差 (0..255)。
const TOLERANCE: i32 = 2;

struct Png {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Png {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().expect("4 bytes")
    }

    /// `expected` はストレートアルファの RGBA8。
    #[track_caller]
    fn assert_pixel(&self, (x, y): (u32, u32), expected: [u8; 4]) {
        let actual = self.at(x, y);
        let ok = actual
            .iter()
            .zip(expected)
            .all(|(&a, e)| (i32::from(a) - i32::from(e)).abs() <= TOLERANCE);
        assert!(
            ok,
            "pixel ({x}, {y}): expected {expected:?}, got {actual:?}"
        );
    }
}

fn out_path(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ugoe-cli-render");
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir.join(name)
}

/// `ugoe render <args> -o <name>` を実行して出力を読む。
fn render(name: &str, args: &[&str]) -> Png {
    let out = out_path(name);
    let output = Command::new(env!("CARGO_BIN_EXE_ugoe"))
        .arg("render")
        .args(args)
        .arg("-o")
        .arg(&out)
        .output()
        .expect("run ugoe");
    assert!(
        output.status.success(),
        "ugoe render failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&out).expect("open output"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut pixels = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut pixels).expect("png data");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    Png {
        width: info.width,
        height: info.height,
        pixels,
    }
}

const WHITE: [u8; 4] = [255; 4];
const CLEAR: [u8; 4] = [0; 4];

#[test]
fn renders_single_quad_on_transparent_background() {
    // キャンバス 256 を 64 に縮小: 四角形は 16..48。
    let img = render(
        "single_quad.png",
        &["fixture:single_quad", "--size", "64x64"],
    );
    assert_eq!((img.width, img.height), (64, 64));
    img.assert_pixel((32, 32), WHITE);
    img.assert_pixel((17, 17), WHITE);
    img.assert_pixel((4, 4), CLEAR);
    img.assert_pixel((50, 32), CLEAR);
}

#[test]
fn applies_params_and_background() {
    // X = 1 で四角形は 64px (出力では 16px) 右へ。
    let img = render(
        "single_quad_x1.png",
        &[
            "fixture:single_quad",
            "--width",
            "64",
            "--param",
            "X=1",
            "--background",
            "#ff0000",
        ],
    );
    assert_eq!((img.width, img.height), (64, 64));
    img.assert_pixel((24, 32), [255, 0, 0, 255]);
    img.assert_pixel((40, 32), WHITE);
    img.assert_pixel((60, 32), WHITE);
}

#[test]
fn contain_fit_letterboxes_and_keeps_y_down() {
    // 128x64 に収める: 縮尺 0.25、左右に 32px の余白。四角形は x 48..80、y 16..48。
    // テクスチャは上半分が赤、下半分が青なので、上下が反転していないことも分かる。
    let texture = out_path("red_over_blue.png");
    write_png(&texture, 1, 2, &[255, 0, 0, 255, 0, 0, 255, 255]);
    let img = render(
        "letterbox.png",
        &[
            "fixture:single_quad",
            "--size",
            "128x64",
            "--texture",
            &format!("0={}", texture.display()),
        ],
    );
    assert_eq!((img.width, img.height), (128, 64));
    img.assert_pixel((40, 32), CLEAR);
    img.assert_pixel((64, 18), [255, 0, 0, 255]);
    img.assert_pixel((64, 46), [0, 0, 255, 255]);
    img.assert_pixel((88, 32), CLEAR);
}

#[test]
fn writes_straight_alpha() {
    // 半透明の背景は乗算済みから戻して書き出す。
    let img = render(
        "half_alpha.png",
        &[
            "fixture:single_quad",
            "--size",
            "8x8",
            "--background",
            "#ff000080",
        ],
    );
    img.assert_pixel((0, 0), [255, 0, 0, 128]);
}

fn write_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(rgba).expect("png data");
    writer.finish().expect("finish png");
}
