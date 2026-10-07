//! ランタイム層が使う `f32` の数学関数。
//!
//! 中身は `std` の有無にかかわらず常に `libm` (純粋な Rust の実装) を使う。`f32::sin` などは `std` にしか無く、
//! しかも OS の C の数学ライブラリによって結果のビット列が変わりうる。ここを通せば、`std` の有無でも OS でも
//! 同じ入力から同じ出力が得られる (`docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる`)。
//! `glam` も feature `libm` で同じ実装を使う。
//!
//! ランタイム層のクレートでは、`std` の `f32`・`f64` の数学関数を直接呼ぶと clippy の `disallowed_methods` が警告する
//! (各クレートの `clippy.toml`)。`abs`・`signum`・`copysign`・`to_radians`・`min`・`max`・`clamp` などは `core` にあり、
//! 結果も IEEE で決まるので、そのまま使ってよい。
//!
//! `sqrt`・`floor`・`ceil` などは IEEE で結果が決まるので、どの実装でも値は同じ。`std` の無い環境で使えるようにするためだけに置く。

/// 正弦 (ラジアン)。
#[inline]
pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

/// 余弦 (ラジアン)。
#[inline]
pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

/// 正弦と余弦 (ラジアン) を同時に求める。`(sin, cos)` の順。
#[inline]
pub fn sin_cos(x: f32) -> (f32, f32) {
    libm::sincosf(x)
}

/// 正接 (ラジアン)。
#[inline]
pub fn tan(x: f32) -> f32 {
    libm::tanf(x)
}

/// 逆正接。`y / x` の角度を象限を区別して `[-π, π]` で返す (`f32::atan2` と同じ引数の順)。
#[inline]
pub fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

/// 平方根。
#[inline]
pub fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

/// `sqrt(x * x + y * y)` を途中の桁あふれ無しに求める。
#[inline]
pub fn hypot(x: f32, y: f32) -> f32 {
    libm::hypotf(x, y)
}

/// `x` の `y` 乗。
#[inline]
pub fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}

/// `e` の `x` 乗。
#[inline]
pub fn exp(x: f32) -> f32 {
    libm::expf(x)
}

/// 自然対数。
#[inline]
pub fn ln(x: f32) -> f32 {
    libm::logf(x)
}

/// 負の無限大の向きへの丸め。
#[inline]
pub fn floor(x: f32) -> f32 {
    libm::floorf(x)
}

/// 正の無限大の向きへの丸め。
#[inline]
pub fn ceil(x: f32) -> f32 {
    libm::ceilf(x)
}

/// 最も近い整数への丸め (ちょうど中間は 0 から遠い向き。`f32::round` と同じ)。
#[inline]
pub fn round(x: f32) -> f32 {
    libm::roundf(x)
}

/// 0 の向きへの丸め。
#[inline]
pub fn trunc(x: f32) -> f32 {
    libm::truncf(x)
}

/// ユークリッドの剰余。`rhs` が正なら結果は `[0, rhs)` (丸めで `rhs` になることはある)。`f32::rem_euclid` と同じ。
#[inline]
pub fn rem_euclid(x: f32, rhs: f32) -> f32 {
    // `%` (fmod) は IEEE で結果が決まり、`core` で使える。式は std の `f32::rem_euclid` と同じ
    let r = x % rhs;
    if r < 0.0 { r + rhs.abs() } else { r }
}

