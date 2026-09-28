//! 親が異なる左右の対のミラーのテスト (`docs/spec/keyform-inference.md#親が異なる対`)。
//!
//! 結果は、ランタイムで評価したキャンバス座標の姿が左右対称になることで確かめる: 自分自身と対にした
//! `AngleX` の値 `k` での左の姿を x = 128 (キャンバスの原点) で反転したものが、`−k` での右の姿になる。

use ugoe_core::{Affine2, Mat2, Vec2};
use ugoe_model::deformer::{RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    Deformer, DeformerKind, KeyformBinding, KeyformGrid, KeyformOrigin, Mesh, Model, Parameter,
    fixtures,
};
use ugoe_rigging::{
    InferError, KeyformTarget, MirrorOptions, ParentWorld, check_mirror, mirror_keyforms,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

const KEYS: [f32; 3] = [-30.0, 0.0, 30.0];
/// キャンバスの原点の x (`keyform_inference` のフィクスチャ)。
const AXIS: f32 = 128.0;

fn parameter(id: &str, mirror: Option<&str>) -> Parameter {
    Parameter {
        id: id.into(),
        name: id.into(),
        min: -30.0,
        max: 30.0,
        default: 0.0,
        kind: Default::default(),
        mirror: mirror.map(Into::into),
        group: None,
    }
}

fn binding() -> Vec<KeyformBinding> {
    vec![KeyformBinding {
        parameter: "AngleX".into(),
        keys: KEYS.to_vec(),
    }]
}

/// AngleX のキーごとの格子 (すべて `authored`)。
fn grid<T: Clone>(cell: impl Fn(f32) -> T) -> KeyformGrid<T> {
    KeyformGrid::new(binding(), KEYS.iter().map(|&k| cell(k)).collect())
}

/// 基準のセルだけが `authored` で、残りは基準と同じ値の `derived` の格子。
fn stale<T: Clone>(base: T) -> KeyformGrid<T> {
    let mut grid = KeyformGrid::new(binding(), vec![base; 3]);
    grid.origins = vec![
        KeyformOrigin::Derived,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    grid
}

fn deformer(id: &str, parent: Option<&str>, mirror: Option<&str>, kind: DeformerKind) -> Deformer {
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: parent.map(Into::into),
        kind,
        mirror: mirror.map(Into::into),
        mirror_axis: None,
    }
}

/// 1×1 のワープ。制御点は `min`〜`max` の矩形を AngleX の値 `k` で x に `k / 3` ずらし、
/// 右上の制御点をさらに `(k / 6, 4)` ずらす (双線形の変形になる)。
fn warp(min: Vec2, max: Vec2) -> DeformerKind {
    DeformerKind::Warp(WarpDeformer {
        rows: 1,
        cols: 1,
        keyforms: grid(|k| {
            let d = Vec2::new(k / 3.0, 0.0);
            WarpKeyform {
                points: vec![
                    min + d,
                    Vec2::new(max.x, min.y) + d + Vec2::new(k / 6.0, 4.0),
                    Vec2::new(min.x, max.y) + d,
                    max + d,
                ],
                opacity: 1.0,
            }
        }),
        depth: None,
    })
}

/// 回転デフォーマ。原点は `origin` を AngleX の値 `k` で x に `k / 3`、角度は `angle + k / 2`。
fn rotation(origin: Vec2, angle: f32, scale: f32) -> DeformerKind {
    DeformerKind::Rotation(RotationDeformer {
        keyforms: grid(|k| RotationKeyform {
            origin: origin + Vec2::new(k / 3.0, 0.0),
            angle: angle + k / 2.0,
            scale,
            reflect_x: false,
            reflect_y: false,
            opacity: 1.0,
        }),
    })
}

/// 左右の親を持つモデル: `WarpL` (左の頬)、`WarpR` (右の頬、左と形も大きさも違う)、
/// `RotL` / `RotR` (回転デフォーマ)。AngleX は自分自身と対にする。
fn model() -> Model {
    let mut model = fixtures::keyform_inference();
    model.parameters = vec![
        parameter("AngleX", Some("AngleX")),
        parameter("AngleY", None),
    ];
    model.deformers.extend([
        deformer(
            "WarpL",
            None,
            None,
            warp(Vec2::new(16.0, 64.0), Vec2::new(112.0, 192.0)),
        ),
        deformer(
            "WarpR",
            None,
            None,
            warp(Vec2::new(136.0, 48.0), Vec2::new(256.0, 208.0)),
        ),
        deformer(
            "RotL",
            None,
            None,
            rotation(Vec2::new(80.0, 100.0), 20.0, 1.2),
        ),
        deformer(
            "RotR",
            None,
            None,
            rotation(Vec2::new(170.0, 90.0), -35.0, 1.5),
        ),
    ]);
    model
}

fn world(model: &Model, parent: Option<&str>, k: f32) -> ParentWorld {
    let parent = parent.map(Into::into);
    let mut world = ParentWorld::new(model, parent.as_ref()).unwrap_or_else(|e| panic!("{e}"));
    world.set_pose([("AngleX", k)]);
    world
}

/// キャンバス座標の点を親の座標系に写す (テストの入力を作るため)。
fn local(model: &Model, parent: Option<&str>, k: f32, canvas: &[Vec2]) -> Vec<Vec2> {
    world(model, parent, k)
        .from_canvas(canvas, canvas)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn reflect(p: Vec2) -> Vec2 {
    Vec2::new(2.0 * AXIS - p.x, p.y)
}

/// 左目のキャンバス座標の基準の姿。
fn eye() -> Vec<Vec2> {
    vec![
        Vec2::new(40.0, 115.0),
        Vec2::new(64.0, 115.0),
        Vec2::new(64.0, 128.0),
        Vec2::new(40.0, 128.0),
    ]
}

fn mesh_keyform(positions: Vec<Vec2>) -> MeshKeyform {
    let mut kf = fixtures::keyform_inference().meshes[0].keyforms.keyforms[0].clone();
    kf.positions = positions;
    kf
}

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model
        .meshes
        .iter_mut()
        .find(|m| m.id == id.into())
        .unwrap_or_else(|| panic!("no {id}"))
}

/// `EyeL` を `left` の下、`EyeR` を `right` の下に置く。左目は AngleX のキーごとにキャンバス座標で
/// 動かしたセル (すべて `authored`)、右目は左目の基準の姿をキャンバス座標で反転した基準のセルだけを持つ。
fn eyes(left: Option<&str>, right: Option<&str>) -> Model {
    let mut model = model();
    let offset = |k: f32| {
        if k < 0.0 {
            Vec2::new(-6.0, 0.0)
        } else if k > 0.0 {
            Vec2::new(3.0, 2.0)
        } else {
            Vec2::ZERO
        }
    };
    let cells: Vec<MeshKeyform> = KEYS
        .iter()
        .map(|&k| {
            let canvas: Vec<Vec2> = eye().iter().map(|&p| p + offset(k)).collect();
            mesh_keyform(local(&model, left, k, &canvas))
        })
        .collect();
    let mirrored: Vec<Vec2> = eye().into_iter().map(reflect).collect();
    let right_base = mesh_keyform(local(&model, right, 0.0, &mirrored));
    let eye_l = mesh_mut(&mut model, "EyeL");
    eye_l.parent = left.map(Into::into);
    eye_l.mirror_axis = None;
    eye_l.keyforms = KeyformGrid::new(binding(), cells);
    let eye_r = mesh_mut(&mut model, "EyeR");
    eye_r.parent = right.map(Into::into);
    eye_r.mirror_axis = None;
    eye_r.keyforms = stale(right_base);
    model
}

fn instance(model: &Model, k: f32) -> ModelInstance {
    let mut instance =
        ModelInstance::new(CompiledModel::compile(model.clone()).unwrap_or_else(|e| panic!("{e}")));
    let index = instance
        .parameters()
        .index_of("AngleX")
        .unwrap_or_else(|| panic!("no AngleX"));
    instance.parameters_mut().set(index, k);
    instance.update();
    instance
}

fn drawn(model: &Model, id: &str, k: f32) -> Vec<Vec2> {
    let instance = instance(model, k);
    let index = instance
        .compiled()
        .mesh_index(id)
        .unwrap_or_else(|| panic!("no {id}"));
    instance.drawables()[index].positions.clone()
}

#[track_caller]
fn assert_near(actual: &[Vec2], expected: &[Vec2], eps: f32) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!(a.distance(*e) < eps, "{actual:?} vs {expected:?}");
    }
}

