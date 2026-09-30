//! テクスチャのアトラス化 (`atlas.rs`) のテスト。

use std::sync::Arc;

use ugoe_core::Vec2;
use ugoe_model::{Mesh, Model, TextureRef, fixtures};

use crate::Textures;
use crate::atlas::{AtlasError, AtlasOptions, KeptReason, build_atlas, build_module_atlas};
use crate::texture::{decode_rgba_png, encode_rgba_png};

/// 決まった種から作る、隣り合う画素の差が大きい画像 (端の扱いの誤りが色に出るように)。
fn noise_png(width: u32, height: u32, seed: u32) -> Vec<u8> {
    let mut state = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let pixels: Vec<u8> = (0..width * height * 4)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect();
    encode_rgba_png(width, height, &pixels)
}

/// `single_quad` のメッシュを、描画順 `draw_order` とテクスチャ番号 `texture` で並べたモデル。
/// テクスチャ `sizes[i]` の名前は `tex<i>`。
pub(crate) fn model_with(meshes: &[(u32, f32)], sizes: &[(u32, u32)]) -> (Model, Textures) {
    let mut model = fixtures::single_quad();
    let base = model.meshes[0].clone();
    model.meshes = meshes
        .iter()
        .enumerate()
        .map(|(i, &(texture, draw_order))| {
            let mut mesh: Mesh = base.clone();
            mesh.id = format!("M{i}").as_str().into();
            mesh.name = format!("M{i}");
            mesh.texture = texture;
            for keyform in &mut mesh.keyforms.keyforms {
                keyform.draw_order = draw_order;
            }
            mesh
        })
        .collect();
    let mut textures = Textures::new();
    model.textures = sizes
        .iter()
        .enumerate()
        .map(|(i, &(width, height))| {
            let name = format!("tex{i}");
            textures.insert(name.clone(), Arc::from(noise_png(width, height, i as u32)));
            TextureRef {
                name,
                width,
                height,
                normal_map: None,
            }
        })
        .collect();
    (model, textures)
}

/// 乗算済みにした画像を、参照レンダラと同じくバイリニア・端でクランプして `uv` で読む (0〜255)。
fn sample(png: &[u8], uv: Vec2) -> [f64; 4] {
    let (width, height, rgba) = decode_rgba_png(png).expect("png");
    let texel = |x: i64, y: i64| -> [f64; 4] {
        let x = x.clamp(0, i64::from(width) - 1) as usize;
        let y = y.clamp(0, i64::from(height) - 1) as usize;
        let p = &rgba[(y * width as usize + x) * 4..][..4];
        let a = f64::from(p[3]) / 255.0;
        [
            f64::from(p[0]) * a,
            f64::from(p[1]) * a,
            f64::from(p[2]) * a,
            f64::from(p[3]),
        ]
    };
    let x = f64::from(uv.x) * f64::from(width) - 0.5;
    let y = f64::from(uv.y) * f64::from(height) - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let mut out = [0.0; 4];
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let t = texel(x0 + dx, y0 + dy);
        for c in 0..4 {
            out[c] += t[c] * w;
        }
    }
    out
}

/// 各メッシュの UV の範囲を細かく読み、アトラス化の前後で色が同じであること。
fn assert_same_look(before: (&Model, &Textures), after: (&Model, &Textures)) {
    let steps = 40;
    for (old, new) in before.0.meshes.iter().zip(&after.0.meshes) {
        let old_png = &before.1[&before.0.textures[old.texture as usize].name];
        let new_png = &after.1[&after.0.textures[new.texture as usize].name];
        // 四角形の UV (0,0)〜(1,1) を双線形に補間した点で比べる。
        let at = |uvs: &[Vec2], s: f32, t: f32| {
            let top = uvs[0] * (1.0 - s) + uvs[1] * s;
            let bottom = uvs[3] * (1.0 - s) + uvs[2] * s;
            top * (1.0 - t) + bottom * t
        };
        for i in 0..=steps {
            for j in 0..=steps {
                let (s, t) = (i as f32 / steps as f32, j as f32 / steps as f32);
                let a = sample(old_png, at(&old.uvs, s, t));
                let b = sample(new_png, at(&new.uvs, s, t));
                for c in 0..4 {
                    assert!(
                        (a[c] - b[c]).abs() < 0.5,
                        "mesh {} at ({s}, {t}) channel {c}: {} vs {}",
                        old.id.as_str(),
                        a[c],
                        b[c]
                    );
                }
            }
        }
    }
}

