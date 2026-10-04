//! PSD の取り込み (`import`) とメッシュの再生成 (`RegenerateMesh`) のテスト。

use std::path::PathBuf;

use ugoe_core::Vec2;
use ugoe_mesher::{GeneratedMesh, MeshError, MeshOptions};
use ugoe_model::validate::validate;
use ugoe_model::{BlendMode, KeyformBinding, KeyformGrid, MaskRef, Mesh, MeshId, Model, fixtures};
use ugoe_psd::{ImportLimits, Layer, LayerNode, LayerTree};

use crate::import::{
    DRAW_ORDER_RANGE, PsdLimit, layer_tree_command, layer_tree_command_with, read_layer_tree,
};
use crate::message::Coded;
use crate::texture::{DEFAULT_MAX_IMAGE_SIDE, decode_rgba_png, huge_png_header, png_size};
use crate::{
    AutoMeshOptions, Command, Document, EditError, Project, ProjectMeta, PsdImport, PsdImportError,
    PsdImportOptions, import_psd,
};

fn fixture_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/layers.psd");
    std::fs::read(path).unwrap()
}

fn empty_document() -> Document {
    Project::empty("Untitled").document()
}

/// `layers.psd` を空のドキュメントに取り込む。
fn imported() -> (Document, PsdImport) {
    let mut doc = empty_document();
    let import = import_psd(
        doc.model(),
        &fixture_bytes(),
        &PsdImportOptions::new("layers"),
    )
    .unwrap();
    doc.apply(import.command.clone()).unwrap();
    (doc, import)
}

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model
        .meshes
        .iter()
        .find(|m| m.id.as_str() == id)
        .unwrap_or_else(|| panic!("mesh {id} not found"))
}

