fn main() {
    // `tauri::generate_context!` はフロントエンドの出力先が無いとコンパイルできないので、
    // フロントエンドを未ビルドのままでも `cargo check --workspace` が通るよう、空のディレクトリを作る。
    let _ = std::fs::create_dir_all("../dist");
    tauri_build::build()
}
