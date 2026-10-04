use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ugoe_core::{Affine2, Mat2, Rgb, Vec2};
use ugoe_model::deformer::RotationKeyform;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{BlendMode, DeformerKind};

use crate::blend_shape;
use crate::deform::{Rotation, Transform, Warp, lerp};
use crate::dynamic::{Dynamic, DynamicOffsets};
use crate::keyform::{self, GridWeights, Weights};
use crate::{CompiledModel, ParameterValues};

/// 描画可能要素 (メッシュ) 1 つの評価結果。座標はキャンバス座標系。
#[derive(Debug, Clone, PartialEq)]
pub struct DrawableState {
    /// キャンバス座標の頂点位置 (`Mesh::uvs` と同じ並び)。
    pub positions: Vec<Vec2>,
    /// 最終的な不透明度 (メッシュ × パーツ × デフォーマ。クランプしない)。
    pub opacity: f32,
    /// 補間後の描画順 (大きいほど手前)。
    pub draw_order: f32,
    /// 補間後の乗算色 (クランプしない)。
    pub multiply_color: Rgb,
    /// 補間後のスクリーン色 (クランプしない)。
    pub screen_color: Rgb,
    /// 合成モード。未知のモードには通常の合成のフォールバックを適用済み (`BlendMode::effective`)。
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

/// 要素の座標系からキャンバス座標への写像 (`docs/spec/evaluation.md#デフォーマのワールド変換を読む`)。
///
/// [`ModelInstance::deformer_world`] などで得る。直前の [`ModelInstance::update`] で評価した
/// デフォーマの状態を借りるので、`update` がメッシュの頂点に使った写像とビット単位で同じ結果になる。
#[derive(Debug, Clone, Copy)]
pub struct WorldTransform<'a>(Transform<'a>);

impl<'a> WorldTransform<'a> {
    /// 恒等写像 (親を持たない要素の親の座標系)。
    pub const IDENTITY: Self = Self(Transform::Identity);

    /// アフィン変換 (回転デフォーマのワールド変換) の写像。
    pub fn from_affine(affine: Affine2) -> Self {
        Self(Transform::Affine(affine))
    }

    /// キャンバス座標の制御点 `points` (行優先、x が速い) を持つワープの写像。
    /// エディタが、まだモデルにないワープの写像を作るのに使う。
    ///
    /// `cols`・`rows` が 0 か、`points` の長さが `(rows + 1) * (cols + 1)` でなければ `None`。
    pub fn from_warp_points(points: &'a [Vec2], cols: usize, rows: usize) -> Option<Self> {
        if cols == 0 || rows == 0 || points.len() != (rows + 1) * (cols + 1) {
            return None;
        }
        Some(Self(Transform::Warp(Warp::new(points, cols, rows))))
    }

    /// この写像を親に持つ回転デフォーマのワールド変換 (`keyform` は補間後の値。不透明度は使わない)。
    ///
    /// 親がワープなら原点と向きだけを写す (`docs/spec/evaluation.md` の回転デフォーマ)。
    pub fn rotation_child(&self, keyform: &RotationKeyform) -> Affine2 {
        self.0.rotation_child(&Rotation {
            origin: keyform.origin,
            angle: keyform.angle,
            scale: keyform.scale,
            reflect_x: keyform.reflect_x,
            reflect_y: keyform.reflect_y,
        })
    }
}

impl WorldTransform<'_> {
    /// 恒等写像 (親を持たない要素の親の座標系、またはキャンバス座標) か。
    pub fn is_identity(&self) -> bool {
        matches!(self.0, Transform::Identity)
    }

    /// 局所座標の点をキャンバス座標に写す。
    pub fn point(&self, local: Vec2) -> Vec2 {
        self.0.apply(local)
    }

    /// 局所座標の点 `local` での写像のヤコビ行列 (列は局所座標の x・y 方向の微分)。
    ///
    /// 解析的に求める: 恒等写像は単位行列、回転デフォーマ (アフィン) は線形部分、
    /// ワープはセルの双線形式の微分 (範囲外では外挿の規則の微分)。
    /// 祖先の変換はワールド座標の制御点・アフィン変換に畳み込み済みなので、連鎖も含む。
    pub fn jacobian(&self, local: Vec2) -> Mat2 {
        self.0.jacobian(local)
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
    scratch: GridWeights,
    /// ブレンドシェイプ格子の重み (`scratch` と同時に使う)。
    shape_scratch: GridWeights,
    /// 祖先を掛け合わせたパーツの不透明度。
    part_opacity: Vec<f32>,
    /// パーツ自身の不透明度の上書き `(値, 重み)` ([`ModelInstance::set_part_opacity_override`])。
    part_opacity_overrides: Vec<(f32, f32)>,
    /// 祖先を掛け合わせたデフォーマの不透明度。
    deformer_opacity: Vec<f32>,
    /// 回転デフォーマのワールド変換 (ワープの要素は使わない)。
    deformer_affine: Vec<Affine2>,
    /// ワープデフォーマのワールド座標の制御点 (回転の要素は空)。
    warp_points: Vec<Vec<Vec2>>,
    /// 動的物理の加算量と 2 段階の評価の状態。使わないモデルでは `None` のまま
    /// (評価の費用は変わらない)。
    pub(crate) dynamic: Option<Box<Dynamic>>,
}

/// デフォーマの評価の仕方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// すべてを評価し、動的物理の加算量があれば加える。
    Full,
    /// 2 段階の評価の 1 段目: すべてを評価し、加算量は加えない。
    First,
    /// 2 段目: 加算量を受けるデフォーマ、1 段目からパラメータが変わったデフォーマと、
    /// それらの子孫だけを評価し直す。
    Second,
}

