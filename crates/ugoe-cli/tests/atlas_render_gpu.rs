//! T0114 の受け入れテスト: `ugoe build --atlas` でテクスチャをアトラスにまとめても、`ugoe render` の絵が
//! 変わらないこと。見本のアバター (`assets/samples/avatar.ugoe`、レイヤーごとに 24 枚のテクスチャ) を使う。
//! `cargo test -p ugoe-cli --features gpu-tests` で実行する (`render_gpu.rs` と同じく GPU が必要)。
//! 絵は `CARGO_TARGET_TMPDIR/ugoe-cli-atlas/` に残るので、目で確かめられる。
#![cfg(feature = "gpu-tests")]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 乗算済みにした 1 チャンネルで、これより大きく違う値は「違う」と数える (8bit 量子化とラスタライザの差)。
/// 出力の PNG はストレートアルファなので、アルファの小さい画素では小さな差が大きく見える。そのため乗算済みで比べる。
const TOLERANCE: u8 = 2;
/// 「違う」値の割合の上限。UV の丸めでバイリニアの重みがわずかにずれ、色の変わり目の数画素だけが
/// 許容誤差を超えることがある。
const MAX_DIFFERENT_RATIO: f64 = 1e-4;
/// どの値もこれより大きくは違わない。
const MAX_DIFFERENCE: u8 = 16;

fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ugoe-cli-atlas");
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir
}

fn avatar() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/samples/avatar.ugoe")
}

