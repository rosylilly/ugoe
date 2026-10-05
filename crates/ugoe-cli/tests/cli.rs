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

/// `ugoe validate --physics`: 物理の設定を検証し、静止姿勢で衝突体に当たっている鎖を警告する (エラーにはしない)。
#[test]
fn validate_warns_about_rest_pose_collisions() {
    use ugoe_core::Vec2;
    use ugoe_model::fixtures;
    use ugoe_model::physics::ColliderShape;

    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let write = |name: &str, json: String| {
        let path = tmp.join(name);
        std::fs::write(&path, json).expect("write json");
        path.to_str().expect("utf-8 path").to_owned()
    };
    let model = write(
        "dynamic_hair.json",
        ugoe_format::model_to_json(&fixtures::dynamic_hair()).expect("model json"),
    );
    let fine = write(
        "fine.physics.json",
        ugoe_format::physics_to_json(&fixtures::dynamic_hair_settings()).expect("physics json"),
    );
    // 肩を、真下に垂れた髪を横切る位置へ動かす。
    let mut settings = fixtures::dynamic_hair_settings();
    settings.colliders[0].shape = ColliderShape::Capsule {
        a: Vec2::new(-10.0, -60.0),
        b: Vec2::new(40.0, -40.0),
        radius: 8.0,
    };
    let crossing = write(
        "crossing.physics.json",
        ugoe_format::physics_to_json(&settings).expect("physics json"),
    );

    let output = ugoe(&["validate", &model, "--physics", &fine]);
    assert_eq!(stdout(&output).trim(), "ok");
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = ugoe(&["validate", &model, "--physics", &crossing]);
    assert_eq!(stdout(&output).trim(), "ok");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: physics rig `Hair` hits collider `Shoulder` in its rest pose"),
        "{stderr}"
    );

    // 物理の設定の誤りはエラー。
    settings.rigs[0].colliders = vec!["Missing".into()];
    let broken = write(
        "broken.physics.json",
        ugoe_format::physics_to_json(&settings).expect("physics json"),
    );
    let output = ugoe(&["validate", &model, "--physics", &broken]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
}

/// `ugoe validate <bundle>`: バンドルの物理が検証に通らなければ、モデルと同じく各エラーを `error: ...` で書く。
#[test]
fn validate_lists_physics_errors_in_a_bundle() {
    use ugoe_model::fixtures;

    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let mut settings = fixtures::dynamic_hair_settings();
    settings.rigs[0].colliders = vec!["Missing".into()];
    let mut bundle = ugoe_format::Bundle::new(fixtures::dynamic_hair());
    bundle.textures = vec![
        ugoe_format::TextureData {
            format: ugoe_format::TextureFormat::Png,
            bytes: ugoe_editor_core::texture::encode_rgba_png(1, 1, &[255; 4]),
        };
        bundle.model.textures.len()
    ];
    bundle.physics = Some(settings);
    let path = tmp.join("broken-physics.ugoe");
    std::fs::write(
        &path,
        ugoe_format::write_bundle(&bundle).expect("write bundle"),
    )
    .expect("write bundle file");

    let output = ugoe(&["validate", path.to_str().expect("utf-8 path")]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .lines()
            .any(|l| l.starts_with("error: ") && l.contains("Missing")),
        "{stderr}"
    );
    // T0015: 各エラーを 1 行ずつ書いたので、最後の文は列を繰り返さずに件数だけ。
    let last = stderr.lines().last().unwrap_or_default();
    assert!(last.ends_with("1 validation error(s)"), "{stderr}");
    assert_eq!(stderr.matches("Missing").count(), 1, "{stderr}");
}

