fn main() {
    // `tauri::generate_context!` はフロントエンドの出力先が存在しないとコンパイルできない。
    // `cargo check --workspace` をフロントエンド未ビルドでも通すため、空ディレクトリを用意する。
    let _ = std::fs::create_dir_all("../dist");
    tauri_build::build()
}
