//! JS に渡す `code` の一覧 (`codes.json`) の検査 (T0452)。
//!
//! 読み込みの警告 (`load_warnings_json` の要素)・着せ替えの失敗 (`DressUpError`)・読み込みの失敗 (`LoadError`)・
//! 描画と読み戻しの失敗 (`RenderError`。T0465)・`WebPuppet.create` の失敗 (`CreateError`。T0475)・設定の差し替えの失敗 (`set_effects_json`・`set_physics_json`・
//! `set_base_physics_json` の検査の失敗。T0469。合成の失敗は着せ替えの失敗に入る。T0476) の `code` と、その種類が持つ補足の値のキーを、すべての種類の見本から集めて `codes.json` と比べる。
//! `@ugoe/runtime` (`bindings/web/src/codes.test.ts`) は同じ `codes.json` を TypeScript の和型と比べるので、
//! Rust と TS の片方だけに種類やキーを足すと、どちらかのテストが落ちる。
//!
//! wasm の種類を足したら、下の `guard` などの網羅の `match` でコンパイルできなくなる。見本を足し、
//! `UGOE_UPDATE_CODES=1 cargo test -p ugoe-wasm codes` で `codes.json` を書き直してから TS の型を直す。
//! SDK とランタイムの誤りと警告 (`ugoe::LoadError`・`DressUpError`・`LoadWarning`・`CompileError`・`ComposeError`) は
//! `#[non_exhaustive]` なので、見本は SDK とランタイムの `samples()` (それぞれの中の網羅の `match` で種類の足し忘れに気づく) を使う。
//! SDK に種類が増えると、その見本の `code` が `codes.json` に無いので、このテストが落ちる。

use std::collections::{BTreeMap, BTreeSet};

use ugoe::format::FormatError;
use ugoe::model::validate::ValidationError;
use ugoe::render::RenderError;
use ugoe::{DressUpError, LoadWarning};

use crate::animation::AnimationError;
use crate::create::CreateError;
use crate::dress_up::{Detached, WardrobeError, error_info};
use crate::frame::FrameError;
use crate::load::{ErrorInfo, LoadError, warning_json};

/// 種類 (`code`) → 補足の値のキー (その種類の見本に出たものの和)。
type Codes = BTreeMap<String, BTreeSet<String>>;

/// 見本の種類がすべて網羅されているかをコンパイルで確かめる (wasm の種類を足すと、ここが網羅の誤りになる)。
/// SDK の種類 (`ugoe::LoadError`・`DressUpError`) は SDK の `samples()` が網羅する。`RenderError` は、ほかの種類が `texture` に
/// まとまるので網羅しない (`LoadError::info` で新しい `code` を割り当てたら、見本を足す)。
fn guard(error: &WardrobeError) {
    match error {
        WardrobeError::Load(e) => match e {
            LoadError::Render(_)
            | LoadError::Texture { .. }
            | LoadError::NormalMap { .. }
            | LoadError::TextureTooLarge { .. }
            | LoadError::NormalMapTooLarge { .. }
            | LoadError::Register(_)
            | LoadError::Model(_)
            | LoadError::DeviceLost => {}
        },
        WardrobeError::NoModel
        | WardrobeError::NotLoaded(_)
        | WardrobeError::Texture(_)
        | WardrobeError::DeviceLost
        | WardrobeError::DressUp(_) => {}
    }
}

fn validation() -> Vec<ValidationError> {
    vec![ValidationError::InvalidModuleName {
        name: String::new(),
    }]
}

/// 読み込みの失敗の見本 (`LoadError` のすべての種類と、`code` が分かれる中身。ファイルとモデルの失敗は SDK の見本)。
/// 網羅は [`wardrobe_errors`] の [`guard`] で確かめる。
fn load_errors() -> Vec<LoadError> {
    let (index, width, height, max) = (0, 2, 1, 1);
    let wasm = vec![
        LoadError::Texture {
            index,
            message: String::new(),
        },
        LoadError::NormalMap {
            index,
            message: String::new(),
        },
        LoadError::TextureTooLarge {
            index,
            width,
            height,
            max,
        },
        LoadError::NormalMapTooLarge {
            index,
            width,
            height,
            max,
        },
        RenderError::TextureTooLarge {
            index: 0,
            width,
            height,
            max,
        }
        .into(),
        RenderError::NormalMapTooLarge {
            index: 0,
            width,
            height,
            max,
        }
        .into(),
        RenderError::MissingTexture(0).into(),
        LoadError::Register(String::new()),
        LoadError::DeviceLost,
    ];
    ugoe::LoadError::samples()
        .into_iter()
        .map(LoadError::Model)
        .chain(wasm)
        .collect()
}

