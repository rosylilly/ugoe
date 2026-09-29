//! ツールの入力の JSON Schema。`Command` の部分は `ugoe_editor_core::codegen::command_schema` から作る
//! (手で写さない)。
//!
//! MCP のツールの `inputSchema` は最上位が `type: object` でなければならないが、`Command` の
//! スキーマは最上位が `oneOf` で、`Batch` の中身が `{"$ref": "#"}` (自分自身) を指す。そこで
//! `Command` のスキーマを `$defs.Command` に移し、`#` への参照を `#/$defs/Command` に書き換えて、
//! ツールの入力のスキーマの `$defs` に置く。

use serde_json::{Map, Value, json};

/// `$defs` の中の `Command` の名前。
const COMMAND_DEF: &str = "Command";

/// `Command` のスキーマを組み替えたもの。
#[derive(Debug, Clone)]
pub struct CommandSchema {
    /// `$defs` (`Command` 自身を含む)。ツールの入力のスキーマの `$defs` にそのまま置く。
    pub defs: Map<String, Value>,
}

impl CommandSchema {
    /// `ugoe_editor_core` の `Command` から作る。
    pub fn generate() -> Self {
        let schema = ugoe_editor_core::codegen::command_schema();
        Self::from_root(schema.to_value())
    }

    /// 最上位が `Command` のスキーマ (draft 2020-12) から作る。
    fn from_root(root: Value) -> Self {
        let mut root = match root {
            Value::Object(map) => map,
            // schemars が作るスキーマは常にオブジェクト。`true` / `false` のスキーマなら何でも受け付ける形にする。
            _ => Map::new(),
        };
        root.remove("$schema");
        let mut defs = match root.remove("$defs") {
            Some(Value::Object(defs)) => defs,
            _ => Map::new(),
        };
        defs.insert(COMMAND_DEF.to_owned(), Value::Object(root));
        defs.values_mut().for_each(rewrite_root_refs);
        Self { defs }
    }

    /// `Command` 1 つを指すスキーマ。
    pub fn command_ref() -> Value {
        json!({ "$ref": format!("#/$defs/{COMMAND_DEF}") })
    }
}

/// `{"$ref": "#"}` を `{"$ref": "#/$defs/Command"}` に書き換える (`#/$defs/...` はそのまま使える)。
fn rewrite_root_refs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get_mut("$ref")
                && r == "#"
            {
                *r = format!("#/$defs/{COMMAND_DEF}");
            }
            map.values_mut().for_each(rewrite_root_refs);
        }
        Value::Array(items) => items.iter_mut().for_each(rewrite_root_refs),
        _ => {}
    }
}

/// ツールの入力のスキーマ (`type: object`)。`command_defs` を渡すと `$defs` に `Command` などを置く。
pub fn object_schema(
    properties: Value,
    required: &[&str],
    command_defs: Option<&CommandSchema>,
) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    });
    if let (Some(defs), Value::Object(map)) = (command_defs, &mut schema) {
        map.insert("$defs".to_owned(), Value::Object(defs.defs.clone()));
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    /// スキーマの中の `$ref` をすべて集める。
    fn refs(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(r)) = map.get("$ref") {
                    out.push(r.clone());
                }
                map.values().for_each(|v| refs(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| refs(v, out)),
            _ => {}
        }
    }

    #[test]
    fn every_ref_points_into_defs() {
        let schema = CommandSchema::generate();
        let tool = object_schema(
            json!({ "command": CommandSchema::command_ref() }),
            &["command"],
            Some(&schema),
        );
        let mut found = Vec::new();
        refs(&tool, &mut found);
        assert!(found.iter().any(|r| r == "#/$defs/Command"));
        for r in &found {
            let name = r
                .strip_prefix("#/$defs/")
                .unwrap_or_else(|| panic!("unexpected $ref: {r}"));
            assert!(schema.defs.contains_key(name), "missing def: {name}");
        }
    }

    #[test]
    fn command_def_is_the_generated_schema() {
        let schema = CommandSchema::generate();
        let command = &schema.defs[COMMAND_DEF];
        assert!(command.get("$schema").is_none());
        assert!(command.get("$defs").is_none());
        let ops = command["oneOf"].as_array().expect("oneOf");
        assert!(
            ops.iter()
                .any(|op| op["properties"]["op"]["const"] == "add_parameter")
        );
        assert!(
            ops.iter()
                .any(|op| op["properties"]["op"]["const"] == "add_keyform_key")
        );
    }
}
