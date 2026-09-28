//! 描画用の複製 (形を元のメッシュと共有する複製)。設計は `docs/editor.md#描画用の複製`。
//!
//! 複製は元と同じ UV・三角形・テクスチャ・親デフォーマを持つ普通のメッシュで、全頂点を元の頂点に
//! 一致させるグルー (ID は [`glue_id`]、`weight = 0`・`intensity = 1`) で形を共有する
//! (`docs/spec/evaluation.md#5-グルー`: `target = pa` なので `pb' = pa`)。モデルの形式は変えず、
//! 複製と元の対応はグルーの ID の規則で表す。

use ugoe_model::mesh::MeshKeyform;
use ugoe_model::mesh::{Glue, GlueKeyform, GluePair};
use ugoe_model::{KeyformGrid, Mesh, MeshId, Model};

use crate::grid;
use crate::keyform::parameter_defaults;
use crate::{EditError, ids};

/// 描画用の複製のグルーの ID の接頭辞。
pub const GLUE_PREFIX: &str = "DrawClone_";

/// 複製 `clone` と元をつなぐグルーの ID (`DrawClone_<複製の ID>`)。
pub fn glue_id(clone: &MeshId) -> String {
    format!("{GLUE_PREFIX}{clone}")
}

/// `glue` が描画用の複製のグルー (ID が `mesh_b` の [`glue_id`]) か。
pub fn is_link(glue: &Glue) -> bool {
    glue.id == glue_id(&glue.mesh_b)
}

/// 複製 `clone` の元のメッシュ。複製でなければ `None`。
pub fn original_of<'a>(model: &'a Model, clone: &MeshId) -> Option<&'a MeshId> {
    model
        .glues
        .iter()
        .find(|g| &g.mesh_b == clone && is_link(g))
        .map(|g| &g.mesh_a)
}

/// メッシュ `original` の複製 (グルーの順)。
pub fn clones_of<'a>(model: &'a Model, original: &MeshId) -> Vec<&'a MeshId> {
    model
        .glues
        .iter()
        .filter(|g| &g.mesh_a == original && is_link(g))
        .map(|g| &g.mesh_b)
        .collect()
}

/// 複製 `clone` が元と食い違っていないか。複製でなければ `false`。
///
/// 次のすべてを満たせば食い違っていない: UV・三角形・テクスチャ・親デフォーマが元と同じ、
/// キーフォームの頂点数が同じ、グルーのペアが全頂点の `(i, i, weight 0)`、グルーの強さが
/// すべてのセルで 1、グルーが元を使う他のグルーより後ろにある。
pub fn is_in_sync(model: &Model, clone: &MeshId) -> bool {
    let Some(index) = model
        .glues
        .iter()
        .position(|g| &g.mesh_b == clone && is_link(g))
    else {
        return false;
    };
    let glue = &model.glues[index];
    let (Some(original), Some(copy)) = (find(model, &glue.mesh_a), find(model, clone)) else {
        return false;
    };
    let n = original.uvs.len();
    copy.uvs == original.uvs
        && copy.indices == original.indices
        && copy.texture == original.texture
        && copy.parent == original.parent
        && copy.keyforms.all_keyforms().all(|k| k.positions.len() == n)
        && glue.pairs == identity_pairs(n)
        && glue.keyforms.all_keyforms().all(|k| k.intensity == 1.0)
        && !model.glues[index + 1..]
            .iter()
            .any(|g| g.mesh_a == glue.mesh_a || g.mesh_b == glue.mesh_a)
}

fn find<'a>(model: &'a Model, id: &MeshId) -> Option<&'a Mesh> {
    model.meshes.iter().find(|m| &m.id == id)
}

fn not_found(id: &MeshId) -> EditError {
    EditError::NotFound {
        kind: "mesh",
        id: id.to_string(),
    }
}

fn identity_pairs(n: usize) -> Vec<GluePair> {
    (0..n)
        .map(|i| {
            let i = i as u16;
            GluePair {
                a: i,
                b: i,
                weight: 0.0,
            }
        })
        .collect()
}

/// メッシュのパラメータが既定値のときのキーフォーム (基準の形)。
fn rest_keyform(model: &Model, mesh: &Mesh) -> Result<MeshKeyform, EditError> {
    let defaults = parameter_defaults(model);
    grid::rest_keyform(&mesh.keyforms, |p| defaults.get(p).copied()).map_err(|error| {
        EditError::InvalidCell {
            kind: "mesh",
            id: mesh.id.to_string(),
            error,
        }
    })
}

/// 形の編集 (頂点・三角形・メッシュの再生成) の対象が複製なら拒否する。複製の形は元に従う。
pub(crate) fn reject_shape_edit(model: &Model, id: &MeshId) -> Result<(), EditError> {
    match original_of(model, id) {
        Some(original) => Err(EditError::DrawCloneShape {
            id: id.to_string(),
            original: original.to_string(),
        }),
        None => Ok(()),
    }
}

