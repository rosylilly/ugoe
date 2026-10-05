//! デフォーマ・パーツ階層・グルーの評価 (`docs/spec/evaluation.md`)。
//! 入力は `ugoe_model::fixtures` のフィクスチャ。期待値の導出はフィクスチャの doc コメントを参照。

use ugoe_core::Vec2;
use ugoe_model::{Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

const TOL: f32 = 1e-4;

fn instance(model: Model) -> ModelInstance {
    ModelInstance::new(CompiledModel::compile(model).expect("compile"))
}

fn set(instance: &mut ModelInstance, id: &str, value: f32) {
    let index = instance.parameters().index_of(id).expect("parameter");
    instance.parameters_mut().set(index, value);
    instance.update();
}

fn mesh_index(instance: &ModelInstance, id: &str) -> usize {
    instance
        .compiled()
        .model()
        .meshes
        .iter()
        .position(|m| m.id == id)
        .expect("mesh")
}

fn positions<'a>(instance: &'a ModelInstance, id: &str) -> &'a [Vec2] {
    &instance.drawables()[mesh_index(instance, id)].positions
}

fn opacity(instance: &ModelInstance, id: &str) -> f32 {
    instance.drawables()[mesh_index(instance, id)].opacity
}

#[track_caller]
fn assert_positions(actual: &[Vec2], expected: &[(f32, f32)]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (a, &(x, y))) in actual.iter().zip(expected).enumerate() {
        assert!(
            a.abs_diff_eq(Vec2::new(x, y), TOL),
            "vertex {i}: {a} != ({x}, {y})\n  actual: {actual:?}"
        );
    }
}

#[track_caller]
fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < TOL, "{actual} != {expected}");
}

// ---- 3. ワープデフォーマ ----

#[test]
fn warp_maps_inside_points_bilinearly() {
    let mut m = instance(fixtures::warp_grid());
    assert_positions(
        positions(&m, "Inner"),
        &[(96.0, 96.0), (160.0, 96.0), (160.0, 160.0), (96.0, 160.0)],
    );
    set(&mut m, "X", 1.0);
    assert_positions(
        positions(&m, "Inner"),
        &[(144.0, 96.0), (208.0, 96.0), (176.0, 160.0), (112.0, 160.0)],
    );
    // 台形: 左上のセル (0,64)-(128,64)-(64,128)-(128,128) の中心
    set(&mut m, "X", -1.0);
    assert_eq!(positions(&m, "Inner")[0], Vec2::new(80.0, 96.0));
}

#[test]
fn warp_extrapolates_outside_points() {
    let mut m = instance(fixtures::warp_grid());
    // アフィンな格子では、格子の外でも同じアフィン写像になる。
    assert_positions(
        positions(&m, "Outer"),
        &[(0.0, 0.0), (256.0, 0.0), (256.0, 256.0), (0.0, 256.0)],
    );
    set(&mut m, "X", 1.0);
    assert_positions(
        positions(&m, "Outer"),
        &[(96.0, 0.0), (352.0, 0.0), (224.0, 256.0), (-32.0, 256.0)],
    );
    // 台形: 境界の点 + 四隅のアフィン近似の軸 (u 軸 (192,0)、v 軸 (0,128)) × はみ出し量。
    // 端のセルを延長する方式なら左上は (-256, 0) まで飛ぶ。
    set(&mut m, "X", -1.0);
    assert_positions(
        positions(&m, "Outer"),
        &[(-96.0, 0.0), (352.0, 0.0), (288.0, 256.0), (-32.0, 256.0)],
    );
}

// ---- 3. 回転デフォーマ ----

#[test]
fn rotation_without_parent_and_with_rotation_parent() {
    let mut m = instance(fixtures::rotation_chain());
    // Hand = Arm ∘ L_hand = (192,128) + 0.5 p
    assert_positions(
        positions(&m, "Palm"),
        &[
            (192.0, 128.0),
            (200.0, 128.0),
            (200.0, 136.0),
            (192.0, 136.0),
        ],
    );
    set(&mut m, "Angle", 30.0);
    assert_positions(
        positions(&m, "Palm"),
        &[
            (183.425_63, 160.0),
            (190.353_83, 164.0),
            (186.353_83, 170.928_2),
            (179.425_63, 166.928_2),
        ],
    );
}

#[test]
fn warp_under_rotation_uses_rotated_control_points() {
    let mut m = instance(fixtures::rotation_chain());
    assert_positions(
        positions(&m, "Sleeve"),
        &[
            (138.0, 153.0),
            (158.0, 153.0),
            (158.0, 163.0),
            (138.0, 163.0),
        ],
    );
    set(&mut m, "Angle", 30.0);
    // Arm の局所座標 (10,25) を 30 度回して (128,128) を足す
    assert!(positions(&m, "Sleeve")[0].abs_diff_eq(Vec2::new(124.160_25, 154.650_64), TOL));
}

// evaluation.md 1. 真偽値のフィールドは重み最大の要素 (同じ重みなら小さいインデックス)
#[test]
fn rotation_reflect_uses_max_weight_keyform() {
    let mut m = instance(fixtures::rotation_chain());
    set(&mut m, "Flip", 1.0);
    // S = diag(-0.5, 0.5)
    assert_positions(
        positions(&m, "Palm"),
        &[
            (192.0, 128.0),
            (184.0, 128.0),
            (184.0, 136.0),
            (192.0, 136.0),
        ],
    );
    set(&mut m, "Flip", 0.5); // 同じ重み → インデックス 0 (反転なし)
    assert_close(positions(&m, "Palm")[1].x, 200.0);
    set(&mut m, "Flip", 0.6);
    assert_close(positions(&m, "Palm")[1].x, 184.0);
}

