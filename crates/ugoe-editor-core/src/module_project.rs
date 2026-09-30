//! 着せ替えのモジュールのプロジェクト (ADR 0024、設計: `docs/editor.md#モジュールのプロジェクト`)。
//!
//! モジュールは本体を参照する別のプロジェクトとして作る。編集中の [`crate::Document`] は、本体と
//! モジュールを合成 (`ugoe_runtime::compose`) した「作業用のモデル」を持ち、`Command` はモデルと同じものを
//! そのまま使う。本体は読み取り専用の下敷きで、コマンドの適用のたびに [`ModuleContext::split`] で
//! 作業用のモデルをモジュールと本体に分け、本体が変わっていないこと、モジュールとして表せることを確かめる。
//!
//! - 作業用のモデルの、モジュールの要素の ID は合成と同じく `<モジュールの名前>/<ID>` になる
//!   (本体の ID との衝突を避けるため)。新しく作った要素は接頭辞なしの ID のままでよい。分けるときに接頭辞を外す。
//! - 描画順は本体の空間 (スロットの範囲に写したもの) で編集し、分けるときに 0〜1000 に戻す。
//! - キャンバスの直下に作られたモジュールのデフォーマとメッシュは、見た目を保ったまま
//!   スロットの取り付け先の下へ移す ([`ModuleContext::normalize`])。

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use ugoe_core::Id;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::keyform::KeyformGrid;
use ugoe_model::module::{DrawOrderRange, ModulePhysics, ModulePose, ModuleRequirements};
use ugoe_model::physics::{PhysicsSettings, PhysicsTarget, VertexOwner};
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{DeformerId, MeshId, Model, Module, PartId, SCHEMA_VERSION, Slot, SlotId};
use ugoe_runtime::{ComposeError, compose, compose_physics};

use crate::command::{ChildRef, KeyformTarget};
use crate::physics_edit::default_settings;
use crate::texture::Textures;
use crate::{Command, EditError, deformer_edit};

/// モジュールの描画順の範囲 (0〜1000)。スロットの範囲に写される。
pub const MODULE_DRAW_ORDER_RANGE: f32 = 1000.0;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModuleError {
    #[error("module name `{0}` must be non-empty, contain no `/` and not start with `#`")]
    InvalidName(String),
    #[error("the base model has no slot `{0}`")]
    UnknownSlot(String),
    #[error(transparent)]
    Compose(#[from] ComposeError),
    /// 本体の要素を変えた (`element` は ``parameter `AngleX` `` のような表記)。
    #[error(
        "{element} belongs to the base model and can't be edited here. Edit it in the base project"
    )]
    ReadOnlyBase { element: String },
    /// モジュールとして表せない (本体の要素への参照・接頭辞を外した ID の重なりなど)。
    #[error("cannot be kept as a module: {reason}")]
    Shape { reason: String },
}

impl From<ModuleError> for EditError {
    fn from(error: ModuleError) -> Self {
        match error {
            ModuleError::ReadOnlyBase { element } => EditError::ReadOnlyBase { element },
            other => EditError::ModuleShape {
                reason: other.to_string(),
            },
        }
    }
}

/// モジュールのプロジェクトが参照する本体 (読み取り専用)。
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleBase {
    /// 本体のファイル (`.ugoeproj` のディレクトリか `.ugoe`) の絶対パス。
    pub path: PathBuf,
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    /// 本体の画像 (テクスチャと法線マップの名前 → PNG)。
    pub textures: Textures,
}

/// 作業用のモデル・物理・画像。
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    pub textures: Textures,
}