/// 着せ替えの失敗の見本 (`WardrobeError` のすべての種類。読み込みの失敗は `load_module` が返しうるもの、
/// 付け外しの失敗は SDK の見本)。
fn wardrobe_errors() -> Vec<WardrobeError> {
    let s = |v: &str| v.to_owned();
    load_errors()
        .into_iter()
        .map(WardrobeError::Load)
        .chain([
            WardrobeError::NoModel,
            WardrobeError::NotLoaded(s("m")),
            WardrobeError::Texture(s("t")),
            WardrobeError::DeviceLost,
        ])
        .chain(
            DressUpError::samples()
                .into_iter()
                .map(WardrobeError::DressUp),
        )
        .collect()
}

/// 描画と読み戻しの失敗の見本 (`FrameError` のすべての種類)。種類を足すと、`match` が網羅の誤りになる。
fn frame_errors() -> Vec<FrameError> {
    let all = vec![FrameError::DeviceLost];
    for error in &all {
        match error {
            FrameError::DeviceLost => {}
        }
    }
    all
}

/// 作成の失敗の見本 (`CreateError` のすべての種類)。種類を足すと、`match` が網羅の誤りになる。
fn create_errors() -> Vec<CreateError> {
    let s = |v: &str| v.to_owned();
    let all = vec![
        CreateError::BackendUnavailable,
        CreateError::ContextUnavailable(s("c")),
        CreateError::AdapterUnavailable(s("a")),
        CreateError::DeviceUnavailable(s("d")),
    ];
    for error in &all {
        match error {
            CreateError::BackendUnavailable
            | CreateError::ContextUnavailable(_)
            | CreateError::AdapterUnavailable(_)
            | CreateError::DeviceUnavailable(_) => {}
        }
    }
    all
}

/// 設定の差し替えの失敗の見本 (`AnimationError` のすべての種類。`code` を持つのは検査と合成の失敗だけ。
/// 合成の失敗の種類は付け外しの失敗と同じ `ugoe::DressUpError` なので、網羅は [`guard`] で確かめる)。
/// 種類を足すと、`match` が網羅の誤りになる。
fn animation_errors() -> Vec<AnimationError> {
    let s = |v: &str| v.to_owned();
    let all = vec![
        FormatError::BadMagic.into(),
        serde_json::from_str::<()>("{").unwrap_err().into(),
        AnimationError::InvalidPhysics(validation()),
        AnimationError::InvalidEffects(validation()),
        AnimationError::DressUp(DressUpError::NotAttached(s("m"))),
        AnimationError::UnknownMotion(s("m")),
        AnimationError::UnknownExpression(s("e")),
        AnimationError::UnknownPriority(s("p")),
        AnimationError::UnknownStage(s("s")),
    ];
    for error in &all {
        match error {
            AnimationError::Format(_)
            | AnimationError::Json(_)
            | AnimationError::InvalidPhysics(_)
            | AnimationError::InvalidEffects(_)
            | AnimationError::DressUp(_)
            | AnimationError::UnknownMotion(_)
            | AnimationError::UnknownExpression(_)
            | AnimationError::UnknownPriority(_)
            | AnimationError::UnknownStage(_) => {}
        }
    }
    all
}

fn add(codes: &mut Codes, info: &ErrorInfo) {
    codes
        .entry(info.code.to_owned())
        .or_default()
        .extend(info.fields.iter().map(|(k, _)| (*k).to_owned()));
}

