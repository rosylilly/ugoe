//! 深度からの回転のテスト (`docs/spec/keyform-inference.md#深度からの回転`)。

use ugoe_core::Vec2;
use ugoe_model::deformer::{RotationDeformer, RotationKeyform, WarpKeyform};
use ugoe_model::{
    Deformer, DeformerKind, DepthRotation, KeyformBinding, KeyformGrid, KeyformInference,
    KeyformOrigin, Model, Parameter, fixtures,
};
use ugoe_rigging::{InferError, InferScope, KeyformTarget, infer_depth_rotation};

const EPS: f32 = 1e-3;

fn face() -> KeyformTarget {
    KeyformTarget::Deformer("Face".into())
}

fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

fn warp_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut ugoe_model::deformer::WarpDeformer {
    let deformer = model
        .deformers
        .iter_mut()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("no {id}"));
    let DeformerKind::Warp(warp) = &mut deformer.kind else {
        panic!("not a warp")
    };
    warp
}

fn settings(model: &mut Model) -> &mut DepthRotation {
    let Some(KeyformInference::DepthRotation(s)) = &mut warp_mut(model, "Face").keyforms.inference
    else {
        panic!("no depth rotation")
    };
    s
}

/// フィクスチャの顔のワープ (制御点 (64,64) (192,64) (64,192) (192,192)、深度 0 0 16 16、
/// 中心 (128,128,0)) を、AngleX・AngleY とも 5 キーの格子にしたモデル。基準のセルだけが authored。
fn model() -> Model {
    let mut model = fixtures::keyform_inference();
    let keys = [-30.0, -15.0, 0.0, 15.0, 30.0];
    let grid = &mut warp_mut(&mut model, "Face").keyforms;
    let base = grid.keyforms[4].clone();
    grid.bindings = vec![binding("AngleX", &keys), binding("AngleY", &keys)];
    grid.keyforms = vec![base; 25];
    grid.origins = vec![KeyformOrigin::Derived; 25];
    grid.origins[12] = KeyformOrigin::Authored;
    model
}

/// 5×5 の格子で AngleX = `x`、AngleY = `y` のセル。
fn cell(x: f32, y: f32) -> usize {
    let i = |v: f32| ((v + 30.0) / 15.0) as usize;
    i(x) + 5 * i(y)
}

/// 回転行列を掛けて `R_y(yaw) · R_x(pitch) · (p − o) + o` を計算する (度)。
/// 右手系ではなく「x 右・y 下・z 手前」の座標で、正の yaw は手前を右へ、正の pitch は手前を上へ回す。
fn expected(p: [f32; 3], o: [f32; 3], yaw: f32, pitch: f32) -> Vec2 {
    let (a, b) = (yaw.to_radians(), pitch.to_radians());
    let ry = [
        [a.cos(), 0.0, a.sin()],
        [0.0, 1.0, 0.0],
        [-a.sin(), 0.0, a.cos()],
    ];
    let rx = [
        [1.0, 0.0, 0.0],
        [0.0, b.cos(), -b.sin()],
        [0.0, b.sin(), b.cos()],
    ];
    let mul = |m: [[f32; 3]; 3], v: [f32; 3]| {
        [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
    };
    let q = mul(ry, mul(rx, [p[0] - o[0], p[1] - o[1], p[2] - o[2]]));
    Vec2::new(q[0] + o[0], q[1] + o[1])
}

fn points(model: &mut Model, cell: usize) -> Vec<Vec2> {
    warp_mut(model, "Face").keyforms.keyforms[cell]
        .points
        .clone()
}

fn assert_points(actual: &[Vec2], expected: &[Vec2]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!(a.distance(*e) < EPS, "{actual:?} vs {expected:?}");
    }
}

const BASE: [[f32; 3]; 4] = [
    [64.0, 64.0, 0.0],
    [192.0, 64.0, 0.0],
    [64.0, 192.0, 16.0],
    [192.0, 192.0, 16.0],
];