fn part_opacity(model: &Model, id: &str) -> f32 {
    let part = model.parts.iter().find(|p| p.id.as_str() == id).unwrap();
    part.keyforms.keyforms[0].opacity
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn imports_parts_hierarchy_and_canvas() {
    let (doc, import) = imported();
    let model = doc.model();
    assert_eq!(validate(model), vec![]);
    assert_eq!(import.root.as_str(), "layers");
    assert_eq!(
        (model.meta.canvas.width, model.meta.canvas.height),
        (64.0, 48.0)
    );
    assert_eq!(model.meta.canvas.origin, Vec2::new(32.0, 24.0));

    // 日本語だけの名前は PSD のレイヤー ID から ID を作り、名前はそのまま残す。
    let parts: Vec<(&str, &str, Option<&str>)> = model
        .parts
        .iter()
        .map(|p| {
            (
                p.id.as_str(),
                p.name.as_str(),
                p.parent.as_ref().map(|p| p.as_str()),
            )
        })
        .collect();
    assert_eq!(
        parts,
        [
            ("layers", "layers", None),
            ("Group9", "体", Some("layers")),
            ("Group8", "顔", Some("Group9")),
        ]
    );
    assert!(close(part_opacity(model, "Group9"), 230.0 / 255.0));
    assert!(close(part_opacity(model, "Group8"), 1.0));

    let meshes: Vec<(&str, &str, &str)> = model
        .meshes
        .iter()
        .map(|m| (m.id.as_str(), m.name.as_str(), m.part.as_str()))
        .collect();
    assert_eq!(
        meshes,
        [
            ("Layer1", "背景", "layers"),
            ("Layer3", "胴", "Group9"),
            ("Layer5", "肌", "Group8"),
            ("Layer6", "影", "Group8"),
            ("Layer7", "目", "Group8"),
            ("Layer10", "メモ", "layers"),
        ]
    );
}

#[test]
fn imports_textures_and_meshes_at_layer_positions() {
    let (doc, _) = imported();
    let model = doc.model();
    assert_eq!(model.textures.len(), 6);
    assert_eq!(doc.textures().len(), 6);
    // (メッシュ, レイヤーの位置, 大きさ)
    let layers = [
        ("Layer1", (0.0, 0.0), (64, 48)),
        ("Layer3", (10.0, 20.0), (20, 24)),
        ("Layer5", (16.0, 4.0), (32, 20)),
        ("Layer6", (20.0, 8.0), (8, 8)),
        ("Layer7", (22.0, 10.0), (6, 4)),
        ("Layer10", (-4.0, -2.0), (12, 8)),
    ];
    let pad = AutoMeshOptions::default().margin.ceil() + 1.0;
    for (index, (id, (x, y), (w, h))) in layers.into_iter().enumerate() {
        let m = mesh(model, id);
        assert_eq!(m.texture as usize, index, "{id}");
        let texture = &model.textures[index];
        assert_eq!(texture.name, id);
        // テクスチャはレイヤーの画像の周りに透明な余白を足したもの。
        let png = doc.texture(id).unwrap();
        let tw = w + 2 * pad as u32;
        let th = h + 2 * pad as u32;
        assert_eq!(png_size(png), Some((tw, th)), "{id}");
        assert_eq!((texture.width, texture.height), (tw, th));

        let keyform = &m.keyforms.keyforms[0];
        assert_eq!(m.keyforms.keyforms.len(), 1);
        assert!(m.indices.len() >= 3 && m.indices.len().is_multiple_of(3));
        let size = Vec2::new(tw as f32, th as f32);
        let origin = Vec2::new(x - pad, y - pad);
        for (uv, p) in m.uvs.iter().zip(&keyform.positions) {
            assert!(
                (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
                "{id} {uv}"
            );
            // 位置 = テクスチャの左上 + UV × 寸法。
            assert!((origin + *uv * size - *p).length() < 1e-3, "{id} {p}");
            // レイヤーの外接矩形 (+ 膨張の余白) の中にある。
            let margin = AutoMeshOptions::default().margin + 1e-3;
            assert!(
                p.x >= x - margin && p.x <= x + w as f32 + margin,
                "{id} {p}"
            );
            assert!(
                p.y >= y - margin && p.y <= y + h as f32 + margin,
                "{id} {p}"
            );
        }
    }
}

#[test]
fn texture_keeps_layer_pixels() {
    let (doc, _) = imported();
    let tree = ugoe_psd::import(&fixture_bytes()).unwrap();
    let LayerNode::Layer(bg) = &tree.root[0] else {
        panic!("expected the background layer");
    };
    let (w, h, rgba) = decode_rgba_png(doc.texture("Layer1").unwrap()).unwrap();
    let pad = (w - bg.width) / 2;
    assert_eq!(h, bg.height + 2 * pad);
    for y in 0..bg.height as usize {
        let src = &bg.rgba[y * bg.width as usize * 4..][..bg.width as usize * 4];
        let start = ((y + pad as usize) * w as usize + pad as usize) * 4;
        assert_eq!(&rgba[start..start + src.len()], src);
    }
    // 余白は透明。
    assert!(rgba[..w as usize * 4].iter().all(|&b| b == 0));
}

#[test]
fn draw_order_increases_bottom_to_top() {
    let (doc, _) = imported();
    let orders: Vec<f32> = doc
        .model()
        .meshes
        .iter()
        .map(|m| m.keyforms.keyforms[0].draw_order)
        .collect();
    let n = orders.len() as f32;
    for (i, order) in orders.iter().enumerate() {
        assert!(close(
            *order,
            DRAW_ORDER_RANGE * (i as f32 + 1.0) / (n + 1.0)
        ));
    }
    assert!(orders.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn maps_opacity_visibility_blend_and_clipping() {
    let (doc, _) = imported();
    let model = doc.model();
    let opacity = |id: &str| mesh(model, id).keyforms.keyforms[0].opacity;
    assert!(close(opacity("Layer1"), 1.0));
    // 影: 塗りの不透明度 80% (204/255)、乗算、肌でクリッピング。
    assert!(close(opacity("Layer6"), 204.0 / 255.0));
    assert_eq!(mesh(model, "Layer6").blend, BlendMode::Multiply);
    assert_eq!(mesh(model, "Layer6").masks, vec![MaskRef::normal("Layer5")]);
    // 非表示の目は不透明度 0 で取り込む。
    assert!(close(opacity("Layer7"), 0.0));
    assert!(mesh(model, "Layer7").masks.is_empty());
    // メモ: 不透明度 50%。
    assert!(close(opacity("Layer10"), 128.0 / 255.0));
    for id in ["Layer1", "Layer3", "Layer5", "Layer7", "Layer10"] {
        assert_eq!(mesh(model, id).blend, BlendMode::Normal, "{id}");
    }
}

#[test]
fn reports_warnings() {
    let (_, import) = imported();
    let messages: Vec<String> = import.warnings.iter().map(ToString::to_string).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("レベル補正: adjustment")),
        "{messages:?}"
    );
    let adjustment = &import.warnings[0];
    assert_eq!(adjustment.code, "psd.adjustment_layer");
    assert_eq!(
        adjustment.args["key"],
        crate::MessageArg::Text("levl".into())
    );
    // レイヤー効果の警告は `ugoe-psd` のテストで試す (`layers.psd` は仕様どおりに書けない偽の効果を持たない)。
    assert_eq!(messages.len(), 1, "{messages:?}");
}

/// 確認用の `character.psd` (T0620) も警告なしで取り込め、クリッピング・非表示・不透明度が写る。
#[test]
fn imports_character_fixture() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/character.psd");
    let mut doc = empty_document();
    let import = import_psd(
        doc.model(),
        &std::fs::read(path).unwrap(),
        &PsdImportOptions::new("character"),
    )
    .unwrap();
    assert!(import.warnings.is_empty(), "{:?}", import.warnings);
    doc.apply(import.command).unwrap();
    let model = doc.model();
    assert_eq!(validate(model), vec![]);
    let names: Vec<&str> = model.parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["character", "体", "頭", "目"]);
    assert_eq!(model.meshes.len(), 12);
    let by_name = |name: &str| model.meshes.iter().find(|m| m.name == name).unwrap();
    // 顔の影は顔でクリッピングし、乗算。頬は不透明度 50%。口 (開き) は非表示。
    let face = by_name("顔").id.clone();
    assert_eq!(
        by_name("顔の影").masks,
        vec![MaskRef::normal(face.as_str())]
    );
    assert_eq!(by_name("顔の影").blend, BlendMode::Multiply);
    assert!(close(
        by_name("頬").keyforms.keyforms[0].opacity,
        128.0 / 255.0
    ));
    assert!(close(
        by_name("口 (開き)").keyforms.keyforms[0].opacity,
        0.0
    ));
}

