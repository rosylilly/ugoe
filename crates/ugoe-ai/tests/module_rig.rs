//! モジュールのプロジェクト向けの自動リギングの提案 (`ugoe_ai::module_rig`) のテスト。ネットワークは使わない。
//!
//! 本体は実際の PSD (`comfyui-see-through-bustup-parts.psd`) にテンプレートのリグ (`propose_rig`) を組み、
//! 頭の回転デフォーマ `HeadRotation` に取り付けるスロット `Headwear` を足したもの。衣装はテストの中で作る
//! レイヤーの木 (帽子と左右の耳飾り) を、モジュールのプロジェクトの作業用のドキュメントに取り込む。
//! 本体の顔の向き (`AngleX` / `AngleY`) は `HeadRotation` の子の顔のワープにあるので、取り込んだだけの衣装は
//! 振り向きに付いていかない。AI の経路は録画 (`tests/cassettes/module_rig_retry.json`、Messages API の応答の形に
//! 合わせて手で作ったもの。1 回目は範囲の外の値、2 回目は正しい値と反映できない注文) の再生で確かめる。

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ugoe::{Puppet, Toggles};
use ugoe_ai::classify::{Label, classify_by_name, overview_png};
use ugoe_ai::claude::{ApiKey, ClaudeConfig, ClaudeProvider};
use ugoe_ai::instructions::Instructions;
use ugoe_ai::module_rig::{
    FollowMode, ModuleRigGuidance, ModuleRigOptions, ModuleRigParams, module_mesh_parts,
    module_rig_request_guided, propose_module_rig, propose_module_rig_with_ai_guided,
};
use ugoe_ai::rig::{RigOptions, RigParams, mesh_parts, propose_rig};
use ugoe_ai::transport::{Cassette, ReplayTransport};
use ugoe_ai::{AiError, Proposal};
use ugoe_core::Vec2;
use ugoe_editor_core::import::layer_tree_command;
use ugoe_editor_core::{
    Command, Document, ModuleBase, ModuleContext, Project, PsdImportOptions, import_psd,
};
use ugoe_model::module::{DrawOrderRange, ModuleRequirements};
use ugoe_model::physics::{PhysicsTarget, VertexOwner};
use ugoe_model::{DeformerKind, Model, Slot, SlotId};
use ugoe_psd::{Layer, LayerNode, LayerTree};
use ugoe_rigging::ParentWorld;
use ugoe_runtime::{CompiledModel, ModelInstance, compose, compose_physics};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// 本体 (リグを組んだ胸から上の人物) と、スロットを足した本体のドキュメント。
struct Base {
    doc: Document,
    /// 顔のメッシュの基準の姿の外接矩形 (キャンバス座標)。
    face: (Vec2, Vec2),
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let name = "comfyui-see-through-bustup-parts.psd";
        let bytes = std::fs::read(repo_path("../../assets/fixtures/psd-real").join(name)).unwrap();
        let mut doc = Document::new(Project::empty(name).model);
        let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(name)).unwrap();
        doc.apply(import.command).unwrap();
        let tree = ugoe_psd::import(&bytes).unwrap();
        let parts = mesh_parts(doc.model(), &classify_by_name(&tree));
        let face = parts
            .iter()
            .find(|(_, p)| p.label == Label::Face)
            .map(|(id, _)| id.clone())
            .unwrap();
        let proposal = propose_rig(
            doc.model(),
            &parts,
            &RigParams::default(),
            &RigOptions::default(),
        )
        .unwrap();
        apply(&mut doc, &proposal);
        for (id, anchor, (min, max)) in [
            ("Headwear", "HeadRotation", (960.0, 1000.0)),
            ("FaceMask", "FaceWarp", (940.0, 950.0)),
        ] {
            doc.apply(Command::AddSlot {
                slot: Slot {
                    id: id.into(),
                    anchor: Some(anchor.into()),
                    draw_order: DrawOrderRange { min, max },
                    replaces: Vec::new(),
                    parameters: Vec::new(),
                    mask_sources: Vec::new(),
                    part: None,
                    colliders: Vec::new(),
                    pose_parts: Vec::new(),
                },
            })
            .unwrap();
        }
        let face = bounds(doc.model(), face.as_str());
        Base { doc, face }
    })
}

