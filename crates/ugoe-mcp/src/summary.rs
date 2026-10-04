//! モデルの要約と、要素 1 つの JSON。エージェントがモデルの構成を読むためのもの。
//!
//! 要約は ID・名前・親子・格子の軸とキー (と着せ替えのスロット) だけを載せ、頂点の座標やキーフォームの中身は載せない
//! (大きなモデルでも数千トークンに収めるため)。中身は `get_element` で要素ごとに読む。

use serde::Serialize;
use serde_json::{Value, json};
use ugoe_editor_core::Document;
use ugoe_model::KeyformGrid;
use ugoe_model::deformer::DeformerKind;

/// モデルの要約。
pub fn model_summary(doc: &Document) -> Value {
    let model = doc.model();
    let parameters: Vec<Value> = model
        .parameters
        .iter()
        .map(|p| {
            let mut v = json!({
                "id": p.id,
                "name": p.name,
                "min": p.min,
                "default": p.default,
                "max": p.max,
                "kind": p.kind,
            });
            insert_some(&mut v, "mirror", &p.mirror);
            insert_some(&mut v, "group", &p.group);
            if p.internal {
                v["internal"] = json!(true);
            }
            v
        })
        .collect();
    let parts: Vec<Value> = model
        .parts
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "parent": p.parent,
                "keyforms": grid(&p.keyforms),
            })
        })
        .collect();
    let deformers: Vec<Value> = model
        .deformers
        .iter()
        .map(|d| {
            let mut v = json!({
                "id": d.id,
                "name": d.name,
                "part": d.part,
                "parent": d.parent,
            });
            match &d.kind {
                DeformerKind::Warp(w) => {
                    v["kind"] = json!("warp");
                    v["rows"] = json!(w.rows);
                    v["cols"] = json!(w.cols);
                    v["keyforms"] = grid(&w.keyforms);
                }
                DeformerKind::Rotation(r) => {
                    v["kind"] = json!("rotation");
                    v["keyforms"] = grid(&r.keyforms);
                }
                DeformerKind::Unknown => v["kind"] = json!("unknown"),
            }
            insert_some(&mut v, "mirror", &d.mirror);
            v
        })
        .collect();
    let meshes: Vec<Value> = model
        .meshes
        .iter()
        .map(|m| {
            let mut v = json!({
                "id": m.id,
                "name": m.name,
                "part": m.part,
                "parent": m.parent,
                "texture": model.textures.get(m.texture as usize).map(|t| &t.name),
                "vertices": m.uvs.len(),
                "triangles": m.indices.len() / 3,
                "keyforms": grid(&m.keyforms),
            });
            if !m.masks.is_empty() {
                v["masks"] = json!(m.masks);
            }
            insert_some(&mut v, "mirror", &m.mirror);
            v
        })
        .collect();
    let glues: Vec<Value> = model
        .glues
        .iter()
        .map(|g| {
            json!({
                "id": g.id,
                "mesh_a": g.mesh_a,
                "mesh_b": g.mesh_b,
                "pairs": g.pairs.len(),
                "keyforms": grid(&g.keyforms),
            })
        })
        .collect();
    let textures: Vec<Value> = model
        .textures
        .iter()
        .map(|t| json!({ "name": t.name, "width": t.width, "height": t.height }))
        .collect();
    let physics = doc.physics().map(|p| {
        p.rigs
            .iter()
            .map(|r| json!({ "id": r.id, "name": r.name, "particles": r.particles.len() }))
            .collect::<Vec<_>>()
    });
    json!({
        "name": model.meta.name,
        "canvas": { "width": model.meta.canvas.width, "height": model.meta.canvas.height },
        "parameters": parameters,
        "parameter_groups": model.parameter_groups,
        "parts": parts,
        "deformers": deformers,
        "meshes": meshes,
        "glues": glues,
        "textures": textures,
        "physics_rigs": physics,
        "slots": model.slots,
        "motions": doc.motions().iter().map(|m| &m.name).collect::<Vec<_>>(),
        "expressions": doc.expressions().iter().map(|e| &e.name).collect::<Vec<_>>(),
    })
}

/// キーフォーム格子の形: 軸 (パラメータとキー)・セルの数・ブレンドシェイプの数。
fn grid<T>(grid: &KeyformGrid<T>) -> Value {
    let mut v = json!({
        "axes": grid
            .bindings
            .iter()
            .map(|b| json!({ "parameter": b.parameter, "keys": b.keys }))
            .collect::<Vec<_>>(),
        "cells": grid.keyforms.len(),
    });
    if !grid.blend_shapes.is_empty() {
        v["blend_shapes"] = grid
            .blend_shapes
            .iter()
            .map(|b| json!(b.bindings.iter().map(|b| &b.parameter).collect::<Vec<_>>()))
            .collect();
    }
    v
}

fn insert_some<T: Serialize>(v: &mut Value, key: &str, value: &Option<T>) {
    if let Some(value) = value {
        v[key] = json!(value);
    }
}

/// `get_element` で読める要素の種類。
pub const ELEMENT_KINDS: &[&str] = &[
    "parameter",
    "part",
    "deformer",
    "mesh",
    "glue",
    "physics_rig",
    "slot",
    "motion",
    "expression",
];

/// 要素 1 つの JSON (モデルのファイルでの形そのまま)。見つからなければ `None`。
pub fn element(doc: &Document, kind: &str, id: &str) -> Option<Result<Value, serde_json::Error>> {
    fn find<'a, T: Serialize + 'a>(
        items: impl IntoIterator<Item = &'a T>,
        key: impl Fn(&T) -> String,
        id: &str,
    ) -> Option<Result<Value, serde_json::Error>> {
        items
            .into_iter()
            .find(|item| key(item) == id)
            .map(serde_json::to_value)
    }
    let model = doc.model();
    match kind {
        "parameter" => find(&model.parameters, |p| p.id.to_string(), id),
        "part" => find(&model.parts, |p| p.id.to_string(), id),
        "deformer" => find(&model.deformers, |d| d.id.to_string(), id),
        "mesh" => find(&model.meshes, |m| m.id.to_string(), id),
        "glue" => find(&model.glues, |g| g.id.to_string(), id),
        "physics_rig" => find(
            doc.physics().map(|p| p.rigs.as_slice()).unwrap_or_default(),
            |r| r.id.to_string(),
            id,
        ),
        "slot" => find(&model.slots, |s| s.id.to_string(), id),
        "motion" => find(doc.motions(), |m| m.name.clone(), id),
        "expression" => find(doc.expressions(), |e| e.name.clone(), id),
        _ => None,
    }
}
