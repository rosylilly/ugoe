use std::fmt::Display;
use std::sync::Arc;

use ugoe_model::deformer::DeformerKind;
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{Canvas, DeformerId, KeyformOrigin, MeshId, Model, ParamId, PartId, TextureRef};
use ugoe_rigging::{InferError, MirrorOptions};

use crate::command::KeyformTarget;
use crate::grid::{self, CellError};
use crate::mesh_edit;
use crate::texture::{Textures, is_valid_texture_name, png_size};
use crate::{Command, automesh, inference};

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
    /// テクスチャ名がファイル名に使えない (`crate::texture::is_valid_texture_name`)。
    #[error("texture name `{name}` is invalid (use [A-Za-z0-9_.-], not starting with `.`)")]
    InvalidTextureName { name: String },
    /// テクスチャの画像が PNG でない (または IHDR が読めない)。
    #[error("texture `{name}`: image is not a PNG")]
    NotPng { name: String },
    /// キャンバスの大きさが 0 以下か有限でない。
    #[error("invalid canvas size {width}x{height}")]
    InvalidCanvas { width: f32, height: f32 },
    /// メッシュを作り直せない (`Command::RegenerateMesh` の条件を満たさない)。
    #[error("mesh `{id}` cannot be regenerated: {reason}")]
    CannotRegenerateMesh { id: String, reason: String },
    /// 自動メッシュの生成に失敗した (画像が透明、設定が不正など)。
    #[error("mesh `{id}`: automatic meshing failed: {message}")]
    Mesher { id: String, message: String },
    #[error("mesh `{id}`: vertex {index} out of range (vertices = {vertices})")]
    VertexOutOfRange {
        id: String,
        index: u16,
        vertices: usize,
    },
    /// 頂点の削除で、メッシュの頂点が 3 つ未満になる。
    #[error("mesh `{id}`: only {remaining} vertices would remain (at least 3 are needed)")]
    TooFewVertices { id: String, remaining: usize },
    /// メッシュの形の編集ができない (退化した三角形・同じ UV の頂点・評価できないモデルなど)。
    #[error("mesh `{id}`: {reason}")]
    MeshGeometry { id: String, reason: String },
    /// 適用するとモデルが検証 (`ugoe_model::validate`) に通らなくなる。新たに生じるエラーの一覧。
    #[error("command would make the model invalid: {}", join(.0))]
    Invalid(Vec<ValidationError>),
    /// キーフォームの推定 (ミラー・`derived` のセルの作り直しを含む) に失敗した。
    /// `target` は書き込み先の格子の要素 (``mesh `EyeL` `` のような表記)。
    #[error("{target}: keyform inference failed: {error}")]
    Inference { target: String, error: InferError },
    /// 格子の推定の方法が未知 (新しい版のファイル) なので、その設定では推定できない。
    #[error("{target}: keyform inference method is not supported by this version")]
    UnsupportedInference { target: String },
}