fn apply(doc: &mut Document, proposal: &Proposal) {
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands.clone(),
    })
    .unwrap();
}

/// メッシュの基準の姿の外接矩形。
fn bounds(model: &Model, mesh: &str) -> (Vec2, Vec2) {
    let points = positions(model, &[], mesh);
    points.iter().fold((points[0], points[0]), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    })
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

/// 塗りつぶした矩形のレイヤー。
fn layer(name: &str, min: Vec2, size: (u32, u32), rgba: [u8; 4]) -> LayerNode {
    LayerNode::Layer(Layer {
        name: name.into(),
        visible: true,
        opacity: 1.0,
        offset: min,
        width: size.0,
        height: size.1,
        rgba: rgba.repeat((size.0 * size.1) as usize),
        blend_mode: "norm".into(),
        id: None,
        fill_opacity: 1.0,
        clipping: false,
    })
}

/// 衣装: 頭の上の帽子と、顔の左右の下に垂れる耳飾り (`earring-l` はキャラクターの左 = 画像の右)。
fn costume() -> LayerTree {
    let (min, max) = base().face;
    let (w, h) = (max.x - min.x, max.y - min.y);
    let hat_min = Vec2::new(min.x - 0.05 * w, min.y - 0.3 * h).round();
    let hat = ((1.1 * w) as u32, (0.45 * h) as u32);
    let earring = (16, (0.3 * h) as u32);
    let y = (min.y + 0.55 * h).round();
    LayerTree {
        width: 1024,
        height: 1024,
        root: vec![
            layer("hat", hat_min, hat, [40, 40, 90, 255]),
            layer(
                "earring-r",
                Vec2::new(min.x - 12.0, y),
                earring,
                [230, 200, 60, 255],
            ),
            layer(
                "earring-l",
                Vec2::new(max.x - 4.0, y),
                earring,
                [230, 200, 60, 255],
            ),
        ],
        warnings: Vec::new(),
    }
}

/// スロット `slot` に付けるモジュール `Costume` の作業用のドキュメントに、衣装を取り込んだもの。
fn module_doc(slot: &str) -> Document {
    let base = base();
    let context = ModuleContext::new(
        ModuleBase {
            path: "/base.ugoeproj".into(),
            model: base.doc.model().clone(),
            physics: base.doc.physics().cloned(),
            textures: base.doc.textures().clone(),
        },
        &SlotId::from(slot),
        "Costume",
        ModuleRequirements::default(),
    )
    .unwrap();
    let context = Arc::new(context);
    let workspace = context
        .workspace(&context.empty_module(), &Default::default())
        .unwrap();
    let mut doc = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context.clone());
    let mut import =
        layer_tree_command(doc.model(), &costume(), &PsdImportOptions::new("costume")).unwrap();
    context.map_new_meshes(&mut import.command);
    doc.apply(import.command).unwrap();
    doc
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

fn max_distance(a: &[Vec2], b: &[Vec2]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(p, q)| (*p - *q).length())
        .fold(0.0, f32::max)
}

