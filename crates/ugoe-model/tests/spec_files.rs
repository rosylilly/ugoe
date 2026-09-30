//! 公開仕様の生成物 (`spec/schema/` と `spec/examples/*/model.json`) がコミット済みで最新か。
//! 作り直しは `just gen`。

use std::path::Path;

use ugoe_model::spec_files::spec_files;

#[test]
fn checked_in_spec_files_are_up_to_date() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, content) in spec_files().unwrap() {
        let on_disk = std::fs::read_to_string(root.join(&name)).unwrap_or_default();
        assert!(on_disk == content, "{name} is stale; run `just gen`");
    }
}
