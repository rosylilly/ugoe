//! 法線マップの生成の提案 (`docs/ai-integration.md#法線マップの生成`、ADR 0015)。
//!
//! - [`propose_bulge_normal_maps`] は、テクスチャのアルファの形からふくらみを作る決定的な方法。AI は使わない。
//!   意味ラベルから部位のまとまり ([`SurfaceGroup`]。顔と目口鼻眉・首と体と腕・髪など) を決め、まとまりの形を
//!   基準の姿のキャンバスに並べて 1 つの形としてふくらませる (`ugoe_rigging::group_bulge_normal_maps`)。
//!   ラベルのないテクスチャは単独でふくらませる (`ugoe_rigging::bulge_normal_map`)。
//! - [`propose_depth_normal_maps`] は、原画の深度推定 ([`DepthEstimator`]、振り向きの提案と共通) から、
//!   テクスチャの画素ごとに基準の姿のキャンバス座標で深度を引き、その勾配を法線にする。
//!
//! どちらも法線マップの画像を [`Command::SetNormalMap`] で置き、モデルのライティングが無効なら
//! [`Command::SetLighting`] で有効にする提案を返す。提案は複製したドキュメントに適用し、
//! `ugoe_model::validate` を通ることを確かめてから返す (ADR 0006)。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;
use ugoe_editor_core::Command;
use ugoe_editor_core::texture::{MAX_TEXTURE_NAME_LEN, Textures, decode_rgba_png, encode_rgba_png};
use ugoe_model::{MeshId, Model};
use ugoe_rigging::{
    BulgeOptions, EDGE_PASSES, HeightField, NormalMap, PlacedTexture, bulge_height, fill_missing,
    group_bulge_normal_maps,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::classify::Label;
use crate::depth::{DepthEstimator, DepthMap, DepthRequest};
use crate::turn::{Builder, PERCENTILE, percentiles};
use crate::{AiError, Proposal};

/// 深度から作る起伏の大きさの基準: 対象のメッシュが覆う範囲の幅に対する比
/// (振り向きの提案の頭の起伏と同じ。`docs/ai-integration.md#深度の決め方-ugoe_aiturn`)。
const DEPTH_RELIEF: f32 = 0.3;
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

/// 法線マップを作るときの部位のまとまり (`docs/ai-integration.md#まとまりでふくらませる`)。
///
/// 同じまとまりのテクスチャは、基準の姿のキャンバスに並べた形を 1 つのふくらみにする。顔の上の目・口は顔の面の
/// 向きを受け (平らに浮かない)、首と体・肩の上の腕は境目でつながる (T0707)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceGroup {
    /// 顔と、顔の上に描く部位 (鼻・頬・目・眉・口)。
    Face,
    /// 首・体・手足と服。形には顔 (`face`) も加える (首が形の上端として上を向かないように)。
    Body,
    HairFront,
    HairSide,
    HairBack,
    Ear,
    Earwear,
    Headwear,
    Tail,
    /// まとめない (眼鏡・その他の装飾・分からないもの)。テクスチャごとに単独でふくらませる。
    Alone,
}

