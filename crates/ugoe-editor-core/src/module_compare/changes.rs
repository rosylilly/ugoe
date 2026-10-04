//! 要素 (パーツ・デフォーマ・メッシュ・グルー・物理のリグと衝突体) ごとの中身の違い。

use std::collections::BTreeSet;
use std::fmt::Display;

use ugoe_model::mesh::MeshKeyform;
use ugoe_model::physics::{ColliderId, ColliderShape, PhysicsCollider, PhysicsRig};
use ugoe_model::{Deformer, DeformerKind, Glue, KeyformGrid, Mesh, Part};

use super::details;

/// `n` 個の `singular` (`plural`)。
fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

fn or_none<T: Display>(value: Option<&T>) -> String {
    value.map_or_else(|| "none".to_owned(), ToString::to_string)
}

/// 名前・所属・親の違い (パーツ・デフォーマ・メッシュで共通)。
fn common_changes<P: Display + PartialEq, Q: Display + PartialEq>(
    out: &mut Vec<String>,
    names: (&str, &str),
    part: Option<(&P, &P)>,
    parent: (Option<&Q>, Option<&Q>),
) {
    if names.0 != names.1 {
        out.push(format!("name {} → {}", names.0, names.1));
    }
    if let Some((a, b)) = part
        && a != b
    {
        out.push(format!("part {a} → {b}"));
    }
    if parent.0 != parent.1 {
        out.push(format!(
            "parent {} → {}",
            or_none(parent.0),
            or_none(parent.1)
        ));
    }
}

/// キーフォームの格子の違い。数が同じなら、対応するキーフォームの組を `values` に渡して中身を比べる。
fn keyform_changes<T: PartialEq>(
    out: &mut Vec<String>,
    old: &KeyformGrid<T>,
    new: &KeyformGrid<T>,
    values: impl FnOnce(&[(&T, &T)], &mut Vec<String>),
) {
    if old.bindings != new.bindings {
        out.push("keyform parameters".to_owned());
    }
    if old.keyforms.len() == new.keyforms.len() {
        let pairs: Vec<(&T, &T)> = old.keyforms.iter().zip(&new.keyforms).collect();
        values(&pairs, out);
    } else {
        out.push(format!(
            "keyforms {} → {}",
            old.keyforms.len(),
            new.keyforms.len()
        ));
    }
    if old.blend_shapes != new.blend_shapes {
        out.push("blend shapes".to_owned());
    }
    if old.interpolation != new.interpolation
        || old.origins != new.origins
        || old.inference != new.inference
        || old.mirror_source != new.mirror_source
    {
        out.push("keyform settings".to_owned());
    }
}

/// `pairs` のうち `differs` なものの数を「`what` in N keyforms」で書く (無ければ書かない)。
fn in_keyforms<T>(
    out: &mut Vec<String>,
    pairs: &[(&T, &T)],
    what: &str,
    differs: impl Fn(&T, &T) -> bool,
) {
    let n = pairs.iter().filter(|(a, b)| differs(a, b)).count();
    if n > 0 {
        out.push(format!("{what} in {}", count(n, "keyform", "keyforms")));
    }
}

/// 違う要素の中身の違い。どこが違うか書き出せなかったら `other settings`。
pub(super) fn or_other(mut out: Vec<String>) -> Vec<String> {
    if out.is_empty() {
        out.push("other settings".to_owned());
    }
    out
}

pub(super) fn part_changes(old: &Part, new: &Part) -> Vec<String> {
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
        in_keyforms(out, pairs, "opacity", |a, b| a != b);
    });
    or_other(out)
}

pub(super) fn deformer_changes(old: &Deformer, new: &Deformer) -> Vec<String> {
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
                out.push(format!(
                    "grid {}×{} → {}×{}",
                    a.cols, a.rows, b.cols, b.rows
                ));
            }
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                if same_grid {
                    in_keyforms(out, pairs, "control points", |a, b| a != b);
                }
            });
        }
        (DeformerKind::Rotation(a), DeformerKind::Rotation(b)) => {
            keyform_changes(&mut out, &a.keyforms, &b.keyforms, |pairs, out| {
                in_keyforms(out, pairs, "origin or angle", |a, b| a != b);
            });
        }
        (a, b) => out.push(format!("type {} → {}", kind_name(a), kind_name(b))),
    }
    or_other(out)
}

