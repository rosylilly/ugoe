//! ライティング: ライティングを受けるか、光源の検査とシェーダに渡す形への変換
//! (`docs/spec/rendering.md#ライティング`・`#ホストでのライティング`)。
//!
//! 光源はホストの状態 (`ugoe::lighting::Lighting` と同じ中身) で、`UgoePuppet` は持たない。
//! 法線マップは `ugoe_puppet_normal_map` で引く。

use ugoe::lighting::MAX_LIGHTS;

use crate::error::{Error, UGOE_ERROR_INVALID_ARGUMENT, UgoeResult, guard};
use crate::puppet::{UgoePuppet, puppet_ref, write};

/// 平行光源 (`ugoe::lighting::Light::Directional`)。
pub const UGOE_LIGHT_DIRECTIONAL: u32 = 0;
/// 点光源 (`ugoe::lighting::Light::Point`)。
pub const UGOE_LIGHT_POINT: u32 = 1;

/// 参照レンダラが使う光源の数の上限 (`ugoe::lighting::MAX_LIGHTS`)。
pub const UGOE_MAX_LIGHTS: usize = 8;

/// 光源 1 つ。座標はキャンバス座標 (ピクセル単位・Y 下向き) + 奥行き (z、キャンバスから手前向き)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeLight {
    /// `UGOE_LIGHT_*`。
    pub kind: u32,
    /// 平行光源では光が進む向き (長さは問わない。0 は誤り)、点光源では位置。
    pub vector: [f32; 3],
    /// 色 (RGB。1 を超えてもよい。負は誤り)。
    pub color: [f32; 3],
    /// 点光源の、明るさが 0 になる距離 (正。`INFINITY` なら減衰しない)。平行光源では使わない。
    pub range: f32,
}

/// シェーダに渡す光源 1 つ (参照シェーダの `Light` と同じ配置)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UgoeLightUniform {
    /// 平行光源: xyz = 光源へ向かう単位ベクトル、w = 0。点光源: xyz = 位置、w = 1。
    pub vector: [f32; 4],
    /// rgb = 色、a = 点光源の `1 / range` (減衰しない点光源と平行光源は 0)。
    pub color: [f32; 4],
}

/// シェーダに渡す光源の組 (参照シェーダの `Lighting` ユニフォームと同じ配置。std140 でもそのまま使える)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UgoeLightingUniform {
    /// rgb = 環境光、a = 0。
    pub ambient: [f32; 4],
    /// 使う光源の数 (`lights` の先頭から)。
    pub count: u32,
    pub padding: [u32; 3],
    pub lights: [UgoeLightUniform; UGOE_MAX_LIGHTS],
}

fn invalid(message: String) -> Error {
    Error::new(UGOE_ERROR_INVALID_ARGUMENT, message)
}

fn check_color(what: &str, [r, g, b]: [f32; 3]) -> Result<(), Error> {
    if [r, g, b].iter().all(|c| c.is_finite() && *c >= 0.0) {
        Ok(())
    } else {
        Err(invalid(format!(
            "{what}: the color must be finite and non-negative, got [{r}, {g}, {b}]"
        )))
    }
}

/// 光源 1 つを検査して、シェーダに渡す形にする。
fn light_uniform(i: usize, light: &UgoeLight) -> Result<UgoeLightUniform, Error> {
    let what = format!("lights[{i}]");
    check_color(&what, light.color)?;
    let [r, g, b] = light.color;
    let [x, y, z] = light.vector;
    match light.kind {
        UGOE_LIGHT_DIRECTIONAL => {
            let length = (x * x + y * y + z * z).sqrt();
            if !(length.is_finite() && length > 0.0) {
                return Err(invalid(format!(
                    "{what}: the direction must be finite and non-zero"
                )));
            }
            Ok(UgoeLightUniform {
                vector: [-x / length, -y / length, -z / length, 0.0],
                color: [r, g, b, 0.0],
            })
        }
        UGOE_LIGHT_POINT => {
            if !light.vector.iter().all(|v| v.is_finite()) {
                return Err(invalid(format!("{what}: the position must be finite")));
            }
            let range = light.range;
            if range.is_nan() || range <= 0.0 {
                return Err(invalid(format!(
                    "{what}: range must be positive, got {range}"
                )));
            }
            Ok(UgoeLightUniform {
                vector: [x, y, z, 1.0],
                color: [r, g, b, 1.0 / range],
            })
        }
        other => Err(invalid(format!("{what}: unknown light kind {other}"))),
    }
}