fn rotated(yaw: f32, pitch: f32, o: [f32; 3]) -> Vec<Vec2> {
    BASE.iter().map(|&p| expected(p, o, yaw, pitch)).collect()
}

#[test]
fn every_cell_follows_the_rotation_formula() {
    let mut model = model();
    let report = infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();
    assert_eq!(report.written.len(), 24);
    let o = [128.0, 128.0, 0.0];
    for (x, y) in [
        (15.0, 0.0),   // 中間のキー
        (-30.0, 0.0),  // 端
        (0.0, -15.0),  // pitch だけ
        (30.0, 30.0),  // 斜め
        (-15.0, 30.0), // 中間と端の斜め
        (15.0, -15.0), // 中間どうしの斜め
    ] {
        assert_points(&points(&mut model, cell(x, y)), &rotated(x, y, o));
    }

    // 手で計算した値: 手前の角 (192,192,16) は yaw 30 だけなら x = 128 + 64 cos30 + 16 sin30 = 191.43。
    // (30, 30) では先に pitch 30 で y = 128 + 64 cos30 − 16 sin30 = 175.43、z = 64 sin30 + 16 cos30 = 45.86、
    // そのあと yaw 30 で x = 128 + 64 cos30 + 45.86 sin30 = 206.35。
    let p = points(&mut model, cell(30.0, 0.0))[3];
    assert_points(&[p], &[Vec2::new(191.426, 192.0)]);
    let p = points(&mut model, cell(30.0, 30.0))[3];
    assert_points(&[p], &[Vec2::new(206.354, 175.426)]);
    // 斜めでも、同じ高さ・同じ深度の点 (手前の下の辺) は同じ高さのまま (水平な線が傾かない)。
    let p = points(&mut model, cell(30.0, 30.0));
    assert!((p[2].y - p[3].y).abs() < EPS, "{p:?}");
    // 奥行き 0 の上の辺は pitch 30 で中心へ cos30 だけ縮み、手前の下の辺は上へずれる。
    let p = points(&mut model, cell(0.0, 30.0));
    assert_points(
        &[p[0]],
        &[Vec2::new(64.0, 128.0 - 64.0 * 30f32.to_radians().cos())],
    );
    assert!(
        p[2].y < 128.0 + 64.0 * 30f32.to_radians().cos(),
        "front moves up"
    );
}

