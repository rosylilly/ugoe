//! モジュールの自動リギングの提案 (`ugoe_ai::module_rig`) のうち、本体のメッシュのキーフォームへの追従・
//! 本体の衝突体・衣装の差分のポーズのテスト (T0519)。ネットワークは使わない。
//!
//! 本体は着せ替えの見本 (`assets/samples/dress-up/dress-up.ugoe`。スロットの取り付け先は頭と体のワープで、
//! 目 `EyeL` は `EyeLOpen` のキーフォームで縦に潰れる)。衝突体・スロット・ポーズはテストの中で足す。

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;

use ugoe_ai::classify::classify_by_name;
use ugoe_ai::module_rig::{
    FollowMode, ModuleRigOptions, ModuleRigParams, module_mesh_parts, propose_module_rig,
};
use ugoe_ai::{AiError, Proposal};
use ugoe_core::Vec2;
use ugoe_editor_core::import::layer_tree_command;
use ugoe_editor_core::physics_check::rest_pose_collisions;
use ugoe_editor_core::{
    ChildRef, Command, Document, Keyform, KeyformTarget, ModuleBase, ModuleContext,
    PsdImportOptions,
};
use ugoe_model::module::{DrawOrderRange, ModuleRequirements};
use ugoe_model::part::PartKeyform;
use ugoe_model::physics::{ColliderShape, PhysicsCollider};
use ugoe_model::pose::{Pose, PoseEntry, PoseGroup};
use ugoe_model::{KeyformGrid, Model, Part, Slot, SlotId};
use ugoe_psd::{Layer, LayerNode, LayerTree};
use ugoe_runtime::{CompiledModel, ModelInstance, compose, compose_physics};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// 着せ替えの見本の本体。
fn sample() -> Document {
    let bytes = std::fs::read(repo_path("../../assets/samples/dress-up/dress-up.ugoe")).unwrap();
    let bundle = ugoe::format::read_bundle(&bytes).unwrap();
    Document::new(bundle.model).with_physics(bundle.physics)
}

fn slot(id: &str, anchor: &str, (min, max): (f32, f32)) -> Slot {
    Slot {
        id: id.into(),
        anchor: Some(anchor.into()),
        draw_order: DrawOrderRange { min, max },
        replaces: Vec::new(),
        parameters: Vec::new(),
        mask_sources: Vec::new(),
        part: None,
        colliders: Vec::new(),
        pose_parts: Vec::new(),
    }
}

/// 本体 `base` のスロット `slot` に付けるモジュール `Costume` の作業用のドキュメントに、`tree` を取り込んだもの。
fn module_doc(base: &Document, slot: &str, tree: &LayerTree) -> Document {
    let context = Arc::new(
        ModuleContext::new(
            ModuleBase {
                path: "/dress-up.ugoe".into(),
                model: base.model().clone(),
                physics: base.physics().cloned(),
                textures: base.textures().clone(),
            },
            &SlotId::from(slot),
            "Costume",
            ModuleRequirements::default(),
        )
        .unwrap(),
    );
    let workspace = context
        .workspace(&context.empty_module(), &Default::default())
        .unwrap();
    let mut doc = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context.clone());
    let mut import =
        layer_tree_command(doc.model(), tree, &PsdImportOptions::new("costume")).unwrap();
    context.map_new_meshes(&mut import.command);
    doc.apply(import.command).unwrap();
    doc
}

fn apply(doc: &mut Document, proposal: &Proposal) {
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands.clone(),
    })
    .unwrap();
}

fn layer(name: &str, min: Vec2, size: (u32, u32)) -> LayerNode {
    LayerNode::Layer(Layer {
        name: name.into(),
        visible: true,
        opacity: 1.0,
        offset: min.round(),
        width: size.0,
        height: size.1,
        rgba: [200, 90, 120, 255].repeat((size.0 * size.1) as usize),
        blend_mode: "norm".into(),
        id: None,
        fill_opacity: 1.0,
        clipping: false,
    })
}

fn group(name: &str, children: Vec<LayerNode>) -> LayerNode {
    LayerNode::Group {
        name: name.into(),
        visible: true,
        children,
        id: None,
        opacity: 1.0,
        blend_mode: "pass".into(),
        clipping: false,
    }
}

fn tree(root: Vec<LayerNode>) -> LayerTree {
    LayerTree {
        width: 512,
        height: 512,
        root,
        warnings: Vec::new(),
    }
}

