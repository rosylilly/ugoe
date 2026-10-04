//! C ABI を Rust から呼んで、`ugoe::Puppet` を直接使ったときと同じ結果になることを確かめる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作って破棄していない `UgoePuppet`、
// 生きている Rust の値を指すポインタ、または意図的に NULL を渡す。借りたポインタは次の update や破棄の前に読む。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use ugoe::Puppet;
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::fixtures;
use ugoe_ffi::*;

/// 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
const FAKE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn single_quad_bundle() -> Vec<u8> {
    let model = fixtures::single_quad();
    let mut bundle = Bundle::new(model.clone());
    bundle.textures = model
        .textures
        .iter()
        .map(|_| TextureData {
            format: TextureFormat::Png,
            bytes: FAKE_PNG.to_vec(),
        })
        .collect();
    bundle.motions = vec![
        ugoe::format::motion_from_json(r#"{ "name": "wave", "duration": 1.0, "curves": [] }"#)
            .unwrap(),
    ];
    bundle.expressions = vec![
        ugoe::format::expression_from_json(
            r#"{ "name": "smile", "parameters": [{ "parameter": "X", "value": 0.5, "op": "add" }] }"#,
        )
        .unwrap(),
    ];
    ugoe::format::write_bundle(&bundle).unwrap()
}

/// パラメータ `X` の ID (C の文字列。ずっと有効)。
fn x_id() -> &'static CStr {
    c"X"
}

/// 呼び出しの結果が成功であることを確かめる (失敗ならメッセージを出す)。
#[track_caller]
fn ok(code: UgoeResult) {
    assert_eq!(code, UGOE_OK, "{}", last_error());
}

fn last_error() -> String {
    let ptr = ugoe_last_error();
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn string(ptr: *const c_char) -> String {
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned()
}

/// テストの間だけ生きる `UgoePuppet`。
struct Handle(*mut UgoePuppet);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { ugoe_puppet_destroy(self.0) };
    }
}

fn from_bundle(bytes: &[u8]) -> Handle {
    let mut p = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut p) });
    assert!(!p.is_null());
    Handle(p)
}

fn positions(p: &Handle, drawable: u32) -> Vec<f32> {
    let mut ptr = ptr::null();
    let mut len = 0;
    ok(unsafe { ugoe_puppet_positions(p.0, drawable, &mut ptr, &mut len) });
    unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
}

#[test]
fn versions() {
    assert_eq!(ugoe_abi_version(), ABI_VERSION);
    assert_eq!(string(ugoe_version()), ugoe::VERSION);
}

#[test]
fn matches_puppet_after_setting_a_parameter() {
    let p = from_bundle(&single_quad_bundle());
    let mut reference = Puppet::new(fixtures::single_quad()).unwrap();

    let mut count = 0;
    ok(unsafe { ugoe_puppet_parameter_count(p.0, &mut count) });
    assert_eq!(count, 1);
    let mut id = ptr::null();
    ok(unsafe { ugoe_puppet_parameter_id(p.0, 0, &mut id) });
    assert_eq!(string(id), "X");

    let x = CString::new("X").unwrap();
    let mut index = u32::MAX;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, x.as_ptr(), &mut index) });
    assert_eq!(index, 0);
    let mut info = UgoeParameterInfo::default();
    ok(unsafe { ugoe_puppet_parameter_info(p.0, index, &mut info) });
    assert_eq!((info.min, info.max), (-1.0, 1.0));

    // 範囲の外の値は丸める。
    ok(unsafe { ugoe_puppet_set_parameter(p.0, index, 5.0) });
    let mut value = 0.0;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, index, &mut value) });
    assert_eq!(value, 1.0);
    ok(unsafe { ugoe_puppet_update(p.0, 1.0 / 60.0) });

    let params = reference.instance_mut().parameters_mut();
    let i = params.index_of("X").unwrap();
    params.set(i, 1.0);
    reference.update(1.0 / 60.0);

    let mut drawables = 0;
    ok(unsafe { ugoe_puppet_drawable_count(p.0, &mut drawables) });
    assert_eq!(drawables as usize, reference.instance().drawables().len());
    for d in 0..drawables {
        let expected: Vec<f32> = reference.instance().drawables()[d as usize]
            .positions
            .iter()
            .flat_map(|v| [v.x, v.y])
            .collect();
        let borrowed = positions(&p, d);
        assert_eq!(borrowed, expected);

        let mut copied = vec![f32::NAN; expected.len()];
        ok(unsafe { ugoe_puppet_copy_positions(p.0, d, copied.as_mut_ptr(), copied.len()) });
        assert_eq!(copied, expected);

        let mut info = std::mem::MaybeUninit::<UgoeDrawableInfo>::uninit();
        ok(unsafe { ugoe_puppet_drawable_info(p.0, d, info.as_mut_ptr()) });
        let info = unsafe { info.assume_init() };
        assert_eq!(info.vertex_count as usize * 2, expected.len());
        let mesh = &reference.instance().compiled().model().meshes[d as usize];
        assert_eq!(string(info.id), mesh.id.as_str());
        assert_eq!(info.index_count as usize, mesh.indices.len());

        let mut uvs = ptr::null();
        let mut len = 0;
        ok(unsafe { ugoe_puppet_drawable_uvs(p.0, d, &mut uvs, &mut len) });
        assert_eq!(len, info.vertex_count as usize * 2);
        let mut indices = ptr::null();
        ok(unsafe { ugoe_puppet_drawable_indices(p.0, d, &mut indices, &mut len) });
        assert_eq!(
            unsafe { std::slice::from_raw_parts(indices, len) },
            mesh.indices
        );

        let mut state = UgoeDrawableState::default();
        ok(unsafe { ugoe_puppet_drawable_state(p.0, d, &mut state) });
        let expected = &reference.instance().drawables()[d as usize];
        assert_eq!(state.opacity, expected.opacity);
        assert_eq!(state.draw_order, expected.draw_order);
        assert_eq!(state.visible, expected.visible);
        assert_eq!(state.blend_mode, UGOE_BLEND_NORMAL);
    }

    let mut order = ptr::null();
    let mut len = 0;
    ok(unsafe { ugoe_puppet_render_order(p.0, &mut order, &mut len) });
    assert_eq!(
        unsafe { std::slice::from_raw_parts(order, len) },
        reference.instance().render_order()
    );

    let mut canvas = UgoeCanvas::default();
    ok(unsafe { ugoe_puppet_canvas(p.0, &mut canvas) });
    assert_eq!((canvas.width, canvas.height), (256.0, 256.0));
    assert_eq!((canvas.origin_x, canvas.origin_y), (128.0, 128.0));
}