#[test]
fn packs_textures_without_changing_the_look() {
    let (model, textures) = model_with(
        &[(0, 0.0), (1, 1.0), (2, 2.0), (1, 3.0)],
        &[(7, 5), (16, 16), (3, 11)],
    );
    let atlased = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    assert_eq!(atlased.report.packed, 3);
    assert_eq!(atlased.report.atlases.len(), 1);
    assert!(atlased.report.kept.is_empty());
    assert_eq!(atlased.model.textures.len(), 1);
    assert_eq!(atlased.model.textures[0].name, "atlas-0");
    let (w, h) = atlased.report.atlases[0];
    assert_eq!(
        (
            atlased.model.textures[0].width,
            atlased.model.textures[0].height
        ),
        (w, h)
    );
    assert_eq!(
        decode_rgba_png(&atlased.textures["atlas-0"]).map(|i| (i.0, i.1)),
        Some((w, h))
    );
    assert!(atlased.model.meshes.iter().all(|m| m.texture == 0));
    assert!(ugoe_model::validate::validate(&atlased.model).is_empty());
    assert_same_look((&model, &textures), (&atlased.model, &atlased.textures));
}

#[test]
fn padding_of_one_is_enough_for_bilinear_sampling() {
    let (model, textures) = model_with(&[(0, 0.0), (1, 1.0)], &[(4, 4), (5, 3)]);
    let options = AtlasOptions {
        padding: 1,
        ..AtlasOptions::default()
    };
    let atlased = build_atlas(&model, &textures, &options).expect("atlas");
    assert_same_look((&model, &textures), (&atlased.model, &atlased.textures));
}

#[test]
fn neighbours_in_draw_order_share_an_atlas() {
    // 宣言順と描画順を変えておく: 描画順は M2 (tex2) → M0 (tex0) → M1 (tex1) → M3 (tex3)。
    // 30x30 (+ 周り 2) の画像は 64 四方に 4 枚まで入るので、描画順の前の 2 枚ずつではなく、
    // 前から詰めて収まらなくなった所で分かれる。
    let (model, textures) = model_with(
        &[(0, 1.0), (1, 2.0), (2, 0.0), (3, 3.0), (4, 4.0)],
        &[(30, 30); 5],
    );
    let options = AtlasOptions {
        max_size: 64,
        padding: 1,
    };
    let atlased = build_atlas(&model, &textures, &options).expect("atlas");
    assert_eq!(atlased.report.atlases.len(), 2);
    let textures_of: Vec<u32> = atlased.model.meshes.iter().map(|m| m.texture).collect();
    // 描画順の先頭の 4 枚 (tex2, tex0, tex1, tex3) が 1 枚目、tex4 が 2 枚目。
    assert_eq!(textures_of, vec![0, 0, 0, 0, 1]);
    assert_same_look((&model, &textures), (&atlased.model, &atlased.textures));
}

#[test]
fn keeps_textures_that_cannot_be_packed_and_removes_unused_ones() {
    let (mut model, textures) = model_with(
        &[(0, 0.0), (1, 1.0), (2, 2.0)],
        &[(8, 8), (100, 8), (8, 8), (8, 8)],
    );
    // tex2 は UV が 0〜1 の外 (端でクランプされることに頼っている)。tex3 はどのメッシュも使わない。
    model.meshes[2].uvs[1] = Vec2::new(1.5, 0.0);
    let options = AtlasOptions {
        max_size: 64,
        padding: 2,
    };
    let atlased = build_atlas(&model, &textures, &options).expect("atlas");
    let report = &atlased.report;
    assert_eq!(report.packed, 1);
    assert_eq!(
        report.kept,
        vec![
            ("tex1".to_owned(), KeptReason::TooLarge),
            ("tex2".to_owned(), KeptReason::UvOutOfRange),
        ]
    );
    assert_eq!(report.removed, vec!["tex3".to_owned()]);
    let names: Vec<&str> = atlased
        .model
        .textures
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, vec!["atlas-0", "tex1", "tex2"]);
    assert_eq!(atlased.textures.len(), 3);
    assert!(Arc::ptr_eq(&atlased.textures["tex1"], &textures["tex1"]));
    let textures_of: Vec<u32> = atlased.model.meshes.iter().map(|m| m.texture).collect();
    assert_eq!(textures_of, vec![0, 1, 2]);
    // 残したテクスチャを使うメッシュの UV は変えない。
    assert_eq!(atlased.model.meshes[1].uvs, model.meshes[1].uvs);
    assert_eq!(atlased.model.meshes[2].uvs, model.meshes[2].uvs);
}

#[test]
fn atlas_names_avoid_kept_texture_names() {
    let (mut model, mut textures) = model_with(&[(0, 0.0), (1, 1.0)], &[(8, 8), (8, 8)]);
    let png = textures.remove("tex1").expect("tex1");
    textures.insert("atlas-0".into(), png);
    model.textures[1].name = "atlas-0".into();
    model.meshes[1].uvs[0] = Vec2::new(-0.1, 0.0);
    let atlased = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    let names: Vec<&str> = atlased
        .model
        .textures
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, vec!["atlas-1", "atlas-0"]);
}

#[test]
fn output_is_deterministic() {
    let (model, textures) = model_with(&[(0, 0.0), (1, 1.0), (2, 2.0)], &[(9, 4), (4, 9), (6, 6)]);
    let a = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    let b = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    assert_eq!(a, b);
}

