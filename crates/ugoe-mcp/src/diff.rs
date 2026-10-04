//! 提案の前後の差分: 種類ごとに、加わった・消えた・変わった要素の ID と、並びが変わったか。
//! エージェント (と承認する人) が、提案で何が変わるかを確かめるためのもの。

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::Serialize;
use serde_json::{Map, Value, json};
use ugoe_editor_core::Document;

/// 1 つの種類の要素: ID → JSON と、モデルの中の並び (並べ替えは描画の前後などを変える)。
#[derive(Default)]
struct Collection {
    items: BTreeMap<String, Value>,
    order: Vec<String>,
}

impl Collection {
    fn insert(&mut self, id: String, value: Value) {
        self.order.push(id.clone());
        self.items.insert(id, value);
    }
}

/// 要素の種類 → 要素。
type Elements = BTreeMap<&'static str, Collection>;

fn collect(doc: &Document) -> Elements {
    fn add<T: Serialize>(
        out: &mut Elements,
        kind: &'static str,
        items: &[T],
        key: impl Fn(&T) -> String,
    ) {
        let collection = out.entry(kind).or_default();
        for item in items {
            collection.insert(key(item), to_json(item));
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
    // テクスチャは寸法などの `TextureRef` に画像の指紋を添える (同じ大きさの画像への差し替えも変化にする)。
    let textures = out.entry("textures").or_default();
    for texture in &model.textures {
        let png = doc.texture(&texture.name).map(fingerprint);
        textures.insert(
            texture.name.clone(),
            json!({ "ref": to_json(texture), "png": png }),
        );
    }
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

/// 画像のバイト列の指紋 (前後で同じプロセスの中で比べるだけなので、`DefaultHasher` でよい)。
fn fingerprint(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// `before` から `after` への差分。変わった種類だけを載せる。`reordered` は、前後の両方にある要素の
/// 並びが変わったこと。
pub fn diff(before: &Document, after: &Document) -> Value {
    let before = collect(before);
    let after = collect(after);
    let empty = Collection::default();
    let mut out = Map::new();
    for (kind, new) in &after {
        let old = before.get(kind).unwrap_or(&empty);
        let (old_items, new_items) = (&old.items, &new.items);
        let added: Vec<&String> = new_items
            .keys()
            .filter(|k| !old_items.contains_key(*k))
            .collect();
        let removed: Vec<&String> = old_items
            .keys()
            .filter(|k| !new_items.contains_key(*k))
            .collect();
        let changed: Vec<&String> = new_items
            .iter()
            .filter(|(k, v)| old_items.get(*k).is_some_and(|o| o != *v))
            .map(|(k, _)| k)
            .collect();
        let kept = |order: &[String], other: &BTreeMap<String, Value>| -> Vec<String> {
            order
                .iter()
                .filter(|k| other.contains_key(*k))
                .cloned()
                .collect()
        };
        let reordered = kept(&old.order, new_items) != kept(&new.order, old_items);
        if added.is_empty() && removed.is_empty() && changed.is_empty() && !reordered {
            continue;
        }
        let mut entry = Map::new();
        for (name, ids) in [("added", added), ("removed", removed), ("changed", changed)] {
            if !ids.is_empty() {
                entry.insert(name.to_owned(), json!(ids));
            }
        }
        if reordered {
            entry.insert("reordered".to_owned(), json!(true));
        }
        out.insert((*kind).to_owned(), Value::Object(entry));
    }
    Value::Object(out)
}

/// モデルの型を JSON にする。モデルの型は常に JSON にできる (保存と同じ形)。
fn to_json<T: Serialize + ?Sized>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use ugoe_editor_core::texture::solid_png;
    use ugoe_editor_core::{Command, Textures};
    use ugoe_model::fixtures;

    use super::*;

    fn with_white(model: ugoe_model::Model) -> Document {
        let mut textures = Textures::new();
        textures.insert("white".to_owned(), solid_png(1, 1, [255; 4]).into());
        Document::with_textures(model, textures)
    }

    fn edited(before: &Document, command: Command) -> Document {
        let mut after = before.detached();
        after.apply(command).unwrap();
        after
    }

    #[test]
    fn reordering_shows_in_the_diff() {
        // 並べ替えは描画の前後を変えるが、要素の中身は変わらない。
        let before = with_white(fixtures::glued_quads());
        let after = edited(
            &before,
            Command::ReorderMesh {
                id: "Left".into(),
                before: None,
            },
        );
        assert_eq!(
            diff(&before, &after),
            json!({ "meshes": { "reordered": true } })
        );
        assert_eq!(diff(&before, &before), json!({}));
    }

    #[test]
    fn replacing_a_texture_image_shows_in_the_diff() {
        // 同じ大きさの画像に差し替えると `TextureRef` は変わらない。
        let before = with_white(fixtures::single_quad());
        let after = edited(
            &before,
            Command::SetTextureImage {
                name: "white".to_owned(),
                png: solid_png(1, 1, [255, 0, 0, 255]),
            },
        );
        assert_eq!(
            diff(&before, &after),
            json!({ "textures": { "changed": ["white"] } })
        );
    }
}
