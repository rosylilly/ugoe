//! レイヤーの意味分類 (`ugoe_ai::classify`) のテスト。ネットワークは使わない。
//!
//! - 名前の規則: `assets/fixtures/psd-real/` の See-Through の PSD に正解のラベルを付けて比べる。
//! - AI の経路: [`FakeProvider`] と、録画 (`tests/cassettes/classify_layers_bustup.json`、
//!   Messages API の応答の形に合わせて手で作ったもの) を再生する [`ClaudeProvider`]。同じ答えを
//!   Chat Completions の形にした録画 (`openai_classify_layers_bustup.json`) を再生する [`OpenAiProvider`]。
//!   同じ録画で、利用者の注文と前の分類を添えた作り直しも確かめる。

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use ugoe_ai::classify::{
    ClassifyGuidance, ClassifyOptions, Label, LayerLabel, Side, apply_answer,
    classification_instructions, classification_requests, classification_requests_guided,
    classify_by_name, classify_layers, classify_layers_guided, layer_thumbnail_png, layers,
    output_schema,
};
use ugoe_ai::claude::{ApiKey, ClaudeConfig, ClaudeProvider};
use ugoe_ai::instructions::Instructions;
use ugoe_ai::openai::{OPENAI_BASE_URL, OpenAiConfig, OpenAiProvider};
use ugoe_ai::transport::{Cassette, ReplayTransport};
use ugoe_ai::{AiError, FakeProvider};
use ugoe_core::Vec2;
use ugoe_psd::{Layer, LayerNode, LayerTree};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn psd(name: &str) -> LayerTree {
    let path = repo_path("../../assets/fixtures/psd-real").join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    ugoe_psd::import(&bytes).unwrap()
}

fn bustup() -> LayerTree {
    psd("comfyui-see-through-bustup-parts.psd")
}

fn full_body() -> LayerTree {
    psd("comfyui-see-through-full-body-parts.psd")
}

/// レイヤー名 (パスの最後) → (ラベル, 左右) の対応に並べ替える。
fn summary(labels: &[LayerLabel]) -> Vec<(String, Label, Side)> {
    labels
        .iter()
        .map(|l| (l.layer_path.last().unwrap().clone(), l.label, l.side))
        .collect()
}

fn expect(rows: &[(&str, Label, Side)]) -> Vec<(String, Label, Side)> {
    rows.iter()
        .map(|(n, l, s)| ((*n).to_owned(), *l, *s))
        .collect()
}

use Label::*;
use Side::{Center as C, Left as L, Right as R};

/// 胸から上の PSD の正解 (描画順)。名前の `-l` / `-r` はキャラクター自身の左右
/// (`ears-r` は画像の左にある)。
fn bustup_truth() -> Vec<(String, Label, Side)> {
    expect(&[
        ("back hair", HairBack, C),
        ("handwear-l", Handwear, L),
        ("handwear-r", Handwear, R),
        ("topwear", Topwear, C),
        ("neck", Neck, C),
        ("ears-r", Ear, R),
        ("earwear-r", Earwear, R),
        ("ears-l", Ear, L),
        ("earwear-l", Earwear, L),
        ("face", Face, C),
        ("mouth-open", MouthOpen, C),
        ("mouth-close", MouthClosed, C),
        ("nose", Nose, C),
        ("eyewhite-r", EyeWhite, R),
        ("irides-r", Iris, R),
        ("eyelash-r", EyelashUpper, R),
        ("eyebrow-r", Eyebrow, R),
        ("eyeclose-r", EyeClosed, R),
        ("eyewhite-l", EyeWhite, L),
        ("irides-l", Iris, L),
        ("eyelash-l", EyelashUpper, L),
        ("eyebrow-l", Eyebrow, L),
        ("eyeclose-l", EyeClosed, L),
        ("front hair", HairFront, C),
    ])
}

#[test]
fn names_classify_the_bustup_psd() {
    let labels = classify_by_name(&bustup());
    assert_eq!(summary(&labels), bustup_truth());
    assert_eq!(labels[13].layer_path, ["face", "eye-r", "eyewhite-r"]);
    assert!(labels.iter().all(|l| l.confidence >= 0.9));
}

