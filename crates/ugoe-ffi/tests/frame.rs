//! 毎フレームの描画の状態をまとめて受け取る関数 (`ugoe_puppet_frame`・`ugoe_puppet_copy_frame`。T0490) を確かめる。
#![allow(unsafe_code)] // C ABI の関数は unsafe なので、呼ぶテストに限り許可する。
#![allow(clippy::unwrap_used)]
// SAFETY (このファイルの unsafe すべて): C ABI の関数には、このテストが作って破棄していない `UgoePuppet`・`UgoeModule`、
// 生きている Rust の値を指すポインタ、または意図的に NULL を渡す。借りたポインタは次の update や破棄の前に読む。

use std::ffi::CStr;
use std::ptr;

use ugoe::Puppet;
use ugoe::model::BlendMode;
use ugoe::runtime::CompiledModel;
use ugoe_ffi::*;

mod common;
use common::*;

const AVATAR: &[u8] = include_bytes!("../../../assets/samples/avatar.ugoe");
const DRESS_UP: &[u8] = include_bytes!("../../../assets/samples/dress-up/dress-up.ugoe");
const CAT_EARS: &[u8] = include_bytes!("../../../assets/samples/dress-up/cat-ears.ugoemod");

#[track_caller]
fn ok(code: UgoeResult) {
    let message = last_message();
    let message = if message.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!(code, UGOE_OK, "{message}");
}

struct Handle(*mut UgoePuppet);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { ugoe_puppet_destroy(self.0) };
    }
}

fn from_bundle(bytes: &[u8]) -> Handle {
    let mut p = ptr::null_mut();
    ok(unsafe { ugoe_puppet_create_from_bundle(bytes.as_ptr(), bytes.len(), &mut p, err()) });
    Handle(p)
}

/// `UgoeDrawableState` の中身 (比べるため)。
type State = (f32, f32, [f32; 3], [f32; 3], u32, f32, bool, u8);

/// 比べやすい形にした 1 フレーム分の状態。
#[derive(Debug, PartialEq)]
struct Snapshot {
    states: Vec<State>,
    render_order: Vec<u32>,
    render_order_changed: bool,
    ranges: Vec<(usize, usize)>,
    positions: Vec<f32>,
}

fn state_tuple(s: &UgoeDrawableState) -> State {
    (
        s.opacity,
        s.draw_order,
        s.multiply_color,
        s.screen_color,
        s.blend_mode,
        s.mask_strength,
        s.visible,
        s.changes,
    )
}

fn slice<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// `ugoe_puppet_frame` で借りたもの。
fn borrowed(p: &Handle) -> Snapshot {
    let mut f = UgoeFrame::default();
    ok(unsafe { ugoe_puppet_frame(p.0, &mut f, err()) });
    let n = f.drawable_count as usize;
    Snapshot {
        states: slice(f.states, n).iter().map(state_tuple).collect(),
        render_order: slice(f.render_order, n).to_vec(),
        render_order_changed: f.render_order_changed,
        ranges: slice(f.position_ranges, n)
            .iter()
            .map(|r| (r.first, r.count))
            .collect(),
        positions: slice(f.positions, f.positions_len).to_vec(),
    }
}

/// `ugoe_puppet_copy_frame` で写したもの (`positions` は `positions` の中身に上書きする)。
fn copied(p: &Handle, positions: &mut Vec<f32>, changed_positions_only: bool) -> Snapshot {
    let mut size = UgoeFrameCopy::default();
    let empty = UgoeFrameBuffers::default();
    let code = unsafe { ugoe_puppet_copy_frame(p.0, &empty, false, &mut size, err()) };
    let n = size.drawable_count as usize;
    if n > 0 {
        assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    }
    let mut states = vec![UgoeDrawableState::default(); n];
    let mut order = vec![u32::MAX; n];
    let mut ranges = vec![UgoePositionRange::default(); n];
    positions.resize(size.positions_len, f32::NAN);
    let buffers = UgoeFrameBuffers {
        states: states.as_mut_ptr(),
        render_order: order.as_mut_ptr(),
        position_ranges: ranges.as_mut_ptr(),
        drawable_capacity: n,
        positions: positions.as_mut_ptr(),
        positions_capacity: positions.len(),
    };
    let mut out = UgoeFrameCopy::default();
    ok(unsafe { ugoe_puppet_copy_frame(p.0, &buffers, changed_positions_only, &mut out, err()) });
    assert_eq!(out.drawable_count, size.drawable_count);
    assert_eq!(out.positions_len, size.positions_len);
    Snapshot {
        states: states.iter().map(state_tuple).collect(),
        render_order: order,
        render_order_changed: out.render_order_changed,
        ranges: ranges.iter().map(|r| (r.first, r.count)).collect(),
        positions: positions.clone(),
    }
}