/// 帽子は顔のワープの振り向きに付いていき、耳飾りは揺れる。パラメータは足さず、本体を変えない。
#[test]
fn the_costume_follows_the_face_turn_without_adding_parameters() {
    let mut doc = module_doc("Headwear");
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume()));
    assert_eq!(parts.len(), 3, "{parts:?}");
    let hat = mesh_id(&doc, "hat");
    let before = doc.model().clone();
    // 取り込んだだけでは、帽子は `AngleX` で動かない (取り付け先の頭の回転は `AngleZ` だけ)。
    let still = max_distance(
        &positions(&before, &[], &hat),
        &positions(&before, &[("AngleX", 30.0)], &hat),
    );
    assert!(still < 1e-3, "{still}");

    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    for command in &proposal.commands {
        assert!(
            !matches!(
                command,
                Command::AddParameter { .. }
                    | Command::SetParameterMirror { .. }
                    | Command::SetEffects { .. }
            ),
            "{command:?}"
        );
    }
    assert!(
        proposal.rationale.contains("FaceWarp"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let model = doc.model();
    assert_eq!(model.parameters, before.parameters);

    // 帽子の頂点は、キーの上で本体の顔のワープの中の点と同じ所へ動く (追従のワープの格子の分だけずれる)。
    let mut world = ParentWorld::new(model, Some(&"FaceWarp".into())).unwrap();
    let rest = positions(model, &[], &hat);
    let guesses = vec![Vec2::splat(0.5); rest.len()];
    let inner = world.from_canvas(&rest, &guesses).unwrap();
    for values in [
        vec![("AngleX", 30.0)],
        vec![("AngleX", -30.0), ("AngleY", 30.0)],
    ] {
        world.set_pose(values.iter().copied());
        let expected = world.to_canvas(&inner);
        let actual = positions(model, &values, &hat);
        let moved = max_distance(&rest, &expected);
        let error = max_distance(&actual, &expected);
        assert!(
            moved > 5.0,
            "{values:?}: the face warp moves the hat area {moved}px"
        );
        assert!(
            error < 0.2 * moved,
            "{values:?}: {error}px off (moved {moved}px)"
        );
    }

    // モジュールに分けられ、本体と合成できる (書き出しの確かめと同じ)。
    let (module, _) = doc.split_module().unwrap().unwrap();
    assert!(
        module
            .deformers
            .iter()
            .any(|d| d.id.as_str() == "FollowFaceWarp")
    );
    assert_eq!(module.physics.rigs.len(), 2, "{:?}", module.physics.rigs);
    for rig in &module.physics.rigs {
        assert_eq!(rig.frame.as_ref().unwrap().as_str(), "FollowFaceWarp");
        assert_eq!(rig.inputs[0].parameter.as_str(), "AngleX");
        let PhysicsTarget::Vertices { owner, weights } = &rig.outputs[0].target else {
            panic!("{:?}", rig.outputs[0].target);
        };
        assert!(matches!(owner, VertexOwner::Warp(w) if w.as_str().starts_with("Sway_earwear")));
        assert_eq!(weights[0], 0.0);
        assert!(*weights.last().unwrap() > 0.5, "{weights:?}");
    }
    let base_model = base().doc.model();
    compose(base_model, &[&module]).unwrap();
    compose_physics(base_model, base().doc.physics().unwrap(), &[&module]).unwrap();
}

/// 耳飾りは頭を振ると遅れて揺れる (物理を切った姿と比べる)。帽子は揺らさない。
#[test]
fn the_earrings_sway_when_the_head_turns() {
    let mut doc = module_doc("Headwear");
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume()));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    apply(&mut doc, &proposal);
    let (module, _) = doc.split_module().unwrap().unwrap();
    let base_model = base().doc.model();
    let model = compose(base_model, &[&module]).unwrap();
    let physics = compose_physics(base_model, base().doc.physics().unwrap(), &[&module]).unwrap();
    let toggles = |physics| Toggles {
        physics,
        eye_blink: false,
        breath: false,
        lip_sync: false,
        pose: false,
    };
    let make = |on| {
        let mut p = Puppet::new(model.clone())
            .unwrap()
            .with_physics(physics.clone());
        p.set_toggles(toggles(on));
        p
    };
    let (mut moving, mut still) = (make(true), make(false));
    let mesh = |p: &Puppet, name: &str| {
        let id = format!("Costume/{}", mesh_id(&doc, name));
        let i = p.instance().compiled().mesh_index(&id).unwrap();
        p.instance().drawables()[i].positions.clone()
    };
    let mut peak: BTreeMap<&str, f32> = BTreeMap::new();
    for frame in 0..90 {
        let value = if frame < 10 { 0.0 } else { 30.0 };
        for p in [&mut moving, &mut still] {
            let params = p.instance_mut().parameters_mut();
            let index = params.index_of("AngleX").unwrap();
            params.set(index, value);
            p.update(1.0 / 60.0);
        }
        for name in ["earring-l", "earring-r", "hat"] {
            let d = max_distance(&mesh(&moving, name), &mesh(&still, name));
            let e = peak.entry(name).or_default();
            *e = e.max(d);
        }
    }
    assert!(peak["earring-l"] > 1.0, "{peak:?}");
    assert!(peak["earring-r"] > 1.0, "{peak:?}");
    assert!(peak["hat"] < 1e-3, "{peak:?}");
}