fn join(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// 編集中のモデル・テクスチャと履歴。
///
/// 暫定実装としてスナップショット方式の Undo を使う (`docs/adr/0005-editor-command-model.md`)。
/// テクスチャの画像は `Arc` で共有するので、スナップショットでバイト列は複製されない。
#[derive(Debug)]
pub struct Document {
    current: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// 次に作る状態の番号。
    next_state: u64,
    /// 最後に保存した状態の番号 (未保存なら `None`)。
    saved: Option<u64>,
}

/// ある時点のドキュメントの状態。`state` は状態ごとに一意な番号 (変更の有無の判定に使う)。
#[derive(Debug, Clone)]
struct Snapshot {
    model: Model,
    textures: Textures,
    state: u64,
}

impl Document {
    /// テクスチャの画像を持たないドキュメント。
    pub fn new(model: Model) -> Self {
        Self::with_textures(model, Textures::new())
    }

    /// モデルとテクスチャの画像 (`Model::textures` の名前 → PNG) からドキュメントを作る。
    /// 作った時点の状態は「保存済み」とみなす。
    pub fn with_textures(model: Model, textures: Textures) -> Self {
        Self {
            current: Snapshot {
                model,
                textures,
                state: 0,
            },
            undo: Vec::new(),
            redo: Vec::new(),
            next_state: 1,
            saved: Some(0),
        }
    }

    pub fn model(&self) -> &Model {
        &self.current.model
    }

    /// テクスチャ名 → PNG のバイト列。
    pub fn textures(&self) -> &Textures {
        &self.current.textures
    }

    /// 名前でテクスチャの PNG を引く。
    pub fn texture(&self, name: &str) -> Option<&[u8]> {
        self.current.textures.get(name).map(|png| &**png)
    }

    /// 最後に [`Document::mark_saved`] した (または作った) ときから状態が変わっているか。
    /// Undo で保存時の状態に戻れば `false` に戻る。
    pub fn is_modified(&self) -> bool {
        self.saved != Some(self.current.state)
    }

    /// 現在の状態を「保存済み」にする。
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.current.state);
    }

    /// コマンドを適用する。失敗時はモデルを変更しない。
    ///
    /// 適用後のモデルを `ugoe_model::validate::validate` で検査し、適用前になかった
    /// 検証エラーが生じたら [`EditError::Invalid`] で拒否する。検証を通るモデルに対しては、
    /// 成功した適用の後も必ず検証を通る。`Batch` は全体を適用した後に 1 回だけ検査する。
    ///
    /// 検査の前に、コマンドが編集したキーフォーム格子の `derived` のセルを格子の推定の設定で
    /// 作り直す (同じ Undo 単位。`docs/editor.md#推定とミラー`)。作り直せなければ拒否する。
    pub fn apply(&mut self, command: Command) -> Result<(), EditError> {
        let mut next = self.current.clone();
        let mut touched = Vec::new();
        apply_to(&mut next.model, &mut next.textures, command, &mut touched)?;
        inference::rederive(&mut next.model, &touched)?;
        let before = validate(&self.current.model);
        let introduced: Vec<_> = validate(&next.model)
            .into_iter()
            .filter(|e| !before.contains(e))
            .collect();
        if !introduced.is_empty() {
            return Err(EditError::Invalid(introduced));
        }
        next.state = self.next_state;
        self.next_state += 1;
        self.undo.push(std::mem::replace(&mut self.current, next));
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.current, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.current, next));
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

