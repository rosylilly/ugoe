use std::fmt::Display;

use ugoe_model::deformer::DeformerKind;
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{DeformerId, KeyformOrigin, MeshId, Model, ParamId, PartId};

use crate::Command;
use crate::grid::{self, CellError};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EditError {
    /// `kind` は要素の種類 (`parameter`、`part`、`deformer`、`mesh`)。
    #[error("{kind} `{id}` already exists")]
    DuplicateId { kind: &'static str, id: String },
    #[error("{kind} `{id}` not found")]
    NotFound { kind: &'static str, id: String },
    #[error("invalid range: min {min} <= default {default} <= max {max} must hold")]
    InvalidRange { min: f32, default: f32, max: f32 },
    /// 子が残っている要素は削除できない。`children` は `mesh \`Quad\`` のような表記。
    #[error("{kind} `{id}` still has children: {}", children.join(", "))]
    HasChildren {
        kind: &'static str,
        id: String,
        children: Vec<String>,
    },
    /// `field` が自分自身を指している (並べ替えの基準など)。
    #[error("{kind} `{id}`: {field} must not refer to itself")]
    SelfReference {
        kind: &'static str,
        id: String,
        field: &'static str,
    },
    #[error("deformer `{id}` is not a {expected} deformer")]
    DeformerKindMismatch { id: String, expected: &'static str },
    #[error("{kind} `{id}`: invalid keyform cell: {error}")]
    InvalidCell {
        kind: &'static str,
        id: String,
        error: CellError,
    },
    /// 適用するとモデルが検証 (`ugoe_model::validate`) に通らなくなる。新たに生じるエラーの一覧。
    #[error("command would make the model invalid: {}", join(.0))]
    Invalid(Vec<ValidationError>),
}

fn join(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// 編集中のモデルと履歴。
///
/// 暫定実装としてスナップショット方式の Undo を使う (`docs/adr/0005-editor-command-model.md`)。
#[derive(Debug)]
pub struct Document {
    model: Model,
    undo: Vec<Model>,
    redo: Vec<Model>,
}

impl Document {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// コマンドを適用する。失敗時はモデルを変更しない。
    ///
    /// 適用後のモデルを `ugoe_model::validate::validate` で検査し、適用前になかった
    /// 検証エラーが生じたら [`EditError::Invalid`] で拒否する。検証を通るモデルに対しては、
    /// 成功した適用の後も必ず検証を通る。`Batch` は全体を適用した後に 1 回だけ検査する。
    pub fn apply(&mut self, command: Command) -> Result<(), EditError> {
        let mut next = self.model.clone();
        apply_to(&mut next, command)?;
        let before = validate(&self.model);
        let introduced: Vec<_> = validate(&next)
            .into_iter()
            .filter(|e| !before.contains(e))
            .collect();
        if !introduced.is_empty() {
            return Err(EditError::Invalid(introduced));
        }
        self.undo.push(std::mem::replace(&mut self.model, next));
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.model, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.model, next));
                true
            }
            None => false,
        }
    }
}

/// ID で要素を探す。
fn find_mut<'a, T, K: PartialEq + Display>(
    items: &'a mut [T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<&'a mut T, EditError> {
    items
        .iter_mut()
        .find(|item| id_of(item) == id)
        .ok_or_else(|| EditError::NotFound {
            kind,
            id: id.to_string(),
        })
}

/// ID が未使用であることを確かめる。
fn ensure_new<T, K: PartialEq + Display>(
    items: &[T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<(), EditError> {
    if items.iter().any(|item| id_of(item) == id) {
        return Err(EditError::DuplicateId {
            kind,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// 子の一覧が空でなければ [`EditError::HasChildren`] にする。
fn ensure_no_children(
    kind: &'static str,
    id: &impl Display,
    children: Vec<String>,
) -> Result<(), EditError> {
    if children.is_empty() {
        return Ok(());
    }
    Err(EditError::HasChildren {
        kind,
        id: id.to_string(),
        children,
    })
}

fn parameter<'a>(
    model: &'a mut Model,
    id: &ParamId,
) -> Result<&'a mut ugoe_model::Parameter, EditError> {
    find_mut(&mut model.parameters, "parameter", id, |p| &p.id)
}

fn part<'a>(model: &'a mut Model, id: &PartId) -> Result<&'a mut ugoe_model::Part, EditError> {
    find_mut(&mut model.parts, "part", id, |p| &p.id)
}

fn deformer<'a>(
    model: &'a mut Model,
    id: &DeformerId,
) -> Result<&'a mut ugoe_model::Deformer, EditError> {
    find_mut(&mut model.deformers, "deformer", id, |d| &d.id)
}

fn mesh<'a>(model: &'a mut Model, id: &MeshId) -> Result<&'a mut ugoe_model::Mesh, EditError> {
    find_mut(&mut model.meshes, "mesh", id, |m| &m.id)
}

