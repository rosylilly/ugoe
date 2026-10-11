//! 要素 (パーツ・デフォーマ・メッシュ・グルー・物理のリグと衝突体) ごとの中身の違い。
//! 違いは種類と引数で返し、画面が今の言語の文にする (`docs/editor.md#違いの種類と引数`。T1677)。

use std::collections::BTreeSet;
use std::fmt::Display;

use serde::Serialize;
use ts_rs::TS;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::physics::{ColliderId, ColliderShape, PhysicsCollider, PhysicsRig};
use ugoe_model::{Deformer, DeformerKind, Glue, KeyformGrid, Mesh, Part};

/// 要素の中身の違い 1 つ (例: `{ kind: "count", what: "vertices", before: 12, after: 14 }`)。
/// 前の値 (`before`) はファイル、後の値 (`after`) は今の編集。値の無いもの (親なし・テクスチャなし) は `null`。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ElementChange {
    /// 名前。
    Name { before: String, after: String },
    /// 所属するパーツ (デフォーマ・メッシュ)。
    Part { before: String, after: String },
    /// 親 (パーツ・デフォーマ・メッシュ)。
    Parent {
        before: Option<String>,
        after: Option<String>,
    },
    /// デフォーマの種類。
    Type {
        before: DeformerType,
        after: DeformerType,
    },
    /// ワープの格子の大きさ。
    Grid { before: GridSize, after: GridSize },
    /// メッシュのテクスチャの名前。
    Texture {
        before: Option<String>,
        after: Option<String>,
    },
    /// 数の違い。
    Count {
        what: CountedItems,
        before: usize,
        after: usize,
    },
    /// 数は同じで、中身の変わったものの数。
    Changed { what: ChangedItems, count: usize },
    /// 値の変わったキーフォームの数。
    InKeyforms { what: KeyformValue, count: usize },
    /// メッシュの描画順 (どのキーフォームでも同じ値のとき。スロットの範囲に写し、小数 2 桁に丸めた値)。
    DrawOrder { before: f64, after: f64 },
    /// グルーの両端のメッシュ。
    GlueMeshes {
        before_a: String,
        before_b: String,
        after_a: String,
        after_b: String,
    },
    /// 物理のリグの衝突体の、足した・除いた ID。
    Colliders {
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// 物理のリグの座標系の基準 (重力の基準のデフォーマ)。
    Frame {
        before: Option<String>,
        after: Option<String>,
    },
    /// 衝突体の取り付け先。
    Attach {
        before: Option<String>,
        after: Option<String>,
    },
    /// 衝突体の形。
    Shape {
        before: ColliderShapeType,
        after: ColliderShapeType,
    },
    /// 衝突体の半径 (小数 2 桁に丸めた値)。
    Radius { before: f64, after: f64 },
    /// ポーズのグループの要素の並び (要素のパーツを ` / ` でつないだもの)。
    Entries { before: String, after: String },
    /// リンクのパーツの、足した・除いたもの。`entry` はグループの要素のパーツ (追従のリンクでは `null`)。
    Links {
        entry: Option<String>,
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// リンクのパーツの並びだけが違う。
    LinksReordered { entry: Option<String> },
    /// キーフォームの軸 (パラメータ)。
    KeyformParameters,
    /// ブレンドシェイプ。
    BlendShapes,
    /// キーフォームの設定 (補間・原点・推定・ミラー)。
    KeyformSettings,
    /// 三角形 (数は同じ)。
    Triangles,
    /// 物理のリグの正規化。
    Normalization,
    /// 衝突体の位置。
    Position,
    /// 合成・カリング・マスク。
    BlendingOrMasks,
    /// ほかの設定 (書き出せなかった違い)。
    OtherSettings,
}

/// [`ElementChange::Count`] の数えるもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CountedItems {
    Keyforms,
    Vertices,
    Triangles,
    VertexPairs,
    Particles,
    Inputs,
    Outputs,
}

/// [`ElementChange::Changed`] の数えるもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChangedItems {
    /// UV の変わった頂点。
    Uvs,
    /// 変わった頂点の組 (グルー)。
    VertexPairs,
    /// 設定の変わった粒子。
    Particles,
    /// 変わった入力。
    Inputs,
    /// 変わった出力。
    Outputs,
}

/// [`ElementChange::InKeyforms`] の、キーフォームで変わった値。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KeyformValue {
    Opacity,
    ControlPoints,
    OriginOrAngle,
    Intensity,
    VertexPositions,
    DrawOrder,
    OpacityOrColors,
}

/// デフォーマの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DeformerType {
    Warp,
    Rotation,
    Unknown,
}