impl SurfaceGroup {
    /// ラベルのまとまり。`background` は法線マップを作らない (`None`)。
    pub const fn of(label: Label) -> Option<Self> {
        use Label::*;
        Some(match label {
            Face | Nose | Cheek | Eye | EyeWhite | Iris | EyeHighlight | EyelashUpper
            | EyelashLower | EyeClosed | Eyebrow | Mouth | MouthOpen | MouthClosed | MouthInner
            | Teeth | Tongue => Self::Face,
            Neck | Body | Arm | Hand | Leg | Foot | Topwear | Bottomwear | Legwear | Footwear
            | Handwear | Neckwear => Self::Body,
            HairFront => Self::HairFront,
            HairSide => Self::HairSide,
            HairBack => Self::HairBack,
            Ear => Self::Ear,
            Earwear => Self::Earwear,
            Headwear => Self::Headwear,
            Tail => Self::Tail,
            // 眼鏡のつるは顔の外に出るので、顔の形に加えない。
            Eyewear | Accessory | Other => Self::Alone,
            Background => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Body => "body",
            Self::HairFront => "hair_front",
            Self::HairSide => "hair_side",
            Self::HairBack => "hair_back",
            Self::Ear => "ear",
            Self::Earwear => "earwear",
            Self::Headwear => "headwear",
            Self::Tail => "tail",
            Self::Alone => "alone",
        }
    }

    /// ラベルが `self` の形にだけ加わるか (法線マップはラベルのまとまりで作る)。
    const fn takes_shape_of(self, label: Label) -> bool {
        matches!((self, label), (Self::Body, Label::Face))
    }

    /// 既定の設定: 強さ (高さと幅の比)・縁だけを丸める幅 (まとまりの形の幅に対する比)・ならし (格子の画素)。
    /// 顔と体のような大きな部位は弱く、髪の房・耳のような細い部位は強めにする (T0707 の見本の値)。
    const fn defaults(self) -> (f32, Option<f32>, f32) {
        match self {
            Self::Face => (0.35, Some(FACE_EDGE), 6.0),
            Self::Body => (0.3, None, 6.0),
            Self::HairFront | Self::HairSide | Self::HairBack | Self::Tail => (0.5, None, 3.0),
            Self::Ear | Self::Headwear => (0.5, None, 2.0),
            Self::Earwear => (0.6, None, 1.0),
            Self::Alone => (0.5, None, 1.0),
        }
    }
}

/// 顔のまとまりで、縁だけを丸める幅の、形の幅に対する比 (見本のアバターの顔の形の幅 約 230 画素で 48 画素)。
const FACE_EDGE: f32 = 0.21;

/// アルファの形からの法線マップの設定 ([`propose_bulge_normal_maps`])。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BulgeNormalMapOptions {
    /// ラベルのないテクスチャ (単独でふくらませる) の設定。
    pub default: BulgeOptions,
    /// まとまりごとの設定 (既定の設定を置き換える。`radius`・`smoothing` はまとまりの格子の画素)。
    pub per_group: BTreeMap<SurfaceGroup, BulgeOptions>,
    /// テクスチャの名前ごとの設定。そのテクスチャはまとまりから外し、単独でこの設定でふくらませる。
    pub per_texture: BTreeMap<String, BulgeOptions>,
    /// 法線マップを作るテクスチャの名前。空なら、メッシュ (マスク専用と、ラベルが `background` の
    /// ものを除く) が使うすべてのテクスチャ。まとまりのほかのテクスチャは、ここになくても形に加わる。
    pub textures: Vec<String>,
    /// すでに法線マップを持つテクスチャも作り直すか。`false` なら飛ばす。
    pub replace_existing: bool,
}

/// テクスチャ 1 枚の法線マップのまとまり ([`normal_map_group`])。
#[derive(Debug, Clone, PartialEq)]
pub struct NormalMapGroup {
    /// まとまり。ラベルがない・単独の設定があるなら `None` (単独でふくらませる)。
    pub group: Option<SurfaceGroup>,
    /// いっしょにふくらませるテクスチャ (このテクスチャを含む。形にだけ加えるものは含めない)。
    pub textures: Vec<String>,
    /// 形にだけ加えるテクスチャ。
    pub shape_only: Vec<String>,
    /// 使う設定 (まとまりの既定か、置き換えたもの)。
    pub options: BulgeOptions,
}

