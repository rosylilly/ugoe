//! 法線マップの生成の提案 (`docs/ai-integration.md#法線マップの生成`、ADR 0015)。
//!
//! - [`propose_bulge_normal_maps`] は、テクスチャのアルファの形からふくらみを作る決定的な方法
//!   (`ugoe_rigging::bulge_normal_map`)。AI は使わない。
//! - [`propose_depth_normal_maps`] は、原画の深度推定 ([`DepthEstimator`]、振り向きの提案と共通) から、
//!   テクスチャの画素ごとに基準の姿のキャンバス座標で深度を引き、その勾配を法線にする。
//!
//! どちらも法線マップの画像を [`Command::SetNormalMap`] で置き、モデルのライティングが無効なら
//! [`Command::SetLighting`] で有効にする提案を返す。提案は複製したドキュメントに適用し、
//! `ugoe_model::validate` を通ることを確かめてから返す (ADR 0006)。

use std::collections::{BTreeMap, BTreeSet};

use ugoe_core::Vec2;
use ugoe_editor_core::Command;
use ugoe_editor_core::texture::{MAX_TEXTURE_NAME_LEN, Textures, decode_rgba_png, encode_rgba_png};
use ugoe_model::{MeshId, Model};
use ugoe_rigging::{BulgeOptions, HeightField, NormalMap, bulge_height};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::classify::Label;
use crate::depth::{DepthEstimator, DepthMap, DepthRequest};
use crate::turn::{Builder, PERCENTILE, percentiles};
use crate::{AiError, Proposal};

/// 深度から作る起伏の大きさの基準: 対象のメッシュが覆う範囲の幅に対する比
/// (振り向きの提案の頭の起伏と同じ。`docs/ai-integration.md#深度の決め方-ugoe_aiturn`)。
const DEPTH_RELIEF: f32 = 0.3;
/// メッシュの三角形が覆わない形の中の画素に、隣の画素の値を広げる回数の上限。
const FILL_PASSES: usize = 64;
/// 法線マップの画像の名前に付ける接尾辞。
const SUFFIX: &str = "_normal";

/// 法線マップを作るテクスチャと、テクスチャ (パーツ) ごとの設定。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormalMapOptions<T> {
    /// 既定の設定。
    pub default: T,
    /// テクスチャの名前ごとの設定 (既定を置き換える)。パーツごとにふくらみの強さを変えるのに使う。
    pub per_texture: BTreeMap<String, T>,
    /// 法線マップを作るテクスチャの名前。空なら、メッシュ (マスク専用と、ラベルが `background` の
    /// ものを除く) が使うすべてのテクスチャ。
    pub textures: Vec<String>,
    /// すでに法線マップを持つテクスチャも作り直すか。`false` なら飛ばす。
    pub replace_existing: bool,
}

impl<T> NormalMapOptions<T> {
    fn for_texture(&self, name: &str) -> &T {
        self.per_texture.get(name).unwrap_or(&self.default)
    }
}

/// 深度推定から法線マップを作る設定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthNormalOptions {
    /// 起伏の強さ。正規化した深度 (0〜1) に、対象のメッシュが覆う範囲の幅 × 0.3 × `strength` を
    /// 掛けてキャンバスのピクセル単位の高さにする。0 以上。
    pub strength: f32,
    /// 輪郭の丸み ([`BulgeOptions::strength`]) を足す強さ。深度推定はパーツの輪郭で落ちないので、
    /// 縁を少し丸めると立体に見えやすい。0 なら足さない。
    pub rim: f32,
    /// 高さをならすガウスぼかしの標準偏差 (テクスチャの画素単位)。
    pub smoothing: f32,
    /// 形とみなすアルファの下限。
    pub alpha_threshold: u8,
}

impl Default for DepthNormalOptions {
    fn default() -> Self {
        Self {
            strength: 1.0,
            rim: 0.3,
            smoothing: 1.5,
            alpha_threshold: 128,
        }
    }
}