#[test]
fn reports_changes_between_updates() {
    let p = from_bundle(&single_quad_bundle());
    ok(unsafe { ugoe_puppet_update(p.0, 0.0) });
    let mut state = UgoeDrawableState::default();
    ok(unsafe { ugoe_puppet_drawable_state(p.0, 0, &mut state) });
    assert_ne!(state.changes & UGOE_CHANGED_POSITIONS, 0);

    ok(unsafe { ugoe_puppet_update(p.0, 0.0) });
    ok(unsafe { ugoe_puppet_drawable_state(p.0, 0, &mut state) });
    assert_eq!(state.changes, 0);
    let mut changed = true;
    ok(unsafe { ugoe_puppet_render_order_changed(p.0, &mut changed) });
    assert!(!changed);

    ok(unsafe { ugoe_puppet_set_parameter(p.0, 0, 0.5) });
    ok(unsafe { ugoe_puppet_update(p.0, 0.0) });
    ok(unsafe { ugoe_puppet_drawable_state(p.0, 0, &mut state) });
    assert_eq!(state.changes, UGOE_CHANGED_POSITIONS);
}

#[test]
fn textures_motions_and_expressions_come_from_the_bundle() {
    let p = from_bundle(&single_quad_bundle());
    let mut count = 0;
    ok(unsafe { ugoe_puppet_texture_count(p.0, &mut count) });
    assert_eq!(count, 1);
    let mut texture = UgoeTexture {
        format: u32::MAX,
        data: ptr::null(),
        len: 0,
    };
    ok(unsafe { ugoe_puppet_texture(p.0, 0, &mut texture) });
    assert_eq!(texture.format, UGOE_TEXTURE_PNG);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(texture.data, texture.len) },
        FAKE_PNG
    );

    ok(unsafe { ugoe_puppet_motion_count(p.0, &mut count) });
    assert_eq!(count, 1);
    let mut name = ptr::null();
    ok(unsafe { ugoe_puppet_motion_name(p.0, 0, &mut name) });
    assert_eq!(string(name), "wave");
    let mut started = false;
    ok(unsafe { ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_NORMAL, &mut started) });
    assert!(started);
    // 再生中のモーションより低い優先度では開始しない。
    ok(unsafe { ugoe_puppet_start_motion(p.0, 0, UGOE_PRIORITY_IDLE, &mut started) });
    assert!(!started);
    assert_eq!(
        unsafe { ugoe_puppet_start_motion(p.0, 0, 99, ptr::null_mut()) },
        UGOE_ERROR_INVALID_ARGUMENT
    );

    ok(unsafe { ugoe_puppet_expression_count(p.0, &mut count) });
    assert_eq!(count, 1);
    ok(unsafe { ugoe_puppet_expression_name(p.0, 0, &mut name) });
    assert_eq!(string(name), "smile");
    ok(unsafe { ugoe_puppet_set_expression(p.0, 0) });
    ok(unsafe { ugoe_puppet_update(p.0, 1.0) });
    let mut value = 0.0;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, 0, &mut value) });
    assert!((value - 0.5).abs() < 1e-5, "{value}");
}

#[test]
fn json_model_has_no_texture_data() {
    let json = ugoe::format::model_to_json(&fixtures::single_quad()).unwrap();
    let mut p = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_json(json.as_ptr(), json.len(), &mut p) });
    let p = Handle(p);
    let mut texture = UgoeTexture {
        format: u32::MAX,
        data: FAKE_PNG.as_ptr(),
        len: 1,
    };
    ok(unsafe { ugoe_puppet_texture(p.0, 0, &mut texture) });
    assert_eq!(texture.format, UGOE_TEXTURE_NONE);
    assert!(texture.data.is_null());
    let mut count = 1;
    ok(unsafe { ugoe_puppet_motion_count(p.0, &mut count) });
    assert_eq!(count, 0);
}

