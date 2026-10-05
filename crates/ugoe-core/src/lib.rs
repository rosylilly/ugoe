//! ugoe 全クレートが共有する最下層の基盤型。
//!
//! このクレートは **依存ゼロに近い状態を保つ** こと。GPU・I/O・シリアライズ形式に
//! 依存するものは置かない (serde の derive のみ許可)。
//!
//! 例外として、コード生成 (TypeScript の型と JSON Schema) のための実装を feature `ts` /
//! `schema` の裏に置く。ランタイムはこれらを有効にしない (`docs/editor.md#型の生成`)。

#![warn(missing_docs)]

#[cfg(any(feature = "ts", feature = "schema"))]
pub mod codegen;
/// 色 ([`Rgb`])。
pub mod color;
pub mod id;
pub mod lighting;

pub use color::Rgb;
pub use glam::{Affine2, Mat2, Vec2};
pub use id::Id;

/// 線形補間。
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        assert_eq!(lerp(1.0, 3.0, 0.0), 1.0);
        assert_eq!(lerp(1.0, 3.0, 1.0), 3.0);
        assert_eq!(lerp(1.0, 3.0, 0.5), 2.0);
    }
}
