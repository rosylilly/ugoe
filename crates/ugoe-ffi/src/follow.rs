//! 受け手の口: 届いたパラメータの値を基準値として書き、物理より後の段だけを進める
//! (`ugoe::Puppet::set_base_values`・`follow`。`docs/spec/motion.md#受け手として進める`)。
//! と、指揮役の口: 直前の `ugoe_puppet_update` (か `ugoe_puppet_follow`) が物理の段に入る直前に持っていた値などを読む
//! (`ugoe::Puppet::values_before_physics`・`lip_sync_applied`・`part_opacities_applied`。T0454)。

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use ugoe::{LipSyncInput, ParameterValuesError, PartOpacityInput};

use crate::error::{
    Error, UGOE_ERROR_BUFFER_TOO_SMALL, UGOE_ERROR_INVALID_ARGUMENT, UgoeError, UgoeResult, guard,
};
use crate::puppet::{UgoePuppet, puppet_mut, puppet_ref, write};

/// `values` から `count` 個の `float` を借りる。`count` が 0 なら `values` は NULL でもよい。
///
/// # Safety
///
/// `count` が 0 でなければ、`values` は `count` 個の `float` を読める領域を指すこと。
unsafe fn floats<'a>(values: *const f32, count: usize) -> Result<&'a [f32], Error> {
    if count == 0 {
        return Ok(&[]);
    }
    if values.is_null() {
        return Err(Error::null("values"));
    }
    // SAFETY: 呼び出し側の約束で、`values` は `count` 個を読める。
    Ok(unsafe { core::slice::from_raw_parts(values, count) })
}

/// モーションのパーツの不透明度のカーブが 1 つのパーツに求めた値と重み (`ugoe::PartOpacityInput`。T0135)。
///
/// 受け手は `ugoe_puppet_follow` に、指揮役の `ugoe::Puppet::part_opacities_applied` をそのまま渡す。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UgoePartOpacityInput {
    /// パーツの番号 (`ugoe_puppet_part_index` と同じ番号。指揮役と受け手は同じモデルを使う)。モデルに無い番号は無視する。
    pub part: u32,
    /// カーブの値 (不透明度)。
    pub value: f32,
    /// フェードを含めた重み (0〜1)。
    pub weight: f32,
}

impl From<PartOpacityInput> for UgoePartOpacityInput {
    fn from(p: PartOpacityInput) -> Self {
        Self {
            part: p.part,
            value: p.value,
            weight: p.weight,
        }
    }
}

impl From<UgoePartOpacityInput> for PartOpacityInput {
    fn from(p: UgoePartOpacityInput) -> Self {
        Self {
            part: p.part,
            value: p.value,
            weight: p.weight,
        }
    }
}

fn invalid(e: ParameterValuesError) -> Error {
    Error::new(UGOE_ERROR_INVALID_ARGUMENT, e.to_string())
}

/// パラメータの値 `values` (`count` 個。パラメータの番号の順) を、すべて **基準値として** 書く (範囲に丸める)。
///
/// `ugoe_puppet_set_parameter` で書いた値は、前のフレームの出力と同じなら基準値にならないが、これは同じでも基準値にする。
/// `count` がパラメータの数と違うか NaN を含めば `UGOE_ERROR_INVALID_ARGUMENT` を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`values` は `count` 個の `float` を読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_base_values(
    puppet: *mut UgoePuppet,
    values: *const f32,
    count: usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let values = unsafe { floats(values, count)? };
        p.puppet.set_base_values(values).map_err(invalid)
    })
}

/// 受け手として 1 フレーム進める: `values` (`count` 個) を基準値として書き、物理 → リップシンク → パーツの不透明度
/// (モーションのカーブとポーズ) → 評価の順に `dt` 秒進める (`ugoe_puppet_update` の後半と同じ順。`ugoe::Puppet::follow`)。
///
/// `values` は指揮役の物理の段に入る直前の値 (`ugoe::Puppet::values_before_physics`)、`lip_sync_level` と
/// `lip_sync_strength` は指揮役のリップシンクの音量と強さ (`ugoe::Puppet::lip_sync_applied`)。`lip_sync_level` が負なら
/// リップシンクを当てない。リップシンクはこの `UgoePuppet` のモデルの対象に当て、`ugoe_puppet_set_lip_sync_level` の音量は使わない。
/// `part_opacities` (`part_opacity_count` 個) は指揮役のモーションのパーツの不透明度のカーブ
/// (`ugoe::Puppet::part_opacities_applied`)。無ければ `part_opacity_count` を 0 にする (`part_opacities` は NULL でよい)。
///
/// モーション・トラッキング・表情・目パチ・呼吸は通さない (値を作った指揮役で済んでいるため)。
/// 同じ列を与えれば、物理を含めて同じ形になる。モーションのイベント
/// (`ugoe_puppet_motion_event_count`) は直前の `ugoe_puppet_update` のまま。
/// 呼ぶと、この `UgoePuppet` から借りた頂点位置と描画順のポインタは無効になる。
///
/// `count` がパラメータの数と違う・NaN を含む・リップシンクの音量が NaN・(当てるときの) 強さかパーツの不透明度の値と重みが
/// 有限でない・`dt` が負か有限でないときは `UGOE_ERROR_INVALID_ARGUMENT` を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`values` は `count` 個の `float`、`part_opacities` は `part_opacity_count` 個の
/// `UgoePartOpacityInput` を読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_follow(
    puppet: *mut UgoePuppet,
    values: *const f32,
    count: usize,
    lip_sync_level: f32,
    lip_sync_strength: f32,
    part_opacities: *const UgoePartOpacityInput,
    part_opacity_count: usize,
    dt: f32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::new(
                UGOE_ERROR_INVALID_ARGUMENT,
                format!("`dt` must be a finite non-negative number (got {dt})"),
            ));
        }
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let values = unsafe { floats(values, count)? };
        let lip_sync = (lip_sync_level.is_nan() || lip_sync_level >= 0.0).then_some(LipSyncInput {
            level: lip_sync_level,
            strength: lip_sync_strength,
        });
        // 有限でない音量・強さ・パーツの不透明度は `Puppet::follow` が拒む (前提は SDK が決め、ここは結果コードに写すだけ)。
        let part_opacities: Vec<PartOpacityInput> = if part_opacity_count == 0 {
            Vec::new()
        } else if part_opacities.is_null() {
            return Err(Error::null("part_opacities"));
        } else {
            // SAFETY: 呼び出し側の約束で、`part_opacities` は `part_opacity_count` 個を読める。
            unsafe { core::slice::from_raw_parts(part_opacities, part_opacity_count) }
                .iter()
                .map(|&p| p.into())
                .collect()
        };
        p.puppet
            .follow(values, lip_sync, &part_opacities, dt)
            .map_err(invalid)
    })
}

