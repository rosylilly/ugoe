//! ブレンドシェイプ格子の差分 (`docs/spec/evaluation.md#6-ブレンドシェイプ`)。
//!
//! 差分は `interp(格子, 現在値) - interp(格子, 既定値)`。既定値での重みはコンパイル時に求めておき、
//! 現在の重みがそれと同じ格子は加算しない。

use alloc::vec::Vec;
use ugoe_core::Vec2;
use ugoe_model::KeyformGrid;

use crate::keyform::{self, GridWeights, ResolvedBinding, Weights};

/// ロード時に解決したブレンドシェイプ格子。
#[derive(Debug)]
pub(crate) struct CompiledBlendShape {
    pub(crate) bindings: Vec<ResolvedBinding>,
    /// パラメータがすべて既定値のときの重み (形のフィールド用)。
    pub(crate) rest: Weights,
    /// パラメータがすべて既定値のときの線形の重み (形以外のフィールド用。線形の格子では `rest` と同じ)。
    pub(crate) rest_linear: Weights,
}

impl CompiledBlendShape {
    /// `defaults` はパラメータの既定値 (`ParameterValues` の初期値)。
    pub(crate) fn new(bindings: Vec<ResolvedBinding>, defaults: &[f32]) -> Self {
        let mut rest = Weights::new();
        keyform::weights(&bindings, defaults, &mut rest);
        let mut rest_linear = Weights::new();
        keyform::linear_weights(&bindings, defaults, &mut rest_linear);
        Self {
            bindings,
            rest,
            rest_linear,
        }
    }

    /// 現在の値 `values` での重み `(形の重み, 形以外の重み)` を `out` に求める。
    /// どちらも既定値での重みと同じ (差分を加えない) なら `None`。
    pub(crate) fn weights<'a>(
        &self,
        values: &[f32],
        out: &'a mut GridWeights,
    ) -> Option<(&'a Weights, &'a Weights)> {
        let (shape, linear) = out.compute(&self.bindings, values);
        (*shape != self.rest || *linear != self.rest_linear).then_some((shape, linear))
    }
}

/// スカラーの差分 `interp(現在) - interp(既定値)`。
pub(crate) fn delta_scalar<T>(
    grid: &KeyformGrid<T>,
    current: &Weights,
    rest: &Weights,
    f: impl Fn(&T) -> f32,
) -> f32 {
    keyform::blend_scalar(grid, current, &f) - keyform::blend_scalar(grid, rest, &f)
}

/// ベクトルの差分 `interp(現在) - interp(既定値)`。
pub(crate) fn delta_vec2<T>(
    grid: &KeyformGrid<T>,
    current: &Weights,
    rest: &Weights,
    f: impl Fn(&T) -> Vec2,
) -> Vec2 {
    keyform::blend_vec2(grid, current, &f) - keyform::blend_vec2(grid, rest, &f)
}

/// 点列 (頂点・制御点) の各点に差分を加える。各点の補間の和は重みの順序で取る。
pub(crate) fn add_points<T>(
    out: &mut [Vec2],
    grid: &KeyformGrid<T>,
    current: &Weights,
    rest: &Weights,
    f: impl Fn(&T) -> &[Vec2],
) {
    let interp = |weights: &Weights, i: usize| {
        let mut sum = Vec2::ZERO;
        for &(k, w) in weights {
            // 点の数は検証で揃えてある。足りないキーフォームの点は 0 とみなす。
            if let Some(&q) = grid.keyforms.get(k).and_then(|k| f(k).get(i)) {
                sum += q * w;
            }
        }
        sum
    };
    for (i, p) in out.iter_mut().enumerate() {
        *p += interp(current, i) - interp(rest, i);
    }
}

#[cfg(test)]
mod tests {
    use ugoe_core::Rgb;
    use ugoe_core::math;
    use ugoe_model::{DeformerKind, Model, fixtures};

    use crate::{CompiledModel, DrawableChanges, ModelInstance};

