//! ビルド時のテクスチャのアトラス化。仕様: `docs/spec/file-formats.md#テクスチャのアトラス化`、
//! 判断の経緯: `docs/adr/0016-draw-batching.md`。
//!
//! レイヤーごとのテクスチャを数枚のアトラスにまとめ、メッシュの `texture` と `uvs` を書き換える。
//! 描画順で隣り合うメッシュが同じテクスチャを使うようになり、レンダラが描画をまとめられる。
//!
//! - **見た目は変えない。** 各画像の周りに端の画素を `padding` 幅だけ複製する。参照レンダラの
//!   サンプラ (バイリニア・端でクランプ、ミップマップなし) で、UV が 0〜1 の範囲なら元の画像と同じ色になる。
//! - UV が 0〜1 の外に出るメッシュのテクスチャ (クランプに頼っている) と、1 枚で上限を超える
//!   テクスチャはまとめずに残す。どのメッシュも使わないテクスチャは取り除く。
//! - 画素がまったく同じ画像は 1 つだけ詰め、それを使うメッシュは同じ場所を指す。
//! - 出力は入力に対して決定的 (同じ入力からは同じバイト列)。

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;
use ugoe_model::{Mesh, Model, Module, TextureRef};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::texture::{ImageError, ImageLimits, decode_png, encode_rgba_png};
use crate::{Project, Textures};

/// アトラスの名前の接頭辞 (`atlas-0`、`atlas-1`…)。
pub(crate) const ATLAS_NAME_PREFIX: &str = "atlas-";

/// アトラス化の設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasOptions {
    /// アトラス 1 枚の幅と高さの上限 (px)。既定の 2048 は WebGL2 でも使える大きさ。
    pub max_size: u32,
    /// 各画像の周りに複製する端の画素の幅 (px)。バイリニアのサンプリングには 1 あれば足りる。
    pub padding: u32,
}

impl Default for AtlasOptions {
    fn default() -> Self {
        Self {
            max_size: 2048,
            padding: 2,
        }
    }
}

/// まとめずに残したテクスチャの理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KeptReason {
    /// 周りの複製を含めると、1 枚で `max_size` を超える。
    TooLarge,
    /// 使うメッシュの UV が 0〜1 の外に出る (または有限でない)。
    UvOutOfRange,
    /// 法線マップを持つ (法線マップは同じ UV 空間なので、テクスチャだけを詰めると食い違う)。
    HasNormalMap,
}

