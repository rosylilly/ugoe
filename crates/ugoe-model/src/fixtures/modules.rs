//! 着せ替え (T0111) のフィクスチャ: スロットを持つ本体と、そこに取り付けるモジュール。

use ugoe_core::Vec2;

use super::{
    base_model, grid1, mesh_keyform, parameter, part, quad_mesh, rect, rotation, rotation_keyform,
    warp,
};
use crate::deformer::WarpKeyform;
use crate::mesh::MaskRef;
use crate::module::{DrawOrderRange, Module, ModulePhysics, ModulePose, ModuleRequirements, Slot};
use crate::part::PartKeyform;
use crate::physics::{
    ColliderShape, NormalizationRange, PhysicsBlend, PhysicsCollider, PhysicsInput,
    PhysicsNormalization, PhysicsOutput, PhysicsParticle, PhysicsRig, PhysicsSource, PhysicsTarget,
};
use crate::pose::{PoseEntry, PoseGroup};
use crate::{KeyformGrid, Model, SCHEMA_VERSION, TextureRef, standard};

/// スロット `Hair` と `Outfit` を公開する本体。[`long_hair_module`] と [`dress_module`] (または [`jacket_module`]) を取り付ける。
/// ポーズを持つ版は [`posed_dress_up_base`]。
///
/// - パラメータ: `AngleX` (標準)、`HairFront` (標準)、`BodySway` (0..1、`Outfit` が公開)、
///   `Secret` (0..1、どのスロットも公開しない)。
/// - パーツ: `Root` → `Hair` (既定の髪)、`Root` → `Body`。
/// - 回転 `Head` (親なし): 原点 `(128,96)`、角度 = `AngleX` / 3 度 (キー -30, 30)。
/// - ワープ `Torso` (親なし、1×1 セル): `(64,128)`〜`(192,256)`。
/// - メッシュ: `Face` (親 `Head`、描画順 500)、`HairDefault` (パーツ `Hair`、親 `Head`、描画順 600)、
///   `BodyMesh` (パーツ `Body`、親 `Torso`、UV 全体、描画順 300)。
/// - スロット `Hair`: 取り付け先 `Head`、描画順 550〜650、`Hair` を置き換える、マスクに `Face` を公開。
/// - スロット `Outfit`: 取り付け先 `Torso`、描画順 300〜400、`BodySway` を公開。モジュールの最上位のパーツは `Body` の子。
pub fn dress_up_base() -> Model {
    let mut model = base_model(
        "dress_up_base",
        vec![
            parameter(standard::ANGLE_X, -30.0, 0.0, 30.0),
            parameter(standard::HAIR_FRONT, -1.0, 0.0, 1.0),
            parameter("BodySway", 0.0, 0.0, 1.0),
            parameter("Secret", 0.0, 0.0, 1.0),
        ],
    );
    let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
    model.parts.extend([
        part("Hair", Some("Root"), opaque()),
        part("Body", Some("Root"), opaque()),
    ]);
    let head = |angle| rotation_keyform(Vec2::new(128.0, 96.0), angle, 1.0);
    model.deformers.extend([
        rotation(
            "Head",
            None,
            grid1(
                standard::ANGLE_X,
                &[-30.0, 30.0],
                vec![head(-10.0), head(10.0)],
            ),
        ),
        warp(
            "Torso",
            None,
            1,
            1,
            KeyformGrid::constant(WarpKeyform {
                points: vec![
                    Vec2::new(64.0, 128.0),
                    Vec2::new(192.0, 128.0),
                    Vec2::new(64.0, 256.0),
                    Vec2::new(192.0, 256.0),
                ],
                opacity: 1.0,
            }),
        ),
    ]);
    model.meshes.extend([
        quad_mesh(
            "Face",
            "Root",
            Some("Head"),
            KeyformGrid::constant(mesh_keyform(rect(-32.0, -32.0, 32.0, 32.0), 1.0, 500.0)),
        ),
        quad_mesh(
            "HairDefault",
            "Hair",
            Some("Head"),
            KeyformGrid::constant(mesh_keyform(rect(-40.0, -48.0, 40.0, 0.0), 1.0, 600.0)),
        ),
        quad_mesh(
            "BodyMesh",
            "Body",
            Some("Torso"),
            KeyformGrid::constant(mesh_keyform(rect(0.0, 0.0, 1.0, 1.0), 1.0, 300.0)),
        ),
    ]);
    model.slots = vec![
        Slot {
            id: "Hair".into(),
            anchor: Some("Head".into()),
            draw_order: DrawOrderRange {
                min: 550.0,
                max: 650.0,
            },
            replaces: vec!["Hair".into()],
            parameters: Vec::new(),
            mask_sources: vec!["Face".into()],
            part: None,
            colliders: Vec::new(),
            pose_parts: Vec::new(),
        },
        Slot {
            id: "Outfit".into(),
            anchor: Some("Torso".into()),
            draw_order: DrawOrderRange {
                min: 300.0,
                max: 400.0,
            },
            replaces: Vec::new(),
            parameters: vec!["BodySway".into()],
            mask_sources: Vec::new(),
            part: Some("Body".into()),
            colliders: Vec::new(),
            pose_parts: Vec::new(),
        },
    ];
    model
}