/// コマンドを適用する。
///
/// キーフォーム格子のセル (値や由来) を変えたコマンドは、その格子の要素を `touched` に積む。
/// 積んだ格子の `derived` のセルは、[`Document::apply`] が適用の後にまとめて作り直す
/// ([`inference::rederive`])。新しいコマンドもセルを変えたら積むこと。推定そのもの
/// (`InferKeyforms`・`MirrorKeyforms`) は、書いた結果を上書きしないように積まない。
fn apply_to(
    model: &mut Model,
    textures: &mut Textures,
    command: Command,
    touched: &mut Vec<KeyformTarget>,
) -> Result<(), EditError> {
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
                apply_to(model, textures, c, touched)?;
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
        Command::SetPartKeyforms { id, keyforms } => {
            grid::replace(&mut part(model, &id)?.keyforms, keyforms);
            touched.push(KeyformTarget::Part(id));
        }
        Command::AddDeformer { deformer } => {
            ensure_new(&model.deformers, "deformer", &deformer.id, |d| &d.id)?;
            model.deformers.push(deformer);
        }
        Command::RemoveDeformer { id } => remove_deformer(model, &id)?,
        Command::RenameDeformer { id, name } => deformer(model, &id)?.name = name,
        Command::SetDeformerParent { id, parent } => deformer(model, &id)?.parent = parent,
        Command::SetDeformerPart { id, part } => deformer(model, &id)?.part = part,
        Command::SetWarpKeyforms { id, keyforms } => {
            match &mut deformer(model, &id)?.kind {
                DeformerKind::Warp(warp) => grid::replace(&mut warp.keyforms, keyforms),
                DeformerKind::Rotation(_) | DeformerKind::Unknown => {
                    return Err(EditError::DeformerKindMismatch {
                        id: id.to_string(),
                        expected: "warp",
                    });
                }
            }
            touched.push(KeyformTarget::Deformer(id));
        }
        Command::SetRotationKeyforms { id, keyforms } => {
            match &mut deformer(model, &id)?.kind {
                DeformerKind::Rotation(rotation) => grid::replace(&mut rotation.keyforms, keyforms),
                DeformerKind::Warp(_) | DeformerKind::Unknown => {
                    return Err(EditError::DeformerKindMismatch {
                        id: id.to_string(),
                        expected: "rotation",
                    });
                }
            }
            touched.push(KeyformTarget::Deformer(id));
        }
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
        Command::SetMeshKeyforms { id, keyforms } => {
            grid::replace(&mut mesh(model, &id)?.keyforms, keyforms);
            touched.push(KeyformTarget::Mesh(id));
        }
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
            grid::set_authored(&mut m.keyforms, index);
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::ReorderMesh { id, before } => reorder_mesh(model, &id, before.as_ref())?,
        Command::AddTexture { name, png } => add_texture(model, textures, name, png)?,
        Command::SetTextureImage { name, png } => set_texture_image(model, textures, name, png)?,
        Command::RemoveTexture { name } => remove_texture(model, textures, &name)?,
        Command::SetCanvas {
            width,
            height,
            origin,
        } => {
            if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
                return Err(EditError::InvalidCanvas { width, height });
            }
            model.meta.canvas = Canvas {
                width,
                height,
                origin,
            };
        }
        Command::RegenerateMesh { id, options } => {
            automesh::regenerate_mesh(model, textures, &id, &options)?;
        }
        Command::InferKeyforms { target, method } => {
            inference::infer_keyforms(model, &target, method)?;
        }
        Command::MirrorKeyforms {
            source,
            target,
            axis,
            tolerance,
            overwrite_authored,
        } => {
            let options = MirrorOptions {
                axis,
                tolerance,
                overwrite_authored,
            };
            inference::mirror(model, &source, &target, options)?;
        }
        Command::MarkKeyformAuthored { target, cells } => {
            inference::mark(model, &target, &cells, KeyformOrigin::Authored)?;
            touched.push(target);
        }
        Command::MarkKeyformDerived { target, cells } => {
            inference::mark(model, &target, &cells, KeyformOrigin::Derived)?;
            touched.push(target);
        }
        // 頂点の編集はすべてのセルの位置を変えるので、`derived` のセルは作り直す。
        Command::MoveMeshVertices { id, moves } => {
            mesh_edit::move_vertices(model, &id, &moves)?;
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::AddMeshVertex { id, position } => {
            mesh_edit::add_vertex(model, &id, position)?;
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::RemoveMeshVertices { id, indices } => {
            mesh_edit::remove_vertices(model, &id, &indices)?;
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::RetriangulateMesh { id } => mesh_edit::retriangulate(model, &id)?,
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
            // 未知の種類はキーフォームを持たない。
            DeformerKind::Unknown => {}
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

/// PNG の寸法を読む。PNG でなければ [`EditError::NotPng`]。
fn png_dimensions(name: &str, png: &[u8]) -> Result<(u32, u32), EditError> {
    png_size(png).ok_or_else(|| EditError::NotPng {
        name: name.to_owned(),
    })
}

/// テクスチャを `Model::textures` の末尾に追加する。寸法は PNG から読む。
///
/// 名前は大文字と小文字を区別せずに重複を拒否する (大文字小文字を区別しないファイルシステムで
/// `textures/<name>.png` が衝突しないように)。
fn add_texture(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    if !is_valid_texture_name(&name) {
        return Err(EditError::InvalidTextureName { name });
    }
    if model
        .textures
        .iter()
        .any(|t| t.name.eq_ignore_ascii_case(&name))
    {
        return Err(EditError::DuplicateId {
            kind: "texture",
            id: name,
        });
    }
    let (width, height) = png_dimensions(&name, &png)?;
    model.textures.push(TextureRef {
        name: name.clone(),
        width,
        height,
    });
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// テクスチャの画像を差し替える。`TextureRef` の寸法も PNG に合わせて更新する。
fn set_texture_image(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    let (width, height) = png_dimensions(&name, &png)?;
    let texture = find_mut(&mut model.textures, "texture", &name, |t| &t.name)?;
    (texture.width, texture.height) = (width, height);
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// テクスチャを削除する。使っているメッシュがあれば拒否する。
/// 後ろのテクスチャを指すメッシュの `texture` (インデックス) は 1 つ詰める。
fn remove_texture(model: &mut Model, textures: &mut Textures, name: &str) -> Result<(), EditError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == name)
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: name.to_owned(),
        })?;
    let children = model
        .meshes
        .iter()
        .filter(|m| usize::try_from(m.texture).is_ok_and(|t| t == index))
        .map(|m| format!("mesh `{}`", m.id))
        .collect();
    ensure_no_children("texture", &name, children)?;
    model.textures.remove(index);
    for mesh in &mut model.meshes {
        if usize::try_from(mesh.texture).is_ok_and(|t| t > index) {
            mesh.texture -= 1;
        }
    }
    textures.remove(name);
    Ok(())
}
