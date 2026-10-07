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

mod common;
use common::*;

const DT: f32 = 1.0 / 60.0;

fn png(tag: u8) -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, tag],
    }
}

#[track_caller]
fn ok(code: UgoeResult) {
    assert_eq!(code, UGOE_OK, "{}", text(last_message()));
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
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut p, err()) });
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
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m, err()) });
    ModuleHandle(m)
}

fn texture(p: &Handle, index: u32) -> Vec<u8> {
    let mut t = UgoeTexture {
        format: u32::MAX,
        data: ptr::null(),
        len: 0,
    };
    ok(unsafe { ugoe_puppet_texture(p.0, index, &mut t, err()) });
    if t.data.is_null() {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(t.data, t.len) }.to_vec()
}

fn count(
    p: &Handle,
    f: unsafe extern "C" fn(*const UgoePuppet, *mut u32, *mut UgoeError) -> UgoeResult,
) -> u32 {
    let mut n = u32::MAX;
    ok(unsafe { f(p.0, &mut n, err()) });
    n
}

fn drawable_ids(p: &Handle) -> Vec<String> {
    (0..count(p, ugoe_puppet_drawable_count))
        .map(|d| {
            let mut info = std::mem::MaybeUninit::<UgoeDrawableInfo>::uninit();
            ok(unsafe { ugoe_puppet_drawable_info(p.0, d, info.as_mut_ptr(), err()) });
            text(unsafe { info.assume_init() }.id)
        })
        .collect()
}

fn changed_textures(p: &Handle) -> Vec<u32> {
    let mut ptr = ptr::null();
    let mut len = usize::MAX;
    ok(unsafe { ugoe_puppet_changed_textures(p.0, &mut ptr, &mut len, err()) });
    if len == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
}

fn param(p: &Handle, id: &str) -> f32 {
    let id = CString::new(id).unwrap();
    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, id.as_ptr(), &mut index, err()) });
    let mut value = f32::NAN;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, index, &mut value, err()) });
    value
}

fn field(key: &str) -> String {
    let key = CString::new(key).unwrap();
    text(last_field(&key))
}

#[test]
fn modules_attach_detach_and_follow_the_puppet() {
    let p = from_model(dress_up_base());
    let hair = module(long_hair_module(), 1);
    let dress = module(dress_module(), 2);

    let mut s = ptr::null();
    ok(unsafe { ugoe_module_name(hair.0, &mut s, err()) });
    assert_eq!(text(s), "LongHair");
    ok(unsafe { ugoe_module_slot(hair.0, &mut s, err()) });
    assert_eq!(text(s), "Hair");
    let slots: Vec<String> = (0..count(&p, ugoe_puppet_slot_count))
        .map(|i| {
            ok(unsafe { ugoe_puppet_slot_id(p.0, i, &mut s, err()) });
            text(s)
        })
        .collect();
    assert!(slots.contains(&"Hair".to_owned()));
    assert!(slots.contains(&"Outfit".to_owned()));
    assert!(changed_textures(&p).is_empty());

    // Rust の Puppet と同じモデルになる。
    let mut reference = Puppet::new(dress_up_base()).unwrap();
    ok(unsafe { ugoe_puppet_attach_module(p.0, hair.0, err()) });
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
    ok(unsafe { ugoe_puppet_attach_module(p.0, dress.0, err()) });
    assert_eq!(changed_textures(&p), [2]);
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 2);
    let names: Vec<String> = (0..2)
        .map(|i| {
            ok(unsafe { ugoe_puppet_attached_module_name(p.0, i, &mut s, err()) });
            text(s)
        })
        .collect();
    assert_eq!(names, ["LongHair", "Dress"]);
    assert_eq!(texture(&p, 1), png(1).bytes);
    assert_eq!(texture(&p, 2), png(2).bytes);
    ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });

    let name = CString::new("LongHair").unwrap();
    ok(unsafe { ugoe_puppet_detach_module(p.0, name.as_ptr(), err()) });
    assert_eq!(count(&p, ugoe_puppet_texture_count), 2);
    assert_eq!(changed_textures(&p), [1]);
    assert_eq!(texture(&p, 1), png(2).bytes);
    assert!(
        !drawable_ids(&p)
            .iter()
            .any(|id| id.starts_with("LongHair/"))
    );

    ok(unsafe { ugoe_puppet_set_modules(p.0, ptr::null(), 0, err()) });
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 0);
    assert_eq!(count(&p, ugoe_puppet_texture_count), 1);
    let base = Puppet::new(dress_up_base()).unwrap();
    assert_eq!(
        count(&p, ugoe_puppet_drawable_count) as usize,
        base.instance().drawables().len()
    );
    let list = [dress.0.cast_const()];
    ok(unsafe { ugoe_puppet_set_modules(p.0, list.as_ptr(), list.len(), err()) });
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 1);
}

/// 同じスロットか同じ名前のモジュールを付けると、`ugoe::Puppet::attach_module` と同じく合成の順の同じ位置で差し替える。
/// スロットと名前で別々のモジュールに当たれば、両方を外して前の方の位置に入れる。
#[test]
fn attaching_replaces_modules_like_the_puppet() {
    let p = from_model(dress_up_base());
    let mut reference = Puppet::new(dress_up_base()).unwrap();
    let attached_names = |p: &Handle| -> Vec<String> {
        (0..count(p, ugoe_puppet_attached_module_count))
            .map(|i| {
                let mut s = ptr::null();
                ok(unsafe { ugoe_puppet_attached_module_name(p.0, i, &mut s, err()) });
                text(s)
            })
            .collect()
    };
    let reference_names = |puppet: &Puppet| -> Vec<String> {
        puppet.modules().iter().map(|m| m.name.clone()).collect()
    };
    let mut attach = |m: Module, tag: u8| {
        let handle = module(m.clone(), tag);
        ok(unsafe { ugoe_puppet_attach_module(p.0, handle.0, err()) });
        let change = reference.attach_module(std::sync::Arc::new(m)).unwrap();
        assert_eq!(changed_textures(&p), change.textures);
        assert_eq!(attached_names(&p), reference_names(&reference));
        let expected: Vec<&str> = reference
            .instance()
            .compiled()
            .model()
            .meshes
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(drawable_ids(&p), expected);
    };
    attach(long_hair_module(), 1);
    attach(dress_module(), 2);

    // 同じスロット (Hair) の別のモジュールは、LongHair の位置で差し替える。
    let mut other_hair = long_hair_module();
    other_hair.name = "OtherHair".into();
    attach(other_hair, 3);
    assert_eq!(attached_names(&p), ["OtherHair", "Dress"]);
    assert_eq!(texture(&p, 1), png(3).bytes);

    // スロットは OtherHair、名前は Dress に当たる。両方を外し、前の方 (OtherHair) の位置に入る。
    let mut renamed = long_hair_module();
    renamed.name = "Dress".into();
    attach(renamed, 4);
    assert_eq!(attached_names(&p), ["Dress"]);
    assert_eq!(texture(&p, 1), png(4).bytes);
}

