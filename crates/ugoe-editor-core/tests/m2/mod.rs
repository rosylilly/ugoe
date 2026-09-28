//! M2 の受け入れテストの通しの手順 (T0616)。
//!
//! PSD の取り込みから深度からの回転までを、エディタと同じ `Command` だけで順に行い、
//! 各段階で検証と仕様 (`docs/spec/keyform-inference.md`) の式を確かめる。
//! `ugoe-cli` の GPU テスト (`tests/m2_render_gpu.rs`) も `#[path]` でこのファイルを読み込むので、
//! 両方のクレートが依存するクレートだけを使う (`Vec2` は `ugoe_editor_core::space` から)。
#![allow(dead_code)]

use std::path::PathBuf;

use ugoe_editor_core::space::Vec2;
use ugoe_editor_core::{
    ChildRef, Command, Document, InferMethod, Keyform, KeyformCell, KeyformTarget, Project,
    PsdImportOptions, Textures, import_psd,
};
use ugoe_model::deformer::{WarpDeformer, WarpKeyform};
use ugoe_model::validate::validate;
use ugoe_model::{
    Deformer, DeformerId, DeformerKind, DepthRotation, KeyformBinding, KeyformGrid,
    KeyformInference, KeyformOrigin, MirrorSource, Model, ParamId, Parameter,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

pub const ANGLE_X: &str = "AngleX";
pub const ANGLE_Y: &str = "AngleY";
pub const KEYS: [f32; 3] = [-30.0, 0.0, 30.0];

/// 顔のワープ (加算合成の対象)。子は `肌`・`影`・`目`。
pub const FACE: &str = "Face";
/// 顔を包む回転デフォーマ。
pub const HEAD: &str = "Head";
/// 左右の対にするワープ (`EyeL` がミラー元)。親は `Face`。
pub const EYE_L: &str = "EyeL";
pub const EYE_R: &str = "EyeR";
/// 深度を持つ振り向きのワープ (深度からの回転の対象)。`Head` と `Face` の間に入れる。
pub const FACE_TURN: &str = "FaceTurn";

/// 基準の姿の見た目の許容誤差 (キャンバスのピクセル)。
/// ワープへの付け替えはニュートン法で解く (残差 1e-3px 以下) ので、それより少し緩める。
const LOOK_TOLERANCE: f32 = 2e-3;
/// 推定したセルと仕様の式の許容誤差 (演算の順の違いと `f32` の三角関数の差を吸収する)。
const FORMULA_TOLERANCE: f32 = 1e-4;

/// 2 軸のセル (`AngleX`, `AngleY`)。
type Cell2 = (f32, f32);
/// 制御点の番号からずれを作る関数。
type PointDelta = fn(f32) -> Vec2;

/// 通しの手順を進めるドキュメントと、コマンドごとの状態の履歴 (Undo / Redo の検査用)。
#[derive(Debug)]
pub struct Workflow {
    pub doc: Document,
    /// `history[0]` は最初の状態、`history[i]` は `i` 個目のコマンドを適用した後の状態。
    pub history: Vec<(Model, Textures)>,
    /// 取り込み直後の、既定値のパラメータでの見た目 (メッシュごとの頂点位置)。
    pub rest_look: Vec<Vec<Vec2>>,
}

impl Workflow {
    fn new() -> Self {
        let doc = Project::empty("m2").document();
        let history = vec![(doc.model().clone(), doc.textures().clone())];
        Self {
            doc,
            history,
            rest_look: Vec::new(),
        }
    }

    pub fn model(&self) -> &Model {
        self.doc.model()
    }

    /// コマンドを適用し、警告がなく、結果が検証を通ることを確かめて履歴に積む。
    #[track_caller]
    pub fn apply(&mut self, command: Command) {
        let label = format!("{command:?}");
        let report = self
            .doc
            .apply_with_report(command)
            .unwrap_or_else(|e| panic!("command failed: {e}\n{label}"));
        assert_eq!(report.warnings, vec![], "unexpected warnings: {label}");
        assert_eq!(validate(self.doc.model()), vec![], "invalid after {label}");
        self.history
            .push((self.doc.model().clone(), self.doc.textures().clone()));
    }

    /// 既定値のパラメータでの見た目が取り込み直後と同じであること。
    #[track_caller]
    pub fn assert_rest_look_unchanged(&self, stage: &str) {
        assert_same_look(&self.rest_look, &evaluate(self.model(), &[]), stage);
    }
}

// ---- 手順 ----

/// 段階 1〜5 をすべて行う。
pub fn run() -> Workflow {
    let mut wf = Workflow::new();
    import_layers(&mut wf);
    create_deformers(&mut wf);
    additive_face(&mut wf);
    mirror_eyes(&mut wf);
    depth_turn(&mut wf);
    wf.assert_rest_look_unchanged("end of the workflow");
    wf
}

pub fn fixture_psd() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/layers.psd");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// 段階 1: `layers.psd` を空のモデルに取り込む。
fn import_layers(wf: &mut Workflow) {
    let import = import_psd(wf.model(), &fixture_psd(), &PsdImportOptions::new("layers"))
        .expect("import layers.psd");
    wf.apply(import.command);
    let model = wf.model();
    assert_eq!(model.meshes.len(), 6);
    assert_eq!(model.textures.len(), 6);
    assert_eq!(wf.doc.textures().len(), 6);
    assert!(model.deformers.is_empty());
    wf.rest_look = evaluate(model, &[]);
    for positions in &wf.rest_look {
        assert!(!positions.is_empty());
        assert!(positions.iter().all(|p| p.is_finite()));
    }
}

/// 段階 2: 顔を包むワープ、それを包む回転デフォーマ、目の左右のワープを作る。見た目は変わらない。
fn create_deformers(wf: &mut Workflow) {
    // `肌`・`影`・`目`
    wf.apply(Command::CreateWarpDeformer {
        id: Some(FACE.into()),
        name: "Face".into(),
        children: vec![mesh_ref("Layer5"), mesh_ref("Layer6"), mesh_ref("Layer7")],
        rows: 2,
        cols: 2,
        parent: None,
    });
    wf.assert_rest_look_unchanged("after creating the face warp");
    wf.apply(Command::CreateRotationDeformer {
        id: Some(HEAD.into()),
        name: "Head".into(),
        children: vec![ChildRef::Deformer(FACE.into())],
        origin: None,
        parent: None,
    });
    wf.assert_rest_look_unchanged("after creating the head rotation");
    for (id, mesh) in [(EYE_L, "Layer6"), (EYE_R, "Layer7")] {
        wf.apply(Command::CreateWarpDeformer {
            id: Some(id.into()),
            name: id.into(),
            children: vec![mesh_ref(mesh)],
            rows: 1,
            cols: 2,
            parent: None,
        });
    }
    wf.assert_rest_look_unchanged("after creating the eye warps");

    let model = wf.model();
    assert_eq!(
        deformer(model, FACE).parent.as_ref().map(|p| p.as_str()),
        Some(HEAD)
    );
    assert_eq!(deformer(model, HEAD).parent, None);
    assert!(matches!(
        deformer(model, HEAD).kind,
        DeformerKind::Rotation(_)
    ));
    // 目のワープの親は子の親 (顔のワープ)。
    for id in [EYE_L, EYE_R] {
        assert_eq!(
            deformer(model, id).parent.as_ref().map(|p| p.as_str()),
            Some(FACE)
        );
    }
}

/// 段階 3: `AngleX` / `AngleY` を作り、顔のワープに 3×3 のキーを置いて端のセルだけを編集し、
/// 斜めのセルを加算合成で推定する。
fn additive_face(wf: &mut Workflow) {
    for id in [ANGLE_X, ANGLE_Y] {
        wf.apply(Command::AddParameter {
            parameter: Parameter {
                id: id.into(),
                name: id.into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
                kind: Default::default(),
                mirror: None,
                group: None,
            },
        });
    }
    add_keys(wf, &deformer_target(FACE), &[ANGLE_X, ANGLE_Y]);
    wf.assert_rest_look_unchanged("after adding keys to the face warp");
    let grid = warp_grid(wf.model(), FACE).clone();
    assert_eq!(grid.keyforms.len(), 9);
    // キーを加えただけのセルは `authored` (格子に推定のセルがないため)。
    assert!((0..9).all(|i| grid.origin(i) == KeyformOrigin::Authored));

    // 斜めのセルを推定に戻す (いまは端のセルが基準と同じなので、基準の値になる)。
    let diagonals: Vec<KeyformCell> = [(-30.0, -30.0), (30.0, -30.0), (-30.0, 30.0), (30.0, 30.0)]
        .into_iter()
        .map(|(x, y)| cell(&[(ANGLE_X, x), (ANGLE_Y, y)]))
        .collect();
    wf.apply(Command::MarkKeyformDerived {
        target: deformer_target(FACE),
        cells: diagonals.clone(),
    });

    // 端のセル (1 軸セル) だけを手で作る: 点ごとに違うずれと、不透明度の変化。
    let base = warp_cell(wf.model(), FACE, &[(ANGLE_X, 0.0), (ANGLE_Y, 0.0)]).clone();
    let edges: [(Cell2, f32, PointDelta); 4] = [
        ((30.0, 0.0), 0.9, |i| {
            Vec2::new(4.0 + 0.1 * i, 0.25 * i - 1.0)
        }),
        ((-30.0, 0.0), 1.0, |i| Vec2::new(-3.5 - 0.05 * i, 0.2 * i)),
        ((0.0, 30.0), 0.8, |i| Vec2::new(0.3 * i - 1.2, -3.0)),
        ((0.0, -30.0), 1.0, |i| Vec2::new(-0.1 * i, 2.5 + 0.05 * i)),
    ];
    for ((x, y), opacity, delta) in edges {
        let points = shifted(&base.points, delta);
        wf.apply(Command::SetKeyformCell {
            target: deformer_target(FACE),
            cell: cell(&[(ANGLE_X, x), (ANGLE_Y, y)]),
            keyform: Keyform::Warp(WarpKeyform { points, opacity }),
        });
    }
    wf.apply(Command::InferKeyforms {
        target: deformer_target(FACE),
        method: InferMethod::Additive,
        overwrite_derived: false,
    });
    wf.assert_rest_look_unchanged("after additive inference");
    assert_face_additive(wf.model());
    let grid = warp_grid(wf.model(), FACE);
    let origin = |x, y| grid.origin(index_of(&grid.bindings, &[(ANGLE_X, x), (ANGLE_Y, y)]));
    assert_eq!(origin(0.0, 0.0), KeyformOrigin::Authored);
    for (x, y) in [(30.0, 0.0), (-30.0, 0.0), (0.0, 30.0), (0.0, -30.0)] {
        assert_eq!(origin(x, y), KeyformOrigin::Authored, "edge ({x}, {y})");
    }
    for (x, y) in [(-30.0, -30.0), (30.0, -30.0), (-30.0, 30.0), (30.0, 30.0)] {
        assert_eq!(origin(x, y), KeyformOrigin::Additive, "diagonal ({x}, {y})");
    }
    // 斜めの不透明度: 1 + (0.9 − 1) + (0.8 − 1)。
    let diagonal = warp_cell(wf.model(), FACE, &[(ANGLE_X, 30.0), (ANGLE_Y, 30.0)]);
    assert!(
        (diagonal.opacity - 0.7).abs() < 1e-6,
        "{}",
        diagonal.opacity
    );
}

/// 顔のワープの斜めのセルが `B + Δ_X + Δ_Y` (`docs/spec/keyform-inference.md#推定の式`) であること。
#[track_caller]
pub fn assert_face_additive(model: &Model) {
    let at = |x, y| warp_cell(model, FACE, &[(ANGLE_X, x), (ANGLE_Y, y)]);
    let base = at(0.0, 0.0);
    for x in [-30.0, 30.0] {
        for y in [-30.0, 30.0] {
            let (dx, dy, diagonal) = (at(x, 0.0), at(0.0, y), at(x, y));
            for (i, p) in diagonal.points.iter().enumerate() {
                let expected = base.points[i]
                    + (dx.points[i] - base.points[i])
                    + (dy.points[i] - base.points[i]);
                assert_close(*p, expected, &format!("Face ({x}, {y}) point {i}"));
            }
            let opacity =
                (base.opacity + (dx.opacity - base.opacity) + (dy.opacity - base.opacity))
                    .clamp(0.0, 1.0);
            assert!((diagonal.opacity - opacity).abs() < 1e-6);
        }
    }
}

/// 段階 4: 目の左右のワープを対にし、`EyeL` のキーフォームを `EyeR` へミラーする。
/// `EyeL` を編集すると `EyeR` が自動でミラーし直される。
fn mirror_eyes(wf: &mut Workflow) {
    // `AngleX` は自分自身と対 (値の符号を反転したキーに対応する)。
    wf.apply(Command::SetParameterMirror {
        id: ANGLE_X.into(),
        mirror: Some(ANGLE_X.into()),
    });
    wf.apply(Command::SetMirrorPair {
        target: deformer_target(EYE_L),
        mirror: Some(EYE_R.into()),
        axis: None,
    });
    let model = wf.model();
    assert_eq!(
        deformer(model, EYE_L).mirror.as_ref().map(|m| m.as_str()),
        Some(EYE_R)
    );
    assert_eq!(
        deformer(model, EYE_R).mirror.as_ref().map(|m| m.as_str()),
        Some(EYE_L)
    );

    for id in [EYE_L, EYE_R] {
        add_keys(wf, &deformer_target(id), &[ANGLE_X]);
    }
    // 元の端のセルを手で作る (UV 空間の小さなずれ)。
    let base = warp_cell(wf.model(), EYE_L, &[(ANGLE_X, 0.0)]).clone();
    for (x, sign) in [(30.0, 1.0), (-30.0, -1.0)] {
        let points = shifted(&base.points, |i| {
            Vec2::new(sign * (0.01 + 0.004 * i), 0.003 * i - 0.005)
        });
        wf.apply(Command::SetKeyformCell {
            target: deformer_target(EYE_L),
            cell: cell(&[(ANGLE_X, x)]),
            keyform: Keyform::Warp(WarpKeyform {
                points,
                opacity: 1.0,
            }),
        });
    }
    // 先の端のセルを推定に戻してからミラーする (基準のセルは `authored` のまま残る)。
    let right_base = warp_cell(wf.model(), EYE_R, &[(ANGLE_X, 0.0)]).clone();
    wf.apply(Command::MarkKeyformDerived {
        target: deformer_target(EYE_R),
        cells: vec![cell(&[(ANGLE_X, -30.0)]), cell(&[(ANGLE_X, 30.0)])],
    });
    wf.apply(Command::MirrorKeyforms {
        source: deformer_target(EYE_L),
        target: deformer_target(EYE_R),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    });
    wf.assert_rest_look_unchanged("after mirroring the eyes");
    assert_eyes_mirrored(wf.model(), &right_base);

    // 元を編集すると、先は同じ Undo 単位でミラーし直される (警告なし: `Workflow::apply`)。
    let edited = shifted(&base.points, |i| Vec2::new(0.02, -0.01 * i));
    wf.apply(Command::SetKeyformCell {
        target: deformer_target(EYE_L),
        cell: cell(&[(ANGLE_X, 30.0)]),
        keyform: Keyform::Warp(WarpKeyform {
            points: edited.clone(),
            opacity: 0.6,
        }),
    });
    assert_eyes_mirrored(wf.model(), &right_base);
    let right = warp_cell(wf.model(), EYE_R, &[(ANGLE_X, -30.0)]);
    assert_eq!(right.opacity, 0.6);
    assert_close(
        right.points[2],
        Vec2::new(2.0 * eye_axis(wf.model()) - edited[0].x, edited[0].y),
        "re-mirrored EyeR point",
    );
}

/// 元 (`EyeL`) の対称軸。省略時はワープの子なので UV 空間の 0.5。
fn eye_axis(model: &Model) -> f32 {
    deformer(model, EYE_L).mirror_axis.unwrap_or(0.5)
}

/// `EyeR` の `AngleX = k` のセルが `EyeL` の `AngleX = −k` のセルの反転
/// (`x' = 2a − x`、列を左右反転) で、由来が `mirror` であること。基準のセルは変わらない。
#[track_caller]
pub fn assert_eyes_mirrored(model: &Model, right_base: &WarpKeyform) {
    let a = eye_axis(model);
    let DeformerKind::Warp(warp) = &deformer(model, EYE_R).kind else {
        panic!("EyeR must be a warp");
    };
    let cols = warp.cols as usize;
    for k in [-30.0, 30.0] {
        let source = warp_cell(model, EYE_L, &[(ANGLE_X, -k)]);
        let target = warp_cell(model, EYE_R, &[(ANGLE_X, k)]);
        for (i, p) in target.points.iter().enumerate() {
            let (r, c) = (i / (cols + 1), i % (cols + 1));
            let s = source.points[r * (cols + 1) + (cols - c)];
            assert_close(
                *p,
                Vec2::new(2.0 * a - s.x, s.y),
                &format!("EyeR {k} point {i}"),
            );
        }
        assert_eq!(target.opacity, source.opacity);
    }
    let grid = warp_grid(model, EYE_R);
    let origin = |k| grid.origin(index_of(&grid.bindings, &[(ANGLE_X, k)]));
    assert_eq!(origin(-30.0), KeyformOrigin::Mirror);
    assert_eq!(origin(30.0), KeyformOrigin::Mirror);
    assert_eq!(origin(0.0), KeyformOrigin::Authored);
    assert_eq!(
        grid.mirror_source,
        Some(MirrorSource {
            id: EYE_L.into(),
            axis: None,
            tolerance: None,
        })
    );
    assert_eq!(warp_cell(model, EYE_R, &[(ANGLE_X, 0.0)]), right_base);
}

/// 段階 5: 深度を持つワープを `Head` と `Face` の間に入れ、深度からの回転で振り向きのセルを作る。
///
/// 既存の要素の `depth` を設定するコマンドはない (T0616 のメモ) ので、深度を持つワープを
/// `AddDeformer` で作り、`ReparentPreservingLook` で顔のワープをその下へ移す。
fn depth_turn(wf: &mut Workflow) {
    // 顔のワープの基準のセルの制御点 (Head の局所座標) を余白付きで覆う 3×3 の格子。
    let face = warp_cell(wf.model(), FACE, &[(ANGLE_X, 0.0), (ANGLE_Y, 0.0)]);
    let (min, max) = bounds(&face.points);
    let margin = 0.25 * (max - min).max_element();
    let (min, max) = (min - Vec2::splat(margin), max + Vec2::splat(margin));
    let size = max - min;
    let points: Vec<Vec2> = (0..3)
        .flat_map(|r| (0..3).map(move |c| min + size * Vec2::new(c as f32, r as f32) / 2.0))
        .collect();
    let depth = vec![0.0, 4.0, 0.0, 4.0, 12.0, 4.0, 0.0, 4.0, 0.0];
    let part = deformer(wf.model(), FACE).part.clone();
    wf.apply(Command::AddDeformer {
        deformer: Deformer {
            id: FACE_TURN.into(),
            name: "FaceTurn".into(),
            part,
            parent: Some(HEAD.into()),
            kind: DeformerKind::Warp(WarpDeformer {
                rows: 2,
                cols: 2,
                keyforms: KeyformGrid::constant(WarpKeyform {
                    points: points.clone(),
                    opacity: 1.0,
                }),
                depth: Some(depth.clone()),
            }),
            mirror: None,
            mirror_axis: None,
        },
    });
    wf.apply(Command::ReparentPreservingLook {
        element: ChildRef::Deformer(FACE.into()),
        new_parent: Some(FACE_TURN.into()),
    });
    wf.assert_rest_look_unchanged("after inserting the depth warp");
    // 顔のワープの格子はすべて写し直され、斜めのセルは作り直されている (写像はアフィン)。
    assert_face_additive(wf.model());

    let target = deformer_target(FACE_TURN);
    add_keys(wf, &target, &[ANGLE_X, ANGLE_Y]);
    let rotation = DepthRotation {
        angle_scale: 1.0,
        pivot: min + size / 2.0,
        pivot_depth: -8.0,
        yaw: Some(ANGLE_X.into()),
        pitch: Some(ANGLE_Y.into()),
    };
    wf.apply(Command::SetKeyformInference {
        target: target.clone(),
        inference: Some(KeyformInference::DepthRotation(rotation.clone())),
    });
    // 基準以外のセルを推定に戻すと、格子の方法 (深度からの回転) で作り直される。
    let others: Vec<KeyformCell> = KEYS
        .iter()
        .flat_map(|&y| KEYS.iter().map(move |&x| (x, y)))
        .filter(|&(x, y)| (x, y) != (0.0, 0.0))
        .map(|(x, y)| cell(&[(ANGLE_X, x), (ANGLE_Y, y)]))
        .collect();
    wf.apply(Command::MarkKeyformDerived {
        target: target.clone(),
        cells: others,
    });
    wf.apply(Command::InferKeyforms {
        target: target.clone(),
        method: InferMethod::DepthRotation,
        overwrite_derived: false,
    });
    assert_depth_rotation(wf.model(), &points, &depth, &rotation, &KEYS);

    // 中間のキー (`AngleX = 15`) を加える: 補間で埋まり (見た目は変わらない)、由来は `derived`。
    // 推定し直すと、回転の式どおり (線形補間とは違う) の値になる。
    wf.apply(Command::AddKeyformKey {
        target: target.clone(),
        parameter: ANGLE_X.into(),
        key: 15.0,
    });
    let grid = warp_grid(wf.model(), FACE_TURN);
    for y in KEYS {
        let index = index_of(&grid.bindings, &[(ANGLE_X, 15.0), (ANGLE_Y, y)]);
        assert_eq!(grid.origin(index), KeyformOrigin::Derived);
    }
    wf.apply(Command::InferKeyforms {
        target,
        method: InferMethod::FromGrid,
        overwrite_derived: false,
    });
    assert_depth_rotation(
        wf.model(),
        &points,
        &depth,
        &rotation,
        &[-30.0, 0.0, 15.0, 30.0],
    );
    wf.assert_rest_look_unchanged("after depth rotation");
}

/// 振り向きのワープのセルが `q = R_y(yaw)·R_x(pitch)·(p − o) + o` の `(q.x, q.y)`
/// (`docs/spec/keyform-inference.md#回転と投影`) で、基準以外の由来が `depth_rotation` であること。
#[track_caller]
pub fn assert_depth_rotation(
    model: &Model,
    base: &[Vec2],
    depth: &[f32],
    rotation: &DepthRotation,
    x_keys: &[f32],
) {
    let grid = warp_grid(model, FACE_TURN);
    for &y in &KEYS {
        for &x in x_keys {
            let index = index_of(&grid.bindings, &[(ANGLE_X, x), (ANGLE_Y, y)]);
            let expected_origin = if (x, y) == (0.0, 0.0) {
                KeyformOrigin::Authored
            } else {
                KeyformOrigin::DepthRotation
            };
            assert_eq!(grid.origin(index), expected_origin, "FaceTurn ({x}, {y})");
            let points = &grid.keyforms[index].points;
            for (i, p) in points.iter().enumerate() {
                let expected = rotate(base[i], depth[i], rotation, x, y);
                assert_close(*p, expected, &format!("FaceTurn ({x}, {y}) point {i}"));
            }
        }
    }
}

/// 仕様の式で点を回して正射影する (`f64` で計算する)。
pub fn rotate(p: Vec2, d: f32, rotation: &DepthRotation, yaw: f32, pitch: f32) -> Vec2 {
    let yaw = f64::from(yaw * rotation.angle_scale).to_radians();
    let pitch = f64::from(pitch * rotation.angle_scale).to_radians();
    let o = [
        f64::from(rotation.pivot.x),
        f64::from(rotation.pivot.y),
        f64::from(rotation.pivot_depth),
    ];
    let v = [
        f64::from(p.x) - o[0],
        f64::from(p.y) - o[1],
        f64::from(d) - o[2],
    ];
    // R_x(pitch)
    let v = [
        v[0],
        v[1] * pitch.cos() - v[2] * pitch.sin(),
        v[1] * pitch.sin() + v[2] * pitch.cos(),
    ];
    // R_y(yaw)
    let v = [v[0] * yaw.cos() + v[2] * yaw.sin(), v[1]];
    Vec2::new((v[0] + o[0]) as f32, (v[1] + o[1]) as f32)
}

// ---- 評価 ----

/// モデルをランタイムで評価し、メッシュごとのキャンバス座標の頂点位置を返す。
pub fn evaluate(model: &Model, values: &[(&str, f32)]) -> Vec<Vec<Vec2>> {
    let mut instance = instance(model, values);
    instance.update();
    instance
        .drawables()
        .iter()
        .map(|d| d.positions.clone())
        .collect()
}

/// パラメータ値を設定したインスタンス (まだ `update` していない)。
pub fn instance(model: &Model, values: &[(&str, f32)]) -> ModelInstance {
    let compiled = CompiledModel::compile(model.clone()).expect("compile model");
    let mut instance = ModelInstance::new(compiled);
    for &(id, value) in values {
        let index = instance
            .parameters()
            .index_of(id)
            .unwrap_or_else(|| panic!("parameter {id}"));
        instance.parameters_mut().set(index, value);
    }
    instance
}

#[track_caller]
pub fn assert_same_look(expected: &[Vec<Vec2>], actual: &[Vec<Vec2>], stage: &str) {
    assert_eq!(expected.len(), actual.len(), "{stage}: mesh count");
    for (m, (e, a)) in expected.iter().zip(actual).enumerate() {
        assert_eq!(e.len(), a.len(), "{stage}: mesh {m} vertex count");
        for (i, (e, a)) in e.iter().zip(a).enumerate() {
            assert!(
                (*e - *a).length() <= LOOK_TOLERANCE,
                "{stage}: mesh {m} vertex {i}: expected {e}, got {a}"
            );
        }
    }
}

// ---- 補助 ----

#[track_caller]
pub fn assert_close(actual: Vec2, expected: Vec2, what: &str) {
    let tolerance = FORMULA_TOLERANCE * expected.length().max(1.0);
    assert!(
        (actual - expected).length() <= tolerance,
        "{what}: expected {expected}, got {actual}"
    );
}

pub fn cell(entries: &[(&str, f32)]) -> KeyformCell {
    entries.iter().map(|&(p, v)| (ParamId::new(p), v)).collect()
}

fn mesh_ref(id: &str) -> ChildRef {
    ChildRef::Mesh(id.into())
}

pub fn deformer_target(id: &str) -> KeyformTarget {
    KeyformTarget::Deformer(DeformerId::new(id))
}

/// `parameters` の各軸にキー `KEYS` を加える (中立のキーを先に加えて軸を作る)。
fn add_keys(wf: &mut Workflow, target: &KeyformTarget, parameters: &[&str]) {
    for parameter in parameters {
        for key in [0.0, -30.0, 30.0] {
            wf.apply(Command::AddKeyformKey {
                target: target.clone(),
                parameter: (*parameter).into(),
                key,
            });
        }
    }
}

pub fn deformer<'a>(model: &'a Model, id: &str) -> &'a Deformer {
    model
        .deformers
        .iter()
        .find(|d| d.id.as_str() == id)
        .unwrap_or_else(|| panic!("deformer {id} not found"))
}

