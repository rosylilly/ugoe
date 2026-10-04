/// ビューアのコマンド。ここに書いたものごとに権限 `allow-<名前>` ができ、`capabilities/default.json` で
/// 許したものだけを webview から呼べる (T0727)。`lib.rs` の `generate_handler!` と揃える。
const COMMANDS: &[&str] = &[
    "initial_file",
    "open_file_dialog",
    "read_model_file",
    "save_file_dialog",
    "write_json_file",
    "write_png_file",
];

fn main() {
    // `tauri::generate_context!` はフロントエンドの出力先が無いとコンパイルできないので、
    // フロントエンドを未ビルドのままでも `cargo check --workspace` が通るよう、空のディレクトリを作る。
    let _ = std::fs::create_dir_all("../dist");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .unwrap_or_else(|e| panic!("failed to build the viewer: {e:#}"));
}