#[test]
fn names_classify_the_full_body_psd() {
    let labels = classify_by_name(&full_body());
    let truth = expect(&[
        ("back hair", HairBack, C),
        ("topwear", Topwear, C),
        ("handwear-r", Handwear, R),
        // 両脚・両足が 1 枚なので中央
        ("legwear", Legwear, C),
        ("neck", Neck, C),
        ("footwear", Footwear, C),
        ("handwear-l", Handwear, L),
        ("bottomwear", Bottomwear, C),
        ("earwear", Earwear, C),
        ("eyebrow-l", Eyebrow, L),
        ("ears-r", Ear, R),
        ("face", Face, C),
        ("eyebrow-r", Eyebrow, R),
        ("ears-l", Ear, L),
        ("nose", Nose, C),
        ("mouth", Mouth, C),
        ("eyelash-r", EyelashUpper, R),
        ("eyelash-l", EyelashUpper, L),
        ("eyewhite-r", EyeWhite, R),
        ("eyewhite-l", EyeWhite, L),
        ("irides-r", Iris, R),
        ("irides-l", Iris, L),
        ("front hair", HairFront, C),
    ]);
    assert_eq!(summary(&labels), truth);
}

fn layer(name: &str, x: f32, w: u32) -> LayerNode {
    LayerNode::Layer(Layer {
        name: name.into(),
        visible: true,
        opacity: 1.0,
        offset: Vec2::new(x, 10.0),
        width: w,
        height: 10,
        rgba: [200, 100, 50, 255].repeat((w * 10) as usize),
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

/// 日本語の名前・グループからの継承・位置による左右を含む小さな木。顔の中心は x = 50。
fn small_tree() -> LayerTree {
    LayerTree {
        width: 200,
        height: 100,
        root: vec![
            layer("顔", 20.0, 60),
            group(
                "目",
                vec![layer("レイヤー 1", 60.0, 10), layer("レイヤー 2", 25.0, 10)],
            ),
            layer("白目_右", 25.0, 10),
            layer("効果", 0.0, 200),
            layer("empty", 0.0, 0),
        ],
        warnings: Vec::new(),
    }
}

#[test]
fn names_inherit_from_groups_and_use_position_for_sides() {
    let labels = classify_by_name(&small_tree());
    assert_eq!(
        summary(&labels),
        expect(&[
            ("顔", Face, C),
            // 名前に左右がない目は、顔の中心より画像の右ならキャラクターの左
            ("レイヤー 1", Eye, L),
            ("レイヤー 2", Eye, R),
            ("白目_右", EyeWhite, R),
            ("効果", Other, C),
            ("empty", Other, C),
        ])
    );
    // グループから受け継いだものは確からしさを下げ、分からないものは 0
    assert!(labels[1].confidence < labels[0].confidence);
    assert_eq!(labels[4].confidence, 0.0);
    let options = ClassifyOptions::default();
    assert!(!labels[0].needs_review(&options));
    assert!(labels[4].needs_review(&options));
}

/// 外部のエージェントに分類を任せるときの部品: 番号の並び・サムネイル・指示・答えの書き込み。
#[test]
fn layers_and_answers_for_external_agents() {
    let tree = small_tree();
    let labels = classify_by_name(&tree);
    let infos = layers(&tree);
    assert_eq!(infos.len(), labels.len());
    for (info, label) in infos.iter().zip(&labels) {
        assert_eq!(info.layer_path, label.layer_path);
    }
    assert_eq!(infos[1].layer_path, ["目", "レイヤー 1"]);
    assert_eq!(infos[1].bbox, [60.0, 10.0, 10.0, 10.0]);
    assert!(infos[1].has_pixels && !infos[5].has_pixels);

    // 小さなレイヤーは min_side まで拡大する (API に送るものと同じ作り方)。
    let options = ClassifyOptions::default();
    let png = layer_thumbnail_png(&tree, 1, &options).unwrap().unwrap();
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    assert_eq!((be(16), be(20)), (70, 70));
    assert!(layer_thumbnail_png(&tree, 5, &options).unwrap().is_none());
    assert!(matches!(
        layer_thumbnail_png(&tree, 6, &options),
        Err(AiError::InvalidRequest(_))
    ));

    let instructions = classification_instructions();
    assert!(instructions.contains("eye_white: white of the eye (sclera)"));
    assert_eq!(Label::EyeWhite.description(), "white of the eye (sclera)");

    // 答えにないレイヤーはそのまま。confidence は丸める。
    let mut next = labels.clone();
    let written = apply_answer(
        &mut next,
        json!({ "layers": [
            { "index": 3, "label": "cheek", "side": "left", "confidence": 1.4 },
            { "index": 1, "label": "eye_closed", "side": "left", "confidence": 0.8 },
        ] }),
    )
    .unwrap();
    assert_eq!(written, 2);
    assert_eq!(
        (next[3].label, next[3].side, next[3].confidence),
        (Cheek, L, 1.0)
    );
    assert_eq!(next[1].label, EyeClosed);
    assert_eq!(next[0], labels[0]);

    // 誤りがあれば何も書かない。
    for bad in [
        json!({ "layers": [{ "index": 0, "label": "face", "side": "center", "confidence": 1 },
                           { "index": 9, "label": "face", "side": "center", "confidence": 1 }] }),
        json!({ "layers": [{ "index": 0, "label": "face", "side": "center", "confidence": 1 },
                           { "index": 0, "label": "ear", "side": "left", "confidence": 1 }] }),
        json!({ "layers": [{ "index": 0, "label": "halo", "side": "center", "confidence": 1 }] }),
    ] {
        let mut copy = labels.clone();
        let err = apply_answer(&mut copy, bad).unwrap_err();
        assert!(matches!(err, AiError::InvalidResponse(_)), "{err}");
        assert_eq!(copy, labels);
    }
}

#[test]
fn schema_lists_the_vocabulary_and_forbids_extra_properties() {
    let schema = output_schema();
    let item = &schema["properties"]["layers"]["items"];
    let labels = item["properties"]["label"]["enum"].as_array().unwrap();
    assert_eq!(labels.len(), Label::ALL.len());
    assert_eq!(labels[6], "eye_white");
    assert_eq!(
        item["properties"]["side"]["enum"],
        json!(["left", "right", "center"])
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(item["additionalProperties"], false);
    // 語彙の名前は JSON の名前と往復する
    for l in Label::ALL {
        assert_eq!(l.as_str().parse::<Label>().unwrap(), l);
        assert_eq!(serde_json::to_value(l).unwrap(), json!(l.as_str()));
    }
}

#[test]
fn requests_are_split_and_keep_images_small() {
    let tree = bustup();
    let options = ClassifyOptions {
        max_layers_per_request: 10,
        ..Default::default()
    };
    let requests = classification_requests(&tree, &options).unwrap();
    // 24 枚を 10・10・4 に分け、それぞれに全体の合成画像を付ける
    let counts: Vec<usize> = requests.iter().map(|r| r.images.len()).collect();
    assert_eq!(counts, [11, 11, 5]);
    for r in &requests {
        assert!(r.system.contains("character's OWN left"));
        for png in &r.images {
            let decoder = png::Decoder::new(std::io::Cursor::new(png.as_slice()));
            let info = decoder.read_info().unwrap();
            let (w, h) = (info.info().width, info.info().height);
            assert!(w.max(h) <= 1024, "{w}x{h}");
        }
    }
    assert!(
        requests[1]
            .prompt
            .contains("\n10. name: \"face / mouth / mouth-open\"")
    );
    // 番号は全体の中の番号のまま (2 回目は 10 から)
    assert!(!requests[1].prompt.contains("\n9. "));
    // 既定では 1 回に収まる
    let one = classification_requests(&tree, &ClassifyOptions::default()).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].images.len(), 25);
}

#[test]
fn provider_answers_override_names_and_gaps_fall_back() {
    let tree = small_tree();
    let fake = FakeProvider::new();
    fake.push(json!({ "layers": [
        { "index": 4, "label": "cheek", "side": "center", "confidence": 1.7 },
        { "index": 1, "label": "iris", "side": "left", "confidence": 0.8 }
    ]}));
    let labels = classify_layers(&fake, &tree, &ClassifyOptions::default()).unwrap();
    assert_eq!(labels[1].label, Iris);
    assert_eq!(labels[4].label, Cheek);
    // 範囲外の確からしさは丸める
    assert_eq!(labels[4].confidence, 1.0);
    // 答えのないレイヤーは名前の規則のまま
    assert_eq!(labels[0].label, Face);
    assert_eq!(labels[3].label, EyeWhite);

    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    let (request, schema) = &calls[0];
    assert_eq!(schema, &output_schema());
    // 画素のない `empty` は送らない: 合成画像 + 5 枚
    assert_eq!(request.images.len(), 6);
    assert!(!request.prompt.contains("empty"));
}

#[test]
fn invalid_answers_are_rejected() {
    let tree = small_tree();
    for (answer, needle) in [
        (
            json!({ "layers": [{ "index": 5, "label": "eye", "side": "left", "confidence": 1 }] }),
            "not in the request",
        ),
        (
            json!({ "layers": [{ "index": -1, "label": "eye", "side": "left", "confidence": 1 }] }),
            "not in the request",
        ),
        (
            json!({ "layers": [
                { "index": 1, "label": "eye", "side": "left", "confidence": 1 },
                { "index": 1, "label": "iris", "side": "left", "confidence": 1 }
            ] }),
            "twice",
        ),
        (
            json!({ "layers": [{ "index": 1, "label": "wing", "side": "left", "confidence": 1 }] }),
            "schema",
        ),
    ] {
        let fake = FakeProvider::new();
        fake.push(answer);
        let err = classify_layers(&fake, &tree, &ClassifyOptions::default()).unwrap_err();
        assert!(
            matches!(&err, AiError::InvalidResponse(m) if m.contains(needle)),
            "{err}"
        );
    }
}

#[test]
fn provider_errors_are_passed_through() {
    let fake = FakeProvider::new();
    fake.push_result(Err(AiError::Truncated));
    let err = classify_layers(&fake, &small_tree(), &ClassifyOptions::default()).unwrap_err();
    assert!(matches!(err, AiError::Truncated));
}

#[test]
fn replays_a_recorded_claude_answer_for_the_bustup_psd() {
    let cassette =
        Cassette::load(repo_path("tests/cassettes/classify_layers_bustup.json")).unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport.clone(),
    );
    let labels = classify_layers(&provider, &bustup(), &ClassifyOptions::default()).unwrap();
    assert_eq!(transport.remaining(), 0);
    assert_eq!(summary(&labels), bustup_truth());

    // 録画の答えの確からしさを使い、低いものは確認に回る
    let options = ClassifyOptions::default();
    let review: Vec<&str> = labels
        .iter()
        .filter(|l| l.needs_review(&options))
        .map(|l| l.layer_path.last().unwrap().as_str())
        .collect();
    assert_eq!(review, ["eyeclose-r"]);
    // 録画が答えなかった前髪は名前の規則で補う
    assert_eq!(labels[23].label, HairFront);
    assert!((labels[23].confidence - 0.9).abs() < 1e-6);

    // 送ったもの: 合成画像 + 24 枚のサムネイル、分類のスキーマ
    let received = transport.received();
    assert_eq!(received.len(), 1);
    let body = received[0].body.as_ref().unwrap();
    let content = body["messages"][0]["content"].as_array().unwrap();
    let images = content.iter().filter(|b| b["type"] == "image").count();
    assert_eq!(images, 25);
    assert_eq!(body["output_config"]["format"]["schema"], output_schema());
    assert!(
        content.last().unwrap()["text"]
            .as_str()
            .unwrap()
            .contains("23. name: \"face / front hair\"")
    );
}

/// ADR 0022: 分類はプロバイダに依存しない。OpenAI 互換の API の録画でも同じ結果になる。
#[test]
fn replays_a_recorded_openai_compatible_answer_for_the_bustup_psd() {
    let cassette = Cassette::load(repo_path(
        "tests/cassettes/openai_classify_layers_bustup.json",
    ))
    .unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = OpenAiProvider::with_transport(
        OpenAiConfig::new(OPENAI_BASE_URL, "gpt-6-sol"),
        Some(ApiKey::new("sk-test").unwrap()),
        transport.clone(),
    );
    let labels = classify_layers(&provider, &bustup(), &ClassifyOptions::default()).unwrap();
    assert_eq!(transport.remaining(), 0);
    assert_eq!(summary(&labels), bustup_truth());

    // 送ったもの: 合成画像 + 24 枚のサムネイル (data URL)、分類のスキーマ (strict)
    let received = transport.received();
    assert_eq!(received.len(), 1);
    let body = received[0].body.as_ref().unwrap();
    assert_eq!(body["messages"][0]["role"], "system");
    let content = body["messages"][1]["content"].as_array().unwrap();
    let images = content.iter().filter(|b| b["type"] == "image_url").count();
    assert_eq!(images, 25);
    let format = &body["response_format"]["json_schema"];
    assert_eq!(format["schema"], output_schema());
    assert_eq!(format["strict"], true);
}

/// 作り直しの入力: 前の分類 (前髪を誤って `headwear` にしたもの) と注文。
fn revision(tree: &LayerTree) -> ClassifyGuidance {
    let mut previous = classify_by_name(tree);
    previous[23].label = Label::Headwear;
    previous[23].confidence = 0.4;
    ClassifyGuidance {
        instructions: Instructions::new("前髪は hair_front。目はもう少し細く閉じる。").unwrap(),
        previous: Some(previous),
    }
}

/// 注文と前の分類を添えて送ったリクエストを確かめる (出力の形は変えない)。
fn check_guided_request(system: &str, prompt: &str, schema: &serde_json::Value) {
    assert!(system.contains("This is a revision."));
    assert!(system.contains("<user_instructions> tags"));
    assert!(system.contains("follow the parts about labels and sides"));
    assert!(prompt.contains("23. name: \"face / front hair\"; bbox: "));
    assert!(prompt.contains("; previous: headwear (center)"));
    assert!(prompt.contains("<user_instructions>\n\"前髪は hair_front。"));
    assert_eq!(schema, &output_schema());
}

#[test]
fn a_claude_revision_sends_the_instructions_and_the_previous_labels() {
    let tree = bustup();
    let cassette =
        Cassette::load(repo_path("tests/cassettes/classify_layers_bustup.json")).unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport.clone(),
    );
    let guidance = revision(&tree);
    let labels =
        classify_layers_guided(&provider, &tree, &ClassifyOptions::default(), &guidance).unwrap();
    // 録画の答えが前の分類を直す。答えのない前髪は前の分類のまま (名前の規則ではない)。
    let mut expected = bustup_truth();
    expected[23].1 = Headwear;
    assert_eq!(summary(&labels), expected);
    assert!((labels[23].confidence - 0.4).abs() < 1e-6);

    let body = transport.received()[0].body.clone().unwrap();
    let content = body["messages"][0]["content"].as_array().unwrap();
    check_guided_request(
        body["system"].as_str().unwrap(),
        content.last().unwrap()["text"].as_str().unwrap(),
        &body["output_config"]["format"]["schema"],
    );
    // 送る前の確認に使うリクエストも同じ文。
    let requests =
        classification_requests_guided(&tree, &ClassifyOptions::default(), &guidance).unwrap();
    assert_eq!(requests[0].system, body["system"].as_str().unwrap());
    // 注文も前の分類もなければ、前と同じリクエスト。
    let plain = classification_requests(&tree, &ClassifyOptions::default()).unwrap();
    let default = classification_requests_guided(
        &tree,
        &ClassifyOptions::default(),
        &ClassifyGuidance::default(),
    )
    .unwrap();
    assert_eq!(plain[0].system, default[0].system);
    assert_eq!(plain[0].prompt, default[0].prompt);
    assert!(!plain[0].system.contains("user_instructions"));
}

