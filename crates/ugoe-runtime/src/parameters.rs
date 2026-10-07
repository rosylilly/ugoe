use alloc::sync::Arc;
use alloc::vec::Vec;

use ugoe_model::{Model, ParamId};

use crate::id_index::IdIndex;

/// コンパイル済みモデル内でのパラメータ連番 (`Model::parameters` のインデックス)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamIndex(pub u32);

/// パラメータの現在値。モーション・物理・アプリはここを書き換える
/// (`spec/evaluation.md#parameter-values`)。
///
/// - 初期値はパラメータの `default`。
/// - 書き込む値はすべて値域 `[min, max]` にクランプする。したがって評価はいつも値域の中の値を見る。
/// - NaN の書き込みは無視する (値は変わらない)。無限大は値域の端にクランプする。
///
/// ID の並びと ID → 連番の表は複製のあいだで共有する (`Arc`)。`clone` と `clone_from` は
/// ID を複製せず、`clone_from` は確保済みの値のバッファを使い回す。
#[derive(Debug)]
pub struct ParameterValues {
    layout: Arc<Layout>,
    min: Vec<f32>,
    max: Vec<f32>,
    default: Vec<f32>,
    values: Vec<f32>,
}

/// パラメータの ID の並びと、ID → 連番の表 (`index_of` を線形探索にしない。T0146)。
#[derive(Debug)]
struct Layout {
    ids: Vec<ParamId>,
    index: IdIndex,
}

/// [`ParameterValues`] の並び (ID の列) の手札。[`ParameterValues::layout`] で得る。
///
/// 連番 ([`ParamIndex`]) を解決して持つ側 (物理・モーションなど) が、解決したときの手札を控えておき、
/// 次のフレームで [`ParameterValues::has_layout`] により並びが同じかを定数時間で確かめる (T0160)。
/// 手札は並びを共有して持ち続けるので、複製は ID を複製しない。並びが解放されて別の並びが同じ
/// アドレスに作られる取り違えも起きない。
#[derive(Debug, Clone)]
pub struct ParameterLayout(Arc<Layout>);

impl ParameterLayout {
    /// パラメータの ID の列 (`ParamIndex` の順)。
    pub fn ids(&self) -> &[ParamId] {
        &self.0.ids
    }
}

impl Clone for ParameterValues {
    fn clone(&self) -> Self {
        Self {
            layout: Arc::clone(&self.layout),
            min: self.min.clone(),
            max: self.max.clone(),
            default: self.default.clone(),
            values: self.values.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        if !Arc::ptr_eq(&self.layout, &source.layout) {
            self.layout = Arc::clone(&source.layout);
        }
        self.min.clone_from(&source.min);
        self.max.clone_from(&source.max);
        self.default.clone_from(&source.default);
        self.values.clone_from(&source.values);
    }
}

impl ParameterValues {
    /// `model` のパラメータの並びで作る。値はすべて既定値。
    pub fn from_model(model: &Model) -> Self {
        let p = &model.parameters;
        Self {
            layout: Arc::new(Layout {
                ids: p.iter().map(|p| p.id.clone()).collect(),
                index: IdIndex::new(p.iter().map(|p| p.id.as_str())),
            }),
            min: p.iter().map(|p| p.min).collect(),
            max: p.iter().map(|p| p.max).collect(),
            default: p.iter().map(|p| p.default).collect(),
            values: p.iter().map(|p| p.default).collect(),
        }
    }

    /// ID が `id` のパラメータの連番。なければ `None`。
    pub fn index_of(&self, id: &str) -> Option<ParamIndex> {
        self.layout.index.get(id).map(ParamIndex)
    }

    /// パラメータの ID の列 (`ParamIndex` の順)。
    ///
    /// 連番を解決して持つ側が並びの変化を確かめるには、まず [`Self::has_layout`] (定数時間) を使う。
    pub fn ids(&self) -> &[ParamId] {
        &self.layout.ids
    }