/// 同じ操作をした `ugoe::Puppet` の出力 (C ABI を通さずに読んだもの)。
fn reference(puppet: &Puppet) -> Snapshot {
    let instance = puppet.instance();
    let mut snapshot = Snapshot {
        states: Vec::new(),
        render_order: instance.render_order().to_vec(),
        render_order_changed: instance.render_order_changed(),
        ranges: Vec::new(),
        positions: Vec::new(),
    };
    for (d, change) in instance.drawables().iter().zip(instance.changes()) {
        let blend = match d.blend.effective() {
            BlendMode::Additive => UGOE_BLEND_ADDITIVE,
            BlendMode::Multiply => UGOE_BLEND_MULTIPLY,
            BlendMode::Normal | BlendMode::Unknown => UGOE_BLEND_NORMAL,
        };
        snapshot.states.push((
            d.opacity,
            d.draw_order,
            [d.multiply_color.r, d.multiply_color.g, d.multiply_color.b],
            [d.screen_color.r, d.screen_color.g, d.screen_color.b],
            blend,
            d.mask_strength,
            d.visible,
            change.bits(),
        ));
        let first = snapshot.positions.len();
        snapshot
            .positions
            .extend(d.positions.iter().flat_map(|v| [v.x, v.y]));
        snapshot.ranges.push((first, d.positions.len() * 2));
    }
    snapshot
}

/// `bytes` から C ABI と同じ手順で作った `ugoe::Puppet`。
fn native(bytes: &[u8]) -> Puppet {
    let bundle = ugoe::format::read_bundle(bytes).unwrap();
    let compiled = CompiledModel::compile(bundle.model).unwrap();
    let mut puppet = Puppet::from_compiled(compiled);
    if let Some(physics) = bundle.physics {
        puppet = puppet.with_physics(physics);
    }
    puppet
}

fn set(p: &Handle, native: &mut Puppet, id: &CStr, value: f32) {
    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, id.as_ptr(), &mut index, err()) });
    ok(unsafe { ugoe_puppet_set_parameter(p.0, index, value, err()) });
    let params = native.instance_mut().parameters_mut();
    let i = params.index_of(id.to_str().unwrap()).unwrap();
    params.set(i, value);
}

#[test]
fn frame_matches_the_puppet() {
    let p = from_bundle(AVATAR);
    let mut native = native(AVATAR);
    // 変わった範囲だけを写すバッファ。毎フレーム同じものに写す。
    let mut incremental = Vec::new();
    let mut full = Vec::new();
    for frame in 0..12 {
        match frame {
            3 => set(&p, &mut native, c"AngleX", 25.0),
            6 => set(&p, &mut native, c"AngleZ", -15.0),
            _ => {}
        }
        ok(unsafe { ugoe_puppet_update(p.0, 1.0 / 30.0, err()) });
        native.update(1.0 / 30.0);
        // フレーム 8 は読まずに次の update へ進む (2 つ前の出力からは、すべてを写し直す)。
        if frame == 8 {
            continue;
        }
        let expected = reference(&native);
        assert_eq!(borrowed(&p), expected, "frame {frame}");
        assert_eq!(borrowed(&p), expected, "frame {frame} (read twice)");
        assert_eq!(copied(&p, &mut full, false), expected, "frame {frame}");
        // 読まなかったフレームの次は、変わった範囲だけでは足りないので、すべてを写す。
        let partial = copied(&p, &mut incremental, frame != 9);
        assert_eq!(partial, expected, "frame {frame} (changed positions only)");
    }
}

/// 描画要素ごとの関数 (読み込みのときに読む情報) と食い違わないか。
fn check_layout(p: &Handle, frame: &Snapshot) {
    let mut count = 0;
    ok(unsafe { ugoe_puppet_drawable_count(p.0, &mut count, err()) });
    assert_eq!(frame.states.len(), count as usize);
    let mut first = 0;
    for (d, &(start, len)) in frame.ranges.iter().enumerate() {
        let mut info = std::mem::MaybeUninit::<UgoeDrawableInfo>::uninit();
        ok(unsafe { ugoe_puppet_drawable_info(p.0, d as u32, info.as_mut_ptr(), err()) });
        let info = unsafe { info.assume_init() };
        assert_eq!((start, len), (first, info.vertex_count as usize * 2));
        first += len;
    }
    assert_eq!(frame.positions.len(), first);
    let mut order = frame.render_order.clone();
    order.sort_unstable();
    assert_eq!(order, (0..count).collect::<Vec<_>>());
}

