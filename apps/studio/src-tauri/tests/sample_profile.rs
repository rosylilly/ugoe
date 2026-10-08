//! 見本のプロファイル (assets/samples の `ugoe-sample-profile.json`・`ugoe-sample-profile.en.json`。T0865) を、スタジオの
//! 読み込み (`Profile::read`。相対パスはプロファイルのフォルダを基準に解く。docs/studio.md#プロファイルのパス) で開ける。
//! フォルダごと別の場所へ写しても (`just check-pack` の `samples\`)、写した先の見本を指す。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use ugoe_studio_lib::hotkeys::HotkeyAction;
use ugoe_studio_lib::profile::{Background, Profile};

const PROFILES: [&str; 2] = ["ugoe-sample-profile.json", "ugoe-sample-profile.en.json"];

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/samples")
}

/// プロファイルの中のファイルのパス (シーンのモデル・モジュール・背景の画像と、ホットキーの着せ替えのモジュール)。
fn files_of(profile: &Profile) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for scene in &profile.scenes {
        files.extend(scene.model.clone());
        files.extend(scene.modules.iter().cloned());
        if let Background::Image { path } = &scene.background {
            files.push(path.clone());
        }
    }
    for hotkey in &profile.hotkeys {
        if let HotkeyAction::DressUp {
            module: Some(module),
            ..
        } = &hotkey.action
        {
            files.push(module.clone());
        }
    }
    files
}

/// `dir` に置いた見本のプロファイルを読み、すべてのファイルが `dir` の下の、あるファイルに解けることを確かめる。
fn check_opens(dir: &Path, name: &str) {
    let text = std::fs::read_to_string(dir.join(name)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let profile = Profile::read(&dir.join(name)).unwrap();
    // 読むときに落としたもの (範囲の外の値・ぶつかるホットキー) が無い
    assert_eq!(
        profile.scenes.len(),
        json["scenes"].as_array().unwrap().len()
    );
    assert_eq!(
        profile.hotkeys.len(),
        json["hotkeys"].as_array().unwrap().len()
    );
    let files = files_of(&profile);
    assert!(files.len() >= 4, "{name}: {files:?}");
    for file in files {
        assert!(file.is_absolute(), "{name}: {}", file.display());
        assert!(file.starts_with(dir), "{name}: {}", file.display());
        assert!(file.is_file(), "{name}: {} does not exist", file.display());
    }
}

#[test]
fn opens_the_sample_profiles_in_the_repository() {
    let dir = samples_dir();
    for name in PROFILES {
        check_opens(&dir, name);
    }
}

#[test]
fn opens_the_sample_profiles_after_moving_the_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let moved = tmp.path().join("ugoe-check").join("samples");
    copy_dir(&samples_dir(), &moved);
    for name in PROFILES {
        check_opens(&moved, name);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(entry.file_name()));
        } else if path
            .extension()
            .is_some_and(|e| e == "ugoe" || e == "ugoemod" || e == "json")
        {
            std::fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}
