//! 曲線補間 (`docs/spec/evaluation.md#曲線補間`)。
//! 入力は `ugoe_model::fixtures::smooth_arc`。すべての格子を線形にしたモデルと比べる。

use ugoe_core::Vec2;
use ugoe_model::{DeformerKind, KeyformGrid, KeyformInterpolation, Model, fixtures};
use ugoe_runtime::{CompiledModel, DrawableChanges, ModelInstance};

const CENTER: Vec2 = Vec2::new(128.0, 128.0);

fn instance(model: Model) -> ModelInstance {
    ModelInstance::new(CompiledModel::compile(model).expect("compile"))
}

fn set(instance: &mut ModelInstance, id: &str, value: f32) {
    let index = instance.parameters().index_of(id).expect("parameter");
    instance.parameters_mut().set(index, value);
    instance.update();
}

/// すべての格子 (ブレンドシェイプ格子を含む) の補間の種類を `interpolation` にしたモデル。
fn with_interpolation(mut model: Model, interpolation: KeyformInterpolation) -> Model {
    fn apply<T>(grid: &mut KeyformGrid<T>, interpolation: KeyformInterpolation) {
        grid.interpolation = interpolation;
        for shape in &mut grid.blend_shapes {
            shape.interpolation = interpolation;
        }
    }
    for mesh in &mut model.meshes {
        apply(&mut mesh.keyforms, interpolation);
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(w) => apply(&mut w.keyforms, interpolation),
            DeformerKind::Rotation(r) => apply(&mut r.keyforms, interpolation),
            DeformerKind::Unknown => {}
        }
    }
    model
}

fn pair() -> (ModelInstance, ModelInstance) {
    let smooth = fixtures::smooth_arc();
    let linear = with_interpolation(smooth.clone(), KeyformInterpolation::Linear);
    (instance(smooth), instance(linear))
}

/// メッシュ `Cheek` の中心の、円弧からの距離 (半径の誤差)。
fn cheek_error(instance: &ModelInstance) -> f32 {
    let p = &instance.drawables()[0].positions;
    let center = (p[0] + p[2]) * 0.5;
    ((center - CENTER).length() - fixtures::SMOOTH_ARC_RADIUS).abs()
}

// evaluation.md 曲線補間: キーの上では線形と同じ値 (ビット単位)
#[test]
fn keys_match_linear_exactly() {
    let (mut smooth, mut linear) = pair();
    for v in [-30.0, 0.0, 30.0] {
        set(&mut smooth, "AngleX", v);
        set(&mut linear, "AngleX", v);
        assert_eq!(smooth.drawables(), linear.drawables(), "AngleX = {v}");
        assert_eq!(smooth.warp_world_points(0), linear.warp_world_points(0));
        assert_eq!(
            smooth.rotation_world_frame(1),
            linear.rotation_world_frame(1)
        );
    }
}

// evaluation.md 曲線補間: 円弧上の 3 つのキーの間では、直線より円弧に近い
#[test]
fn intermediate_values_follow_the_arc() {
    let (mut smooth, mut linear) = pair();
    for v in [-25.0, -15.0, -5.0, 5.0, 15.0, 25.0] {
        set(&mut smooth, "AngleX", v);
        set(&mut linear, "AngleX", v);
        let (s, l) = (cheek_error(&smooth), cheek_error(&linear));
        assert!(s * 5.0 < l, "mesh: {s} vs {l} at {v}");
        // ワープの制御点と回転の原点も同じ曲線で動く。
        let arc = |i: &ModelInstance| {
            let d = i.warp_world_points(0).expect("warp")[0] - Vec2::new(64.0, 64.0);
            ((CENTER + d - Vec2::new(0.0, fixtures::SMOOTH_ARC_RADIUS) - CENTER).length()
                - fixtures::SMOOTH_ARC_RADIUS)
                .abs()
        };
        assert!(arc(&smooth) * 5.0 < arc(&linear), "warp at {v}");
        let origin = |i: &ModelInstance| {
            let t = i.rotation_world_frame(1).expect("rotation").translation;
            ((t - CENTER).length() - fixtures::SMOOTH_ARC_RADIUS).abs()
        };
        assert!(origin(&smooth) * 5.0 < origin(&linear), "rotation at {v}");
    }
}