#[test]
fn dress_up_failures_carry_the_wasm_codes() {
    let p = from_model(dress_up_base());
    let mut stray = long_hair_module();
    stray.slot = "Nope".into();
    let stray = module(stray, 1);
    assert_eq!(
        unsafe { ugoe_puppet_attach_module(p.0, stray.0, err()) },
        UGOE_ERROR_DRESS_UP
    );
    assert_eq!(text(last_kind()), "unknown_slot");
    assert_eq!(field("module"), "LongHair");
    assert_eq!(field("slot"), "Nope");
    // 失敗したときは何も変えない。
    assert_eq!(count(&p, ugoe_puppet_attached_module_count), 0);

    let name = CString::new("Dress").unwrap();
    assert_eq!(
        unsafe { ugoe_puppet_detach_module(p.0, name.as_ptr(), err()) },
        UGOE_ERROR_DRESS_UP
    );
    assert_eq!(text(last_kind()), "not_attached");
    assert_eq!(field("module"), "Dress");

    let garbage = b"not a module";
    let mut m = ptr::null_mut();
    assert_eq!(
        unsafe { ugoe_module_create(garbage.as_ptr(), garbage.len(), &mut m, err()) },
        UGOE_ERROR_FORMAT
    );
    assert!(m.is_null());
    assert_eq!(text(last_kind()), "invalid_file");

    // 番号の補足の値は 10 進の文字列。本体がアバタープロファイルを持たなければ `actual` は無い (T0157)。
    let mut picky = long_hair_module();
    picky.requires.avatar_level = Some(2);
    let picky = module(picky, 1);
    assert_eq!(
        unsafe { ugoe_puppet_attach_module(p.0, picky.0, err()) },
        UGOE_ERROR_DRESS_UP
    );
    assert_eq!(text(last_kind()), "avatar_level");
    assert_eq!(field("module"), "LongHair");
    assert_eq!(field("required"), "2");
    assert!(last_field(c"actual").is_null());

    // 本体がスロットを持たなければ付けられない。
    let q = from_model(fixtures::single_quad());
    let hair = module(long_hair_module(), 1);
    assert_eq!(
        unsafe { ugoe_puppet_attach_module(q.0, hair.0, err()) },
        UGOE_ERROR_DRESS_UP
    );
    assert!(!text(last_kind()).is_empty());
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
    ok(unsafe { ugoe_puppet_uses_lighting(p.0, &mut uses, err()) });
    assert!(uses);
    let mut lit = false;
    ok(unsafe { ugoe_puppet_drawable_lit(p.0, 0, &mut lit, err()) });
    assert!(lit);

    let mut t = UgoeTexture {
        format: u32::MAX,
        data: ptr::null(),
        len: 0,
    };
    ok(unsafe { ugoe_puppet_normal_map(p.0, 0, &mut t, err()) });
    assert_eq!(t.format, UGOE_TEXTURE_NONE);
    ok(unsafe { ugoe_puppet_normal_map(p.0, 1, &mut t, err()) });
    assert_eq!(t.format, UGOE_TEXTURE_PNG);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(t.data, t.len) },
        png(9).bytes
    );
    assert_eq!(
        unsafe { ugoe_puppet_normal_map(p.0, 2, &mut t, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let q = from_model(fixtures::single_quad());
    ok(unsafe { ugoe_puppet_uses_lighting(q.0, &mut uses, err()) });
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
    ok(unsafe {
        ugoe_lighting_uniform(
            ambient.as_ptr(),
            lights.as_ptr(),
            lights.len(),
            &mut u,
            err(),
        )
    });
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
    ok(unsafe { ugoe_lighting_uniform(ptr::null(), ptr::null(), 0, &mut u, err()) });
    assert_eq!(u.ambient, [1.0, 1.0, 1.0, 0.0]);
    assert_eq!(u.count, 0);

    // 上限を超えた光源は先頭だけを使う。
    let many = vec![lights[0]; UGOE_MAX_LIGHTS + 2];
    ok(unsafe { ugoe_lighting_uniform(ptr::null(), many.as_ptr(), many.len(), &mut u, err()) });
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
            unsafe { ugoe_lighting_uniform(ptr::null(), &light, 1, &mut u, err()) },
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
    ok(unsafe { ugoe_puppet_hit_area_id(p.0, 0, &mut s, err()) });
    assert_eq!(text(s), "Body");

    let body = CString::new("Body").unwrap();
    let head = CString::new("Head").unwrap();
    let (x, y) = (220.0, 100.0);
    let mut hit = true;
    ok(unsafe { ugoe_puppet_hit_test(p.0, body.as_ptr(), x, y, &mut hit, err()) });
    assert!(!hit);
    ok(unsafe { ugoe_puppet_set_parameter(p.0, 0, 1.0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
    ok(unsafe { ugoe_puppet_hit_test(p.0, body.as_ptr(), x, y, &mut hit, err()) });
    assert!(hit);
    assert_eq!(
        unsafe { ugoe_puppet_hit_test(p.0, head.as_ptr(), x, y, &mut hit, err()) },
        UGOE_ERROR_NOT_FOUND
    );

    let mut n = usize::MAX;
    assert_eq!(
        unsafe { ugoe_puppet_hit_areas_at(p.0, x, y, ptr::null_mut(), 0, &mut n, err()) },
        UGOE_ERROR_BUFFER_TOO_SMALL
    );
    assert_eq!(n, 1);
    let mut hits = [u32::MAX; 2];
    ok(unsafe {
        ugoe_puppet_hit_areas_at(p.0, x, y, hits.as_mut_ptr(), hits.len(), &mut n, err())
    });
    assert_eq!((n, hits[0]), (1, 0));
    ok(unsafe { ugoe_puppet_hit_areas_at(p.0, 10.0, 10.0, ptr::null_mut(), 0, &mut n, err()) });
    assert_eq!(n, 0);

    let mut index = u32::MAX;
    ok(unsafe { ugoe_puppet_standard_hit_area(p.0, body.as_ptr(), &mut index, err()) });
    assert_eq!(index, 0);
    assert_eq!(
        unsafe { ugoe_puppet_standard_hit_area(p.0, head.as_ptr(), &mut index, err()) },
        UGOE_ERROR_NOT_FOUND
    );
    ok(unsafe { ugoe_puppet_hit_test_standard(p.0, body.as_ptr(), x, y, &mut hit, err()) });
    assert!(hit);
    ok(unsafe { ugoe_puppet_hit_test_standard(p.0, head.as_ptr(), x, y, &mut hit, err()) });
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
    ok(unsafe { ugoe_blend_shape_index(name.as_ptr(), &mut index, err()) });
    let mut s = ptr::null();
    ok(unsafe { ugoe_blend_shape_name(index, &mut s, err()) });
    assert_eq!(text(s), "eyeBlinkLeft");
    let neutral = CString::new("_neutral").unwrap();
    assert_eq!(
        unsafe { ugoe_blend_shape_index(neutral.as_ptr(), &mut index, err()) },
        UGOE_ERROR_NOT_FOUND
    );
    assert_eq!(
        unsafe { ugoe_blend_shape_name(UGOE_BLEND_SHAPE_COUNT as u32, &mut s, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let mut input = tracking_input();
    ok(unsafe { ugoe_blend_shape_index(name.as_ptr(), &mut index, err()) });
    input.blend_shapes[index as usize] = 0.75;
    input.head_yaw = 10.0;
    ok(unsafe { ugoe_puppet_set_tracking(p.0, &input, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
    assert!((param(&p, standard::EYE_L_OPEN) - 0.25).abs() < 1e-4);
    assert!((param(&p, standard::ANGLE_X) - 10.0).abs() < 1e-4);

    // 入力をやめても最後の値は残る。
    ok(unsafe { ugoe_puppet_set_tracking(p.0, ptr::null(), err()) });
    ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
    assert!((param(&p, standard::EYE_L_OPEN) - 0.25).abs() < 1e-4);

    input.head_yaw = f32::INFINITY;
    assert_eq!(
        unsafe { ugoe_puppet_set_tracking(p.0, &input, err()) },
        UGOE_ERROR_INVALID_ARGUMENT
    );

    let mut n = 0;
    assert_eq!(
        unsafe { ugoe_puppet_tracking_targets(p.0, ptr::null_mut(), 0, &mut n, err()) },
        UGOE_ERROR_BUFFER_TOO_SMALL
    );
    let mut targets = vec![0u32; n];
    ok(unsafe { ugoe_puppet_tracking_targets(p.0, targets.as_mut_ptr(), n, &mut n, err()) });
    let angle_x = CString::new(standard::ANGLE_X).unwrap();
    ok(unsafe { ugoe_puppet_parameter_index(p.0, angle_x.as_ptr(), &mut index, err()) });
    assert!(targets.contains(&index));
    assert!(targets.is_sorted());
}

#[test]
fn stages_can_be_turned_off() {
    let p = from_model(fixtures::avatar_profile());
    let eye = standard::EYE_L_OPEN;
    ok(unsafe { ugoe_puppet_set_stage_enabled(p.0, UGOE_STAGE_EYE_BLINK, false, err()) });
    ok(unsafe { ugoe_puppet_set_stage_enabled(p.0, UGOE_STAGE_BREATH, false, err()) });
    let before = param(&p, eye);
    for _ in 0..600 {
        ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
        assert_eq!(param(&p, eye), before);
    }
    assert_eq!(
        unsafe { ugoe_puppet_set_stage_enabled(p.0, 99, true, err()) },
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
        ugoe_puppet_set_parameters_by_id(
            p.0,
            ptrs.as_ptr(),
            values.as_ptr(),
            3,
            &mut applied,
            err(),
        )
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
                err(),
            )
        },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(param(&p, standard::ANGLE_X), 12.0);
    ok(unsafe {
        ugoe_puppet_set_parameters_by_id(p.0, ptr::null(), ptr::null(), 0, ptr::null_mut(), err())
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
    ok(unsafe { ugoe_puppet_set_expression(p.0, 0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0, err()) });
    assert!((param(&p, "X") - 0.5).abs() < 1e-5);
    ok(unsafe { ugoe_puppet_stop_expressions(p.0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0, err()) });
    assert!(param(&p, "X").abs() < 1e-5);
}

#[test]
fn load_warnings_have_codes_and_fields() {
    let p = from_model(fixtures::single_quad());
    assert_eq!(count(&p, ugoe_puppet_load_warning_count), 0);

    // 新しい版を模したバンドル (知らない任意の機能と未知の値。assets/compat/README.md)。
    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoe");
    let mut raw = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut raw, err()) });
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
        ok(unsafe { ugoe_puppet_load_warning(p.0, i, &mut w, err()) });
        let code = text(w.code);
        assert!(!text(w.message).is_empty());
        let key = if code == "unknown_optional_feature" {
            &feature
        } else {
            &path
        };
        ok(unsafe { ugoe_puppet_load_warning_field(p.0, i, key.as_ptr(), &mut value, err()) });
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
        unsafe { ugoe_puppet_load_warning_field(p.0, 0, other.as_ptr(), &mut value, err()) },
        UGOE_ERROR_NOT_FOUND
    );
    assert_eq!(
        unsafe { ugoe_puppet_load_warning(p.0, n, &mut w, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );

    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoemod");
    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m, err()) });
    let future = ModuleHandle(m);
    let mut n = 0;
    ok(unsafe { ugoe_module_warning_count(future.0, &mut n, err()) });
    assert!(n > 0);
    ok(unsafe { ugoe_module_warning(future.0, 0, &mut w, err()) });
    assert!(!text(w.code).is_empty());

    let hair = module(long_hair_module(), 1);
    let mut n = u32::MAX;
    ok(unsafe { ugoe_module_warning_count(hair.0, &mut n, err()) });
    assert_eq!(n, 0);
}

/// 警告 1 つ: `(code, message, 補足の値)`。
type WarningRow = (String, String, Vec<(String, String)>);

/// 警告を [`WarningRow`] の一覧にする。
fn warnings(
    n: u32,
    warning: impl Fn(u32, *mut UgoeWarning) -> UgoeResult,
    field: impl Fn(u32, *const c_char, *mut *const c_char) -> UgoeResult,
) -> Vec<WarningRow> {
    (0..n)
        .map(|i| {
            let mut w = UgoeWarning {
                code: ptr::null(),
                message: ptr::null(),
            };
            ok(warning(i, &mut w));
            let fields = ["texture", "feature", "path"]
                .into_iter()
                .filter_map(|key| {
                    let key = CString::new(key).unwrap();
                    let mut value = ptr::null();
                    (field(i, key.as_ptr(), &mut value) == UGOE_OK)
                        .then(|| (key.into_string().unwrap(), text(value)))
                })
                .collect();
            (text(w.code), text(w.message), fields)
        })
        .collect()
}

/// 読み込みの報告の警告の、T0409 の C ABI が出していた形。
fn expected_read_warnings(report: &ugoe::format::ReadReport) -> Vec<WarningRow> {
    use ugoe::format::ReadWarning;
    report
        .warnings
        .iter()
        .map(|w| {
            let (code, key, value) = match w {
                ReadWarning::UnknownOptionalFeature(id) => {
                    ("unknown_optional_feature", "feature", id)
                }
                ReadWarning::UnknownValue(path) => ("unknown_value", "path", path),
            };
            (
                code.to_owned(),
                w.to_string(),
                vec![(key.to_owned(), value.clone())],
            )
        })
        .collect()
}

/// SDK に種類の対応を移しても (T0412)、C ABI の警告は T0409 と同じ code・文・補足の値のまま。
#[test]
fn load_warnings_are_unchanged_by_the_shared_codes() {
    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoe");
    let (_, report) = ugoe::format::read_bundle_with_report(bytes).unwrap();
    let mut raw = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut raw, err()) });
    let p = Handle(raw);
    let n = count(&p, ugoe_puppet_load_warning_count);
    let actual = warnings(
        n,
        |i, out| unsafe { ugoe_puppet_load_warning(p.0, i, out, err()) },
        |i, key, out| unsafe { ugoe_puppet_load_warning_field(p.0, i, key, out, err()) },
    );
    let expected = expected_read_warnings(&report);
    assert!(!expected.is_empty());
    assert_eq!(actual, expected);

    let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoemod");
    let (_, report) = ugoe::format::module_file::read_module_with_report(bytes).unwrap();
    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(bytes.as_ptr(), bytes.len(), &mut m, err()) });
    let future = ModuleHandle(m);
    let mut n = 0;
    ok(unsafe { ugoe_module_warning_count(future.0, &mut n, err()) });
    let actual = warnings(
        n,
        |i, out| unsafe { ugoe_module_warning(future.0, i, out, err()) },
        |i, key, out| unsafe { ugoe_module_warning_field(future.0, i, key, out, err()) },
    );
    assert_eq!(actual, expected_read_warnings(&report));
}

const MOTION: &str = r#"{
    "name": "nod",
    "duration": 1.0,
    "looping": false,
    "fade_in": 0.0,
    "fade_out": 0.0,
    "curves": [{
        "target": { "kind": "parameter", "id": "AngleX" },
        "segments": [
            { "type": "start", "p": { "t": 0.0, "v": 0.0 } },
            { "type": "linear", "to": { "t": 1.0, "v": 30.0 } }
        ]
    }],
    "events": [{ "t": 0.5, "value": "half" }, { "t": 0.75, "value": "late" }]
}"#;

fn motion_events(p: &Handle) -> Vec<(f32, String)> {
    (0..count(p, ugoe_puppet_motion_event_count))
        .map(|i| {
            let mut e = UgoeMotionEvent {
                time: f32::NAN,
                value: ptr::null(),
            };
            ok(unsafe { ugoe_puppet_motion_event(p.0, i, &mut e, err()) });
            (e.time, text(e.value))
        })
        .collect()
}

/// モーションのイベント (T0412) は `ugoe::Puppet::motion_events` と同じものを、発火した update の後だけ返す。
#[test]
fn motion_events_follow_the_puppet() {
    let motion = ugoe::format::motion_from_json(MOTION).unwrap();
    let model = fixtures::pose_and_effects();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    bundle.motions = vec![motion.clone()];
    let p = from_bundle(&bundle);
    let mut rust = Puppet::new(fixtures::pose_and_effects()).unwrap();
    assert!(motion_events(&p).is_empty());

    ok(unsafe { ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_FORCE, ptr::null_mut(), err()) });
    rust.start_motion(motion, ugoe::motion::Priority::Force);
    for (dt, expected) in [
        (0.25, vec![]),
        (
            0.6,
            vec![(0.5, "half".to_owned()), (0.75, "late".to_owned())],
        ),
        (0.1, vec![]),
    ] {
        ok(unsafe { ugoe_puppet_update(p.0, dt, err()) });
        rust.update(dt);
        let actual = motion_events(&p);
        assert_eq!(actual, expected);
        let from_rust: Vec<_> = rust
            .motion_events()
            .iter()
            .map(|e| (e.t, e.value.clone()))
            .collect();
        assert_eq!(actual, from_rust);
    }
    let mut e = UgoeMotionEvent {
        time: 0.0,
        value: ptr::null(),
    };
    assert_eq!(
        unsafe { ugoe_puppet_motion_event(p.0, 0, &mut e, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );
}

fn motion_event_sources(p: &Handle) -> Vec<(String, u64)> {
    (0..count(p, ugoe_puppet_motion_event_count))
        .map(|i| {
            let mut s = UgoeMotionEventSource {
                motion: ptr::null(),
                number: u64::MAX,
            };
            ok(unsafe { ugoe_puppet_motion_event_source(p.0, i, &mut s, err()) });
            (text(s.motion), s.number)
        })
        .collect()
}

/// イベントを発火したモーション (T0462) は `ugoe::Puppet::motion_event_sources` と同じ。始め直してフェードアウト中の
/// モーションのイベントは、前に始めた番号を持つ。
#[test]
fn motion_event_sources_name_the_motion_that_fired() {
    // 始め直すと前の再生は 1 秒かけてフェードアウトし、その間も時間が進んでイベントを発火する。
    let motion =
        ugoe::format::motion_from_json(&MOTION.replace(r#""fade_out": 0.0"#, r#""fade_out": 1.0"#))
            .unwrap();
    let model = fixtures::pose_and_effects();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    bundle.motions = vec![motion.clone()];
    let p = from_bundle(&bundle);
    let mut rust = Puppet::new(fixtures::pose_and_effects()).unwrap();
    let start = |rust: &mut Puppet| {
        ok(unsafe {
            ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_FORCE, ptr::null_mut(), err())
        });
        rust.start_motion(motion.clone(), ugoe::motion::Priority::Force);
    };
    start(&mut rust);
    let nod = |number| ("nod".to_owned(), number);
    for (restart, dt, expected) in [
        (false, 0.25, vec![]),
        (true, 0.3, vec![("half", nod(1))]),
        (false, 0.3, vec![("late", nod(1)), ("half", nod(2))]),
    ] {
        if restart {
            start(&mut rust);
        }
        ok(unsafe { ugoe_puppet_update(p.0, dt, err()) });
        rust.update(dt);
        let values: Vec<_> = motion_events(&p).into_iter().map(|(_, v)| v).collect();
        let sources = motion_event_sources(&p);
        let actual: Vec<_> = values
            .iter()
            .map(String::as_str)
            .zip(sources.iter().cloned())
            .collect();
        assert_eq!(actual, expected);
        let from_rust: Vec<_> = rust
            .motion_event_sources()
            .iter()
            .map(|s| (s.name.clone(), s.number))
            .collect();
        assert_eq!(sources, from_rust);
    }
    let mut s = UgoeMotionEventSource {
        motion: ptr::null(),
        number: 0,
    };
    assert_eq!(
        unsafe { ugoe_puppet_motion_event_source(p.0, 2, &mut s, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );
    assert_eq!(
        unsafe { ugoe_puppet_motion_event_source(p.0, 0, ptr::null_mut(), err()) },
        UGOE_ERROR_NULL_POINTER
    );
}

fn physics_json(p: &Handle) -> String {
    let mut len = usize::MAX;
    let code = unsafe { ugoe_puppet_physics_json(p.0, ptr::null_mut(), 0, &mut len, err()) };
    if len == 0 {
        ok(code);
        return String::new();
    }
    assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    let mut buf = vec![0u8; len];
    ok(unsafe { ugoe_puppet_physics_json(p.0, buf.as_mut_ptr(), buf.len(), &mut len, err()) });
    assert_eq!(len, buf.len());
    String::from_utf8(buf).unwrap()
}

fn set_physics(p: &Handle, json: &str) -> UgoeResult {
    unsafe { ugoe_puppet_set_physics_json(p.0, json.as_ptr(), json.len(), err()) }
}

/// 直前の失敗の種類と、番号で列挙した補足の値 (T0464)。
fn last_kind_and_fields() -> (String, Vec<(String, String)>) {
    let fields = (0..last_field_count())
        .map(|i| (text(last_field_key_at(i)), text(last_field_value_at(i))))
        .collect();
    (text(last_kind()), fields)
}

/// 物理の設定の検査の失敗の種類と補足の値 (着せ替えの `invalid_physics` と同じ。T0469)。
fn invalid_physics(errors: usize, codes: &str) -> (String, Vec<(String, String)>) {
    (
        "invalid_physics".into(),
        vec![
            ("errors".into(), errors.to_string()),
            ("error_codes".into(), codes.into()),
        ],
    )
}

/// 物理の設定の差し替え (T0412) は `ugoe::Puppet::set_physics` と同じく検査し、失敗したら何も変えない。
#[test]
fn physics_settings_can_be_replaced() {
    let p = from_model(fixtures::physics_hair());
    assert_eq!(physics_json(&p), "");

    let settings = fixtures::physics_hair_settings();
    ok(set_physics(
        &p,
        &ugoe::format::physics_to_json(&settings).unwrap(),
    ));
    assert_eq!(
        ugoe::format::physics_from_json(&physics_json(&p)).unwrap(),
        settings
    );

    // Rust の Puppet と同じように揺れる。
    let mut rust = Puppet::new(fixtures::physics_hair())
        .unwrap()
        .with_physics(settings.clone());
    let angle = CString::new(standard::ANGLE_X).unwrap();
    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, angle.as_ptr(), &mut index, err()) });
    for frame in 0..8 {
        let value = (frame as f32 * 0.7).sin() * 30.0;
        ok(unsafe { ugoe_puppet_set_parameter(p.0, index, value, err()) });
        ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
        let params = rust.instance_mut().parameters_mut();
        let i = params.index_of(standard::ANGLE_X).unwrap();
        params.set(i, value);
        rust.update(DT);
    }
    let front = {
        let params = rust.instance().parameters();
        params.get(params.index_of(standard::HAIR_FRONT).unwrap())
    };
    assert!(front.abs() > 1e-3);
    assert!((param(&p, standard::HAIR_FRONT) - front).abs() < 1e-5);
    ok(unsafe { ugoe_puppet_reset_physics(p.0, err()) });

    // 検査を通らない設定と読めない JSON は、何も変えずに失敗する。
    let mut bad = settings.clone();
    bad.rigs[0].inputs[0].parameter = "Missing".into();
    assert_eq!(
        set_physics(&p, &ugoe::format::physics_to_json(&bad).unwrap()),
        UGOE_ERROR_INVALID_MODEL
    );
    // 文は検証エラーの列の共通の形 (T0015。`docs/spec/model.md#検証エラーの文`)。
    let errors = ugoe::model::validate::validate_physics(&bad, &fixtures::physics_hair());
    assert_eq!(
        text(last_message()),
        ugoe::model::validate::describe_errors("physics settings", &errors)
    );
    assert!(text(last_message()).contains("Missing"));
    assert_eq!(
        last_kind_and_fields(),
        invalid_physics(errors.len(), "unknown_reference")
    );
    assert_eq!(set_physics(&p, "{"), UGOE_ERROR_FORMAT);
    assert_eq!(
        ugoe::format::physics_from_json(&physics_json(&p)).unwrap(),
        settings
    );

    // NULL で外す。
    ok(unsafe { ugoe_puppet_set_physics_json(p.0, ptr::null(), 0, err()) });
    assert_eq!(physics_json(&p), "");
}

const TURN: &str = r#"{
    "name": "turn",
    "duration": 2.0,
    "looping": false,
    "fade_in": 0.0,
    "fade_out": 0.0,
    "curves": [
        {
            "target": { "kind": "parameter", "id": "AngleX" },
            "segments": [{ "type": "start", "p": { "t": 0.0, "v": -20.0 } }]
        },
        {
            "target": { "kind": "parameter", "id": "EyeLOpen" },
            "segments": [{ "type": "start", "p": { "t": 0.0, "v": 1.0 } }]
        }
    ]
}"#;

/// トラッキング中のモーションの優先 (T0722) は `ugoe::Puppet::set_motion_over_tracking` と
/// `set_tracking_kept` と同じ結果になる。
#[test]
fn motion_over_tracking_follows_the_puppet() {
    let mut model = fixtures::avatar_profile();
    model.effects = ugoe::model::effects::Effects::default();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    bundle.motions = vec![ugoe::format::motion_from_json(TURN).unwrap()];

    let eye_blink = CString::new("eyeBlinkLeft").unwrap();
    let mut blink = 0;
    ok(unsafe { ugoe_blend_shape_index(eye_blink.as_ptr(), &mut blink, err()) });
    let mut input = tracking_input();
    input.blend_shapes[blink as usize] = 0.75;
    input.head_yaw = 15.0;

    // (優先, トラッキングのままにするパラメータ) → (AngleX, EyeLOpen)
    let run = |over: bool, kept: &[&str]| {
        let p = from_bundle(&bundle);
        let mut enabled = true;
        ok(unsafe { ugoe_puppet_motion_over_tracking(p.0, &mut enabled, err()) });
        assert!(!enabled, "off by default");
        ok(unsafe { ugoe_puppet_set_motion_over_tracking(p.0, over, err()) });
        ok(unsafe { ugoe_puppet_motion_over_tracking(p.0, &mut enabled, err()) });
        assert_eq!(enabled, over);
        let indices: Vec<u32> = kept
            .iter()
            .map(|id| {
                let id = CString::new(*id).unwrap();
                let mut index = 0;
                ok(unsafe { ugoe_puppet_parameter_index(p.0, id.as_ptr(), &mut index, err()) });
                index
            })
            .collect();
        ok(unsafe { ugoe_puppet_set_tracking_kept(p.0, indices.as_ptr(), indices.len(), err()) });
        ok(unsafe { ugoe_puppet_set_tracking(p.0, &input, err()) });
        ok(unsafe {
            ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_NORMAL, ptr::null_mut(), err())
        });
        for _ in 0..3 {
            ok(unsafe { ugoe_puppet_update(p.0, 0.1, err()) });
        }
        (
            param(&p, standard::ANGLE_X),
            param(&p, standard::EYE_L_OPEN),
        )
    };
    let close = |(a, e): (f32, f32), (ea, ee): (f32, f32)| {
        assert!(
            (a - ea).abs() < 1e-4 && (e - ee).abs() < 1e-4,
            "({a}, {e}) != ({ea}, {ee})"
        );
    };
    close(run(false, &[]), (15.0, 0.25));
    close(run(true, &[]), (-20.0, 1.0));
    close(run(true, &[standard::EYE_L_OPEN]), (-20.0, 0.25));

    // 範囲外の番号は何も変えない。NULL は個数が 0 のときだけ許す。
    let p = from_bundle(&bundle);
    let bad = [u32::MAX];
    assert_eq!(
        unsafe { ugoe_puppet_set_tracking_kept(p.0, bad.as_ptr(), 1, err()) },
        UGOE_ERROR_OUT_OF_RANGE
    );
    assert_eq!(
        unsafe { ugoe_puppet_set_tracking_kept(p.0, ptr::null(), 1, err()) },
        UGOE_ERROR_NULL_POINTER
    );
    ok(unsafe { ugoe_puppet_set_tracking_kept(p.0, ptr::null(), 0, err()) });
}

