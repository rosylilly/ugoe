//! 互換性テスト用の資料 (`assets/compat/`) の生成と検査。
//! 資料の置き方・追加と再生成の手順: `docs/testing.md#互換性テスト`。
//! 仕様: `docs/spec/file-formats.md#機能の宣言と段階的な劣化`、判断の経緯: ADR 0013。
//!
//! - `dev/`: 今の書き出し (`write_bundle` / `write_module`) で作ったファイル。今のコードと一致することを検査する。
//! - `<版>/` (例 `1.0.0/`): リリースの時点の `dev/` の写し。作り直さず、今のランタイムで読めることだけを検査する。
//! - `future/`: 新しい版で書き出したファイルを模したもの。知らない機能・知らない値・知らないフィールドと
//!   チャンク、新しいスキーマの版を含む。コンテナはこのファイルの [`container`] で、書き出しのコードを使わずに組み立てる。
//!
//! 資料を作り直すには `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test compat` を実行する。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};
use ugoe_format::{
    Bundle, FeatureDecl, FormatError, ModuleFile, ReadWarning, TextureData, TextureFormat,
    WriteOptions, read_bundle_with_report, read_module_with_report, write_bundle,
    write_bundle_with, write_module, write_module_with,
};
use ugoe_model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe_model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe_model::{BlendMode, DeformerKind, MaskMode, Model, TextureRef, fixtures, standard};
use ugoe_runtime::{CompiledModel, DrawableState, ModelInstance, compose};

/// 資料を作り直すときに立てる環境変数。
const REGENERATE: &str = "UGOE_UPDATE_FIXTURES";

/// 今の書き出しで作る資料の置き場所。
const DEV: &str = "dev";

/// 新しい版を模した資料の置き場所。
const FUTURE: &str = "future";

/// 着せ替えのモジュール (`.ugoemod`) を取り付ける本体。どの版のディレクトリにも置く。
const MODULE_BASE: &str = "dress-up-base.ugoe";

/// 試験用の機能 ID。`test.` で始まる ID は試験のために予約されていて、本物の機能には使わない。
const FUTURE_OPTIONAL: &str = "test.future_optional";
const FUTURE_REQUIRED: &str = "test.future_required";

fn compat_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat")
}

fn read_file(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (作り直すには {REGENERATE}=1 cargo test -p ugoe-format --test compat)",
            path.display()
        )
    })
}

// --- 画像 ---------------------------------------------------------------------------------------

/// 1×1 の白い PNG。
const WHITE_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xf8, 0x0f, 0x04, 0x00,
    0x09, 0xfb, 0x03, 0xfd, 0x68, 0xfa, 0x1c, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

/// 1×1 の平らな法線 (`(128, 128, 255)`) の PNG。
const FLAT_NORMAL_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x68, 0x68, 0xf8, 0xff,
    0x1f, 0x00, 0x06, 0x82, 0x02, 0xff, 0x6c, 0xe0, 0x43, 0x23, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn png(bytes: &[u8]) -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: bytes.to_vec(),
    }
}

/// テクスチャの一覧に合わせた画像 (テクスチャと法線マップ) を作る。
fn images(textures: &[TextureRef]) -> (Vec<TextureData>, Vec<TextureData>) {
    let colors = textures.iter().map(|_| png(WHITE_PNG)).collect();
    let normals = textures
        .iter()
        .filter(|t| t.normal_map.is_some())
        .map(|_| png(FLAT_NORMAL_PNG))
        .collect();
    (colors, normals)
}

fn bundle(model: Model) -> Bundle {
    let (textures, normal_maps) = images(&model.textures);
    Bundle {
        textures,
        normal_maps,
        ..Bundle::new(model)
    }
}

fn module_file(module: ugoe_model::module::Module) -> ModuleFile {
    let (textures, normal_maps) = images(&module.textures);
    ModuleFile {
        textures,
        normal_maps,
        ..ModuleFile::new(module)
    }
}

// --- 今の版の資料 (dev/) ------------------------------------------------------------------------

