//! コマンドのエラー。ADR 0029 の形 (`code`・`args`・`message`) で画面に渡し、画面はカタログの `errors.<code>` を
//! `args` で引いて訳す (`src/lib/errors.ts`。カタログに無いコードは `message` を出す)。コードの一覧は README の
//! 「コマンドのエラー」。コードは公開の約束として扱い、名前を変えない。
//!
//! 操作 API (`docs/studio-api.md`) の `-32010` の `data` は、これとは別の形 (`ProfileError`・`SceneError`・
//! `AnimationError` をそのまま書いた `{ code, ...フィールド }`。コードも別の名前) で、外のツールとの約束なので変えない。
//! そこへこの型の `code`・`args` を `messageCode`・`args` として足す (`api::rpc::RpcError::failed`。T0839)。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::animation::AnimationError;
use crate::profile::ProfileError;
use crate::stage::{Asset, SceneError};

/// コマンドのエラー (`code`・`args`・`message`)。`message` は英語の文 (ログと、訳の無い画面の向け)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct CommandError {
    pub(crate) code: &'static str,
    pub(crate) args: BTreeMap<&'static str, Value>,
    pub(crate) message: String,
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            args: BTreeMap::new(),
            message: message.into(),
        }
    }

    fn arg(mut self, name: &'static str, value: impl Into<Value>) -> Self {
        self.args.insert(name, value.into());
        self
    }

    fn path(self, path: &Path) -> Self {
        self.arg("path", path.display().to_string())
    }

    /// 別のスレッドの処理が終わらなかった (パニックした・取り消された)・OS の呼び出しが失敗した。ビューアと同じコード。
    pub(crate) fn internal(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new("internal", format!("the studio failed: {reason}")).arg("reason", reason)
    }

    /// シーン `revision` は今のシーンではない (受け手は次のシーンの通知を待つ)。
    pub(crate) fn scene_not_current(revision: u32) -> Self {
        Self::new(
            "scene_file.not_current",
            format!("scene {revision} is not the current scene"),
        )
        .arg("revision", revision)
    }

    /// 今のシーン `revision` に、求められたファイルが無い。
    pub(crate) fn scene_no_file(revision: u32) -> Self {
        Self::new(
            "scene_file.not_found",
            format!("scene {revision} has no such file"),
        )
        .arg("revision", revision)
    }

    /// 刻みの速さが 30 / 60 / 120 でない。
    pub(crate) fn unsupported_tick_rate(rate: u32) -> Self {
        Self::new(
            "tick_rate.unsupported",
            format!("unsupported tick rate: {rate}"),
        )
        .arg("rate", rate)
    }

    /// 出力が報告した描画の負荷が、数でない・負。
    pub(crate) fn invalid_render_stats() -> Self {
        Self::new("render_stats.invalid", "invalid render stats")
    }

    /// 使う GPU の設定を、機械ごとの設定ファイルに書けなかった (`reason` は英語の文。T0860)。
    pub(crate) fn gpu_settings_save_failed(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new(
            "gpu_settings.save_failed",
            format!("the GPU setting could not be saved: {reason}"),
        )
        .arg("reason", reason)
    }

    /// OS の「グラフィックの設定」を開けない OS (Windows の外。T0860)。
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn graphics_settings_unsupported() -> Self {
        Self::new(
            "graphics_settings.unsupported",
            "the graphics settings can be opened only on Windows",
        )
    }

    /// OS の「グラフィックの設定」を開けなかった (`reason` は英語の文。T0860)。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn graphics_settings_failed(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new(
            "graphics_settings.failed",
            format!("the graphics settings could not be opened: {reason}"),
        )
        .arg("reason", reason)
    }

    /// 出力ウィンドウが無い。
    pub(crate) fn output_window_missing() -> Self {
        Self::new("output_window.missing", "the output window is missing")
    }

    /// 出力ウィンドウの操作が失敗した (`reason` は英語の文)。
    pub(crate) fn output_window_failed(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new(
            "output_window.failed",
            format!("the output window failed: {reason}"),
        )
        .arg("reason", reason)
    }

    /// 出力ウィンドウの大きさが 16〜8192 の外。
    pub(crate) fn unsupported_output_size(width: u32, height: u32) -> Self {
        Self::new(
            "output_window.unsupported_size",
            format!("unsupported output size: {width}x{height}"),
        )
        .arg("width", width)
        .arg("height", height)
    }

    /// 仮想カメラの解像度・フレームレートを選べない (`ugoe_studio_vcam::Config::validate` の誤り)。
    pub(crate) fn invalid_camera_config(config: ugoe_studio_vcam::Config) -> Self {
        Self::new(
            "virtual_camera.invalid_config",
            format!(
                "unsupported virtual camera settings: {}x{} at {} fps",
                config.width, config.height, config.fps
            ),
        )
        .arg("width", config.width)
        .arg("height", config.height)
        .arg("fps", config.fps)
    }

    /// 読もうとしたパスを、利用者が選んでいない (ダイアログで選んだ・今のプロファイルにある・最近のプロファイルのどれでもない。
    /// T0838)。ビューアと同じコード。
    pub(crate) fn not_chosen(path: &Path) -> Self {
        Self::new(
            "file.not_chosen",
            format!("{}: the file was not chosen by the user", path.display()),
        )
        .path(path)
    }

    /// 書こうとしたパスを、保存のダイアログで選んでいない (選んだパスにも 1 回しか書けない。T0838)。ビューアと同じコード。
    pub(crate) fn not_chosen_for_save(path: &Path) -> Self {
        Self::new(
            "file.not_chosen_for_save",
            format!(
                "{}: the path was not chosen in the save dialog",
                path.display()
            ),
        )
        .path(path)
    }

    /// OS のファイルのダイアログが失敗した (`reason` は英語の文)。ビューアと同じコード。
    pub(crate) fn dialog_failed(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new("dialog.failed", format!("the dialog failed: {reason}")).arg("reason", reason)
    }

    /// 操作 API のポートが 1024 未満。
    pub(crate) fn unsupported_api_port(port: u16, min: u16) -> Self {
        Self::new("api.unsupported_port", format!("unsupported port: {port}"))
            .arg("port", port)
            .arg("min", min)
    }

    /// 承認を待つクライアントに `id` が無い (もう断った・切れた)。
    pub(crate) fn unknown_pending_client(id: &str) -> Self {
        Self::new(
            "api.unknown_pending_client",
            format!("no pending client `{id}`"),
        )
        .arg("id", id)
    }

    /// 承認したクライアントが多すぎる (`max` まで)。
    pub(crate) fn too_many_api_clients(max: usize) -> Self {
        Self::new(
            "api.too_many_clients",
            "too many approved clients; revoke one first",
        )
        .arg("max", max)
    }

    /// OS の乱数を取れず、トークン・鍵を作れなかった (`reason` は英語の文)。
    pub(crate) fn random_failed(reason: impl std::fmt::Display) -> Self {
        let reason = reason.to_string();
        Self::new(
            "api.random_failed",
            format!("cannot get random bytes from the OS: {reason}"),
        )
        .arg("reason", reason)
    }
}

