//! キーフォーム: パラメータ空間の格子点ごとに形状 (等) を持ち、多重線形補間で合成する。
//! 詳細な数式は `docs/spec/evaluation.md` を参照。
//! セルの由来と推定の設定 (オーサリング用) は `docs/spec/keyform-inference.md`。

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;

use crate::ParamId;

/// 1 本のパラメータ軸と、その軸上のキー位置 (昇順・重複なし)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KeyformBinding {
    /// 軸にするパラメータ。
    pub parameter: ParamId,
    /// 軸上のキー位置 (パラメータの値)。
    pub keys: Vec<f32>,
}

/// N 本のパラメータ軸の直積格子上に並んだキーフォーム。
///
/// `keyforms.len() == bindings.iter().map(|b| b.keys.len()).product()` (軸 0 本なら 1)。
/// 並びは **軸 0 が最も速く変化する** 順 (列優先、column-major)。
/// 格子座標 `(i0, i1, ..)` の線形インデックスは `i0 + n0 * (i1 + n1 * (i2 + ..))`。
///
/// `origins`・`inference`・`mirror_source` はオーサリング用のデータで、ランタイムは読み飛ばす
/// (`docs/spec/keyform-inference.md`)。
///
/// `blend_shapes` はブレンドシェイプ格子の列で、この格子の補間結果に差分を加算する
/// (`docs/spec/evaluation.md#6-ブレンドシェイプ`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
// `blend_shapes` の `default` が既定で求める境界 (`T: Default`) を外す。
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct KeyformGrid<T> {
    /// パラメータ軸。空ならキーフォームは 1 つで、パラメータに依存しない。
    #[serde(default)]
    pub bindings: Vec<KeyformBinding>,
    /// 格子点ごとのキーフォーム。並びは上記の列優先の順。
    pub keyforms: Vec<T>,
    /// キーの間の補間の種類 (`docs/spec/evaluation.md#曲線補間`)。既定は線形。
    #[serde(default, skip_serializing_if = "KeyformInterpolation::is_linear")]
    pub interpolation: KeyformInterpolation,
    /// セルごとの由来。並びは `keyforms` と同じ。
    /// 空ならすべて `authored`。空でなければ長さは `keyforms` と同じ。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<KeyformOrigin>,
    /// 方法を記録していない `derived` のセルを作り直すときの推定の設定。`None` なら加算合成 (`additive`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<KeyformInference>,
    /// `mirror` のセルのミラー元と、ミラーの設定。ミラーで作ったセルがあれば記録する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_source: Option<MirrorSource>,
    /// ブレンドシェイプ格子 (`docs/spec/model.md#ブレンドシェイプ`)。軸はすべて `kind = blend_shape` の
    /// パラメータで、補間した値と既定値での値の差を、宣言順にこの格子の補間結果へ加える。
    /// ブレンドシェイプ格子自身は `blend_shapes` を持たない (入れ子にしない)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blend_shapes: Vec<KeyformGrid<T>>,
}

impl<T> KeyformGrid<T> {
    /// 軸とキーフォームから格子を作る。すべてのセルは `authored`、推定の設定はなし。
    pub fn new(bindings: Vec<KeyformBinding>, keyforms: Vec<T>) -> Self {
        Self {
            bindings,
            keyforms,
            interpolation: KeyformInterpolation::Linear,
            origins: Vec::new(),
            inference: None,
            mirror_source: None,
            blend_shapes: Vec::new(),
        }
    }

    /// パラメータに依存しない、単一キーフォームの格子。
    pub fn constant(value: T) -> Self {
        Self::new(Vec::new(), vec![value])
    }

    /// 格子として期待されるキーフォーム数。
    ///
    /// 積が `usize` からあふれるとき (壊れたファイルなど) は `usize::MAX` を返す。どの `keyforms` の長さとも一致しない。
    pub fn expected_len(&self) -> usize {
        self.bindings
            .iter()
            .try_fold(1usize, |len, b| len.checked_mul(b.keys.len()))
            .unwrap_or(usize::MAX)
    }

    /// この格子とブレンドシェイプ格子のすべてのキーフォーム (この格子のものが先、次にブレンドシェイプ格子の宣言順)。
    /// 頂点数を変える編集など、すべてのセルに同じ操作をするときに使う。
    pub fn all_keyforms(&self) -> impl Iterator<Item = &T> {
        self.keyforms
            .iter()
            .chain(self.blend_shapes.iter().flat_map(|g| g.keyforms.iter()))
    }