/// パラメータの値 `values` (ほかは既定値) でのメッシュの頂点 (キャンバス座標)。
fn positions(model: &Model, values: &[(&str, f32)], mesh: &str) -> Vec<Vec2> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for (id, value) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, *value);
    }
    instance.update();
    let index = instance.compiled().mesh_index(mesh).unwrap();
    instance.drawables()[index].positions.clone()
}

fn bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    points.iter().fold((points[0], points[0]), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    })
}

fn mesh_id(doc: &Document, name: &str) -> String {
    let context = doc.module().unwrap();
    doc.model()
        .meshes
        .iter()
        .find(|m| m.name == name && !context.is_base_mesh(&m.id))
        .unwrap()
        .id
        .to_string()
}

/// 目の上のアイシャドウは、本体の目のメッシュのキーフォーム (`EyeLOpen`) に従って目と一緒に潰れる。
/// 同じスロットの帽子 (形の決まった飾り) は目に従わない。パラメータは足さない。
#[test]
fn eye_makeup_follows_the_closing_eye() {
    let base = sample();
    let eye = bounds(&positions(base.model(), &[], "EyeL"));
    let size = eye.1 - eye.0;
    let costume = tree(vec![
        layer(
            "makeup",
            // 取り込みの余白で、輪郭の頂点は目のメッシュの少し外に出る。
            eye.0 + Vec2::new(0.05, 0.05) * size,
            ((0.9 * size.x) as u32, (0.45 * size.y) as u32),
        ),
        layer("hat", Vec2::new(180.0, 20.0), (150, 60)),
    ]);
    let mut doc = module_doc(&base, "Headwear", &costume);
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("base mesh `EyeL`"),
        "{}",
        proposal.rationale
    );
    let before = doc.model().parameters.clone();
    apply(&mut doc, &proposal);
    let model = doc.model();
    assert_eq!(model.parameters, before);
    let makeup = mesh_id(&doc, "makeup");
    let height = |values: &[(&str, f32)]| {
        let (lo, hi) = bounds(&positions(model, values, &makeup));
        hi.y - lo.y
    };
    let (open, closed) = (height(&[]), height(&[("EyeLOpen", 0.0)]));
    assert!(closed < 0.5 * open, "open {open}, closed {closed}");
    // 目の中心と同じだけ動く (目は上下から中心へ潰れる)。
    let center = |values: &[(&str, f32)], mesh: &str| {
        let (lo, hi) = bounds(&positions(model, values, mesh));
        (lo + hi) * 0.5
    };
    let eye_shift = center(&[("EyeLOpen", 0.0)], "EyeL").y - center(&[], "EyeL").y;
    let makeup_shift = center(&[("EyeLOpen", 0.0)], &makeup).y - center(&[], &makeup).y;
    assert!(
        makeup_shift * eye_shift >= 0.0,
        "{makeup_shift} vs {eye_shift}"
    );
    let hat = mesh_id(&doc, "hat");
    let still = positions(model, &[], &hat)
        .iter()
        .zip(positions(model, &[("EyeLOpen", 0.0)], &hat))
        .map(|(a, b)| (*a - b).length())
        .fold(0.0, f32::max);
    assert!(still < 1e-4, "{still}");
    let (module, _) = doc.split_module().unwrap().unwrap();
    compose(base.model(), &[&module]).unwrap();
}