/// モジュールのプロジェクトの情報: 本体・スロット・モジュールの名前と条件。本体は編集中に変わらない
/// (読み直すと替わる)。名前・スロット・条件は `Command::SetModuleSettings` で替わる (`Document` がスナップショットごとに持つ)。
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleContext {
    base: ModuleBase,
    slot: Slot,
    name: String,
    requires: ModuleRequirements,
    /// 作業用のモデルから本体の要素だけを残したときの形 (置き換えるパーツを隠したもの)。
    expected: Model,
    /// 作業用の物理の、本体の部分 (本体に物理がなければ既定の設定)。
    expected_physics: PhysicsSettings,
    /// `expected` のパーツ・デフォーマ・メッシュの ID。
    base_parts: HashSet<String>,
    base_deformers: HashSet<String>,
    base_meshes: HashSet<String>,
}

impl ModuleContext {
    /// 本体 `base` のスロット `slot` に付ける、名前 `name` のモジュールの情報。
    pub fn new(
        base: ModuleBase,
        slot: &SlotId,
        name: impl Into<String>,
        requires: ModuleRequirements,
    ) -> Result<Self, ModuleError> {
        let name = name.into();
        if !Module::is_valid_name(&name) {
            return Err(ModuleError::InvalidName(name));
        }
        let slot = base
            .model
            .slots
            .iter()
            .find(|s| &s.id == slot)
            .cloned()
            .ok_or_else(|| ModuleError::UnknownSlot(slot.to_string()))?;
        let empty = empty_module(&name, &slot.id);
        let expected = compose(&base.model, &[&empty])?;
        let expected_physics = base.physics.clone().unwrap_or_else(default_settings);
        let base_parts = expected.parts.iter().map(|p| p.id.to_string()).collect();
        let base_deformers = expected
            .deformers
            .iter()
            .map(|d| d.id.to_string())
            .collect();
        let base_meshes = expected.meshes.iter().map(|m| m.id.to_string()).collect();
        Ok(Self {
            base,
            slot,
            name,
            requires,
            expected,
            expected_physics,
            base_parts,
            base_deformers,
            base_meshes,
        })
    }

    /// 本体は同じで、スロット・名前・条件を替えた情報 ([`Command::SetModuleSettings`])。
    pub fn reconfigured(
        &self,
        slot: &SlotId,
        name: impl Into<String>,
        requires: ModuleRequirements,
    ) -> Result<Self, ModuleError> {
        Self::new(self.base.clone(), slot, name, requires)
    }

    /// スロット・名前・条件は同じで、本体を `base` に替えた情報 (本体のファイルが変わったときの読み直し)。
    pub fn rebased(&self, base: ModuleBase) -> Result<Self, ModuleError> {
        Self::new(
            base,
            &self.slot.id,
            self.name.clone(),
            self.requires.clone(),
        )
    }

