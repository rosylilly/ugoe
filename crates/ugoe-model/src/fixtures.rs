//! テスト・サンプル用の小さなモデル。下流クレートのテストはここを共通の入力に使う。
//! 新しい機能をテストしたいときは、ここに関数を追加する (既存フィクスチャの挙動は変えない)。

use ugoe_core::{Rgb, Vec2};

use crate::deformer::{
    Deformer, DeformerKind, RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform,
};
use crate::keyform::{DepthRotation, KeyformBinding, KeyformGrid, KeyformInference, KeyformOrigin};
use crate::mesh::{BlendMode, Glue, GlueKeyform, GluePair, MaskMode, MaskRef, Mesh, MeshKeyform};
use crate::parameter::{Parameter, ParameterKind};
use crate::part::{Part, PartKeyform};
use crate::pose::{DEFAULT_FADE_DURATION, PoseEntry, PoseGroup};
use crate::standard;
use crate::{Canvas, Effects, Model, ModelMeta, Pose, SCHEMA_VERSION, TextureRef};

/// 256×256 キャンバスに四角形メッシュ 1 枚。
/// パラメータ `X` (-1..1) で四角形が左右に 64px 平行移動する。
pub fn single_quad() -> Model {
    let quad = |dx: f32| MeshKeyform {
        positions: vec![
            Vec2::new(64.0 + dx, 64.0),
            Vec2::new(192.0 + dx, 64.0),
            Vec2::new(192.0 + dx, 192.0),
            Vec2::new(64.0 + dx, 192.0),
        ],
        opacity: 1.0,
        draw_order: 500.0,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
        mask_strength: 1.0,
    };
    Model {
        schema_version: SCHEMA_VERSION,
        meta: ModelMeta {
            name: "single_quad".into(),
            canvas: Canvas {
                width: 256.0,
                height: 256.0,
                origin: Vec2::new(128.0, 128.0),
            },
        },
        parameters: vec![Parameter {
            id: "X".into(),
            name: "X".into(),
            min: -1.0,
            max: 1.0,
            default: 0.0,
            kind: ParameterKind::Normal,
            mirror: None,
            group: None,
        }],
        parameter_groups: vec![],
        parts: vec![Part {
            id: "Root".into(),
            name: "Root".into(),
            parent: None,
            keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        }],
        deformers: vec![],
        meshes: vec![Mesh {
            id: "Quad".into(),
            name: "Quad".into(),
            part: "Root".into(),
            parent: None,
            texture: 0,
            uvs: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            blend: BlendMode::Normal,
            culling: false,
            masks: vec![],
            mask_mode: MaskMode::Union,
            mask_only: false,
            keyforms: KeyformGrid::new(
                vec![KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![-1.0, 0.0, 1.0],
                }],
                vec![quad(-64.0), quad(0.0), quad(64.0)],
            ),
            mirror: None,
            mirror_axis: None,
            depth: None,
        }],
        glues: vec![],
        textures: vec![TextureRef {
            name: "white".into(),
            width: 1,
            height: 1,
        }],
        hit_areas: vec![],
        pose: Pose::default(),
        effects: Effects::default(),
    }
}

/// `single_quad` に、全標準パラメータ・ポーズ・標準のエフェクト設定を加えたもの。
///
/// パーツ `ArmA` (袖 `SleeveA` が追従) と `ArmB` が 1 つのポーズグループを作る。
/// `ArmA` と `SleeveA` の不透明度は 1、`ArmB` は 0。
pub fn pose_and_effects() -> Model {
    let mut model = single_quad();
    model.meta.name = "pose_and_effects".into();
    model.parameters.extend(
        standard::ALL
            .iter()
            .map(|&(id, min, default, max)| Parameter {
                id: id.into(),
                name: id.into(),
                min,
                max,
                default,
                kind: ParameterKind::Normal,
                mirror: None,
                group: None,
            }),
    );
    let part = |id: &str, opacity: f32| Part {
        id: id.into(),
        name: id.into(),
        parent: Some("Root".into()),
        keyforms: KeyformGrid::constant(PartKeyform { opacity }),
    };
    model
        .parts
        .extend([part("ArmA", 1.0), part("SleeveA", 1.0), part("ArmB", 0.0)]);
    model.pose = Pose {
        fade_duration: DEFAULT_FADE_DURATION,
        groups: vec![PoseGroup {
            entries: vec![
                PoseEntry {
                    part: "ArmA".into(),
                    links: vec!["SleeveA".into()],
                },
                PoseEntry {
                    part: "ArmB".into(),
                    links: vec![],
                },
            ],
        }],
    };
    model.effects = Effects::standard();
    model
}

// ---- デフォーマ・パーツ階層・グルーのフィクスチャ (T0104) ----
//
// どれも 256×256 キャンバス、パーツ `Root` を持つ。期待値の計算は各関数の doc コメントにある。

