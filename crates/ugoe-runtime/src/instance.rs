use std::sync::Arc;

use ugoe_core::{Affine2, Rgb, Vec2};
use ugoe_model::{BlendMode, DeformerKind, Model};

use crate::deform::{Rotation, Transform, Warp};
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
    /// 祖先を掛け合わせたパーツの不透明度。
    part_opacity: Vec<f32>,
    /// 祖先を掛け合わせたデフォーマの不透明度。
    deformer_opacity: Vec<f32>,
    /// 回転デフォーマのワールド変換 (ワープの要素は使わない)。
    deformer_affine: Vec<Affine2>,
    /// ワープデフォーマのワールド座標の制御点 (回転の要素は空)。
    warp_points: Vec<Vec<Vec2>>,
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
        let model = &compiled.model;
        let warp_points = model
            .deformers
            .iter()
            .map(|d| match &d.kind {
                DeformerKind::Warp(w) => {
                    vec![Vec2::ZERO; (usize::from(w.rows) + 1) * (usize::from(w.cols) + 1)]
                }
                DeformerKind::Rotation(_) => Vec::new(),
            })
            .collect();
        // 重みの個数は軸数 N に対して最大 2^N (`keyform::weights`)。
        let max_weights = 1usize << compiled.max_axes.min(20);
        let mut instance = Self {
            parameters,
            drawables,
            render_order: Vec::with_capacity(model.meshes.len()),
            scratch: Weights::with_capacity(max_weights),
            part_opacity: vec![1.0; model.parts.len()],
            deformer_opacity: vec![1.0; model.deformers.len()],
            deformer_affine: vec![Affine2::IDENTITY; model.deformers.len()],
            warp_points,
            compiled,
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

    /// 現在のパラメータ値からすべての描画可能要素を再計算する (`docs/spec/evaluation.md` の 2.)。
    ///
    /// ヒープ確保はしない (バッファは [`ModelInstance::new`] で確保したものを再利用する)。
    pub fn update(&mut self) {
        self.update_parts();
        self.update_deformers();
        self.update_meshes();
        self.apply_glues();
        self.sort_render_order();
    }

    /// パーツの不透明度を根から葉へ伝播する。
    fn update_parts(&mut self) {
        let compiled = &*self.compiled;
        let values = self.parameters.as_slice();
        for &p in &compiled.part_order {
            let p = p as usize;
            let part = &compiled.parts[p];
            keyform::weights(&part.bindings, values, &mut self.scratch);
            let own =
                keyform::blend_scalar(&compiled.model.parts[p].keyforms, &self.scratch, |k| {
                    k.opacity
                });
            let parent = part.parent.map_or(1.0, |i| self.part_opacity[i as usize]);
            self.part_opacity[p] = own * parent;
        }
    }

    /// デフォーマをトポロジカル順に評価し、ワールド変換と不透明度を作る。
    fn update_deformers(&mut self) {
        let compiled = &*self.compiled;
        let model = &compiled.model;
        let values = self.parameters.as_slice();
        for &d in &compiled.deformer_order {
            let d = d as usize;
            let deformer = &compiled.deformers[d];
            keyform::weights(&deformer.bindings, values, &mut self.scratch);
            let weights = &self.scratch;
            let own_opacity = match &model.deformers[d].kind {
                DeformerKind::Warp(warp) => {
                    // 自分の制御点を取り出して親の変換を借りる (空の Vec との交換なので確保は起きない)。
                    let mut points = std::mem::take(&mut self.warp_points[d]);
                    points.fill(Vec2::ZERO);
                    for &(k, w) in weights {
                        for (p, q) in points.iter_mut().zip(&warp.keyforms.keyforms[k].points) {
                            *p += *q * w;
                        }
                    }
                    let parent = world_transform(
                        model,
                        &self.deformer_affine,
                        &self.warp_points,
                        deformer.parent,
                    );
                    for p in &mut points {
                        *p = parent.apply(*p);
                    }
                    self.warp_points[d] = points;
                    keyform::blend_scalar(&warp.keyforms, weights, |k| k.opacity)
                }
                DeformerKind::Rotation(rotation) => {
                    let grid = &rotation.keyforms;
                    let local = Rotation {
                        origin: keyform::blend_vec2(grid, weights, |k| k.origin),
                        angle: keyform::blend_scalar(grid, weights, |k| k.angle),
                        scale: keyform::blend_scalar(grid, weights, |k| k.scale),
                        reflect_x: keyform::blend_bool(grid, weights, |k| k.reflect_x),
                        reflect_y: keyform::blend_bool(grid, weights, |k| k.reflect_y),
                    };
                    let parent = world_transform(
                        model,
                        &self.deformer_affine,
                        &self.warp_points,
                        deformer.parent,
                    );
                    self.deformer_affine[d] = parent.rotation_child(&local);
                    keyform::blend_scalar(grid, weights, |k| k.opacity)
                }
            };
            let parent_opacity = deformer
                .parent
                .map_or(1.0, |i| self.deformer_opacity[i as usize]);
            self.deformer_opacity[d] = own_opacity * parent_opacity;
        }
    }

    /// メッシュのキーフォームを補間し、親デフォーマのワールド変換でキャンバス座標に写す。
    fn update_meshes(&mut self) {
        let compiled = &*self.compiled;
        let model = &compiled.model;
        let values = self.parameters.as_slice();
        for (i, mesh) in model.meshes.iter().enumerate() {
            let cm = &compiled.meshes[i];
            keyform::weights(&cm.bindings, values, &mut self.scratch);
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
            let parent =
                world_transform(model, &self.deformer_affine, &self.warp_points, cm.parent);
            for p in &mut out.positions {
                *p = parent.apply(*p);
            }
            let own = keyform::blend_scalar(grid, &self.scratch, |k| k.opacity);
            let part = self.part_opacity[cm.part as usize];
            let deformer = cm.parent.map_or(1.0, |d| self.deformer_opacity[d as usize]);
            out.opacity = own * part * deformer;
            out.draw_order = keyform::blend_scalar(grid, &self.scratch, |k| k.draw_order);
            out.multiply_color = Rgb::new(mul[0], mul[1], mul[2]);
            out.screen_color = Rgb::new(scr[0], scr[1], scr[2]);
            out.visible = out.opacity > 0.0;
        }
    }

    /// グルーを `glues` 配列の順に適用する。
    fn apply_glues(&mut self) {
        let compiled = &*self.compiled;
        let values = self.parameters.as_slice();
        for (glue, cg) in compiled.model.glues.iter().zip(&compiled.glues) {
            keyform::weights(&cg.bindings, values, &mut self.scratch);
            let intensity = keyform::blend_scalar(&glue.keyforms, &self.scratch, |k| k.intensity);
            let (a, b) = (cg.mesh_a as usize, cg.mesh_b as usize);
            for pair in &glue.pairs {
                let (ia, ib) = (usize::from(pair.a), usize::from(pair.b));
                let pa = self.drawables[a].positions[ia];
                let pb = self.drawables[b].positions[ib];
                let target = lerp(pa, pb, pair.weight);
                self.drawables[a].positions[ia] = lerp(pa, target, intensity);
                self.drawables[b].positions[ib] = lerp(pb, target, intensity);
            }
        }
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
        // 同じ描画順なら宣言順 (安定ソートと同じ結果)。インデックスを第 2 キーにして
        // 全順序にすれば、確保をしない不安定ソートで済む。
        self.render_order.sort_unstable_by(|&a, &b| {
            d[a as usize]
                .draw_order
                .total_cmp(&d[b as usize].draw_order)
                .then(a.cmp(&b))
        });
    }
}

