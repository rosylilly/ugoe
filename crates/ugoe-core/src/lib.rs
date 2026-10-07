//! ugoe 全クレートが共有する最下層の基盤型。
//!
//! このクレートは **依存ゼロに近い状態を保つ** こと。GPU・I/O・シリアライズ形式に
//! 依存するものは置かない (serde の derive のみ許可)。
//!
//! 例外として、コード生成 (TypeScript の型と JSON Schema) のための実装を feature `ts` /
//! `schema` の裏に置く。ランタイムはこれらを有効にしない (`docs/editor.md#型の生成`)。
//!
//! `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない
//! (`docs/portability.md#ランタイム層が守る制約-移植性のため`)。
//! ただしコード生成の feature (`ts`・`schema`) を有効にしたときだけは std の prelude を使う
//! (ts-rs と schemars の derive が生成するコードが `String`・`format!` などを prelude から使うため。ツール層だけが有効にする)。

#![cfg_attr(not(any(feature = "ts", feature = "schema")), no_std)]
#![warn(missing_docs)]

extern crate alloc;

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