fn base_physics_json(p: &Handle) -> String {
    let mut len = usize::MAX;
    let code = unsafe { ugoe_puppet_base_physics_json(p.0, ptr::null_mut(), 0, &mut len, err()) };
    if len == 0 {
        ok(code);
        return String::new();
    }
    assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    let mut buf = vec![0u8; len];
    ok(unsafe { ugoe_puppet_base_physics_json(p.0, buf.as_mut_ptr(), buf.len(), &mut len, err()) });
    String::from_utf8(buf).unwrap()
}

fn set_base_physics(p: &Handle, json: &str) -> UgoeResult {
    unsafe { ugoe_puppet_set_base_physics_json(p.0, json.as_ptr(), json.len(), err()) }
}

/// 本体の物理の設定の差し替え (T0416) は `ugoe::Puppet::set_base_physics` と同じで、付け外しの後も保たれる。
#[test]
fn base_physics_survives_dress_up() {
    let p = from_model(dress_up_base());
    let mut rust = Puppet::new(dress_up_base()).unwrap();
    let hair = module(long_hair_module(), 1);
    ok(unsafe { ugoe_puppet_attach_module(p.0, hair.0, err()) });
    rust.attach_module(std::sync::Arc::new(long_hair_module()))
        .unwrap();
    assert_eq!(base_physics_json(&p), "");

    let windy = ugoe::model::physics::PhysicsSettings {
        gravity: ugoe::core::Vec2::new(0.0, 1.0),
        wind: ugoe::core::Vec2::new(0.3, 0.0),
        fps: ugoe::physics::DEFAULT_FPS,
        rigs: Vec::new(),
        colliders: Vec::new(),
    };
    ok(set_base_physics(
        &p,
        &ugoe::format::physics_to_json(&windy).unwrap(),
    ));
    rust.set_base_physics(Some(windy.clone())).unwrap();
    let parse = |json: &str| ugoe::format::physics_from_json(json).unwrap();
    assert_eq!(parse(&base_physics_json(&p)), windy);
    assert_eq!(
        &parse(&physics_json(&p)),
        rust.physics().unwrap().settings()
    );

    let dress = module(dress_module(), 2);
    ok(unsafe { ugoe_puppet_attach_module(p.0, dress.0, err()) });
    let composed = parse(&physics_json(&p));
    assert_eq!(composed.wind, windy.wind);
    assert_eq!(composed.rigs[0].id.as_str(), "LongHair/Sway");

    // 検査を通らない設定と読めない JSON は、何も変えずに失敗する。
    let mut bad = windy.clone();
    bad.colliders.push(ugoe::model::physics::PhysicsCollider {
        id: "Chest".into(),
        name: "Chest".into(),
        attach: Some("Missing".into()),
        shape: ugoe::model::physics::ColliderShape::Circle {
            center: ugoe::core::Vec2::ZERO,
            radius: 1.0,
        },
    });
    assert_eq!(
        set_base_physics(&p, &ugoe::format::physics_to_json(&bad).unwrap()),
        UGOE_ERROR_INVALID_MODEL
    );
    assert_eq!(
        last_kind_and_fields(),
        invalid_physics(1, "unknown_reference")
    );
    assert_eq!(set_base_physics(&p, "{"), UGOE_ERROR_FORMAT);
    assert_eq!(parse(&base_physics_json(&p)), windy);
    assert_eq!(parse(&physics_json(&p)), composed);

    // NULL で本体の物理を外す。モジュールのリグは既定の設定に加わる。
    ok(unsafe { ugoe_puppet_set_base_physics_json(p.0, ptr::null(), 0, err()) });
    assert_eq!(base_physics_json(&p), "");
    let composed = parse(&physics_json(&p));
    assert_eq!(composed.wind, ugoe::core::Vec2::ZERO);
    assert_eq!(composed.rigs.len(), 1);
}