    /// 作業用の ID から、合成で付いた接頭辞 (`<モジュールの名前>/`) を外したもの (表示用)。
    pub fn display_id<'a>(&self, id: &'a str) -> &'a str {
        self.unprefixed(id)
    }

    pub fn base(&self) -> &ModuleBase {
        &self.base
    }

    pub fn slot(&self) -> &Slot {
        &self.slot
    }

    /// モジュールの名前 (作業用のモデルの ID の接頭辞にも使う)。
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn requires(&self) -> &ModuleRequirements {
        &self.requires
    }

    /// 要素が本体のものか (作業用のモデルの ID で。置き換えるパーツを隠すために挟んだパーツを含む)。
    pub fn is_base_part(&self, id: &PartId) -> bool {
        self.base_parts.contains(id.as_str())
    }

    pub fn is_base_deformer(&self, id: &DeformerId) -> bool {
        self.base_deformers.contains(id.as_str())
    }

    pub fn is_base_mesh(&self, id: &MeshId) -> bool {
        self.base_meshes.contains(id.as_str())
    }

    /// 空のモジュール (新規作成用)。
    pub fn empty_module(&self) -> Module {
        let mut module = empty_module(&self.name, &self.slot.id);
        module.requires = self.requires.clone();
        module
    }

    /// モジュール `module` と、その画像 `images` (テクスチャと法線マップの名前 → PNG) から作業用のモデルを作る。
    /// モジュールの名前・スロット・条件はこの情報のものを使う。
    pub fn workspace(&self, module: &Module, images: &Textures) -> Result<Workspace, ModuleError> {
        let mut module = module.clone();
        module.name = self.name.clone();
        module.slot = self.slot.id.clone();
        module.requires = self.requires.clone();
        let mut model = compose(&self.base.model, &[&module])?;
        let physics = if self.base.physics.is_some() || !module.physics.is_empty() {
            Some(compose_physics(
                &self.base.model,
                &self.expected_physics,
                &[&module],
            )?)
        } else {
            None
        };
        // 合成は法線マップの名前を付け替えないので、本体の画像と重ならないようにここで付け替える。
        let mut textures = self.base.textures.clone();
        let offset = self.base.model.textures.len();
        for (texture, own) in model.textures[offset..].iter_mut().zip(&module.textures) {
            if let Some(png) = images.get(&own.name) {
                textures.insert(texture.name.clone(), png.clone());
            }
            if let Some(normal) = &own.normal_map {
                let renamed = self.prefixed(normal);
                if let Some(png) = images.get(normal) {
                    textures.insert(renamed.clone(), png.clone());
                }
                texture.normal_map = Some(renamed);
            }
        }
        Ok(Workspace {
            model,
            physics,
            textures,
        })
    }

    fn prefixed(&self, id: &str) -> String {
        format!("{}/{id}", self.name)
    }

    /// 作業用の ID からモジュールの中の ID を作る (接頭辞を外す)。
    fn unprefixed<'a>(&self, id: &'a str) -> &'a str {
        id.strip_prefix(self.name.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(id)
    }

    /// コマンドを適用した直後の作業用のモデルを整える: キャンバスの直下にあるモジュールのデフォーマと
    /// メッシュを、見た目を保ったままスロットの取り付け先の下へ移し、最上位のパーツをスロットのパーツの下へ移す。
    pub(crate) fn normalize(
        &self,
        model: &mut Model,
        touched: &mut Vec<KeyformTarget>,
    ) -> Result<(), EditError> {
        if let Some(part) = &self.slot.part {
            for p in &mut model.parts {
                if p.parent.is_none() && !self.is_base_part(&p.id) {
                    p.parent = Some(part.clone());
                }
            }
        }
        let Some(anchor) = self.slot.anchor.clone() else {
            return Ok(());
        };
        let mut roots: Vec<ChildRef> = model
            .deformers
            .iter()
            .filter(|d| d.parent.is_none() && !self.is_base_deformer(&d.id))
            .map(|d| ChildRef::Deformer(d.id.clone()))
            .collect();
        roots.extend(
            model
                .meshes
                .iter()
                .filter(|m| m.parent.is_none() && !self.is_base_mesh(&m.id))
                .map(|m| ChildRef::Mesh(m.id.clone())),
        );
        for element in &roots {
            deformer_edit::reparent(model, element, Some(&anchor), touched)?;
        }
        Ok(())
    }

    /// 作業用のモデル・物理・画像を、モジュールとその画像 (モジュールの中の名前 → PNG) に分ける。
    ///
    /// 本体の部分が本体と同じでなければ [`ModuleError::ReadOnlyBase`]、モジュールとして表せなければ
    /// [`ModuleError::Shape`]。本体との整合 (マスク・衝突体・パラメータの公開) は検査しない
    /// (書き出しの前に合成して確かめる)。
    pub fn split(
        &self,
        model: &Model,
        physics: Option<&PhysicsSettings>,
        textures: &Textures,
    ) -> Result<(Module, Textures), ModuleError> {
        self.check_base(model, physics, textures)?;
        Splitter::new(self, model)?.module(model, physics, textures)
    }

    /// 作業用のモデルの本体の部分が本体と同じことを確かめる。
    fn check_base(
        &self,
        model: &Model,
        physics: Option<&PhysicsSettings>,
        textures: &Textures,
    ) -> Result<(), ModuleError> {
        let expected = &self.expected;
        let mut residue = model.clone();
        residue.parts.retain(|p| self.is_base_part(&p.id));
        residue.deformers.retain(|d| self.is_base_deformer(&d.id));
        residue.meshes.retain(|m| self.is_base_mesh(&m.id));
        let base_glues: HashSet<&str> = expected.glues.iter().map(|g| g.id.as_str()).collect();
        residue.glues.retain(|g| base_glues.contains(g.id.as_str()));
        let offset = expected.textures.len();
        if residue.textures.len() < offset {
            return Err(read_only("the textures".into()));
        }
        residue.textures.truncate(offset);
        let groups = expected.pose.groups.len();
        if residue.pose.groups.len() < groups {
            return Err(read_only("the pose".into()));
        }
        residue.pose.groups.truncate(groups);
        for entry in residue.pose.groups.iter_mut().flat_map(|g| &mut g.entries) {
            entry.links.retain(|p| self.is_base_part(p));
        }
        if residue != *expected {
            return Err(read_only(first_difference(expected, &residue)));
        }
        for (name, png) in &self.base.textures {
            if textures.get(name).is_none_or(|now| now != png) {
                return Err(read_only(format!("the image `{name}`")));
            }
        }

        let base = &self.expected_physics;
        match physics {
            None if self.base.physics.is_none() => {}
            None => return Err(read_only("the physics settings".into())),
            Some(now) => {
                let globals_same =
                    now.gravity == base.gravity && now.wind == base.wind && now.fps == base.fps;
                if !globals_same {
                    return Err(read_only(
                        "the physics settings (gravity, wind, frequency)".into(),
                    ));
                }
                let base_rigs: Vec<_> = now
                    .rigs
                    .iter()
                    .filter(|r| base.rigs.iter().any(|b| b.id == r.id))
                    .collect();
                if base_rigs.len() != base.rigs.len()
                    || base_rigs.iter().zip(&base.rigs).any(|(a, b)| *a != b)
                {
                    let id = base
                        .rigs
                        .iter()
                        .find(|b| !now.rigs.contains(b))
                        .map_or_else(String::new, |r| r.id.clone());
                    return Err(read_only(format!("physics rig `{id}`")));
                }
                let base_colliders: Vec<_> = now
                    .colliders
                    .iter()
                    .filter(|c| base.colliders.iter().any(|b| b.id == c.id))
                    .collect();
                if base_colliders.len() != base.colliders.len()
                    || base_colliders
                        .iter()
                        .zip(&base.colliders)
                        .any(|(a, b)| *a != b)
                {
                    let id = base
                        .colliders
                        .iter()
                        .find(|b| !now.colliders.contains(b))
                        .map_or_else(String::new, |c| c.id.clone());
                    return Err(read_only(format!("physics collider `{id}`")));
                }
            }
        }
        Ok(())
    }

    /// 描画順を、モジュールの空間 (0〜1000) からスロットの範囲へ写す (合成と同じ)。
    pub fn map_draw_order(&self, order: f32) -> f32 {
        map_draw_order(self.slot.draw_order, order)
    }

    /// PSD の取り込みなどで作ったコマンドの、新しいメッシュの描画順 (0〜1000) をスロットの範囲に写す。
    pub fn map_new_meshes(&self, command: &mut Command) {
        match command {
            Command::AddMesh { mesh } => {
                for keyform in mesh.keyforms.all_keyforms_mut() {
                    keyform.draw_order = self.map_draw_order(keyform.draw_order);
                }
            }
            Command::Batch { commands, .. } => {
                for command in commands {
                    self.map_new_meshes(command);
                }
            }
            _ => {}
        }
    }
}