    fn instance(model: Model) -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(model).unwrap())
    }

    fn set(instance: &mut ModelInstance, id: &str, value: f32) {
        let i = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(i, value);
    }

    /// ブレンドシェイプ格子をすべて取り除いたモデル。
    fn without_blend_shapes(mut model: Model) -> Model {
        for mesh in &mut model.meshes {
            mesh.keyforms.blend_shapes.clear();
        }
        for deformer in &mut model.deformers {
            match &mut deformer.kind {
                DeformerKind::Warp(w) => w.keyforms.blend_shapes.clear(),
                DeformerKind::Rotation(r) => r.keyforms.blend_shapes.clear(),
                DeformerKind::Unknown => {}
            }
        }
        model
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
    }

    // evaluation.md 6.: 既定値では差分が 0 (ブレンドシェイプ格子がないのとビット単位で同じ)
    #[test]
    fn default_values_leave_the_result_unchanged() {
        let mut with = instance(fixtures::blend_shapes());
        let mut without = instance(without_blend_shapes(fixtures::blend_shapes()));
        assert_eq!(with.drawables(), without.drawables());
        // 通常のパラメータを動かしても、ブレンドシェイプのパラメータが既定値なら変わらない。
        for x in [-1.0, -0.3, 1.0] {
            set(&mut with, "X", x);
            set(&mut without, "X", x);
            with.update();
            without.update();
            assert_eq!(with.drawables(), without.drawables());
        }
    }

    // evaluation.md 6.: メッシュのすべての数値フィールドに差分を加える
    #[test]
    fn mesh_adds_delta_to_every_numeric_field() {
        let mut inst = instance(fixtures::blend_shapes());
        set(&mut inst, "X", 1.0);
        set(&mut inst, "Smile", 1.0);
        inst.update();
        let mouth = &inst.drawables()[0];
        // 通常の格子で +64 平行移動し、Smile で頂点 0 だけ (-8, -16)。
        assert_eq!(mouth.positions[0], ugoe_core::Vec2::new(120.0, 48.0));
        assert_eq!(mouth.positions[2], ugoe_core::Vec2::new(256.0, 192.0));
        assert_eq!(mouth.opacity, 0.5);
        assert_eq!(mouth.draw_order, 600.0);
        assert_eq!(mouth.multiply_color, Rgb::new(1.0, 0.5, 0.5));
        assert_eq!(mouth.screen_color, Rgb::new(0.0, 0.0, 0.25));
        assert_eq!(mouth.mask_strength, 0.5);

        set(&mut inst, "Smile", 0.5);
        inst.update();
        let mouth = &inst.drawables()[0];
        assert_eq!(mouth.positions[0], ugoe_core::Vec2::new(124.0, 56.0));
        assert_eq!(mouth.opacity, 0.75);
        assert_eq!(mouth.draw_order, 550.0);
    }

    // evaluation.md 6.: 差分は既定値での値からの差 (既定値がキーの間にある場合)
    #[test]
    fn delta_is_relative_to_the_default_value() {
        let mut inst = instance(fixtures::blend_shapes());
        set(&mut inst, "Wide", 1.0);
        inst.update();
        let mouth = &inst.drawables()[0];
        assert_eq!((mouth.positions[0].x, mouth.positions[1].x), (48.0, 208.0));
        set(&mut inst, "Wide", 0.0);
        inst.update();
        let mouth = &inst.drawables()[0];
        assert_eq!((mouth.positions[0].x, mouth.positions[1].x), (80.0, 176.0));
    }

    // evaluation.md 6.: ワープの制御点と不透明度
    #[test]
    fn warp_adds_delta_to_control_points_and_opacity() {
        let mut inst = instance(fixtures::blend_shapes());
        set(&mut inst, "Smile", 1.0);
        inst.update();
        let face = inst.warp_world_points(0).unwrap();
        assert_eq!(face[2], ugoe_core::Vec2::new(64.0, 224.0));
        assert_eq!(face[1], ugoe_core::Vec2::new(192.0, 64.0));
        // Cheek の UV (0.25, 0.75) は、下の行が 32 下がった格子で y = 64 + 160 * 0.75。
        let cheek = &inst.drawables()[1];
        assert_eq!(cheek.positions[3], ugoe_core::Vec2::new(96.0, 184.0));
        assert_eq!(cheek.opacity, 0.5);
    }

    // evaluation.md 6.: 回転デフォーマの原点・角度・拡大率・不透明度
    #[test]
    fn rotation_adds_delta_to_origin_angle_scale_and_opacity() {
        let mut inst = instance(fixtures::blend_shapes());
        set(&mut inst, "Wide", 1.0);
        inst.update();
        let frame = inst.rotation_world_frame(1).unwrap();
        assert_eq!(frame.translation, ugoe_core::Vec2::new(136.0, 128.0));
        let (sin, cos) = math::sin_cos(10f32.to_radians());
        let x_axis = frame.matrix2.x_axis;
        assert_close(x_axis.x, 1.5 * cos);
        assert_close(x_axis.y, 1.5 * sin);
        // 不透明度の既定値での値は 0.75 なので、Wide = 1 (0.5) では 1 - 0.25。
        assert_eq!(inst.drawables()[2].opacity, 0.75);
    }

    // evaluation.md 6.: 複数の格子は宣言順に 1 つずつ加える
    #[test]
    fn grids_are_added_in_declaration_order() {
        // 1 + 2^24 は 2^24 に丸められるので、+2^24 を先に加えると 0、-2^24 を先に加えると 1 になる。
        let big = 16_777_216.0;
        let mut model = fixtures::blend_shapes();
        let mouth = &mut model.meshes[0].keyforms;
        for kf in mouth.keyforms.iter_mut() {
            kf.draw_order = 1.0;
        }
        let smile = &mut mouth.blend_shapes[0].keyforms;
        (smile[0].draw_order, smile[1].draw_order) = (0.0, big);
        // Wide の既定値 0.5 での値は 2^24 なので、Wide = 1 の差分は -2^24。
        let wide = &mut mouth.blend_shapes[1].keyforms;
        (wide[0].draw_order, wide[1].draw_order) = (2.0 * big, 0.0);

        let draw_order = |model: Model| {
            let mut inst = instance(model);
            set(&mut inst, "Smile", 1.0);
            set(&mut inst, "Wide", 1.0);
            inst.update();
            inst.drawables()[0].draw_order
        };
        assert_eq!(draw_order(model.clone()), 0.0);
        model.meshes[0].keyforms.blend_shapes.swap(0, 1);
        assert_eq!(draw_order(model), 1.0);
    }

    // evaluation.md 変更フラグ: ブレンドシェイプによる変化も報告する
    #[test]
    fn changes_report_blend_shape_updates() {
        let mut inst = instance(fixtures::blend_shapes());
        inst.update();
        let rest = inst.drawables().to_vec();
        set(&mut inst, "Smile", 1.0);
        inst.update();
        let mouth = DrawableChanges::POSITIONS
            | DrawableChanges::OPACITY
            | DrawableChanges::DRAW_ORDER
            | DrawableChanges::MULTIPLY_COLOR
            | DrawableChanges::SCREEN_COLOR
            | DrawableChanges::MASK_STRENGTH;
        assert_eq!(inst.changes()[0], mouth);
        assert_eq!(
            inst.changes()[1],
            DrawableChanges::POSITIONS | DrawableChanges::OPACITY
        );
        assert_eq!(inst.changes()[2], DrawableChanges::NONE);

        // 既定値に戻すと元の出力にビット単位で戻る。
        set(&mut inst, "Smile", 0.0);
        inst.update();
        assert_eq!(inst.drawables(), &rest[..]);
        assert_eq!(inst.changes()[0], mouth);
        inst.update();
        assert!(inst.changes().iter().all(|c| c.is_empty()));
    }

    // evaluation.md デフォーマのワールド変換を読む: mesh_local_positions もブレンドシェイプを含む
    #[test]
    fn mesh_local_positions_include_blend_shapes() {
        let mut inst = instance(fixtures::blend_shapes());
        set(&mut inst, "Smile", 0.25);
        set(&mut inst, "Wide", 0.75);
        inst.update();
        // Mouth は親を持たないので、局所座標がそのままキャンバス座標。
        assert_eq!(
            inst.mesh_local_positions(0).as_ref(),
            Some(&inst.drawables()[0].positions)
        );
    }
}
