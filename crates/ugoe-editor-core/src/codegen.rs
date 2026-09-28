//! Rust の型から生成するもの: エディタの TypeScript の型と、`Command` の JSON Schema。
//! 手順と生成物の置き場所は `docs/editor.md#型の生成`。
//!
//! - TypeScript の型は ts-rs、JSON Schema は schemars で作る。どちらも serde の属性を読むので、
//!   JSON の形 (タグ・名前・省略できるフィールド) は serde の実装と一致する。
//! - 未知の値 (`Unknown`) は書き出せないので、どちらにも含めない。
//! - 生成物はリポジトリに置く。`just gen` で作り直し、CI は最新かどうかを検査する。

use std::collections::BTreeMap;

use ts_rs::{Config, TS, TypeVisitor};
use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{BlendMode, KeyformGrid, MaskMode, MaskRef, Model, fixtures};

use crate::command::{InferMethod, KeyformTarget};
use crate::{AutoMeshOptions, Command, KeyformCell, VertexMove};

/// チェックインした `Command` の JSON Schema (`schema/command.schema.json`)。AI のツール定義に使う。
/// [`command_schema_json`] の出力と同じであることをテストで確かめる。
pub const COMMAND_SCHEMA_JSON: &str = include_str!("../schema/command.schema.json");

/// `Command` の JSON Schema (draft 2020-12、読み込み側の形: 省略できるフィールドは必須にしない)。
pub fn command_schema() -> schemars::Schema {
    schemars::schema_for!(Command)
}

/// [`command_schema`] をファイルに書く形 (整形した JSON と末尾の改行) にする。
pub fn command_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&command_schema()).map(|json| json + "\n")
}

/// 生成するファイルの先頭に置く注意書き。
pub const GENERATED_HEADER: &str = "// このファイルは生成物です。手で編集せず `just gen` で作り直してください (docs/editor.md#型の生成)。\n";

/// 複数の型の TypeScript の宣言を 1 つのファイルにまとめる。
///
/// [`TsBindings::add`] した型と、その型が参照する型をすべて (再帰的に) 宣言する。
/// 宣言は型の名前の順に並べるので、出力は追加の順序によらない。
#[derive(Debug)]
pub struct TsBindings {
    /// 型の名前 (ジェネリクスを除く) → 宣言。
    decls: BTreeMap<String, String>,
}

impl Default for TsBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl TsBindings {
    pub fn new() -> Self {
        Self {
            decls: BTreeMap::new(),
        }
    }

    /// エディタのフロントエンドが使う `ugoe-editor-core` / `ugoe-model` の型を追加する。
    pub fn with_editor_types(mut self) -> Self {
        self.add::<Command>();
        self.add::<Model>();
        self.add::<Motion>();
        self.add::<Expression>();
        self.add::<PhysicsSettings>();
        self
    }

    /// `T` とその依存を追加する。
    pub fn add<T: TS + 'static + ?Sized>(&mut self) {
        let cfg = ts_config();
        let name = T::ident(&cfg);
        if self.decls.contains_key(&name) {
            return;
        }
        let mut decl = T::docs().unwrap_or_default();
        decl.push_str("export ");
        decl.push_str(&T::decl(&cfg));
        self.decls.insert(name, decl);
        T::visit_dependencies(self);
    }

    /// ファイルの中身。
    pub fn render(&self) -> String {
        let mut out = GENERATED_HEADER.to_owned();
        for decl in self.decls.values() {
            out.push('\n');
            out.push_str(decl);
            out.push('\n');
        }
        out
    }
}

fn ts_config() -> Config {
    // u64 (目パチのシードなど) も JSON では数値なので、bigint ではなく number にする。
    Config::new().with_large_int("number")
}

impl TypeVisitor for TsBindings {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        // プリミティブや配列など、宣言を持たない型は飛ばす。
        if T::output_path().is_some() {
            self.add::<T>();
        }
    }
}

/// 生成した型の検査用の TypeScript: serde で書き出した JSON を、生成した型の値として書く。
///
/// フロントエンドの型検査 (`svelte-check`) がこのファイルを検査するので、生成した型と serde の
/// JSON がずれる (フィールド名・タグ・必須のフィールドが違う) と型エラーになる。
/// `types_module` はこのファイルから見た型のモジュール (例: `./types`)。
pub fn typescript_samples(types_module: &str) -> Result<String, serde_json::Error> {
    let models = [
        fixtures::single_quad(),
        fixtures::pose_and_effects(),
        fixtures::rotation_under_warp(),
        fixtures::glued_quads(),
        fixtures::mixed_masks(),
        fixtures::keyform_inference(),
    ];
    let mut out = GENERATED_HEADER.to_owned();
    out.push_str(&format!(
        "// serde の JSON が生成した型に合うかを型検査で確かめるための値 (ugoe_editor_core::codegen)。\n\
         import type {{ Command, Model }} from \"{types_module}\";\n\n"
    ));
    out.push_str("export const models: Model[] = ");
    out.push_str(&serde_json::to_string_pretty(&models)?);
    out.push_str(";\n\nexport const commands: Command[] = ");
    out.push_str(&serde_json::to_string_pretty(&sample_commands())?);
    out.push_str(";\n");
    Ok(out)
}

