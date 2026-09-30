//! T0409 で足した C ABI (着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告・パラメータの一括設定・
//! 表情の解除・段の有効無効) を Rust から呼んで、`ugoe::Puppet` と同じ結果になることを確かめる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作って破棄していない `UgoePuppet` /
// `UgoeModule`、生きている Rust の値を指すポインタ、または意図的に NULL を渡す。借りたポインタは無効になる前に読む。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use ugoe::Puppet;
use ugoe::format::module_file::{ModuleFile, write_module};
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::avatar::{AvatarHitArea, AvatarProfile};
use ugoe::model::fixtures::{self, dress_module, dress_up_base, long_hair_module};
use ugoe::model::module::Module;
use ugoe::model::{HitArea, TextureRef, standard};
use ugoe_ffi::*;

const DT: f32 = 1.0 / 60.0;

fn png(tag: u8) -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, tag],
    }
}

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

struct ModuleHandle(*mut UgoeModule);

impl Drop for ModuleHandle {
    fn drop(&mut self) {
        unsafe { ugoe_module_destroy(self.0) };
    }
}

fn from_bundle(bundle: &Bundle) -> Handle {
    let bytes = ugoe::format::write_bundle(bundle).unwrap();
    let mut p = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut p) });
    Handle(p)
}

fn from_model(model: ugoe::model::Model) -> Handle {
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    from_bundle(&bundle)
}

fn module(module: Module, tag: u8) -> ModuleHandle {
    let mut file = ModuleFile::new(module);
    file.textures = file.module.textures.iter().map(|_| png(tag)).collect();
    let bytes = write_module(&file).unwrap();
    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m) });
    ModuleHandle(m)
}

fn texture(p: &Handle, index: u32) -> Vec<u8> {
    let mut t = UgoeTexture {
        format: u32::MAX,
        data: ptr::null(),
        len: 0,
    };
    ok(unsafe { ugoe_puppet_texture(p.0, index, &mut t) });
    if t.data.is_null() {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(t.data, t.len) }.to_vec()
}

fn count(p: &Handle, f: unsafe extern "C" fn(*const UgoePuppet, *mut u32) -> UgoeResult) -> u32 {
    let mut n = u32::MAX;
    ok(unsafe { f(p.0, &mut n) });
    n
}

fn drawable_ids(p: &Handle) -> Vec<String> {
    (0..count(p, ugoe_puppet_drawable_count))
        .map(|d| {
            let mut info = std::mem::MaybeUninit::<UgoeDrawableInfo>::uninit();
            ok(unsafe { ugoe_puppet_drawable_info(p.0, d, info.as_mut_ptr()) });
            text(unsafe { info.assume_init() }.id)
        })
        .collect()
}

fn changed_textures(p: &Handle) -> Vec<u32> {
    let mut ptr = ptr::null();
    let mut len = usize::MAX;
    ok(unsafe { ugoe_puppet_changed_textures(p.0, &mut ptr, &mut len) });
    if len == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
}

fn param(p: &Handle, id: &str) -> f32 {
    let id = CString::new(id).unwrap();
    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, id.as_ptr(), &mut index) });
    let mut value = f32::NAN;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, index, &mut value) });
    value
}

fn field(key: &str) -> String {
    let key = CString::new(key).unwrap();
    text(unsafe { ugoe_last_error_field(key.as_ptr()) })
}

