//! 分類の答えのキャッシュ (`classification_cache.rs`) のテスト。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Project;
use crate::classification_cache::{
    CLASSIFICATION_CACHE_VERSION, CachedClassification, MAX_CLASSIFICATION_CACHE_BYTES,
    PsdIdentity, classification_cache_path, read_classification_cache, write_classification_cache,
};
use crate::project::ProjectError;

/// レイヤー 1 枚の答えの代わり (このクレートは `ugoe_ai::classify::LayerLabel` を知らない)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Answer {
    layer_path: Vec<String>,
    label: String,
}

fn answer(path: &[&str], label: &str) -> Answer {
    Answer {
        layer_path: path.iter().map(|n| (*n).to_owned()).collect(),
        label: label.to_owned(),
    }
}

fn cache(psd: &Path, bytes: &[u8]) -> CachedClassification<Answer> {
    CachedClassification {
        psd: psd.to_path_buf(),
        identity: PsdIdentity::of(bytes),
        by_ai: true,
        instructions: Some("the ribbon is a hair accessory".into()),
        layers: vec![
            answer(&["Face", "Eye L"], "eye_white"),
            answer(&["Hair"], "front_hair"),
        ],
    }
}

#[test]
fn the_identity_is_fnv1a_of_the_bytes() {
    // FNV-1a 64 の既知の値 (空・"a")。ファイルに残すので、値が変わらないことを確かめる。
    assert_eq!(PsdIdentity::of(b"").hash, 0xcbf2_9ce4_8422_2325);
    assert_eq!(PsdIdentity::of(b"a").hash, 0xaf63_dc4c_8601_ec8c);
    let id = PsdIdentity::of(b"8BPS");
    assert_eq!(id.size, 4);
    assert!(id.matches(b"8BPS"));
    assert!(!id.matches(b"8BPX"));
    assert!(!id.matches(b"8BPS!"));
}

#[test]
fn a_cache_round_trips_with_a_relative_psd_path() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("a.ugoeproj");
    Project::empty("a").save(&project).unwrap();
    let psd = project.join("sources").join("a.psd");
    let written = cache(&psd, b"psd bytes");
    write_classification_cache(&project, &written).unwrap();

    let text = fs::read_to_string(classification_cache_path(&project)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["version"], CLASSIFICATION_CACHE_VERSION);
    assert_eq!(json["psd"]["path"], "sources/a.psd");
    assert_eq!(json["psd"]["size"], 9);
    assert_eq!(
        json["psd"]["hash"],
        format!("{:016x}", PsdIdentity::of(b"psd bytes").hash)
    );
    assert!(text.ends_with("}\n"));

    let read = read_classification_cache::<Answer>(&project)
        .unwrap()
        .unwrap();
    assert_eq!(read.psd, crate::project::normalize_path(&psd));
    assert_eq!(read.identity, written.identity);
    assert!(read.by_ai);
    assert_eq!(read.instructions, written.instructions);
    assert_eq!(read.layers, written.layers);

    // 中身が同じなら書き直さない。
    let before = fs::metadata(classification_cache_path(&project))
        .unwrap()
        .modified()
        .unwrap();
    write_classification_cache(&project, &written).unwrap();
    let after = fs::metadata(classification_cache_path(&project))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn a_psd_outside_the_project_is_written_as_an_absolute_path() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("a.ugoeproj");
    let psd = dir.path().join("art").join("a.psd");
    write_classification_cache(&project, &cache(&psd, b"x")).unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(classification_cache_path(&project)).unwrap()).unwrap();
    let written = json["psd"]["path"].as_str().unwrap();
    assert!(Path::new(written).is_absolute(), "{written}");
    let read = read_classification_cache::<Answer>(&project)
        .unwrap()
        .unwrap();
    assert_eq!(read.psd, crate::project::normalize_path(&psd));
}

#[test]
fn saving_and_loading_the_project_leave_the_cache_alone() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("a.ugoeproj");
    Project::empty("a").save(&project).unwrap();
    write_classification_cache(&project, &cache(&project.join("a.psd"), b"x")).unwrap();
    let loaded = Project::load(&project).unwrap();
    loaded.save(&project).unwrap();
    assert!(
        read_classification_cache::<Answer>(&project)
            .unwrap()
            .is_some()
    );
    // project.json は変えない。
    assert!(!loaded.meta.editor.contains_key("classification"));
}

#[test]
fn a_missing_or_unreadable_cache_is_treated_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    assert!(
        read_classification_cache::<Answer>(project)
            .unwrap()
            .is_none()
    );
    let file = classification_cache_path(project);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let good = serde_json::json!({
        "version": CLASSIFICATION_CACHE_VERSION,
        "psd": { "path": "a.psd", "size": 1, "hash": "00000000000000ff" },
        "by_ai": false,
        "layers": [{ "layer_path": ["A"], "label": "face" }],
    });
    fs::write(&file, good.to_string()).unwrap();
    let read = read_classification_cache::<Answer>(project)
        .unwrap()
        .unwrap();
    assert_eq!(read.identity.hash, 0xff);
    assert!(read.instructions.is_none());

    let mut broken = Vec::new();
    for (key, value) in [
        (
            "version",
            serde_json::json!(CLASSIFICATION_CACHE_VERSION + 1),
        ),
        ("version", serde_json::json!("1")),
        ("by_ai", serde_json::json!(null)),
        ("layers", serde_json::json!([{ "label": "face" }])),
        (
            "psd",
            serde_json::json!({ "path": "a.psd", "size": 1, "hash": "xyz" }),
        ),
        (
            "psd",
            serde_json::json!({ "path": "", "size": 1, "hash": "ff" }),
        ),
        (
            "psd",
            serde_json::json!({ "path": "a.psd", "size": -1, "hash": "ff" }),
        ),
    ] {
        let mut value_json = good.clone();
        value_json[key] = value;
        broken.push(value_json.to_string());
    }
    broken.push("{".into());
    broken.push(String::new());
    for text in broken {
        fs::write(&file, &text).unwrap();
        assert!(
            read_classification_cache::<Answer>(project)
                .unwrap()
                .is_none(),
            "{text}"
        );
    }
}

#[test]
fn too_many_layers_are_treated_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let file = classification_cache_path(dir.path());
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let layers = vec![serde_json::json!({ "layer_path": ["A"], "label": "face" }); 65_537];
    let text = serde_json::json!({
        "version": CLASSIFICATION_CACHE_VERSION,
        "psd": { "path": "a.psd", "size": 1, "hash": "ff" },
        "by_ai": false,
        "layers": layers,
    });
    fs::write(&file, text.to_string()).unwrap();
    assert!(
        read_classification_cache::<Answer>(dir.path())
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_cache_that_is_not_a_regular_file_or_too_large_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = classification_cache_path(dir.path());
    // ディレクトリ (通常のファイルでない) は開かない。
    fs::create_dir_all(&file).unwrap();
    assert!(matches!(
        read_classification_cache::<Answer>(dir.path()),
        Err(ProjectError::NotRegularFile { .. })
    ));
    fs::remove_dir(&file).unwrap();
    // 上限より大きいファイルは読まない (中身を作らずに大きさだけを伸ばす)。
    let f = fs::File::create(&file).unwrap();
    f.set_len(MAX_CLASSIFICATION_CACHE_BYTES + 1).unwrap();
    assert!(matches!(
        read_classification_cache::<Answer>(dir.path()),
        Err(ProjectError::FileTooLarge { .. })
    ));
}