/// 揺れる後ろ髪の振り子は、スロットが公開する本体の衝突体に当たる。静止姿勢で当たる衝突体は外す。
#[test]
fn swaying_parts_collide_with_the_exposed_colliders() {
    let mut base = sample();
    let head = bounds(&positions(base.model(), &[], "Face"));
    let hair_min = Vec2::new(head.0.x, head.0.y);
    let hair_size = ((head.1.x - head.0.x) as u32, 260);
    let root = Vec2::new(hair_min.x + hair_size.0 as f32 / 2.0, hair_min.y);
    for (id, center, radius) in [
        // 振り子の鎖の上 (静止姿勢で当たる)。
        ("Neck", root + Vec2::new(0.0, 150.0), 10.0),
        // 鎖の横 (振り向きで揺れた髪が当たる)。
        ("Shoulder", root + Vec2::new(60.0, 230.0), 20.0),
    ] {
        base.apply(Command::AddPhysicsCollider {
            collider: PhysicsCollider {
                id: id.into(),
                name: id.into(),
                attach: None,
                shape: ColliderShape::Circle { center, radius },
            },
        })
        .unwrap();
    }
    let mut back = slot("Back", "HeadWarp", (100.0, 140.0));
    back.colliders = vec!["Neck".into(), "Shoulder".into()];
    base.apply(Command::AddSlot { slot: back }).unwrap();
    let costume = tree(vec![layer("hair_back", hair_min, hair_size)]);
    let mut doc = module_doc(&base, "Back", &costume);
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("`Shoulder`"),
        "{}",
        proposal.rationale
    );
    assert!(
        proposal.rationale.contains("base colliders `Neck` at rest"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let physics = doc.physics().unwrap();
    let rig = physics.rigs.last().unwrap();
    assert_eq!(rig.colliders, vec!["Shoulder".to_owned()]);
    // `Neck` を外せば、粒子に衝突の半径を付けても静止姿勢で当たらない。
    assert!(
        rig.particles[1..].iter().all(|p| p.collision_radius > 0.0),
        "{:?}",
        rig.particles
    );
    assert!(
        rest_pose_collisions(doc.model(), physics)
            .unwrap()
            .is_empty()
    );
    let (module, _) = doc.split_module().unwrap().unwrap();
    compose_physics(base.model(), base.physics().unwrap(), &[&module]).unwrap();

    // 衝突体を使わない組み方では当てない。
    let off = ModuleRigOptions {
        colliders: false,
        ..ModuleRigOptions::default()
    };
    let proposal = propose_module_rig(
        &module_doc(&base, "Back", &costume),
        &parts,
        &ModuleRigParams::default(),
        &off,
    )
    .unwrap();
    let added = proposal
        .commands
        .iter()
        .find_map(|c| match c {
            Command::AddPhysicsRig { rig } => Some(rig),
            _ => None,
        })
        .unwrap();
    assert!(added.colliders.is_empty());
}

/// 名前の末尾の A / B だけが違う衣装は、ポーズにする。本体の腕の差分 (スロットが公開するもの) と同じ文字で
/// 絵の重なる袖は本体の要素に追従させ、腕と重ならない飾りの差分は衣装だけのグループにする
/// (メッシュの差分はパーツを作って入れる)。
#[test]
fn variants_become_poses() {
    let mut base = sample();
    for (id, name) in [("ArmA", "Arm A"), ("ArmB", "Arm B")] {
        base.apply(Command::AddPart {
            part: Part {
                id: id.into(),
                name: name.into(),
                parent: Some("Body".into()),
                keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
            },
        })
        .unwrap();
        // 腕の差分の絵 (見本には腕がないので、服のメッシュを写したもの)。
        let mut arm = base
            .model()
            .meshes
            .iter()
            .find(|m| m.id.as_str() == "Shirt")
            .unwrap()
            .clone();
        arm.id = format!("{id}Mesh").as_str().into();
        arm.name = name.into();
        arm.part = id.into();
        base.apply(Command::AddMesh { mesh: arm }).unwrap();
    }
    base.apply(Command::SetPose {
        pose: Pose {
            groups: vec![PoseGroup {
                entries: vec![
                    PoseEntry {
                        part: "ArmA".into(),
                        links: Vec::new(),
                    },
                    PoseEntry {
                        part: "ArmB".into(),
                        links: Vec::new(),
                    },
                ],
            }],
            ..Pose::default()
        },
    })
    .unwrap();
    let mut sleeves = slot("Sleeves", "BodyWarp", (350.0, 380.0));
    sleeves.pose_parts = vec!["ArmA".into(), "ArmB".into()];
    base.apply(Command::AddSlot { slot: sleeves }).unwrap();

    let costume = tree(vec![
        group(
            "Sleeve A",
            vec![layer("sleeve-a", Vec2::new(150.0, 300.0), (40, 60))],
        ),
        group(
            "Sleeve B",
            vec![layer("sleeve-b", Vec2::new(150.0, 300.0), (40, 80))],
        ),
        layer("badge A", Vec2::new(250.0, 20.0), (20, 20)),
        layer("badge B", Vec2::new(250.0, 20.0), (24, 24)),
    ]);
    let mut doc = module_doc(&base, "Sleeves", &costume);
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("`ArmA`"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let (module, _) = doc.split_module().unwrap().unwrap();
    let name = |id: &ugoe_model::PartId| {
        module
            .parts
            .iter()
            .find(|p| &p.id == id)
            .unwrap()
            .name
            .clone()
    };
    assert_eq!(module.pose.links.len(), 2, "{:?}", module.pose);
    assert_eq!(module.pose.links[0].part.as_str(), "ArmA");
    assert_eq!(name(&module.pose.links[0].links[0]), "Sleeve A");
    assert_eq!(module.pose.links[1].part.as_str(), "ArmB");
    assert_eq!(name(&module.pose.links[1].links[0]), "Sleeve B");
    assert_eq!(module.pose.groups.len(), 1, "{:?}", module.pose);
    let badges: Vec<String> = module.pose.groups[0]
        .entries
        .iter()
        .map(|e| name(&e.part))
        .collect();
    assert_eq!(badges, ["badge A", "badge B"]);
    let composed = compose(base.model(), &[&module]).unwrap();
    assert_eq!(composed.pose.groups.len(), 2);

    // ポーズにしない組み方で、ほかに組むものがなければ断る。
    let off = ModuleRigOptions {
        poses: false,
        ..ModuleRigOptions::default()
    };
    let err = propose_module_rig(
        &module_doc(&base, "Sleeves", &costume),
        &parts,
        &ModuleRigParams::default(),
        &off,
    )
    .unwrap_err();
    assert!(matches!(err, AiError::InvalidRequest(_)), "{err}");
}

/// 点 `p` を含む三角形の重心座標で、`from` の点を `to` の点へ写す (三角形の外なら `None`)。
fn carried(from: &[Vec2], to: &[Vec2], indices: &[u16], p: Vec2) -> Option<Vec2> {
    indices.chunks(3).find_map(|t| {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| from[i as usize]);
        let (v0, v1, v2) = (b - a, c - a, p - a);
        let det = v0.x * v1.y - v1.x * v0.y;
        let v = (v2.x * v1.y - v1.x * v2.y) / det;
        let w = (v0.x * v2.y - v2.x * v0.y) / det;
        let u = 1.0 - v - w;
        (u >= -1e-4 && v >= -1e-4 && w >= -1e-4).then(|| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| to[i as usize]);
            a * u + b * v + c * w
        })
    })
}

