// リリースビルドの Windows でコンソールのウィンドウを開かない。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ugoe_studio_lib::run()
}