impl ModelInstance {
    /// インスタンスを作り、既定値のパラメータで 1 回 [`ModelInstance::update`] する。
    /// 評価に使うバッファはここで確保する。
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
                blend: m.blend.effective(),
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
                DeformerKind::Rotation(_) | DeformerKind::Unknown => Vec::new(),
            })
            .collect();
        // 重みの個数は、線形の軸 2 つ・曲線の軸 4 つまでの積 (`keyform::weights`)。
        // 軸の多い格子で事前確保が膨らみすぎないように上限を置く (超えた分は評価中に確保する)。
        let max_weights = compiled.max_weights.min(1 << 20);
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
            scratch: GridWeights::with_capacity(max_weights),
            shape_scratch: GridWeights::with_capacity(max_weights),
            part_opacity: vec![1.0; model.parts.len()],
            part_opacity_overrides: vec![(1.0, 0.0); model.parts.len()],
            deformer_opacity: vec![1.0; model.deformers.len()],
            deformer_affine: vec![Affine2::IDENTITY; model.deformers.len()],
            warp_points,
            dynamic: None,
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

    /// 評価しているモデル。
    pub fn compiled(&self) -> &Arc<CompiledModel> {
        &self.compiled
    }

    /// パラメータの現在値。
    pub fn parameters(&self) -> &ParameterValues {
        &self.parameters
    }

    /// パラメータの現在値。書き換えは次の [`ModelInstance::update`] で出力に反映される。
    pub fn parameters_mut(&mut self) -> &mut ParameterValues {
        &mut self.parameters
    }

    /// パーツ `part` (`Model::parts` のインデックス) 自身の不透明度を、キーフォームで補間した値 `o` から
    /// `lerp(o, value, weight)` に上書きする。子への伝播はこの値で行う。重み 0 で上書きしない (既定)。
    ///
    /// 次の [`ModelInstance::update`] から効き、変えるまで保たれる。モーションのパーツの不透明度と
    /// ポーズが使う (`docs/spec/evaluation.md#2-評価の順序`)。
    ///
    /// # Panics
    ///
    /// `part` が範囲外のとき。
    pub fn set_part_opacity_override(&mut self, part: usize, value: f32, weight: f32) {
        self.part_opacity_overrides[part] = (value, weight.clamp(0.0, 1.0));
    }

    /// パーツ `part` の不透明度の上書き `(値, 重み)`。
    ///
    /// # Panics
    ///
    /// `part` が範囲外のとき。
    pub fn part_opacity_override(&self, part: usize) -> (f32, f32) {
        self.part_opacity_overrides[part]
    }

    /// すべてのパーツの不透明度の上書きをやめる。
    pub fn clear_part_opacity_overrides(&mut self) {
        self.part_opacity_overrides.fill((1.0, 0.0));
    }

    /// 現在のパラメータ値からすべての描画可能要素を再計算する (`docs/spec/evaluation.md` の 2.)。
    ///
    /// ヒープ確保はしない (バッファは [`ModelInstance::new`] で確保したものを再利用する)。
    ///
    /// [`ModelInstance::update_first_stage`] の後に呼ぶと、2 段階の評価の 2 段目になる
    /// (`docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`)。
    pub fn update(&mut self) {
        let pending = self
            .dynamic
            .as_mut()
            .is_some_and(|d| std::mem::take(&mut d.pending));
        let stage = if pending { Stage::Second } else { Stage::Full };
        self.update_parts();
        self.update_deformers(stage);
        self.update_meshes();
        self.apply_glues();
        self.sort_render_order();
        self.detect_changes();
        self.update_count += 1;
    }

    /// 2 段階の評価の 1 段目: 今のパラメータ値でデフォーマだけを評価する
    /// (`docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`)。
    ///
    /// 動的物理の加算量は加えない。この後、デフォーマのワールド変換 ([`ModelInstance::deformer_world`]
    /// など) は 1 段目の結果を返す。物理がそれを読んで加算量 ([`ModelInstance::dynamic_offsets_mut`])
    /// やパラメータを書き換えた後、[`ModelInstance::update`] を呼ぶと 2 段目として、加算量を受ける
    /// デフォーマ・パラメータが変わったデフォーマとその子孫だけを評価し直してから、残りを計算する。
    ///
    /// 出力 (`drawables` など) と `update_count` は変わらない。最初の呼び出しで 2 段階の評価の
    /// バッファを確保する。
    pub fn update_first_stage(&mut self) {
        let dynamic = self
            .dynamic
            .get_or_insert_with(|| Box::new(Dynamic::new(&self.compiled.model)));
        dynamic.snapshot.copy_from_slice(self.parameters.as_slice());
        dynamic.pending = true;
        self.update_deformers(Stage::First);
    }

    /// 動的物理の加算量。最初の呼び出しでバッファを確保する (動的物理を使わないモデルでは
    /// 呼ばないこと。呼ばなければ評価の費用は変わらない)。
    pub fn dynamic_offsets_mut(&mut self) -> &mut DynamicOffsets {
        &mut self
            .dynamic
            .get_or_insert_with(|| Box::new(Dynamic::new(&self.compiled.model)))
            .offsets
    }

    /// 動的物理の加算量。[`ModelInstance::dynamic_offsets_mut`] も [`ModelInstance::update_first_stage`]
    /// も呼んでいなければ `None`。
    pub fn dynamic_offsets(&self) -> Option<&DynamicOffsets> {
        self.dynamic.as_ref().map(|d| &d.offsets)
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
            let (_, weights) = self.scratch.compute(&part.bindings, values);
            let own =
                keyform::blend_scalar(&compiled.model.parts[p].keyforms, weights, |k| k.opacity);
            let (value, weight) = self.part_opacity_overrides[p];
            let own = own + (value - own) * weight;
            let parent = part.parent.map_or(1.0, |i| self.part_opacity[i as usize]);
            self.part_opacity[p] = own * parent;
        }
    }

    /// デフォーマをトポロジカル順に評価し、ワールド変換と不透明度を作る。
    fn update_deformers(&mut self, stage: Stage) {
        let compiled = &*self.compiled;
        let model = &compiled.model;
        let values = self.parameters.as_slice();
        // 加算量は 1 段目では加えない。
        let mut dynamic = self.dynamic.as_deref_mut();
        if stage == Stage::First {
            dynamic = None;
        }
        for &d in &compiled.deformer_order {
            let d = d as usize;
            let deformer = &compiled.deformers[d];
            if stage == Stage::Second
                && let Some(dynamic) = dynamic.as_deref_mut()
            {
                let changed = dynamic.offsets.targets[d]
                    || deformer.parent.is_some_and(|p| dynamic.dirty[p as usize])
                    || deformer
                        .bindings
                        .iter()
                        .chain(deformer.blend_shapes.iter().flat_map(|s| &s.bindings))
                        .any(|b| {
                            let i = b.param.0 as usize;
                            values[i].to_bits() != dynamic.snapshot[i].to_bits()
                        });
                dynamic.dirty[d] = changed;
                if !changed {
                    continue;
                }
            }
            let offsets = dynamic.as_deref().map(|d| &d.offsets);
            // 形 (制御点・原点・角度・拡大率) は `weights`、それ以外は `linear` の重みで補間する。
            let (weights, linear) = self.scratch.compute(&deformer.bindings, values);
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
                    let mut opacity = keyform::blend_scalar(&warp.keyforms, linear, |k| k.opacity);
                    let shapes = warp
                        .keyforms
                        .blend_shapes
                        .iter()
                        .zip(&deformer.blend_shapes);
                    for (grid, shape) in shapes {
                        let Some((current, current_linear)) =
                            shape.weights(values, &mut self.shape_scratch)
                        else {
                            continue;
                        };
                        let (rest, rest_linear) = (&shape.rest, &shape.rest_linear);
                        blend_shape::add_points(&mut points, grid, current, rest, |k| &k.points);
                        opacity +=
                            blend_shape::delta_scalar(grid, current_linear, rest_linear, |k| {
                                k.opacity
                            });
                    }
                    let parent = world_transform(
                        compiled,
                        &self.deformer_affine,
                        &self.warp_points,
                        deformer.parent,
                    );
                    for p in &mut points {
                        *p = parent.apply(*p);
                    }
                    if let Some(offsets) = offsets {
                        for (p, o) in points.iter_mut().zip(offsets.warp_points(d)) {
                            *p += *o;
                        }
                    }
                    self.warp_points[d] = points;
                    opacity
                }
                DeformerKind::Rotation(rotation) => {
                    let grid = &rotation.keyforms;
                    let mut local = Rotation {
                        origin: keyform::blend_vec2(grid, weights, |k| k.origin),
                        angle: keyform::blend_scalar(grid, weights, |k| k.angle),
                        scale: keyform::blend_scalar(grid, weights, |k| k.scale),
                        reflect_x: keyform::blend_bool(grid, linear, |k| k.reflect_x),
                        reflect_y: keyform::blend_bool(grid, linear, |k| k.reflect_y),
                    };
                    let mut opacity = keyform::blend_scalar(grid, linear, |k| k.opacity);
                    // 反転 (真偽値) は差分を持たない。
                    for (shape_grid, shape) in grid.blend_shapes.iter().zip(&deformer.blend_shapes)
                    {
                        let Some((current, current_linear)) =
                            shape.weights(values, &mut self.shape_scratch)
                        else {
                            continue;
                        };
                        let (g, rest) = (shape_grid, &shape.rest);
                        local.origin += blend_shape::delta_vec2(g, current, rest, |k| k.origin);
                        local.angle += blend_shape::delta_scalar(g, current, rest, |k| k.angle);
                        local.scale += blend_shape::delta_scalar(g, current, rest, |k| k.scale);
                        opacity +=
                            blend_shape::delta_scalar(g, current_linear, &shape.rest_linear, |k| {
                                k.opacity
                            });
                    }
                    if let Some(offsets) = offsets {
                        local.angle += offsets.angle(d);
                    }
                    let parent = world_transform(
                        compiled,
                        &self.deformer_affine,
                        &self.warp_points,
                        deformer.parent,
                    );
                    let mut world = parent.rotation_child(&local);
                    if let Some(offsets) = offsets {
                        world.translation += offsets.origin(d);
                    }
                    self.deformer_affine[d] = world;
                    opacity
                }
                // 未知の種類は恒等変換で、不透明度も変えない。子は親の変換をそのまま受ける
                // (`world_transform` が親をたどる)。
                DeformerKind::Unknown => 1.0,
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
            // 頂点位置は `weights`、それ以外は `linear` の重みで補間する (線形の格子では同じ列)。
            let (weights, linear) = self.scratch.compute(&cm.bindings, values);
            let grid = &mesh.keyforms;
            let out = &mut self.drawables[i];
            out.positions.fill(Vec2::ZERO);
            for &(k, w) in weights {
                for (p, q) in out.positions.iter_mut().zip(&grid.keyforms[k].positions) {
                    *p += *q * w;
                }
            }
            let mut mul = [0.0f32; 3];
            let mut scr = [0.0f32; 3];
            for &(k, w) in linear {
                let kf = &grid.keyforms[k];
                mul[0] += kf.multiply_color.r * w;
                mul[1] += kf.multiply_color.g * w;
                mul[2] += kf.multiply_color.b * w;
                scr[0] += kf.screen_color.r * w;
                scr[1] += kf.screen_color.g * w;
                scr[2] += kf.screen_color.b * w;
            }
            let mut own = keyform::blend_scalar(grid, linear, |k| k.opacity);
            let mut draw_order = keyform::blend_scalar(grid, linear, |k| k.draw_order);
            let mut mask_strength = keyform::blend_scalar(grid, linear, |k| k.mask_strength);
            for (shape_grid, shape) in grid.blend_shapes.iter().zip(&cm.blend_shapes) {
                let Some((current_shape, current)) = shape.weights(values, &mut self.shape_scratch)
                else {
                    continue;
                };
                let g = shape_grid;
                blend_shape::add_points(&mut out.positions, g, current_shape, &shape.rest, |k| {
                    &k.positions
                });
                // 形以外のフィールドは線形の重みで差分を求める。
                let rest = &shape.rest_linear;
                own += blend_shape::delta_scalar(g, current, rest, |k| k.opacity);
                draw_order += blend_shape::delta_scalar(g, current, rest, |k| k.draw_order);
                let colors: [fn(&MeshKeyform) -> f32; 6] = [
                    |k| k.multiply_color.r,
                    |k| k.multiply_color.g,
                    |k| k.multiply_color.b,
                    |k| k.screen_color.r,
                    |k| k.screen_color.g,
                    |k| k.screen_color.b,
                ];
                for (c, f) in mul.iter_mut().chain(&mut scr).zip(colors) {
                    *c += blend_shape::delta_scalar(g, current, rest, f);
                }
                mask_strength += blend_shape::delta_scalar(g, current, rest, |k| k.mask_strength);
            }
            let parent = world_transform(
                compiled,
                &self.deformer_affine,
                &self.warp_points,
                cm.parent,
            );
            for p in &mut out.positions {
                *p = parent.apply(*p);
            }
            if let Some(dynamic) = &self.dynamic {
                for (p, o) in out
                    .positions
                    .iter_mut()
                    .zip(dynamic.offsets.mesh_positions(i))
                {
                    *p += *o;
                }
            }
            let part = self.part_opacity[cm.part as usize];
            let deformer = cm.parent.map_or(1.0, |d| self.deformer_opacity[d as usize]);
            out.opacity = own * part * deformer;
            out.draw_order = draw_order;
            out.multiply_color = Rgb::new(mul[0], mul[1], mul[2]);
            out.screen_color = Rgb::new(scr[0], scr[1], scr[2]);
            out.mask_strength = mask_strength.clamp(0.0, 1.0);
            out.visible = !mesh.mask_only && out.opacity > 0.0;
        }
    }

    /// グルーを `glues` 配列の順に適用する。
    fn apply_glues(&mut self) {
        let compiled = &*self.compiled;
        let values = self.parameters.as_slice();
        for (glue, cg) in compiled.model.glues.iter().zip(&compiled.glues) {
            let (_, weights) = self.scratch.compute(&cg.bindings, values);
            let intensity = keyform::blend_scalar(&glue.keyforms, weights, |k| k.intensity);
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

    /// 直前の [`ModelInstance::update`] で評価したパーツ `part` (`Model::parts` のインデックス) の不透明度。
    /// 上書き ([`ModelInstance::set_part_opacity_override`]) を当てた自身の値に祖先の値を掛けたもので、
    /// メッシュの不透明度に掛かる値 (`docs/spec/evaluation.md` の 2.)。
    ///
    /// # Panics
    ///
    /// `part` が範囲外のとき。
    pub fn part_opacity(&self, part: usize) -> f32 {
        self.part_opacity[part]
    }

    // ---- デフォーマのワールド変換 (docs/spec/evaluation.md#デフォーマのワールド変換を読む) ----
    //
    // どれも直前の `update` の結果を読むだけで、確保も再評価もしない (`mesh_local_positions` を除く)。
    // 添字は `Model::deformers` / `Model::meshes` のもの (`CompiledModel::deformer_index` などで引く)。

    /// デフォーマ `deformer` のワールド変換 (その子の座標系 → キャンバス座標)。
    ///
    /// ワープは UV 空間、回転デフォーマは原点を (0,0) としたピクセル単位の座標系を写す。
    /// 未知の種類のデフォーマは恒等変換なので、親のワールド変換と同じになる。
    ///
    /// # Panics
    ///
    /// `deformer` が範囲外のとき。
    pub fn deformer_world(&self, deformer: usize) -> WorldTransform<'_> {
        assert!(
            deformer < self.deformer_affine.len(),
            "deformer out of range"
        );
        self.world(Some(deformer as u32))
    }

    /// デフォーマ `deformer` の親の座標系 (キーフォームの原点・制御点の座標系) → キャンバス座標。
    /// 親がなければ恒等写像。
    ///
    /// # Panics
    ///
    /// `deformer` が範囲外のとき。
    pub fn deformer_parent_world(&self, deformer: usize) -> WorldTransform<'_> {
        self.world(self.compiled.deformers[deformer].parent)
    }

    /// メッシュ `mesh` の親の座標系 (キーフォームの頂点の座標系) → キャンバス座標。
    /// 親がなければ恒等写像。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mesh_parent_world(&self, mesh: usize) -> WorldTransform<'_> {
        self.world(self.compiled.meshes[mesh].parent)
    }

    /// `deformer_world(deformer).point(local)`。
    pub fn deformer_world_point(&self, deformer: usize, local: Vec2) -> Vec2 {
        self.deformer_world(deformer).point(local)
    }

    /// `deformer_world(deformer).jacobian(local)`。
    pub fn deformer_world_jacobian(&self, deformer: usize, local: Vec2) -> Mat2 {
        self.deformer_world(deformer).jacobian(local)
    }

    /// ワープ `deformer` のキャンバス座標の制御点 (行優先、x が速い)。ワープでなければ `None`。
    pub fn warp_world_points(&self, deformer: usize) -> Option<&[Vec2]> {
        match self.compiled.model.deformers.get(deformer)?.kind {
            DeformerKind::Warp(_) => Some(&self.warp_points[deformer]),
            _ => None,
        }
    }

    /// 回転デフォーマ `deformer` のワールド変換 (`world(p) = translation + matrix2 * p`)。
    /// `matrix2` の列は、局所座標の x 軸・y 軸の単位ベクトルの像。回転デフォーマでなければ `None`。
    pub fn rotation_world_frame(&self, deformer: usize) -> Option<Affine2> {
        match self.compiled.model.deformers.get(deformer)?.kind {
            DeformerKind::Rotation(_) => Some(self.deformer_affine[deformer]),
            _ => None,
        }
    }

    /// メッシュ `mesh` のキーフォームを今のパラメータ値で補間した頂点位置 (親の座標系。
    /// 親のワールド変換とグルーを適用する前)。
    ///
    /// `update` の途中の値は保持していないので、呼ぶたびに補間し直して新しい `Vec` を返す
    /// (毎フレームの描画ではなく、エディタなどの問い合わせ用)。`update` の後にパラメータを
    /// 変えていなければ、`update` が使った値と同じになる。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mesh_local_positions(&self, mesh: usize) -> Vec<Vec2> {
        let grid = &self.compiled.model.meshes[mesh].keyforms;
        let mut weights = Weights::new();
        keyform::weights(
            &self.compiled.meshes[mesh].bindings,
            self.parameters.as_slice(),
            &mut weights,
        );
        let n = self.drawables[mesh].positions.len();
        let mut out = vec![Vec2::ZERO; n];
        for &(k, w) in &weights {
            for (p, q) in out.iter_mut().zip(&grid.keyforms[k].positions) {
                *p += *q * w;
            }
        }
        let shapes = &self.compiled.meshes[mesh].blend_shapes;
        let mut scratch = GridWeights::default();
        for (shape_grid, shape) in grid.blend_shapes.iter().zip(shapes) {
            if let Some((current, _)) = shape.weights(self.parameters.as_slice(), &mut scratch) {
                let rest = &shape.rest;
                blend_shape::add_points(&mut out, shape_grid, current, rest, |k| &k.positions);
            }
        }
        out
    }

    /// 回転デフォーマ `deformer` のキーフォームを今のパラメータ値で補間した値 (ブレンドシェイプの
    /// 差分を含み、動的物理の加算量を加える前)。回転デフォーマでなければ `None`。
    ///
    /// [`ModelInstance::mesh_local_positions`] と同じく問い合わせ用で、呼ぶたびに補間し直す。
    pub fn rotation_local_keyform(&self, deformer: usize) -> Option<RotationKeyform> {
        let DeformerKind::Rotation(rotation) = &self.compiled.model.deformers.get(deformer)?.kind
        else {
            return None;
        };
        let grid = &rotation.keyforms;
        let values = self.parameters.as_slice();
        let mut scratch = GridWeights::default();
        let (weights, linear) =
            scratch.compute(&self.compiled.deformers[deformer].bindings, values);
        let mut out = RotationKeyform {
            origin: keyform::blend_vec2(grid, weights, |k| k.origin),
            angle: keyform::blend_scalar(grid, weights, |k| k.angle),
            scale: keyform::blend_scalar(grid, weights, |k| k.scale),
            reflect_x: keyform::blend_bool(grid, linear, |k| k.reflect_x),
            reflect_y: keyform::blend_bool(grid, linear, |k| k.reflect_y),
            opacity: keyform::blend_scalar(grid, linear, |k| k.opacity),
        };
        let shapes = &self.compiled.deformers[deformer].blend_shapes;
        let mut shape_scratch = GridWeights::default();
        for (g, shape) in grid.blend_shapes.iter().zip(shapes) {
            if let Some((current, current_linear)) = shape.weights(values, &mut shape_scratch) {
                let rest = &shape.rest;
                out.origin += blend_shape::delta_vec2(g, current, rest, |k| k.origin);
                out.angle += blend_shape::delta_scalar(g, current, rest, |k| k.angle);
                out.scale += blend_shape::delta_scalar(g, current, rest, |k| k.scale);
                let rest = &shape.rest_linear;
                out.opacity += blend_shape::delta_scalar(g, current_linear, rest, |k| k.opacity);
            }
        }
        Some(out)
    }

    fn world(&self, parent: Option<u32>) -> WorldTransform<'_> {
        WorldTransform(world_transform(
            &self.compiled,
            &self.deformer_affine,
            &self.warp_points,
            parent,
        ))
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

