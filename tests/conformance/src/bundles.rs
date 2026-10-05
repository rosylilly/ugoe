//! 計画で `generated` と書いたモデル (`.ugoe`) を作る。
//!
//! `ugoe_model::fixtures` のマスクのモデルに色と半透明のテクスチャを付け、マスク・合成モード・
//! 乗算色とスクリーン色・裏面カリング・乗算済みアルファへの変換の違いが画素に出るようにする。
//! 1 つの要素が多くのマスクを読むモデル (`many-masks`) と、パラメータで入れ子のマスクが動くモデル
//! (`moving-masks`。アニメーション中にマスクが 1 フレーム遅れないかを `before` で確かめる) も作る。
//! `lip-sync-hair` は動的物理の髪で、リップシンクの対象を物理の入力 (`AngleX`) にしたもの
//! (受け手が物理より後の段を同じ順に当てるかを確かめる。T0134)。
//! `part-switch` はポーズ (`ArmA`+`SleeveA` と `ArmB`) とポーズに属さないパーツ `Badge` にメッシュを持たせ、
//! パーツを切り替えるモーション `switch` を入れたもの (受け手がモーションのパーツの不透明度のカーブを当てるかを確かめる。T0135)。
//! もとは Godot の `build.mjs --conformance` の `conformance_bundles` (T0411) で、T0418 でここに移した。
//!
//! KTX2 のテクスチャのバンドルも作る (T0222・T0223。もとは Godot の `example/tests/ktx2.gd`)。圧縮しない KTX2 の画素は PNG と同じで、KTX2 も
//! 乗算していないアルファとして読んで乗算すれば (`docs/spec/rendering.md#前提`)、PNG のバンドルと同じ画像になる。
//! - `avatar-ktx2` / `avatar-lit-ktx2`: 見本 (`assets/samples/avatar.ugoe`・`avatar-lit.ugoe`) のテクスチャと法線マップを、
//!   同じ画素の `VK_FORMAT_R8G8B8A8_SRGB` の KTX2 にしたもの (転送関数が sRGB でも値はそのまま使う)。
//! - `ktx2-mips`: `nested-masks` のテクスチャを `VK_FORMAT_R8G8B8A8_UNORM` の zstd で超圧縮した KTX2 にしたもの。
//!   ミップマップの 2 段目からは別の色 (不透明なマゼンタ) で、最初の段だけを使わなければ画像が変わる。
//! - `avatar-bc3`: 見本のテクスチャを `VK_FORMAT_BC3_UNORM_BLOCK` にしたもの (Godot の `ktx2.gd` の BC3 を移した)。
//! - `avatar-lit-bc`: ライティングの見本のテクスチャを zstd の `VK_FORMAT_BC7_SRGB_BLOCK` に、法線マップを
//!   `VK_FORMAT_BC1_RGB_UNORM_BLOCK` にしたもの。ブロック圧縮は画素が変わるので、PNG のバンドルとは同じ画像にならない
//!   (`docs/adr/0030-compressed-ktx2-decoding.md`)。

use std::path::{Path, PathBuf};

use ugoe::core::{Rgb, Vec2};
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::{
    BlendMode, KeyformBinding, KeyformGrid, MaskRef, Mesh, MeshId, Model, TextureRef, fixtures,
};
use ugoe_headless::image::{Ktx2Compression, Ktx2Options};
use ugoe_headless::{Rgba8, image};

use crate::Error;

/// 作るモデルの名前 (`<名前>.ugoe` に書く)。
pub const NAMES: [&str; 13] = [
    "nested-masks",
    "mixed-masks",
    "intersection-masks",
    "blend-modes",
    "many-masks",
    "moving-masks",
    "lip-sync-hair",
    "part-switch",
    "avatar-ktx2",
    "avatar-lit-ktx2",
    "ktx2-mips",
    "avatar-bc3",
    "avatar-lit-bc",
];

/// 圧縮していない sRGB の KTX2 (`R8G8B8A8_SRGB`)。
const RGBA8_SRGB: Ktx2Options = Ktx2Options {
    srgb: true,
    zstd: false,
    compression: Ktx2Compression::None,
};