/// 受け手の口 (T0133): 同じ値と `dt` の列を `ugoe_puppet_follow` で与えると、`ugoe::Puppet::follow` と同じ形になる。
/// 数の違い・NaN・負の `dt` は何も変えない。
#[test]
fn follow_matches_the_puppet() {
    let model = fixtures::dynamic_hair();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    bundle.physics = Some(fixtures::dynamic_hair_settings());
    let p = from_bundle(&bundle);
    let mut native = Puppet::new(model)
        .unwrap()
        .with_physics(fixtures::dynamic_hair_settings());
    let mut count = 0;
    ok(unsafe { ugoe_puppet_parameter_count(p.0, &mut count, err()) });
    assert_eq!(count, 2);
    let mut drawables = 0;
    ok(unsafe { ugoe_puppet_drawable_count(p.0, &mut drawables, err()) });
    assert!(drawables > 0);

    for frame in 0..60 {
        let t = frame as f32 * DT;
        let values = [30.0 * (t * 6.0).sin(), 10.0 * (t * 4.0).cos()];
        ok(unsafe {
            ugoe_puppet_follow(
                p.0,
                values.as_ptr(),
                values.len(),
                -1.0,
                0.0,
                ptr::null(),
                0,
                DT,
                err(),
            )
        });
        native.follow(&values, None, &[], DT).unwrap();
        for (i, drawable) in native.instance().drawables().iter().enumerate() {
            let positions = positions(p.0, i as u32);
            let expected: Vec<f32> = drawable.positions.iter().flat_map(|v| [v.x, v.y]).collect();
            assert_eq!(positions, expected, "frame {frame}, drawable {i}");
        }
    }

    // 受け付けない値は何も変えない。
    let before = (param(&p, "AngleX"), param(&p, "AngleZ"));
    let one = [0.0f32];
    let code = unsafe {
        ugoe_puppet_follow(
            p.0,
            one.as_ptr(),
            one.len(),
            -1.0,
            0.0,
            ptr::null(),
            0,
            DT,
            err(),
        )
    };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
    assert!(text(last_message()).contains("expected 2"));
    let nan = [0.0, f32::NAN];
    let code = unsafe { ugoe_puppet_set_base_values(p.0, nan.as_ptr(), nan.len(), err()) };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
    let values = [1.0f32, 2.0];
    let code = unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            -1.0,
            0.0,
            ptr::null(),
            0,
            -1.0,
            err(),
        )
    };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
    let code = unsafe { ugoe_puppet_set_base_values(p.0, ptr::null(), 2, err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    assert_eq!((param(&p, "AngleX"), param(&p, "AngleZ")), before);

    // 基準値として書くと、次の update でもその値のまま (段が何も重ならないモデル)。
    ok(unsafe { ugoe_puppet_set_base_values(p.0, values.as_ptr(), values.len(), err()) });
    ok(unsafe { ugoe_puppet_update(p.0, DT, err()) });
    assert_eq!((param(&p, "AngleX"), param(&p, "AngleZ")), (1.0, 2.0));
}

/// 受け手のリップシンク (T0134): `ugoe_puppet_follow` の音量と強さは `ugoe::Puppet::follow` と同じに当たり、負の音量なら当てない。
#[test]
fn follow_applies_the_lip_sync_like_the_puppet() {
    let model = fixtures::pose_and_effects();
    let p = from_model(model.clone());
    let mut native = Puppet::new(model).unwrap();
    let values: Vec<f32> = native.instance().parameters().as_slice().to_vec();
    let input = ugoe::LipSyncInput {
        level: 0.8,
        strength: 0.5,
    };
    ok(unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            0.8,
            0.5,
            ptr::null(),
            0,
            DT,
            err(),
        )
    });
    native.follow(&values, Some(input), &[], DT).unwrap();
    let mouth = param(&p, standard::MOUTH_OPEN_Y);
    assert!((mouth - 0.4).abs() < 1e-6, "{mouth}");
    let id = native
        .instance()
        .parameters()
        .index_of(standard::MOUTH_OPEN_Y)
        .unwrap();
    assert_eq!(native.instance().parameters().get(id), mouth);

    ok(unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            -1.0,
            0.5,
            ptr::null(),
            0,
            DT,
            err(),
        )
    });
    assert_eq!(param(&p, standard::MOUTH_OPEN_Y), 0.0);
    let code = unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            0.5,
            f32::NAN,
            ptr::null(),
            0,
            DT,
            err(),
        )
    };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);

    // 無限大の強さは、音量と今の値が等しいと 0 × ∞ で NaN を書くので受け付けない (何も変えない)。
    let before = param(&p, standard::MOUTH_OPEN_Y);
    for strength in [f32::INFINITY, f32::NEG_INFINITY] {
        let code = unsafe {
            ugoe_puppet_follow(
                p.0,
                values.as_ptr(),
                values.len(),
                before,
                strength,
                ptr::null(),
                0,
                DT,
                err(),
            )
        };
        assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT, "{strength}");
        assert_eq!(param(&p, standard::MOUTH_OPEN_Y), before);
    }
    // 当てないとき (音量が負) は強さを見ない。
    ok(unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            -1.0,
            f32::INFINITY,
            ptr::null(),
            0,
            DT,
            err(),
        )
    });
}