fn parameter(id: &str, min: f32, default: f32, max: f32) -> Parameter {
    Parameter {
        id: id.into(),
        name: id.into(),
        min,
        max,
        default,
        kind: ParameterKind::Normal,
        mirror: None,
        group: None,
    }
}

fn part(id: &str, parent: Option<&str>, keyforms: KeyformGrid<PartKeyform>) -> Part {
    Part {
        id: id.into(),
        name: id.into(),
        parent: parent.map(Into::into),
        keyforms,
    }
}

fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

/// 1 軸 `parameter` のキー `keys` にキーフォーム `keyforms` を並べた格子。
fn grid1<T>(parameter: &str, keys: &[f32], keyforms: Vec<T>) -> KeyformGrid<T> {
    KeyformGrid::new(vec![binding(parameter, keys)], keyforms)
}

fn mesh_keyform(positions: [Vec2; 4], opacity: f32, draw_order: f32) -> MeshKeyform {
    MeshKeyform {
        positions: positions.to_vec(),
        opacity,
        draw_order,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
        mask_strength: 1.0,
    }
}

/// 四角形 (頂点 4 つ、三角形 2 枚) のメッシュ。
fn quad_mesh(
    id: &str,
    part: &str,
    parent: Option<&str>,
    keyforms: KeyformGrid<MeshKeyform>,
) -> Mesh {
    Mesh {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: parent.map(Into::into),
        texture: 0,
        uvs: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        blend: BlendMode::Normal,
        culling: false,
        masks: vec![],
        mask_mode: MaskMode::Union,
        mask_only: false,
        keyforms,
        mirror: None,
        mirror_axis: None,
        depth: None,
    }
}

/// 左上 `(x0, y0)`、右下 `(x1, y1)` の四角形の頂点 (時計回り)。
fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> [Vec2; 4] {
    [
        Vec2::new(x0, y0),
        Vec2::new(x1, y0),
        Vec2::new(x1, y1),
        Vec2::new(x0, y1),
    ]
}

fn warp(
    id: &str,
    parent: Option<&str>,
    cols: u16,
    rows: u16,
    keyforms: KeyformGrid<WarpKeyform>,
) -> Deformer {
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Warp(WarpDeformer {
            rows,
            cols,
            keyforms,
            depth: None,
        }),
        mirror: None,
        mirror_axis: None,
    }
}

fn rotation(id: &str, parent: Option<&str>, keyforms: KeyformGrid<RotationKeyform>) -> Deformer {
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Rotation(RotationDeformer { keyforms }),
        mirror: None,
        mirror_axis: None,
    }
}

fn rotation_keyform(origin: Vec2, angle: f32, scale: f32) -> RotationKeyform {
    RotationKeyform {
        origin,
        angle,
        scale,
        reflect_x: false,
        reflect_y: false,
        opacity: 1.0,
    }
}

/// 格子の制御点 (行優先、x が速い) を `f(x, y)` で作る。
fn warp_points(cols: u16, rows: u16, f: impl Fn(u16, u16) -> Vec2) -> Vec<Vec2> {
    (0..=rows)
        .flat_map(|y| (0..=cols).map(move |x| (x, y)))
        .map(|(x, y)| f(x, y))
        .collect()
}

fn base_model(name: &str, parameters: Vec<Parameter>) -> Model {
    Model {
        schema_version: SCHEMA_VERSION,
        meta: ModelMeta {
            name: name.into(),
            canvas: Canvas {
                width: 256.0,
                height: 256.0,
                origin: Vec2::new(128.0, 128.0),
            },
        },
        parameters,
        parameter_groups: vec![],
        parts: vec![part(
            "Root",
            None,
            KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        )],
        deformers: vec![],
        meshes: vec![],
        glues: vec![],
        textures: vec![TextureRef {
            name: "white".into(),
            width: 1,
            height: 1,
        }],
        hit_areas: vec![],
        pose: Pose::default(),
        effects: Effects::default(),
    }
}