/// どの AngleX のキーでも、右目の姿が左目の姿の反転になっている。
#[track_caller]
fn assert_eyes_symmetric(model: &Model) {
    for k in KEYS {
        let left: Vec<Vec2> = drawn(model, "EyeL", k).into_iter().map(reflect).collect();
        assert_near(&drawn(model, "EyeR", -k), &left, 1e-2);
    }
}

fn mirror(model: &mut Model, from: &str, to: &str) -> Result<Vec<usize>, InferError> {
    let (from, to) = if model.meshes.iter().any(|m| m.id == from.into()) {
        (
            KeyformTarget::Mesh(from.into()),
            KeyformTarget::Mesh(to.into()),
        )
    } else {
        (
            KeyformTarget::Deformer(from.into()),
            KeyformTarget::Deformer(to.into()),
        )
    };
    mirror_keyforms(model, &from, &to, &MirrorOptions::default()).map(|r| r.written)
}

#[test]
fn meshes_under_different_warps_mirror_through_the_canvas() {
    let mut model = eyes(Some("WarpL"), Some("WarpR"));
    assert_eq!(mirror(&mut model, "EyeL", "EyeR").unwrap(), vec![0, 2]);
    let right = &mesh_mut(&mut model, "EyeR").keyforms;
    assert_eq!(right.origin(0), KeyformOrigin::Mirror);
    assert_eq!(right.mirror_source.as_ref().unwrap().id, "EyeL");
    assert_eyes_symmetric(&model);
}