/// 取り付け先が顔のワープそのもので、揺らすものもなければ、組むものがない。揺れの大きさ 0 も同じ。
#[test]
fn nothing_to_rig_is_reported() {
    let doc = module_doc("FaceMask");
    let hat_only: BTreeMap<_, _> = module_mesh_parts(&doc, &classify_by_name(&costume()))
        .into_iter()
        .filter(|(_, p)| p.label == Label::Headwear)
        .collect();
    let err = propose_module_rig(
        &doc,
        &hat_only,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(&err, AiError::InvalidRequest(m) if m.contains("nothing to rig")),
        "{err}"
    );
    let all = module_mesh_parts(&doc, &classify_by_name(&costume()));
    let off = ModuleRigParams {
        sway_scale: 0.0,
        ..ModuleRigParams::default()
    };
    assert!(matches!(
        propose_module_rig(&doc, &all, &off, &ModuleRigOptions::default()),
        Err(AiError::InvalidRequest(_))
    ));
    // 揺らすなら、取り付け先の直下に揺れの取り付け先のワープを作る (従う本体のワープはない)。
    let proposal = propose_module_rig(
        &doc,
        &all,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("SwayRoot"),
        "{}",
        proposal.rationale
    );
    let mut doc = doc;
    apply(&mut doc, &proposal);
    let (module, _) = doc.split_module().unwrap().unwrap();
    let root = module
        .deformers
        .iter()
        .find(|d| d.id.as_str() == "SwayRoot")
        .unwrap();
    assert!(root.parent.is_none());
    assert!(matches!(&root.kind, DeformerKind::Warp(w) if w.keyforms.bindings.is_empty()));
}

/// モデルのプロジェクトには使えない。
#[test]
fn a_model_project_is_rejected() {
    let doc = base().doc.detached();
    let err = propose_module_rig(
        &doc,
        &BTreeMap::new(),
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(&err, AiError::InvalidRequest(m) if m.contains("not a module project")),
        "{err}"
    );
}

