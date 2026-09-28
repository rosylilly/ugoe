//! `ugoe sample avatar`: アバタープロファイル (`docs/spec/avatar-profile.md`) に準拠した見本のモデルを、
//! PSD から組み立てる (T0702)。
//!
//! リグはすべて `ugoe_editor_core::Command` の列で組む (エディタで手で組むのと同じ操作)。
//! 部位はレイヤーの名前で見つけ、変形の量はその部位の外接矩形の大きさに対する割合で決めるので、
//! 座標の定数は持たない。`Model.avatar` だけは設定するコマンドがまだないので、組み終えたモデルに
//! 直接書く ([`avatar_profile`])。
//!
//! 入力は `assets/fixtures/psd-real/comfyui-see-through-full-body-parts.psd` (全身の立ち絵を
//! 目・白目・まつげ・眉・口などに分けたもの)。全身の絵なので、キャンバスを胸から上に切り詰める。

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
use ugoe_model::standard as sp;
use ugoe_model::{
    AvatarProfile, DeformerId, DeformerKind, DepthRotation, HitArea, KeyformGrid, KeyformInference,
    MaskRef, MeshId, Model, ParamId, Parameter,
};

/// 既定の入力 (リポジトリのルートからの相対パス)。
pub const DEFAULT_PSD: &str = "assets/fixtures/psd-real/comfyui-see-through-full-body-parts.psd";

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
    "earwear",
    "nose",
    "mouth",
    "eyewhite-l",
    "eyewhite-r",
    "irides-l",
    "irides-r",
    "eyelash-l",
    "eyelash-r",
    "eyebrow-l",
    "eyebrow-r",
    "front hair",
];

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
    let layout = rig.crop_to_bust()?;
    rig.add_parameters()?;
    rig.rig_body(&layout)?;
    rig.rig_head(&layout)?;

    let mut project =
        Project::from_document(empty.meta, &rig.document, None, Vec::new(), Vec::new());
    // TODO(T0631): `Model.avatar` と当たり判定を設定するコマンドができたら、コマンドで書く。
    let face = rig.mesh("face")?;
    let body = rig.mesh("topwear")?;
    project.model.hit_areas = vec![
        HitArea {
            id: HIT_AREA_HEAD.to_owned(),
            mesh: face,
        },
        HitArea {
            id: HIT_AREA_BODY.to_owned(),
            mesh: body,
        },
    ];
    project.model.avatar = Some(avatar_profile());
    Ok(project)
}

