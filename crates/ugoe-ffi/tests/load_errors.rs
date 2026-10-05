//! モデルの読み込み (`ugoe_puppet_create_from_*`) の失敗の種類 (T0457): `ugoe_last_error_kind` と補足の値が、
//! SDK の `ugoe::LoadError` (wasm の `LoadError` の `code`) と同じになる。戻り値は T0457 の前と同じ。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作ったバイト列と生きている Rust の値を指すポインタを渡す。
// 失敗した呼び出しは何も作らない。借りた文字列は次の失敗の前に読む。

use std::ffi::{CStr, CString, c_char};
use std::path::Path;
use std::ptr;

use ugoe::format::{Bundle, TextureData, TextureFormat, write_bundle};
use ugoe::model::fixtures;
use ugoe_ffi::*;

fn text(ptr: *const c_char) -> Option<String> {
    (!ptr.is_null()).then(|| {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    })
}

fn field(key: &str) -> Option<String> {
    let key = CString::new(key).unwrap();
    text(unsafe { ugoe_last_error_field(key.as_ptr()) })
}

/// 失敗の戻り値・種類・補足の値。
#[derive(Debug, PartialEq)]
struct Failure {
    code: UgoeResult,
    kind: Option<String>,
    fields: Vec<(&'static str, String)>,
}

/// 補足の値のキー (`ugoe::LoadError::fields` の全部)。
const KEYS: [&str; 7] = [
    "found",
    "supported",
    "features",
    "errors",
    "limit",
    "chunk",
    "max",
];

fn last_failure(code: UgoeResult) -> Failure {
    assert_ne!(code, UGOE_OK);
    let fields: Vec<_> = KEYS
        .iter()
        .filter_map(|&k| field(k).map(|v| (k, v)))
        .collect();
    // 番号での列挙 (T0464) も、キーで読んだものと同じ組を返す。
    let mut listed: Vec<_> = (0..ugoe_last_error_field_count())
        .map(|i| {
            let key = text(ugoe_last_error_field_key_at(i)).unwrap();
            (key, text(ugoe_last_error_field_value_at(i)).unwrap())
        })
        .collect();
    let mut by_key: Vec<_> = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    listed.sort();
    by_key.sort();
    assert_eq!(listed, by_key);
    Failure {
        code,
        kind: text(ugoe_last_error_kind()),
        fields,
    }
}

fn from_bundle(bytes: &[u8]) -> Failure {
    let mut p = ptr::null_mut();
    let code = unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut p) };
    assert!(p.is_null());
    last_failure(code)
}

fn from_json(json: &str) -> Failure {
    let mut p = ptr::null_mut();
    let code = unsafe { ugoe_puppet_create_from_json(json.as_ptr(), json.len(), &mut p) };
    assert!(p.is_null());
    last_failure(code)
}

fn failure(code: UgoeResult, kind: &str, fields: &[(&'static str, &str)]) -> Failure {
    Failure {
        code,
        kind: Some(kind.to_owned()),
        fields: fields.iter().map(|&(k, v)| (k, v.to_owned())).collect(),
    }
}

fn compat(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat/future");
    std::fs::read(path.join(name)).unwrap()
}

/// 検査を通らないモデル (三角形の数に合わない添字)。
fn invalid_model() -> ugoe::model::Model {
    let mut model = fixtures::single_quad();
    model.meshes[0].indices.push(0);
    model
}

#[test]
fn bundle_failures_have_kinds() {
    assert_eq!(
        from_bundle(b"not a bundle"),
        failure(UGOE_ERROR_FORMAT, "invalid_file", &[])
    );

    let supported = ugoe::model::SCHEMA_VERSION.to_string();
    let newer = from_bundle(&compat("newer-schema.ugoe"));
    assert_eq!(newer.code, UGOE_ERROR_FORMAT);
    assert_eq!(newer.kind.as_deref(), Some("unsupported_version"));
    assert_eq!(newer.fields[1], ("supported", supported));
    let found: u32 = newer.fields[0].1.parse().unwrap();
    assert!(found > ugoe::model::SCHEMA_VERSION, "{newer:?}");

    let required = from_bundle(&compat("required-feature.ugoe"));
    assert_eq!(required.code, UGOE_ERROR_FORMAT);
    assert_eq!(required.kind.as_deref(), Some("unsupported_features"));
    assert_eq!(required.fields[0].0, "features");
    assert!(!required.fields[0].1.is_empty());

    // 読むときの検査を通らないモデルは、戻り値は `UGOE_ERROR_FORMAT` のまま、種類は `invalid_model`。
    let mut bundle = Bundle::new(invalid_model());
    bundle.textures = vec![
        TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        };
        bundle.model.textures.len()
    ];
    let bytes = write_bundle(&bundle).unwrap();
    assert_eq!(
        from_bundle(&bytes),
        failure(UGOE_ERROR_FORMAT, "invalid_model", &[("errors", "1")])
    );
}

#[test]
fn json_failures_have_kinds() {
    assert_eq!(
        from_json("{}"),
        failure(UGOE_ERROR_FORMAT, "invalid_file", &[])
    );

    let newer = format!(
        r#"{{"schema_version": {}}}"#,
        ugoe::model::SCHEMA_VERSION + 1
    );
    assert_eq!(
        from_json(&newer),
        failure(
            UGOE_ERROR_FORMAT,
            "unsupported_version",
            &[
                ("found", &(ugoe::model::SCHEMA_VERSION + 1).to_string()),
                ("supported", &ugoe::model::SCHEMA_VERSION.to_string()),
            ],
        )
    );

    // JSON は読むときに検査しないので、コンパイルの前の検査で `UGOE_ERROR_INVALID_MODEL`・`invalid_model`。
    let json = ugoe::format::model_to_json(&invalid_model()).unwrap();
    assert_eq!(
        from_json(&json),
        failure(
            UGOE_ERROR_INVALID_MODEL,
            "invalid_model",
            &[("errors", "1")]
        )
    );

    // UTF-8 でない JSON は引数の誤りで、種類を持たない (wasm では起きない)。
    let bytes = [0xff_u8, 0xfe];
    let mut p = ptr::null_mut();
    let code = unsafe { ugoe_puppet_create_from_json(bytes.as_ptr(), bytes.len(), &mut p) };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
    assert!(ugoe_last_error_kind().is_null());
}

/// 種類を持たない失敗 (NULL の引数など) は、前の読み込みの失敗の種類を消す。
#[test]
fn kinds_follow_the_last_failure() {
    from_bundle(b"not a bundle");
    let mut p = ptr::null_mut();
    let code = unsafe { ugoe_puppet_create_from_bundle(ptr::null(), 4, &mut p) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    assert!(ugoe_last_error_kind().is_null());
}