/// デフォーマ `parent` のワールド変換 (`None` なら恒等写像)。`parent` は評価済みであること。
///
/// 未知の種類のデフォーマは恒等変換なので、そのワールド変換は親のワールド変換と同じ。
fn world_transform<'a>(
    compiled: &CompiledModel,
    affine: &[Affine2],
    warp_points: &'a [Vec<Vec2>],
    parent: Option<u32>,
) -> Transform<'a> {
    let mut parent = parent;
    while let Some(d) = parent {
        let d = d as usize;
        match &compiled.model.deformers[d].kind {
            DeformerKind::Warp(w) => {
                return Transform::Warp(Warp::new(
                    &warp_points[d],
                    usize::from(w.cols),
                    usize::from(w.rows),
                ));
            }
            DeformerKind::Rotation(_) => return Transform::Affine(affine[d]),
            DeformerKind::Unknown => parent = compiled.deformers[d].parent,
        }
    }
    Transform::Identity
}

#[cfg(test)]
mod tests {
    use ugoe_model::{Model, fixtures};

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

    // NaN を書き込んでも評価は止まらない (書き込みは無視される)。
    #[test]
    fn nan_parameter_does_not_panic() {
        let mut instance = instance(fixtures::single_quad());
        set(&mut instance, "X", 0.5);
        instance.update();
        let before = instance.drawables().to_vec();
        set(&mut instance, "X", f32::NAN);
        instance.update();
        assert_eq!(instance.drawables(), &before[..]);
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
            id(&instance.scratch.shape),
            id(&instance.scratch.linear),
            id(&instance.shape_scratch.shape),
            id(&instance.shape_scratch.linear),
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
        // 曲線補間・ブレンドシェイプ (重みの作業領域を 2 組使う) を含むすべてのフィクスチャ。
        for model in all_fixtures() {
            let mut instance = instance(model);
            let before = buffers(&instance);
            for t in [0.0, 0.3, 0.5, 1.0] {
                set_all(&mut instance, t);
                assert_eq!(buffers(&instance), before);
            }
        }
    }

