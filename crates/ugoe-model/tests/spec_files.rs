//! 公開仕様の生成物 (`spec/schema/` と `spec/examples/*/model.json`) がコミット済みで最新か。
//! 作り直しは `just gen`。

use std::path::Path;

use serde_json::Value;
use ugoe_model::spec_files::{spec_files, spec_schemas};

#[test]
fn checked_in_spec_files_are_up_to_date() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, content) in spec_files().unwrap() {
        let on_disk = std::fs::read_to_string(root.join(&name)).unwrap_or_default();
        assert!(on_disk == content, "{name} is stale; run `just gen`");
    }
}

/// 文字列の enum は、すべて `oneOf` の `const` の並びの形で書かれている (`enum` のキーワードを使わない)。
#[test]
fn string_enums_are_written_as_one_of_consts() {
    fn visit(file: &str, path: &str, value: &Value) {
        match value {
            Value::Object(map) => {
                assert!(
                    !map.contains_key("enum"),
                    "{file}: {path} uses `enum`; string enums are `oneOf` of `const`"
                );
                for (key, child) in map {
                    visit(file, &format!("{path}/{key}"), child);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    visit(file, &format!("{path}/{index}"), child);
                }
            }
            _ => {}
        }
    }
    for (file, json) in spec_schemas().unwrap() {
        let schema: Value = serde_json::from_str(&json).unwrap();
        visit(file, "", &schema);
    }
}
