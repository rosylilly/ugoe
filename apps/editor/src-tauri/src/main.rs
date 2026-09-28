// リリースビルドの Windows で余計なコンソールを出さない。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ugoe_editor_lib::run()
}
