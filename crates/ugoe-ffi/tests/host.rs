//! T0414 で足した C ABI (パーツの一覧と不透明度の上書き・ポーズの表示の選択・効果の設定の差し替え) と
//! T0416 で足した評価後のパーツの不透明度と、T0478 で足した表示中のポーズの要素を Rust から呼んで、
//! `ugoe::Puppet` と同じ結果になることを確かめる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作って破棄していない `UgoePuppet`、
// 生きている Rust の値を指すポインタ、または意図的に NULL を渡す。借りたポインタは無効になる前に読む。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use ugoe::Puppet;
use ugoe::format::module_file::{ModuleFile, write_module};
use ugoe::format::{Bundle, TextureData, TextureFormat, motion_from_json, write_bundle};
use ugoe::model::effects::Effects;
use ugoe::model::pose::{PoseEntry, PoseGroup};
use ugoe::model::{Model, fixtures, standard};
use ugoe_ffi::*;

const DT: f32 = 1.0 / 60.0;

#[track_caller]
fn ok(code: UgoeResult) {
    assert_eq!(code, UGOE_OK, "{}", text(ugoe_last_error()));
}

fn text(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

struct Handle(*mut UgoePuppet);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { ugoe_puppet_destroy(self.0) };
    }
}

fn from_model(model: &Model) -> Handle {
    let json = ugoe::format::model_to_json(model).unwrap();
    let mut p = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_json(json.as_ptr(), json.len(), &mut p) });
    Handle(p)
}

fn part_index(p: &Handle, id: &str) -> u32 {
    let id = CString::new(id).unwrap();
    let mut index = u32::MAX;
    ok(unsafe { ugoe_puppet_part_index(p.0, id.as_ptr(), &mut index) });
    index
}

fn part_opacity(p: &Handle, index: u32) -> (f32, f32) {
    let mut out = UgoePartOpacity {
        value: f32::NAN,
        weight: f32::NAN,
    };
    ok(unsafe { ugoe_puppet_part_opacity(p.0, index, &mut out) });
    (out.value, out.weight)
}

fn show_pose(p: &Handle, part: &str) -> bool {
    let part = CString::new(part).unwrap();
    let mut shown = false;
    ok(unsafe { ugoe_puppet_show_pose(p.0, part.as_ptr(), &mut shown) });
    shown
}

fn param(p: &Handle, id: &str) -> f32 {
    let id = CString::new(id).unwrap();
    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, id.as_ptr(), &mut index) });
    let mut value = f32::NAN;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, index, &mut value) });
    value
}

fn rust_param(puppet: &Puppet, id: &str) -> f32 {
    let params = puppet.instance().parameters();
    params.get(params.index_of(id).unwrap())
}

fn set_effects(p: &Handle, json: &str) -> UgoeResult {
    unsafe { ugoe_puppet_set_effects_json(p.0, json.as_ptr(), json.len()) }
}

/// パーツの一覧は `Model::parts` の並びで、ID から番号を引ける。
#[test]
fn parts_are_listed_in_model_order() {
    let model = fixtures::pose_and_effects();
    let p = from_model(&model);
    let mut n = u32::MAX;
    ok(unsafe { ugoe_puppet_part_count(p.0, &mut n) });
    assert_eq!(n as usize, model.parts.len());
    for (i, part) in model.parts.iter().enumerate() {
        let mut id = ptr::null();
        ok(unsafe { ugoe_puppet_part_id(p.0, i as u32, &mut id) });
        assert_eq!(text(id), part.id.as_str());
        assert_eq!(part_index(&p, part.id.as_str()), i as u32);
    }

    let mut id = ptr::null();
    assert_eq!(
        unsafe { ugoe_puppet_part_id(p.0, n, &mut id) },
        UGOE_ERROR_OUT_OF_RANGE
    );
    let missing = CString::new("Missing").unwrap();
    let mut index = 0;
    assert_eq!(
        unsafe { ugoe_puppet_part_index(p.0, missing.as_ptr(), &mut index) },
        UGOE_ERROR_NOT_FOUND
    );
    let mut out = UgoePartOpacity::default();
    assert_eq!(
        unsafe { ugoe_puppet_part_opacity(p.0, n, &mut out) },
        UGOE_ERROR_OUT_OF_RANGE
    );
}