/// KTX2 にする見本 (生成するモデルの名前・リポジトリのルートからのパス・テクスチャと法線マップの書き方)。
const KTX2_SAMPLES: [(&str, &str, Ktx2Options, Ktx2Options); 4] = [
    (
        "avatar-ktx2",
        "assets/samples/avatar.ugoe",
        RGBA8_SRGB,
        RGBA8_SRGB,
    ),
    (
        "avatar-lit-ktx2",
        "assets/samples/avatar-lit.ugoe",
        RGBA8_SRGB,
        RGBA8_SRGB,
    ),
    (
        "avatar-bc3",
        "assets/samples/avatar.ugoe",
        Ktx2Options {
            srgb: false,
            zstd: false,
            compression: Ktx2Compression::Bc3,
        },
        RGBA8_SRGB,
    ),
    (
        "avatar-lit-bc",
        "assets/samples/avatar-lit.ugoe",
        Ktx2Options {
            srgb: true,
            zstd: true,
            compression: Ktx2Compression::Bc7,
        },
        Ktx2Options {
            srgb: false,
            zstd: false,
            compression: Ktx2Compression::Bc1,
        },
    ),
];

/// すべてのモデルを `dir` に書き、書いたパスを返す。
pub fn write_all(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    std::fs::create_dir_all(dir).map_err(Error::io(format!("creating {}", dir.display())))?;
    let png = encode_png(&texture())?;
    let mut paths = Vec::new();
    for name in NAMES {
        let bundle = if let Some(&(_, sample, textures, normal_maps)) =
            KTX2_SAMPLES.iter().find(|(n, ..)| *n == name)
        {
            sample_as_ktx2(sample, textures, normal_maps)?
        } else if name == "ktx2-mips" {
            let mut bundle = Bundle::new(model("nested-masks")?);
            bundle.textures = vec![mipmapped_ktx2()?];
            bundle
        } else {
            let mut bundle = Bundle::new(model(name)?);
            if name == "lip-sync-hair" {
                bundle.physics = Some(fixtures::dynamic_hair_settings());
            }
            if name == "part-switch" {
                bundle.motions = vec![switch_motion()];
            }
            bundle.textures = vec![TextureData {
                format: TextureFormat::Png,
                bytes: png.clone(),
            }];
            bundle
        };
        let bytes = ugoe::format::write_bundle(&bundle).map_err(|source| Error::Format {
            context: format!("writing the generated bundle `{name}`"),
            source,
        })?;
        let path = dir.join(format!("{name}.ugoe"));
        std::fs::write(&path, bytes).map_err(Error::io(format!("writing {}", path.display())))?;
        paths.push(path);
    }
    Ok(paths)
}

fn model(name: &str) -> Result<Model, Error> {
    Ok(match name {
        "nested-masks" => colored(fixtures::nested_masks()),
        "mixed-masks" => colored(fixtures::mixed_masks()),
        "intersection-masks" => colored(fixtures::intersection_masks()),
        "blend-modes" => blend_modes(),
        "many-masks" => many_masks(),
        "moving-masks" => moving_masks(),
        "lip-sync-hair" => lip_sync_hair(),
        "part-switch" => part_switch(),
        other => return Err(Error::Invalid(format!("unknown generated model `{other}`"))),
    })
}

/// 64x64 の、色とアルファが位置で変わるテクスチャ (乗算していないアルファ)。
fn texture() -> Rgba8 {
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
    Rgba8 {
        width: SIZE,
        height: SIZE,
        pixels: rgba,
    }
}

/// 乗算していないアルファの `straight` を PNG にする。
fn encode_png(straight: &Rgba8) -> Result<Vec<u8>, Error> {
    // PNG はストレートアルファ。書き出しの関数は乗算済みを受け取るので、先に掛けておく。
    let mut premultiplied = straight.clone();
    image::premultiply(&mut premultiplied.pixels);
    image::encode_png_unpremultiplied(&premultiplied).map_err(|source| Error::Headless {
        context: "encoding the texture of the generated bundles".to_owned(),
        source,
    })
}

