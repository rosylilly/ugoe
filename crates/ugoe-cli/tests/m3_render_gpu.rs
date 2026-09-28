//! M3 の受け入れテスト (T0310) の描画: エディタのコマンドで物理とモーションを付けたモデルを `.ugoe` に
//! 書き出し、`ugoe render --motion --time` でモーションの途中の絵を PNG に描く。
//! `cargo test -p ugoe-cli --features gpu-tests` で実行する (`render_gpu.rs` と同じく GPU が必要)。
//! モデルのテクスチャは白一色なので、黒の背景に白い形として描く。
//! 絵は一時ディレクトリの `ugoe-m3-render/` に残るので、目で確かめられる (`docs/testing.md`)。
#![cfg(feature = "gpu-tests")]

#[path = "../../ugoe-editor-core/tests/m3/mod.rs"]
mod m3;

use std::path::{Path, PathBuf};
use std::process::Command;

use ugoe_editor_core::{Project, ProjectMeta, export_bundle};

fn out_dir() -> PathBuf {
    // 目で確かめられるように残す (次の実行で上書きする)。
    let dir = std::env::temp_dir().join("ugoe-m3-render");
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir
}

/// `ugoe render <bundle> --scale 2 --background black <args> -o <out>` を実行し、PNG のバイト列を返す。
fn render(bundle: &Path, out: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_ugoe"))
        .arg("render")
        .arg(bundle)
        .args(["--scale", "2", "--background", "black"])
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

#[test]
fn renders_frames_in_the_middle_of_a_motion() {
    let wf = m3::run();
    let project = Project::from_document(ProjectMeta::new("m3"), &wf.doc);
    let dir = out_dir();
    let bundle = dir.join("m3.ugoe");
    std::fs::write(&bundle, export_bundle(&project).expect("export")).expect("write bundle");

    let frame = |name: &str, args: &[&str]| render(&bundle, &dir.join(name), args);
    let rest = frame("rest.png", &[]);
    let start = frame("shake-0.00.png", &["--motion", m3::SHAKE, "--time", "0"]);
    let quarter = frame("shake-0.25.png", &["--motion", m3::SHAKE, "--time", "0.25"]);
    let again = frame(
        "shake-0.25-again.png",
        &["--motion", m3::SHAKE, "--time", "0.25"],
    );
    let still = frame(
        "shake-0.25-no-physics.png",
        &["--motion", m3::SHAKE, "--time", "0.25", "--no-physics"],
    );
    let tilt = frame("tilt.png", &["--expression", m3::TILT]);

    // 決定的: 同じ時刻からは同じ PNG。
    assert!(quarter == again, "two renders at 0.25s differ");
    // モーションの途中では頭が動き、髪が遅れてついてくる (物理を止めた絵とも違う)。
    assert!(quarter != start, "the motion did not move the model");
    assert!(quarter != still, "physics did not change the frame");
    // 表情 (頭を 10 度傾ける) も絵に出る。
    assert!(tilt != rest, "the expression did not change the frame");
    println!("wrote {}", dir.display());
}
