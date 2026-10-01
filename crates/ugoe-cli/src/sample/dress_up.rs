//! `ugoe sample dress-up`: 着せ替えの見本 (T0710)。スロットを持つ本体と、そこに付ける衣装のモジュール。
//!
//! 絵は PSD を使わずにコードで描き ([`super::paint`])、モデルとモジュールも要素を直接並べて作る
//! (見本のアバターのような推定は使わない)。本体は胸から上のデフォルメした人物で、髪 (`Hair`)・頭の飾り
//! (`Headwear`)・服 (`Outfit`) のスロットを持つ。モジュールは次の 4 つ。
//!
//! | モジュール | スロット | 内容 |
//! | --- | --- | --- |
//! | `LongHair` | `Hair` | 紺の長い髪。後ろ髪は物理で揺れ、体の後ろに回る (本体の肌を反転マスクに使う) |
//! | `TwinTails` | `Hair` | 金のツインテール。房が物理で揺れる |
//! | `CatEars` | `Headwear` | 猫耳のカチューシャ |
//! | `Sailor` | `Outfit` | セーラー服 |
//!
//! 本体はアバタープロファイルのレベル 2 を名乗り (T0715)、3 つのスロットをアプリに公開する (T0712)。
//! 本体の物理は既定の後ろ髪の揺れ (モジュールのプロジェクトで本体のリグを確かめられる。T0715)。
//! [`DressUpSample::save_projects`] は、本体とモジュールをエディタで開けるプロジェクトとしても書き出す。
//!
//! 座標はすべてキャンバス (512×512) で考え、要素の親の座標系へは [`Space`] で写す。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use ugoe_core::{Rgb, Vec2};
use ugoe_editor_core::{Project, ProjectMeta, Textures as ProjectTextures};
use ugoe_format::{Bundle, ModuleFile, TextureData, TextureFormat};
use ugoe_model::avatar::{
    AvatarHitArea, AvatarMeta, AvatarPermission, CommercialUsage, CreditNotation, HIT_AREA_BODY,
    HIT_AREA_HEAD, Modification,
};
use ugoe_model::deformer::{RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::module::DrawOrderRange;
use ugoe_model::parameter::ParameterKind;
use ugoe_model::part::PartKeyform;
use ugoe_model::physics::{
    NormalizationRange, PhysicsBlend, PhysicsInput, PhysicsNormalization, PhysicsOutput,
    PhysicsParticle, PhysicsRig, PhysicsSettings, PhysicsSource, PhysicsTarget,
};
use ugoe_model::standard as sp;
use ugoe_model::{
    AvatarProfile, BlendMode, Canvas, Deformer, DeformerKind, Effects, HitArea, KeyformBinding,
    KeyformGrid, MaskMode, MaskRef, Mesh, Model, ModelMeta, Module, ModulePhysics, ModulePose,
    ModuleRequirements, Parameter, Part, Pose, SCHEMA_VERSION, Slot, TextureRef,
};

use super::paint::{Color, Layer, Shape, rgb, with_alpha};

/// 本体のファイル名。モジュールは `<名前の kebab-case>.ugoemod` ([`DressUpSample::files`])。
pub const BASE_FILE: &str = "dress-up.ugoe";

/// 本体のプロジェクトのディレクトリ名。モジュールのプロジェクトは `<名前の kebab-case>.ugoeproj`
/// ([`DressUpSample::save_projects`])。
pub const BASE_PROJECT: &str = "dress-up.ugoeproj";

/// 本体の名前。
const NAME: &str = "ugoe sample dress-up";

const CANVAS: f32 = 512.0;
/// 頭の回転 (`AngleZ`) の中心 (首の付け根)。
const NECK: Vec2 = Vec2::new(256.0, 300.0);
/// 頭の中心 (髪の丸みの中心)。
const HEAD: Vec2 = Vec2::new(256.0, 198.0);
/// 頭のワープ `HeadWarp` が覆う矩形 (キャンバス座標、既定の姿)。髪の房の先まで入れる。
const HEAD_WARP: (Vec2, Vec2) = (Vec2::new(80.0, 40.0), Vec2::new(432.0, 504.0));
/// 体のワープ `BodyWarp` が覆う矩形。
const BODY_WARP: (Vec2, Vec2) = (Vec2::new(120.0, 256.0), Vec2::new(392.0, 512.0));
/// 目の中心 (`EyeR` は画面の左、`EyeL` は画面の右)。
const EYE_R: Vec2 = Vec2::new(222.0, 228.0);
const EYE_L: Vec2 = Vec2::new(290.0, 228.0);
const MOUTH: Vec2 = Vec2::new(256.0, 261.0);
/// 視線 (`EyeBallX`・`EyeBallY` = ±1) で瞳が動く量 (px)。
const GAZE: Vec2 = Vec2::new(4.0, 3.0);
/// 眉 (`BrowLY`・`BrowRY` = ±1) が上下する量 (px)。
const BROW_LIFT: f32 = 5.0;
/// `BodyAngleZ` の 1 あたりの体の上の辺と頭の傾き (度)。
const BODY_TILT: f32 = 0.5;
/// 体が `BodyAngleZ` で傾くときの軸 (腰。キャンバスの下の外)。
const BODY_PIVOT: Vec2 = Vec2::new(256.0, 600.0);

/// 輪郭線の幅 (px)。
const LINE: f32 = 2.5;
const SKIN: Color = rgb(0xffe6d8);
const SKIN_LINE: Color = rgb(0xb4806f);
const EYE: Color = rgb(0x3d2f5e);
const IRIS: Color = rgb(0x7b63b8);
const WHITE: Color = rgb(0xffffff);
const SHIRT: Color = rgb(0xf4f6fa);
const SHIRT_LINE: Color = rgb(0x8a92a8);

/// 本体のスロットの描画順の範囲。髪は顔 (500) と目・眉を挟む (後ろ髪は顔の奥、前髪は眉の手前)。
const HAIR_RANGE: (f32, f32) = (420.0, 680.0);
const HEADWEAR_RANGE: (f32, f32) = (900.0, 1000.0);
const OUTFIT_RANGE: (f32, f32) = (260.0, 340.0);

/// 見本の本体とモジュール。
pub struct DressUpSample {
    pub base: Bundle,
    pub modules: Vec<ModuleFile>,
}

impl DressUpSample {
    /// 書き出すファイル (名前とバイト列)。本体が先、モジュールは作った順。
    pub fn files(&self) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let mut files = vec![(BASE_FILE.to_owned(), ugoe_format::write_bundle(&self.base)?)];
        for file in &self.modules {
            let name = format!("{}.ugoemod", kebab_case(&file.module.name));
            files.push((name, ugoe_format::write_module(file)?));
        }
        Ok(files)
    }

    /// 本体とモジュールを、エディタで開けるプロジェクトとして `dir` に書き出す: 本体 `dress-up.ugoeproj` と、
    /// それを参照するモジュールのプロジェクト `<名前の kebab-case>.ugoeproj` (同じディレクトリに並べる)。
    /// 書いたプロジェクトのパスを、本体を先に返す。
    ///
    /// 形式は `ugoe_editor_core::Project` が書く (エディタの「New Module…」と同じく、書いた本体を読み直して
    /// モジュールのプロジェクトを作る)。
    pub fn save_projects(&self, dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let base_dir = dir.join(BASE_PROJECT);
        let model = &self.base.model;
        let mut base = Project::new(
            ProjectMeta::new(NAME),
            model.clone(),
            images(&model.textures, &self.base.textures)?,
        );
        base.physics = self.base.physics.clone();
        base.motions = self.base.motions.clone();
        base.expressions = self.base.expressions.clone();
        base.save(&base_dir)
            .with_context(|| format!("saving {}", base_dir.display()))?;
        let mut written = vec![base_dir.clone()];
        for file in &self.modules {
            let module = &file.module;
            let path = dir.join(format!("{}.ugoeproj", kebab_case(&module.name)));
            let empty = Project::new_module(&module.name, &base_dir, &module.slot)
                .with_context(|| format!("creating the project of `{}`", module.name))?;
            let context = empty
                .module
                .as_ref()
                .ok_or_else(|| anyhow!("`{}` is not a module project", module.name))?
                .reconfigured(&module.slot, module.name.clone(), module.requires.clone())?;
            let workspace =
                context.workspace(module, &images(&module.textures, &file.textures)?)?;
            let project = Project {
                model: workspace.model,
                textures: workspace.textures,
                physics: workspace.physics,
                module: Some(Arc::new(context)),
                ..empty
            };
            project
                .save(&path)
                .with_context(|| format!("saving {}", path.display()))?;
            written.push(path);
        }
        Ok(written)
    }
}

/// テクスチャの参照と中身 (PNG) を、プロジェクトの画像 (名前 → PNG) にする。
fn images(refs: &[TextureRef], data: &[TextureData]) -> anyhow::Result<ProjectTextures> {
    if refs.len() != data.len() {
        bail!("{} textures but {} images", refs.len(), data.len());
    }
    refs.iter()
        .zip(data)
        .map(|(r, d)| {
            if d.format != TextureFormat::Png {
                bail!("texture `{}` is not a PNG", r.name);
            }
            Ok((r.name.clone(), Arc::from(d.bytes.as_slice())))
        })
        .collect()
}