/// アルファの形から、部位のまとまりごとにふくらみの法線マップを作る提案 (決定的。AI は使わない)。
///
/// `textures` はテクスチャの画像 (`Document::textures`)。`labels` はメッシュの意味ラベルで、まとまり
/// ([`SurfaceGroup`]) を決め、対象を選ぶときに背景を除くのに使う。ラベルのないテクスチャは単独でふくらませる。
/// 1 枚のテクスチャを使うメッシュのラベルが違えば、モデルで先のメッシュのラベルを使う。
pub fn propose_bulge_normal_maps(
    model: &Model,
    textures: &Textures,
    labels: &BTreeMap<MeshId, Label>,
    options: &BulgeNormalMapOptions,
) -> Result<Proposal, AiError> {
    let targets = targets(
        model,
        labels,
        &options.textures,
        options.per_texture.keys(),
        options.replace_existing,
    )?;
    let mut planner = GroupPlanner::new(model, textures, labels, options);
    let mut maps = Vec::new();
    let mut lines = Vec::new();
    let mut done = BTreeSet::new();
    for (index, _) in &targets {
        if done.contains(index) {
            continue;
        }
        let group = planner.group_of(*index)?;
        let wanted: Vec<usize> = group
            .members
            .iter()
            .copied()
            .filter(|i| targets.iter().any(|(t, _)| t == i))
            .collect();
        let made = planner.make(&group)?;
        for (i, map) in group.members.iter().zip(made) {
            if wanted.contains(i) {
                done.insert(*i);
                maps.push((*i, map));
            }
        }
        lines.push(group_line(model, &group, &wanted));
    }
    build(
        model,
        maps,
        "アルファの形からの法線マップ",
        format!(
            "意味ラベルから部位のまとまりを決め、まとまりのテクスチャのアルファの形を基準の姿のキャンバスに並べて 1 つの形としてふくらませ、\
             その高さの勾配を法線にした (ラベルのないテクスチャは単独で)。\n{}",
            lines.join("\n")
        ),
    )
}

/// テクスチャ `texture` をどのまとまりで、どの設定でふくらませるか ([`propose_bulge_normal_maps`] と同じ規則)。
/// エディタで設定の既定値といっしょにふくらむテクスチャを示すのに使う。
pub fn normal_map_group(
    model: &Model,
    textures: &Textures,
    labels: &BTreeMap<MeshId, Label>,
    options: &BulgeNormalMapOptions,
    texture: &str,
) -> Result<NormalMapGroup, AiError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == texture)
        .ok_or_else(|| invalid(format!("texture `{texture}` not found")))?;
    let mut planner = GroupPlanner::new(model, textures, labels, options);
    let group = planner.group_of(index)?;
    let names = |list: &[usize]| {
        list.iter()
            .map(|&i| model.textures[i].name.clone())
            .collect()
    };
    Ok(NormalMapGroup {
        group: group.group,
        textures: names(&group.members),
        shape_only: names(&group.shape_only),
        options: group.options,
    })
}

/// 1 つのまとまりの計画: メンバー・形にだけ加えるもの (テクスチャの番号) と設定。
struct PlannedGroup {
    group: Option<SurfaceGroup>,
    members: Vec<usize>,
    shape_only: Vec<usize>,
    options: BulgeOptions,
}

/// まとまりを決め、テクスチャの画素をキャンバスに置いて法線マップを作る (画像と置いた位置を覚えておく)。
struct GroupPlanner<'a> {
    model: &'a Model,
    textures: &'a Textures,
    labels: &'a BTreeMap<MeshId, Label>,
    options: &'a BulgeNormalMapOptions,
    /// テクスチャごとのまとまり (ラベルのないもの・背景だけが使うものは入らない)。
    groups: BTreeMap<usize, (SurfaceGroup, Label)>,
    instance: Option<ModelInstance>,
    placed: BTreeMap<usize, (Image, TexelCanvas)>,
}

impl<'a> GroupPlanner<'a> {
    fn new(
        model: &'a Model,
        textures: &'a Textures,
        labels: &'a BTreeMap<MeshId, Label>,
        options: &'a BulgeNormalMapOptions,
    ) -> Self {
        let mut groups = BTreeMap::new();
        for mesh in &model.meshes {
            let Some(&label) = labels.get(&mesh.id) else {
                continue;
            };
            let Ok(index) = usize::try_from(mesh.texture) else {
                continue;
            };
            if mesh.mask_only || index >= model.textures.len() {
                continue;
            }
            if let Some(group) = SurfaceGroup::of(label) {
                groups.entry(index).or_insert((group, label));
            }
        }
        Self {
            model,
            textures,
            labels,
            options,
            groups,
            instance: None,
            placed: BTreeMap::new(),
        }
    }

