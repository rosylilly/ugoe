//! 公開仕様 (`spec/`) の生成物: JSON Schema (`spec/schema/`) と例のモデル (`spec/examples/*/model.json`)。
//! `just gen` で書き出し、テストで最新かどうかを確かめる。
//!
//! - 仕様の文書は `spec/schema/README.md` と `spec/examples/README.md`。
//! - 例の期待値 (`expected.json`) は生成しない。仕様の式から手で導いた値を置き、
//!   ランタイムがそれを満たすことを `ugoe-format` のテスト (`tests/spec_examples.rs`) で確かめる。
//! - スキーマは読み込み側の形 (省略できるフィールドは必須にしない) で、この版の書き手が書く値だけを受け付ける。
//!   未知の値 (`Unknown`) と未知のフィールドの扱いは仕様の文書が決める (スキーマは未知のフィールドを拒まない)。
//! - 生成物は公開仕様の一部なので CC BY 4.0。`$comment` にその旨を書く。

use schemars::generate::SchemaSettings;
use schemars::transform::RecursiveTransform;
use schemars::{JsonSchema, Schema};
use serde_json::{Value, json};

use crate::expression::Expression;
use crate::fixtures;
use crate::motion::Motion;
use crate::physics::PhysicsSettings;
use crate::{Model, Module, SCHEMA_VERSION};

/// 公開仕様の版 (`spec/README.md#versions`)。`SCHEMA_VERSION` と対で変える。
pub const SPEC_VERSION: &str = "0.1";

/// `$id` の前半。ファイル名を後ろに付ける。
pub const ID_BASE: &str = "https://raw.githubusercontent.com/rosylilly/ugoe/main/spec/schema/";

/// 例のモデル: (`spec/examples/` の下のディレクトリ名, 元のフィクスチャ)。
pub fn example_models() -> Vec<(&'static str, Model)> {
    vec![
        ("single-quad", fixtures::single_quad()),
        ("warp-grid", fixtures::warp_grid()),
        ("rotation-chain", fixtures::rotation_chain()),
        ("rotation-under-warp", fixtures::rotation_under_warp()),
        ("part-opacity", fixtures::part_opacity_hierarchy()),
        ("glued-quads", fixtures::glued_quads()),
        ("blend-shapes", fixtures::blend_shapes()),
    ]
}

/// 生成するファイルの一覧: (リポジトリのルートからのパス, 整形した JSON と末尾の改行)。
pub fn spec_files() -> Result<Vec<(String, String)>, serde_json::Error> {
    let mut files = spec_schemas()?
        .into_iter()
        .map(|(name, json)| (format!("spec/schema/{name}"), json))
        .collect::<Vec<_>>();
    for (dir, model) in example_models() {
        files.push((
            format!("spec/examples/{dir}/model.json"),
            serde_json::to_string_pretty(&model)? + "\n",
        ));
    }
    Ok(files)
}

/// 生成する JSON Schema の一覧: (ファイル名, 整形した JSON と末尾の改行)。
pub fn spec_schemas() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    [
        schema::<Model>("model.schema.json", "Model", true)?,
        schema::<Module>("module.schema.json", "Module", true)?,
        schema::<PhysicsSettings>("physics.schema.json", "PhysicsSettings", false)?,
        schema::<Motion>("motion.schema.json", "Motion", false)?,
        schema::<Expression>("expression.schema.json", "Expression", false)?,
    ]
    .into_iter()
    .map(|(name, schema)| Ok((name, serde_json::to_string_pretty(&schema)? + "\n")))
    .collect()
}

fn schema<T: JsonSchema>(
    file: &'static str,
    title: &str,
    versioned: bool,
) -> Result<(&'static str, Schema), serde_json::Error> {
    // doc コメント (日本語・実装向け) は公開仕様に載せない。フィールドの意味は spec/ の文書に書く
    let mut schema = SchemaSettings::draft2020_12()
        .with_transform(RecursiveTransform(|schema: &mut Schema| {
            schema.remove("description");
            if let Some(default) = schema.get_mut("default") {
                shorten_f32(default);
            }
        }))
        .into_generator()
        .into_root_schema_for::<T>();
    schema.insert("$id".into(), Value::String(format!("{ID_BASE}{file}")));
    schema.insert(
        "title".into(),
        Value::String(format!("ugoe {title} (Specification {SPEC_VERSION} Draft)")),
    );
    schema.insert(
        "$comment".into(),
        Value::String(format!(
            "Part of the ugoe Specification {SPEC_VERSION} (Draft), \
             Copyright (c) 2026 Sho KUSANO and the ugoe contributors, licensed under CC BY 4.0 \
             (https://creativecommons.org/licenses/by/4.0/). Generated from the reference \
             implementation; see spec/schema/README.md."
        )),
    );
    if versioned {
        // この版のスキーマは、この版の schema_version のデータだけを受け付ける
        if let Some(Value::Object(version)) = schema
            .get_mut("properties")
            .and_then(|p| p.get_mut("schema_version"))
        {
            version.insert("const".into(), json!(SCHEMA_VERSION));
        }
    }
    Ok((file, schema))
}

/// 既定値の `f32` を、`f64` に広げたときの長い 10 進 (`0.05000000074505806`) ではなく、
/// `f32` として往復する最短の 10 進 (`0.05`) で書く。
fn shorten_f32(value: &mut Value) {
    match value {
        Value::Number(n) if n.is_f64() => {
            if let Some(x) = n.as_f64()
                && f64::from(x as f32) == x
                && let Ok(short) = (x as f32).to_string().parse::<f64>()
                && let Some(short) = serde_json::Number::from_f64(short)
            {
                *n = short;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(shorten_f32),
        Value::Object(map) => map.values_mut().for_each(shorten_f32),
        _ => {}
    }
}
