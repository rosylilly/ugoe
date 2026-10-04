use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::json;

use crate::message::{Coded, Message, MessageArg};
use crate::message_catalog::{message_codes, message_samples};
use crate::project::ProjectError;
use crate::proposal::ProposalError;
use crate::{CellError, EditError, ModuleError, message_args};

/// コードは小文字の `snake_case` を `.` でつないだもの (ADR 0029)。
fn assert_code_shape(code: &str) {
    let parts: Vec<&str> = code.split('.').collect();
    assert!(parts.len() >= 2, "{code}");
    for part in parts {
        assert!(!part.is_empty(), "{code}");
        assert!(
            part.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{code}"
        );
    }
}

#[test]
fn message_serializes_code_args_and_english_text() {
    let error = ProjectError::BaseNotFound {
        path: PathBuf::from("/work/body.ugoeproj"),
    };
    let message = error.to_message();
    assert_eq!(
        serde_json::to_value(&message).unwrap(),
        json!({
            "code": "project.base_not_found",
            "args": { "path": "/work/body.ugoeproj" },
            "message": "base model not found: /work/body.ugoeproj",
        })
    );
    // 英語の文はエラーの表示と同じ (ログ・MCP・CLI で使う)。
    assert_eq!(message.to_string(), error.to_string());
    let back: Message = serde_json::from_value(serde_json::to_value(&message).unwrap()).unwrap();
    assert_eq!(back, message);
}

#[test]
fn numbers_are_numbers_and_non_finite_values_are_text() {
    let message = EditError::InvalidRange {
        min: 1.0,
        default: f32::NAN,
        max: 0.5,
    }
    .to_message();
    assert_eq!(message.code, "edit.invalid_range");
    assert_eq!(message.args["min"], MessageArg::Number(1.0));
    assert_eq!(message.args["max"], MessageArg::Number(0.5));
    assert_eq!(message.args["default"], MessageArg::Text("NaN".into()));
    assert_eq!(
        serde_json::to_value(&message.args).unwrap(),
        json!({ "min": 1.0, "default": "NaN", "max": 0.5 })
    );
}

#[test]
fn wrapped_errors_carry_their_cause() {
    let edit = EditError::InvalidCell {
        kind: "mesh",
        id: "Face".into(),
        error: CellError::UnknownAxis("AngleX".into()),
    };
    let text = edit.to_string();
    let message = ProposalError::Rejected(edit).to_message();
    assert_eq!(message.code, "proposal.rejected");
    assert!(message.message.contains(&text));
    let cause = message.cause.as_deref().unwrap();
    assert_eq!(cause.code, "edit.invalid_cell");
    assert_eq!(
        cause.args,
        message_args! { "kind" => "mesh", "id" => "Face" }
    );
    let cell = cause.cause.as_deref().unwrap();
    assert_eq!(cell.code, "cell.unknown_axis");
    assert_eq!(cell.args, message_args! { "parameter" => "AngleX" });
    // 原因が無ければ `cause` は書かない。
    let plain = serde_json::to_value(cell).unwrap();
    assert!(plain.get("cause").is_none(), "{plain}");
}

#[test]
fn transparent_errors_use_the_inner_code() {
    let message = ProjectError::Module(ModuleError::UnknownSlot("hair".into())).to_message();
    assert_eq!(message.code, "module.unknown_slot");
    assert_eq!(message.args, message_args! { "slot" => "hair" });
    assert_eq!(message.message, "the base model has no slot `hair`");
}

#[test]
fn codes_have_the_stable_shape() {
    let samples: Vec<Message> = vec![
        EditError::NotModule.to_message(),
        EditError::DuplicateId {
            kind: "part",
            id: "P".into(),
        }
        .to_message(),
        CellError::OutOfRange { index: 3, len: 2 }.to_message(),
        ProposalError::Stale.to_message(),
        ProjectError::NotAModule.to_message(),
        ProjectError::MissingModuleLink.to_message(),
        ModuleError::ColliderNeedsBasePhysics {
            rig: "Hair".into(),
            collider: "Head".into(),
            slot: "hair".into(),
        }
        .to_message(),
        ugoe_format::FormatError::BadMagic.to_message(),
        ugoe_format::FormatError::Truncated.to_message(),
    ];
    for message in &samples {
        assert_code_shape(&message.code);
    }
    assert_eq!(samples[8].code, "format.corrupt");
    assert_eq!(
        samples[8].args,
        message_args! { "detail" => "unexpected end of input" }
    );
}