#[test]
fn undo_removes_everything() {
    let (mut doc, _) = imported();
    assert!(doc.is_modified());
    assert!(doc.undo());
    assert_eq!(doc.model(), empty_document().model());
    assert!(doc.textures().is_empty());
    assert!(!doc.is_modified());
    assert!(doc.redo());
    assert_eq!(doc.model().meshes.len(), 6);
}

#[test]
fn project_round_trip_keeps_textures() {
    let (doc, _) = imported();
    let dir = std::env::temp_dir().join(format!(
        "ugoe-editor-core-{}-psd-import.ugoeproj",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let project = Project::from_document(ProjectMeta::new("layers"), &doc);
    project.save(&dir).unwrap();
    let loaded = Project::load(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let reopened = loaded.document();
    assert_eq!(reopened.model(), doc.model());
    assert_eq!(reopened.textures(), doc.textures());
    crate::export_bundle(&loaded).unwrap();
}

#[test]
fn import_into_existing_model_uses_new_top_level_part() {
    let mut doc = Document::new(fixtures::single_quad());
    let options = PsdImportOptions::new("layers");
    let first = import_psd(doc.model(), &fixture_bytes(), &options).unwrap();
    // キャンバスは変えず、大きさの違いを警告する。
    assert!(first.warnings.iter().any(|w| w.path.is_empty()
        && w.message.starts_with(
            "the PSD canvas (64x48) differs from the model canvas (256x256); \
                     the model canvas is kept"
        )));
    let canvas = first
        .warnings
        .iter()
        .find(|w| w.code == "psd.canvas_mismatch")
        .unwrap();
    assert_eq!(
        canvas.args,
        crate::message_args! {
            "psd_width" => 64u32,
            "psd_height" => 48u32,
            "model_width" => 256.0f32,
            "model_height" => 256.0f32,
        }
    );
    doc.apply(first.command).unwrap();
    // 同じ PSD をもう一度取り込むと、ID とテクスチャ名が重ならないように番号を付ける。
    let second = import_psd(doc.model(), &fixture_bytes(), &options).unwrap();
    assert_eq!(second.root.as_str(), "layers_2");
    doc.apply(second.command).unwrap();

    let model = doc.model();
    assert_eq!(model.meta.canvas.width, 256.0);
    assert_eq!(model.meshes.len(), 1 + 12);
    assert_eq!(model.textures.len(), 1 + 12);
    let part = model
        .parts
        .iter()
        .find(|p| p.id.as_str() == "layers_2")
        .unwrap();
    assert_eq!(part.parent, None);
    let copy = mesh(model, "Layer6_2");
    assert_eq!(copy.masks, vec![MaskRef::normal("Layer5_2")]);
    assert_eq!(model.textures[copy.texture as usize].name, "Layer6_2");
    assert_eq!(validate(model), vec![]);
}

fn raster(name: &str, id: u32, clipping: bool, width: u32, height: u32) -> LayerNode {
    LayerNode::Layer(Layer {
        name: name.into(),
        visible: true,
        opacity: 0.5,
        offset: Vec2::new(4.0, 4.0),
        width,
        height,
        rgba: vec![255; (width * height * 4) as usize],
        blend_mode: "norm".into(),
        id: Some(id),
        fill_opacity: 1.0,
        clipping,
    })
}

fn tree(root: Vec<LayerNode>) -> LayerTree {
    LayerTree {
        width: 32,
        height: 32,
        root,
        warnings: vec![],
    }
}

#[test]
fn clipping_onto_group_and_skipped_base() {
    let group = LayerNode::Group {
        name: "Arm".into(),
        visible: true,
        children: vec![
            raster("Upper", 1, false, 4, 4),
            raster("Lower", 2, false, 4, 4),
        ],
        id: Some(3),
        opacity: 1.0,
        blend_mode: "pass".into(),
        clipping: false,
    };
    let mut scrn = raster("Glow", 7, false, 4, 4);
    if let LayerNode::Layer(l) = &mut scrn {
        l.blend_mode = "scrn".into();
    }
    let tree = tree(vec![
        // 下に何もないクリッピングは無視する (Photoshop と同じ)。
        raster("First", 8, true, 4, 4),
        group,
        raster("Tattoo", 4, true, 4, 4),
        raster("Empty", 5, false, 0, 0),
        raster("Orphan", 6, true, 4, 4),
        scrn,
    ]);
    let model = Project::empty("x").document().model().clone();
    let import = layer_tree_command(&model, &tree, &PsdImportOptions::new("t")).unwrap();
    let mut doc = Document::new(model);
    doc.apply(import.command).unwrap();
    let model = doc.model();

    assert!(mesh(model, "First").masks.is_empty());
    // グループが土台なら、グループの中のすべてのメッシュでマスクする。
    assert_eq!(
        mesh(model, "Tattoo").masks,
        vec![MaskRef::normal("Upper"), MaskRef::normal("Lower")]
    );
    // 土台のレイヤーの不透明度はクリッピングされた側に掛かる (グループなら 1)。
    assert!(close(
        mesh(model, "Tattoo").keyforms.keyforms[0].opacity,
        0.5
    ));
    assert!(mesh(model, "Orphan").masks.is_empty());
    assert!(model.meshes.iter().all(|m| m.id.as_str() != "Empty"));
    assert_eq!(mesh(model, "Glow").blend, BlendMode::Normal);

    let messages: Vec<String> = import.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        messages,
        [
            "Empty: empty layer skipped",
            "Orphan: clipping base was not imported, imported unclipped",
            "Glow: blend mode `scrn` is not supported, using normal",
        ]
    );
}

#[test]
fn clipped_layer_over_hidden_base_is_hidden() {
    let mut base = raster("Base", 1, false, 4, 4);
    if let LayerNode::Layer(l) = &mut base {
        l.visible = false;
    }
    let tree = tree(vec![base, raster("Clip", 2, true, 4, 4)]);
    let model = Project::empty("x").document().model().clone();
    let import = layer_tree_command(&model, &tree, &PsdImportOptions::new("t")).unwrap();
    let mut doc = Document::new(model);
    doc.apply(import.command).unwrap();
    let clip = mesh(doc.model(), "Clip");
    assert_eq!(clip.masks, vec![MaskRef::normal("Base")]);
    assert!(close(clip.keyforms.keyforms[0].opacity, 0.0));
}

#[test]
fn rejects_invalid_mesh_options() {
    let model = Project::empty("x").document().model().clone();
    let mut options = PsdImportOptions::new("t");
    options.mesh.edge_spacing = 0.0;
    let err = import_psd(&model, &fixture_bytes(), &options).unwrap_err();
    assert!(
        matches!(err, PsdImportError::InvalidMeshOptions(_)),
        "{err}"
    );
    // `margin` の上限 (1024) は mesher と同じ検証で、レイヤーごとでなく最初に弾く。
    let mut options = PsdImportOptions::new("t");
    options.mesh.margin = 1025.0;
    let err = import_psd(&model, &fixture_bytes(), &options).unwrap_err();
    assert!(
        matches!(err, PsdImportError::InvalidMeshOptions(_)),
        "{err}"
    );
    let err = import_psd(&model, b"not a psd", &PsdImportOptions::new("t")).unwrap_err();
    assert!(matches!(err, PsdImportError::Psd(_)), "{err}");
}

#[test]
fn limit_exceeded_tells_which_limit() {
    // `layers.psd` (64 x 48) のチャンネル 1 枚は 3072 バイト。
    let limits = ImportLimits::default().with_max_channel_decompressed(100);
    let err = read_layer_tree(&fixture_bytes(), limits).unwrap_err();
    assert!(
        matches!(
            err,
            PsdImportError::LimitExceeded {
                limit: PsdLimit::Channel,
                max: 100
            }
        ),
        "{err:?}"
    );
    let message = err.to_message();
    assert_eq!(message.code, "psd.channel_limit_exceeded");
    assert_eq!(message.args["max"], 100u64.into());
    assert!(
        message.message.contains("import limit of 100 bytes"),
        "{}",
        message.message
    );

    let limits = ImportLimits::default().with_max_total_decompressed(5000);
    let err = read_layer_tree(&fixture_bytes(), limits).unwrap_err();
    assert!(
        matches!(
            err,
            PsdImportError::LimitExceeded {
                limit: PsdLimit::Total,
                max: 5000
            }
        ),
        "{err:?}"
    );
    let message = err.to_message();
    assert_eq!(message.code, "psd.total_limit_exceeded");
    assert_eq!(message.args["max"], 5000u64.into());

    // ほかの読み込みのエラーは `psd.read_failed` のまま。
    let err = read_layer_tree(b"not a psd", ImportLimits::default()).unwrap_err();
    assert_eq!(err.code(), "psd.read_failed");
    // 既定の上限では読める。
    read_layer_tree(&fixture_bytes(), ImportLimits::default()).unwrap();
}

/// 幅 8 のレイヤーだけ `TooLarge` を返し、ほかは本物の自動メッシュを使う。
fn too_large_for_width_8(
    rgba: &[u8],
    width: u32,
    height: u32,
    options: &MeshOptions,
) -> Result<GeneratedMesh, MeshError> {
    let pad = options.margin.ceil() as u32 + 1;
    if width == 8 + 2 * pad {
        return Err(MeshError::TooLarge {
            width: 20000,
            height: 30000,
        });
    }
    ugoe_mesher::generate(rgba, width, height, options)
}

#[test]
fn too_large_layer_is_skipped_with_a_warning() {
    let tree = tree(vec![
        raster("Below", 1, false, 4, 4),
        raster("Huge", 2, false, 8, 4),
        raster("Above", 3, false, 4, 4),
    ]);
    let model = Project::empty("x").document().model().clone();
    let import = layer_tree_command_with(
        &model,
        &tree,
        &PsdImportOptions::new("t"),
        too_large_for_width_8,
    )
    .unwrap();
    let warning = import
        .warnings
        .iter()
        .find(|w| w.code == "psd.layer_too_large")
        .expect("a warning for the huge layer");
    assert_eq!(warning.path, vec!["Huge".to_owned()]);
    assert_eq!(warning.args["width"], 20000u64.into());
    assert_eq!(warning.args["height"], 30000u64.into());
    assert_eq!(warning.args["max"], ugoe_mesher::MAX_GRID_CELLS.into());
    assert!(warning.message.contains("skipped"), "{}", warning.message);

    let mut doc = Document::new(model);
    doc.apply(import.command).unwrap();
    let ids: Vec<&str> = doc.model().meshes.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["Below", "Above"]);
}

#[test]
fn regenerate_mesh_uses_texture_and_keeps_placement() {
    let (mut doc, _) = imported();
    let before = mesh(doc.model(), "Layer1").clone();
    let options = AutoMeshOptions {
        edge_spacing: 8.0,
        interior_spacing: 8.0,
        ..AutoMeshOptions::default()
    };
    doc.apply(Command::RegenerateMesh {
        id: "Layer1".into(),
        options,
    })
    .unwrap();
    let after = mesh(doc.model(), "Layer1");
    assert!(after.uvs.len() > before.uvs.len());
    assert_eq!(after.keyforms.keyforms[0].positions.len(), after.uvs.len());
    // 頂点以外の値は変わらない。
    assert_eq!(
        after.keyforms.keyforms[0].opacity,
        before.keyforms.keyforms[0].opacity
    );
    assert_eq!(
        after.keyforms.keyforms[0].draw_order,
        before.keyforms.keyforms[0].draw_order
    );
    // 位置は取り込み時と同じ写像 (テクスチャの左上 + UV × 寸法) のまま。
    let pad = AutoMeshOptions::default().margin.ceil() + 1.0;
    let size = Vec2::new(64.0 + 2.0 * pad, 48.0 + 2.0 * pad);
    for (uv, p) in after.uvs.iter().zip(&after.keyforms.keyforms[0].positions) {
        assert!((Vec2::splat(-pad) + *uv * size - *p).length() < 1e-3, "{p}");
    }
    // 1 回の Undo で元に戻る。
    assert!(doc.undo());
    assert_eq!(mesh(doc.model(), "Layer1"), &before);
}

/// テクスチャの画像がデコードの上限を超えるなら、画素を確保せずにコードつきのエラーで拒む。
#[test]
fn regenerate_mesh_rejects_textures_larger_than_the_limit() {
    let (mut doc, _) = imported();
    doc.apply(Command::SetTextureImage {
        name: "Layer1".into(),
        png: huge_png_header(),
    })
    .unwrap();
    let before = mesh(doc.model(), "Layer1").clone();
    let err = doc
        .apply(Command::RegenerateMesh {
            id: "Layer1".into(),
            options: AutoMeshOptions::default(),
        })
        .unwrap_err();
    assert_eq!(
        err,
        EditError::TextureTooLarge {
            texture: "Layer1".into(),
            width: 65535,
            height: 65535,
            max: DEFAULT_MAX_IMAGE_SIDE,
        }
    );
    assert_eq!(err.to_message().code, "edit.texture_too_large");
    assert_eq!(mesh(doc.model(), "Layer1"), &before);
}

#[test]
fn regenerate_mesh_keeps_keyforms_with_equal_positions() {
    let (mut doc, _) = imported();
    let mut m = mesh(doc.model(), "Layer3").clone();
    let mut faded = m.keyforms.keyforms[0].clone();
    faded.opacity = 0.25;
    m.keyforms = KeyformGrid::new(
        vec![KeyformBinding {
            parameter: "Fade".into(),
            keys: vec![0.0, 1.0],
        }],
        vec![m.keyforms.keyforms[0].clone(), faded],
    );
    let param = fixtures::single_quad().parameters[0].clone();
    let param = ugoe_model::Parameter {
        id: "Fade".into(),
        ..param
    };
    doc.apply(Command::AddParameter { parameter: param })
        .unwrap();
    doc.apply(Command::SetMeshKeyforms {
        id: m.id.clone(),
        keyforms: m.keyforms.clone(),
    })
    .unwrap();
    doc.apply(Command::RegenerateMesh {
        id: m.id.clone(),
        options: AutoMeshOptions::default(),
    })
    .unwrap();
    let after = mesh(doc.model(), "Layer3");
    assert_eq!(after.keyforms.keyforms.len(), 2);
    assert_eq!(after.keyforms.keyforms[1].opacity, 0.25);
    assert_eq!(
        after.keyforms.keyforms[0].positions,
        after.keyforms.keyforms[1].positions
    );
}

#[test]
fn regenerate_mesh_rejects_deformed_keyforms() {
    let (mut doc, _) = imported();
    let mut keyform = mesh(doc.model(), "Layer3").keyforms.keyforms[0].clone();
    keyform.positions[0].x += 5.0;
    doc.apply(Command::SetMeshKeyform {
        id: "Layer3".into(),
        cell: Default::default(),
        keyform,
    })
    .unwrap();
    // 単一のキーフォームでも、UV のアフィン写像でなくなっていれば拒否する。
    let err = doc
        .apply(Command::RegenerateMesh {
            id: "Layer3".into(),
            options: AutoMeshOptions::default(),
        })
        .unwrap_err();
    assert!(
        matches!(err, EditError::CannotRegenerateMesh { .. }),
        "{err}"
    );

    let err = doc
        .apply(Command::RegenerateMesh {
            id: "Nope".into(),
            options: AutoMeshOptions::default(),
        })
        .unwrap_err();
    assert!(matches!(err, EditError::NotFound { .. }), "{err}");

    let err = doc
        .apply(Command::RegenerateMesh {
            id: "Layer1".into(),
            options: AutoMeshOptions {
                edge_spacing: -1.0,
                ..AutoMeshOptions::default()
            },
        })
        .unwrap_err();
    assert!(matches!(err, EditError::Mesher { .. }), "{err}");
}

#[test]
fn regenerate_mesh_rejects_different_keyform_positions() {
    let (mut doc, _) = imported();
    let base = mesh(doc.model(), "Layer5").keyforms.keyforms[0].clone();
    let mut moved = base.clone();
    for p in &mut moved.positions {
        *p += Vec2::new(3.0, 0.0);
    }
    let param = ugoe_model::Parameter {
        id: "Move".into(),
        ..fixtures::single_quad().parameters[0].clone()
    };
    doc.apply(Command::AddParameter { parameter: param })
        .unwrap();
    doc.apply(Command::SetMeshKeyforms {
        id: MeshId::new("Layer5".to_owned()),
        keyforms: KeyformGrid::new(
            vec![KeyformBinding {
                parameter: "Move".into(),
                keys: vec![0.0, 1.0],
            }],
            vec![base, moved],
        ),
    })
    .unwrap();
    let err = doc
        .apply(Command::RegenerateMesh {
            id: "Layer5".into(),
            options: AutoMeshOptions::default(),
        })
        .unwrap_err();
    let EditError::CannotRegenerateMesh { reason, .. } = err else {
        panic!("{err}");
    };
    assert!(reason.contains("different vertex positions"), "{reason}");
}

#[test]
fn regenerate_mesh_command_json() {
    let command: Command = serde_json::from_str(r#"{"op":"regenerate_mesh","id":"Hair"}"#).unwrap();
    assert_eq!(
        command,
        Command::RegenerateMesh {
            id: "Hair".into(),
            options: AutoMeshOptions::default(),
        }
    );
    let command: Command = serde_json::from_str(
        r#"{"op":"regenerate_mesh","id":"Hair","options":{"edge_spacing":12}}"#,
    )
    .unwrap();
    let Command::RegenerateMesh { options, .. } = command else {
        panic!();
    };
    assert_eq!(options.edge_spacing, 12.0);
    assert_eq!(options.margin, AutoMeshOptions::default().margin);

    let command: Command =
        serde_json::from_str(r#"{"op":"set_canvas","width":64,"height":48,"origin":[32,24]}"#)
            .unwrap();
    let mut doc = empty_document();
    doc.apply(command).unwrap();
    assert_eq!(doc.model().meta.canvas.width, 64.0);
    assert!(matches!(
        doc.apply(Command::SetCanvas {
            width: 0.0,
            height: 1.0,
            origin: Vec2::ZERO
        }),
        Err(EditError::InvalidCanvas { .. })
    ));
}
