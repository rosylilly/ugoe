//! フェイストラッキングの入力 (`docs/spec/avatar-profile.md#フェイストラッキングとの対応`)。
//!
//! 平滑化・キャリブレーション・回転行列からの角度の計算・鏡像の入れ替えはホストが行う
//! (`docs/spec/avatar-profile.md#実装の範囲`)。

use std::ffi::{CString, c_char};
use std::sync::OnceLock;

use ugoe::model::avatar::ARKIT_BLEND_SHAPES;
use ugoe::motion::tracking::{BLEND_SHAPE_COUNT, blend_shape_index};
use ugoe::motion::{BodyInput, Rotation, TrackingInput, TrackingMap};

use crate::error::{
    Error, UGOE_ERROR_INVALID_ARGUMENT, UGOE_ERROR_NOT_FOUND, UgoeResult, c_string, guard,
};
use crate::hit::write_indices;
use crate::puppet::{UgoePuppet, c_str, puppet_mut, puppet_ref, write};

/// ARKit のブレンドシェイプの数 (`UgoeTrackingInput::blend_shapes` の長さ)。
pub const UGOE_BLEND_SHAPE_COUNT: usize = 52;

/// 1 フレーム分のトラッキングの入力 (`ugoe::motion::TrackingInput`)。
///
/// 回転は度で、符号はアバタープロファイルの向きの約束に合わせる (`docs/spec/avatar-profile.md#向きと左右の約束`)。
/// 体の値は NaN なら省略 (体が映っていない・信頼度が低い)。省略した体の回転は頭の回転 × 0.3 を使い、
/// 省略した肩のパラメータには書き込まない。ブレンドシェイプの `Left` / `Right` はトラッキングされた人自身の左右。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeTrackingInput {
    /// 頭のヨー (正でキャラクターから見て左を向く。`AngleX` の向き)。
    pub head_yaw: f32,
    /// 頭のピッチ (正で上を向く。`AngleY` の向き)。
    pub head_pitch: f32,
    /// 頭のロール (正で時計回り。`AngleZ` の向き)。
    pub head_roll: f32,
    /// 体のヨー (`BodyAngleX` の向き)。NaN なら省略。
    pub body_yaw: f32,
    /// 体のピッチ (`BodyAngleY` の向き)。NaN なら省略。
    pub body_pitch: f32,
    /// 体のロール (`BodyAngleZ` の向き)。NaN なら省略。
    pub body_roll: f32,
    /// 左肩の上下 (0 が中立、1 ですくめた状態、負で下げた状態。`ShoulderLY`)。NaN なら省略。
    pub shoulder_left: f32,
    /// 右肩の上下 (`ShoulderRY`)。NaN なら省略。
    pub shoulder_right: f32,
    /// ARKit のブレンドシェイプの値 (0〜1)。添字は `ugoe_blend_shape_name` の順。
    pub blend_shapes: [f32; UGOE_BLEND_SHAPE_COUNT],
}

fn invalid(name: &str, value: f32) -> Error {
    Error::new(
        UGOE_ERROR_INVALID_ARGUMENT,
        format!("`{name}` must be finite (got {value})"),
    )
}

fn finite(name: &str, value: f32) -> Result<f32, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid(name, value))
    }
}

/// NaN なら省略。無限大は誤り。
fn optional(name: &str, value: f32) -> Result<Option<f32>, Error> {
    if value.is_nan() {
        Ok(None)
    } else {
        finite(name, value).map(Some)
    }
}

impl UgoeTrackingInput {
    fn input(&self) -> Result<TrackingInput, Error> {
        const { assert!(UGOE_BLEND_SHAPE_COUNT == BLEND_SHAPE_COUNT) };
        let mut blend_shapes = [0.0; BLEND_SHAPE_COUNT];
        for (i, (dst, &v)) in blend_shapes.iter_mut().zip(&self.blend_shapes).enumerate() {
            if !v.is_finite() {
                return Err(invalid(ARKIT_BLEND_SHAPES[i], v));
            }
            *dst = v;
        }
        Ok(TrackingInput {
            head: Rotation {
                yaw: finite("head_yaw", self.head_yaw)?,
                pitch: finite("head_pitch", self.head_pitch)?,
                roll: finite("head_roll", self.head_roll)?,
            },
            body: BodyInput {
                yaw: optional("body_yaw", self.body_yaw)?,
                pitch: optional("body_pitch", self.body_pitch)?,
                roll: optional("body_roll", self.body_roll)?,
                shoulder_left: optional("shoulder_left", self.shoulder_left)?,
                shoulder_right: optional("shoulder_right", self.shoulder_right)?,
            },
            blend_shapes,
        })
    }
}