/// アトラス化の結果の要約。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
pub struct AtlasReport {
    /// アトラスにまとめた元のテクスチャの数 (重複として除いたものを含む)。
    pub packed: usize,
    /// 作ったアトラスの幅と高さ (出力の `Model::textures` の先頭から順に並ぶ)。
    pub atlases: Vec<(u32, u32)>,
    /// まとめずに残したテクスチャ (元の名前と理由)。アトラスの後ろに元の順で並ぶ。
    pub kept: Vec<(String, KeptReason)>,
    /// どのメッシュも使わないので取り除いたテクスチャの名前。
    pub removed: Vec<String>,
    /// 画素が同じ画像が先にあったので詰めなかったテクスチャ (名前と、代わりに使う画像の名前)。
    /// 描画順で最初に使われる順に並ぶ。
    pub deduplicated: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum AtlasError {
    #[error("texture `{name}` has no image data")]
    MissingTexture { name: String },
    #[error("texture `{name}` is not a readable PNG")]
    NotPng { name: String },
    #[error("mesh `{mesh}` refers to texture {texture}, which does not exist")]
    TextureOutOfRange { mesh: String, texture: u32 },
    #[error("max_size must be positive")]
    InvalidMaxSize,
    /// 画像の一辺が、デコードの上限 (`ugoe_format::image::ImageLimits`) を超える。
    #[error("texture `{name}` is {width}x{height}, larger than the limit {max}x{max}")]
    TextureTooLarge {
        name: String,
        width: u32,
        height: u32,
        max: u32,
    },
}

/// アトラス化したモデルとテクスチャ。
#[derive(Debug, Clone, PartialEq)]
pub struct Atlased {
    pub model: Model,
    pub textures: Textures,
    pub report: AtlasReport,
}

/// アトラス化した着せ替えのモジュールとテクスチャ。
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasedModule {
    pub module: Module,
    pub textures: Textures,
    pub report: AtlasReport,
}

/// `model` のテクスチャ (`textures` は名前 → PNG) をアトラスにまとめる。
///
/// 出力の `Model::textures` は、アトラス (`atlas-0`…) の後ろに、まとめずに残したテクスチャを元の順で並べる。
/// アトラスは、既定のパラメータでの描画順で最初に使われる順に画像を詰め、上限に収まらなくなったら次の
/// アトラスに移る (描画順で近いメッシュが同じアトラスに入るように)。
pub fn build_atlas(
    model: &Model,
    textures: &Textures,
    options: &AtlasOptions,
) -> Result<Atlased, AtlasError> {
    let order = first_use_order(model);
    let packed = pack_textures(&model.textures, &model.meshes, textures, options, order)?;
    let mut out = model.clone();
    out.textures = packed.refs;
    out.meshes = packed.meshes;
    Ok(Atlased {
        model: out,
        textures: packed.textures,
        report: packed.report,
    })
}

/// 着せ替えのモジュールのテクスチャ (`Module::textures`) を、モジュールの単位でアトラスにまとめる。
/// 規則は [`build_atlas`] と同じ。本体のテクスチャとは混ぜない。
///
/// モジュールは本体に付けないと描画順が決まらないので、画像は **メッシュの宣言順で** 最初に使われる順に詰める。
pub fn build_module_atlas(
    module: &Module,
    textures: &Textures,
    options: &AtlasOptions,
) -> Result<AtlasedModule, AtlasError> {
    let order = first_use_order_of(
        &module.meshes,
        module.textures.len(),
        0..module.meshes.len(),
    );
    let packed = pack_textures(&module.textures, &module.meshes, textures, options, order)?;
    let mut out = module.clone();
    out.textures = packed.refs;
    out.meshes = packed.meshes;
    Ok(AtlasedModule {
        module: out,
        textures: packed.textures,
        report: packed.report,
    })
}

/// [`pack_textures`] の結果。
struct Packed {
    refs: Vec<TextureRef>,
    meshes: Vec<Mesh>,
    textures: Textures,
    report: AtlasReport,
}

/// テクスチャの参照 `refs` と、それを番号で使うメッシュ `meshes` をアトラス化する。
/// `order_of_use` は画像を詰める順 (テクスチャの番号)。
fn pack_textures(
    refs: &[TextureRef],
    meshes: &[Mesh],
    textures: &Textures,
    options: &AtlasOptions,
    order_of_use: Vec<usize>,
) -> Result<Packed, AtlasError> {
    if options.max_size == 0 {
        return Err(AtlasError::InvalidMaxSize);
    }
    for mesh in meshes {
        if mesh.texture as usize >= refs.len() {
            return Err(AtlasError::TextureOutOfRange {
                mesh: mesh.id.as_str().to_owned(),
                texture: mesh.texture,
            });
        }
    }

    // 各テクスチャを使うメッシュの UV が 0〜1 に収まるか。
    let mut used = vec![false; refs.len()];
    let mut uv_in_range = vec![true; refs.len()];
    for mesh in meshes {
        let t = mesh.texture as usize;
        used[t] = true;
        uv_in_range[t] &= mesh
            .uvs
            .iter()
            .all(|uv| (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y));
    }

    let mut report = AtlasReport::default();
    let pad = options.padding;
    // まとめる画像 (元の番号・画素)。
    let mut images: Vec<Option<Image>> = vec![None; refs.len()];
    let mut kept = Vec::new();
    for (t, texture) in refs.iter().enumerate() {
        if !used[t] {
            report.removed.push(texture.name.clone());
            continue;
        }
        let png = textures
            .get(&texture.name)
            .ok_or_else(|| AtlasError::MissingTexture {
                name: texture.name.clone(),
            })?;
        if !uv_in_range[t] {
            kept.push(t);
            report
                .kept
                .push((texture.name.clone(), KeptReason::UvOutOfRange));
            continue;
        }
        if texture.normal_map.is_some() {
            kept.push(t);
            report
                .kept
                .push((texture.name.clone(), KeptReason::HasNormalMap));
            continue;
        }
        let image = decode_png(png, ImageLimits::default()).map_err(|error| match error {
            ImageError::TooLarge { width, height, max } => AtlasError::TextureTooLarge {
                name: texture.name.clone(),
                width,
                height,
                max,
            },
            _ => AtlasError::NotPng {
                name: texture.name.clone(),
            },
        })?;
        let (width, height, rgba) = (image.width, image.height, image.pixels);
        let fits = |size: u32| u64::from(size) + 2 * u64::from(pad) <= u64::from(options.max_size);
        if !fits(width) || !fits(height) {
            kept.push(t);
            report
                .kept
                .push((texture.name.clone(), KeptReason::TooLarge));
            continue;
        }
        images[t] = Some(Image {
            width,
            height,
            rgba,
        });
    }

    // 描画順で最初に使われる順に並べ、順に詰める。画素が同じ画像は最初のものだけを詰める。
    let mut order: Vec<usize> = Vec::new();
    let mut same_as: Vec<Option<usize>> = vec![None; refs.len()];
    for t in order_of_use {
        let Some(image) = images[t].as_ref() else {
            continue;
        };
        match order.iter().find(|&&u| images[u].as_ref() == Some(image)) {
            Some(&first) => {
                same_as[t] = Some(first);
                report
                    .deduplicated
                    .push((refs[t].name.clone(), refs[first].name.clone()));
            }
            None => order.push(t),
        }
    }
    let item = |t: usize| {
        let image = images[t].as_ref().map_or((0, 0), |i| (i.width, i.height));
        (image.0 + 2 * pad, image.1 + 2 * pad)
    };
    let mut pages: Vec<(Vec<usize>, Packing)> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut packing: Option<Packing> = None;
    for &t in &order {
        let mut candidate = current.clone();
        candidate.push(t);
        let sizes: Vec<_> = candidate.iter().map(|&t| item(t)).collect();
        match pack(&sizes, options.max_size) {
            Some(p) => {
                current = candidate;
                packing = Some(p);
            }
            None => {
                if let Some(p) = packing.take() {
                    pages.push((std::mem::take(&mut current), p));
                }
                // 1 枚だけなら必ず収まる (上で大きすぎるものは除いた)。
                packing = pack(&[item(t)], options.max_size);
                current = vec![t];
            }
        }
    }
    if let Some(p) = packing {
        pages.push((current, p));
    }

    // 出力のテクスチャ: アトラス → 残したもの。元の番号 → (新しい番号・画像を置いた位置)。
    let mut out_meshes = meshes.to_vec();
    let mut out_textures = Textures::new();
    let mut new_refs = Vec::new();
    let mut placement: Vec<Option<(u32, Placement)>> = vec![None; refs.len()];
    let kept_names: BTreeSet<String> = kept
        .iter()
        .map(|&t| refs[t].name.to_ascii_lowercase())
        .collect();
    let mut atlas_number = 0;
    for (members, packing) in &pages {
        let name = loop {
            let name = format!("{ATLAS_NAME_PREFIX}{atlas_number}");
            atlas_number += 1;
            if !kept_names.contains(&name) {
                break name;
            }
        };
        let (aw, ah) = (packing.width, packing.height);
        // アトラスは詰めた画像 (どれもデコードできた PNG) を覆う程度の大きさなので、積はあふれない (T0012)。
        let mut rgba = vec![0u8; aw as usize * ah as usize * 4];
        let index = u32::try_from(new_refs.len()).unwrap_or(u32::MAX);
        for (&t, &(x, y)) in members.iter().zip(&packing.positions) {
            let Some(image) = images[t].as_ref() else {
                continue;
            };
            blit_extruded(&mut rgba, aw, image, x, y, pad);
            placement[t] = Some((
                index,
                Placement {
                    x: x + pad,
                    y: y + pad,
                    width: image.width,
                    height: image.height,
                    atlas_width: aw,
                    atlas_height: ah,
                },
            ));
        }
        report.atlases.push((aw, ah));
        out_textures.insert(name.clone(), Arc::from(encode_rgba_png(aw, ah, &rgba)));
        new_refs.push(TextureRef {
            name,
            width: aw,
            height: ah,
            normal_map: None,
        });
    }
    for (t, first) in same_as.iter().enumerate() {
        if let Some(first) = *first {
            placement[t] = placement[first];
        }
    }
    report.packed = placement.iter().filter(|p| p.is_some()).count();
    let mut kept_index = vec![None; refs.len()];
    for &t in &kept {
        let texture = &refs[t];
        kept_index[t] = Some(u32::try_from(new_refs.len()).unwrap_or(u32::MAX));
        if let Some(png) = textures.get(&texture.name) {
            out_textures.insert(texture.name.clone(), png.clone());
        }
        if let Some(name) = &texture.normal_map
            && let Some(png) = textures.get(name)
        {
            out_textures.insert(name.clone(), png.clone());
        }
        new_refs.push(texture.clone());
    }
    for mesh in &mut out_meshes {
        let t = mesh.texture as usize;
        if let Some((index, place)) = placement[t] {
            mesh.texture = index;
            for uv in &mut mesh.uvs {
                *uv = place.map(*uv);
            }
        } else if let Some(index) = kept_index[t] {
            mesh.texture = index;
        }
    }

    Ok(Packed {
        refs: new_refs,
        meshes: out_meshes,
        textures: out_textures,
        report,
    })
}

impl Project {
    /// モデルとテクスチャを [`build_atlas`] でアトラス化したプロジェクト (書き出し用)。
    /// 物理・モーション・表情はそのまま。
    pub fn with_atlas(&self, options: &AtlasOptions) -> Result<(Self, AtlasReport), AtlasError> {
        let atlased = build_atlas(&self.model, &self.textures, options)?;
        let project = Self {
            model: atlased.model,
            textures: atlased.textures,
            meta: self.meta.clone(),
            physics: self.physics.clone(),
            motions: self.motions.clone(),
            expressions: self.expressions.clone(),
            // アトラス化したものは書き出し用の合成済みのモデルで、モジュールとしては保存しない。
            module: None,
        };
        Ok((project, atlased.report))
    }
}

/// 非乗算 RGBA8 の画像。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// アトラスの中で画像を置いた場所 (周りの複製を除いた本体の左上)。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placement {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) atlas_width: u32,
    pub(crate) atlas_height: u32,
}