/// `CreateDrawClone`: `source` の描画用の複製を作り、その ID を返す。
///
/// `source` が複製なら、その元の複製を作る (複製の複製は作らない)。複製は元の直後
/// (`meshes` の中。同じ描画順なら元のすぐ手前) に置き、グルーは `glues` の末尾に加える。
pub(crate) fn create(
    model: &mut Model,
    source: &MeshId,
    id: Option<MeshId>,
    name: String,
) -> Result<MeshId, EditError> {
    let original_id = original_of(model, source).unwrap_or(source).clone();
    let index = model
        .meshes
        .iter()
        .position(|m| m.id == original_id)
        .ok_or_else(|| not_found(source))?;
    let original = &model.meshes[index];
    let id = match id {
        Some(id) => {
            if find(model, &id).is_some() {
                return Err(EditError::DuplicateId {
                    kind: "mesh",
                    id: id.to_string(),
                });
            }
            id
        }
        None => {
            // メッシュとグルーの両方で空いている ID。
            let base = format!("{original_id}_Clone");
            let taken: Vec<String> = model
                .meshes
                .iter()
                .map(|m| m.id.to_string())
                .chain(
                    model
                        .glues
                        .iter()
                        .filter_map(|g| g.id.strip_prefix(GLUE_PREFIX).map(str::to_owned)),
                )
                .collect();
            MeshId::new(ids::unique_id(&base, taken.iter().map(String::as_str)))
        }
    };
    let glue = glue_id(&id);
    if model.glues.iter().any(|g| g.id == glue) {
        return Err(EditError::DuplicateId {
            kind: "glue",
            id: glue,
        });
    }
    let name = if name.is_empty() {
        format!("{}の複製", original.name)
    } else {
        name
    };
    let rest = rest_keyform(model, original)?;
    let clone = Mesh {
        id: id.clone(),
        name,
        keyforms: KeyformGrid::constant(rest),
        mirror: None,
        mirror_axis: None,
        depth: None,
        ..original.clone()
    };
    let n = clone.uvs.len();
    model.meshes.insert(index + 1, clone);
    model.glues.push(Glue {
        id: glue,
        mesh_a: original_id,
        mesh_b: id.clone(),
        pairs: identity_pairs(n),
        keyforms: KeyformGrid::constant(GlueKeyform { intensity: 1.0 }),
    });
    Ok(id)
}

/// 複製 `clone` を元に合わせ直す: UV・三角形・テクスチャ・親を写し、すべてのセルの頂点位置を
/// 元の基準の形にし、グルーのペアと強さを作り直して `glues` の末尾へ移す。
/// 描画順・不透明度・色・マスク・パーツなど、複製が持つ値は変えない。
fn sync_one(model: &mut Model, clone: &MeshId) -> Result<(), EditError> {
    let glue_index = model
        .glues
        .iter()
        .position(|g| &g.mesh_b == clone && is_link(g))
        .ok_or_else(|| EditError::NotDrawClone {
            id: clone.to_string(),
        })?;
    let original_id = model.glues[glue_index].mesh_a.clone();
    let original = find(model, &original_id).ok_or_else(|| not_found(&original_id))?;
    let rest = rest_keyform(model, original)?;
    let (uvs, indices, texture, parent) = (
        original.uvs.clone(),
        original.indices.clone(),
        original.texture,
        original.parent.clone(),
    );
    let n = uvs.len();
    let copy = model
        .meshes
        .iter_mut()
        .find(|m| &m.id == clone)
        .ok_or_else(|| not_found(clone))?;
    copy.uvs = uvs;
    copy.indices = indices;
    copy.texture = texture;
    copy.parent = parent;
    if copy.depth.as_ref().is_some_and(|d| d.len() != n) {
        copy.depth = None;
    }
    for keyform in copy.keyforms.all_keyforms_mut() {
        keyform.positions.clone_from(&rest.positions);
    }
    let mut glue = model.glues.remove(glue_index);
    glue.pairs = identity_pairs(n);
    for keyform in glue.keyforms.all_keyforms_mut() {
        keyform.intensity = 1.0;
    }
    model.glues.push(glue);
    Ok(())
}

/// `SyncDrawClone`: `id` が複製ならその複製を、元ならそのすべての複製を合わせ直す。
pub(crate) fn sync(model: &mut Model, id: &MeshId) -> Result<(), EditError> {
    if find(model, id).is_none() {
        return Err(not_found(id));
    }
    let targets: Vec<MeshId> = if original_of(model, id).is_some() {
        vec![id.clone()]
    } else {
        clones_of(model, id).into_iter().cloned().collect()
    };
    if targets.is_empty() {
        return Err(EditError::NotDrawClone { id: id.to_string() });
    }
    for clone in &targets {
        sync_one(model, clone)?;
    }
    Ok(())
}

/// コマンドの適用の後に呼ぶ: 形 (UV・三角形)・テクスチャ・親が `before` から変わった元のメッシュの
/// 複製を合わせ直す。これで頂点の追加・削除・三角形の作り直し・メッシュの再生成・親の付け替えに
/// 複製が追従する (どのコマンドで変わったかによらない)。
pub(crate) fn follow(before: &Model, after: &mut Model) -> Result<(), EditError> {
    let changed: Vec<MeshId> = after
        .glues
        .iter()
        .filter(|g| is_link(g))
        .filter(|g| {
            let (Some(old), Some(new)) = (find(before, &g.mesh_a), find(after, &g.mesh_a)) else {
                return false;
            };
            old.uvs != new.uvs
                || old.indices != new.indices
                || old.texture != new.texture
                || old.parent != new.parent
        })
        .map(|g| g.mesh_b.clone())
        .collect();
    for clone in &changed {
        if find(after, clone).is_some() {
            sync_one(after, clone)?;
        }
    }
    Ok(())
}