#[test]
fn zero_depth_is_a_flat_card() {
    let mut model = model();
    warp_mut(&mut model, "Face").depth = Some(vec![0.0; 4]);
    infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();
    let (c30, c15) = (30f32.to_radians().cos(), 15f32.to_radians().cos());
    // 深度が 0 なら平らな板を回すだけ: yaw だけなら横、pitch だけなら縦に、中心に向かって cos 倍縮む。
    let square = |sx: f32, sy: f32| {
        [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
            .map(|(dx, dy)| Vec2::new(128.0 + 64.0 * dx * sx, 128.0 + 64.0 * dy * sy))
    };
    assert_points(&points(&mut model, cell(30.0, 0.0)), &square(c30, 1.0));
    assert_points(&points(&mut model, cell(0.0, -15.0)), &square(1.0, c15));
    // 斜めは pitch で傾いた板を yaw で回すので平行四辺形になる。水平な辺は水平のまま、
    // 縦の辺が傾く (pitch 30 で奥へ回った上の辺が、yaw -15 で右へずれる)。
    let p = points(&mut model, cell(-15.0, 30.0));
    let flat = [
        [64.0, 64.0, 0.0],
        [192.0, 64.0, 0.0],
        [64.0, 192.0, 0.0],
        [192.0, 192.0, 0.0],
    ];
    let expect: Vec<Vec2> = flat
        .iter()
        .map(|&q| expected(q, [128.0, 128.0, 0.0], -15.0, 30.0))
        .collect();
    assert_points(&p, &expect);
    assert!(
        (p[1] - p[0]).y.abs() < EPS && (p[1] - p[0]).x > 0.0,
        "{p:?}"
    );
    assert!((p[2] - p[0]).x < 0.0, "{p:?}");
    assert_points(&[p[1] - p[0]], &[p[3] - p[2]]);
    // 板の中心は動かない。
    let center = p.iter().copied().sum::<Vec2>() / 4.0;
    assert_points(&[center], &[Vec2::new(128.0, 128.0)]);
}

#[test]
fn pivot_position_and_depth_are_honoured() {
    let mut model = model();
    {
        let s = settings(&mut model);
        s.pivot = Vec2::new(64.0, 64.0);
        s.pivot_depth = 16.0;
        s.angle_scale = 0.5;
    }
    infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();
    let o = [64.0, 64.0, 16.0];
    // angle_scale 0.5: AngleX = 30 は 15 度。
    let p = points(&mut model, cell(30.0, -30.0));
    assert_points(&p, &rotated(15.0, -15.0, o));
    // 中心と同じ位置・深度の点は動かない。中心より奥の点 (深度 0 < 16) は yaw で左へ回る。
    assert!(p[0].x < 64.0, "{p:?}");
    let mut model = self::model();
    settings(&mut model).pivot_depth = 16.0;
    warp_mut(&mut model, "Face").depth = Some(vec![16.0; 4]);
    settings(&mut model).pivot = Vec2::new(192.0, 192.0);
    infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();
    for c in 0..25 {
        assert_points(&points(&mut model, c)[3..], &[Vec2::new(192.0, 192.0)]);
    }
}

#[test]
fn other_axes_and_authored_corrections_are_added() {
    let mut model = model();
    model.parameters.push(Parameter {
        id: "Smile".into(),
        name: "Smile".into(),
        min: 0.0,
        max: 1.0,
        default: 0.0,
        kind: Default::default(),
        mirror: None,
        group: None,
        internal: false,
    });
    let grid = &mut warp_mut(&mut model, "Face").keyforms;
    let base = grid.keyforms[12].clone();
    grid.bindings.push(binding("Smile", &[0.0, 1.0]));
    grid.keyforms = vec![base.clone(); 50];
    grid.origins = vec![KeyformOrigin::Derived; 50];
    let shift = |kf: &WarpKeyform, d: Vec2| WarpKeyform {
        points: kf.points.iter().map(|&p| p + d).collect(),
        opacity: kf.opacity,
    };
    // 基準・Smile = 1 (全体が 4 下がる)・AngleX = 30 を手で直したもの (全体が 2 右へ)。
    let x30 = cell(30.0, 0.0);
    let fixed = {
        let r = rotated(30.0, 0.0, [128.0, 128.0, 0.0]);
        WarpKeyform {
            points: r.iter().map(|&p| p + Vec2::new(2.0, 0.0)).collect(),
            opacity: 1.0,
        }
    };
    for (i, kf) in [
        (12, base.clone()),
        (12 + 25, shift(&base, Vec2::new(0.0, 4.0))),
        (x30, fixed.clone()),
    ] {
        grid.keyforms[i] = kf;
        grid.origins[i] = KeyformOrigin::Authored;
    }
    infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();

    let o = [128.0, 128.0, 0.0];
    let plus = |v: Vec<Vec2>, d: Vec2| v.into_iter().map(|p| p + d).collect::<Vec<_>>();
    // 手で直したセルは変えない。
    assert_eq!(points(&mut model, x30), fixed.points);
    // 斜め (30, 30) には AngleX = 30 の修正 (+2) が入る。
    assert_points(
        &points(&mut model, cell(30.0, 30.0)),
        &plus(rotated(30.0, 30.0, o), Vec2::new(2.0, 0.0)),
    );
    // 修正は加算合成と同じく軸の上で補間・外挿される: 15 は +1、-30 は -2。
    assert_points(
        &points(&mut model, cell(15.0, 0.0)),
        &plus(rotated(15.0, 0.0, o), Vec2::new(1.0, 0.0)),
    );
    assert_points(
        &points(&mut model, cell(-30.0, 0.0)),
        &plus(rotated(-30.0, 0.0, o), Vec2::new(-2.0, 0.0)),
    );
    // 回さない軸 (Smile) の変化は足し合わさる。
    assert_points(
        &points(&mut model, 25 + cell(0.0, -15.0)),
        &plus(rotated(0.0, -15.0, o), Vec2::new(0.0, 4.0)),
    );
    assert_points(
        &points(&mut model, 25 + cell(30.0, 30.0)),
        &plus(rotated(30.0, 30.0, o), Vec2::new(2.0, 4.0)),
    );
}

#[test]
fn mesh_needs_an_affine_parent() {
    let mut model = model();
    let eye = KeyformTarget::Mesh("EyeL".into());
    {
        let mesh = model.meshes.iter_mut().find(|m| m.id == "EyeL").unwrap();
        let base = mesh.keyforms.keyforms[0].clone();
        mesh.keyforms = KeyformGrid::new(vec![binding("AngleX", &[0.0, 30.0])], vec![base; 2]);
        mesh.keyforms.origins = vec![KeyformOrigin::Authored, KeyformOrigin::Derived];
        mesh.keyforms.inference = Some(KeyformInference::DepthRotation(DepthRotation {
            angle_scale: 1.0,
            pivot: Vec2::new(0.0, 0.0),
            pivot_depth: 0.0,
            yaw: Some("AngleX".into()),
            pitch: None,
        }));
    }
    // 親が顔のワープ (UV 空間) なのでエラー。
    let before = model.clone();
    assert_eq!(
        infer_depth_rotation(&mut model, &eye, InferScope::AllDerived),
        Err(InferError::NonAffineParent(eye.clone()))
    );
    assert_eq!(model, before);

    // 親を拡大率 2 の回転デフォーマにすると回せる。局所座標の 1 単位は 2px なので、深度 8px は 4。
    model.deformers.push(Deformer {
        id: "Head".into(),
        name: "Head".into(),
        part: "Root".into(),
        parent: None,
        kind: DeformerKind::Rotation(RotationDeformer {
            keyforms: KeyformGrid::constant(RotationKeyform {
                origin: Vec2::new(128.0, 128.0),
                angle: 0.0,
                scale: 2.0,
                reflect_x: false,
                reflect_y: false,
                opacity: 1.0,
            }),
        }),
        mirror: None,
        mirror_axis: None,
    });
    let mesh = model.meshes.iter_mut().find(|m| m.id == "EyeL").unwrap();
    mesh.parent = Some("Head".into());
    let base = mesh.keyforms.keyforms[0].positions.clone();
    infer_depth_rotation(&mut model, &eye, InferScope::AllDerived).unwrap();
    let mesh = model.meshes.iter().find(|m| m.id == "EyeL").unwrap();
    let expected: Vec<Vec2> = base
        .iter()
        .map(|p| expected([p.x, p.y, 4.0], [0.0, 0.0, 0.0], 30.0, 0.0))
        .collect();
    assert_points(&mesh.keyforms.keyforms[1].positions, &expected);
}

#[test]
fn configuration_errors() {
    let mut model = model();
    warp_mut(&mut model, "Face").keyforms.inference = None;
    assert_eq!(
        infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
        Err(InferError::NotDepthRotation(face()))
    );

    let mut model = self::model();
    settings(&mut model).pitch = Some("EyeOpen".into());
    assert_eq!(
        infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
        Err(InferError::MissingRotationAxis {
            parameter: "EyeOpen".into()
        })
    );

    let mut model = self::model();
    warp_mut(&mut model, "Face").depth = None;
    assert_eq!(
        infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
        Err(InferError::MissingDepth(face()))
    );
    warp_mut(&mut model, "Face").depth = Some(vec![0.0; 3]);
    assert_eq!(
        infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
        Err(InferError::DepthLengthMismatch {
            expected: 4,
            found: 3
        })
    );

    let mut model = self::model();
    assert_eq!(
        infer_depth_rotation(
            &mut model,
            &KeyformTarget::Part("Root".into()),
            InferScope::AllDerived
        ),
        Err(InferError::UnsupportedTarget(KeyformTarget::Part(
            "Root".into()
        )))
    );
}

/// 書いたセルの由来は `depth_rotation` になる。`Own` では別の方法 (`additive`) のセルを書き換えず、
/// 手で直したセルと同じく推定の元にする (T0614、`docs/spec/keyform-inference.md#方法ごとの書き換えの範囲`)。
#[test]
fn own_scope_records_the_method_and_keeps_other_methods_as_sources() {
    let mut model = model();
    let o = [128.0, 128.0, 0.0];
    let x30 = cell(30.0, 0.0);
    let other = WarpKeyform {
        points: rotated(30.0, 0.0, o)
            .into_iter()
            .map(|p| p + Vec2::new(2.0, 0.0))
            .collect(),
        opacity: 1.0,
    };
    {
        let grid = &mut warp_mut(&mut model, "Face").keyforms;
        grid.keyforms[x30] = other.clone();
        grid.origins[x30] = KeyformOrigin::Additive;
    }
    let report = infer_depth_rotation(&mut model, &face(), InferScope::Own).unwrap();
    assert_eq!(report.written.len(), 23);
    assert!(!report.written.contains(&x30));
    let grid = &warp_mut(&mut model, "Face").keyforms;
    for &i in &report.written {
        assert_eq!(grid.origins[i], KeyformOrigin::DepthRotation, "cell {i}");
    }
    assert_eq!(grid.origins[x30], KeyformOrigin::Additive);
    assert_eq!(grid.keyforms[x30], other);
    // `additive` のセルの回転の式からのずれ (+2) が、軸の上で外挿される。
    assert_points(
        &points(&mut model, cell(-30.0, 0.0)),
        &rotated(-30.0, 0.0, o)
            .into_iter()
            .map(|p| p + Vec2::new(-2.0, 0.0))
            .collect::<Vec<_>>(),
    );

    // `AllDerived` では `additive` のセルも回転の式どおりに書き換える。
    let report = infer_depth_rotation(&mut model, &face(), InferScope::AllDerived).unwrap();
    assert_eq!(report.written.len(), 24);
    assert_points(&points(&mut model, x30), &rotated(30.0, 0.0, o));
    assert_eq!(
        warp_mut(&mut model, "Face").keyforms.origins[x30],
        KeyformOrigin::DepthRotation
    );
}

/// 親をたどれない (親がない・循環する) 要素はエラーで、モデルを変えない。
#[test]
fn broken_parent_chains_are_errors() {
    let mut model = model();
    let head = |id: &str, parent: &str| Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: Some(parent.into()),
        kind: DeformerKind::Rotation(RotationDeformer {
            keyforms: KeyformGrid::constant(RotationKeyform {
                origin: Vec2::ZERO,
                angle: 0.0,
                scale: 1.0,
                reflect_x: false,
                reflect_y: false,
                opacity: 1.0,
            }),
        }),
        mirror: None,
        mirror_axis: None,
    };
    let face_parent = |model: &mut Model, parent: &str| {
        model
            .deformers
            .iter_mut()
            .find(|d| d.id == "Face")
            .unwrap()
            .parent = Some(parent.into());
    };

    face_parent(&mut model, "Missing");
    let before = model.clone();
    assert_eq!(
        infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
        Err(InferError::UnknownParent {
            deformer: "Missing".into()
        })
    );
    assert_eq!(model, before);

    model.deformers.push(head("A", "B"));
    model.deformers.push(head("B", "A"));
    face_parent(&mut model, "A");
    let before = model.clone();
    assert!(
        matches!(
            infer_depth_rotation(&mut model, &face(), InferScope::AllDerived),
            Err(InferError::ParentCycle { .. })
        ),
        "a cycle through A and B"
    );
    assert_eq!(model, before);
}
