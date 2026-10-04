// アプリのコマンドの名前 (`COMMANDS`)。ウィンドウごとの許可は `capabilities/` (T0686)。
include!("src/acl.rs");

fn main() {
    // `tauri::generate_context!` はフロントエンドの出力先が存在しないとコンパイルできない。
    // `cargo check --workspace` をフロントエンド未ビルドでも通すため、空ディレクトリを用意する。
    let _ = std::fs::create_dir_all("../dist");
    println!("cargo:rerun-if-changed=src/acl.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run the tauri build script");
}
