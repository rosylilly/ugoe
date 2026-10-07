//! モーションのイベントと物理の設定の差し替え (T0412)。wasm の `motion_events`・`set_physics_json`・
//! `physics_json`・`reset_physics` と同じ。効果の設定の差し替え (T0414) は wasm の `set_effects_json` と同じ。
//! 本体の物理の設定 (T0416) は wasm の `set_base_physics_json`・`base_physics_json` と同じ。

use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_char;

use ugoe::DressUpError;
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::{ValidationError, describe_errors};

use crate::dress_up::dress_up_error;
use crate::error::{
    Error, UGOE_ERROR_BUFFER_TOO_SMALL, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_ARGUMENT,
    UGOE_ERROR_INVALID_MODEL, UgoeError, UgoeResult, c_string, guard,
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

/// モーションのイベントを発火したモーション (T0462。`ugoe::Puppet::motion_event_sources`)。
///
/// `UgoeMotionEvent` は ABI を保つために変えず、別の構造体にした (crates/ugoe-ffi/README.md#abi-の版)。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeMotionEventSource {
    /// モーションの名前 (UTF-8、NUL 終端)。次の `ugoe_puppet_update` まで有効。
    pub motion: *const c_char,
    /// モーションを始めた番号 (この `UgoePuppet` で開始したモーションに 1 から振る。開始しなかった要求では進まない)。
    /// 同じ名前のモーションを始め直したものを見分けるのに使う。
    pub number: u64,
}

/// 直前の `ugoe_puppet_update` で発火したモーションのイベント (C の文字列にしたもの)。
#[derive(Debug, Default)]
pub(crate) struct MotionEvents(Vec<MotionEventEntry>);

/// 発火したイベント 1 つ (値と、発火したモーション)。
#[derive(Debug)]
struct MotionEventEntry {
    time: f32,
    value: CString,
    number: u64,
    motion: CString,
}

impl MotionEvents {
    /// `puppet` の直前の `update` のイベントで作り直す。
    pub(crate) fn refresh(&mut self, puppet: &ugoe::Puppet) {
        self.0 = puppet
            .motion_events()
            .iter()
            .zip(puppet.motion_event_sources())
            .map(|(e, source)| MotionEventEntry {
                time: e.t,
                value: c_string(&e.value),
                number: source.number,
                motion: c_string(&source.name),
            })
            .collect();
    }

    fn get(&self, index: u32) -> Result<&MotionEventEntry, Error> {
        self.0
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("motion event", index as usize, self.0.len()))
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let entry = p.motion_events.get(index)?;
        let event = UgoeMotionEvent {
            time: entry.time,
            value: entry.value.as_ptr(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", event) }
    })
}

/// 直前の `ugoe_puppet_update` で発火した `index` 番目のモーションのイベントを発火したモーション (T0462)。
/// `index` は `ugoe_puppet_motion_event` と同じ並び。クロスフェードでフェードアウト中のモーションのイベントは、
/// そのモーションの名前と番号になる (今のモーションとは限らない)。文字列は次の `ugoe_puppet_update` まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_event_source(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeMotionEventSource,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let entry = p.motion_events.get(index)?;
        let source = UgoeMotionEventSource {
            motion: entry.motion.as_ptr(),
            number: entry.number,
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", source) }
    })
}