/// 読み込みの上限を超えたエラーは、壊れたファイル (`format.corrupt`) と分けて、どの上限かをコードで示す。
#[test]
fn read_limit_errors_have_their_own_codes() {
    use ugoe_format::{ChunkTag, FormatError, ReadLimit};
    let chunk = FormatError::ReadLimitExceeded {
        tag: ChunkTag::MOTN,
        limit: ReadLimit::ChunkDecompressed,
        max: 256,
    }
    .to_message();
    assert_eq!(chunk.code, "format.chunk_limit_exceeded");
    assert_eq!(
        chunk.args,
        message_args! { "chunk" => "MOTN", "max" => 256u64 }
    );
    let total = FormatError::ReadLimitExceeded {
        tag: ChunkTag::TEXR,
        limit: ReadLimit::TotalDecompressed,
        max: 1024,
    }
    .to_message();
    assert_eq!(total.code, "format.total_limit_exceeded");
    assert_eq!(
        total.args,
        message_args! { "chunk" => "TEXR", "max" => 1024u64 }
    );
}

/// ソースに書いたコード (`"area.name"` の文字列) を取り出す。
fn codes_in(source: &str) -> Vec<String> {
    const AREAS: [&str; 15] = [
        "edit.",
        "cell.",
        "proposal.",
        "project.",
        "module.",
        "export.",
        "atlas.",
        "overflow.",
        "lighting.",
        "space.",
        "psd.",
        "format.",
        "avatar.",
        "compile.",
        "inference.",
    ];
    source
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|s| {
            AREAS.iter().any(|a| s.starts_with(a))
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.')
                && !s.ends_with(".json")
        })
        .map(str::to_owned)
        .collect()
}

/// すべてのコードが `docs/editor.md#エラーと警告のコード` の一覧にある。
#[test]
fn every_code_is_documented() {
    let docs = include_str!("../../../docs/editor.md");
    let sources = [
        include_str!("message_codes.rs"),
        include_str!("import.rs"),
        include_str!("inference.rs"),
        include_str!("project.rs"),
    ];
    let codes: Vec<String> = sources.iter().flat_map(|s| codes_in(s)).collect();
    assert!(codes.len() > 100, "{}", codes.len());
    for code in codes {
        assert_code_shape(&code);
        assert!(
            docs.contains(&format!("`{code}`")),
            "{code} is not in docs/editor.md"
        );
    }
}

/// ソースに書いたすべてのコードに見本があり (`message_catalog`)、見本にソースに無いコードが無い。
#[test]
fn every_code_has_a_sample() {
    let sources = [
        include_str!("message_codes.rs"),
        include_str!("import.rs"),
        include_str!("inference.rs"),
        include_str!("project.rs"),
    ];
    let mut in_source: BTreeSet<String> = sources.iter().flat_map(|s| codes_in(s)).collect();
    // `codes_in` はファイル名 (`.json`) を除くので、ファイル名と同じ綴りのコードを足す。
    in_source.insert("project.json".to_owned());
    let sampled: BTreeSet<String> = message_codes(message_samples()).into_keys().collect();
    let missing: Vec<_> = in_source.difference(&sampled).collect();
    assert!(
        missing.is_empty(),
        "no sample in message_catalog.rs: {missing:?}"
    );
    let extra: Vec<_> = sampled.difference(&in_source).collect();
    assert!(extra.is_empty(), "not in the sources: {extra:?}");
}

/// コードの直後に `message_args! { … }` を書いた箇所 (その場で出す警告) の、コード → 引数の名前。
fn inline_args(source: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for code in codes_in(source).into_iter().collect::<BTreeSet<_>>() {
        for (start, _) in source.match_indices(&format!("\"{code}\"")) {
            let after = source[start + code.len() + 2..].trim_start();
            let Some(after) = after.strip_prefix(',') else {
                continue;
            };
            let after = after.trim_start();
            let after = after.strip_prefix("crate::").unwrap_or(after);
            let Some(body) = after.strip_prefix("message_args! {") else {
                continue;
            };
            let close = body.find('}').unwrap();
            let names: BTreeSet<String> = body[..close]
                .split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect();
            out.insert(code.clone(), names);
        }
    }
    out
}

/// 手で書いた警告の見本の引数が、ソースで警告を出す箇所の引数と同じ。
#[test]
fn inline_warning_samples_match_the_source() {
    let inline: BTreeMap<String, BTreeSet<String>> =
        [include_str!("import.rs"), include_str!("project.rs")]
            .iter()
            .flat_map(|s| inline_args(s))
            .collect();
    assert!(inline.len() >= 10, "{inline:?}");
    assert!(inline.contains_key("module.draw_order_outside_slot"));
    let codes = message_codes(message_samples());
    for (code, names) in inline {
        assert_eq!(codes.get(&code), Some(&names), "{code}");
    }
}

/// 補間の種類の引数は JSON の名前 (`smooth`) で渡す (Rust の表記 `Smooth` ではない)。
#[test]
fn interpolation_is_passed_by_its_json_name() {
    let message = EditError::UnsupportedInterpolation {
        target: "mesh `Face`".into(),
        interpolation: ugoe_model::KeyformInterpolation::Smooth,
    }
    .to_message();
    assert_eq!(
        message.args,
        message_args! { "target" => "mesh `Face`", "interpolation" => "smooth" }
    );
}