/// [`dress_up_base`] に腕の差分のポーズを加え、スロット `Outfit` から追従できるようにした本体。
/// [`sleeves_module`] を取り付ける (ほかのモジュールも [`dress_up_base`] と同じに取り付けられる)。
///
/// - パーツ `ArmA` と `ArmB` (`Body` の子、不透明度 1)。
/// - メッシュ `ArmAMesh` (パーツ `ArmA`、UV `(0,0)`〜`(0.5,1)`) と `ArmBMesh` (パーツ `ArmB`、UV `(0.5,0)`〜`(1,1)`)。
///   どちらも親 `Torso`、描画順 450。
/// - ポーズ: グループ 1 つ (`ArmA`, `ArmB` の順。最初は `ArmA` を表示する)。
/// - スロット `Outfit` の `pose_parts`: `ArmA`, `ArmB`。
pub fn posed_dress_up_base() -> Model {
    let mut model = dress_up_base();
    model.meta.name = "posed_dress_up_base".into();
    let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
    let mesh = |id: &str, part: &str, left: f32| {
        quad_mesh(
            id,
            part,
            Some("Torso"),
            KeyformGrid::constant(mesh_keyform(rect(left, 0.0, left + 0.5, 1.0), 1.0, 450.0)),
        )
    };
    let entry = |part: &str| PoseEntry {
        part: part.into(),
        links: Vec::new(),
    };
    model.parts.extend([
        part("ArmA", Some("Body"), opaque()),
        part("ArmB", Some("Body"), opaque()),
    ]);
    model
        .meshes
        .extend([mesh("ArmAMesh", "ArmA", 0.0), mesh("ArmBMesh", "ArmB", 0.5)]);
    model.pose.groups = vec![PoseGroup {
        entries: vec![entry("ArmA"), entry("ArmB")],
    }];
    model.slots[1].pose_parts = vec!["ArmA".into(), "ArmB".into()];
    model
}