impl Placement {
    /// 元の画像の UV をアトラスの UV に写す。
    pub(crate) fn map(&self, uv: ugoe_core::Vec2) -> ugoe_core::Vec2 {
        let axis = |u: f32, offset: u32, size: u32, total: u32| {
            ((f64::from(offset) + f64::from(u) * f64::from(size)) / f64::from(total)) as f32
        };
        ugoe_core::Vec2::new(
            axis(uv.x, self.x, self.width, self.atlas_width),
            axis(uv.y, self.y, self.height, self.atlas_height),
        )
    }

    /// アトラスの UV を元の画像の UV に戻す ([`Placement::map`] の逆。丸めの分だけずれ得る)。
    pub(crate) fn unmap(&self, uv: ugoe_core::Vec2) -> ugoe_core::Vec2 {
        let axis = |u: f32, offset: u32, size: u32, total: u32| {
            ((f64::from(u) * f64::from(total) - f64::from(offset)) / f64::from(size)) as f32
        };
        ugoe_core::Vec2::new(
            axis(uv.x, self.x, self.width, self.atlas_width),
            axis(uv.y, self.y, self.height, self.atlas_height),
        )
    }
}

/// 詰め方: アトラスの大きさと、各項目 (周りの複製を含む) の左上。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Packing {
    width: u32,
    height: u32,
    positions: Vec<(u32, u32)>,
}

