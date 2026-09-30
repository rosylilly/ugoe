//! モーションのイベントと物理の設定の差し替え (T0412)。wasm の `motion_events`・`set_physics_json`・
//! `physics_json`・`reset_physics` と同じ。

use std::ffi::{CString, c_char};

use crate::error::{
    Error, UGOE_ERROR_BUFFER_TOO_SMALL, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_ARGUMENT,
    UGOE_ERROR_INVALID_MODEL, UgoeResult, c_string, guard,
};
use crate::puppet::{UgoePuppet, bytes, puppet_mut, puppet_ref, write};

/// 発火したモーションのイベント (`docs/spec/motion.md`)。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeMotionEvent {
    /// モーションの中の時刻 (秒)。
    pub time: f32,
    /// イベントの値 (UTF-8、NUL 終端)。次の `ugoe_puppet_update` まで有効。
    pub value: *const c_char,
}

/// 直前の `ugoe_puppet_update` で発火したモーションのイベント (C の文字列にしたもの)。
#[derive(Debug, Default)]
pub(crate) struct MotionEvents(Vec<(f32, CString)>);

impl MotionEvents {
    /// `puppet` の直前の `update` のイベントで作り直す。
    pub(crate) fn refresh(&mut self, puppet: &ugoe::Puppet) {
        self.0 = puppet
            .motion_events()
            .iter()
            .map(|e| (e.t, c_string(&e.value)))
            .collect();
    }
}

/// 直前の `ugoe_puppet_update` で発火したモーションのイベントの数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_event_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.motion_events.0.len() as u32) }
    })
}

/// 直前の `ugoe_puppet_update` で発火した `index` 番目のモーションのイベント。開始した順のモーションごとに、
/// 発火した順に並ぶ (ループをまたいだら周回の順)。文字列は次の `ugoe_puppet_update` まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_event(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeMotionEvent,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let events = &p.motion_events.0;
        let (time, value) = events
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("motion event", index as usize, events.len()))?;
        let event = UgoeMotionEvent {
            time: *time,
            value: value.as_ptr(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", event) }
    })
}

/// 物理の設定を JSON (`*.physics.json` の形。UTF-8、`len` バイト。NUL 終端は要らない) で差し替える。
/// `json` が NULL なら物理を外す。粒子は次の `ugoe_puppet_update` で、その時の入力に対する静止姿勢から始める。
///
/// JSON を読めなければ `UGOE_ERROR_FORMAT`、モデルに対する検査 (`docs/spec/physics.md`) を通らなければ
/// `UGOE_ERROR_INVALID_MODEL` を返し、何も変えない。
///
/// 着せ替えのモジュールを付けているときは、付けたモデルに対して差し替える。次に付け外しをすると、
/// 最初にモジュールを付けたときの本体の設定から合成し直すので、ここで差し替えた設定は失われる。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`json` は NULL か `len` バイトの読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_physics_json(
    puppet: *mut UgoePuppet,
    json: *const u8,
    len: usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let settings = if json.is_null() {
            None
        } else {
            // SAFETY: 呼び出し側の約束。
            let json = std::str::from_utf8(unsafe { bytes(json, len, "json")? })
                .map_err(|_| Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`json` is not UTF-8"))?;
            let settings = ugoe::format::physics_from_json(json)
                .map_err(|e| Error::new(UGOE_ERROR_FORMAT, e.to_string()))?;
            Some(settings)
        };
        p.puppet.set_physics(settings).map_err(|errors| {
            let list: Vec<String> = errors.iter().map(ToString::to_string).collect();
            Error::new(
                UGOE_ERROR_INVALID_MODEL,
                format!("physics settings failed validation: {}", list.join("; ")),
            )
        })
    })
}

/// 今の物理の設定の JSON (UTF-8。NUL 終端しない) を `out` (`capacity` バイト) に書き、`*out_len` にバイト数を書く。
/// 物理が無ければ `*out_len` は 0。`capacity` が足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out_len` だけを書く
/// (長さを知るには `capacity` を 0 にして呼ぶ。そのとき `out` は NULL でもよい)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は `capacity` バイトを書き込める領域、`out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_physics_json(
    puppet: *const UgoePuppet,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let json = match p.puppet.physics() {
            Some(physics) => ugoe::format::physics_to_json(physics.settings())
                .map_err(|e| Error::new(UGOE_ERROR_FORMAT, e.to_string()))?,
            None => String::new(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", json.len())? };
        if capacity < json.len() {
            return Err(Error::new(
                UGOE_ERROR_BUFFER_TOO_SMALL,
                format!("`out` holds {capacity} bytes but {} are needed", json.len()),
            ));
        }
        if json.is_empty() {
            return Ok(());
        }
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: `out` は `capacity` (≥ json.len()) バイトを書き込める (呼び出し側の約束)。
        unsafe { std::ptr::copy_nonoverlapping(json.as_ptr(), out, json.len()) };
        Ok(())
    })
}

/// 物理の粒子を初期姿勢に戻す (次の `ugoe_puppet_update` で静止姿勢に置き直す)。物理が無ければ何もしない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_reset_physics(puppet: *mut UgoePuppet) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        unsafe { puppet_mut(puppet)? }.puppet.reset_physics();
        Ok(())
    })
}