/// `LongHair` → `long-hair`。
fn kebab_case(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// 見本の本体とモジュールを作り、検証と合成を確かめる。
pub fn build_dress_up() -> anyhow::Result<DressUpSample> {
    let mut base = base();
    base.motions = super::animations::motions()?;
    base.expressions = super::animations::dress_up_expressions()?;
    super::animations::check(&base.model, &base.motions, &base.expressions)?;
    let modules = vec![long_hair(), twin_tails(), cat_ears(), sailor()];
    let physics = base
        .physics
        .clone()
        .unwrap_or_else(ugoe_editor_core::physics_edit::default_settings);
    check(&base.model, &physics, &modules)?;
    Ok(DressUpSample { base, modules })
}

/// 本体とモジュールがそれぞれ検証を通り、同じスロットのものを除くどの組み合わせでも合成できること。
fn check(base: &Model, physics: &PhysicsSettings, modules: &[ModuleFile]) -> anyhow::Result<()> {
    let errors = ugoe_model::validate::validate(base);
    if !errors.is_empty() {
        bail!("the base model is invalid: {errors:?}");
    }
    let errors = ugoe_model::validate::validate_physics(physics, base);
    if !errors.is_empty() {
        bail!("the base physics is invalid: {errors:?}");
    }
    // 警告 (値域の違いなど) も見本には残さない。
    let report = ugoe_model::avatar::check::check_profile(base);
    if !report.issues.is_empty() || report.claimed != Some(report.satisfied) {
        bail!(
            "the base model does not conform to the avatar profile: {:?}",
            report
        );
    }
    for file in modules {
        let errors = ugoe_model::validate::validate_module(&file.module);
        if !errors.is_empty() {
            bail!("module `{}` is invalid: {errors:?}", file.module.name);
        }
    }
    for hair in ["LongHair", "TwinTails"] {
        let set: Vec<&Module> = modules
            .iter()
            .map(|f| &f.module)
            .filter(|m| m.slot.as_str() != "Hair" || m.name == hair)
            .collect();
        for n in 1..=set.len() {
            let modules = &set[..n];
            let model = ugoe_runtime::compose(base, modules)
                .with_context(|| format!("composing {:?}", names(modules)))?;
            let composed = ugoe_runtime::compose_physics(base, physics, modules)
                .with_context(|| format!("composing the physics of {:?}", names(modules)))?;
            let errors = ugoe_model::validate::validate_physics(&composed, &model);
            if !errors.is_empty() {
                bail!("the physics of {:?} is invalid: {errors:?}", names(modules));
            }
        }
    }
    Ok(())
}

fn names(modules: &[&Module]) -> Vec<String> {
    modules.iter().map(|m| m.name.clone()).collect()
}

// ---- 座標と要素の組み立て ----

/// 要素の親の座標系。キャンバス座標の点を、既定の姿でのその座標系の点に写す。
#[derive(Debug, Clone, Copy)]
enum Space {
    /// 回転デフォーマの子 (原点のキャンバス座標)。既定の姿では回転も拡大もしない。
    Rotation(Vec2),
    /// ワープデフォーマの子 (覆う矩形)。既定の姿では格子が矩形。
    Warp(Vec2, Vec2),
}

impl Space {
    fn local(self, p: Vec2) -> Vec2 {
        match self {
            Self::Rotation(origin) => p - origin,
            Self::Warp(min, max) => (p - min) / (max - min),
        }
    }
}

/// 頭のワープの子 (メッシュと、モジュールの最上位の要素)。
const IN_HEAD: Space = Space::Warp(HEAD_WARP.0, HEAD_WARP.1);
/// 体のワープの子。
const IN_BODY: Space = Space::Warp(BODY_WARP.0, BODY_WARP.1);

fn parameter(id: &str) -> Parameter {
    let (min, default, max) = sp::range(id).unwrap_or((0.0, 0.0, 1.0));
    Parameter {
        id: id.into(),
        name: id.into(),
        min,
        max,
        default,
        kind: ParameterKind::Normal,
        mirror: None,
        group: None,
        internal: false,
    }
}

fn part(id: &str, parent: Option<&str>) -> Part {
    Part {
        id: id.into(),
        name: id.into(),
        parent: parent.map(Into::into),
        keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
    }
}

/// 軸 `axes` (パラメータとキー) の格子。キーフォームは `f(各軸の値)` で作る (軸 0 が最も速く変わる)。
fn grid<T>(axes: &[(&str, &[f32])], f: impl Fn(&[f32]) -> T) -> KeyformGrid<T> {
    let bindings: Vec<KeyformBinding> = axes
        .iter()
        .map(|(parameter, keys)| KeyformBinding {
            parameter: (*parameter).into(),
            keys: keys.to_vec(),
        })
        .collect();
    let count: usize = axes.iter().map(|(_, keys)| keys.len()).product();
    let keyforms = (0..count)
        .map(|mut index| {
            let values: Vec<f32> = axes
                .iter()
                .map(|(_, keys)| {
                    let value = keys[index % keys.len()];
                    index /= keys.len();
                    value
                })
                .collect();
            f(&values)
        })
        .collect();
    KeyformGrid::new(bindings, keyforms)
}

fn rotation(
    id: &str,
    part: &str,
    parent: Option<&str>,
    keyforms: KeyformGrid<RotationKeyform>,
) -> Deformer {
    Deformer {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Rotation(RotationDeformer { keyforms }),
        mirror: None,
        mirror_axis: None,
    }
}

fn rotation_keyform(origin: Vec2, angle: f32) -> RotationKeyform {
    RotationKeyform {
        origin,
        angle,
        scale: 1.0,
        reflect_x: false,
        reflect_y: false,
        opacity: 1.0,
    }
}

/// 回転しない回転デフォーマ (物理が角度を足す揺れものの根元)。原点 `origin` はキャンバス座標。
fn pivot(id: &str, part: &str, origin: Vec2) -> Deformer {
    rotation(
        id,
        part,
        None,
        KeyformGrid::constant(rotation_keyform(IN_HEAD.local(origin), 0.0)),
    )
}

/// 矩形 `min`〜`max` を覆う `cols`×`rows` セルのワープ。制御点は `f(キャンバスの点, 各軸の値)` で動かし、
/// 親の座標系 `space` で書く。
#[allow(clippy::too_many_arguments)]
fn warp(
    id: &str,
    part: &str,
    parent: Option<&str>,
    space: Space,
    (min, max): (Vec2, Vec2),
    (cols, rows): (u16, u16),
    axes: &[(&str, &[f32])],
    f: impl Fn(Vec2, &[f32]) -> Vec2,
) -> Deformer {
    let points: Vec<Vec2> = (0..=rows)
        .flat_map(|y| (0..=cols).map(move |x| (x, y)))
        .map(|(x, y)| {
            min + (max - min)
                * Vec2::new(
                    f32::from(x) / f32::from(cols),
                    f32::from(y) / f32::from(rows),
                )
        })
        .collect();
    let keyforms = grid(axes, |values| WarpKeyform {
        points: points.iter().map(|p| space.local(f(*p, values))).collect(),
        opacity: 1.0,
    });
    Deformer {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Warp(WarpDeformer {
            rows,
            cols,
            keyforms,
            depth: None,
        }),
        mirror: None,
        mirror_axis: None,
    }
}

/// テクスチャの参照と中身を並べて集める。
#[derive(Default)]
struct Textures {
    refs: Vec<TextureRef>,
    data: Vec<TextureData>,
}

impl Textures {
    /// `layer` をテクスチャ `name` として加え、番号を返す。
    fn add(&mut self, name: &str, layer: &Layer) -> u32 {
        self.refs.push(TextureRef {
            name: name.to_owned(),
            width: layer.width(),
            height: layer.height(),
            normal_map: None,
        });
        self.data.push(TextureData {
            format: TextureFormat::Png,
            bytes: layer.png(),
        });
        self.refs.len() as u32 - 1
    }
}

/// 1 枚の絵を貼るメッシュの作り方。
struct Sheet<'a> {
    id: &'a str,
    part: &'a str,
    parent: Option<&'a str>,
    space: Space,
    draw_order: f32,
    /// 格子のセルの数 (横, 縦)。
    cells: (u16, u16),
}

impl<'a> Sheet<'a> {
    /// 本体の頭のワープの子の絵。
    fn head(id: &'a str, part: &'a str, draw_order: f32, cells: (u16, u16)) -> Self {
        Self {
            id,
            part,
            parent: Some("HeadWarp"),
            space: IN_HEAD,
            draw_order,
            cells,
        }
    }

    /// 本体の体のワープの子の絵。
    fn body(id: &'a str, part: &'a str, draw_order: f32) -> Self {
        Self {
            id,
            part,
            parent: Some("BodyWarp"),
            space: IN_BODY,
            draw_order,
            cells: (4, 4),
        }
    }