/// アルファの形から、テクスチャごとにふくらみの法線マップを作る提案 (決定的。AI は使わない)。
///
/// `textures` はテクスチャの画像 (`Document::textures`)。`labels` はメッシュの意味ラベルで、
/// 対象を選ぶときに背景を除くのに使う (なくてよい)。
pub fn propose_bulge_normal_maps(
    model: &Model,
    textures: &Textures,
    labels: &BTreeMap<MeshId, Label>,
    options: &NormalMapOptions<BulgeOptions>,
) -> Result<Proposal, AiError> {
    let targets = targets(model, labels, options)?;
    let mut maps = Vec::new();
    for (index, name) in &targets {
        let image = decode(textures, name)?;
        let map = ugoe_rigging::bulge_normal_map(
            image.width,
            image.height,
            &image.rgba,
            options.for_texture(name),
        )
        .map_err(|e| invalid(format!("texture `{name}`: {e}")))?;
        maps.push((*index, map));
    }
    let lines = maps
        .iter()
        .map(|(index, _)| {
            let name = &model.textures[*index].name;
            let s = options.for_texture(name).strength;
            format!("- `{name}`: ふくらみ {s}")
        })
        .collect::<Vec<_>>();
    build(
        model,
        maps,
        "アルファの形からの法線マップ",
        format!(
            "テクスチャのアルファの形から、輪郭からの距離に応じてふくらんだ高さを作り、その勾配を法線にした。\n{}",
            lines.join("\n")
        ),
    )
}

/// 原画の深度推定から、テクスチャごとに法線マップを作る提案。
///
/// 推定器 `estimator` に `request` (原画。キャンバス全体に対応する) を送り、返った深度マップを
/// [`plan_depth_normal_maps`] で法線マップにする。
pub fn propose_depth_normal_maps(
    model: &Model,
    textures: &Textures,
    estimator: &dyn DepthEstimator,
    request: &DepthRequest,
    labels: &BTreeMap<MeshId, Label>,
    options: &NormalMapOptions<DepthNormalOptions>,
) -> Result<Proposal, AiError> {
    let map = estimator.estimate(request)?;
    plan_depth_normal_maps(model, textures, &map, labels, options)
}

/// 深度マップから法線マップの提案を作る (推定器を呼ばない部分)。
///
/// 1. 対象のテクスチャの画素ごとに、それを覆うメッシュの三角形 (UV 空間) から基準の姿のキャンバス座標を
///    求め、深度マップを引く。三角形が覆わない形の中の画素には、隣の画素の値を広げる。
/// 2. 対象のすべての画素の深度の 2% と 98% の分位点を 0 と 1 に写す (テクスチャごとではなく全体で
///    正規化するので、小さなパーツの起伏が大きくならない)。
/// 3. 起伏 (覆う範囲の幅 × 0.3 × `strength`) を掛け、テクスチャの画素単位に直して高さにする。
///    `rim` の分だけアルファの形からのふくらみを足し、ならして勾配から法線を作る。
pub fn plan_depth_normal_maps(
    model: &Model,
    textures: &Textures,
    map: &DepthMap,
    labels: &BTreeMap<MeshId, Label>,
    options: &NormalMapOptions<DepthNormalOptions>,
) -> Result<Proposal, AiError> {
    map.check()?;
    let targets = targets(model, labels, options)?;
    let canvas = Vec2::new(model.meta.canvas.width, model.meta.canvas.height);
    if !(canvas.x > 0.0 && canvas.y > 0.0) {
        return Err(invalid("the canvas is empty".into()));
    }
    let instance = evaluate(model)?;

    let mut layers = Vec::new();
    for (index, name) in &targets {
        let o = options.for_texture(name);
        check_depth_options(name, o)?;
        let image = decode(textures, name)?;
        let layer = TexelCanvas::rasterize(model, &instance, labels, *index as u32, &image, o);
        layers.push((*index, image, layer));
    }

    // 全体で正規化する。
    let (mut min, mut max) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    let mut raw = Vec::new();
    for (_, _, layer) in &layers {
        for p in layer.positions.iter().flatten() {
            min = min.min(*p);
            max = max.max(*p);
            raw.push(map.sample(p.x / canvas.x, p.y / canvas.y));
        }
    }
    if raw.is_empty() {
        return Err(invalid(
            "no mesh covers the textures to make normal maps for".into(),
        ));
    }
    let (lo, hi) = percentiles(raw, PERCENTILE);
    let normalize = |v: f32| {
        if hi - lo <= f32::EPSILON * hi.abs().max(1.0) {
            0.0
        } else {
            ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
        }
    };
    let width = (max.x - min.x).max(1.0);

    let mut maps = Vec::new();
    let mut lines = Vec::new();
    for (index, image, layer) in layers {
        let name = &model.textures[index].name;
        let o = options.for_texture(name);
        // キャンバスのピクセル単位の起伏を、テクスチャの画素単位に直す。
        let relief = width * DEPTH_RELIEF * o.strength / layer.canvas_per_texel;
        let depth: Vec<Option<f32>> = layer
            .positions
            .iter()
            .map(|p| p.map(|p| normalize(map.sample(p.x / canvas.x, p.y / canvas.y)) * relief))
            .collect();
        let values = fill(
            depth,
            &layer.mask,
            image.width as usize,
            image.height as usize,
        );
        let mut field = HeightField {
            width: image.width,
            height: image.height,
            values,
            mask: layer.mask,
        };
        if o.rim > 0.0 {
            let rim = bulge_height(
                image.width,
                image.height,
                &image.rgba,
                &BulgeOptions {
                    strength: o.rim,
                    radius: None,
                    smoothing: 0.0,
                    alpha_threshold: o.alpha_threshold,
                },
            )
            .map_err(|e| invalid(format!("texture `{name}`: {e}")))?;
            field
                .add(&rim)
                .map_err(|e| invalid(format!("texture `{name}`: {e}")))?;
        }
        field.smooth(o.smoothing);
        lines.push(format!(
            "- `{name}`: 起伏 {:.1} px (キャンバス)・縁の丸み {}",
            relief * layer.canvas_per_texel,
            o.rim
        ));
        maps.push((index, field.to_normal_map()));
    }
    build(
        model,
        maps,
        "深度推定からの法線マップ",
        format!(
            "原画から推定した深度をテクスチャの画素ごとに基準の姿の位置で引き、全体で正規化した起伏の勾配を法線にした。\n{}",
            lines.join("\n")
        ),
    )
}

