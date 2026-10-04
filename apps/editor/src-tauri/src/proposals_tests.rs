//! `proposals.rs` のテスト。ネットワークは使わない (AI の経路は送る前の確認までと、[`FakeProvider`] で確かめる)。
//! 入力は `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd`。

use std::sync::OnceLock;

use serde_json::json;
use ugoe_ai::FakeProvider;
use ugoe_ai::classify::Label;
use ugoe_model::Model;

use super::*;
use crate::ai::AiReply;
use crate::ai::tests::state;
use crate::error::IpcResult;

fn bustup_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd")
}

/// 「New from PSD…」の後の状態 (取り込みは重いので 1 回だけ行い、複製して使う)。
fn from_psd(secret: Option<&str>) -> AppState {
    static IMPORTED: OnceLock<(Model, PathBuf)> = OnceLock::new();
    let (model, path) = IMPORTED.get_or_init(|| {
        let path = bustup_path();
        let (bytes, options) = crate::psd::read_psd(&path.display().to_string(), None).unwrap();
        let (_, document, _) = crate::project::from_psd(&bytes, &options).unwrap();
        (document.model().clone(), path)
    });
    let state = state(secret);
    *state.document.lock().unwrap() = Document::new(model.clone());
    state
        .project
        .lock()
        .unwrap()
        .proposals
        .set_psd(path.clone());
    state
}