#[test]
fn reports_bad_input() {
    let (model, mut textures) = model_with(&[(0, 0.0)], &[(4, 4)]);
    textures.insert("tex0".into(), Arc::from(&b"not a png"[..]));
    assert!(matches!(
        build_atlas(&model, &textures, &AtlasOptions::default()),
        Err(AtlasError::NotPng { .. })
    ));
    assert!(matches!(
        build_atlas(&model, &Textures::new(), &AtlasOptions::default()),
        Err(AtlasError::MissingTexture { .. })
    ));
    let mut out_of_range = model.clone();
    out_of_range.meshes[0].texture = 3;
    assert!(matches!(
        build_atlas(&out_of_range, &textures, &AtlasOptions::default()),
        Err(AtlasError::TextureOutOfRange { .. })
    ));
    let options = AtlasOptions {
        max_size: 0,
        padding: 0,
    };
    assert!(matches!(
        build_atlas(&model, &textures, &options),
        Err(AtlasError::InvalidMaxSize)
    ));
}

#[test]
fn keeps_textures_with_a_normal_map() {
    let (mut model, mut textures) = model_with(&[(0, 0.0), (1, 1.0)], &[(8, 8), (8, 8)]);
    model.textures[1].normal_map = Some("tex1_normal".into());
    textures.insert("tex1_normal".into(), Arc::from(noise_png(8, 8, 99)));
    let atlased = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    assert_eq!(atlased.report.packed, 1);
    assert_eq!(
        atlased.report.kept,
        vec![("tex1".to_owned(), KeptReason::HasNormalMap)]
    );
    assert_eq!(atlased.model.textures[1], model.textures[1]);
    assert!(Arc::ptr_eq(
        &atlased.textures["tex1_normal"],
        &textures["tex1_normal"]
    ));
    // 法線マップを持つテクスチャを使うメッシュの UV は変えない。
    assert_eq!(atlased.model.meshes[1].texture, 1);
    assert_eq!(atlased.model.meshes[1].uvs, model.meshes[1].uvs);
}

#[test]
fn identical_images_are_packed_once() {
    // tex1 は tex0 と同じ画像。tex2 は大きさだけ同じ別の画像。
    let (model, mut textures) = model_with(
        &[(0, 0.0), (1, 1.0), (2, 2.0), (1, 3.0)],
        &[(6, 6), (6, 6), (6, 6)],
    );
    let same = textures["tex0"].to_vec();
    textures.insert("tex1".into(), Arc::from(same));
    let atlased = build_atlas(&model, &textures, &AtlasOptions::default()).expect("atlas");
    let report = &atlased.report;
    assert_eq!(report.packed, 3);
    assert_eq!(
        report.deduplicated,
        vec![("tex1".to_owned(), "tex0".to_owned())]
    );
    // 2 枚分 (6 + 2 × 2 = 10 px 四方が 2 つ) だけ詰める。
    assert_eq!(report.atlases, vec![(10, 20)]);
    assert_eq!(atlased.model.meshes[0].uvs, atlased.model.meshes[1].uvs);
    assert_ne!(atlased.model.meshes[0].uvs, atlased.model.meshes[2].uvs);
    assert_same_look((&model, &textures), (&atlased.model, &atlased.textures));
}

/// モジュールのテクスチャはモジュールの単位でまとめる。描画順は本体に付けないと決まらないので、メッシュの宣言順で詰める。
#[test]
fn packs_module_textures_in_declaration_order() {
    // 宣言順は tex1 → tex0 (描画順は逆)。1 枚に 1 つしか入らない上限にして、詰める順をアトラスの番号で見る。
    let (model, textures) = model_with(&[(1, 1.0), (0, 0.0), (2, 2.0)], &[(8, 8), (6, 6), (4, 4)]);
    let mut module = fixtures::long_hair_module();
    module.meshes = model.meshes.clone();
    module.textures = model.textures.clone();
    let options = AtlasOptions {
        max_size: 12,
        padding: 2,
    };
    let atlased = build_module_atlas(&module, &textures, &options).expect("atlas");
    assert_eq!(atlased.report.packed, 3);
    assert_eq!(atlased.report.atlases, vec![(10, 10), (12, 12), (8, 8)]);
    let names: Vec<_> = atlased
        .module
        .textures
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["atlas-0", "atlas-1", "atlas-2"]);
    assert_eq!(atlased.module.meshes[0].texture, 0);
    assert_eq!(atlased.module.meshes[1].texture, 1);
    // モジュールのほかの部分は変えない。
    assert_eq!(atlased.module.parts, module.parts);
    assert_eq!(atlased.module.physics, module.physics);
    // 見た目は変わらない (モデルに入れ直して比べる)。
    let mut after = model.clone();
    after.meshes = atlased.module.meshes.clone();
    after.textures = atlased.module.textures.clone();
    assert_same_look((&model, &textures), (&after, &atlased.textures));
}
