//! 生成物 (フロントエンドの型と `Command` の JSON Schema) を書き出す。`just gen` から呼ぶ。
//! 手順は `docs/editor.md#型の生成`。

use std::fs;

use ugoe_editor_lib::bindings::{generated_files, repo_root};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root();
    for (name, content) in generated_files()? {
        let path = root.join(name);
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