/// 衝突体の形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ColliderShapeType {
    Circle,
    Capsule,
    Unknown,
}

/// ワープの格子の大きさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

fn or_none<T: Display>(value: Option<&T>) -> Option<String> {
    value.map(ToString::to_string)
}

/// 名前・所属・親の違い (パーツ・デフォーマ・メッシュで共通)。
fn common_changes<P: Display + PartialEq, Q: Display + PartialEq>(
    out: &mut Vec<ElementChange>,
    names: (&str, &str),
    part: Option<(&P, &P)>,
    parent: (Option<&Q>, Option<&Q>),
) {
    if names.0 != names.1 {
        out.push(ElementChange::Name {
            before: names.0.to_owned(),
            after: names.1.to_owned(),
        });
    }
    if let Some((a, b)) = part
        && a != b
    {
        out.push(ElementChange::Part {
            before: a.to_string(),
            after: b.to_string(),
        });
    }
    if parent.0 != parent.1 {
        out.push(ElementChange::Parent {
            before: or_none(parent.0),
            after: or_none(parent.1),
        });
    }
}

/// キーフォームの格子の違い。数が同じなら、対応するキーフォームの組を `values` に渡して中身を比べる。
fn keyform_changes<T: PartialEq>(
    out: &mut Vec<ElementChange>,
    old: &KeyformGrid<T>,
    new: &KeyformGrid<T>,
    values: impl FnOnce(&[(&T, &T)], &mut Vec<ElementChange>),
) {
    if old.bindings != new.bindings {
        out.push(ElementChange::KeyformParameters);
    }
    if old.keyforms.len() == new.keyforms.len() {
        let pairs: Vec<(&T, &T)> = old.keyforms.iter().zip(&new.keyforms).collect();
        values(&pairs, out);
    } else {
        out.push(count(
            CountedItems::Keyforms,
            old.keyforms.len(),
            new.keyforms.len(),
        ));
    }
    if old.blend_shapes != new.blend_shapes {
        out.push(ElementChange::BlendShapes);
    }
    if old.interpolation != new.interpolation
        || old.origins != new.origins
        || old.inference != new.inference
        || old.mirror_source != new.mirror_source
    {
        out.push(ElementChange::KeyformSettings);
    }
}

fn count(what: CountedItems, before: usize, after: usize) -> ElementChange {
    ElementChange::Count {
        what,
        before,
        after,
    }
}

/// `pairs` のうち `differs` なものの数を、`what` の変わったキーフォームの数として書く (無ければ書かない)。
fn in_keyforms<T>(
    out: &mut Vec<ElementChange>,
    pairs: &[(&T, &T)],
    what: KeyformValue,
    differs: impl Fn(&T, &T) -> bool,
) {
    let count = pairs.iter().filter(|(a, b)| differs(a, b)).count();
    if count > 0 {
        out.push(ElementChange::InKeyforms { what, count });
    }
}

/// 並べた 2 つの列のうち、違う組の数 (`what` として書く。無ければ書かない)。
fn changed<T: PartialEq>(out: &mut Vec<ElementChange>, what: ChangedItems, old: &[T], new: &[T]) {
    let count = old.iter().zip(new).filter(|(a, b)| a != b).count();
    if count > 0 {
        out.push(ElementChange::Changed { what, count });
    }
}

/// 違う要素の中身の違い。どこが違うか書き出せなかったら [`ElementChange::OtherSettings`]。
pub(super) fn or_other(mut out: Vec<ElementChange>) -> Vec<ElementChange> {
    if out.is_empty() {
        out.push(ElementChange::OtherSettings);
    }
    out
}

pub(super) fn part_changes(old: &Part, new: &Part) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes::<String, _>(
        &mut out,
        (&old.name, &new.name),
        None,
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        in_keyforms(out, pairs, KeyformValue::Opacity, |a, b| a != b);
    });
    or_other(out)
}

pub(super) fn deformer_changes(old: &Deformer, new: &Deformer) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes(
        &mut out,
        (&old.name, &new.name),
        Some((&old.part, &new.part)),
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    match (&old.kind, &new.kind) {
        (DeformerKind::Warp(a), DeformerKind::Warp(b)) => {
            let same_grid = (a.rows, a.cols) == (b.rows, b.cols);
            if !same_grid {
                out.push(ElementChange::Grid {
                    before: GridSize {
                        cols: a.cols,
                        rows: a.rows,
                    },
                    after: GridSize {
                        cols: b.cols,
                        rows: b.rows,
                    },
                });
            }
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                if same_grid {
                    in_keyforms(out, pairs, KeyformValue::ControlPoints, |a, b| a != b);
                }
            });
        }
        (DeformerKind::Rotation(a), DeformerKind::Rotation(b)) => {
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                in_keyforms(out, pairs, KeyformValue::OriginOrAngle, |a, b| a != b);
            });
        }
        (a, b) => out.push(ElementChange::Type {
            before: deformer_type(a),
            after: deformer_type(b),
        }),
    }
    or_other(out)
}