#[test]
fn mesh_under_a_rotation_mirrors_onto_a_mesh_under_a_warp() {
    for (left, right) in [
        (Some("RotL"), Some("WarpR")),
        (Some("WarpL"), Some("RotR")),
        (None, Some("WarpR")),
        (Some("RotL"), None),
    ] {
        let mut model = eyes(left, right);
        mirror(&mut model, "EyeL", "EyeR").unwrap();
        assert_eyes_symmetric(&model);
    }
}

#[test]
fn warps_under_different_parents_mirror_their_control_points() {
    let mut model = model();
    // 左の子のワープは RotL の下 (局所座標)、右はキャンバス直下。基準のセルは左右対称にする。
    let canvas = |k: f32| {
        let d = Vec2::new(0.0, k / 5.0);
        vec![
            Vec2::new(50.0, 90.0) + d,
            Vec2::new(90.0, 92.0),
            Vec2::new(52.0, 140.0),
            Vec2::new(95.0, 138.0) - d,
        ]
    };
    let left: Vec<WarpKeyform> = KEYS
        .iter()
        .map(|&k| WarpKeyform {
            points: local(&model, Some("RotL"), k, &canvas(k)),
            opacity: 1.0,
        })
        .collect();
    let flip = |points: Vec<Vec2>| -> Vec<Vec2> {
        let p: Vec<Vec2> = points.into_iter().map(reflect).collect();
        vec![p[1], p[0], p[3], p[2]]
    };
    let right_base = WarpKeyform {
        points: flip(canvas(0.0)),
        opacity: 1.0,
    };
    let kind = |keyforms| {
        DeformerKind::Warp(WarpDeformer {
            rows: 1,
            cols: 1,
            keyforms,
            depth: None,
        })
    };
    model.deformers.extend([
        deformer(
            "ChildL",
            Some("RotL"),
            Some("ChildR"),
            kind(KeyformGrid::new(binding(), left)),
        ),
        deformer("ChildR", None, Some("ChildL"), kind(stale(right_base))),
    ]);
    assert_eq!(mirror(&mut model, "ChildL", "ChildR").unwrap(), vec![0, 2]);
    for k in KEYS {
        let points = |k: f32, id: &str| {
            let instance = instance(&model, k);
            let index = instance.compiled().deformer_index(id).unwrap();
            instance.warp_world_points(index).unwrap().to_vec()
        };
        assert_near(&points(-k, "ChildR"), &flip(points(k, "ChildL")), 1e-2);
    }
}