/// 着せ替えの本体に、法線マップ・モーション・表情・メタデータを加えたバンドル。圧縮して書き出す。
fn dress_up_base_bundle() -> Bundle {
    let mut model = fixtures::dress_up_base();
    model.lighting = true;
    model.textures[0].normal_map = Some("body_normal".into());
    let p = |t, v| Point { t, v };
    let nod = Motion {
        name: "Nod".into(),
        duration: 1.0,
        looping: true,
        fade_in: 0.25,
        fade_out: 0.25,
        curves: vec![
            Curve {
                target: CurveTarget::Parameter(standard::ANGLE_X.into()),
                segments: vec![
                    Segment::Start { p: p(0.0, 0.0) },
                    Segment::Linear { to: p(0.25, 30.0) },
                    Segment::Bezier {
                        c1: p(0.5, 30.0),
                        c2: p(0.75, -30.0),
                        to: p(1.0, 0.0),
                    },
                ],
                fade_in: None,
                fade_out: None,
            },
            Curve {
                target: CurveTarget::PartOpacity("Hair".into()),
                segments: vec![
                    Segment::Start { p: p(0.0, 1.0) },
                    Segment::Stepped { to: p(1.0, 0.0) },
                ],
                fade_in: None,
                fade_out: None,
            },
        ],
        events: Vec::new(),
    };
    let tilt = Expression {
        name: "Tilt".into(),
        fade_in: 0.5,
        fade_out: 0.5,
        parameters: vec![ExpressionParameter {
            parameter: standard::ANGLE_X.into(),
            value: 10.0,
            op: ExpressionOp::Add,
        }],
    };
    Bundle {
        motions: vec![nod],
        expressions: vec![tilt],
        meta: Some(json!({ "author": "ugoe", "license": "CC0-1.0" })),
        ..bundle(model)
    }
}

fn with_physics(model: Model, physics: ugoe_model::physics::PhysicsSettings) -> Bundle {
    Bundle {
        physics: Some(physics),
        ..bundle(model)
    }
}

/// `dev/` に置く資料。1.0 の基準の機能 (`docs/spec/file-formats.md#1.0-の基準の機能`) をすべて含むようにする。
fn dev_corpus() -> Vec<(&'static str, Vec<u8>)> {
    let compressed = WriteOptions { compress: true };
    let bundles = [
        ("single-quad.ugoe", bundle(fixtures::single_quad())),
        (
            "pose-and-effects.ugoe",
            bundle(fixtures::pose_and_effects()),
        ),
        (
            "intersection-masks.ugoe",
            bundle(fixtures::intersection_masks()),
        ),
        (
            "keyform-inference.ugoe",
            bundle(fixtures::keyform_inference()),
        ),
        ("blend-shapes.ugoe", bundle(fixtures::blend_shapes())),
        ("smooth-arc.ugoe", bundle(fixtures::smooth_arc())),
        ("avatar-profile.ugoe", bundle(fixtures::avatar_profile())),
        (
            "physics-additive.ugoe",
            with_physics(
                fixtures::physics_additive(),
                fixtures::physics_additive_settings(),
            ),
        ),
        (
            "dynamic-hair.ugoe",
            with_physics(fixtures::dynamic_hair(), fixtures::dynamic_hair_settings()),
        ),
    ];
    let mut corpus: Vec<(&'static str, Vec<u8>)> = bundles
        .into_iter()
        .map(|(name, bundle)| (name, write_bundle(&bundle).expect(name)))
        .collect();
    corpus.push((
        MODULE_BASE,
        write_bundle_with(&dress_up_base_bundle(), compressed).expect(MODULE_BASE),
    ));
    corpus.push((
        "long-hair.ugoemod",
        write_module(&module_file(fixtures::long_hair_module())).expect("long-hair"),
    ));
    corpus.push((
        "dress.ugoemod",
        write_module_with(&module_file(fixtures::dress_module()), compressed).expect("dress"),
    ));
    corpus
}

// --- 新しい版を模した資料 (future/) -------------------------------------------------------------

/// 仕様 (`docs/spec/file-formats.md#バンドル-ugoe`) のとおりにコンテナを組み立てる。
/// 書き出しのコード (`ugoe_format::bundle`) を使わず、仕様だけから作る。
fn container(chunks: &[(&[u8; 4], u32, Vec<u8>)]) -> Vec<u8> {
    let align = |n: usize| n.div_ceil(16) * 16;
    let mut out = Vec::new();
    out.extend_from_slice(b"UGOE");
    out.extend_from_slice(&0u32.to_le_bytes()); // コンテナの版
    out.extend_from_slice(&0u32.to_le_bytes()); // 予約フラグ
    out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    let mut offset = 16 + 24 * chunks.len();
    for (tag, flags, payload) in chunks {
        offset = align(offset);
        out.extend_from_slice(*tag);
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&(offset as u64).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        offset += payload.len();
    }
    for (_, _, payload) in chunks {
        out.resize(align(out.len()), 0);
        out.extend_from_slice(payload);
    }
    out
}

fn msgpack(value: &Value) -> Vec<u8> {
    rmp_serde::to_vec_named(value).expect("msgpack")
}

fn to_value(value: &impl Serialize) -> Value {
    serde_json::to_value(value).expect("json")
}

fn feat(features: &[FeatureDecl]) -> Vec<u8> {
    msgpack(&json!({ "features": features }))
}

/// 新しい版で増えたチャンク。圧縮のフラグを立てているが中身は zstd ではない
/// (未知のチャンクは展開もしないことを確かめる)。
fn future_chunk() -> (&'static [u8; 4], u32, Vec<u8>) {
    (b"XFUT", 1, b"a chunk added in a future version".to_vec())
}

/// `items` のうち `id` が一致する要素。
fn by_id<'a>(items: &'a mut Value, id: &str) -> &'a mut Value {
    items
        .as_array_mut()
        .and_then(|items| items.iter_mut().find(|item| item["id"] == id))
        .unwrap_or_else(|| panic!("no item {id}"))
}

