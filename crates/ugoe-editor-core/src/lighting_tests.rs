//! ライティングの操作 (`lighting.rs`) と、`set_mesh_properties` の `lit` のテスト。

use std::sync::Arc;

use ugoe_model::{Model, fixtures};

use crate::lighting::{
    BulgeSettings, LightingError, PreviewLight, PreviewLighting, assign_normal_map,
    generate_normal_map, normal_map_name,
};
use crate::message::Coded;
use crate::texture::{
    DEFAULT_MAX_IMAGE_SIDE, decode_rgba_png, encode_rgba_png, huge_png_header, solid_png,
};
use crate::{Command, Document, ProjectMeta, Textures};

const SIZE: u32 = 32;

/// 中心に不透明な円のあるテクスチャ `white` を持つ 1 枚の四角。
fn disc_document() -> Document {
    let mut model = fixtures::single_quad();
    model.textures[0].width = SIZE;
    model.textures[0].height = SIZE;
    let c = SIZE as f32 / 2.0;
    let rgba: Vec<u8> = (0..SIZE * SIZE)
        .flat_map(|i| {
            let (x, y) = ((i % SIZE) as f32 + 0.5, (i / SIZE) as f32 + 0.5);
            let a = if (x - c).hypot(y - c) < c - 2.0 {
                255
            } else {
                0
            };
            [255, 255, 255, a]
        })
        .collect();
    let textures: Textures = [(
        "white".to_owned(),
        Arc::from(encode_rgba_png(SIZE, SIZE, &rgba)),
    )]
    .into_iter()
    .collect();
    Document::with_textures(model, textures)
}

fn lit(id: &str, lit: bool) -> Command {
    Command::SetMeshProperties {
        id: id.into(),
        part: None,
        texture: None,
        texture_name: None,
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
        lit: Some(lit),
    }
}

#[test]
fn set_mesh_properties_changes_lit_and_undoes() {
    let mut doc = Document::new(fixtures::single_quad());
    assert!(doc.model().meshes[0].lit);
    doc.apply(lit("Quad", false)).unwrap();
    assert!(!doc.model().meshes[0].lit);
    // 省略すれば変えない。
    let cmd: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Quad","culling":true}"#).unwrap();
    doc.apply(cmd).unwrap();
    assert!(!doc.model().meshes[0].lit);
    assert!(doc.undo());
    assert!(doc.undo());
    assert!(doc.model().meshes[0].lit);
    assert!(doc.redo());
    assert!(!doc.model().meshes[0].lit);
}

#[test]
fn generated_normal_map_bulges_and_enables_lighting_in_one_undo() {
    let mut doc = disc_document();
    let before = (doc.model().clone(), doc.textures().clone());
    assert!(!doc.model().lighting);
    let cmd = generate_normal_map(
        doc.model(),
        doc.textures(),
        "white",
        BulgeSettings::default(),
    )
    .unwrap();
    doc.apply(cmd).unwrap();
    assert!(doc.model().lighting);
    assert_eq!(
        doc.model().textures[0].normal_map.as_deref(),
        Some("white_normal")
    );
    let png = doc.texture("white_normal").unwrap();
    let (w, h, rgba) = decode_rgba_png(png).unwrap();
    assert_eq!((w, h), (SIZE, SIZE));
    // 円の左側の法線は左 (R < 128)、右側は右 (R > 128)、上側は上 (G > 128。G が画像の上向き)。
    let px = |x: u32, y: u32| &rgba[((y * w + x) * 4) as usize..][..3];
    assert!(px(5, 16)[0] < 110, "{:?}", px(5, 16));
    assert!(px(26, 16)[0] > 146, "{:?}", px(26, 16));
    assert!(px(16, 5)[1] > 146, "{:?}", px(16, 5));

    // 1 回の Undo で画像ごとライティングも戻る。
    assert!(doc.undo());
    assert_eq!((doc.model().clone(), doc.textures().clone()), before);
}

#[test]
fn stronger_bulge_tilts_normals_more() {
    let doc = disc_document();
    let red_at_left = |strength: f32| {
        let mut doc = Document::with_textures(doc.model().clone(), doc.textures().clone());
        let settings = BulgeSettings {
            strength,
            ..BulgeSettings::default()
        };
        doc.apply(generate_normal_map(doc.model(), doc.textures(), "white", settings).unwrap())
            .unwrap();
        let (w, _, rgba) = decode_rgba_png(doc.texture("white_normal").unwrap()).unwrap();
        rgba[((10 * w + 5) * 4) as usize]
    };
    assert!(red_at_left(1.0) < red_at_left(0.3));
    // 縁の丸み (edge) を指定すると、中は平ら。
    let mut edged = Document::with_textures(doc.model().clone(), doc.textures().clone());
    let settings = BulgeSettings {
        edge: Some(3.0),
        ..BulgeSettings::default()
    };
    edged
        .apply(generate_normal_map(edged.model(), edged.textures(), "white", settings).unwrap())
        .unwrap();
    let (w, _, rgba) = decode_rgba_png(edged.texture("white_normal").unwrap()).unwrap();
    let center = &rgba[((16 * w + 13) * 4) as usize..][..3];
    assert!(
        center[0].abs_diff(128) <= 3 && center[2] > 250,
        "{center:?}"
    );
}