/// 物理の設定を JSON (`*.physics.json` の形。UTF-8、`len` バイト。NUL 終端は要らない) で差し替える。
/// `json` が NULL なら物理を外す。粒子は次の `ugoe_puppet_update` で、その時の入力に対する静止姿勢から始める。
///
/// JSON を読めなければ `UGOE_ERROR_FORMAT`、モデルに対する検査 (`docs/spec/physics.md`) を通らなければ
/// `UGOE_ERROR_INVALID_MODEL` (`ugoe_error_kind` は `"invalid_physics"`、`ugoe_error_field` の `"errors"`・`"error_codes"` は
/// 検査のエラーの数と種類。着せ替えの `invalid_physics` と同じ。T0469) を返し、何も変えない。
///
/// 着せ替えのモジュールを付けているときは、付けたモデルに対して差し替える。次に付け外しをすると、
/// 本体の設定から合成し直すので、ここで差し替えた設定は失われる。付けている間も保つ本体の設定は
/// `ugoe_puppet_set_base_physics_json` で渡す。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`json` は NULL か `len` バイトの読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_physics_json(
    puppet: *mut UgoePuppet,
    json: *const u8,
    len: usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let settings = unsafe { read_physics(json, len)? };
        p.puppet
            .set_physics(settings)
            .map_err(|errors| invalid_physics(&errors))
    })
}

/// 本体の物理の設定を JSON (`ugoe_puppet_set_physics_json` と同じ形) で差し替える。`json` が NULL なら外す
/// (`ugoe::Puppet::set_base_physics`、`docs/spec/modules.md#puppet-での着せ替え`)。
///
/// 着せ替えのモジュールを付けていれば、付けているモジュールの組と合成し直した設定に差し替え、この後の付け外しでも
/// この設定から合成する (`ugoe_puppet_set_physics_json` で渡した設定は次の付け外しで失われる)。付けたことがなければ
/// `ugoe_puppet_set_physics_json` と同じ。粒子は次の `ugoe_puppet_update` で、その時の入力に対する静止姿勢から始める。
///
/// JSON を読めなければ `UGOE_ERROR_FORMAT`、本体のモデルか合成したモデルに対する検査を通らなければ
/// `UGOE_ERROR_INVALID_MODEL` (種類と補足の値は `ugoe_puppet_set_physics_json` と同じ `"invalid_physics"`)、合成できなければ `UGOE_ERROR_DRESS_UP` を返し、何も変えない。本体の物理の有無が変わって
/// モデルを差し替えたときは、着せ替えの操作と同じく ID の文字列のポインタが無効になる。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`json` は NULL か `len` バイトの読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_base_physics_json(
    puppet: *mut UgoePuppet,
    json: *const u8,
    len: usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let settings = unsafe { read_physics(json, len)? };
        let before = Arc::clone(p.puppet.instance().compiled());
        p.puppet
            .set_base_physics(settings)
            .map_err(|error| match error {
                DressUpError::Physics(errors) => invalid_physics(&errors),
                error => dress_up_error(error),
            })?;
        if !Arc::ptr_eq(&before, p.puppet.instance().compiled()) {
            p.refresh_model();
        }
        Ok(())
    })
}

/// `json` (NULL か `len` バイトの UTF-8) を物理の設定として読む。NULL なら `None`。
///
/// # Safety
///
/// `json` は NULL か `len` バイトの読める領域を指すこと。
unsafe fn read_physics(json: *const u8, len: usize) -> Result<Option<PhysicsSettings>, Error> {
    if json.is_null() {
        return Ok(None);
    }
    // SAFETY: 呼び出し側の約束。
    let json = core::str::from_utf8(unsafe { bytes(json, len, "json")? })
        .map_err(|_| Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`json` is not UTF-8"))?;
    ugoe::format::physics_from_json(json)
        .map(Some)
        .map_err(|e| Error::new(UGOE_ERROR_FORMAT, e.to_string()))
}

/// 物理の設定の検査の失敗。戻り値は前と同じ `UGOE_ERROR_INVALID_MODEL` のまま、種類と補足の値は着せ替えの
/// `invalid_physics` と同じ (`errors`・`error_codes`。T0469)。
fn invalid_physics(errors: &[ValidationError]) -> Error {
    Error::new(
        UGOE_ERROR_INVALID_MODEL,
        describe_errors("physics settings", errors),
    )
    .with_kind("invalid_physics", validation_fields(errors))
}

/// 検査の失敗の補足の値 (`errors`・`error_codes`。`ugoe::validation_fields` を 10 進と `, ` でつないだ文字列にする)。
fn validation_fields(errors: &[ValidationError]) -> Vec<(&'static str, String)> {
    ugoe::validation_fields(errors)
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect()
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let settings = p.puppet.physics().map(|physics| physics.settings());
        // SAFETY: 呼び出し側の約束。
        unsafe { write_physics(settings, out, capacity, out_len) }
    })
}