/// デコードしたテクスチャ (非乗算 RGBA8)。
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn decode(textures: &Textures, name: &str) -> Result<Image, AiError> {
    let png = textures
        .get(name)
        .ok_or_else(|| invalid(format!("texture `{name}` has no image")))?;
    let (width, height, rgba) = decode_rgba_png(png)
        .ok_or_else(|| invalid(format!("texture `{name}` is not a readable PNG")))?;
    Ok(Image {
        width,
        height,
        rgba,
    })
}

/// 法線マップを作るテクスチャ (番号と名前)。
fn targets<T>(
    model: &Model,
    labels: &BTreeMap<MeshId, Label>,
    options: &NormalMapOptions<T>,
) -> Result<Vec<(usize, String)>, AiError> {
    let find = |name: &str| {
        model
            .textures
            .iter()
            .position(|t| t.name == name)
            .ok_or_else(|| invalid(format!("texture `{name}` not found")))
    };
    for name in options.per_texture.keys() {
        find(name)?;
    }
    let indices: BTreeSet<usize> = if options.textures.is_empty() {
        model
            .meshes
            .iter()
            .filter(|m| !m.mask_only && labels.get(&m.id) != Some(&Label::Background))
            .filter_map(|m| usize::try_from(m.texture).ok())
            .filter(|&i| i < model.textures.len())
            .collect()
    } else {
        options
            .textures
            .iter()
            .map(|name| find(name))
            .collect::<Result<_, _>>()?
    };
    let targets: Vec<_> = indices
        .into_iter()
        .filter(|&i| options.replace_existing || model.textures[i].normal_map.is_none())
        .map(|i| (i, model.textures[i].name.clone()))
        .collect();
    if targets.is_empty() {
        return Err(invalid(
            "there is no texture to make a normal map for".into(),
        ));
    }
    Ok(targets)
}

