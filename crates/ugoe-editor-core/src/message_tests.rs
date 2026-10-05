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

/// モーション・表情の検証エラーは `format.corrupt` に落とさず、名前と検証エラーの一覧を引数にする (T1606)。
/// `details` は `describe_errors` と同じく先頭の `DESCRIBED_ERRORS_MAX` 件までで、残りは件数だけ。
#[test]
fn motion_and_expression_validation_errors_have_their_own_codes() {
    use ugoe_format::FormatError;
    use ugoe_model::validate::{DESCRIBED_ERRORS_MAX, ValidationError};
    let error = |i: usize| ValidationError::DuplicateId {
        kind: "part",
        id: format!("p{i}"),
    };

    let motion = FormatError::InvalidMotion {
        name: "idle".into(),
        errors: vec![error(0)],
    }
    .to_message();
    assert_eq!(motion.code, "format.invalid_motion");
    assert_eq!(
        motion.args,
        message_args! { "count" => 1usize, "details" => error(0).to_string(), "name" => "idle" }
    );

    let errors: Vec<ValidationError> = (0..DESCRIBED_ERRORS_MAX + 2).map(error).collect();
    let expression = FormatError::InvalidExpression {
        name: "smile".into(),
        errors: errors.clone(),
    }
    .to_message();
    assert_eq!(expression.code, "format.invalid_expression");
    let mut details = errors[..DESCRIBED_ERRORS_MAX]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    details.push_str("; and 2 more");
    assert_eq!(
        expression.args,
        message_args! { "count" => errors.len(), "details" => details, "name" => "smile" }
    );

    // プロジェクトの読み込みで拒んだときは、ファイルの場所の下に原因として付く。
    let project = ProjectError::Format {
        path: "motions/idle.motion.json".into(),
        source: FormatError::InvalidMotion {
            name: "idle".into(),
            errors: vec![error(0)],
        },
    }
    .to_message();
    assert_eq!(project.code, "project.format");
    assert_eq!(
        project.cause.map(|cause| cause.code),
        Some("format.invalid_motion".to_owned())
    );
}

/// ソースに書いたコード (`"area.name"` の文字列) を取り出す。
fn codes_in(source: &str) -> Vec<String> {
    const AREAS: [&str; 20] = [
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
        "validation.",
        "infer.",
        "world.",
        "normal_map.",
        "compose.",
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
    let codes: Vec<String> = sources
        .iter()
        .flat_map(|s| codes_in(s))
        .chain(sourced_codes())
        .collect();
    assert!(codes.len() > 100, "{}", codes.len());
    for code in codes {
        assert_code_shape(&code);
        assert!(
            docs.contains(&format!("`{code}`")),
            "{code} is not in docs/editor.md"
        );
    }
}

/// ソースに書いたすべてのコード (と元のクレートのコードから組み立てるコード) に見本があり (`message_catalog`)、
/// 見本にそれら以外のコードが無い。
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
    // 元のクレートのエラーのコードは、ソースに並べず組み立てる。
    in_source.extend(sourced_codes());
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

/// 元のクレートの `code()` (`Self::… => "name"` の `match`) に書いたコード。
fn crate_codes(source: &str) -> Vec<&str> {
    let start = source
        .find("pub fn code(&self) -> &'static str {")
        .expect("no code() in the source");
    let body = &source[start..];
    let body = &body[..body.find("\n    }\n").expect("no end of code()")];
    body.split("=> \"")
        .skip(1)
        .map(|s| &s[..s.find('"').unwrap()])
        .collect()
}

/// 元のクレートのすべてのコードに領域の名前を付けたもの (`validation.duplicate_id` など)。editor-core はこれらを
/// 一覧に持たず組み立てる (`message_codes.rs` の `sourced`) ので、ソースのコード (`codes_in`) の代わりにこれを使う。
fn sourced_codes() -> BTreeSet<String> {
    let sources = [
        (
            "validation",
            include_str!("../../ugoe-model/src/validate/code.rs"),
        ),
        ("infer", include_str!("../../ugoe-rigging/src/error.rs")),
        ("world", include_str!("../../ugoe-rigging/src/world.rs")),
        (
            "normal_map",
            include_str!("../../ugoe-rigging/src/normal.rs"),
        ),
        ("compose", include_str!("../../ugoe-runtime/src/compose.rs")),
        ("psd_read", include_str!("../../ugoe-psd/src/lib.rs")),
        ("automesh", include_str!("../../ugoe-mesher/src/lib.rs")),
    ];
    sources
        .iter()
        .flat_map(|(area, source)| {
            crate_codes(source)
                .into_iter()
                .map(move |code| format!("{area}.{code}"))
        })
        .collect()
}

/// 検証・推定・写像・法線マップ・合成・PSD の読み込み・自動メッシュのエラーのコードは、元のエラーのコード (`ValidationError::code` など) に
/// 領域の名前を付けたもの (元のクレートでコードを固定し、ここはそれから組み立てる。T1618)。見本は元のクレートの
/// すべてのコードを含む (組み立てたコードが `docs/editor.md` の一覧にあることは `every_code_is_documented` が確かめる)。
#[test]
fn source_error_codes_follow_the_crate_codes() {
    use crate::message_catalog::{
        compose_errors, infer_errors, mesh_errors, normal_map_errors, psd_errors,
        validation_errors, world_errors,
    };
    let mut built = BTreeSet::new();
    let mut check = |code: std::borrow::Cow<'static, str>, expected: String| {
        assert_eq!(code, expected);
        built.insert(expected);
    };
    for error in validation_errors() {
        check(Coded::code(&error), format!("validation.{}", error.code()));
    }
    for error in infer_errors() {
        check(Coded::code(&error), format!("infer.{}", error.code()));
    }
    for error in world_errors() {
        check(Coded::code(&error), format!("world.{}", error.code()));
    }
    for error in normal_map_errors() {
        check(Coded::code(&error), format!("normal_map.{}", error.code()));
    }
    for error in compose_errors() {
        check(Coded::code(&error), format!("compose.{}", error.code()));
    }
    for error in psd_errors() {
        check(Coded::code(&error), format!("psd_read.{}", error.code()));
    }
    for error in mesh_errors() {
        check(Coded::code(&error), format!("automesh.{}", error.code()));
    }
    let all = sourced_codes();
    // 27 (検証) + 27 (推定) + 4 (写像) + 4 (法線マップ) + 13 (合成) + 20 (PSD の読み込み) + 5 (自動メッシュ)。
    // 読み取りの誤りで一覧が欠けないように。
    assert!(all.len() >= 100, "{}", all.len());
    assert_eq!(built, all, "every crate code has a sample");
}