    /// 画像 `layer` (テクスチャ `texture`) を覆う格子のメッシュ。頂点は既定の姿でのキャンバス座標から
    /// `f(頂点, 各軸の値)` で (位置, 不透明度) を決める。
    fn mesh(
        &self,
        texture: u32,
        layer: &Layer,
        axes: &[(&str, &[f32])],
        f: impl Fn(Vec2, &[f32]) -> (Vec2, f32),
    ) -> Mesh {
        let (min, max) = layer.bounds();
        let (cols, rows) = self.cells;
        let mut uvs = Vec::new();
        for y in 0..=rows {
            for x in 0..=cols {
                uvs.push(Vec2::new(
                    f32::from(x) / f32::from(cols),
                    f32::from(y) / f32::from(rows),
                ));
            }
        }
        let mut indices = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                let a = y * (cols + 1) + x;
                let (b, c) = (a + 1, a + cols + 1);
                indices.extend([a, b, c + 1, a, c + 1, c]);
            }
        }
        let canvas: Vec<Vec2> = uvs.iter().map(|uv| min + (max - min) * *uv).collect();
        let keyforms = grid(axes, |values| {
            let mut opacity = 1.0;
            let positions = canvas
                .iter()
                .map(|p| {
                    let (q, o) = f(*p, values);
                    opacity = o;
                    self.space.local(q)
                })
                .collect();
            MeshKeyform {
                positions,
                opacity,
                draw_order: self.draw_order,
                multiply_color: Rgb::WHITE,
                screen_color: Rgb::BLACK,
                mask_strength: 1.0,
            }
        });
        Mesh {
            id: self.id.into(),
            name: self.id.into(),
            part: self.part.into(),
            parent: self.parent.map(Into::into),
            texture,
            uvs,
            indices,
            blend: BlendMode::Normal,
            culling: false,
            masks: Vec::new(),
            mask_mode: MaskMode::Union,
            mask_only: false,
            keyforms,
            mirror: None,
            mirror_axis: None,
            depth: None,
            lit: true,
        }
    }

    /// 動かないメッシュ。
    fn still(&self, texture: u32, layer: &Layer) -> Mesh {
        self.mesh(texture, layer, &[], |p, _| (p, 1.0))
    }
}

/// `x = 256` で左右を入れ替えた点。
fn mirror(p: Vec2) -> Vec2 {
    Vec2::new(CANVAS - p.x, p.y)
}

// ---- 本体 ----

/// 本体: 胸から上の人物。既定の髪 (パーツ `Hair`) と服 (パーツ `Clothes`) はスロットのモジュールで置き換わる。
/// パラメータはアバタープロファイルのレベル 2 の分 (T0715)。既定の後ろ髪は物理で揺れる。
fn base() -> Bundle {
    let mut textures = Textures::default();
    let parameters = [
        sp::ANGLE_X,
        sp::ANGLE_Y,
        sp::ANGLE_Z,
        sp::BODY_ANGLE_X,
        sp::BODY_ANGLE_Y,
        sp::BODY_ANGLE_Z,
        sp::BREATH,
        sp::EYE_L_OPEN,
        sp::EYE_R_OPEN,
        sp::EYE_BALL_X,
        sp::EYE_BALL_Y,
        sp::BROW_L_Y,
        sp::BROW_R_Y,
        sp::MOUTH_OPEN_Y,
        sp::MOUTH_FORM,
    ]
    .map(parameter)
    .to_vec();
    let parts = vec![
        part("Body", None),
        part("Head", Some("Body")),
        part("Hair", Some("Head")),
        part("Clothes", Some("Body")),
    ];

    // 体: 上ほど `BodyAngleX` で横に、`Breath`・`BodyAngleY` で上下に動き、`BodyAngleZ` で腰の下を軸に傾く。
    // 頭の回転の中心 (首) も同じだけ動かし、頭も体と一緒に傾ける。
    let body_warp = warp(
        "BodyWarp",
        "Body",
        None,
        Space::Rotation(Vec2::ZERO),
        BODY_WARP,
        (2, 2),
        &[
            (sp::BODY_ANGLE_X, &[-10.0, 0.0, 10.0]),
            (sp::BREATH, &[0.0, 1.0]),
            (sp::BODY_ANGLE_Y, &[-10.0, 0.0, 10.0]),
            (sp::BODY_ANGLE_Z, &[-10.0, 0.0, 10.0]),
        ],
        |p, v| body_pose(p, v[0], v[1], v[2], v[3]),
    );
    // `BodyAngleZ` は首の位置が円を描くので、0 もキーにする (2 つのキーの間の直線では既定の姿で首がずれる)。
    let head_rotation = rotation(
        "HeadRotation",
        "Head",
        None,
        grid(
            &[
                (sp::ANGLE_Z, &[-30.0, 30.0]),
                (sp::BODY_ANGLE_X, &[-10.0, 10.0]),
                (sp::BODY_ANGLE_Y, &[-10.0, 10.0]),
                (sp::BODY_ANGLE_Z, &[-10.0, 0.0, 10.0]),
            ],
            |v| {
                rotation_keyform(
                    body_pose(NECK, v[1], 0.0, v[2], v[3]),
                    v[0] / 3.0 + v[3] * BODY_TILT,
                )
            },
        ),
    );
    // 顔の向き: 顔の中ほどを頂点にした丸みの分だけ、`AngleX` で横に、`AngleY` で縦に動かす
    // (目・口は大きく、輪郭と後ろ髪は小さく動くので、向きが変わって見える)。
    let head_warp = warp(
        "HeadWarp",
        "Head",
        Some("HeadRotation"),
        Space::Rotation(NECK),
        HEAD_WARP,
        (8, 8),
        &[
            (sp::ANGLE_X, &[-30.0, 0.0, 30.0]),
            (sp::ANGLE_Y, &[-30.0, 0.0, 30.0]),
        ],
        |p, v| {
            let d = (p - Vec2::new(256.0, 205.0)) / Vec2::new(130.0, 140.0);
            let dome = (1.0 - d.length_squared()).max(0.0);
            p + Vec2::new(v[0] / 30.0 * 16.0, -v[1] / 30.0 * 12.0) * dome
        },
    );
    // 既定の後ろ髪の揺れ (物理のリグ `HairSway` が角度を足す)。
    let hair_sway = rotation(
        "HairSway",
        "Hair",
        Some("HeadWarp"),
        KeyformGrid::constant(rotation_keyform(IN_HEAD.local(HEAD), 0.0)),
    );

    let hair = Palette {
        fill: rgb(0x8c5a43),
        line: rgb(0x4f3126),
        sheen: rgb(0xb07a5e),
    };

    let mut meshes = Vec::new();
    let art = hair_back(&hair, bob_back());
    let t = textures.add("hair-back", &art);
    meshes.push(on_pivot("HairBack", "Hair", ("HairSway", HEAD), 150.0, (4, 4)).still(t, &art));
    let art = skin();
    let t = textures.add("skin", &art);
    meshes.push(Sheet::body("Skin", "Body", 200.0).still(t, &art));
    let art = shirt();
    let t = textures.add("shirt", &art);
    meshes.push(Sheet::body("Shirt", "Clothes", 250.0).still(t, &art));
    let art = face();
    let t = textures.add("face", &art);
    meshes.push(Sheet::head("Face", "Head", 500.0, (4, 4)).still(t, &art));
    for (id, center) in [("L", EYE_L), ("R", EYE_R)] {
        let parameter = if id == "L" {
            sp::EYE_L_OPEN
        } else {
            sp::EYE_R_OPEN
        };
        // 開いた目は閉じるにつれ下寄りの線へつぶれて消え、閉じた目の線が現れる。
        let keys: &[f32] = &[0.0, 0.3, 1.0];
        let squash = center.y + 7.0;
        let blink = move |p: Vec2, open: f32| {
            let (scale, opacity) = match open {
                x if x <= 0.0 => (0.08, 0.0),
                x if x < 1.0 => (0.4, 1.0),
                _ => (1.0, 1.0),
            };
            (Vec2::new(p.x, squash + (p.y - squash) * scale), opacity)
        };
        let (open_id, iris_id, closed_id) = (
            format!("Eye{id}"),
            format!("Eye{id}Iris"),
            format!("Eye{id}Closed"),
        );
        let art = eye_open(center);
        let t = textures.add(&format!("eye-{}", id.to_lowercase()), &art);
        meshes.push(Sheet::head(&open_id, "Head", 530.0, (1, 1)).mesh(
            t,
            &art,
            &[(parameter, keys)],
            |p, v| blink(p, v[0]),
        ));
        // 瞳は視線 (`EyeBallX`・`EyeBallY`) で目の中を動き、目の形でくり抜く。
        let art = eye_iris(center);
        let t = textures.add(&format!("eye-{}-iris", id.to_lowercase()), &art);
        let mut iris = Sheet::head(&iris_id, "Head", 535.0, (1, 1)).mesh(
            t,
            &art,
            &[
                (parameter, keys),
                (sp::EYE_BALL_X, &[-1.0, 0.0, 1.0]),
                (sp::EYE_BALL_Y, &[-1.0, 0.0, 1.0]),
            ],
            |p, v| blink(p + Vec2::new(v[1] * GAZE.x, -v[2] * GAZE.y), v[0]),
        );
        iris.masks = vec![MaskRef::normal(open_id.as_str())];
        meshes.push(iris);
        let art = eye_closed(center);
        let t = textures.add(&format!("eye-{}-closed", id.to_lowercase()), &art);
        meshes.push(Sheet::head(&closed_id, "Head", 530.0, (1, 1)).mesh(
            t,
            &art,
            &[(parameter, keys)],
            |p, v| (p, if v[0] <= 0.0 { 1.0 } else { 0.0 }),
        ));
    }
    // 眉は左右別々に `BrowLY`・`BrowRY` で上下する (正で上)。
    for (id, center, parameter, outer) in [
        ("L", EYE_L, sp::BROW_L_Y, 1.0),
        ("R", EYE_R, sp::BROW_R_Y, -1.0),
    ] {
        let art = brow(center, outer);
        let t = textures.add(&format!("brow-{}", id.to_lowercase()), &art);
        meshes.push(
            Sheet::head(&format!("Brow{id}"), "Head", 540.0, (1, 1)).mesh(
                t,
                &art,
                &[(parameter, &[-1.0, 0.0, 1.0])],
                |p, v| (p - Vec2::new(0.0, v[0] * BROW_LIFT), 1.0),
            ),
        );
    }
    // 口: `MouthOpenY` で上の辺へつぶれ、`MouthForm` で口角が上がる (正で笑顔) か下がる。
    let art = mouth();
    let t = textures.add("mouth", &art);
    meshes.push(Sheet::head("Mouth", "Head", 530.0, (4, 2)).mesh(
        t,
        &art,
        &[
            (sp::MOUTH_OPEN_Y, &[0.0, 1.0]),
            (sp::MOUTH_FORM, &[-1.0, 0.0, 1.0]),
        ],
        |p, v| {
            let scale = 0.2 + 0.8 * v[0];
            let corner = ((p.x - MOUTH.x) / 14.0).powi(2);
            let y = MOUTH.y + (p.y - MOUTH.y) * scale - v[1] * 4.0 * corner;
            (Vec2::new(p.x, y), 1.0)
        },
    ));
    let art = bangs(&hair, &bob_bangs());
    let t = textures.add("hair-front", &art);
    meshes.push(Sheet::head("HairFront", "Hair", 650.0, (4, 4)).still(t, &art));

    let slot = |id: &str, anchor: &str, (min, max): (f32, f32), part: &str| Slot {
        id: id.into(),
        anchor: Some(anchor.into()),
        draw_order: DrawOrderRange { min, max },
        replaces: Vec::new(),
        parameters: Vec::new(),
        mask_sources: Vec::new(),
        part: Some(part.into()),
        colliders: Vec::new(),
        pose_parts: Vec::new(),
    };
    let slots = vec![
        Slot {
            replaces: vec!["Hair".into()],
            // 長い後ろ髪を体の奥に見せるため、肌を反転マスクに使える。
            mask_sources: vec!["Skin".into()],
            ..slot("Hair", "HeadWarp", HAIR_RANGE, "Head")
        },
        slot("Headwear", "HeadWarp", HEADWEAR_RANGE, "Head"),
        Slot {
            replaces: vec!["Clothes".into()],
            ..slot("Outfit", "BodyWarp", OUTFIT_RANGE, "Body")
        },
    ];

    let model = Model {
        schema_version: SCHEMA_VERSION,
        meta: ModelMeta {
            name: NAME.to_owned(),
            canvas: Canvas {
                width: CANVAS,
                height: CANVAS,
                origin: Vec2::splat(CANVAS / 2.0),
            },
        },
        parameters,
        parameter_groups: Vec::new(),
        parts,
        deformers: vec![body_warp, head_rotation, head_warp, hair_sway],
        meshes,
        glues: Vec::new(),
        textures: textures.refs,
        hit_areas: vec![
            HitArea {
                id: HIT_AREA_HEAD.to_owned(),
                mesh: "Face".into(),
            },
            HitArea {
                id: HIT_AREA_BODY.to_owned(),
                mesh: "Skin".into(),
            },
        ],
        pose: Pose::default(),
        effects: Effects::standard(),
        avatar: Some(avatar_profile(&slots)),
        slots,
        lighting: false,
    };
    Bundle {
        textures: textures.data,
        physics: Some(base_physics()),
        ..Bundle::new(model)
    }
}