fn read_only(element: String) -> ModuleError {
    ModuleError::ReadOnlyBase { element }
}

fn shape(reason: String) -> ModuleError {
    ModuleError::Shape { reason }
}

fn empty_module(name: &str, slot: &SlotId) -> Module {
    Module {
        schema_version: SCHEMA_VERSION,
        name: name.to_owned(),
        slot: slot.clone(),
        requires: ModuleRequirements::default(),
        parts: Vec::new(),
        deformers: Vec::new(),
        meshes: Vec::new(),
        glues: Vec::new(),
        textures: Vec::new(),
        physics: ModulePhysics::default(),
        pose: ModulePose::default(),
    }
}

fn map_draw_order(range: DrawOrderRange, order: f32) -> f32 {
    range.min + (range.max - range.min) * order / MODULE_DRAW_ORDER_RANGE
}

/// [`map_draw_order`] の逆。範囲の幅が 0 なら 500 (中央) にする (合成ではどれも `min` になる)。
/// 小数第 3 位で丸めた値が同じ描画順に写るなら丸めた値を使う (保存のたびに誤差が積もらないように)。
fn unmap_draw_order(range: DrawOrderRange, order: f32) -> f32 {
    let width = range.max - range.min;
    if width <= 0.0 {
        return MODULE_DRAW_ORDER_RANGE / 2.0;
    }
    let raw = (order - range.min) * MODULE_DRAW_ORDER_RANGE / width;
    let rounded = (raw * 1000.0).round() / 1000.0;
    if map_draw_order(range, rounded) == order {
        rounded
    } else {
        raw
    }
}

