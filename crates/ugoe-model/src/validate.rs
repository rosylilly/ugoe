//! モデルの構造的整合性チェック。ランタイム・エディタ・AI 出力の受け入れ時に使う。
//! 検査項目は `docs/spec/model.md#検証`。

use alloc::borrow::ToOwned;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt::Display;

use ugoe_core::Vec2;

use crate::deformer::DeformerKind;
use crate::effects::Effects;
use crate::parameter::ParameterKind;
use crate::{KeyformGrid, KeyformInference, Model, Parameter};

mod code;
mod describe;
mod module;
mod motion;
mod physics;

pub use describe::{DESCRIBED_ERRORS_MAX, describe_errors};
pub use module::validate_module;
pub use motion::{MotionWarning, motion_warnings, validate_expression, validate_motion};
pub use physics::validate_physics;

/// モデルの構造の検証で見つけたエラー (`docs/spec/model.md#検証`)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    /// 同じ種類の要素で ID が重複している。
    #[error("duplicate {kind} id `{id}`")]
    DuplicateId {
        /// 要素の種類 (`parameter`・`part`・`deformer`・`mesh` など)。
        kind: &'static str,
        /// 重複した ID。
        id: String,
    },
    /// キーフォーム数が、格子の各軸のキー数の積と違う。
    #[error("{kind} `{id}`: keyform count {actual} != expected {expected}")]
    KeyformCount {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// 各軸のキー数の積。
        expected: usize,
        /// 実際のキーフォーム数。
        actual: usize,
    },
    /// パラメータの結び付けのキーが狭義の昇順でない。
    #[error("{kind} `{id}`: keys of parameter `{parameter}` are not strictly ascending: {keys:?}")]
    KeysNotAscending {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// パラメータの ID。
        parameter: String,
        /// 結び付けのキー。
        keys: Vec<f32>,
    },
    /// パラメータの結び付けにキーが無い。
    #[error("{kind} `{id}`: binding of parameter `{parameter}` has no keys")]
    EmptyKeys {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// パラメータの ID。
        parameter: String,
    },
    /// 1 つの格子で同じパラメータを 2 回以上結び付けている。
    #[error("{kind} `{id}`: parameter `{parameter}` is bound more than once")]
    DuplicateBinding {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// パラメータの ID。
        parameter: String,
    },
    /// メッシュの `indices` の長さが 3 の倍数でない。
    #[error("mesh `{id}`: indices length {len} is not a multiple of 3")]
    IndicesNotTriangles {
        /// メッシュの ID。
        id: String,
        /// `indices` の長さ。
        len: usize,
    },
    /// メッシュの `indices` が頂点数の外を指す。
    #[error("mesh `{id}`: index {index} out of range (vertices = {vertices})")]
    IndexOutOfRange {
        /// メッシュの ID。
        id: String,
        /// 範囲外の頂点のインデックス。
        index: u16,
        /// メッシュの頂点数。
        vertices: usize,
    },
    /// メッシュのキーフォームの頂点数が `uvs` の数と違う。
    #[error("mesh `{id}`: keyform has {actual} positions, expected {expected}")]
    VertexCount {
        /// メッシュの ID。
        id: String,
        /// メッシュの頂点数 (`uvs` の数)。
        expected: usize,
        /// キーフォームの頂点の数。
        actual: usize,
    },
    /// ワープデフォーマのキーフォームの制御点の数が `(rows + 1) * (cols + 1)` と違う。
    #[error("warp deformer `{id}`: keyform has {actual} control points, expected {expected}")]
    WarpPointCount {
        /// ワープデフォーマの ID。
        id: String,
        /// 格子から決まる制御点の数。
        expected: usize,
        /// キーフォームの制御点の数。
        actual: usize,
    },
    /// ワープデフォーマの `rows` か `cols` が 0。
    #[error("warp deformer `{id}`: rows ({rows}) and cols ({cols}) must both be at least 1")]
    EmptyWarpGrid {
        /// ワープデフォーマの ID。
        id: String,
        /// 格子の行数。
        rows: u16,
        /// 格子の列数。
        cols: u16,
    },
    /// `field` は参照元のフィールド名 (`part`, `parent`, `masks` など)。
    #[error("{kind} `{id}`: {field} refers to unknown {target_kind} `{target}`")]
    UnknownReference {
        /// 参照元の要素の種類。
        kind: &'static str,
        /// 参照元の要素の ID。
        id: String,
        /// 参照元のフィールド名。
        field: &'static str,
        /// 参照先の要素の種類。
        target_kind: &'static str,
        /// 参照先の ID。
        target: String,
    },
    /// メッシュのテクスチャのインデックスが `Model::textures` の外。
    #[error("mesh `{id}`: texture index {texture} out of range (textures = {textures})")]
    TextureOutOfRange {
        /// メッシュの ID。
        id: String,
        /// 範囲外のテクスチャのインデックス。
        texture: u32,
        /// モデルのテクスチャの数。
        textures: usize,
    },
    /// グルーの頂点のインデックスが、メッシュの頂点数の外。
    #[error("glue `{id}`: vertex {index} of mesh `{mesh}` out of range (vertices = {vertices})")]
    GlueVertexOutOfRange {
        /// グルーの ID。
        id: String,
        /// 頂点を持つメッシュの ID。
        mesh: String,
        /// 範囲外の頂点のインデックス。
        index: u16,
        /// メッシュの頂点数。
        vertices: usize,
    },
    /// `cycle` は循環を構成する要素の ID (子 → 親の順)。
    #[error("{kind} parent chain has a cycle: {cycle:?}")]
    ParentCycle {
        /// 循環した要素の種類 (`part` か `deformer`)。
        kind: &'static str,
        /// 循環を構成する要素の ID (子 → 親の順)。
        cycle: Vec<String>,
    },
    /// `cycle` は循環を構成するメッシュの ID (マスクされる側 → マスクの順)。
    #[error("mesh masks have a cycle: {cycle:?}")]
    MaskCycle {
        /// 循環を構成するメッシュの ID (マスクされる側 → マスクの順)。
        cycle: Vec<String>,
    },
    /// パラメータが `min <= default <= max` を満たさない。
    #[error("parameter `{id}`: expected min <= default <= max, got {min} / {default} / {max}")]
    ParameterRange {
        /// パラメータの ID。
        id: String,
        /// 最小値。
        min: f32,
        /// 既定値。
        default: f32,
        /// 最大値。
        max: f32,
    },
    /// 同じパーツがポーズ全体 (全グループの `part` と `links`) で 2 回以上現れた。
    #[error("part `{part}` appears more than once in pose groups")]
    DuplicatePosePart {
        /// 重複したパーツの ID。
        part: String,
    },
    /// モジュールの名前が空、`/` を含む、または `#` で始まる (`docs/spec/modules.md#モジュール`)。
    #[error("module name `{name}` must be non-empty, contain no `/` and not start with `#`")]
    InvalidModuleName {
        /// 不正なモジュールの名前。
        name: String,
    },
    /// `origins` が空でなく、長さがキーフォーム数と違う。
    #[error("{kind} `{id}`: keyform origins length {actual} != keyform count {expected}")]
    OriginCount {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// キーフォーム数。
        expected: usize,
        /// `origins` の長さ。
        actual: usize,
    },
    /// 深度からの回転の `yaw` / `pitch` のパラメータが、格子の軸になっていない。
    #[error("{kind} `{id}`: inference parameter `{parameter}` is not a keyform binding")]
    UnboundInferenceParameter {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// 格子の軸にないパラメータの ID。
        parameter: String,
    },
    /// `mirror` の対が双方向でない (`id` → `mirror` なのに、`mirror` の `mirror` が `back`)。
    #[error("{kind} `{id}`: mirror `{mirror}` does not mirror back (its mirror is {back:?})")]
    AsymmetricMirror {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// `id` の `mirror` が指す要素の ID。
        mirror: String,
        /// `mirror` の要素の `mirror` (無ければ `None`)。
        back: Option<String>,
    },
    /// 深度の長さが頂点数 (メッシュ) または制御点数 (ワープ) と違う。
    #[error("{kind} `{id}`: depth has {actual} values, expected {expected}")]
    DepthCount {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// 頂点数 (メッシュ) または制御点数 (ワープ)。
        expected: usize,
        /// 深度の値の数。
        actual: usize,
    },
    /// 参照先のデフォーマの種類が、参照元の求める種類 (`expected`) と違う。
    #[error(
        "{kind} `{id}`: {field} refers to deformer `{deformer}`, which is not a {expected} deformer"
    )]
    DeformerKindMismatch {
        /// 参照元の要素の種類。
        kind: &'static str,
        /// 参照元の要素の ID。
        id: String,
        /// 参照元のフィールド名。
        field: &'static str,
        /// 参照先のデフォーマの ID。
        deformer: String,
        /// 求めるデフォーマの種類 (`warp` か `rotation`)。
        expected: &'static str,
    },
    /// 物理の頂点への出力の重みの数が、頂点数 (メッシュ) または制御点数 (ワープ) と違う。
    #[error("physics rig `{rig}`: outputs[{output}] has {actual} weights, expected {expected}")]
    WeightCount {
        /// 物理のリグの ID。
        rig: String,
        /// リグの `outputs` のインデックス。
        output: usize,
        /// 出力先の頂点数 (メッシュ) または制御点数 (ワープ)。
        expected: usize,
        /// 重みの数。
        actual: usize,
    },
    /// 格子の軸のパラメータの種類が、格子の種類と合わない。通常の格子の軸は `blend_shape` 以外、
    /// ブレンドシェイプ格子の軸は `blend_shape` (`expected`)。未知の種類はどちらにも置ける。
    #[error(
        "{kind} `{id}`: parameter `{parameter}` of a keyform binding must be of kind {expected}"
    )]
    ParameterKindMismatch {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// パラメータの ID。
        parameter: String,
        /// 軸に求めるパラメータの種類。
        expected: &'static str,
    },
    /// ブレンドシェイプ格子 (`keyforms.blend_shapes[index]`) の不正。`reason` は理由。
    #[error("{kind} `{id}`: keyforms.blend_shapes[{index}] {reason}")]
    InvalidBlendShape {
        /// 要素の種類 (`mesh`・`deformer`・`part` など)。
        kind: &'static str,
        /// 要素の ID。
        id: String,
        /// `keyforms.blend_shapes` のインデックス。
        index: usize,
        /// 不正の理由。
        reason: &'static str,
    },
    /// `field` は値の場所 (`pose.fade_duration`、`effects.breath[0].cycle` など)。
    #[error("{field} = {value} must be {expected}")]
    InvalidValue {
        /// 値の場所 (`pose.fade_duration`、`effects.breath[0].cycle` など)。
        field: String,
        /// 実際の値。
        value: f32,
        /// 満たすべき条件 (`finite`・`finite and >= 0` など)。
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
    let glues = collect_ids("glue", model.glues.iter().map(|g| &g.id), &mut errors);
    collect_ids(
        "hit area",
        model.hit_areas.iter().map(|h| &h.id),
        &mut errors,
    );

    let canvas = &model.meta.canvas;
    check_finite("meta.canvas.width", canvas.width, &mut errors);
    check_finite("meta.canvas.height", canvas.height, &mut errors);
    check_vec2("meta.canvas.origin", canvas.origin, &mut errors);

    for param in &model.parameters {
        let id = param.id.to_string();
        let range = [
            ("min", param.min),
            ("default", param.default),
            ("max", param.max),
        ];
        let mut finite = true;
        for (name, value) in range {
            finite &= value.is_finite();
            check_finite(&format!("parameters[{id}].{name}"), value, &mut errors);
        }
        // 有限でない値は上で報告したので、順序は有限のときだけ見る。
        if finite && !(param.min <= param.default && param.default <= param.max) {
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
        let exists = |id: &str| parts.contains_key(&crate::PartId::new(id));
        check_grid(
            &refs,
            &keyforms_path(&refs),
            &params,
            &exists,
            &part.keyforms,
            &mut errors,
        );
        let kinds = Kinds::new(&params, &model.parameters, false);
        check_blend_shapes(&refs, &kinds, &exists, &part.keyforms, &mut errors);
        for (k, kf) in part.keyforms.all_keyforms().enumerate() {
            check_finite(
                &format!("parts[{id}].keyforms[{k}].opacity"),
                kf.opacity,
                &mut errors,
            );
        }
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
        let path = format!("deformers[{id}]");
        check_mirror_axis(&path, deformer.mirror_axis, &mut errors);
        let exists = |id: &str| deformers.contains_key(&crate::DeformerId::new(id));
        match &deformer.kind {
            DeformerKind::Warp(warp) => {
                check_grid(
                    &refs,
                    &keyforms_path(&refs),
                    &params,
                    &exists,
                    &warp.keyforms,
                    &mut errors,
                );
                let kinds = Kinds::new(&params, &model.parameters, true);
                check_blend_shapes(&refs, &kinds, &exists, &warp.keyforms, &mut errors);
                let expected = warp_point_count(warp.rows, warp.cols);
                check_depth(&refs, &path, warp.depth.as_deref(), expected, &mut errors);
                if warp.rows == 0 || warp.cols == 0 {
                    errors.push(ValidationError::EmptyWarpGrid {
                        id: id.clone(),
                        rows: warp.rows,
                        cols: warp.cols,
                    });
                }
                for (k, kf) in warp.keyforms.all_keyforms().enumerate() {
                    if kf.points.len() != expected {
                        errors.push(ValidationError::WarpPointCount {
                            id: id.clone(),
                            expected,
                            actual: kf.points.len(),
                        });
                    }
                    let cell = format!("{path}.keyforms[{k}]");
                    check_points(&format!("{cell}.points"), &kf.points, &mut errors);
                    check_finite(&format!("{cell}.opacity"), kf.opacity, &mut errors);
                }
            }
            DeformerKind::Rotation(rotation) => {
                check_grid(
                    &refs,
                    &keyforms_path(&refs),
                    &params,
                    &exists,
                    &rotation.keyforms,
                    &mut errors,
                );
                let kinds = Kinds::new(&params, &model.parameters, true);
                check_blend_shapes(&refs, &kinds, &exists, &rotation.keyforms, &mut errors);
                for (k, kf) in rotation.keyforms.all_keyforms().enumerate() {
                    let cell = format!("{path}.keyforms[{k}]");
                    check_vec2(&format!("{cell}.origin"), kf.origin, &mut errors);
                    check_finite(&format!("{cell}.angle"), kf.angle, &mut errors);
                    check_finite(&format!("{cell}.scale"), kf.scale, &mut errors);
                    check_finite(&format!("{cell}.opacity"), kf.opacity, &mut errors);
                }
            }
            // 未知の種類は中身を持たない (恒等変換として扱う) ので、検査することがない。
            DeformerKind::Unknown => {}
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
    check_mirrors(
        "deformer",
        &model.deformers,
        |d| &d.id,
        |d| d.mirror.as_ref(),
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
        let exists = |id: &str| meshes.contains_key(&crate::MeshId::new(id));
        check_grid(
            &refs,
            &keyforms_path(&refs),
            &params,
            &exists,
            &mesh.keyforms,
            &mut errors,
        );
        let kinds = Kinds::new(&params, &model.parameters, true);
        check_blend_shapes(&refs, &kinds, &exists, &mesh.keyforms, &mut errors);
        let path = format!("meshes[{id}]");
        check_mirror_axis(&path, mesh.mirror_axis, &mut errors);
        check_depth(
            &refs,
            &path,
            mesh.depth.as_deref(),
            mesh.uvs.len(),
            &mut errors,
        );
        if mesh.indices.len() % 3 != 0 {
            errors.push(ValidationError::IndicesNotTriangles {
                id: id.clone(),
                len: mesh.indices.len(),
            });
        }
        check_points(&format!("{path}.uvs"), &mesh.uvs, &mut errors);
        let vertices = mesh.uvs.len();
        if let Some(&index) = mesh.indices.iter().find(|&&i| usize::from(i) >= vertices) {
            errors.push(ValidationError::IndexOutOfRange {
                id: id.clone(),
                index,
                vertices,
            });
        }
        // ブレンドシェイプ格子のセルは、この格子のセルの後に続けて数える。
        for (k, kf) in mesh.keyforms.all_keyforms().enumerate() {
            if kf.positions.len() != vertices {
                errors.push(ValidationError::VertexCount {
                    id: id.clone(),
                    expected: vertices,
                    actual: kf.positions.len(),
                });
            }
            // 値の範囲 (不透明度・`mask_strength` の 0〜1 など) はランタイムがクランプするので、
            // ここでは有限値であることだけを見る。
            let cell = format!("{path}.keyforms[{k}]");
            check_points(&format!("{cell}.positions"), &kf.positions, &mut errors);
            let values = [
                ("opacity", kf.opacity),
                ("draw_order", kf.draw_order),
                ("multiply_color.r", kf.multiply_color.r),
                ("multiply_color.g", kf.multiply_color.g),
                ("multiply_color.b", kf.multiply_color.b),
                ("screen_color.r", kf.screen_color.r),
                ("screen_color.g", kf.screen_color.g),
                ("screen_color.b", kf.screen_color.b),
                ("mask_strength", kf.mask_strength),
            ];
            for (name, value) in values {
                check_finite(&format!("{cell}.{name}"), value, &mut errors);
            }
        }
    }
    check_mirrors(
        "mesh",
        &model.meshes,
        |m| &m.id,
        |m| m.mirror.as_ref(),
        &meshes,
        &mut errors,
    );
    check_mask_cycles(model, &meshes, &mut errors);

    for glue in &model.glues {
        let refs = Refs {
            kind: "glue",
            id: glue.id.as_str(),
        };
        let a = refs.check(&meshes, "mesh_a", "mesh", &glue.mesh_a, &mut errors);
        let b = refs.check(&meshes, "mesh_b", "mesh", &glue.mesh_b, &mut errors);
        let exists = |id: &str| glues.contains_key(&crate::GlueId::new(id));
        check_grid(
            &refs,
            &keyforms_path(&refs),
            &params,
            &exists,
            &glue.keyforms,
            &mut errors,
        );
        let kinds = Kinds::new(&params, &model.parameters, false);
        check_blend_shapes(&refs, &kinds, &exists, &glue.keyforms, &mut errors);
        let path = format!("glues[{}]", glue.id);
        for (k, kf) in glue.keyforms.all_keyforms().enumerate() {
            check_finite(
                &format!("{path}.keyforms[{k}].intensity"),
                kf.intensity,
                &mut errors,
            );
        }
        // 有限でない重みは最初の 1 つだけ報告する。
        if let Some((p, pair)) = glue
            .pairs
            .iter()
            .enumerate()
            .find(|(_, p)| !p.weight.is_finite())
        {
            check_finite(
                &format!("{path}.pairs[{p}].weight"),
                pair.weight,
                &mut errors,
            );
        }
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
                    id: glue.id.to_string(),
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
            id: hit_area.id.as_str(),
        };
        refs.check(&meshes, "mesh", "mesh", &hit_area.mesh, &mut errors);
    }

    check_pose(model, &parts, &mut errors);
    check_effects(&model.effects, &params, &mut errors);
    module::check_slots(model, &params, &parts, &deformers, &meshes, &mut errors);

    errors
}

/// ポーズの検査: パーツの参照、パーツの重複、フェード時間。
fn check_pose(
    model: &Model,
    parts: &BTreeMap<&crate::PartId, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let pose = &model.pose;
    check_non_negative("pose.fade_duration", pose.fade_duration, errors);
    let mut seen = BTreeSet::new();
    let mut reported = BTreeSet::new();
    for (index, group) in pose.groups.iter().enumerate() {
        let id = index.to_string();
        let refs = Refs {
            kind: "pose group",
            id: &id,
        };
        for entry in &group.entries {
            let targets = core::iter::once(("part", &entry.part))
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

/// エフェクトの設定 (目パチ・呼吸・リップシンク) を、参照先のモデルと合わせて検査する。検出したエラーを全て返す。
///
/// [`validate`] のエフェクトの検査と同じ (`docs/spec/model.md#検証`)。モデルの `effects` は見ない。
/// 実行中に設定を差し替える `ugoe::Puppet::try_set_effects` が使う。
pub fn validate_effects(effects: &Effects, model: &Model) -> Vec<ValidationError> {
    // モデル側の重複 ID は `validate` が報告するので、ここでは表を作るだけにする。
    let mut ignored = Vec::new();
    let params = collect_ids(
        "parameter",
        model.parameters.iter().map(|p| &p.id),
        &mut ignored,
    );
    let mut errors = Vec::new();
    check_effects(effects, &params, &mut errors);
    errors
}

/// エフェクトの検査: 対象パラメータの参照と時間・周期の値。
fn check_effects(
    effects: &Effects,
    params: &BTreeMap<&crate::ParamId, usize>,
    errors: &mut Vec<ValidationError>,
) {
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
        let path = format!("effects.breath.parameters[{index}]");
        check_positive(&format!("{path}.cycle"), breath.cycle, errors);
        let values = [
            ("offset", breath.offset),
            ("peak", breath.peak),
            ("weight", breath.weight),
        ];
        for (name, value) in values {
            check_finite(&format!("{path}.{name}"), value, errors);
        }
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
fn collect_ids<'a, K: Ord + Display + 'a>(
    kind: &'static str,
    ids: impl Iterator<Item = &'a K>,
    errors: &mut Vec<ValidationError>,
) -> BTreeMap<&'a K, usize> {
    let mut map = BTreeMap::new();
    let mut reported = BTreeSet::new();
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
    fn check<K: Ord + Display>(
        &self,
        table: &BTreeMap<&K, usize>,
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

/// キーフォーム格子の共通検査: キーフォーム数、キーが空でないこと・昇順・重複なし・有限、パラメータの参照と重複、
/// ミラー元の参照。`same_kind` は格子を持つ要素と同じ種類の要素に、その ID があるか。
/// `path` は値の場所に使う格子の場所 (`meshes[Face].keyforms`、`….keyforms.blend_shapes[0]` など)。
fn check_grid<T>(
    refs: &Refs<'_>,
    path: &str,
    params: &BTreeMap<&crate::ParamId, usize>,
    same_kind: &dyn Fn(&str) -> bool,
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
    let mut seen = BTreeSet::new();
    for (b, binding) in grid.bindings.iter().enumerate() {
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
        // NaN を含む比較 (`partial_cmp` が `None`) も不正とする。キーが 1 つの軸の NaN は比較では見つからないので別に見る。
        if binding.keys.iter().any(|k| k.is_nan())
            || binding
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
        // 無限大のキーは昇順の検査を通るが、区間の幅が無限大になり補間の係数が NaN になる。
        // NaN は上で報告したので、ここでは無限大の最初の 1 つだけを報告する。
        if let Some((k, &key)) = binding
            .keys
            .iter()
            .enumerate()
            .find(|(_, k)| k.is_infinite())
        {
            check_finite(&format!("{path}.bindings[{b}].keys[{k}]"), key, errors);
        }
    }
    if !grid.origins.is_empty() && grid.origins.len() != grid.keyforms.len() {
        errors.push(ValidationError::OriginCount {
            kind: refs.kind,
            id: refs.id.to_owned(),
            expected: grid.keyforms.len(),
            actual: grid.origins.len(),
        });
    }
    if let Some(source) = &grid.mirror_source {
        if !same_kind(&source.id) {
            errors.push(ValidationError::UnknownReference {
                kind: refs.kind,
                id: refs.id.to_owned(),
                field: "keyforms.mirror_source",
                target_kind: refs.kind,
                target: source.id.clone(),
            });
        }
        let path = format!("{path}.mirror_source");
        let values = [("axis", source.axis), ("tolerance", source.tolerance)];
        for (name, value) in values {
            if let Some(value) = value {
                check_finite(&format!("{path}.{name}"), value, errors);
            }
        }
    }
    if let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference {
        let path = format!("{path}.inference");
        let values = [
            ("angle_scale", rotation.angle_scale),
            ("pivot.x", rotation.pivot.x),
            ("pivot.y", rotation.pivot.y),
            ("pivot_depth", rotation.pivot_depth),
        ];
        for (name, value) in values {
            check_finite(&format!("{path}.{name}"), value, errors);
        }
        let axes = [("yaw", &rotation.yaw), ("pitch", &rotation.pitch)];
        for (field, parameter) in axes {
            let Some(parameter) = parameter else {
                continue;
            };
            // 存在しないパラメータは参照の検査だけで報告する。
            if refs
                .check(params, field, "parameter", parameter, errors)
                .is_some()
                && !grid.bindings.iter().any(|b| &b.parameter == parameter)
            {
                errors.push(ValidationError::UnboundInferenceParameter {
                    kind: refs.kind,
                    id: refs.id.to_owned(),
                    parameter: parameter.to_string(),
                });
            }
        }
    }
}

/// ブレンドシェイプ格子の検査に使う、パラメータの種類の表。
struct Kinds<'a> {
    params: &'a BTreeMap<&'a crate::ParamId, usize>,
    parameters: &'a [Parameter],
    /// 要素がブレンドシェイプ格子を持てるか (メッシュ・ワープ・回転デフォーマ)。
    supported: bool,
}

impl<'a> Kinds<'a> {
    fn new(
        params: &'a BTreeMap<&'a crate::ParamId, usize>,
        parameters: &'a [Parameter],
        supported: bool,
    ) -> Self {
        Self {
            params,
            parameters,
            supported,
        }
    }

    /// パラメータの種類。存在しないパラメータは `None` (参照の検査で報告済み)。
    fn of(&self, parameter: &crate::ParamId) -> Option<ParameterKind> {
        self.params.get(parameter).map(|&i| self.parameters[i].kind)
    }
}

/// ブレンドシェイプの検査 (`docs/spec/model.md#ブレンドシェイプ`): 通常の格子の軸に `blend_shape` の
/// パラメータがないこと、ブレンドシェイプ格子の軸がすべて `blend_shape` のパラメータで 1 本以上あること、
/// 入れ子がないこと、同じパラメータが 2 つのブレンドシェイプ格子の軸にならないこと、
/// ブレンドシェイプ格子を持てない要素 (パーツ・グルー) が持たないこと。
/// ブレンドシェイプ格子の格子としての検査 (キーフォーム数など) も行う。
fn check_blend_shapes<T>(
    refs: &Refs<'_>,
    kinds: &Kinds<'_>,
    same_kind: &dyn Fn(&str) -> bool,
    grid: &KeyformGrid<T>,
    errors: &mut Vec<ValidationError>,
) {
    let mismatch = |parameter: &crate::ParamId, expected| ValidationError::ParameterKindMismatch {
        kind: refs.kind,
        id: refs.id.to_owned(),
        parameter: parameter.to_string(),
        expected,
    };
    let invalid = |index, reason| ValidationError::InvalidBlendShape {
        kind: refs.kind,
        id: refs.id.to_owned(),
        index,
        reason,
    };
    // 未知の種類 (`ParameterKind::Unknown`) はどちらの格子にも置ける (未知の値は検証エラーにしない)。
    for binding in &grid.bindings {
        if kinds.of(&binding.parameter) == Some(ParameterKind::BlendShape) {
            errors.push(mismatch(&binding.parameter, "normal"));
        }
    }
    if !kinds.supported && !grid.blend_shapes.is_empty() {
        errors.push(invalid(0, "is not supported on this element"));
        return;
    }
    let mut earlier = BTreeSet::new();
    let base = keyforms_path(refs);
    for (index, shape) in grid.blend_shapes.iter().enumerate() {
        let path = format!("{base}.blend_shapes[{index}]");
        check_grid(refs, &path, kinds.params, same_kind, shape, errors);
        if shape.bindings.is_empty() {
            errors.push(invalid(index, "has no bindings"));
        }
        if !shape.blend_shapes.is_empty() {
            errors.push(invalid(index, "must not have nested blend shape grids"));
        }
        for binding in &shape.bindings {
            if kinds.of(&binding.parameter) == Some(ParameterKind::Normal) {
                errors.push(mismatch(&binding.parameter, "blend_shape"));
            }
        }
        // 同じ格子の中の重複は `check_grid` が報告するので、前の格子との重複だけを見る。
        let mut reported = BTreeSet::new();
        for binding in &shape.bindings {
            if earlier.contains(&binding.parameter) && reported.insert(&binding.parameter) {
                errors.push(ValidationError::DuplicateBinding {
                    kind: refs.kind,
                    id: refs.id.to_owned(),
                    parameter: binding.parameter.to_string(),
                });
            }
        }
        earlier.extend(shape.bindings.iter().map(|b| &b.parameter));
    }
}

/// ワープの制御点数 `(rows+1)*(cols+1)`。32 ビットの `usize` (wasm32) であふれるときは `usize::MAX`
/// (どの長さとも一致しない)。
fn warp_point_count(rows: u16, cols: u16) -> usize {
    (usize::from(rows) + 1).saturating_mul(usize::from(cols) + 1)
}

/// エラーメッセージの値の場所に使う、要素の種類の配列名。
fn plural(kind: &str) -> &str {
    match kind {
        "mesh" => "meshes",
        "part" => "parts",
        "deformer" => "deformers",
        "glue" => "glues",
        _ => kind,
    }
}

/// 要素の最上位の格子の値の場所 (`meshes[Face].keyforms` など)。
fn keyforms_path(refs: &Refs<'_>) -> String {
    format!("{}[{}].keyforms", plural(refs.kind), refs.id)
}

/// 有限値であること。
fn check_finite(field: &str, value: f32, errors: &mut Vec<ValidationError>) {
    if !value.is_finite() {
        errors.push(ValidationError::InvalidValue {
            field: field.to_owned(),
            value,
            expected: "finite",
        });
    }
}

/// 2 つの成分が有限値であること。
fn check_vec2(field: &str, value: Vec2, errors: &mut Vec<ValidationError>) {
    check_finite(&format!("{field}.x"), value.x, errors);
    check_finite(&format!("{field}.y"), value.y, errors);
}

/// 点の列の成分がすべて有限値であること (有限でない点は最初の 1 つだけ報告する)。
fn check_points(field: &str, points: &[Vec2], errors: &mut Vec<ValidationError>) {
    if let Some((index, &point)) = points
        .iter()
        .enumerate()
        .find(|(_, p)| !(p.x.is_finite() && p.y.is_finite()))
    {
        check_vec2(&format!("{field}[{index}]"), point, errors);
    }
}

/// 対称軸が指定されていれば有限値であること。
fn check_mirror_axis(path: &str, axis: Option<f32>, errors: &mut Vec<ValidationError>) {
    if let Some(axis) = axis {
        check_finite(&format!("{path}.mirror_axis"), axis, errors);
    }
}

/// 深度があれば、長さが `expected` で、すべて有限値であること (有限でない値は最初の 1 つだけ報告する)。
fn check_depth(
    refs: &Refs<'_>,
    path: &str,
    depth: Option<&[f32]>,
    expected: usize,
    errors: &mut Vec<ValidationError>,
) {
    let Some(depth) = depth else {
        return;
    };
    if depth.len() != expected {
        errors.push(ValidationError::DepthCount {
            kind: refs.kind,
            id: refs.id.to_owned(),
            expected,
            actual: depth.len(),
        });
    }
    if let Some((index, &value)) = depth.iter().enumerate().find(|(_, v)| !v.is_finite()) {
        check_finite(&format!("{path}.depth[{index}]"), value, errors);
    }
}

/// ミラーの参照先が存在し、対が双方向であること (自分自身との対は常に正しい)。
fn check_mirrors<T, K: Ord + Display>(
    kind: &'static str,
    items: &[T],
    id_of: impl Fn(&T) -> &K,
    mirror_of: impl Fn(&T) -> Option<&K>,
    table: &BTreeMap<&K, usize>,
    errors: &mut Vec<ValidationError>,
) {
    for item in items {
        let id = id_of(item);
        let Some(mirror) = mirror_of(item) else {
            continue;
        };
        let id_string = id.to_string();
        let refs = Refs {
            kind,
            id: &id_string,
        };
        let Some(target) = refs.check(table, "mirror", kind, mirror, errors) else {
            continue;
        };
        let back = mirror_of(&items[target]);
        if mirror != id && back != Some(id) {
            errors.push(ValidationError::AsymmetricMirror {
                kind,
                id: id_string.clone(),
                mirror: mirror.to_string(),
                back: back.map(ToString::to_string),
            });
        }
    }
}

/// 親をたどる連鎖に循環があれば、循環ごとに 1 件報告する。未定義の親は参照検査に任せて無視する。
fn check_cycles<T, K: Ord + Display>(
    kind: &'static str,
    items: &[T],
    id_of: impl Fn(&T) -> &K,
    parent_of: impl Fn(&T) -> Option<&K>,
    table: &BTreeMap<&K, usize>,
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
    table: &BTreeMap<&crate::MeshId, usize>,
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
    let mut reported = BTreeSet::new();
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
    use crate::fixtures::{keyform_inference, pose_and_effects, single_quad};
    use crate::mesh::{Glue, GlueKeyform, GluePair, MaskRef};
    use crate::part::{Part, PartKeyform};
    use crate::pose::{PoseEntry, PoseGroup};
    use crate::{
        Effects, HitArea, KeyformBinding, KeyformOrigin, MirrorSource, ParameterGroup, TextureRef,
    };

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
            mirror: None,
            mirror_axis: None,
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
                depth: None,
            }),
            mirror: None,
            mirror_axis: None,
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

    /// パラメータグループ・パーツ・ワープ・回転・マスク・グルー・当たり判定を持つ、検証を通るモデル。
    fn all_kinds() -> Model {
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
        model
    }

    #[test]
    fn valid_model_with_all_kinds_has_no_errors() {
        assert_eq!(validate(&all_kinds()), vec![]);
    }

    // docs/spec/model.md#検証: モデルの数値はすべて有限値 (値の範囲はランタイムに任せる)
    #[test]
    fn non_finite_model_values_are_reported() {
        let inf = f32::INFINITY;
        let mut model = all_kinds();
        model.meta.canvas.width = inf;
        model.meta.canvas.origin.y = -inf;
        model.parts[0].keyforms.keyforms[0].opacity = inf;
        let DeformerKind::Warp(w) = &mut model.deformers[0].kind else {
            unreachable!()
        };
        w.keyforms.keyforms[0].points[4].x = inf;
        w.keyforms.keyforms[0].opacity = -inf;
        let DeformerKind::Rotation(r) = &mut model.deformers[1].kind else {
            unreachable!()
        };
        let rot = &mut r.keyforms.keyforms[0];
        (rot.origin.x, rot.angle, rot.scale, rot.opacity) = (inf, inf, inf, inf);
        let quad = &mut model.meshes[0];
        quad.uvs[2].y = inf;
        // 有限でない頂点は最初の 1 つだけ報告する。
        let kf = &mut quad.keyforms.keyforms[1];
        kf.positions[1].y = inf;
        kf.positions[3].x = inf;
        kf.opacity = inf;
        kf.draw_order = inf;
        kf.multiply_color.g = inf;
        kf.screen_color.b = -inf;
        model.glues[0].pairs[0].weight = inf;
        model.glues[0].keyforms.keyforms[0].intensity = inf;
        let errors = validate(&model);
        assert_eq!(
            errors,
            vec![
                invalid("meta.canvas.width", inf, "finite"),
                invalid("meta.canvas.origin.y", -inf, "finite"),
                invalid("parts[Root].keyforms[0].opacity", inf, "finite"),
                invalid("deformers[W].keyforms[0].points[4].x", inf, "finite"),
                invalid("deformers[W].keyforms[0].opacity", -inf, "finite"),
                invalid("deformers[R].keyforms[0].origin.x", inf, "finite"),
                invalid("deformers[R].keyforms[0].angle", inf, "finite"),
                invalid("deformers[R].keyforms[0].scale", inf, "finite"),
                invalid("deformers[R].keyforms[0].opacity", inf, "finite"),
                invalid("meshes[Quad].uvs[2].y", inf, "finite"),
                invalid("meshes[Quad].keyforms[1].positions[1].y", inf, "finite"),
                invalid("meshes[Quad].keyforms[1].opacity", inf, "finite"),
                invalid("meshes[Quad].keyforms[1].draw_order", inf, "finite"),
                invalid("meshes[Quad].keyforms[1].multiply_color.g", inf, "finite"),
                invalid("meshes[Quad].keyforms[1].screen_color.b", -inf, "finite"),
                invalid("glues[G].keyforms[0].intensity", inf, "finite"),
                invalid("glues[G].pairs[0].weight", inf, "finite"),
            ]
        );

        // NaN も同じ。範囲の外の有限値 (不透明度 2 など) はランタイムがクランプするのでエラーにしない。
        let mut model = all_kinds();
        model.meshes[0].keyforms.keyforms[0].opacity = f32::NAN;
        model.meshes[0].keyforms.keyforms[1].opacity = 2.0;
        model.parts[0].keyforms.keyforms[0].opacity = -1.0;
        let errors = validate(&model);
        assert!(
            matches!(
                errors.as_slice(),
                [ValidationError::InvalidValue { field, .. }]
                    if field == "meshes[Quad].keyforms[0].opacity"
            ),
            "{errors:?}"
        );
    }

    // T0145: 無限大のキーは昇順の検査を通るので、有限値の検査で報告する (ブレンドシェイプ格子も)。
    #[test]
    fn infinite_keys_are_reported() {
        let mut model = single_quad();
        model.meshes[0].keyforms.bindings[0].keys[0] = f32::NEG_INFINITY;
        assert_eq!(
            validate(&model),
            vec![invalid(
                "meshes[Quad].keyforms.bindings[0].keys[0]",
                f32::NEG_INFINITY,
                "finite"
            )]
        );

        let mut model = crate::fixtures::blend_shapes();
        let mouth = &mut model.meshes[0].keyforms;
        mouth.blend_shapes[1].bindings[0].keys[1] = f32::INFINITY;
        assert_eq!(
            validate(&model),
            vec![invalid(
                "meshes[Mouth].keyforms.blend_shapes[1].bindings[0].keys[1]",
                f32::INFINITY,
                "finite"
            )]
        );
    }

    // T0145: パラメータの値域は有限値。順序の検査は有限のときだけ行う。
    #[test]
    fn parameter_range_must_be_finite() {
        let mut model = single_quad();
        let p = &mut model.parameters[0];
        (p.min, p.max) = (f32::NEG_INFINITY, f32::INFINITY);
        assert_eq!(
            validate(&model),
            vec![
                invalid("parameters[X].min", f32::NEG_INFINITY, "finite"),
                invalid("parameters[X].max", f32::INFINITY, "finite"),
            ]
        );
        model.parameters[0].default = f32::NAN;
        let fields: Vec<_> = validate(&model)
            .into_iter()
            .map(|e| match e {
                ValidationError::InvalidValue { field, .. } => field,
                e => panic!("unexpected {e}"),
            })
            .collect();
        assert_eq!(
            fields,
            [
                "parameters[X].min",
                "parameters[X].default",
                "parameters[X].max"
            ]
        );
    }

    // 呼吸の中心値・振幅・重みも有限値。
    #[test]
    fn breath_values_must_be_finite() {
        let mut model = pose_and_effects();
        let breath = &mut model.effects.breath.parameters[0];
        (breath.offset, breath.peak, breath.weight) =
            (f32::INFINITY, -f32::INFINITY, f32::INFINITY);
        assert_eq!(
            validate(&model),
            vec![
                invalid(
                    "effects.breath.parameters[0].offset",
                    f32::INFINITY,
                    "finite"
                ),
                invalid(
                    "effects.breath.parameters[0].peak",
                    -f32::INFINITY,
                    "finite"
                ),
                invalid(
                    "effects.breath.parameters[0].weight",
                    f32::INFINITY,
                    "finite"
                ),
            ]
        );
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

    /// キーが 1 つだけの軸でも NaN を拒む (比較する隣のキーがない)。
    #[test]
    fn single_nan_key_is_rejected() {
        let mut model = single_quad();
        let keyforms = &mut model.meshes[0].keyforms;
        keyforms.bindings[0].keys = vec![f32::NAN];
        keyforms.keyforms.truncate(1);
        let errors = validate(&model);
        assert!(
            matches!(
                errors.as_slice(),
                [ValidationError::KeysNotAscending { kind: "mesh", id, parameter, .. }]
                    if id == "Quad" && parameter == "X"
            ),
            "{errors:?}"
        );
    }

    /// 軸のキー数の積があふれる格子 (壊れた・悪意のあるファイル) でも止まらず、キーフォーム数の不一致として報告する。
    #[test]
    fn keyform_count_overflow_is_reported() {
        let mut model = single_quad();
        let ids: Vec<String> = (0..usize::BITS).map(|i| format!("P{i}")).collect();
        for id in &ids {
            model.parameters.push(Parameter {
                id: id.as_str().into(),
                name: id.clone(),
                ..model.parameters[0].clone()
            });
        }
        let keyforms = &mut model.meshes[0].keyforms;
        keyforms.bindings = ids
            .iter()
            .map(|id| KeyformBinding {
                parameter: id.as_str().into(),
                keys: vec![0.0, 1.0],
            })
            .collect();
        keyforms.keyforms.clear();
        assert_eq!(
            validate(&model),
            vec![ValidationError::KeyformCount {
                kind: "mesh",
                id: "Quad".into(),
                expected: usize::MAX,
                actual: 0,
            }]
        );
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
        model.parts[0].keyforms = KeyformGrid::new(
            vec![
                KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![0.0],
                },
                KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![1.0],
                },
            ],
            vec![PartKeyform { opacity: 1.0 }],
        );
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
            normal_map: None,
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

    /// `validate_effects` は渡した設定だけを、モデルのパラメータと合わせて検査する (モデルの `effects` は見ない)。
    #[test]
    fn validate_effects_checks_the_given_effects() {
        let mut model = pose_and_effects();
        assert_eq!(validate_effects(&model.effects, &model), vec![]);

        let mut effects = model.effects.clone();
        effects.eye_blink.interval = f32::INFINITY;
        effects.breath.parameters[0].weight = f32::NEG_INFINITY;
        effects.lip_sync.parameters.push("Missing".into());
        model.effects.eye_blink.closing = -1.0;
        assert_eq!(
            validate_effects(&effects, &model),
            vec![
                invalid(
                    "effects.eye_blink.interval",
                    f32::INFINITY,
                    "finite and > 0"
                ),
                invalid(
                    "effects.breath.parameters[0].weight",
                    f32::NEG_INFINITY,
                    "finite"
                ),
                unknown("effect", "lip_sync", "parameters", "parameter", "Missing"),
            ]
        );
    }

    fn invalid(field: &str, value: f32, expected: &'static str) -> ValidationError {
        ValidationError::InvalidValue {
            field: field.into(),
            value,
            expected,
        }
    }

    fn face_warp(model: &mut Model) -> &mut WarpDeformer {
        match &mut model.deformers[0].kind {
            DeformerKind::Warp(w) => w,
            _ => panic!("not a warp"),
        }
    }

    #[test]
    fn inference_fixture_is_valid() {
        assert_eq!(validate(&keyform_inference()), vec![]);
    }

    #[test]
    fn origin_count() {
        let mut model = keyform_inference();
        face_warp(&mut model).keyforms.origins.pop();
        // 空の `origins` は「すべて authored」なので正しい。
        model.meshes[0].keyforms.origins.clear();
        model.parts[0].keyforms.origins = vec![KeyformOrigin::Derived; 2];
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::OriginCount {
                    kind: "part",
                    id: "Root".into(),
                    expected: 1,
                    actual: 2,
                },
                ValidationError::OriginCount {
                    kind: "deformer",
                    id: "Face".into(),
                    expected: 9,
                    actual: 8,
                },
            ]
        );
    }

    #[test]
    fn mirror_source_reference() {
        let mut model = keyform_inference();
        // 同じ種類の要素ならよい (自分自身も)。
        model.meshes[1].keyforms.mirror_source = Some(MirrorSource {
            id: model.meshes[0].id.to_string(),
            axis: None,
            tolerance: None,
        });
        face_warp(&mut model).keyforms.mirror_source = Some(MirrorSource {
            id: "Face".into(),
            axis: Some(0.5),
            tolerance: None,
        });
        assert_eq!(validate(&model), vec![]);

        // 別の種類の要素の ID や存在しない ID は、参照が壊れている。
        model.meshes[1].keyforms.mirror_source = Some(MirrorSource {
            id: "Face".into(),
            axis: None,
            tolerance: Some(f32::NAN),
        });
        face_warp(&mut model).keyforms.mirror_source = Some(MirrorSource {
            id: "Missing".into(),
            axis: None,
            tolerance: None,
        });
        let mesh = model.meshes[1].id.to_string();
        let errors = validate(&model);
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors.contains(&ValidationError::UnknownReference {
            kind: "deformer",
            id: "Face".into(),
            field: "keyforms.mirror_source",
            target_kind: "deformer",
            target: "Missing".into(),
        }));
        assert!(errors.contains(&ValidationError::UnknownReference {
            kind: "mesh",
            id: mesh.clone(),
            field: "keyforms.mirror_source",
            target_kind: "mesh",
            target: "Face".into(),
        }));
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::InvalidValue { field, .. }
                if *field == format!("meshes[{mesh}].keyforms.mirror_source.tolerance")
        )));
    }

    #[test]
    fn depth_rotation_parameters() {
        let mut model = keyform_inference();
        // 存在するが格子の軸ではないパラメータ。
        let mut angle_z = model.parameters[0].clone();
        angle_z.id = "AngleZ".into();
        model.parameters.push(angle_z);
        let Some(KeyformInference::DepthRotation(rotation)) =
            &mut face_warp(&mut model).keyforms.inference
        else {
            panic!("not depth rotation");
        };
        rotation.yaw = Some("NoParam".into());
        rotation.pitch = Some("AngleZ".into());
        rotation.angle_scale = f32::INFINITY;
        rotation.pivot_depth = f32::NEG_INFINITY;
        assert_eq!(
            validate(&model),
            vec![
                invalid(
                    "deformers[Face].keyforms.inference.angle_scale",
                    f32::INFINITY,
                    "finite"
                ),
                invalid(
                    "deformers[Face].keyforms.inference.pivot_depth",
                    f32::NEG_INFINITY,
                    "finite"
                ),
                unknown("deformer", "Face", "yaw", "parameter", "NoParam"),
                ValidationError::UnboundInferenceParameter {
                    kind: "deformer",
                    id: "Face".into(),
                    parameter: "AngleZ".into(),
                },
            ]
        );
    }

    #[test]
    fn mirror_references() {
        let mut model = keyform_inference();
        // 自分自身との対は正しい。
        model.meshes[2].mirror = Some("Nose".into());
        model.deformers[0].mirror = Some("NoDeformer".into());
        model.meshes.push(model.meshes[2].clone());
        model.meshes[3].id = "Mouth".into();
        model.meshes[3].mirror = Some("NoMesh".into());
        assert_eq!(
            validate(&model),
            vec![
                unknown("deformer", "Face", "mirror", "deformer", "NoDeformer"),
                unknown("mesh", "Mouth", "mirror", "mesh", "NoMesh"),
            ]
        );
    }

    #[test]
    fn mirror_must_be_symmetric() {
        let mut model = keyform_inference();
        // EyeL → EyeR → Nose、Nose → (なし)。
        model.meshes[1].mirror = Some("Nose".into());
        model.deformers.push(rotation("A", None));
        model.deformers.push(rotation("B", None));
        model.deformers[1].mirror = Some("B".into());
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::AsymmetricMirror {
                    kind: "deformer",
                    id: "A".into(),
                    mirror: "B".into(),
                    back: None,
                },
                ValidationError::AsymmetricMirror {
                    kind: "mesh",
                    id: "EyeL".into(),
                    mirror: "EyeR".into(),
                    back: Some("Nose".into()),
                },
                ValidationError::AsymmetricMirror {
                    kind: "mesh",
                    id: "EyeR".into(),
                    mirror: "Nose".into(),
                    back: None,
                },
            ]
        );
    }

    #[test]
    fn mirror_axis_must_be_finite() {
        let mut model = keyform_inference();
        model.meshes[0].mirror_axis = Some(f32::INFINITY);
        model.deformers[0].mirror_axis = Some(f32::NEG_INFINITY);
        assert_eq!(
            validate(&model),
            vec![
                invalid("deformers[Face].mirror_axis", f32::NEG_INFINITY, "finite"),
                invalid("meshes[EyeL].mirror_axis", f32::INFINITY, "finite"),
            ]
        );
    }

    #[test]
    fn depth_count_and_values() {
        let mut model = keyform_inference();
        face_warp(&mut model).depth = Some(vec![0.0; 3]);
        model.meshes[0].depth = Some(vec![0.0, f32::INFINITY, 0.0, f32::NEG_INFINITY]);
        model.meshes[2].depth = Some(vec![0.0; 5]);
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::DepthCount {
                    kind: "deformer",
                    id: "Face".into(),
                    expected: 4,
                    actual: 3,
                },
                invalid("meshes[EyeL].depth[1]", f32::INFINITY, "finite"),
                ValidationError::DepthCount {
                    kind: "mesh",
                    id: "Nose".into(),
                    expected: 4,
                    actual: 5,
                },
            ]
        );
    }

    fn mismatch(
        kind: &'static str,
        id: &str,
        parameter: &str,
        expected: &'static str,
    ) -> ValidationError {
        ValidationError::ParameterKindMismatch {
            kind,
            id: id.into(),
            parameter: parameter.into(),
            expected,
        }
    }

    fn invalid_shape(
        kind: &'static str,
        id: &str,
        index: usize,
        reason: &'static str,
    ) -> ValidationError {
        ValidationError::InvalidBlendShape {
            kind,
            id: id.into(),
            index,
            reason,
        }
    }

    #[test]
    fn blend_shape_fixture_is_valid() {
        assert_eq!(validate(&crate::fixtures::blend_shapes()), vec![]);
    }

    // docs/spec/model.md#ブレンドシェイプ: 通常の格子とブレンドシェイプ格子のパラメータの種類
    #[test]
    fn blend_shape_parameter_kinds_must_match_grids() {
        let mut model = crate::fixtures::blend_shapes();
        let mouth = &mut model.meshes[0].keyforms;
        // 通常の格子に blend_shape の軸、ブレンドシェイプ格子に通常の軸。
        mouth.bindings[0].parameter = "Smile".into();
        mouth.blend_shapes[1].bindings[0].parameter = "X".into();
        // 未知の種類はどちらにも置ける。
        model.parameters[2].kind = ParameterKind::Unknown;
        let DeformerKind::Rotation(jaw) = &mut model.deformers[1].kind else {
            panic!("not a rotation");
        };
        jaw.keyforms.bindings.push(KeyformBinding {
            parameter: "Wide".into(),
            keys: vec![0.0],
        });
        assert_eq!(
            validate(&model),
            vec![
                mismatch("mesh", "Mouth", "Smile", "normal"),
                mismatch("mesh", "Mouth", "X", "blend_shape"),
            ]
        );
    }

    #[test]
    fn blend_shape_grid_structure() {
        let mut model = crate::fixtures::blend_shapes();
        let mouth = &mut model.meshes[0].keyforms;
        // 軸なし・入れ子・キーフォーム数の不一致・頂点数の不一致。
        let mut empty = KeyformGrid::constant(mouth.keyforms[1].clone());
        empty.blend_shapes.push(mouth.blend_shapes[0].clone());
        mouth.blend_shapes.push(empty);
        mouth.blend_shapes[1].keyforms.pop();
        mouth.blend_shapes[0].keyforms[1].positions.pop();
        // 2 つのブレンドシェイプ格子に同じパラメータ (同じ格子の中の重複は 1 回だけ報告する)。
        let DeformerKind::Warp(face) = &mut model.deformers[0].kind else {
            panic!("not a warp");
        };
        let mut again = face.keyforms.blend_shapes[0].clone();
        again.blend_shapes.clear();
        face.keyforms.blend_shapes.push(again);
        face.keyforms.blend_shapes[0].keyforms[0].points.pop();
        assert_eq!(
            validate(&model),
            vec![
                ValidationError::DuplicateBinding {
                    kind: "deformer",
                    id: "Face".into(),
                    parameter: "Smile".into(),
                },
                ValidationError::WarpPointCount {
                    id: "Face".into(),
                    expected: 4,
                    actual: 3,
                },
                ValidationError::KeyformCount {
                    kind: "mesh",
                    id: "Mouth".into(),
                    expected: 2,
                    actual: 1,
                },
                invalid_shape("mesh", "Mouth", 2, "has no bindings"),
                invalid_shape("mesh", "Mouth", 2, "must not have nested blend shape grids"),
                ValidationError::VertexCount {
                    id: "Mouth".into(),
                    expected: 4,
                    actual: 3,
                },
            ]
        );
    }

    #[test]
    fn blend_shapes_are_not_supported_on_parts_and_glues() {
        let mut model = crate::fixtures::blend_shapes();
        let shape = grid_on_smile(PartKeyform { opacity: 0.0 });
        model.parts[0].keyforms.blend_shapes.push(shape);
        let mut g = glue("Mouth", "Chin", vec![]);
        g.keyforms
            .blend_shapes
            .push(grid_on_smile(GlueKeyform { intensity: 1.0 }));
        model.glues.push(g);
        assert_eq!(
            validate(&model),
            vec![
                invalid_shape("part", "Root", 0, "is not supported on this element"),
                invalid_shape("glue", "G", 0, "is not supported on this element"),
            ]
        );
    }

    fn grid_on_smile<T: Clone>(keyform: T) -> KeyformGrid<T> {
        KeyformGrid::new(
            vec![KeyformBinding {
                parameter: "Smile".into(),
                keys: vec![0.0, 1.0],
            }],
            vec![keyform.clone(), keyform],
        )
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