/// `ugoe validate <bundle>`: 数値の壊れたモーションは各エラーを書いて失敗し、時刻が減る点は警告だけ書く。
#[test]
fn validate_checks_motions_in_a_bundle() {
    use ugoe_model::motion::{Curve, CurveTarget, Motion, Point, Segment};

    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let write = |name: &str, motion: Motion| {
        let mut bundle = ugoe_format::Bundle::new(ugoe_model::fixtures::single_quad());
        bundle.textures = vec![
            ugoe_format::TextureData {
                format: ugoe_format::TextureFormat::Png,
                bytes: ugoe_editor_core::texture::encode_rgba_png(1, 1, &[255; 4]),
            };
            bundle.model.textures.len()
        ];
        bundle.motions = vec![motion];
        let path = tmp.join(name);
        std::fs::write(
            &path,
            ugoe_format::write_bundle(&bundle).expect("write bundle"),
        )
        .expect("write bundle file");
        path
    };
    let motion = |second: f32, value: f32| Motion {
        name: "nod".to_owned(),
        duration: 1.0,
        looping: false,
        fade_in: 0.0,
        fade_out: 0.0,
        curves: vec![Curve {
            target: CurveTarget::Parameter("X".into()),
            segments: vec![
                Segment::Start {
                    p: Point { t: 0.5, v: 0.0 },
                },
                Segment::Linear {
                    to: Point {
                        t: second,
                        v: value,
                    },
                },
            ],
            fade_in: None,
            fade_out: None,
        }],
        events: Vec::new(),
    };

    let broken = write("broken-motion.ugoe", motion(1.0, f32::NAN));
    let output = ugoe(&["validate", broken.to_str().expect("utf-8 path")]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .lines()
            .any(|l| l == "error: nod: curves[0].segments[1].to.v = NaN must be finite"),
        "{stderr}"
    );

    let decreasing = write("decreasing-motion.ugoe", motion(0.25, 1.0));
    let output = ugoe(&["validate", decreasing.to_str().expect("utf-8 path")]);
    assert_eq!(stdout(&output).trim(), "ok");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .lines()
            .any(|l| l.starts_with("warning: motion `nod`: curves[0].segments[1].to.t")),
        "{stderr}"
    );
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
    assert!(ids.contains(&"ShoulderLY"));
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

/// `ugoe import-psd` は通常のファイルだけを、`ugoe_psd::MAX_PSD_FILE_BYTES` まで読む
/// (装置・FIFO・ディレクトリや巨大なファイルで止まったりメモリを使い果たしたりしない)。
#[cfg(unix)]
#[test]
fn import_psd_refuses_non_regular_files_and_files_over_the_limit() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let tmp = temp.path();
    let project = tmp.join("out.ugoeproj");
    let import = |input: &std::path::Path| {
        let output = ugoe(&[
            "import-psd",
            input.to_str().expect("utf-8 path"),
            "-o",
            project.to_str().expect("utf-8 path"),
        ]);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    };

    let fifo = tmp.join("fifo.psd");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("run mkfifo");
    assert!(status.success());
    for input in [std::path::Path::new("/dev/zero"), tmp, &fifo] {
        let err = import(input);
        assert!(err.contains("not a regular file"), "{err}");
    }

    // 上限より 1 バイト大きい疎なファイル (中身は書かないので、ディスクは使わない)。
    let large = tmp.join("large.psd");
    std::fs::File::create(&large)
        .expect("create large file")
        .set_len(ugoe_psd::MAX_PSD_FILE_BYTES + 1)
        .expect("extend large file");
    let err = import(&large);
    assert!(
        err.contains(&format!(
            "the limit is {} bytes",
            ugoe_psd::MAX_PSD_FILE_BYTES
        )),
        "{err}"
    );
    assert!(!project.exists());
}

