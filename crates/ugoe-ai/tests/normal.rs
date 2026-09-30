//! 法線マップの生成の提案 (`ugoe_ai::normal`) のテスト。ネットワークは使わない。
//!
//! 決定的な方法は実際の PSD (`assets/fixtures/psd-real/`) を取り込んだモデルで、深度推定からの方法は
//! キャンバス全体を覆う 1 枚の四角形と、決めた深度マップ ([`FixedDepthEstimator`]) で確かめる。

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use ugoe_ai::classify::{Label, label_meshes_by_name};
use ugoe_ai::depth::{DepthMap, DepthRequest, FixedDepthEstimator};
use ugoe_ai::normal::{
    BulgeNormalMapOptions, DepthNormalOptions, NormalMapOptions, SurfaceGroup, normal_map_group,
    normal_map_groups, plan_depth_normal_maps, propose_bulge_normal_maps,
    propose_depth_normal_maps,
};
use ugoe_ai::{AiError, Proposal};
use ugoe_core::Vec2;
use ugoe_editor_core::texture::{Textures, decode_rgba_png, encode_rgba_png};
use ugoe_editor_core::{Command, Document, Project, PsdImportOptions, import_psd};
use ugoe_model::{Model, TextureRef, fixtures};
use ugoe_rigging::{BulgeOptions, decode_normal};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// 胸から上の PSD を取り込んだドキュメント (テクスチャの画像付き)。
fn bustup() -> Document {
    let name = "comfyui-see-through-bustup-parts.psd";
    let bytes = std::fs::read(repo_path("../../assets/fixtures/psd-real").join(name)).unwrap();
    let mut doc = Document::new(Project::empty(name).model);
    let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(name)).unwrap();
    doc.apply(import.command).unwrap();
    doc
}

fn apply(doc: &mut Document, proposal: &Proposal) {
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands.clone(),
    })
    .unwrap();
    assert!(ugoe_model::validate::validate(doc.model()).is_empty());
}

/// 法線マップの画像 (デコードしたもの) と、その画素の法線。
struct Decoded {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Decoded {
    fn of(doc: &Document, texture: &str) -> Self {
        let t = doc
            .model()
            .textures
            .iter()
            .find(|t| t.name == texture)
            .unwrap();
        let name = t.normal_map.as_deref().expect("has a normal map");
        let (width, height, rgba) = decode_rgba_png(doc.texture(name).unwrap()).unwrap();
        Self {
            width,
            height,
            rgba,
        }
    }

    fn normal(&self, x: u32, y: u32) -> [f32; 3] {
        let i = (y * self.width + x) as usize * 4;
        decode_normal([self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]])
    }

    fn is_flat(&self) -> bool {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [128, 128, 255])
    }
}

#[test]
fn bulge_proposal_gives_every_part_a_normal_map() {
    let mut doc = bustup();
    let before = doc.model().clone();
    let proposal = propose_bulge_normal_maps(
        doc.model(),
        doc.textures(),
        &BTreeMap::new(),
        &BulgeNormalMapOptions::default(),
    )
    .unwrap();
    apply(&mut doc, &proposal);
    let model = doc.model();
    assert!(model.lighting, "lighting is turned on");
    for texture in &model.textures {
        if texture.name.ends_with("_normal") {
            continue;
        }
        let name = texture.normal_map.as_deref().unwrap_or_else(|| {
            panic!("`{}` has no normal map", texture.name);
        });
        assert_eq!(name, format!("{}_normal", texture.name));
        let map = Decoded::of(&doc, &texture.name);
        assert_eq!((map.width, map.height), (texture.width, texture.height));
        assert!(!map.is_flat(), "`{}` bulges", texture.name);
    }
    // 1 回の Undo で画像ごと戻る。
    assert!(doc.undo());
    assert_eq!(doc.model(), &before);
    assert!(doc.textures().keys().all(|k| !k.ends_with("_normal")));
}

