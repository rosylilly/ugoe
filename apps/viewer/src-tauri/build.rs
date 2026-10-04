// アプリのコマンドの名前 (`COMMANDS`)。ウィンドウの許可は `capabilities/` (T0727・T0024)。
include!("src/acl.rs");

fn main() {
    // `tauri::generate_context!` はフロントエンドの出力先が無いとコンパイルできないので、
    // フロントエンドを未ビルドのままでも `cargo check --workspace` が通るよう、空のディレクトリを作る。
    let _ = std::fs::create_dir_all("../dist");
    println!("cargo:rerun-if-changed=src/acl.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .unwrap_or_else(|e| panic!("failed to build the viewer: {e:#}"));
}