/// 受け手のパーツの不透明度 (T0135): `ugoe_puppet_follow` のパーツの不透明度のカーブは `ugoe::Puppet::follow` と同じに当たる。
/// NaN は何も変えず、数が 0 でなければ NULL を受け付けない。
#[test]
fn follow_applies_the_part_opacities_like_the_puppet() {
    let model = fixtures::pose_and_effects();
    let p = from_model(model.clone());
    let mut native = Puppet::new(model).unwrap();
    let values: Vec<f32> = native.instance().parameters().as_slice().to_vec();
    let part = |id: &str| native.instance().compiled().part_index(id).unwrap() as u32;
    let opacities = [
        UgoePartOpacityInput {
            part: part("ArmB"),
            value: 1.0,
            weight: 0.8,
        },
        UgoePartOpacityInput {
            part: part("Root"),
            value: 0.25,
            weight: 0.5,
        },
    ];
    let inputs: Vec<ugoe::PartOpacityInput> = opacities.iter().map(|&o| o.into()).collect();
    for _ in 0..3 {
        ok(unsafe {
            ugoe_puppet_follow(
                p.0,
                values.as_ptr(),
                values.len(),
                -1.0,
                0.0,
                opacities.as_ptr(),
                opacities.len(),
                0.1,
                err(),
            )
        });
        native.follow(&values, None, &inputs, 0.1).unwrap();
        assert_eq!(
            states(p.0)[0].opacity,
            native.instance().drawables()[0].opacity
        );
    }
    assert!((native.instance().drawables()[0].opacity - 0.625).abs() < 1e-6);

    let nan = [UgoePartOpacityInput {
        part: 0,
        value: f32::NAN,
        weight: 1.0,
    }];
    let code = unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            -1.0,
            0.0,
            nan.as_ptr(),
            1,
            DT,
            err(),
        )
    };
    assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
    // 無限大の値と重みも受け付けない。
    for (value, weight) in [
        (f32::INFINITY, 1.0),
        (1.0, f32::INFINITY),
        (f32::NEG_INFINITY, 0.5),
    ] {
        let infinite = [UgoePartOpacityInput {
            part: 0,
            value,
            weight,
        }];
        let code = unsafe {
            ugoe_puppet_follow(
                p.0,
                values.as_ptr(),
                values.len(),
                -1.0,
                0.0,
                infinite.as_ptr(),
                1,
                DT,
                err(),
            )
        };
        assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT, "{value} {weight}");
    }
    let code = unsafe {
        ugoe_puppet_follow(
            p.0,
            values.as_ptr(),
            values.len(),
            -1.0,
            0.0,
            ptr::null(),
            1,
            DT,
            err(),
        )
    };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
}

