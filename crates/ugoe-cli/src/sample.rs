//! `ugoe sample avatar`: アバタープロファイル (`docs/spec/avatar-profile.md`) に準拠した見本のモデルを、
//! PSD から組み立てる (T0702、T0704 でバストアップの PSD に作り直した)。
//!
//! リグはすべて `ugoe_editor_core::Command` の列で組む (エディタで手で組むのと同じ操作)。
//! 部位はレイヤーの名前で見つけ、変形の量はその部位の外接矩形の大きさに対する割合で決めるので、
//! 座標の定数は持たない。`Model.avatar` と当たり判定だけは設定するコマンドがまだないので、組み終えた
//! モデルに直接書く ([`avatar_profile`])。
//!
//! 入力は `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd` (胸から上の絵を
//! 目・白目・まつげ・眉・口などに分けたもの)。目は開き (`eyewhite`・`irides`・`eyelash`) と
//! 閉じ (`eyeclose`)、口は閉じ (`mouth-close`) と開き (`mouth-open`) の別のレイヤーがあるので、
//! 不透明度と変形でつなぐ。

pub mod dress_up;
pub mod lit;
mod paint;

use std::collections::BTreeMap;

use anyhow::{Context, anyhow, bail};
use ugoe_core::Vec2;
use ugoe_editor_core::space::{ParameterValues, canvas_to_local, local_to_canvas};
use ugoe_editor_core::{
    ChildRef, Command, DepthPreset, Document, InferMethod, Keyform, KeyformCell, KeyformTarget,
    Project, PsdImportOptions,
};
use ugoe_model::avatar::{
    AvatarHitArea, AvatarMeta, AvatarPermission, CommercialUsage, CreditNotation, HIT_AREA_BODY,
    HIT_AREA_HEAD, Modification,
};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::physics::{
    NormalizationRange, PhysicsBlend, PhysicsInput, PhysicsNormalization, PhysicsOutput,
    PhysicsParticle, PhysicsRig, PhysicsSource, PhysicsTarget, VertexOwner,
};
use ugoe_model::standard as sp;
use ugoe_model::{
    AvatarProfile, DeformerId, DeformerKind, DepthRotation, HitArea, KeyformGrid, KeyformInference,
    KeyformInterpolation, MaskRef, MeshId, Model, ParamId, Parameter,
};

/// 既定の入力 (リポジトリのルートからの相対パス)。
pub const DEFAULT_PSD: &str = "assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd";

/// 見本のモデルの名前 (プロジェクト名と `Model.avatar.meta.name`)。
const NAME: &str = "ugoe sample avatar";

/// 使うパラメータ。値域は標準の値 (`ugoe_model::standard`)。
const PARAMETERS: &[&str] = &[
    sp::ANGLE_X,
    sp::ANGLE_Y,
    sp::ANGLE_Z,
    sp::BODY_ANGLE_X,
    sp::BODY_ANGLE_Y,
    sp::BODY_ANGLE_Z,
    sp::BREATH,
    sp::EYE_L_OPEN,
    sp::EYE_R_OPEN,
    sp::EYE_L_SMILE,
    sp::EYE_R_SMILE,
    sp::EYE_L_WIDE,
    sp::EYE_R_WIDE,
    sp::EYE_BALL_X,
    sp::EYE_BALL_Y,
    sp::BROW_L_Y,
    sp::BROW_R_Y,
    sp::BROW_L_ANGLE,
    sp::BROW_R_ANGLE,
    sp::BROW_INNER_UP,
    sp::MOUTH_OPEN_Y,
    sp::MOUTH_FORM,
    sp::MOUTH_X,
    sp::JAW_X,
    sp::MOUTH_PUCKER,
    sp::MOUTH_PRESS,
    sp::MOUTH_SHRUG,
];

/// 左右の対のパラメータ (エディタのミラー用)。同じ ID は符号を反転して対応する。
const MIRRORS: &[(&str, &str)] = &[
    (sp::ANGLE_X, sp::ANGLE_X),
    (sp::ANGLE_Z, sp::ANGLE_Z),
    (sp::BODY_ANGLE_X, sp::BODY_ANGLE_X),
    (sp::BODY_ANGLE_Z, sp::BODY_ANGLE_Z),
    (sp::EYE_BALL_X, sp::EYE_BALL_X),
    (sp::MOUTH_X, sp::MOUTH_X),
    (sp::JAW_X, sp::JAW_X),
    (sp::EYE_L_OPEN, sp::EYE_R_OPEN),
    (sp::EYE_L_SMILE, sp::EYE_R_SMILE),
    (sp::EYE_L_WIDE, sp::EYE_R_WIDE),
    (sp::BROW_L_Y, sp::BROW_R_Y),
    (sp::BROW_L_ANGLE, sp::BROW_R_ANGLE),
];

/// 頭に属するレイヤー (頭の回転で傾く)。`back hair` 以外は顔のワープに入る。
const HEAD_LAYERS: &[&str] = &[
    "back hair",
    "face",
    "ears-l",
    "ears-r",
    "earwear-l",
    "earwear-r",
    "nose",
    "mouth-open",
    "mouth-close",
    "eyewhite-l",
    "eyewhite-r",
    "irides-l",
    "irides-r",
    "eyelash-l",
    "eyelash-r",
    "eyeclose-l",
    "eyeclose-r",
    "eyebrow-l",
    "eyebrow-r",
    "front hair",
];

/// 目の開き (`EyeLOpen`) による、開いた目と閉じた目の切り替え。
///
/// 開いた目は閉じる線へ縦につぶれていく (1 から `SQUASH_KEY` までで大きく)。つぶれて細くなった所で、閉じた目を先に不透明にしてから
/// 開いた目を消すので、途中の値で目が消えたり、半透明の 2 つの目が重なったりしない。
mod eye_fade {
    /// 閉じる線の高さ (開いた目の外接矩形の上端 0〜下端 1)。
    pub const LINE: f32 = 0.7;
    /// 開いた目を大きくつぶすキーと、そこでの高さ (開いた目に対する割合)。
    pub const SQUASH_KEY: f32 = 0.35;
    pub const SQUASH_HEIGHT: f32 = 0.22;
    /// 閉じた目 (笑い目 = 0) の弧を平らにする量 (閉じた目の絵の高さに対する割合)。
    pub const FLATTEN: f32 = 0.5;
    /// 閉じた目を平らなまま保つ笑い目の値。これより上で絵の笑った閉じ目へ移る。
    pub const SMILE_FLAT: f32 = 0.5;
    /// 閉じた目が現れ始める値 (これより上では透明)。
    pub const CLOSED_IN: f32 = 0.36;
    /// 閉じた目が不透明になる値。開いた目はここまで不透明。
    pub const CLOSED_FULL: f32 = 0.3;
    /// 開いた目が消える値。
    pub const OPEN_OUT: f32 = 0.22;
}

/// 口の開き (`MouthOpenY`) による、閉じた口と開いた口の切り替え。
///
/// 開いた口は閉じた口の大きさの細い隙間から開く。隙間が閉じた口の下で不透明になってから、閉じた口を消す。
mod mouth_fade {
    /// 開いた口 (細い隙間) が不透明になる値。
    pub const OPEN_FULL: f32 = 0.12;
    /// 閉じた口が消え始める値 (開いた口が不透明になってから)。
    pub const CLOSED_OUT_START: f32 = OPEN_FULL;
    /// 閉じた口が消える値。
    pub const CLOSED_OUT: f32 = 0.3;
}

