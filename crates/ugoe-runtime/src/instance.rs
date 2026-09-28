use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// マスクの効き具合。キーフォームを補間して 0〜1 にクランプした値 (`docs/spec/rendering.md#マスク`)。
    /// マスクを持たないメッシュでは意味を持たない。
    pub mask_strength: f32,
    /// 色として描くか。不透明度が 0 のメッシュとマスク専用メッシュ (`Mesh::mask_only`) は `false`。
    /// マスクとしての効き具合には影響しない。
    pub visible: bool,
}

/// 描画可能要素 1 つについて、前回の [`ModelInstance::update`] から変わった出力の集合
/// (`docs/spec/evaluation.md#8-出力`)。
///
/// ビットの集合で、定数を `|` で組み合わせて [`DrawableChanges::contains`] などで調べる。
/// 比較は値のビット列の一致で行う (`-0.0` と `0.0` は異なる、同じビット列の NaN は同じ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DrawableChanges(u8);

impl DrawableChanges {
    /// 何も変わっていない。
    pub const NONE: Self = Self(0);
    /// `positions` (頂点位置のどれか)。
    pub const POSITIONS: Self = Self(1 << 0);
    /// `opacity`。
    pub const OPACITY: Self = Self(1 << 1);
    /// `draw_order` (この要素自身の値。並び全体の変化は [`ModelInstance::render_order_changed`])。
    pub const DRAW_ORDER: Self = Self(1 << 2);
    /// `visible`。
    pub const VISIBILITY: Self = Self(1 << 3);
    /// `multiply_color`。
    pub const MULTIPLY_COLOR: Self = Self(1 << 4);
    /// `screen_color`。
    pub const SCREEN_COLOR: Self = Self(1 << 5);
    /// `mask_strength`。
    pub const MASK_STRENGTH: Self = Self(1 << 6);
    /// 上のすべて。
    pub const ALL: Self = Self((1 << 7) - 1);

