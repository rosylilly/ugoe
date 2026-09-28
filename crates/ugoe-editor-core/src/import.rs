//! PSD の取り込み: `ugoe_psd::LayerTree` から、パーツ・テクスチャ・メッシュを作る
//! 1 つの `Command::Batch` を組み立てる。規則は `docs/editor.md#psd-の取り込み`。
//!
//! モデルは書き換えず、コマンドを返すだけ (Undo の 1 単位として `Document::apply` する)。
//! エディタ (Tauri)・CLI・AI が同じ関数を使う。

use std::collections::HashSet;
use std::fmt;

use serde::Serialize;
use ts_rs::TS;
use ugoe_core::{Rgb, Vec2};
use ugoe_mesher::MeshError;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::part::PartKeyform;
use ugoe_model::{BlendMode, KeyformGrid, MaskMode, MaskRef, Mesh, MeshId, Model, Part, PartId};
use ugoe_psd::{Layer, LayerNode, LayerTree, PsdError};

use crate::Command;
use crate::automesh::AutoMeshOptions;
use crate::ids::unique_id;
use crate::texture::{MAX_TEXTURE_NAME_LEN, encode_rgba_png};

/// 取り込んだメッシュの描画順を並べる範囲 (`docs/spec/model.md` の慣例 0〜1000)。
pub const DRAW_ORDER_RANGE: f32 = 1000.0;

/// 取り込みの設定。
#[derive(Debug, Clone, PartialEq)]
pub struct PsdImportOptions {
    /// 取り込んだもの全体をまとめるパーツの表示名 (通常は PSD のファイル名)。
    pub name: String,
    /// 自動メッシュの設定。
    pub mesh: AutoMeshOptions,
}

impl PsdImportOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mesh: AutoMeshOptions::default(),
        }
    }
}

/// 取り込みの結果。
#[derive(Debug, Clone)]
pub struct PsdImport {
    /// 取り込み全体を表す `Command::Batch` (Undo の 1 単位)。
    pub command: Command,
    /// 取り込んだもの全体をまとめる最上位のパーツ。
    pub root: PartId,
    /// PSD の読み込み (`ugoe-psd`) と対応付けの警告。見た目が PSD と変わりうるもの。
    pub warnings: Vec<PsdImportWarning>,
}

/// 取り込みの警告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PsdImportWarning {
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    /// ドキュメント全体についての警告では空。
    pub path: Vec<String>,
    pub message: String,
}

