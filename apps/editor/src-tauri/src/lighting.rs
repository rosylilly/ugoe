//! ライティングの画面 (`docs/editor.md#ライティングの画面`) の IPC コマンド。
//!
//! 法線マップの生成は `ugoe_ai::normal` (部位のまとまりごとにふくらませる決定的な方法。AI は使わない) を、画像の割り当ては
//! `ugoe_editor_core::lighting` を呼び、ここは入力 (意味ラベル) を集めて適用するだけを行う。
//! プレビューの光源の読み書きは `project.rs` (`get_preview_lighting` / `set_preview_lighting`)。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::collections::BTreeMap;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_ai::classify::{Label, LayerLabel, label_meshes_by_name};
use ugoe_ai::normal::{BulgeNormalMapOptions, normal_map_group, propose_bulge_normal_maps};
use ugoe_ai::rig::mesh_parts;
use ugoe_editor_core::lighting::{BulgeSettings, assign_normal_map};
use ugoe_editor_core::{ApplyReport, Command, Document};
use ugoe_model::{MeshId, Model};

use crate::AppState;
use crate::ai::lock_project;
use crate::commands::lock;
use crate::project::absolute_path;

/// テクスチャの法線マップを、どのまとまりで作るか (`get_normal_map_group` の戻り値)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct NormalMapGroupView {
    /// まとまり (`face`・`body`・`hair_front` など。`ugoe_ai::normal::SurfaceGroup`)。ラベルがなければ `null` (単独)。
    pub group: Option<String>,
    /// いっしょにふくらませるテクスチャ (このテクスチャを含む)。
    pub textures: Vec<String>,
    /// 形にだけ加えるテクスチャ (体のまとまりの顔など)。
    pub shape_only: Vec<String>,
    /// まとまりの既定の設定 (長さはまとまりの格子の画素)。
    pub defaults: BulgeSettings,
    /// ラベルを PSD の分類 (AI の提案のパネル) から取ったか。偽ならメッシュの名前の規則だけ。
    pub from_classification: bool,
}

/// メッシュの意味ラベル: メッシュの名前の規則に、PSD の分類 (あれば) を重ねる。
fn mesh_labels(model: &Model, layers: Option<&[LayerLabel]>) -> BTreeMap<MeshId, Label> {
    let mut labels = label_meshes_by_name(model);
    if let Some(layers) = layers {
        labels.extend(
            mesh_parts(model, layers)
                .into_iter()
                .map(|(id, part)| (id, part.label)),
        );
    }
    labels
}

/// 最後に分類した PSD のレイヤーのラベル (ドキュメントのロックより先に取る)。
fn layer_labels(state: &AppState) -> Result<Option<Vec<LayerLabel>>, String> {
    Ok(lock_project(state)?.proposals.layer_labels())
}

/// テクスチャ `texture` の法線マップのまとまりと、その既定の設定。
pub(crate) fn normal_map_group_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
    texture: &str,
) -> Result<NormalMapGroupView, String> {
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let group = normal_map_group(
        model,
        document.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
        texture,
    )
    .map_err(|e| e.to_string())?;
    Ok(NormalMapGroupView {
        group: group.group.map(|g| g.as_str().to_owned()),
        textures: group.textures,
        shape_only: group.shape_only,
        defaults: BulgeSettings {
            strength: group.options.strength,
            edge: group.options.radius,
            smoothing: group.options.smoothing,
        },
        from_classification: layers.is_some(),
    })
}

/// テクスチャ `texture` の法線マップを、まとまりの形から作るコマンド (1 つの Batch)。
/// `settings` はまとまり全体の設定 (`None` ならまとまりの既定)。`whole_group` なら、まとまりのすべてのテクスチャに作る。
pub(crate) fn generate_normal_map_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
    texture: &str,
    settings: Option<BulgeSettings>,
    whole_group: bool,
) -> Result<Command, String> {
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let mut options = BulgeNormalMapOptions {
        textures: vec![texture.to_owned()],
        replace_existing: true,
        ..Default::default()
    };
    let group = normal_map_group(model, document.textures(), &labels, &options, texture)
        .map_err(|e| e.to_string())?;
    if whole_group {
        options.textures = group.textures;
    }
    if let Some(settings) = settings {
        match group.group {
            Some(g) => {
                options.per_group.insert(g, settings.into());
            }
            None => options.default = settings.into(),
        }
    }
    let proposal = propose_bulge_normal_maps(model, document.textures(), &labels, &options)
        .map_err(|e| e.to_string())?;
    Ok(Command::Batch {
        label: "Generate normal map".into(),
        commands: proposal.commands,
    })
}

/// テクスチャ `texture` の法線マップをどのまとまりで作るか (画面で既定の設定といっしょに示す)。
#[tauri::command]
pub fn get_normal_map_group(
    state: State<'_, AppState>,
    texture: String,
) -> Result<NormalMapGroupView, String> {
    let layers = layer_labels(&state)?;
    let document = lock(&state)?;
    normal_map_group_in(&document, layers.as_deref(), &texture)
}

/// テクスチャ `texture` の法線マップを、部位のまとまり (意味ラベルから決める) の形のふくらみから作って付ける
/// (AI は使わない)。`settings` はまとまり全体の設定 (`null` ならまとまりの既定)。`whole_group` なら、
/// まとまりのすべてのテクスチャに作り直す。モデルのライティングが無効なら有効にする。1 回の Undo で戻る。
#[tauri::command]
pub fn generate_texture_normal_map(
    state: State<'_, AppState>,
    texture: String,
    settings: Option<BulgeSettings>,
    whole_group: bool,
) -> Result<ApplyReport, String> {
    let layers = layer_labels(&state)?;
    let mut document = lock(&state)?;
    let command = generate_normal_map_in(
        &document,
        layers.as_deref(),
        &texture,
        settings,
        whole_group,
    )?;
    document
        .apply_with_report(command)
        .map_err(|e| e.to_string())
}