/// ポーズの表示の選択は `PosePlayer::show` と同じで、パーツの不透明度の上書きが Rust の Puppet と一致する。
#[test]
fn showing_a_pose_entry_follows_the_puppet() {
    let model = fixtures::pose_and_effects();
    let p = from_model(&model);
    let mut rust = Puppet::new(model.clone()).unwrap();
    let parts: Vec<u32> = ["ArmA", "SleeveA", "ArmB"]
        .iter()
        .map(|id| part_index(&p, id))
        .collect();

    // 最初の update の前は上書きが無い。
    for &i in &parts {
        assert_eq!(part_opacity(&p, i).1, 0.0);
    }

    // リンクのパーツと、どの要素でもないパーツは何も変えない。
    assert!(!show_pose(&p, "SleeveA"));
    assert!(!show_pose(&p, "Missing"));
    // `out_shown` は NULL でもよい。
    let arm_b = CString::new("ArmB").unwrap();
    ok(unsafe { ugoe_puppet_show_pose(p.0, arm_b.as_ptr(), ptr::null_mut()) });
    assert!(rust.pose_mut().show(&"ArmB".into()));

    let compiled = rust.instance().compiled().clone();
    for frame in 0..40 {
        if frame == 35 {
            assert!(show_pose(&p, "ArmA"));
            assert!(rust.pose_mut().show(&"ArmA".into()));
        }
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        rust.update(DT);
        for (id, &i) in ["ArmA", "SleeveA", "ArmB"].iter().zip(&parts) {
            let expected = rust
                .instance()
                .part_opacity_override(compiled.part_index(id).unwrap());
            assert_eq!(part_opacity(&p, i), expected, "{id} at frame {frame}");
        }
    }
    // ArmB を 0.5 秒 (fade_duration) より長く表示した後、ArmA に戻す途中。
    let (arm_a, _) = part_opacity(&p, parts[0]);
    assert!(arm_a > 0.0 && arm_a < 1.0, "{arm_a}");
    assert_eq!(part_opacity(&p, parts[1]).0, arm_a);
}

/// 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
fn png() -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
    }
}

/// ポーズの各グループで表示中の要素のパーツの ID (要素の無いグループは `None`)。
fn visible_pose_parts(p: &Handle) -> Vec<Option<String>> {
    let mut groups = u32::MAX;
    ok(unsafe { ugoe_puppet_pose_group_count(p.0, &mut groups) });
    (0..groups)
        .map(|group| {
            let mut part = u32::MAX;
            let code = unsafe { ugoe_puppet_pose_visible_part(p.0, group, &mut part) };
            if code == UGOE_ERROR_NOT_FOUND {
                return None;
            }
            ok(code);
            let mut id = ptr::null();
            ok(unsafe { ugoe_puppet_part_id(p.0, part, &mut id) });
            Some(text(id))
        })
        .collect()
}

fn some(id: &str) -> Option<String> {
    Some(id.to_owned())
}

