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

use std::path::{Path, PathBuf};

use ugoe::core::{Rgb, Vec2};
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::{
    BlendMode, KeyformBinding, KeyformGrid, MaskRef, Mesh, MeshId, Model, TextureRef, fixtures,
};
use ugoe_headless::{Rgba8, image};

use crate::Error;

/// 作るモデルの名前 (`<名前>.ugoe` に書く)。
pub const NAMES: [&str; 8] = [
    "nested-masks",
    "mixed-masks",
    "intersection-masks",
    "blend-modes",
    "many-masks",
    "moving-masks",
    "lip-sync-hair",
    "part-switch",
];

/// すべてのモデルを `dir` に書き、書いたパスを返す。
pub fn write_all(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    std::fs::create_dir_all(dir).map_err(Error::io(format!("creating {}", dir.display())))?;
    let png = texture()?;
    let mut paths = Vec::new();
    for name in NAMES {
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

/// 64x64 の、色とアルファが位置で変わるテクスチャ (PNG、乗算していないアルファ)。
fn texture() -> Result<Vec<u8>, Error> {
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
    // PNG はストレートアルファ。書き出しの関数は乗算済みを受け取るので、先に掛けておく。
    image::premultiply(&mut rgba);
    image::encode_png_unpremultiplied(&Rgba8 {
        width: SIZE,
        height: SIZE,
        pixels: rgba,
    })
    .map_err(|source| Error::Headless {
        context: "encoding the texture of the generated bundles".to_owned(),
        source,
    })
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
        if mesh.id.as_str() == "Pupil" {
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
            assert_eq!(bundle.textures.len(), 1);
            ugoe::Puppet::new(bundle.model).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
