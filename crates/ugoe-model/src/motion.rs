//! モーション (時間軸アニメーション) データ。再生は `ugoe-motion`、仕様は `docs/spec/motion.md`。

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::{ParamId, PartId};

/// モーション (`*.motion.json`)。パラメータ・パーツの不透明度などを時間で動かすカーブの集まり (`docs/spec/motion.md#モーション`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Motion {
    /// モーションの名前 (表示・選択に使う)。
    pub name: String,
    /// 秒。
    pub duration: f32,
    /// `duration` で折り返して繰り返すか。ループの境目ではフェードをかけない。
    #[serde(default)]
    pub looping: bool,
    /// フェードインの秒数。0 以下ならフェードインしない。
    #[serde(default)]
    pub fade_in: f32,
    /// フェードアウトの秒数。ループしないモーションは終わりのこの秒数前から、ループするものは止めてから
    /// この秒数でフェードアウトする。0 以下ならフェードしない。
    #[serde(default)]
    pub fade_out: f32,
    /// 時間で値を動かすカーブ。
    pub curves: Vec<Curve>,
    /// 再生時刻が通過したフレームで発火するイベント。
    #[serde(default)]
    pub events: Vec<MotionEvent>,
}

/// 1 つの対象の値を時間で動かすカーブ (`docs/spec/motion.md#カーブ`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Curve {
    /// カーブが動かす対象。
    pub target: CurveTarget,
    /// カーブの形。最初は必ず [`Segment::Start`] で、以降の各セグメントの始点は直前のセグメントの終点。
    pub segments: Vec<Segment>,
    /// カーブ個別のフェード (未指定ならモーション全体の値)。
    #[serde(default)]
    pub fade_in: Option<f32>,
    /// カーブ個別のフェードアウトの秒数 (未指定ならモーション全体の値)。終了時刻はモーションのものを使う。
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
    /// パラメータの値。
    Parameter(ParamId),
    /// パーツの不透明度を上書きする。
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
    /// 時刻 (秒)。
    pub t: f32,
    /// その時刻での対象の値 (パラメータの値・不透明度など)。
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
        /// 最初の点。
        p: Point,
    },
    /// 始点から `to` まで線形補間する。
    Linear {
        /// 区間の終点。
        to: Point,
    },
    /// 始点・`c1`・`c2`・`to` の 3 次ベジェ。制御点の時刻は区間内にあるものとする
    /// (範囲外は評価時にクランプする)。
    Bezier {
        /// 始点側の制御点。
        c1: Point,
        /// 終点側の制御点。
        c2: Point,
        /// 区間の終点。
        to: Point,
    },
    /// 終点時刻まで始点の値を保つ。
    Stepped {
        /// 区間の終点。
        to: Point,
    },
    /// 始点直後から終点の値になる。
    InverseStepped {
        /// 区間の終点。
        to: Point,
    },
    /// 未知の値 (新しい版のファイル)。そのカーブ全体を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// モーションの中の時刻に置くイベント。再生時刻がこの時刻を通過したフレームで発火する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MotionEvent {
    /// 発火する時刻 (秒)。
    pub t: f32,
    /// イベントの値 (アプリが解釈する任意の文字列)。
    pub value: String,
}
