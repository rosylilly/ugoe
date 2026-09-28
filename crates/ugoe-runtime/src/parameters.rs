use ugoe_model::{Model, ParamId};

/// コンパイル済みモデル内でのパラメータ連番。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamIndex(pub u32);

/// パラメータの現在値。モーション・物理・アプリはここを書き換える。
#[derive(Debug, Clone)]
pub struct ParameterValues {
    ids: Vec<ParamId>,
    min: Vec<f32>,
    max: Vec<f32>,
    default: Vec<f32>,
    values: Vec<f32>,
}

impl ParameterValues {
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

    pub fn index_of(&self, id: &str) -> Option<ParamIndex> {
        self.ids
            .iter()
            .position(|p| p.as_str() == id)
            .map(|i| ParamIndex(i as u32))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: ParamIndex) -> f32 {
        self.values[index.0 as usize]
    }

    /// 値域にクランプして設定する。
    pub fn set(&mut self, index: ParamIndex, value: f32) {
        let i = index.0 as usize;
        self.values[i] = value.clamp(self.min[i], self.max[i]);
    }

    /// 現在値に `weight` の重みで `value` をブレンドする (モーションのフェード用)。
    pub fn blend(&mut self, index: ParamIndex, value: f32, weight: f32) {
        let current = self.get(index);
        self.set(index, current + (value - current) * weight);
    }

    pub fn reset(&mut self) {
        self.values.copy_from_slice(&self.default);
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }
}
