//! ミラーのテスト (`docs/spec/keyform-inference.md#ミラー`)。

use ugoe_core::Vec2;
use ugoe_model::deformer::{RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    Deformer, DeformerKind, KeyformBinding, KeyformGrid, KeyformOrigin, Mesh, MirrorSource, Model,
    Parameter, fixtures,
};
use ugoe_rigging::{InferError, InferScope, KeyformTarget, MirrorOptions, mirror_keyforms};

const EPS: f32 = 1e-4;

fn parameter(id: &str, default: f32, mirror: Option<&str>) -> Parameter {
    Parameter {
        id: id.into(),
        name: id.into(),
        min: -30.0,
        max: 30.0,
        default,
        kind: Default::default(),
        mirror: mirror.map(Into::into),
        group: None,
        internal: false,
    }
}

/// `keyform_inference` のフィクスチャに、自分自身と対にした `AngleX` と、対にした目の開閉を加えたモデル。
/// 目のメッシュ (親は顔のワープ、UV 空間) は `EyeL` ↔ `EyeR`、鼻は自分自身と対応する。
fn model() -> Model {
    let mut model = fixtures::keyform_inference();
    model.parameters = vec![
        parameter("AngleX", 0.0, Some("AngleX")),
        parameter("AngleY", 0.0, None),
        parameter("EyeLOpen", 1.0, Some("EyeROpen")),
        parameter("EyeROpen", 1.0, Some("EyeLOpen")),
    ];
    model
}

fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model
        .meshes
        .iter_mut()
        .find(|m| m.id == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model
        .meshes
        .iter()
        .find(|m| m.id == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

fn shifted(kf: &MeshKeyform, dx: f32, dy: f32) -> MeshKeyform {
    let mut kf = kf.clone();
    for p in &mut kf.positions {
        *p += Vec2::new(dx, dy);
    }
    kf
}

/// 格子を `keyforms` で置き換える。`authored` の添字以外は `derived`。
fn set_grid<T: Clone>(
    grid: &mut KeyformGrid<T>,
    bindings: Vec<KeyformBinding>,
    keyforms: Vec<T>,
    authored: &[usize],
) {
    let len = keyforms.len();
    *grid = KeyformGrid::new(bindings, keyforms);
    grid.origins = (0..len)
        .map(|i| {
            if authored.contains(&i) {
                KeyformOrigin::Authored
            } else {
                KeyformOrigin::Derived
            }
        })
        .collect();
}

fn assert_points(actual: &[Vec2], expected: &[Vec2]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!(a.distance(*e) < EPS, "{actual:?} vs {expected:?}");
    }
}

fn target(id: &str) -> KeyformTarget {
    KeyformTarget::Mesh(id.into())
}

/// `EyeL` の格子 (AngleX -30 0 30) と、同じ軸で基準だけが `authored` の `EyeR` の格子を作る。
fn eye_model() -> Model {
    let mut model = model();
    let keys = [-30.0, 0.0, 30.0];
    let left = mesh(&model, "EyeL").keyforms.keyforms[0].clone();
    let right = mesh(&model, "EyeR").keyforms.keyforms[0].clone();
    set_grid(
        &mut mesh_mut(&mut model, "EyeL").keyforms,
        vec![binding("AngleX", &keys)],
        vec![
            shifted(&left, -0.05, 0.0),
            left.clone(),
            shifted(&left, 0.02, 0.01),
        ],
        &[0, 1, 2],
    );
    set_grid(
        &mut mesh_mut(&mut model, "EyeR").keyforms,
        vec![binding("AngleX", &keys)],
        vec![right.clone(); 3],
        &[1],
    );
    model
}

#[test]
fn mesh_pair_reflects_matched_vertices_and_self_mirrored_keys() {
    let mut model = eye_model();
    let report = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap();
    // 基準のセル (AngleX = 0) は authored なので書かない。
    assert_eq!(report.written, vec![0, 2]);

    let right = &mesh(&model, "EyeR").keyforms;
    assert_eq!(
        right.origins,
        vec![
            KeyformOrigin::Mirror,
            KeyformOrigin::Authored,
            KeyformOrigin::Mirror
        ]
    );
    // ミラー元を先の格子に記録する。
    assert_eq!(
        right.mirror_source,
        Some(MirrorSource {
            id: "EyeL".into(),
            axis: None,
            tolerance: None,
        })
    );
    // EyeL の頂点 (0.2,0.4) (0.4,0.4) (0.4,0.5) (0.2,0.5) を x = 0.5 で反転すると
    // EyeR の頂点 (0.6,0.4) (0.8,0.4) (0.8,0.5) (0.6,0.5) の 1 0 3 2 番目に重なる。
    // AngleX = -30 の EyeR は AngleX = +30 の EyeL (+0.02, +0.01) を反転したもの。
    assert_points(
        &right.keyforms[0].positions,
        &[
            Vec2::new(0.58, 0.41),
            Vec2::new(0.78, 0.41),
            Vec2::new(0.78, 0.51),
            Vec2::new(0.58, 0.51),
        ],
    );
    // AngleX = +30 の EyeR は AngleX = -30 の EyeL (-0.05) を反転したもの。
    assert_points(
        &right.keyforms[2].positions,
        &[
            Vec2::new(0.65, 0.4),
            Vec2::new(0.85, 0.4),
            Vec2::new(0.85, 0.5),
            Vec2::new(0.65, 0.5),
        ],
    );
    // 頂点以外のフィールドはそのまま写す。
    assert_eq!(right.keyforms[2].draw_order, 500.0);

    // 同じ入力からは同じ結果 (決定的)。
    let again = {
        let mut m = eye_model();
        mirror_keyforms(
            &mut m,
            &target("EyeL"),
            &target("EyeR"),
            &MirrorOptions::default(),
        )
        .unwrap();
        m
    };
    assert_eq!(model, again);
}

#[test]
fn unmatched_vertices_are_listed() {
    let mut model = eye_model();
    // EyeR の頂点 1 と 3 を 2px (顔のワープは 128px 幅なので UV で 2/128) ずらす。
    let right = mesh_mut(&mut model, "EyeR");
    right.keyforms.keyforms[1].positions[1].x += 2.0 / 128.0;
    right.keyforms.keyforms[1].positions[3].y -= 2.0 / 128.0;
    let before = model.clone();

    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        InferError::UnmatchedVertices {
            vertices: vec![1, 3],
            tolerance: 1.0 / 128.0,
        }
    );
    assert_eq!(model, before, "errors must not change the model");

    // 許容距離を広げれば対応する。
    let options = MirrorOptions {
        tolerance: Some(3.0 / 128.0),
        ..Default::default()
    };
    mirror_keyforms(&mut model, &target("EyeL"), &target("EyeR"), &options).unwrap();
}