#[test]
fn modules_attach_detach_and_follow_the_puppet() {
    let p = from_model(dress_up_base());
    let hair = module(long_hair_module(), 1);
    let dress = module(dress_module(), 2);

    let mut s = ptr::null();
    ok(unsafe { ugoe_module_name(hair.0, &mut s) });
    assert_eq!(text(s), "LongHair");
    ok(unsafe { ugoe_module_slot(hair.0, &mut s) });
    assert_eq!(text(s), "Hair");
    let slots: Vec<String> = (0..count(&p, ugoe_puppet_slot_count))
        .map(|i| {
            ok(unsafe { ugoe_puppet_slot_id(p.0, i, &mut s) });
            text(s)
        })
        .collect();
    assert!(slots.contains(&"Hair".to_owned()));
    assert!(slots.contains(&"Outfit".to_owned()));
    assert!(changed_textures(&p).is_empty());

    // Rust の Puppet と同じモデルになる。
    let mut reference = Puppet::new(dress_up_base()).unwrap();
    ok(unsafe { ugoe_puppet_attach_module(p.0, hair.0) });
    let change = reference
        .attach_module(std::sync::Arc::new(long_hair_module()))
        .unwrap();
    assert_eq!(changed_textures(&p), change.textures);
    assert_eq!(count(&p, ugoe_puppet_texture_count), change.texture_count);
    let expected: Vec<&str> = reference
        .instance()
        .compiled()
        .model()
        .meshes
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(drawable_ids(&p), expected);
    assert_eq!(texture(&p, 0), png(0).bytes);
    assert_eq!(texture(&p, 1), png(1).bytes);

    // 付けたモジュールは、UgoeModule を破棄しても残る。
    drop(hair);
    ok(unsafe { ugoe_puppet_attach_module(p.0, dress.0) });
    assert_eq!(changed_textures(&p), [2]);
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 2);
    let names: Vec<String> = (0..2)
        .map(|i| {
            ok(unsafe { ugoe_puppet_attached_module_name(p.0, i, &mut s) });
            text(s)
        })
        .collect();
    assert_eq!(names, ["LongHair", "Dress"]);
    assert_eq!(texture(&p, 1), png(1).bytes);
    assert_eq!(texture(&p, 2), png(2).bytes);
    ok(unsafe { ugoe_puppet_update(p.0, DT) });

    let name = CString::new("LongHair").unwrap();
    ok(unsafe { ugoe_puppet_detach_module(p.0, name.as_ptr()) });
    assert_eq!(count(&p, ugoe_puppet_texture_count), 2);
    assert_eq!(changed_textures(&p), [1]);
    assert_eq!(texture(&p, 1), png(2).bytes);
    assert!(
        !drawable_ids(&p)
            .iter()
            .any(|id| id.starts_with("LongHair/"))
    );

    ok(unsafe { ugoe_puppet_set_modules(p.0, ptr::null(), 0) });
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 0);
    assert_eq!(count(&p, ugoe_puppet_texture_count), 1);
    let base = Puppet::new(dress_up_base()).unwrap();
    assert_eq!(
        count(&p, ugoe_puppet_drawable_count) as usize,
        base.instance().drawables().len()
    );
    let list = [dress.0.cast_const()];
    ok(unsafe { ugoe_puppet_set_modules(p.0, list.as_ptr(), list.len()) });
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 1);
}

#[test]
fn dress_up_failures_carry_the_wasm_codes() {
    let p = from_model(dress_up_base());
    let mut stray = long_hair_module();
    stray.slot = "Nope".into();
    let stray = module(stray, 1);
    assert_eq!(
        unsafe { ugoe_puppet_attach_module(p.0, stray.0) },
        UGOE_ERROR_DRESS_UP
    );
    assert_eq!(text(ugoe_last_error_kind()), "unknown_slot");
    assert_eq!(field("module"), "LongHair");
    assert_eq!(field("slot"), "Nope");
    // 失敗したときは何も変えない。
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 0);

    let name = CString::new("Dress").unwrap();
    assert_eq!(
        unsafe { ugoe_puppet_detach_module(p.0, name.as_ptr()) },
        UGOE_ERROR_DRESS_UP
    );
    assert_eq!(text(ugoe_last_error_kind()), "not_attached");
    assert_eq!(field("module"), "Dress");

    let garbage = b"not a module";
    let mut m = ptr::null_mut();
    assert_eq!(
        unsafe { ugoe_module_create(garbage.as_ptr(), garbage.len(), &mut m) },
        UGOE_ERROR_FORMAT
    );
    assert!(m.is_null());
    assert_eq!(text(ugoe_last_error_kind()), "invalid_file");

    // 本体がスロットを持たなければ付けられない。
    let q = from_model(fixtures::single_quad());
    let hair = module(long_hair_module(), 1);
    assert_eq!(
        unsafe { ugoe_puppet_attach_module(q.0, hair.0) },
        UGOE_ERROR_DRESS_UP
    );
    assert!(!text(ugoe_last_error_kind()).is_empty());
}

