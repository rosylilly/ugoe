//! T0414 で足した C ABI (パーツの一覧と不透明度の上書き・ポーズの表示の選択・効果の設定の差し替え) を
//! Rust から呼んで、`ugoe::Puppet` と同じ結果になることを確かめる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作って破棄していない `UgoePuppet`、
// 生きている Rust の値を指すポインタ、または意図的に NULL を渡す。借りたポインタは無効になる前に読む。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use ugoe::Puppet;
use ugoe::model::effects::Effects;
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