/// 新しい種類のデフォーマ `Bend` を `parent` の下に足し、メッシュ `mesh` の親にする。
fn insert_future_deformer(root: &mut Value, part: &str, parent: &str, mesh: &str) {
    root["deformers"].as_array_mut().expect("deformers").push(json!({
        "id": "Bend",
        "name": "Bend",
        "part": part,
        "parent": parent,
        "kind": { "type": "bezier_lattice", "handles": [[0.0, 1.0], [1.0, 0.0]], "stiffness": 0.5 },
    }));
    by_id(&mut root["meshes"], mesh)["parent"] = json!("Bend");
}

/// 知らない任意の機能・知らない値・知らないフィールドとチャンクを含むバンドル。
/// 元は `dev/dress-up-base.ugoe` と同じ内容。
fn future_optional_bundle() -> Vec<u8> {
    let base = dress_up_base_bundle();

    let mut model = to_value(&base.model);
    model["future_settings"] = json!({ "quality": "high", "levels": [1, 2, 3] });
    by_id(&mut model["parameters"], standard::ANGLE_X)["kind"] = json!("future_kind");
    insert_future_deformer(&mut model, "Root", "Head", "Face");
    let face = by_id(&mut model["meshes"], "Face");
    face["blend"] = json!("overlay");
    face["mask_mode"] = json!("exclusive_or");
    face["future_field"] = json!({ "nested": [1, 2, 3] });

    let mut motion = to_value(&base.motions[0]);
    motion["curves"][1]["segments"][1] = json!({ "type": "hermite", "to": { "t": 1.0, "v": 0.0 } });
    motion["curves"]
        .as_array_mut()
        .expect("curves")
        .push(json!({
            "target": { "kind": "camera", "id": { "zoom": 2.0 } },
            "segments": [{ "type": "start", "p": { "t": 0.0, "v": 1.0 } }],
        }));

    let mut expression = to_value(&base.expressions[0]);
    expression["parameters"]
        .as_array_mut()
        .expect("parameters")
        .push(json!({ "parameter": standard::ANGLE_X, "value": 1.0, "op": "screen" }));

    let mut chunks = vec![
        (
            b"FEAT",
            0,
            feat(&[
                FeatureDecl::optional(FUTURE_OPTIONAL),
                // 知っている機能の宣言は警告にならない。
                FeatureDecl::optional("mask.intersection"),
            ]),
        ),
        (b"MODL", 0, msgpack(&model)),
        (b"MOTN", 0, msgpack(&motion)),
        (b"EXPR", 0, msgpack(&expression)),
        (b"META", 0, serde_json::to_vec(&base.meta).expect("meta")),
        future_chunk(),
    ];
    chunks.extend(base.textures.iter().map(|t| (b"TEXR", 0, t.bytes.clone())));
    chunks.extend(
        base.normal_maps
            .iter()
            .map(|t| (b"NRML", 0, t.bytes.clone())),
    );
    container(&chunks)
}

