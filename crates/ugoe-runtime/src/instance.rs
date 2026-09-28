use std::sync::Arc;

use ugoe_core::{Rgb, Vec2};
use ugoe_model::BlendMode;

use crate::keyform::{self, Weights};
use crate::{CompiledModel, ParameterValues};

/// 描画可能要素 (メッシュ) 1 つの評価結果。座標はキャンバス座標系。
#[derive(Debug, Clone, PartialEq)]
pub struct DrawableState {
    pub positions: Vec<Vec2>,
    pub opacity: f32,
    pub draw_order: f32,
    pub multiply_color: Rgb,
    pub screen_color: Rgb,
    pub blend: BlendMode,
    pub visible: bool,
}

/// モデル 1 体分の実行時状態。
#[derive(Debug)]
pub struct ModelInstance {
    compiled: Arc<CompiledModel>,
    parameters: ParameterValues,
    drawables: Vec<DrawableState>,
    render_order: Vec<u32>,
    scratch: Weights,
}

impl ModelInstance {
    pub fn new(compiled: Arc<CompiledModel>) -> Self {
        let parameters = ParameterValues::from_model(&compiled.model);
        let drawables = compiled
            .model
            .meshes
            .iter()
            .map(|m| DrawableState {
                positions: vec![Vec2::ZERO; m.uvs.len()],
                opacity: 1.0,
                draw_order: 0.0,
                multiply_color: Rgb::WHITE,
                screen_color: Rgb::BLACK,
                blend: m.blend,
                visible: true,
            })
            .collect();
        let mut instance = Self {
            compiled,
            parameters,
            drawables,
            render_order: Vec::new(),
            scratch: Weights::new(),
        };
        instance.update();
        instance
    }

    pub fn compiled(&self) -> &Arc<CompiledModel> {
        &self.compiled
    }

    pub fn parameters(&self) -> &ParameterValues {
        &self.parameters
    }

    pub fn parameters_mut(&mut self) -> &mut ParameterValues {
        &mut self.parameters
    }

    /// 現在のパラメータ値からすべての描画可能要素を再計算する。
    pub fn update(&mut self) {
        let values = self.parameters.as_slice();
        for (i, mesh) in self.compiled.model.meshes.iter().enumerate() {
            keyform::weights(&self.compiled.mesh_bindings[i], values, &mut self.scratch);
            let grid = &mesh.keyforms;
            let out = &mut self.drawables[i];
            out.positions.fill(Vec2::ZERO);
            let mut mul = [0.0f32; 3];
            let mut scr = [0.0f32; 3];
            for &(k, w) in &self.scratch {
                let kf = &grid.keyforms[k];
                for (p, q) in out.positions.iter_mut().zip(&kf.positions) {
                    *p += *q * w;
                }
                mul[0] += kf.multiply_color.r * w;
                mul[1] += kf.multiply_color.g * w;
                mul[2] += kf.multiply_color.b * w;
                scr[0] += kf.screen_color.r * w;
                scr[1] += kf.screen_color.g * w;
                scr[2] += kf.screen_color.b * w;
            }
            out.opacity = keyform::blend_scalar(grid, &self.scratch, |k| k.opacity);
            out.draw_order = keyform::blend_scalar(grid, &self.scratch, |k| k.draw_order);
            out.multiply_color = Rgb::new(mul[0], mul[1], mul[2]);
            out.screen_color = Rgb::new(scr[0], scr[1], scr[2]);
            // TODO(T0104): 親デフォーマ変形・パーツ不透明度の乗算。
            out.visible = out.opacity > 0.0;
        }
        self.sort_render_order();
    }

    /// 描画可能要素の評価結果 (モデル宣言順)。
    pub fn drawables(&self) -> &[DrawableState] {
        &self.drawables
    }

    /// 奥から手前への描画順 (drawables のインデックス)。
    pub fn render_order(&self) -> &[u32] {
        &self.render_order
    }

    fn sort_render_order(&mut self) {
        self.render_order.clear();
        self.render_order.extend(0..self.drawables.len() as u32);
        let d = &self.drawables;
        // 安定ソート: 同じ描画順なら宣言順。
        self.render_order.sort_by(|&a, &b| {
            d[a as usize]
                .draw_order
                .total_cmp(&d[b as usize].draw_order)
        });
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    #[test]
    fn single_quad_translates_with_parameter() {
        let compiled = CompiledModel::compile(fixtures::single_quad()).unwrap();
        let mut instance = ModelInstance::new(compiled);
        assert_eq!(instance.drawables()[0].positions[0], Vec2::new(64.0, 64.0));

        let x = instance.parameters().index_of("X").unwrap();
        instance.parameters_mut().set(x, 0.5);
        instance.update();
        assert_eq!(instance.drawables()[0].positions[0], Vec2::new(96.0, 64.0));

        instance.parameters_mut().set(x, 10.0); // クランプされる
        instance.update();
        assert_eq!(instance.drawables()[0].positions[0], Vec2::new(128.0, 64.0));
    }
}
