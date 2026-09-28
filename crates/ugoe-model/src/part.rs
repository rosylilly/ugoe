use serde::{Deserialize, Serialize};
use ugoe_core::Id;

use crate::KeyformGrid;

pub type PartId = Id<Part>;

/// パーツ: メッシュ・デフォーマをまとめる論理グループ。不透明度は親子で乗算される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: PartId,
    pub name: String,
    pub parent: Option<PartId>,
    pub keyforms: KeyformGrid<PartKeyform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PartKeyform {
    pub opacity: f32,
}
