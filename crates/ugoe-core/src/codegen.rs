//! コード生成 (TypeScript の型と JSON Schema) の補助。feature `ts` / `schema` でだけ有効。
//! 生成の手順は `docs/editor.md#型の生成`。

/// [`crate::Vec2`] の JSON 表現 `[x, y]` (glam の serde 形式)。
///
/// glam の型には `TS` / `JsonSchema` を実装できないので、`Vec2` のフィールドに
/// `#[ts(as = "ugoe_core::codegen::Vec2Repr")]` や `#[schemars(with = "..")]` を付けて使う。
/// 生成される型の名前は `Vec2`。
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename = "Vec2"))]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "Vec2")
)]
pub struct Vec2Repr(pub f32, pub f32);