impl From<ProfileError> for CommandError {
    fn from(e: ProfileError) -> Self {
        let message = e.to_string();
        match e {
            ProfileError::Io {
                path,
                message: reason,
            } => Self::new("profile.io", message)
                .path(&path)
                .arg("reason", reason),
            ProfileError::TooLarge { path, bytes } => Self::new("profile.too_large", message)
                .path(&path)
                .arg("size", bytes),
            ProfileError::NotJson {
                path,
                message: reason,
            } => Self::new("profile.not_json", message)
                .path(&path)
                .arg("reason", reason),
            ProfileError::NotProfile { path } => {
                Self::new("profile.not_profile", message).path(&path)
            }
            ProfileError::FutureVersion {
                path,
                version,
                supported,
            } => Self::new("profile.future_version", message)
                .path(&path)
                .arg("version", version)
                .arg("supported", supported),
            ProfileError::Invalid {
                path,
                message: reason,
            } => Self::new("profile.invalid", message)
                .path(&path)
                .arg("reason", reason),
        }
    }
}

/// シーンのファイルの種類の名前 (`args.asset`。stage.rs の `Asset` の JSON と同じ)。
fn asset_name(asset: Asset) -> &'static str {
    match asset {
        Asset::Model => "model",
        Asset::Module => "module",
        Asset::Background => "background",
    }
}