/// 片目の部位と、その目のパラメータ。
struct EyeSide {
    /// `l` / `r` (レイヤー名の接尾辞)。
    suffix: &'static str,
    open: &'static str,
    smile: &'static str,
    wide: &'static str,
    brow_y: &'static str,
    brow_angle: &'static str,
}

const EYES: [EyeSide; 2] = [
    EyeSide {
        suffix: "l",
        open: sp::EYE_L_OPEN,
        smile: sp::EYE_L_SMILE,
        wide: sp::EYE_L_WIDE,
        brow_y: sp::BROW_L_Y,
        brow_angle: sp::BROW_L_ANGLE,
    },
    EyeSide {
        suffix: "r",
        open: sp::EYE_R_OPEN,
        smile: sp::EYE_R_SMILE,
        wide: sp::EYE_R_WIDE,
        brow_y: sp::BROW_R_Y,
        brow_angle: sp::BROW_R_ANGLE,
    },
];

/// PSD から見本のモデルのプロジェクトを作る。
pub fn build_avatar(psd: &[u8]) -> anyhow::Result<Project> {
    let empty = Project::empty(NAME);
    let mut document = empty.document();
    let import =
        ugoe_editor_core::import::import_psd(document.model(), psd, &PsdImportOptions::new(NAME))
            .context("importing the PSD")?;
    document.apply(import.command)?;

    let mut rig = Rig { document };
    let layout = Layout {
        face: rig.mesh_bounds("face")?,
        canvas: Vec2::new(
            rig.model().meta.canvas.width,
            rig.model().meta.canvas.height,
        ),
    };
    rig.add_parameters()?;
    rig.rig_body(&layout)?;
    rig.rig_head(&layout)?;

    let face = rig.mesh("face")?;
    let body = rig.mesh("topwear")?;
    rig.apply(Command::SetHitAreas {
        hit_areas: vec![
            HitArea {
                id: HIT_AREA_HEAD.to_owned(),
                mesh: face,
            },
            HitArea {
                id: HIT_AREA_BODY.to_owned(),
                mesh: body,
            },
        ],
    })?;
    rig.apply(Command::SetAvatarProfile {
        profile: Some(avatar_profile()),
    })?;
    Ok(Project::from_document(empty.meta, &rig.document))
}

/// 見本のモデルのアバタープロファイル (レベル 3)。絵は MIT で配布している PSD から作った。
fn avatar_profile() -> AvatarProfile {
    AvatarProfile {
        level: 3,
        meta: AvatarMeta {
            name: NAME.to_owned(),
            version: Some("0.2.0".to_owned()),
            authors: vec!["ugoe contributors".to_owned()],
            references: vec![DEFAULT_PSD.to_owned()],
            license_url: Some("https://opensource.org/license/mit".to_owned()),
            avatar_permission: AvatarPermission::Everyone,
            commercial_usage: CommercialUsage::Corporation,
            credit_notation: CreditNotation::Unnecessary,
            allow_redistribution: true,
            modification: Modification::AllowRedistribution,
            third_party_licenses: Some(
                "Art: AI-generated PSD (ChatGPT + ComfyUI See-Through) provided by the ugoe \
                 maintainers under the MIT license."
                    .to_owned(),
            ),
            ..AvatarMeta::default()
        },
        hit_areas: vec![
            AvatarHitArea {
                name: HIT_AREA_HEAD.to_owned(),
                hit_area: HIT_AREA_HEAD.to_owned(),
            },
            AvatarHitArea {
                name: HIT_AREA_BODY.to_owned(),
                hit_area: HIT_AREA_BODY.to_owned(),
            },
        ],
        ..AvatarProfile::default()
    }
}

/// キャンバス座標の軸に平行な矩形。
#[derive(Debug, Clone, Copy)]
struct Rect {
    min: Vec2,
    max: Vec2,
}

impl Rect {
    fn of(points: &[Vec2]) -> Option<Self> {
        let first = *points.first()?;
        let (min, max) = points
            .iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        Some(Self { min, max })
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn size(self) -> Vec2 {
        self.max - self.min
    }

    fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// 点 `p` の、矩形の中での位置 (左上 (0,0)〜右下 (1,1))。
    fn normalize(self, p: Vec2) -> Vec2 {
        let size = self.size().max(Vec2::splat(f32::EPSILON));
        (p - self.min) / size
    }
}

/// 横の位置 `x` (0〜1) での、中ほどが 1・両端が 0 の弧。
fn arc(x: f32) -> f32 {
    1.0 - (2.0 * x - 1.0).powi(2)
}

/// 横の位置 `x` (0〜1) での、両端が 1・中ほどが 0 の重み (口角)。
fn corner(x: f32) -> f32 {
    (2.0 * x - 1.0).powi(2)
}

/// 変形の大きさの基準。
struct Layout {
    /// 顔 (`face`) の外接矩形。
    face: Rect,
    /// キャンバスの大きさ。
    canvas: Vec2,
}

/// 組み立て中のドキュメントと、要素の探索・形の編集の補助。
struct Rig {
    document: Document,
}

impl Rig {
    fn model(&self) -> &Model {
        self.document.model()
    }

    fn apply(&mut self, command: Command) -> anyhow::Result<()> {
        self.document.apply(command)?;
        Ok(())
    }

    /// レイヤー名 (メッシュの名前) からメッシュの ID を探す。
    fn mesh(&self, name: &str) -> anyhow::Result<MeshId> {
        self.model()
            .meshes
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.id.clone())
            .ok_or_else(|| anyhow!("the PSD has no layer named `{name}`"))
    }

    fn meshes(&self, names: &[&str]) -> anyhow::Result<Vec<ChildRef>> {
        names
            .iter()
            .map(|name| self.mesh(name).map(ChildRef::Mesh))
            .collect()
    }

    /// 要素の基準の姿 (すべてのパラメータが既定値) の点 (親の座標系) と、その格子の基準のセル。
    fn base(&self, target: &KeyformTarget) -> anyhow::Result<(Vec<Vec2>, Keyform, KeyformCell)> {
        self.keyform_at(target, &[])
    }

    /// 要素の格子のセル `cell` (書かない軸は既定値) の点 (親の座標系)・キーフォームと、すべての軸を書いたセル。
    fn keyform_at(
        &self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
    ) -> anyhow::Result<(Vec<Vec2>, Keyform, KeyformCell)> {
        let model = self.model();
        let mut values: BTreeMap<ParamId, f32> = model
            .parameters
            .iter()
            .map(|p| (p.id.clone(), p.default))
            .collect();
        for (parameter, value) in cell {
            values.insert(ParamId::new(*parameter), *value);
        }
        fn find<T: Clone>(
            grid: &KeyformGrid<T>,
            values: &BTreeMap<ParamId, f32>,
        ) -> anyhow::Result<(T, KeyformCell)> {
            let mut index = 0;
            let mut stride = 1;
            let mut cell = KeyformCell::new();
            for binding in &grid.bindings {
                let value = values.get(&binding.parameter).copied().unwrap_or(0.0);
                let key = binding
                    .keys
                    .iter()
                    .position(|k| *k == value)
                    .ok_or_else(|| anyhow!("axis {} has no key {value}", binding.parameter))?;
                index += key * stride;
                stride *= binding.keys.len();
                cell.insert(binding.parameter.clone(), value);
            }
            Ok((grid.keyforms[index].clone(), cell))
        }
        match target {
            KeyformTarget::Mesh(id) => {
                let mesh = model
                    .meshes
                    .iter()
                    .find(|m| m.id == *id)
                    .ok_or_else(|| anyhow!("no mesh `{id}`"))?;
                let (keyform, cell) = find(&mesh.keyforms, &values)?;
                Ok((keyform.positions.clone(), Keyform::Mesh(keyform), cell))
            }
            KeyformTarget::Deformer(id) => {
                let deformer = model
                    .deformers
                    .iter()
                    .find(|d| d.id == *id)
                    .ok_or_else(|| anyhow!("no deformer `{id}`"))?;
                match &deformer.kind {
                    DeformerKind::Warp(warp) => {
                        let (keyform, cell) = find(&warp.keyforms, &values)?;
                        Ok((keyform.points.clone(), Keyform::Warp(keyform), cell))
                    }
                    DeformerKind::Rotation(rotation) => {
                        let (keyform, cell) = find(&rotation.keyforms, &values)?;
                        Ok((vec![keyform.origin], Keyform::Rotation(keyform), cell))
                    }
                    DeformerKind::Unknown => bail!("deformer `{id}` has an unknown kind"),
                }
            }
            _ => bail!("{target} is not supported"),
        }
    }