/// 本体と作業用のモデルの本体の部分で、最初に違う要素の表記。
fn first_difference(expected: &Model, actual: &Model) -> String {
    fn by_id<T: PartialEq>(
        kind: &str,
        expected: &[T],
        actual: &[T],
        id: impl Fn(&T) -> String,
    ) -> Option<String> {
        for e in expected {
            match actual.iter().find(|a| id(a) == id(e)) {
                Some(a) if a == e => {}
                _ => return Some(format!("{kind} `{}`", id(e))),
            }
        }
        if let Some(extra) = actual
            .iter()
            .find(|a| !expected.iter().any(|e| id(e) == id(a)))
        {
            return Some(format!("{kind} `{}`", id(extra)));
        }
        (expected != actual).then(|| format!("the order of the {kind}s"))
    }
    if expected.meta != actual.meta {
        return "the canvas".into();
    }
    by_id("parameter", &expected.parameters, &actual.parameters, |p| {
        p.id.to_string()
    })
    .or_else(|| {
        by_id(
            "parameter group",
            &expected.parameter_groups,
            &actual.parameter_groups,
            |g| g.id.clone(),
        )
    })
    .or_else(|| by_id("part", &expected.parts, &actual.parts, |p| p.id.to_string()))
    .or_else(|| {
        by_id("deformer", &expected.deformers, &actual.deformers, |d| {
            d.id.to_string()
        })
    })
    .or_else(|| {
        by_id("mesh", &expected.meshes, &actual.meshes, |m| {
            m.id.to_string()
        })
    })
    .or_else(|| by_id("glue", &expected.glues, &actual.glues, |g| g.id.clone()))
    .or_else(|| {
        by_id("texture", &expected.textures, &actual.textures, |t| {
            t.name.clone()
        })
    })
    .or_else(|| {
        by_id("hit area", &expected.hit_areas, &actual.hit_areas, |h| {
            h.id.clone()
        })
    })
    .or_else(|| by_id("slot", &expected.slots, &actual.slots, |s| s.id.to_string()))
    .or_else(|| (expected.pose != actual.pose).then(|| "the pose".to_owned()))
    .or_else(|| (expected.avatar != actual.avatar).then(|| "the avatar profile".to_owned()))
    .unwrap_or_else(|| "the model settings (effects, lighting)".to_owned())
}

/// 作業用のモデルからモジュールを組み立てる。
struct Splitter<'a> {
    context: &'a ModuleContext,
    /// 作業用の ID → モジュールの中の ID (種類ごと)。
    parts: BTreeMap<String, String>,
    deformers: BTreeMap<String, String>,
    meshes: BTreeMap<String, String>,
}

