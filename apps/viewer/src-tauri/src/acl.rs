// アプリのコマンドの許可 (T0727。acl.rs の形は T0024 でエディタ・スタジオに揃えた)。
//
// `build.rs` がこのファイルを `include!` し、`COMMANDS` を `tauri_build::AppManifest::commands` に渡す。
// 載せたコマンドごとに `allow-<名前>` (`_` は `-`) の許可ができ、どのウィンドウが呼べるかは `capabilities/` で決める
// (アプリの許可の表を持つと、どの capability にも載っていないコマンドは呼べない)。
// `build.rs` からも読むので、ここには `//!` の文書コメントや `use` を書かない。

/// アプリのコマンドの名前 (`lib.rs` の `generate_handler!` と同じものを同じ順に)。
pub const COMMANDS: &[&str] = &[
    "initial_file",
    "open_file_dialog",
    "read_model_file",
    "save_file_dialog",
    "write_json_file",
    "write_png_file",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// `generate_handler!` に並べたコマンドの名前 (パスの最後)。
    fn handler_commands() -> Vec<String> {
        let source = include_str!("lib.rs");
        let start = source
            .find("generate_handler![")
            .expect("generate_handler! in lib.rs");
        let body = &source[start + "generate_handler![".len()..];
        let body = &body[..body.find(']').expect("end of generate_handler!")];
        body.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|path| path.rsplit("::").next().unwrap_or(path).to_owned())
            .collect()
    }

    /// capability のファイルにある、アプリのコマンドの許可 (`allow-…`) の名前 (`_` に戻す)。
    fn allowed(capability: &str) -> BTreeSet<String> {
        let json: serde_json::Value = serde_json::from_str(capability).unwrap();
        json["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.as_str())
            .filter(|p| !p.contains(':'))
            .map(|p| {
                p.strip_prefix("allow-")
                    .expect("app permission must be allow-*")
                    .replace('-', "_")
            })
            .collect()
    }

    /// `tauri.conf.json` の `app.security` (CSP と、Tauri に書き換えさせない指示)。
    fn security(config: &str) -> serde_json::Value {
        let config: serde_json::Value = serde_json::from_str(config).unwrap();
        config["app"]["security"].clone()
    }

    #[test]
    fn commands_match_the_invoke_handler() {
        // 載せ漏らしたコマンドはどのウィンドウからも呼べなくなる
        assert_eq!(handler_commands(), COMMANDS);
    }

    #[test]
    fn main_window_is_allowed_every_command() {
        let main = allowed(include_str!("../capabilities/default.json"));
        let expected: BTreeSet<String> = COMMANDS.iter().map(|c| c.to_string()).collect();
        assert_eq!(main, expected);
    }

    #[test]
    fn capability_is_only_for_the_main_window() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        assert_eq!(json["windows"], serde_json::json!(["main"]));
    }

    #[test]
    fn webview_cannot_open_file_dialogs_by_itself() {
        // ダイアログはバックエンドが開き、選ばれたパスだけを読み書きできるようにする (README の「CSP とコマンドの許可」)
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let permissions = json["permissions"].as_array().unwrap();
        assert!(
            permissions
                .iter()
                .filter_map(|p| p.as_str())
                .all(|p| !p.starts_with("dialog:"))
        );
    }

    #[test]
    fn content_security_policy_allows_wasm_and_ipc_only_from_the_app() {
        let security = security(include_str!("../tauri.conf.json"));
        let csp = &security["csp"];
        let directive = |name: &str| csp[name].as_str().unwrap_or_default().to_owned();
        assert_eq!(directive("default-src"), "'self'");
        assert!(directive("script-src").contains("'wasm-unsafe-eval'"));
        assert!(!directive("script-src").contains("'unsafe-eval'"));
        assert!(!directive("script-src").contains("'unsafe-inline'"));
        assert!(directive("connect-src").contains("ipc:"));
        assert!(directive("connect-src").contains("http://ipc.localhost"));
        assert!(directive("img-src").contains("blob:"));
        assert_eq!(directive("object-src"), "'none'");
        assert_eq!(directive("frame-ancestors"), "'none'");
    }

    #[test]
    fn content_security_policy_is_the_same_in_every_app() {
        // 3 つのアプリは同じ CSP を使う (docs/architecture.md#webview-の-csp-とコマンドの許可)
        let own = security(include_str!("../tauri.conf.json"));
        assert_eq!(
            own,
            security(include_str!("../../../editor/src-tauri/tauri.conf.json"))
        );
        assert_eq!(
            own,
            security(include_str!("../../../studio/src-tauri/tauri.conf.json"))
        );
    }
}