/// 本体の物理: 既定の後ろ髪の揺れ (`HairSway`)。重力などはエディタの既定の設定と同じで、モジュールの物理も
/// この設定で動く。
///
/// 衝突体はスロットに公開しない: 公開すると、物理を読まずにモデルだけを開いたホストでは、そのスロットに
/// モジュールを付けられなくなる (スロットの `colliders` の参照先が無い)。
fn base_physics() -> PhysicsSettings {
    PhysicsSettings {
        rigs: vec![pendulum("HairSway", 120.0, 6.0, 0.4)],
        ..ugoe_editor_core::physics_edit::default_settings()
    }
}

/// 体の点 `p` (既定の姿のキャンバス座標) を、体のパラメータの値で動かした位置。上ほど `BodyAngleX` で横に、
/// `Breath` と `BodyAngleY` で上下に動き、`BodyAngleZ` で `BODY_PIVOT` を軸に傾く (上ほど大きく)。
fn body_pose(p: Vec2, x: f32, breath: f32, y: f32, z: f32) -> Vec2 {
    let lift = ((BODY_WARP.1.y - p.y) / (BODY_WARP.1.y - BODY_WARP.0.y)).clamp(0.0, 1.0);
    let q = p + Vec2::new(x, -3.0 * breath - 0.4 * y) * lift;
    // 下の辺 (キャンバスの端で切れた胴) は動かさず、上ほど大きく傾ける。
    let (sin, cos) = (z * BODY_TILT * lift).to_radians().sin_cos();
    let d = q - BODY_PIVOT;
    BODY_PIVOT + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
}

/// 本体のアバタープロファイル (レベル 2)。スロットはすべてアプリに公開する (利用者が着せ替えを選べる)。
/// 絵はコードで描いたもので、リポジトリと同じ MIT で配布する。
fn avatar_profile(slots: &[Slot]) -> AvatarProfile {
    AvatarProfile {
        level: 2,
        meta: AvatarMeta {
            name: NAME.to_owned(),
            version: Some("0.1.0".to_owned()),
            authors: vec!["ugoe contributors".to_owned()],
            license_url: Some("https://opensource.org/license/mit".to_owned()),
            avatar_permission: AvatarPermission::Everyone,
            commercial_usage: CommercialUsage::Corporation,
            credit_notation: CreditNotation::Unnecessary,
            allow_redistribution: true,
            modification: Modification::AllowRedistribution,
            ..AvatarMeta::default()
        },
        hit_areas: [HIT_AREA_HEAD, HIT_AREA_BODY]
            .map(|name| AvatarHitArea {
                name: name.to_owned(),
                hit_area: name.to_owned(),
            })
            .to_vec(),
        slots: slots.iter().map(|s| s.id.to_string()).collect(),
        ..AvatarProfile::default()
    }
}

/// 髪の色。
struct Palette {
    fill: Color,
    line: Color,
    /// つやの色。
    sheen: Color,
}

fn face() -> Layer {
    let shape = Shape::ellipse(Vec2::new(256.0, 205.0), 84.0, 90.0)
        .union(Shape::circle(Vec2::new(173.0, 214.0), 14.0))
        .union(Shape::circle(Vec2::new(339.0, 214.0), 14.0));
    let mut layer = Layer::new(Vec2::new(150.0, 108.0), Vec2::new(362.0, 300.0));
    layer.paint(&shape, SKIN, SKIN_LINE, LINE);
    let blush = with_alpha(rgb(0xff9aa8), 110);
    for x in [208.0, 304.0] {
        layer.fill(&Shape::ellipse(Vec2::new(x, 246.0), 15.0, 8.0), blush);
    }
    layer
}

/// 首と胴 (肌)。服の下に隠れる。
fn skin() -> Layer {
    let shape =
        Shape::rounded_rect(Vec2::new(234.0, 262.0), Vec2::new(278.0, 350.0), 8.0).union(torso());
    let mut layer = Layer::new(Vec2::new(160.0, 256.0), Vec2::new(352.0, 512.0));
    layer.paint(&shape, SKIN, SKIN_LINE, LINE);
    layer
}

/// 胴の形 (肌と服で同じ。服の輪郭が肌からはみ出さないように)。
fn torso() -> Shape {
    Shape::rounded_rect(Vec2::new(166.0, 332.0), Vec2::new(346.0, 600.0), 60.0)
}

/// 既定の服 (丸首のシャツ)。
fn shirt() -> Layer {
    let shape = torso().subtract(Shape::ellipse(Vec2::new(256.0, 334.0), 30.0, 22.0));
    let mut layer = Layer::new(Vec2::new(160.0, 326.0), Vec2::new(352.0, 512.0));
    layer.paint(&shape, SHIRT, SHIRT_LINE, LINE);
    layer
}

fn eye_open(center: Vec2) -> Layer {
    let shape = Shape::ellipse(center, 12.0, 17.0);
    let mut layer = Layer::new(
        center - Vec2::new(15.0, 20.0),
        center + Vec2::new(15.0, 20.0),
    );
    layer.fill(&shape, EYE);
    layer
}

