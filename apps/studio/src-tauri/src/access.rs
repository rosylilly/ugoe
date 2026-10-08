//! webview に読み書きを許したパス (T0838。ビューアの `FileAccess` (T0727) と同じ考え)。
//!
//! webview で思わぬスクリプトが動いても、利用者が選んでいないファイルを読み書きできないようにする。ファイルのダイアログは
//! バックエンドが開き (`open_file_dialog`・`save_file_dialog`)、選ばれたパスを覚える。コマンドの一覧と例外は
//! README の「ファイルの読み書きの許可」。
//!
//! - 読める: 開くダイアログで選んだパス (このセッションの間)・今のプロファイルにあるパス (シーンのモデル・モジュール・背景の画像と、
//!   ホットキーの着せ替えのモジュール)・最近のプロファイル (アプリのデータの場所にバックエンドが書いたもの)。
//!   プロファイルにあるパスは、利用者がそのプロファイルを選んで開いた (または前に自分で選んで保存した) ものとして扱う。
//!   ファイルの中の相対パスは読むときに絶対パスに解いてある (`profile::resolve_path`) ので、解いた絶対パスで比べる。
//!   フォルダの外を指す相対パスは解かずに残るが、`stage::read_asset` が相対パスを読まない。
//! - 書ける: 保存のダイアログで選んだパス。1 回書いたら忘れる。今の保存先 (`save_profile` の `path` を省いたとき) は
//!   バックエンドが持つので、選び直さずに書ける。

use std::path::{Path, PathBuf};

use crate::error::CommandError;
use crate::hotkeys::{Hotkey, HotkeyAction};
use crate::profile::{Background, Profile, SceneSettings};
use crate::stage::Stage;

/// webview に読み書きを許したパス。ダイアログで選んだパスの扱いはエディタ・ビューアと同じ `ugoe_fs::FileAccess` で、
/// ここではスタジオの規則 (プロファイルにあるパス・最近のプロファイルも読める) とエラーのコードを足す。
#[derive(Debug, Default)]
pub(crate) struct FileAccess(ugoe_fs::FileAccess);

impl FileAccess {
    /// 開くダイアログで選んだパスを、読めるパスに加える。
    pub(crate) fn grant_readable(&self, paths: impl IntoIterator<Item = PathBuf>) {
        self.0.grant_readable(paths);
    }

    /// 保存のダイアログで選んだパスを、1 回だけ書けるパスにする。
    pub(crate) fn grant_writable(&self, path: PathBuf) {
        self.0.grant_writable(path);
    }

    /// 書けるパスなら、書けるパスから除いて `Ok` を返す (同じパスにもう 1 回書くには、またダイアログで選ぶ)。
    pub(crate) fn take_writable(&self, path: &Path) -> Result<(), CommandError> {
        if self.0.take_writable(path) {
            Ok(())
        } else {
            Err(CommandError::not_chosen_for_save(path))
        }
    }

    /// `path` を読んでよいか (ダイアログで選んだ・`stage` のプロファイルにある・最近のプロファイル)。
    pub(crate) fn check_readable(&self, path: &Path, stage: &Stage) -> Result<(), CommandError> {
        let allowed = self.0.is_chosen(path)
            || profile_paths(stage.profile()).any(|p| p == path)
            || stage.recent().iter().any(|p| p == path);
        if allowed {
            Ok(())
        } else {
            Err(CommandError::not_chosen(path))
        }
    }

    /// シーンの設定が指すファイル (モデル・モジュール・背景の画像) をすべて読んでよいか。
    pub(crate) fn check_scene(
        &self,
        scene: &SceneSettings,
        stage: &Stage,
    ) -> Result<(), CommandError> {
        scene_paths(scene).try_for_each(|path| self.check_readable(path, stage))
    }

    /// ホットキーの割り当てが指すファイル (着せ替えのモジュール) をすべて読んでよいか。
    pub(crate) fn check_hotkeys(
        &self,
        hotkeys: &[Hotkey],
        stage: &Stage,
    ) -> Result<(), CommandError> {
        hotkeys
            .iter()
            .filter_map(hotkey_path)
            .try_for_each(|path| self.check_readable(path, stage))
    }
}

