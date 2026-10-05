//! ライティングの画面 (`docs/editor.md#ライティングの画面`) の IPC コマンド。
//!
//! 法線マップの生成は `ugoe_ai::normal` (部位のまとまりごとにふくらませる決定的な方法。AI は使わない) を、画像の割り当ては
//! `ugoe_editor_core::lighting` を呼び、ここは入力 (意味ラベル) を集めて適用するだけを行う。
//! プレビューの光源の読み書きは `project.rs` (`get_preview_lighting` / `set_preview_lighting`)。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_ai::Proposal;
use ugoe_ai::classify::{Label, LayerLabel, label_meshes_by_name};
use ugoe_ai::normal::{
    BulgeNormalMapOptions, NormalMapGroup, SurfaceGroup, normal_map_group, normal_map_groups,
    propose_bulge_normal_maps,
};
use ugoe_ai::rig::mesh_parts;
use ugoe_editor_core::Message;
use ugoe_editor_core::lighting::{BulgeSettings, assign_normal_map};
use ugoe_editor_core::{ApplyReport, Command, Document};
use ugoe_model::{MeshId, Model};

use crate::AppState;
use crate::commands::{blocking, lock};
use crate::error::{EditorError, IpcResult};
use crate::project::lock_both;
use crate::proposals::Base;
use ugoe_fs::read_regular_file;

/// 法線マップとして読む PNG の大きさの上限 (スタジオの背景の画像と同じ)。
const MAX_NORMAL_MAP_BYTES: u64 = 64 * 1024 * 1024;

/// テクスチャの法線マップを、どのまとまりで作るか (`get_normal_map_group` の戻り値)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct NormalMapGroupView {
    /// まとまり (`face`・`body`・`hair_front` など。`ugoe_ai::normal::SurfaceGroup`)。ラベルがなければ `null` (単独)。
    pub group: Option<String>,
    /// 法線マップを作るテクスチャ (このテクスチャを含む。`get_normal_map_groups` の単独のまとまり
    /// (`alone` と `null`) では、単独でふくらませるすべてのテクスチャ)。
    pub textures: Vec<String>,
    /// 形にだけ加えるテクスチャ (体のまとまりの顔など)。
    pub shape_only: Vec<String>,
    /// `textures` のうち、形に加えず向きだけを受けるテクスチャ (顔のまとまりの眼鏡)。
    pub oriented: Vec<String>,
    /// `textures` のうち、すでに法線マップを持つテクスチャ。
    pub with_normal_map: Vec<String>,
    /// まとまりの既定の設定 (長さ `edge`・`smoothing` はまとまりの格子の画素)。
    pub defaults: BulgeSettings,
    /// まとまりの格子の 1 画素がキャンバスの何ピクセルか。画面は縁の幅とならしをキャンバスのピクセルで見せ、
    /// 送るときにこれで割って格子の画素に直す (T0522)。単独でふくらませるもの (`alone` と `null`) は、
    /// テクスチャの画素が単位なので `null`。
    pub canvas_per_pixel: Option<f32>,
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

/// ドキュメントと、最後に分類した PSD のレイヤーのラベル (分類の直しを重ねたもの)。
fn document_and_labels(
    state: &AppState,
) -> IpcResult<(std::sync::MutexGuard<'_, Document>, Option<Vec<LayerLabel>>)> {
    let (document, session) = lock_both(state)?;
    let layers = session.proposals.layer_labels(&document);
    Ok((document, layers))
}

impl NormalMapGroupView {
    fn new(model: &Model, group: NormalMapGroup, from_classification: bool) -> Self {
        let with_normal_map = group
            .textures
            .iter()
            .filter(|name| {
                model
                    .textures
                    .iter()
                    .any(|t| t.name == **name && t.normal_map.is_some())
            })
            .cloned()
            .collect();
        Self {
            with_normal_map,
            group: group.group.map(|g| g.as_str().to_owned()),
            textures: group.textures,
            shape_only: group.shape_only,
            oriented: group.oriented,
            defaults: BulgeSettings {
                strength: group.options.strength,
                edge: group.options.radius,
                smoothing: group.options.smoothing,
            },
            canvas_per_pixel: group.canvas_per_pixel,
            from_classification,
        }
    }
}

/// テクスチャ `texture` の法線マップのまとまりと、その既定の設定。
pub(crate) fn normal_map_group_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
    texture: &str,
) -> IpcResult<NormalMapGroupView> {
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let group = normal_map_group(
        model,
        document.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
        texture,
    )
    .map_err(EditorError::Ai)?;
    Ok(NormalMapGroupView::new(model, group, layers.is_some()))
}