/// AI の答えが範囲の外なら問い直し、2 回目の値を使う。反映できない注文は分けて返す。
#[test]
fn the_ai_answer_is_retried_and_unapplied_instructions_are_listed() {
    let mut doc = module_doc("Headwear");
    let tree = costume();
    let parts = module_mesh_parts(&doc, &classify_by_name(&tree));
    let image = overview_png(&tree, 256).unwrap();
    let cassette = Cassette::load(repo_path("tests/cassettes/module_rig_retry.json")).unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport.clone(),
    );
    let guidance = ModuleRigGuidance {
        instructions: Instructions::new(
            "Make the earrings swing a lot. Add a parameter for the hat brim.",
        )
        .unwrap(),
        ..ModuleRigGuidance::default()
    };
    let rig = propose_module_rig_with_ai_guided(
        &provider,
        &doc,
        &parts,
        &image,
        &ModuleRigOptions::default(),
        3,
        &guidance,
    )
    .unwrap();
    assert_eq!(transport.remaining(), 0);
    assert_eq!(rig.rejected.len(), 1);
    assert!(
        rig.rejected[0]
            .error
            .contains("sway_scale is 3, but it must be between 0 and 2"),
        "{}",
        rig.rejected[0].error
    );
    assert_eq!(rig.params.sway_scale, 1.6);
    assert_eq!(rig.params.sway_length, 0.8);
    assert_eq!(rig.unapplied.len(), 1, "{:?}", rig.unapplied);
    assert!(rig.unapplied[0].contains("hat brim"));
    assert!(!rig.reason.contains("Not applied"));
    assert!(rig.proposal.rationale.contains("揺れの大きさ 1.60"));
    apply(&mut doc, &rig.proposal);
    let (module, _) = doc.split_module().unwrap().unwrap();
    // 耳飾りの重みの最大 0.8 × 1.6 は 1 までに抑える。
    let PhysicsTarget::Vertices { weights, .. } = &module.physics.rigs[0].outputs[0].target else {
        panic!();
    };
    assert_eq!(weights.iter().copied().fold(0.0, f32::max), 1.0);

    // 送った文: スロット・衣装の部位・注文。2 回目は前の答えとエラーを添える。
    let received = transport.received();
    assert_eq!(received.len(), 2);
    let text = |i: usize| received[i].body.as_ref().unwrap().to_string();
    assert!(text(0).contains("slot `Headwear`"), "{}", text(0));
    assert!(text(0).contains("earwear (left)"));
    assert!(text(0).contains("follows the base warp `FaceWarp`"));
    assert!(text(0).contains("user_instructions"));
    assert!(text(0).contains("cannot add"));
    assert!(!text(0).contains("Attempt 1"));
    assert!(text(1).contains("Attempt 1 was rejected"));
    // 送る前の確認に使うリクエストは、1 回目と同じ。
    let request =
        module_rig_request_guided(&module_doc("Headwear"), &parts, &image, &[], &guidance).unwrap();
    assert!(request.system.contains("cannot add parameters"));
    assert!(request.prompt.contains("follows the base warp `FaceWarp`"));
    assert_eq!(request.images.len(), 1);
}