/// テクスチャの番号を、既定のパラメータでの描画順で最初に使われる順に並べる。
/// どのメッシュも使わない番号は含めない。モデルをコンパイルできなければ宣言順を使う。
fn first_use_order(model: &Model) -> Vec<usize> {
    let mesh_order: Vec<usize> = match CompiledModel::compile(model.clone()) {
        Ok(compiled) => {
            let mut instance = ModelInstance::new(compiled);
            instance.update();
            instance
                .render_order()
                .iter()
                .map(|&i| i as usize)
                .collect()
        }
        Err(_) => (0..model.meshes.len()).collect(),
    };
    first_use_order_of(&model.meshes, model.textures.len(), mesh_order)
}

/// テクスチャの番号を、メッシュを `mesh_order` の順に見て最初に使われる順に並べる。
fn first_use_order_of(
    meshes: &[Mesh],
    texture_count: usize,
    mesh_order: impl IntoIterator<Item = usize>,
) -> Vec<usize> {
    let mut seen = vec![false; texture_count];
    let mut order = Vec::new();
    // 描画順に含まれないメッシュがあっても漏らさないように、宣言順で補う。
    for m in mesh_order.into_iter().chain(0..meshes.len()) {
        let Some(mesh) = meshes.get(m) else {
            continue;
        };
        let t = mesh.texture as usize;
        if t < seen.len() && !seen[t] {
            seen[t] = true;
            order.push(t);
        }
    }
    order
}