#[test]
fn errors_are_reported_with_codes_and_messages() {
    let mut p = ptr::null_mut();
    let garbage = b"not a bundle";
    let code = unsafe { ugoe_puppet_create_from_bundle(garbage.as_ptr(), garbage.len(), &mut p) };
    assert_eq!(code, UGOE_ERROR_FORMAT);
    assert!(p.is_null());
    assert!(!last_error().is_empty());

    let json = b"{}";
    let code = unsafe { ugoe_puppet_create_from_json(json.as_ptr(), json.len(), &mut p) };
    assert_eq!(code, UGOE_ERROR_FORMAT);

    let code = unsafe { ugoe_puppet_create_from_bundle(ptr::null(), 4, &mut p) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    assert!(last_error().contains("bytes"));

    let mut count = 0;
    assert_eq!(
        unsafe { ugoe_puppet_parameter_count(ptr::null(), &mut count) },
        UGOE_ERROR_NULL_POINTER
    );
    assert_eq!(
        unsafe { ugoe_puppet_update(ptr::null_mut(), 0.0) },
        UGOE_ERROR_NULL_POINTER
    );
    // NULL の破棄は何もしない。
    unsafe { ugoe_puppet_destroy(ptr::null_mut()) };

    let p = from_bundle(&single_quad_bundle());
    assert_eq!(
        unsafe { ugoe_puppet_update(p.0, f32::NAN) },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { ugoe_puppet_parameter_count(p.0, ptr::null_mut()) },
        UGOE_ERROR_NULL_POINTER
    );
    assert_eq!(
        unsafe { ugoe_puppet_set_parameter(p.0, 1, 0.0) },
        UGOE_ERROR_OUT_OF_RANGE
    );
    assert!(last_error().contains("parameter index 1"));
    let missing = CString::new("Missing").unwrap();
    let mut index = 0;
    assert_eq!(
        unsafe { ugoe_puppet_parameter_index(p.0, missing.as_ptr(), &mut index) },
        UGOE_ERROR_NOT_FOUND
    );
    let bad_utf8 = [0xffu8, 0];
    assert_eq!(
        unsafe { ugoe_puppet_parameter_index(p.0, bad_utf8.as_ptr().cast(), &mut index) },
        UGOE_ERROR_INVALID_ARGUMENT
    );

    // NaN の値は、どの入口でも受け付けず何も変えない。
    let mut value = f32::NAN;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, 0, &mut value) });
    assert_eq!(
        unsafe { ugoe_puppet_set_parameter(p.0, 0, f32::NAN) },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { ugoe_puppet_set_lip_sync_level(p.0, f32::NAN) },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    let mut after = f32::NAN;
    ok(unsafe { ugoe_puppet_get_parameter(p.0, 0, &mut after) });
    assert_eq!(after, value);
    // 無限大は範囲に丸める。
    ok(unsafe { ugoe_puppet_set_parameter(p.0, 0, f32::NEG_INFINITY) });
    ok(unsafe { ugoe_puppet_get_parameter(p.0, 0, &mut after) });
    assert_eq!(after, -1.0);

    // UTF-8 でない ID は、何番目かをメッセージに書く。
    let ids = [x_id().as_ptr(), bad_utf8.as_ptr().cast()];
    let values = [0.0f32, 0.0];
    assert_eq!(
        unsafe {
            ugoe_puppet_set_parameters_by_id(p.0, ids.as_ptr(), values.as_ptr(), 2, ptr::null_mut())
        },
        UGOE_ERROR_INVALID_ARGUMENT
    );
    assert!(last_error().contains("ids[1]"), "{}", last_error());

    // 作る関数は `out` が NULL なら何も作らない。
    let bytes = single_quad_bundle();
    assert_eq!(
        unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), ptr::null_mut()) },
        UGOE_ERROR_NULL_POINTER
    );

    let mut small = [0.0f32; 3];
    assert_eq!(
        unsafe { ugoe_puppet_copy_positions(p.0, 0, small.as_mut_ptr(), small.len()) },
        UGOE_ERROR_BUFFER_TOO_SMALL
    );
    assert_eq!(small, [0.0; 3]);
    let mut state = UgoeDrawableState::default();
    assert_eq!(
        unsafe { ugoe_puppet_drawable_state(p.0, 9, &mut state) },
        UGOE_ERROR_OUT_OF_RANGE
    );
}

#[test]
fn last_error_is_per_thread() {
    let mut count = 0;
    assert_eq!(
        unsafe { ugoe_puppet_parameter_count(ptr::null(), &mut count) },
        UGOE_ERROR_NULL_POINTER
    );
    let other = std::thread::spawn(|| ugoe_last_error().is_null())
        .join()
        .unwrap();
    assert!(other);
    assert!(!ugoe_last_error().is_null());
}