/// 表示中のポーズの要素は、`ugoe_puppet_show_pose` の選択 (すぐ) とモーションの要求 (次の update の後) の両方に従う。
#[test]
fn visible_pose_parts_follow_show_pose_and_motions() {
    let model = fixtures::pose_and_effects();
    let motion = motion_from_json(
        r#"{
            "name": "arm_a",
            "duration": 1.0,
            "looping": true,
            "fade_in": 0.0,
            "fade_out": 0.0,
            "curves": [{
                "target": { "kind": "part_opacity", "id": "ArmA" },
                "segments": [
                    { "type": "start", "p": { "t": 0.0, "v": 1.0 } },
                    { "type": "linear", "to": { "t": 1.0, "v": 1.0 } }
                ]
            }]
        }"#,
    )
    .unwrap();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png()).collect();
    bundle.motions = vec![motion];
    let bytes = write_bundle(&bundle).unwrap();
    let mut raw = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut raw) });
    let p = Handle(raw);

    assert_eq!(visible_pose_parts(&p), [some("ArmA")]);
    assert!(show_pose(&p, "ArmB"));
    assert_eq!(visible_pose_parts(&p), [some("ArmB")]);
    ok(unsafe { ugoe_puppet_update(p.0, DT) });
    assert_eq!(visible_pose_parts(&p), [some("ArmB")]);

    let mut started = false;
    ok(unsafe { ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_NORMAL, &mut started) });
    assert!(started);
    ok(unsafe { ugoe_puppet_update(p.0, DT) });
    assert_eq!(visible_pose_parts(&p), [some("ArmA")]);

    let mut part = 0;
    assert_eq!(
        unsafe { ugoe_puppet_pose_visible_part(p.0, 1, &mut part) },
        UGOE_ERROR_OUT_OF_RANGE
    );
    assert_eq!(
        unsafe { ugoe_puppet_pose_visible_part(p.0, 0, ptr::null_mut()) },
        UGOE_ERROR_NULL_POINTER
    );
    assert_eq!(
        unsafe { ugoe_puppet_pose_group_count(ptr::null(), &mut part) },
        UGOE_ERROR_NULL_POINTER
    );
}

/// 要素の無いグループは `UGOE_ERROR_NOT_FOUND` で、ほかのグループの番号はずれない。付けたモジュールのグループも入る。
#[test]
fn visible_pose_parts_skip_empty_groups_and_include_modules() {
    let mut model = fixtures::pose_and_effects();
    model.pose.groups.insert(
        0,
        PoseGroup {
            entries: Vec::new(),
        },
    );
    let p = from_model(&model);
    assert_eq!(visible_pose_parts(&p), [None, some("ArmA")]);

    let p = from_model(&fixtures::dress_up_base());
    assert!(visible_pose_parts(&p).is_empty());
    let mut module = fixtures::long_hair_module();
    module.pose.groups = vec![PoseGroup {
        entries: vec![PoseEntry {
            part: "Hair".into(),
            links: Vec::new(),
        }],
    }];
    let mut file = ModuleFile::new(module);
    file.textures = file.module.textures.iter().map(|_| png()).collect();
    let bytes = write_module(&file).unwrap();
    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m) });
    ok(unsafe { ugoe_puppet_attach_module(p.0, m) });
    unsafe { ugoe_module_destroy(m) };
    assert_eq!(visible_pose_parts(&p), [some("LongHair/Hair")]);
}

/// 評価後のパーツの不透明度は `ModelInstance::part_opacity` と同じで、ポーズのフェードと祖先が掛かる。
#[test]
fn evaluated_part_opacity_follows_the_puppet() {
    let model = fixtures::pose_and_effects();
    let p = from_model(&model);
    let mut rust = Puppet::new(model.clone()).unwrap();
    let evaluated = |index: u32| {
        let mut out = f32::NAN;
        ok(unsafe { ugoe_puppet_part_evaluated_opacity(p.0, index, &mut out) });
        out
    };
    assert!(show_pose(&p, "ArmB"));
    assert!(rust.pose_mut().show(&"ArmB".into()));
    for frame in 0..20 {
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        rust.update(DT);
        for i in 0..model.parts.len() {
            let expected = rust.instance().part_opacity(i);
            assert_eq!(evaluated(i as u32), expected, "part {i} at frame {frame}");
        }
    }
    // フェードの途中。親 `Root` の不透明度は 1 なので、上書きの値と同じ。
    let arm_b = part_index(&p, "ArmB");
    let value = evaluated(arm_b);
    assert!(value > 0.0 && value < 1.0, "{value}");
    assert_eq!(part_opacity(&p, arm_b).0, value);

    let mut out = 0.0;
    assert_eq!(
        unsafe { ugoe_puppet_part_evaluated_opacity(p.0, model.parts.len() as u32, &mut out) },
        UGOE_ERROR_OUT_OF_RANGE
    );
}

