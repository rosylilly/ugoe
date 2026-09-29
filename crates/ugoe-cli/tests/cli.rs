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
fn validate_checks_the_avatar_profile() {
    let out = stdout(&ugoe(&[
        "validate",
        "fixture:avatar-profile",
        "--profile",
        "avatar",
    ]));
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["avatar profile: claims level 3, satisfies level 3", "ok"]
    );

    // 構造は正しいが、プロファイルを持たないモデル。
    assert_eq!(
        stdout(&ugoe(&["validate", "fixture:single_quad"])).trim(),
        "ok"
    );
    let output = ugoe(&["validate", "fixture:single_quad", "--profile", "avatar"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no avatar profile"), "{stderr}");
}

#[test]
fn standard_params_include_extended_parameters() {
    let out = stdout(&ugoe(&["standard-params"]));
    let ids: Vec<&str> = out
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert!(ids.contains(&"AngleX"));
    assert!(ids.contains(&"TongueOut"));
    assert!(ids.contains(&"MouthVowelA"));
    assert_eq!(ids.iter().filter(|&&id| id == "EyeLSmile").count(), 1);
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

/// M3 の受け入れ (T0310): コミットした見本のアバター (T0702) がアバタープロファイルの検査を通る。
#[test]
fn committed_avatar_sample_passes_the_avatar_profile() {
    let sample = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/samples/avatar.ugoe"
    );
    let out = stdout(&ugoe(&["validate", sample, "--profile", "avatar"]));
    let lines: Vec<&str> = out.lines().collect();
    // 見本は T0704 で作り直すので、レベルは固定しない (主張したレベルを満たしていればよい)。
    let (claimed, satisfied) = lines[0]
        .strip_prefix("avatar profile: claims level ")
        .and_then(|rest| rest.split_once(", satisfies level "))
        .unwrap_or_else(|| panic!("{out}"));
    assert_eq!(claimed, satisfied, "{out}");
    assert_eq!(lines[1..], ["ok"]);
}

#[test]
fn render_rejects_unknown_motion_before_touching_gpu() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("never.png");
    let output = ugoe(&[
        "render",
        "fixture:single_quad",
        "-o",
        out.to_str().expect("utf-8 path"),
        "--motion",
        "Idle",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no motion named `Idle`"), "{stderr}");
    assert!(!out.exists());
}

/// `ugoe build`: プロジェクトとバンドルのどちらからも書き出せ、`--atlas` でテクスチャが 1 枚にまとまる。
/// 見た目が変わらないことは `atlas_render_gpu.rs` で確かめる。
#[test]
fn build_packs_textures_into_an_atlas() {
    let psd = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures/layers.psd");
    // 同時に走る別のテストの実行と重ならないように、プロセスごとのディレクトリに書く。
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create output dir");
    let project = tmp.join("layers-build.ugoeproj");
    let plain = tmp.join("layers-build.ugoe");
    let atlased = tmp.join("layers-build-atlas.ugoe");
    let rebuilt = tmp.join("layers-build-rebuilt.ugoe");
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    stdout(&ugoe(&["import-psd", &path(&psd), "-o", &path(&project)]));
    let textures = |bundle: &std::path::Path| -> Vec<String> {
        let info = stdout(&ugoe(&["info", &path(bundle), "--json"]));
        let v: serde_json::Value = serde_json::from_str(&info).expect("json");
        v["textures"]
            .as_array()
            .expect("textures")
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_owned())
            .collect()
    };

    stdout(&ugoe(&["build", &path(&project), "-o", &path(&plain)]));
    assert_eq!(textures(&plain).len(), 6);

    let out = stdout(&ugoe(&[
        "build",
        &path(&project),
        "-o",
        &path(&atlased),
        "--atlas",
    ]));
    assert!(
        out.contains("packed 6 texture(s) into 1 atlas(es)"),
        "{out}"
    );
    assert_eq!(textures(&atlased), ["atlas-0"]);
    assert_eq!(stdout(&ugoe(&["validate", &path(&atlased)])).trim(), "ok");

    // バンドルからも作れる。
    stdout(&ugoe(&[
        "build",
        &path(&plain),
        "-o",
        &path(&rebuilt),
        "--atlas",
    ]));
    assert_eq!(textures(&rebuilt), ["atlas-0"]);

    // アトラスの設定は --atlas と一緒にだけ使える。
    let output = ugoe(&[
        "build",
        &path(&project),
        "-o",
        &path(&rebuilt),
        "--atlas-max-size",
        "512",
    ]);
    assert!(!output.status.success());
}