/// ワープデフォーマ 1 つ (2×2 セル) と、その子のメッシュ 2 枚。
///
/// - パラメータ `X` (-1..1、既定 0)。
/// - ワープ `Warp` の制御点:
///   - `X = 0`: `(64,64)`〜`(192,192)` の正方格子。写像は `F(u,v) = (64 + 128u, 64 + 128v)`。
///   - `X = 1`: 上の行ほど右へずらしたせん断 (上 +64、中 +32、下 0)。
///     写像はアフィン `F(u,v) = (128 + 128u - 64v, 64 + 128v)`。
///   - `X = -1`: 上の行だけ幅を 2 倍にした台形 (x = 0, 128, 256)。アフィンではない。
/// - メッシュ `Inner`: UV `(0.25,0.25)`〜`(0.75,0.75)` (格子の内側)。
/// - メッシュ `Outer`: UV `(-0.5,-0.5)`〜`(1.5,1.5)` (4 頂点とも格子の外側)。
pub fn warp_grid() -> Model {
    let mut model = base_model("warp_grid", vec![parameter("X", -1.0, 0.0, 1.0)]);
    let keyform = |f: &dyn Fn(u16, u16) -> Vec2| WarpKeyform {
        points: warp_points(2, 2, f),
        opacity: 1.0,
    };
    let square = |x: u16, y: u16| Vec2::new(64.0 + 64.0 * f32::from(x), 64.0 + 64.0 * f32::from(y));
    let keystone = |x: u16, y: u16| {
        if y == 0 {
            Vec2::new(128.0 * f32::from(x), 64.0)
        } else {
            square(x, y)
        }
    };
    let shear = |x: u16, y: u16| square(x, y) + Vec2::new(32.0 * f32::from(2 - y), 0.0);
    model.deformers.push(warp(
        "Warp",
        None,
        2,
        2,
        grid1(
            "X",
            &[-1.0, 0.0, 1.0],
            vec![keyform(&keystone), keyform(&square), keyform(&shear)],
        ),
    ));
    model.meshes.push(quad_mesh(
        "Inner",
        "Root",
        Some("Warp"),
        KeyformGrid::constant(mesh_keyform(rect(0.25, 0.25, 0.75, 0.75), 1.0, 500.0)),
    ));
    model.meshes.push(quad_mesh(
        "Outer",
        "Root",
        Some("Warp"),
        KeyformGrid::constant(mesh_keyform(rect(-0.5, -0.5, 1.5, 1.5), 1.0, 400.0)),
    ));
    model
}

/// 入れ子の回転デフォーマと、回転デフォーマの子のワープ。
///
/// - パラメータ `Angle` (-30..30、既定 0) と `Flip` (0..1、既定 0)。
/// - 回転 `Arm` (親なし): 原点 `(128,128)`、角度 = `Angle` 度 (キー -30, 30)、拡大率 1。
/// - 回転 `Hand` (親 `Arm`): 原点 `(64,0)`、角度 0、拡大率 0.5。`Flip = 1` のキーフォームだけ
///   `reflect_x = true` (キー 0, 1)。
/// - メッシュ `Palm` (親 `Hand`): `(0,0)`〜`(16,16)`。
/// - ワープ `Cloth` (親 `Arm`、1×1 セル): 制御点は `Arm` の局所座標で `(0,20)`〜`(40,40)`。
/// - メッシュ `Sleeve` (親 `Cloth`): UV `(0.25,0.25)`〜`(0.75,0.75)`。
pub fn rotation_chain() -> Model {
    let mut model = base_model(
        "rotation_chain",
        vec![
            parameter("Angle", -30.0, 0.0, 30.0),
            parameter("Flip", 0.0, 0.0, 1.0),
        ],
    );
    let arm = |angle| rotation_keyform(Vec2::new(128.0, 128.0), angle, 1.0);
    let hand = |reflect_x| RotationKeyform {
        reflect_x,
        ..rotation_keyform(Vec2::new(64.0, 0.0), 0.0, 0.5)
    };
    model.deformers.extend([
        rotation(
            "Arm",
            None,
            grid1("Angle", &[-30.0, 30.0], vec![arm(-30.0), arm(30.0)]),
        ),
        rotation(
            "Hand",
            Some("Arm"),
            grid1("Flip", &[0.0, 1.0], vec![hand(false), hand(true)]),
        ),
        warp(
            "Cloth",
            Some("Arm"),
            1,
            1,
            KeyformGrid::constant(WarpKeyform {
                points: vec![
                    Vec2::new(0.0, 20.0),
                    Vec2::new(40.0, 20.0),
                    Vec2::new(0.0, 40.0),
                    Vec2::new(40.0, 40.0),
                ],
                opacity: 1.0,
            }),
        ),
    ]);
    model.meshes.extend([
        quad_mesh(
            "Palm",
            "Root",
            Some("Hand"),
            KeyformGrid::constant(mesh_keyform(rect(0.0, 0.0, 16.0, 16.0), 1.0, 500.0)),
        ),
        quad_mesh(
            "Sleeve",
            "Root",
            Some("Cloth"),
            KeyformGrid::constant(mesh_keyform(rect(0.25, 0.25, 0.75, 0.75), 1.0, 500.0)),
        ),
    ]);
    model
}