    /// 要素の基準の姿の点をキャンバス座標で返す。
    fn canvas_points(&self, target: &KeyformTarget) -> anyhow::Result<Vec<Vec2>> {
        let (local, ..) = self.base(target)?;
        Ok(local_to_canvas(
            self.model(),
            target,
            &ParameterValues::new(),
            &local,
        )?)
    }

    /// 要素 (の基準の姿) のキャンバス座標での外接矩形。
    fn bounds(&self, target: &KeyformTarget) -> anyhow::Result<Rect> {
        Rect::of(&self.canvas_points(target)?).ok_or_else(|| anyhow!("{target} has no points"))
    }

    fn mesh_bounds(&self, name: &str) -> anyhow::Result<Rect> {
        self.bounds(&KeyformTarget::Mesh(self.mesh(name)?))
    }

    /// 要素の格子の `parameter` の軸にキー `keys` を加える。既定値のキーを先に加えるので、
    /// 他の軸との組み合わせのセルは加算合成で推定される (`docs/manual/tutorial.md` の 6.1)。
    fn add_keys(
        &mut self,
        target: &KeyformTarget,
        parameter: &str,
        keys: &[f32],
    ) -> anyhow::Result<()> {
        let default = sp::range(parameter)
            .map(|(_, default, _)| default)
            .ok_or_else(|| anyhow!("`{parameter}` is not a standard parameter"))?;
        let mut ordered = vec![default];
        ordered.extend(keys.iter().copied().filter(|k| *k != default));
        for key in ordered {
            self.apply(Command::AddKeyformKey {
                target: target.clone(),
                parameter: ParamId::new(parameter),
                key,
            })?;
        }
        Ok(())
    }

    /// 格子のセル `cell` (書かない軸は既定値) を、そのセルの今のキーフォームを `edit` で変えたものにする。
    fn set_cell(
        &mut self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
        edit: impl FnOnce(Keyform) -> anyhow::Result<Keyform>,
    ) -> anyhow::Result<()> {
        let (_, current, full_cell) = self.keyform_at(target, cell)?;
        let keyform = edit(current)?;
        self.apply(Command::SetKeyformCell {
            target: target.clone(),
            cell: full_cell,
            keyform,
        })
    }