fn deformer_type(kind: &DeformerKind) -> DeformerType {
    match kind {
        DeformerKind::Warp(_) => DeformerType::Warp,
        DeformerKind::Rotation(_) => DeformerType::Rotation,
        _ => DeformerType::Unknown,
    }
}

/// グルーの中身の違い (両端のメッシュ・頂点の組・強さのキーフォーム)。
pub(super) fn glue_changes(old: &Glue, new: &Glue) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if (&old.mesh_a, &old.mesh_b) != (&new.mesh_a, &new.mesh_b) {
        out.push(ElementChange::GlueMeshes {
            before_a: old.mesh_a.to_string(),
            before_b: old.mesh_b.to_string(),
            after_a: new.mesh_a.to_string(),
            after_b: new.mesh_b.to_string(),
        });
    }
    if old.pairs.len() == new.pairs.len() {
        changed(&mut out, ChangedItems::VertexPairs, &old.pairs, &new.pairs);
    } else {
        out.push(count(
            CountedItems::VertexPairs,
            old.pairs.len(),
            new.pairs.len(),
        ));
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        in_keyforms(out, pairs, KeyformValue::Intensity, |a, b| a != b);
    });
    or_other(out)
}

/// 物理のリグの中身の違い (入力・出力・粒子・正規化・衝突体・座標系の基準)。
pub(super) fn rig_changes(old: &PhysicsRig, new: &PhysicsRig) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(ElementChange::Name {
            before: old.name.clone(),
            after: new.name.clone(),
        });
    }
    listed(
        &mut out,
        (CountedItems::Inputs, ChangedItems::Inputs),
        &old.inputs,
        &new.inputs,
    );
    listed(
        &mut out,
        (CountedItems::Outputs, ChangedItems::Outputs),
        &old.outputs,
        &new.outputs,
    );
    listed(
        &mut out,
        (CountedItems::Particles, ChangedItems::Particles),
        &old.particles,
        &new.particles,
    );
    if old.normalization != new.normalization {
        out.push(ElementChange::Normalization);
    }
    let colliders = |c: &[ColliderId]| c.iter().cloned().collect::<BTreeSet<ColliderId>>();
    let (a, b) = (colliders(&old.colliders), colliders(&new.colliders));
    if a != b {
        out.push(ElementChange::Colliders {
            added: b.difference(&a).map(ToString::to_string).collect(),
            removed: a.difference(&b).map(ToString::to_string).collect(),
        });
    }
    if old.frame != new.frame {
        out.push(ElementChange::Frame {
            before: or_none(old.frame.as_ref()),
            after: or_none(new.frame.as_ref()),
        });
    }
    or_other(out)
}

/// 並び (入力・出力・粒子) の違い。数が違えば数、同じなら違うものの数。
fn listed<T: PartialEq>(
    out: &mut Vec<ElementChange>,
    (counted, what): (CountedItems, ChangedItems),
    old: &[T],
    new: &[T],
) {
    if old.len() == new.len() {
        changed(out, what, old, new);
    } else {
        out.push(count(counted, old.len(), new.len()));
    }
}

/// 衝突体の中身の違い (名前・取り付け先・形・位置・半径)。
pub(super) fn collider_changes(old: &PhysicsCollider, new: &PhysicsCollider) -> Vec<ElementChange> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(ElementChange::Name {
            before: old.name.clone(),
            after: new.name.clone(),
        });
    }
    if old.attach != new.attach {
        out.push(ElementChange::Attach {
            before: or_none(old.attach.as_ref()),
            after: or_none(new.attach.as_ref()),
        });
    }
    match (&old.shape, &new.shape) {
        (
            ColliderShape::Circle {
                center: c0,
                radius: r0,
            },
            ColliderShape::Circle {
                center: c1,
                radius: r1,
            },
        ) => {
            if c0 != c1 {
                out.push(ElementChange::Position);
            }
            radius(&mut out, *r0, *r1);
        }
        (
            ColliderShape::Capsule {
                a: a0,
                b: b0,
                radius: r0,
            },
            ColliderShape::Capsule {
                a: a1,
                b: b1,
                radius: r1,
            },
        ) => {
            if (a0, b0) != (a1, b1) {
                out.push(ElementChange::Position);
            }
            radius(&mut out, *r0, *r1);
        }
        (a, b) => out.push(ElementChange::Shape {
            before: shape_type(a),
            after: shape_type(b),
        }),
    }
    or_other(out)
}