/// 大きさ `sizes` の長方形を `max_size` 四方に収まるように詰める。収まらなければ `None`。
///
/// 高さの降順 (同じなら元の順) に棚へ並べる。棚の幅の候補 (一番広い項目以上の 2 の冪と `max_size`) を
/// 試し、面積が最小 (同じなら長い辺が短いもの) になる詰め方を選ぶ。
fn pack(sizes: &[(u32, u32)], max_size: u32) -> Option<Packing> {
    let widest = sizes.iter().map(|s| s.0).max()?;
    let mut widths: Vec<u32> =
        std::iter::successors(Some(widest.next_power_of_two()), |w| w.checked_mul(2))
            .take_while(|&w| w < max_size)
            .collect();
    widths.push(max_size);
    widths
        .into_iter()
        .filter(|&w| w >= widest)
        .filter_map(|w| shelf_pack(sizes, w, max_size))
        .min_by_key(|p| {
            (
                u64::from(p.width) * u64::from(p.height),
                p.width.max(p.height),
            )
        })
}

/// 幅 `shelf_width` の棚に詰める。高さが `max_height` を超えれば `None`。
fn shelf_pack(sizes: &[(u32, u32)], shelf_width: u32, max_height: u32) -> Option<Packing> {
    let mut by_height: Vec<usize> = (0..sizes.len()).collect();
    by_height.sort_by_key(|&i| std::cmp::Reverse(sizes[i].1));
    let mut positions = vec![(0, 0); sizes.len()];
    let (mut x, mut y, mut shelf_height, mut width) = (0u32, 0u32, 0u32, 0u32);
    for i in by_height {
        let (w, h) = sizes[i];
        if x + w > shelf_width {
            y += shelf_height;
            x = 0;
            shelf_height = 0;
        }
        positions[i] = (x, y);
        x += w;
        width = width.max(x);
        shelf_height = shelf_height.max(h);
        if u64::from(y) + u64::from(shelf_height) > u64::from(max_height) {
            return None;
        }
    }
    Some(Packing {
        width,
        height: y + shelf_height,
        positions,
    })
}

/// `image` をアトラス (`atlas_width` 幅) の `(x, y)` から、周りに端の画素を `pad` 幅だけ複製して書く。
fn blit_extruded(rgba: &mut [u8], atlas_width: u32, image: &Image, x: u32, y: u32, pad: u32) {
    let (w, h) = (image.width, image.height);
    for dy in 0..h + 2 * pad {
        let sy = dy.saturating_sub(pad).min(h - 1);
        for dx in 0..w + 2 * pad {
            let sx = dx.saturating_sub(pad).min(w - 1);
            let src = (sy as usize * w as usize + sx as usize) * 4;
            let dst = ((y + dy) as usize * atlas_width as usize + (x + dx) as usize) * 4;
            rgba[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
        }
    }
}