#[test]
fn import_psd_writes_project_and_bundle() {
    let psd = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures/layers.psd");
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let project = tmp.join("layers-import.ugoeproj");
    let bundle = tmp.join("layers-import.ugoe");
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
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
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

/// `ugoe build`: 互換性テストの資料 (`assets/compat/dev`) のすべてを入力にして、同じ内容のファイルを書ける。
/// 法線マップ付きのバンドル (T0131) と、着せ替えのモジュール (`.ugoemod`) を含む。
/// 資料は無圧縮のものと圧縮したものがあり、`--compress` の有無のどちらかとはバイト列まで一致する。
#[test]
fn build_round_trips_every_compat_file() {
    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat/dev");
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let mut inputs: Vec<_> = std::fs::read_dir(&dev)
        .expect("read assets/compat/dev")
        .map(|e| e.expect("dir entry").path())
        .collect();
    inputs.sort();
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    let mut normal_maps = 0;
    let mut modules = 0;
    for input in &inputs {
        let name = input.file_name().expect("file name").to_string_lossy();
        let original = std::fs::read(input).expect("read compat file");
        let is_module = input.extension().is_some_and(|e| e == "ugoemod");
        let mut outputs = Vec::new();
        for (prefix, extra) in [("plain", None), ("compressed", Some("--compress"))] {
            let output = tmp.join(format!("{prefix}-{name}"));
            let (input, output) = (path(input), path(&output));
            let mut args = vec!["build", &input, "-o", &output];
            args.extend(extra);
            stdout(&ugoe(&args));
            let rebuilt = std::fs::read(&output).expect("read rebuilt");
            if is_module {
                let before = ugoe_format::read_module(&original).expect("module");
                let after = ugoe_format::read_module(&rebuilt).expect("rebuilt module");
                assert_eq!(after, before, "{name}");
            } else {
                let before = ugoe_format::read_bundle(&original).expect("bundle");
                let after = ugoe_format::read_bundle(&rebuilt).expect("rebuilt bundle");
                assert_eq!(after, before, "{name}");
                normal_maps += before.normal_maps.len();
            }
            outputs.push(rebuilt);
        }
        modules += usize::from(is_module);
        assert!(
            outputs.contains(&original),
            "{name}: no byte-identical rebuild"
        );
    }
    // 法線マップ付きのバンドルとモジュールを実際に通したこと。
    assert!(normal_maps > 0, "no compat bundle has a normal map");
    assert!(modules > 0, "no compat module");
}

/// `ugoe validate`: 着せ替えのモジュールを単体で検査し、`--module` で本体に付けた結果も検査する。
/// モジュールは描けないので `info` は分かるエラーにする。
#[test]
fn validate_accepts_modules() {
    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat/dev");
    let file = |name: &str| dev.join(name).to_str().expect("utf-8 path").to_owned();
    let (base, dress, hair) = (
        file("dress-up-base.ugoe"),
        file("dress.ugoemod"),
        file("long-hair.ugoemod"),
    );
    assert_eq!(stdout(&ugoe(&["validate", &dress])).trim(), "ok");
    assert_eq!(
        stdout(&ugoe(&[
            "validate", &base, "--module", &dress, "--module", &hair
        ]))
        .trim(),
        "ok"
    );

    let output = ugoe(&["validate", &dress, "--profile", "avatar"]);
    assert!(!output.status.success());
    let output = ugoe(&["render", &dress, "-o", "never-written.png"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is a dress-up module"), "{stderr}");
    // スロットの無い本体には付けられない。
    let output = ugoe(&["validate", &file("single-quad.ugoe"), "--module", &dress]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("slot `Outfit` does not exist"), "{stderr}");
}

/// `ugoe info <module>.ugoemod`: モジュールの概要 (スロット・条件・要素の数・テクスチャ) を出す。
#[test]
fn info_describes_modules() {
    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat/dev");
    let file = |name: &str| dev.join(name).to_str().expect("utf-8 path").to_owned();
    let text = stdout(&ugoe(&["info", &file("dress.ugoemod")]));
    assert!(text.contains("module:     Dress"), "{text}");
    assert!(text.contains("slot:       Outfit"), "{text}");
    assert!(
        text.contains("requires:   avatar level -; slots Hair; parameters BodySway"),
        "{text}"
    );
    assert!(text.contains("  0: `dress` 1x1 (png)"), "{text}");

    let json = stdout(&ugoe(&["info", &file("long-hair.ugoemod"), "--json"]));
    let v: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(v["kind"], "module");
    assert_eq!(v["name"], "LongHair");
    assert_eq!(v["slot"], "Hair");
    assert_eq!(v["meshes"], 2);
    assert_eq!(v["physics"]["rigs"], 1);
    assert_eq!(v["parameters"], serde_json::json!(["AngleX", "HairFront"]));
    assert_eq!(v["textures"][0]["name"], "long_hair");
    assert_eq!(v["textures"][0]["format"], "png");
    assert!(v["textures"][0]["normal_map"].is_null());

    // モデルの JSON は `kind: "model"`。
    let json = stdout(&ugoe(&["info", "fixture:single_quad", "--json"]));
    let v: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(v["kind"], "model");
}

/// `ugoe build <module>.ugoemod --atlas`: モジュールのテクスチャをモジュールの単位でまとめ、本体に付けられるままにする。
/// 見た目が変わらないことは `atlas_render_gpu.rs` で確かめる。
#[test]
fn build_packs_module_textures_into_an_atlas() {
    let samples =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/samples/dress-up");
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    let (base, module, atlased) = (
        samples.join("dress-up.ugoe"),
        samples.join("twin-tails.ugoemod"),
        tmp.join("twin-tails.ugoemod"),
    );
    let out = stdout(&ugoe(&[
        "build",
        &path(&module),
        "-o",
        &path(&atlased),
        "--atlas",
    ]));
    assert!(
        out.contains("packed 6 texture(s) into 1 atlas(es)"),
        "{out}"
    );
    let info = stdout(&ugoe(&["info", &path(&atlased), "--json"]));
    let v: serde_json::Value = serde_json::from_str(&info).expect("json");
    assert_eq!(v["textures"].as_array().expect("textures").len(), 1);
    assert_eq!(v["textures"][0]["name"], "atlas-0");
    assert_eq!(
        stdout(&ugoe(&[
            "validate",
            &path(&base),
            "--module",
            &path(&atlased)
        ]))
        .trim(),
        "ok"
    );
    // モジュールのほかの部分はそのまま。
    let read = |p: &std::path::Path| {
        ugoe_format::read_module(&std::fs::read(p).expect("read module")).expect("module")
    };
    let (before, after) = (read(&module), read(&atlased));
    assert_eq!(after.module.parts, before.module.parts);
    assert_eq!(after.module.physics, before.module.physics);
    assert_eq!(after.meta, before.meta);
}

/// 手で書いた資料の置き場 (`spec/examples/bundles/`・`bindings/unity/tests/Fixtures/`)。
fn repo_file(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// `ugoe build <file>.model.json`: JSON の隣の `textures/<name>.png` を画像に使い、無い画像は 1x1 で埋める。
/// 読み直すと同じモデルになる。
#[test]
fn build_writes_a_bundle_from_model_json() {
    use ugoe_editor_core::texture::solid_png;

    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    let json = tmp.join("lit-quads.model.json");
    std::fs::copy(
        repo_file("spec/examples/bundles/lit-quads.model.json"),
        &json,
    )
    .expect("copy model");
    std::fs::create_dir(tmp.join("textures")).expect("create textures");
    let (base, bumps) = (
        solid_png(2, 2, [200, 160, 120, 255]),
        solid_png(2, 2, [100, 140, 230, 255]),
    );
    std::fs::write(tmp.join("textures/base.png"), &base).expect("write base");
    std::fs::write(tmp.join("textures/bumps_a.png"), &bumps).expect("write bumps_a");

    let bundle = tmp.join("lit-quads.ugoe");
    let output = ugoe(&["build", &path(&json), "-o", &path(&bundle)]);
    stdout(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for name in ["texture `flat`", "texture `cloth`", "normal map `bumps_b`"] {
        assert!(
            stderr.contains(&format!("warning: {name} has no image")),
            "{stderr}"
        );
    }
    assert!(!stderr.contains("`base`"), "{stderr}");

    let read =
        ugoe_format::read_bundle(&std::fs::read(&bundle).expect("read bundle")).expect("bundle");
    let expected =
        ugoe_format::model_from_json(&std::fs::read_to_string(&json).expect("read json"))
            .expect("model");
    assert_eq!(read.model, expected);
    assert_eq!(read.textures.len(), 4);
    assert_eq!(read.textures[0].bytes, base);
    let white = solid_png(1, 1, [255, 255, 255, 255]);
    assert!(read.textures[1..].iter().all(|t| t.bytes == white));
    // 法線マップは `normal_map` を持つテクスチャごとに 1 つ (base → bumps_a、cloth と cloth_dark → bumps_b)。
    let flat = solid_png(1, 1, [128, 128, 255, 255]);
    let normals: Vec<&[u8]> = read
        .normal_maps
        .iter()
        .map(|t| t.bytes.as_slice())
        .collect();
    assert_eq!(normals, [bumps.as_slice(), &flat, &flat]);

    // `--atlas`: 法線マップの無いテクスチャだけをまとめ、法線マップのあるものは警告して残す。
    let atlased = tmp.join("lit-quads-atlas.ugoe");
    let output = ugoe(&["build", &path(&json), "-o", &path(&atlased), "--atlas"]);
    let out = stdout(&output);
    assert!(
        out.contains("packed 1 texture(s) into 1 atlas(es)"),
        "{out}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: texture `base` was not packed (it has a normal map)"),
        "{stderr}"
    );
    assert_eq!(stdout(&ugoe(&["validate", &path(&atlased)])).trim(), "ok");
}

/// `ugoe build <file>.module.json`: モジュールとして書き、読み直すと同じモジュールになる (`--atlas`・`--compress` とも)。
/// Unity の資料 (`unknown-mask.ugoemod`) と公開仕様の例 (`short-hair.ugoemod`) のモジュールも作れる。
#[test]
fn build_writes_a_module_from_module_json() {
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    let read = |p: &std::path::Path| {
        ugoe_format::read_module(&std::fs::read(p).expect("read module")).expect("module")
    };
    for (json, made) in [
        (
            "bindings/unity/tests/Fixtures/unknown-mask.module.json",
            "bindings/unity/tests/Fixtures/unknown-mask.ugoemod",
        ),
        (
            "spec/examples/bundles/short-hair.module.json",
            "spec/examples/bundles/short-hair.ugoemod",
        ),
    ] {
        let json = repo_file(json);
        let expected = read(&repo_file(made));
        let module =
            ugoe_format::module_from_json(&std::fs::read_to_string(&json).expect("read json"))
                .expect("module");
        assert_eq!(expected.module, module, "{made}");

        let out = tmp.join("plain.ugoemod");
        stdout(&ugoe(&["build", &path(&json), "-o", &path(&out)]));
        let built = read(&out);
        assert_eq!(built.module, module, "{made}");
        assert_eq!(built.textures.len(), module.textures.len());

        let out = tmp.join("atlas.ugoemod");
        let printed = stdout(&ugoe(&[
            "build",
            &path(&json),
            "-o",
            &path(&out),
            "--atlas",
            "--compress",
        ]));
        assert!(printed.contains("into 1 atlas(es)"), "{printed}");
        let built = read(&out);
        assert_eq!(built.module.parts, module.parts, "{made}");
        assert_eq!(built.module.physics, module.physics, "{made}");
        assert_eq!(built.module.textures[0].name, "atlas-0");
    }
}

/// `ugoe build` に渡した JSON が壊れている・検証に通らない・画像が PNG でないときは、バンドルと同じ形で失敗する。
#[test]
fn build_rejects_broken_or_invalid_json() {
    let temp = tempfile::tempdir().expect("create output dir");
    let tmp = temp.path();
    let path = |p: &std::path::Path| p.to_str().expect("utf-8 path").to_owned();
    let out = path(&tmp.join("out.ugoe"));
    let fails = |json: &std::path::Path| {
        let output = ugoe(&["build", &path(json), "-o", &out]);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    };

    let broken = tmp.join("broken.model.json");
    std::fs::write(&broken, "{\"schema_version\": 0,").expect("write broken");
    let stderr = fails(&broken);
    assert!(stderr.contains("parsing"), "{stderr}");

    // メッシュが無いテクスチャを指すモデル。
    let mut model: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_file(
            "bindings/unity/tests/Fixtures/mask-base.model.json",
        ))
        .expect("read model"),
    )
    .expect("model json");
    model["meshes"][0]["texture"] = 7.into();
    let invalid = tmp.join("invalid.model.json");
    std::fs::write(&invalid, model.to_string()).expect("write invalid model");
    let stderr = fails(&invalid);
    assert!(stderr.contains("error: "), "{stderr}");
    assert!(stderr.contains("1 validation error(s)"), "{stderr}");

    // 名前に `/` を含むモジュール。
    let mut module: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_file("spec/examples/bundles/short-hair.module.json"))
            .expect("read module"),
    )
    .expect("module json");
    module["name"] = "hair/short".into();
    let invalid = tmp.join("invalid.module.json");
    std::fs::write(&invalid, module.to_string()).expect("write invalid module");
    let stderr = fails(&invalid);
    assert!(stderr.contains("error: "), "{stderr}");
    assert!(stderr.contains("1 validation error(s)"), "{stderr}");

    // 隣の画像が PNG でない。
    let dir = tmp.join("not-png");
    std::fs::create_dir_all(dir.join("textures")).expect("create textures");
    let json = dir.join("mask-base.model.json");
    std::fs::copy(
        repo_file("bindings/unity/tests/Fixtures/mask-base.model.json"),
        &json,
    )
    .expect("copy model");
    std::fs::write(dir.join("textures/white.png"), b"not a png").expect("write image");
    let stderr = fails(&json);
    assert!(stderr.contains("is not a PNG"), "{stderr}");
}