/// シーンが読むファイルのパス。
fn scene_paths(scene: &SceneSettings) -> impl Iterator<Item = &Path> {
    let background = match &scene.background {
        Background::Image { path } => Some(path.as_path()),
        _ => None,
    };
    scene
        .model
        .as_deref()
        .into_iter()
        .chain(scene.modules.iter().map(PathBuf::as_path))
        .chain(background)
}

/// ホットキーの操作が読むファイルのパス (着せ替えのモジュール)。
fn hotkey_path(hotkey: &Hotkey) -> Option<&Path> {
    match &hotkey.action {
        HotkeyAction::DressUp {
            module: Some(path), ..
        } => Some(path),
        _ => None,
    }
}

/// プロファイルにあるパス (すべてのシーンが読むファイルと、ホットキーの着せ替えのモジュール)。
fn profile_paths(profile: &Profile) -> impl Iterator<Item = &Path> {
    profile
        .scenes
        .iter()
        .flat_map(scene_paths)
        .chain(profile.hotkeys.iter().filter_map(hotkey_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(id: &str, model: &str) -> SceneSettings {
        SceneSettings {
            id: id.into(),
            name: id.into(),
            model: Some(PathBuf::from(model)),
            modules: Vec::new(),
            lighting: None,
            transform: Default::default(),
            background: Background::Transparent,
            pose: Vec::new(),
        }
    }

    /// プロファイル (モデル・モジュール・背景の画像・ホットキーのモジュール) を開いた舞台。
    fn stage_with_profile(dir: &Path) -> Stage {
        let mut a = scene("a", "/m/a.ugoe");
        a.modules = vec![PathBuf::from("/m/hat.ugoemod")];
        a.background = Background::Image {
            path: PathBuf::from("/m/bg.png"),
        };
        let hotkeys: Vec<Hotkey> = serde_json::from_value(serde_json::json!([
            { "keys": "Ctrl+1", "action": { "kind": "dressUp", "slot": "hat", "module": "/m/cap.ugoemod" } }
        ]))
        .unwrap();
        let mut profile = Profile::new();
        profile.scenes = vec![a];
        profile.hotkeys = hotkeys;
        let path = dir.join("profile.json");
        profile.write(&path).unwrap();
        let mut stage = Stage::new(Some(dir.join("recent.json")));
        stage
            .open_profile(&path, &mut crate::conductor::Conductor::new())
            .ok();
        stage
    }

    #[test]
    fn reads_only_chosen_files_and_files_in_the_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let stage = stage_with_profile(tmp.path());
        let access = FileAccess::default();
        for path in ["/m/a.ugoe", "/m/hat.ugoemod", "/m/bg.png", "/m/cap.ugoemod"] {
            assert!(
                access.check_readable(Path::new(path), &stage).is_ok(),
                "{path}"
            );
        }
        // 開いたプロファイルは最近のプロファイルになり、開き直せる。
        let profile = tmp.path().join("profile.json");
        assert!(access.check_readable(&profile, &stage).is_ok());
        // 選んでいない・プロファイルに無いパスは読めない (同じディレクトリのほかのファイル・ほかのアプリの設定)。
        for path in ["/m/other.ugoe", "/home/u/.config/app/settings.json"] {
            let error = access.check_readable(Path::new(path), &stage).unwrap_err();
            assert_eq!(error.code, "file.not_chosen", "{path}");
        }
        access.grant_readable([PathBuf::from("/m/other.ugoe")]);
        assert!(
            access
                .check_readable(Path::new("/m/other.ugoe"), &stage)
                .is_ok()
        );
    }

    /// 相対パスで書いたプロファイルのファイルは、解いた絶対パスで読める (T0852)。
    #[test]
    fn reads_files_written_as_relative_paths_by_their_absolute_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("profile.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "format": "ugoe-studio-profile",
                "version": 1,
                "scenes": [{ "id": "a", "name": "A", "model": "m/a.ugoe", "modules": ["../x.ugoemod"] }],
            })
            .to_string(),
        )
        .unwrap();
        let mut stage = Stage::new(None);
        stage
            .open_profile(&path, &mut crate::conductor::Conductor::new())
            .ok();
        let access = FileAccess::default();
        assert!(
            access
                .check_readable(&tmp.path().join("m/a.ugoe"), &stage)
                .is_ok()
        );
        // 字面の相対パスは、読めるパスにならない。
        assert!(
            access
                .check_readable(Path::new("m/a.ugoe"), &stage)
                .is_err()
        );
        // フォルダの外を指す相対パスは解かない。そのまま残るが、読むところ (`stage::read_asset`) が読まない。
        assert!(
            access
                .check_readable(&tmp.path().join("../x.ugoemod"), &stage)
                .is_err()
        );
    }

    #[test]
    fn checks_every_file_of_a_scene() {
        let tmp = tempfile::tempdir().unwrap();
        let stage = stage_with_profile(tmp.path());
        let access = FileAccess::default();
        // プロファイルにあるファイルだけのシーン (別のシーンへの複製・並べ替えなど)
        let mut b = scene("b", "/m/a.ugoe");
        b.modules = vec![PathBuf::from("/m/cap.ugoemod")];
        assert!(access.check_scene(&b, &stage).is_ok());
        // 選んでいないモデル・モジュール・背景の画像はどれも拒む
        let other = scene("b", "/etc/passwd.json");
        assert_eq!(
            access.check_scene(&other, &stage).unwrap_err().code,
            "file.not_chosen"
        );
        let mut module = b.clone();
        module.modules.push(PathBuf::from("/x/evil.ugoemod"));
        assert!(access.check_scene(&module, &stage).is_err());
        let mut background = b.clone();
        background.background = Background::Image {
            path: PathBuf::from("/x/secret.png"),
        };
        assert!(access.check_scene(&background, &stage).is_err());
        // ダイアログで選べば使える
        access.grant_readable([PathBuf::from("/x/secret.png")]);
        assert!(access.check_scene(&background, &stage).is_ok());
    }

    #[test]
    fn checks_the_modules_of_dress_up_hotkeys() {
        let tmp = tempfile::tempdir().unwrap();
        let stage = stage_with_profile(tmp.path());
        let access = FileAccess::default();
        let hotkeys = |module: &str| -> Vec<Hotkey> {
            serde_json::from_value(serde_json::json!([
                { "keys": "Ctrl+1", "action": { "kind": "stopMotions" } },
                { "keys": "Ctrl+2", "action": { "kind": "dressUp", "slot": "hat", "module": module } },
                { "keys": "Ctrl+3", "action": { "kind": "dressUp", "slot": "hat", "module": null } }
            ]))
            .unwrap()
        };
        assert!(
            access
                .check_hotkeys(&hotkeys("/m/hat.ugoemod"), &stage)
                .is_ok()
        );
        assert!(
            access
                .check_hotkeys(&hotkeys("/x/evil.ugoemod"), &stage)
                .is_err()
        );
    }

    #[test]
    fn writes_only_once_to_the_path_chosen_in_the_save_dialog() {
        let tmp = tempfile::tempdir().unwrap();
        let stage = stage_with_profile(tmp.path());
        let access = FileAccess::default();
        let path = PathBuf::from("/home/u/profile.json");
        assert_eq!(
            access.take_writable(&path).unwrap_err().code,
            "file.not_chosen_for_save"
        );
        access.grant_writable(path.clone());
        assert!(access.take_writable(Path::new("/home/u/.bashrc")).is_err());
        assert!(access.take_writable(&path).is_ok());
        assert!(access.take_writable(&path).is_err());
        // 書けるパスは読めるパスにならない。
        assert!(access.check_readable(&path, &stage).is_err());
    }
}