    fn group_of(&mut self, index: usize) -> Result<PlannedGroup, AiError> {
        let name = &self.model.textures[index].name;
        let alone = |options: BulgeOptions| PlannedGroup {
            group: None,
            members: vec![index],
            shape_only: Vec::new(),
            options,
        };
        if let Some(o) = self.options.per_texture.get(name) {
            return Ok(alone(*o));
        }
        let Some(&(group, _)) = self.groups.get(&index) else {
            return Ok(alone(self.options.default));
        };
        if group == SurfaceGroup::Alone {
            let options = self.options.per_group.get(&group).copied();
            let mut planned = alone(options.unwrap_or_else(|| resolve(group, 0.0)));
            planned.group = Some(group);
            return Ok(planned);
        }
        let free = |i: &usize| {
            !self
                .options
                .per_texture
                .contains_key(&self.model.textures[*i].name)
        };
        let members: Vec<usize> = self
            .groups
            .iter()
            .filter(|(i, (g, _))| *g == group && free(i))
            .map(|(i, _)| *i)
            .collect();
        let shape_only: Vec<usize> = self
            .groups
            .iter()
            .filter(|(i, (_, l))| group.takes_shape_of(*l) && free(i))
            .map(|(i, _)| *i)
            .collect();
        let options = match self.options.per_group.get(&group) {
            Some(o) => *o,
            None => {
                // 縁の幅は形の幅に比べて決めるので、置いてから測る。
                let mut width = 0.0;
                if group.defaults().1.is_some() {
                    let (lo, hi, cell) = self.extent(&members)?;
                    width = (hi - lo) / cell;
                }
                resolve(group, width)
            }
        };
        Ok(PlannedGroup {
            group: Some(group),
            members,
            shape_only,
            options,
        })
    }

    /// メンバーの形の中の画素が覆うキャンバスの x の範囲と、最も細かいテクスチャの 1 画素の大きさ。
    fn extent(&mut self, members: &[usize]) -> Result<(f32, f32, f32), AiError> {
        let (mut lo, mut hi, mut cell) = (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY);
        for &i in members {
            let (_, canvas) = self.place(i)?;
            cell = cell.min(canvas.canvas_per_texel);
            for p in canvas.positions.iter().flatten() {
                lo = lo.min(p.x);
                hi = hi.max(p.x);
            }
        }
        if lo > hi {
            return Ok((0.0, 0.0, 1.0));
        }
        Ok((lo, hi, cell))
    }

    fn place(&mut self, index: usize) -> Result<&(Image, TexelCanvas), AiError> {
        if !self.placed.contains_key(&index) {
            if self.instance.is_none() {
                self.instance = Some(evaluate(self.model)?);
            }
            let name = &self.model.textures[index].name;
            let image = decode(self.textures, name)?;
            let instance = self
                .instance
                .as_ref()
                .ok_or_else(|| invalid("no instance".into()))?;
            let canvas = TexelCanvas::rasterize(
                self.model,
                instance,
                self.labels,
                index as u32,
                &image,
                BulgeOptions::default().alpha_threshold,
            );
            self.placed.insert(index, (image, canvas));
        }
        self.placed
            .get(&index)
            .ok_or_else(|| invalid("texture was not placed".into()))
    }

    /// まとまりのメンバーの法線マップ (メンバーの並び)。
    fn make(&mut self, group: &PlannedGroup) -> Result<Vec<NormalMap>, AiError> {
        if group.group.is_none() || group.group == Some(SurfaceGroup::Alone) {
            // 単独: キャンバスに置かずテクスチャの画素でふくらませる (メッシュが覆わなくても作れる)。
            let index = group.members[0];
            let name = &self.model.textures[index].name;
            let image = decode(self.textures, name)?;
            let mut field = bulge_height(image.width, image.height, &image.rgba, &group.options)
                .map_err(|e| invalid(format!("texture `{name}`: {e}")))?;
            field.smooth(group.options.smoothing);
            let mut map = field.to_normal_map();
            map.extend_edges(&field.mask, EDGE_PASSES);
            return Ok(vec![map]);
        }
        for &i in group.members.iter().chain(&group.shape_only) {
            self.place(i)?;
        }
        let placed = |list: &[usize]| -> Vec<PlacedTexture<'_>> {
            list.iter()
                .filter_map(|i| self.placed.get(i))
                .map(|(image, canvas)| PlacedTexture {
                    width: image.width,
                    height: image.height,
                    rgba: &image.rgba,
                    positions: &canvas.positions,
                    canvas_per_texel: canvas.canvas_per_texel,
                })
                .collect()
        };
        group_bulge_normal_maps(
            &placed(&group.members),
            &placed(&group.shape_only),
            &group.options,
        )
        .map_err(|e| {
            let g = group.group.map_or("", SurfaceGroup::as_str);
            invalid(format!("group `{g}`: {e}"))
        })
    }
}