/// 法線マップを作るすべてのテクスチャの、まとまりの一覧 (AI タブ。`ugoe_ai::normal::normal_map_groups`)。
pub(crate) fn normal_map_groups_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
) -> IpcResult<Vec<NormalMapGroupView>> {
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let groups = normal_map_groups(
        model,
        document.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
    )
    .map_err(EditorError::Ai)?;
    Ok(groups
        .into_iter()
        .map(|g| NormalMapGroupView::new(model, g, layers.is_some()))
        .collect())
}

/// AI タブで選んだ、法線マップを提案する 1 つのまとまりと設定 (`propose_normal_maps` の引数)。
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
pub struct NormalMapGroupChoice {
    /// まとまり (`NormalMapGroupView::group`。`null` はラベルのないテクスチャ)。
    pub group: Option<String>,
    /// まとまり全体の設定 (単独のまとまりでは 1 枚ごとの設定)。
    pub settings: BulgeSettings,
}

/// 提案のコマンドを、画面でまとめて出す区切り (法線マップの提案のまとまりごと)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CommandSection {
    /// 区切りの名前 (まとまり `face` など。ラベルのないものは `unlabeled`、ライティングを有効にするのは `lighting`)。
    pub label: String,
    /// 区切りに入るコマンドの番号 (提案の中の 0 始まり)。コマンドの対象 (テクスチャの名前) は
    /// `ProposalCommand::subject`。
    pub indices: Vec<usize>,
}

/// ラベルのないテクスチャのまとまりの、区切りの名前。
const UNLABELED: &str = "unlabeled";

/// 選んだまとまりのテクスチャに、まとまりごとの設定で法線マップを作る提案 (AI は使わない) と、コマンドの区切り。
/// `replace_existing` なら法線マップを持つテクスチャも作り直す。モジュールのプロジェクトでは断る。
pub(crate) fn normal_map_proposal_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
    choices: &[NormalMapGroupChoice],
    replace_existing: bool,
) -> IpcResult<(Proposal, Vec<CommandSection>)> {
    if document.module().is_some() {
        // 本体のテクスチャとライティングの設定は変えられない (ADR 0024)。
        return Err(EditorError::NormalMapsModelOnly.into());
    }
    if choices.is_empty() {
        return Err(EditorError::NoGroupChosen.into());
    }
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let groups = normal_map_groups(
        model,
        document.textures(),
        &labels,
        &BulgeNormalMapOptions::default(),
    )
    .map_err(EditorError::Ai)?;
    let mut options = BulgeNormalMapOptions {
        replace_existing,
        ..Default::default()
    };
    // テクスチャの名前 → 区切りの名前。
    let mut section_of = BTreeMap::new();
    for choice in choices {
        let group = match &choice.group {
            None => None,
            Some(name) => Some(
                serde_json::from_value::<SurfaceGroup>(serde_json::Value::String(name.clone()))
                    .map_err(|_| EditorError::UnknownGroup(name.clone()))?,
            ),
        };
        let row = groups
            .iter()
            .find(|g| g.group == group)
            .ok_or_else(|| EditorError::EmptyGroup(section_name(group)))?;
        match group {
            Some(g) => {
                options.per_group.insert(g, choice.settings.into());
            }
            None => options.default = choice.settings.into(),
        }
        for t in &row.textures {
            section_of.insert(t.clone(), section_name(group));
            options.textures.push(t.clone());
        }
    }
    let mut proposal = propose_bulge_normal_maps(model, document.textures(), &labels, &options)
        .map_err(EditorError::Ai)?;
    proposal.title.push_str(" — no AI");
    let mut sections: Vec<CommandSection> = Vec::new();
    for (i, command) in proposal.commands.iter().enumerate() {
        let label = match command {
            Command::SetNormalMap { texture, .. } => section_of
                .get(texture)
                .cloned()
                .unwrap_or_else(|| UNLABELED.to_owned()),
            _ => "lighting".to_owned(),
        };
        match sections.iter_mut().find(|s| s.label == label) {
            Some(s) => s.indices.push(i),
            None => sections.push(CommandSection {
                label,
                indices: vec![i],
            }),
        }
    }
    // 選んだまとまりの順に並べ、ライティングを最後に置く。
    let order: Vec<String> = choices
        .iter()
        .map(|c| c.group.clone().unwrap_or_else(|| UNLABELED.to_owned()))
        .collect();
    sections.sort_by_key(|s| {
        order
            .iter()
            .position(|o| *o == s.label)
            .unwrap_or(usize::MAX)
    });
    Ok((proposal, sections))
}