fn values_before_physics(p: &Handle) -> Vec<f32> {
    let mut len = usize::MAX;
    let code =
        unsafe { ugoe_puppet_values_before_physics(p.0, ptr::null_mut(), 0, &mut len, err()) };
    assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    let mut values = vec![f32::NAN; len];
    let mut written = usize::MAX;
    ok(unsafe {
        ugoe_puppet_values_before_physics(
            p.0,
            values.as_mut_ptr(),
            values.len(),
            &mut written,
            err(),
        )
    });
    assert_eq!(written, len);
    values
}

fn lip_sync_applied(p: &Handle) -> (f32, f32) {
    let (mut level, mut strength) = (f32::NAN, f32::NAN);
    ok(unsafe { ugoe_puppet_lip_sync_applied(p.0, &mut level, &mut strength, err()) });
    (level, strength)
}

fn part_opacities_applied(p: &Handle) -> Vec<UgoePartOpacityInput> {
    let mut len = usize::MAX;
    let code =
        unsafe { ugoe_puppet_part_opacities_applied(p.0, ptr::null_mut(), 0, &mut len, err()) };
    if len == 0 {
        ok(code);
        return Vec::new();
    }
    assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    let mut parts = vec![UgoePartOpacityInput::default(); len];
    let mut written = usize::MAX;
    ok(unsafe {
        ugoe_puppet_part_opacities_applied(
            p.0,
            parts.as_mut_ptr(),
            parts.len(),
            &mut written,
            err(),
        )
    });
    assert_eq!(written, len);
    parts
}

