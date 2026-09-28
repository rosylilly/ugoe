//! GPU を使わない範囲で `ugoe` を実行して確かめるテスト。

use std::process::{Command, Output};

fn ugoe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ugoe"))
        .args(args)
        .output()
        .expect("run ugoe")
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("utf-8")
}

#[test]
fn fixture_list_names_every_fixture() {
    let listed = stdout(&ugoe(&["fixture", "list"]));
    assert!(listed.lines().any(|l| l == "single_quad"));
    assert!(listed.lines().any(|l| l == "intersection_masks"));
    assert_eq!(stdout(&ugoe(&["fixture"])), listed);
}

#[test]
fn fixture_json_round_trips_through_info() {
    let json = stdout(&ugoe(&["fixture", "single-quad"]));
    let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("single_quad.json");
    std::fs::write(&path, json).expect("write fixture");
    let info = stdout(&ugoe(&[
        "info",
        path.to_str().expect("utf-8 path"),
        "--json",
    ]));
    let v: serde_json::Value = serde_json::from_str(&info).expect("json");
    assert_eq!(v["name"], "single_quad");
    assert_eq!(v["parameters"][0]["id"], "X");
    assert_eq!(
        stdout(&ugoe(&["validate", "fixture:single_quad"])).trim(),
        "ok"
    );
}

#[test]
fn unknown_fixture_lists_known_names() {
    let output = ugoe(&["fixture", "nope"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("single_quad"), "{stderr}");
}

/// 引数の誤りは GPU を使う前に報告される。
#[test]
fn render_rejects_unknown_param_before_touching_gpu() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("never.png");
    let output = ugoe(&[
        "render",
        "fixture:single_quad",
        "-o",
        out.to_str().expect("utf-8 path"),
        "--param",
        "Nope=1",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown parameter `Nope` (known: X)"),
        "{stderr}"
    );
    assert!(!out.exists());
}

#[test]
fn render_rejects_bad_size() {
    let output = ugoe(&[
        "render",
        "fixture:single_quad",
        "-o",
        "x.png",
        "--size",
        "0x10",
    ]);
    assert!(!output.status.success());
}

#[test]
fn import_psd_writes_project_and_bundle() {
    let psd = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures/layers.psd");
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let project = tmp.join("layers-import.ugoeproj");
    let bundle = tmp.join("layers-import.ugoe");
    let _ = std::fs::remove_dir_all(&project);
    let output = ugoe(&[
        "import-psd",
        psd.to_str().expect("utf-8 path"),
        "-o",
        project.to_str().expect("utf-8 path"),
        "--bundle",
        bundle.to_str().expect("utf-8 path"),
    ]);
    let out = stdout(&output);
    assert!(out.contains("3 parts, 6 meshes, 64x48 canvas"), "{out}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: レベル補正: adjustment"),
        "{stderr}"
    );
    assert!(project.join("textures").is_dir());
    assert_eq!(
        stdout(&ugoe(&["validate", bundle.to_str().expect("utf-8 path")])).trim(),
        "ok"
    );
}