    /// 並びの手札。控えておき、後で [`Self::has_layout`] で並びが同じかを確かめる。
    /// `Arc` の複製だけで、ID は複製しない。
    pub fn layout(&self) -> ParameterLayout {
        ParameterLayout(Arc::clone(&self.layout))
    }

    /// 並びが `layout` と同じであることを定数時間で確かめる (ポインタの比較)。
    ///
    /// `true` なら ID の列は `layout.ids()` と同じで、`layout` を得たときに解決した連番はそのまま使える。
    /// 並びは複製 (`clone`・`clone_from`) のあいだで共有されるので、同じ値から複製したもの同士は `true`。
    /// 別々に作った (`from_model` を呼び直した) ものは、ID の列が同じでも `false` を返す。
    /// 呼び出し側は `false` のとき ID の列を比べるか、解決し直す。
    pub fn has_layout(&self, layout: &ParameterLayout) -> bool {
        Arc::ptr_eq(&self.layout, &layout.0)
    }

    /// パラメータの数。
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// パラメータを 1 つも持たないか。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 現在値。
    ///
    /// # Panics
    ///
    /// `index` が範囲外のとき (`min`・`max`・`default_value`・`set`・`blend` も同じ)。
    pub fn get(&self, index: ParamIndex) -> f32 {
        self.values[index.0 as usize]
    }

    /// 値域の下限。
    pub fn min(&self, index: ParamIndex) -> f32 {
        self.min[index.0 as usize]
    }

    /// 値域の上限。
    pub fn max(&self, index: ParamIndex) -> f32 {
        self.max[index.0 as usize]
    }

    /// 既定値。
    pub fn default_value(&self, index: ParamIndex) -> f32 {
        self.default[index.0 as usize]
    }

    /// 値域にクランプして設定する。NaN は書き込まずに無視する (値は変わらない。
    /// `docs/spec/evaluation.md` の評価はいつも値域の中の値を見る)。
    pub fn set(&mut self, index: ParamIndex, value: f32) {
        let i = index.0 as usize;
        if !value.is_nan() {
            self.values[i] = value.clamp(self.min[i], self.max[i]);
        }
    }

    /// 現在値に `weight` の重みで `value` をブレンドする (モーションのフェード用)。
    /// 結果が NaN なら [`ParameterValues::set`] と同じく無視する。
    pub fn blend(&mut self, index: ParamIndex, value: f32, weight: f32) {
        let current = self.get(index);
        self.set(index, current + (value - current) * weight);
    }

    /// `previous` にある同じ ID のパラメータの値を引き継ぐ (値域にクランプする)。
    /// 組み替え (`docs/spec/modules.md#合成`) で、合成し直したモデルに前の値を移すときに使う。
    pub fn carry_over(&mut self, previous: &ParameterValues) {
        for (i, id) in self.layout.ids.iter().enumerate() {
            if let Some(j) = previous.index_of(id.as_str()) {
                let value = previous.get(j);
                self.values[i] = value.clamp(self.min[i], self.max[i]);
            }
        }
    }

    /// すべての値を既定値に戻す。
    pub fn reset(&mut self) {
        self.values.copy_from_slice(&self.default);
    }

    /// 現在値の列 (`ParamIndex` の順)。
    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_accessors_return_model_values() {
        let params = ParameterValues::from_model(&ugoe_model::fixtures::physics_hair());
        let index = params.index_of(ugoe_model::standard::ANGLE_X).unwrap();
        assert_eq!(params.min(index), -30.0);
        assert_eq!(params.max(index), 30.0);
        assert_eq!(params.default_value(index), 0.0);
    }

