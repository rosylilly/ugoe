//! SDK の誤りと警告の種類 (`code`) が、すべて C ABI の種類 (`ugoe_error_kind`・`UgoeWarning::code`) と戻り値になり、
//! README の表 (か説明) に載っているかの検査 (T0198)。
//!
//! 見本は SDK の `samples()` (`ugoe::LoadError`・`DressUpError`・`LoadWarning`。SDK の中の網羅の `match` で種類の足し忘れに気づく)。
//! SDK に種類が増えると、戻り値の決まっていない種類か README に無い種類として、ここのテストが落ちる。

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ugoe::{DressUpError, LoadError, LoadWarning};

use crate::dress_up::dress_up_error;
use crate::error::{
    TestError, UGOE_ERROR_DRESS_UP, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_MODEL,
    UGOE_ERROR_LIMIT_EXCEEDED, UgoeResult, guard,
};
use crate::limits::load_error;
use crate::warning::Warning;

/// README の `## {heading}` の節 (次の `## ` の見出しまで)。
fn readme_section(heading: &str) -> &'static str {
    include_str!("../README.md")
        .split("\n## ")
        .find(|section| section.starts_with(&format!("{heading}\n")))
        .unwrap_or_else(|| panic!("README.md has no section `{heading}`"))
}

/// `code` が README の節に `` `code` `` として出てくるか。
fn assert_documented(section: &str, code: &str) {
    assert!(
        readme_section(section).contains(&format!("`{code}`")),
        "`{code}` is not documented in the `{section}` section of crates/ugoe-ffi/README.md"
    );
}

/// 失敗の戻り値・種類・補足の値 (値は 10 進の文字列)。
fn failure(error: crate::error::Error) -> (UgoeResult, Option<String>, Vec<(String, String)>) {
    let out = TestError::new();
    let code = guard(out.0, || Err(error));
    let fields = (0..out.field_count())
        .map(|i| (out.key_at(i).unwrap(), out.value_at(i).unwrap()))
        .collect();
    (code, out.kind(), fields)
}

fn sdk_fields(fields: Vec<(&'static str, impl ToString)>) -> Vec<(String, String)> {
    fields
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_string()))
        .collect()
}

#[test]
fn every_load_error_kind_has_a_result_code() {
    for sample in LoadError::samples() {
        let kind = sample.code();
        let fields = sdk_fields(sample.fields());
        let compile = matches!(sample, LoadError::Compile(_));
        // 戻り値は README の表と同じ。知らない種類は、戻り値を決めて表に足すまで落とす。
        let expected = match kind {
            "invalid_file" | "unsupported_version" | "unsupported_features" => UGOE_ERROR_FORMAT,
            "invalid_model" if compile => UGOE_ERROR_INVALID_MODEL,
            "invalid_model" => UGOE_ERROR_FORMAT,
            "compile" => UGOE_ERROR_INVALID_MODEL,
            "read_limit_exceeded" => UGOE_ERROR_LIMIT_EXCEEDED,
            other => panic!("decide the result code of the SDK load error kind `{other}`"),
        };
        assert_eq!(
            failure(load_error(sample)),
            (expected, Some(kind.to_owned()), fields),
            "{kind}"
        );
        assert_documented("モデルの読み込みの失敗 (T0457)", kind);
    }
}

#[test]
fn every_dress_up_error_kind_has_a_result_code() {
    for sample in DressUpError::samples() {
        let kind = sample.code();
        let fields = sdk_fields(sample.fields());
        assert_eq!(
            failure(dress_up_error(sample)),
            (UGOE_ERROR_DRESS_UP, Some(kind.to_owned()), fields),
            "{kind}"
        );
        assert_documented("着せ替え (T0409)", kind);
    }
}

#[test]
fn every_load_warning_kind_reaches_the_c_abi() {
    for sample in LoadWarning::samples() {
        let warning = Warning::new(&sample, &sample.to_string());
        assert_eq!(warning.code(), sample.code());
        let expected = sdk_fields(sample.fields());
        let actual: Vec<_> = warning
            .fields()
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(actual, expected, "{}", sample.code());
        // C ABI で出さない警告 (KTX2・物理の引き継ぎ) も、出さないことを README に書く。
        assert_documented("読み込みの警告 (T0409)", sample.code());
    }
}
