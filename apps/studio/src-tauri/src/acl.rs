// アプリのコマンドの許可 (T0820。ADR 0028 の 6.)。
//
// `build.rs` がこのファイルを `include!` し、`COMMANDS` を `tauri_build::AppManifest::commands` に渡す。
// 載せたコマンドごとに `allow-<名前>` (`_` は `-`) の許可ができ、どのウィンドウが呼べるかは `capabilities/` で決める
// (アプリの許可の表を持つと、どの capability にも載っていないコマンドは呼べない)。
// `build.rs` からも読むので、ここには `//!` の文書コメントや `use` を書かない。

/// アプリのコマンドの名前 (`lib.rs` の `generate_handler!` と同じものを同じ順に)。
pub const COMMANDS: &[&str] = &[
    "restore_profile",
    "new_profile",
    "open_profile",
    "save_profile",
    "forget_recent_profile",
    "add_scene",
    "duplicate_scene",
    "move_scene",
    "remove_scene",
    "update_scene",
    "activate_scene",
    "set_output_preset",
    "scene_module",
    "scene_background",
    "subscribe_frames",
    "scene_model",
    "subscribe_status",
    "set_parameter",
    "reset_parameters",
    "output_state",
    "set_output_visible",
    "set_output_on_top",
    "set_output_size",
    "tick_rate",
    "set_tick_rate",
    "report_render_stats",
    "report_load_failure",
    "subscribe_pulse",
    "set_tracking",
    "set_tracking_paused",
    "set_auto_eye_blink",
    "set_lip_sync_level",
    "set_lip_sync_paused",
    "play_motion",
    "stop_motions",
    "set_expression",
    "add_expression",
    "remove_expression",
    "clear_expressions",
    "set_motion_settings",
    "set_input_settings",
    "set_hotkeys",
    "suspend_hotkeys",
    "module_slot",
    "virtual_camera_state",
    "start_virtual_camera",
    "stop_virtual_camera",
    "set_virtual_camera_settings",
    "api_state",
    "set_api_enabled",
    "set_api_port",
    "api_token",
    "regenerate_api_token",
    "approve_api_client",
    "deny_api_client",
    "revoke_api_client",
    "disconnect_api_client",
    "set_source_enabled",
    "source_url",
    "regenerate_source_token",
];

/// 出力ウィンドウ (受け手) が呼べるコマンド (`capabilities/output.json`)。フロントエンドの `OutputFeed` (`src/lib/feed.ts`) の口。
/// 操作ウィンドウ (`capabilities/control.json`) は、これ以外のコマンドをすべて呼べる。
#[allow(dead_code)] // build.rs では使わない
pub const OUTPUT_COMMANDS: &[&str] = &[
    "subscribe_frames",
    "scene_model",
    "scene_module",
    "scene_background",
    "report_render_stats",
    "report_load_failure",
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

    #[test]
    fn commands_match_the_invoke_handler() {
        // 載せ漏らしたコマンドはどのウィンドウからも呼べなくなる
        assert_eq!(handler_commands(), COMMANDS);
    }

    #[test]
    fn output_window_is_allowed_only_the_receiver_commands() {
        let output = allowed(include_str!("../capabilities/output.json"));
        let expected: BTreeSet<String> = OUTPUT_COMMANDS.iter().map(|c| c.to_string()).collect();
        assert_eq!(output, expected);
    }

    #[test]
    fn control_window_is_allowed_every_other_command() {
        let control = allowed(include_str!("../capabilities/control.json"));
        let expected: BTreeSet<String> = COMMANDS
            .iter()
            .filter(|c| !OUTPUT_COMMANDS.contains(c))
            .map(|c| c.to_string())
            .collect();
        assert_eq!(control, expected);
    }

    #[test]
    fn content_security_policy_allows_wasm_and_ipc_only_from_the_app() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = &config["app"]["security"]["csp"];
        let directive = |name: &str| csp[name].as_str().unwrap_or_default().to_owned();
        assert_eq!(directive("default-src"), "'self'");
        assert!(directive("script-src").contains("'wasm-unsafe-eval'"));
        assert!(!directive("script-src").contains("'unsafe-eval'"));
        assert!(!directive("script-src").contains("'unsafe-inline'"));
        assert!(directive("connect-src").contains("ipc:"));
        assert!(directive("connect-src").contains("http://ipc.localhost"));
        assert!(directive("img-src").contains("blob:"));
        assert_eq!(directive("object-src"), "'none'");
    }
}