fn section_name(group: Option<SurfaceGroup>) -> String {
    group.map_or(UNLABELED, SurfaceGroup::as_str).to_owned()
}

/// テクスチャ `texture` の法線マップを、まとまりの形から作るコマンド (1 つの Batch)。
/// `settings` はまとまり全体の設定 (`None` ならまとまりの既定)。`whole_group` なら、まとまりのすべてのテクスチャに作る。
pub(crate) fn generate_normal_map_in(
    document: &Document,
    layers: Option<&[LayerLabel]>,
    texture: &str,
    settings: Option<BulgeSettings>,
    whole_group: bool,
) -> IpcResult<Command> {
    let model = document.model();
    let labels = mesh_labels(model, layers);
    let mut options = BulgeNormalMapOptions {
        textures: vec![texture.to_owned()],
        replace_existing: true,
        ..Default::default()
    };
    let group = normal_map_group(model, document.textures(), &labels, &options, texture)
        .map_err(EditorError::Ai)?;
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
        .map_err(EditorError::Ai)?;
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
) -> IpcResult<NormalMapGroupView> {
    let (document, layers) = document_and_labels(&state)?;
    normal_map_group_in(&document, layers.as_deref(), &texture)
}

/// 法線マップを作るすべてのテクスチャの、まとまりの一覧と既定の設定 (AI タブでまとまりごとに設定して提案する)。
/// 単独でふくらませるもの (`alone` と、ラベルのない `null`) はそれぞれ 1 行にまとめて最後に置く。
#[tauri::command]
pub fn get_normal_map_groups(state: State<'_, AppState>) -> IpcResult<Vec<NormalMapGroupView>> {
    let (document, layers) = document_and_labels(&state)?;
    normal_map_groups_in(&document, layers.as_deref())
}

/// テクスチャ `texture` の法線マップを、部位のまとまり (意味ラベルから決める) の形のふくらみから作って付ける
/// (AI は使わない)。`settings` はまとまり全体の設定 (`null` ならまとまりの既定)。`whole_group` なら、
/// まとまりのすべてのテクスチャに作り直す。モデルのライティングが無効なら有効にする。1 回の Undo で戻る。
#[tauri::command]
pub async fn generate_texture_normal_map(
    app: AppHandle,
    texture: String,
    settings: Option<BulgeSettings>,
    whole_group: bool,
) -> IpcResult<ApplyReport> {
    blocking(app, move |state| {
        generate_texture_normal_map_in(state, &texture, settings, whole_group)
    })
    .await
}

/// [`generate_texture_normal_map`] の本体。作る間はロックを持たず (まとまりのすべてのテクスチャでは時間がかかる)、
/// 複製から作って、適用だけをロックの中で行う (提案の `propose_normal_maps_in` と同じ形)。作る間にモデルが
/// 変わっていたら適用せずに [`EditorError::ModelChanged`]。
pub(crate) fn generate_texture_normal_map_in(
    state: &AppState,
    texture: &str,
    settings: Option<BulgeSettings>,
    whole_group: bool,
) -> IpcResult<ApplyReport> {
    let (document, base, layers) = {
        let (document, session) = lock_both(state)?;
        let base = Base::of(&document, &session);
        (
            document.detached(),
            base,
            session.proposals.layer_labels(&document),
        )
    };
    let command =
        generate_normal_map_in(&document, layers.as_deref(), texture, settings, whole_group)?;
    let (mut document, session) = lock_both(state)?;
    base.check_edit(&document, &session)?;
    document.apply_with_report(command).map_err(Message::from)
}

/// PNG ファイル `path` (開くのダイアログで選んだもの) をテクスチャ `texture` の法線マップとして付ける。
/// `flip_green` なら G を反転する (DirectX 形式の画像)。1 回の Undo で戻る。
#[tauri::command]
pub async fn assign_texture_normal_map(
    app: AppHandle,
    texture: String,
    path: String,
    flip_green: bool,
) -> IpcResult<ApplyReport> {
    blocking(app, move |state| {
        let path = crate::access::readable_path(state, &path)?;
        // 通常のファイルだけを読む (装置・FIFO・ディレクトリは読まない。`ugoe_fs::read_regular_file`)。
        let png = read_regular_file(&path, MAX_NORMAL_MAP_BYTES)
            .map_err(|e| EditorError::io(&path, e))?;
        let mut document = lock(state)?;
        let command = assign_normal_map(document.model(), &texture, png, flip_green)?;
        document.apply_with_report(command).map_err(Message::from)
    })
    .await
}

