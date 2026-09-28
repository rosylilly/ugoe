//! 未知の値のフォールバック (`docs/spec/file-formats.md#未知の値の読み込み`)。
//! 入力は `ugoe_model::fixtures` のフィクスチャに、新しい版で増えた値を足したもの。

use serde_json::json;
use ugoe_model::{BlendMode, Deformer, DeformerKind, MaskMode, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance, ParamIndex};

fn instance(model: Model) -> ModelInstance {
    ModelInstance::new(CompiledModel::compile(model).expect("compile"))
}

/// `parent` の子として未知の種類のデフォーマ `id` を足し、`children` の親をそれに付け替える。
fn insert_unknown(model: &mut Model, id: &str, parent: Option<&str>, children: &[&str]) {
    model.deformers.push(Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Unknown,
        mirror: None,
        mirror_axis: None,
    });
    for deformer in &mut model.deformers {
        if children.contains(&deformer.id.as_str()) {
            deformer.parent = Some(id.into());
        }
    }
    for mesh in &mut model.meshes {
        if children.contains(&mesh.id.as_str()) {
            mesh.parent = Some(id.into());
        }
    }
}

/// すべてのパラメータの組 (最小・既定・最大) で、2 つのモデルの出力が一致することを確かめる。
fn assert_same_output(expected: Model, actual: Model) {
    let mut expected = instance(expected);
    let mut actual = instance(actual);
    let ranges: Vec<[f32; 3]> = expected
        .compiled()
        .model()
        .parameters
        .iter()
        .map(|p| [p.min, p.default, p.max])
        .collect();
    let combos = 3usize.pow(ranges.len() as u32);
    for combo in 0..combos {
        let mut rest = combo;
        for (index, range) in ranges.iter().enumerate() {
            let value = range[rest % 3];
            rest /= 3;
            let index = ParamIndex(index as u32);
            expected.parameters_mut().set(index, value);
            actual.parameters_mut().set(index, value);
        }
        expected.update();
        actual.update();
        assert_eq!(actual.drawables(), expected.drawables(), "combo {combo}");
    }
}

#[test]
fn unknown_deformer_is_identity_under_rotation_and_warp() {
    let baseline = fixtures::rotation_chain();
    let mut model = baseline.clone();
    // 回転の子 (回転 `Hand`) とワープの子 (メッシュ `Sleeve`) の間に、未知のデフォーマを挟む。
    insert_unknown(&mut model, "FutureA", Some("Arm"), &["Hand"]);
    insert_unknown(&mut model, "FutureB", Some("Cloth"), &["Sleeve"]);
    assert_same_output(baseline, model);

    let baseline = fixtures::rotation_under_warp();
    let mut model = baseline.clone();
    // 未知のデフォーマが 2 段続いても、親の変換がそのまま伝わる。
    insert_unknown(&mut model, "Outer", Some("Body"), &["Head", "Cheek"]);
    insert_unknown(&mut model, "Inner", Some("Outer"), &["Head"]);
    assert_same_output(baseline, model);
}

#[test]
fn unknown_deformer_at_root_keeps_canvas_coordinates() {
    let baseline = fixtures::single_quad();
    let mut model = baseline.clone();
    insert_unknown(&mut model, "Future", None, &["Quad"]);
    assert_same_output(baseline, model);
}

#[test]
fn unknown_deformer_from_json_evaluates_as_identity() {
    let baseline = fixtures::rotation_chain();
    let mut value = serde_json::to_value(&baseline).unwrap();
    value["deformers"].as_array_mut().unwrap().push(json!({
        "id": "Future",
        "name": "Future",
        "part": "Root",
        "parent": "Arm",
        "kind": { "type": "bezier", "rows": 3, "handles": [[0.0, 1.0], [2.0, 3.0]] },
    }));
    value["deformers"][1]["parent"] = json!("Future");
    let model: Model = serde_json::from_value(value).unwrap();
    assert_eq!(model.deformers[3].kind, DeformerKind::Unknown);
    assert_same_output(baseline, model);
}

#[test]
fn unknown_blend_and_mask_modes_fall_back() {
    let mut model = fixtures::intersection_masks();
    let tongue = model
        .meshes
        .iter()
        .position(|m| m.id.as_str() == "Tongue")
        .unwrap();
    model.meshes[tongue].mask_mode = MaskMode::Unknown;
    model.meshes[tongue].blend = BlendMode::Unknown;
    let m = instance(model);
    assert_eq!(m.compiled().mask_mode(tongue), MaskMode::Union);
    assert_eq!(m.drawables()[tongue].blend, BlendMode::Normal);
}