/// 知らない任意の機能・知らない値・知らないフィールドとチャンクを含むモジュール。
/// 元は `dev/long-hair.ugoemod` と同じ内容。
fn future_optional_module() -> Vec<u8> {
    let base = module_file(fixtures::long_hair_module());

    let mut module = to_value(&base.module);
    module["future_settings"] = json!({ "quality": "high" });
    insert_future_deformer(&mut module, "Hair", "Sway", "Front");
    by_id(&mut module["meshes"], "Back")["blend"] = json!("overlay");
    let rig = &mut module["physics"]["rigs"][0];
    rig["inputs"][0]["source"] = json!("acceleration");
    rig["future_damping"] = json!(0.25);

    let mut chunks = vec![
        (b"FEAT", 0, feat(&[FeatureDecl::optional(FUTURE_OPTIONAL)])),
        (b"MODU", 0, msgpack(&module)),
        future_chunk(),
    ];
    chunks.extend(base.textures.iter().map(|t| (b"TEXR", 0, t.bytes.clone())));
    container(&chunks)
}

/// 知らない必須の機能を宣言したファイル。本体 (`tag`) は今の版では読めない形にしてあり、
/// デコードの前に機能の宣言で拒否されることを確かめる。
fn future_required(tag: &[u8; 4]) -> Vec<u8> {
    let payload = json!({
        "schema_version": ugoe_model::SCHEMA_VERSION,
        "future_layout": { "nodes": [{ "id": "Root", "children": [] }] },
    });
    container(&[
        (
            b"FEAT",
            0,
            feat(&[
                FeatureDecl::optional(FUTURE_OPTIONAL),
                FeatureDecl::required(FUTURE_REQUIRED),
            ]),
        ),
        (tag, 0, msgpack(&payload)),
        (b"TEXR", 0, WHITE_PNG.to_vec()),
    ])
}

/// スキーマの版を上げたファイル。本体 (`tag`) は今の版では読めない形にしてあり
/// (`schema_version` は先頭でなく、ほかのフィールドの形も違う)、デコードの前に版で拒否されることを確かめる。
fn future_schema(tag: &[u8; 4]) -> Vec<u8> {
    let payload = json!({
        "meta": { "name": ["a", "newer", "layout"] },
        "schema_version": ugoe_model::SCHEMA_VERSION + 1,
        "parameters": { "AngleX": { "range": [-30.0, 30.0] } },
        "nodes": [{ "id": "Root", "children": [] }],
    });
    container(&[
        (tag, 0, msgpack(&payload)),
        (b"TEXR", 0, WHITE_PNG.to_vec()),
    ])
}

fn future_corpus() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("optional-feature.ugoe", future_optional_bundle()),
        ("required-feature.ugoe", future_required(b"MODL")),
        ("newer-schema.ugoe", future_schema(b"MODL")),
        ("optional-feature.ugoemod", future_optional_module()),
        ("required-feature.ugoemod", future_required(b"MODU")),
        ("newer-schema.ugoemod", future_schema(b"MODU")),
    ]
}

// --- 資料が最新か -------------------------------------------------------------------------------

/// ディレクトリの中の資料 (`.ugoe` と `.ugoemod`) の名前。
fn corpus_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".ugoe") || name.ends_with(".ugoemod"))
        .collect();
    names.sort();
    names
}