#[test]
fn regenerating_replaces_the_own_image_and_lighting_stays() {
    let mut doc = disc_document();
    doc.apply(Command::SetLighting { enabled: true }).unwrap();
    let cmd = generate_normal_map(
        doc.model(),
        doc.textures(),
        "white",
        BulgeSettings::default(),
    )
    .unwrap();
    // ライティングが有効なら SetLighting を含めない。
    let Command::Batch { commands, .. } = &cmd else {
        panic!("{cmd:?}")
    };
    assert_eq!(commands.len(), 1);
    doc.apply(cmd).unwrap();
    assert_eq!(normal_map_name(doc.model(), "white"), "white_normal");
}

#[test]
fn shared_normal_map_gets_a_new_name() {
    let mut model: Model = fixtures::single_quad();
    let mut other = model.textures[0].clone();
    other.name = "other".into();
    model.textures.push(other);
    model.textures[0].normal_map = Some("white_normal".into());
    model.textures[1].normal_map = Some("white_normal".into());
    assert_eq!(normal_map_name(&model, "white"), "white_normal2");
    model.textures[1].normal_map = None;
    assert_eq!(normal_map_name(&model, "white"), "white_normal");
}

#[test]
fn assigned_normal_map_can_flip_green() {
    let mut doc = disc_document();
    let png = solid_png(4, 4, [128, 200, 230, 255]);
    doc.apply(assign_normal_map(doc.model(), "white", png.clone(), false).unwrap())
        .unwrap();
    assert_eq!(doc.texture("white_normal"), Some(png.as_slice()));
    doc.apply(assign_normal_map(doc.model(), "white", png, true).unwrap())
        .unwrap();
    let (_, _, rgba) = decode_rgba_png(doc.texture("white_normal").unwrap()).unwrap();
    assert_eq!(&rgba[..4], &[128, 55, 230, 255]);

    assert!(matches!(
        assign_normal_map(doc.model(), "white", vec![1, 2, 3], false),
        Err(LightingError::UnreadableNormalMap)
    ));
    assert!(matches!(
        assign_normal_map(doc.model(), "nope", solid_png(1, 1, [0; 4]), false),
        Err(LightingError::TextureNotFound(_))
    ));
}

/// 上限を超える画像は、画素を確保する前にコードつきのエラーで拒む (外から来た PNG でメモリを使い切らない)。
#[test]
fn images_larger_than_the_limit_are_rejected() {
    let doc = disc_document();
    let err = assign_normal_map(doc.model(), "white", huge_png_header(), false).unwrap_err();
    assert!(
        matches!(
            err,
            LightingError::NormalMapTooLarge {
                width: 65535,
                height: 65535,
                max: DEFAULT_MAX_IMAGE_SIDE,
            }
        ),
        "{err}"
    );
    assert_eq!(err.code(), "lighting.normal_map_too_large");

    let mut textures = doc.textures().clone();
    textures.insert("white".into(), Arc::from(huge_png_header()));
    let err =
        generate_normal_map(doc.model(), &textures, "white", BulgeSettings::default()).unwrap_err();
    assert!(
        matches!(&err, LightingError::TextureTooLarge { texture, .. } if texture == "white"),
        "{err}"
    );
    assert_eq!(err.code(), "lighting.texture_too_large");
}

#[test]
fn preview_lighting_is_kept_in_the_project_and_default_is_not_written() {
    let mut meta = ProjectMeta::new("x");
    assert_eq!(meta.preview_lighting(), PreviewLighting::default());
    assert!(
        !meta
            .set_preview_lighting(PreviewLighting::default())
            .unwrap()
    );
    assert!(meta.editor.is_empty());

    let lighting = PreviewLighting {
        enabled: true,
        ambient: [0.2, 0.2, 0.3],
        ambient_intensity: 1.0,
        lights: vec![PreviewLight::Point {
            position: [100.0, 50.0, 200.0],
            color: [1.0, 0.9, 0.8],
            intensity: 1.5,
            range: Some(800.0),
        }],
    };
    assert!(meta.set_preview_lighting(lighting.clone()).unwrap());
    // project.json に書いて読み直しても同じ。
    let json = serde_json::to_string(&meta).unwrap();
    let back: ProjectMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(back.preview_lighting(), lighting);
    // 足りないフィールドは既定値で読む。
    let partial: ProjectMeta = serde_json::from_str(
        r#"{"format_version":1,"name":"x","editor":{"lighting":{"enabled":false}}}"#,
    )
    .unwrap();
    assert!(!partial.preview_lighting().enabled);
    assert_eq!(
        partial.preview_lighting().lights,
        PreviewLighting::default().lights
    );

    assert!(
        meta.set_preview_lighting(PreviewLighting::default())
            .unwrap()
    );
    assert!(meta.editor.is_empty());
}

#[test]
fn invalid_preview_lighting_is_rejected() {
    let mut meta = ProjectMeta::new("x");
    let with = |light| PreviewLighting {
        lights: vec![light],
        ..PreviewLighting::default()
    };
    for bad in [
        with(PreviewLight::Directional {
            direction: [0.0; 3],
            color: [1.0; 3],
            intensity: 1.0,
        }),
        with(PreviewLight::Point {
            position: [0.0; 3],
            color: [1.0; 3],
            intensity: 1.0,
            range: Some(0.0),
        }),
        with(PreviewLight::Point {
            position: [0.0; 3],
            color: [-1.0, 1.0, 1.0],
            intensity: 1.0,
            range: None,
        }),
        PreviewLighting {
            ambient_intensity: f64::NAN,
            ..PreviewLighting::default()
        },
        PreviewLighting {
            lights: vec![PreviewLighting::default().lights[0].clone(); 9],
            ..PreviewLighting::default()
        },
    ] {
        assert!(meta.set_preview_lighting(bad).is_err());
    }
    assert!(meta.editor.is_empty());
}