/// ワープの子の回転デフォーマと、ワープの子のワープ。
///
/// - パラメータ `Turn` (0..1、既定 0)。
/// - ワープ `Body` (親なし、1×1 セル):
///   - `Turn = 0`: `(64,64)`〜`(192,192)` の正方形。`F(u,v) = (64 + 128u, 64 + 128v)`。
///   - `Turn = 1`: 同じ正方形を中心 `(128,128)` まわりに時計回りに 90 度回したもの
///     (`P00 = (192,64)`、`P10 = (192,192)`、`P01 = (64,64)`、`P11 = (64,192)`)。
///   - `Turn = 0.5`: 制御点の線形補間なので、45 度回って 1/√2 に縮んだ菱形になる。
/// - 回転 `Head` (親 `Body`): 原点 UV `(0.5,0.25)`、角度 0、拡大率 1。
/// - メッシュ `Face` (親 `Head`): `(0,-10)`〜`(10,0)` (ピクセル単位)。
/// - ワープ `Cheek` (親 `Body`、1×1 セル): 制御点は `Body` の UV で `(0.25,0.5)`〜`(0.75,1.0)`。
/// - メッシュ `Blush` (親 `Cheek`): UV `(0.5,0.5)` に縮退した四角形。
pub fn rotation_under_warp() -> Model {
    let mut model = base_model(
        "rotation_under_warp",
        vec![parameter("Turn", 0.0, 0.0, 1.0)],
    );
    let body = |points: [Vec2; 4]| WarpKeyform {
        points: points.to_vec(),
        opacity: 1.0,
    };
    model.deformers.extend([
        warp(
            "Body",
            None,
            1,
            1,
            grid1(
                "Turn",
                &[0.0, 1.0],
                vec![
                    body([
                        Vec2::new(64.0, 64.0),
                        Vec2::new(192.0, 64.0),
                        Vec2::new(64.0, 192.0),
                        Vec2::new(192.0, 192.0),
                    ]),
                    body([
                        Vec2::new(192.0, 64.0),
                        Vec2::new(192.0, 192.0),
                        Vec2::new(64.0, 64.0),
                        Vec2::new(64.0, 192.0),
                    ]),
                ],
            ),
        ),
        rotation(
            "Head",
            Some("Body"),
            KeyformGrid::constant(rotation_keyform(Vec2::new(0.5, 0.25), 0.0, 1.0)),
        ),
        warp(
            "Cheek",
            Some("Body"),
            1,
            1,
            KeyformGrid::constant(body([
                Vec2::new(0.25, 0.5),
                Vec2::new(0.75, 0.5),
                Vec2::new(0.25, 1.0),
                Vec2::new(0.75, 1.0),
            ])),
        ),
    ]);
    model.meshes.extend([
        quad_mesh(
            "Face",
            "Root",
            Some("Head"),
            KeyformGrid::constant(mesh_keyform(rect(0.0, -10.0, 10.0, 0.0), 1.0, 500.0)),
        ),
        quad_mesh(
            "Blush",
            "Root",
            Some("Cheek"),
            KeyformGrid::constant(mesh_keyform(rect(0.5, 0.5, 0.5, 0.5), 1.0, 500.0)),
        ),
    ]);
    model
}

/// パーツ階層・デフォーマの不透明度・描画順の同順位。
///
/// - パラメータ `Fade` (0..1、既定 0)。
/// - パーツ: `Root` (不透明度 `Fade = 0` で 1、`Fade = 1` で 0.5) → `Child` (0.5) → `Grandchild` (0.8)。
/// - 回転デフォーマ (恒等変換): `Outer` (不透明度 0.5) → `InnerRot` (不透明度 0.5)。
/// - メッシュ (宣言順):
///   - `A`: パーツ `Grandchild`、親 `InnerRot`、描画順 500。不透明度 = 1 × (1 × 0.5 × 0.8) × (0.5 × 0.5)。
///   - `B`: パーツ `Root`、親なし、描画順 500。不透明度は `Fade` で 1 → 0。
///   - `C`: パーツ `Child`、親 `Outer`、描画順 300。
///   - `D`: パーツ `Root`、親なし、描画順は `Fade` で 500 → 100。
pub fn part_opacity_hierarchy() -> Model {
    let mut model = base_model(
        "part_opacity_hierarchy",
        vec![parameter("Fade", 0.0, 0.0, 1.0)],
    );
    let opacity = |opacity| PartKeyform { opacity };
    model.parts = vec![
        part(
            "Root",
            None,
            grid1("Fade", &[0.0, 1.0], vec![opacity(1.0), opacity(0.5)]),
        ),
        part("Child", Some("Root"), KeyformGrid::constant(opacity(0.5))),
        part(
            "Grandchild",
            Some("Child"),
            KeyformGrid::constant(opacity(0.8)),
        ),
    ];
    let half = RotationKeyform {
        opacity: 0.5,
        ..rotation_keyform(Vec2::ZERO, 0.0, 1.0)
    };
    // 子を先に宣言して、トポロジカル順の並べ替えも検査する。
    model.deformers.extend([
        rotation("InnerRot", Some("Outer"), KeyformGrid::constant(half)),
        rotation("Outer", None, KeyformGrid::constant(half)),
    ]);
    let square = rect(64.0, 64.0, 192.0, 192.0);
    model.meshes.extend([
        quad_mesh(
            "A",
            "Grandchild",
            Some("InnerRot"),
            KeyformGrid::constant(mesh_keyform(square, 1.0, 500.0)),
        ),
        quad_mesh(
            "B",
            "Root",
            None,
            grid1(
                "Fade",
                &[0.0, 1.0],
                vec![
                    mesh_keyform(square, 1.0, 500.0),
                    mesh_keyform(square, 0.0, 500.0),
                ],
            ),
        ),
        quad_mesh(
            "C",
            "Child",
            Some("Outer"),
            KeyformGrid::constant(mesh_keyform(square, 1.0, 300.0)),
        ),
        quad_mesh(
            "D",
            "Root",
            None,
            grid1(
                "Fade",
                &[0.0, 1.0],
                vec![
                    mesh_keyform(square, 1.0, 500.0),
                    mesh_keyform(square, 1.0, 100.0),
                ],
            ),
        ),
    ]);
    model
}

