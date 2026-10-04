//! 評価結果を Godot の 2D の描画 (RenderingServer のキャンバスアイテム) で描く。
//!
//! - 描画可能要素ごとに 1 つのキャンバスアイテムと `ShaderMaterial` を持ち、描画順は draw index で決める。
//! - マスクのカバレッジは、すべてを 1 枚のアトラス (`SubViewport`) に並べて描く (アトラスでの場所は
//!   [`AtlasLayout`]、シェーダはそれを表 [`MaskTable`] から読む)。マスクを読む要素は、アトラスをキャンバス座標から読む
//!   (ノードの変形に左右されない)。
//! - 入れ子のマスク (マスク用メッシュ自身がマスクを読む) は段に分ける ([`crate::model::MaskPlan::levels`])。
//!   段 `k` のアトラスは、段 `k − 1` のアトラスを写してから段 `k` のカバレッジを描く。段 `k − 1` のビューポートを
//!   段 `k` のビューポートの子にするので、Godot は同じフレームの中で必ず先に描く (子のビューポートが先)。
//!   要素は最後の段のアトラスだけを読む。
//! - 裏面カリングは CPU で三角形を選んで行う (キャンバス座標の向きだけで決まるため)。

use godot::classes::image::Format;
use godot::classes::rendering_server::{CanvasItemTextureFilter, CanvasItemTextureRepeat};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{
    Image, ImageTexture, Node, Node2D, RenderingServer, Shader, ShaderMaterial, SubViewport,
    ViewportTexture,
};
use godot::prelude::*;
use ugoe::core::Vec2 as UVec2;
use ugoe::format::{TextureData, TextureFormat};
use ugoe::model::{BlendMode, MaskMode};
use ugoe::runtime::{DrawableChanges, DrawableState};

use crate::model::{
    AtlasLayout, LightUniforms, LoadedModel, MaskArea, MaskShrink, MaskTable, bounds, front_facing,
    premultiply,
};
use crate::shader::{self, ShaderKind};

/// アトラスの段 1 つ分 (`SubViewport` 1 つ)。
struct MaskLevel {
    viewport: Gd<SubViewport>,
    /// ビューポートの中の根。カバレッジのキャンバスアイテムの親の親。
    _root: Gd<Node2D>,
    /// 前の段のアトラスを写すキャンバスアイテム。カバレッジのキャンバスアイテムはこの子 (写した後に描く)。
    copy: Rid,
    _copy_material: Option<Gd<ShaderMaterial>>,
    texture: Gd<ViewportTexture>,
}

/// マスクのカバレッジ 1 つ分。
struct MaskItem {
    /// マスクとして使うメッシュの番号。
    mesh: usize,
    item: Rid,
    material: Gd<ShaderMaterial>,
    area: Option<MaskArea>,
    /// アトラスでの左上の画素。
    slot: (u32, u32),
}

/// 描画可能要素 1 つ分。
struct DrawItem {
    item: Rid,
    material: Gd<ShaderMaterial>,
}

/// テクスチャの番号ごとの、デコードしたテクスチャと法線マップ。
#[derive(Default)]
pub struct DecodedTexture {
    color: Option<Gd<ImageTexture>>,
    normal: Option<Gd<ImageTexture>>,
}

/// テクスチャの番号ごとのデコードの結果 (`None` はまだデコードしていない)。
/// 着せ替えでレンダラを作り直すときに、変わっていないテクスチャを使い回す。
pub type TextureCache = Vec<Option<DecodedTexture>>;

/// 1 体分の描画の資源。
pub struct Renderer {
    container: Rid,
    items: Vec<DrawItem>,
    levels: Vec<MaskLevel>,
    masks: Vec<MaskItem>,
    table: MaskTable,
    table_texture: Option<Gd<ImageTexture>>,
    /// アトラスの大きさ (画素)。
    atlas: (u32, u32),
    textures: TextureCache,
    /// 読み込んでから最初の描画か (すべてを描き直す)。
    fresh: bool,
    /// 直前の描画で使ったマスクの解像度 (キャンバスの 1 単位あたりの画素数)。
    mask_scale: f32,
    /// アトラスに収まらなかったときに、マスクの解像度に掛ける倍率 (解像度が変わると 1 に戻す)。
    shrink: MaskShrink,
    // 毎フレームの作業用
    points: PackedVector2Array,
    uvs: Vec<PackedVector2Array>,
    indices: Vec<PackedInt32Array>,
    culled: Vec<i32>,
    colors: PackedColorArray,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("drawables", &self.items.len())
            .field("masks", &self.masks.len())
            .field("levels", &self.levels.len())
            .finish_non_exhaustive()
    }
}