/// 本体の物理の設定 (`ugoe::Puppet::base_physics`) の JSON。書き方は `ugoe_puppet_physics_json` と同じ。
/// 着せ替えのモジュールを付けていれば、合成する前の本体の設定 (付けたことがなければ今の設定)。
/// モデルを読み直すホストが、モジュールのリグを含まない本体の設定を引き継ぐために使う。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は `capacity` バイトを書き込める領域、`out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_base_physics_json(
    puppet: *const UgoePuppet,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_physics(p.puppet.base_physics(), out, capacity, out_len) }
    })
}

/// 物理の設定の JSON を可変長の結果の規約で書く (`ugoe_puppet_physics_json`)。`None` なら長さ 0。
///
/// # Safety
///
/// `out` は `capacity` バイトを書き込める領域、`out_len` は書き込める領域を指すこと。
unsafe fn write_physics(
    settings: Option<&PhysicsSettings>,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> Result<(), Error> {
    let json = match settings {
        Some(settings) => ugoe::format::physics_to_json(settings)
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
    unsafe { core::ptr::copy_nonoverlapping(json.as_ptr(), out, json.len()) };
    Ok(())
}

/// 物理の粒子を初期姿勢に戻す (次の `ugoe_puppet_update` で静止姿勢に置き直す)。物理が無ければ何もしない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_reset_physics(
    puppet: *mut UgoePuppet,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        unsafe { puppet_mut(puppet)? }.puppet.reset_physics();
        Ok(())
    })
}

/// 目パチ・呼吸・リップシンクの設定を JSON (モデルの `effects` と同じ形。UTF-8、`len` バイト。NUL 終端は要らない)
/// で差し替える。省略したフィールドはモデルと同じ既定値になる。
///
/// 再生の状態は初めからになる (目パチは新しい `seed` で目を開けた状態から、呼吸の位相は 0)。
/// リップシンクの音量は保つ (`docs/spec/motion.md#エフェクト設定の差し替え`、`ugoe::Puppet::set_effects`)。
/// 同じ設定を渡しても初めからになる。JSON を読めなければ `UGOE_ERROR_FORMAT` を返し、何も変えない。
///
/// 設定はモデルの読み込みと同じに検査する (`ugoe::Puppet::try_set_effects`。有限でない値・範囲の外の値・モデルにない
/// パラメータ)。通らなければ `UGOE_ERROR_INVALID_MODEL` (`ugoe_error_kind` は `"invalid_model"`、
/// `ugoe_error_field` の `"errors"` は検査のエラーの数、`"error_codes"` は種類を `, ` でつないだもの) を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`json` は `len` バイトの読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_effects_json(
    puppet: *mut UgoePuppet,
    json: *const u8,
    len: usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        if json.is_null() {
            return Err(Error::null("json"));
        }
        // SAFETY: 呼び出し側の約束。
        let json = core::str::from_utf8(unsafe { bytes(json, len, "json")? })
            .map_err(|_| Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`json` is not UTF-8"))?;
        let effects = serde_json::from_str(json)
            .map_err(|e| Error::new(UGOE_ERROR_FORMAT, format!("effects: {e}")))?;
        p.puppet
            .try_set_effects(effects)
            .map_err(|errors| invalid_effects(&errors))
    })
}

/// エフェクトの設定の検査の失敗。種類と補足の値は読み込みの `invalid_model` と同じ (`ugoe::LoadError::fields`)。
fn invalid_effects(errors: &[ValidationError]) -> Error {
    Error::new(UGOE_ERROR_INVALID_MODEL, describe_errors("effects", errors))
        .with_kind("invalid_model", validation_fields(errors))
}