/// 隙間を空けて並べた四角形 2 枚と、それをつなぐグルー 2 つ。
///
/// - パラメータ `Glue` (0..1、既定 1)。
/// - メッシュ `Left`: `(0,0)`〜`(100,100)`。メッシュ `Right`: `(110,0)`〜`(210,100)`。
/// - グルー `Seam` (強さ = `Glue`、キー 0, 1): `Left` の頂点 1 と `Right` の頂点 0、
///   `Left` の頂点 2 と `Right` の頂点 3 を、重み 0.5 (中点) で引き寄せる。
/// - グルー `Pull` (強さ 0.5): `Left` の頂点 1 と `Right` の頂点 1 を重み 0 (`Left` 側) で引き寄せる。
///   `Seam` の後に適用されるので、`Seam` で動いた後の `Left` の頂点 1 が目標になる。
pub fn glued_quads() -> Model {
    let mut model = base_model("glued_quads", vec![parameter("Glue", 0.0, 1.0, 1.0)]);
    model.meshes.extend([
        quad_mesh(
            "Left",
            "Root",
            None,
            KeyformGrid::constant(mesh_keyform(rect(0.0, 0.0, 100.0, 100.0), 1.0, 500.0)),
        ),
        quad_mesh(
            "Right",
            "Root",
            None,
            KeyformGrid::constant(mesh_keyform(rect(110.0, 0.0, 210.0, 100.0), 1.0, 500.0)),
        ),
    ]);
    let pair = |a, b, weight| GluePair { a, b, weight };
    model.glues = vec![
        Glue {
            id: "Seam".into(),
            mesh_a: "Left".into(),
            mesh_b: "Right".into(),
            pairs: vec![pair(1, 0, 0.5), pair(2, 3, 0.5)],
            keyforms: grid1(
                "Glue",
                &[0.0, 1.0],
                vec![
                    GlueKeyform { intensity: 0.0 },
                    GlueKeyform { intensity: 1.0 },
                ],
            ),
        },
        Glue {
            id: "Pull".into(),
            mesh_a: "Left".into(),
            mesh_b: "Right".into(),
            pairs: vec![pair(1, 1, 0.0)],
            keyforms: KeyformGrid::constant(GlueKeyform { intensity: 0.5 }),
        },
    ];
    model
}

// ---- マスクのフィクスチャ (T0107) ----
//
// どれも 256×256 キャンバス、パーツ `Root`、白い 1×1 テクスチャ (アルファ 1) を持つ。
// 見える範囲は `docs/spec/rendering.md#マスク` の式から求めたもの (四角形の範囲で書く)。

/// `rect` の四角形メッシュ (親なし・パーツ `Root`・不透明度 1)。
fn masked_quad(
    id: &str,
    rect: [Vec2; 4],
    draw_order: f32,
    masks: Vec<MaskRef>,
    mask_mode: MaskMode,
) -> Mesh {
    Mesh {
        masks,
        mask_mode,
        ..quad_mesh(
            id,
            "Root",
            None,
            KeyformGrid::constant(mesh_keyform(rect, 1.0, draw_order)),
        )
    }
}