    // 動的物理の 2 段階の評価も、最初の 1 段目と加算量の書き込みで確保した後は確保しない。
    #[test]
    fn two_stage_update_does_not_reallocate_buffers() {
        fn dynamic_buffers(instance: &ModelInstance) -> Vec<(usize, usize)> {
            let d = instance.dynamic.as_deref().unwrap();
            let mut out = buffers(instance);
            out.extend([d.snapshot.as_ptr() as usize, d.dirty.as_ptr() as usize].map(|p| (p, 0)));
            out
        }
        let mut instance = instance(fixtures::dynamic_hair());
        let hair1 = instance.compiled().deformer_index("Hair1").unwrap();
        let cloth = instance.compiled().deformer_index("Cloth").unwrap();
        let ribbon = instance.compiled().mesh_index("Ribbon").unwrap();
        let write_offsets = |instance: &mut ModelInstance, t: f32| {
            let offsets = instance.dynamic_offsets_mut();
            offsets.clear();
            offsets.add_angle(hair1, 10.0 * t);
            offsets.warp_points_mut(cloth).unwrap()[0] = Vec2::splat(t);
            offsets.mesh_positions_mut(ribbon).unwrap()[0] = Vec2::splat(t);
        };
        instance.update_first_stage();
        write_offsets(&mut instance, 0.0);
        instance.update();
        let before = dynamic_buffers(&instance);
        for t in [0.0, 0.3, 0.5, 1.0] {
            instance.update_first_stage();
            write_offsets(&mut instance, t);
            set_all(&mut instance, t);
            assert_eq!(dynamic_buffers(&instance), before);
        }
    }