/// `dir` の資料を `expected` と比べる。作り直すときは書き込み、余分な資料を消す。
fn sync_dir(dir: &str, expected: Vec<(&'static str, Vec<u8>)>, regenerate: bool) -> Vec<String> {
    let dir = compat_dir().join(dir);
    let expected: BTreeMap<&str, Vec<u8>> = expected.into_iter().collect();
    if regenerate {
        std::fs::create_dir_all(&dir).expect("create dir");
        for name in corpus_names(&dir) {
            if !expected.contains_key(name.as_str()) {
                std::fs::remove_file(dir.join(name)).expect("remove stale file");
            }
        }
        for (name, bytes) in &expected {
            std::fs::write(dir.join(name), bytes).expect("write corpus file");
        }
        return Vec::new();
    }
    let mut problems = Vec::new();
    let found = corpus_names(&dir);
    for name in &found {
        if !expected.contains_key(name.as_str()) {
            problems.push(format!("{}: 余分な資料", dir.join(name).display()));
        }
    }
    for (name, bytes) in &expected {
        let path = dir.join(name);
        match std::fs::read(&path) {
            Ok(actual) if &actual == bytes => {}
            Ok(_) => problems.push(format!("{}: 中身が今のコードと違う", path.display())),
            Err(_) => problems.push(format!("{}: ない", path.display())),
        }
    }
    problems
}

#[test]
fn corpus_is_up_to_date() {
    let regenerate = std::env::var_os(REGENERATE).is_some();
    let mut problems = sync_dir(DEV, dev_corpus(), regenerate);
    problems.extend(sync_dir(FUTURE, future_corpus(), regenerate));
    assert!(
        problems.is_empty(),
        "assets/compat/ が古い。形式やフィクスチャを変えたなら \
         {REGENERATE}=1 cargo test -p ugoe-format --test compat で作り直す \
         (docs/testing.md#互換性テスト):\n{}",
        problems.join("\n")
    );
}

/// 今の書き出しで作ったファイルは、読んで書き直すと同じバイト列になる (往復で何も失わない)。
#[test]
fn dev_corpus_roundtrips_byte_for_byte() {
    for (name, bytes) in dev_corpus() {
        let rewritten: Vec<Vec<u8>> = [false, true]
            .into_iter()
            .map(|compress| {
                let options = WriteOptions { compress };
                if name.ends_with(".ugoemod") {
                    let file = read_module_with_report(&bytes).expect(name).0;
                    write_module_with(&file, options).expect(name)
                } else {
                    let bundle = read_bundle_with_report(&bytes).expect(name).0;
                    write_bundle_with(&bundle, options).expect(name)
                }
            })
            .collect();
        assert!(rewritten.contains(&bytes), "{name}");
    }
}

// --- どの版のファイルも読める -------------------------------------------------------------------

fn evaluate(model: Model) -> Vec<DrawableState> {
    let compiled = CompiledModel::compile(model).expect("compile");
    let mut instance = ModelInstance::new(compiled);
    instance.update();
    instance.drawables().to_vec()
}

/// 版のディレクトリ (`future/` 以外) の名前。
fn release_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(compat_dir())
        .expect("assets/compat")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.is_dir() && !path.ends_with(FUTURE))
        .collect();
    dirs.sort();
    dirs
}