/// 入れ子のマスク (白目 → 黒目 → 瞳孔)。依存の順とは逆に宣言する。
///
/// - メッシュ `Pupil`: `(112,40)`〜`(144,120)`、描画順 600、マスク `Iris`。
/// - メッシュ `Iris`: `(96,64)`〜`(160,192)`、描画順 500、マスク `Sclera`。
/// - メッシュ `Sclera`: `(64,96)`〜`(192,160)`、描画順 400、マスクなし。
///
/// 見える範囲: `Iris` は `(96,96)`〜`(160,160)`。`Pupil` は入れ子を継承するので `(112,96)`〜`(144,120)`
/// (継承しなければ `(112,64)`〜`(144,120)`)。マスクの依存の深さは `Sclera` 0、`Iris` 1、`Pupil` 2。
pub fn nested_masks() -> Model {
    let mut model = base_model("nested_masks", vec![]);
    model.meshes.extend([
        masked_quad(
            "Pupil",
            rect(112.0, 40.0, 144.0, 120.0),
            600.0,
            vec![MaskRef::normal("Iris")],
            MaskMode::Union,
        ),
        masked_quad(
            "Iris",
            rect(96.0, 64.0, 160.0, 192.0),
            500.0,
            vec![MaskRef::normal("Sclera")],
            MaskMode::Union,
        ),
        masked_quad(
            "Sclera",
            rect(64.0, 96.0, 192.0, 160.0),
            400.0,
            vec![],
            MaskMode::Union,
        ),
    ]);
    model
}

/// 1 つのメッシュで通常マスクと反転マスクを併用し、`mask_strength` をパラメータで動かす。
///
/// - パラメータ `Strength` (0..1、既定 1)。
/// - メッシュ `Face`: `(32,32)`〜`(224,224)`、描画順 300。
/// - メッシュ `Hair`: `(32,32)`〜`(224,96)`、描画順 700。
/// - メッシュ `Brow`: `(64,64)`〜`(192,128)`、描画順 500、マスク `Face` (通常) と `Hair` (反転)。
///   `mask_strength` のキーフォームは `Strength = 0` で -1、`Strength = 1` で 2 (範囲外はクランプされる)。
///   補間値は `-1 + 3 × Strength` なので、`Strength = 0.5` で 0.5、`Strength ≤ 1/3` で 0、`Strength ≥ 2/3` で 1。
///
/// `mask_strength = 1` での `Brow` の見える範囲は `(64,96)`〜`(192,128)` (顔の内側かつ髪の外側)。
pub fn mixed_masks() -> Model {
    let mut model = base_model("mixed_masks", vec![parameter("Strength", 0.0, 1.0, 1.0)]);
    let brow = |mask_strength| MeshKeyform {
        mask_strength,
        ..mesh_keyform(rect(64.0, 64.0, 192.0, 128.0), 1.0, 500.0)
    };
    model.meshes.extend([
        masked_quad(
            "Face",
            rect(32.0, 32.0, 224.0, 224.0),
            300.0,
            vec![],
            MaskMode::Union,
        ),
        masked_quad(
            "Hair",
            rect(32.0, 32.0, 224.0, 96.0),
            700.0,
            vec![],
            MaskMode::Union,
        ),
        Mesh {
            masks: vec![MaskRef::normal("Face"), MaskRef::inverted("Hair")],
            ..quad_mesh(
                "Brow",
                "Root",
                None,
                grid1("Strength", &[0.0, 1.0], vec![brow(-1.0), brow(2.0)]),
            )
        },
    ]);
    model
}

/// 共通部分のマスクとマスク専用メッシュ (舌を口内と輪郭の両方に収める)。
///
/// - メッシュ `Face`: `(32,32)`〜`(224,224)`、描画順 300。
/// - メッシュ `MouthMask`: `(64,128)`〜`(192,192)`、描画順 400、`mask_only`。
/// - メッシュ `Tongue`: `(96,160)`〜`(240,240)`、描画順 500、マスク `MouthMask` と `Face` を `intersection` で結合。
///
/// 見える範囲: `Tongue` は `(96,160)`〜`(192,192)`。`MouthMask` は色としては描かれない (`visible = false`)。
pub fn intersection_masks() -> Model {
    let mut model = base_model("intersection_masks", vec![]);
    model.meshes.extend([
        masked_quad(
            "Face",
            rect(32.0, 32.0, 224.0, 224.0),
            300.0,
            vec![],
            MaskMode::Union,
        ),
        Mesh {
            mask_only: true,
            ..masked_quad(
                "MouthMask",
                rect(64.0, 128.0, 192.0, 192.0),
                400.0,
                vec![],
                MaskMode::Union,
            )
        },
        masked_quad(
            "Tongue",
            rect(96.0, 160.0, 240.0, 240.0),
            500.0,
            vec![MaskRef::normal("MouthMask"), MaskRef::normal("Face")],
            MaskMode::Intersection,
        ),
    ]);
    model
}

// ---- キーフォーム推定のオーサリング用データ (T0108) ----