    /// 格子のセル `cell` (書かない軸は既定値) の形を、基準の姿の点をキャンバス座標で `offset` だけ
    /// ずらしたものにする。`offset` は点のキャンバス座標と、要素の外接矩形の中での位置 (0〜1) を受け取る。
    /// 不透明度などの形以外の値はそのセルのまま。
    fn shape(
        &mut self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
        offset: impl Fn(Vec2, Vec2) -> Vec2,
    ) -> anyhow::Result<()> {
        let (local, ..) = self.base(target)?;
        let values = ParameterValues::new();
        let canvas = local_to_canvas(self.model(), target, &values, &local)?;
        let rect = Rect::of(&canvas).ok_or_else(|| anyhow!("{target} has no points"))?;
        let moved: Vec<Vec2> = canvas
            .iter()
            .map(|p| *p + offset(*p, rect.normalize(*p)))
            .collect();
        let points = canvas_to_local(self.model(), target, &values, &moved, &local)?;
        self.set_cell(target, cell, |base| match base {
            Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                positions: points,
                ..keyform
            })),
            Keyform::Warp(keyform) => Ok(Keyform::Warp(WarpKeyform { points, ..keyform })),
            _ => bail!("{target} is not a mesh or a warp deformer"),
        })
    }

    /// メッシュ `id` の不透明度を、`parameter` のキーごとに `(キー, 不透明度)` の値にする
    /// (他の軸は既定値のセル。形は変えない)。
    fn fade(&mut self, id: &MeshId, parameter: &str, keys: &[(f32, f32)]) -> anyhow::Result<()> {
        let target = KeyformTarget::Mesh(id.clone());
        let values: Vec<f32> = keys.iter().map(|(key, _)| *key).collect();
        self.add_keys(&target, parameter, &values)?;
        for (key, opacity) in keys {
            self.set_cell(&target, &[(parameter, *key)], |base| match base {
                Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                    opacity: *opacity,
                    ..keyform
                })),
                _ => bail!("{target} is not a mesh"),
            })?;
        }
        Ok(())
    }

    /// 回転デフォーマのセル `cell` の角度を `angle` 度にする。
    fn rotate(&mut self, id: &DeformerId, cell: &[(&str, f32)], angle: f32) -> anyhow::Result<()> {
        let target = KeyformTarget::Deformer(id.clone());
        self.set_cell(&target, cell, |base| match base {
            Keyform::Rotation(keyform) => {
                Ok(Keyform::Rotation(RotationKeyform { angle, ..keyform }))
            }
            _ => bail!("{target} is not a rotation deformer"),
        })
    }

    fn add_parameters(&mut self) -> anyhow::Result<()> {
        for id in PARAMETERS {
            let (min, default, max) =
                sp::range(id).ok_or_else(|| anyhow!("`{id}` is not a standard parameter"))?;
            self.apply(Command::AddParameter {
                parameter: Parameter {
                    id: ParamId::new(*id),
                    name: (*id).to_owned(),
                    min,
                    max,
                    default,
                    kind: Default::default(),
                    mirror: None,
                    group: None,
                    internal: false,
                },
            })?;
        }
        for (id, mirror) in MIRRORS {
            self.apply(Command::SetParameterMirror {
                id: ParamId::new(*id),
                mirror: Some(ParamId::new(*mirror)),
            })?;
        }
        Ok(())
    }

    /// 体: 回転 (`BodyAngleZ`) とワープ (`BodyAngleX` / `BodyAngleY` / `Breath`)。
    fn rig_body(&mut self, layout: &Layout) -> anyhow::Result<()> {
        let all: Vec<ChildRef> = self
            .model()
            .meshes
            .iter()
            .map(|m| ChildRef::Mesh(m.id.clone()))
            .collect();
        let rotation = DeformerId::new("BodyRotation");
        self.apply(Command::CreateRotationDeformer {
            id: Some(rotation.clone()),
            name: "Body rotation".to_owned(),
            children: all.clone(),
            parts: Vec::new(),
            origin: Some(Vec2::new(layout.canvas.x * 0.5, layout.canvas.y)),
            parent: None,
        })?;
        let warp = DeformerId::new("BodyWarp");
        self.apply(Command::CreateWarpDeformer {
            id: Some(warp.clone()),
            name: "Body warp".to_owned(),
            children: all,
            parts: Vec::new(),
            rows: 4,
            cols: 4,
            parent: None,
        })?;

        let target = KeyformTarget::Deformer(rotation.clone());
        self.add_keys(&target, sp::BODY_ANGLE_Z, &[-10.0, 10.0])?;
        for value in [-10.0, 10.0] {
            self.rotate(&rotation, &[(sp::BODY_ANGLE_Z, value)], value * 0.4)?;
        }

        // 変形の量は顔の大きさに対する割合。上 (頭の側) ほど大きく動かす。
        let face = layout.face.size();
        let canvas_height = layout.canvas.y;
        let lift = move |p: Vec2| (1.0 - p.y / canvas_height).clamp(0.0, 1.0);
        let target = KeyformTarget::Deformer(warp);
        self.add_keys(&target, sp::BODY_ANGLE_X, &[-10.0, 10.0])?;
        self.add_keys(&target, sp::BODY_ANGLE_Y, &[-10.0, 10.0])?;
        self.add_keys(&target, sp::BREATH, &[1.0])?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(sp::BODY_ANGLE_X, 10.0 * sign)], |p, _| {
                Vec2::new(sign * 0.12 * face.x * lift(p), 0.0)
            })?;
            self.shape(&target, &[(sp::BODY_ANGLE_Y, 10.0 * sign)], |p, _| {
                Vec2::new(0.0, -sign * 0.06 * face.y * lift(p))
            })?;
        }
        self.shape(&target, &[(sp::BREATH, 1.0)], |p, _| {
            Vec2::new(0.0, -0.025 * face.y * lift(p))
        })?;
        Ok(())
    }

    /// 頭: 傾き (`AngleZ`)、深度からの回転による向き (`AngleX` / `AngleY`)、目・眉・口。
    fn rig_head(&mut self, layout: &Layout) -> anyhow::Result<()> {
        let face = layout.face;
        let head = DeformerId::new("HeadRotation");
        self.apply(Command::CreateRotationDeformer {
            id: Some(head.clone()),
            name: "Head rotation".to_owned(),
            children: self.meshes(HEAD_LAYERS)?,
            parts: Vec::new(),
            origin: Some(Vec2::new(face.center().x, face.max.y)),
            parent: None,
        })?;
        self.add_keys(
            &KeyformTarget::Deformer(head.clone()),
            sp::ANGLE_Z,
            &[-30.0, 30.0],
        )?;
        for value in [-30.0, 30.0] {
            self.rotate(&head, &[(sp::ANGLE_Z, value)], value / 3.0)?;
        }

        let face_warp = DeformerId::new("FaceWarp");
        self.apply(Command::CreateWarpDeformer {
            id: Some(face_warp.clone()),
            name: "Face warp".to_owned(),
            children: self.meshes(&HEAD_LAYERS[1..])?,
            parts: Vec::new(),
            rows: 6,
            cols: 6,
            parent: None,
        })?;
        let back_hair = DeformerId::new("BackHairWarp");
        self.apply(Command::CreateWarpDeformer {
            id: Some(back_hair.clone()),
            name: "Back hair warp".to_owned(),
            children: self.meshes(&["back hair"])?,
            parts: Vec::new(),
            rows: 3,
            cols: 3,
            parent: None,
        })?;

        let mut eye_warps = Vec::new();
        for eye in &EYES {
            let upper = eye.suffix.to_uppercase();
            let open = DeformerId::new(format!("Eye{upper}Warp"));
            let parts = ["eyewhite", "irides", "eyelash"].map(|p| format!("{p}-{}", eye.suffix));
            let names: Vec<&str> = parts.iter().map(String::as_str).collect();
            self.apply(Command::CreateWarpDeformer {
                id: Some(open.clone()),
                name: format!("Eye {upper} warp"),
                children: self.meshes(&names)?,
                parts: Vec::new(),
                rows: 2,
                cols: 4,
                parent: None,
            })?;
            let closed = DeformerId::new(format!("Eye{upper}CloseWarp"));
            self.apply(Command::CreateWarpDeformer {
                id: Some(closed.clone()),
                name: format!("Eye {upper} close warp"),
                children: self.meshes(&[format!("eyeclose-{}", eye.suffix).as_str()])?,
                parts: Vec::new(),
                rows: 2,
                cols: 4,
                parent: None,
            })?;
            eye_warps.push((open, closed));
        }
        let mouth = DeformerId::new("MouthWarp");
        self.apply(Command::CreateWarpDeformer {
            id: Some(mouth.clone()),
            name: "Mouth warp".to_owned(),
            children: self.meshes(&["mouth-open", "mouth-close"])?,
            parts: Vec::new(),
            rows: 2,
            cols: 2,
            parent: None,
        })?;

        // 顔の向き: 顔はドーム、後ろ髪は奥の平面として、同じ中心のまわりに回す。
        let pivot = self.depth_rotation(
            &face_warp,
            DepthPreset::Dome {
                peak: 0.35 * face.size().x,
            },
            None,
        )?;
        self.depth_rotation(
            &back_hair,
            DepthPreset::Uniform {
                value: -0.12 * face.size().x,
            },
            Some(pivot),
        )?;

        for (eye, (open, closed)) in EYES.iter().zip(&eye_warps) {
            self.rig_eye(eye, open, closed)?;
            self.rig_brow(eye, face)?;
        }
        self.rig_mouth(&mouth)?;
        self.rig_physics(&head, &face_warp, &back_hair)?;
        Ok(())
    }

    /// 揺れもの: 前髪の毛先・後ろ髪・耳飾りを、動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)
    /// で頭の動きに遅れて揺らす。
    ///
    /// どのリグも根元を頭の回転デフォーマ (`frame`) に取り付け、振り子の先の粒子の横の変位を、ワープの制御点か
    /// メッシュの頂点に重み付きで足す (`vertices`)。重みは根元側が 0・先が 1 なので、生え際や耳の金具は動かない。
    /// 入力は `AngleX` (頭を横に向ける動きはデフォーマを動かさないので、根元の平行移動で与える)。`AngleZ` と
    /// 体の動きは頭のデフォーマを動かすので、それだけで根元が動く。衝突体は使わない。
    fn rig_physics(
        &mut self,
        head: &DeformerId,
        face_warp: &DeformerId,
        back_warp: &DeformerId,
    ) -> anyhow::Result<()> {
        // 揺れの出力先のワープ (キーフォームは持たない)。
        let front = DeformerId::new("FrontHairSwayWarp");
        let back = DeformerId::new("BackHairSwayWarp");
        for (id, name, layer, parent) in [
            (&front, "Front hair sway warp", "front hair", face_warp),
            (&back, "Back hair sway warp", "back hair", back_warp),
        ] {
            self.apply(Command::CreateWarpDeformer {
                id: Some(id.clone()),
                name: name.to_owned(),
                children: self.meshes(&[layer])?,
                parts: Vec::new(),
                rows: 3,
                cols: 3,
                parent: Some(parent.clone()),
            })?;
        }

        // 前髪: 生え際から毛先までの 1 区間。毛先だけを小さく揺らす (顔が読めるように)。
        let rect = self.mesh_bounds("front hair")?;
        let weights = self.sway_weights(&KeyformTarget::Deformer(front.clone()), rect, 0.4, 0.5)?;
        self.add_pendulum(
            "FrontHair",
            "Front hair",
            head,
            Vec2::new(rect.center().x, rect.min.y),
            &[0.8 * rect.size().y],
            (0.9, 0.8, 1.5),
            12.0,
            VertexOwner::Warp(front),
            weights,
        )?;

        // 後ろ髪: 顔の中ほどの高さから毛先までの 2 区間。前髪より大きくゆっくり揺らす。根元を頭頂に置くと、
        // 頭を傾けたとき (`AngleZ`、あごが中心) に根元が大きく動きすぎるので、顔の中ほどに置く。
        let rect = self.mesh_bounds("back hair")?;
        let weights = self.sway_weights(&KeyformTarget::Deformer(back.clone()), rect, 0.3, 0.8)?;
        let top = self.mesh_bounds("face")?.center().y;
        let half = 0.5 * (rect.max.y - top);
        self.add_pendulum(
            "BackHair",
            "Back hair",
            head,
            Vec2::new(rect.center().x, top),
            &[half, half],
            (0.93, 0.6, 1.5),
            28.0,
            VertexOwner::Warp(back),
            weights,
        )?;

        // 耳飾り: 耳の金具 (上の 35%) は動かさず、下に垂れた飾りを揺らす。レイヤーには耳から離れた所に
        // 数画素のごみがあるので、耳の横の範囲にある頂点だけを耳飾りとみなす。
        for side in ["l", "r"] {
            let ear = self.mesh_bounds(&format!("ears-{side}"))?;
            let id = self.mesh(&format!("earwear-{side}"))?;
            let points = self.canvas_points(&KeyformTarget::Mesh(id.clone()))?;
            let on_ear = |p: &Vec2| p.x >= ear.min.x && p.x <= ear.max.x;
            let rect = Rect::of(&points.iter().copied().filter(on_ear).collect::<Vec<_>>())
                .ok_or_else(|| anyhow!("earwear-{side} has no vertices below the ear"))?;
            let weights = points
                .iter()
                .map(|p| {
                    if on_ear(p) {
                        ((rect.normalize(*p).y - 0.35) / 0.65).clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                })
                .collect();
            self.add_pendulum(
                &format!("Earring{}", side.to_uppercase()),
                &format!("Earring {}", side.to_uppercase()),
                head,
                Vec2::new(rect.center().x, rect.min.y),
                &[rect.size().y],
                // 短く硬い飾りなので、速く振れて (加速度が大きい) 遅れすぎないようにする。
                (0.85, 1.0, 4.0),
                10.0,
                VertexOwner::Mesh(id),
                weights,
            )?;
        }
        Ok(())
    }

    /// ワープの制御点ごとの揺れの重み: 要素 (`rect`) の上から `start` の割合までは 0、そこから下端へ
    /// 2 乗で増えて `max` になる。
    fn sway_weights(
        &self,
        warp: &KeyformTarget,
        rect: Rect,
        start: f32,
        max: f32,
    ) -> anyhow::Result<Vec<f32>> {
        Ok(self
            .canvas_points(warp)?
            .iter()
            .map(|p| {
                let t = ((rect.normalize(*p).y - start) / (1.0 - start)).clamp(0.0, 1.0);
                max * t * t
            })
            .collect())
    }

    /// 真下に垂れる振り子 (根元 `root` はキャンバス座標、区間の長さ `lengths`) の動的物理のリグを加える。
    /// 根元は `head` に取り付け、先の粒子の横の変位を `owner` の頂点に `weights` の重みで足す。
    /// `(mobility, delay, acceleration)` は揺れの減衰・速さ・重力の強さ、`shift` は `AngleX` = ±30 での根元の平行移動 (px)。
    #[allow(clippy::too_many_arguments)]
    fn add_pendulum(
        &mut self,
        id: &str,
        name: &str,
        head: &DeformerId,
        root: Vec2,
        lengths: &[f32],
        (mobility, delay, acceleration): (f32, f32, f32),
        shift: f32,
        owner: VertexOwner,
        weights: Vec<f32>,
    ) -> anyhow::Result<()> {
        // 根元の位置は頭の回転デフォーマの局所座標で書く (その子の `FaceWarp` の親の座標系)。
        let local = canvas_to_local(
            self.model(),
            &KeyformTarget::Deformer(DeformerId::new("FaceWarp")),
            &ParameterValues::new(),
            &[root],
            &[root],
        )?[0];
        let mut particles = vec![PhysicsParticle {
            position: local,
            mobility: 1.0,
            delay: 1.0,
            acceleration: 1.0,
            radius: 0.0,
            collision_radius: 0.0,
        }];
        let mut y = 0.0;
        for length in lengths {
            y += length;
            particles.push(PhysicsParticle {
                position: Vec2::new(0.0, y),
                mobility,
                delay,
                acceleration,
                radius: *length,
                collision_radius: 0.0,
            });
        }
        self.apply(Command::AddPhysicsRig {
            rig: PhysicsRig {
                id: id.to_owned(),
                name: name.to_owned(),
                inputs: vec![PhysicsInput {
                    parameter: ParamId::new(sp::ANGLE_X),
                    source: PhysicsSource::X,
                    weight: 100.0,
                    reflect: false,
                }],
                outputs: vec![PhysicsOutput {
                    target: PhysicsTarget::Vertices { owner, weights },
                    particle: lengths.len() as u32,
                    source: PhysicsSource::X,
                    scale: 1.0,
                    weight: 100.0,
                    reflect: false,
                    blend: PhysicsBlend::Overwrite,
                }],
                particles,
                normalization: PhysicsNormalization {
                    position: NormalizationRange {
                        min: -shift,
                        default: 0.0,
                        max: shift,
                    },
                    angle: NormalizationRange {
                        min: -10.0,
                        default: 0.0,
                        max: 10.0,
                    },
                },
                colliders: Vec::new(),
                frame: Some(head.clone()),
            },
        })
    }

    /// ワープ `id` に深度 `depth` を与え、`AngleX` / `AngleY` のセルを深度からの回転で作る。
    /// キーの間は曲線補間にする (中間の角度でも回転の道筋に沿う。`docs/spec/evaluation.md#曲線補間`)。
    /// 回転の中心は `pivot` (親の座標系。省略すると基準の姿の制御点の中心) で、使った中心を返す。
    fn depth_rotation(
        &mut self,
        id: &DeformerId,
        depth: DepthPreset,
        pivot: Option<Vec2>,
    ) -> anyhow::Result<Vec2> {
        let target = KeyformTarget::Deformer(id.clone());
        self.add_keys(&target, sp::ANGLE_X, &[-30.0, 30.0])?;
        self.add_keys(&target, sp::ANGLE_Y, &[-30.0, 30.0])?;
        self.apply(Command::ApplyDepthPreset {
            target: ChildRef::Deformer(id.clone()),
            preset: depth,
        })?;
        let pivot = match pivot {
            Some(pivot) => pivot,
            None => {
                let (local, ..) = self.base(&target)?;
                Rect::of(&local)
                    .ok_or_else(|| anyhow!("{target} has no points"))?
                    .center()
            }
        };
        self.apply(Command::SetKeyformInference {
            target: target.clone(),
            inference: Some(KeyformInference::DepthRotation(DepthRotation {
                angle_scale: 0.7,
                pivot,
                pivot_depth: 0.0,
                yaw: Some(ParamId::new(sp::ANGLE_X)),
                pitch: Some(ParamId::new(sp::ANGLE_Y)),
            })),
        })?;
        let mut cells = Vec::new();
        for x in [-30.0, 0.0, 30.0] {
            for y in [-30.0, 0.0, 30.0] {
                if x != 0.0 || y != 0.0 {
                    cells.push(KeyformCell::from([
                        (ParamId::new(sp::ANGLE_X), x),
                        (ParamId::new(sp::ANGLE_Y), y),
                    ]));
                }
            }
        }
        self.apply(Command::MarkKeyformDerived {
            target: target.clone(),
            cells,
        })?;
        self.apply(Command::InferKeyforms {
            target: target.clone(),
            method: InferMethod::DepthRotation,
            overwrite_derived: true,
        })?;
        self.apply(Command::SetKeyformInterpolation {
            target,
            blend_shape: None,
            interpolation: KeyformInterpolation::Smooth,
        })?;
        Ok(pivot)
    }

    /// 目: 開閉 (`EyeLOpen`)・笑い目 (`EyeLSmile`)・見開き (`EyeLWide`) と、黒目の移動 (`EyeBallX` / `EyeBallY`)。
    ///
    /// 開いた目 (白目・黒目・まつげ) はワープ `open_warp`、閉じた目 (`eyeclose`) はワープ `closed_warp` に入れる。
    /// 閉じた目の絵は上向きの弧 (笑った閉じ目) なので、`EyeLSmile` = 0 では弧を平らにしたふつうの閉じ目、
    /// 1 で絵のままの笑った閉じ目にする。
    fn rig_eye(
        &mut self,
        eye: &EyeSide,
        open_warp: &DeformerId,
        closed_warp: &DeformerId,
    ) -> anyhow::Result<()> {
        use eye_fade::*;
        let white = self.mesh(&format!("eyewhite-{}", eye.suffix))?;
        let iris = self.mesh(&format!("irides-{}", eye.suffix))?;
        let lash = self.mesh(&format!("eyelash-{}", eye.suffix))?;
        let closed = self.mesh(&format!("eyeclose-{}", eye.suffix))?;
        let white_rect = self.bounds(&KeyformTarget::Mesh(white.clone()))?;
        let rect = self
            .bounds(&KeyformTarget::Mesh(lash.clone()))?
            .union(white_rect);
        let h = rect.size().y;
        let closed_rect = self.bounds(&KeyformTarget::Mesh(closed.clone()))?;
        // 閉じる線: 開いた目の下寄り (上まぶたが下りてきて下まぶたと合う所)。
        let line = rect.min.y + LINE * h;

        let target = KeyformTarget::Deformer(open_warp.clone());
        self.add_keys(&target, eye.open, &[0.0, SQUASH_KEY])?;
        self.add_keys(&target, eye.smile, &[1.0])?;
        self.add_keys(&target, eye.wide, &[1.0])?;
        // 閉じていく目: 閉じる線へ縦につぶし、中ほどを少し下げる (下向きの弧)。
        // 1 から `SQUASH_KEY` までで大きく細くなり、閉じた目と入れ替わる。
        // 笑い目との組み合わせは加算合成では下まぶたがつぶれた目を越えて裏返るので、弧を上向きにした形を書く。
        for (key, height) in [(0.0, 0.04), (SQUASH_KEY, SQUASH_HEIGHT)] {
            for (smile, bend) in [(0.0, 0.04), (1.0, -0.06)] {
                self.shape(&target, &[(eye.open, key), (eye.smile, smile)], |p, n| {
                    let y = line + (p.y - line) * height + bend * h * arc(n.x);
                    Vec2::new(0.0, y - p.y)
                })?;
            }
        }
        // 笑い目 (開いた目): 下まぶたを押し上げ、中ほどをより上げる (上向きの弧)。フェイストラッキングでは
        // 真顔でも `EyeLSmile` が 0.5 前後になるので、開いた目では控えめにする (笑った閉じ目は閉じた目の絵)。
        self.shape(&target, &[(eye.smile, 1.0)], |_, n| {
            Vec2::new(0.0, -(0.18 + 0.08 * arc(n.x)) * h * n.y)
        })?;
        self.shape(&target, &[(eye.wide, 1.0)], |_, n| {
            Vec2::new(0.0, -0.18 * h * (1.0 - n.y))
        })?;

        // 閉じた目の弧: 笑い目 = 1 が絵のまま。`SMILE_FLAT` 以下では中ほどを下げて平らにする。
        // フェイストラッキングの `EyeLSmile` (`eyeSquint`) は瞬きのたびに 0.7 前後になるので、ふつうの瞬きが
        // 笑った閉じ目にならないように、平らな形を `SMILE_FLAT` まで保つ。
        // 基準のセルを最後に書き換える (`shape` は基準のセルからのずれで書くため)。
        let target = KeyformTarget::Deformer(closed_warp.clone());
        self.add_keys(&target, eye.smile, &[SMILE_FLAT, 1.0])?;
        let (closed_center, closed_h) = (closed_rect.center().y, closed_rect.size().y);
        for smile in [SMILE_FLAT, 0.0] {
            self.shape(&target, &[(eye.smile, smile)], |p, n| {
                let y = line + (p.y - closed_center) * 0.6 + (FLATTEN * arc(n.x) - 0.2) * closed_h;
                Vec2::new(0.0, y - p.y)
            })?;
        }

        // 開いた目と閉じた目の切り替え (`eye_fade`)。
        for id in [&white, &iris, &lash] {
            self.fade(
                id,
                eye.open,
                &[(0.0, 0.0), (OPEN_OUT, 0.0), (CLOSED_FULL, 1.0), (1.0, 1.0)],
            )?;
        }
        self.fade(
            &closed,
            eye.open,
            &[(0.0, 1.0), (CLOSED_FULL, 1.0), (CLOSED_IN, 0.0), (1.0, 0.0)],
        )?;

        // 黒目は白目で切り抜き、視線で動かす。`EyeBallX` は正でキャラクターの左 (画面の右)、`EyeBallY` は正で上。
        self.apply(Command::SetMeshProperties {
            id: iris.clone(),
            part: None,
            texture: None,
            blend: None,
            culling: None,
            masks: Some(vec![MaskRef::normal(white)]),
            mask_mode: None,
            mask_only: None,
            lit: None,
        })?;
        let range = white_rect.size() * Vec2::new(0.22, 0.12);
        let target = KeyformTarget::Mesh(iris);
        self.add_keys(&target, sp::EYE_BALL_X, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::EYE_BALL_Y, &[-1.0, 1.0])?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(sp::EYE_BALL_X, sign)], |_, _| {
                Vec2::new(sign * range.x, 0.0)
            })?;
            self.shape(&target, &[(sp::EYE_BALL_Y, sign)], |_, _| {
                Vec2::new(0.0, -sign * range.y)
            })?;
        }
        Ok(())
    }

    /// 眉: 上下 (`BrowLY`)・傾き (`BrowLAngle`、正で外側が上がる)・内側の持ち上げ (`BrowInnerUp`)。
    /// 眉は PSD のとおり前髪の奥に描く (手前に描くと髪の上に線が浮いて見えるため)。
    fn rig_brow(&mut self, eye: &EyeSide, face: Rect) -> anyhow::Result<()> {
        let id = self.mesh(&format!("eyebrow-{}", eye.suffix))?;

        let target = KeyformTarget::Mesh(id);
        let brow = self.bounds(&target)?;
        let eye_height = self
            .mesh_bounds(&format!("eyelash-{}", eye.suffix))?
            .size()
            .y;
        // 外側 (顔の中心から遠い側) が 1、内側が 0 になる位置。
        let outward = if brow.center().x >= face.center().x {
            1.0
        } else {
            -1.0
        };
        let along = move |n: Vec2| if outward > 0.0 { n.x } else { 1.0 - n.x };
        let tilt = brow.size().x * 15f32.to_radians().tan();

        self.add_keys(&target, eye.brow_y, &[-1.0, 1.0])?;
        self.add_keys(&target, eye.brow_angle, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::BROW_INNER_UP, &[1.0])?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(eye.brow_y, sign)], |_, _| {
                Vec2::new(0.0, -sign * 0.3 * eye_height)
            })?;
            self.shape(&target, &[(eye.brow_angle, sign)], |_, n| {
                Vec2::new(0.0, -sign * (along(n) - 0.5) * tilt)
            })?;
        }
        self.shape(&target, &[(sp::BROW_INNER_UP, 1.0)], |_, n| {
            Vec2::new(0.0, -0.3 * eye_height * (1.0 - along(n)))
        })?;
        Ok(())
    }

    /// 口: 開き (`MouthOpenY`)・形 (`MouthForm`、正で笑い)・左右 (`MouthX`)・あごの左右 (`JawX`)・
    /// すぼめ (`MouthPucker`)・押し付け (`MouthPress`)・口元の持ち上げ (`MouthShrug`)。
    ///
    /// 開いた口 (`mouth-open`) は、閉じた口 (`mouth-close`) の大きさの細い隙間から開く (`mouth_fade`)。
    /// 口角は両方のメッシュを、閉じた口の高さを基準にした同じ量だけ曲げる。
    fn rig_mouth(&mut self, warp: &DeformerId) -> anyhow::Result<()> {
        use mouth_fade::*;
        let open_id = self.mesh("mouth-open")?;
        let closed_id = self.mesh("mouth-close")?;
        let open = self.bounds(&KeyformTarget::Mesh(open_id.clone()))?;
        let closed = self.bounds(&KeyformTarget::Mesh(closed_id.clone()))?;
        let bend = 0.45 * closed.size().y;

        // 閉じた口: 口角と、開きでの消え方。
        let target = KeyformTarget::Mesh(closed_id.clone());
        self.fade(
            &closed_id,
            sp::MOUTH_OPEN_Y,
            &[(0.0, 1.0), (CLOSED_OUT_START, 1.0), (CLOSED_OUT, 0.0)],
        )?;
        self.add_keys(&target, sp::MOUTH_FORM, &[-1.0, 1.0])?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(sp::MOUTH_FORM, sign)], |_, n| {
                Vec2::new(0.0, -sign * bend * corner(n.x))
            })?;
        }

        // 開いた口: すべてのセルを書く。細い隙間は閉じた口の横幅・高さの半分で、閉じた口の中心に置く。
        let slit = Vec2::new(
            closed.size().x / open.size().x,
            0.5 * closed.size().y / open.size().y,
        );
        let target = KeyformTarget::Mesh(open_id.clone());
        self.add_keys(&target, sp::MOUTH_OPEN_Y, &[OPEN_FULL, 1.0])?;
        self.add_keys(&target, sp::MOUTH_FORM, &[-1.0, 1.0])?;
        // 基準のセル (開き 0・形 0) は最後に書く (`shape` は基準のセルからのずれで書くため)。
        for amount in [1.0, OPEN_FULL, 0.0] {
            for form in [-1.0, 1.0, 0.0] {
                self.shape(
                    &target,
                    &[(sp::MOUTH_OPEN_Y, amount), (sp::MOUTH_FORM, form)],
                    |p, n| {
                        let at = if amount >= 1.0 {
                            p
                        } else {
                            closed.center() + (p - open.center()) * slit
                        };
                        at - p + Vec2::new(0.0, -form * bend * corner(n.x))
                    },
                )?;
            }
        }
        // 不透明度は形の軸のすべてのセルに書く (書いたセルは加算合成で作り直されないため)。
        for form in [-1.0, 0.0, 1.0] {
            self.set_cell(
                &target,
                &[(sp::MOUTH_OPEN_Y, 0.0), (sp::MOUTH_FORM, form)],
                |keyform| match keyform {
                    Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                        opacity: 0.0,
                        ..keyform
                    })),
                    _ => bail!("mouth-open is not a mesh"),
                },
            )?;
        }

        // 口のまわり: 左右・すぼめ・押し付け・持ち上げはワープで両方の口を動かす。
        let size = open.size();
        let target = KeyformTarget::Deformer(warp.clone());
        self.add_keys(&target, sp::MOUTH_X, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::JAW_X, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::MOUTH_PUCKER, &[1.0])?;
        self.add_keys(&target, sp::MOUTH_PRESS, &[1.0])?;
        self.add_keys(&target, sp::MOUTH_SHRUG, &[1.0])?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(sp::MOUTH_X, sign)], |_, _| {
                Vec2::new(sign * 0.2 * size.x, 0.0)
            })?;
            // あご: 下ほど横に動かす (開いた口の下の唇が寄る)。
            self.shape(&target, &[(sp::JAW_X, sign)], |_, n| {
                Vec2::new(sign * 0.1 * size.x * n.y, 0.0)
            })?;
        }
        self.shape(&target, &[(sp::MOUTH_PUCKER, 1.0)], |_, n| {
            Vec2::new(-(n.x - 0.5) * 0.35 * size.x, (n.y - 0.5) * 0.1 * size.y)
        })?;
        self.shape(&target, &[(sp::MOUTH_PRESS, 1.0)], |_, n| {
            Vec2::new(0.0, -(n.y - 0.5) * 0.3 * size.y)
        })?;
        self.shape(&target, &[(sp::MOUTH_SHRUG, 1.0)], |_, n| {
            Vec2::new(0.0, -0.12 * size.y * n.y)
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ugoe_model::avatar::check::check_profile;
    use ugoe_runtime::{CompiledModel, ModelInstance};

    use super::*;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn build() -> Project {
        let psd = std::fs::read(repo_path(DEFAULT_PSD)).unwrap();
        build_avatar(&psd).unwrap()
    }

    /// パラメータの値 `params` で評価した、メッシュごとの頂点位置と不透明度。
    fn evaluate(model: &Model, params: &[(&str, f32)]) -> Vec<(Vec<Vec2>, f32)> {
        let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
        for (id, value) in params {
            let parameters = instance.parameters_mut();
            let index = parameters.index_of(id).unwrap();
            parameters.set(index, *value);
        }
        instance.update();
        instance
            .drawables()
            .iter()
            .map(|d| (d.positions.clone(), d.opacity))
            .collect()
    }

    /// 2 つの評価の、頂点の最大の移動量 (px) と不透明度の最大の差。
    fn difference(a: &[(Vec<Vec2>, f32)], b: &[(Vec<Vec2>, f32)]) -> (f32, f32) {
        a.iter()
            .zip(b)
            .fold((0.0f32, 0.0f32), |(moved, faded), (x, y)| {
                let m =
                    x.0.iter()
                        .zip(&y.0)
                        .map(|(p, q)| p.distance(*q))
                        .fold(0.0, f32::max);
                (moved.max(m), faded.max((x.1 - y.1).abs()))
            })
    }

    #[test]
    fn sample_avatar_conforms_and_every_parameter_moves_the_art() {
        let project = build();
        let model = &project.model;
        assert!(ugoe_model::validate::validate(model).is_empty());
        let report = check_profile(model);
        assert_eq!(report.errors().count(), 0, "{:?}", report.issues);
        assert_eq!(report.claimed, Some(3));
        assert_eq!(report.satisfied, 3);

        let neutral = evaluate(model, &[]);
        for parameter in &model.parameters {
            for value in [parameter.min, parameter.max] {
                if value == parameter.default {
                    continue;
                }
                let posed = evaluate(model, &[(parameter.id.as_str(), value)]);
                let (moved, faded) = difference(&neutral, &posed);
                assert!(
                    moved > 1.0 || faded > 0.5,
                    "{} = {value} barely changes the art (moved {moved} px, opacity {faded})",
                    parameter.id
                );
            }
        }
    }

    /// メッシュの名前から、評価の結果 (`drawables` はメッシュの宣言順) の添字を引く。
    fn index(model: &Model, name: &str) -> usize {
        model.meshes.iter().position(|m| m.name == name).unwrap()
    }

    /// 目と口の開閉で、開いた絵と閉じた絵の少なくとも一方が途中の値でも不透明 (消えたり、半透明の 2 枚が
    /// 重なったりしない) こと。
    #[test]
    fn open_and_closed_layers_never_vanish_or_double() {
        let project = build();
        let model = &project.model;
        for (parameter, open, closed) in [
            (sp::EYE_L_OPEN, "eyelash-l", "eyeclose-l"),
            (sp::EYE_R_OPEN, "eyelash-r", "eyeclose-r"),
            (sp::MOUTH_OPEN_Y, "mouth-open", "mouth-close"),
        ] {
            for step in 0..=100 {
                let value = step as f32 / 100.0;
                let posed = evaluate(model, &[(parameter, value)]);
                let a = posed[index(model, open)].1;
                let b = posed[index(model, closed)].1;
                assert!(
                    a >= 1.0 - 1e-4 || b >= 1.0 - 1e-4,
                    "{parameter} = {value}: {open} {a}, {closed} {b}"
                );
            }
            // 端では片方だけが見える (0 で閉じた絵、1 で開いた絵)。
            let shown = |value: f32| {
                let posed = evaluate(model, &[(parameter, value)]);
                (posed[index(model, open)].1, posed[index(model, closed)].1)
            };
            assert_eq!(shown(0.0), (0.0, 1.0), "{parameter} = 0");
            assert_eq!(shown(1.0), (1.0, 0.0), "{parameter} = 1");
        }
    }

    /// 頭を横に振ると、動的物理で耳飾り・前髪の毛先・後ろ髪が遅れて揺れ、止めると元に戻る。
    /// 前髪は顔が読めるように後ろ髪より小さく揺れ、静止しているときは物理のない評価と同じ姿になる。
    #[test]
    fn physics_sways_earrings_and_hair_after_a_head_shake() {
        let project = build();
        let model = &project.model;
        let mut puppet = ugoe::Puppet::new(model.clone()).unwrap();
        puppet.set_physics(project.physics.clone()).unwrap();
        assert!(puppet.physics().unwrap().is_dynamic());
        puppet.set_toggles(ugoe::Toggles {
            eye_blink: false,
            breath: false,
            ..ugoe::Toggles::default()
        });
        let layers = ["earwear-l", "earwear-r", "front hair", "back hair"];
        let angle = puppet
            .instance()
            .parameters()
            .index_of(sp::ANGLE_X)
            .unwrap();
        // 物理のない評価からの、頂点の最大のずれ (px)。
        let lag = |puppet: &ugoe::Puppet, value: f32| -> Vec<f32> {
            let still = evaluate(model, &[(sp::ANGLE_X, value)]);
            layers
                .iter()
                .map(|name| {
                    let i = index(model, name);
                    let (a, b) = (&puppet.instance().drawables()[i].positions, &still[i].0);
                    a.iter()
                        .zip(b)
                        .map(|(p, q)| p.distance(*q))
                        .fold(0.0, f32::max)
                })
                .collect()
        };
        const FPS: f32 = 60.0;
        let mut peak = vec![0.0f32; layers.len()];
        for frame in 0..(6.0 * FPS) as usize {
            let t = frame as f32 / FPS;
            // 0.8 秒周期で 2 往復 (±25) 振ってから止める。
            let value = if t < 1.6 {
                25.0 * (std::f32::consts::TAU * t / 0.8).sin()
            } else {
                0.0
            };
            puppet.instance_mut().parameters_mut().set(angle, value);
            puppet.update(1.0 / FPS);
            let now = lag(&puppet, value);
            if frame == 0 {
                assert!(now.iter().all(|d| *d < 0.01), "at rest: {now:?}");
            }
            for (p, d) in peak.iter_mut().zip(&now) {
                *p = p.max(*d);
            }
        }
        let [ear_l, ear_r, front, back] = peak[..] else {
            unreachable!()
        };
        assert!(ear_l > 2.0 && ear_r > 2.0, "earrings: {peak:?}");
        assert!(front > 2.0 && back > 2.0 * front, "hair: {peak:?}");
        let settled = lag(&puppet, 0.0);
        assert!(settled.iter().all(|d| *d < 0.5), "not settled: {settled:?}");
    }

    #[test]
    fn eye_open_closes_only_its_own_eye() {
        let project = build();
        let model = &project.model;
        let neutral = evaluate(model, &[]);
        let closed = evaluate(model, &[(sp::EYE_L_OPEN, 0.0)]);
        let moved = |name: &str| {
            let i = index(model, name);
            difference(&neutral[i..=i], &closed[i..=i])
        };
        let (left, faded) = moved("eyelash-l");
        assert!(left > 3.0 && faded == 1.0, "{left} {faded}");
        assert_eq!(moved("eyeclose-l").1, 1.0);
        assert_eq!(moved("eyelash-r"), (0.0, 0.0));
        assert_eq!(moved("eyeclose-r"), (0.0, 0.0));
    }

    /// コミットした `assets/samples/avatar.ugoe` が、今のジェネレータの出力と (浮動小数点の誤差を除いて) 同じこと。
    /// 違えば `just cli sample avatar -o assets/samples/avatar.ugoe` で作り直す。
    #[test]
    fn committed_sample_is_up_to_date() {
        let bytes = std::fs::read(repo_path("assets/samples/avatar.ugoe")).unwrap();
        let bundle = ugoe_format::read_bundle(&bytes).unwrap();
        let committed = bundle.model;
        let project = build();
        assert_eq!(
            bundle.physics, project.physics,
            "regenerate assets/samples/avatar.ugoe"
        );
        let fresh = project.model;
        assert_eq!(check_profile(&committed).satisfied, 3);
        let ids = |m: &Model| {
            (
                m.parameters
                    .iter()
                    .map(|p| p.id.clone())
                    .collect::<Vec<_>>(),
                m.deformers.iter().map(|d| d.id.clone()).collect::<Vec<_>>(),
                m.meshes.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            ids(&committed),
            ids(&fresh),
            "regenerate assets/samples/avatar.ugoe"
        );
        assert_eq!(committed.avatar, fresh.avatar);
        for params in [
            vec![],
            vec![
                (sp::ANGLE_X, 30.0),
                (sp::ANGLE_Y, -30.0),
                (sp::EYE_L_OPEN, 0.0),
            ],
            vec![
                (sp::MOUTH_OPEN_Y, 0.2),
                (sp::MOUTH_FORM, 1.0),
                (sp::BROW_R_Y, 1.0),
                (sp::EYE_R_OPEN, 0.35),
            ],
        ] {
            let (moved, faded) =
                difference(&evaluate(&committed, &params), &evaluate(&fresh, &params));
            assert!(
                moved < 0.01 && faded < 1e-4,
                "regenerate assets/samples/avatar.ugoe ({moved} px)"
            );
        }
    }
}