impl From<SceneError> for CommandError {
    fn from(e: SceneError) -> Self {
        let message = e.to_string();
        match e {
            SceneError::UnknownScene { id } => Self::new("scene.unknown", message).arg("id", id),
            // 文が種類で変わる (「モデルが見つからない」など) ので、種類ごとにコードを分ける。
            SceneError::Missing { asset, path } => {
                let code = match asset {
                    Asset::Model => "scene.model_missing",
                    Asset::Module => "scene.module_missing",
                    Asset::Background => "scene.background_missing",
                };
                Self::new(code, message)
                    .arg("asset", asset_name(asset))
                    .path(&path)
            }
            SceneError::Unreadable {
                asset,
                path,
                message: reason,
            } => Self::new("scene.unreadable", message)
                .arg("asset", asset_name(asset))
                .path(&path)
                .arg("reason", reason),
            SceneError::WrongType { asset, path } => Self::new("scene.wrong_type", message)
                .arg("asset", asset_name(asset))
                .path(&path),
            SceneError::TooLarge { asset, path, bytes } => Self::new("scene.too_large", message)
                .arg("asset", asset_name(asset))
                .path(&path)
                .arg("size", bytes),
            SceneError::DressUp { message: reason } => {
                Self::new("scene.dress_up", message).arg("reason", reason)
            }
            SceneError::UnknownPose { part } => {
                Self::new("scene.unknown_pose", message).arg("part", part)
            }
        }
    }
}

impl From<AnimationError> for CommandError {
    fn from(e: AnimationError) -> Self {
        let message = e.to_string();
        match e {
            AnimationError::NoModel => Self::new("animation.no_model", message),
            AnimationError::UnknownMotion { name } => {
                Self::new("animation.unknown_motion", message).arg("name", name)
            }
            AnimationError::UnknownExpression { name } => {
                Self::new("animation.unknown_expression", message).arg("name", name)
            }
        }
    }
}

impl CommandError {
    /// 再生中のモーションより優先度が低くて、モーションが始まらなかった (ホットキー)。
    pub(crate) fn motion_rejected() -> Self {
        Self::new(
            "animation.motion_rejected",
            "the motion did not start because a motion with a higher priority is playing",
        )
    }
}