    /// [`KeyformGrid::all_keyforms`] の可変版。
    pub fn all_keyforms_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.keyforms.iter_mut().chain(
            self.blend_shapes
                .iter_mut()
                .flat_map(|g| g.keyforms.iter_mut()),
        )
    }

    /// パラメータ `parameter` を軸に持つブレンドシェイプ格子の添字。
    pub fn blend_shape_index(&self, parameter: &ParamId) -> Option<usize> {
        self.blend_shapes
            .iter()
            .position(|g| g.bindings.iter().any(|b| &b.parameter == parameter))
    }

    /// 線形インデックス `index` のセルの由来。`origins` が空なら `authored`。
    /// 未知の由来 (`KeyformOrigin::Unknown`) も `authored` として返す。
    pub fn origin(&self, index: usize) -> KeyformOrigin {
        match self.origins.get(index).copied().unwrap_or_default() {
            KeyformOrigin::Unknown => KeyformOrigin::Authored,
            origin => origin,
        }
    }

    /// 線形インデックス `index` のセルを作り直す推定の方法
    /// (`docs/spec/keyform-inference.md#セルの由来`)。
    ///
    /// 方法を記録したセル (`additive` / `depth_rotation` / `mirror`) はその由来を返す。
    /// 方法を記録していない `derived` のセルは格子の `inference` の方法 (`None` なら `additive`) を返し、
    /// `inference` が未知なら `None` (作り直さない)。`authored` と未知の由来も `None`。
    pub fn derivation(&self, index: usize) -> Option<KeyformOrigin> {
        match self.origin(index) {
            KeyformOrigin::Derived => match &self.inference {
                None | Some(KeyformInference::Additive) => Some(KeyformOrigin::Additive),
                Some(KeyformInference::DepthRotation(_)) => Some(KeyformOrigin::DepthRotation),
                Some(KeyformInference::Unknown) => None,
            },
            KeyformOrigin::Authored | KeyformOrigin::Unknown => None,
            origin => Some(origin),
        }
    }
}

/// キーフォーム格子のキーの間の補間の種類 (`docs/spec/evaluation.md#曲線補間`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum KeyformInterpolation {
    /// 各軸で隣り合う 2 つのキーの間を直線で補間する (多重線形)。
    #[default]
    Linear,
    /// キーが 3 つ以上の軸で、キーを通る滑らかな曲線 (3 次エルミート) で補間する。
    /// 形 (頂点・制御点・回転の原点・角度・拡大率) だけに効き、不透明度・描画順・色などは線形のまま。
    Smooth,
    /// 未知の値 (新しい版のファイル)。線形として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl KeyformInterpolation {
    /// フォールバックを適用した値 (`Unknown` は `Linear`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Linear,
            interpolation => interpolation,
        }
    }

    /// 既定値 (`Linear`) か。書き出しの省略に使う (`Unknown` は省略しない)。
    pub fn is_linear(&self) -> bool {
        *self == Self::Linear
    }
}

/// キーフォームのセルの由来 (`docs/spec/keyform-inference.md#セルの由来`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum KeyformOrigin {
    /// 人が作った (または手で修正した) セル。推定はこのセルを上書きしない。
    #[default]
    Authored,
    /// 推定で作ったセル (方法の記録なし)。格子の `inference` の方法で作ったものとして扱う。
    /// 方法を記録しなかった版 (T0614 より前) のファイルと、`MarkKeyformDerived` がこの由来を書く。
    Derived,
    /// 加算合成で作ったセル (`docs/spec/keyform-inference.md#加算合成`)。
    Additive,
    /// 深度からの回転で作ったセル (`docs/spec/keyform-inference.md#深度からの回転`)。
    DepthRotation,
    /// ミラーで作ったセル (`docs/spec/keyform-inference.md#ミラー`)。
    /// ミラー元は格子の `mirror_source` に記録する。
    Mirror,
    /// 未知の値 (新しい版のファイル)。`authored` として扱う (推定で上書きしない)。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl KeyformOrigin {
    /// 推定で作ったセル (`authored` と未知の由来以外) か。
    pub fn is_derived(self) -> bool {
        !matches!(self, KeyformOrigin::Authored | KeyformOrigin::Unknown)
    }
}

/// ミラーで作ったセル (`KeyformOrigin::Mirror`) のミラー元と、ミラーの設定
/// (`docs/spec/keyform-inference.md#ミラー`)。エディタがミラーをやり直すときに使う。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MirrorSource {
    /// ミラー元の要素の ID。格子を持つ要素と同じ種類 (メッシュならメッシュ、デフォーマならデフォーマ)。
    /// 自分自身の ID なら、左右対称な 1 つの要素の中で写し合ったセル。
    ///
    /// 格子を持つ要素はメッシュとデフォーマの 2 種類で、どちらを指すかは格子の持ち主で決まるため、
    /// `Id<T>` ではなく `String` にしている (`docs/spec/model.md#ID`)。
    pub id: String,
    /// 対称軸 `x = a` の `a` を明示した場合の値。`None` なら元の要素の `mirror_axis` (または既定値)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<f32>,
    /// 頂点の対応付けの許容距離を明示した場合の値。`None` ならキャンバスの 1px。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f32>,
}