    // spec/evaluation.md#parameter-values: 有限値でない書き込みは無視してよい。NaN を値に入れると
    // キーフォームの重みが求まらない (`keyform::locate` が範囲外を指す) ので、NaN の書き込みは無視する。
    #[test]
    fn nan_writes_are_ignored() {
        let mut params = ParameterValues::from_model(&ugoe_model::fixtures::physics_hair());
        let index = params.index_of(ugoe_model::standard::ANGLE_X).unwrap();
        params.set(index, 12.0);
        params.set(index, f32::NAN);
        assert_eq!(params.get(index), 12.0);
        params.blend(index, f32::NAN, 0.5);
        params.blend(index, 0.0, f32::NAN);
        assert_eq!(params.get(index), 12.0);
        // 無限大は値域にクランプする。
        params.set(index, f32::INFINITY);
        assert_eq!(params.get(index), 30.0);
    }

    // T0146: `index_of` は ID の並びの連番を返す。ID の表は複製のあいだで共有し、複製しない。
    #[test]
    fn index_of_matches_order_and_clones_share_ids() {
        let model = ugoe_model::fixtures::pose_and_effects();
        let params = ParameterValues::from_model(&model);
        for (i, p) in model.parameters.iter().enumerate() {
            assert_eq!(params.index_of(p.id.as_str()), Some(ParamIndex(i as u32)));
        }
        assert_eq!(params.index_of("Missing"), None);
        let copy = params.clone();
        assert!(core::ptr::eq(copy.ids(), params.ids()));
        let mut other = ParameterValues::from_model(&ugoe_model::fixtures::single_quad());
        other.clone_from(&params);
        assert!(core::ptr::eq(other.ids(), params.ids()));
        assert_eq!(
            other.index_of(model.parameters[0].id.as_str()),
            Some(ParamIndex(0))
        );
    }

    // T0160: 並びの手札は、同じ値から複製したものと同じ・並びを変えたものと違う。
    #[test]
    fn has_layout_follows_shared_layout() {
        let hair = ugoe_model::fixtures::physics_hair();
        let params = ParameterValues::from_model(&hair);
        let layout = params.layout();
        assert!(params.has_layout(&layout));
        assert_eq!(layout.ids(), params.ids());
        assert!(params.clone().has_layout(&layout));
        let mut copy = ParameterValues::from_model(&ugoe_model::fixtures::single_quad());
        assert!(!copy.has_layout(&layout));
        copy.clone_from(&params);
        assert!(copy.has_layout(&layout));
        // 値を書き換えても並びは変わらない。
        copy.set(ParamIndex(0), 1.0);
        assert!(copy.has_layout(&layout));
        // 並びを変えたものは違う。
        let mut reversed = hair.clone();
        reversed.parameters.reverse();
        assert!(!ParameterValues::from_model(&reversed).has_layout(&layout));
        // 別々に作ったものは、ID の列が同じでも違う (ポインタの比較)。
        let fresh = ParameterValues::from_model(&hair);
        assert!(!fresh.has_layout(&layout));
        assert_eq!(fresh.ids(), layout.ids());
    }

    #[test]
    fn clone_from_copies_values_ranges_and_ids() {
        let hair = ugoe_model::fixtures::physics_hair();
        let mut source = ParameterValues::from_model(&hair);
        let index = source.index_of(ugoe_model::standard::ANGLE_X).unwrap();
        source.set(index, 7.0);
        // 同じ並び: 値を写す。
        let mut copy = ParameterValues::from_model(&hair);
        copy.clone_from(&source);
        assert_eq!(copy.as_slice(), source.as_slice());
        assert_eq!(copy.ids(), source.ids());
        // 違う並び・値域: ID と値域も写す。
        let mut other = ugoe_model::fixtures::physics_relay();
        other.parameters.reverse();
        for p in &mut other.parameters {
            p.max = 99.0;
        }
        let mut copy = ParameterValues::from_model(&other);
        copy.clone_from(&source);
        assert_eq!(copy.ids(), source.ids());
        assert_eq!(copy.as_slice(), source.as_slice());
        assert_eq!(copy.max(index), source.max(index));
        assert_eq!(copy.min(index), source.min(index));
        assert_eq!(copy.default_value(index), source.default_value(index));
    }
}