fn ugoe(args: &[&OsStr]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ugoe"))
        .args(args)
        .output()
        .expect("run ugoe");
    assert!(
        output.status.success(),
        "ugoe failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

/// `ugoe render <bundle> <args> -o <out>` を実行し、RGBA8 の画素を返す。
fn render(bundle: &Path, out: &Path, args: &[&str]) -> (u32, u32, Vec<u8>) {
    let mut all = vec![OsStr::new("render"), bundle.as_os_str()];
    all.extend(args.iter().map(OsStr::new));
    all.extend([OsStr::new("-o"), out.as_os_str()]);
    ugoe(&all);
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(out).expect("open output"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut pixels = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut pixels).expect("png data");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    (info.width, info.height, pixels)
}

/// `original` と `atlased` を同じ引数で描き、乗算済みの値がほぼ同じであること。
fn assert_same_image(name: &str, original: &Path, atlased: &Path, args: &[&str]) {
    let dir = out_dir();
    let original = render(original, &dir.join(format!("{name}-layers.png")), args);
    let packed = render(atlased, &dir.join(format!("{name}-atlas.png")), args);
    assert_close(name, &original, &packed);
}

/// 2 枚の絵の乗算済みの値がほぼ同じであること。
fn assert_close(name: &str, original: &(u32, u32, Vec<u8>), packed: &(u32, u32, Vec<u8>)) {
    assert_eq!((original.0, original.1), (packed.0, packed.1));
    let mut different = 0usize;
    let mut max = 0u8;
    for (a, b) in original
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .zip(packed.2.as_chunks::<4>().0)
    {
        for (x, y) in premultiply(a).into_iter().zip(premultiply(b)) {
            let d = x.abs_diff(y);
            max = max.max(d);
            different += usize::from(d > TOLERANCE);
        }
    }
    let ratio = different as f64 / original.2.len() as f64;
    assert!(
        ratio <= MAX_DIFFERENT_RATIO && max <= MAX_DIFFERENCE,
        "{name}: {different} value(s) differ by more than {TOLERANCE} (max {max})"
    );
}

/// ストレートアルファの RGBA8 の画素を乗算済みにする。
fn premultiply(p: &[u8]) -> [u8; 4] {
    let a = u16::from(p[3]);
    let m = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
    [m(p[0]), m(p[1]), m(p[2]), p[3]]
}

#[test]
fn atlas_does_not_change_the_rendered_image() {
    let dir = out_dir();
    let atlased = dir.join("avatar-atlas.ugoe");
    let summary = ugoe(&[
        OsStr::new("build"),
        avatar().as_os_str(),
        OsStr::new("--atlas"),
        OsStr::new("-o"),
        atlased.as_os_str(),
    ]);
    assert!(
        summary.contains("packed 24 texture(s) into 1 atlas(es)"),
        "{summary}"
    );

    let poses: [(&str, &[&str]); 3] = [
        ("rest", &[]),
        (
            "turned",
            &[
                "--param",
                "AngleX=25",
                "--param",
                "AngleY=-15",
                "--param",
                "EyeLOpen=0.3",
            ],
        ),
        (
            "mouth",
            &["--param", "MouthOpenY=1", "--param", "BodyAngleZ=8"],
        ),
    ];
    for (name, args) in poses {
        assert_same_image(name, &avatar(), &atlased, args);
    }
}

/// 不透明で隣り合う画素の差が大きい小さな画像を、4 つの四角形に拡大して描く。見本のアバターは
/// 画像の周りが透明なので、端の複製 (padding) が足りなくても絵に出にくい。こちらは端の誤りがそのまま出る。
#[test]
fn atlas_keeps_opaque_texture_edges() {
    use ugoe_core::Vec2;
    use ugoe_editor_core::texture::encode_rgba_png;
    use ugoe_editor_core::{Project, ProjectMeta, Textures, export_bundle};
    use ugoe_model::TextureRef;

    let mut model = ugoe_model::fixtures::single_quad();
    let base = model.meshes[0].clone();
    let mut textures = Textures::new();
    model.meshes.clear();
    model.textures.clear();
    let sizes = [(5, 4), (3, 7), (6, 6), (4, 3)];
    for (i, &(width, height)) in sizes.iter().enumerate() {
        let mut state = 0x9e37_79b9_u32.wrapping_mul(i as u32 + 1);
        let pixels: Vec<u8> = (0..width * height)
            .flat_map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let [r, g, b, _] = state.to_le_bytes();
                [r, g, b, 255]
            })
            .collect();
        let name = format!("noise{i}");
        textures.insert(name.clone(), encode_rgba_png(width, height, &pixels).into());
        model.textures.push(TextureRef {
            name,
            width,
            height,
            normal_map: None,
        });

        // 128 四方の四角形 (64..192) を 96 四方にして、2x2 に並べる。
        let offset = Vec2::new(8.0 + 124.0 * (i % 2) as f32, 8.0 + 124.0 * (i / 2) as f32);
        let mut mesh = base.clone();
        mesh.id = format!("Quad{i}").as_str().into();
        mesh.name = format!("Quad{i}");
        mesh.texture = i as u32;
        for keyform in &mut mesh.keyforms.keyforms {
            for p in &mut keyform.positions {
                *p = (*p - Vec2::new(64.0, 64.0)) * 0.75 + offset;
            }
        }
        model.meshes.push(mesh);
    }

    let dir = out_dir();
    let original = dir.join("noise.ugoe");
    let project = Project::new(ProjectMeta::new("noise"), model, textures);
    std::fs::write(&original, export_bundle(&project).expect("export")).expect("write bundle");
    let atlased = dir.join("noise-atlas.ugoe");
    ugoe(&[
        OsStr::new("build"),
        original.as_os_str(),
        OsStr::new("--atlas"),
        OsStr::new("--atlas-padding"),
        OsStr::new("1"),
        OsStr::new("-o"),
        atlased.as_os_str(),
    ]);
    assert_same_image("noise", &original, &atlased, &["--scale", "4"]);

    // 4 枚のテクスチャを切り替えていた描画 (切り替え 3 回・4 回の描画) が、1 回の描画になる。
    let before = frame_stats(&original, &dir.join("noise-stats-layers.png"));
    let after = frame_stats(&atlased, &dir.join("noise-stats-atlas.png"));
    let pair = |v: &serde_json::Value| (v["state_changes"].as_u64(), v["draw_calls"].as_u64());
    assert_eq!(pair(&before), (Some(3), Some(4)), "{before}");
    assert_eq!(pair(&after), (Some(0), Some(1)), "{after}");
}

/// `ugoe render <bundle> --stats` の描画の統計 (`ugoe_render::FrameStats`)。
fn frame_stats(bundle: &Path, out: &Path) -> serde_json::Value {
    let stdout = ugoe(&[
        OsStr::new("render"),
        bundle.as_os_str(),
        OsStr::new("--stats"),
        OsStr::new("-o"),
        out.as_os_str(),
    ]);
    let line = stdout
        .lines()
        .find_map(|l| l.strip_prefix("stats: "))
        .unwrap_or_else(|| panic!("no stats in {stdout}"));
    serde_json::from_str(line).expect("stats json")
}