/// 見本のアバター (`assets/samples/avatar.ugoe`) では、頭の回転の下に顔のワープと後ろ髪のワープが並ぶ。
/// 頭の飾りのスロット (描画順は前髪より手前) の帽子は、覆う割合の大きい後ろ髪のワープではなく、描画順の近い
/// 前髪・顔を動かす顔のワープに従う (振り向きで後ろ髪と逆へ動かない)。
#[test]
fn the_costume_follows_the_layer_it_is_drawn_over() {
    let bytes = std::fs::read(repo_path("../../assets/samples/avatar.ugoe")).unwrap();
    let bundle = ugoe::format::read_bundle(&bytes).unwrap();
    let mut base = Document::new(bundle.model);
    base.apply(Command::AddSlot {
        slot: Slot {
            id: "Headwear".into(),
            anchor: Some("HeadRotation".into()),
            draw_order: DrawOrderRange {
                min: 970.0,
                max: 1000.0,
            },
            replaces: Vec::new(),
            parameters: Vec::new(),
            mask_sources: Vec::new(),
            part: None,
            colliders: Vec::new(),
            pose_parts: Vec::new(),
        },
    })
    .unwrap();
    let context = Arc::new(
        ModuleContext::new(
            ModuleBase {
                path: "/avatar.ugoe".into(),
                model: base.model().clone(),
                physics: None,
                textures: Default::default(),
            },
            &SlotId::from("Headwear"),
            "Hat",
            ModuleRequirements::default(),
        )
        .unwrap(),
    );
    let workspace = context
        .workspace(&context.empty_module(), &Default::default())
        .unwrap();
    let mut doc =
        Document::with_textures(workspace.model, workspace.textures).with_module(context.clone());
    let tree = LayerTree {
        width: 1024,
        height: 1024,
        root: vec![layer(
            "hat",
            Vec2::new(340.0, 90.0),
            (340, 170),
            [70, 60, 110, 255],
        )],
        warnings: Vec::new(),
    };
    let mut import = layer_tree_command(doc.model(), &tree, &PsdImportOptions::new("hat")).unwrap();
    context.map_new_meshes(&mut import.command);
    doc.apply(import.command).unwrap();
    let parts = module_mesh_parts(&doc, &classify_by_name(&tree));
    let proposal = propose_module_rig(
        &doc,
        &parts,
        &ModuleRigParams::default(),
        &ModuleRigOptions::default(),
    )
    .unwrap();
    assert!(
        proposal.rationale.contains("本体のワープ `FaceWarp`"),
        "{}",
        proposal.rationale
    );
    assert!(
        !proposal.rationale.contains("EyeLWarp"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    // `AngleX` = 30 で、帽子は顔と同じ向き (画面の右) へ動く。
    let hat = mesh_id(&doc, "hat");
    let dx = |values: &[(&str, f32)]| {
        let p = positions(doc.model(), values, &hat);
        p.iter().map(|v| v.x).sum::<f32>() / p.len() as f32
    };
    assert!(dx(&[("AngleX", 30.0)]) - dx(&[]) > 3.0);
}

/// 追従をメッシュに直接写す形では、追従のワープを作らず、帽子の頂点はキーの上で本体の顔のワープの中の点と
/// ほぼ同じ所へ動く (ワープで近似する形の 2 割よりずっと小さい)。耳飾りは揺れの取り付け先のワープで揺れる。
#[test]
fn the_mesh_mode_follows_the_face_warp_at_the_keys() {
    let mut doc = module_doc("Headwear");
    let parts = module_mesh_parts(&doc, &classify_by_name(&costume()));
    let options = ModuleRigOptions {
        follow: FollowMode::Mesh,
        ..ModuleRigOptions::default()
    };
    let proposal = propose_module_rig(&doc, &parts, &ModuleRigParams::default(), &options).unwrap();
    assert!(
        proposal.rationale.contains("直接写す"),
        "{}",
        proposal.rationale
    );
    apply(&mut doc, &proposal);
    let model = doc.model();
    let hat = mesh_id(&doc, "hat");
    let mut world = ParentWorld::new(model, Some(&"FaceWarp".into())).unwrap();
    let rest = positions(model, &[], &hat);
    let guesses = vec![Vec2::splat(0.5); rest.len()];
    let inner = world.from_canvas(&rest, &guesses).unwrap();
    for values in [
        vec![("AngleX", 30.0)],
        vec![("AngleX", -30.0), ("AngleY", 30.0)],
    ] {
        world.set_pose(values.iter().copied());
        let expected = world.to_canvas(&inner);
        let actual = positions(model, &values, &hat);
        let moved = max_distance(&rest, &expected);
        let error = max_distance(&actual, &expected);
        assert!(moved > 5.0, "{values:?}: {moved}px");
        assert!(
            error < 0.02 * moved,
            "{values:?}: {error}px off (moved {moved}px)"
        );
    }
    let (module, _) = doc.split_module().unwrap().unwrap();
    assert!(
        !module
            .deformers
            .iter()
            .any(|d| d.id.as_str().starts_with("Follow")),
        "{:?}",
        module.deformers.iter().map(|d| &d.id).collect::<Vec<_>>()
    );
    let hat_mesh = module.meshes.iter().find(|m| m.name == "hat").unwrap();
    let axes: Vec<&str> = hat_mesh
        .keyforms
        .bindings
        .iter()
        .map(|b| b.parameter.as_str())
        .collect();
    assert!(
        axes.contains(&"AngleX") && axes.contains(&"AngleY"),
        "{axes:?}"
    );
    assert_eq!(module.physics.rigs.len(), 2);
    for rig in &module.physics.rigs {
        assert_eq!(rig.frame.as_ref().unwrap().as_str(), "SwayRoot");
    }
    let base_model = base().doc.model();
    compose(base_model, &[&module]).unwrap();
}
