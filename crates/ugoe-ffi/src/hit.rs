//! 当たり判定 (`docs/spec/evaluation.md#9-当たり判定`) と、アバタープロファイルの標準の当たり判定
//! (`docs/spec/avatar-profile.md#アプリ向けの情報`)。
//!
//! どれも直前の `ugoe_puppet_update` で評価した変形後の形で判定する (不透明度 0 や非表示のメッシュでも当たる)。
//! 座標はキャンバス座標 (ピクセル単位・Y 下向き)。

use alloc::format;
use core::ffi::c_char;

use ugoe::core::Vec2;

use crate::error::{
    Error, UGOE_ERROR_BUFFER_TOO_SMALL, UGOE_ERROR_NOT_FOUND, UgoeError, UgoeResult, guard,
};
use crate::puppet::{UgoePuppet, c_str, puppet_ref, write};

/// 当たり判定 (`Model::hit_areas`) の数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_hit_area_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.hit_area_ids.len() as u32) }
    })
}

/// `index` 番目の当たり判定の ID (UTF-8、NUL 終端)。ポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_hit_area_id(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let id = p
            .hit_area_ids
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("hit area", index as usize, p.hit_area_ids.len()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", id.as_ptr()) }
    })
}

/// 点 `(x, y)` を含む当たり判定の番号を、手前から順に `out` (`capacity` 個) に書く。`*out_count` には
/// 当たった数を書く。`capacity` が足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out_count` だけを書く
/// (数を知るには `capacity` を 0 にして呼ぶ。そのとき `out` は NULL でもよい)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は `capacity` 個の `uint32_t` を書き込める領域、
/// `out_count` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_hit_areas_at(
    puppet: *const UgoePuppet,
    x: f32,
    y: f32,
    out: *mut u32,
    capacity: usize,
    out_count: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let hits = p.puppet.instance().hit_areas_at(Vec2::new(x, y));
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_count, "out_count", hits.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_indices(&hits, out, capacity) }
    })
}

/// 番号の並びをホストのバッファに書く。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL`。
///
/// # Safety
///
/// `out` は NULL か、`capacity` 個の `uint32_t` を書き込める領域を指すこと。
pub(crate) unsafe fn write_indices(
    values: &[usize],
    out: *mut u32,
    capacity: usize,
) -> Result<(), Error> {
    if capacity < values.len() {
        return Err(Error::new(
            UGOE_ERROR_BUFFER_TOO_SMALL,
            format!(
                "`out` holds {capacity} values but {} are needed",
                values.len()
            ),
        ));
    }
    if values.is_empty() {
        return Ok(());
    }
    if out.is_null() {
        return Err(Error::null("out"));
    }
    // SAFETY: `out` は `capacity` (≥ values.len()) 個を書き込める (呼び出し側の約束)。
    let out = unsafe { core::slice::from_raw_parts_mut(out, values.len()) };
    for (dst, &v) in out.iter_mut().zip(values) {
        *dst = v as u32;
    }
    Ok(())
}

/// ID が `id` (UTF-8、NUL 終端) の当たり判定が点 `(x, y)` を含むか。知らない ID なら `UGOE_ERROR_NOT_FOUND`。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`id` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_hit_test(
    puppet: *const UgoePuppet,
    id: *const c_char,
    x: f32,
    y: f32,
    out: *mut bool,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let id = unsafe { c_str(id, "id")? };
        let instance = p.puppet.instance();
        let index = instance
            .compiled()
            .hit_area_index(id)
            .ok_or_else(|| Error::new(UGOE_ERROR_NOT_FOUND, format!("no hit area `{id}`")))?;
        let hit = instance.hit_area_contains(index, Vec2::new(x, y));
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", hit) }
    })
}

/// 標準の当たり判定の名前 `name` (`"Head"`・`"Body"`。`Model.avatar.hit_areas`) に対応する当たり判定の番号。
/// 対応が無ければ `UGOE_ERROR_NOT_FOUND`。ID は `ugoe_puppet_hit_area_id` で得る。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`name` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_standard_hit_area(
    puppet: *const UgoePuppet,
    name: *const c_char,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let name = unsafe { c_str(name, "name")? };
        let index = p
            .puppet
            .standard_hit_area(name)
            .and_then(|id| p.puppet.instance().compiled().hit_area_index(id))
            .ok_or_else(|| {
                Error::new(
                    UGOE_ERROR_NOT_FOUND,
                    format!("no hit area for the standard name `{name}`"),
                )
            })?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", index as u32) }
    })
}

/// 標準の名前 `name` の当たり判定が点 `(x, y)` を含むか。対応が無ければ false (エラーにしない。
/// `ugoe::Puppet::hit_test_standard` と同じ)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`name` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_hit_test_standard(
    puppet: *const UgoePuppet,
    name: *const c_char,
    x: f32,
    y: f32,
    out: *mut bool,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let name = unsafe { c_str(name, "name")? };
        let hit = p.puppet.hit_test_standard(name, Vec2::new(x, y));
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", hit) }
    })
}