/// 格子の `derived` のセルを作る推定の方法と、その設定。
/// JSON では `{"method": "additive"}` や `{"method": "depth_rotation", "pivot": .., ..}` と書く。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "snake_case")]
#[non_exhaustive]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(origins: Vec<KeyformOrigin>, inference: Option<KeyformInference>) -> KeyformGrid<u8> {
        let mut grid = KeyformGrid::new(Vec::new(), vec![0; origins.len()]);
        grid.origins = origins;
        grid.inference = inference;
        grid
    }

    #[test]
    fn derivation_of_each_origin() {
        use KeyformOrigin::*;
        let origins = vec![Authored, Derived, Additive, DepthRotation, Mirror, Unknown];
        let g = grid(origins.clone(), None);
        let expected = [
            None,
            Some(Additive),
            Some(Additive),
            Some(DepthRotation),
            Some(Mirror),
        ];
        for (i, e) in expected.into_iter().chain([None]).enumerate() {
            assert_eq!(g.derivation(i), e, "cell {i}");
        }
        // 方法を記録していない `derived` のセルは、格子の `inference` の方法で作り直す。
        let depth = KeyformInference::DepthRotation(super::DepthRotation {
            angle_scale: 1.0,
            pivot: Vec2::new(0.0, 0.0),
            pivot_depth: 0.0,
            yaw: None,
            pitch: None,
        });
        assert_eq!(
            grid(origins.clone(), Some(depth)).derivation(1),
            Some(DepthRotation)
        );
        let unknown = grid(origins, Some(KeyformInference::Unknown));
        assert_eq!(unknown.derivation(1), None);
        assert_eq!(unknown.derivation(2), Some(Additive));
        assert!(Mirror.is_derived() && Derived.is_derived());
        assert!(!Authored.is_derived() && !Unknown.is_derived());
    }

    #[test]
    fn origins_and_mirror_source_json() {
        // 方法を記録しない古いファイル (`derived` だけ) もそのまま読める。
        let json = r#"{"keyforms":[0,0,0,0,0],
            "origins":["authored","derived","additive","depth_rotation","mirror"],
            "mirror_source":{"id":"EyeL","axis":0.5}}"#;
        let g: KeyformGrid<u8> = serde_json::from_str(json).unwrap();
        assert_eq!(
            g.origins,
            [
                KeyformOrigin::Authored,
                KeyformOrigin::Derived,
                KeyformOrigin::Additive,
                KeyformOrigin::DepthRotation,
                KeyformOrigin::Mirror,
            ]
        );
        let source = g.mirror_source.clone().unwrap();
        assert_eq!(
            (source.id.as_str(), source.axis, source.tolerance),
            ("EyeL", Some(0.5), None)
        );
        let back: KeyformGrid<u8> =
            serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        let plain = serde_json::to_value(KeyformGrid::constant(0u8)).unwrap();
        assert!(plain.get("mirror_source").is_none());
    }

    #[test]
    fn interpolation_json() {
        // 省略 (古いファイル) は線形で、線形は書き出さない。
        let g: KeyformGrid<u8> = serde_json::from_str(r#"{"keyforms":[0]}"#).unwrap();
        assert_eq!(g.interpolation, KeyformInterpolation::Linear);
        assert!(
            serde_json::to_value(&g)
                .unwrap()
                .get("interpolation")
                .is_none()
        );
        let json = r#"{"keyforms":[0],"interpolation":"smooth"}"#;
        let mut g: KeyformGrid<u8> = serde_json::from_str(json).unwrap();
        assert_eq!(g.interpolation, KeyformInterpolation::Smooth);
        assert_eq!(serde_json::to_value(&g).unwrap()["interpolation"], "smooth");
        // 未知の値は線形として扱い、書き出せない。
        let json = r#"{"keyforms":[0],"interpolation":"bezier"}"#;
        g = serde_json::from_str(json).unwrap();
        assert_eq!(g.interpolation, KeyformInterpolation::Unknown);
        assert_eq!(g.interpolation.effective(), KeyformInterpolation::Linear);
        assert!(serde_json::to_value(&g).is_err());
    }
}