/// スロット `Hair` に取り付けるモジュール `LongHair`。
///
/// - パーツ `Hair` (本体のパーツと同じ ID。合成で `LongHair/Hair` になる)。
/// - 回転 `Sway` (親なし = `Head` の子): 原点 `(0,-32)`、角度 = `HairFront` × 15 度 (キー -1, 1)。
/// - メッシュ `Back` (親 `Sway`、描画順 0、本体の `Face` でマスク) と `Front` (親 `Sway`、描画順 1000、
///   モジュールの `Back` でマスク)。テクスチャはモジュールの 0 番 (`long_hair`)。
/// - 物理: リグ `Sway` (`AngleX` → x、粒子 1 の角度を `Sway` の角度に足す、衝突体 `Collar`)、
///   衝突体 `Collar` (取り付け先なし = `Head`)。
pub fn long_hair_module() -> Module {
    let mut sway = rotation(
        "Sway",
        None,
        grid1(
            standard::HAIR_FRONT,
            &[-1.0, 1.0],
            vec![
                rotation_keyform(Vec2::new(0.0, -32.0), -15.0, 1.0),
                rotation_keyform(Vec2::new(0.0, -32.0), 15.0, 1.0),
            ],
        ),
    );
    sway.part = "Hair".into();
    let mut back = quad_mesh(
        "Back",
        "Hair",
        Some("Sway"),
        KeyformGrid::constant(mesh_keyform(rect(-48.0, 0.0, 48.0, 96.0), 1.0, 0.0)),
    );
    back.masks = vec![MaskRef::normal("Face")];
    let mut front = quad_mesh(
        "Front",
        "Hair",
        Some("Sway"),
        KeyformGrid::constant(mesh_keyform(rect(-40.0, 0.0, 40.0, 24.0), 1.0, 1000.0)),
    );
    front.masks = vec![MaskRef::normal("Back")];
    let range = NormalizationRange {
        min: -10.0,
        default: 0.0,
        max: 10.0,
    };
    let particle = |y: f32| PhysicsParticle {
        position: Vec2::new(0.0, y),
        mobility: 0.9,
        delay: 0.8,
        acceleration: 1.5,
        radius: y,
        collision_radius: 0.0,
    };
    Module {
        schema_version: SCHEMA_VERSION,
        name: "LongHair".into(),
        slot: "Hair".into(),
        requires: ModuleRequirements::default(),
        parts: vec![part(
            "Hair",
            None,
            KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        )],
        deformers: vec![sway],
        meshes: vec![back, front],
        glues: Vec::new(),
        textures: vec![TextureRef {
            name: "long_hair".into(),
            width: 1,
            height: 1,
            normal_map: None,
        }],
        physics: ModulePhysics {
            rigs: vec![PhysicsRig {
                id: "Sway".into(),
                name: "Sway".into(),
                inputs: vec![PhysicsInput {
                    parameter: standard::ANGLE_X.into(),
                    source: PhysicsSource::X,
                    weight: 100.0,
                    reflect: false,
                }],
                outputs: vec![PhysicsOutput {
                    target: PhysicsTarget::DeformerAngle {
                        deformer: "Sway".into(),
                    },
                    particle: 1,
                    source: PhysicsSource::Angle,
                    scale: 1.0,
                    weight: 100.0,
                    reflect: false,
                    blend: PhysicsBlend::Overwrite,
                }],
                particles: vec![particle(0.0), particle(10.0)],
                normalization: PhysicsNormalization {
                    position: range,
                    angle: range,
                },
                colliders: vec!["Collar".into()],
                frame: None,
            }],
            colliders: vec![PhysicsCollider {
                id: "Collar".into(),
                name: "Collar".into(),
                attach: None,
                shape: ColliderShape::Circle {
                    center: Vec2::new(0.0, 40.0),
                    radius: 8.0,
                },
            }],
        },
        pose: ModulePose::default(),
    }
}

/// スロット `Outfit` に取り付けるモジュール `Dress`。
///
/// - パーツ `Dress`。メッシュ `Skirt` (親なし = `Torso` の子、UV `(0,0.5)`〜`(1,1)`)。
///   描画順は 500 (合成で 350 になる)。`BodySway` (キー 0, 1) で右へ 0.25 ずれる。
/// - 条件: スロット `Hair` とパラメータ `BodySway` があること。
pub fn dress_module() -> Module {
    let skirt = |dx: f32| mesh_keyform(rect(dx, 0.5, 1.0 + dx, 1.0), 1.0, 500.0);
    Module {
        schema_version: SCHEMA_VERSION,
        name: "Dress".into(),
        slot: "Outfit".into(),
        requires: ModuleRequirements {
            avatar_level: None,
            slots: vec!["Hair".into()],
            parameters: vec!["BodySway".into()],
        },
        parts: vec![part(
            "Dress",
            None,
            KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        )],
        deformers: Vec::new(),
        meshes: vec![quad_mesh(
            "Skirt",
            "Dress",
            None,
            grid1("BodySway", &[0.0, 1.0], vec![skirt(0.0), skirt(0.25)]),
        )],
        glues: Vec::new(),
        textures: vec![TextureRef {
            name: "dress".into(),
            width: 1,
            height: 1,
            normal_map: None,
        }],
        physics: ModulePhysics::default(),
        pose: ModulePose::default(),
    }
}