    /// ビット列 (C ABI などに渡す用)。
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// 定義されていないビットを捨てて作る。
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL.0)
    }

    /// 何も変わっていないか。
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `other` のビットをすべて含むか。
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `other` のビットを 1 つでも含むか。
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// `other` のビットを加える。
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitOr for DrawableChanges {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DrawableChanges {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for DrawableChanges {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// モデル 1 体分の実行時状態。
#[derive(Debug)]
pub struct ModelInstance {
    /// プロセス内で一意な番号 ([`ModelInstance::id`])。
    id: u64,
    /// `update` を呼んだ回数 ([`ModelInstance::update_count`])。
    update_count: u64,
    compiled: Arc<CompiledModel>,
    parameters: ParameterValues,
    drawables: Vec<DrawableState>,
    render_order: Vec<u32>,
    /// 前回の `update` の出力 (変更フラグを作るための比較用。`blend` は使わない)。
    previous: Vec<DrawableState>,
    previous_render_order: Vec<u32>,
    changes: Vec<DrawableChanges>,
    render_order_changed: bool,
    /// 次の `update` で、比較せずにすべてを変更ありとして報告する。
    report_all: bool,
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
        let drawables: Vec<DrawableState> = compiled
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
                mask_strength: 1.0,
                visible: !m.mask_only,
            })
            .collect();
        let model = &compiled.model;
        let mesh_count = model.meshes.len();
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
        // 番号は一意であればよい (順序は使わない) ので Relaxed で足りる。
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let mut instance = Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            update_count: 0,
            parameters,
            render_order: Vec::with_capacity(mesh_count),
            previous: drawables.clone(),
            previous_render_order: Vec::with_capacity(mesh_count),
            changes: vec![DrawableChanges::ALL; mesh_count],
            render_order_changed: true,
            report_all: true,
            drawables,
            scratch: Weights::with_capacity(max_weights),
            part_opacity: vec![1.0; model.parts.len()],
            deformer_opacity: vec![1.0; model.deformers.len()],
            deformer_affine: vec![Affine2::IDENTITY; model.deformers.len()],
            warp_points,
            compiled,
        };
        instance.update();
        // new の中の update はホストから見えないので、ホストが最初に呼ぶ update も
        // すべてを変更ありとして報告する (new の直後に update してから読むホストのため)。
        instance.report_all = true;
        instance
    }

    /// このインスタンスを識別する番号。プロセス内で一意で、インスタンスの生存中は変わらない。
    ///
    /// 描画側が「どのインスタンスの出力を転送済みか」を覚えるために使う
    /// (`docs/spec/evaluation.md#変更フラグ`)。
    pub fn id(&self) -> u64 {
        self.id
    }

    /// [`ModelInstance::update`] を呼んだ回数 ([`ModelInstance::new`] の中の 1 回を含む)。
    ///
    /// 出力 (`drawables` など) は `update` でだけ変わるので、この値が同じなら出力も同じ。
    /// 前回見た値のちょうど 1 つ後なら、[`ModelInstance::changes`] はそのとき見た出力からの変更を表す
    /// (`docs/spec/evaluation.md#変更フラグ`)。
    pub fn update_count(&self) -> u64 {
        self.update_count
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
        self.detect_changes();
        self.update_count += 1;
    }

    /// 次の [`ModelInstance::update`] で、比較せずにすべての要素と描画順を変更ありとして報告させる。
    ///
    /// ホストが描画側の資源 (GPU バッファなど) を作り直したときに使う。
    pub fn mark_all_changed(&mut self) {
        self.report_all = true;
    }

    /// 前回の `update` の出力と比べて変更フラグを作り、今回の出力を控えておく。
    ///
    /// 比較はビット列の一致で行う (決定的で、NaN が続いても毎フレーム変更ありにならない)。
    /// 頂点位置は変わったときだけコピーするので、止まっているメッシュの費用は比較 1 回分で済む。
    fn detect_changes(&mut self) {
        let report_all = std::mem::take(&mut self.report_all);
        for ((cur, prev), changes) in self
            .drawables
            .iter()
            .zip(&mut self.previous)
            .zip(&mut self.changes)
        {
            let mut c = DrawableChanges::NONE;
            if report_all || !same_positions(&cur.positions, &prev.positions) {
                c |= DrawableChanges::POSITIONS;
                prev.positions.copy_from_slice(&cur.positions);
            }
            if report_all || !same_f32(cur.opacity, prev.opacity) {
                c |= DrawableChanges::OPACITY;
            }
            if report_all || !same_f32(cur.draw_order, prev.draw_order) {
                c |= DrawableChanges::DRAW_ORDER;
            }
            if report_all || cur.visible != prev.visible {
                c |= DrawableChanges::VISIBILITY;
            }
            if report_all || !same_rgb(cur.multiply_color, prev.multiply_color) {
                c |= DrawableChanges::MULTIPLY_COLOR;
            }
            if report_all || !same_rgb(cur.screen_color, prev.screen_color) {
                c |= DrawableChanges::SCREEN_COLOR;
            }
            if report_all || !same_f32(cur.mask_strength, prev.mask_strength) {
                c |= DrawableChanges::MASK_STRENGTH;
            }
            prev.opacity = cur.opacity;
            prev.draw_order = cur.draw_order;
            prev.visible = cur.visible;
            prev.multiply_color = cur.multiply_color;
            prev.screen_color = cur.screen_color;
            prev.mask_strength = cur.mask_strength;
            *changes = c;
        }
        self.render_order_changed = report_all || self.render_order != self.previous_render_order;
        if self.render_order_changed {
            self.previous_render_order.clear();
            self.previous_render_order
                .extend_from_slice(&self.render_order);
        }
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
            out.mask_strength =
                keyform::blend_scalar(grid, &self.scratch, |k| k.mask_strength).clamp(0.0, 1.0);
            out.visible = !mesh.mask_only && out.opacity > 0.0;
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

    /// 前回の [`ModelInstance::update`] から変わった出力 (`drawables` と同じ並び、
    /// `docs/spec/evaluation.md#8-出力`)。
    ///
    /// [`ModelInstance::new`] の直後と、その後に最初に呼んだ `update` の後は、すべてが変更ありになる。
    pub fn changes(&self) -> &[DrawableChanges] {
        &self.changes
    }

    /// 前回の [`ModelInstance::update`] から [`ModelInstance::render_order`] が変わったか。
    /// 最初の報告では `true` (規則は [`ModelInstance::changes`] と同じ)。
    pub fn render_order_changed(&self) -> bool {
        self.render_order_changed
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

/// ビット列が同じか (変更フラグ用)。
fn same_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn same_rgb(a: Rgb, b: Rgb) -> bool {
    same_f32(a.r, b.r) && same_f32(a.g, b.g) && same_f32(a.b, b.b)
}

fn same_positions(a: &[Vec2], b: &[Vec2]) -> bool {
    a.iter()
        .zip(b)
        .all(|(p, q)| same_f32(p.x, q.x) && same_f32(p.y, q.y))
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
            id(&instance.previous),
            id(&instance.previous_render_order),
            id(&instance.changes),
            id(&instance.scratch),
            id(&instance.part_opacity),
            id(&instance.deformer_opacity),
            id(&instance.deformer_affine),
            id(&instance.warp_points),
        ];
        out.extend(instance.drawables.iter().map(|d| id(&d.positions)));
        out.extend(instance.previous.iter().map(|d| id(&d.positions)));
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
            fixtures::nested_masks(),
            fixtures::mixed_masks(),
            fixtures::intersection_masks(),
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

    // ---- 変更フラグ (docs/spec/evaluation.md#8-出力) ----

    fn instance(model: Model) -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(model).unwrap())
    }

    fn set(instance: &mut ModelInstance, id: &str, value: f32) {
        let i = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(i, value);
    }

    fn mesh_index(instance: &ModelInstance, id: &str) -> usize {
        let meshes = &instance.compiled().model().meshes;
        meshes.iter().position(|m| m.id.as_str() == id).unwrap()
    }

    fn assert_no_changes(instance: &ModelInstance) {
        assert!(instance.changes().iter().all(|c| c.is_empty()));
        assert!(!instance.render_order_changed());
    }

    #[test]
    fn changes_report_everything_after_new_and_first_update() {
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        assert_eq!(instance.changes(), &[DrawableChanges::ALL; 4]);
        assert!(instance.render_order_changed());

        // パラメータを変えなくても、ホストが最初に呼ぶ update はすべてを報告する。
        instance.update();
        assert_eq!(instance.changes(), &[DrawableChanges::ALL; 4]);
        assert!(instance.render_order_changed());

        instance.update();
        assert_no_changes(&instance);
    }

    #[test]
    fn unchanged_parameters_report_no_changes() {
        for model in [
            fixtures::single_quad(),
            fixtures::warp_grid(),
            fixtures::rotation_chain(),
            fixtures::rotation_under_warp(),
            fixtures::part_opacity_hierarchy(),
            fixtures::glued_quads(),
            fixtures::mixed_masks(),
            fixtures::intersection_masks(),
        ] {
            let mut instance = instance(model);
            instance.update();
            for _ in 0..3 {
                instance.update();
                assert_no_changes(&instance);
            }
        }
    }

    #[test]
    fn changes_flag_only_the_mesh_that_moved() {
        // `Flip` は回転 `Hand` の反転だけを変えるので、動くのは `Palm` だけ (`Sleeve` は動かない)。
        let mut instance = instance(fixtures::rotation_chain());
        instance.update();
        let palm = mesh_index(&instance, "Palm");
        set(&mut instance, "Flip", 1.0);
        instance.update();
        for (i, &c) in instance.changes().iter().enumerate() {
            let expected = if i == palm {
                DrawableChanges::POSITIONS
            } else {
                DrawableChanges::NONE
            };
            assert_eq!(c, expected, "mesh {i}");
        }
        assert!(!instance.render_order_changed());

        instance.update();
        assert_no_changes(&instance);

        // 変えた値を戻せば、戻したフレームで再び変更ありになる。
        set(&mut instance, "Flip", 0.0);
        instance.update();
        assert!(instance.changes()[palm].contains(DrawableChanges::POSITIONS));
    }

    #[test]
    fn changes_flag_opacity_visibility_and_draw_order() {
        // `Fade` はパーツ `Root` の不透明度 (全メッシュ)、`B` の不透明度、`D` の描画順を動かす。
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        instance.update();
        set(&mut instance, "Fade", 0.5);
        instance.update();
        let [a, b, c, d] = instance.changes() else {
            panic!("4 meshes");
        };
        assert_eq!(*a, DrawableChanges::OPACITY);
        assert_eq!(*b, DrawableChanges::OPACITY);
        assert_eq!(*c, DrawableChanges::OPACITY);
        assert_eq!(*d, DrawableChanges::OPACITY | DrawableChanges::DRAW_ORDER);
        // 描画順は C(300) → A, B, D(500) から C, D(300) → A, B へ。
        assert!(instance.render_order_changed());
        assert_eq!(instance.render_order(), &[2, 3, 0, 1]);

        // `Fade = 1` で `B` の不透明度が 0 になり、見えなくなる。
        set(&mut instance, "Fade", 1.0);
        instance.update();
        let b = instance.changes()[1];
        assert!(b.contains(DrawableChanges::OPACITY | DrawableChanges::VISIBILITY));
        assert!(!instance.drawables()[1].visible);
        // `D` の描画順は 100 になって先頭へ。
        assert!(instance.changes()[3].contains(DrawableChanges::DRAW_ORDER));
        assert!(instance.render_order_changed());
        assert_eq!(instance.render_order(), &[3, 2, 0, 1]);
        assert!(!instance.changes()[0].contains(DrawableChanges::VISIBILITY));
    }

    #[test]
    fn changes_flag_draw_order_without_render_order_change() {
        // `Fade` 0.2 → 0.21 で `D` の描画順は 420 → 416。並びは C, D, A, B のまま。
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        set(&mut instance, "Fade", 0.2);
        instance.update();
        assert_eq!(instance.render_order(), &[2, 3, 0, 1]);
        set(&mut instance, "Fade", 0.21);
        instance.update();
        assert!(instance.changes()[3].contains(DrawableChanges::DRAW_ORDER));
        assert!(!instance.render_order_changed());
    }

    #[test]
    fn changes_flag_mask_strength_after_clamping() {
        let mut instance = instance(fixtures::mixed_masks());
        instance.update();
        let brow = mesh_index(&instance, "Brow");

        // 補間値は 2 → 1.7 と変わるが、クランプ後は 1 のままなので変更なし。
        set(&mut instance, "Strength", 0.9);
        instance.update();
        assert_no_changes(&instance);

        set(&mut instance, "Strength", 0.5);
        instance.update();
        for (i, &c) in instance.changes().iter().enumerate() {
            let expected = if i == brow {
                DrawableChanges::MASK_STRENGTH
            } else {
                DrawableChanges::NONE
            };
            assert_eq!(c, expected, "mesh {i}");
        }
    }

    #[test]
    fn mark_all_changed_reports_everything_once() {
        let mut instance = instance(fixtures::single_quad());
        instance.update();
        instance.update();
        assert_no_changes(&instance);
        instance.mark_all_changed();
        instance.update();
        assert_eq!(instance.changes(), &[DrawableChanges::ALL]);
        assert!(instance.render_order_changed());
        instance.update();
        assert_no_changes(&instance);
    }

    #[test]
    fn ids_are_unique_and_update_count_increments() {
        let mut a = instance(fixtures::single_quad());
        let b = instance(fixtures::single_quad());
        assert_ne!(a.id(), b.id());
        let (id, count) = (a.id(), a.update_count());
        assert_eq!(count, 1, "new runs update once");
        a.update();
        assert_eq!((a.id(), a.update_count()), (id, count + 1));
        a.mark_all_changed();
        assert_eq!(
            a.update_count(),
            count + 1,
            "only update advances the count"
        );
    }

    #[test]
    fn drawable_changes_set_operations() {
        let c = DrawableChanges::POSITIONS | DrawableChanges::OPACITY;
        assert!(c.contains(DrawableChanges::POSITIONS));
        assert!(!c.contains(DrawableChanges::POSITIONS | DrawableChanges::VISIBILITY));
        assert!(c.intersects(DrawableChanges::OPACITY | DrawableChanges::VISIBILITY));
        assert!(!c.is_empty());
        assert!(DrawableChanges::NONE.is_empty());
        assert!(DrawableChanges::ALL.contains(c));
        assert_eq!(
            DrawableChanges::from_bits_truncate(0xff),
            DrawableChanges::ALL
        );
        assert_eq!(c & DrawableChanges::OPACITY, DrawableChanges::OPACITY);
    }
}