/// 瞳とハイライト。視線で動かし、開いた目 (`eye_open`) の形でくり抜く。
fn eye_iris(center: Vec2) -> Layer {
    let mut layer = Layer::new(
        center + Vec2::new(-10.0, -12.0),
        center + Vec2::new(10.0, 16.0),
    );
    layer.fill(
        &Shape::ellipse(center + Vec2::new(0.0, 6.0), 9.0, 9.0),
        IRIS,
    );
    layer.fill(&Shape::circle(center + Vec2::new(-4.0, -7.0), 4.5), WHITE);
    layer.fill(
        &Shape::circle(center + Vec2::new(4.0, 7.0), 2.0),
        with_alpha(WHITE, 200),
    );
    layer
}

/// 閉じた目 (下に弧を描く線)。
fn eye_closed(center: Vec2) -> Layer {
    let arc = Shape::ring(center + Vec2::new(0.0, -12.0), 16.0, 1.8)
        .intersect(Shape::below(center.y - 3.0));
    let mut layer = Layer::new(
        center + Vec2::new(-18.0, -6.0),
        center + Vec2::new(18.0, 8.0),
    );
    layer.fill(&arc, EYE);
    layer
}

/// 片方の眉。`outer` は目尻の向き (画面で +1 が右)。目尻の側が少し下がる。
fn brow(center: Vec2, outer: f32) -> Layer {
    let color = rgb(0x5a3a30);
    let (y, dy) = (center.y - 32.0, 2.0);
    let shape = Shape::capsule(
        Vec2::new(center.x - 10.0 * outer, y),
        Vec2::new(center.x + 11.0 * outer, y + dy),
        2.2,
    );
    let mut layer = Layer::new(
        Vec2::new(center.x - 16.0, y - 5.0),
        Vec2::new(center.x + 16.0, y + dy + 5.0),
    );
    layer.fill(&shape, color);
    layer
}

/// 開いた口 (上が平らな半円)。閉じるときは上の辺へつぶす。
fn mouth() -> Layer {
    let shape = Shape::ellipse(MOUTH, 11.0, 10.0).intersect(Shape::below(MOUTH.y));
    let mut layer = Layer::new(
        MOUTH + Vec2::new(-14.0, -4.0),
        MOUTH + Vec2::new(14.0, 14.0),
    );
    layer.paint(&shape, rgb(0xb8404e), rgb(0x7a2a33), 1.5);
    layer.fill(
        &Shape::ellipse(MOUTH + Vec2::new(0.0, 8.0), 6.0, 3.5).intersect(shape),
        rgb(0xf08a95),
    );
    layer
}

// ---- 髪 (本体の既定の髪と、髪のモジュールで共通) ----

/// 前髪の房 (中心の x、房の下端の y、半径)。
struct Bangs {
    locks: Vec<(f32, f32, f32)>,
    /// 顔の横に垂らす房の筋 (画面の左のもの。右は左右を入れ替える)。
    side: Vec<(Vec2, f32)>,
}

fn bob_back() -> Shape {
    Shape::circle(HEAD, 108.0).union(Shape::rounded_rect(
        Vec2::new(148.0, 190.0),
        Vec2::new(364.0, 302.0),
        30.0,
    ))
}

fn bob_bangs() -> Bangs {
    Bangs {
        locks: vec![
            (182.0, 178.0, 17.0),
            (210.0, 186.0, 17.0),
            (238.0, 182.0, 16.0),
            (266.0, 188.0, 17.0),
            (294.0, 182.0, 17.0),
            (324.0, 176.0, 17.0),
        ],
        side: vec![
            (Vec2::new(178.0, 170.0), 13.0),
            (Vec2::new(172.0, 232.0), 12.0),
            (Vec2::new(171.0, 270.0), 5.0),
        ],
    }
}

/// 後ろ髪の絵 (形 `shape`)。
fn hair_back(palette: &Palette, shape: Shape) -> Layer {
    let (min, max) = bounds_of(&shape, Vec2::new(60.0, 40.0), Vec2::new(452.0, 512.0));
    let mut layer = Layer::new(min, max);
    layer.paint(&shape, palette.fill, palette.line, LINE);
    layer
}

/// 前髪の絵: 頭の上を覆う帽子の形から房が垂れ、顔の横に房を垂らす。
fn bangs(palette: &Palette, bangs: &Bangs) -> Layer {
    let locks = Shape::union_all(
        bangs
            .locks
            .iter()
            .map(|(x, tip, r)| Shape::ellipse(Vec2::new(*x, tip - 34.0), *r, 34.0)),
    );
    let cap = Shape::ellipse(Vec2::new(256.0, 122.0), 112.0, 46.0)
        .union(locks)
        .intersect(Shape::circle(HEAD, 106.0));
    let side_r = Shape::stroke(&bangs.side);
    let side_l = Shape::stroke(
        &bangs
            .side
            .iter()
            .map(|(p, r)| (mirror(*p), *r))
            .collect::<Vec<_>>(),
    );
    let shape = cap.clone().union(side_r).union(side_l);
    let (min, max) = bounds_of(&shape, Vec2::new(60.0, 40.0), Vec2::new(452.0, 512.0));
    let mut layer = Layer::new(min, max);
    layer.paint(&shape, palette.fill, palette.line, LINE);
    // 頭の丸みに沿ったつや。
    layer.fill(
        &Shape::ring(HEAD, 80.0, 3.5)
            .intersect(Shape::above(140.0))
            .intersect(cap.grow(-5.0)),
        palette.sheen,
    );
    layer
}

