//! モーション (時間軸アニメーション) データ。再生は `ugoe-motion`、仕様は `docs/spec/motion.md`。

use serde::{Deserialize, Serialize};

use crate::{ParamId, PartId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub struct Curve {
    pub target: CurveTarget,
    pub segments: Vec<Segment>,
    /// カーブ個別のフェード (未指定ならモーション全体の値)。
    #[serde(default)]
    pub fade_in: Option<f32>,
    #[serde(default)]
    pub fade_out: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum CurveTarget {
    Parameter(ParamId),
    PartOpacity(PartId),
    /// 目パチ・リップシンク等の「エフェクトグループ」の重み。
    EffectGroup(String),
}

/// 点 (time, value)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub t: f32,
    pub v: f32,
}

/// 連続するセグメント列。各セグメントの始点は直前セグメントの終点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionEvent {
    pub t: f32,
    pub value: String,
}