fn rs() -> Gd<RenderingServer> {
    RenderingServer::singleton()
}

fn make_shader(kind: ShaderKind) -> Gd<Shader> {
    let mut s = Shader::new_gd();
    s.set_code(&shader::source(kind));
    s
}

fn material(shader: &Gd<Shader>) -> Gd<ShaderMaterial> {
    let mut m = ShaderMaterial::new_gd();
    m.set_shader(shader);
    m
}

fn vec2(p: UVec2) -> Vector2 {
    Vector2::new(p.x, p.y)
}

/// 画像をデコードする。読めなければ `None`。
fn decode(data: &TextureData, what: &str) -> Option<Gd<Image>> {
    let mut image = Image::new_gd();
    let bytes = PackedByteArray::from(data.bytes.as_slice());
    let err = match data.format {
        TextureFormat::Png => image.load_png_from_buffer(&bytes),
        TextureFormat::Ktx2 => image.load_ktx_from_buffer(&bytes),
    };
    if err != godot::global::Error::OK || image.is_empty() {
        godot_warn!("ugoe: failed to decode a {what} ({:?})", data.format);
        return None;
    }
    Some(image)
}

/// 画像を Godot のテクスチャにする。PNG は乗算済みアルファに変換する。読めなければ `None` (白で描く)。
fn load_texture(data: &TextureData) -> Option<Gd<ImageTexture>> {
    let mut image = decode(data, "texture")?;
    // TODO(T0448): KTX2 は乗算済みかどうかを確かめずにそのまま使っている (仕様が決まっていない)。
    if data.format == TextureFormat::Png {
        if image.get_format() != Format::RGBA8 {
            image.convert(Format::RGBA8);
        }
        let mut rgba = image.get_data().to_vec();
        premultiply(&mut rgba);
        let (w, h) = (image.get_width(), image.get_height());
        image.set_data(
            w,
            h,
            false,
            Format::RGBA8,
            &PackedByteArray::from(rgba.as_slice()),
        );
    }
    ImageTexture::create_from_image(&image)
}

/// 法線マップを Godot のテクスチャにする。アルファは掛けない (`docs/spec/rendering.md#ホストでのライティング`)。
/// 読めなければ `None` (法線 `(0, 0, 1)`)。
fn load_normal_map(data: &TextureData) -> Option<Gd<ImageTexture>> {
    let image = decode(data, "normal map")?;
    ImageTexture::create_from_image(&image)
}

fn texture_rid(textures: &TextureCache, index: usize) -> Rid {
    textures
        .get(index)
        .and_then(Option::as_ref)
        .and_then(|t| t.color.as_ref())
        .map_or(Rid::Invalid, |t| t.get_rid())
}

fn new_item(parent: Rid) -> Rid {
    let mut rs = rs();
    let item = rs.canvas_item_create();
    rs.canvas_item_set_parent(item, parent);
    rs.canvas_item_set_default_texture_filter(item, CanvasItemTextureFilter::LINEAR);
    rs.canvas_item_set_default_texture_repeat(item, CanvasItemTextureRepeat::DISABLED);
    item
}

/// マスクを読む uniform (表と、要素が読むマスクの列) を設定する。
fn set_mask_params(
    material: &mut Gd<ShaderMaterial>,
    atlas: &Gd<ViewportTexture>,
    table: &Option<Gd<ImageTexture>>,
    first: i32,
    count: usize,
    mode: MaskMode,
) {
    if count == 0 {
        return;
    }
    material.set_shader_parameter("mask_atlas", &atlas.to_variant());
    if let Some(table) = table {
        material.set_shader_parameter("mask_table", &table.to_variant());
    }
    material.set_shader_parameter("mask_first", &first.to_variant());
    material.set_shader_parameter("mask_count", &(count as i32).to_variant());
    material.set_shader_parameter(
        "mask_intersection",
        &(mode == MaskMode::Intersection).to_variant(),
    );
}