#[test]
fn strength_can_be_set_per_part() {
    let doc = bustup();
    let first = doc.model().textures[0].name.clone();
    let second = doc.model().textures[1].name.clone();
    let options = BulgeNormalMapOptions {
        per_texture: [(
            first.clone(),
            BulgeOptions {
                strength: 0.0,
                ..Default::default()
            },
        )]
        .into(),
        textures: vec![first.clone(), second.clone()],
        ..Default::default()
    };
    let proposal =
        propose_bulge_normal_maps(doc.model(), doc.textures(), &BTreeMap::new(), &options).unwrap();
    let mut doc = doc;
    apply(&mut doc, &proposal);
    assert!(Decoded::of(&doc, &first).is_flat());
    assert!(!Decoded::of(&doc, &second).is_flat());
    // 選ばなかったテクスチャには付けない。
    assert!(doc.model().textures[2].normal_map.is_none());
}

#[test]
fn existing_normal_maps_are_kept_or_replaced() {
    let mut doc = bustup();
    let texture = doc.model().textures[0].name.clone();
    let only = |replace_existing| BulgeNormalMapOptions {
        textures: vec![texture.clone()],
        replace_existing,
        ..Default::default()
    };
    let proposal =
        propose_bulge_normal_maps(doc.model(), doc.textures(), &BTreeMap::new(), &only(false))
            .unwrap();
    apply(&mut doc, &proposal);
    let first = doc.texture(&format!("{texture}_normal")).unwrap().to_vec();

    // 作り直さない設定なら、作るものがない。
    assert!(matches!(
        propose_bulge_normal_maps(doc.model(), doc.textures(), &BTreeMap::new(), &only(false)),
        Err(AiError::InvalidRequest(_))
    ));
    // 作り直すなら、同じ名前の画像を差し替える (ライティングはすでに有効なので触らない)。
    let options = BulgeNormalMapOptions {
        default: BulgeOptions {
            strength: 2.0,
            ..Default::default()
        },
        ..only(true)
    };
    let proposal =
        propose_bulge_normal_maps(doc.model(), doc.textures(), &BTreeMap::new(), &options).unwrap();
    assert_eq!(proposal.commands.len(), 1);
    apply(&mut doc, &proposal);
    let model = doc.model();
    assert_eq!(
        model.textures[0].normal_map.as_deref(),
        Some(format!("{texture}_normal").as_str())
    );
    assert_ne!(
        doc.texture(&format!("{texture}_normal")).unwrap(),
        &first[..]
    );
}

#[test]
fn unknown_textures_are_rejected() {
    let doc = bustup();
    let options = BulgeNormalMapOptions {
        textures: vec!["nope".into()],
        ..Default::default()
    };
    assert!(matches!(
        propose_bulge_normal_maps(doc.model(), doc.textures(), &BTreeMap::new(), &options),
        Err(AiError::InvalidRequest(_))
    ));
}

/// メッシュ `mesh` の、キャンバスの点 `p` での法線 (取り込んだままのメッシュはテクスチャを等倍で置いている)。
fn normal_on_canvas(doc: &Document, mesh: &str, p: Vec2) -> [f32; 3] {
    let model = doc.model();
    let m = model.meshes.iter().find(|m| m.name == mesh).unwrap();
    let t = &model.textures[m.texture as usize];
    let size = Vec2::new(t.width as f32, t.height as f32);
    let origin = m.keyforms.keyforms[0].positions[0] - m.uvs[0] * size;
    let texel = p - origin;
    Decoded::of(doc, &t.name).normal(texel.x as u32, texel.y as u32)
}