/// 各版のファイル (今の版を含む) を、今のランタイムが警告なしで読み、評価できる。
/// モジュールは同じディレクトリの本体 ([`MODULE_BASE`]) に取り付けて評価する。
#[test]
fn every_release_loads_without_warnings() {
    let dirs = release_dirs();
    assert!(dirs.iter().any(|dir| dir.ends_with(DEV)), "{dirs:?}");
    for dir in dirs {
        let names = corpus_names(&dir);
        assert!(!names.is_empty(), "{}: 資料がない", dir.display());
        let base_path = dir.join(MODULE_BASE);
        let (base, _) = read_bundle_with_report(&read_file(&base_path))
            .unwrap_or_else(|e| panic!("{}: {e}", base_path.display()));
        for name in names {
            let path = dir.join(&name);
            let bytes = read_file(&path);
            let context = path.display();
            if name.ends_with(".ugoemod") {
                let (file, report) =
                    read_module_with_report(&bytes).unwrap_or_else(|e| panic!("{context}: {e}"));
                assert!(report.warnings.is_empty(), "{context}: {report:?}");
                let model = compose(&base.model, &[&file.module])
                    .unwrap_or_else(|e| panic!("{context}: {e}"));
                evaluate(model);
            } else {
                let (bundle, report) =
                    read_bundle_with_report(&bytes).unwrap_or_else(|e| panic!("{context}: {e}"));
                assert!(report.warnings.is_empty(), "{context}: {report:?}");
                evaluate(bundle.model);
            }
        }
    }
}

// --- 新しい版のファイルを劣化させて読む ---------------------------------------------------------

fn future_file(name: &str) -> Vec<u8> {
    read_file(&compat_dir().join(FUTURE).join(name))
}

fn dev_file(name: &str) -> Vec<u8> {
    read_file(&compat_dir().join(DEV).join(name))
}

fn warning_strings(warnings: &[ReadWarning]) -> Vec<String> {
    warnings.iter().map(ToString::to_string).collect()
}

/// 知らない任意の機能を宣言したバンドルは、フォールバックを適用して読める。
/// フォールバックの結果は、新しい機能を使わない元のファイル (`dev/dress-up-base.ugoe`) と同じ表示になる。
#[test]
fn unknown_optional_features_in_bundles_fall_back() {
    let (bundle, report) = read_bundle_with_report(&future_file("optional-feature.ugoe"))
        .expect("future bundle loads");
    assert_eq!(
        report.features,
        [
            FeatureDecl::optional(FUTURE_OPTIONAL),
            FeatureDecl::optional("mask.intersection"),
        ]
    );
    assert_eq!(
        warning_strings(&report.warnings),
        [
            format!("unknown optional feature {FUTURE_OPTIONAL}"),
            "unknown value at MODL: parameters[AngleX].kind".into(),
            "unknown value at MODL: deformers[Bend].kind".into(),
            "unknown value at MODL: meshes[Face].blend".into(),
            "unknown value at MODL: meshes[Face].mask_mode".into(),
            "unknown value at MOTN#0: curves[1].segments[1]".into(),
            "unknown value at MOTN#0: curves[2].target".into(),
            "unknown value at EXPR#0: parameters[1].op".into(),
        ]
    );

    let (dev, _) = read_bundle_with_report(&dev_file(MODULE_BASE)).expect("dev bundle");
    let model = &bundle.model;
    let face = model
        .meshes
        .iter()
        .position(|m| m.id.as_str() == "Face")
        .expect("Face");
    assert_eq!(model.meshes[face].blend, BlendMode::Unknown);
    assert_eq!(model.meshes[face].mask_mode, MaskMode::Unknown);
    let bend = model
        .deformers
        .iter()
        .find(|d| d.id.as_str() == "Bend")
        .expect("Bend");
    assert_eq!(bend.kind, DeformerKind::Unknown);

    // フォールバック: 合成モードは通常、マスクは和集合、未知のデフォーマは恒等変換。
    let compiled = CompiledModel::compile(model.clone()).expect("compile");
    assert_eq!(compiled.mask_mode(face), MaskMode::Union);
    let future_drawables = evaluate(model.clone());
    assert_eq!(future_drawables[face].blend, BlendMode::Normal);
    assert_drawables_eq(&future_drawables, &evaluate(dev.model.clone()));

    // モーションと表情: 知らない値を含む要素だけが Unknown になり、ほかは元のまま読める。
    assert_eq!(bundle.motions[0].curves[0], dev.motions[0].curves[0]);
    assert_eq!(bundle.motions[0].curves[2].target, CurveTarget::Unknown);
    assert_eq!(
        bundle.expressions[0].parameters[0],
        dev.expressions[0].parameters[0]
    );
    assert_eq!(
        bundle.expressions[0].parameters[1].op,
        ExpressionOp::Unknown
    );
    assert_eq!(bundle.textures, dev.textures);
    assert_eq!(bundle.normal_maps, dev.normal_maps);
    assert_eq!(bundle.meta, dev.meta);

    // 知らない値は書き出せない (docs/spec/file-formats.md#再保存)。
    assert!(matches!(
        write_bundle(&bundle),
        Err(FormatError::UnknownValues(_))
    ));
}