impl From<tauri::Error> for CommandError {
    fn from(e: tauri::Error) -> Self {
        Self::internal(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn serializes_as_code_args_and_message() {
        let error = CommandError::from(ProfileError::FutureVersion {
            path: PathBuf::from("/p/a.json"),
            version: 9,
            supported: 1,
        });
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "profile.future_version",
                "args": { "path": "/p/a.json", "version": 9, "supported": 1 },
                "message": "/p/a.json: profile version 9 is newer than this studio (up to 1)",
            })
        );
        // 引数の無いエラーも `args` を (空で) 持つ。
        assert_eq!(
            serde_json::to_value(CommandError::from(AnimationError::NoModel)).unwrap(),
            json!({ "code": "animation.no_model", "args": {}, "message": "no model is on air" })
        );
    }

    #[test]
    fn scene_errors_carry_the_asset() {
        let error = CommandError::from(SceneError::Missing {
            asset: Asset::Module,
            path: PathBuf::from("/m/hat.ugoemod"),
        });
        assert_eq!(error.code, "scene.module_missing");
        assert_eq!(
            serde_json::to_value(&error.args).unwrap(),
            json!({ "asset": "module", "path": "/m/hat.ugoemod" })
        );
        let error = CommandError::from(SceneError::Unreadable {
            asset: Asset::Model,
            path: PathBuf::from("/m/a.ugoe"),
            message: "broken".into(),
        });
        assert_eq!(error.code, "scene.unreadable");
        assert_eq!(error.args["reason"], json!("broken"));
        // `asset` の名前は stage.rs の `Asset` の JSON と同じ。
        for asset in [Asset::Model, Asset::Module, Asset::Background] {
            assert_eq!(
                json!(asset_name(asset)),
                serde_json::to_value(asset).unwrap()
            );
        }
    }

    /// すべてのコードが ADR 0029 の書き方 (小文字の `snake_case` を `.` でつなぐ) で、`message` を持ち、
    /// 両方の言語のカタログに `errors.<code>` の文がある。文が差し込む引数は `args` にある。
    #[test]
    fn every_code_is_dotted_snake_case_and_translated() {
        let catalogs: Vec<Value> = [
            include_str!("../../locales/en.json"),
            include_str!("../../locales/ja.json"),
        ]
        .iter()
        .map(|text| serde_json::from_str(text).unwrap())
        .collect();
        let path = PathBuf::from("/p");
        let errors = [
            CommandError::internal("boom"),
            CommandError::scene_not_current(3),
            CommandError::scene_no_file(3),
            CommandError::unsupported_tick_rate(45),
            CommandError::invalid_render_stats(),
            CommandError::gpu_settings_save_failed("x"),
            CommandError::graphics_settings_unsupported(),
            CommandError::graphics_settings_failed("x"),
            CommandError::output_window_missing(),
            CommandError::output_window_failed("x"),
            CommandError::unsupported_output_size(1, 2),
            CommandError::invalid_camera_config(ugoe_studio_vcam::Config {
                width: 1,
                height: 2,
                fps: 3,
            }),
            CommandError::unsupported_api_port(80, 1024),
            CommandError::unknown_pending_client("a"),
            CommandError::too_many_api_clients(64),
            CommandError::random_failed("x"),
            CommandError::motion_rejected(),
            CommandError::not_chosen(&path),
            CommandError::not_chosen_for_save(&path),
            CommandError::dialog_failed("x"),
            ProfileError::Io {
                path: path.clone(),
                message: "x".into(),
            }
            .into(),
            ProfileError::TooLarge {
                path: path.clone(),
                bytes: 1,
            }
            .into(),
            ProfileError::NotJson {
                path: path.clone(),
                message: "x".into(),
            }
            .into(),
            ProfileError::NotProfile { path: path.clone() }.into(),
            ProfileError::FutureVersion {
                path: path.clone(),
                version: 2,
                supported: 1,
            }
            .into(),
            ProfileError::Invalid {
                path: path.clone(),
                message: "x".into(),
            }
            .into(),
            SceneError::UnknownScene { id: "s".into() }.into(),
            SceneError::Missing {
                asset: Asset::Model,
                path: path.clone(),
            }
            .into(),
            SceneError::Missing {
                asset: Asset::Module,
                path: path.clone(),
            }
            .into(),
            SceneError::Unreadable {
                asset: Asset::Module,
                path: path.clone(),
                message: "x".into(),
            }
            .into(),
            SceneError::Missing {
                asset: Asset::Background,
                path: path.clone(),
            }
            .into(),
            SceneError::WrongType {
                asset: Asset::Model,
                path: path.clone(),
            }
            .into(),
            SceneError::TooLarge {
                asset: Asset::Model,
                path: path.clone(),
                bytes: 1,
            }
            .into(),
            SceneError::DressUp {
                message: "x".into(),
            }
            .into(),
            SceneError::UnknownPose { part: "p".into() }.into(),
            AnimationError::UnknownMotion { name: "m".into() }.into(),
            AnimationError::UnknownExpression { name: "e".into() }.into(),
        ];
        for error in errors {
            assert!(
                error.code.split('.').all(|part| !part.is_empty()
                    && part.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
                "{}",
                error.code
            );
            assert!(!error.message.is_empty(), "{}", error.code);
            for catalog in &catalogs {
                let text = error
                    .code
                    .split('.')
                    .fold(&catalog["errors"], |node, part| &node[part]);
                let text = text.as_str().unwrap_or_else(|| panic!("{}", error.code));
                for rest in text.split("{{").skip(1) {
                    let name = rest.split("}}").next().unwrap();
                    assert!(error.args.contains_key(name), "{}: {name}", error.code);
                }
            }
        }
    }
}