impl<'a> Splitter<'a> {
    fn new(context: &'a ModuleContext, model: &Model) -> Result<Self, ModuleError> {
        let parts = context.renames(
            "part",
            model
                .parts
                .iter()
                .filter(|p| !context.is_base_part(&p.id))
                .map(|p| p.id.as_str()),
        )?;
        let deformers = context.renames(
            "deformer",
            model
                .deformers
                .iter()
                .filter(|d| !context.is_base_deformer(&d.id))
                .map(|d| d.id.as_str()),
        )?;
        let meshes = context.renames(
            "mesh",
            model
                .meshes
                .iter()
                .filter(|m| !context.is_base_mesh(&m.id))
                .map(|m| m.id.as_str()),
        )?;
        Ok(Self {
            context,
            parts,
            deformers,
            meshes,
        })
    }

    fn part(&self, id: &PartId, what: &str) -> Result<PartId, ModuleError> {
        self.parts
            .get(id.as_str())
            .map(|s| Id::new(s.clone()))
            .ok_or_else(|| shape(format!("{what} refers to the base part `{id}`")))
    }

    fn deformer(&self, id: &DeformerId, what: &str) -> Result<DeformerId, ModuleError> {
        self.deformers
            .get(id.as_str())
            .map(|s| Id::new(s.clone()))
            .ok_or_else(|| shape(format!("{what} refers to the base deformer `{id}`")))
    }

    fn mesh(&self, id: &MeshId, what: &str) -> Result<MeshId, ModuleError> {
        self.meshes
            .get(id.as_str())
            .map(|s| Id::new(s.clone()))
            .ok_or_else(|| shape(format!("{what} refers to the base mesh `{id}`")))
    }

    /// 親のデフォーマ。スロットの取り付け先は最上位 (`None`) に戻す。
    fn parent(
        &self,
        parent: Option<&DeformerId>,
        what: &str,
    ) -> Result<Option<DeformerId>, ModuleError> {
        let anchor = self.context.slot.anchor.as_ref();
        match parent {
            Some(p) if Some(p) == anchor => Ok(None),
            Some(p) => self.deformer(p, what).map(Some),
            None if anchor.is_none() => Ok(None),
            None => Err(shape(format!(
                "{what} must be under the slot anchor `{}`",
                anchor.map_or("", |a| a.as_str())
            ))),
        }
    }

    /// 格子のミラー元 (ブレンドシェイプ格子を含む) を、同じ種類のモジュールの要素の ID に付け替える。
    fn grid<T>(
        &self,
        grid: &mut KeyformGrid<T>,
        ids: &BTreeMap<String, String>,
        what: &str,
    ) -> Result<(), ModuleError> {
        let rename = |g: &mut KeyformGrid<T>| -> Result<(), ModuleError> {
            if let Some(source) = &mut g.mirror_source {
                source.id = ids.get(&source.id).cloned().ok_or_else(|| {
                    shape(format!(
                        "{what} mirrors keyforms from the base element `{}`",
                        source.id
                    ))
                })?;
            }
            Ok(())
        };
        for shape_grid in &mut grid.blend_shapes {
            rename(shape_grid)?;
        }
        rename(grid)
    }