/// 光源の組を検査し、シェーダに渡す形 (`UgoeLightingUniform`) にする。陰影の式は
/// `docs/spec/rendering.md#ライティング` (参照シェーダと同じ)。
///
/// `ambient` は環境光 (RGB の 3 つの float)。NULL なら白 (1, 1, 1)。`lights` は `count` 個の光源 (`count` が 0 なら NULL でもよい)。
/// 光源を渡さないときは環境光が白で光源なしとする (ライティングを受ける要素も、受けない場合と同じ色になる)。
///
/// 向きが 0・有限でない値・`range` が正でない・負の色・知らない種類は `UGOE_ERROR_INVALID_ARGUMENT`
/// (参照レンダラは使えない光源を黙って捨てるが、ここでは先に知らせる)。`UGOE_MAX_LIGHTS` を超える光源は、先頭の
/// `UGOE_MAX_LIGHTS` 個だけを使う (参照レンダラと同じ)。
///
/// # Safety
///
/// `ambient` は NULL か 3 つの float を、`lights` は `count` 個の `UgoeLight` を読める領域を、
/// `out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_lighting_uniform(
    ambient: *const f32,
    lights: *const UgoeLight,
    count: usize,
    out: *mut UgoeLightingUniform,
) -> UgoeResult {
    const { assert!(UGOE_MAX_LIGHTS == MAX_LIGHTS) };
    guard(|| {
        let ambient = if ambient.is_null() {
            [1.0; 3]
        } else {
            // SAFETY: 呼び出し側の約束で、`ambient` は 3 つの float を読める。
            unsafe { ambient.cast::<[f32; 3]>().read_unaligned() }
        };
        check_color("ambient", ambient)?;
        let lights: &[UgoeLight] = if count == 0 {
            &[]
        } else if lights.is_null() {
            return Err(Error::null("lights"));
        } else {
            // SAFETY: 呼び出し側の約束で、`lights` は `count` 個の `UgoeLight` を読める。
            unsafe { std::slice::from_raw_parts(lights, count) }
        };
        let mut uniform = UgoeLightingUniform {
            ambient: [ambient[0], ambient[1], ambient[2], 0.0],
            ..Default::default()
        };
        for (i, light) in lights.iter().enumerate() {
            let light = light_uniform(i, light)?;
            if i < UGOE_MAX_LIGHTS {
                uniform.lights[i] = light;
                uniform.count += 1;
            }
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", uniform) }
    })
}

/// このモデルにライティングを受けるメッシュがあるか (`Model::lighting` と `Mesh::lit` の両方が true。
/// `ugoe::Puppet::uses_lighting`)。光源の操作を出すかの判断に使う。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_uses_lighting(
    puppet: *const UgoePuppet,
    out: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.puppet.uses_lighting()) }
    })
}

/// `index` 番目の描画可能要素がライティングを受けるか (`Model::lighting` と `Mesh::lit` の両方が true)。
/// 受けない要素には陰影を掛けない。受ける要素は、テクスチャの法線マップ (`ugoe_puppet_normal_map`) を読むか、
/// 無ければ法線 `(0, 0, 1)` として陰影を掛ける。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_lit(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let model = p.puppet.instance().compiled().model();
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", model.lighting && model.meshes[i].lit) }
    })
}
