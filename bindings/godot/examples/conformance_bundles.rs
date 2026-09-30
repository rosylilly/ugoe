//! 描画の比べ合わせ (`build.mjs --conformance`) に使う `.ugoe` を書き出す。
//!
//! `ugoe_model::fixtures` のマスクのモデルに色と半透明のテクスチャを付け、マスク・合成モード・
//! 乗算色とスクリーン色・裏面カリング・乗算済みアルファへの変換の違いが画素に出るようにする。
//!
//!   cargo run --example conformance_bundles -- <出力先のディレクトリ>

use std::path::PathBuf;

use ugoe::core::Rgb;
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::{BlendMode, Mesh, Model, TextureRef, fixtures};

/// 64x64 の、色とアルファが位置で変わるテクスチャ (PNG、乗算していないアルファ)。
fn texture() -> Vec<u8> {
    const SIZE: u32 = 64;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (fx, fy) = (x as f32 / (SIZE - 1) as f32, y as f32 / (SIZE - 1) as f32);
            rgba.extend([
                (255.0 * fx) as u8,
                (255.0 * fy) as u8,
                (255.0 * (1.0 - fx)) as u8,
                (64.0 + 191.0 * (1.0 - fy * fx)) as u8,
            ]);
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&rgba).expect("png data");
    writer.finish().expect("png finish");
    out
}

fn tint(mesh: &mut Mesh, multiply: Rgb, screen: Rgb) {
    for k in &mut mesh.keyforms.keyforms {
        k.multiply_color = multiply;
        k.screen_color = screen;
    }
}

fn colored(mut model: Model) -> Model {
    let colors = [
        (Rgb::new(1.0, 0.4, 0.4), Rgb::BLACK),
        (Rgb::new(0.4, 1.0, 0.4), Rgb::new(0.0, 0.0, 0.3)),
        (Rgb::new(0.5, 0.5, 1.0), Rgb::BLACK),
    ];
    for (mesh, (m, s)) in model.meshes.iter_mut().zip(colors.iter().cycle()) {
        tint(mesh, *m, *s);
    }
    model.textures = vec![TextureRef {
        name: "gradient".into(),
        width: 64,
        height: 64,
        normal_map: None,
    }];
    model
}

/// 3 枚の四角形を重ね、合成モードを normal・additive・multiply にする。1 枚は裏返してカリングする。
fn blend_modes() -> Model {
    let mut model = colored(fixtures::nested_masks());
    model.meta.name = "blend_modes".into();
    let modes = [BlendMode::Multiply, BlendMode::Additive, BlendMode::Normal];
    for (mesh, blend) in model.meshes.iter_mut().zip(modes) {
        mesh.masks.clear();
        mesh.blend = blend;
    }
    // 最後のメッシュは三角形の向きを逆にして裏向きにし、カリングで消す。もう 1 枚は表のままカリングする。
    let last = model.meshes.len() - 1;
    for tri in model.meshes[last].indices.chunks_mut(3) {
        tri.swap(1, 2);
    }
    model.meshes[last].culling = true;
    model.meshes[0].culling = true;
    model
}

fn main() {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: conformance_bundles <dir>"),
    );
    std::fs::create_dir_all(&out).expect("create dir");
    let png = texture();
    let models = [
        ("nested-masks", colored(fixtures::nested_masks())),
        ("mixed-masks", colored(fixtures::mixed_masks())),
        (
            "intersection-masks",
            colored(fixtures::intersection_masks()),
        ),
        ("blend-modes", blend_modes()),
    ];
    for (name, model) in models {
        let mut bundle = Bundle::new(model);
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: png.clone(),
        }];
        let bytes = ugoe::format::write_bundle(&bundle).expect("write bundle");
        let path = out.join(format!("{name}.ugoe"));
        std::fs::write(&path, bytes).expect("write file");
        println!("{}", path.display());
    }
}