/// 型の検査に使うコマンド。すべてのバリアントと、省略できるフィールドの有無の両方を含める。
fn sample_commands() -> Vec<Command> {
    let quad = fixtures::single_quad();
    let parameter = quad.parameters[0].clone();
    let part = quad.parts[0].clone();
    let mesh = fixtures::keyform_inference().meshes.remove(0);
    // `rotation_chain` はワープ (`Cloth`) と回転デフォーマを持つ。
    let deformers = fixtures::rotation_chain().deformers;
    let deformer = deformers[0].clone();
    let warp_keyforms = deformers
        .iter()
        .find_map(|d| match &d.kind {
            DeformerKind::Warp(w) => Some(w.keyforms.clone()),
            _ => None,
        })
        .unwrap_or_else(empty);
    let rotation_keyforms = deformers
        .iter()
        .find_map(|d| match &d.kind {
            DeformerKind::Rotation(r) => Some(r.keyforms.clone()),
            _ => None,
        })
        .unwrap_or_else(empty);
    let cell: KeyformCell = mesh
        .keyforms
        .bindings
        .iter()
        .map(|b| (b.parameter.clone(), b.keys[0]))
        .collect();
    let mesh_id = mesh.id.clone();
    let mesh_cell = cell.clone();
    let (pid, part_id, mid, did) = (
        parameter.id.clone(),
        part.id.clone(),
        mesh.id.clone(),
        deformer.id.clone(),
    );
    vec![
        Command::AddParameter {
            parameter: parameter.clone(),
        },
        Command::RemoveParameter { id: pid.clone() },
        Command::SetParameterRange {
            id: pid.clone(),
            min: -1.0,
            default: 0.0,
            max: 1.0,
        },
        Command::Batch {
            label: "batch".to_owned(),
            commands: vec![Command::RenameParameter {
                id: pid.clone(),
                name: "Angle".to_owned(),
            }],
        },
        Command::SetParameterGroup {
            id: pid.clone(),
            group: None,
        },
        Command::SetParameterMirror {
            id: pid.clone(),
            mirror: Some(pid),
        },
        Command::AddPart { part: part.clone() },
        Command::RemovePart {
            id: part_id.clone(),
        },
        Command::RenamePart {
            id: part_id.clone(),
            name: "Body".to_owned(),
        },
        Command::SetPartParent {
            id: part_id.clone(),
            parent: None,
        },
        Command::SetPartKeyforms {
            id: part_id.clone(),
            keyforms: part.keyforms,
        },
        Command::Batch {
            label: "deformers".to_owned(),
            commands: deformers
                .iter()
                .map(|d| Command::AddDeformer {
                    deformer: d.clone(),
                })
                .collect(),
        },
        Command::AddDeformer { deformer },
        Command::RemoveDeformer { id: did.clone() },
        Command::RenameDeformer {
            id: did.clone(),
            name: "Warp".to_owned(),
        },
        Command::SetDeformerParent {
            id: did.clone(),
            parent: Some(did.clone()),
        },
        Command::SetDeformerPart {
            id: did.clone(),
            part: part_id.clone(),
        },
        Command::SetWarpKeyforms {
            id: did.clone(),
            keyforms: warp_keyforms,
        },
        Command::SetRotationKeyforms {
            id: did.clone(),
            keyforms: rotation_keyforms,
        },
        Command::AddMesh { mesh: mesh.clone() },
        Command::RemoveMesh { id: mid.clone() },
        Command::RenameMesh {
            id: mid.clone(),
            name: "Face".to_owned(),
        },
        Command::SetMeshProperties {
            id: mid.clone(),
            part: None,
            texture: None,
            blend: None,
            culling: None,
            masks: None,
            mask_mode: None,
            mask_only: None,
        },
        Command::SetMeshProperties {
            id: mid.clone(),
            part: Some(part_id),
            texture: Some(0),
            blend: Some(BlendMode::Multiply),
            culling: Some(true),
            masks: Some(vec![MaskRef::inverted(mid.clone())]),
            mask_mode: Some(MaskMode::Intersection),
            mask_only: Some(false),
        },
        Command::SetMeshParent {
            id: mid.clone(),
            parent: Some(did),
        },
        Command::SetMeshKeyform {
            id: mid.clone(),
            cell,
            keyform: mesh.keyforms.keyforms[0].clone(),
        },
        Command::SetMeshKeyforms {
            id: mid.clone(),
            keyforms: mesh.keyforms,
        },
        Command::ReorderMesh {
            id: mid.clone(),
            before: None,
        },
        Command::AddTexture {
            name: "face.png".to_owned(),
            png: vec![137, 80, 78, 71],
        },
        Command::SetTextureImage {
            name: "face.png".to_owned(),
            png: vec![137, 80, 78, 71],
        },
        Command::RemoveTexture {
            name: "face.png".to_owned(),
        },
        Command::SetCanvas {
            width: 1024.0,
            height: 2048.0,
            origin: Vec2::new(512.0, 1024.0),
        },
        Command::RegenerateMesh {
            id: mesh_id.clone(),
            options: AutoMeshOptions::default(),
        },
        Command::InferKeyforms {
            target: KeyformTarget::Mesh(mesh_id.clone()),
            method: InferMethod::DepthRotation,
        },
        Command::MirrorKeyforms {
            source: KeyformTarget::Deformer(deformers[0].id.clone()),
            target: KeyformTarget::Glue("Glue".to_owned()),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        },
        Command::MirrorKeyforms {
            source: KeyformTarget::Part(quad.parts[0].id.clone()),
            target: KeyformTarget::Part(quad.parts[0].id.clone()),
            axis: Some(512.0),
            tolerance: Some(0.5),
            overwrite_authored: true,
        },
        Command::MarkKeyformAuthored {
            target: KeyformTarget::Mesh(mesh_id.clone()),
            cells: vec![mesh_cell.clone()],
        },
        Command::MarkKeyformDerived {
            target: KeyformTarget::Mesh(mesh_id.clone()),
            cells: vec![mesh_cell, KeyformCell::new()],
        },
        Command::MoveMeshVertices {
            id: mesh_id.clone(),
            moves: vec![VertexMove {
                index: 2,
                delta: Vec2::new(4.0, -8.0),
            }],
        },
        Command::AddMeshVertex {
            id: mesh_id.clone(),
            position: Vec2::new(512.0, 300.0),
        },
        Command::RemoveMeshVertices {
            id: mesh_id.clone(),
            indices: vec![0, 3],
        },
        Command::RetriangulateMesh { id: mesh_id },
    ]
}