fn apply_to(model: &mut Model, command: Command) -> Result<(), EditError> {
    match command {
        Command::AddParameter { parameter } => {
            ensure_new(&model.parameters, "parameter", &parameter.id, |p| &p.id)?;
            model.parameters.push(parameter);
        }
        Command::RemoveParameter { id } => remove_parameter(model, &id)?,
        Command::SetParameterRange {
            id,
            min,
            default,
            max,
        } => {
            if !(min <= default && default <= max) {
                return Err(EditError::InvalidRange { min, default, max });
            }
            let p = parameter(model, &id)?;
            (p.min, p.default, p.max) = (min, default, max);
        }
        Command::Batch { commands, .. } => {
            for c in commands {
                apply_to(model, c)?;
            }
        }
        Command::RenameParameter { id, name } => parameter(model, &id)?.name = name,
        Command::SetParameterGroup { id, group } => parameter(model, &id)?.group = group,
        Command::SetParameterMirror { id, mirror } => parameter(model, &id)?.mirror = mirror,
        Command::AddPart { part } => {
            ensure_new(&model.parts, "part", &part.id, |p| &p.id)?;
            model.parts.push(part);
        }
        Command::RemovePart { id } => remove_part(model, &id)?,
        Command::RenamePart { id, name } => part(model, &id)?.name = name,
        Command::SetPartParent { id, parent } => part(model, &id)?.parent = parent,
        Command::SetPartKeyforms { id, keyforms } => part(model, &id)?.keyforms = keyforms,
        Command::AddDeformer { deformer } => {
            ensure_new(&model.deformers, "deformer", &deformer.id, |d| &d.id)?;
            model.deformers.push(deformer);
        }
        Command::RemoveDeformer { id } => remove_deformer(model, &id)?,
        Command::RenameDeformer { id, name } => deformer(model, &id)?.name = name,
        Command::SetDeformerParent { id, parent } => deformer(model, &id)?.parent = parent,
        Command::SetDeformerPart { id, part } => deformer(model, &id)?.part = part,
        Command::SetWarpKeyforms { id, keyforms } => match &mut deformer(model, &id)?.kind {
            DeformerKind::Warp(warp) => warp.keyforms = keyforms,
            DeformerKind::Rotation(_) => {
                return Err(EditError::DeformerKindMismatch {
                    id: id.to_string(),
                    expected: "warp",
                });
            }
        },
        Command::SetRotationKeyforms { id, keyforms } => match &mut deformer(model, &id)?.kind {
            DeformerKind::Rotation(rotation) => rotation.keyforms = keyforms,
            DeformerKind::Warp(_) => {
                return Err(EditError::DeformerKindMismatch {
                    id: id.to_string(),
                    expected: "rotation",
                });
            }
        },
        Command::AddMesh { mesh } => {
            ensure_new(&model.meshes, "mesh", &mesh.id, |m| &m.id)?;
            model.meshes.push(mesh);
        }
        Command::RemoveMesh { id } => remove_mesh(model, &id)?,
        Command::RenameMesh { id, name } => mesh(model, &id)?.name = name,
        Command::SetMeshProperties {
            id,
            part,
            texture,
            blend,
            culling,
            masks,
            mask_mode,
            mask_only,
        } => {
            let m = mesh(model, &id)?;
            if let Some(part) = part {
                m.part = part;
            }
            if let Some(texture) = texture {
                m.texture = texture;
            }
            if let Some(blend) = blend {
                m.blend = blend;
            }
            if let Some(culling) = culling {
                m.culling = culling;
            }
            if let Some(masks) = masks {
                m.masks = masks;
            }
            if let Some(mask_mode) = mask_mode {
                m.mask_mode = mask_mode;
            }
            if let Some(mask_only) = mask_only {
                m.mask_only = mask_only;
            }
        }
        Command::SetMeshParent { id, parent } => mesh(model, &id)?.parent = parent,
        Command::SetMeshKeyforms { id, keyforms } => mesh(model, &id)?.keyforms = keyforms,
        Command::SetMeshKeyform { id, cell, keyform } => {
            let m = mesh(model, &id)?;
            let index =
                grid::cell_index(&m.keyforms, &cell).map_err(|error| EditError::InvalidCell {
                    kind: "mesh",
                    id: id.to_string(),
                    error,
                })?;
            m.keyforms.keyforms[index] = keyform;
            // 手で編集したセルは `authored` になる (`docs/spec/keyform-inference.md#セルの由来`)。
            if let Some(origin) = m.keyforms.origins.get_mut(index) {
                *origin = KeyformOrigin::Authored;
            }
        }
        Command::ReorderMesh { id, before } => reorder_mesh(model, &id, before.as_ref())?,
    }
    Ok(())
}