/// PNG ファイル `path` (開くのダイアログで選んだもの) をテクスチャ `texture` の法線マップとして付ける。
/// `flip_green` なら G を反転する (DirectX 形式の画像)。1 回の Undo で戻る。
#[tauri::command]
pub fn assign_texture_normal_map(
    state: State<'_, AppState>,
    texture: String,
    path: String,
    flip_green: bool,
) -> Result<ApplyReport, String> {
    let path = absolute_path(&path)?;
    let png = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut document = lock(&state)?;
    let command = assign_normal_map(document.model(), &texture, png, flip_green)
        .map_err(|e| e.to_string())?;
    document
        .apply_with_report(command)
        .map_err(|e| e.to_string())
}

/// テクスチャか法線マップの画像 `name` の PNG (画面で確かめるため)。生のバイト列で送る。
#[tauri::command]
pub fn get_texture_png(
    state: State<'_, AppState>,
    name: String,
) -> Result<tauri::ipc::Response, String> {
    let document = lock(&state)?;
    let png = document
        .texture(&name)
        .ok_or_else(|| format!("image `{name}` not found"))?;
    Ok(tauri::ipc::Response::new(png.to_vec()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::PathBuf;

    use ugoe_editor_core::texture::decode_rgba_png;
    use ugoe_editor_core::{Project, PsdImportOptions, import_psd};

    use super::*;

    /// 胸から上の PSD を取り込んだドキュメント (メッシュの名前はレイヤーの名前)。
    fn bustup() -> Document {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd");
        let bytes = std::fs::read(path).unwrap();
        let name = "bustup";
        let mut doc = Document::new(Project::empty(name).model);
        let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(name)).unwrap();
        doc.apply(import.command).unwrap();
        doc
    }

    fn texture_of(doc: &Document, mesh: &str) -> String {
        let m = doc.model().meshes.iter().find(|m| m.name == mesh).unwrap();
        doc.model().textures[m.texture as usize].name.clone()
    }

    /// 法線マップの画像が、すべて正面向きか。
    fn is_flat(doc: &Document, texture: &str) -> bool {
        let t = doc
            .model()
            .textures
            .iter()
            .find(|t| t.name == texture)
            .unwrap();
        let png = doc.texture(t.normal_map.as_deref().unwrap()).unwrap();
        let (_, _, rgba) = decode_rgba_png(png).unwrap();
        rgba.as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [128, 128, 255])
    }

    #[test]
    fn a_texture_is_generated_with_its_group() {
        let mut doc = bustup();
        let eye = texture_of(&doc, "eyewhite-l");
        let view = normal_map_group_in(&doc, None, &eye).unwrap();
        assert_eq!(view.group.as_deref(), Some("face"));
        assert!(view.textures.contains(&texture_of(&doc, "face")));
        assert!(view.defaults.edge.is_some());
        assert!(!view.from_classification);

        // 既定の設定で作る: この 1 枚の法線マップと、ライティングを有効にする。1 回の Undo で戻る。
        let before = doc.model().clone();
        let command = generate_normal_map_in(&doc, None, &eye, None, false).unwrap();
        doc.apply(command).unwrap();
        assert!(doc.model().lighting);
        let with_maps: Vec<_> = doc
            .model()
            .textures
            .iter()
            .filter(|t| t.normal_map.is_some())
            .map(|t| t.name.clone())
            .collect();
        assert_eq!(with_maps, vec![eye.clone()]);
        // 目は顔の上で平らに浮かない (顔のふくらみの向きを受ける)。
        assert!(!is_flat(&doc, &eye));
        assert!(doc.undo());
        assert_eq!(doc.model(), &before);

        // 設定はまとまり全体に掛かる (強さ 0 なら平ら)。
        let flat = BulgeSettings {
            strength: 0.0,
            ..BulgeSettings::default()
        };
        let command = generate_normal_map_in(&doc, None, &eye, Some(flat), false).unwrap();
        doc.apply(command).unwrap();
        assert!(is_flat(&doc, &eye));

        // まとまり全体に作る。
        let command = generate_normal_map_in(&doc, None, &eye, None, true).unwrap();
        doc.apply(command).unwrap();
        let count = doc
            .model()
            .textures
            .iter()
            .filter(|t| t.normal_map.is_some())
            .count();
        assert_eq!(count, view.textures.len());
        assert!(!is_flat(&doc, &eye));
    }

    #[test]
    fn classification_overrides_the_names() {
        let doc = bustup();
        let neck = texture_of(&doc, "neck");
        assert_eq!(
            normal_map_group_in(&doc, None, &neck)
                .unwrap()
                .group
                .as_deref(),
            Some("body")
        );
        // 分類で首を髪 (前髪) にすると、前髪のまとまりになる。
        let layers = vec![LayerLabel {
            layer_path: vec!["neck".into()],
            label: Label::HairFront,
            side: ugoe_ai::classify::Side::Center,
            confidence: 1.0,
        }];
        let view = normal_map_group_in(&doc, Some(&layers), &neck).unwrap();
        assert_eq!(view.group.as_deref(), Some("hair_front"));
        assert!(view.from_classification);
        assert!(view.textures.contains(&texture_of(&doc, "front hair")));
    }
}
