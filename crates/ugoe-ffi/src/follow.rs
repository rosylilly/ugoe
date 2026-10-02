//! 受け手の口: 届いたパラメータの値を基準値として書き、物理だけを進める
//! (`ugoe::Puppet::set_base_values`・`follow`。`docs/spec/motion.md#受け手として進める`)。

use ugoe::ParameterValuesError;

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

/// 受け手として 1 フレーム進める: `values` (`count` 個) を基準値として書き、物理とポーズの段だけを `dt` 秒進めて評価する。
///
/// モーション・トラッキング・表情・目パチ・呼吸・リップシンクは通さない (値を作った指揮役で済んでいるため)。
/// 同じ `values` と `dt` の列を与えれば、物理を含めて同じ形になる。モーションのイベント
/// (`ugoe_puppet_motion_event_count`) は直前の `ugoe_puppet_update` のまま。
/// 呼ぶと、この `UgoePuppet` から借りた頂点位置と描画順のポインタは無効になる。
///
/// `count` がパラメータの数と違うか NaN を含むか、`dt` が負か有限でなければ `UGOE_ERROR_INVALID_ARGUMENT` を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`values` は `count` 個の `float` を読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_follow(
    puppet: *mut UgoePuppet,
    values: *const f32,
    count: usize,
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
        p.puppet.follow(values, dt).map_err(invalid)
    })
}