/// 本体のワープ (顔の向き) と、その下の目の回転デフォーマ (視線で傾く) と目のメッシュのキーフォーム (開閉) の
/// 動きを合わせて写す (T0521)。取り付け先が頭の回転なので、衣装は顔のワープに従い、化粧は目の上で、
/// 振り向き・傾き・閉じ具合を重ねた目と同じ所へ動く。
#[test]
fn makeup_follows_the_face_warp_the_eye_rotation_and_the_eye_mesh_together() {
    let mut base = sample();
    // 目 `EyeL` を、視線 `EyeBallX` で傾く回転デフォーマ `EyeLTilt` (顔のワープの子) に入れる。
    base.apply(Command::CreateRotationDeformer {
        id: Some("EyeLTilt".into()),
        name: "Eye L tilt".into(),
        children: vec![ChildRef::Mesh("EyeL".into())],
        parts: Vec::new(),
        origin: None,
        parent: None,
    })
    .unwrap();
    let target = KeyformTarget::Deformer("EyeLTilt".into());
    for key in [0.0, 1.0] {
        base.apply(Command::AddKeyformKey {
            target: target.clone(),
            parameter: "EyeBallX".into(),
            key,
        })
        .unwrap();
    }
    let tilt = base
        .model()
        .deformers
        .iter()
        .find(|d| d.id.as_str() == "EyeLTilt")
        .unwrap();
    let ugoe_model::DeformerKind::Rotation(rotation) = &tilt.kind else {
        panic!("not a rotation deformer");
    };
    let mut tilted = rotation.keyforms.keyforms[0];
    tilted.angle = 25.0;
    base.apply(Command::SetKeyformCell {
        target,
        cell: [("EyeBallX".into(), 1.0)].into_iter().collect(),
        keyform: Keyform::Rotation(tilted),
    })
    .unwrap();
    base.apply(Command::AddSlot {
        slot: slot("Face", "HeadRotation", (700.0, 800.0)),
    })
    .unwrap();

    let eye_rest = positions(base.model(), &[], "EyeL");
    let eye = bounds(&eye_rest);
    let size = eye.1 - eye.0;
    let costume = tree(vec![layer(
        "makeup",
        eye.0 + Vec2::new(0.05, 0.05) * size,
        ((0.9 * size.x) as u32, (0.45 * size.y) as u32),
    )]);
    let pose = [("AngleX", 30.0), ("EyeLOpen", 0.3), ("EyeBallX", 1.0)];
    let indices = base
        .model()
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "EyeL")
        .unwrap()
        .indices
        .clone();
    let eye_posed = positions(base.model(), &pose, "EyeL");
    for follow in [FollowMode::Mesh, FollowMode::Warp] {
        let mut doc = module_doc(&base, "Face", &costume);
        let parts = module_mesh_parts(&doc, &classify_by_name(&costume));
        let options = ModuleRigOptions {
            follow,
            ..ModuleRigOptions::default()
        };
        let proposal =
            propose_module_rig(&doc, &parts, &ModuleRigParams::default(), &options).unwrap();
        assert!(
            proposal.rationale.contains("base mesh `EyeL`")
                && proposal.rationale.contains("`EyeBallX`"),
            "{follow:?}: {}",
            proposal.rationale
        );
        apply(&mut doc, &proposal);
        let makeup = mesh_id(&doc, "makeup");
        let rest = positions(doc.model(), &[], &makeup);
        let posed = positions(doc.model(), &pose, &makeup);
        let mut checked = 0;
        let mut worst = 0.0_f32;
        for (r, p) in rest.iter().zip(&posed) {
            let Some(expected) = carried(&eye_rest, &eye_posed, &indices, *r) else {
                continue;
            };
            checked += 1;
            worst = worst.max((expected - *p).length());
        }
        // 取り込みの余白で、目の三角形の中にある頂点は一部 (目の中は重心座標で写すので、ずれは小さい。
        // 顔のワープの曲がりの分だけ残る)。
        assert!(
            checked >= 3,
            "{follow:?}: {checked} vertices inside the eye"
        );
        let motion = rest
            .iter()
            .zip(&posed)
            .map(|(a, b)| (*a - *b).length())
            .fold(0.0, f32::max);
        assert!(motion > 10.0, "{follow:?}: moved only {motion} px");
        assert!(
            worst < 0.08 * motion,
            "{follow:?}: off by {worst} px of {motion} px"
        );
        let (module, _) = doc.split_module().unwrap().unwrap();
        compose(base.model(), &[&module]).unwrap();
    }
}

