use std::collections::HashMap;

use ugoe_model::{Model, ParamId};

/// コンパイル済みモデル内でのパラメータ連番 (`Model::parameters` のインデックス)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamIndex(pub u32);

/// パラメータの現在値。モーション・物理・アプリはここを書き換える
/// (`spec/evaluation.md#parameter-values`)。
///
/// - 初期値はパラメータの `default`。
/// - 書き込む値はすべて値域 `[min, max]` にクランプする。したがって評価はいつも値域の中の値を見る。
/// - NaN の書き込みは無視する (値は変わらない)。無限大は値域の端にクランプする。
#[derive(Debug, Clone)]
pub struct ParameterValues {
    ids: Vec<ParamId>,
    min: Vec<f32>,
    max: Vec<f32>,
    default: Vec<f32>,
    values: Vec<f32>,
}

impl ParameterValues {
    /// `model` のパラメータの並びで作る。値はすべて既定値。
    pub fn from_model(model: &Model) -> Self {
        let p = &model.parameters;
        Self {
            ids: p.iter().map(|p| p.id.clone()).collect(),
            min: p.iter().map(|p| p.min).collect(),
            max: p.iter().map(|p| p.max).collect(),
            default: p.iter().map(|p| p.default).collect(),
            values: p.iter().map(|p| p.default).collect(),
        }
    }

    /// ID が `id` のパラメータの連番。なければ `None`。
    pub fn index_of(&self, id: &str) -> Option<ParamIndex> {
        self.ids
            .iter()
            .position(|p| p.as_str() == id)
            .map(|i| ParamIndex(i as u32))
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
        let values: HashMap<&str, f32> = previous
            .ids
            .iter()
            .map(|id| id.as_str())
            .zip(previous.values.iter().copied())
            .collect();
        for i in 0..self.ids.len() {
            if let Some(&value) = values.get(self.ids[i].as_str()) {
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
}