fn empty<T>() -> KeyformGrid<T> {
    KeyformGrid::new(Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_command_schema_is_up_to_date() {
        assert!(
            command_schema_json().unwrap() == COMMAND_SCHEMA_JSON,
            "crates/ugoe-editor-core/schema/command.schema.json is stale; run `just gen`"
        );
    }

    #[test]
    fn command_schema_lists_every_op() {
        let schema: serde_json::Value = serde_json::from_str(COMMAND_SCHEMA_JSON).unwrap();
        let text = schema.to_string();
        for command in sample_commands() {
            let value = serde_json::to_value(&command).unwrap();
            let op = value["op"].as_str().unwrap();
            assert!(text.contains(&format!("\"{op}\"")), "missing op {op}");
        }
        // 未知の値は書き出せないので、スキーマにも現れない。
        assert!(!text.contains("\"unknown\""));
    }

    #[test]
    fn typescript_declares_editor_types() {
        let ts = TsBindings::new().with_editor_types().render();
        for name in [
            "Command",
            "Model",
            "Mesh",
            "KeyformGrid<T>",
            "Vec2",
            "Rgb",
            "Motion",
        ] {
            assert!(
                ts.contains(&format!("export type {name} = ")),
                "missing {name}"
            );
        }
        // ID は文字列、u64 は number、未知の値は含めない。
        assert!(ts.contains("export type Vec2 = [number, number];"));
        assert!(!ts.contains("bigint"));
        assert!(!ts.contains("unknown"));
    }

    #[test]
    fn samples_cover_every_command_variant() {
        use std::collections::BTreeSet;

        let schema = serde_json::to_value(command_schema()).unwrap();
        let expected: BTreeSet<&str> = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["properties"]["op"]["const"].as_str().unwrap())
            .collect();
        // `batch` の中のコマンドも数える。
        let json = serde_json::to_string(&sample_commands()).unwrap();
        let missing: Vec<&str> = expected
            .into_iter()
            .filter(|op| !json.contains(&format!("\"op\":\"{op}\"")))
            .collect();
        assert!(
            missing.is_empty(),
            "add samples to `sample_commands` for {missing:?}"
        );
    }
}