fn kind_name(kind: &DeformerKind) -> &'static str {
    match kind {
        DeformerKind::Warp(_) => "warp",
        DeformerKind::Rotation(_) => "rotation",
        DeformerKind::Unknown => "unknown",
    }
}

/// グルーの中身の違い (両端のメッシュ・頂点の組・強さのキーフォーム)。
pub(super) fn glue_changes(old: &Glue, new: &Glue) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if (&old.mesh_a, &old.mesh_b) != (&new.mesh_a, &new.mesh_b) {
        out.push(format!(
            "meshes {} + {} → {} + {}",
            old.mesh_a, old.mesh_b, new.mesh_a, new.mesh_b
        ));
    }
    if old.pairs.len() == new.pairs.len() {
        let n = old
            .pairs
            .iter()
            .zip(&new.pairs)
            .filter(|(a, b)| a != b)
            .count();
        if n > 0 {
            out.push(format!("{} of vertices", count(n, "pair", "pairs")));
        }
    } else {
        out.push(format!(
            "vertex pairs {} → {}",
            old.pairs.len(),
            new.pairs.len()
        ));
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        in_keyforms(out, pairs, "intensity", |a, b| a != b);
    });
    or_other(out)
}

/// 物理のリグの中身の違い (入力・出力・粒子・正規化・衝突体・座標系の基準)。
pub(super) fn rig_changes(old: &PhysicsRig, new: &PhysicsRig) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(format!("name {} → {}", old.name, new.name));
    }
    listed(&mut out, "inputs", &old.inputs, &new.inputs);
    listed(&mut out, "outputs", &old.outputs, &new.outputs);
    if old.particles.len() == new.particles.len() {
        let n = old
            .particles
            .iter()
            .zip(&new.particles)
            .filter(|(a, b)| a != b)
            .count();
        if n > 0 {
            out.push(format!("settings of {}", count(n, "particle", "particles")));
        }
    } else {
        out.push(format!(
            "particles {} → {}",
            old.particles.len(),
            new.particles.len()
        ));
    }
    if old.normalization != new.normalization {
        out.push("normalization".to_owned());
    }
    let colliders = |c: &[ColliderId]| c.iter().cloned().collect::<BTreeSet<ColliderId>>();
    let (a, b) = (colliders(&old.colliders), colliders(&new.colliders));
    if a != b {
        let added: Vec<&str> = b.difference(&a).map(ColliderId::as_str).collect();
        let removed: Vec<&str> = a.difference(&b).map(ColliderId::as_str).collect();
        if let Some(details) = details(&added, &removed, &[], "changed") {
            out.push(format!("colliders ({details})"));
        }
    }
    if old.frame != new.frame {
        out.push(format!(
            "frame {} → {}",
            or_none(old.frame.as_ref()),
            or_none(new.frame.as_ref())
        ));
    }
    or_other(out)
}

/// 並び (入力・出力) の違い。数が違えば数、同じなら違うものの数。
fn listed<T: PartialEq>(out: &mut Vec<String>, what: &str, old: &[T], new: &[T]) {
    if old.len() == new.len() {
        let n = old.iter().zip(new).filter(|(a, b)| a != b).count();
        if n > 0 {
            out.push(format!("{n} {what}"));
        }
    } else {
        out.push(format!("{what} {} → {}", old.len(), new.len()));
    }
}