#[test]
fn self_symmetric_mesh_mirrors_one_side_onto_the_other() {
    let mut model = model();
    let nose = mesh(&model, "Nose").keyforms.keyforms[0].clone();
    // 左 (AngleX = -30) だけ手で作る。右 (+30) と、上下 (AngleY) の軸の中立でないセルは derived。
    let mut left = nose.clone();
    left.positions[0].x -= 0.03; // 左上だけ動かす
    left.opacity = 0.5;
    let mut faded = nose.clone();
    faded.opacity = 0.25;
    set_grid(
        &mut mesh_mut(&mut model, "Nose").keyforms,
        vec![
            binding("AngleX", &[-30.0, 0.0, 30.0]),
            binding("AngleY", &[0.0, 30.0]),
        ],
        vec![left, nose.clone(), nose.clone(), nose.clone(), nose, faded],
        &[0, 1],
    );
    let report = mirror_keyforms(
        &mut model,
        &target("Nose"),
        &target("Nose"),
        &MirrorOptions::default(),
    )
    .unwrap();
    // 自分自身に写るセル (AngleX = 0) と、元が derived のセル (AngleY = 30 の行) は書かない。
    assert_eq!(report.written, vec![2]);
    let grid = &mesh(&model, "Nose").keyforms;
    // 鼻 (0.45..0.55) の左上 (0.42, 0.5) は反転すると右上 (0.58, 0.5) になる。
    assert_points(
        &grid.keyforms[2].positions,
        &[
            Vec2::new(0.45, 0.5),
            Vec2::new(0.58, 0.5),
            Vec2::new(0.55, 0.7),
            Vec2::new(0.45, 0.7),
        ],
    );
    assert_eq!(grid.keyforms[2].opacity, 0.5);
    assert_eq!(grid.keyforms[5].opacity, 0.25, "derived source is not used");

    // もう一度実行しても変わらない。
    let before = model.clone();
    mirror_keyforms(
        &mut model,
        &target("Nose"),
        &target("Nose"),
        &MirrorOptions::default(),
    )
    .unwrap();
    assert_eq!(model, before);
}