    fn module(
        &self,
        model: &Model,
        physics: Option<&PhysicsSettings>,
        images: &Textures,
    ) -> Result<(Module, Textures), ModuleError> {
        let context = self.context;
        let slot = &context.slot;
        let mut module = context.empty_module();

        for part in model.parts.iter().filter(|p| !context.is_base_part(&p.id)) {
            let what = format!("part `{}`", part.id);
            let mut part = part.clone();
            part.id = self.part(&part.id, &what)?;
            part.parent = match &part.parent {
                None => None,
                Some(p) if Some(p) == slot.part.as_ref() => None,
                Some(p) => Some(self.part(p, &what)?),
            };
            self.grid(&mut part.keyforms, &self.parts, &what)?;
            module.parts.push(part);
        }
        for deformer in model
            .deformers
            .iter()
            .filter(|d| !context.is_base_deformer(&d.id))
        {
            let what = format!("deformer `{}`", deformer.id);
            let mut deformer = deformer.clone();
            deformer.id = self.deformer(&deformer.id, &what)?;
            deformer.part = self.part(&deformer.part, &what)?;
            deformer.parent = self.parent(deformer.parent.as_ref(), &what)?;
            deformer.mirror = match &deformer.mirror {
                Some(m) => Some(self.deformer(m, &what)?),
                None => None,
            };
            match &mut deformer.kind {
                DeformerKind::Warp(warp) => {
                    self.grid(&mut warp.keyforms, &self.deformers, &what)?
                }
                DeformerKind::Rotation(rotation) => {
                    self.grid(&mut rotation.keyforms, &self.deformers, &what)?;
                }
                DeformerKind::Unknown => {}
            }
            module.deformers.push(deformer);
        }
        let offset = context.expected.textures.len() as u32;
        for mesh in model.meshes.iter().filter(|m| !context.is_base_mesh(&m.id)) {
            let what = format!("mesh `{}`", mesh.id);
            let mut mesh = mesh.clone();
            mesh.id = self.mesh(&mesh.id, &what)?;
            mesh.part = self.part(&mesh.part, &what)?;
            mesh.parent = self.parent(mesh.parent.as_ref(), &what)?;
            mesh.texture = mesh
                .texture
                .checked_sub(offset)
                .ok_or_else(|| shape(format!("{what} uses the base texture {}", mesh.texture)))?;
            mesh.mirror = match &mesh.mirror {
                Some(m) => Some(self.mesh(m, &what)?),
                None => None,
            };
            // 本体のメッシュへのマスクはそのまま残す (スロットの `mask_sources` かは合成が検査する)。
            for mask in &mut mesh.masks {
                if let Some(own) = self.meshes.get(mask.mesh.as_str()) {
                    mask.mesh = Id::new(own.clone());
                }
            }
            self.grid(&mut mesh.keyforms, &self.meshes, &what)?;
            for keyform in mesh.keyforms.all_keyforms_mut() {
                keyform.draw_order = unmap_draw_order(slot.draw_order, keyform.draw_order);
            }
            module.meshes.push(mesh);
        }
        let base_glues: HashSet<&str> = context
            .expected
            .glues
            .iter()
            .map(|g| g.id.as_str())
            .collect();
        let glues: Vec<_> = model
            .glues
            .iter()
            .filter(|g| !base_glues.contains(g.id.as_str()))
            .collect();
        let glue_ids = context.renames("glue", glues.iter().map(|g| g.id.as_str()))?;
        for glue in glues {
            let what = format!("glue `{}`", glue.id);
            let mut glue = glue.clone();
            glue.id = glue_ids[&glue.id].clone();
            glue.mesh_a = self.mesh(&glue.mesh_a, &what)?;
            glue.mesh_b = self.mesh(&glue.mesh_b, &what)?;
            self.grid(&mut glue.keyforms, &BTreeMap::new(), &what)?;
            module.glues.push(glue);
        }

        let own_textures = &model.textures[offset as usize..];
        let texture_names =
            context.renames("texture", own_textures.iter().map(|t| t.name.as_str()))?;
        let mut module_images = Textures::new();
        let mut copy_image = |from: &str, to: &str| -> Result<(), ModuleError> {
            let png = images
                .get(from)
                .ok_or_else(|| shape(format!("the image `{from}` is missing")))?;
            module_images.insert(to.to_owned(), png.clone());
            Ok(())
        };
        for texture in own_textures {
            let mut texture = texture.clone();
            let name = texture_names[&texture.name].clone();
            copy_image(&texture.name, &name)?;
            texture.name = name;
            if let Some(normal) = &texture.normal_map {
                let renamed = context.unprefixed(normal).to_owned();
                copy_image(normal, &renamed)?;
                texture.normal_map = Some(renamed);
            }
            module.textures.push(texture);
        }

        for (index, group) in model.pose.groups.iter().enumerate() {
            if index < context.expected.pose.groups.len() {
                for entry in &group.entries {
                    let links = entry
                        .links
                        .iter()
                        .filter(|p| !context.is_base_part(p))
                        .map(|p| self.part(p, "the pose"))
                        .collect::<Result<Vec<_>, _>>()?;
                    if !links.is_empty() {
                        module.pose.links.push(PoseEntry {
                            part: entry.part.clone(),
                            links,
                        });
                    }
                }
            } else {
                let entries = group
                    .entries
                    .iter()
                    .map(|entry| {
                        Ok(PoseEntry {
                            part: self.part(&entry.part, "a pose group of the module")?,
                            links: entry
                                .links
                                .iter()
                                .map(|p| self.part(p, "a pose group of the module"))
                                .collect::<Result<_, ModuleError>>()?,
                        })
                    })
                    .collect::<Result<_, ModuleError>>()?;
                module.pose.groups.push(PoseGroup { entries });
            }
        }

        if let Some(physics) = physics {
            self.physics(physics, &mut module)?;
        }
        Ok((module, module_images))
    }