/// 法線マップを置くコマンドを組み、複製に適用して検証する。
fn build(
    model: &Model,
    maps: Vec<(usize, NormalMap)>,
    title: &str,
    rationale: String,
) -> Result<Proposal, AiError> {
    let mut b = Builder::new(model);
    let mut taken: BTreeSet<String> = model
        .textures
        .iter()
        .flat_map(|t| std::iter::once(&t.name).chain(t.normal_map.as_ref()))
        .map(|n| n.to_ascii_lowercase())
        .collect();
    for (index, map) in maps {
        let texture = &model.textures[index];
        // いまの法線マップをこのテクスチャだけが指していれば、同じ名前の画像を差し替える
        // (ほかのテクスチャと共有していれば、それらを変えないように新しい名前にする)。
        let own = texture.normal_map.clone().filter(|n| {
            model
                .textures
                .iter()
                .filter(|t| t.normal_map.as_ref() == Some(n))
                .count()
                == 1
        });
        let name = match own {
            Some(name) => name,
            None => unique_name(&texture.name, &mut taken),
        };
        b.apply(Command::SetNormalMap {
            texture: texture.name.clone(),
            normal_map: Some(name),
            png: Some(encode_rgba_png(map.width, map.height, &map.rgba)),
        })?;
    }
    if !model.lighting {
        b.apply(Command::SetLighting { enabled: true })?;
    }
    b.validate()?;
    Ok(Proposal {
        title: title.into(),
        rationale,
        commands: b.commands,
    })
}

/// `<texture>_normal` (重なれば `_normal2`・`_normal3`…)。テクスチャの名前の長さの上限に収める。
fn unique_name(texture: &str, taken: &mut BTreeSet<String>) -> String {
    for n in 1.. {
        let suffix = if n == 1 {
            SUFFIX.to_owned()
        } else {
            format!("{SUFFIX}{n}")
        };
        // テクスチャの名前は ASCII なので、バイトで切っても文字の途中にならない。
        let base = &texture[..texture.len().min(MAX_TEXTURE_NAME_LEN - suffix.len())];
        let name = format!("{base}{suffix}");
        if taken.insert(name.to_ascii_lowercase()) {
            return name;
        }
    }
    unreachable!("the loop only ends by returning")
}

fn check_depth_options(name: &str, o: &DepthNormalOptions) -> Result<(), AiError> {
    for (field, value) in [
        ("strength", o.strength),
        ("rim", o.rim),
        ("smoothing", o.smoothing),
    ] {
        if !(value.is_finite() && value >= 0.0) {
            return Err(invalid(format!(
                "texture `{name}`: invalid option {field} = {value}"
            )));
        }
    }
    Ok(())
}

/// 基準の姿 (パラメータが既定値) を評価する。
fn evaluate(model: &Model) -> Result<ModelInstance, AiError> {
    let compiled = CompiledModel::compile(model.clone())
        .map_err(|e| invalid(format!("the model cannot be evaluated: {e}")))?;
    let mut instance = ModelInstance::new(compiled);
    instance.update();
    Ok(instance)
}

/// テクスチャの画素ごとの基準の姿のキャンバス座標。
struct TexelCanvas {
    /// 形の中で、メッシュの三角形が覆う画素の位置。
    positions: Vec<Option<Vec2>>,
    /// 形の中の画素か。
    mask: Vec<bool>,
    /// テクスチャの 1 画素がキャンバスの何ピクセルに当たるか (三角形の面積で重み付けした平均)。
    canvas_per_texel: f32,
}