fn encode_ktx2(levels: &[Rgba8], options: Ktx2Options) -> Result<TextureData, Error> {
    Ok(TextureData {
        format: TextureFormat::Ktx2,
        bytes: image::encode_ktx2(levels, options).map_err(|source| Error::Headless {
            context: "encoding a KTX2 texture of the generated bundles".to_owned(),
            source,
        })?,
    })
}

/// 見本のバンドル `sample` を読み、テクスチャと法線マップ (PNG) を、同じ画素をそれぞれ `textures` と `normal_maps` の
/// 書き方で KTX2 にしたものに差し替える (ブロック圧縮なら画素は圧縮で変わる)。
fn sample_as_ktx2(
    sample: &str,
    textures: Ktx2Options,
    normal_maps: Ktx2Options,
) -> Result<Bundle, Error> {
    let path = crate::repo_root().join(sample);
    let bytes = std::fs::read(&path).map_err(Error::io(format!("reading {}", path.display())))?;
    let mut bundle = ugoe::format::read_bundle(&bytes).map_err(|source| Error::Format {
        context: format!("reading {}", path.display()),
        source,
    })?;
    let images = bundle.textures.iter_mut().map(|data| (data, textures));
    let maps = bundle
        .normal_maps
        .iter_mut()
        .map(|data| (data, normal_maps));
    for (data, options) in images.chain(maps) {
        let straight =
            image::decode_image(data.format, &data.bytes).map_err(|source| Error::Headless {
                context: format!("decoding a texture of {sample}"),
                source,
            })?;
        *data = encode_ktx2(&[straight], options)?;
    }
    Ok(bundle)
}

/// [`texture`] のミップマップ付きの KTX2 (`R8G8B8A8_UNORM`、zstd)。2 段目からは不透明なマゼンタ。
fn mipmapped_ktx2() -> Result<TextureData, Error> {
    let base = texture();
    let mut levels = vec![base.clone()];
    let mut side = base.width / 2;
    while side >= 1 {
        levels.push(Rgba8 {
            width: side,
            height: side,
            pixels: [255, 0, 255, 255].repeat((side * side) as usize),
        });
        side /= 2;
    }
    encode_ktx2(
        &levels,
        Ktx2Options {
            srgb: false,
            zstd: true,
            compression: Ktx2Compression::None,
        },
    )
}

fn tint(mesh: &mut Mesh, multiply: Rgb, screen: Rgb) {
    for k in &mut mesh.keyforms.keyforms {
        k.multiply_color = multiply;
        k.screen_color = screen;
    }
}

/// 動的物理の髪 (`fixtures::dynamic_hair`。物理は [`write_all`] が付ける) で、リップシンクの対象を物理の入力の `AngleX` にしたもの。
fn lip_sync_hair() -> Model {
    let mut model = colored(fixtures::dynamic_hair());
    model.meta.name = "lip_sync_hair".into();
    model.effects.lip_sync.parameters = vec![ugoe::model::standard::ANGLE_X.into()];
    model
}

/// ポーズとエフェクトのフィクスチャ (`fixtures::pose_and_effects`) の、ポーズの要素のパーツ (`ArmA`・`SleeveA`・`ArmB`) と
/// ポーズに属さないパーツ `Badge` に 1 枚ずつ四角形を置いたもの。パーツの不透明度がメッシュの不透明度に出る。
fn part_switch() -> Model {
    let mut model = fixtures::pose_and_effects();
    model.meta.name = "part_switch".into();
    let mut badge = model.parts[1].clone();
    badge.id = "Badge".into();
    badge.name = "Badge".into();
    model.parts.push(badge);
    let template = model.meshes[0].clone();
    let boxes = [
        ("ArmA", 16.0, 16.0, 112.0, 112.0),
        ("SleeveA", 144.0, 16.0, 240.0, 112.0),
        ("ArmB", 16.0, 144.0, 112.0, 240.0),
        ("Badge", 144.0, 144.0, 240.0, 240.0),
    ];
    for (i, (part, x0, y0, x1, y1)) in boxes.into_iter().enumerate() {
        let mut mesh = template.clone();
        mesh.id = MeshId::new(format!("{part}Quad"));
        mesh.name = mesh.id.as_str().to_owned();
        mesh.part = part.into();
        place(&mut mesh, x0, y0, x1, y1);
        for k in &mut mesh.keyforms.keyforms {
            k.draw_order = 600.0 + i as f32;
        }
        model.meshes.push(mesh);
    }
    colored(model)
}