/// `values` を `out` (`capacity` 個) に写し、数を `*out_count` に書く。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、
/// `*out_count` だけを書く (`ugoe_puppet_tracking_targets` と同じ)。
///
/// # Safety
///
/// `out` は NULL か `capacity` 個の `T` を書き込める領域、`out_count` は書き込める領域を指すこと。
unsafe fn copy_out<S: Copy, T: From<S>>(
    values: &[S],
    out: *mut T,
    capacity: usize,
    out_count: *mut usize,
) -> Result<(), Error> {
    // SAFETY: 呼び出し側の約束。
    unsafe { write(out_count, "out_count", values.len())? };
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
        *dst = v.into();
    }
    Ok(())
}

/// 指揮役の口: 直前の `ugoe_puppet_update` (か `ugoe_puppet_follow`) で、物理の段に入る直前のパラメータの値 (パラメータの番号の順。
/// `ugoe::Puppet::values_before_physics`) を `out` (`capacity` 個) に写し、数 (パラメータの数) を `*out_count` に書く。
/// 受け手の `ugoe_puppet_follow` の `values` にそのまま渡す。
///
/// 物理の段を止めていても同じ位置の値。`ugoe_puppet_update` の前は作ったときの値。`ugoe_puppet_follow` の後は渡した値
/// (値域にクランプしたもの。受け手がさらに別の受け手へ中継できる。後の 2 つの口も同じ)。
/// `capacity` が足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out_count` だけを書く。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は NULL か `capacity` 個の `float` を書き込める領域、
/// `out_count` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_values_before_physics(
    puppet: *const UgoePuppet,
    out: *mut f32,
    capacity: usize,
    out_count: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { copy_out(p.puppet.values_before_physics(), out, capacity, out_count) }
    })
}

/// 指揮役の口: 直前の `ugoe_puppet_update` (か `ugoe_puppet_follow`) でリップシンクの段が当てた音量を `*out_level`、強さ (エフェクトグループ `LipSync`
/// の強さ) を `*out_strength` に書く (`ugoe::Puppet::lip_sync_applied`)。受け手の `ugoe_puppet_follow` の `lip_sync_level` と
/// `lip_sync_strength` にそのまま渡す。
///
/// 段を止めていたか音量が無ければ (当てていなければ) `*out_level` は -1、`*out_strength` は 1 (`ugoe_puppet_follow` は
/// 負の音量を「当てない」とみなす)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out_level` と `out_strength` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_lip_sync_applied(
    puppet: *const UgoePuppet,
    out_level: *mut f32,
    out_strength: *mut f32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        if out_strength.is_null() {
            return Err(Error::null("out_strength"));
        }
        let (level, strength) = p
            .puppet
            .lip_sync_applied()
            .map_or((-1.0, 1.0), |l| (l.level, l.strength));
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_level, "out_level", level)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_strength, "out_strength", strength) }
    })
}

/// 指揮役の口: 直前の `ugoe_puppet_update` (か `ugoe_puppet_follow`) で、モーションのパーツの不透明度のカーブが値を求めたパーツ (カーブのあるパーツだけ。
/// `ugoe::Puppet::part_opacities_applied`) を `out` (`capacity` 個) に写し、数を `*out_count` に書く。受け手の
/// `ugoe_puppet_follow` の `part_opacities` と `part_opacity_count` にそのまま渡す。
///
/// 段の有効無効やポーズとは関係なく、カーブの値と重み。着せ替えの操作の後は、次の `ugoe_puppet_update` まで 0 個。
/// `capacity` が足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out_count` だけを書く。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は NULL か `capacity` 個の `UgoePartOpacityInput` を書き込める領域、
/// `out_count` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_part_opacities_applied(
    puppet: *const UgoePuppet,
    out: *mut UgoePartOpacityInput,
    capacity: usize,
    out_count: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { copy_out(p.puppet.part_opacities_applied(), out, capacity, out_count) }
    })
}
