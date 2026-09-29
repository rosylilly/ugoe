//! `proposals.rs` のテスト。ネットワークは使わない (AI の経路は送る前の確認までと、[`FakeProvider`] で確かめる)。
//! 入力は `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd`。

use std::sync::OnceLock;

use serde_json::json;
use ugoe_ai::FakeProvider;
use ugoe_ai::classify::Label;

use super::*;
use crate::ai::tests::state;
use crate::ai::{AiReply, SEND_DISABLED};

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
        classify_in(&state, None, true, None).unwrap_err(),
        SEND_DISABLED
    );
    done(classify_in(&state, None, false, None).unwrap());
    assert_eq!(propose_in(&state, true, None).unwrap_err(), SEND_DISABLED);
}

/// API キーがなければ、送る前に断る (案内の文言は `ApiKeys::resolve`)。
#[test]
fn missing_key_is_reported_before_sending() {
    let state = from_psd(None);
    let err = classify_in(&state, None, true, None).unwrap_err();
    assert!(err.contains("API key"), "{err}");
}

#[test]
fn inputs_are_required() {
    let state = state(None);
    assert_eq!(classify_in(&state, None, false, None).unwrap_err(), NO_PSD);
    let err = propose_in(&state, false, None).unwrap_err();
    assert!(err.contains("classify"), "{err}");
    let err = classify_in(&state, Some("relative.psd".into()), false, None).unwrap_err();
    assert!(err.contains("absolute"), "{err}");
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
    let proposal = rig(&model, &labels, b"png", Some(&fake)).unwrap();
    assert_eq!(fake.calls().len(), 2);
    assert!(proposal.title.ends_with("— AI"), "{}", proposal.title);
    assert!(
        proposal.rationale.contains("1 回分"),
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
    let labels = classify(&tree, Some(&replay("openai_classify_layers_bustup.json"))).unwrap();
    assert!(labels.iter().any(|l| l.label == Label::Face));

    let state = from_psd(None);
    let model = state.document.lock().unwrap().model().clone();
    let image = overview(&tree).unwrap();
    let provider = replay("openai_auto_rig_retry.json");
    let proposal = rig(&model, &labels, &image, Some(&provider)).unwrap();
    assert!(proposal.title.ends_with("— AI"), "{}", proposal.title);
    assert!(
        proposal.rationale.contains("1 回分"),
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
