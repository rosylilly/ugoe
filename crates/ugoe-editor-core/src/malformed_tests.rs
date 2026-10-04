//! 「不正だが読み込めるデータ」への編集のテスト。
//!
//! 読み込んだファイルは検証しない (問題を増やさない編集はできる) ので、編集の関数は空の列・未知の値・
//! 長さの合わない格子を受け取りうる。そのようなモデルにコマンドを適用しても、拒否するか適用するかは
//! 問わず、パニックしないことを確かめる (T0668 の横断・T0014)。

use std::panic::{AssertUnwindSafe, catch_unwind};

use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::mesh::{BlendMode, MaskMode};
use ugoe_model::parameter::ParameterKind;
use ugoe_model::{
    KeyformBinding, KeyformGrid, KeyformInference, KeyformInterpolation, KeyformOrigin,
    MirrorSource, Model, fixtures,
};

use crate::VertexMove;
use crate::{ChildRef, Command, Document, InferMethod, Keyform, KeyformCell, KeyformTarget};

/// 格子の壊し方。
#[derive(Debug, Clone, Copy)]
enum Corruption {
    /// セルがない (軸はある)。
    NoCells,
    /// セルが 1 つ足りない (1 つしかなければ 1 つ多い)。
    CellCountMismatch,
    /// 最初の軸のキーがない。
    KeylessAxis,
    /// 由来の数がセルの数と合わず、ミラー元の要素がない。
    OriginsMismatch,
    /// 未知の値 (補間・由来・推定の設定)。
    UnknownValues,
    /// 軸もセルもないブレンドシェイプ格子と、セルの数の合わないブレンドシェイプ格子。
    BrokenBlendShapes,
}

const CORRUPTIONS: [Corruption; 6] = [
    Corruption::NoCells,
    Corruption::CellCountMismatch,
    Corruption::KeylessAxis,
    Corruption::OriginsMismatch,
    Corruption::UnknownValues,
    Corruption::BrokenBlendShapes,
];

fn corrupt<T: Clone>(grid: &mut KeyformGrid<T>, how: Corruption, parameter: &str) {
    match how {
        Corruption::NoCells => grid.keyforms.clear(),
        Corruption::CellCountMismatch => {
            if grid.keyforms.len() > 1 {
                grid.keyforms.pop();
            } else if let Some(first) = grid.keyforms.first().cloned() {
                grid.keyforms.push(first);
            }
        }
        Corruption::KeylessAxis => match grid.bindings.first_mut() {
            Some(binding) => binding.keys.clear(),
            None => grid.bindings.push(KeyformBinding {
                parameter: parameter.into(),
                keys: Vec::new(),
            }),
        },
        Corruption::OriginsMismatch => {
            grid.origins = vec![KeyformOrigin::Mirror; grid.keyforms.len() + 2];
            grid.mirror_source = Some(MirrorSource {
                id: "Missing".into(),
                axis: None,
                tolerance: None,
            });
        }
        Corruption::UnknownValues => {
            grid.interpolation = KeyformInterpolation::Unknown;
            grid.origins = vec![KeyformOrigin::Unknown; grid.keyforms.len()];
            grid.inference = Some(KeyformInference::Unknown);
        }
        Corruption::BrokenBlendShapes => {
            let mut short = KeyformGrid::new(
                vec![KeyformBinding {
                    parameter: parameter.into(),
                    keys: vec![0.0, 1.0],
                }],
                grid.keyforms.iter().take(1).cloned().collect(),
            );
            short
                .blend_shapes
                .push(KeyformGrid::new(Vec::new(), Vec::new()));
            grid.blend_shapes
                .push(KeyformGrid::new(Vec::new(), Vec::new()));
            grid.blend_shapes.push(short);
        }
    }
}

/// モデルのすべての格子を `how` で壊し、要素にも同じ種類の不正を加える。
fn corrupted(model: &Model, how: Corruption) -> Model {
    let mut model = model.clone();
    let parameter = model
        .parameters
        .first()
        .map_or_else(|| "X".to_owned(), |p| p.id.to_string());
    for part in &mut model.parts {
        corrupt(&mut part.keyforms, how, &parameter);
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(warp) => {
                corrupt(&mut warp.keyforms, how, &parameter);
                if let Some(first) = warp.keyforms.keyforms.first_mut() {
                    // 制御点の数が `rows`・`cols` と合わない。
                    first.points.pop();
                }
            }
            DeformerKind::Rotation(rotation) => corrupt(&mut rotation.keyforms, how, &parameter),
            DeformerKind::Unknown => {}
        }
    }
    for mesh in &mut model.meshes {
        corrupt(&mut mesh.keyforms, how, &parameter);
        match how {
            Corruption::NoCells => {
                mesh.uvs.clear();
                mesh.indices.clear();
                mesh.depth = Some(Vec::new());
            }
            Corruption::CellCountMismatch => {
                // 頂点の数・三角形の添字・深度の数が合わない。
                if let Some(first) = mesh.keyforms.keyforms.first_mut() {
                    first.positions.pop();
                }
                mesh.indices.push(u16::MAX);
                mesh.depth = Some(vec![1.0]);
            }
            Corruption::UnknownValues => {
                mesh.blend = BlendMode::Unknown;
                mesh.mask_mode = MaskMode::Unknown;
            }
            _ => {}
        }
    }
    for glue in &mut model.glues {
        corrupt(&mut glue.keyforms, how, &parameter);
        if matches!(how, Corruption::CellCountMismatch) {
            glue.pairs.push(ugoe_model::mesh::GluePair {
                a: u16::MAX,
                b: u16::MAX,
                weight: 0.5,
            });
        }
    }
    if matches!(how, Corruption::UnknownValues) {
        for parameter in &mut model.parameters {
            parameter.kind = ParameterKind::Unknown;
        }
        if let Some(deformer) = model.deformers.last_mut() {
            deformer.kind = DeformerKind::Unknown;
        }
    }
    model
}

