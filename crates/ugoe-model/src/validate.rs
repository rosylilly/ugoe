//! モデルの構造的整合性チェック。ランタイム・エディタ・AI 出力の受け入れ時に使う。
//! 検査項目は `docs/spec/model.md#検証`。

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;

use crate::deformer::DeformerKind;
use crate::{KeyformGrid, Model};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("duplicate {kind} id `{id}`")]
    DuplicateId { kind: &'static str, id: String },
    #[error("{kind} `{id}`: keyform count {actual} != expected {expected}")]
    KeyformCount {
        kind: &'static str,
        id: String,
        expected: usize,
        actual: usize,
    },
    #[error("{kind} `{id}`: keys of parameter `{parameter}` are not strictly ascending: {keys:?}")]
    KeysNotAscending {
        kind: &'static str,
        id: String,
        parameter: String,
        keys: Vec<f32>,
    },
    #[error("{kind} `{id}`: binding of parameter `{parameter}` has no keys")]
    EmptyKeys {
        kind: &'static str,
        id: String,
        parameter: String,
    },
    #[error("{kind} `{id}`: parameter `{parameter}` is bound more than once")]
    DuplicateBinding {
        kind: &'static str,
        id: String,
        parameter: String,
    },
    #[error("mesh `{id}`: indices length {len} is not a multiple of 3")]
    IndicesNotTriangles { id: String, len: usize },
    #[error("mesh `{id}`: index {index} out of range (vertices = {vertices})")]
    IndexOutOfRange {
        id: String,
        index: u16,
        vertices: usize,
    },
    #[error("mesh `{id}`: keyform has {actual} positions, expected {expected}")]
    VertexCount {
        id: String,
        expected: usize,
        actual: usize,
    },
    #[error("warp deformer `{id}`: keyform has {actual} control points, expected {expected}")]
    WarpPointCount {
        id: String,
        expected: usize,
        actual: usize,
    },
    #[error("warp deformer `{id}`: rows ({rows}) and cols ({cols}) must both be at least 1")]
    EmptyWarpGrid { id: String, rows: u16, cols: u16 },
    /// `field` は参照元のフィールド名 (`part`, `parent`, `masks` など)。
    #[error("{kind} `{id}`: {field} refers to unknown {target_kind} `{target}`")]
    UnknownReference {
        kind: &'static str,
        id: String,
        field: &'static str,
        target_kind: &'static str,
        target: String,
    },
    #[error("mesh `{id}`: texture index {texture} out of range (textures = {textures})")]
    TextureOutOfRange {
        id: String,
        texture: u32,
        textures: usize,
    },
    #[error("glue `{id}`: vertex {index} of mesh `{mesh}` out of range (vertices = {vertices})")]
    GlueVertexOutOfRange {
        id: String,
        mesh: String,
        index: u16,
        vertices: usize,
    },
    /// `cycle` は循環を構成する要素の ID (子 → 親の順)。
    #[error("{kind} parent chain has a cycle: {cycle:?}")]
    ParentCycle {
        kind: &'static str,
        cycle: Vec<String>,
    },
    /// `cycle` は循環を構成するメッシュの ID (マスクされる側 → マスクの順)。
    #[error("mesh masks have a cycle: {cycle:?}")]
    MaskCycle { cycle: Vec<String> },
    #[error("parameter `{id}`: expected min <= default <= max, got {min} / {default} / {max}")]
    ParameterRange {
        id: String,
        min: f32,
        default: f32,
        max: f32,
    },
    /// 同じパーツがポーズ全体 (全グループの `part` と `links`) で 2 回以上現れた。
    #[error("part `{part}` appears more than once in pose groups")]
    DuplicatePosePart { part: String },
    /// `field` は値の場所 (`pose.fade_duration`、`effects.breath[0].cycle` など)。
    #[error("{field} = {value} must be {expected}")]
    InvalidValue {
        field: String,
        value: f32,
        expected: &'static str,
    },
}

