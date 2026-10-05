// アプリのコマンドの許可 (T0686)。
//
// `build.rs` がこのファイルを `include!` し、`COMMANDS` を `tauri_build::AppManifest::commands` に渡す。
// 載せたコマンドごとに `allow-<名前>` (`_` は `-`) の許可ができ、どのウィンドウが呼べるかは `capabilities/` で決める
// (アプリの許可の表を持つと、どの capability にも載っていないコマンドは呼べない。ウィンドウを足しても既定ではすべてを許さない)。
// `build.rs` からも読むので、ここには `//!` の文書コメントや `use` を書かない。

/// アプリのコマンドの名前 (`lib.rs` の `generate_handler!` と同じものを同じ順に)。
pub const COMMANDS: &[&str] = &[
    "app_info",
    "apply_command",
    "get_model_json",
    "get_animations",
    "get_physics",
    "undo",
    "redo",
    "history_status",
    "set_undo_limit",
    "project_status",
    "new_project",
    "new_project_from_psd",
    "open_project",
    "open_sample_project",
    "reload_project",
    "check_external_change",
    "save_project",
    "save_project_as",
    "get_export_settings",
    "set_export_settings",
    "get_preview_lighting",
    "set_preview_lighting",
    "export_bundle",
    "get_model_bundle",
    "module_status",
    "read_module_base",
    "new_module_project",
    "open_module_with_base",
    "export_module",
    "module_open_problem",
    "check_base_change",
    "reload_base",
    "get_preview_base_bundle",
    "get_preview_module",
    "compare_preview_module",
    "import_psd",
    "local_to_canvas",
    "canvas_to_local",
    "find_overflows",
    "verify_overflows",
    "check_physics",
    "check_avatar_profile",
    "ai_settings",
    "set_ai_provider",
    "store_api_key",
    "delete_api_key",
    "set_ai_send_externally",
    "forget_ai_consent",
    "test_ai_connection",
    "cancel_ai_requests",
    "get_proposals",
    "classify_psd_layers",
    "propose_rig_from_psd",
    "regenerate_rig_proposal",
    "propose_normal_maps",
    "preview_proposal",
    "get_proposal_preview_bundle",
    "clear_proposal_preview",
    "adopt_proposal",
    "reject_proposal",
    "dismiss_stale_rigs",
    "mcp_bridge_status",
    "set_mcp_bridge_enabled",
    "set_mcp_bridge_port",
    "regenerate_mcp_bridge_token",
    "get_mcp_bridge_token",
    "answer_mcp_approval",
    "open_claude_desktop_bundle",
    "save_claude_desktop_bundle",
    "write_json_file",
    "write_png_file",
    "get_preview_modules",
    "set_preview_modules",
    "read_module_file",
    "get_normal_map_group",
    "get_normal_map_groups",
    "generate_texture_normal_map",
    "assign_texture_normal_map",
    "get_texture_png",
    "get_texture_versions",
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
            security(include_str!("../../../viewer/src-tauri/tauri.conf.json"))
        );
        assert_eq!(
            own,
            security(include_str!("../../../studio/src-tauri/tauri.conf.json"))
        );
    }
}
