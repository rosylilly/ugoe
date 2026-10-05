//! PSD のレイヤーの分類の直し (`docs/editor.md#分類の直し`)。
//!
//! 分類 (`ugoe_ai::classify`) の結果そのものはエディタを開いている間だけ持つが、利用者が直したラベル・左右と
//! 「これでよい」と認めた答えは、レイヤーのパスごとの [`LayerCorrection`] として [`crate::Document`] が持ち
//! (Undo の対象。[`crate::Command::SetLayerCorrection`])、`project.json` の `editor.classification` に残す。
//! 分類し直しても、直しは分類の上に重ねる (重ねるのはエディタ。`apps/editor/src-tauri/src/proposals.rs`)。
//!
//! このクレートは `ugoe-ai` に依存できないので、ラベルは文字列 (`eye_white` など) で持ち、形 (小文字の英数字と `_`)
//! だけを確かめる。語彙にないラベルは重ねる側が無視する。

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::project::ProjectMeta;

/// `project.json` の `editor` の中で、分類の直しを置くキー。
const CLASSIFICATION_KEY: &str = "classification";
/// `editor.classification` の中で、直しの列を置くキー。
const CORRECTIONS_KEY: &str = "corrections";

/// 直しの数の上限 (外から来た `project.json` やコマンドで際限なく増やさない)。
pub const MAX_LAYER_CORRECTIONS: usize = 65_536;
/// レイヤーのパスの深さ (グループの入れ子 + 1) の上限。
pub const MAX_LAYER_PATH_DEPTH: usize = 64;
/// レイヤーのパスの 1 つの名前の長さ (文字数) の上限。
pub const MAX_LAYER_NAME_CHARS: usize = 1024;
/// ラベルの長さ (バイト数) の上限。
pub const MAX_LABEL_LEN: usize = 64;

/// 左右 (キャラクター自身の左右。`ugoe_ai::classify::Side` と同じ値)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LayerSide {
    Left,
    Right,
    Center,
}

impl LayerSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
        }
    }
}

/// 利用者が決めた 1 枚のレイヤーの分類 (直したもの・認めたもの)。分類の答えより優先する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct LayerCorrection {
    /// 意味ラベル (`face`・`eye_white` など。`docs/ai-integration.md#レイヤーの意味分類`)。
    pub label: String,
    pub side: LayerSide,
}

/// レイヤーのパス (外側のグループから順に、最後がレイヤー自身の名前) → 直し。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerCorrections {
    map: BTreeMap<Vec<String>, LayerCorrection>,
}

/// `project.json` の `editor.classification.corrections` の 1 件。
#[derive(Debug, Serialize, Deserialize)]
struct StoredCorrection {
    path: Vec<String>,
    label: String,
    side: LayerSide,
}

impl LayerCorrections {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// レイヤー `path` の直し。
    pub fn get(&self, path: &[String]) -> Option<&LayerCorrection> {
        self.map.get(path)
    }

    /// パスの順に並べた直し。
    pub fn iter(&self) -> impl Iterator<Item = (&[String], &LayerCorrection)> {
        self.map.iter().map(|(path, c)| (path.as_slice(), c))
    }

    /// レイヤー `path` の直しを `correction` にする (`None` なら消す)。形が正しくなければ理由を返し、何も変えない。
    /// 変わったら `true`。
    pub fn set(
        &mut self,
        path: Vec<String>,
        correction: Option<LayerCorrection>,
    ) -> Result<bool, String> {
        check_path(&path)?;
        match correction {
            Some(correction) => {
                check_label(&correction.label)?;
                if self.map.get(&path) == Some(&correction) {
                    return Ok(false);
                }
                if !self.map.contains_key(&path) && self.map.len() >= MAX_LAYER_CORRECTIONS {
                    return Err(format!("at most {MAX_LAYER_CORRECTIONS} corrections"));
                }
                self.map.insert(path, correction);
                Ok(true)
            }
            None => Ok(self.map.remove(&path).is_some()),
        }
    }

    /// `editor.classification` の値から読む。形の違う値・正しくない件は捨て (同じパスは先の件)、上限を超えた分も捨てる。
    fn from_value(value: Option<&serde_json::Value>) -> Self {
        let mut corrections = Self::default();
        let Some(items) = value
            .and_then(|v| v.get(CORRECTIONS_KEY))
            .and_then(serde_json::Value::as_array)
        else {
            return corrections;
        };
        for item in items {
            if corrections.len() >= MAX_LAYER_CORRECTIONS {
                break;
            }
            let Ok(stored) = serde_json::from_value::<StoredCorrection>(item.clone()) else {
                continue;
            };
            if corrections.map.contains_key(&stored.path) {
                continue;
            }
            let correction = LayerCorrection {
                label: stored.label,
                side: stored.side,
            };
            // 正しくない件は捨てる (読み込みは失敗させない)。
            let _ = corrections.set(stored.path, Some(correction));
        }
        corrections
    }

    /// `editor.classification` に書く値 (直しがなければ `None` で、項目を書かない)。
    fn to_value(&self) -> Option<serde_json::Value> {
        if self.is_empty() {
            return None;
        }
        let items: Vec<serde_json::Value> = self
            .map
            .iter()
            .map(|(path, c)| {
                serde_json::json!({ "path": path, "label": c.label, "side": c.side.as_str() })
            })
            .collect();
        Some(serde_json::json!({ CORRECTIONS_KEY: items }))
    }
}

fn check_path(path: &[String]) -> Result<(), String> {
    if path.is_empty() {
        return Err("the layer path is empty".into());
    }
    if path.len() > MAX_LAYER_PATH_DEPTH {
        return Err(format!(
            "the layer path is deeper than {MAX_LAYER_PATH_DEPTH}"
        ));
    }
    if path
        .iter()
        .any(|name| name.chars().count() > MAX_LAYER_NAME_CHARS)
    {
        return Err(format!(
            "a layer name is longer than {MAX_LAYER_NAME_CHARS} characters"
        ));
    }
    Ok(())
}

fn check_label(label: &str) -> Result<(), String> {
    let ok = !label.is_empty()
        && label.len() <= MAX_LABEL_LEN
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "label `{label}` must be 1-{MAX_LABEL_LEN} lowercase letters, digits or `_`"
        ))
    }
}

impl ProjectMeta {
    /// プロジェクトに残した分類の直し (`editor.classification`)。ない・読めなければ空。
    pub fn layer_corrections(&self) -> LayerCorrections {
        LayerCorrections::from_value(self.editor.get(CLASSIFICATION_KEY))
    }

    /// 分類の直しを残す。空なら `editor.classification` を消す (既存のプロジェクトの `project.json` を変えないため)。
    pub fn set_layer_corrections(&mut self, corrections: &LayerCorrections) {
        match corrections.to_value() {
            Some(value) => self.editor.insert(CLASSIFICATION_KEY.to_owned(), value),
            None => self.editor.remove(CLASSIFICATION_KEY),
        };
    }
}
