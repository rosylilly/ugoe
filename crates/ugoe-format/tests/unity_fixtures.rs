//! Unity のバインディングの検査が使うファイル (`bindings/unity/tests/Fixtures/`) を、そこに置いた JSON から作り、
//! 置いたファイルが JSON と食い違わないことを確かめる。作り方の説明は `bindings/unity/tests/Fixtures/README.md`。
//!
//! ファイルを作り直すには `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test unity_fixtures` を実行する。

use std::path::{Path, PathBuf};

use ugoe_format::{
    ModuleFile, TextureData, TextureFormat, model_from_json, module_from_json, read_module,
    write_module,
};
use ugoe_runtime::{ComposeError, compose};

/// ファイルを作り直すときに立てる環境変数。
const REGENERATE: &str = "UGOE_UPDATE_FIXTURES";

/// 1×1 の白い PNG。
const WHITE_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xf8, 0x0f, 0x04, 0x00,
    0x09, 0xfb, 0x03, 0xfd, 0x68, 0xfa, 0x1c, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bindings/unity/tests/Fixtures")
}

fn read_text(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `<name>.module.json` から `<name>.ugoemod` を作る (テクスチャはすべて 1×1 の白)。作り直すときは書き出し、
/// そうでなければ置いたファイルのモジュールが JSON と同じことを確かめる。
fn module_fixture(name: &str) -> ModuleFile {
    let module = module_from_json(&read_text(&format!("{name}.module.json")))
        .unwrap_or_else(|e| panic!("{name}.module.json: {e}"));
    let path = fixtures_dir().join(format!("{name}.ugoemod"));
    if std::env::var_os(REGENERATE).is_some() {
        let mut file = ModuleFile::new(module);
        file.textures = file
            .module
            .textures
            .iter()
            .map(|_| TextureData {
                format: TextureFormat::Png,
                bytes: WHITE_PNG.to_vec(),
            })
            .collect();
        let bytes = write_module(&file).expect("write module");
        std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        return file;
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (作り直すには {REGENERATE}=1 cargo test -p ugoe-format --test unity_fixtures)",
            path.display()
        )
    });
    let file = read_module(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        serde_json::to_value(&file.module).expect("module to JSON"),
        serde_json::to_value(&module).expect("module to JSON"),
        "{} differs from {name}.module.json (作り直すには {REGENERATE}=1 cargo test -p ugoe-format --test unity_fixtures)",
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
