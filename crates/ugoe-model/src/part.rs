use alloc::string::String;

use serde::{Deserialize, Serialize};
use ugoe_core::Id;

use crate::KeyformGrid;

/// パーツの ID。モデルの中で一意。
pub type PartId = Id<Part>;

/// パーツ: メッシュ・デフォーマをまとめる論理グループ。不透明度は親子で乗算される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Part {
    /// モデルの中で一意な ID。
    pub id: PartId,
    /// 表示名 (エディタ用)。
    pub name: String,
    /// 親のパーツ。`None` なら最上位。
    pub parent: Option<PartId>,
    /// 不透明度のキーフォーム。
    pub keyforms: KeyformGrid<PartKeyform>,
}

/// パーツの 1 つのキーフォーム。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PartKeyform {
    /// 不透明度 (0〜1)。子のパーツ・デフォーマ・メッシュに乗算される。
    pub opacity: f32,
}
