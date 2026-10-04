//! 受け手の口: 届いたパラメータの値を基準値として書き、物理より後の段だけを進める
//! (`ugoe::Puppet::set_base_values`・`follow`。`docs/spec/motion.md#受け手として進める`)。

use ugoe::{LipSyncInput, ParameterValuesError, PartOpacityInput};

use crate::error::{Error, UGOE_ERROR_INVALID_ARGUMENT, UgoeResult, guard};
use crate::puppet::{UgoePuppet, puppet_mut};

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
    Ok(unsafe { std::slice::from_raw_parts(values, count) })
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
) -> UgoeResult {
    guard(|| {
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
) -> UgoeResult {
    guard(|| {
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
        // NaN は `Puppet::follow` が拒む。無限大の強さ・値・重みは 0 × ∞ で NaN を書きうるので、ここで拒む。
        if lip_sync.is_some() && lip_sync_strength.is_infinite() {
            return Err(Error::new(
                UGOE_ERROR_INVALID_ARGUMENT,
                format!("`lip_sync_strength` must be finite (got {lip_sync_strength})"),
            ));
        }
        let part_opacities: Vec<PartOpacityInput> = if part_opacity_count == 0 {
            Vec::new()
        } else if part_opacities.is_null() {
            return Err(Error::null("part_opacities"));
        } else {
            // SAFETY: 呼び出し側の約束で、`part_opacities` は `part_opacity_count` 個を読める。
            unsafe { std::slice::from_raw_parts(part_opacities, part_opacity_count) }
                .iter()
                .map(|&p| p.into())
                .collect()
        };
        if let Some(i) = part_opacities
            .iter()
            .position(|p| p.value.is_infinite() || p.weight.is_infinite())
        {
            return Err(Error::new(
                UGOE_ERROR_INVALID_ARGUMENT,
                format!("`part_opacities[{i}]` must be finite"),
            ));
        }
        p.puppet
            .follow(values, lip_sync, &part_opacities, dt)
            .map_err(invalid)
    })
}