/// `part-switch` のモーション `switch`: 0.4 秒で `ArmA` から `ArmB` に切り替え (ポーズのクロスフェード)、`Badge` の不透明度を
/// 1 から 0.2 へ下げる。フェードインとフェードアウトがあるので、カーブの重みも変わる。
fn switch_motion() -> Motion {
    let p = |t: f32, v: f32| Point { t, v };
    let curve = |part: &str, segments: Vec<Segment>| Curve {
        target: CurveTarget::PartOpacity(part.into()),
        segments,
        fade_in: None,
        fade_out: None,
    };
    Motion {
        name: "switch".into(),
        duration: 0.8,
        looping: false,
        fade_in: 0.3,
        fade_out: 0.3,
        curves: vec![
            curve(
                "ArmA",
                vec![
                    Segment::Start { p: p(0.0, 1.0) },
                    Segment::Stepped { to: p(0.4, 0.0) },
                ],
            ),
            curve(
                "ArmB",
                vec![
                    Segment::Start { p: p(0.0, 0.0) },
                    Segment::Stepped { to: p(0.4, 1.0) },
                ],
            ),
            curve(
                "Badge",
                vec![
                    Segment::Start { p: p(0.0, 1.0) },
                    Segment::Linear { to: p(0.8, 0.2) },
                ],
            ),
        ],
        events: vec![],
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

/// 四角形のメッシュ `mesh` の位置を `(x0, y0)`〜`(x1, y1)` にする (頂点の順は元のまま)。
fn place(mesh: &mut Mesh, x0: f32, y0: f32, x1: f32, y1: f32) {
    for k in &mut mesh.keyforms.keyforms {
        let lo = k.positions.iter().fold(k.positions[0], |lo, &p| lo.min(p));
        for p in &mut k.positions {
            p.x = if p.x <= lo.x { x0 } else { x1 };
            p.y = if p.y <= lo.y { y0 } else { y1 };
        }
    }
}

/// `Face` が 12 個のマスク (うち 3 個は反転) を読む。以前のバインディングの上限 (8) を超える数。
fn many_masks() -> Model {
    let mut model = colored(fixtures::nested_masks());
    model.meta.name = "many_masks".into();
    let template = model.meshes[2].clone();
    model.meshes.clear();
    let mut face = template.clone();
    face.id = MeshId::new("Face");
    place(&mut face, 24.0, 24.0, 232.0, 232.0);
    face.masks = Vec::new();
    for i in 0..12u16 {
        let mut mask = template.clone();
        mask.id = MeshId::new(format!("Mask{i}"));
        mask.masks.clear();
        let (col, row) = (f32::from(i % 4), f32::from(i / 4));
        let (x, y) = (16.0 + col * 56.0, 24.0 + row * 72.0);
        place(&mut mask, x, y, x + 64.0, y + 60.0);
        for k in &mut mask.keyforms.keyforms {
            k.draw_order = 100.0 + f32::from(i);
        }
        face.masks.push(if i % 4 == 3 {
            MaskRef::inverted(mask.id.as_str())
        } else {
            MaskRef::normal(mask.id.as_str())
        });
        model.meshes.push(mask);
    }
    for k in &mut face.keyforms.keyforms {
        k.draw_order = 900.0;
    }
    model.meshes.push(face);
    model
}

/// 入れ子のマスク (白目 → 黒目 → 瞳孔) の白目と黒目を、パラメータ `Move` (-1〜1) で左右に動かす。
fn moving_masks() -> Model {
    let mut model = colored(fixtures::nested_masks());
    model.meta.name = "moving_masks".into();
    let mut parameter = fixtures::mixed_masks().parameters[0].clone();
    parameter.id = "Move".into();
    parameter.name = "Move".into();
    parameter.min = -1.0;
    parameter.max = 1.0;
    parameter.default = 0.0;
    model.parameters.push(parameter);
    for mesh in &mut model.meshes {
        if mesh.id == "Pupil" {
            continue;
        }
        let base = mesh.keyforms.keyforms[0].clone();
        let shifted = |dx: f32| {
            let mut k = base.clone();
            for p in &mut k.positions {
                *p += Vec2::new(dx, 0.0);
            }
            k
        };
        mesh.keyforms = KeyformGrid::new(
            vec![KeyformBinding {
                parameter: "Move".into(),
                keys: vec![-1.0, 1.0],
            }],
            vec![shifted(-40.0), shifted(40.0)],
        );
    }
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_bundles_read_back() {
        let dir =
            std::env::temp_dir().join(format!("ugoe-conformance-bundles-{}", std::process::id()));
        let paths = write_all(&dir).unwrap();
        assert_eq!(paths.len(), NAMES.len());
        for path in &paths {
            let bundle = ugoe_format::read_bundle(&std::fs::read(path).unwrap()).unwrap();
            assert!(!bundle.textures.is_empty());
            ugoe::Puppet::new(bundle.model).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn decoded(data: &[TextureData]) -> Vec<Rgba8> {
        data.iter()
            .map(|d| image::decode_image_premultiplied(d.format, &d.bytes).unwrap())
            .collect()
    }

    /// 圧縮していない KTX2 のバンドルは、元の PNG のバンドルと同じ乗算済みの画素にデコードされる (なので参照の画像も
    /// 同じになる)。ブロック圧縮のバンドルは大きさが同じで、画素は近い。見本の法線マップも KTX2 にする。
    #[test]
    fn ktx2_bundles_decode_like_the_png_ones() {
        for (name, sample, textures, normal_maps) in KTX2_SAMPLES {
            let png =
                ugoe_format::read_bundle(&std::fs::read(crate::repo_root().join(sample)).unwrap())
                    .unwrap();
            let ktx2 = sample_as_ktx2(sample, textures, normal_maps).unwrap();
            assert!(
                ktx2.textures
                    .iter()
                    .chain(&ktx2.normal_maps)
                    .all(|t| t.format == TextureFormat::Ktx2)
            );
            assert_eq!(ktx2.model, png.model);
            for (ktx2, png, options) in [
                (&ktx2.textures, &png.textures, textures),
                (&ktx2.normal_maps, &png.normal_maps, normal_maps),
            ] {
                let (ktx2, png) = (decoded(ktx2), decoded(png));
                if options.compression == Ktx2Compression::None {
                    assert_eq!(ktx2, png, "{name}");
                    continue;
                }
                assert_eq!(ktx2.len(), png.len(), "{name}");
                for (a, b) in ktx2.iter().zip(&png) {
                    assert_eq!((a.width, a.height), (b.width, b.height), "{name}");
                    // 乗算済みの画素の平均の差。書き出しの圧縮は簡単なもので (`ugoe_headless::image::Ktx2Compression`)、
                    // 細かい小さなテクスチャでは BC7 で 10 ほど違う。
                    let total: u64 = a
                        .pixels
                        .iter()
                        .zip(&b.pixels)
                        .map(|(x, y)| u64::from(x.abs_diff(*y)))
                        .sum();
                    let mean = total as f64 / a.pixels.len() as f64;
                    assert!(mean < 12.0, "{name}: mean difference {mean}");
                }
            }
        }
        let lit = sample_as_ktx2("assets/samples/avatar-lit.ugoe", RGBA8_SRGB, RGBA8_SRGB).unwrap();
        assert!(!lit.normal_maps.is_empty());
        let png = TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(&texture()).unwrap(),
        };
        assert_eq!(decoded(&[mipmapped_ktx2().unwrap()]), decoded(&[png]));
    }
}