fn angle(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

/// メッシュの形の中心 (キャンバス)。
fn mesh_center(doc: &Document, mesh: &str) -> Vec2 {
    let m = doc.model().meshes.iter().find(|m| m.name == mesh).unwrap();
    let ps = &m.keyforms.keyforms[0].positions;
    let (lo, hi) = ps.iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    (lo + hi) * 0.5
}

/// 名前から付けたラベルでまとまりを決めると、目・鼻・口は顔の面の向きを受け、首・腕は体と境目でつながる (T0518)。
#[test]
fn labelled_parts_bulge_as_groups() {
    let mut doc = bustup();
    let labels = label_meshes_by_name(doc.model());
    let proposal = propose_bulge_normal_maps(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("`face`"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    for mesh in [
        "eyewhite-l",
        "eyewhite-r",
        "nose",
        "irides-l",
        "mouth-close",
    ] {
        let p = mesh_center(&doc, mesh);
        let (a, b) = (
            normal_on_canvas(&doc, mesh, p),
            normal_on_canvas(&doc, "face", p),
        );
        assert!(angle(a, b) < 3.0, "{mesh}: {a:?} vs face {b:?}");
    }
    for (a, b, p) in [
        ("neck", "topwear", Vec2::new(512.0, 400.0)),
        ("handwear-l", "topwear", Vec2::new(690.0, 520.0)),
        ("handwear-r", "topwear", Vec2::new(335.0, 520.0)),
    ] {
        let (na, nb) = (normal_on_canvas(&doc, a, p), normal_on_canvas(&doc, b, p));
        assert!(angle(na, nb) < 3.0, "{a} {na:?} vs {b} {nb:?} at {p:?}");
    }
    // 顔の中ほどは正面を向く。
    assert!(normal_on_canvas(&doc, "face", mesh_center(&doc, "nose"))[2] > 0.97);
}

/// まとまりの一部だけを作るときも、形はまとまり全体で決まる (エディタで 1 枚ずつ作っても同じ向き)。
#[test]
fn one_texture_of_a_group_uses_the_whole_shape() {
    let doc = bustup();
    let labels = label_meshes_by_name(doc.model());
    let eye = doc
        .model()
        .meshes
        .iter()
        .find(|m| m.name == "eyewhite-l")
        .unwrap();
    let eye_texture = doc.model().textures[eye.texture as usize].name.clone();
    let group = normal_map_group(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
        &eye_texture,
    )
    .unwrap();
    assert_eq!(group.group, Some(SurfaceGroup::Face));
    assert_eq!(group.textures.len(), 14, "{:?}", group.textures);
    // 縁の幅は顔のまとまりの形の幅 (約 230 画素) の 0.21 (見本のアバターと同じ 48 画素)。
    let radius = group.options.radius.unwrap();
    assert!((radius - 48.0).abs() < 3.0, "{radius}");

    let options = BulgeNormalMapOptions {
        textures: vec![eye_texture.clone()],
        ..Default::default()
    };
    let one = propose_bulge_normal_maps(doc.model(), doc.textures(), &labels, &options).unwrap();
    assert_eq!(one.commands.len(), 2, "the eye's normal map and lighting");
    let all = propose_bulge_normal_maps(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
    )
    .unwrap();
    let png = |p: &Proposal| {
        p.commands
            .iter()
            .find_map(|c| match c {
                Command::SetNormalMap { texture, png, .. } if *texture == eye_texture => {
                    png.clone()
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(png(&one), png(&all));

    // 首は体のまとまりで、顔を形に加える。
    let neck = doc
        .model()
        .meshes
        .iter()
        .find(|m| m.name == "neck")
        .unwrap();
    let neck_texture = &doc.model().textures[neck.texture as usize].name;
    let group = normal_map_group(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
        neck_texture,
    )
    .unwrap();
    assert_eq!(group.group, Some(SurfaceGroup::Body));
    assert_eq!(group.shape_only.len(), 1);
    assert!(group.options.radius.is_none());
}

/// 眼鏡のラベルのテクスチャは顔のまとまりに入り、形に加えず顔の面の向きだけを受ける (T0520)。
#[test]
fn eyewear_takes_the_face_orientation_only() {
    let mut doc = bustup();
    let mut labels = label_meshes_by_name(doc.model());
    // 目の白目を眼鏡に見立てる (顔の上にあり、顔の形の中)。
    let eye = doc
        .model()
        .meshes
        .iter()
        .find(|m| m.name == "eyewhite-l")
        .unwrap();
    let eye_texture = doc.model().textures[eye.texture as usize].name.clone();
    labels.insert(eye.id.clone(), Label::Eyewear);
    let group = normal_map_group(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
        &eye_texture,
    )
    .unwrap();
    assert_eq!(group.group, Some(SurfaceGroup::Face));
    assert_eq!(group.oriented, vec![eye_texture.clone()]);
    assert!(group.textures.contains(&eye_texture));
    assert_eq!(group.textures.len(), 14, "{:?}", group.textures);

    let proposal = propose_bulge_normal_maps(
        doc.model(),
        doc.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("向きだけを受けたもの"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let p = mesh_center(&doc, "eyewhite-l");
    let (a, b) = (
        normal_on_canvas(&doc, "eyewhite-l", p),
        normal_on_canvas(&doc, "face", p),
    );
    assert!(angle(a, b) < 3.0, "eyewear {a:?} vs face {b:?}");
}

/// まとまりの一覧: まとまりごとに 1 行、単独のものは最後に 1 行。すべての対象のテクスチャが 1 回ずつ入る (T0520)。
#[test]
fn groups_list_every_target_once() {
    let doc = bustup();
    let labels = label_meshes_by_name(doc.model());
    let options = BulgeNormalMapOptions::default();
    let groups = normal_map_groups(doc.model(), doc.textures(), &labels, &options).unwrap();
    let names: Vec<_> = groups.iter().map(|g| g.group).collect();
    assert_eq!(names.first(), Some(&Some(SurfaceGroup::Face)));
    assert_eq!(
        names
            .iter()
            .filter(|g| **g == Some(SurfaceGroup::Face))
            .count(),
        1
    );
    let mut all: Vec<String> = groups.iter().flat_map(|g| g.textures.clone()).collect();
    let count = all.len();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), count, "no texture is listed twice");
    let proposal =
        propose_bulge_normal_maps(doc.model(), doc.textures(), &labels, &options).unwrap();
    let made = proposal
        .commands
        .iter()
        .filter(|c| matches!(c, Command::SetNormalMap { .. }))
        .count();
    assert_eq!(count, made);
    // それぞれのまとまりの設定は、1 枚ずつ尋ねたときと同じ。
    for g in groups.iter().filter(|g| g.group.is_some()) {
        let one = normal_map_group(
            doc.model(),
            doc.textures(),
            &labels,
            &options,
            &g.textures[0],
        )
        .unwrap();
        assert_eq!(one.options, g.options);
    }
    // ラベルがなければ、すべて単独の 1 行。
    let alone = normal_map_groups(doc.model(), doc.textures(), &BTreeMap::new(), &options).unwrap();
    assert_eq!(alone.len(), 1);
    assert_eq!(alone[0].group, None);
    assert_eq!(alone[0].textures.len(), count);
}

// --- 深度推定から ---

/// キャンバス 256×256 を覆う四角形 1 枚に、一辺 `size` の不透明なテクスチャ `skin` を貼ったモデル。
/// `mirror` なら UV を左右反転する。
fn canvas_quad(size: u32, mirror: bool) -> (Model, Textures) {
    let mut model = fixtures::single_quad();
    model.meta.canvas.width = 256.0;
    model.meta.canvas.height = 256.0;
    let corners = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(256.0, 0.0),
        Vec2::new(256.0, 256.0),
        Vec2::new(0.0, 256.0),
    ];
    let mesh = &mut model.meshes[0];
    for k in &mut mesh.keyforms.keyforms {
        k.positions = corners.clone();
    }
    if mirror {
        for uv in &mut mesh.uvs {
            uv.x = 1.0 - uv.x;
        }
    }
    model.textures = vec![TextureRef {
        name: "skin".into(),
        width: size,
        height: size,
        normal_map: None,
    }];
    let rgba = vec![255u8; (size * size * 4) as usize];
    let textures = [(
        "skin".to_owned(),
        Arc::from(encode_rgba_png(size, size, &rgba)),
    )]
    .into();
    (model, textures)
}

/// キャンバスを覆う 32×32 の深度マップ。
fn depth_map(f: impl Fn(f32, f32) -> f32) -> DepthMap {
    let n = 32u32;
    let values = (0..n * n)
        .map(|i| {
            f(
                ((i % n) as f32 + 0.5) / n as f32,
                ((i / n) as f32 + 0.5) / n as f32,
            )
        })
        .collect();
    DepthMap::new(n, n, values).unwrap()
}

/// 中心が最も手前の半球。
fn dome() -> DepthMap {
    depth_map(|u, v| {
        let r2 = (u - 0.5).powi(2) + (v - 0.5).powi(2);
        (0.25 - r2).max(0.0).sqrt()
    })
}

fn depth_options(rim: f32) -> NormalMapOptions<DepthNormalOptions> {
    NormalMapOptions {
        default: DepthNormalOptions {
            rim,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn depth_normals(model: Model, textures: Textures, map: &DepthMap, rim: f32) -> Decoded {
    let proposal = plan_depth_normal_maps(
        &model,
        &textures,
        map,
        &BTreeMap::new(),
        &depth_options(rim),
    )
    .unwrap();
    let mut doc = Document::with_textures(model, textures);
    apply(&mut doc, &proposal);
    Decoded::of(&doc, "skin")
}

#[test]
fn depth_dome_gives_radial_normals() {
    let (model, textures) = canvas_quad(64, false);
    let estimator = FixedDepthEstimator::new(dome()).unwrap();
    let proposal = propose_depth_normal_maps(
        &model,
        &textures,
        &estimator,
        &DepthRequest {
            image: textures["skin"].to_vec(),
        },
        &BTreeMap::new(),
        &depth_options(0.0),
    )
    .unwrap();
    let mut doc = Document::with_textures(model, textures);
    apply(&mut doc, &proposal);
    let map = Decoded::of(&doc, "skin");
    let center = map.normal(32, 32);
    assert!(center[2] > 0.99, "{center:?}");
    // 右の側は右へ、上の側は上 (法線マップの +y) へ傾く。
    assert!(map.normal(48, 32)[0] > 0.2, "{:?}", map.normal(48, 32));
    assert!(map.normal(16, 32)[0] < -0.2);
    assert!(map.normal(32, 16)[1] > 0.2, "{:?}", map.normal(32, 16));
    assert!(map.normal(32, 48)[1] < -0.2);
}

#[test]
fn depth_follows_the_uv_mapping() {
    // キャンバスの右ほど手前の坂。左右反転した UV では、テクスチャの左ほど手前になる。
    let ramp = depth_map(|u, _| u);
    let (model, textures) = canvas_quad(64, false);
    let n = depth_normals(model, textures, &ramp, 0.0).normal(32, 32);
    assert!(n[0] < -0.2 && n[1].abs() < 0.02, "{n:?}");
    let (model, textures) = canvas_quad(64, true);
    let n = depth_normals(model, textures, &ramp, 0.0).normal(32, 32);
    assert!(n[0] > 0.2 && n[1].abs() < 0.02, "{n:?}");
}

#[test]
fn depth_relief_is_measured_on_the_canvas() {
    // テクスチャの解像度を変えても (キャンバスでの大きさが同じなら) 傾きは同じ。
    let (model, textures) = canvas_quad(64, false);
    let fine = depth_normals(model, textures, &dome(), 0.0).normal(48, 32);
    let (model, textures) = canvas_quad(32, false);
    let coarse = depth_normals(model, textures, &dome(), 0.0).normal(24, 16);
    assert!((fine[0] - coarse[0]).abs() < 0.05, "{fine:?} {coarse:?}");
}

#[test]
fn rim_rounds_a_flat_depth() {
    let flat = depth_map(|_, _| 1.0);
    let (model, textures) = canvas_quad(64, false);
    assert!(depth_normals(model, textures, &flat, 0.0).is_flat());
    let (model, textures) = canvas_quad(64, false);
    let map = depth_normals(model, textures, &flat, 0.5);
    assert!(map.normal(62, 32)[0] > 0.2, "the right rim faces right");
    assert!(map.normal(32, 32)[2] > 0.99);
}