#[test]
fn rotations_under_different_parents_mirror_their_world_frames() {
    // 左の子の回転デフォーマはワープの下、右は拡大率の違う回転デフォーマの下。
    // 角度・拡大率は、キャンバス座標のワールド変換が反転になるように決まる。
    let mut model = model();
    let left: Vec<RotationKeyform> = KEYS
        .iter()
        .map(|&k| RotationKeyform {
            origin: local(
                &model,
                Some("WarpL"),
                k,
                &[Vec2::new(60.0, 110.0 + k / 10.0)],
            )[0],
            angle: 10.0 + k,
            scale: 0.8,
            reflect_x: true,
            reflect_y: false,
            opacity: 1.0,
        })
        .collect();
    let right_base = RotationKeyform {
        origin: local(
            &model,
            Some("RotR"),
            0.0,
            &[reflect(Vec2::new(60.0, 110.0))],
        )[0],
        ..left[1]
    };
    let kind = |keyforms| DeformerKind::Rotation(RotationDeformer { keyforms });
    model.deformers.extend([
        deformer(
            "ChildL",
            Some("WarpL"),
            Some("ChildR"),
            kind(KeyformGrid::new(binding(), left)),
        ),
        deformer(
            "ChildR",
            Some("RotR"),
            Some("ChildL"),
            kind(stale(right_base)),
        ),
    ]);
    assert_eq!(mirror(&mut model, "ChildL", "ChildR").unwrap(), vec![0, 2]);
    let f = Mat2::from_diagonal(Vec2::new(-1.0, 1.0));
    let frame = |k: f32, id: &str| -> Affine2 {
        let instance = instance(&model, k);
        let index = instance.compiled().deformer_index(id).unwrap();
        instance.rotation_world_frame(index).unwrap()
    };
    for k in [-30.0, 30.0] {
        let (left, right) = (frame(k, "ChildL"), frame(-k, "ChildR"));
        assert!(
            right.translation.distance(reflect(left.translation)) < 1e-2,
            "{right:?} vs {left:?}"
        );
        let expected = f * left.matrix2 * f;
        for (a, e) in [
            (right.matrix2.x_axis, expected.x_axis),
            (right.matrix2.y_axis, expected.y_axis),
        ] {
            assert!(a.distance(e) < 1e-4, "{right:?} vs {expected:?}");
        }
    }
}

#[test]
fn same_parent_pairs_keep_the_parent_space_formula() {
    // 親が同じ (どちらも `Face`) なら、親の座標系で `x' = 2a − x` と反転した値そのもの。
    let mut model = fixtures::keyform_inference();
    model.parameters = vec![
        parameter("AngleX", Some("AngleX")),
        parameter("AngleY", None),
    ];
    let left = mesh_mut(&mut model, "EyeL").keyforms.keyforms[0].clone();
    let right = mesh_mut(&mut model, "EyeR").keyforms.keyforms[0].clone();
    let shift = |kf: &MeshKeyform, d: Vec2| {
        let mut kf = kf.clone();
        kf.positions.iter_mut().for_each(|p| *p += d);
        kf
    };
    mesh_mut(&mut model, "EyeL").keyforms =
        grid(|k| shift(&left, Vec2::new(k / 1000.0, k.abs() / 3000.0)));
    mesh_mut(&mut model, "EyeR").keyforms = stale(right);
    let source = model.clone();
    mirror(&mut model, "EyeL", "EyeR").unwrap();
    let source = &source
        .meshes
        .iter()
        .find(|m| m.id == "EyeL".into())
        .unwrap();
    let right = &mesh_mut(&mut model, "EyeR").keyforms;
    for (t, s) in [(0, 2), (2, 0)] {
        let p = &source.keyforms.keyforms[s].positions;
        // 頂点の対応は 1 0 3 2 (`tests/mirror.rs`)。
        let expected: Vec<Vec2> = [1, 0, 3, 2]
            .iter()
            .map(|&i| Vec2::new(2.0 * 0.5 - p[i].x, p[i].y))
            .collect();
        assert_eq!(right.keyforms[t].positions, expected);
    }
}