#[test]
fn paired_parameters_map_to_the_same_key() {
    let mut model = eye_model();
    let left = mesh(&model, "EyeL").keyforms.keyforms[1].clone();
    let right = mesh(&model, "EyeR").keyforms.keyforms[1].clone();
    // EyeL は EyeLOpen、EyeR は EyeROpen の軸。
    set_grid(
        &mut mesh_mut(&mut model, "EyeL").keyforms,
        vec![binding("EyeLOpen", &[0.0, 1.0])],
        vec![shifted(&left, 0.0, 0.03), left],
        &[0, 1],
    );
    set_grid(
        &mut mesh_mut(&mut model, "EyeR").keyforms,
        vec![binding("EyeROpen", &[0.0, 1.0])],
        vec![right.clone(), right],
        &[1],
    );
    let report = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap();
    assert_eq!(report.written, vec![0]);
    // EyeROpen = 0 は EyeLOpen = 0 (0.03 下へ) を反転したもの。
    assert_points(
        &mesh(&model, "EyeR").keyforms.keyforms[0].positions,
        &[
            Vec2::new(0.6, 0.43),
            Vec2::new(0.8, 0.43),
            Vec2::new(0.8, 0.53),
            Vec2::new(0.6, 0.53),
        ],
    );
}

#[test]
fn self_mirrored_parameter_needs_the_negated_key() {
    let mut model = eye_model();
    let right = mesh(&model, "EyeR").keyforms.keyforms[1].clone();
    set_grid(
        &mut mesh_mut(&mut model, "EyeR").keyforms,
        vec![binding("AngleX", &[0.0, 15.0])],
        vec![right.clone(), right],
        &[0],
    );
    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        InferError::MissingMirrorKey {
            parameter: "AngleX".into(),
            key: -15.0,
        }
    );

    // 軸が 1 対 1 に対応しなければエラー。
    let mut model = eye_model();
    let right = mesh(&model, "EyeR").keyforms.keyforms[1].clone();
    set_grid(
        &mut mesh_mut(&mut model, "EyeR").keyforms,
        vec![binding("AngleY", &[0.0])],
        vec![right],
        &[0],
    );
    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        InferError::MirrorAxisMismatch {
            parameter: "AngleY".into()
        }
    );

    // 元の格子の軸が余ってもエラー (先の格子に AngleY の軸がない)。
    let mut model = eye_model();
    let left = mesh(&model, "EyeL").keyforms.keyforms[1].clone();
    set_grid(
        &mut mesh_mut(&mut model, "EyeL").keyforms,
        vec![
            binding("AngleX", &[-30.0, 0.0, 30.0]),
            binding("AngleY", &[0.0, 30.0]),
        ],
        vec![left; 6],
        &[0, 1, 2, 3, 4, 5],
    );
    let before = model.clone();
    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        InferError::MirrorAxisMismatch {
            parameter: "AngleY".into()
        }
    );
    assert_eq!(model, before);
}

#[test]
fn authored_target_cells_are_kept_unless_overwrite_is_set() {
    let mut model = eye_model();
    let marker = {
        let right = mesh_mut(&mut model, "EyeR");
        right.keyforms.origins[2] = KeyformOrigin::Authored;
        right.keyforms.keyforms[2].opacity = 0.125;
        right.keyforms.keyforms[2].clone()
    };
    let report = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("EyeR"),
        &MirrorOptions::default(),
    )
    .unwrap();
    assert_eq!(report.written, vec![0]);
    assert_eq!(mesh(&model, "EyeR").keyforms.keyforms[2], marker);

    let options = MirrorOptions {
        overwrite_authored: true,
        ..Default::default()
    };
    let report = mirror_keyforms(&mut model, &target("EyeL"), &target("EyeR"), &options).unwrap();
    assert_eq!(report.written, vec![0, 1, 2]);
    let right = &mesh(&model, "EyeR").keyforms;
    assert_eq!(right.keyforms[2].opacity, 1.0);
    assert_eq!(right.origins, vec![KeyformOrigin::Mirror; 3]);
}

