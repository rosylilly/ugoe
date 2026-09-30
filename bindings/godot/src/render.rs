//! 評価結果を Godot の 2D の描画 (RenderingServer のキャンバスアイテム) で描く。
//!
//! - 描画可能要素ごとに 1 つのキャンバスアイテムと `ShaderMaterial` を持ち、描画順は draw index で決める。
//! - マスクとして参照されるメッシュごとに `SubViewport` を 1 つ持ち、そのメッシュのカバレッジを描く。
//!   マスクを読む要素は、そのテクスチャをキャンバス座標から読む (ノードの変形に左右されない)。
//! - 裏面カリングは CPU で三角形を選んで行う (キャンバス座標の向きだけで決まるため)。

use godot::classes::image::Format;
use godot::classes::rendering_server::{CanvasItemTextureFilter, CanvasItemTextureRepeat};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{
    Image, ImageTexture, Node, Node2D, RenderingServer, Shader, ShaderMaterial, SubViewport,
};
use godot::prelude::*;
use ugoe::core::Vec2 as UVec2;
use ugoe::format::TextureFormat;
use ugoe::model::{BlendMode, MaskMode};
use ugoe::runtime::{DrawableChanges, DrawableState};

use crate::model::{
    DrawableInfo, LoadedModel, MaskArea, MaskSlot, bounds, front_facing, premultiply,
};
use crate::shader::{self, ShaderKind};

/// マスクのカバレッジを描く `SubViewport` 1 つ分。
struct MaskTarget {
    /// マスクとして使うメッシュの番号。
    mesh: usize,
    viewport: Gd<SubViewport>,
    /// キャンバス座標をテクスチャの画素へ写すノード。
    root: Gd<Node2D>,
    item: Rid,
    material: Gd<ShaderMaterial>,
    texture: Variant,
    area: Option<MaskArea>,
}

/// 描画可能要素 1 つ分。
struct DrawItem {
    item: Rid,
    material: Gd<ShaderMaterial>,
}

