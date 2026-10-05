//! キーフォームの推定 (`docs/spec/keyform-inference.md`、ADR 0010)。
//!
//! 推定はツール層で行い、結果はふつうのキーフォームとして格子に焼き込む。
//! ランタイムの評価は推定を知らない。書き込むのは `derived` のセルだけで、`authored` のセルは変えない
//! (ミラーの `MirrorOptions::overwrite_authored` を除く)。書いたセルの由来には推定の方法
//! (`additive` / `depth_rotation` / `mirror`) を記録する。`InferScope::Own` なら別の方法で作ったセルは
//! 書き換えない。
//!
//! 法線マップの生成 (アルファの形からふくらみを作る決定的な方法) も持つ ([`bulge_normal_map`]、部位のまとまりを
//! 1 つの形にする [`group_bulge_normal_maps`]、形に加えず向きだけを受けるテクスチャを含める
//! [`group_bulge_normal_maps_oriented`])。
//!
//! 格子のセルと線形インデックスの変換とキーの比べ方 ([`grid_index`]) は、`ugoe-editor-core`・`ugoe-ai` と共有する。

mod additive;
mod depth;
mod error;
mod grid;
pub mod grid_index;
mod mirror;
mod normal;
mod space;
mod target;
mod values;
mod world;

pub use additive::{InferReport, InferScope, infer_additive};
pub use depth::infer_depth_rotation;
pub use error::InferError;
pub use mirror::{MirrorOptions, check_mirror, mirror_keyforms};
pub use normal::{
    BulgeOptions, EDGE_PASSES, GroupNormalMaps, HeightField, MAX_GROUP_CELLS, NormalMap,
    NormalMapError, PlacedTexture, bulge_height, bulge_normal_map, decode_normal, encode_normal,
    fill_missing, group_bulge_normal_maps, group_bulge_normal_maps_oriented,
};
pub use target::{KeyformTarget, infer_additive_target};
pub use values::KeyformValues;
pub use world::{ParentWorld, WorldError};
