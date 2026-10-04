//! パラメータの ID から解決した添字の控え。並びが変わったら解決し直す (T0325)。

use ugoe_runtime::{ParamIndex, ParameterLayout, ParameterValues};

/// ID の列を `ParameterValues` の添字に解決して持つ。
///
/// 解決したときの並びの手札を控え、次に渡された `ParameterValues` が同じ並びなら定数時間で
/// そのまま使う。手札が違っても ID の列が同じ (`from_model` で作り直したなど) なら、解決し直さずに
/// 手札だけ取り替える。並びが違えば解決し直す (別のパラメータに書かないため)。
#[derive(Debug, Clone, Default)]
pub(crate) struct ResolvedParams {
    layout: Option<ParameterLayout>,
    indices: Vec<Option<ParamIndex>>,
}

impl ResolvedParams {
    /// `parameters` の並びでの添字 (`ids` と同じ順。`ids` の `None` とモデルにない ID は `None`)。
    ///
    /// `ids` は解決し直すときだけ読む。同じ持ち主には毎回同じ列を渡すこと。
    pub(crate) fn get<'a>(
        &mut self,
        parameters: &ParameterValues,
        ids: impl IntoIterator<Item = Option<&'a str>>,
    ) -> &[Option<ParamIndex>] {
        if let Some(layout) = &self.layout {
            if parameters.has_layout(layout) {
                return &self.indices;
            }
            if layout.ids() == parameters.ids() {
                self.layout = Some(parameters.layout());
                return &self.indices;
            }
        }
        self.indices.clear();
        self.indices.extend(
            ids.into_iter()
                .map(|id| id.and_then(|id| parameters.index_of(id))),
        );
        self.layout = Some(parameters.layout());
        &self.indices
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    #[test]
    fn resolves_again_only_when_layout_changes() {
        let mut model = fixtures::single_quad();
        let mut y = model.parameters[0].clone();
        y.id = "Y".into();
        model.parameters.push(y);
        let params = ParameterValues::from_model(&model);
        let mut resolved = ResolvedParams::default();
        let ids = [Some("Y"), None, Some("Missing"), Some("X")];
        assert_eq!(
            resolved.get(&params, ids),
            [Some(ParamIndex(1)), None, None, Some(ParamIndex(0))]
        );
        // 同じ並びなら `ids` を読まない。
        assert_eq!(resolved.get(&params.clone(), []).len(), 4);
        // 作り直した同じ並びも、手札を取り替えるだけ。
        let fresh = ParameterValues::from_model(&model);
        assert_eq!(resolved.get(&fresh, []).len(), 4);
        assert!(fresh.has_layout(resolved.layout.as_ref().unwrap()));
        // 並びが違えば解決し直す。
        model.parameters.reverse();
        let reversed = ParameterValues::from_model(&model);
        assert_eq!(
            resolved.get(&reversed, ids),
            [Some(ParamIndex(0)), None, None, Some(ParamIndex(1))]
        );
    }
}