    fn physics(&self, physics: &PhysicsSettings, module: &mut Module) -> Result<(), ModuleError> {
        let context = self.context;
        let base = &context.expected_physics;
        let rigs: Vec<_> = physics
            .rigs
            .iter()
            .filter(|r| !base.rigs.iter().any(|b| b.id == r.id))
            .collect();
        let colliders: Vec<_> = physics
            .colliders
            .iter()
            .filter(|c| !base.colliders.iter().any(|b| b.id == c.id))
            .collect();
        let rig_ids = context.renames("physics rig", rigs.iter().map(|r| r.id.as_str()))?;
        let collider_ids =
            context.renames("physics collider", colliders.iter().map(|c| c.id.as_str()))?;
        for rig in rigs {
            let what = format!("physics rig `{}`", rig.id);
            let mut rig = rig.clone();
            rig.id = rig_ids[&rig.id].clone();
            rig.frame = match &rig.frame {
                Some(f) => Some(self.deformer(f, &what)?),
                None => None,
            };
            // 本体の衝突体への参照はそのまま残す (スロットの `colliders` かは合成が検査する)。
            for collider in &mut rig.colliders {
                if let Some(own) = collider_ids.get(collider.as_str()) {
                    collider.clone_from(own);
                }
            }
            for output in &mut rig.outputs {
                match &mut output.target {
                    PhysicsTarget::DeformerAngle { deformer }
                    | PhysicsTarget::DeformerOffset { deformer } => {
                        *deformer = self.deformer(deformer, &what)?;
                    }
                    PhysicsTarget::Vertices { owner, .. } => match owner {
                        VertexOwner::Mesh(mesh) => *mesh = self.mesh(mesh, &what)?,
                        VertexOwner::Warp(warp) => *warp = self.deformer(warp, &what)?,
                    },
                    PhysicsTarget::Parameter { .. } | PhysicsTarget::Unknown => {}
                }
            }
            module.physics.rigs.push(rig);
        }
        for collider in colliders {
            let what = format!("physics collider `{}`", collider.id);
            let mut collider = collider.clone();
            collider.id = collider_ids[&collider.id].clone();
            collider.attach = self.parent(collider.attach.as_ref(), &what)?;
            module.physics.colliders.push(collider);
        }
        Ok(())
    }
}

impl ModuleContext {
    /// 作業用の ID → モジュールの中の ID。接頭辞を外すと重なるなら [`ModuleError::Shape`]。
    fn renames<'a>(
        &self,
        kind: &str,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<BTreeMap<String, String>, ModuleError> {
        let mut renames = BTreeMap::new();
        let mut taken: BTreeMap<String, &str> = BTreeMap::new();
        for id in ids {
            let own = self.unprefixed(id).to_owned();
            if let Some(other) = taken.insert(own.clone(), id) {
                return Err(shape(format!(
                    "{kind}s `{other}` and `{id}` would both be `{own}` in the module; rename one"
                )));
            }
            renames.insert(id.to_owned(), own);
        }
        Ok(renames)
    }
}