impl Renderer {
    /// 資源を作る。`owner` はキャンバスアイテムの親とマスクのビューポートを持つノード。
    /// `textures` は前のレンダラから引き継ぐデコード済みのテクスチャ (空でよい)。
    pub fn new(
        owner: &mut Gd<Node2D>,
        model: &LoadedModel,
        mut textures: TextureCache,
        lighting: &LightUniforms,
    ) -> Self {
        let premul = make_shader(ShaderKind::PremulAlpha);
        let multiply = make_shader(ShaderKind::Multiply);
        let coverage = make_shader(ShaderKind::Coverage);
        textures.resize_with(model.texture_count(), || None);
        for (i, slot) in textures.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(DecodedTexture {
                    color: model.texture(i).and_then(load_texture),
                    normal: model.normal_map(i).and_then(load_normal_map),
                });
            }
        }

        let container = new_item(owner.get_canvas_item());
        let origin = model.model().meta.canvas.origin;
        let plan = &model.masks;
        let table = MaskTable::new(&model.drawables, plan.sources.len());
        let table_texture = (!plan.sources.is_empty()).then(|| {
            let (w, h) = table.size();
            let image = Image::create_from_data(
                w as i32,
                h as i32,
                false,
                Format::RGBAF,
                &PackedByteArray::from(table.bytes().as_slice()),
            );
            image
                .and_then(|i| ImageTexture::create_from_image(&i))
                .unwrap_or_else(ImageTexture::new_gd)
        });

        // アトラスの段。段 k − 1 のビューポートを段 k のビューポートの子にする (子が先に描かれる)。
        let copy_shader = make_shader(ShaderKind::Copy);
        let mut levels: Vec<MaskLevel> = Vec::new();
        for k in 0..plan.level_count() {
            let mut viewport = SubViewport::new_alloc();
            viewport.set_name(&format!("UgoeMaskAtlas{k}"));
            viewport.set_transparent_background(true);
            viewport.set_disable_3d(true);
            viewport.set_update_mode(UpdateMode::ALWAYS);
            viewport.set_size(Vector2i::new(1, 1));
            let root = Node2D::new_alloc();
            viewport.add_child(&root);
            if let Some(previous) = levels.last() {
                viewport.add_child(&previous.viewport);
            }
            let copy = new_item(root.get_canvas_item());
            let copy_material = (k > 0).then(|| {
                let m = material(&copy_shader);
                let mut rs = rs();
                rs.canvas_item_set_material(copy, m.get_rid());
                rs.canvas_item_set_default_texture_filter(copy, CanvasItemTextureFilter::NEAREST);
                m
            });
            let Some(texture) = viewport.get_texture() else {
                godot_error!("ugoe: a mask viewport has no texture");
                break;
            };
            levels.push(MaskLevel {
                viewport,
                _root: root,
                copy,
                _copy_material: copy_material,
                texture,
            });
        }
        if let Some(top) = levels.last() {
            owner
                .clone()
                .upcast::<Node>()
                .add_child_ex(&top.viewport)
                .internal(godot::classes::node::InternalMode::BACK)
                .done();
        }

        let masks = if levels.len() == plan.level_count() {
            plan.sources
                .iter()
                .zip(&plan.levels)
                .map(|(&mesh, &level)| {
                    let level = level as usize;
                    let item = new_item(levels[level].copy);
                    let mut material = material(&coverage);
                    rs().canvas_item_set_material(item, material.get_rid());
                    let info = &model.drawables[mesh];
                    if level > 0 {
                        set_mask_params(
                            &mut material,
                            &levels[level - 1].texture,
                            &table_texture,
                            table.first[mesh],
                            info.masks.len(),
                            info.mask_mode,
                        );
                    }
                    MaskItem {
                        mesh,
                        item,
                        material,
                        area: None,
                        slot: (0, 0),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let items = model
            .drawables
            .iter()
            .zip(&model.model().meshes)
            .enumerate()
            .map(|(d, (info, mesh))| {
                let item = new_item(container);
                let shader = if mesh.blend.effective() == BlendMode::Multiply {
                    &multiply
                } else {
                    &premul
                };
                let mut material = material(shader);
                material.set_shader_parameter("canvas_origin", &vec2(origin).to_variant());
                material.set_shader_parameter(
                    "additive",
                    &(mesh.blend.effective() == BlendMode::Additive).to_variant(),
                );
                if let (Some(top), false) = (levels.last(), masks.is_empty()) {
                    set_mask_params(
                        &mut material,
                        &top.texture,
                        &table_texture,
                        table.first[d],
                        info.masks.len(),
                        info.mask_mode,
                    );
                }
                if info.lit {
                    let normal = textures
                        .get(info.texture)
                        .and_then(Option::as_ref)
                        .and_then(|t| t.normal.as_ref());
                    let mode = if normal.is_some() { 2 } else { 1 };
                    material.set_shader_parameter("lit_mode", &mode.to_variant());
                    if let Some(normal) = normal {
                        material.set_shader_parameter("normal_map", &normal.to_variant());
                    }
                    set_light_params(&mut material, lighting);
                }
                rs().canvas_item_set_material(item, material.get_rid());
                DrawItem { item, material }
            })
            .collect();

        let uvs = model
            .drawables
            .iter()
            .map(|d| d.uvs.iter().map(|&p| vec2(p)).collect())
            .collect();
        let indices = model
            .drawables
            .iter()
            .map(|d| d.indices.iter().map(|&i| i32::from(i)).collect())
            .collect();

        Self {
            container,
            items,
            levels,
            masks,
            table,
            table_texture,
            atlas: (0, 0),
            textures,
            fresh: true,
            mask_scale: 0.0,
            shrink: MaskShrink::default(),
            points: PackedVector2Array::new(),
            uvs,
            indices,
            culled: Vec::new(),
            colors: PackedColorArray::from([Color::WHITE].as_slice()),
        }
    }

    /// 光源を変える (ライティングを受ける要素だけ)。
    pub fn set_lighting(&mut self, model: &LoadedModel, lighting: &LightUniforms) {
        for (item, info) in self.items.iter_mut().zip(&model.drawables) {
            if info.lit {
                set_light_params(&mut item.material, lighting);
            }
        }
    }

    /// デコードしたテクスチャを取り出す (着せ替えでレンダラを作り直すときに使い回す)。
    pub fn take_textures(&mut self) -> TextureCache {
        std::mem::take(&mut self.textures)
    }

    /// アトラスに収まらずにマスクの解像度に掛けている倍率 (1 なら下げていない)。
    pub fn mask_shrink(&self) -> f32 {
        self.shrink.factor
    }

    /// 直前の `Puppet::update` の結果を描く。`mask_scale` はマスクのテクスチャのキャンバス 1 単位あたりの画素数。
    pub fn draw(&mut self, model: &LoadedModel, mask_scale: f32) {
        let instance = model.puppet.instance();
        let states = instance.drawables();
        let changes = instance.changes();
        let origin = model.model().meta.canvas.origin;
        let fresh = std::mem::take(&mut self.fresh);
        let rescale = (mask_scale - self.mask_scale).abs() > 1e-3 * mask_scale;
        self.mask_scale = mask_scale;
        if rescale {
            self.shrink = MaskShrink::default();
        }
        let mut rs = rs();

        // 1. マスクのカバレッジ: 大きさを決め、アトラスに並べ、幾何を送る。
        let mut resized = false;
        let mut moved = vec![false; self.masks.len()];
        for (s, mask) in self.masks.iter_mut().enumerate() {
            let state = &states[mask.mesh];
            if !(fresh || rescale || changes[mask.mesh].contains(DrawableChanges::POSITIONS)) {
                continue;
            }
            let (min, max) = bounds(&state.positions).unwrap_or((UVec2::ZERO, UVec2::ZERO));
            let current = mask.area.map(|a| (a.width, a.height));
            let area = MaskArea::fit(min, max, mask_scale * self.shrink.factor, current);
            if mask.area != Some(area) {
                resized |= current != Some((area.width, area.height));
                mask.area = Some(area);
                moved[s] = true;
            }
            self.points = state.positions.iter().map(|&p| vec2(p)).collect();
            rs.canvas_item_clear(mask.item);
            if !self.indices[mask.mesh].is_empty() {
                rs.canvas_item_add_triangle_array_ex(
                    mask.item,
                    &self.indices[mask.mesh],
                    &self.points,
                    &self.colors,
                )
                .uvs(&self.uvs[mask.mesh])
                .texture(texture_rid(
                    &self.textures,
                    model.drawables[mask.mesh].texture,
                ))
                .done();
            }
        }
        // 解像度を下げている間にマスクが動いたら、戻せるかを確かめる (戻したら並べ直す)。
        let restored = self.shrink.factor < 1.0
            && moved.iter().any(|&m| m)
            && self.try_restore(model, mask_scale);
        if resized && !restored && !self.masks.is_empty() {
            self.pack(model, mask_scale);
        }
        if resized || restored {
            moved.fill(true);
        }
        for (s, mask) in self.masks.iter_mut().enumerate() {
            if fresh || changes[mask.mesh].contains(DrawableChanges::MASK_STRENGTH) {
                let strength = states[mask.mesh].mask_strength;
                mask.material
                    .set_shader_parameter("mask_strength", &strength.to_variant());
            }
            if !moved[s] {
                continue;
            }
            let Some(area) = mask.area else { continue };
            // キャンバス座標 → アトラスの画素: slot + (p − origin) × scale
            let offset = Vector2::new(mask.slot.0 as f32, mask.slot.1 as f32)
                - vec2(area.origin) * area.scale;
            rs.canvas_item_set_transform(
                mask.item,
                Transform2D::from_cols(
                    Vector2::new(area.scale, 0.0),
                    Vector2::new(0.0, area.scale),
                    offset,
                ),
            );
            self.table.set_source(s, &area, mask.slot, self.atlas);
        }
        if moved.iter().any(|&m| m)
            && let Some(texture) = &mut self.table_texture
        {
            let (w, h) = self.table.size();
            if let Some(image) = Image::create_from_data(
                w as i32,
                h as i32,
                false,
                Format::RGBAF,
                &PackedByteArray::from(self.table.bytes().as_slice()),
            ) {
                if texture.get_size() == Vector2::new(w as f32, h as f32) {
                    texture.update(&image);
                } else {
                    texture.set_image(&image);
                }
            }
        }

        // 2. 色 (描画順)
        for (order, &d) in instance.render_order().iter().enumerate() {
            rs.canvas_item_set_draw_index(self.items[d as usize].item, order as i32);
        }
        for (d, (state, info)) in states.iter().zip(&model.drawables).enumerate() {
            let changed = changes[d];
            let item = &mut self.items[d];
            if fresh || changed.contains(DrawableChanges::VISIBILITY) {
                rs.canvas_item_set_visible(item.item, state.visible);
            }
            if !state.visible {
                continue;
            }
            // 見えるようになった要素は、見えない間の変化を反映していないので描き直す。
            let redraw = fresh
                || changed.contains(DrawableChanges::POSITIONS)
                || changed.contains(DrawableChanges::VISIBILITY);
            if redraw {
                self.points = state.positions.iter().map(|&p| vec2(p - origin)).collect();
                let indices = if info.culling {
                    front_facing(&state.positions, &info.indices, &mut self.culled);
                    &PackedInt32Array::from(self.culled.as_slice())
                } else {
                    &self.indices[d]
                };
                rs.canvas_item_clear(item.item);
                // 頂点番号が空だと Godot は頂点を順に三角形として描くので、描く三角形が無ければ何も足さない。
                if !indices.is_empty() {
                    rs.canvas_item_add_triangle_array_ex(
                        item.item,
                        indices,
                        &self.points,
                        &self.colors,
                    )
                    .uvs(&self.uvs[d])
                    .texture(texture_rid(&self.textures, info.texture))
                    .done();
                }
            }
            set_color_params(&mut item.material, state, changed, redraw);
        }
    }

    /// マスク用メッシュの今の外接矩形 (`self.masks` の順)。
    fn mask_bounds(&self, model: &LoadedModel) -> Vec<(UVec2, UVec2)> {
        let states = model.puppet.instance().drawables();
        self.masks
            .iter()
            .map(|m| bounds(&states[m.mesh].positions).unwrap_or((UVec2::ZERO, UVec2::ZERO)))
            .collect()
    }

    /// 下げているマスクの解像度を 1 段戻せるなら戻して並べ直す (`MaskShrink::try_restore`)。戻したら true。
    fn try_restore(&mut self, model: &LoadedModel, mask_scale: f32) -> bool {
        let bounds = self.mask_bounds(model);
        let Some((areas, layout)) = self.shrink.try_restore(&bounds, mask_scale) else {
            return false;
        };
        for (mask, area) in self.masks.iter_mut().zip(areas) {
            mask.area = Some(area);
        }
        self.apply_layout(&layout);
        true
    }

    /// カバレッジをアトラスに並べ直し、ビューポートの大きさを合わせる。
    /// 収まらなければ、収まるまでマスクの解像度を下げる (`MaskShrink::shrink_to_fit`)。
    fn pack(&mut self, model: &LoadedModel, mask_scale: f32) {
        let sizes: Vec<(u32, u32)> = self
            .masks
            .iter()
            .map(|m| m.area.map_or((1, 1), |a| (a.width, a.height)))
            .collect();
        let layout = match AtlasLayout::pack(&sizes, MaskArea::MAX_SIZE) {
            Some(layout) => layout,
            None => {
                let bounds = self.mask_bounds(model);
                let Some((areas, layout)) = self.shrink.shrink_to_fit(&bounds, mask_scale) else {
                    godot_error!("ugoe: masks do not fit in the mask atlas");
                    return;
                };
                for (mask, area) in self.masks.iter_mut().zip(areas) {
                    mask.area = Some(area);
                }
                layout
            }
        };
        self.apply_layout(&layout);
    }

    /// カバレッジの配置 `layout` を当て、ビューポートの大きさを合わせる。
    fn apply_layout(&mut self, layout: &AtlasLayout) {
        for (mask, &slot) in self.masks.iter_mut().zip(&layout.positions) {
            mask.slot = slot;
        }
        // 大きさは足りる間は使い回す (面積の 4 倍より大きければ縮める)。
        let (w, h) = self.atlas;
        let fits = w >= layout.width && h >= layout.height;
        let too_big =
            u64::from(w) * u64::from(h) > 4 * u64::from(layout.width) * u64::from(layout.height);
        if fits && !too_big {
            return;
        }
        // 少し余裕を持たせて、頻繁に作り直さないようにする。
        let grow = |n: u32| n.next_multiple_of(64).min(MaskArea::MAX_SIZE);
        self.atlas = (grow(layout.width), grow(layout.height));
        let size = Vector2i::new(self.atlas.0 as i32, self.atlas.1 as i32);
        let mut rs = rs();
        for k in 0..self.levels.len() {
            self.levels[k].viewport.set_size(size);
            if k > 0 {
                let copy = self.levels[k].copy;
                let previous = self.levels[k - 1].texture.get_rid();
                rs.canvas_item_clear(copy);
                rs.canvas_item_add_texture_rect(
                    copy,
                    Rect2::new(Vector2::ZERO, Vector2::new(size.x as f32, size.y as f32)),
                    previous,
                );
            }
        }
    }

    /// マスクのビューポートをノードから外して解放する (モデルを外すとき)。
    ///
    /// ノードごと破棄されるときは Godot が子として解放するので呼ばなくてよい。
    pub fn remove_viewports(&mut self) {
        // 下の段は上の段の子なので、いちばん上の段を外せば全部が外れる。
        if let Some(top) = self.levels.last_mut()
            && top.viewport.is_instance_valid()
        {
            top.viewport.queue_free();
        }
    }
}

impl Drop for Renderer {
    /// キャンバスアイテムを解放する。
    fn drop(&mut self) {
        let mut rs = rs();
        for item in &self.items {
            rs.free_rid(item.item);
        }
        for mask in &self.masks {
            rs.free_rid(mask.item);
        }
        for level in &self.levels {
            rs.free_rid(level.copy);
        }
        rs.free_rid(self.container);
    }
}

fn set_light_params(material: &mut Gd<ShaderMaterial>, lighting: &LightUniforms) {
    let [r, g, b] = lighting.ambient;
    material.set_shader_parameter("ambient_light", &Vector3::new(r, g, b).to_variant());
    material.set_shader_parameter("light_count", &(lighting.vectors.len() as i32).to_variant());
    let pad = |v: &[[f32; 4]]| -> PackedVector4Array {
        (0..ugoe::render::MAX_LIGHTS)
            .map(|i| {
                let [x, y, z, w] = v.get(i).copied().unwrap_or([0.0; 4]);
                Vector4::new(x, y, z, w)
            })
            .collect()
    };
    material.set_shader_parameter("light_vectors", &pad(&lighting.vectors).to_variant());
    material.set_shader_parameter("light_colors", &pad(&lighting.colors).to_variant());
}

fn set_color_params(
    material: &mut Gd<ShaderMaterial>,
    state: &DrawableState,
    changed: DrawableChanges,
    all: bool,
) {
    if all || changed.contains(DrawableChanges::OPACITY) {
        material.set_shader_parameter("opacity", &state.opacity.to_variant());
    }
    if all || changed.contains(DrawableChanges::MULTIPLY_COLOR) {
        let c = state.multiply_color;
        material.set_shader_parameter("multiply_color", &Vector3::new(c.r, c.g, c.b).to_variant());
    }
    if all || changed.contains(DrawableChanges::SCREEN_COLOR) {
        let c = state.screen_color;
        material.set_shader_parameter("screen_color", &Vector3::new(c.r, c.g, c.b).to_variant());
    }
    if all || changed.contains(DrawableChanges::MASK_STRENGTH) {
        material.set_shader_parameter("mask_strength", &state.mask_strength.to_variant());
    }
}
