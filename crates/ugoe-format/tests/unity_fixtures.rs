//! Unity のバインディングの検査が使うファイル (`bindings/unity/tests/Fixtures/`) が、そこに置いた JSON と
//! 食い違わないことを確かめる。ファイルは `ugoe build` で作る (作り方は `bindings/unity/tests/Fixtures/README.md`)。

use std::path::{Path, PathBuf};

use ugoe_format::{ModuleFile, model_from_json, module_from_json, read_module};
use ugoe_runtime::{ComposeError, compose};

/// 食い違ったときに示す、作り直しの案内。
const REBUILD: &str = "作り直し方は bindings/unity/tests/Fixtures/README.md";

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bindings/unity/tests/Fixtures")
}

fn read_text(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// 置いた `<name>.ugoemod` を読み、モジュールが `<name>.module.json` と同じことを確かめる。
fn module_fixture(name: &str) -> ModuleFile {
    let module = module_from_json(&read_text(&format!("{name}.module.json")))
        .unwrap_or_else(|e| panic!("{name}.module.json: {e}"));
    let path = fixtures_dir().join(format!("{name}.ugoemod"));
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e} ({REBUILD})", path.display()));
    let file = read_module(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        serde_json::to_value(&file.module).expect("module to JSON"),
        serde_json::to_value(&module).expect("module to JSON"),
        "{} differs from {name}.module.json ({REBUILD})",
        path.display()
    );
    assert_eq!(
        file.textures.len(),
        module.textures.len(),
        "{}: one image per texture",
        path.display()
    );
    file
}

/// `unknown-mask.ugoemod` を `mask-base.model.json` に付けると `unknown_mask` になる (`ResultTests.cs`)。
#[test]
fn unknown_mask_module_fails_on_the_base() {
    let base = model_from_json(&read_text("mask-base.model.json")).expect("mask-base.model.json");
    let file = module_fixture("unknown-mask");
    let error = compose(&base, &[&file.module]).expect_err("the slot does not lend Skin");
    let ComposeError::UnknownMask { module, mesh, mask } = error else {
        panic!("expected unknown_mask, got {error:?}");
    };
    assert_eq!(
        (module.as_str(), mesh.as_str(), mask.as_str()),
        ("unknown_mask", "Back", "Skin")
    );
}