pub fn warp_grid<'a>(model: &'a Model, id: &str) -> &'a KeyformGrid<WarpKeyform> {
    match &deformer(model, id).kind {
        DeformerKind::Warp(warp) => &warp.keyforms,
        other => panic!("{id} must be a warp: {other:?}"),
    }
}

pub fn warp_cell<'a>(model: &'a Model, id: &str, cell: &[(&str, f32)]) -> &'a WarpKeyform {
    let grid = warp_grid(model, id);
    &grid.keyforms[index_of(&grid.bindings, cell)]
}

/// セルの線形インデックス (軸 0 が最も速く変化する。`KeyformGrid` の doc コメント)。
pub fn index_of(bindings: &[KeyformBinding], cell: &[(&str, f32)]) -> usize {
    assert_eq!(bindings.len(), cell.len(), "the cell must name every axis");
    let mut index = 0;
    let mut stride = 1;
    for binding in bindings {
        let (_, value) = cell
            .iter()
            .find(|(p, _)| *p == binding.parameter.as_str())
            .unwrap_or_else(|| panic!("axis {} missing", binding.parameter.as_str()));
        let i = binding
            .keys
            .iter()
            .position(|k| k == value)
            .unwrap_or_else(|| panic!("no key {value} on {}", binding.parameter.as_str()));
        index += i * stride;
        stride *= binding.keys.len();
    }
    index
}

fn shifted(points: &[Vec2], delta: impl Fn(f32) -> Vec2) -> Vec<Vec2> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| *p + delta(i as f32))
        .collect()
}

fn bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    let min = points
        .iter()
        .fold(Vec2::splat(f32::INFINITY), |a, p| a.min(*p));
    let max = points
        .iter()
        .fold(Vec2::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    (min, max)
}