/// フェイストラッキングの入力を設定する。`input` が NULL なら入力をやめる (トラッキングでパラメータに触らない)。
///
/// 入力は次の `ugoe_puppet_update` から毎フレーム、モーションの直後にモデルの `avatar.tracking` に従って
/// パラメータに書き込む。書き込んだ値は基準値に含まれるので、表情・目パチ・呼吸はその上に重なる
/// (トラッキング中は `ugoe_puppet_set_stage_enabled` で目パチを止めるとよい)。入力をやめても、最後に書き込んだ値は残る。
/// 値は呼び出しの間だけ読む。頭の回転とブレンドシェイプが有限でなければ `UGOE_ERROR_INVALID_ARGUMENT`。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`input` は NULL か読める `UgoeTrackingInput` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_tracking(
    puppet: *mut UgoePuppet,
    input: *const UgoeTrackingInput,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let input = unsafe { input.as_ref() }
            .map(UgoeTrackingInput::input)
            .transpose()?;
        // SAFETY: 呼び出し側の約束。
        unsafe { puppet_mut(puppet)? }.puppet.set_tracking(input);
        Ok(())
    })
}

/// トラッキングが書き込むパラメータの番号 (昇順) を `out` (`capacity` 個) に書き、数を `*out_count` に書く。
/// 足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out_count` だけを書く (`ugoe_puppet_hit_areas_at` と同じ)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は `capacity` 個の `uint32_t` を書き込める領域、
/// `out_count` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_tracking_targets(
    puppet: *const UgoePuppet,
    out: *mut u32,
    capacity: usize,
    out_count: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let instance = p.puppet.instance();
        let map = TrackingMap::for_model(instance.compiled().model(), instance.parameters());
        let mut targets: Vec<usize> = map.targets().map(|i| i.0 as usize).collect();
        targets.sort_unstable();
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_count, "out_count", targets.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_indices(&targets, out, capacity) }
    })
}

/// トラッキング中に、再生中のモーションが動かすパラメータではモーションを優先するか (既定は `false`。T0722)。
///
/// `true` の間、`ugoe_puppet_update` はトラッキングの値をモーションの前に書き込み、モーションがフェードの重みで
/// その上に重なる。モーションが動かさないパラメータと `ugoe_puppet_set_tracking_kept` で選んだパラメータは
/// トラッキングの値のまま。`false` なら仕様の適用順 (モーション → トラッキング)
/// (`docs/spec/motion.md#トラッキング中のモーションの優先`)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_motion_over_tracking(
    puppet: *mut UgoePuppet,
    enabled: bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        unsafe { puppet_mut(puppet)? }
            .puppet
            .set_motion_over_tracking(enabled);
        Ok(())
    })
}

/// `ugoe_puppet_set_motion_over_tracking` で設定した値を `*out` に書く。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_over_tracking(
    puppet: *const UgoePuppet,
    out: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let enabled = unsafe { puppet_ref(puppet)? }.puppet.motion_over_tracking();
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", enabled) }
    })
}

/// モーションを優先しても (`ugoe_puppet_set_motion_over_tracking`)、トラッキングのままにするパラメータを、
/// パラメータの番号 `indices` (`count` 個) で設定する (目は除く、など。既定は無し)。`count` が 0 なら無しに戻す
/// (`indices` は NULL でもよい)。範囲外の番号があれば `UGOE_ERROR_OUT_OF_RANGE` を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`indices` は `count` 個の `uint32_t` を読める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_tracking_kept(
    puppet: *mut UgoePuppet,
    indices: *const u32,
    count: usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let indices: &[u32] = if count == 0 {
            &[]
        } else if indices.is_null() {
            return Err(Error::null("indices"));
        } else {
            // SAFETY: 呼び出し側の約束で、`indices` は `count` 個を読める。
            unsafe { std::slice::from_raw_parts(indices, count) }
        };
        let parameters = &p.puppet.instance().compiled().model().parameters;
        let ids = indices
            .iter()
            .map(|&i| {
                parameters
                    .get(i as usize)
                    .map(|param| param.id.as_str().to_owned())
                    .ok_or_else(|| Error::out_of_range("parameter", i as usize, parameters.len()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        p.puppet.set_tracking_kept(ids);
        Ok(())
    })
}

fn blend_shape_names() -> &'static [CString] {
    static NAMES: OnceLock<Vec<CString>> = OnceLock::new();
    NAMES.get_or_init(|| ARKIT_BLEND_SHAPES.iter().map(|n| c_string(n)).collect())
}

/// `index` 番目の ARKit のブレンドシェイプの名前 (UTF-8、NUL 終端。`"eyeBlinkLeft"` など)。
/// ポインタはずっと有効。
///
/// # Safety
///
/// `out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_blend_shape_name(index: u32, out: *mut *const c_char) -> UgoeResult {
    guard(|| {
        let names = blend_shape_names();
        let name = names
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("blend shape", index as usize, names.len()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", name.as_ptr()) }
    })
}

/// 名前 (UTF-8、NUL 終端) から ARKit のブレンドシェイプの添字を引く。MediaPipe Face Landmarker の
/// カテゴリ名はそのまま使える。知らない名前 (`_neutral` など) なら `UGOE_ERROR_NOT_FOUND`。
///
/// # Safety
///
/// `name` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_blend_shape_index(name: *const c_char, out: *mut u32) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let name = unsafe { c_str(name, "name")? };
        let index = blend_shape_index(name)
            .ok_or_else(|| Error::new(UGOE_ERROR_NOT_FOUND, format!("no blend shape `{name}`")))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", index as u32) }
    })
}