/// `f64` のユークリッドの剰余 ([`rem_euclid`] と同じ。長く動く時間の累積に使う)。
#[inline]
pub fn rem_euclid_f64(x: f64, rhs: f64) -> f64 {
    let r = x % rhs;
    if r < 0.0 { r + rhs.abs() } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn trigonometry() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert!((sin(FRAC_PI_2) - 1.0).abs() < 1e-7);
        assert!((cos(PI) + 1.0).abs() < 1e-7);
        assert!((tan(PI / 4.0) - 1.0).abs() < 1e-6);
        for k in -50..50 {
            let x = k as f32 * 0.37;
            assert_eq!(sin_cos(x), (sin(x), cos(x)));
        }
        assert!((atan2(1.0, 0.0) - FRAC_PI_2).abs() < 1e-7);
        assert!((atan2(0.0, -1.0) - PI).abs() < 1e-7);
        assert_eq!(atan2(0.0, 1.0), 0.0);
    }

    #[test]
    fn powers_and_roots() {
        assert_eq!(sqrt(9.0), 3.0);
        assert!(sqrt(-1.0).is_nan());
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert!(hypot(f32::MAX, f32::MAX).is_infinite());
        assert!((hypot(1e30, 1e30) / 1e30 - core::f32::consts::SQRT_2).abs() < 1e-6);
        assert_eq!(powf(2.0, 10.0), 1024.0);
        assert_eq!(powf(0.5, 0.0), 1.0);
        assert_eq!(exp(0.0), 1.0);
        assert_eq!(ln(1.0), 0.0);
        assert!((ln(exp(2.0)) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn rounding() {
        assert_eq!(floor(-1.5), -2.0);
        assert_eq!(ceil(-1.5), -1.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -3.0);
        assert_eq!(trunc(-1.7), -1.0);
    }

    #[test]
    fn rem_euclid_matches_definition() {
        assert_eq!(rem_euclid(7.0, 3.0), 1.0);
        assert_eq!(rem_euclid(-1.0, 3.0), 2.0);
        assert_eq!(rem_euclid(-1.0, -3.0), 2.0);
        assert_eq!(rem_euclid(370.0, 360.0), 10.0);
        assert!(rem_euclid(f32::NAN, 1.0).is_nan());
        assert!(rem_euclid(1.0, 0.0).is_nan());
        assert_eq!(rem_euclid_f64(-1.0, 3.0), 2.0);
        assert_eq!(rem_euclid_f64(1e12 + 0.25, 0.5), 0.25);
    }

    /// std があるときは std の結果とも比べる。IEEE で結果が決まる関数はビット単位で同じ。
    /// 超越関数は OS の数学ライブラリによって最後の桁が変わりうるので、1 ULP まで許す。
    #[cfg(feature = "std")]
    #[test]
    #[allow(clippy::disallowed_methods)]
    fn agrees_with_std() {
        extern crate std;
        fn ulps(a: f32, b: f32) -> u32 {
            (a.to_bits() as i64 - b.to_bits() as i64).unsigned_abs() as u32
        }
        for k in -2000..2000 {
            let x = k as f32 * 0.0137;
            assert_eq!(sqrt(x.abs()).to_bits(), x.abs().sqrt().to_bits());
            assert_eq!(floor(x).to_bits(), x.floor().to_bits());
            assert_eq!(ceil(x).to_bits(), x.ceil().to_bits());
            assert_eq!(round(x).to_bits(), x.round().to_bits());
            assert_eq!(trunc(x).to_bits(), x.trunc().to_bits());
            assert_eq!(rem_euclid(x, 3.5).to_bits(), x.rem_euclid(3.5).to_bits());
            let y = f64::from(x) * 1e5;
            assert_eq!(
                rem_euclid_f64(y, 0.7).to_bits(),
                y.rem_euclid(0.7).to_bits()
            );
            assert!(ulps(sin(x), x.sin()) <= 1, "sin({x})");
            assert!(ulps(cos(x), x.cos()) <= 1, "cos({x})");
            assert!(ulps(atan2(x, 0.7), x.atan2(0.7)) <= 1, "atan2({x})");
            assert!(ulps(hypot(x, 0.7), x.hypot(0.7)) <= 1, "hypot({x})");
            assert!(ulps(exp(x * 0.1), (x * 0.1).exp()) <= 1, "exp({x})");
            assert!(
                ulps(powf(x.abs(), 0.37), x.abs().powf(0.37)) <= 1,
                "powf({x})"
            );
        }
    }
}
