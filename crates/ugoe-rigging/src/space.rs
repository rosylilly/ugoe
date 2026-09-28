//! 要素の親の座標系 (`docs/spec/keyform-inference.md#対称軸`)。

use ugoe_model::{DeformerId, DeformerKind, Model};

use crate::InferError;
use crate::grid::Layout;

/// 親の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpaceKind {
    /// 親なし (キャンバス座標)。
    Canvas,
    /// ワープデフォーマの UV 空間。
    Warp,
    /// 回転デフォーマの局所座標 (原点が (0,0))。
    Rotation,
}

/// 要素の頂点・制御点・原点を表す、親の座標系。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Space {
    pub kind: SpaceKind,
    /// この座標系の 1 単位が、基準のセルでキャンバスの何ピクセルにあたるか (おおよそ)。
    ///
    /// キャンバスは 1。回転デフォーマはその基準のセルの拡大率を掛ける。ワープはその基準のセルの
    /// 制御点の外接矩形の長いほうの辺を掛ける。回転デフォーマの子にはワープの伸縮が伝わらない
    /// (`docs/spec/evaluation.md#回転デフォーマ`) ので、ワープの下の回転デフォーマでたどるのをやめる。
    pub unit: f32,
}

impl Space {
    /// `mirror_axis` を省略したときの対称軸 (`docs/spec/keyform-inference.md#対称軸`)。
    pub fn default_axis(&self, model: &Model) -> f32 {
        match self.kind {
            SpaceKind::Canvas => model.meta.canvas.origin.x,
            SpaceKind::Warp => 0.5,
            SpaceKind::Rotation => 0.0,
        }
    }
}

/// 親 `parent` の座標系。
pub(crate) fn parent_space(
    model: &Model,
    parent: Option<&DeformerId>,
) -> Result<Space, InferError> {
    let mut kind = SpaceKind::Canvas;
    let mut unit = 1.0;
    let mut current = parent.cloned();
    let mut steps = 0;
    while let Some(id) = current {
        steps += 1;
        if steps > model.deformers.len() {
            return Err(InferError::ParentCycle { deformer: id });
        }
        let deformer = model.deformers.iter().find(|d| d.id == id).ok_or_else(|| {
            InferError::UnknownParent {
                deformer: id.clone(),
            }
        })?;
        let under_warp = |parent: &Option<DeformerId>| {
            parent.as_ref().is_some_and(|p| {
                model
                    .deformers
                    .iter()
                    .any(|d| &d.id == p && matches!(d.kind, DeformerKind::Warp(_)))
            })
        };
        let (this, scale, next) = if let DeformerKind::Warp(warp) = &deformer.kind {
            let layout = Layout::new(&model.parameters, &warp.keyforms)?;
            let points = &warp.keyforms.keyforms[layout.base()].points;
            (SpaceKind::Warp, extent(points), deformer.parent.clone())
        } else if let DeformerKind::Rotation(rotation) = &deformer.kind {
            let layout = Layout::new(&model.parameters, &rotation.keyforms)?;
            let scale = rotation.keyforms.keyforms[layout.base()].scale;
            let next = (!under_warp(&deformer.parent))
                .then(|| deformer.parent.clone())
                .flatten();
            (SpaceKind::Rotation, scale, next)
        } else {
            return Err(InferError::UnsupportedParent { deformer: id });
        };
        if steps == 1 {
            kind = this;
        }
        unit *= scale;
        current = next;
    }
    // 潰れた格子などで大きさが分からないときは、ピクセルと同じとみなす。
    if !(unit.is_finite() && unit > 0.0) {
        unit = 1.0;
    }
    Ok(Space { kind, unit })
}

/// 点の外接矩形の長いほうの辺の長さ。
fn extent(points: &[ugoe_core::Vec2]) -> f32 {
    let Some(&first) = points.first() else {
        return 0.0;
    };
    let (min, max) = points
        .iter()
        .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)));
    (max - min).max_element()
}