/// 1 体分の描画の資源。
pub struct Renderer {
    container: Rid,
    items: Vec<DrawItem>,
    masks: Vec<MaskTarget>,
    textures: Vec<Option<Gd<ImageTexture>>>,
    /// 読み込んでから最初の描画か (すべてを描き直す)。
    fresh: bool,
    /// 直前の描画で使ったマスクの解像度 (キャンバスの 1 単位あたりの画素数)。
    mask_scale: f32,
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

/// 画像を Godot のテクスチャにする。PNG は乗算済みアルファに変換する。読めなければ `None` (白で描く)。
fn load_texture(data: &ugoe::format::TextureData) -> Option<Gd<ImageTexture>> {
    let mut image = Image::new_gd();
    let bytes = PackedByteArray::from(data.bytes.as_slice());
    let err = match data.format {
        TextureFormat::Png => image.load_png_from_buffer(&bytes),
        TextureFormat::Ktx2 => image.load_ktx_from_buffer(&bytes),
    };
    if err != godot::global::Error::OK || image.is_empty() {
        godot_warn!("ugoe: failed to decode a texture ({:?})", data.format);
        return None;
    }
    // TODO(T0406): KTX2 が圧縮形式なら、乗算済みかどうかを確かめずにそのまま使っている。
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

fn texture_rid(textures: &[Option<Gd<ImageTexture>>], index: usize) -> Rid {
    textures
        .get(index)
        .and_then(Option::as_ref)
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

/// 要素のマスクの uniform を設定する。
fn set_mask_params(
    material: &mut Gd<ShaderMaterial>,
    info: &DrawableInfo,
    masks: &[MaskTarget],
    first: bool,
) {
    if info.masks.is_empty() {
        return;
    }
    for (i, MaskSlot { source, .. }) in info.masks.iter().enumerate() {
        let target = &masks[*source];
        if first {
            material.set_shader_parameter(&shader::mask_texture_param(i), &target.texture);
        }
        if let Some(area) = target.area {
            let [x, y, w, h] = area.rect();
            material.set_shader_parameter(
                &shader::mask_rect_param(i),
                &Vector4::new(x, y, w, h).to_variant(),
            );
        }
    }
    if first {
        let inverts = info
            .masks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.invert)
            .fold(0i32, |acc, (i, _)| acc | (1 << i));
        material.set_shader_parameter("mask_count", &(info.masks.len() as i32).to_variant());
        material.set_shader_parameter("mask_inverts", &inverts.to_variant());
        material.set_shader_parameter(
            "mask_intersection",
            &(info.mask_mode == MaskMode::Intersection).to_variant(),
        );
    }
}

impl Renderer {
    /// 資源を作る。`owner` はキャンバスアイテムの親とマスクのビューポートを持つノード。
    pub fn new(owner: &mut Gd<Node2D>, model: &LoadedModel) -> Self {
        let premul = make_shader(ShaderKind::PremulAlpha);
        let multiply = make_shader(ShaderKind::Multiply);
        let coverage = make_shader(ShaderKind::Coverage);
        let textures: Vec<_> = model
            .textures
            .iter()
            .map(|t| t.as_ref().and_then(load_texture))
            .collect();

        let container = new_item(owner.get_canvas_item());
        let origin = model.model().meta.canvas.origin;

        let masks = model
            .masks
            .sources
            .iter()
            .map(|&mesh| {
                let mut viewport = SubViewport::new_alloc();
                viewport.set_name(&format!("UgoeMask{mesh}"));
                viewport.set_transparent_background(true);
                viewport.set_disable_3d(true);
                viewport.set_update_mode(UpdateMode::ALWAYS);
                viewport.set_size(Vector2i::new(1, 1));
                let root = Node2D::new_alloc();
                viewport.add_child(&root);
                // 入れ子のマスクを先に作るので、先に足したビューポートが先に描かれることを期待する。
                owner
                    .clone()
                    .upcast::<Node>()
                    .add_child_ex(&viewport)
                    .internal(godot::classes::node::InternalMode::BACK)
                    .done();
                let item = new_item(root.get_canvas_item());
                let material = material(&coverage);
                rs().canvas_item_set_material(item, material.get_rid());
                let texture = viewport
                    .get_texture()
                    .map_or(Variant::nil(), |t| t.to_variant());
                MaskTarget {
                    mesh,
                    viewport,
                    root,
                    item,
                    material,
                    texture,
                    area: None,
                }
            })
            .collect::<Vec<_>>();

        let items = model
            .drawables
            .iter()
            .zip(&model.model().meshes)
            .map(|(_, mesh)| {
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
            masks,
            textures,
            fresh: true,
            mask_scale: 0.0,
            points: PackedVector2Array::new(),
            uvs,
            indices,
            culled: Vec::new(),
            colors: PackedColorArray::from([Color::WHITE].as_slice()),
        }
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
        let mut rs = rs();

        // 1. マスクのカバレッジ (入れ子のマスクが先)
        let mut moved = vec![false; self.masks.len()];
        for s in 0..self.masks.len() {
            let mesh = self.masks[s].mesh;
            let state = &states[mesh];
            let changed = changes[mesh];
            if fresh || rescale || changed.contains(DrawableChanges::POSITIONS) {
                if let Some((min, max)) = bounds(&state.positions) {
                    let target = &mut self.masks[s];
                    let current = target.area.map(|a| (a.width, a.height));
                    let area = MaskArea::fit(min, max, mask_scale, current);
                    if target.area != Some(area) {
                        if current != Some((area.width, area.height)) {
                            target
                                .viewport
                                .set_size(Vector2i::new(area.width as i32, area.height as i32));
                        }
                        target.root.set_scale(Vector2::new(area.scale, area.scale));
                        target.root.set_position(-vec2(area.origin) * area.scale);
                        target.area = Some(area);
                        moved[s] = true;
                    }
                }
                self.points = state.positions.iter().map(|&p| vec2(p)).collect();
                let target = &self.masks[s];
                rs.canvas_item_clear(target.item);
                if !self.indices[mesh].is_empty() {
                    rs.canvas_item_add_triangle_array_ex(
                        target.item,
                        &self.indices[mesh],
                        &self.points,
                        &self.colors,
                    )
                    .uvs(&self.uvs[mesh])
                    .texture(texture_rid(&self.textures, model.drawables[mesh].texture))
                    .done();
                }
            }
            let info = &model.drawables[mesh];
            let reads_moved = info.masks.iter().any(|m| moved[m.source]);
            let (targets_before, rest) = self.masks.split_at_mut(s);
            let target = &mut rest[0];
            if fresh || reads_moved {
                set_mask_params(&mut target.material, info, targets_before, fresh);
            }
            if fresh || changed.contains(DrawableChanges::MASK_STRENGTH) {
                target
                    .material
                    .set_shader_parameter("mask_strength", &state.mask_strength.to_variant());
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
            if fresh || info.masks.iter().any(|m| moved[m.source]) {
                set_mask_params(&mut item.material, info, &self.masks, fresh);
            }
        }
    }

    /// マスクのビューポートをノードから外して解放する (モデルを外すとき)。
    ///
    /// ノードごと破棄されるときは Godot が子として解放するので呼ばなくてよい。
    pub fn remove_viewports(&mut self) {
        for mask in &mut self.masks {
            if mask.viewport.is_instance_valid() {
                mask.viewport.queue_free();
            }
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
        rs.free_rid(self.container);
    }
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