/// 衝突体の中身の違い (名前・取り付け先・形・位置・半径)。
pub(super) fn collider_changes(old: &PhysicsCollider, new: &PhysicsCollider) -> Vec<String> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if old.name != new.name {
        out.push(format!("name {} → {}", old.name, new.name));
    }
    if old.attach != new.attach {
        out.push(format!(
            "attached to {} → {}",
            or_none(old.attach.as_ref()),
            or_none(new.attach.as_ref())
        ));
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
                out.push("position".to_owned());
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
                out.push("position".to_owned());
            }
            radius(&mut out, *r0, *r1);
        }
        (a, b) => out.push(format!("shape {} → {}", shape_name(a), shape_name(b))),
    }
    or_other(out)
}

fn radius(out: &mut Vec<String>, old: f32, new: f32) {
    if old != new {
        out.push(format!("radius {} → {}", number(old), number(new)));
    }
}

fn shape_name(shape: &ColliderShape) -> &'static str {
    match shape {
        ColliderShape::Circle { .. } => "circle",
        ColliderShape::Capsule { .. } => "capsule",
        ColliderShape::Unknown => "unknown",
    }
}

/// メッシュの中身の違い (頂点・三角形・キーフォーム・描画順・テクスチャなど)。テクスチャは名前で比べる。
/// 描画順は `map` で写して書く。
pub(super) fn mesh_changes(
    (old, old_texture): (&Mesh, Option<&str>),
    (new, new_texture): (&Mesh, Option<&str>),
    map: &dyn Fn(f32) -> f32,
) -> Vec<String> {
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
        out.push(format!(
            "texture {} → {}",
            old_texture.unwrap_or("none"),
            new_texture.unwrap_or("none")
        ));
    }
    let same_vertices = old.uvs.len() == new.uvs.len();
    if same_vertices {
        let n = old.uvs.iter().zip(&new.uvs).filter(|(a, b)| a != b).count();
        if n > 0 {
            out.push(format!("UVs of {}", count(n, "vertex", "vertices")));
        }
    } else {
        out.push(format!("vertices {} → {}", old.uvs.len(), new.uvs.len()));
    }
    if old.indices != new.indices {
        out.push(if old.indices.len() == new.indices.len() {
            "triangles".to_owned()
        } else {
            format!(
                "triangles {} → {}",
                old.indices.len() / 3,
                new.indices.len() / 3
            )
        });
    }
    keyform_changes(&mut out, &old.keyforms, &new.keyforms, |pairs, out| {
        if same_vertices {
            in_keyforms(out, pairs, "vertex positions", |a, b| {
                a.positions != b.positions
            });
        }
        draw_order(out, pairs, map);
        in_keyforms(out, pairs, "opacity or colors", |a, b| {
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
        out.push("blending or masks".to_owned());
    }
    if old.mirror != new.mirror
        || old.mirror_axis != new.mirror_axis
        || old.depth != new.depth
        || old.lit != new.lit
    {
        out.push("other settings".to_owned());
    }
    or_other(out)
}

/// 描画順の違い。どのキーフォームでも同じ値なら `draw order 500 → 520`、そうでなければ変わったキーフォームの数。
fn draw_order(
    out: &mut Vec<String>,
    pairs: &[(&MeshKeyform, &MeshKeyform)],
    map: &dyn Fn(f32) -> f32,
) {
    let n = pairs
        .iter()
        .filter(|(a, b)| a.draw_order != b.draw_order)
        .count();
    if n == 0 {
        return;
    }
    let single = |values: Vec<f32>| {
        let first = values[0];
        values.iter().all(|&v| v == first).then_some(first)
    };
    let olds = pairs.iter().map(|(a, _)| a.draw_order).collect();
    let news = pairs.iter().map(|(_, b)| b.draw_order).collect();
    match (single(olds), single(news)) {
        (Some(a), Some(b)) => out.push(format!(
            "draw order {} → {}",
            number(map(a)),
            number(map(b))
        )),
        _ => out.push(format!("draw order in {}", count(n, "keyform", "keyforms"))),
    }
}

/// 表示用の数 (写したときの丸めのずれを出さないように、小数 2 桁まで)。
fn number(value: f32) -> String {
    let rounded = (f64::from(value) * 100.0).round() / 100.0;
    format!("{rounded}")
}