/// 検出したエラーを全て返す (最初の 1 件で止まらない)。
pub fn validate(model: &Model) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let params = collect_ids(
        "parameter",
        model.parameters.iter().map(|p| &p.id),
        &mut errors,
    );
    let groups = collect_ids(
        "parameter group",
        model.parameter_groups.iter().map(|g| &g.id),
        &mut errors,
    );
    let parts = collect_ids("part", model.parts.iter().map(|p| &p.id), &mut errors);
    let deformers = collect_ids(
        "deformer",
        model.deformers.iter().map(|d| &d.id),
        &mut errors,
    );
    let meshes = collect_ids("mesh", model.meshes.iter().map(|m| &m.id), &mut errors);
    collect_ids("glue", model.glues.iter().map(|g| &g.id), &mut errors);
    collect_ids(
        "hit area",
        model.hit_areas.iter().map(|h| &h.id),
        &mut errors,
    );

    for param in &model.parameters {
        let id = param.id.to_string();
        if !(param.min <= param.default && param.default <= param.max) {
            errors.push(ValidationError::ParameterRange {
                id: id.clone(),
                min: param.min,
                default: param.default,
                max: param.max,
            });
        }
        let refs = Refs {
            kind: "parameter",
            id: &id,
        };
        if let Some(mirror) = &param.mirror {
            refs.check(&params, "mirror", "parameter", mirror, &mut errors);
        }
        if let Some(group) = &param.group {
            refs.check(&groups, "group", "parameter group", group, &mut errors);
        }
    }

    for part in &model.parts {
        let id = part.id.to_string();
        let refs = Refs {
            kind: "part",
            id: &id,
        };
        if let Some(parent) = &part.parent {
            refs.check(&parts, "parent", "part", parent, &mut errors);
        }
        check_grid(&refs, &params, &part.keyforms, &mut errors);
    }
    check_cycles(
        "part",
        &model.parts,
        |p| &p.id,
        |p| p.parent.as_ref(),
        &parts,
        &mut errors,
    );

    for deformer in &model.deformers {
        let id = deformer.id.to_string();
        let refs = Refs {
            kind: "deformer",
            id: &id,
        };
        refs.check(&parts, "part", "part", &deformer.part, &mut errors);
        if let Some(parent) = &deformer.parent {
            refs.check(&deformers, "parent", "deformer", parent, &mut errors);
        }
        match &deformer.kind {
            DeformerKind::Warp(warp) => {
                check_grid(&refs, &params, &warp.keyforms, &mut errors);
                if warp.rows == 0 || warp.cols == 0 {
                    errors.push(ValidationError::EmptyWarpGrid {
                        id: id.clone(),
                        rows: warp.rows,
                        cols: warp.cols,
                    });
                }
                let expected = (usize::from(warp.rows) + 1) * (usize::from(warp.cols) + 1);
                for kf in &warp.keyforms.keyforms {
                    if kf.points.len() != expected {
                        errors.push(ValidationError::WarpPointCount {
                            id: id.clone(),
                            expected,
                            actual: kf.points.len(),
                        });
                    }
                }
            }
            DeformerKind::Rotation(rotation) => {
                check_grid(&refs, &params, &rotation.keyforms, &mut errors);
            }
        }
    }
    check_cycles(
        "deformer",
        &model.deformers,
        |d| &d.id,
        |d| d.parent.as_ref(),
        &deformers,
        &mut errors,
    );

    for mesh in &model.meshes {
        let id = mesh.id.to_string();
        let refs = Refs {
            kind: "mesh",
            id: &id,
        };
        refs.check(&parts, "part", "part", &mesh.part, &mut errors);
        if let Some(parent) = &mesh.parent {
            refs.check(&deformers, "parent", "deformer", parent, &mut errors);
        }
        for mask in &mesh.masks {
            refs.check(&meshes, "masks", "mesh", &mask.mesh, &mut errors);
        }
        if usize::try_from(mesh.texture).map_or(true, |t| t >= model.textures.len()) {
            errors.push(ValidationError::TextureOutOfRange {
                id: id.clone(),
                texture: mesh.texture,
                textures: model.textures.len(),
            });
        }
        check_grid(&refs, &params, &mesh.keyforms, &mut errors);
        if mesh.indices.len() % 3 != 0 {
            errors.push(ValidationError::IndicesNotTriangles {
                id: id.clone(),
                len: mesh.indices.len(),
            });
        }
        let vertices = mesh.uvs.len();
        if let Some(&index) = mesh.indices.iter().find(|&&i| usize::from(i) >= vertices) {
            errors.push(ValidationError::IndexOutOfRange {
                id: id.clone(),
                index,
                vertices,
            });
        }
        for (k, kf) in mesh.keyforms.keyforms.iter().enumerate() {
            if kf.positions.len() != vertices {
                errors.push(ValidationError::VertexCount {
                    id: id.clone(),
                    expected: vertices,
                    actual: kf.positions.len(),
                });
            }
            // 0〜1 へのクランプはランタイムが行うので、ここでは有限値であることだけを見る。
            if !kf.mask_strength.is_finite() {
                errors.push(ValidationError::InvalidValue {
                    field: format!("meshes[{id}].keyforms[{k}].mask_strength"),
                    value: kf.mask_strength,
                    expected: "finite",
                });
            }
        }
    }
    check_mask_cycles(model, &meshes, &mut errors);

    for glue in &model.glues {
        let refs = Refs {
            kind: "glue",
            id: &glue.id,
        };
        let a = refs.check(&meshes, "mesh_a", "mesh", &glue.mesh_a, &mut errors);
        let b = refs.check(&meshes, "mesh_b", "mesh", &glue.mesh_b, &mut errors);
        check_grid(&refs, &params, &glue.keyforms, &mut errors);
        // 参照先メッシュが存在する側だけ頂点インデックスを検査する。
        let sides = [
            (
                a,
                &glue.mesh_a,
                glue.pairs.iter().map(|p| p.a).collect::<Vec<_>>(),
            ),
            (b, &glue.mesh_b, glue.pairs.iter().map(|p| p.b).collect()),
        ];
        for (mesh_index, mesh_id, indices) in sides {
            let Some(mesh_index) = mesh_index else {
                continue;
            };
            let vertices = model.meshes[mesh_index].uvs.len();
            if let Some(&index) = indices.iter().find(|&&i| usize::from(i) >= vertices) {
                errors.push(ValidationError::GlueVertexOutOfRange {
                    id: glue.id.clone(),
                    mesh: mesh_id.to_string(),
                    index,
                    vertices,
                });
            }
        }
    }

    for hit_area in &model.hit_areas {
        let refs = Refs {
            kind: "hit area",
            id: &hit_area.id,
        };
        refs.check(&meshes, "mesh", "mesh", &hit_area.mesh, &mut errors);
    }

    check_pose(model, &parts, &mut errors);
    check_effects(model, &params, &mut errors);

    errors
}