// evaluation.md 曲線補間: 不透明度・描画順などの形以外のフィールドは線形の重みで補間する
#[test]
fn non_shape_fields_stay_linear() {
    let (mut smooth, mut linear) = pair();
    for v in [-20.0, -10.0, 10.0, 20.0] {
        set(&mut smooth, "AngleX", v);
        set(&mut linear, "AngleX", v);
        let (s, l) = (&smooth.drawables()[0], &linear.drawables()[0]);
        assert_eq!(
            (s.opacity, s.draw_order, s.multiply_color, s.mask_strength),
            (l.opacity, l.draw_order, l.multiply_color, l.mask_strength)
        );
        assert_ne!(s.positions, l.positions);
    }
    // -30..0 では不透明度 1・描画順 500 のまま (曲線の重みなら 1 を超え、500 を下回る)。
    set(&mut smooth, "AngleX", -15.0);
    assert_eq!(smooth.drawables()[0].opacity, 1.0);
    assert_eq!(smooth.drawables()[0].draw_order, 500.0);
}

// evaluation.md 曲線補間 / 6. ブレンドシェイプ: ブレンドシェイプ格子も曲線で補間する
#[test]
fn blend_shape_grids_interpolate_smoothly() {
    let (mut smooth, mut linear) = pair();
    let rest = smooth.drawables()[0].positions[0];
    // キー 0・0.5・1 の値 0・10・40 は放物線 40 s² の上にあり、曲線補間はそれを再現する。
    for s in [0.25f32, 0.5, 0.75, 1.0] {
        set(&mut smooth, "Squash", s);
        let dy = smooth.drawables()[0].positions[0].y - rest.y;
        assert!((dy - 40.0 * s * s).abs() < 1e-3, "{dy} at {s}");
    }
    set(&mut linear, "Squash", 0.25);
    assert_eq!(linear.drawables()[0].positions[0].y - rest.y, 5.0);
    // 問い合わせの API も同じ値を返す。
    set(&mut smooth, "AngleX", 12.0);
    set(&mut smooth, "Squash", 0.3);
    assert_eq!(
        smooth.mesh_local_positions(0).as_ref(),
        Some(&smooth.drawables()[0].positions)
    );
    let frame = smooth.rotation_world_frame(1).expect("rotation");
    let local = smooth.rotation_local_keyform(1).expect("rotation");
    assert_eq!(frame.translation, local.origin);
}

// evaluation.md 用語と前提: 決定的
#[test]
fn smooth_is_deterministic() {
    let (mut a, _) = pair();
    let (mut b, _) = pair();
    for v in [-27.3, -3.1, 0.7, 29.9] {
        set(&mut a, "AngleX", v);
        set(&mut b, "AngleX", v);
        assert_eq!(a.drawables(), b.drawables());
        let first = a.drawables().to_vec();
        a.update();
        assert_eq!(a.drawables(), &first[..]);
    }
}

// evaluation.md 変更フラグ: 形だけが変わる区間では POSITIONS だけが立つ
#[test]
fn change_flags() {
    let (mut smooth, _) = pair();
    set(&mut smooth, "AngleX", -20.0);
    set(&mut smooth, "AngleX", -10.0);
    assert_eq!(smooth.changes()[0], DrawableChanges::POSITIONS);
    smooth.update();
    assert!(smooth.changes()[0].is_empty());
    set(&mut smooth, "AngleX", 10.0);
    assert!(smooth.changes()[0].contains(DrawableChanges::POSITIONS | DrawableChanges::OPACITY));
}