/// テクスチャか法線マップの画像 `name` の PNG (画面で確かめるため)。生のバイト列で送る。
#[tauri::command]
pub fn get_texture_png(
    state: State<'_, AppState>,
    name: String,
) -> IpcResult<tauri::ipc::Response> {
    let document = lock(&state)?;
    let png = document
        .texture(&name)
        .ok_or_else(|| EditorError::ImageNotFound(name.clone()))?;
    Ok(tauri::ipc::Response::new(png.to_vec()))
}

/// テクスチャか法線マップの画像 `names` の版 (中身のハッシュ。無い画像は `None`)。画面は編集のたびに PNG を
/// 読み直さず、版が変わったときだけ `get_texture_png` で読む (同じ名前の画像の差し替えも映す。T1614)。
#[tauri::command]
pub fn get_texture_versions(
    state: State<'_, AppState>,
    names: Vec<String>,
) -> IpcResult<Vec<Option<String>>> {
    let document = lock(&state)?;
    Ok(texture_versions(&document, &names))
}

fn texture_versions(document: &Document, names: &[String]) -> Vec<Option<String>> {
    names
        .iter()
        .map(|name| document.texture(name).map(texture_version))
        .collect()
}

/// PNG のバイト列の版。同じプロセスの中で比べるだけなので `std` の `DefaultHasher` を使う。
fn texture_version(png: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    png.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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
        assert!(view.canvas_per_pixel.is_some_and(|c| c > 0.0));
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
    fn texture_versions_change_with_the_image() {
        let mut doc = bustup();
        let eye = texture_of(&doc, "eyewhite-l");
        let face = texture_of(&doc, "face");
        let flat = BulgeSettings {
            strength: 0.0,
            ..BulgeSettings::default()
        };
        let command = generate_normal_map_in(&doc, None, &eye, Some(flat), false).unwrap();
        doc.apply(command).unwrap();
        let map = doc.model().textures.iter().find(|t| t.name == eye).unwrap();
        let names = vec![
            eye.clone(),
            map.normal_map.clone().unwrap(),
            face,
            "missing".to_owned(),
        ];
        let before = texture_versions(&doc, &names);
        assert!(before[..3].iter().all(Option::is_some));
        assert_eq!(before[3], None);
        // 同じ名前の法線マップを作り直すと、その版だけが変わる。
        let command = generate_normal_map_in(&doc, None, &eye, None, false).unwrap();
        doc.apply(command).unwrap();
        let after = texture_versions(&doc, &names);
        assert_eq!(
            doc.model()
                .textures
                .iter()
                .find(|t| t.name == eye)
                .unwrap()
                .normal_map,
            Some(names[1].clone())
        );
        assert_ne!(after[1], before[1]);
        assert_eq!(after[0], before[0]);
        assert_eq!(after[2], before[2]);
        assert!(doc.undo());
        assert_eq!(texture_versions(&doc, &names), before);
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
            variant: None,
            name_disagreement: None,
        }];
        let view = normal_map_group_in(&doc, Some(&layers), &neck).unwrap();
        assert_eq!(view.group.as_deref(), Some("hair_front"));
        assert!(view.from_classification);
        assert!(view.textures.contains(&texture_of(&doc, "front hair")));
    }

    /// 分類で眼鏡にしたテクスチャは、顔のまとまりの向きだけを受ける (T0520)。
    #[test]
    fn eyewear_takes_the_face_direction() {
        let mut doc = bustup();
        let eye = texture_of(&doc, "eyewhite-l");
        let layers = vec![LayerLabel {
            layer_path: vec!["eyewhite-l".into()],
            label: Label::Eyewear,
            side: ugoe_ai::classify::Side::Left,
            confidence: 1.0,
            variant: None,
            name_disagreement: None,
        }];
        let view = normal_map_group_in(&doc, Some(&layers), &eye).unwrap();
        assert_eq!(view.group.as_deref(), Some("face"));
        assert_eq!(view.oriented, vec![eye.clone()]);
        assert!(view.with_normal_map.is_empty());
        let command = generate_normal_map_in(&doc, Some(&layers), &eye, None, false).unwrap();
        doc.apply(command).unwrap();
        assert!(!is_flat(&doc, &eye));
        let groups = normal_map_groups_in(&doc, Some(&layers)).unwrap();
        let face = groups
            .iter()
            .find(|g| g.group.as_deref() == Some("face"))
            .unwrap();
        assert_eq!(face.oriented, vec![eye.clone()]);
        assert_eq!(face.with_normal_map, vec![eye]);
    }
}