/// ポーズの検査: パーツの参照、パーツの重複、フェード時間。
fn check_pose(
    model: &Model,
    parts: &HashMap<&crate::PartId, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let pose = &model.pose;
    check_non_negative("pose.fade_duration", pose.fade_duration, errors);
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for (index, group) in pose.groups.iter().enumerate() {
        let id = index.to_string();
        let refs = Refs {
            kind: "pose group",
            id: &id,
        };
        for entry in &group.entries {
            let targets = std::iter::once(("part", &entry.part))
                .chain(entry.links.iter().map(|link| ("links", link)));
            for (field, part) in targets {
                refs.check(parts, field, "part", part, errors);
                if !seen.insert(part) && reported.insert(part) {
                    errors.push(ValidationError::DuplicatePosePart {
                        part: part.to_string(),
                    });
                }
            }
        }
    }
}

/// エフェクトの検査: 対象パラメータの参照と時間・周期の値。
fn check_effects(
    model: &Model,
    params: &HashMap<&crate::ParamId, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let effects = &model.effects;

    let blink = &effects.eye_blink;
    let refs = Refs {
        kind: "effect",
        id: "eye_blink",
    };
    for param in &blink.parameters {
        refs.check(params, "parameters", "parameter", param, errors);
    }
    check_positive("effects.eye_blink.interval", blink.interval, errors);
    check_non_negative("effects.eye_blink.closing", blink.closing, errors);
    check_non_negative("effects.eye_blink.closed", blink.closed, errors);
    check_non_negative("effects.eye_blink.opening", blink.opening, errors);

    let refs = Refs {
        kind: "effect",
        id: "breath",
    };
    for (index, breath) in effects.breath.parameters.iter().enumerate() {
        refs.check(params, "parameters", "parameter", &breath.parameter, errors);
        check_positive(
            &format!("effects.breath.parameters[{index}].cycle"),
            breath.cycle,
            errors,
        );
    }

    let refs = Refs {
        kind: "effect",
        id: "lip_sync",
    };
    for param in &effects.lip_sync.parameters {
        refs.check(params, "parameters", "parameter", param, errors);
    }
}

/// 0 以上の有限値であること。
fn check_non_negative(field: &str, value: f32, errors: &mut Vec<ValidationError>) {
    if !(value.is_finite() && value >= 0.0) {
        errors.push(ValidationError::InvalidValue {
            field: field.to_owned(),
            value,
            expected: "finite and >= 0",
        });
    }
}

/// 0 より大きい有限値であること。
fn check_positive(field: &str, value: f32, errors: &mut Vec<ValidationError>) {
    if !(value.is_finite() && value > 0.0) {
        errors.push(ValidationError::InvalidValue {
            field: field.to_owned(),
            value,
            expected: "finite and > 0",
        });
    }
}

/// ID → 配列インデックスの表を作り、重複 ID を報告する (重複時は最初の要素を採る)。
fn collect_ids<'a, K: Eq + Hash + Display + 'a>(
    kind: &'static str,
    ids: impl Iterator<Item = &'a K>,
    errors: &mut Vec<ValidationError>,
) -> HashMap<&'a K, usize> {
    let mut map = HashMap::new();
    let mut reported = HashSet::new();
    for (index, id) in ids.enumerate() {
        if map.contains_key(id) {
            if reported.insert(id) {
                errors.push(ValidationError::DuplicateId {
                    kind,
                    id: id.to_string(),
                });
            }
        } else {
            map.insert(id, index);
        }
    }
    map
}

/// 参照元 (エラーメッセージ用)。
struct Refs<'a> {
    kind: &'static str,
    id: &'a str,
}