/// 警告の JSON (`load_warnings_json` の要素) を、`code` と `warning` を除いたキーにする。
fn add_warning(codes: &mut Codes, json: &serde_json::Value) {
    let object = json.as_object().unwrap();
    let keys = object
        .keys()
        .filter(|k| *k != "code" && *k != "warning")
        .cloned();
    codes
        .entry(object["code"].as_str().unwrap().to_owned())
        .or_default()
        .extend(keys);
}

/// 今の実装が返す `code` とキーの一覧 (`codes.json` の形)。
fn actual() -> serde_json::Value {
    let mut load_error = Codes::new();
    for error in load_errors() {
        add(&mut load_error, &error.info());
    }
    let mut dress_up_error = Codes::new();
    let wardrobe = wardrobe_errors();
    for error in &wardrobe {
        guard(error);
        add(&mut dress_up_error, &error_info(error));
    }
    let mut load_warning = Codes::new();
    for warning in LoadWarning::samples() {
        add_warning(&mut load_warning, &warning_json(&warning));
    }
    // 付け直せずに外したモジュール (`module_detached`)。`error` の中身は着せ替えの失敗と同じ。
    for error in wardrobe {
        let detached = Detached {
            module: "m".into(),
            slot: "s".into(),
            error,
        };
        add_warning(&mut load_warning, &detached.to_json());
    }
    let mut render_error = Codes::new();
    for error in frame_errors() {
        add(&mut render_error, &error.info());
    }
    let mut create_error = Codes::new();
    for error in create_errors() {
        add(&mut create_error, &error.info());
    }
    // `set_base_physics_json` の合成の失敗は `name` が `"DressUpError"` なので、着せ替えの失敗の一覧に入る (T0476)。
    let mut settings_error = Codes::new();
    for error in animation_errors() {
        if let Some(info) = error.info() {
            let codes = match error.name() {
                WardrobeError::NAME => &mut dress_up_error,
                _ => &mut settings_error,
            };
            add(codes, &info);
        }
    }
    serde_json::json!({
        "load_warning": load_warning,
        "dress_up_error": dress_up_error,
        "load_error": load_error,
        "render_error": render_error,
        "create_error": create_error,
        "settings_error": settings_error,
    })
}

#[test]
fn codes_json_lists_every_code_and_field() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("codes.json");
    let actual = actual();
    if std::env::var_os("UGOE_UPDATE_CODES").is_some() {
        let text = serde_json::to_string_pretty(&actual).unwrap();
        std::fs::write(&path, text + "\n").unwrap();
    }
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        actual,
        expected,
        "crates/ugoe-wasm/codes.json is out of date; run `UGOE_UPDATE_CODES=1 cargo test -p ugoe-wasm codes` \
         and update bindings/web/src (warnings.ts・limits.ts・render-errors.ts・create-errors.ts・settings-errors.ts)\n{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
}

/// SDK の見本は、どれも SDK の `code` と補足の値のキーのまま JS に渡る (wasm の側で別の種類にまとめない。
/// 例外は、着せ替えの読み込みの失敗で上限を超えたもの以外を `invalid_file` にまとめること)。
#[test]
fn sdk_samples_keep_their_codes() {
    let keys = |fields: &[(&'static str, serde_json::Value)]| -> Vec<&str> {
        fields.iter().map(|(k, _)| *k).collect()
    };
    for sample in ugoe::LoadError::samples() {
        let (code, fields) = (sample.code(), sample.fields());
        let info = LoadError::Model(sample).info();
        assert_eq!(info.code, code);
        let expected: Vec<_> = fields.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys(&info.fields), expected, "{code}");
    }
    for sample in DressUpError::samples() {
        let code = sample.code();
        let expected: Vec<_> = sample.fields().iter().map(|(k, _)| *k).collect();
        let info = error_info(&WardrobeError::DressUp(sample));
        assert_eq!(info.code, code);
        assert_eq!(keys(&info.fields), expected, "{code}");
    }
    for sample in LoadWarning::samples() {
        let json = warning_json(&sample);
        assert_eq!(json["code"], sample.code());
        for (key, _) in sample.fields() {
            assert!(json.get(key).is_some(), "{}: {key}", sample.code());
        }
    }
}