#[test]
fn explicit_axis_is_mapped_to_the_canvas_from_the_source_parent() {
    // 元がキャンバス直下なら `mirror_axis` はそのままキャンバス座標の x。
    let mut model = eyes(None, Some("WarpR"));
    // 右目を x = 120 の反転の位置に置き直す (基準のセル)。
    let mirrored: Vec<Vec2> = eye()
        .into_iter()
        .map(|p| Vec2::new(240.0 - p.x, p.y))
        .collect();
    let right_base = local(&model, Some("WarpR"), 0.0, &mirrored);
    let eye_r = mesh_mut(&mut model, "EyeR");
    eye_r.keyforms.keyforms[1].positions = right_base;
    // 既定の軸 (キャンバスの原点) では対応しない。
    let err = check_mirror(
        &model,
        &KeyformTarget::Mesh("EyeL".into()),
        &KeyformTarget::Mesh("EyeR".into()),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, InferError::UnmatchedVertices { .. }),
        "{err:?}"
    );
    mesh_mut(&mut model, "EyeL").mirror_axis = Some(120.0);
    mirror(&mut model, "EyeL", "EyeR").unwrap();
    for k in KEYS {
        let left: Vec<Vec2> = drawn(&model, "EyeL", k)
            .into_iter()
            .map(|p| Vec2::new(240.0 - p.x, p.y))
            .collect();
        assert_near(&drawn(&model, "EyeR", -k), &left, 1e-2);
    }
}

#[test]
fn unmatched_vertices_and_tolerance_are_in_canvas_pixels() {
    let mut model = eyes(None, Some("WarpR"));
    // 右目の頂点 2 をキャンバス座標で 3px ずらす。
    let mut canvas: Vec<Vec2> = eye().into_iter().map(reflect).collect();
    canvas[2].x += 3.0;
    let right_base = local(&model, Some("WarpR"), 0.0, &canvas);
    mesh_mut(&mut model, "EyeR").keyforms.keyforms[1].positions = right_base;
    let before = model.clone();
    let err = mirror(&mut model, "EyeL", "EyeR").unwrap_err();
    assert_eq!(
        err,
        InferError::UnmatchedVertices {
            vertices: vec![2],
            tolerance: 1.0
        }
    );
    assert_eq!(model, before);
    // 許容距離は元の親の座標系 (ここではキャンバス直下なのでピクセル) で指定する。
    let options = MirrorOptions {
        tolerance: Some(4.0),
        ..Default::default()
    };
    mirror_keyforms(
        &mut model,
        &KeyformTarget::Mesh("EyeL".into()),
        &KeyformTarget::Mesh("EyeR".into()),
        &options,
    )
    .unwrap();
}

#[test]
fn unsupported_target_parent_is_rejected() {
    let mut model = eyes(Some("WarpL"), Some("WarpR"));
    model
        .deformers
        .push(deformer("Future", None, None, DeformerKind::Unknown));
    mesh_mut(&mut model, "EyeR").parent = Some("Future".into());
    let before = model.clone();
    let err = mirror(&mut model, "EyeL", "EyeR").unwrap_err();
    assert_eq!(
        err,
        InferError::UnsupportedParent {
            deformer: "Future".into()
        }
    );
    assert_eq!(model, before);
}
