//! キーフォーム: パラメータ空間の格子点ごとに形状 (等) を持ち、多重線形補間で合成する。
//! 詳細な数式は `docs/spec/evaluation.md` を参照。
//! セルの由来と推定の設定 (オーサリング用) は `docs/spec/keyform-inference.md`。

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;

use crate::ParamId;

/// 1 本のパラメータ軸と、その軸上のキー位置 (昇順・重複なし)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KeyformBinding {
    pub parameter: ParamId,
    pub keys: Vec<f32>,
}

/// N 本のパラメータ軸の直積格子上に並んだキーフォーム。
///
/// `keyforms.len() == bindings.iter().map(|b| b.keys.len()).product()` (軸 0 本なら 1)。
/// 並びは **軸 0 が最も速く変化する** 行優先 (column-major 的) 順。
/// 格子座標 `(i0, i1, ..)` の線形インデックスは `i0 + n0 * (i1 + n1 * (i2 + ..))`。
///
/// `origins` と `inference` はオーサリング用のデータで、ランタイムは読み飛ばす
/// (`docs/spec/keyform-inference.md`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KeyformGrid<T> {
    #[serde(default)]
    pub bindings: Vec<KeyformBinding>,
    pub keyforms: Vec<T>,
    /// セルごとの由来。並びは `keyforms` と同じ。
    /// 空ならすべて `authored`。空でなければ長さは `keyforms` と同じ。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<KeyformOrigin>,
    /// `derived` のセルを作り直すときの推定の設定。`None` なら加算合成 (`additive`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<KeyformInference>,
}

impl<T> KeyformGrid<T> {
    /// 軸とキーフォームから格子を作る。すべてのセルは `authored`、推定の設定はなし。
    pub fn new(bindings: Vec<KeyformBinding>, keyforms: Vec<T>) -> Self {
        Self {
            bindings,
            keyforms,
            origins: Vec::new(),
            inference: None,
        }
    }

    /// パラメータに依存しない、単一キーフォームの格子。
    pub fn constant(value: T) -> Self {
        Self::new(Vec::new(), vec![value])
    }

    /// 格子として期待されるキーフォーム数。
    pub fn expected_len(&self) -> usize {
        self.bindings.iter().map(|b| b.keys.len()).product()
    }

    /// 線形インデックス `index` のセルの由来。`origins` が空なら `authored`。
    /// 未知の由来 (`KeyformOrigin::Unknown`) も `authored` として返す。
    pub fn origin(&self, index: usize) -> KeyformOrigin {
        match self.origins.get(index).copied().unwrap_or_default() {
            KeyformOrigin::Unknown => KeyformOrigin::Authored,
            origin => origin,
        }
    }
}

/// キーフォームのセルの由来 (`docs/spec/keyform-inference.md#セルの由来`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum KeyformOrigin {
    /// 人が作った (または手で修正した) セル。推定はこのセルを上書きしない。
    #[default]
    Authored,
    /// 推定で作ったセル。推定の元になったセルが変わると、エディタが作り直す。
    Derived,
    /// 未知の値 (新しい版のファイル)。`authored` として扱う (推定で上書きしない)。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 格子の `derived` のセルを作る推定の方法と、その設定。
/// JSON では `{"method": "additive"}` や `{"method": "depth_rotation", "pivot": .., ..}` と書く。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum KeyformInference {
    /// 加算合成 (`docs/spec/keyform-inference.md#加算合成`)。
    Additive,
    /// 深度からの回転 (`docs/spec/keyform-inference.md#深度からの回転`)。
    DepthRotation(DepthRotation),
    /// 未知の値 (新しい版のファイル)。推定で作り直さない (`derived` のセルをそのまま使う)。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 深度からの回転の設定。対象の深度はメッシュ・ワープデフォーマの `depth` に持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DepthRotation {
    /// パラメータの値に掛けて角度 (度) にする係数。既定 1。
    #[serde(default = "one")]
    pub angle_scale: f32,
    /// 回転の中心の位置 (対象の頂点・制御点と同じ親座標系)。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub pivot: Vec2,
    /// 回転の中心の深度 (キャンバスのピクセル単位、手前が正)。既定 0。
    #[serde(default)]
    pub pivot_depth: f32,
    /// 縦軸まわりの回転 (yaw) に使う軸のパラメータ (通常 `AngleX`)。`None` なら回さない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw: Option<ParamId>,
    /// 横軸まわりの回転 (pitch) に使う軸のパラメータ (通常 `AngleY`)。`None` なら回さない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch: Option<ParamId>,
}

fn one() -> f32 {
    1.0
}