/// まとまりの既定の設定 (`width` は形の幅。格子の画素)。
fn resolve(group: SurfaceGroup, width: f32) -> BulgeOptions {
    let (strength, edge, smoothing) = group.defaults();
    BulgeOptions {
        strength,
        radius: edge.map(|r| (r * width).max(1.0)),
        smoothing,
        ..BulgeOptions::default()
    }
}

/// 提案の理由の 1 行。
fn group_line(model: &Model, group: &PlannedGroup, made: &[usize]) -> String {
    let name = |i: &usize| format!("`{}`", model.textures[*i].name);
    let o = &group.options;
    let mut settings = format!("ふくらみ {}", o.strength);
    if let Some(r) = o.radius {
        settings.push_str(&format!("・縁 {r:.0} px"));
    }
    settings.push_str(&format!("・ならし {}", o.smoothing));
    match group.group {
        Some(g) if g != SurfaceGroup::Alone => {
            let mut line = format!(
                "- まとまり `{}` ({}): {settings}。作った法線マップ: {}",
                g.as_str(),
                group
                    .members
                    .iter()
                    .map(name)
                    .collect::<Vec<_>>()
                    .join(", "),
                made.iter().map(name).collect::<Vec<_>>().join(", ")
            );
            if !group.shape_only.is_empty() {
                line.push_str(&format!(
                    "。形に加えたもの: {}",
                    group
                        .shape_only
                        .iter()
                        .map(name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            line
        }
        _ => format!("- {} (単独): {settings}", name(&group.members[0])),
    }
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
    let targets = targets(
        model,
        labels,
        &options.textures,
        options.per_texture.keys(),
        options.replace_existing,
    )?;
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
        let layer = TexelCanvas::rasterize(
            model,
            &instance,
            labels,
            *index as u32,
            &image,
            o.alpha_threshold,
        );
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
        let values = fill_missing(
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

/// 法線マップを作るテクスチャ (番号と名前)。`overridden` はテクスチャごとの設定があるテクスチャの名前 (あるか確かめる)。
fn targets<'n>(
    model: &Model,
    labels: &BTreeMap<MeshId, Label>,
    textures: &[String],
    overridden: impl IntoIterator<Item = &'n String>,
    replace_existing: bool,
) -> Result<Vec<(usize, String)>, AiError> {
    let find = |name: &str| {
        model
            .textures
            .iter()
            .position(|t| t.name == name)
            .ok_or_else(|| invalid(format!("texture `{name}` not found")))
    };
    for name in overridden {
        find(name)?;
    }
    let indices: BTreeSet<usize> = if textures.is_empty() {
        model
            .meshes
            .iter()
            .filter(|m| !m.mask_only && labels.get(&m.id) != Some(&Label::Background))
            .filter_map(|m| usize::try_from(m.texture).ok())
            .filter(|&i| i < model.textures.len())
            .collect()
    } else {
        textures
            .iter()
            .map(|name| find(name))
            .collect::<Result<_, _>>()?
    };
    let targets: Vec<_> = indices
        .into_iter()
        .filter(|&i| replace_existing || model.textures[i].normal_map.is_none())
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
        alpha_threshold: u8,
    ) -> Self {
        let (w, h) = (image.width as usize, image.height as usize);
        let size = Vec2::new(image.width as f32, image.height as f32);
        let threshold = alpha_threshold.max(1);
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
}
