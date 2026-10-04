//! 文字列ベースの安定 ID。
//!
//! オーサリング・ファイル・AI 連携では人間 (とエージェント) が読める文字列 ID を使い、
//! ランタイムはロード時に ID を連番インデックスへ解決する。
//! 型パラメータ `T` は ID の種類 (パラメータ / パーツ / メッシュ …) を区別するための幽霊型。

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 種類 `T` の文字列 ID。比較・ハッシュは文字列だけで行い、シリアライズでは文字列そのものになる。
pub struct Id<T> {
    value: String,
    _kind: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// 文字列から作る。中身の検査はしない (空文字列も通る)。
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            _kind: PhantomData,
        }
    }

    /// ID の文字列。
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl<T> From<&str> for Id<T> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.value)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// TypeScript では単なる `string` (種類の区別は Rust 側だけ)。
#[cfg(feature = "ts")]
impl<T: 'static> ts_rs::TS for Id<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "string".to_owned()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}

/// JSON Schema では文字列 (`docs/spec/model.md#ID`)。
#[cfg(feature = "schema")]
impl<T> schemars::JsonSchema for Id<T> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Id".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "要素の ID。新しい要素には `[A-Za-z0-9_]+` の形式を使う。",
        })
    }
}