#[test]
fn normal_maps_and_lit_drawables() {
    let mut model = fixtures::single_quad();
    model.lighting = true;
    model.meshes[0].lit = true;
    let mut second: TextureRef = model.textures[0].clone();
    second.normal_map = Some("normal".into());
    model.textures.push(second);
    let mut bundle = Bundle::new(model);
    bundle.textures = vec![png(0), png(1)];
    bundle.normal_maps = vec![png(9)];
    let p = from_bundle(&bundle);

    let mut uses = false;
    ok(unsafe { ugoe_puppet_uses_lighting(p.0, &mut uses) });
    assert!(uses);
    let mut lit = false;
    ok(unsafe { ugoe_puppet_drawable_lit(p.0, 0, &mut lit) });
    assert!(lit);

    let mut t = UgoeTexture {
        format: u32::MAX,
        data: ptr::null(),
        len: 0,
    };
    ok(unsafe { ugoe_puppet_normal_map(p.0, 0, &mut t) });
    assert_eq!(t.format, UGOE_TEXTURE_NONE);
    ok(unsafe { ugoe_puppet_normal_map(p.0, 1, &mut t) });
    assert_eq!(t.format, UGOE_TEXTURE_PNG);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(t.data, t.len) },
        png(9).bytes
    );
    assert_eq!(
        unsafe { ugoe_puppet_normal_map(p.0, 2, &mut t) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let q = from_model(fixtures::single_quad());
    ok(unsafe { ugoe_puppet_uses_lighting(q.0, &mut uses) });
    assert!(!uses);
}

#[test]
fn lighting_uniform_matches_the_reference_layout() {
    let lights = [
        UgoeLight {
            kind: UGOE_LIGHT_DIRECTIONAL,
            vector: [0.0, 0.0, -2.0],
            color: [1.0, 1.0, 1.0],
            range: 0.0,
        },
        UgoeLight {
            kind: UGOE_LIGHT_POINT,
            vector: [1.0, 2.0, 3.0],
            color: [2.0, 1.0, 0.5],
            range: 4.0,
        },
        UgoeLight {
            kind: UGOE_LIGHT_POINT,
            vector: [0.0; 3],
            color: [1.0; 3],
            range: f32::INFINITY,
        },
    ];
    let ambient = [0.1, 0.2, 0.3];
    let mut u = UgoeLightingUniform::default();
    ok(unsafe { ugoe_lighting_uniform(ambient.as_ptr(), lights.as_ptr(), lights.len(), &mut u) });
    assert_eq!(u.ambient, [0.1, 0.2, 0.3, 0.0]);
    assert_eq!(u.count, 3);
    assert_eq!(u.lights[0].vector, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(u.lights[1].vector, [1.0, 2.0, 3.0, 1.0]);
    assert_eq!(u.lights[1].color, [2.0, 1.0, 0.5, 0.25]);
    assert_eq!(u.lights[2].color[3], 0.0);
    // 32 + 8 × 32 バイト (参照シェーダのユニフォームと同じ)。
    assert_eq!(
        std::mem::size_of::<UgoeLightingUniform>(),
        32 + 32 * UGOE_MAX_LIGHTS
    );

    // 既定は環境光が白で光源なし。
    ok(unsafe { ugoe_lighting_uniform(ptr::null(), ptr::null(), 0, &mut u) });
    assert_eq!(u.ambient, [1.0, 1.0, 1.0, 0.0]);
    assert_eq!(u.count, 0);

    // 上限を超えた光源は先頭だけを使う。
    let many = vec![lights[0]; UGOE_MAX_LIGHTS + 2];
    ok(unsafe { ugoe_lighting_uniform(ptr::null(), many.as_ptr(), many.len(), &mut u) });
    assert_eq!(u.count as usize, UGOE_MAX_LIGHTS);

    let bad = [
        UgoeLight {
            vector: [0.0; 3],
            ..lights[0]
        },
        UgoeLight {
            range: 0.0,
            ..lights[1]
        },
        UgoeLight {
            range: f32::NAN,
            ..lights[1]
        },
        UgoeLight {
            color: [-1.0, 0.0, 0.0],
            ..lights[0]
        },
        UgoeLight {
            kind: 7,
            ..lights[0]
        },
    ];
    for light in bad {
        assert_eq!(
            unsafe { ugoe_lighting_uniform(ptr::null(), &light, 1, &mut u) },
            UGOE_ERROR_INVALID_ARGUMENT,
            "{light:?}"
        );
    }
}

#[test]
fn hit_tests_follow_the_evaluated_shape() {
    let mut model = fixtures::single_quad();
    model.hit_areas.push(HitArea {
        id: "Body".into(),
        mesh: "Quad".into(),
    });
    model.avatar = Some(AvatarProfile {
        hit_areas: vec![AvatarHitArea {
            name: "Body".into(),
            hit_area: "Body".into(),
        }],
        ..AvatarProfile::default()
    });
    let p = from_model(model);
    assert_eq!(count(&p, ugoe_puppet_hit_area_count), 1);
    let mut s = ptr::null();
    ok(unsafe { ugoe_puppet_hit_area_id(p.0, 0, &mut s) });
    assert_eq!(text(s), "Body");

    let body = CString::new("Body").unwrap();
    let head = CString::new("Head").unwrap();
    let (x, y) = (220.0, 100.0);
    let mut hit = true;
    ok(unsafe { ugoe_puppet_hit_test(p.0, body.as_ptr(), x, y, &mut hit) });
    assert!(!hit);
    ok(unsafe { ugoe_puppet_set_parameter(p.0, 0, 1.0) });
    ok(unsafe { ugoe_puppet_update(p.0, DT) });
    ok(unsafe { ugoe_puppet_hit_test(p.0, body.as_ptr(), x, y, &mut hit) });
    assert!(hit);
    assert_eq!(
        unsafe { ugoe_puppet_hit_test(p.0, head.as_ptr(), x, y, &mut hit) },
        UGOE_ERROR_NOT_FOUND
    );

    let mut n = usize::MAX;
    assert_eq!(
        unsafe { ugoe_puppet_hit_areas_at(p.0, x, y, ptr::null_mut(), 0, &mut n) },
        UGOE_ERROR_BUFFER_TOO_SMALL
    );
    assert_eq!(n, 1);
    let mut hits = [u32::MAX; 2];
    ok(unsafe { ugoe_puppet_hit_areas_at(p.0, x, y, hits.as_mut_ptr(), hits.len(), &mut n) });
    assert_eq!((n, hits[0]), (1, 0));
    ok(unsafe { ugoe_puppet_hit_areas_at(p.0, 10.0, 10.0, ptr::null_mut(), 0, &mut n) });
    assert_eq!(n, 0);

    let mut index = u32::MAX;
    ok(unsafe { ugoe_puppet_standard_hit_area(p.0, body.as_ptr(), &mut index) });
    assert_eq!(index, 0);
    assert_eq!(
        unsafe { ugoe_puppet_standard_hit_area(p.0, head.as_ptr(), &mut index) },
        UGOE_ERROR_NOT_FOUND
    );
    ok(unsafe { ugoe_puppet_hit_test_standard(p.0, body.as_ptr(), x, y, &mut hit) });
    assert!(hit);
    ok(unsafe { ugoe_puppet_hit_test_standard(p.0, head.as_ptr(), x, y, &mut hit) });
    assert!(!hit);
}

fn tracking_input() -> UgoeTrackingInput {
    UgoeTrackingInput {
        head_yaw: 0.0,
        head_pitch: 0.0,
        head_roll: 0.0,
        body_yaw: f32::NAN,
        body_pitch: f32::NAN,
        body_roll: f32::NAN,
        shoulder_left: f32::NAN,
        shoulder_right: f32::NAN,
        blend_shapes: [0.0; UGOE_BLEND_SHAPE_COUNT],
    }
}

#[test]
fn tracking_writes_into_the_parameters() {
    let mut model = fixtures::avatar_profile();
    model.effects = ugoe::model::effects::Effects::default();
    let p = from_model(model);

    let name = CString::new("eyeBlinkLeft").unwrap();
    let mut index = u32::MAX;
    ok(unsafe { ugoe_blend_shape_index(name.as_ptr(), &mut index) });
    let mut s = ptr::null();
    ok(unsafe { ugoe_blend_shape_name(index, &mut s) });
    assert_eq!(text(s), "eyeBlinkLeft");
    let neutral = CString::new("_neutral").unwrap();
    assert_eq!(
        unsafe { ugoe_blend_shape_index(neutral.as_ptr(), &mut index) },
        UGOE_ERROR_NOT_FOUND
    );
    assert_eq!(
        unsafe { ugoe_blend_shape_name(UGOE_BLEND_SHAPE_COUNT as u32, &mut s) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let mut input = tracking_input();
    ok(unsafe { ugoe_blend_shape_index(name.as_ptr(), &mut index) });
    input.blend_shapes[index as usize] = 0.75;
    input.head_yaw = 10.0;
    ok(unsafe { ugoe_puppet_set_tracking(p.0, &input) });
    ok(unsafe { ugoe_puppet_update(p.0, DT) });
    assert!((param(&p, standard::EYE_L_OPEN) - 0.25).abs() < 1e-4);
    assert!((param(&p, standard::ANGLE_X) - 10.0).abs() < 1e-4);

    // 入力をやめても最後の値は残る。
    ok(unsafe { ugoe_puppet_set_tracking(p.0, ptr::null()) });
    ok(unsafe { ugoe_puppet_update(p.0, DT) });
    assert!((param(&p, standard::EYE_L_OPEN) - 0.25).abs() < 1e-4);

    input.head_yaw = f32::INFINITY;
    assert_eq!(
        unsafe { ugoe_puppet_set_tracking(p.0, &input) },
        UGOE_ERROR_INVALID_ARGUMENT
    );

    let mut n = 0;
    assert_eq!(
        unsafe { ugoe_puppet_tracking_targets(p.0, ptr::null_mut(), 0, &mut n) },
        UGOE_ERROR_BUFFER_TOO_SMALL
    );
    let mut targets = vec![0u32; n];
    ok(unsafe { ugoe_puppet_tracking_targets(p.0, targets.as_mut_ptr(), n, &mut n) });
    let angle_x = CString::new(standard::ANGLE_X).unwrap();
    ok(unsafe { ugoe_puppet_parameter_index(p.0, angle_x.as_ptr(), &mut index) });
    assert!(targets.contains(&index));
    assert!(targets.is_sorted());
}

#[test]
fn stages_can_be_turned_off() {
    let p = from_model(fixtures::avatar_profile());
    let eye = standard::EYE_L_OPEN;
    ok(unsafe { ugoe_puppet_set_stage_enabled(p.0, UGOE_STAGE_EYE_BLINK, false) });
    ok(unsafe { ugoe_puppet_set_stage_enabled(p.0, UGOE_STAGE_BREATH, false) });
    let before = param(&p, eye);
    for _ in 0..600 {
        ok(unsafe { ugoe_puppet_update(p.0, DT) });
        assert_eq!(param(&p, eye), before);
    }
    assert_eq!(
        unsafe { ugoe_puppet_set_stage_enabled(p.0, 99, true) },
        UGOE_ERROR_INVALID_ARGUMENT
    );
}

#[test]
fn parameters_can_be_set_by_id_in_bulk() {
    let p = from_model(fixtures::avatar_profile());
    let ids = [standard::ANGLE_X, "Missing", standard::ANGLE_Y].map(|s| CString::new(s).unwrap());
    let ptrs = ids.each_ref().map(|s| s.as_ptr());
    let values = [12.0, 1.0, 1000.0];
    let mut applied = 0;
    ok(unsafe {
        ugoe_puppet_set_parameters_by_id(p.0, ptrs.as_ptr(), values.as_ptr(), 3, &mut applied)
    });
    assert_eq!(applied, 2);
    assert_eq!(param(&p, standard::ANGLE_X), 12.0);
    assert_eq!(param(&p, standard::ANGLE_Y), 30.0);

    // NaN があれば何も変えない。
    let values = [0.0, 0.0, f32::NAN];
    assert_eq!(
        unsafe {
            ugoe_puppet_set_parameters_by_id(
                p.0,
                ptrs.as_ptr(),
                values.as_ptr(),
                3,
                ptr::null_mut(),
            )
        },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(param(&p, standard::ANGLE_X), 12.0);
    ok(unsafe {
        ugoe_puppet_set_parameters_by_id(p.0, ptr::null(), ptr::null(), 0, ptr::null_mut())
    });
}

#[test]
fn expressions_can_be_stopped() {
    let mut bundle = Bundle::new(fixtures::single_quad());
    bundle.textures = vec![png(0)];
    bundle.expressions = vec![
        ugoe::format::expression_from_json(
            r#"{ "name": "smile", "fade_in": 0, "fade_out": 0, "parameters": [{ "parameter": "X", "value": 0.5, "op": "add" }] }"#,
        )
        .unwrap(),
    ];
    let p = from_bundle(&bundle);
    ok(unsafe { ugoe_puppet_set_expression(p.0, 0) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0) });
    assert!((param(&p, "X") - 0.5).abs() < 1e-5);
    ok(unsafe { ugoe_puppet_stop_expressions(p.0) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0) });
    assert!(param(&p, "X").abs() < 1e-5);
}

#[test]
fn load_warnings_have_codes_and_fields() {
    let p = from_model(fixtures::single_quad());
    assert_eq!(count(&p, ugoe_puppet_load_warning_count), 0);

    // 新しい版を模したバンドル (知らない任意の機能と未知の値。assets/compat/README.md)。
    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoe");
    let mut raw = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut raw) });
    let p = Handle(raw);
    let n = count(&p, ugoe_puppet_load_warning_count);
    let mut w = UgoeWarning {
        code: ptr::null(),
        message: ptr::null(),
    };
    let mut codes = Vec::new();
    let feature = CString::new("feature").unwrap();
    let path = CString::new("path").unwrap();
    let mut value = ptr::null();
    for i in 0..n {
        ok(unsafe { ugoe_puppet_load_warning(p.0, i, &mut w) });
        let code = text(w.code);
        assert!(!text(w.message).is_empty());
        let key = if code == "unknown_optional_feature" {
            &feature
        } else {
            &path
        };
        ok(unsafe { ugoe_puppet_load_warning_field(p.0, i, key.as_ptr(), &mut value) });
        assert!(!text(value).is_empty());
        codes.push(code);
    }
    assert!(
        codes.contains(&"unknown_optional_feature".to_owned()),
        "{codes:?}"
    );
    assert!(codes.contains(&"unknown_value".to_owned()), "{codes:?}");
    let other = CString::new("texture").unwrap();
    assert_eq!(
        unsafe { ugoe_puppet_load_warning_field(p.0, 0, other.as_ptr(), &mut value) },
        UGOE_ERROR_NOT_FOUND
    );
    assert_eq!(
        unsafe { ugoe_puppet_load_warning(p.0, n, &mut w) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoemod");
    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m) });
    let future = ModuleHandle(m);
    let mut n = 0;
    ok(unsafe { ugoe_module_warning_count(future.0, &mut n) });
    assert!(n > 0);
    ok(unsafe { ugoe_module_warning(future.0, 0, &mut w) });
    assert!(!text(w.code).is_empty());

    let hair = module(long_hair_module(), 1);
    let mut n = u32::MAX;
    ok(unsafe { ugoe_module_warning_count(hair.0, &mut n) });
    assert_eq!(n, 0);
}
