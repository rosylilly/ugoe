//! ポーズ (排他的なパーツ切り替え) の設定データ。再生は `ugoe-motion`、仕様は `docs/spec/model.md#ポーズ` と `docs/spec/motion.md#エフェクト`。

use serde::{Deserialize, Serialize};

use crate::PartId;

/// ポーズの既定のクロスフェード時間 (秒)。
pub const DEFAULT_FADE_DURATION: f32 = 0.5;

/// ポーズ設定。グループごとに、表示中のパーツを 1 つに保つ。
///
/// `groups` が空ならポーズは何もしない (`Default` はこの状態)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Pose {
    /// 表示パーツを切り替えるときのクロスフェード時間 (秒)。0 以上。
    pub fade_duration: f32,
    pub groups: Vec<PoseGroup>,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            fade_duration: DEFAULT_FADE_DURATION,
            groups: Vec::new(),
        }
    }
}

/// 互いに排他的なパーツの組 (例: 腕の差分 A / B)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PoseGroup {
    pub entries: Vec<PoseEntry>,
}

/// ポーズグループの 1 要素。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PoseEntry {
    pub part: PartId,
    /// `part` と同じ不透明度に追従するパーツ (例: 腕に付属する袖)。
    #[serde(default)]
    pub links: Vec<PartId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let pose: Pose =
            serde_json::from_str(r#"{"groups":[{"entries":[{"part":"A"}]}]}"#).unwrap();
        assert_eq!(
            pose,
            Pose {
                fade_duration: DEFAULT_FADE_DURATION,
                groups: vec![PoseGroup {
                    entries: vec![PoseEntry {
                        part: "A".into(),
                        links: vec![],
                    }],
                }],
            }
        );
        let json = serde_json::to_string(&pose).unwrap();
        assert_eq!(serde_json::from_str::<Pose>(&json).unwrap(), pose);
    }
}