/// 指揮役の口 (T0454): 物理の段に入る直前の値・当てたリップシンク・パーツの不透明度のカーブは `ugoe::Puppet` の
/// `values_before_physics`・`lip_sync_applied`・`part_opacities_applied` と同じで、それを `ugoe_puppet_follow` に
/// そのまま渡した受け手は、物理を含めて 1 つの `Puppet` と同じ形になる。
#[test]
fn conductor_values_match_the_puppet_and_drive_a_follower() {
    use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
    let p = |t: f32, v: f32| Point { t, v };
    let motion = Motion {
        name: "switch".into(),
        duration: 1.0,
        looping: false,
        fade_in: 0.2,
        fade_out: 0.2,
        curves: vec![
            Curve {
                target: CurveTarget::PartOpacity("ArmB".into()),
                segments: vec![
                    Segment::Start { p: p(0.0, 0.0) },
                    Segment::Stepped { to: p(0.5, 1.0) },
                ],
                fade_in: None,
                fade_out: None,
            },
            Curve {
                target: CurveTarget::Parameter(standard::ANGLE_X.into()),
                segments: vec![
                    Segment::Start { p: p(0.0, 0.0) },
                    Segment::Linear { to: p(1.0, 30.0) },
                ],
                fade_in: None,
                fade_out: None,
            },
        ],
        events: vec![],
    };
    let model = fixtures::pose_and_effects();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model.textures.iter().map(|_| png(0)).collect();
    bundle.motions = vec![motion.clone()];
    let conductor = from_bundle(&bundle);
    let follower = from_bundle(&bundle);
    let mut native = Puppet::new(model).unwrap();

    // update の前は作ったときの値で、リップシンクもカーブも無い。
    assert_eq!(
        values_before_physics(&conductor),
        native.values_before_physics()
    );
    assert_eq!(lip_sync_applied(&conductor), (-1.0, 1.0));
    assert!(part_opacities_applied(&conductor).is_empty());

    ok(unsafe {
        ugoe_puppet_start_motion(conductor.0, 0, UGOE_PRIORITY_NORMAL, ptr::null_mut(), err())
    });
    native.start_motion(motion, ugoe::motion::Priority::Normal);
    let mut saw_curves = false;
    for frame in 0..80 {
        let level = (frame % 5) as f32 / 4.0;
        ok(unsafe { ugoe_puppet_set_lip_sync_level(conductor.0, level, err()) });
        native.set_lip_sync_level(Some(level));
        ok(unsafe { ugoe_puppet_update(conductor.0, DT, err()) });
        native.update(DT);

        let values = values_before_physics(&conductor);
        assert_eq!(values, native.values_before_physics(), "frame {frame}");
        let (level, strength) = lip_sync_applied(&conductor);
        let expected = native.lip_sync_applied().unwrap();
        assert_eq!((level, strength), (expected.level, expected.strength));
        let parts = part_opacities_applied(&conductor);
        let expected: Vec<UgoePartOpacityInput> = native
            .part_opacities_applied()
            .iter()
            .map(|&p| p.into())
            .collect();
        assert_eq!(parts, expected, "frame {frame}");
        saw_curves |= !parts.is_empty();

        ok(unsafe {
            ugoe_puppet_follow(
                follower.0,
                values.as_ptr(),
                values.len(),
                level,
                strength,
                parts.as_ptr(),
                parts.len(),
                DT,
                err(),
            )
        });
        let (a, b) = (states(conductor.0), states(follower.0));
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.iter().zip(&b).enumerate() {
            assert_eq!(a.opacity, b.opacity, "frame {frame}, drawable {i}");
        }
        assert_eq!(
            param(&follower, standard::MOUTH_OPEN_Y),
            param(&conductor, standard::MOUTH_OPEN_Y)
        );
        // 受け手も同じ値を持つ (さらに別の受け手へ中継できる)。
        assert_eq!(values_before_physics(&follower), values);
        assert_eq!(lip_sync_applied(&follower), (level, strength));
        assert_eq!(part_opacities_applied(&follower), parts);
    }
    assert!(saw_curves);

    // リップシンクの段を止めると当てていない。
    ok(unsafe { ugoe_puppet_set_stage_enabled(conductor.0, UGOE_STAGE_LIP_SYNC, false, err()) });
    ok(unsafe { ugoe_puppet_update(conductor.0, DT, err()) });
    assert_eq!(lip_sync_applied(&conductor), (-1.0, 1.0));

    // NULL は受け付けない。
    let mut value = 0.0f32;
    assert_eq!(
        unsafe { ugoe_puppet_lip_sync_applied(conductor.0, &mut value, ptr::null_mut(), err()) },
        UGOE_ERROR_NULL_POINTER
    );
    assert_eq!(
        unsafe { ugoe_puppet_lip_sync_applied(conductor.0, ptr::null_mut(), &mut value, err()) },
        UGOE_ERROR_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            ugoe_puppet_values_before_physics(conductor.0, &mut value, 1, ptr::null_mut(), err())
        },
        UGOE_ERROR_NULL_POINTER
    );
    let mut len = 0;
    assert_eq!(
        unsafe {
            ugoe_puppet_values_before_physics(conductor.0, ptr::null_mut(), 100, &mut len, err())
        },
        UGOE_ERROR_NULL_POINTER
    );
}