/// 格子の最初のセル (各軸の最初のキー)。
fn first_cell<T>(grid: &KeyformGrid<T>) -> KeyformCell {
    grid.bindings
        .iter()
        .filter_map(|b| b.keys.first().map(|&k| (b.parameter.clone(), k)))
        .collect()
}

/// 要素の格子を対象にするコマンド。
fn grid_commands<T>(
    target: &KeyformTarget,
    grid: &KeyformGrid<T>,
    keyform: Option<Keyform>,
    parameter: &str,
) -> Vec<Command> {
    let key = grid
        .bindings
        .first()
        .and_then(|b| b.keys.first().copied())
        .unwrap_or(0.0);
    let mut commands = vec![
        Command::AddKeyformKey {
            target: target.clone(),
            parameter: parameter.into(),
            key: 0.25,
        },
        Command::RemoveKeyformKey {
            target: target.clone(),
            parameter: parameter.into(),
            key,
        },
        Command::InferKeyforms {
            target: target.clone(),
            method: InferMethod::FromGrid,
            overwrite_derived: true,
        },
        Command::InferKeyforms {
            target: target.clone(),
            method: InferMethod::DepthRotation,
            overwrite_derived: true,
        },
        Command::MirrorKeyforms {
            source: target.clone(),
            target: target.clone(),
            axis: Some(0.5),
            tolerance: None,
            overwrite_authored: true,
        },
        Command::MarkKeyformDerived {
            target: target.clone(),
            cells: vec![first_cell(grid)],
        },
        Command::MarkKeyformAuthored {
            target: target.clone(),
            cells: vec![first_cell(grid)],
        },
        Command::SetKeyformInterpolation {
            target: target.clone(),
            blend_shape: None,
            interpolation: KeyformInterpolation::Smooth,
        },
        Command::SetKeyformInterpolation {
            target: target.clone(),
            blend_shape: Some(parameter.into()),
            interpolation: KeyformInterpolation::Linear,
        },
    ];
    if let Some(keyform) = keyform {
        commands.push(Command::SetKeyformCell {
            target: target.clone(),
            cell: first_cell(grid),
            keyform,
        });
    }
    commands
}