fn hidden_layer(name: &str, min: Vec2, size: (u32, u32)) -> LayerNode {
    let LayerNode::Layer(mut layer) = layer(name, min, size) else {
        unreachable!()
    };
    layer.visible = false;
    LayerNode::Layer(layer)
}

/// 名前では差分と分からない衣装も、絵が重なり意味ラベルが同じで、PSD で片方を隠してあればポーズにする (T0521)。
/// 見えているものが最初に表示され、隠してあったメッシュは (作ったパーツで隠したまま) ポーズで出せるようになる。
/// どれも見えている重なった絵 (眼鏡とレンズ) と、ラベルの違う隠した絵 (ネクタイ) は差分にしない。
#[test]
fn overlapping_hidden_alternatives_become_poses() {
    let base = sample();
    let costume = tree(vec![
        layer("hat", Vec2::new(180.0, 20.0), (150, 60)),
        hidden_layer("cap", Vec2::new(185.0, 22.0), (140, 60)),
        hidden_layer("tie", Vec2::new(190.0, 30.0), (120, 40)),
        layer("glasses", Vec2::new(200.0, 180.0), (110, 30)),
        layer("glasses lens", Vec2::new(202.0, 182.0), (106, 26)),
    ]);
    let mut doc = module_doc(&base, "Headwear", &costume);
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("shown and hidden in the PSD"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let model = doc.model();
    assert_eq!(model.pose.groups.len(), 1, "{:?}", model.pose);
    let part = |id: &ugoe_model::PartId| model.parts.iter().find(|p| &p.id == id).unwrap();
    let entries: Vec<(&str, f32)> = model.pose.groups[0]
        .entries
        .iter()
        .map(|e| {
            let p = part(&e.part);
            (p.name.as_str(), p.keyforms.keyforms[0].opacity)
        })
        .collect();
    assert_eq!(entries, [("hat", 1.0), ("cap", 0.0)]);
    let cap = model
        .meshes
        .iter()
        .find(|m| m.id.as_str() == mesh_id(&doc, "cap"))
        .unwrap();
    assert!(cap.keyforms.keyforms.iter().all(|k| k.opacity == 1.0));
    let tie = model
        .meshes
        .iter()
        .find(|m| m.id.as_str() == mesh_id(&doc, "tie"))
        .unwrap();
    assert!(tie.keyforms.keyforms.iter().all(|k| k.opacity == 0.0));
    let (module, _) = doc.split_module().unwrap().unwrap();
    assert_eq!(module.pose.groups.len(), 1);
    compose(base.model(), &[&module]).unwrap();
}