/// 注文なしの分類 (`classify_psd_layers`)。
fn classify_in(
    state: &AppState,
    psd: Option<String>,
    use_ai: bool,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<Classification>> {
    classify_guided_in(state, psd, use_ai, &Ask::default(), consent)
}

/// 注文なしの提案 (`propose_rig_from_psd`)。
fn propose_in(
    state: &AppState,
    use_ai: bool,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<ProposalView>> {
    propose_guided_in(state, use_ai, None, ModuleRigChoice::default(), consent)
}

fn done<T>(reply: AiReply<T>) -> T {
    match reply {
        AiReply::Done { value } => value,
        AiReply::NeedsConsent { .. } => panic!("expected no consent to be needed"),
    }
}

/// AI を使わない経路 (名前の規則 → テンプレートの既定値) は何も送らず、キーがなくても動く。
/// 提案は置くだけでドキュメントを変えず、採用は 1 回の Undo で戻る。
#[test]
fn rules_path_proposes_without_sending() {
    let state = from_psd(None);
    let classification = done(classify_in(&state, None, false, None).unwrap());
    assert!(!classification.by_ai);
    assert_eq!(classification.psd, bustup_path().display().to_string());
    assert!(classification.layers.iter().any(|l| l.label == "face"));

    let base = state.document.lock().unwrap().state_id();
    let view = done(propose_in(&state, false, None).unwrap());
    assert!(view.title.ends_with("— rules"), "{}", view.title);
    assert!(!view.commands.is_empty());
    assert!(!view.stale);
    let mut document = state.document.lock().unwrap();
    assert_eq!(document.state_id(), base);
    assert!(document.model().parameters.is_empty());

    let mut session = state.project.lock().unwrap();
    let panel = session.proposals.state(&document);
    assert_eq!(panel.proposals.len(), 1);
    assert!(panel.classification.is_some());
    session
        .proposals
        .proposals
        .adopt(&mut document, view.id, &[])
        .unwrap();
    assert!(
        document
            .model()
            .parameters
            .iter()
            .any(|p| p.id.as_str() == "AngleX")
    );
    assert!(document.undo());
    assert!(document.model().parameters.is_empty());
    assert_eq!(document.state_id(), base);
}

/// AI を使う経路は、同意がなければ送らずに送る内容を返す。分類の結果・提案は変わらない。
#[test]
fn ai_path_asks_before_sending() {
    let state = from_psd(Some("sk-test"));
    let AiReply::NeedsConsent { disclosure } = classify_in(&state, None, true, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    // 合成画像と、レイヤーごとのサムネイル (32 枚ごとに合成画像を添えて分ける)。
    assert!(disclosure.image_count > 1, "{disclosure:?}");
    assert_eq!(disclosure.retries, 0);
    assert!(
        state
            .project
            .lock()
            .unwrap()
            .proposals
            .classification
            .is_none()
    );

    done(classify_in(&state, None, false, None).unwrap());
    let AiReply::NeedsConsent { disclosure } = propose_in(&state, true, None).unwrap() else {
        panic!("expected to ask for consent");
    };
    assert_eq!(disclosure.image_count, 1);
    assert_eq!(disclosure.retries, DEFAULT_MAX_ATTEMPTS - 1);
    assert!(state.project.lock().unwrap().proposals.proposals.is_empty());
}

/// 「外部に送らない」のプロジェクトでは AI の経路を断る (名前の規則は使える)。
#[test]
fn disabled_projects_do_not_send() {
    let state = from_psd(Some("sk-test"));
    state.project.lock().unwrap().set_ai_send_externally(false);
    assert_eq!(
        classify_in(&state, None, true, None).unwrap_err().code,
        "ai.send_disabled"
    );
    done(classify_in(&state, None, false, None).unwrap());
    assert_eq!(
        propose_in(&state, true, None).unwrap_err().code,
        "ai.send_disabled"
    );
}

/// API キーがなければ、送る前に断る (案内の文言は `ApiKeys::resolve`)。
#[test]
fn missing_key_is_reported_before_sending() {
    let state = from_psd(None);
    let err = classify_in(&state, None, true, None).unwrap_err();
    assert!(err.to_string().contains("API key"), "{err}");
}

#[test]
fn inputs_are_required() {
    let state = state(None);
    assert_eq!(
        classify_in(&state, None, false, None).unwrap_err().code,
        "proposal.no_psd"
    );
    let err = propose_in(&state, false, None).unwrap_err();
    assert!(err.to_string().contains("classify"), "{err}");
    let err = classify_in(&state, Some("relative.psd".into()), false, None).unwrap_err();
    assert!(err.to_string().contains("absolute"), "{err}");
}

/// 顔のないモデル (分類した PSD と違うモデル) では提案せず、提案の一覧も変わらない。
#[test]
fn a_model_without_the_classified_layers_is_rejected() {
    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    *state.document.lock().unwrap() = Document::new(ugoe_model::fixtures::single_quad());
    assert!(propose_in(&state, false, None).is_err());
    assert!(state.project.lock().unwrap().proposals.proposals.is_empty());
}

/// 別の PSD を選ぶと前の分類を捨てる。同じ PSD なら残す。
#[test]
fn choosing_another_psd_drops_the_classification() {
    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let mut session = state.project.lock().unwrap();
    session.proposals.set_psd(bustup_path());
    assert!(session.proposals.classification.is_some());
    session
        .proposals
        .set_psd(PathBuf::from("/elsewhere/other.psd"));
    assert!(session.proposals.classification.is_none());
}

/// AI の値で組む経路: 通らない答えは問い直し、理由にその旨と題名に `AI` を付ける。
#[test]
fn ai_rig_notes_retries() {
    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let model = state.document.lock().unwrap().model().clone();
    let labels = state
        .project
        .lock()
        .unwrap()
        .proposals
        .classification
        .clone()
        .unwrap()
        .labels;
    assert!(labels.iter().any(|l| l.label == Label::Face));
    let answer = |turn: f32| {
        json!({
            "turn_angle_scale": turn,
            "eye_close_line": 0.7,
            "eye_close_bend": 0.04,
            "mouth_open_height": 0.35,
            "reason": "test"
        })
    };
    let fake = FakeProvider::new();
    fake.push(answer(5.0)).push(answer(0.7));
    let proposal = rig(
        &Document::new(model),
        &labels,
        b"png",
        Some(&fake),
        &RigAsk::default(),
    )
    .unwrap()
    .proposal;
    assert_eq!(fake.calls().len(), 2);
    assert!(proposal.title.ends_with("— AI"), "{}", proposal.title);
    assert!(
        proposal.rationale.contains("Asked the AI again 1 time(s)"),
        "{}",
        proposal.rationale
    );
}

/// ADR 0022: 分類とリギングの提案は、選んだプロバイダ (ここでは OpenAI 互換の API の録画) でも同じ流れで動く。
#[test]
fn an_openai_compatible_provider_classifies_and_rigs() {
    use ugoe_ai::claude::ApiKey;
    use ugoe_ai::openai::{OPENAI_BASE_URL, OpenAiConfig, OpenAiProvider};
    use ugoe_ai::transport::{Cassette, ReplayTransport};

    let replay = |name: &str| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/ugoe-ai/tests/cassettes")
            .join(name);
        OpenAiProvider::with_transport(
            OpenAiConfig::new(OPENAI_BASE_URL, "gpt-6-sol"),
            Some(ApiKey::new("sk-test").unwrap()),
            ReplayTransport::new(Cassette::load(path).unwrap()),
        )
    };
    let tree = read_tree(&bustup_path()).unwrap();
    let labels = classify(
        &tree,
        Some(&replay("openai_classify_layers_bustup.json")),
        &ClassifyGuidance::default(),
    )
    .unwrap();
    assert!(labels.iter().any(|l| l.label == Label::Face));

    let state = from_psd(None);
    let model = state.document.lock().unwrap().model().clone();
    let image = overview(&tree).unwrap();
    let provider = replay("openai_auto_rig_retry.json");
    let proposal = rig(
        &Document::new(model),
        &labels,
        &image,
        Some(&provider),
        &RigAsk::default(),
    )
    .unwrap()
    .proposal;
    assert!(proposal.title.ends_with("— AI"), "{}", proposal.title);
    assert!(
        proposal.rationale.contains("Asked the AI again 1 time(s)"),
        "{}",
        proposal.rationale
    );
    let mut document = state.document.lock().unwrap();
    let view = state
        .project
        .lock()
        .unwrap()
        .proposals
        .proposals
        .add(
            &document,
            proposal.title,
            proposal.rationale,
            proposal.commands,
        )
        .unwrap();
    state
        .project
        .lock()
        .unwrap()
        .proposals
        .proposals
        .adopt(&mut document, view.id, &[])
        .unwrap();
    assert!(
        document
            .model()
            .parameters
            .iter()
            .any(|p| p.id.as_str() == "AngleX")
    );
}

/// 採用した後に手で直したセルは `authored` になる (提案が推定に任せた `derived` のセルでも)。
#[test]
fn cells_edited_after_adoption_become_authored() {
    use ugoe_editor_core::{Command, Keyform, KeyformTarget};
    use ugoe_model::{DeformerKind, KeyformOrigin};

    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let view = done(propose_in(&state, false, None).unwrap());
    let mut document = state.document.lock().unwrap();
    state
        .project
        .lock()
        .unwrap()
        .proposals
        .proposals
        .adopt(&mut document, view.id, &[])
        .unwrap();
    // 由来が `derived` 系のセルを持つワープを探す。
    let (id, index, cell, keyform) = document
        .model()
        .deformers
        .iter()
        .find_map(|d| {
            let DeformerKind::Warp(warp) = &d.kind else {
                return None;
            };
            let grid = &warp.keyforms;
            let index = (0..grid.keyforms.len()).find(|&i| grid.origin(i).is_derived())?;
            let mut rest = index;
            let cell = grid
                .bindings
                .iter()
                .map(|b| {
                    let key = b.keys[rest % b.keys.len()];
                    rest /= b.keys.len();
                    (b.parameter.clone(), key)
                })
                .collect();
            Some((d.id.clone(), index, cell, grid.keyforms[index].clone()))
        })
        .expect("the rig leaves derived cells to inference");
    let mut edited = keyform;
    edited.points[0].x += 1.0;
    document
        .apply(Command::SetKeyformCell {
            target: KeyformTarget::Deformer(id.clone()),
            cell,
            keyform: Keyform::Warp(edited),
        })
        .unwrap();
    let deformer = document
        .model()
        .deformers
        .iter()
        .find(|d| d.id == id)
        .unwrap();
    let DeformerKind::Warp(warp) = &deformer.kind else {
        unreachable!()
    };
    assert_eq!(warp.keyforms.origin(index), KeyformOrigin::Authored);
}

/// 実際のリグの提案で、どのコマンドを外しても、それに (直接・間接に) 依存するコマンドを一緒に外せば
/// プレビューできる (画面の一部だけの採用は `requires` でこの閉包を作る)。
#[test]
fn excluding_a_command_with_its_dependents_always_applies() {
    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let view = done(propose_in(&state, false, None).unwrap());
    let document = state.document.lock().unwrap();
    let session = state.project.lock().unwrap();
    for start in 0..view.commands.len() {
        let mut excluded = vec![start];
        let mut i = 0;
        while i < excluded.len() {
            let k = excluded[i];
            for (j, c) in view.commands.iter().enumerate() {
                if c.requires.contains(&k) && !excluded.contains(&j) {
                    excluded.push(j);
                }
            }
            i += 1;
        }
        if excluded.len() == view.commands.len() {
            continue;
        }
        if let Err(e) = session
            .proposals
            .proposals
            .preview(&document, view.id, &excluded)
        {
            panic!("excluding #{start} ({}): {e}", view.commands[start].op);
        }
    }
}

/// 注文は AI に送るときだけ添え、送る前の確認に文字数を示す。長すぎる注文は送る前に断る。
#[test]
fn instructions_are_disclosed_before_sending() {
    let state = from_psd(Some("sk-test"));
    let ask = Ask {
        instructions: Some("  帽子は headwear  "),
        revise: false,
    };
    let AiReply::NeedsConsent { disclosure } =
        classify_guided_in(&state, None, true, &ask, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    assert_eq!(disclosure.instruction_chars, 12);
    let AiReply::NeedsConsent { disclosure: plain } =
        classify_in(&state, None, true, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    assert_eq!(plain.instruction_chars, 0);
    assert!(disclosure.text_bytes > plain.text_bytes);

    // 分類の作り直しは、今の分類がなければ断る。
    let revise = Ask {
        instructions: Some("帽子は headwear"),
        revise: true,
    };
    let err = classify_guided_in(&state, None, true, &revise, None).unwrap_err();
    assert!(err.to_string().contains("classify"), "{err}");

    done(classify_in(&state, None, false, None).unwrap());
    let AiReply::NeedsConsent { disclosure } = propose_guided_in(
        &state,
        true,
        Some("振り向きは控えめに"),
        ModuleRigChoice::default(),
        None,
    )
    .unwrap() else {
        panic!("expected to ask for consent");
    };
    assert_eq!(disclosure.instruction_chars, 9);
    assert_eq!(disclosure.retries, DEFAULT_MAX_ATTEMPTS - 1);

    let long = "a".repeat(Instructions::MAX_CHARS + 1);
    let err =
        propose_guided_in(&state, true, Some(&long), ModuleRigChoice::default(), None).unwrap_err();
    assert!(err.to_string().contains("limit"), "{err}");
    // AI を使わないなら注文は送らない (長さも問わない)。
    let view = done(
        propose_guided_in(&state, false, Some(&long), ModuleRigChoice::default(), None).unwrap(),
    );
    assert_eq!(view.instructions, None);
}

/// このパネルで作った提案は作り直せる。作り直しはいつも AI に送り、前の答えと注文を確認に含める。
#[test]
fn proposals_made_here_can_be_regenerated() {
    let state = from_psd(Some("sk-test"));
    done(classify_in(&state, None, false, None).unwrap());
    let view = done(propose_in(&state, false, None).unwrap());
    let panel = {
        let document = state.document.lock().unwrap();
        state.project.lock().unwrap().proposals.state(&document)
    };
    assert_eq!(panel.regenerable, [view.id]);

    let AiReply::NeedsConsent { disclosure } = regenerate_in(
        &state,
        view.id,
        Some("目はもう少し細く閉じる"),
        ModuleRigChoice::default(),
        None,
    )
    .unwrap() else {
        panic!("expected to ask for consent");
    };
    assert_eq!(disclosure.instruction_chars, 11);
    assert_eq!(disclosure.image_count, 1);
    let err =
        regenerate_in(&state, view.id + 1, None, ModuleRigChoice::default(), None).unwrap_err();
    assert!(err.to_string().contains("cannot be regenerated"), "{err}");

    // 「外部に送らない」のプロジェクトでは作り直さない (AI なしの作り直しはない)。
    state.project.lock().unwrap().set_ai_send_externally(false);
    assert_eq!(
        regenerate_in(&state, view.id, None, ModuleRigChoice::default(), None)
            .unwrap_err()
            .code,
        "ai.send_disabled"
    );

    // 却下した提案は作り直せない。
    let panel = reject_in(&state, view.id).unwrap();
    assert!(panel.regenerable.is_empty());
    assert!(state.project.lock().unwrap().proposals.rigs.is_empty());
}

/// 注文を添えた作り直し (OpenAI 互換の API の録画): 新しい提案は前の提案の後ろに並び、作り直しの元・注文・
/// 反映できなかった注文を記す。前の提案は残り、プレビューと採用 (1 回の Undo) はどちらでもできる。
#[test]
fn a_regenerated_proposal_is_listed_after_the_original() {
    use ugoe_ai::claude::ApiKey;
    use ugoe_ai::openai::{OPENAI_BASE_URL, OpenAiConfig, OpenAiProvider};
    use ugoe_ai::transport::{Cassette, ReplayTransport};

    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let original = done(propose_in(&state, false, None).unwrap());
    let (model, base) = {
        let document = state.document.lock().unwrap();
        let session = state.project.lock().unwrap();
        (document.model().clone(), Base::of(&document, &session))
    };
    let (labels, previous) = {
        let session = state.project.lock().unwrap();
        (
            session.proposals.classification.clone().unwrap().labels,
            session.proposals.rigs[&original.id].clone(),
        )
    };
    let PreviousAnswer::Model(ref model_answer) = previous else {
        panic!("{previous:?}");
    };
    assert_eq!(model_answer.params, RigParams::default());

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/ugoe-ai/tests/cassettes/openai_rig_instructions.json");
    let transport = std::sync::Arc::new(ReplayTransport::new(Cassette::load(path).unwrap()));
    let provider = OpenAiProvider::with_transport(
        OpenAiConfig::new(OPENAI_BASE_URL, "gpt-6-sol"),
        Some(ApiKey::new("sk-test").unwrap()),
        transport.clone(),
    );
    let text = "振り向きは控えめに。髪はもっと揺らして。";
    let guidance = RigAsk {
        instructions: Instructions::new(text).unwrap(),
        previous: Some(previous),
        ..RigAsk::default()
    };
    let tree = read_tree(&bustup_path()).unwrap();
    let image = overview(&tree).unwrap();
    let rigged = rig(
        &Document::new(model),
        &labels,
        &image,
        Some(&provider),
        &guidance,
    )
    .unwrap();
    let sent = transport.received()[0].body.clone().unwrap().to_string();
    assert!(sent.contains("Your previous answer"), "{sent}");
    assert!(sent.contains("The template defaults"));
    let PreviousAnswer::Model(ref answer) = rigged.previous else {
        panic!("{:?}", rigged.previous);
    };
    assert_eq!(answer.params.turn_angle_scale, 0.5);
    let origin = ProposalOrigin {
        revises: Some(original.id),
        instructions: Some(text.into()),
        unapplied: rigged.unapplied.clone(),
    };
    let revised = place(&state, base, rigged, origin).unwrap();
    assert_eq!(revised.revises, Some(original.id));
    assert_eq!(revised.instructions.as_deref(), Some(text));
    assert_eq!(revised.unapplied.len(), 2);
    assert!(revised.unapplied[0].starts_with("make the hair sway more"));
    assert!(!revised.rationale.contains("Not applied"));

    let mut document = state.document.lock().unwrap();
    let mut session = state.project.lock().unwrap();
    let panel = session.proposals.state(&document);
    let ids: Vec<u32> = panel.proposals.iter().map(|p| p.id).collect();
    assert_eq!(ids, [original.id, revised.id]);
    assert_eq!(panel.regenerable, [original.id, revised.id]);
    session
        .proposals
        .proposals
        .preview(&document, original.id, &[])
        .unwrap();
    session
        .proposals
        .proposals
        .adopt(&mut document, revised.id, &[])
        .unwrap();
    assert!(document.undo());
    assert!(document.model().parameters.is_empty());
    assert_eq!(session.proposals.state(&document).proposals.len(), 2);
}

/// モジュールのプロジェクト: 本体 (見本の PSD にテンプレートのリグを組み、頭の回転に付くスロット `Headwear` を
/// 足したもの) に、帽子と耳飾りの衣装を取り込んだ作業用のドキュメント。分類は名前の規則。
fn module_state() -> (AppState, Vec<LayerLabel>) {
    use std::sync::Arc;
    use ugoe_editor_core::import::layer_tree_command;
    use ugoe_editor_core::{ModuleBase, ModuleContext, PsdImportOptions};
    use ugoe_model::module::{DrawOrderRange, ModuleRequirements};
    use ugoe_model::{Slot, SlotId};
    use ugoe_psd::{Layer, LayerNode};

    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let view = done(propose_in(&state, false, None).unwrap());
    let mut base = {
        let mut document = state.document.lock().unwrap();
        let mut session = state.project.lock().unwrap();
        session
            .proposals
            .proposals
            .adopt(&mut document, view.id, &[])
            .unwrap();
        document.detached()
    };
    base.apply(ugoe_editor_core::Command::AddSlot {
        slot: Slot {
            id: "Headwear".into(),
            anchor: Some("HeadRotation".into()),
            draw_order: DrawOrderRange {
                min: 960.0,
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
                path: "/base.ugoeproj".into(),
                model: base.model().clone(),
                physics: base.physics().cloned(),
                textures: base.textures().clone(),
            },
            &SlotId::from("Headwear"),
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
    let layer = |name: &str, x: f32, y: f32, w: u32, h: u32| {
        LayerNode::Layer(Layer {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            offset: ugoe_editor_core::space::Vec2::new(x, y),
            width: w,
            height: h,
            rgba: [200, 60, 60, 255].repeat((w * h) as usize),
            blend_mode: "norm".into(),
            id: None,
            fill_opacity: 1.0,
            clipping: false,
        })
    };
    // 顔 (見本の PSD では横 390〜640・縦 200〜520 ほど) の上の帽子と、左右の耳飾り。
    let tree = ugoe_psd::LayerTree {
        width: 1024,
        height: 1024,
        root: vec![
            layer("hat", 370.0, 130.0, 290, 140),
            layer("earring-r", 380.0, 420.0, 16, 90),
            layer("earring-l", 630.0, 420.0, 16, 90),
        ],
        warnings: Vec::new(),
    };
    let mut import =
        layer_tree_command(doc.model(), &tree, &PsdImportOptions::new("costume")).unwrap();
    context.map_new_meshes(&mut import.command);
    doc.apply(import.command).unwrap();
    *state.document.lock().unwrap() = doc;
    {
        let mut session = state.project.lock().unwrap();
        session.proposals = ProposalSession::default();
        session.proposals.classification = Some(Classified {
            psd: PathBuf::from("/costume.psd"),
            labels: classify_by_name(&tree),
            by_ai: false,
            instructions: None,
        });
    }
    (state, classify_by_name(&tree))
}

/// モジュールのプロジェクトでは、パラメータを足さないモジュール向けの提案を作り、採用できる (本体は変わらない)。
/// 作り直しはモジュールの前の答えを送り、モデルの前の答えは断る。
#[test]
fn module_projects_get_module_rigs() {
    let (state, labels) = module_state();
    let view = done(propose_in(&state, false, None).unwrap());
    assert!(view.title.contains("module"), "{}", view.title);
    assert!(view.title.ends_with("— rules"), "{}", view.title);
    let parameters = state.document.lock().unwrap().model().parameters.clone();
    {
        let mut document = state.document.lock().unwrap();
        let mut session = state.project.lock().unwrap();
        session
            .proposals
            .proposals
            .adopt(&mut document, view.id, &[])
            .unwrap();
    }
    let document = state.document.lock().unwrap().detached();
    assert_eq!(document.model().parameters, parameters);
    let (module, _) = document.split_module().unwrap().unwrap();
    assert!(!module.physics.rigs.is_empty());
    assert!(
        module
            .deformers
            .iter()
            .any(|d| d.id.as_str() == "FollowFaceWarp")
    );

    // AI の経路 (作り直し): 前の答えを添える。
    let (state, _) = module_state();
    let original = done(propose_in(&state, false, None).unwrap());
    let previous = state.project.lock().unwrap().proposals.rigs[&original.id].clone();
    assert!(
        matches!(previous, PreviousAnswer::Module(_)),
        "{previous:?}"
    );
    let doc = state.document.lock().unwrap().detached();
    let fake = FakeProvider::new();
    fake.push(json!({ "sway_scale": 1.3, "sway_length": 1.0, "reason": "light earrings" }));
    let ask = RigAsk {
        instructions: Instructions::new("Swing the earrings more.").unwrap(),
        previous: Some(previous),
        ..RigAsk::default()
    };
    let rigged = rig(&doc, &labels, b"png", Some(&fake), &ask).unwrap();
    assert!(rigged.proposal.title.ends_with("— AI"));
    let calls = fake.calls();
    assert!(
        calls[0].0.prompt.contains("Your previous answer"),
        "{}",
        calls[0].0.prompt
    );
    assert!(calls[0].0.system.contains("cannot add parameters"));
    let PreviousAnswer::Module(answer) = rigged.previous else {
        panic!();
    };
    assert_eq!(answer.params.sway_scale, 1.3);

    let wrong = RigAsk {
        previous: Some(PreviousAnswer::Model(PreviousRig {
            params: RigParams::default(),
            reason: String::new(),
        })),
        ..RigAsk::default()
    };
    let err = rig(&doc, &labels, b"png", Some(&fake), &wrong).unwrap_err();
    assert!(err.to_string().contains("another kind of project"), "{err}");
}

/// モジュールのリグの組み方 (AI タブの選択) が提案に届く: 直接写す形では追従のワープを作らず、衣装の
/// メッシュのキーフォームに本体の顔のワープの軸が付く。
#[test]
fn module_rig_choice_reaches_the_proposal() {
    let (state, _) = module_state();
    let direct = ModuleRigChoice {
        direct: true,
        ..ModuleRigChoice::default()
    };
    let view = done(propose_guided_in(&state, false, None, direct, None).unwrap());
    {
        let mut document = state.document.lock().unwrap();
        let mut session = state.project.lock().unwrap();
        session
            .proposals
            .proposals
            .adopt(&mut document, view.id, &[])
            .unwrap();
    }
    let document = state.document.lock().unwrap().detached();
    let (module, _) = document.split_module().unwrap().unwrap();
    assert!(
        !module
            .deformers
            .iter()
            .any(|d| d.id.as_str().starts_with("Follow"))
    );
    let hat = module.meshes.iter().find(|m| m.name == "hat").unwrap();
    assert!(
        hat.keyforms
            .bindings
            .iter()
            .any(|b| b.parameter.as_str() == "AngleX")
    );
    assert_eq!(
        ModuleRigOptions::from(ModuleRigChoice::default()),
        ModuleRigOptions::default()
    );
}

/// モジュールのリグの提案を採用した後に、本体の変形を写した衣装のメッシュの形を編集すると、提案し直すよう知らせる
/// (T0521)。形を変えないキーフォームの編集では知らせず、知らせを消すと今の形を覚え直す。
#[test]
fn editing_a_rigged_costume_mesh_asks_to_propose_again() {
    let (state, _) = module_state();
    let direct = ModuleRigChoice {
        direct: true,
        ..ModuleRigChoice::default()
    };
    let view = done(propose_guided_in(&state, false, None, direct, None).unwrap());
    adopt_in(&state, view.id, &[]).unwrap();
    let stale = |state: &AppState| {
        let document = state.document.lock().unwrap();
        state
            .project
            .lock()
            .unwrap()
            .proposals
            .stale_rigs(&document)
    };
    assert!(stale(&state).is_empty());
    let hat = {
        let document = state.document.lock().unwrap();
        let context = document.module().unwrap();
        document
            .model()
            .meshes
            .iter()
            .find(|m| m.name == "hat" && !context.is_base_mesh(&m.id))
            .unwrap()
            .id
            .clone()
    };
    // 形を変えない編集 (カリング) では知らせない。
    state
        .document
        .lock()
        .unwrap()
        .apply(Command::SetMeshProperties {
            id: hat.clone(),
            part: None,
            texture: None,
            blend: None,
            culling: Some(true),
            masks: None,
            mask_mode: None,
            mask_only: None,
            lit: None,
        })
        .unwrap();
    assert!(stale(&state).is_empty());
    let start = state
        .document
        .lock()
        .unwrap()
        .model()
        .meshes
        .iter()
        .find(|m| m.id == hat)
        .unwrap()
        .uvs
        .len();
    state
        .document
        .lock()
        .unwrap()
        .apply(Command::RemoveMeshVertices {
            id: hat.clone(),
            indices: vec![u16::try_from(start - 1).unwrap()],
        })
        .unwrap();
    let found = stale(&state);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].name, "hat");
    let after = dismiss_stale_rigs_in(&state).unwrap();
    assert!(after.stale_rigs.is_empty());
    // 採用を取り消した (軸のない) メッシュは知らせない。
    state.document.lock().unwrap().undo();
    state.document.lock().unwrap().undo();
    state.document.lock().unwrap().undo();
    assert!(stale(&state).is_empty());
}

/// 本体の変形を写した衣装のメッシュの形の指紋は `project.json` の `editor.rigged_meshes` に残り、プロジェクトを
/// 開き直しても (同じメタ情報から作り直しても) 形を編集したことを知らせる。ID はファイルの中の ID (T0523)。
#[test]
fn rigged_costume_shapes_are_kept_in_the_project() {
    use crate::project::ProjectSession;

    let (state, _) = module_state();
    let direct = ModuleRigChoice {
        direct: true,
        ..ModuleRigChoice::default()
    };
    let view = done(propose_guided_in(&state, false, None, direct, None).unwrap());
    adopt_in(&state, view.id, &[]).unwrap();
    let project = {
        let document = state.document.lock().unwrap();
        let session = state.project.lock().unwrap();
        let context = document.module().unwrap();
        let hat = document
            .model()
            .meshes
            .iter()
            .find(|m| m.name == "hat" && !context.is_base_mesh(&m.id))
            .unwrap()
            .id
            .clone();
        let saved = session.editor_value(RIGGED_MESHES_KEY).unwrap().clone();
        let map = saved.as_object().unwrap();
        // 新しく取り込んだメッシュは接頭辞がなく、開き直すと付く (どちらでも同じ ID で覚える)。
        let file_id = context.display_id(hat.as_str());
        assert_eq!(map[file_id].as_str().unwrap().len(), 16, "{saved}");
        assert!(map.keys().all(|id| !id.starts_with("Costume/")), "{saved}");
        session.to_project(&document)
    };
    // 開き直した姿: 残したメタ情報から作り直したセッションで、頂点を消すと知らせる。
    let (mut session, mut document) = ProjectSession::from_project(project.clone(), None);
    assert!(session.proposals.stale_rigs(&document).is_empty());
    let hat = document
        .model()
        .meshes
        .iter()
        .find(|m| m.name == "hat" && !document.module().unwrap().is_base_mesh(&m.id))
        .unwrap()
        .id
        .clone();
    let last = document
        .model()
        .meshes
        .iter()
        .find(|m| m.id == hat)
        .unwrap()
        .uvs
        .len()
        - 1;
    document
        .apply(Command::RemoveMeshVertices {
            id: hat.clone(),
            indices: vec![u16::try_from(last).unwrap()],
        })
        .unwrap();
    let found = session.proposals.stale_rigs(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].mesh, hat.as_str());
    // 知らせを消すと覚え直した指紋がメタ情報に入る。
    session.proposals.forget_stale(&document);
    let rigged = session.proposals.rigged_value();
    session.set_editor_value(RIGGED_MESHES_KEY, rigged);
    assert!(session.proposals.stale_rigs(&document).is_empty());
    assert_ne!(
        session.editor_value(RIGGED_MESHES_KEY),
        project.meta.editor.get(RIGGED_MESHES_KEY)
    );
    // 項目のない (前からの) プロジェクトと、形の違う値は何も覚えない。
    let mut old = project.clone();
    old.meta.editor.remove(RIGGED_MESHES_KEY);
    let (session, _) = ProjectSession::from_project(old.clone(), None);
    assert!(session.proposals.rigged_value().is_none());
    old.meta
        .editor
        .insert(RIGGED_MESHES_KEY.into(), json!(["hat"]));
    let (session, _) = ProjectSession::from_project(old, None);
    assert!(session.proposals.rigged_value().is_none());
}

/// 法線マップの提案 (AI なし): まとまりごとに設定を選び、コマンドはまとまりごとに区切られる。まとまりを外して採用でき、
/// 1 回の Undo で戻る (T0520)。
#[test]
fn normal_maps_are_proposed_by_group() {
    use crate::lighting::normal_map_groups_in;
    use ugoe_editor_core::lighting::BulgeSettings;

    // テクスチャの画像が要るので、取り込んだドキュメントをそのまま使う。
    let state = state(None);
    let (bytes, options) =
        crate::psd::read_psd(&bustup_path().display().to_string(), None).unwrap();
    let (_, document, _) = crate::project::from_psd(&bytes, &options).unwrap();
    *state.document.lock().unwrap() = document;
    let groups = {
        let document = state.document.lock().unwrap();
        normal_map_groups_in(&document, None).unwrap()
    };
    let face = groups
        .iter()
        .find(|g| g.group.as_deref() == Some("face"))
        .unwrap();
    assert!(face.textures.len() > 5);
    // すべてのまとまりを既定で、顔だけ弱く。
    let choices: Vec<NormalMapGroupChoice> = groups
        .iter()
        .map(|g| NormalMapGroupChoice {
            group: g.group.clone(),
            settings: if g.group.as_deref() == Some("face") {
                BulgeSettings {
                    strength: 0.1,
                    ..g.defaults
                }
            } else {
                g.defaults
            },
        })
        .collect();
    let base = state.document.lock().unwrap().state_id();
    let view = propose_normal_maps_in(&state, &choices, false).unwrap();
    assert!(view.title.ends_with("— no AI"), "{}", view.title);
    assert!(
        view.rationale.contains("strength 0.1"),
        "{}",
        view.rationale
    );
    assert_eq!(state.document.lock().unwrap().state_id(), base);

    let panel = {
        let document = state.document.lock().unwrap();
        state.project.lock().unwrap().proposals.state(&document)
    };
    let sections = &panel
        .sections
        .iter()
        .find(|s| s.id == view.id)
        .unwrap()
        .sections;
    let labels: Vec<&str> = sections.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels.first(), Some(&"face"));
    assert_eq!(labels.last(), Some(&"lighting"));
    let covered: usize = sections.iter().map(|s| s.indices.len()).sum();
    assert_eq!(covered, view.commands.len());
    let face_section = &sections[0];
    assert_eq!(face_section.indices.len(), face.textures.len());
    // コマンドの対象はテクスチャの名前 (`proposal::analyze`)。
    let mut subjects: Vec<String> = face_section
        .indices
        .iter()
        .map(|&i| view.commands[i].subject.clone().unwrap())
        .collect();
    subjects.sort();
    let mut textures = face.textures.clone();
    textures.sort();
    assert_eq!(subjects, textures);

    // 顔のまとまりを外して採用する。
    let excluded = face_section.indices.clone();
    {
        let mut document = state.document.lock().unwrap();
        let mut session = state.project.lock().unwrap();
        session
            .proposals
            .proposals
            .adopt(&mut document, view.id, &excluded)
            .unwrap();
        let model = document.model();
        assert!(model.lighting);
        for t in &model.textures {
            if face.textures.contains(&t.name) {
                assert!(t.normal_map.is_none(), "{}", t.name);
            }
        }
        assert!(model.textures.iter().any(|t| t.normal_map.is_some()));
        assert!(document.undo());
        assert_eq!(document.state_id(), base);
    }

    // 選ばないと断る。知らないまとまりも断る。
    assert!(propose_normal_maps_in(&state, &[], false).is_err());
    let unknown = NormalMapGroupChoice {
        group: Some("wings".into()),
        settings: BulgeSettings::default(),
    };
    assert!(propose_normal_maps_in(&state, &[unknown], false).is_err());
}

/// 提案を作る間に別のプロジェクト (ここでは同じモデルの写し) に替わったら、提案を置かない (開いた直後のドキュメントは
/// どれも状態の番号が同じなので、番号だけでは気づけない)。
#[test]
fn proposals_made_for_another_project_are_not_placed() {
    let state = from_psd(None);
    done(classify_in(&state, None, false, None).unwrap());
    let (base, document) = {
        let document = state.document.lock().unwrap();
        let session = state.project.lock().unwrap();
        (Base::of(&document, &session), document.detached())
    };
    let labels = state
        .project
        .lock()
        .unwrap()
        .proposals
        .layer_labels()
        .unwrap();
    let rigged = rig(&document, &labels, &[], None, &RigAsk::default()).unwrap();
    // 別のプロジェクトを開く (`open_project` と同じ置き換え)。
    let (session, _) = crate::project::initial_state();
    *state.document.lock().unwrap() = Document::new(document.model().clone());
    *state.project.lock().unwrap() = session;
    let origin = ProposalOrigin {
        revises: None,
        instructions: None,
        unapplied: Vec::new(),
    };
    let err = place(&state, base, rigged, origin).unwrap_err();
    assert_eq!(err.code, "proposal.model_changed");
    assert!(state.project.lock().unwrap().proposals.proposals.is_empty());
}

/// 法線マップを作るコマンド (`generate_texture_normal_map`) は複製から作り、適用だけをロックの中で行う。1 回の Undo で戻る。
/// 作る間にモデルが変われば適用しない (T0685)。
#[test]
fn texture_normal_maps_are_made_from_a_copy() {
    use crate::lighting::generate_texture_normal_map_in;

    let state = state(None);
    let (bytes, options) =
        crate::psd::read_psd(&bustup_path().display().to_string(), None).unwrap();
    let (_, document, _) = crate::project::from_psd(&bytes, &options).unwrap();
    *state.document.lock().unwrap() = document;
    let texture = state.document.lock().unwrap().model().textures[0]
        .name
        .clone();
    let report = generate_texture_normal_map_in(&state, &texture, None, false).unwrap();
    assert!(report.warnings.is_empty());
    {
        let mut document = state.document.lock().unwrap();
        let made = |d: &Document| {
            d.model()
                .textures
                .iter()
                .find(|t| t.name == texture)
                .unwrap()
                .normal_map
                .is_some()
        };
        assert!(made(&document));
        assert!(document.undo());
        assert!(!made(&document));
    }

    // 作り始めた後でドキュメントが変わった (つないだエージェントの編集など) なら、作った編集は適用しない。
    let base = {
        let document = state.document.lock().unwrap();
        let session = state.project.lock().unwrap();
        Base::of(&document, &session)
    };
    assert!(state.document.lock().unwrap().redo());
    let err = {
        let document = state.document.lock().unwrap();
        let session = state.project.lock().unwrap();
        base.check_edit(&document, &session).unwrap_err()
    };
    assert_eq!(err.code, "editor.model_changed");
}

/// PSD の取り込み (`import_psd`) はモデルの写しに対して作り、適用は 1 回の Undo で戻る。取り込んだ PSD を覚える (T0685)。
#[test]
fn psd_imports_are_made_from_a_copy() {
    let state = state(None);
    let path = bustup_path().display().to_string();
    crate::psd::import_psd_in(&state, &path, None).unwrap();
    let mut document = state.document.lock().unwrap();
    assert!(!document.model().meshes.is_empty());
    assert_eq!(
        state.project.lock().unwrap().proposals.psd.as_deref(),
        Some(bustup_path().as_path())
    );
    assert!(document.undo());
    assert!(document.model().meshes.is_empty());
}