impl Refs<'_> {
    /// `target` が `table` にあればそのインデックスを返し、なければエラーを積む。
    fn check<K: Eq + Hash + Display>(
        &self,
        table: &HashMap<&K, usize>,
        field: &'static str,
        target_kind: &'static str,
        target: &K,
        errors: &mut Vec<ValidationError>,
    ) -> Option<usize> {
        let found = table.get(target).copied();
        if found.is_none() {
            errors.push(ValidationError::UnknownReference {
                kind: self.kind,
                id: self.id.to_owned(),
                field,
                target_kind,
                target: target.to_string(),
            });
        }
        found
    }
}

/// キーフォーム格子の共通検査: キーフォーム数、キーが空でないこと・昇順・重複なし、パラメータの参照と重複。
fn check_grid<T>(
    refs: &Refs<'_>,
    params: &HashMap<&crate::ParamId, usize>,
    grid: &KeyformGrid<T>,
    errors: &mut Vec<ValidationError>,
) {
    let expected = grid.expected_len();
    if grid.keyforms.len() != expected {
        errors.push(ValidationError::KeyformCount {
            kind: refs.kind,
            id: refs.id.to_owned(),
            expected,
            actual: grid.keyforms.len(),
        });
    }
    let mut seen = HashSet::new();
    for binding in &grid.bindings {
        refs.check(
            params,
            "keyform binding",
            "parameter",
            &binding.parameter,
            errors,
        );
        if !seen.insert(&binding.parameter) {
            errors.push(ValidationError::DuplicateBinding {
                kind: refs.kind,
                id: refs.id.to_owned(),
                parameter: binding.parameter.to_string(),
            });
        }
        if binding.keys.is_empty() {
            errors.push(ValidationError::EmptyKeys {
                kind: refs.kind,
                id: refs.id.to_owned(),
                parameter: binding.parameter.to_string(),
            });
        }
        // NaN を含む比較 (`partial_cmp` が `None`) も不正とする。
        if binding
            .keys
            .windows(2)
            .any(|w| w[0].partial_cmp(&w[1]) != Some(Ordering::Less))
        {
            errors.push(ValidationError::KeysNotAscending {
                kind: refs.kind,
                id: refs.id.to_owned(),
                parameter: binding.parameter.to_string(),
                keys: binding.keys.clone(),
            });
        }
    }
}

/// 親をたどる連鎖に循環があれば、循環ごとに 1 件報告する。未定義の親は参照検査に任せて無視する。
fn check_cycles<T, K: Eq + Hash + Display>(
    kind: &'static str,
    items: &[T],
    id_of: impl Fn(&T) -> &K,
    parent_of: impl Fn(&T) -> Option<&K>,
    table: &HashMap<&K, usize>,
    errors: &mut Vec<ValidationError>,
) {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unvisited,
        OnPath,
        Done,
    }
    let mut state = vec![State::Unvisited; items.len()];
    for start in 0..items.len() {
        let mut path = Vec::new();
        let mut current = Some(start);
        while let Some(index) = current {
            match state[index] {
                State::Done => break,
                State::OnPath => {
                    let begin = path.iter().position(|&i| i == index).unwrap_or(0);
                    errors.push(ValidationError::ParentCycle {
                        kind,
                        cycle: path[begin..]
                            .iter()
                            .map(|&i| id_of(&items[i]).to_string())
                            .collect(),
                    });
                    break;
                }
                State::Unvisited => {
                    state[index] = State::OnPath;
                    path.push(index);
                    current = parent_of(&items[index]).and_then(|p| table.get(p).copied());
                }
            }
        }
        for index in path {
            state[index] = State::Done;
        }
    }
}