impl fmt::Display for PsdImportWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path.join(" / "), self.message)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PsdImportError {
    #[error(transparent)]
    Psd(#[from] PsdError),
    #[error("invalid mesh options: {0}")]
    InvalidMeshOptions(&'static str),
    #[error("layer `{layer}`: automatic meshing failed: {source}")]
    Mesh {
        layer: String,
        #[source]
        source: MeshError,
    },
}

/// PSD のバイト列を読み、`model` に取り込むコマンドを作る。
pub fn import_psd(
    model: &Model,
    bytes: &[u8],
    options: &PsdImportOptions,
) -> Result<PsdImport, PsdImportError> {
    let tree = ugoe_psd::import(bytes)?;
    layer_tree_command(model, &tree, options)
}

/// 読み込み済みの [`LayerTree`] から、`model` に取り込むコマンドを作る。
pub fn layer_tree_command(
    model: &Model,
    tree: &LayerTree,
    options: &PsdImportOptions,
) -> Result<PsdImport, PsdImportError> {
    options
        .mesh
        .check()
        .map_err(PsdImportError::InvalidMeshOptions)?;
    let mut builder = Builder::new(model, options.mesh);
    builder.warnings.extend(tree.warnings.iter().map(|w| {
        let text = w.to_string();
        // `ImportWarning` の表示は `パス: 内容` なので、内容だけを取り出す。
        let prefix = format!("{}: ", w.path.join(" / "));
        let message = text.strip_prefix(&prefix).unwrap_or(&text).to_owned();
        PsdImportWarning {
            path: w.path.clone(),
            message,
        }
    }));

    let mut commands = Vec::new();
    // 何もないモデル (新規プロジェクト) なら、キャンバスを PSD に合わせる。
    let empty = model.parts.is_empty()
        && model.meshes.is_empty()
        && model.deformers.is_empty()
        && model.textures.is_empty();
    let (width, height) = (tree.width as f32, tree.height as f32);
    if empty {
        commands.push(Command::SetCanvas {
            width,
            height,
            origin: Vec2::new(width / 2.0, height / 2.0),
        });
    } else if model.meta.canvas.width != width || model.meta.canvas.height != height {
        builder.warn(
            &[],
            format!(
                "the PSD canvas ({}x{}) differs from the model canvas ({}x{}); \
                 the model canvas is kept and the layers are placed at their PSD pixel \
                 positions, so they may not line up with the canvas \
                 (import into an empty project to use the PSD canvas)",
                tree.width, tree.height, model.meta.canvas.width, model.meta.canvas.height
            ),
        );
    }

    let root = builder.add_part(&options.name, "PSD", None, 1.0);
    builder.walk(&tree.root, &root, &[])?;

    // 描画順: 下から上へ、ドキュメント全体で 0〜1000 に等間隔に並べる。
    let count = builder.meshes.len();
    for (i, (mesh, _)) in builder.meshes.iter_mut().enumerate() {
        let order = DRAW_ORDER_RANGE * (i + 1) as f32 / (count + 1) as f32;
        for keyform in &mut mesh.keyforms.keyforms {
            keyform.draw_order = order;
        }
    }

    commands.extend(
        builder
            .parts
            .into_iter()
            .map(|part| Command::AddPart { part }),
    );
    for (mesh, (name, png)) in builder.meshes {
        commands.push(Command::AddTexture { name, png });
        commands.push(Command::AddMesh { mesh });
    }
    Ok(PsdImport {
        command: Command::Batch {
            label: format!("Import PSD {}", options.name),
            commands,
        },
        root,
        warnings: builder.warnings,
    })
}

/// 兄弟の並びの中で、クリッピングの土台になる (直前の、クリッピングでない) 要素。
enum Base {
    /// 下に要素がない。Photoshop と同じく、クリッピングの指定は無視する。
    Nothing,
    /// 土台を取り込まなかった (空のレイヤーなど)。
    Skipped,
    /// 土台のメッシュ (グループなら中のすべてのメッシュ) と、クリッピングされた側に掛ける
    /// 不透明度 (土台のレイヤーの不透明度。非表示なら 0)。
    Meshes { ids: Vec<MeshId>, opacity: f32 },
}

struct Builder {
    part_ids: HashSet<String>,
    mesh_ids: HashSet<String>,
    /// 小文字にしたテクスチャ名 (大文字小文字を区別せずに一意にするため)。
    texture_names: HashSet<String>,
    /// 追加するテクスチャのインデックスの始まり。
    texture_base: usize,
    options: AutoMeshOptions,
    /// テクスチャの周りに足す透明な余白 (px)。膨張した輪郭がテクスチャの内側に収まるように。
    padding: u32,
    parts: Vec<Part>,
    /// 下から上の順のメッシュと、そのテクスチャ (名前・PNG)。
    meshes: Vec<(Mesh, (String, Vec<u8>))>,
    warnings: Vec<PsdImportWarning>,
}

impl Builder {
    fn new(model: &Model, options: AutoMeshOptions) -> Self {
        Self {
            part_ids: model.parts.iter().map(|p| p.id.to_string()).collect(),
            mesh_ids: model.meshes.iter().map(|m| m.id.to_string()).collect(),
            texture_names: model
                .textures
                .iter()
                .map(|t| t.name.to_ascii_lowercase())
                .collect(),
            texture_base: model.textures.len(),
            options,
            padding: options.margin.ceil() as u32 + 1,
            parts: Vec::new(),
            meshes: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn warn(&mut self, path: &[String], message: impl Into<String>) {
        self.warnings.push(PsdImportWarning {
            path: path.to_vec(),
            message: message.into(),
        });
    }

    fn add_part(
        &mut self,
        name: &str,
        fallback: &str,
        parent: Option<&PartId>,
        opacity: f32,
    ) -> PartId {
        let id = unique_id(
            &id_base(name, fallback),
            self.part_ids.iter().map(String::as_str),
        );
        self.part_ids.insert(id.clone());
        let id = PartId::new(id);
        self.parts.push(Part {
            id: id.clone(),
            name: name.to_owned(),
            parent: parent.cloned(),
            keyforms: KeyformGrid::constant(PartKeyform { opacity }),
        });
        id
    }

    /// 兄弟の並び (下から上) を取り込む。
    fn walk(
        &mut self,
        nodes: &[LayerNode],
        part: &PartId,
        path: &[String],
    ) -> Result<(), PsdImportError> {
        let mut base = Base::Nothing;
        for node in nodes {
            match node {
                LayerNode::Group {
                    name,
                    visible,
                    children,
                    id,
                    opacity,
                    blend_mode,
                    clipping,
                } => {
                    let path = [path, std::slice::from_ref(name)].concat();
                    if !matches!(blend_mode.as_str(), "pass" | "norm") {
                        self.warn(
                            &path,
                            format!("group blend mode `{blend_mode}` ignored (parts have none)"),
                        );
                    }
                    if *clipping {
                        self.warn(
                            &path,
                            "clipping of a group is not supported, imported unclipped",
                        );
                    }
                    let opacity = if *visible { *opacity } else { 0.0 };
                    let fallback = id.map_or_else(|| "Group".to_owned(), |id| format!("Group{id}"));
                    let child = self.add_part(name, &fallback, Some(part), opacity);
                    let start = self.meshes.len();
                    self.walk(children, &child, &path)?;
                    if !*clipping {
                        let ids: Vec<MeshId> = self.meshes[start..]
                            .iter()
                            .map(|(m, _)| m.id.clone())
                            .collect();
                        base = if ids.is_empty() {
                            Base::Skipped
                        } else {
                            let opacity = if *visible { 1.0 } else { 0.0 };
                            Base::Meshes { ids, opacity }
                        };
                    }
                }
                LayerNode::Layer(layer) => {
                    let path = [path, std::slice::from_ref(&layer.name)].concat();
                    let clip = layer.clipping.then_some(&base);
                    let added = self.add_layer(layer, part, &path, clip)?;
                    if !layer.clipping {
                        base = match added {
                            Some(id) => Base::Meshes {
                                ids: vec![id],
                                opacity: if layer.visible { layer.opacity } else { 0.0 },
                            },
                            None => Base::Skipped,
                        };
                    }
                }
            }
        }
        Ok(())
    }

    /// ラスタレイヤーを 1 つのテクスチャと 1 つのメッシュにする。取り込まなければ `None`。
    fn add_layer(
        &mut self,
        layer: &Layer,
        part: &PartId,
        path: &[String],
        clip: Option<&Base>,
    ) -> Result<Option<MeshId>, PsdImportError> {
        if layer.width == 0 || layer.height == 0 {
            self.warn(path, "empty layer skipped");
            return Ok(None);
        }
        let pad = self.padding;
        let (width, height) = (layer.width + 2 * pad, layer.height + 2 * pad);
        let rgba = pad_image(&layer.rgba, layer.width, layer.height, pad);
        let generated = match ugoe_mesher::generate(&rgba, width, height, &self.options.into()) {
            Ok(g) => g,
            Err(MeshError::Empty) => {
                self.warn(path, "fully transparent layer skipped");
                return Ok(None);
            }
            Err(MeshError::TooComplex) => {
                self.warn(path, "layer too complex to mesh, skipped");
                return Ok(None);
            }
            Err(source) => {
                return Err(PsdImportError::Mesh {
                    layer: path.join(" / "),
                    source,
                });
            }
        };

        let fallback = layer
            .id
            .map_or_else(|| "Layer".to_owned(), |id| format!("Layer{id}"));
        let id = unique_id(
            &id_base(&layer.name, &fallback),
            self.mesh_ids.iter().map(String::as_str),
        );
        self.mesh_ids.insert(id.clone());
        let texture = self.texture_name(&id);

        let blend = match layer.blend_mode.as_str() {
            "norm" => BlendMode::Normal,
            "mul" => BlendMode::Multiply,
            "lddg" | "add" => BlendMode::Additive,
            other => {
                self.warn(
                    path,
                    format!("blend mode `{other}` is not supported, using normal"),
                );
                BlendMode::Normal
            }
        };

        let mut opacity = if layer.visible {
            layer.opacity * layer.fill_opacity
        } else {
            0.0
        };
        let mut masks = Vec::new();
        match clip {
            None | Some(Base::Nothing) => {}
            Some(Base::Skipped) => {
                self.warn(path, "clipping base was not imported, imported unclipped");
            }
            Some(Base::Meshes { ids, opacity: base }) => {
                masks = ids.iter().cloned().map(MaskRef::normal).collect();
                opacity *= base;
            }
        }

        let size = Vec2::new(width as f32, height as f32);
        let origin = layer.offset - Vec2::splat(pad as f32);
        let mesh = Mesh {
            id: MeshId::new(id.clone()),
            name: layer.name.clone(),
            part: part.clone(),
            parent: None,
            texture: u32::try_from(self.texture_base + self.meshes.len()).unwrap_or(u32::MAX),
            uvs: generated.vertices.iter().map(|v| *v / size).collect(),
            indices: generated.indices,
            blend,
            culling: false,
            masks,
            mask_mode: MaskMode::Union,
            mask_only: false,
            keyforms: KeyformGrid::constant(MeshKeyform {
                positions: generated.vertices.iter().map(|v| origin + *v).collect(),
                opacity,
                // 全メッシュを並べ終えてから決める。
                draw_order: 0.0,
                multiply_color: Rgb::WHITE,
                screen_color: Rgb::BLACK,
                mask_strength: 1.0,
            }),
            mirror: None,
            mirror_axis: None,
            depth: None,
        };
        let png = encode_rgba_png(width, height, &rgba);
        self.meshes.push((mesh, (texture, png)));
        Ok(Some(MeshId::new(id)))
    }

    /// メッシュ ID から、大文字小文字を区別せずに一意なテクスチャ名を作る。
    fn texture_name(&mut self, mesh_id: &str) -> String {
        let mut base = mesh_id.to_owned();
        base.truncate(MAX_TEXTURE_NAME_LEN - 8);
        let mut name = base.clone();
        let mut n = 2u64;
        while self.texture_names.contains(&name.to_ascii_lowercase()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        self.texture_names.insert(name.to_ascii_lowercase());
        name
    }
}

/// ID の元にする文字列。名前が英数字を含まなければ (日本語だけの名前など) `fallback`。
fn id_base<'a>(name: &'a str, fallback: &'a str) -> std::borrow::Cow<'a, str> {
    if name.chars().any(|c| c.is_ascii_alphanumeric()) {
        name.into()
    } else {
        fallback.into()
    }
}

/// 画像の周りに `pad` px の透明な余白を足す。
fn pad_image(rgba: &[u8], width: u32, height: u32, pad: u32) -> Vec<u8> {
    let (w, h, p) = (width as usize, height as usize, pad as usize);
    let stride = (w + 2 * p) * 4;
    let mut out = vec![0; stride * (h + 2 * p)];
    for (y, row) in rgba.chunks_exact(w * 4).take(h).enumerate() {
        let start = (y + p) * stride + p * 4;
        out[start..start + w * 4].copy_from_slice(row);
    }
    out
}