/// 知らない任意の機能を宣言したモジュールは、フォールバックを適用して読め、本体に取り付けられる。
#[test]
fn unknown_optional_features_in_modules_fall_back() {
    let (file, report) = read_module_with_report(&future_file("optional-feature.ugoemod"))
        .expect("future module loads");
    assert_eq!(report.features, [FeatureDecl::optional(FUTURE_OPTIONAL)]);
    assert_eq!(
        warning_strings(&report.warnings),
        [
            format!("unknown optional feature {FUTURE_OPTIONAL}"),
            "unknown value at MODU: deformers[Bend].kind".into(),
            "unknown value at MODU: meshes[Back].blend".into(),
            "unknown value at MODU: physics.rigs[Sway].inputs[0].source".into(),
        ]
    );

    let (dev, _) = read_module_with_report(&dev_file("long-hair.ugoemod")).expect("dev module");
    let (base, _) = read_bundle_with_report(&dev_file(MODULE_BASE)).expect("dev base");
    let future_model = compose(&base.model, &[&file.module]).expect("compose future module");
    let dev_model = compose(&base.model, &[&dev.module]).expect("compose dev module");
    assert_drawables_eq(&evaluate(future_model), &evaluate(dev_model));
    assert_eq!(file.textures, dev.textures);
}

/// 知らない必須の機能を宣言したファイルは、足りない機能の ID を示して拒否する。
#[test]
fn unknown_required_features_are_rejected() {
    let missing = |result: Result<(), FormatError>| match result {
        Err(FormatError::UnsupportedFeatures(ids)) => ids,
        other => panic!("unexpected result: {other:?}"),
    };
    let bundle = read_bundle_with_report(&future_file("required-feature.ugoe")).map(|_| ());
    assert_eq!(missing(bundle), [FUTURE_REQUIRED]);
    let module = read_module_with_report(&future_file("required-feature.ugoemod")).map(|_| ());
    assert_eq!(missing(module), [FUTURE_REQUIRED]);
}

/// スキーマの版が新しいファイルは、形が違っていてもデコードのエラーでなく版のエラーで拒否する。
#[test]
fn newer_schema_versions_are_rejected() {
    let version = |result: Result<(), FormatError>| match result {
        Err(FormatError::UnsupportedVersion { found, supported }) => (found, supported),
        other => panic!("unexpected result: {other:?}"),
    };
    let expected = (ugoe_model::SCHEMA_VERSION + 1, ugoe_model::SCHEMA_VERSION);
    let bundle = read_bundle_with_report(&future_file("newer-schema.ugoe")).map(|_| ());
    assert_eq!(version(bundle), expected);
    let module = read_module_with_report(&future_file("newer-schema.ugoemod")).map(|_| ());
    assert_eq!(version(module), expected);
}

/// 描画可能要素の評価結果を、頂点に許容誤差を付けて比べる。
fn assert_drawables_eq(actual: &[DrawableState], expected: &[DrawableState]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(a.positions.len(), e.positions.len(), "drawable #{index}");
        for (pa, pe) in a.positions.iter().zip(&e.positions) {
            assert!(
                (pa.x - pe.x).abs() < 1e-4 && (pa.y - pe.y).abs() < 1e-4,
                "drawable #{index}: {pa:?} != {pe:?}"
            );
        }
        let rest = |d: &DrawableState| {
            (
                d.opacity,
                d.draw_order,
                d.multiply_color,
                d.screen_color,
                d.blend,
                d.mask_strength,
                d.visible,
            )
        };
        assert_eq!(rest(a), rest(e), "drawable #{index}");
    }
}
