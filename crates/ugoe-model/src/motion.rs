//! モーション (時間軸アニメーション) データ。再生は `ugoe-motion`、仕様は `docs/spec/motion.md`。

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::{ParamId, PartId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Motion {
    pub name: String,
    /// 秒。
    pub duration: f32,
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub fade_in: f32,
    #[serde(default)]
    pub fade_out: f32,
    pub curves: Vec<Curve>,
    #[serde(default)]
    pub events: Vec<MotionEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Curve {
    pub target: CurveTarget,
    pub segments: Vec<Segment>,
    /// カーブ個別のフェード (未指定ならモーション全体の値)。
    #[serde(default)]
    pub fade_in: Option<f32>,
    #[serde(default)]
    pub fade_out: Option<f32>,
}

/// カーブの対象。JSON では `{"kind": "parameter", "id": "ParamAngleX"}` と書く。
///
/// 隣接タグ付きの表現では `#[serde(other)]` が中身 (`id`) を読めないので、
/// 読み込みは手で書く ([`CurveTarget::deserialize`])。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum CurveTarget {
    Parameter(ParamId),
    PartOpacity(PartId),
    /// 目パチ・リップシンク等の「エフェクトグループ」の重み。
    EffectGroup(String),
    /// 未知の対象 (新しい版のファイル)。そのカーブを無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl<'de> Deserialize<'de> for CurveTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// `{kind, id}` をそのまま読む。`id` は知っている種類ではすべて文字列なので、
        /// 文字列以外 (未知の種類の中身) は読み捨てる。
        #[derive(Deserialize)]
        struct Raw {
            kind: String,
            #[serde(default)]
            id: Option<StringOrIgnored>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let id = || match raw.id.clone() {
            Some(StringOrIgnored(Some(id))) => Ok(id),
            _ => Err(de::Error::missing_field("id")),
        };
        Ok(match raw.kind.as_str() {
            "parameter" => Self::Parameter(ParamId::new(id()?)),
            "part_opacity" => Self::PartOpacity(PartId::new(id()?)),
            "effect_group" => Self::EffectGroup(id()?),
            _ => Self::Unknown,
        })
    }
}

/// 文字列ならその値、それ以外の値なら `None` として読み捨てる。
#[derive(Clone)]
struct StringOrIgnored(Option<String>);

impl<'de> Deserialize<'de> for StringOrIgnored {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StringOrIgnored;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any value")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(Some(v.to_owned())))
            }

            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_bytes<E: de::Error>(self, _: &[u8]) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StringOrIgnored(None))
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                StringOrIgnored::deserialize(d)
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}
                Ok(StringOrIgnored(None))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map
                    .next_entry::<de::IgnoredAny, de::IgnoredAny>()?
                    .is_some()
                {}
                Ok(StringOrIgnored(None))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

/// 点 (time, value)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Point {
    pub t: f32,
    pub v: f32,
}

/// 連続するセグメント列。各セグメントの始点は直前セグメントの終点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Segment {
    /// カーブの最初の点。先頭に 1 つだけ置く。
    Start {
        p: Point,
    },
    Linear {
        to: Point,
    },
    Bezier {
        c1: Point,
        c2: Point,
        to: Point,
    },
    /// 終点時刻まで始点の値を保つ。
    Stepped {
        to: Point,
    },
    /// 始点直後から終点の値になる。
    InverseStepped {
        to: Point,
    },
    /// 未知の値 (新しい版のファイル)。そのカーブ全体を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MotionEvent {
    pub t: f32,
    pub value: String,
}