#[test]
fn own_scope_rewrites_only_mirrored_cells() {
    let mut model = eye_model();
    {
        let right = &mut mesh_mut(&mut model, "EyeR").keyforms;
        // AngleX = -30 は加算合成で作ったセル、+30 はミラーで作ったセル。
        right.origins[0] = KeyformOrigin::Additive;
        right.origins[2] = KeyformOrigin::Mirror;
        right.keyforms[0].opacity = 0.25;
    }
    let options = MirrorOptions {
        axis: Some(0.5),
        tolerance: Some(0.01),
        scope: InferScope::Own,
        ..Default::default()
    };
    let report = mirror_keyforms(&mut model, &target("EyeL"), &target("EyeR"), &options).unwrap();
    assert_eq!(report.written, vec![2]);
    let right = &mesh(&model, "EyeR").keyforms;
    assert_eq!(right.keyforms[0].opacity, 0.25);
    assert_eq!(right.origins[0], KeyformOrigin::Additive);
    assert_eq!(
        right.mirror_source,
        Some(MirrorSource {
            id: "EyeL".into(),
            axis: Some(0.5),
            tolerance: Some(0.01),
        })
    );

    // 方法を記録していない `derived` のセルは格子の方法 (加算合成) のセルなので、`Own` では書かない。
    // 1 つも書かなければ、ミラー元の記録も変えない。
    let mut model = eye_model();
    let options = MirrorOptions {
        scope: InferScope::Own,
        ..Default::default()
    };
    let report = mirror_keyforms(&mut model, &target("EyeL"), &target("EyeR"), &options).unwrap();
    assert!(report.written.is_empty());
    assert_eq!(mesh(&model, "EyeR").keyforms.mirror_source, None);
}

#[test]
fn only_mirror_pairs_can_be_mirrored() {
    let mut model = eye_model();
    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &target("Nose"),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(err, InferError::NotMirrorPair { .. }), "{err:?}");
    let err = mirror_keyforms(
        &mut model,
        &target("EyeL"),
        &KeyformTarget::Deformer("Face".into()),
        &MirrorOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, InferError::IncompatibleMirror { .. }),
        "{err:?}"
    );
}

fn deformer_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Deformer {
    model
        .deformers
        .iter_mut()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

fn deformer(id: &str, mirror: &str, kind: DeformerKind) -> Deformer {
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: None,
        kind,
        mirror: Some(mirror.into()),
        mirror_axis: None,
    }
}

