//! 公開仕様の生成物 (`spec/schema/` と `spec/examples/*/model.json`) を書き出す。`just gen` から呼ぶ。

use std::fs;
use std::path::Path;

use ugoe_model::spec_files::spec_files;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, content) in spec_files()? {
        let path = root.join(&name);
        if fs::read_to_string(&path).is_ok_and(|old| old == content) {
            continue;
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, content)?;
        println!("wrote {name}");
    }
    Ok(())
}