fn radius(out: &mut Vec<ElementChange>, old: f32, new: f32) {
    if old != new {
        out.push(ElementChange::Radius {
            before: number(old),
            after: number(new),
        });
    }
}

fn shape_type(shape: &ColliderShape) -> ColliderShapeType {
    match shape {
        ColliderShape::Circle { .. } => ColliderShapeType::Circle,
        ColliderShape::Capsule { .. } => ColliderShapeType::Capsule,
        _ => ColliderShapeType::Unknown,
    }
}

/// メッシュの中身の違い (頂点・三角形・キーフォーム・描画順・テクスチャなど)。テクスチャは名前で比べる。
/// 描画順は `map` で写して書く。
pub(super) fn mesh_changes(
    (old, old_texture): (&Mesh, Option<&str>),
    (new, new_texture): (&Mesh, Option<&str>),
    map: &dyn Fn(f32) -> f32,
) -> Vec<ElementChange> {
    let same = old_texture == new_texture && {
        let mut new = new.clone();
        new.texture = old.texture;
        *old == new
    };
    if same {
        return Vec::new();
    }
    let mut out = Vec::new();
    common_changes(
        &mut out,
        (&old.name, &new.name),
        Some((&old.part, &new.part)),
        (old.parent.as_ref(), new.parent.as_ref()),
    );
    if old_texture != new_texture {
        out.push(ElementChange::Texture {
            before: old_texture.map(str::to_owned),
            after: new_texture.map(str::to_owned),
        });
    }
    let same_vertices = old.uvs.len() == new.uvs.len();
    if same_vertices {
        changed(&mut out, ChangedItems::Uvs, &old.uvs, &new.uvs);
    } else {
        out.push(count(CountedItems::Vertices, old.uvs.len(), new.uvs.len()));
    }
    if old.indices != new.indices {
        out.push(if old.indices.len() == new.indices.len() {
            ElementChange::Triangles
        } else {
            count(
                CountedItems::Triangles,
                old.indices.len() / 3,
                new.indices.len() / 3,
            )
        });
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        if same_vertices {
            in_keyforms(out, pairs, KeyformValue::VertexPositions, |a, b| {
                a.positions != b.positions
            });
        }
        draw_order(out, pairs, map);
        in_keyforms(out, pairs, KeyformValue::OpacityOrColors, |a, b| {
            a.opacity != b.opacity
                || a.multiply_color != b.multiply_color
                || a.screen_color != b.screen_color
                || a.mask_strength != b.mask_strength
        });
    });
    if old.blend != new.blend
        || old.culling != new.culling
        || old.masks != new.masks
        || old.mask_mode != new.mask_mode
        || old.mask_only != new.mask_only
    {
        out.push(ElementChange::BlendingOrMasks);
    }
    if old.mirror != new.mirror
        || old.mirror_axis != new.mirror_axis
        || old.depth != new.depth
        || old.lit != new.lit
    {
        out.push(ElementChange::OtherSettings);
    }
    or_other(out)
}

/// 描画順の違い。どのキーフォームでも同じ値なら前後の値、そうでなければ変わったキーフォームの数。
fn draw_order(
    out: &mut Vec<ElementChange>,
    pairs: &[(&MeshKeyform, &MeshKeyform)],
    map: &dyn Fn(f32) -> f32,
) {
    let count = pairs
        .iter()
        .filter(|(a, b)| a.draw_order != b.draw_order)
        .count();
    if count == 0 {
        return;
    }
    let single = |values: Vec<f32>| {
        let first = values[0];
        values.iter().all(|&v| v == first).then_some(first)
    };
    let olds = pairs.iter().map(|(a, _)| a.draw_order).collect();
    let news = pairs.iter().map(|(_, b)| b.draw_order).collect();
    match (single(olds), single(news)) {
        (Some(a), Some(b)) => out.push(ElementChange::DrawOrder {
            before: number(map(a)),
            after: number(map(b)),
        }),
        _ => out.push(ElementChange::InKeyforms {
            what: KeyformValue::DrawOrder,
            count,
        }),
    }
}

/// 表示用の数 (写したときの丸めのずれを出さないように、小数 2 桁まで)。
fn number(value: f32) -> f64 {
    (f64::from(value) * 100.0).round() / 100.0
}