/// モデルのすべての要素に、格子・形・階層を編集するコマンドを作る。
fn commands(model: &Model) -> Vec<Command> {
    let parameter = model
        .parameters
        .first()
        .map_or_else(|| "X".to_owned(), |p| p.id.to_string());
    let first_deformer = model.deformers.first().map(|d| d.id.clone());
    let mut commands = Vec::new();
    for p in &model.parameters {
        commands.push(Command::RemoveParameter { id: p.id.clone() });
        commands.push(Command::SetParameterRange {
            id: p.id.clone(),
            min: -2.0,
            default: 0.5,
            max: 2.0,
        });
    }
    for part in &model.parts {
        let target = KeyformTarget::Part(part.id.clone());
        let keyform = part.keyforms.keyforms.first().map(|k| Keyform::Part(*k));
        commands.extend(grid_commands(&target, &part.keyforms, keyform, &parameter));
        commands.push(Command::RemovePart {
            id: part.id.clone(),
        });
    }
    for mesh in &model.meshes {
        let id = mesh.id.clone();
        let target = KeyformTarget::Mesh(id.clone());
        let keyform = mesh.keyforms.keyforms.first().cloned().map(Keyform::Mesh);
        commands.extend(grid_commands(
            &target,
            &mesh.keyforms,
            keyform.clone(),
            &parameter,
        ));
        if let Some(Keyform::Mesh(keyform)) = keyform {
            commands.push(Command::SetMeshKeyform {
                id: id.clone(),
                cell: first_cell(&mesh.keyforms),
                keyform,
            });
        }
        commands.extend([
            Command::MoveMeshVertices {
                id: id.clone(),
                moves: vec![VertexMove {
                    index: 0,
                    delta: Vec2::new(1.0, 1.0),
                }],
            },
            Command::AddMeshVertex {
                id: id.clone(),
                position: Vec2::new(100.0, 100.0),
            },
            Command::RemoveMeshVertices {
                id: id.clone(),
                indices: vec![0],
            },
            Command::RetriangulateMesh { id: id.clone() },
            Command::RegenerateMesh {
                id: id.clone(),
                options: Default::default(),
            },
            Command::SetMeshDrawOrder {
                id: id.clone(),
                draw_order: 10.0,
            },
            Command::CreateDrawClone {
                source: id.clone(),
                id: None,
                name: String::new(),
            },
            Command::SyncDrawClone { id: id.clone() },
            Command::SetMirrorPair {
                target: target.clone(),
                mirror: Some(id.to_string()),
                axis: Some(0.5),
            },
            Command::SetDepth {
                target: ChildRef::Mesh(id.clone()),
                depth: Some(vec![1.0; mesh.uvs.len()]),
            },
            Command::ReparentPreservingLook {
                element: ChildRef::Mesh(id.clone()),
                new_parent: None,
            },
            Command::ReparentPreservingLook {
                element: ChildRef::Mesh(id.clone()),
                new_parent: first_deformer.clone(),
            },
            Command::CreateWarpDeformer {
                id: None,
                name: String::new(),
                children: vec![ChildRef::Mesh(id.clone())],
                parts: Vec::new(),
                rows: 2,
                cols: 2,
                parent: None,
            },
            Command::CreateRotationDeformer {
                id: None,
                name: String::new(),
                children: vec![ChildRef::Mesh(id.clone())],
                parts: Vec::new(),
                origin: None,
                parent: None,
            },
            Command::RemoveMesh { id },
        ]);
    }
    for deformer in &model.deformers {
        let id = deformer.id.clone();
        let target = KeyformTarget::Deformer(id.clone());
        match &deformer.kind {
            DeformerKind::Warp(warp) => {
                let keyform = warp.keyforms.keyforms.first().cloned().map(Keyform::Warp);
                commands.extend(grid_commands(&target, &warp.keyforms, keyform, &parameter));
            }
            DeformerKind::Rotation(rotation) => {
                let keyform = rotation
                    .keyforms
                    .keyforms
                    .first()
                    .map(|k| Keyform::Rotation(*k));
                commands.extend(grid_commands(
                    &target,
                    &rotation.keyforms,
                    keyform,
                    &parameter,
                ));
            }
            DeformerKind::Unknown => {
                commands.extend(grid_commands(
                    &target,
                    &KeyformGrid::<()>::new(Vec::new(), Vec::new()),
                    None,
                    &parameter,
                ));
            }
        }
        commands.extend([
            Command::SetDepth {
                target: ChildRef::Deformer(id.clone()),
                depth: Some(vec![1.0; 4]),
            },
            Command::ReparentPreservingLook {
                element: ChildRef::Deformer(id.clone()),
                new_parent: None,
            },
            Command::MoveRotationOrigin {
                id: id.clone(),
                delta: Vec2::new(4.0, 4.0),
            },
            Command::CreateWarpDeformer {
                id: None,
                name: String::new(),
                children: vec![ChildRef::Deformer(id.clone())],
                parts: Vec::new(),
                rows: 1,
                cols: 1,
                parent: None,
            },
            Command::CreatePhysicsRig {
                id: None,
                name: String::new(),
                deformers: vec![id.clone()],
            },
            Command::UnwrapDeformer { id: id.clone() },
            Command::RemoveDeformer { id },
        ]);
    }
    for glue in &model.glues {
        let target = KeyformTarget::Glue(glue.id.clone());
        let keyform = glue.keyforms.keyforms.first().map(|k| Keyform::Glue(*k));
        commands.extend(grid_commands(&target, &glue.keyforms, keyform, &parameter));
        commands.push(Command::SetGlue {
            glue: glue.clone(),
            distance: Some(4.0),
        });
        commands.push(Command::RemoveGlue {
            id: glue.id.clone(),
        });
    }
    commands
}

#[test]
fn edits_on_malformed_models_do_not_panic() {
    let models = [
        fixtures::single_quad(),
        fixtures::rotation_chain(),
        fixtures::rotation_under_warp(),
        fixtures::glued_quads(),
        fixtures::keyform_inference(),
        fixtures::blend_shapes(),
    ];
    let mut panics = Vec::new();
    for model in &models {
        for how in CORRUPTIONS {
            let broken = corrupted(model, how);
            for command in commands(&broken) {
                let label = format!("{} / {how:?} / {command:?}", model.meta.name);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let mut doc = Document::new(broken.clone());
                    // 拒否してもよい。拒否したらモデルは変わらない。
                    if doc.apply(command).is_err() {
                        assert_eq!(doc.model(), &broken);
                    }
                }));
                if result.is_err() {
                    panics.push(label);
                }
            }
        }
    }
    assert!(panics.is_empty(), "panicked:\n{}", panics.join("\n"));
}
