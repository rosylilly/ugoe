//! 提案の前後の差分: 種類ごとに、加わった・消えた・変わった要素の ID。
//! エージェント (と承認する人) が、提案で何が変わるかを確かめるためのもの。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value, json};
use ugoe_editor_core::Document;

/// 要素の種類 → (ID → JSON)。
type Elements = BTreeMap<&'static str, BTreeMap<String, Value>>;

fn collect(doc: &Document) -> Elements {
    fn add<T: Serialize>(
        out: &mut Elements,
        kind: &'static str,
        items: &[T],
        key: impl Fn(&T) -> String,
    ) {
        let map = out.entry(kind).or_default();
        for item in items {
            map.insert(key(item), to_json(item));
        }
    }
    let model = doc.model();
    let mut out = Elements::new();
    add(&mut out, "parameters", &model.parameters, |p| {
        p.id.to_string()
    });
    add(&mut out, "parameter_groups", &model.parameter_groups, |g| {
        g.id.clone()
    });
    add(&mut out, "parts", &model.parts, |p| p.id.to_string());
    add(&mut out, "deformers", &model.deformers, |d| {
        d.id.to_string()
    });
    add(&mut out, "meshes", &model.meshes, |m| m.id.to_string());
    add(&mut out, "glues", &model.glues, |g| g.id.clone());
    add(&mut out, "textures", &model.textures, |t| t.name.clone());
    add(&mut out, "hit_areas", &model.hit_areas, |h| h.id.clone());
    add(&mut out, "slots", &model.slots, |s| s.id.to_string());
    add(&mut out, "motions", doc.motions(), |m| m.name.clone());
    add(&mut out, "expressions", doc.expressions(), |e| {
        e.name.clone()
    });
    let rigs = doc.physics().map(|p| p.rigs.as_slice()).unwrap_or_default();
    add(&mut out, "physics_rigs", rigs, |r| r.id.clone());
    // 集まりでないもの (キャンバス・ポーズ・エフェクト・物理の全体の設定など) は 1 つの要素として比べる。
    let whole = out.entry("model").or_default();
    whole.insert("meta".to_owned(), to_json(&model.meta));
    whole.insert("pose".to_owned(), to_json(&model.pose));
    whole.insert("effects".to_owned(), to_json(&model.effects));
    whole.insert("avatar".to_owned(), to_json(&model.avatar));
    whole.insert("physics".to_owned(), to_json(&doc.physics()));
    out
}

/// `before` から `after` への差分。変わった種類だけを載せる。
pub fn diff(before: &Document, after: &Document) -> Value {
    let before = collect(before);
    let after = collect(after);
    let mut out = Map::new();
    for (kind, new) in &after {
        let old = before.get(kind).cloned().unwrap_or_default();
        let added: Vec<&String> = new.keys().filter(|k| !old.contains_key(*k)).collect();
        let removed: Vec<&String> = old.keys().filter(|k| !new.contains_key(*k)).collect();
        let changed: Vec<&String> = new
            .iter()
            .filter(|(k, v)| old.get(*k).is_some_and(|o| o != *v))
            .map(|(k, _)| k)
            .collect();
        if added.is_empty() && removed.is_empty() && changed.is_empty() {
            continue;
        }
        let mut entry = Map::new();
        for (name, ids) in [("added", added), ("removed", removed), ("changed", changed)] {
            if !ids.is_empty() {
                entry.insert(name.to_owned(), json!(ids));
            }
        }
        out.insert((*kind).to_owned(), Value::Object(entry));
    }
    Value::Object(out)
}

/// モデルの型を JSON にする。モデルの型は常に JSON にできる (保存と同じ形)。
fn to_json<T: Serialize + ?Sized>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}