#[test]
fn rotation_under_warp_takes_origin_and_angle_from_warp() {
    let mut m = instance(fixtures::rotation_under_warp());
    // 原点 = F(0.5, 0.25) = (128, 96)、δ = 0。ワープの 128 倍の伸縮は伝わらない。
    assert_positions(
        positions(&m, "Face"),
        &[(128.0, 86.0), (138.0, 86.0), (138.0, 96.0), (128.0, 96.0)],
    );
    // 90 度回した格子: 原点 (160, 128)、δ = 90 度
    set(&mut m, "Turn", 1.0);
    assert_positions(
        positions(&m, "Face"),
        &[
            (170.0, 128.0),
            (170.0, 138.0),
            (160.0, 138.0),
            (160.0, 128.0),
        ],
    );
    // 菱形 (45 度、1/√2 倍): 原点 (144, 112)、δ = 45 度、長さは保たれる
    set(&mut m, "Turn", 0.5);
    assert_positions(
        positions(&m, "Face"),
        &[
            (151.071_07, 104.928_93),
            (158.142_14, 112.0),
            (151.071_07, 119.071_07),
            (144.0, 112.0),
        ],
    );
}

#[test]
fn warp_under_warp_maps_control_points_through_parent() {
    let mut m = instance(fixtures::rotation_under_warp());
    // Cheek の UV (0.5, 0.5) = Body の UV (0.5, 0.75)
    assert_positions(&positions(&m, "Blush")[..1], &[(128.0, 160.0)]);
    set(&mut m, "Turn", 1.0);
    assert_positions(&positions(&m, "Blush")[..1], &[(96.0, 128.0)]);
    set(&mut m, "Turn", 0.5);
    assert_positions(&positions(&m, "Blush")[..1], &[(112.0, 144.0)]);
}

// ---- 2. パーツの不透明度 / 3. デフォーマの不透明度 / 4. メッシュ ----

#[test]
fn opacity_propagates_through_parts_and_deformers() {
    let mut m = instance(fixtures::part_opacity_hierarchy());
    // A = 1 × (1 × 0.5 × 0.8) × (0.5 × 0.5)
    assert_close(opacity(&m, "A"), 0.1);
    assert_close(opacity(&m, "B"), 1.0);
    // C = 1 × (1 × 0.5) × 0.5
    assert_close(opacity(&m, "C"), 0.25);
    assert_close(opacity(&m, "D"), 1.0);
    // 恒等変換の回転デフォーマ越しでも位置は変わらない
    assert_positions(
        positions(&m, "A"),
        &[(64.0, 64.0), (192.0, 64.0), (192.0, 192.0), (64.0, 192.0)],
    );

    set(&mut m, "Fade", 1.0);
    assert_close(opacity(&m, "A"), 0.05);
    assert_close(opacity(&m, "C"), 0.125);
    assert_close(opacity(&m, "D"), 0.5);
    let b = &m.drawables()[mesh_index(&m, "B")];
    assert_eq!(b.opacity, 0.0);
    assert!(!b.visible);
    assert!(m.drawables()[mesh_index(&m, "A")].visible);
}

// ---- 7. 描画順 ----

#[test]
fn render_order_is_stable_for_ties() {
    let mut m = instance(fixtures::part_opacity_hierarchy());
    // C (300) の後に、描画順 500 の A, B, D を宣言順で
    assert_eq!(m.render_order(), &[2, 0, 1, 3]);
    // D が 300 になり C と同順位 → 宣言順で C, D
    set(&mut m, "Fade", 0.5);
    assert_eq!(m.render_order(), &[2, 3, 0, 1]);
    // 不可視の B も描画順には含める
    set(&mut m, "Fade", 1.0);
    assert_eq!(m.render_order(), &[3, 2, 0, 1]);
}

// ---- 5. グルー ----

#[test]
fn glues_apply_in_array_order() {
    let mut m = instance(fixtures::glued_quads());
    // Seam で中点 (105, y) に寄せた後、Pull が Right の頂点 1 を Left の頂点 1 へ半分寄せる
    assert_positions(
        positions(&m, "Left"),
        &[(0.0, 0.0), (105.0, 0.0), (105.0, 100.0), (0.0, 100.0)],
    );
    assert_positions(
        positions(&m, "Right"),
        &[(105.0, 0.0), (157.5, 0.0), (210.0, 100.0), (105.0, 100.0)],
    );

    set(&mut m, "Glue", 0.5);
    assert_positions(
        positions(&m, "Left"),
        &[(0.0, 0.0), (102.5, 0.0), (102.5, 100.0), (0.0, 100.0)],
    );
    assert_positions(
        positions(&m, "Right"),
        &[(107.5, 0.0), (156.25, 0.0), (210.0, 100.0), (107.5, 100.0)],
    );

    // Seam の強さ 0: Pull だけが効く
    set(&mut m, "Glue", 0.0);
    assert_positions(
        positions(&m, "Right"),
        &[(110.0, 0.0), (155.0, 0.0), (210.0, 100.0), (110.0, 100.0)],
    );
}

#[test]
fn deformers_are_evaluated_parents_first() {
    let compiled = CompiledModel::compile(fixtures::part_opacity_hierarchy()).unwrap();
    // InnerRot (0) は Outer (1) の子
    assert_eq!(compiled.deformer_order(), &[1, 0]);
}