    // ---- デフォーマのワールド変換 (docs/spec/evaluation.md#デフォーマのワールド変換を読む) ----

    fn all_fixtures() -> Vec<Model> {
        vec![
            fixtures::single_quad(),
            fixtures::pose_and_effects(),
            fixtures::warp_grid(),
            fixtures::rotation_chain(),
            fixtures::rotation_under_warp(),
            fixtures::part_opacity_hierarchy(),
            fixtures::glued_quads(),
            fixtures::nested_masks(),
            fixtures::mixed_masks(),
            fixtures::intersection_masks(),
            fixtures::keyform_inference(),
            fixtures::blend_shapes(),
            fixtures::smooth_arc(),
        ]
    }

    /// すべてのパラメータを値域の `t` (0..1) の位置にする。
    fn set_all(instance: &mut ModelInstance, t: f32) {
        let ranges: Vec<_> = instance
            .compiled()
            .model()
            .parameters
            .iter()
            .map(|p| (p.min, p.max))
            .collect();
        for (i, (min, max)) in ranges.into_iter().enumerate() {
            instance
                .parameters_mut()
                .set(crate::ParamIndex(i as u32), min + (max - min) * t);
        }
        instance.update();
    }

    // 親のワールド変換で局所座標の頂点を写すと、update の出力とビット単位で一致する
    // (グルーで動かされるメッシュを除く)。
    #[test]
    fn mesh_parent_world_reproduces_update_output() {
        for model in all_fixtures() {
            let glued: Vec<String> = model
                .glues
                .iter()
                .flat_map(|g| [g.mesh_a.to_string(), g.mesh_b.to_string()])
                .collect();
            let mut instance = instance(model);
            for t in [0.0, 0.25, 0.5, 0.8, 1.0] {
                set_all(&mut instance, t);
                let model = instance.compiled().model();
                for (i, mesh) in model.meshes.iter().enumerate() {
                    if glued.contains(&mesh.id.to_string()) {
                        continue;
                    }
                    let world = instance.mesh_parent_world(i);
                    assert_eq!(world.is_identity(), mesh.parent.is_none());
                    let mapped: Vec<Vec2> = instance
                        .mesh_local_positions(i)
                        .into_iter()
                        .map(|p| world.point(p))
                        .collect();
                    assert_eq!(mapped, instance.drawables()[i].positions, "{}", mesh.id);
                    if let Some(parent) = &mesh.parent {
                        let d = instance.compiled().deformer_index(parent.as_str()).unwrap();
                        for &p in &instance.mesh_local_positions(i) {
                            assert_eq!(instance.deformer_world_point(d, p), world.point(p));
                        }
                    }
                }
            }
        }
    }