/// キーフォーム推定のデータ (由来・推定の設定・ミラー・深度) を持つ顔。値の検証は T0610〜T0612 で使う。
///
/// - パラメータ `AngleX` / `AngleY` (-30..30、既定 0)。
/// - ワープ `Face` (1×1 セル): 制御点は `(64,64)`〜`(192,192)`。軸は `AngleX` × `AngleY` (キー -30, 0, 30) の 9 セル。
///   中央のセル (線形インデックス 4) だけ `authored` で、残りは `derived` (値は仮に中央と同じ)。
///   推定は深度からの回転 (中心 `(128,128)`、深度 0、yaw = `AngleX`、pitch = `AngleY`)。
///   制御点の深度は上の行が 0、下の行が 16。
/// - メッシュ `EyeL` (UV 空間 `(0.2,0.4)`〜`(0.4,0.5)`) と `EyeR` (`(0.6,0.4)`〜`(0.8,0.5)`): 親 `Face`。
///   互いに `mirror` で対にし、対称軸は `u = 0.5`。頂点の深度はどちらも 8。
/// - メッシュ `Nose` (`(0.45,0.5)`〜`(0.55,0.7)`): 親 `Face`。`mirror` なし (自分自身と対応する)。
pub fn keyform_inference() -> Model {
    let mut model = base_model(
        "keyform_inference",
        vec![
            parameter("AngleX", -30.0, 0.0, 30.0),
            parameter("AngleY", -30.0, 0.0, 30.0),
        ],
    );
    let face = WarpKeyform {
        points: warp_points(1, 1, |x, y| {
            Vec2::new(64.0 + 128.0 * f32::from(x), 64.0 + 128.0 * f32::from(y))
        }),
        opacity: 1.0,
    };
    let keys = [-30.0, 0.0, 30.0];
    let mut grid = KeyformGrid::new(
        vec![binding("AngleX", &keys), binding("AngleY", &keys)],
        vec![face; 9],
    );
    grid.origins = (0..9)
        .map(|i| {
            if i == 4 {
                KeyformOrigin::Authored
            } else {
                KeyformOrigin::Derived
            }
        })
        .collect();
    grid.inference = Some(KeyformInference::DepthRotation(DepthRotation {
        angle_scale: 1.0,
        pivot: Vec2::new(128.0, 128.0),
        pivot_depth: 0.0,
        yaw: Some("AngleX".into()),
        pitch: Some("AngleY".into()),
    }));
    let mut face = warp("Face", None, 1, 1, grid);
    if let DeformerKind::Warp(w) = &mut face.kind {
        w.depth = Some(vec![0.0, 0.0, 16.0, 16.0]);
    }
    model.deformers.push(face);

    let feature = |id: &str, rect: [Vec2; 4], mirror: Option<&str>, depth: Option<f32>| Mesh {
        mirror: mirror.map(Into::into),
        mirror_axis: mirror.map(|_| 0.5),
        depth: depth.map(|d| vec![d; 4]),
        ..quad_mesh(
            id,
            "Root",
            Some("Face"),
            KeyformGrid::constant(mesh_keyform(rect, 1.0, 500.0)),
        )
    };
    model.meshes.extend([
        feature("EyeL", rect(0.2, 0.4, 0.4, 0.5), Some("EyeR"), Some(8.0)),
        feature("EyeR", rect(0.6, 0.4, 0.8, 0.5), Some("EyeL"), Some(8.0)),
        feature("Nose", rect(0.45, 0.5, 0.55, 0.7), None, None),
    ]);
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quad_is_valid() {
        assert_eq!(crate::validate::validate(&single_quad()), vec![]);
    }

    #[test]
    fn single_quad_roundtrips_json() {
        let model = single_quad();
        let json = serde_json::to_string(&model).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
    }

    #[test]
    fn pose_and_effects_is_valid() {
        assert_eq!(crate::validate::validate(&pose_and_effects()), vec![]);
    }

    #[test]
    fn pose_and_effects_roundtrips_json() {
        let model = pose_and_effects();
        let json = serde_json::to_string(&model).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
    }

    #[test]
    fn deformer_fixtures_are_valid_and_roundtrip_json() {
        for model in [
            warp_grid(),
            rotation_chain(),
            rotation_under_warp(),
            part_opacity_hierarchy(),
            glued_quads(),
            nested_masks(),
            mixed_masks(),
            intersection_masks(),
            keyform_inference(),
        ] {
            assert_eq!(
                crate::validate::validate(&model),
                vec![],
                "{}",
                model.meta.name
            );
            let json = serde_json::to_string(&model).unwrap();
            let back: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, back);
        }
    }

    #[test]
    fn json_without_mask_fields_uses_defaults() {
        let mut json = serde_json::to_value(single_quad()).unwrap();
        let mesh = json["meshes"][0].as_object_mut().unwrap();
        for key in ["masks", "mask_mode", "mask_only"] {
            assert!(mesh.remove(key).is_some(), "{key}");
        }
        for kf in json["meshes"][0]["keyforms"]["keyforms"]
            .as_array_mut()
            .unwrap()
        {
            assert!(
                kf.as_object_mut()
                    .unwrap()
                    .remove("mask_strength")
                    .is_some()
            );
        }
        let model: Model = serde_json::from_value(json).unwrap();
        assert_eq!(model, single_quad());
    }

    #[test]
    fn mask_json_format() {
        let json = serde_json::to_value(&intersection_masks().meshes[2]).unwrap();
        assert_eq!(json["mask_mode"], "intersection");
        assert_eq!(
            json["masks"],
            serde_json::json!([
                { "mesh": "MouthMask", "invert": false },
                { "mesh": "Face", "invert": false },
            ])
        );
        // `invert` は省略できる (既定 false)。
        let r: MaskRef = serde_json::from_value(serde_json::json!({ "mesh": "Hair" })).unwrap();
        assert_eq!(r, MaskRef::normal("Hair"));
    }

    #[test]
    fn json_without_pose_and_effects_uses_defaults() {
        let mut json = serde_json::to_value(single_quad()).unwrap();
        let object = json.as_object_mut().unwrap();
        assert!(object.remove("pose").is_some());
        assert!(object.remove("effects").is_some());
        let model: Model = serde_json::from_value(json).unwrap();
        assert_eq!(model, single_quad());
        assert_eq!(model.pose.fade_duration, DEFAULT_FADE_DURATION);
    }

    #[test]
    fn json_without_inference_fields_uses_defaults() {
        // 由来・推定の設定・ミラー・深度を持たない (T0108 より前の) JSON。
        let json = serde_json::json!({
            "id": "W",
            "name": "W",
            "part": "Root",
            "parent": null,
            "kind": {
                "type": "warp",
                "rows": 1,
                "cols": 1,
                "keyforms": {
                    "keyforms": [{ "points": [[0, 0], [1, 0], [0, 1], [1, 1]], "opacity": 1 }],
                },
            },
        });
        let deformer: Deformer = serde_json::from_value(json).unwrap();
        assert_eq!(deformer.mirror, None);
        assert_eq!(deformer.mirror_axis, None);
        let DeformerKind::Warp(warp) = &deformer.kind else {
            panic!("not a warp");
        };
        assert_eq!(warp.depth, None);
        assert!(warp.keyforms.origins.is_empty());
        assert_eq!(warp.keyforms.inference, None);
        assert_eq!(warp.keyforms.origin(0), KeyformOrigin::Authored);

        // 既定値のフィールドは書き出さないので、既存のモデルの JSON は変わらない。
        let json = serde_json::to_value(single_quad()).unwrap();
        let mesh = json["meshes"][0].as_object().unwrap();
        for key in ["mirror", "mirror_axis", "depth"] {
            assert!(!mesh.contains_key(key), "{key}");
        }
        let grid = json["meshes"][0]["keyforms"].as_object().unwrap();
        for key in ["origins", "inference"] {
            assert!(!grid.contains_key(key), "{key}");
        }
        let model: Model = serde_json::from_value(json).unwrap();
        assert_eq!(model, single_quad());
    }

    #[test]
    fn inference_json_format() {
        let model = keyform_inference();
        let json = serde_json::to_value(&model.deformers[0]).unwrap();
        let grid = &json["kind"]["keyforms"];
        assert_eq!(grid["origins"][0], "derived");
        assert_eq!(grid["origins"][4], "authored");
        assert_eq!(
            grid["inference"],
            serde_json::json!({
                "method": "depth_rotation",
                "angle_scale": 1.0,
                "pivot": [128.0, 128.0],
                "pivot_depth": 0.0,
                "yaw": "AngleX",
                "pitch": "AngleY",
            })
        );
        assert_eq!(
            json["kind"]["depth"],
            serde_json::json!([0.0, 0.0, 16.0, 16.0])
        );
        let eye = serde_json::to_value(&model.meshes[0]).unwrap();
        assert_eq!(eye["mirror"], "EyeR");
        assert_eq!(eye["mirror_axis"], 0.5);

        // 加算合成は設定を持たない。深度からの回転の `angle_scale` などは省略できる。
        let additive: KeyformInference =
            serde_json::from_value(serde_json::json!({ "method": "additive" })).unwrap();
        assert_eq!(additive, KeyformInference::Additive);
        let rotation: KeyformInference = serde_json::from_value(
            serde_json::json!({ "method": "depth_rotation", "pivot": [1.0, 2.0] }),
        )
        .unwrap();
        assert_eq!(
            rotation,
            KeyformInference::DepthRotation(DepthRotation {
                angle_scale: 1.0,
                pivot: Vec2::new(1.0, 2.0),
                pivot_depth: 0.0,
                yaw: None,
                pitch: None,
            })
        );
    }
}
