//! M2 の受け入れテスト (T0616) の描画: 通しの手順で作ったモデルを `.ugoe` に書き出し、
//! `ugoe render` で斜めの姿勢を PNG に描く。
//! `cargo test -p ugoe-cli --features gpu-tests` で実行する (`render_gpu.rs` と同じく GPU が必要)。
#![cfg(feature = "gpu-tests")]

#[path = "../../ugoe-editor-core/tests/m2/mod.rs"]
mod m2;

use std::path::{Path, PathBuf};
use std::process::Command;

use ugoe_editor_core::{Project, ProjectMeta, export_bundle};

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

    fn visible_pixels(&self) -> usize {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .count()
    }
}

fn out_dir() -> PathBuf {
    // 目で確かめられるように残す (次の実行で上書きする)。
    let dir = std::env::temp_dir().join("ugoe-m2-render");
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir
}

/// `ugoe render <bundle> --param ... --scale 4 -o <out>` を実行し、PNG のバイト列と画素を返す。
fn render(bundle: &Path, out: &Path, params: &[&str]) -> (Vec<u8>, Png) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ugoe"));
    command.arg("render").arg(bundle).args(["--scale", "4"]);
    for param in params {
        command.args(["--param", param]);
    }
    let output = command.arg("-o").arg(out).output().expect("run ugoe");
    assert!(
        output.status.success(),
        "ugoe render failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(out).expect("read output");
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes.clone()));
    let mut reader = decoder.read_info().expect("png header");
    let mut pixels = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut pixels).expect("png data");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    (
        bytes,
        Png {
            width: info.width,
            height: info.height,
            pixels,
        },
    )
}

#[test]
fn renders_exported_bundle_at_diagonal_pose() {
    let wf = m2::run();
    let project = Project::from_document(ProjectMeta::new("m2"), &wf.doc, None);
    let dir = out_dir();
    let bundle = dir.join("m2.ugoe");
    std::fs::write(&bundle, export_bundle(&project).expect("export")).expect("write bundle");

    let diagonal = ["AngleX=30", "AngleY=30"];
    let (first_bytes, first) = render(&bundle, &dir.join("diagonal.png"), &diagonal);
    let (second_bytes, _) = render(&bundle, &dir.join("diagonal-again.png"), &diagonal);
    let (_, rest) = render(&bundle, &dir.join("rest.png"), &[]);

    // キャンバス 64x48 を 4 倍で描く。
    assert_eq!((first.width, first.height), (256, 192));
    // 決定的: 同じ入力から同じ PNG。
    assert!(first_bytes == second_bytes, "two renders differ");
    // 背景のレイヤーがキャンバスを覆う (模様のアルファが 0 の画素がわずかにある) ので、ほぼ全体が見える。
    let visible = first.visible_pixels();
    assert!(
        visible > 256 * 192 * 9 / 10,
        "only {visible} visible pixels"
    );
    // 背景は動かないので、顔から離れた角は基準の姿と同じ。
    for (x, y) in [(2, 2), (253, 189), (2, 189)] {
        assert_eq!(first.at(x, y), rest.at(x, y), "pixel ({x}, {y})");
    }
    // 顔は振り向くので、基準の姿とは違う画素がある。
    let changed = first
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rest.pixels.as_chunks::<4>().0.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 200, "only {changed} pixels changed");
    println!("wrote {}", dir.display());
}
