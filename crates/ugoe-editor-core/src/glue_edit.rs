//! グルーの作成・設定・削除 (T0659)。規則は `docs/editor.md#グルーの編集`。
//!
//! 頂点の組は、基準の姿 (パラメータが既定値) でのキャンバス座標 (親のデフォーマで写した後・グルーの前) で、
//! メッシュ B の頂点ごとに `distance` 以内でいちばん近いメッシュ A の頂点を選んで作る。

use ugoe_core::Vec2;
use ugoe_model::mesh::{Glue, GlueKeyform, GluePair};
use ugoe_model::{GlueId, KeyformGrid, MeshId, Model};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::EditError;
use crate::ids;

/// 作るグルーの頂点の組の既定の重み (両方のメッシュの真ん中に寄せる)。
pub const DEFAULT_GLUE_WEIGHT: f32 = 0.5;

/// `CreateGlue`: `mesh_a` と `mesh_b` の、`distance` 以内の頂点をつなぐグルーを `glues` の末尾に加え、その ID を返す。
/// 重みは [`DEFAULT_GLUE_WEIGHT`]、強さは 1 (パラメータに依存しない 1 つのキーフォーム)。
pub(crate) fn create(
    model: &mut Model,
    id: Option<GlueId>,
    mesh_a: MeshId,
    mesh_b: MeshId,
    distance: f32,
) -> Result<GlueId, EditError> {
    let id = match id {
        Some(id) => {
            if model.glues.iter().any(|g| g.id == id) {
                return Err(EditError::DuplicateId {
                    kind: "glue",
                    id: id.to_string(),
                });
            }
            id
        }
        None => {
            // モジュールのプロジェクトのメッシュ (`<モジュール>/<ID>`) は、接頭辞を除いた ID から作る。
            let short = |m: &MeshId| m.as_str().rsplit('/').next().unwrap_or("").to_owned();
            let base = format!("Glue_{}_{}", short(&mesh_a), short(&mesh_b));
            GlueId::new(ids::unique_id(
                &base,
                model.glues.iter().map(|g| g.id.as_str()),
            ))
        }
    };
    let pairs = pair_vertices(model, &mesh_a, &mesh_b, distance, DEFAULT_GLUE_WEIGHT)?;
    model.glues.push(Glue {
        id: id.clone(),
        mesh_a,
        mesh_b,
        pairs,
        keyforms: KeyformGrid::constant(GlueKeyform { intensity: 1.0 }),
    });
    Ok(id)
}

/// `SetGlue`: `glue.id` のグルーを `glue` で置き換える。`distance` があれば、頂点の組を両端のメッシュから
/// 作り直す (重みは `glue.pairs` の先頭の重み。組が無ければ [`DEFAULT_GLUE_WEIGHT`])。
pub(crate) fn set(
    model: &mut Model,
    mut glue: Glue,
    distance: Option<f32>,
) -> Result<(), EditError> {
    let index = position(model, glue.id.as_str())?;
    if let Some(distance) = distance {
        let weight = glue.pairs.first().map_or(DEFAULT_GLUE_WEIGHT, |p| p.weight);
        glue.pairs = pair_vertices(model, &glue.mesh_a, &glue.mesh_b, distance, weight)?;
    }
    if let Some(pair) = glue.pairs.iter().find(|p| !p.weight.is_finite()) {
        return Err(invalid(
            glue.id.as_str(),
            format!("weight {} is not finite", pair.weight),
        ));
    }
    model.glues[index] = glue;
    Ok(())
}

/// `RemoveGlue`: グルーを消す。
pub(crate) fn remove(model: &mut Model, id: &str) -> Result<(), EditError> {
    let index = position(model, id)?;
    model.glues.remove(index);
    Ok(())
}

/// 基準の姿のキャンバス座標で、メッシュ `mesh_b` の頂点ごとに `distance` 以内でいちばん近い `mesh_a` の頂点を
/// 選んだ組 (B の頂点の順)。組が 1 つもできなければエラー。
pub fn pair_vertices(
    model: &Model,
    mesh_a: &MeshId,
    mesh_b: &MeshId,
    distance: f32,
    weight: f32,
) -> Result<Vec<GluePair>, EditError> {
    let what = || format!("{mesh_a} + {mesh_b}");
    if !(distance.is_finite() && distance >= 0.0) {
        return Err(invalid(
            &what(),
            format!("distance {distance} is not valid"),
        ));
    }
    if !weight.is_finite() {
        return Err(invalid(&what(), format!("weight {weight} is not finite")));
    }
    if mesh_a == mesh_b {
        return Err(invalid(&what(), "the two meshes must be different".into()));
    }
    let index = |id: &MeshId| {
        model
            .meshes
            .iter()
            .position(|m| &m.id == id)
            .ok_or_else(|| EditError::NotFound {
                kind: "mesh",
                id: id.to_string(),
            })
    };
    let (a, b) = (index(mesh_a)?, index(mesh_b)?);
    let compiled = CompiledModel::compile(model.clone())
        .map_err(|e| invalid(&what(), format!("the model cannot be evaluated: {e}")))?;
    let instance = ModelInstance::new(compiled);
    let rest = |mesh: usize| -> Vec<Vec2> {
        let world = instance.mesh_parent_world(mesh);
        instance
            .mesh_local_positions(mesh)
            .into_iter()
            .map(|p| world.point(p))
            .collect()
    };
    let (points_a, points_b) = (rest(a), rest(b));
    let limit = distance * distance;
    let mut pairs = Vec::new();
    for (j, pb) in points_b.iter().enumerate() {
        let nearest = points_a
            .iter()
            .enumerate()
            .map(|(i, pa)| (i, pa.distance_squared(*pb)))
            .filter(|(_, d)| *d <= limit)
            .min_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, _)) = nearest {
            // 頂点の番号は u16 (メッシュの頂点は 65536 未満)。
            let (Ok(a), Ok(b)) = (u16::try_from(i), u16::try_from(j)) else {
                return Err(invalid(&what(), "too many vertices".into()));
            };
            pairs.push(GluePair { a, b, weight });
        }
    }
    if pairs.is_empty() {
        return Err(invalid(
            &what(),
            format!("no vertices are within {distance} px of each other"),
        ));
    }
    Ok(pairs)
}

fn position(model: &Model, id: &str) -> Result<usize, EditError> {
    model
        .glues
        .iter()
        .position(|g| g.id.as_str() == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "glue",
            id: id.to_owned(),
        })
}

fn invalid(id: &str, reason: String) -> EditError {
    EditError::InvalidGlue {
        id: id.to_owned(),
        reason,
    }
}