/// 見本のアバターでも、アトラス化で本描画の状態の切り替え (テクスチャの束縛し直し) と描画コマンドが減る (ADR 0016)。
#[test]
fn atlas_reduces_state_changes() {
    let dir = out_dir();
    let atlased = dir.join("avatar-atlas-stats.ugoe");
    ugoe(&[
        OsStr::new("build"),
        avatar().as_os_str(),
        OsStr::new("--atlas"),
        OsStr::new("-o"),
        atlased.as_os_str(),
    ]);
    let before = frame_stats(&avatar(), &dir.join("stats-layers.png"));
    let after = frame_stats(&atlased, &dir.join("stats-atlas.png"));
    let get = |v: &serde_json::Value, key: &str| v[key].as_u64().expect(key);
    // 描く要素は同じ。
    assert_eq!(get(&before, "drawables"), get(&after, "drawables"));
    // 見本は T0704 で作り直すので、数は固定せずに減ることだけを確かめる (固定の数は下の noise で見る)。
    for key in ["state_changes", "draw_calls"] {
        assert!(
            get(&after, key) < get(&before, key),
            "{key}: {before} -> {after}"
        );
    }
}

/// 着せ替えのモジュールのアトラス化 (T0132): 見本のモジュールを `ugoe build --atlas` でまとめ、本体に付けて描いた絵が
/// 変わらず、テクスチャの切り替えが減ること。
#[test]
fn module_atlas_does_not_change_the_rendered_image() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/samples/dress-up");
    let base = samples.join("dress-up.ugoe");
    let dir = out_dir();
    let names = ["long-hair", "sailor", "cat-ears"];
    let mut original_args = Vec::new();
    let mut atlased_args = Vec::new();
    for name in names {
        let module = samples.join(format!("{name}.ugoemod"));
        let atlased = dir.join(format!("{name}-atlas.ugoemod"));
        let summary = ugoe(&[
            OsStr::new("build"),
            module.as_os_str(),
            OsStr::new("--atlas"),
            OsStr::new("-o"),
            atlased.as_os_str(),
        ]);
        assert!(summary.contains("into 1 atlas(es)"), "{summary}");
        for (args, path) in [(&mut original_args, module), (&mut atlased_args, atlased)] {
            args.push("--module".to_owned());
            args.push(path.to_str().expect("utf-8 path").to_owned());
        }
    }
    let poses: [(&str, &[&str]); 2] = [
        ("dressed-rest", &[]),
        (
            "dressed-turned",
            &["--param", "AngleX=25", "--param", "BodyAngleX=8"],
        ),
    ];
    for (name, pose) in poses {
        let with = |modules: &[String]| -> Vec<String> {
            let mut all: Vec<String> = pose.iter().map(|s| (*s).to_owned()).collect();
            all.extend(modules.iter().cloned());
            all
        };
        let (a, b) = (with(&original_args), with(&atlased_args));
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        let b: Vec<&str> = b.iter().map(String::as_str).collect();
        let original = render(&base, &dir.join(format!("{name}-layers.png")), &a);
        let packed = render(&base, &dir.join(format!("{name}-atlas.png")), &b);
        assert_close(name, &original, &packed);
    }

    // 本体 → 各モジュールの画像の切り替えが減る (13 回の描画が 10 回になる)。
    let draw_calls = |modules: &[String], out: &str| {
        let mut all = vec![
            OsStr::new("render"),
            base.as_os_str(),
            OsStr::new("--stats"),
        ];
        all.extend(modules.iter().map(OsStr::new));
        let out = dir.join(out);
        all.extend([OsStr::new("-o"), out.as_os_str()]);
        let stdout = ugoe(&all);
        let line = stdout
            .lines()
            .find_map(|l| l.strip_prefix("stats: "))
            .unwrap_or_else(|| panic!("no stats in {stdout}"));
        let stats: serde_json::Value = serde_json::from_str(line).expect("stats json");
        stats["draw_calls"].as_u64().expect("draw_calls")
    };
    let before = draw_calls(&original_args, "dressed-stats-layers.png");
    let after = draw_calls(&atlased_args, "dressed-stats-atlas.png");
    assert!(after < before, "draw calls {before} -> {after}");
}