/// ADR 0022: OpenAI 互換の API でも、注文と前の分類を同じように送る。
#[test]
fn an_openai_compatible_revision_sends_the_same_guidance() {
    let tree = bustup();
    let cassette = Cassette::load(repo_path(
        "tests/cassettes/openai_classify_layers_bustup.json",
    ))
    .unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = OpenAiProvider::with_transport(
        OpenAiConfig::new(OPENAI_BASE_URL, "gpt-6-sol"),
        Some(ApiKey::new("sk-test").unwrap()),
        transport.clone(),
    );
    let labels = classify_layers_guided(
        &provider,
        &tree,
        &ClassifyOptions::default(),
        &revision(&tree),
    )
    .unwrap();
    assert_eq!(labels[23].label, Headwear);
    let body = transport.received()[0].body.clone().unwrap();
    let content = body["messages"][1]["content"].as_array().unwrap();
    let prompt = content.iter().rev().find(|c| c["type"] == "text").unwrap();
    check_guided_request(
        body["messages"][0]["content"].as_str().unwrap(),
        prompt["text"].as_str().unwrap(),
        &body["response_format"]["json_schema"]["schema"],
    );
}

#[test]
fn a_previous_classification_must_cover_every_layer() {
    let fake = FakeProvider::new();
    let guidance = ClassifyGuidance {
        previous: Some(Vec::new()),
        ..ClassifyGuidance::default()
    };
    let err = classify_layers_guided(&fake, &bustup(), &ClassifyOptions::default(), &guidance)
        .unwrap_err();
    assert!(matches!(err, AiError::InvalidRequest(_)), "{err}");
    assert!(fake.calls().is_empty());
}