    // ヤコビ行列は中心差分と一致する (ワープはセルの境目を避けた点、格子の外を含む)。
    #[test]
    fn deformer_world_jacobian_matches_finite_difference() {
        let warp_samples = [
            Vec2::new(0.23, 0.41),
            Vec2::new(0.77, 0.12),
            Vec2::new(0.61, 0.93),
            Vec2::new(-0.3, 0.6),
            Vec2::new(1.4, 1.2),
            Vec2::new(0.4, -0.7),
        ];
        let rotation_samples = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, -5.0),
            Vec2::new(-37.0, 21.0),
        ];
        let mut checked = 0;
        for model in all_fixtures() {
            let mut instance = instance(model);
            for t in [0.0, 0.3, 1.0] {
                set_all(&mut instance, t);
                for (d, deformer) in instance.compiled().model().deformers.iter().enumerate() {
                    let (samples, h) = match deformer.kind {
                        DeformerKind::Warp(_) => (&warp_samples[..], 1e-3),
                        _ => (&rotation_samples[..], 0.5),
                    };
                    let world = instance.deformer_world(d);
                    for &p in samples {
                        let j = world.jacobian(p);
                        let dx = (world.point(p + Vec2::new(h, 0.0))
                            - world.point(p - Vec2::new(h, 0.0)))
                            / (2.0 * h);
                        let dy = (world.point(p + Vec2::new(0.0, h))
                            - world.point(p - Vec2::new(0.0, h)))
                            / (2.0 * h);
                        let tol = 1e-2 * j.x_axis.length().max(j.y_axis.length()).max(1.0);
                        assert!(
                            j.x_axis.abs_diff_eq(dx, tol),
                            "{}: {} vs {dx}",
                            deformer.id,
                            j.x_axis
                        );
                        assert!(
                            j.y_axis.abs_diff_eq(dy, tol),
                            "{}: {} vs {dy}",
                            deformer.id,
                            j.y_axis
                        );
                        assert_eq!(j, instance.deformer_world_jacobian(d, p));
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 0);
    }

    // 制御点・回転の原点と軸は、ワールド変換で格子点・単位ベクトルを写したものと同じ。
    #[test]
    fn warp_points_and_rotation_frames_match_world_transform() {
        let mut seen = (0, 0);
        for model in all_fixtures() {
            let mut instance = instance(model);
            set_all(&mut instance, 0.7);
            for (d, deformer) in instance.compiled().model().deformers.iter().enumerate() {
                let world = instance.deformer_world(d);
                match &deformer.kind {
                    DeformerKind::Warp(w) => {
                        let points = instance.warp_world_points(d).unwrap();
                        let (cols, rows) = (usize::from(w.cols), usize::from(w.rows));
                        assert_eq!(points.len(), (cols + 1) * (rows + 1));
                        for y in 0..=rows {
                            for x in 0..=cols {
                                let uv = Vec2::new(x as f32 / cols as f32, y as f32 / rows as f32);
                                let p = points[y * (cols + 1) + x];
                                assert!(world.point(uv).abs_diff_eq(p, 1e-3), "{}", deformer.id);
                            }
                        }
                        assert!(instance.rotation_world_frame(d).is_none());
                        seen.0 += 1;
                    }
                    DeformerKind::Rotation(_) => {
                        let frame = instance.rotation_world_frame(d).unwrap();
                        assert_eq!(world.point(Vec2::ZERO), frame.translation);
                        assert_eq!(world.jacobian(Vec2::ZERO), frame.matrix2);
                        assert!(instance.warp_world_points(d).is_none());
                        seen.1 += 1;
                    }
                    DeformerKind::Unknown => {}
                }
            }
        }
        assert!(seen.0 > 0 && seen.1 > 0);
    }

    // 回転デフォーマの軸はパラメータに従って回る (rotation_chain: Angle で Arm が時計回り)。
    #[test]
    fn rotation_world_frame_follows_parameters() {
        let mut instance = instance(fixtures::rotation_chain());
        let arm = instance.compiled().deformer_index("Arm").unwrap();
        let hand = instance.compiled().deformer_index("Hand").unwrap();
        let frame = instance.rotation_world_frame(hand).unwrap();
        assert!(frame.translation.abs_diff_eq(Vec2::new(192.0, 128.0), 1e-4));
        assert!(
            frame
                .matrix2
                .abs_diff_eq(Mat2::from_diagonal(Vec2::splat(0.5)), 1e-6)
        );
        set(&mut instance, "Angle", 30.0);
        instance.update();
        let (s, c) = 30f32.to_radians().sin_cos();
        let frame = instance.rotation_world_frame(arm).unwrap();
        assert!(frame.translation.abs_diff_eq(Vec2::new(128.0, 128.0), 1e-4));
        assert!(
            frame
                .matrix2
                .abs_diff_eq(Mat2::from_cols(Vec2::new(c, s), Vec2::new(-s, c)), 1e-5)
        );
        assert_eq!(instance.compiled().deformer_index("Nope"), None);
        assert!(instance.warp_world_points(99).is_none());
    }

    // 作った写像は、ランタイムが評価したものと同じ結果になる。
    #[test]
    fn constructed_transforms_match_evaluated_ones() {
        let mut instance = instance(fixtures::rotation_under_warp());
        set_all(&mut instance, 0.7);
        let model = instance.compiled().model().clone();
        for (d, deformer) in model.deformers.iter().enumerate() {
            let world = instance.deformer_world(d);
            let built = match &deformer.kind {
                DeformerKind::Warp(w) => WorldTransform::from_warp_points(
                    instance.warp_world_points(d).unwrap(),
                    usize::from(w.cols),
                    usize::from(w.rows),
                )
                .unwrap(),
                _ => WorldTransform::from_affine(instance.rotation_world_frame(d).unwrap()),
            };
            for p in [Vec2::new(0.3, 0.7), Vec2::new(-4.0, 9.0)] {
                assert_eq!(built.point(p), world.point(p));
                assert_eq!(built.jacobian(p), world.jacobian(p));
            }
            // 子の回転デフォーマのワールド変換を親の写像から作り直せる。
            for (c, child) in model.deformers.iter().enumerate() {
                if child.parent.as_ref() != Some(&deformer.id) {
                    continue;
                }
                if let DeformerKind::Rotation(r) = &child.kind
                    && r.keyforms.bindings.is_empty()
                {
                    let frame = built.rotation_child(&r.keyforms.keyforms[0]);
                    assert_eq!(Some(frame), instance.rotation_world_frame(c));
                }
            }
        }
        assert!(WorldTransform::from_warp_points(&[Vec2::ZERO; 3], 1, 1).is_none());
        assert!(WorldTransform::from_warp_points(&[], 0, 0).is_none());
        assert!(WorldTransform::IDENTITY.is_identity());
    }

    // 親のないデフォーマの親の座標系は恒等写像。
    #[test]
    fn root_parent_world_is_identity() {
        let instance = instance(fixtures::rotation_chain());
        let arm = instance.compiled().deformer_index("Arm").unwrap();
        let hand = instance.compiled().deformer_index("Hand").unwrap();
        let root = instance.deformer_parent_world(arm);
        assert!(root.is_identity());
        assert_eq!(root.point(Vec2::new(3.0, 4.0)), Vec2::new(3.0, 4.0));
        assert_eq!(root.jacobian(Vec2::new(3.0, 4.0)), Mat2::IDENTITY);
        assert!(!instance.deformer_parent_world(hand).is_identity());
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

    /// パーツの不透明度の上書きは `lerp(own, value, weight)` で、子へ伝播する。
    #[test]
    fn part_opacity_override_replaces_own_opacity() {
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        let child = instance.compiled().part_index("Child").unwrap();
        let grandchild = instance.compiled().part_index("Grandchild").unwrap();
        assert_eq!(instance.compiled().part_index("Missing"), None);

        instance.set_part_opacity_override(child, 1.0, 1.0);
        instance.update();
        assert!((instance.part_opacity[child] - 1.0).abs() < 1e-6);
        assert!((instance.part_opacity[grandchild] - 0.8).abs() < 1e-6);

        // 重み 0.5 なら own = lerp(0.5, 0.0, 0.5) = 0.25。
        instance.set_part_opacity_override(child, 0.0, 0.5);
        instance.update();
        assert!((instance.part_opacity[child] - 0.25).abs() < 1e-6);
        assert_eq!(instance.part_opacity_override(child), (0.0, 0.5));

        instance.clear_part_opacity_overrides();
        instance.update();
        assert!((instance.part_opacity[child] - 0.5).abs() < 1e-6);
    }

    /// `part_opacity` は祖先を掛けた評価後の値で、`update` まで変わらない。
    #[test]
    fn part_opacity_reads_the_evaluated_value() {
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        let [root, child, grandchild] =
            ["Root", "Child", "Grandchild"].map(|id| instance.compiled().part_index(id).unwrap());
        assert!((instance.part_opacity(root) - 1.0).abs() < 1e-6);
        assert!((instance.part_opacity(child) - 0.5).abs() < 1e-6);
        assert!((instance.part_opacity(grandchild) - 0.4).abs() < 1e-6);

        set(&mut instance, "Fade", 1.0);
        instance.set_part_opacity_override(child, 0.0, 0.5);
        // update の前は前回の値のまま。
        assert!((instance.part_opacity(child) - 0.5).abs() < 1e-6);
        instance.update();
        // Root = 0.5、Child = lerp(0.5, 0, 0.5) × 0.5 = 0.125、Grandchild = 0.8 × 0.125。
        assert!((instance.part_opacity(root) - 0.5).abs() < 1e-6);
        assert!((instance.part_opacity(child) - 0.125).abs() < 1e-6);
        assert!((instance.part_opacity(grandchild) - 0.1).abs() < 1e-6);
        // メッシュ A (Grandchild、デフォーマ 0.25) の不透明度に掛かる。
        let a = mesh_index(&instance, "A");
        assert!((instance.drawables()[a].opacity - 0.1 * 0.25).abs() < 1e-6);
    }

    /// 上書きを変えると、影響を受けるメッシュ (パーツ `Child` とその子孫) だけが変更ありになる。
    #[test]
    fn part_opacity_override_marks_affected_drawables() {
        let mut instance = instance(fixtures::part_opacity_hierarchy());
        instance.update();
        instance.update();
        assert_no_changes(&instance);

        let child = instance.compiled().part_index("Child").unwrap();
        instance.set_part_opacity_override(child, 1.0, 1.0);
        instance.update();
        let a = mesh_index(&instance, "A");
        let c = mesh_index(&instance, "C");
        for (i, changes) in instance.changes().iter().enumerate() {
            if i == a || i == c {
                assert_eq!(*changes, DrawableChanges::OPACITY, "mesh {i}");
            } else {
                assert!(changes.is_empty(), "mesh {i}");
            }
        }

        // 同じ上書きのままなら変更なし。
        instance.update();
        assert_no_changes(&instance);
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