#[test]
fn warp_reflects_control_points_and_flips_columns() {
    let mut model = model();
    // 2 列 1 行 (制御点 3×2) のワープ。キャンバスの原点は x = 128 (既定の対称軸)。
    let points = |xs: [f32; 3]| {
        let mut p: Vec<Vec2> = xs.iter().map(|&x| Vec2::new(x, 10.0)).collect();
        p.extend(xs.iter().map(|&x| Vec2::new(x, 50.0)));
        p
    };
    let warp = |points: Vec<Vec2>, opacity: f32, authored: &[usize]| {
        let kf = WarpKeyform { points, opacity };
        let mut grid = KeyformGrid::constant(kf.clone());
        set_grid(
            &mut grid,
            vec![binding("AngleX", &[0.0, 30.0])],
            vec![kf.clone(), WarpKeyform { opacity, ..kf }],
            authored,
        );
        DeformerKind::Warp(WarpDeformer {
            rows: 1,
            cols: 2,
            keyforms: grid,
            depth: None,
        })
    };
    let mut left = warp(points([10.0, 30.0, 60.0]), 1.0, &[0, 1]);
    if let DeformerKind::Warp(w) = &mut left {
        // AngleX = 30 は左上の制御点だけ動かす。
        w.keyforms.keyforms[1].points[0] = Vec2::new(0.0, 5.0);
        w.keyforms.keyforms[1].opacity = 0.75;
        // 自分自身と対にした AngleX: 先の 30 は元の -30 に対応するので、元にも -30 を持たせる。
        w.keyforms.bindings[0].keys = vec![-30.0, 0.0];
        w.keyforms.keyforms.swap(0, 1);
    }
    model.deformers.push(deformer("WarpL", "WarpR", left));
    model.deformers.push(deformer(
        "WarpR",
        "WarpL",
        warp(points([196.0, 226.0, 246.0]), 1.0, &[0]),
    ));

    let report = mirror_keyforms(
        &mut model,
        &KeyformTarget::Deformer("WarpL".into()),
        &KeyformTarget::Deformer("WarpR".into()),
        &MirrorOptions::default(),
    )
    .unwrap();
    assert_eq!(report.written, vec![1]);
    let DeformerKind::Warp(right) = &deformer_mut(&mut model, "WarpR").kind else {
        unreachable!()
    };
    let kf = &right.keyforms.keyforms[1];
    // 行ごとに列の順を反転する: 元の (0,5) (30,10) (60,10) → (196,10) (226,10) (256,5)。
    assert_points(
        &kf.points,
        &[
            Vec2::new(196.0, 10.0),
            Vec2::new(226.0, 10.0),
            Vec2::new(256.0, 5.0),
            Vec2::new(196.0, 50.0),
            Vec2::new(226.0, 50.0),
            Vec2::new(246.0, 50.0),
        ],
    );
    assert_eq!(kf.opacity, 0.75);
}

#[test]
fn rotation_reflects_the_origin_and_negates_the_angle() {
    let mut model = model();
    let rot = |origin: Vec2, angle: f32| RotationKeyform {
        origin,
        angle,
        scale: 1.5,
        reflect_x: true,
        reflect_y: false,
        opacity: 1.0,
    };
    let kind = |keyforms: Vec<RotationKeyform>, authored: &[usize]| {
        let mut grid = KeyformGrid::constant(keyforms[0]);
        set_grid(
            &mut grid,
            vec![binding("AngleY", &[0.0, 30.0])],
            keyforms,
            authored,
        );
        DeformerKind::Rotation(RotationDeformer { keyforms: grid })
    };
    let mut left = deformer(
        "ArmL",
        "ArmR",
        kind(
            vec![
                rot(Vec2::new(100.0, 50.0), 0.0),
                rot(Vec2::new(90.0, 60.0), 15.0),
            ],
            &[0, 1],
        ),
    );
    // 対称軸を明示する (親なしの既定はキャンバスの原点の x)。
    left.mirror_axis = Some(120.0);
    model.deformers.push(left);
    model.deformers.push(deformer(
        "ArmR",
        "ArmL",
        kind(vec![rot(Vec2::new(140.0, 50.0), 0.0); 2], &[0]),
    ));
    mirror_keyforms(
        &mut model,
        &KeyformTarget::Deformer("ArmL".into()),
        &KeyformTarget::Deformer("ArmR".into()),
        &MirrorOptions::default(),
    )
    .unwrap();
    let DeformerKind::Rotation(right) = &deformer_mut(&mut model, "ArmR").kind else {
        unreachable!()
    };
    let kf = right.keyforms.keyforms[1];
    assert_points(&[kf.origin], &[Vec2::new(150.0, 60.0)]);
    assert_eq!(kf.angle, -15.0);
    assert_eq!((kf.scale, kf.reflect_x), (1.5, true));

    // `axis` を指定するとその実行に限り対称軸を上書きする。
    let options = MirrorOptions {
        axis: Some(100.0),
        ..Default::default()
    };
    mirror_keyforms(
        &mut model,
        &KeyformTarget::Deformer("ArmL".into()),
        &KeyformTarget::Deformer("ArmR".into()),
        &options,
    )
    .unwrap();
    let DeformerKind::Rotation(right) = &deformer_mut(&mut model, "ArmR").kind else {
        unreachable!()
    };
    assert_points(
        &[right.keyforms.keyforms[1].origin],
        &[Vec2::new(110.0, 60.0)],
    );
}