/// `lerp(a, b, t) = a + (b - a) * t`。
fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

/// デフォーマ `parent` のワールド変換 (`None` なら恒等写像)。`parent` は評価済みであること。
fn world_transform<'a>(
    model: &Model,
    affine: &[Affine2],
    warp_points: &'a [Vec<Vec2>],
    parent: Option<u32>,
) -> Transform<'a> {
    let Some(d) = parent else {
        return Transform::Identity;
    };
    let d = d as usize;
    match &model.deformers[d].kind {
        DeformerKind::Warp(w) => Transform::Warp(Warp::new(
            &warp_points[d],
            usize::from(w.cols),
            usize::from(w.rows),
        )),
        DeformerKind::Rotation(_) => Transform::Affine(affine[d]),
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

    /// すべてのバッファの (先頭アドレス, 容量)。再確保が起きれば変わる。
    fn buffers(instance: &ModelInstance) -> Vec<(usize, usize)> {
        fn id<T>(v: &Vec<T>) -> (usize, usize) {
            (v.as_ptr() as usize, v.capacity())
        }
        let mut out = vec![
            id(&instance.drawables),
            id(&instance.render_order),
            id(&instance.scratch),
            id(&instance.part_opacity),
            id(&instance.deformer_opacity),
            id(&instance.deformer_affine),
            id(&instance.warp_points),
        ];
        out.extend(instance.drawables.iter().map(|d| id(&d.positions)));
        out.extend(instance.warp_points.iter().map(id));
        out
    }

    // README.md「update() で毎フレームのヒープ確保をしない」: new で確保したバッファが
    // update で再確保されないことを、アドレスと容量が変わらないことで確かめる。
    #[test]
    fn update_does_not_reallocate_buffers() {
        for model in [
            fixtures::single_quad(),
            fixtures::warp_grid(),
            fixtures::rotation_chain(),
            fixtures::rotation_under_warp(),
            fixtures::part_opacity_hierarchy(),
            fixtures::glued_quads(),
        ] {
            let mut instance = ModelInstance::new(CompiledModel::compile(model).unwrap());
            let before = buffers(&instance);
            let params: Vec<_> = instance
                .compiled()
                .model()
                .parameters
                .iter()
                .map(|p| (p.min, p.max))
                .collect();
            for t in [0.0, 0.3, 0.5, 1.0] {
                for (i, &(min, max)) in params.iter().enumerate() {
                    instance
                        .parameters_mut()
                        .set(crate::ParamIndex(i as u32), min + (max - min) * t);
                }
                instance.update();
                assert_eq!(buffers(&instance), before);
            }
        }
    }
}