/// スロット `Outfit` に取り付ける、ポーズのグループを持つモジュール `Jacket` ([`dress_module`] とは同じスロットなので一緒に付けられない)。
///
/// 着せ替えの直後 (モジュールのポーズのグループが現れたとき) に、選んだ要素をフェードせずに出すことを確かめるためのもの
/// (T0850)。
///
/// - パーツ `Closed` と `Open` (どちらも不透明度 1。合成で `Jacket/Closed`・`Jacket/Open` になり、本体の `Body` の子になる)。
/// - メッシュ `ClosedMesh` (パーツ `Closed`、UV `(0,0)`〜`(1,1)`) と `OpenMesh` (パーツ `Open`、UV `(0,0)`〜`(0.5,1)`)。
///   どちらも親なし (= `Torso` の子)、描画順 500 (合成で 350 になる)。テクスチャはモジュールの 0 番 (`jacket`)。
/// - ポーズ: グループ 1 つ (`Closed`, `Open` の順。最初は `Closed` を表示する)。クロスフェードの時間は本体のもの。
pub fn jacket_module() -> Module {
    let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
    let mesh = |id: &str, part: &str, right: f32| {
        quad_mesh(
            id,
            part,
            None,
            KeyformGrid::constant(mesh_keyform(rect(0.0, 0.0, right, 1.0), 1.0, 500.0)),
        )
    };
    let entry = |part: &str| PoseEntry {
        part: part.into(),
        links: Vec::new(),
    };
    Module {
        schema_version: SCHEMA_VERSION,
        name: "Jacket".into(),
        slot: "Outfit".into(),
        requires: ModuleRequirements::default(),
        parts: vec![part("Closed", None, opaque()), part("Open", None, opaque())],
        deformers: Vec::new(),
        meshes: vec![
            mesh("ClosedMesh", "Closed", 1.0),
            mesh("OpenMesh", "Open", 0.5),
        ],
        glues: Vec::new(),
        textures: vec![TextureRef {
            name: "jacket".into(),
            width: 1,
            height: 1,
            normal_map: None,
        }],
        physics: ModulePhysics::default(),
        pose: ModulePose {
            groups: vec![PoseGroup {
                entries: vec![entry("Closed"), entry("Open")],
            }],
            links: Vec::new(),
        },
    }
}

/// [`posed_dress_up_base`] のスロット `Outfit` に取り付ける、本体の腕の差分に袖を追従させるモジュール `Sleeves`。
///
/// - パーツ `SleeveA` と `SleeveB` (どちらも不透明度 1。合成で `Sleeves/SleeveA`・`Sleeves/SleeveB` になり、本体の `Body` の子になる)。
/// - メッシュ `SleeveAMesh` (パーツ `SleeveA`、UV `(0,0)`〜`(0.5,0.5)`) と `SleeveBMesh` (パーツ `SleeveB`、UV `(0.5,0)`〜`(1,0.5)`)。
///   どちらも親なし (= `Torso` の子)、描画順 500 (合成で 350 になる)。テクスチャはモジュールの 0 番 (`sleeves`)。
/// - ポーズ: 追従 2 つ (本体の `ArmA` に `SleeveA`、`ArmB` に `SleeveB`)。グループは持たない。
pub fn sleeves_module() -> Module {
    let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
    let mesh = |id: &str, part: &str, left: f32| {
        quad_mesh(
            id,
            part,
            None,
            KeyformGrid::constant(mesh_keyform(rect(left, 0.0, left + 0.5, 0.5), 1.0, 500.0)),
        )
    };
    let link = |arm: &str, sleeve: &str| PoseEntry {
        part: arm.into(),
        links: vec![sleeve.into()],
    };
    Module {
        schema_version: SCHEMA_VERSION,
        name: "Sleeves".into(),
        slot: "Outfit".into(),
        requires: ModuleRequirements::default(),
        parts: vec![
            part("SleeveA", None, opaque()),
            part("SleeveB", None, opaque()),
        ],
        deformers: Vec::new(),
        meshes: vec![
            mesh("SleeveAMesh", "SleeveA", 0.0),
            mesh("SleeveBMesh", "SleeveB", 0.5),
        ],
        glues: Vec::new(),
        textures: vec![TextureRef {
            name: "sleeves".into(),
            width: 1,
            height: 1,
            normal_map: None,
        }],
        physics: ModulePhysics::default(),
        pose: ModulePose {
            groups: Vec::new(),
            links: vec![link("ArmA", "SleeveA"), link("ArmB", "SleeveB")],
        },
    }
}