/// マスクの参照の循環 (自分自身を含む) を報告する。未定義の参照は参照検査に任せて無視する。
///
/// 宣言順・`masks` の並び順の深さ優先探索で、後退辺 1 本につき 1 件報告する。
/// 同じ循環を重複した参照でたどった場合は 1 件にまとめる。
fn check_mask_cycles(
    model: &Model,
    table: &HashMap<&crate::MeshId, usize>,
    errors: &mut Vec<ValidationError>,
) {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unvisited,
        OnPath,
        Done,
    }
    let edges: Vec<Vec<usize>> = model
        .meshes
        .iter()
        .map(|m| {
            m.masks
                .iter()
                .filter_map(|r| table.get(&r.mesh).copied())
                .collect()
        })
        .collect();
    let mut state = vec![State::Unvisited; edges.len()];
    let mut reported = HashSet::new();
    for start in 0..edges.len() {
        if state[start] != State::Unvisited {
            continue;
        }
        // (メッシュ, 次に見る辺の番号) のスタック。スタック上のメッシュが現在の経路。
        let mut stack = vec![(start, 0usize)];
        state[start] = State::OnPath;
        while let Some(top) = stack.last_mut() {
            let node = top.0;
            let Some(&target) = edges[node].get(top.1) else {
                state[node] = State::Done;
                stack.pop();
                continue;
            };
            top.1 += 1;
            match state[target] {
                State::Done => {}
                State::OnPath => {
                    let begin = stack.iter().position(|&(n, _)| n == target).unwrap_or(0);
                    let cycle: Vec<usize> = stack[begin..].iter().map(|&(n, _)| n).collect();
                    if reported.insert(cycle.clone()) {
                        errors.push(ValidationError::MaskCycle {
                            cycle: cycle
                                .iter()
                                .map(|&n| model.meshes[n].id.to_string())
                                .collect(),
                        });
                    }
                }
                State::Unvisited => {
                    state[target] = State::OnPath;
                    stack.push((target, 0));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe_core::Vec2;

    use super::*;
    use crate::deformer::{Deformer, RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform};
    use crate::fixtures::{pose_and_effects, single_quad};
    use crate::mesh::{Glue, GlueKeyform, GluePair, MaskRef};
    use crate::part::{Part, PartKeyform};
    use crate::pose::{PoseEntry, PoseGroup};
    use crate::{Effects, HitArea, KeyformBinding, ParameterGroup, TextureRef};

    fn part(id: &str, parent: Option<&str>) -> Part {
        Part {
            id: id.into(),
            name: id.into(),
            parent: parent.map(Into::into),
            keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        }
    }

    fn rotation(id: &str, parent: Option<&str>) -> Deformer {
        Deformer {
            id: id.into(),
            name: id.into(),
            part: "Root".into(),
            parent: parent.map(Into::into),
            kind: DeformerKind::Rotation(RotationDeformer {
                keyforms: KeyformGrid::constant(RotationKeyform {
                    origin: Vec2::new(0.0, 0.0),
                    angle: 0.0,
                    scale: 1.0,
                    reflect_x: false,
                    reflect_y: false,
                    opacity: 1.0,
                }),
            }),
        }
    }

    fn warp(id: &str, rows: u16, cols: u16, points: usize) -> Deformer {
        Deformer {
            id: id.into(),
            name: id.into(),
            part: "Root".into(),
            parent: None,
            kind: DeformerKind::Warp(WarpDeformer {
                rows,
                cols,
                keyforms: KeyformGrid::constant(WarpKeyform {
                    points: vec![Vec2::new(0.0, 0.0); points],
                    opacity: 1.0,
                }),
            }),
        }
    }

    fn glue(mesh_a: &str, mesh_b: &str, pairs: Vec<GluePair>) -> Glue {
        Glue {
            id: "G".into(),
            mesh_a: mesh_a.into(),
            mesh_b: mesh_b.into(),
            pairs,
            keyforms: KeyformGrid::constant(GlueKeyform { intensity: 1.0 }),
        }
    }

    fn unknown(
        kind: &'static str,
        id: &str,
        field: &'static str,
        target_kind: &'static str,
        target: &str,
    ) -> ValidationError {
        ValidationError::UnknownReference {
            kind,
            id: id.into(),
            field,
            target_kind,
            target: target.into(),
        }
    }

    #[test]
    fn valid_model_with_all_kinds_has_no_errors() {
        let mut model = single_quad();
        model.parameter_groups.push(ParameterGroup {
            id: "Face".into(),
            name: "Face".into(),
        });
        model.parameters[0].group = Some("Face".into());
        model.parameters[0].mirror = Some("X".into());
        model.parts.push(part("Child", Some("Root")));
        model.deformers.push(warp("W", 1, 2, 6));
        model.deformers.push(rotation("R", Some("W")));
        model.meshes[0].parent = Some("R".into());
        let mut mask = model.meshes[0].clone();
        mask.id = "Mask".into();
        model.meshes.push(mask);
        model.meshes[0].masks.push(MaskRef::normal("Mask"));
        model.glues.push(glue(
            "Quad",
            "Mask",
            vec![GluePair {
                a: 3,
                b: 0,
                weight: 0.5,
            }],
        ));
        model.hit_areas.push(HitArea {
            id: "Body".into(),
            mesh: "Quad".into(),
        });
        assert_eq!(validate(&model), vec![]);
    }

    #[test]
    fn duplicate_ids() {
        let mut model = single_quad();
        let quad = model.meshes[0].clone();
        model.meshes.push(quad.clone());
        model.meshes.push(quad);
        model.parts.push(part("Root", None));
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::DuplicateId {
                    kind: "part",
                    id: "Root".into()
                },
                ValidationError::DuplicateId {
                    kind: "mesh",
                    id: "Quad".into()
                },
            ]
        );
    }

    #[test]
    fn keyform_count_mismatch_in_part() {
        let mut model = single_quad();
        model.parts[0]
            .keyforms
            .keyforms
            .push(PartKeyform { opacity: 0.5 });
        assert_eq!(
            validate(&model),
            vec![ValidationError::KeyformCount {
                kind: "part",
                id: "Root".into(),
                expected: 1,
                actual: 2,
            }]
        );
    }

    #[test]
    fn keyform_count_mismatch_in_deformer_and_glue() {
        let mut model = single_quad();
        let mut d = rotation("R", None);
        if let DeformerKind::Rotation(r) = &mut d.kind {
            r.keyforms.keyforms.clear();
        }
        model.deformers.push(d);
        let mut g = glue("Quad", "Quad", vec![]);
        g.keyforms.bindings.push(KeyformBinding {
            parameter: "X".into(),
            keys: vec![0.0, 1.0],
        });
        model.glues.push(g);
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::KeyformCount {
                    kind: "deformer",
                    id: "R".into(),
                    expected: 1,
                    actual: 0,
                },
                ValidationError::KeyformCount {
                    kind: "glue",
                    id: "G".into(),
                    expected: 2,
                    actual: 1,
                },
            ]
        );
    }

    #[test]
    fn keys_must_be_strictly_ascending() {
        for keys in [
            vec![1.0, 0.0, -1.0],
            vec![-1.0, 0.0, 0.0],
            vec![-1.0, f32::NAN, 1.0],
        ] {
            let mut model = single_quad();
            model.meshes[0].keyforms.bindings[0].keys = keys.clone();
            let errors = validate(&model);
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert!(
                matches!(
                    &errors[0],
                    ValidationError::KeysNotAscending { kind: "mesh", id, parameter, .. }
                        if id == "Quad" && parameter == "X"
                ),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn keys_checked_in_warp_deformer() {
        let mut model = single_quad();
        let mut d = warp("W", 1, 1, 4);
        if let DeformerKind::Warp(w) = &mut d.kind {
            w.keyforms.bindings.push(KeyformBinding {
                parameter: "X".into(),
                keys: vec![1.0, 1.0],
            });
            w.keyforms.keyforms.push(w.keyforms.keyforms[0].clone());
        }
        model.deformers.push(d);
        assert_eq!(
            validate(&model),
            vec![ValidationError::KeysNotAscending {
                kind: "deformer",
                id: "W".into(),
                parameter: "X".into(),
                keys: vec![1.0, 1.0],
            }]
        );
    }

    #[test]
    fn binding_without_keys() {
        let mut model = single_quad();
        model.meshes[0].keyforms.bindings[0].keys.clear();
        model.meshes[0].keyforms.keyforms.clear();
        assert_eq!(
            validate(&model),
            vec![ValidationError::EmptyKeys {
                kind: "mesh",
                id: "Quad".into(),
                parameter: "X".into(),
            }]
        );
    }

    #[test]
    fn duplicate_binding_parameter() {
        let mut model = single_quad();
        model.parts[0].keyforms = KeyformGrid {
            bindings: vec![
                KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![0.0],
                },
                KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![1.0],
                },
            ],
            keyforms: vec![PartKeyform { opacity: 1.0 }],
        };
        assert_eq!(
            validate(&model),
            vec![ValidationError::DuplicateBinding {
                kind: "part",
                id: "Root".into(),
                parameter: "X".into(),
            }]
        );
    }

    #[test]
    fn warp_point_count() {
        let mut model = single_quad();
        model.deformers.push(warp("W", 2, 3, 11));
        assert_eq!(
            validate(&model),
            vec![ValidationError::WarpPointCount {
                id: "W".into(),
                expected: 12,
                actual: 11,
            }]
        );
    }

    #[test]
    fn warp_grid_needs_at_least_one_cell() {
        let mut model = single_quad();
        model.deformers.push(warp("W", 0, 3, 4));
        assert_eq!(
            validate(&model),
            vec![ValidationError::EmptyWarpGrid {
                id: "W".into(),
                rows: 0,
                cols: 3,
            }]
        );
    }

    #[test]
    fn unknown_mesh_references() {
        let mut model = single_quad();
        let mesh = &mut model.meshes[0];
        mesh.part = "Nope".into();
        mesh.parent = Some("NoDeformer".into());
        mesh.masks.push(MaskRef::inverted("NoMask"));
        mesh.keyforms.bindings[0].parameter = "Y".into();
        assert_eq!(
            validate(&model),
            vec![
                unknown("mesh", "Quad", "part", "part", "Nope"),
                unknown("mesh", "Quad", "parent", "deformer", "NoDeformer"),
                unknown("mesh", "Quad", "masks", "mesh", "NoMask"),
                unknown("mesh", "Quad", "keyform binding", "parameter", "Y"),
            ]
        );
    }

    #[test]
    fn unknown_part_and_deformer_references() {
        let mut model = single_quad();
        model.parts.push(part("Child", Some("NoPart")));
        let mut d = rotation("R", Some("NoParent"));
        d.part = "NoPart".into();
        model.deformers.push(d);
        assert_eq!(
            validate(&model),
            vec![
                unknown("part", "Child", "parent", "part", "NoPart"),
                unknown("deformer", "R", "part", "part", "NoPart"),
                unknown("deformer", "R", "parent", "deformer", "NoParent"),
            ]
        );
    }

    #[test]
    fn unknown_parameter_mirror_and_group() {
        let mut model = single_quad();
        model.parameters[0].mirror = Some("NoParam".into());
        model.parameters[0].group = Some("NoGroup".into());
        assert_eq!(
            validate(&model),
            vec![
                unknown("parameter", "X", "mirror", "parameter", "NoParam"),
                unknown("parameter", "X", "group", "parameter group", "NoGroup"),
            ]
        );
    }

    #[test]
    fn texture_out_of_range() {
        let mut model = single_quad();
        model.meshes[0].texture = 1;
        assert_eq!(
            validate(&model),
            vec![ValidationError::TextureOutOfRange {
                id: "Quad".into(),
                texture: 1,
                textures: 1,
            }]
        );
        model.textures.push(TextureRef {
            name: "second".into(),
            width: 1,
            height: 1,
        });
        assert_eq!(validate(&model), vec![]);
    }

    #[test]
    fn unknown_glue_meshes() {
        let mut model = single_quad();
        model.glues.push(glue(
            "NoA",
            "NoB",
            vec![GluePair {
                a: 100,
                b: 100,
                weight: 0.5,
            }],
        ));
        // 参照先がない側の頂点インデックスは検査しない。
        assert_eq!(
            validate(&model),
            vec![
                unknown("glue", "G", "mesh_a", "mesh", "NoA"),
                unknown("glue", "G", "mesh_b", "mesh", "NoB"),
            ]
        );
    }

    #[test]
    fn glue_vertex_out_of_range() {
        let mut model = single_quad();
        model.glues.push(glue(
            "Quad",
            "Quad",
            vec![
                GluePair {
                    a: 3,
                    b: 0,
                    weight: 0.5,
                },
                GluePair {
                    a: 4,
                    b: 1,
                    weight: 0.5,
                },
            ],
        ));
        assert_eq!(
            validate(&model),
            vec![ValidationError::GlueVertexOutOfRange {
                id: "G".into(),
                mesh: "Quad".into(),
                index: 4,
                vertices: 4,
            }]
        );
    }

    #[test]
    fn unknown_hit_area_mesh() {
        let mut model = single_quad();
        model.hit_areas.push(HitArea {
            id: "Head".into(),
            mesh: "NoMesh".into(),
        });
        assert_eq!(
            validate(&model),
            vec![unknown("hit area", "Head", "mesh", "mesh", "NoMesh")]
        );
    }

    #[test]
    fn part_cycle() {
        let mut model = single_quad();
        model.parts.push(part("A", Some("B")));
        model.parts.push(part("B", Some("C")));
        model.parts.push(part("C", Some("A")));
        model.parts.push(part("D", Some("A")));
        assert_eq!(
            validate(&model),
            vec![ValidationError::ParentCycle {
                kind: "part",
                cycle: vec!["A".into(), "B".into(), "C".into()],
            }]
        );
    }

    #[test]
    fn mask_cycles() {
        let mut model = single_quad();
        let with_masks = |id: &str, masks: &[MaskRef]| {
            let mut mesh = model.meshes[0].clone();
            mesh.id = id.into();
            mesh.masks = masks.to_vec();
            mesh
        };
        let meshes = [
            // A → B → C → A の循環。C は A を 2 回参照するが報告は 1 件。
            with_masks("A", &[MaskRef::normal("B")]),
            with_masks("B", &[MaskRef::inverted("C"), MaskRef::normal("D")]),
            with_masks("C", &[MaskRef::normal("A"), MaskRef::inverted("A")]),
            // D は循環しない (共有されているだけ)。
            with_masks("D", &[]),
            // E は循環を参照するが、循環の一部ではない。
            with_masks("E", &[MaskRef::normal("A"), MaskRef::normal("NoMask")]),
            // 自分自身をマスクにする。
            with_masks("F", &[MaskRef::inverted("F")]),
        ];
        model.meshes.extend(meshes);
        assert_eq!(
            validate(&model),
            vec![
                unknown("mesh", "E", "masks", "mesh", "NoMask"),
                ValidationError::MaskCycle {
                    cycle: vec!["A".into(), "B".into(), "C".into()],
                },
                ValidationError::MaskCycle {
                    cycle: vec!["F".into()],
                },
            ]
        );
    }

    #[test]
    fn shared_masks_are_not_cycles() {
        // 菱形の依存 (A → B, A → C, B → D, C → D) は循環ではない。
        let mut model = crate::fixtures::nested_masks();
        let mut mesh = model.meshes[0].clone();
        mesh.id = "Extra".into();
        mesh.masks = vec![MaskRef::normal("Pupil"), MaskRef::inverted("Sclera")];
        model.meshes.push(mesh);
        assert_eq!(validate(&model), vec![]);
    }

    #[test]
    fn mask_strength_must_be_finite() {
        let mut model = single_quad();
        model.meshes[0].keyforms.keyforms[1].mask_strength = f32::INFINITY;
        // 範囲外でも有限値ならランタイムがクランプするので許す。
        model.meshes[0].keyforms.keyforms[2].mask_strength = 2.0;
        assert_eq!(
            validate(&model),
            vec![ValidationError::InvalidValue {
                field: "meshes[Quad].keyforms[1].mask_strength".into(),
                value: f32::INFINITY,
                expected: "finite",
            }]
        );
    }

    #[test]
    fn deformer_self_parent_is_cycle() {
        let mut model = single_quad();
        model.deformers.push(rotation("R", Some("R")));
        assert_eq!(
            validate(&model),
            vec![ValidationError::ParentCycle {
                kind: "deformer",
                cycle: vec!["R".into()],
            }]
        );
    }

    #[test]
    fn parameter_range() {
        for (min, default, max) in [(0.0, -1.0, 1.0), (0.0, 2.0, 1.0), (1.0, 0.0, -1.0)] {
            let mut model = single_quad();
            let p = &mut model.parameters[0];
            (p.min, p.default, p.max) = (min, default, max);
            assert_eq!(
                validate(&model),
                vec![ValidationError::ParameterRange {
                    id: "X".into(),
                    min,
                    default,
                    max,
                }]
            );
        }
    }

    fn pose_entry(part: &str, links: &[&str]) -> PoseEntry {
        PoseEntry {
            part: part.into(),
            links: links.iter().map(|&l| l.into()).collect(),
        }
    }

    #[test]
    fn unknown_pose_parts() {
        let mut model = pose_and_effects();
        model.pose.groups.push(PoseGroup {
            entries: vec![pose_entry("NoPart", &["NoLink"])],
        });
        assert_eq!(
            validate(&model),
            vec![
                unknown("pose group", "1", "part", "part", "NoPart"),
                unknown("pose group", "1", "links", "part", "NoLink"),
            ]
        );
    }

    #[test]
    fn part_in_multiple_pose_groups() {
        let mut model = pose_and_effects();
        // 同じグループ内での重複、別グループとの重複、リンクとの重複をそれぞれ 1 回だけ報告する。
        model.pose.groups[0].entries.push(pose_entry("ArmB", &[]));
        model.pose.groups.push(PoseGroup {
            entries: vec![pose_entry("Root", &["SleeveA"]), pose_entry("ArmA", &[])],
        });
        model.pose.groups.push(PoseGroup {
            entries: vec![pose_entry("ArmA", &[])],
        });
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::DuplicatePosePart {
                    part: "ArmB".into()
                },
                ValidationError::DuplicatePosePart {
                    part: "SleeveA".into()
                },
                ValidationError::DuplicatePosePart {
                    part: "ArmA".into()
                },
            ]
        );
    }

    #[test]
    fn unknown_effect_parameters() {
        let mut model = single_quad();
        model.effects = Effects::standard();
        model.effects.breath.parameters.truncate(1);
        assert_eq!(
            validate(&model),
            vec![
                unknown("effect", "eye_blink", "parameters", "parameter", "EyeLOpen"),
                unknown("effect", "eye_blink", "parameters", "parameter", "EyeROpen"),
                unknown("effect", "breath", "parameters", "parameter", "AngleX"),
                unknown(
                    "effect",
                    "lip_sync",
                    "parameters",
                    "parameter",
                    "MouthOpenY"
                ),
            ]
        );
    }

    #[test]
    fn invalid_durations() {
        let invalid =
            |field: &str, value: f32, expected: &'static str| ValidationError::InvalidValue {
                field: field.into(),
                value,
                expected,
            };
        let mut model = pose_and_effects();
        model.pose.fade_duration = -0.5;
        let blink = &mut model.effects.eye_blink;
        blink.interval = 0.0;
        blink.closing = -0.1;
        blink.closed = f32::INFINITY;
        blink.opening = 0.0;
        model.effects.breath.parameters[1].cycle = 0.0;
        let errors = validate(&model);
        assert_eq!(
            errors,
            vec![
                invalid("pose.fade_duration", -0.5, "finite and >= 0"),
                invalid("effects.eye_blink.interval", 0.0, "finite and > 0"),
                invalid("effects.eye_blink.closing", -0.1, "finite and >= 0"),
                invalid("effects.eye_blink.closed", f32::INFINITY, "finite and >= 0"),
                invalid("effects.breath.parameters[1].cycle", 0.0, "finite and > 0"),
            ]
        );

        // NaN は `PartialEq` で比較できないので、フィールド名だけ確かめる。
        let mut model = pose_and_effects();
        model.effects.breath.parameters[0].cycle = f32::NAN;
        let errors = validate(&model);
        assert!(
            matches!(
                errors.as_slice(),
                [ValidationError::InvalidValue { field, .. }]
                    if field == "effects.breath.parameters[0].cycle"
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn reports_all_errors() {
        let mut model = single_quad();
        model.parameters[0].default = 5.0;
        model.meshes[0].part = "Nope".into();
        model.meshes[0].texture = 9;
        model.meshes[0].indices.push(0);
        assert_eq!(validate(&model).len(), 4);
    }
}