/// 効果の設定の差し替えは `Puppet::set_effects` と同じで、目パチ・呼吸が同じ値になる。
#[test]
fn effect_settings_can_be_replaced() {
    let model = fixtures::pose_and_effects();
    let p = from_model(&model);
    let mut rust = Puppet::new(model).unwrap();

    let mut effects = Effects::standard();
    effects.eye_blink.interval = 0.5;
    effects.eye_blink.seed = u64::MAX - 1;
    let json = serde_json::to_string(&effects).unwrap();
    // seed は正確に読む (f64 に丸めない)。
    assert!(json.contains(&(u64::MAX - 1).to_string()));

    ok(unsafe { ugoe_puppet_set_lip_sync_level(p.0, 0.5) });
    rust.set_lip_sync_level(Some(0.5));
    for _ in 0..10 {
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        rust.update(DT);
    }
    ok(set_effects(&p, &json));
    rust.set_effects(effects);
    let mut closed = false;
    for frame in 0..120 {
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        rust.update(DT);
        for id in [
            standard::EYE_L_OPEN,
            standard::BREATH,
            standard::MOUTH_OPEN_Y,
        ] {
            assert_eq!(
                param(&p, id),
                rust_param(&rust, id),
                "{id} at frame {frame}"
            );
        }
        closed |= param(&p, standard::EYE_L_OPEN) < 0.5;
    }
    assert!(closed, "the eyes never blinked");
    // リップシンクの音量は保つ。
    assert!(param(&p, standard::MOUTH_OPEN_Y) > 0.0);

    // 省略したフィールドは既定値。読めない JSON は何も変えずに失敗する。
    ok(set_effects(&p, "{}"));
    assert_eq!(set_effects(&p, "{"), UGOE_ERROR_FORMAT);
    assert_eq!(set_effects(&p, "[1]"), UGOE_ERROR_FORMAT);
    assert_eq!(
        unsafe { ugoe_puppet_set_effects_json(p.0, ptr::null(), 0) },
        UGOE_ERROR_NULL_POINTER
    );
}

/// 効果の設定はモデルの読み込みと同じに検査する。有限でない値・範囲の外の値・モデルにないパラメータは
/// `UGOE_ERROR_INVALID_MODEL` (種類 `invalid_model`、補足の値 `errors`・`error_codes`) で拒み、今の設定を変えない (T0461・T0469)。
#[test]
fn invalid_effect_settings_are_rejected_without_changes() {
    let model = fixtures::pose_and_effects();
    let p = from_model(&model);
    ok(set_effects(
        &p,
        r#"{ "lip_sync": { "parameters": ["MouthOpenY"] } }"#,
    ));
    ok(unsafe { ugoe_puppet_set_lip_sync_level(p.0, 0.5) });

    let errors = CString::new("errors").unwrap();
    let error_codes = CString::new("error_codes").unwrap();
    for (json, count, codes) in [
        // `1e39` は `f32` の無限大になる。
        (
            r#"{ "eye_blink": { "interval": 1e39 } }"#,
            "1",
            "invalid_value",
        ),
        (
            r#"{ "eye_blink": { "interval": 0, "closing": -1 } }"#,
            "2",
            "invalid_value",
        ),
        (
            r#"{ "breath": { "parameters": [{ "parameter": "AngleX", "offset": 0, "peak": -1e39, "cycle": 3, "weight": 1 }] } }"#,
            "1",
            "invalid_value",
        ),
        (
            r#"{ "lip_sync": { "parameters": ["Missing"] } }"#,
            "1",
            "unknown_reference",
        ),
    ] {
        assert_eq!(set_effects(&p, json), UGOE_ERROR_INVALID_MODEL, "{json}");
        assert_eq!(text(ugoe_last_error_kind()), "invalid_model", "{json}");
        assert_eq!(
            text(unsafe { ugoe_last_error_field(errors.as_ptr()) }),
            count,
            "{json}"
        );
        assert_eq!(
            text(unsafe { ugoe_last_error_field(error_codes.as_ptr()) }),
            codes,
            "{json}"
        );
        assert!(text(ugoe_last_error()).starts_with("effects failed validation"));
        // リップシンクは前の設定のまま。
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        assert!(
            (param(&p, standard::MOUTH_OPEN_Y) - 0.5).abs() < 1e-5,
            "{json}"
        );
    }
}
