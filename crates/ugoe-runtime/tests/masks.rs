//! マスクのデータのランタイム出力 (`docs/spec/rendering.md#マスク`)。
//! 入力は `ugoe_model::fixtures` のマスクのフィクスチャ。

use ugoe_model::{MaskMode, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance, ResolvedMaskRef};

const TOL: f32 = 1e-4;

fn instance(model: Model) -> ModelInstance {
    ModelInstance::new(CompiledModel::compile(model).expect("compile"))
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

fn mask_ref(instance: &ModelInstance, id: &str, invert: bool) -> ResolvedMaskRef {
    ResolvedMaskRef {
        mesh: mesh_index(instance, id) as u32,
        invert,
    }
}

#[test]
fn nested_masks_are_ordered_by_dependency() {
    let m = instance(fixtures::nested_masks());
    let c = m.compiled();
    let (pupil, iris, sclera) = (
        mesh_index(&m, "Pupil"),
        mesh_index(&m, "Iris"),
        mesh_index(&m, "Sclera"),
    );
    assert_eq!(c.mask_refs(pupil).unwrap(), [mask_ref(&m, "Iris", false)]);
    assert_eq!(c.mask_refs(iris).unwrap(), [mask_ref(&m, "Sclera", false)]);
    assert_eq!(c.mask_refs(sclera).unwrap(), []);
    assert_eq!(
        [
            c.mask_depth(sclera),
            c.mask_depth(iris),
            c.mask_depth(pupil)
        ],
        [Some(0), Some(1), Some(2)]
    );
    // T0187: 範囲外の番号は `None`。
    let out = m.compiled().model().meshes.len();
    assert_eq!(c.mask_refs(out), None);
    assert_eq!(c.mask_mode(out), None);
    assert_eq!(c.mask_depth(out), None);
    // 宣言順は Pupil, Iris, Sclera だが、マスクが先に並ぶ。Pupil はマスクとして使われないので含まない。
    assert_eq!(c.mask_order(), [sclera as u32, iris as u32]);
}

#[test]
fn mask_order_puts_every_mask_after_its_own_masks() {
    for model in [
        fixtures::nested_masks(),
        fixtures::mixed_masks(),
        fixtures::intersection_masks(),
    ] {
        let c = CompiledModel::compile(model).expect("compile");
        let order = c.mask_order();
        let position = |mesh: u32| order.iter().position(|&i| i == mesh);
        for (i, &mesh) in order.iter().enumerate() {
            for r in c.mask_refs(mesh as usize).unwrap() {
                assert!(position(r.mesh).is_some_and(|p| p < i));
            }
        }
        // マスクとして参照されるメッシュはすべて含まれる。
        for mesh in 0..c.model().meshes.len() {
            for r in c.mask_refs(mesh).unwrap() {
                assert!(position(r.mesh).is_some());
            }
        }
    }
}

#[test]
fn mixed_masks_resolve_invert_per_reference() {
    let m = instance(fixtures::mixed_masks());
    let brow = mesh_index(&m, "Brow");
    assert_eq!(
        m.compiled().mask_refs(brow).unwrap(),
        [mask_ref(&m, "Face", false), mask_ref(&m, "Hair", true)]
    );
    assert_eq!(m.compiled().mask_mode(brow), Some(MaskMode::Union));
}

#[test]
fn mask_strength_is_interpolated_and_clamped() {
    let mut m = instance(fixtures::mixed_masks());
    let brow = mesh_index(&m, "Brow");
    let face = mesh_index(&m, "Face");
    let strength = m.parameters().index_of("Strength").expect("parameter");
    // 補間値は -1 + 3 × Strength。
    for (value, expected) in [(0.0, 0.0), (0.2, 0.0), (0.5, 0.5), (0.6, 0.8), (1.0, 1.0)] {
        m.parameters_mut().set(strength, value);
        m.update();
        let actual = m.drawables()[brow].mask_strength;
        assert!(
            (actual - expected).abs() < TOL,
            "Strength = {value}: {actual} != {expected}"
        );
        // キーフォームで指定しないメッシュは既定の 1。
        assert_eq!(m.drawables()[face].mask_strength, 1.0);
    }
}

#[test]
fn intersection_and_mask_only() {
    let m = instance(fixtures::intersection_masks());
    let (face, mouth, tongue) = (
        mesh_index(&m, "Face"),
        mesh_index(&m, "MouthMask"),
        mesh_index(&m, "Tongue"),
    );
    let c = m.compiled();
    assert_eq!(c.mask_mode(tongue), Some(MaskMode::Intersection));
    assert_eq!(c.mask_mode(face), Some(MaskMode::Union));
    assert_eq!(
        c.mask_refs(tongue).unwrap(),
        [
            mask_ref(&m, "MouthMask", false),
            mask_ref(&m, "Face", false)
        ]
    );
    assert_eq!(c.mask_order(), [face as u32, mouth as u32]);
    let d = m.drawables();
    // マスク専用メッシュは不透明度 1 でも色としては描かない。
    assert_eq!(d[mouth].opacity, 1.0);
    assert!(!d[mouth].visible);
    assert!(d[face].visible);
    assert!(d[tongue].visible);
}