impl TexelCanvas {
    fn rasterize(
        model: &Model,
        instance: &ModelInstance,
        labels: &BTreeMap<MeshId, Label>,
        texture: u32,
        image: &Image,
        options: &DepthNormalOptions,
    ) -> Self {
        let (w, h) = (image.width as usize, image.height as usize);
        let size = Vec2::new(image.width as f32, image.height as f32);
        let threshold = options.alpha_threshold.max(1);
        let mask: Vec<bool> = image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3] >= threshold)
            .collect();
        let mut positions = vec![None; w * h];
        let (mut canvas_area, mut texel_area) = (0.0f32, 0.0f32);
        let compiled = instance.compiled();
        for mesh in &model.meshes {
            if mesh.texture != texture
                || mesh.mask_only
                || labels.get(&mesh.id) == Some(&Label::Background)
            {
                continue;
            }
            let Some(state) = compiled
                .mesh_index(mesh.id.as_str())
                .and_then(|i| instance.drawables().get(i))
            else {
                continue;
            };
            for tri in mesh.indices.as_chunks::<3>().0 {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(usize::from);
                let (Some(&ua), Some(&ub), Some(&uc)) =
                    (mesh.uvs.get(a), mesh.uvs.get(b), mesh.uvs.get(c))
                else {
                    continue;
                };
                let (Some(&pa), Some(&pb), Some(&pc)) = (
                    state.positions.get(a),
                    state.positions.get(b),
                    state.positions.get(c),
                ) else {
                    continue;
                };
                let (ta, tb, tc) = (ua * size, ub * size, uc * size);
                let area = cross(tb - ta, tc - ta);
                if area.abs() <= f32::EPSILON {
                    continue;
                }
                texel_area += area.abs();
                canvas_area += cross(pb - pa, pc - pa).abs();
                // 画素の中心 `(x + 0.5, y + 0.5)` が外接矩形に入る範囲。
                let lo = ta.min(tb).min(tc);
                let hi = ta.max(tb).max(tc);
                let first = |v: f32| (v - 0.5).ceil().max(0.0) as usize;
                let end =
                    |v: f32, n: usize| ((v - 0.5).floor() + 1.0).clamp(0.0, n as f32) as usize;
                for y in first(lo.y)..end(hi.y, h) {
                    for x in first(lo.x)..end(hi.x, w) {
                        let i = y * w + x;
                        if !mask[i] || positions[i].is_some() {
                            continue;
                        }
                        let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                        // 重心座標。辺の上 (丸めの誤差を含む) も覆うものとする。
                        let l1 = cross(p - ta, tc - ta) / cross(tb - ta, tc - ta);
                        let l2 = cross(tb - ta, p - ta) / cross(tb - ta, tc - ta);
                        let l0 = 1.0 - l1 - l2;
                        const EPS: f32 = -1e-4;
                        if l0 >= EPS && l1 >= EPS && l2 >= EPS {
                            positions[i] = Some(pa * l0 + pb * l1 + pc * l2);
                        }
                    }
                }
            }
        }
        let canvas_per_texel = if texel_area > 0.0 && canvas_area > 0.0 {
            (canvas_area / texel_area).sqrt()
        } else {
            1.0
        };
        Self {
            positions,
            mask,
            canvas_per_texel,
        }
    }
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// 値のない形の中の画素に、隣 (上下左右) の値の平均を広げる ([`FILL_PASSES`] 回まで)。
/// それでも値のない画素は、値のある画素の最小値 (なければ 0) にする。
fn fill(mut values: Vec<Option<f32>>, mask: &[bool], w: usize, h: usize) -> Vec<f32> {
    for _ in 0..FILL_PASSES {
        let mut next = values.clone();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !mask[i] || values[i].is_some() {
                    continue;
                }
                let neighbors = [
                    x.checked_sub(1).map(|x| y * w + x),
                    (x + 1 < w).then_some(i + 1),
                    y.checked_sub(1).map(|y| y * w + x),
                    (y + 1 < h).then_some(i + w),
                ];
                let (sum, n) = neighbors
                    .into_iter()
                    .flatten()
                    .filter_map(|j| values[j])
                    .fold((0.0, 0), |(s, n), v| (s + v, n + 1));
                if n > 0 {
                    next[i] = Some(sum / n as f32);
                    changed = true;
                }
            }
        }
        values = next;
        if !changed {
            break;
        }
    }
    let floor = values
        .iter()
        .flatten()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let floor = if floor.is_finite() { floor } else { 0.0 };
    values
        .into_iter()
        .zip(mask)
        .map(|(v, &m)| if m { v.unwrap_or(floor) } else { 0.0 })
        .collect()
}

fn invalid(message: String) -> AiError {
    AiError::InvalidRequest(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_avoid_existing_textures_and_normal_maps() {
        let mut taken: BTreeSet<String> = ["face_normal".to_owned()].into();
        assert_eq!(unique_name("Face", &mut taken), "Face_normal2");
        assert_eq!(unique_name("Face", &mut taken), "Face_normal3");
        assert_eq!(unique_name("eye", &mut taken), "eye_normal");
        let long = "a".repeat(MAX_TEXTURE_NAME_LEN);
        let name = unique_name(&long, &mut taken);
        assert_eq!(name.len(), MAX_TEXTURE_NAME_LEN);
        assert!(name.ends_with(SUFFIX));
    }

    #[test]
    fn fill_spreads_values_into_uncovered_pixels() {
        let values = vec![Some(1.0), None, None, None];
        let mask = [true, true, true, false];
        assert_eq!(fill(values, &mask, 4, 1), vec![1.0, 1.0, 1.0, 0.0]);
        // 何も覆っていなければ 0。
        assert_eq!(fill(vec![None; 2], &[true, true], 2, 1), vec![0.0, 0.0]);
    }
}