/// 形 `shape` を覆う矩形 (探す範囲 `min`〜`max`、4px ごとに調べ、輪郭線の分だけ広げる)。
fn bounds_of(shape: &Shape, min: Vec2, max: Vec2) -> (Vec2, Vec2) {
    let mut lo = max;
    let mut hi = min;
    let mut y = min.y;
    while y <= max.y {
        let mut x = min.x;
        while x <= max.x {
            let p = Vec2::new(x, y);
            if shape.distance(p) < 4.0 {
                lo = lo.min(p);
                hi = hi.max(p);
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let margin = Vec2::splat(4.0 + LINE);
    (
        (lo - margin).max(Vec2::ZERO),
        (hi + margin).min(Vec2::splat(CANVAS)),
    )
}

// ---- モジュール ----

/// モジュールの枠。要素とテクスチャを加えて [`ModuleFile`] にする。
struct ModuleBuilder {
    module: Module,
    textures: Textures,
}

impl ModuleBuilder {
    fn new(name: &str, slot: &str, part: &str) -> Self {
        Self {
            module: Module {
                schema_version: SCHEMA_VERSION,
                name: name.into(),
                slot: slot.into(),
                requires: ModuleRequirements::default(),
                parts: vec![self::part(part, None)],
                deformers: Vec::new(),
                meshes: Vec::new(),
                glues: Vec::new(),
                textures: Vec::new(),
                physics: ModulePhysics::default(),
                pose: ModulePose::default(),
            },
            textures: Textures::default(),
        }
    }

    /// 動かない絵を加える。`sheet.id` をテクスチャの名前にも使う。
    fn add(&mut self, sheet: Sheet, layer: &Layer) -> &mut Mesh {
        let texture = self.textures.add(&kebab_case(sheet.id), layer);
        self.module.meshes.push(sheet.still(texture, layer));
        self.module.meshes.last_mut().expect("just pushed")
    }

    fn finish(self) -> ModuleFile {
        ModuleFile {
            module: Module {
                textures: self.textures.refs,
                ..self.module
            },
            textures: self.textures.data,
            normal_maps: Vec::new(),
            meta: None,
        }
    }
}

/// 頭のワープ (スロットの取り付け先) の直下の絵。
fn on_head<'a>(id: &'a str, part: &'a str, draw_order: f32, cells: (u16, u16)) -> Sheet<'a> {
    Sheet {
        id,
        part,
        parent: None,
        space: IN_HEAD,
        draw_order,
        cells,
    }
}

/// 回転デフォーマ `deformer` (原点 `origin`) の子の絵。
fn on_pivot<'a>(
    id: &'a str,
    part: &'a str,
    (deformer, origin): (&'a str, Vec2),
    draw_order: f32,
    cells: (u16, u16),
) -> Sheet<'a> {
    Sheet {
        id,
        part,
        parent: Some(deformer),
        space: Space::Rotation(origin),
        draw_order,
        cells,
    }
}

/// 回転デフォーマ `deformer` を、長さ `length` (px) の振り子で揺らす動的物理のリグ。
/// 根元はデフォーマの原点で、頭と一緒に動く。`AngleX` = ±30 で根元を `shift` px ずらす入力も加える。
fn pendulum(deformer: &str, length: f32, shift: f32, scale: f32) -> PhysicsRig {
    let range = |r: f32| NormalizationRange {
        min: -r,
        default: 0.0,
        max: r,
    };
    PhysicsRig {
        id: deformer.to_owned(),
        name: deformer.to_owned(),
        inputs: vec![PhysicsInput {
            parameter: sp::ANGLE_X.into(),
            source: PhysicsSource::X,
            weight: 100.0,
            reflect: false,
        }],
        outputs: vec![PhysicsOutput {
            target: PhysicsTarget::DeformerAngle {
                deformer: deformer.into(),
            },
            particle: 1,
            source: PhysicsSource::Angle,
            scale,
            weight: 100.0,
            reflect: false,
            blend: PhysicsBlend::Overwrite,
        }],
        particles: vec![
            PhysicsParticle {
                position: Vec2::ZERO,
                mobility: 1.0,
                delay: 1.0,
                acceleration: 1.0,
                radius: 0.0,
                collision_radius: 0.0,
            },
            PhysicsParticle {
                position: Vec2::new(0.0, length),
                mobility: 0.9,
                delay: 0.8,
                acceleration: 1.5,
                radius: length,
                collision_radius: 0.0,
            },
        ],
        normalization: PhysicsNormalization {
            position: range(shift),
            angle: range(10.0),
        },
        colliders: Vec::new(),
        frame: None,
    }
}

/// `LongHair` (スロット `Hair`): 紺の長い髪。後ろ髪は頭の中心を軸に物理で揺れ、本体の肌 (`Skin`) を
/// 反転マスクにして体の奥に見せる。顔の横の房は胸の前に垂れる。
fn long_hair() -> ModuleFile {
    let palette = Palette {
        fill: rgb(0x3a3f6e),
        line: rgb(0x1e2140),
        sheen: rgb(0x6068a8),
    };
    let mut builder = ModuleBuilder::new("LongHair", "Hair", "Hair");
    builder.module.deformers.push(pivot("Sway", "Hair", HEAD));
    let back = Shape::circle(HEAD, 110.0).union(Shape::rounded_rect(
        Vec2::new(140.0, 196.0),
        Vec2::new(372.0, 470.0),
        46.0,
    ));
    let art = hair_back(&palette, back);
    builder
        .add(on_pivot("Back", "Hair", ("Sway", HEAD), 0.0, (4, 6)), &art)
        .masks = vec![MaskRef::inverted("Skin")];
    let art = bangs(
        &palette,
        &Bangs {
            locks: vec![
                (184.0, 176.0, 18.0),
                (214.0, 188.0, 18.0),
                (244.0, 180.0, 15.0),
                (274.0, 186.0, 17.0),
                (302.0, 190.0, 18.0),
                (330.0, 178.0, 17.0),
            ],
            side: vec![
                (Vec2::new(176.0, 168.0), 15.0),
                (Vec2::new(168.0, 250.0), 14.0),
                (Vec2::new(166.0, 330.0), 11.0),
                (Vec2::new(170.0, 384.0), 4.0),
            ],
        },
    );
    builder.add(on_head("Front", "Hair", 1000.0, (4, 6)), &art);
    builder.module.physics.rigs = vec![pendulum("Sway", 250.0, 10.0, 0.6)];
    builder.finish()
}

/// `TwinTails` (スロット `Hair`): 金のツインテール。房は結び目を軸に物理で揺れる。
fn twin_tails() -> ModuleFile {
    let palette = Palette {
        fill: rgb(0xf2c35e),
        line: rgb(0x9c7128),
        sheen: rgb(0xffe39a),
    };
    let mut builder = ModuleBuilder::new("TwinTails", "Hair", "Hair");
    let back = Shape::circle(HEAD, 106.0).union(Shape::rounded_rect(
        Vec2::new(150.0, 196.0),
        Vec2::new(362.0, 276.0),
        26.0,
    ));
    let art = hair_back(&palette, back);
    builder.add(on_head("Back", "Hair", 100.0, (4, 4)), &art);

    // 画面の右 (キャラクターの左) の房。左の房は左右を入れ替える。
    let tie = Vec2::new(350.0, 128.0);
    let tail = [
        (tie, 12.0),
        (Vec2::new(374.0, 160.0), 21.0),
        (Vec2::new(388.0, 215.0), 26.0),
        (Vec2::new(392.0, 275.0), 25.0),
        (Vec2::new(386.0, 335.0), 21.0),
        (Vec2::new(374.0, 385.0), 14.0),
        (Vec2::new(362.0, 425.0), 4.0),
    ];
    let red = (rgb(0xe24d63), rgb(0x8e2536));
    for (side, flip) in [("L", false), ("R", true)] {
        let at = |p: Vec2| if flip { mirror(p) } else { p };
        let deformer = format!("Tail{side}");
        let origin = at(tie);
        builder
            .module
            .deformers
            .push(pivot(&deformer, "Hair", origin));
        let points: Vec<(Vec2, f32)> = tail.iter().map(|(p, r)| (at(*p), *r)).collect();
        let shape = Shape::stroke(&points);
        let (min, max) = bounds_of(&shape, Vec2::new(60.0, 40.0), Vec2::new(452.0, 512.0));
        let mut art = Layer::new(min, max);
        art.paint(&shape, palette.fill, palette.line, LINE);
        // 房の中ほどのつや。
        let sheen: Vec<(Vec2, f32)> = tail[1..5]
            .iter()
            .map(|(p, _)| (at(*p + Vec2::new(-6.0, 0.0)), 3.0))
            .collect();
        art.fill(
            &Shape::stroke(&sheen).intersect(shape.grow(-8.0)),
            palette.sheen,
        );
        let id = format!("Tail{side}Mesh");
        builder.add(
            on_pivot(&id, "Hair", (&deformer, origin), 50.0, (3, 6)),
            &art,
        );
        // 結び目のリボン。
        let bow = Shape::polygon(&[
            origin,
            at(tie + Vec2::new(-18.0, -14.0)),
            at(tie + Vec2::new(-16.0, 14.0)),
        ])
        .union(Shape::polygon(&[
            origin,
            at(tie + Vec2::new(18.0, -14.0)),
            at(tie + Vec2::new(16.0, 14.0)),
        ]))
        .union(Shape::circle(origin, 6.0));
        let mut art = Layer::new(origin - Vec2::splat(24.0), origin + Vec2::splat(24.0));
        art.paint(&bow, red.0, red.1, 2.0);
        let id = format!("Ribbon{side}");
        builder.add(
            on_pivot(&id, "Hair", (&deformer, origin), 950.0, (1, 1)),
            &art,
        );
        builder
            .module
            .physics
            .rigs
            .push(pendulum(&deformer, 280.0, 10.0, 1.0));
    }

    let art = bangs(
        &palette,
        &Bangs {
            locks: vec![
                (186.0, 174.0, 15.0),
                (212.0, 182.0, 15.0),
                (238.0, 186.0, 15.0),
                (264.0, 186.0, 15.0),
                (290.0, 182.0, 15.0),
                (318.0, 176.0, 15.0),
            ],
            side: vec![
                (Vec2::new(178.0, 172.0), 11.0),
                (Vec2::new(174.0, 230.0), 9.0),
                (Vec2::new(175.0, 262.0), 4.0),
            ],
        },
    );
    builder.add(on_head("Front", "Hair", 1000.0, (4, 4)), &art);
    builder.finish()
}

/// `CatEars` (スロット `Headwear`): 猫耳のカチューシャ。
fn cat_ears() -> ModuleFile {
    let (fill, line, inner) = (rgb(0x3b3440), rgb(0x1d1920), rgb(0xf6a9bf));
    let mut builder = ModuleBuilder::new("CatEars", "Headwear", "CatEars");
    let ear = |points: [Vec2; 3]| Shape::polygon(&points).grow(4.0);
    let left = [
        Vec2::new(176.0, 146.0),
        Vec2::new(184.0, 66.0),
        Vec2::new(238.0, 112.0),
    ];
    let inner_left = [
        Vec2::new(190.0, 130.0),
        Vec2::new(194.0, 88.0),
        Vec2::new(224.0, 114.0),
    ];
    let mut art = Layer::new(Vec2::new(160.0, 54.0), Vec2::new(352.0, 160.0));
    for flip in [false, true] {
        let at = |p: Vec2| if flip { mirror(p) } else { p };
        art.paint(&ear(left.map(at)), fill, line, LINE);
        art.fill(&ear(inner_left.map(at)).grow(-3.0), inner);
    }
    builder.add(on_head("Ears", "CatEars", 0.0, (2, 1)), &art);
    let band = Shape::ring(HEAD, 104.0, 5.0).intersect(Shape::above(150.0));
    let mut art = Layer::new(Vec2::new(144.0, 86.0), Vec2::new(368.0, 156.0));
    art.paint(&band, fill, line, 2.0);
    builder.add(on_head("Band", "CatEars", 1000.0, (4, 1)), &art);
    builder.finish()
}

/// `Sailor` (スロット `Outfit`): セーラー服。体のワープに付き、既定のシャツ (`Clothes`) を置き換える。
fn sailor() -> ModuleFile {
    let mut builder = ModuleBuilder::new("Sailor", "Outfit", "Sailor");
    let on_body = |id, draw_order, cells| Sheet {
        id,
        part: "Sailor",
        parent: None,
        space: IN_BODY,
        draw_order,
        cells,
    };
    let bounds = (Vec2::new(160.0, 320.0), Vec2::new(352.0, 512.0));

    let top = torso().subtract(Shape::polygon(&[
        Vec2::new(228.0, 320.0),
        Vec2::new(284.0, 320.0),
        Vec2::new(256.0, 378.0),
    ]));
    let mut art = Layer::new(bounds.0, bounds.1);
    art.paint(&top, SHIRT, SHIRT_LINE, LINE);
    builder.add(on_body("Top", 0.0, (4, 4)), &art);

    let collar = Shape::polygon(&[
        Vec2::new(170.0, 352.0),
        Vec2::new(204.0, 330.0),
        Vec2::new(308.0, 330.0),
        Vec2::new(342.0, 352.0),
        Vec2::new(256.0, 430.0),
    ])
    .subtract(Shape::polygon(&[
        Vec2::new(224.0, 322.0),
        Vec2::new(288.0, 322.0),
        Vec2::new(256.0, 382.0),
    ]))
    .intersect(torso());
    let mut art = Layer::new(bounds.0, bounds.1);
    art.paint(&collar, rgb(0x26335f), rgb(0x141b35), 2.0);
    art.fill(
        &collar.clone().grow(-5.0).subtract(collar.grow(-7.0)),
        WHITE,
    );
    builder.add(on_body("Collar", 500.0, (4, 4)), &art);

    let scarf = Shape::polygon(&[
        Vec2::new(236.0, 402.0),
        Vec2::new(276.0, 402.0),
        Vec2::new(262.0, 452.0),
        Vec2::new(256.0, 444.0),
        Vec2::new(250.0, 452.0),
    ])
    .union(Shape::circle(Vec2::new(256.0, 402.0), 9.0));
    let mut art = Layer::new(Vec2::new(228.0, 388.0), Vec2::new(284.0, 458.0));
    art.paint(&scarf, rgb(0xd8404f), rgb(0x7e1f2a), 2.0);
    builder.add(on_body("Scarf", 1000.0, (1, 1)), &art);
    builder.finish()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ugoe_runtime::{CompiledModel, ModelInstance};

    use super::*;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    /// コミットした `assets/samples/dress-up/` が、今のジェネレータの出力とバイト列で同じこと。
    /// 違えば `just cli sample dress-up -o assets/samples/dress-up` で作り直す。
    #[test]
    fn committed_sample_is_up_to_date() {
        let sample = build_dress_up().unwrap();
        let files = sample.files().unwrap();
        let dir = repo_path("assets/samples/dress-up");
        for (name, bytes) in &files {
            let committed = std::fs::read(dir.join(name))
                .unwrap_or_else(|e| panic!("{name}: {e} (regenerate assets/samples/dress-up)"));
            assert!(
                committed == *bytes,
                "{name} differs (regenerate assets/samples/dress-up)"
            );
        }
        let mut committed: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".ugoe") || n.ends_with(".ugoemod"))
            .collect();
        committed.sort();
        let mut generated: Vec<String> = files.into_iter().map(|(n, _)| n).collect();
        generated.sort();
        assert_eq!(committed, generated);
    }

    /// 本体はアバタープロファイルのレベル 2 に (警告もなく) 準拠し、スロットをすべて公開する。
    #[test]
    fn base_conforms_to_the_avatar_profile() {
        let sample = build_dress_up().unwrap();
        let model = &sample.base.model;
        let report = ugoe_model::avatar::check::check_profile(model);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        assert_eq!((report.claimed, report.satisfied), (Some(2), 2));
        let avatar = model.avatar.as_ref().unwrap();
        assert_eq!(avatar.slots, ["Hair", "Headwear", "Outfit"]);
        assert_eq!(avatar.hit_areas.len(), 2);
    }

    /// 書き出したプロジェクトをエディタの中核で開ける: 本体は見本の本体と同じモデル、モジュールのプロジェクトは
    /// 本体を参照し、書き出す (`export_module`) と見本の `.ugoemod` とバイト列で同じになる。
    #[test]
    fn projects_open_in_the_editor_core() {
        let sample = build_dress_up().unwrap();
        let dir = std::env::temp_dir().join(format!("ugoe-cli-dress-up-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = sample.save_projects(&dir).unwrap();
        let names: Vec<String> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "dress-up.ugoeproj",
                "long-hair.ugoeproj",
                "twin-tails.ugoeproj",
                "cat-ears.ugoeproj",
                "sailor.ugoeproj"
            ]
        );

        let base = Project::load(&written[0]).unwrap();
        assert!(base.module.is_none());
        assert_eq!(base.model, sample.base.model);
        assert_eq!(base.physics, sample.base.physics);
        assert_eq!(base.motions, sample.base.motions);
        assert_eq!(base.expressions, sample.base.expressions);
        let bundle =
            ugoe_format::read_bundle(&ugoe_editor_core::export_bundle(&base).unwrap()).unwrap();
        assert_eq!(bundle.textures, sample.base.textures);

        for (path, file) in written[1..].iter().zip(&sample.modules) {
            let meta: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path.join("project.json")).unwrap()).unwrap();
            assert_eq!(meta["kind"], "module");
            assert_eq!(meta["module"]["base"], "../dress-up.ugoeproj");
            assert_eq!(meta["module"]["slot"], file.module.slot.as_str());

            let project = Project::load(path).unwrap();
            let context = project.module.as_ref().unwrap();
            assert_eq!(context.name(), file.module.name);
            let document = project.document();
            assert!(document.module().is_some());
            let export = ugoe_editor_core::export_module(&project).unwrap();
            assert!(export.warnings.is_empty(), "{:?}", export.warnings);
            let exported = ugoe_format::read_module(&export.bytes).unwrap();
            assert_eq!(exported.module, file.module, "{}", file.module.name);
            assert!(
                export.bytes == ugoe_format::write_module(file).unwrap(),
                "{} is exported differently",
                file.module.name
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// モジュールのプロジェクトの作業用のモデルで、エディタの座標変換 (親の座標系 ⇔ キャンバス) が使える
    /// (スロットの取り付け先が別々のデフォーマでも。T0517 の `ancestors_only` の修正)。モジュールの
    /// メッシュとデフォーマの既定の姿の位置が評価の結果と一致し、キャンバスから戻すと元の値になる。
    #[test]
    fn module_projects_convert_coordinates() {
        use ugoe_editor_core::KeyformTarget;
        use ugoe_editor_core::space::{ParameterValues, canvas_to_local, local_to_canvas};

        let sample = build_dress_up().unwrap();
        let dir =
            std::env::temp_dir().join(format!("ugoe-cli-dress-up-space-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = sample.save_projects(&dir).unwrap();
        let values = ParameterValues::new();
        for path in &written[1..] {
            let project = Project::load(path).unwrap();
            let context = project.module.as_ref().unwrap();
            let model = &project.model;
            let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
            instance.update();
            let mut checked = 0;
            for (index, mesh) in model.meshes.iter().enumerate() {
                if context.is_base_mesh(&mesh.id) {
                    continue;
                }
                let target = KeyformTarget::Mesh(mesh.id.clone());
                let local = &mesh.keyforms.keyforms[0].positions;
                let canvas = local_to_canvas(model, &target, &values, local).unwrap();
                let evaluated = &instance.drawables()[index].positions;
                assert!(max_shift(&canvas, evaluated) < 0.01, "{}", mesh.id);
                let back = canvas_to_local(model, &target, &values, &canvas, local).unwrap();
                let error = max_shift(&back, local);
                // ワープの子は UV 空間 (0〜1) なので、キャンバスの 0.01px ほどの誤差。
                assert!(error < 1e-3, "{}: {error}", mesh.id);
                checked += 1;
            }
            for deformer in &model.deformers {
                if context.is_base_deformer(&deformer.id) {
                    continue;
                }
                let DeformerKind::Rotation(rotation) = &deformer.kind else {
                    continue;
                };
                let target = KeyformTarget::Deformer(deformer.id.clone());
                let origin = [rotation.keyforms.keyforms[0].origin];
                let canvas = local_to_canvas(model, &target, &values, &origin).unwrap();
                let back = canvas_to_local(model, &target, &values, &canvas, &origin).unwrap();
                assert!(max_shift(&back, &origin) < 1e-3, "{}", deformer.id);
                checked += 1;
            }
            assert!(checked >= 2, "{}", path.display());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn modules_share_slots_and_fit_the_base() {
        let sample = build_dress_up().unwrap();
        let slots: Vec<&str> = sample
            .modules
            .iter()
            .map(|f| f.module.slot.as_str())
            .collect();
        assert_eq!(slots, ["Hair", "Hair", "Headwear", "Outfit"]);
        for file in &sample.modules {
            let module = &file.module;
            assert_eq!(
                module.textures.len(),
                file.textures.len(),
                "{}",
                module.name
            );
            assert!(
                sample.base.model.slots.iter().any(|s| s.id == module.slot),
                "{}",
                module.name
            );
        }
        let files = sample.files().unwrap();
        let total: usize = files.iter().map(|(_, b)| b.len()).sum();
        assert!(total < 200_000, "the sample is {total} bytes");
    }

    /// パラメータの値 `params` で評価した、メッシュ `id` の頂点位置 (キャンバス座標)。
    fn positions(model: &Model, params: &[(&str, f32)], id: &str) -> Vec<Vec2> {
        let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
        for (param, value) in params {
            let parameters = instance.parameters_mut();
            let index = parameters.index_of(param).unwrap();
            parameters.set(index, *value);
        }
        instance.update();
        let index = model
            .meshes
            .iter()
            .position(|m| m.id.as_str() == id)
            .unwrap();
        instance.drawables()[index].positions.clone()
    }

    fn max_shift(a: &[Vec2], b: &[Vec2]) -> f32 {
        a.iter()
            .zip(b)
            .map(|(p, q)| p.distance(*q))
            .fold(0.0, f32::max)
    }

    /// 既定の姿 (口は開く) では、どのメッシュも描いた位置 (キャンバス座標で絵を置いた矩形) にある。
    #[test]
    fn meshes_sit_where_they_were_drawn() {
        let sample = build_dress_up().unwrap();
        let base = &sample.base.model;
        let refs: Vec<&Module> = sample.modules.iter().map(|f| &f.module).collect();
        for set in [&refs[..1], &refs[1..]] {
            let model = ugoe_runtime::compose(base, set).unwrap();
            for mesh in &model.meshes {
                let evaluated = positions(&model, &[(sp::MOUTH_OPEN_Y, 1.0)], mesh.id.as_str());
                let rest: Vec<Vec2> = mesh.keyforms.keyforms[0].positions.clone();
                // 格子の四隅が、絵の矩形の四隅 (整数の座標) になる。
                let (min, max) = evaluated.iter().fold(
                    (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
                    |(lo, hi), p| (lo.min(*p), hi.max(*p)),
                );
                assert!(
                    (min - min.round()).abs().max_element() < 0.01
                        && (max - max.round()).abs().max_element() < 0.01,
                    "{}: {min} {max} (rest {:?})",
                    mesh.id,
                    rest.first()
                );
            }
        }
    }

    /// 頭を動かすと顔と付けた髪・耳が一緒に動き、目と口のパラメータがその部分だけを動かす。
    #[test]
    fn parameters_move_the_head_and_modules_follow() {
        let sample = build_dress_up().unwrap();
        let base = &sample.base.model;
        let refs: Vec<&Module> = sample.modules.iter().map(|f| &f.module).collect();
        let model = ugoe_runtime::compose(base, &[refs[0], refs[2], refs[3]]).unwrap();
        for (param, value) in [
            (sp::ANGLE_X, 30.0),
            (sp::ANGLE_Y, -30.0),
            (sp::ANGLE_Z, 30.0),
            (sp::BODY_ANGLE_X, 10.0),
        ] {
            for id in ["Face", "LongHair/Front", "CatEars/Band"] {
                let moved = max_shift(
                    &positions(&model, &[], id),
                    &positions(&model, &[(param, value)], id),
                );
                assert!(moved > 3.0, "{param} = {value} moves {id} only {moved} px");
            }
        }
        let still = positions(&model, &[], "Sailor/Collar");
        let breath = positions(&model, &[(sp::BREATH, 1.0)], "Sailor/Collar");
        assert!(max_shift(&still, &breath) > 1.0);
        let open = positions(&model, &[], "EyeL");
        let closed = positions(&model, &[(sp::EYE_L_OPEN, 0.0)], "EyeL");
        assert!(max_shift(&open, &closed) > 10.0);
        assert_eq!(
            max_shift(
                &positions(&model, &[], "EyeR"),
                &positions(&model, &[(sp::EYE_L_OPEN, 0.0)], "EyeR")
            ),
            0.0
        );
    }

    /// レベル 2 のパラメータが、それぞれの部分だけを動かす: 視線は瞳、眉は片方の眉、口の形は口、
    /// 体の Y・Z は体と、その上に載る頭。
    #[test]
    fn level_2_parameters_move_their_parts() {
        let sample = build_dress_up().unwrap();
        let model = &sample.base.model;
        let moved = |param: &str, value: f32, id: &str| {
            max_shift(
                &positions(model, &[], id),
                &positions(model, &[(param, value)], id),
            )
        };
        for (param, value, moving, still) in [
            (sp::EYE_BALL_X, 1.0, "EyeLIris", "EyeL"),
            (sp::EYE_BALL_Y, -1.0, "EyeRIris", "EyeR"),
            (sp::BROW_L_Y, 1.0, "BrowL", "BrowR"),
            (sp::BROW_R_Y, -1.0, "BrowR", "BrowL"),
            (sp::MOUTH_FORM, 1.0, "Mouth", "Face"),
            (sp::BODY_ANGLE_Y, 10.0, "Skin", "HairSway"),
            (sp::BODY_ANGLE_Z, 10.0, "Face", "HairSway"),
            (sp::BODY_ANGLE_Z, -10.0, "Skin", "HairSway"),
        ] {
            let m = moved(param, value, moving);
            assert!(m > 2.0, "{param} = {value} moves {moving} only {m} px");
            if still != "HairSway" {
                assert_eq!(moved(param, value, still), 0.0, "{param} moves {still}");
            }
        }
        // 視線は正でキャラクターの左 (画面の右)・上。笑顔 (`MouthForm` = 1) で口角が上がる。
        let iris = |params: &[(&str, f32)]| positions(model, params, "EyeLIris")[0];
        assert!(iris(&[(sp::EYE_BALL_X, 1.0)]).x > iris(&[]).x);
        assert!(iris(&[(sp::EYE_BALL_Y, 1.0)]).y < iris(&[]).y);
        let corner = |form: f32| positions(model, &[(sp::MOUTH_FORM, form)], "Mouth")[0].y;
        assert!(corner(1.0) < corner(0.0) && corner(0.0) < corner(-1.0));
    }

    /// 本体の物理 (既定の後ろ髪の揺れ) が、頭を振ると揺れて止まる。
    #[test]
    fn base_hair_sways_with_the_base_physics() {
        let sample = build_dress_up().unwrap();
        let physics = sample.base.physics.clone().unwrap();
        assert_eq!(physics.rigs.len(), 1);
        let mut puppet = ugoe::Puppet::new(sample.base.model.clone())
            .unwrap()
            .with_physics(physics);
        puppet.set_toggles(ugoe::Toggles {
            eye_blink: false,
            breath: false,
            ..ugoe::Toggles::default()
        });
        let model = &sample.base.model;
        let index = model
            .meshes
            .iter()
            .position(|m| m.id.as_str() == "HairBack")
            .unwrap();
        let angle = puppet
            .instance()
            .parameters()
            .index_of(sp::ANGLE_X)
            .unwrap();
        let rest = puppet.instance().drawables()[index].positions.clone();
        let mut peak = 0.0f32;
        for frame in 0..360 {
            let value = if frame < 48 && (frame / 12) % 2 == 0 {
                30.0
            } else if frame < 48 {
                -30.0
            } else {
                0.0
            };
            puppet.instance_mut().parameters_mut().set(angle, value);
            puppet.update(1.0 / 60.0);
            let still = positions(model, &[(sp::ANGLE_X, value)], "HairBack");
            peak = peak.max(max_shift(
                &puppet.instance().drawables()[index].positions,
                &still,
            ));
        }
        assert!(peak > 2.0 && peak < 60.0, "peak {peak}");
        let settled = max_shift(&puppet.instance().drawables()[index].positions, &rest);
        assert!(settled < 1.0, "did not settle: {settled}");
    }

    /// 頭を振ると、髪のモジュールの揺れものが物理で遅れて揺れ、止めると元に戻る。
    #[test]
    fn hair_modules_sway_after_a_head_shake() {
        let sample = build_dress_up().unwrap();
        for (index, meshes) in [
            (0, &["LongHair/Back"][..]),
            (1, &["TwinTails/TailLMesh", "TwinTails/TailRMesh"][..]),
        ] {
            let mut puppet = ugoe::Puppet::new(sample.base.model.clone()).unwrap();
            puppet.set_toggles(ugoe::Toggles {
                eye_blink: false,
                breath: false,
                ..ugoe::Toggles::default()
            });
            puppet
                .attach_module(std::sync::Arc::new(sample.modules[index].module.clone()))
                .unwrap();
            let model = puppet.instance().compiled().model().clone();
            let find = |id: &str| {
                model
                    .meshes
                    .iter()
                    .position(|m| m.id.as_str() == id)
                    .unwrap()
            };
            let angle = puppet
                .instance()
                .parameters()
                .index_of(sp::ANGLE_X)
                .unwrap();
            let rest: Vec<Vec<Vec2>> = meshes
                .iter()
                .map(|id| puppet.instance().drawables()[find(id)].positions.clone())
                .collect();
            let mut peak = 0.0f32;
            const FPS: f32 = 60.0;
            for frame in 0..(6.0 * FPS) as usize {
                let t = frame as f32 / FPS;
                // 0.2 秒ごとに ±30 を 4 回行き来してから止める。
                let value = if t < 0.8 {
                    if ((t / 0.2) as usize).is_multiple_of(2) {
                        30.0
                    } else {
                        -30.0
                    }
                } else {
                    0.0
                };
                puppet.instance_mut().parameters_mut().set(angle, value);
                puppet.update(1.0 / FPS);
                let still = positions(&model, &[(sp::ANGLE_X, value)], meshes[0]);
                let now = &puppet.instance().drawables()[find(meshes[0])].positions;
                peak = peak.max(max_shift(now, &still));
            }
            assert!(peak > 5.0 && peak < 120.0, "{meshes:?}: peak {peak}");
            for (id, rest) in meshes.iter().zip(&rest) {
                let now = &puppet.instance().drawables()[find(id)].positions;
                let settled = max_shift(now, rest);
                assert!(settled < 1.0, "{id} did not settle: {settled}");
            }
        }
    }
}