#[test]
fn frame_follows_dress_up() {
    let p = from_bundle(DRESS_UP);
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    let before = borrowed(&p);
    check_layout(&p, &before);
    assert!(!before.render_order_changed);

    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(CAT_EARS.as_ptr(), CAT_EARS.len(), &mut m, err()) });
    ok(unsafe { ugoe_puppet_attach_module(p.0, m, err()) });
    // 着せ替えの後はすぐに新しいモデルの出力になり、すべてが変わったと報告する。
    let after = borrowed(&p);
    check_layout(&p, &after);
    assert!(after.states.len() > before.states.len());
    assert!(after.render_order_changed);
    assert!(after.states.iter().all(|s| s.7 == 0x7f));
    assert_eq!(copied(&p, &mut Vec::new(), false), after);

    ok(unsafe { ugoe_puppet_update(p.0, 1.0 / 60.0, err()) });
    let next = borrowed(&p);
    check_layout(&p, &next);
    assert_eq!(copied(&p, &mut Vec::new(), false), next);
    unsafe { ugoe_module_destroy(m) };
}

#[test]
fn copy_frame_reports_what_it_needs() {
    let p = from_bundle(AVATAR);
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    let frame = borrowed(&p);
    let n = frame.states.len();
    let mut states = vec![UgoeDrawableState::default(); n];
    let mut order = vec![0; n];
    let mut ranges = vec![UgoePositionRange::default(); n];
    let mut positions = vec![f32::NAN; frame.positions.len() - 1];
    let mut buffers = UgoeFrameBuffers {
        states: states.as_mut_ptr(),
        render_order: order.as_mut_ptr(),
        position_ranges: ranges.as_mut_ptr(),
        drawable_capacity: n,
        positions: positions.as_mut_ptr(),
        positions_capacity: positions.len(),
    };
    let mut out = UgoeFrameCopy::default();
    let code = unsafe { ugoe_puppet_copy_frame(p.0, &buffers, false, &mut out, err()) };
    assert_eq!(code, UGOE_ERROR_BUFFER_TOO_SMALL);
    assert_eq!(out.drawable_count as usize, n);
    assert_eq!(out.positions_len, frame.positions.len());
    // バッファには何も書かない。
    assert!(positions.iter().all(|v| v.is_nan()));
    assert!(order.iter().all(|&i| i == 0));

    // 大きさが足りていても、要るポインタが NULL なら書かない。
    buffers.positions = ptr::null_mut();
    buffers.positions_capacity = frame.positions.len();
    let code = unsafe { ugoe_puppet_copy_frame(p.0, &buffers, false, &mut out, err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);

    let code = unsafe { ugoe_puppet_copy_frame(p.0, ptr::null(), false, &mut out, err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    let code = unsafe { ugoe_puppet_frame(p.0, ptr::null_mut(), err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    let mut f = UgoeFrame::default();
    let code = unsafe { ugoe_puppet_frame(ptr::null_mut(), &mut f, err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
}

fn changed(p: &Handle) -> bool {
    let mut out = false;
    ok(unsafe { ugoe_puppet_changed(p.0, &mut out, err()) });
    out
}

#[test]
fn changed_reports_visible_changes() {
    let p = from_bundle(DRESS_UP);
    assert!(changed(&p), "作った直後");
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(changed(&p), "最初の update の後");
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(!changed(&p), "止まったモデルの 2 回目の update の後");
    assert!(!changed(&p), "読んでも変わらない");

    let mut index = 0;
    ok(unsafe { ugoe_puppet_parameter_index(p.0, c"AngleX".as_ptr(), &mut index, err()) });
    ok(unsafe { ugoe_puppet_set_parameter(p.0, index, 20.0, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(changed(&p), "パラメータを変えた後");
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(!changed(&p));

    let mut m = ptr::null_mut();
    ok(unsafe { ugoe_module_create(CAT_EARS.as_ptr(), CAT_EARS.len(), &mut m, err()) });
    ok(unsafe { ugoe_puppet_attach_module(p.0, m, err()) });
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(changed(&p), "着せ替えの後");
    ok(unsafe { ugoe_puppet_update(p.0, 0.0, err()) });
    assert!(!changed(&p));
    unsafe { ugoe_module_destroy(m) };
}

#[test]
fn changed_matches_the_puppet() {
    let p = from_bundle(AVATAR);
    let mut native = native(AVATAR);
    let mut seen = [false; 2];
    for frame in 0..120 {
        if frame == 30 {
            set(&p, &mut native, c"AngleX", 25.0);
        }
        ok(unsafe { ugoe_puppet_update(p.0, 1.0 / 30.0, err()) });
        native.update(1.0 / 30.0);
        assert_eq!(changed(&p), native.changed(), "frame {frame}");
        seen[usize::from(native.changed())] = true;
    }
    assert_eq!(seen, [true, true], "true と false の両方を確かめる");
}

#[test]
fn changed_rejects_null() {
    let p = from_bundle(AVATAR);
    let mut out = false;
    let code = unsafe { ugoe_puppet_changed(ptr::null(), &mut out, err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
    let code = unsafe { ugoe_puppet_changed(p.0, ptr::null_mut(), err()) };
    assert_eq!(code, UGOE_ERROR_NULL_POINTER);
}