/// 検証のエラーの一覧は、`count`・`details` に加えて、先頭の 10 件を訳せる形 (`items`) で渡す。
#[test]
fn validation_lists_carry_coded_items() {
    let errors: Vec<_> = (0..12)
        .map(|i| ugoe_model::validate::ValidationError::DuplicateId {
            kind: "part",
            id: format!("P{i}"),
        })
        .collect();
    let message = EditError::Invalid(errors).to_message();
    assert_eq!(message.code, "edit.invalid_model");
    assert_eq!(message.args["count"], MessageArg::Number(12.0));
    assert_eq!(message.items.len(), 10);
    assert_eq!(message.items[0].code, "validation.duplicate_id");
    assert_eq!(
        message.items[0].args,
        message_args! { "kind" => "part", "id" => "P0" }
    );
    assert_eq!(message.items[0].message, "duplicate part id `P0`");
    let json = serde_json::to_value(&message).unwrap();
    assert_eq!(json["items"][9]["args"]["id"], "P9");
    // 一覧の無いメッセージは `items` を書かない。
    let plain = serde_json::to_value(EditError::NotModule.to_message()).unwrap();
    assert!(plain.get("items").is_none(), "{plain}");
}

/// 推定・合成・法線マップの失敗は、`detail` の英語の文に加えて、元のエラーを原因に載せる。
#[test]
fn source_errors_are_carried_as_causes() {
    let inference = EditError::Inference {
        target: "mesh `EyeL`".into(),
        error: ugoe_rigging::InferError::MissingNeutralKey {
            parameter: "AngleX".into(),
            default: 0.0,
        },
    }
    .to_message();
    assert_eq!(inference.code, "edit.inference_failed");
    assert!(inference.args.contains_key("detail"));
    let cause = inference.cause.as_deref().unwrap();
    assert_eq!(cause.code, "infer.missing_neutral_key");
    assert_eq!(
        cause.args,
        message_args! { "parameter" => "AngleX", "default" => 0.0f32 }
    );

    let compose = ModuleError::Compose(ugoe_runtime::ComposeError::SlotOccupied {
        slot: "head".into(),
        first: "cap".into(),
        second: "hat".into(),
    })
    .to_message();
    assert_eq!(compose.code, "module.compose_failed");
    let cause = compose.cause.as_deref().unwrap();
    assert_eq!(cause.code, "compose.slot_occupied");
    assert_eq!(
        cause.args,
        message_args! { "slot" => "head", "module" => "hat", "attached" => "cap" }
    );

    // 推定の中の写像の失敗は、さらにその原因に載る。
    let parent =
        ugoe_rigging::InferError::ParentTransform(ugoe_rigging::WorldError::Singular { index: 3 })
            .to_message();
    assert_eq!(parent.cause.as_deref().unwrap().code, "world.singular");

    let lighting =
        crate::lighting::LightingError::Generate(ugoe_rigging::NormalMapError::EmptyImage {
            width: 0,
            height: 2,
        })
        .to_message();
    assert_eq!(lighting.code, "lighting.generate_failed");
    assert_eq!(
        lighting.cause.as_deref().unwrap().code,
        "normal_map.empty_image"
    );
}
