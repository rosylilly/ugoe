//! パーツの一覧・パーツの不透明度の上書き・ポーズの表示の選択 (T0414)。評価後のパーツの不透明度 (T0416)。

use std::ffi::c_char;

use crate::error::{Error, UGOE_ERROR_NOT_FOUND, UgoeResult, guard};
use crate::puppet::{UgoePuppet, c_str, puppet_mut, puppet_ref, write};

/// パーツ自身の不透明度の上書き (`docs/spec/motion.md#パーツの不透明度`)。
///
/// 評価ではパーツ自身の不透明度 `o` (キーフォームの補間) を `lerp(o, value, weight)` にしてから子へ伝える。
/// `weight` が 0 なら上書きしない (そのとき `value` に意味は無い)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoePartOpacity {
    pub value: f32,
    pub weight: f32,
}

/// パーツ (`Model::parts`) の数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.part_ids.len() as u32) }
    })
}

/// `index` 番目のパーツの ID (UTF-8、NUL 終端)。ポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_id(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let id = p
            .part_ids
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("part", index as usize, p.part_ids.len()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", id.as_ptr()) }
    })
}

/// ID (UTF-8、NUL 終端) からパーツの番号を引く。無ければ `UGOE_ERROR_NOT_FOUND`。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`id` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_index(
    puppet: *const UgoePuppet,
    id: *const c_char,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let id = unsafe { c_str(id, "id")? };
        let index = p
            .puppet
            .instance()
            .compiled()
            .part_index(id)
            .ok_or_else(|| Error::new(UGOE_ERROR_NOT_FOUND, format!("no part `{id}`")))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", index as u32) }
    })
}

/// 直前の `ugoe_puppet_update` が `index` 番目のパーツに書いた不透明度の上書き (モーションのパーツの不透明度の
/// カーブとポーズの結果。`docs/spec/motion.md#フレーム内の適用順`)。評価に渡した値で、公開仕様の時間の例の
/// `part_opacity` と同じもの。上書きしていなければ `weight` は 0。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_opacity(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoePartOpacity,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let len = p.part_ids.len();
        if index as usize >= len {
            return Err(Error::out_of_range("part", index as usize, len));
        }
        let (value, weight) = p.puppet.instance().part_opacity_override(index as usize);
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", UgoePartOpacity { value, weight }) }
    })
}

/// 直前の評価 (`ugoe_puppet_update` か着せ替えの操作) での `index` 番目のパーツの不透明度。上書きを当てた自身の値に
/// 祖先の値を掛けたもので、そのパーツのメッシュの不透明度に掛かる (`docs/spec/evaluation.md` の 2.、
/// `ugoe::runtime::ModelInstance::part_opacity`)。上書きの値は `ugoe_puppet_part_opacity`。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_evaluated_opacity(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut f32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let len = p.part_ids.len();
        if index as usize >= len {
            return Err(Error::out_of_range("part", index as usize, len));
        }
        let opacity = p.puppet.instance().part_opacity(index as usize);
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", opacity) }
    })
}

/// ポーズで、`part` (UTF-8、NUL 終端) を `part` に持つ要素をそのグループの表示中の要素にする
/// (`links` のパーツは対象外)。不透明度は次の `ugoe_puppet_update` からフェードで近づく。
/// そのフレームにモーションが同じグループの要素を 0 より大きい値で要求すれば、要求が勝つ
/// (`docs/spec/motion.md#ポーズ`、`ugoe::motion::PosePlayer::show`)。
///
/// どの要素の `part` でもなければ何も変えず、`*out_shown` に false を書く (エラーにはしない。
/// `out_shown` は NULL でもよい)。ポーズの段を止めている間も表示中の要素は変わる。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`part` は NUL 終端の文字列、`out_shown` は NULL か書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_show_pose(
    puppet: *mut UgoePuppet,
    part: *const c_char,
    out_shown: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let part = unsafe { c_str(part, "part")? };
        let shown = p.puppet.pose_mut().show(&part.into());
        if !out_shown.is_null() {
            // SAFETY: 呼び出し側の約束。
            unsafe { out_shown.write(shown) };
        }
        Ok(())
    })
}