/// パラメータを削除し、参照を外す: キーフォーム格子の軸は既定値の断面に畳み、
/// 他のパラメータの `mirror` とエフェクトの対象からは取り除く。
fn remove_parameter(model: &mut Model, id: &ParamId) -> Result<(), EditError> {
    let index = model
        .parameters
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "parameter",
            id: id.to_string(),
        })?;
    let removed = model.parameters.remove(index);
    let default = removed.default;

    for part in &mut model.parts {
        grid::collapse_axis(&mut part.keyforms, id, default);
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(warp) => grid::collapse_axis(&mut warp.keyforms, id, default),
            DeformerKind::Rotation(rotation) => {
                grid::collapse_axis(&mut rotation.keyforms, id, default)
            }
        }
    }
    for mesh in &mut model.meshes {
        grid::collapse_axis(&mut mesh.keyforms, id, default);
    }
    for glue in &mut model.glues {
        grid::collapse_axis(&mut glue.keyforms, id, default);
    }

    for param in &mut model.parameters {
        if param.mirror.as_ref() == Some(id) {
            param.mirror = None;
        }
    }
    let effects = &mut model.effects;
    effects.eye_blink.parameters.retain(|p| p != id);
    effects.breath.parameters.retain(|b| &b.parameter != id);
    effects.lip_sync.parameters.retain(|p| p != id);
    Ok(())
}

/// パーツを削除する。子パーツ・所属する要素があれば拒否し、ポーズからは取り除く。
fn remove_part(model: &mut Model, id: &PartId) -> Result<(), EditError> {
    let index = model
        .parts
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "part",
            id: id.to_string(),
        })?;
    let children = model
        .parts
        .iter()
        .filter(|p| p.parent.as_ref() == Some(id))
        .map(|p| format!("part `{}`", p.id))
        .chain(
            model
                .deformers
                .iter()
                .filter(|d| &d.part == id)
                .map(|d| format!("deformer `{}`", d.id)),
        )
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| &m.part == id)
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .collect();
    ensure_no_children("part", id, children)?;
    model.parts.remove(index);

    for group in &mut model.pose.groups {
        group.entries.retain(|e| &e.part != id);
        for entry in &mut group.entries {
            entry.links.retain(|l| l != id);
        }
    }
    model.pose.groups.retain(|g| !g.entries.is_empty());
    Ok(())
}

/// デフォーマを削除する。子デフォーマ・子メッシュがあれば拒否する。
fn remove_deformer(model: &mut Model, id: &DeformerId) -> Result<(), EditError> {
    let index = model
        .deformers
        .iter()
        .position(|d| &d.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "deformer",
            id: id.to_string(),
        })?;
    let children = model
        .deformers
        .iter()
        .filter(|d| d.parent.as_ref() == Some(id))
        .map(|d| format!("deformer `{}`", d.id))
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| m.parent.as_ref() == Some(id))
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .collect();
    ensure_no_children("deformer", id, children)?;
    model.deformers.remove(index);
    for deformer in &mut model.deformers {
        if deformer.mirror.as_ref() == Some(id) {
            deformer.mirror = None;
        }
    }
    Ok(())
}

/// メッシュを削除する。マスクの参照から外し、このメッシュを使うグルーと当たり判定を削除する。
fn remove_mesh(model: &mut Model, id: &MeshId) -> Result<(), EditError> {
    let index = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    model.meshes.remove(index);
    for mesh in &mut model.meshes {
        mesh.masks.retain(|m| &m.mesh != id);
    }
    model.glues.retain(|g| &g.mesh_a != id && &g.mesh_b != id);
    model.hit_areas.retain(|h| &h.mesh != id);
    for mesh in &mut model.meshes {
        if mesh.mirror.as_ref() == Some(id) {
            mesh.mirror = None;
        }
    }
    Ok(())
}

/// `meshes` の中でメッシュ `id` を `before` の直前 (`None` なら末尾) へ移す。
fn reorder_mesh(model: &mut Model, id: &MeshId, before: Option<&MeshId>) -> Result<(), EditError> {
    if before == Some(id) {
        return Err(EditError::SelfReference {
            kind: "mesh",
            id: id.to_string(),
            field: "before",
        });
    }
    let from = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    if let Some(before) = before
        && !model.meshes.iter().any(|m| &m.id == before)
    {
        return Err(EditError::NotFound {
            kind: "mesh",
            id: before.to_string(),
        });
    }
    let moved = model.meshes.remove(from);
    let to = match before {
        Some(before) => model
            .meshes
            .iter()
            .position(|m| &m.id == before)
            .unwrap_or(model.meshes.len()),
        None => model.meshes.len(),
    };
    model.meshes.insert(to, moved);
    Ok(())
}