/// 見本のモデルのアバタープロファイル (レベル 3)。絵は MIT で配布している PSD から作った。
fn avatar_profile() -> AvatarProfile {
    AvatarProfile {
        level: 3,
        meta: AvatarMeta {
            name: NAME.to_owned(),
            version: Some("0.1.0".to_owned()),
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

    fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    /// 点 `p` の、矩形の中での位置 (左上 (0,0)〜右下 (1,1))。
    fn normalize(self, p: Vec2) -> Vec2 {
        let size = self.size().max(Vec2::splat(f32::EPSILON));
        (p - self.min) / size
    }
}

/// 切り詰めた後のキャンバスでの、変形の大きさの基準。
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
        let model = self.model();
        let defaults: BTreeMap<ParamId, f32> = model
            .parameters
            .iter()
            .map(|p| (p.id.clone(), p.default))
            .collect();
        fn neutral<T: Clone>(
            grid: &KeyformGrid<T>,
            defaults: &BTreeMap<ParamId, f32>,
        ) -> anyhow::Result<(T, KeyformCell)> {
            let mut index = 0;
            let mut stride = 1;
            let mut cell = KeyformCell::new();
            for binding in &grid.bindings {
                let default = defaults.get(&binding.parameter).copied().unwrap_or(0.0);
                let key = binding
                    .keys
                    .iter()
                    .position(|k| *k == default)
                    .ok_or_else(|| {
                        anyhow!("axis {} has no key at its default", binding.parameter)
                    })?;
                index += key * stride;
                stride *= binding.keys.len();
                cell.insert(binding.parameter.clone(), default);
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
                let (keyform, cell) = neutral(&mesh.keyforms, &defaults)?;
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
                        let (keyform, cell) = neutral(&warp.keyforms, &defaults)?;
                        Ok((keyform.points.clone(), Keyform::Warp(keyform), cell))
                    }
                    DeformerKind::Rotation(rotation) => {
                        let (keyform, cell) = neutral(&rotation.keyforms, &defaults)?;
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

    /// 格子のセル `cell` (書かない軸は既定値) の形を、基準の姿の点をキャンバス座標で `offset` だけ
    /// ずらしたものにする。`offset` は点のキャンバス座標と、要素の外接矩形の中での位置 (0〜1) を受け取る。
    fn shape(
        &mut self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
        offset: impl Fn(Vec2, Vec2) -> Vec2,
    ) -> anyhow::Result<()> {
        let (local, base, mut full_cell) = self.base(target)?;
        let values = ParameterValues::new();
        let canvas = local_to_canvas(self.model(), target, &values, &local)?;
        let rect = Rect::of(&canvas).ok_or_else(|| anyhow!("{target} has no points"))?;
        let moved: Vec<Vec2> = canvas
            .iter()
            .map(|p| *p + offset(*p, rect.normalize(*p)))
            .collect();
        let points = canvas_to_local(self.model(), target, &values, &moved, &local)?;
        let keyform = match base {
            Keyform::Mesh(keyform) => Keyform::Mesh(MeshKeyform {
                positions: points,
                ..keyform
            }),
            Keyform::Warp(keyform) => Keyform::Warp(WarpKeyform { points, ..keyform }),
            _ => bail!("{target} is not a mesh or a warp deformer"),
        };
        for (parameter, value) in cell {
            full_cell.insert(ParamId::new(*parameter), *value);
        }
        self.apply(Command::SetKeyformCell {
            target: target.clone(),
            cell: full_cell,
            keyform,
        })
    }

    /// 回転デフォーマのセル `cell` の角度を `angle` 度にする。
    fn rotate(&mut self, id: &DeformerId, cell: &[(&str, f32)], angle: f32) -> anyhow::Result<()> {
        let target = KeyformTarget::Deformer(id.clone());
        let (_, base, mut full_cell) = self.base(&target)?;
        let Keyform::Rotation(keyform) = base else {
            bail!("{target} is not a rotation deformer");
        };
        for (parameter, value) in cell {
            full_cell.insert(ParamId::new(*parameter), *value);
        }
        self.apply(Command::SetKeyformCell {
            target,
            cell: full_cell,
            keyform: Keyform::Rotation(RotationKeyform { angle, ..keyform }),
        })
    }

    /// 全身の絵を、顔を中心にした胸から上に切り詰める。
    ///
    /// キャンバスの外に出るメッシュ (靴など) とそのテクスチャを消し、全体を回転デフォーマ `Root` で
    /// 包んで平行移動し、キャンバスを小さくする。
    fn crop_to_bust(&mut self) -> anyhow::Result<Layout> {
        let face = self.mesh_bounds("face")?;
        let head = face.union(self.mesh_bounds("front hair")?);
        let width = (face.size().x * 2.6).round();
        let height = (width * 1.2).round();
        let min = Vec2::new(
            (face.center().x - width / 2.0).round(),
            (head.min.y - face.size().y * 0.15).round(),
        );
        let crop = Rect {
            min,
            max: min + Vec2::new(width, height),
        };

        let outside: Vec<(MeshId, u32)> = self
            .model()
            .meshes
            .iter()
            .filter(|m| {
                let positions = &m.keyforms.keyforms[0].positions;
                Rect::of(positions).is_none_or(|r| !r.intersects(crop))
            })
            .map(|m| (m.id.clone(), m.texture))
            .collect();
        for (id, _) in &outside {
            self.apply(Command::RemoveMesh { id: id.clone() })?;
        }
        // 後ろのテクスチャから消す (消すと後ろの番号が詰まるため)。
        let mut textures: Vec<u32> = outside.iter().map(|(_, t)| *t).collect();
        textures.sort_unstable_by(|a, b| b.cmp(a));
        textures.dedup();
        for index in textures {
            let name = self.model().textures[index as usize].name.clone();
            self.apply(Command::RemoveTexture { name })?;
        }

        let root = self
            .model()
            .parts
            .iter()
            .find(|p| p.parent.is_none())
            .map(|p| p.id.clone())
            .ok_or_else(|| anyhow!("the import made no root part"))?;
        let root_id = DeformerId::new("Root");
        let origin = crop.center();
        self.apply(Command::CreateRotationDeformer {
            id: Some(root_id.clone()),
            name: "Root".to_owned(),
            children: Vec::new(),
            parts: vec![root],
            origin: Some(origin),
            parent: None,
        })?;
        let target = KeyformTarget::Deformer(root_id);
        let (_, base, cell) = self.base(&target)?;
        let Keyform::Rotation(keyform) = base else {
            bail!("Root is not a rotation deformer");
        };
        self.apply(Command::SetKeyformCell {
            target,
            cell,
            keyform: Keyform::Rotation(RotationKeyform {
                origin: origin - crop.min,
                ..keyform
            }),
        })?;
        let canvas = crop.size();
        self.apply(Command::SetCanvas {
            width: canvas.x,
            height: canvas.y,
            origin: canvas * 0.5,
        })?;
        Ok(Layout {
            face: Rect {
                min: face.min - crop.min,
                max: face.max - crop.min,
            },
            canvas,
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
            self.rotate(&rotation, &[(sp::BODY_ANGLE_Z, value)], value * 0.5)?;
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
            let id = DeformerId::new(format!("Eye{}Warp", eye.suffix.to_uppercase()));
            let parts = ["eyewhite", "irides", "eyelash"].map(|p| format!("{p}-{}", eye.suffix));
            let names: Vec<&str> = parts.iter().map(String::as_str).collect();
            self.apply(Command::CreateWarpDeformer {
                id: Some(id.clone()),
                name: format!("Eye {} warp", eye.suffix.to_uppercase()),
                children: self.meshes(&names)?,
                parts: Vec::new(),
                rows: 2,
                cols: 2,
                parent: None,
            })?;
            eye_warps.push(id);
        }
        let mouth = DeformerId::new("MouthWarp");
        self.apply(Command::CreateWarpDeformer {
            id: Some(mouth.clone()),
            name: "Mouth warp".to_owned(),
            children: self.meshes(&["mouth"])?,
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

        for (eye, warp) in EYES.iter().zip(&eye_warps) {
            self.rig_eye(eye, warp)?;
            self.rig_brow(eye, face)?;
        }
        self.rig_mouth(&mouth)?;
        Ok(())
    }

    /// ワープ `id` に深度 `depth` を与え、`AngleX` / `AngleY` のセルを深度からの回転で作る。
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
            target,
            method: InferMethod::DepthRotation,
            overwrite_derived: true,
        })?;
        Ok(pivot)
    }

    /// 目: 閉じる (`EyeLOpen` = 0)・笑い目 (`EyeLSmile`)・見開き (`EyeLWide`) と、黒目の移動 (`EyeBallX` / `EyeBallY`)。
    fn rig_eye(&mut self, eye: &EyeSide, warp: &DeformerId) -> anyhow::Result<()> {
        let white = self.mesh(&format!("eyewhite-{}", eye.suffix))?;
        let lash = self.bounds(&KeyformTarget::Mesh(
            self.mesh(&format!("eyelash-{}", eye.suffix))?,
        ))?;
        let white_rect = self.bounds(&KeyformTarget::Mesh(white.clone()))?;
        let rect = lash.union(white_rect);
        let (top, h) = (rect.min.y, rect.size().y);

        let target = KeyformTarget::Deformer(warp.clone());
        self.add_keys(&target, eye.open, &[0.0])?;
        self.add_keys(&target, eye.smile, &[1.0])?;
        self.add_keys(&target, eye.wide, &[1.0])?;
        // 閉じた目: 上下を目の下寄りの細い帯に寄せ、中ほどを少し下げる (下向きの弧)。
        self.shape(&target, &[(eye.open, 0.0)], |p, n| {
            let arc = 1.0 - (2.0 * n.x - 1.0).powi(2);
            let y = top + 0.62 * h + (n.y - 0.5) * 0.12 * h + 0.06 * h * arc;
            Vec2::new(0.0, y - p.y)
        })?;
        // 笑い目: 下から押し上げ、中ほどを上げる (上向きの弧)。
        self.shape(&target, &[(eye.smile, 1.0)], |p, n| {
            let arc = 1.0 - (2.0 * n.x - 1.0).powi(2);
            let y = top + 0.45 * h + (n.y - 0.5) * 0.15 * h - 0.12 * h * arc;
            Vec2::new(0.0, y - p.y)
        })?;
        self.shape(&target, &[(eye.wide, 1.0)], |_, n| {
            Vec2::new(0.0, -0.18 * h * (1.0 - n.y))
        })?;

        // 黒目は白目で切り抜き、視線で動かす。`EyeBallX` は正でキャラクターの左 (画面の右)、`EyeBallY` は正で上。
        let iris = self.mesh(&format!("irides-{}", eye.suffix))?;
        self.apply(Command::SetMeshProperties {
            id: iris.clone(),
            part: None,
            texture: None,
            blend: None,
            culling: None,
            masks: Some(vec![MaskRef::normal(white)]),
            mask_mode: None,
            mask_only: None,
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
    /// 眉は前髪に隠れないように前髪の手前に描く。
    fn rig_brow(&mut self, eye: &EyeSide, face: Rect) -> anyhow::Result<()> {
        let id = self.mesh(&format!("eyebrow-{}", eye.suffix))?;
        let hair = self.mesh("front hair")?;
        let hair_order = self
            .model()
            .meshes
            .iter()
            .find(|m| m.id == hair)
            .map_or(0.0, |m| m.keyforms.keyforms[0].draw_order);
        self.apply(Command::SetMeshDrawOrder {
            id: id.clone(),
            draw_order: hair_order + 1.0,
        })?;

        let target = KeyformTarget::Mesh(id);
        let brow = self.bounds(&target)?;
        let eye_height = self
            .bounds(&KeyformTarget::Mesh(
                self.mesh(&format!("eyelash-{}", eye.suffix))?,
            ))?
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
                Vec2::new(0.0, -sign * 0.4 * eye_height)
            })?;
            self.shape(&target, &[(eye.brow_angle, sign)], |_, n| {
                Vec2::new(0.0, -sign * (along(n) - 0.5) * tilt)
            })?;
        }
        self.shape(&target, &[(sp::BROW_INNER_UP, 1.0)], |_, n| {
            Vec2::new(0.0, -0.35 * eye_height * (1.0 - along(n)))
        })?;
        Ok(())
    }

    /// 口: 開き (`MouthOpenY`)・形 (`MouthForm`、正で笑い)・左右 (`MouthX`)・すぼめ (`MouthPucker`)・
    /// 押し付け (`MouthPress`)・口元の持ち上げ (`MouthShrug`)。
    fn rig_mouth(&mut self, warp: &DeformerId) -> anyhow::Result<()> {
        let lips = self.mesh_bounds("mouth")?;
        let size = lips.size();
        let inside = self.add_mouth_inside(warp, lips)?;
        let target = KeyformTarget::Mesh(inside);
        self.add_keys(&target, sp::MOUTH_OPEN_Y, &[1.0])?;
        let (_, base, _) = self.base(&target)?;
        let Keyform::Mesh(keyform) = base else {
            bail!("{target} is not a mesh");
        };
        self.apply(Command::SetKeyformCell {
            target,
            cell: KeyformCell::from([(ParamId::new(sp::MOUTH_OPEN_Y), 1.0)]),
            keyform: Keyform::Mesh(MeshKeyform {
                opacity: 1.0,
                ..keyform
            }),
        })?;

        let target = KeyformTarget::Deformer(warp.clone());
        self.add_keys(&target, sp::MOUTH_OPEN_Y, &[1.0])?;
        self.add_keys(&target, sp::MOUTH_FORM, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::MOUTH_X, &[-1.0, 1.0])?;
        self.add_keys(&target, sp::MOUTH_PUCKER, &[1.0])?;
        self.add_keys(&target, sp::MOUTH_PRESS, &[1.0])?;
        self.add_keys(&target, sp::MOUTH_SHRUG, &[1.0])?;

        self.shape(&target, &[(sp::MOUTH_OPEN_Y, 1.0)], |_, n| {
            Vec2::new(-(n.x - 0.5) * 0.15 * size.x, (-0.2 + 1.0 * n.y) * size.y)
        })?;
        for sign in [-1.0, 1.0] {
            self.shape(&target, &[(sp::MOUTH_FORM, sign)], |_, n| {
                let corner = (2.0 * n.x - 1.0).powi(2);
                Vec2::new(0.0, -sign * 0.45 * size.y * corner)
            })?;
            self.shape(&target, &[(sp::MOUTH_X, sign)], |_, _| {
                Vec2::new(sign * 0.3 * size.x, 0.0)
            })?;
        }
        self.shape(&target, &[(sp::MOUTH_PUCKER, 1.0)], |_, n| {
            Vec2::new(-(n.x - 0.5) * 0.45 * size.x, (n.y - 0.5) * 0.25 * size.y)
        })?;
        self.shape(&target, &[(sp::MOUTH_PRESS, 1.0)], |_, n| {
            Vec2::new(0.0, -(n.y - 0.5) * 0.5 * size.y)
        })?;
        self.shape(&target, &[(sp::MOUTH_SHRUG, 1.0)], |_, n| {
            Vec2::new(0.0, -0.25 * size.y * n.y)
        })?;
        Ok(())
    }

    /// 口の中 (唇の間に見える暗い楕円) のメッシュを、唇の外接矩形 `lips` から作って口のワープに入れる。
    /// PSD の唇は閉じた 1 枚の絵なので、開いたときに唇の間に見せる。既定では透明 (`MouthOpenY` で現れる)。
    fn add_mouth_inside(&mut self, warp: &DeformerId, lips: Rect) -> anyhow::Result<MeshId> {
        const W: u32 = 32;
        const H: u32 = 16;
        let mut rgba = Vec::with_capacity((W * H * 4) as usize);
        for y in 0..H {
            for x in 0..W {
                let dx = (x as f32 + 0.5) / W as f32 * 2.0 - 1.0;
                let dy = (y as f32 + 0.5) / H as f32 * 2.0 - 1.0;
                let r = (dx * dx + dy * dy).sqrt();
                // 縁を 1〜2 画素ぼかした楕円。
                let alpha = ((1.0 - r) * 6.0).clamp(0.0, 1.0);
                rgba.extend_from_slice(&[120, 40, 52, (alpha * 255.0).round() as u8]);
            }
        }
        let name = "mouth_inside".to_owned();
        self.apply(Command::AddTexture {
            name: name.clone(),
            png: ugoe_editor_core::texture::encode_rgba_png(W, H, &rgba),
        })?;
        let texture = self
            .model()
            .textures
            .iter()
            .position(|t| t.name == name)
            .ok_or_else(|| anyhow!("texture `{name}` was not added"))? as u32;

        let lips_id = self.mesh("mouth")?;
        let lips_mesh = self
            .model()
            .meshes
            .iter()
            .find(|m| m.id == lips_id)
            .ok_or_else(|| anyhow!("no mesh `{lips_id}`"))?;
        let lips_keyform = lips_mesh.keyforms.keyforms[0].clone();
        let part = lips_mesh.part.clone();
        // 唇の合わせ目 (横 56%・縦 16%) を覆う四角形。
        let center = lips.center();
        let half = lips.size() * Vec2::new(0.28, 0.08);
        let corners = [
            center - half,
            Vec2::new(center.x + half.x, center.y - half.y),
            center + half,
            Vec2::new(center.x - half.x, center.y + half.y),
        ];
        let id = MeshId::new("MouthInside");
        self.apply(Command::AddMesh {
            mesh: ugoe_model::Mesh {
                id: id.clone(),
                name: "mouth inside".to_owned(),
                part,
                parent: None,
                texture,
                uvs: vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
                blend: Default::default(),
                culling: false,
                masks: Vec::new(),
                mask_mode: Default::default(),
                mask_only: false,
                keyforms: KeyformGrid::constant(MeshKeyform {
                    positions: corners.to_vec(),
                    opacity: 0.0,
                    draw_order: lips_keyform.draw_order + 1.0,
                    ..lips_keyform
                }),
                mirror: None,
                mirror_axis: None,
                depth: None,
            },
        })?;
        self.apply(Command::ReparentPreservingLook {
            element: ChildRef::Mesh(id.clone()),
            new_parent: Some(warp.clone()),
        })?;
        Ok(id)
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

    #[test]
    fn eye_open_closes_only_its_own_eye() {
        let project = build();
        let model = &project.model;
        let index = |name: &str| model.meshes.iter().position(|m| m.name == name).unwrap();
        let neutral = evaluate(model, &[]);
        let closed = evaluate(model, &[(sp::EYE_L_OPEN, 0.0)]);
        let (left, _) = difference(
            &neutral[index("eyelash-l")..=index("eyelash-l")],
            &closed[index("eyelash-l")..=index("eyelash-l")],
        );
        let (right, _) = difference(
            &neutral[index("eyelash-r")..=index("eyelash-r")],
            &closed[index("eyelash-r")..=index("eyelash-r")],
        );
        assert!(left > 3.0, "{left}");
        assert_eq!(right, 0.0);
    }

    /// コミットした `assets/samples/avatar.ugoe` が、今のジェネレータの出力と (浮動小数点の誤差を除いて) 同じこと。
    /// 違えば `just cli sample avatar -o assets/samples/avatar.ugoe` で作り直す。
    #[test]
    fn committed_sample_is_up_to_date() {
        let bytes = std::fs::read(repo_path("assets/samples/avatar.ugoe")).unwrap();
        let committed = ugoe_format::read_bundle(&bytes).unwrap().model;
        let fresh = build().model;
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
                (sp::MOUTH_OPEN_Y, 1.0),
                (sp::MOUTH_FORM, 1.0),
                (sp::BROW_R_Y, 1.0),
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
