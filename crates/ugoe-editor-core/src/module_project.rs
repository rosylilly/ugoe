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
//!   スロットの取り付け先の下へ移す (`ModuleContext::normalize`)。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_model::module::{DrawOrderRange, ModulePhysics, ModulePose, ModuleRequirements};
use ugoe_model::physics::{ColliderId, ColliderShape, PhysicsRigId, PhysicsSettings};
use ugoe_model::{DeformerId, MeshId, Model, Module, PartId, SCHEMA_VERSION, Slot, SlotId};
use ugoe_runtime::{ComposeError, compose, compose_physics};

use crate::command::{ChildRef, KeyformTarget};
use crate::physics_edit::default_settings;
use crate::texture::Textures;
use crate::{Command, EditError, deformer_edit};

mod split;

use split::Splitter;

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
    /// 本体に物理が無いのに、モジュールのリグがスロットの公開する本体の衝突体を参照している (本体に衝突体が無い。
    /// `docs/spec/modules.md#合成`)。本体に物理と衝突体を足すか、リグから外す (T0660)。
    #[error(
        "rig `{rig}` collides with `{collider}`, which slot `{slot}` exposes, but the base model has no physics. \
         Add physics with the collider `{collider}` to the base project, or remove the collider from the rig"
    )]
    ColliderNeedsBasePhysics {
        rig: String,
        collider: String,
        slot: String,
    },
    /// 本体の要素を変えた (`element` は ``parameter `AngleX` `` のような表記)。
    #[error(
        "{element} belongs to the base model and can't be edited here. Edit it in the base project"
    )]
    ReadOnlyBase { element: String },
    /// モジュールとして表せない (本体の要素への参照・接頭辞を外した ID の重なりなど)。
    #[error("cannot be kept as a module: {reason}")]
    Shape { reason: String },
    /// モジュールのプロジェクトでない ([`crate::Document::rebase_module_on`])。[`EditError::NotModule`]・
    /// [`crate::ProjectError::NotAModule`] に変わる。
    #[error("this is not a module project")]
    NotModule,
}

/// モジュールの物理のリグが参照する、スロットの公開する本体の衝突体 1 つ (本体に物理が無いと当たれない。
/// [`ModuleContext::drop_unavailable_colliders`] が外したもの。T0661)。ID はモジュールの中のもの。
/// エラー型と違い、呼び出し側へデータとして返すので ID は型付きで持つ (TS ではどちらも `string`。T1600)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ColliderReference {
    pub rig: PhysicsRigId,
    pub collider: ColliderId,
}

/// 編集の側に同じ意味の種類があるもの (読み取り専用の本体・モジュールとして表せない・モジュールでない) はそれに、
/// ほかは [`EditError::Module`] に包んで、コード (`module.*`) と引数を保つ (T0678)。
impl From<ModuleError> for EditError {
    fn from(error: ModuleError) -> Self {
        match error {
            ModuleError::ReadOnlyBase { element } => EditError::ReadOnlyBase { element },
            ModuleError::Shape { reason } => EditError::ModuleShape { reason },
            ModuleError::NotModule => EditError::NotModule,
            other => EditError::Module(other),
        }
    }
}

/// 次に読み込む本体の版 ([`ModuleContext::base_revision`])。
static NEXT_BASE_REVISION: AtomicU64 = AtomicU64::new(1);

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
    /// 本体の版 ([`ModuleContext::base_revision`])。
    base_revision: u64,
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
        let revision = NEXT_BASE_REVISION.fetch_add(1, Ordering::Relaxed);
        Self::with_revision(base, revision, slot, name.into(), requires)
    }

    fn with_revision(
        base: ModuleBase,
        base_revision: u64,
        slot: &SlotId,
        name: String,
        requires: ModuleRequirements,
    ) -> Result<Self, ModuleError> {
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
            base_revision,
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
        Self::with_revision(
            self.base.clone(),
            self.base_revision,
            slot,
            name.into(),
            requires,
        )
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

    /// 本体の版。本体を読み込むたび ([`ModuleContext::new`]・[`ModuleContext::rebased`]) にプロセスの中で
    /// 重ならない番号になり、名前・スロット・条件を替えても ([`ModuleContext::reconfigured`]) 変わらない。
    /// 本体から作るもの (プレビューで比べる本体だけの `.ugoe`) を、本体が変わったときだけ作り直すのに使う。
    pub fn base_revision(&self) -> u64 {
        self.base_revision
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
            Some(self.compose_physics(&module)?)
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

    /// モジュール `module` の物理を、本体の物理 (なければ既定の設定) に合成する (`compose_physics`)。
    ///
    /// 作者のツールでは、本体に物理が無くても既定の設定で合成するので、スロットが公開する本体の衝突体への参照は
    /// 指す先が無くエラーになる (保存のときに参照が黙って消えないように。T0655)。そのときは
    /// [`ModuleError::ColliderNeedsBasePhysics`] で、本体に物理を足すよう案内する。
    pub(crate) fn compose_physics(&self, module: &Module) -> Result<PhysicsSettings, ModuleError> {
        compose_physics(&self.base.model, &self.expected_physics, &[module]).map_err(|error| {
            match error {
                ComposeError::UnknownCollider { rig, collider, .. }
                    if self.base.physics.is_none()
                        && self.slot.colliders.iter().any(|c| c == collider.as_str()) =>
                {
                    ModuleError::ColliderNeedsBasePhysics {
                        rig,
                        collider,
                        slot: self.slot.id.to_string(),
                    }
                }
                other => other.into(),
            }
        })
    }

    /// スロットが公開する本体の衝突体のうち、本体の物理に無いもの (本体の物理の検証はスロットの `colliders` を
    /// 確かめるので、実際には本体に物理が無いときのスロットの `colliders` のすべて)。モジュールのリグはこれに当たれない。
    pub fn unavailable_colliders(&self) -> Vec<String> {
        let base = self.base.physics.as_ref();
        self.slot
            .colliders
            .iter()
            .filter(|id| !base.is_some_and(|p| p.colliders.iter().any(|c| &c.id == *id)))
            .map(ToString::to_string)
            .collect()
    }

    /// モジュール `module` のリグから、[`Self::unavailable_colliders`] (本体に無い、スロットの公開する衝突体) への
    /// 参照を外し、外したものを返す (リグの順・参照の順)。モジュール自身の衝突体への参照は残す。
    /// 物理の無い本体でモジュールのプロジェクトを開くとき、利用者が外すことを確かめてから使う (黙って外さない。T0655・T0661)。
    pub fn drop_unavailable_colliders(&self, module: &mut Module) -> Vec<ColliderReference> {
        let unavailable = self.unavailable_colliders();
        let own: HashSet<&str> = module
            .physics
            .colliders
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        let mut dropped = Vec::new();
        for rig in &mut module.physics.rigs {
            rig.colliders.retain(|collider| {
                let id = collider.as_str();
                let drop = !own.contains(id) && unavailable.iter().any(|u| u == id);
                if drop {
                    dropped.push(ColliderReference {
                        rig: rig.id.clone(),
                        collider: collider.clone(),
                    });
                }
                !drop
            });
        }
        dropped
    }

    fn prefixed(&self, id: &str) -> String {
        format!("{}/{id}", self.name)
    }

    /// この情報の作業用のモデルの要素 `target` の、名前を替える前の情報 `previous` の作業用のモデルでの ID
    /// (接頭辞を付け替える。本体の要素はそのまま)。`keep_look` の付け替えの確かめで、前のモデルと突き合わせるのに使う。
    pub(crate) fn previous_target(&self, previous: &Self, target: &KeyformTarget) -> KeyformTarget {
        let renamed = |id: &str, base: bool| {
            id.strip_prefix(self.name.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .filter(|_| !base)
                .map(|rest| previous.prefixed(rest))
        };
        match target {
            KeyformTarget::Part(id) => renamed(id.as_str(), self.is_base_part(id))
                .map(|id| KeyformTarget::Part(id.as_str().into())),
            KeyformTarget::Mesh(id) => renamed(id.as_str(), self.is_base_mesh(id))
                .map(|id| KeyformTarget::Mesh(id.as_str().into())),
            KeyformTarget::Deformer(id) => renamed(id.as_str(), self.is_base_deformer(id))
                .map(|id| KeyformTarget::Deformer(id.as_str().into())),
            KeyformTarget::Glue(id) => {
                let base = self.base.model.glues.iter().any(|g| &g.id == id);
                renamed(id.as_str(), base).map(|id| KeyformTarget::Glue(id.as_str().into()))
            }
        }
        .unwrap_or_else(|| target.clone())
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

    /// 前の取り付け先 `old_anchor` の局所座標のまま合成し直した作業用のモデルと物理で、モジュールの最上位のデフォーマと
    /// メッシュ、取り付け先に付く衝突体を、このスロットの取り付け先の局所座標に写し直す (見た目を保って移す。
    /// `Command::SetModuleSettings` の `keep_look`)。写し方は `ReparentPreservingLook` と同じで、基準の姿で見た目が変わらない。
    ///
    /// 物理のリグの粒子は写し直さない。モジュールのリグの根元はモジュールのデフォーマ (出力先か `frame`) の局所座標か
    /// キャンバス座標で表され (`docs/spec/physics.md#物理空間と根元の取り付け先`)、どちらも基準の姿でのキャンバスでの
    /// 位置がデフォーマの写し直しで保たれるため。
    pub(crate) fn keep_look(
        &self,
        model: &mut Model,
        physics: Option<&mut PhysicsSettings>,
        old_anchor: Option<&DeformerId>,
        touched: &mut Vec<KeyformTarget>,
    ) -> Result<(), EditError> {
        let anchor = self.slot.anchor.as_ref();
        if anchor == old_anchor {
            return Ok(());
        }
        if let Some(physics) = physics {
            // 取り付け先はどちらも本体のデフォーマなので、モジュールの要素を動かす前に求めても同じ。
            let rest = deformer_edit::Rest::new(model)?;
            let (from, to) = (rest.frame(old_anchor), rest.frame(anchor));
            let remap = |p: &mut Vec2| -> Result<(), EditError> {
                *p = to
                    .inverse(from.map(*p))
                    .map_err(|reason| EditError::DeformerEdit {
                        reason: format!("cannot keep the physics colliders in place: {reason}"),
                    })?;
                Ok(())
            };
            let base = &self.expected_physics;
            for collider in &mut physics.colliders {
                if collider.attach.as_ref() != anchor
                    || base.colliders.iter().any(|b| b.id == collider.id)
                {
                    continue;
                }
                match &mut collider.shape {
                    ColliderShape::Circle { center, .. } => remap(center)?,
                    ColliderShape::Capsule { a, b, .. } => {
                        remap(a)?;
                        remap(b)?;
                    }
                    _ => {}
                }
            }
        }
        let mut tops = Vec::new();
        for d in &mut model.deformers {
            if d.parent.as_ref() == anchor && !self.is_base_deformer(&d.id) {
                d.parent = old_anchor.cloned();
                tops.push(ChildRef::Deformer(d.id.clone()));
            }
        }
        for m in &mut model.meshes {
            if m.parent.as_ref() == anchor && !self.is_base_mesh(&m.id) {
                m.parent = old_anchor.cloned();
                tops.push(ChildRef::Mesh(m.id.clone()));
            }
        }
        // 局所座標は前の取り付け先のものなので、親を前の取り付け先に戻した姿から付け替える。
        for element in &tops {
            deformer_edit::reparent(model, element, anchor, touched)?;
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
                        .map_or_else(String::new, |r| r.id.to_string());
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
                        .map_or_else(String::new, |c| c.id.to_string());
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
            |g| g.id.to_string(),
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
    .or_else(|| by_id("glue", &expected.glues, &actual.glues, |g| g.id.to_string()))
    .or_else(|| {
        by_id("texture", &expected.textures, &actual.textures, |t| {
            t.name.clone()
        })
    })
    .or_else(|| {
        by_id("hit area", &expected.hit_areas, &actual.hit_areas, |h| {
            h.id.to_string()
        })
    })
    .or_else(|| by_id("slot", &expected.slots, &actual.slots, |s| s.id.to_string()))
    .or_else(|| (expected.pose != actual.pose).then(|| "the pose".to_owned()))
    .or_else(|| (expected.avatar != actual.avatar).then(|| "the avatar profile".to_owned()))
    .unwrap_or_else(|| "the model settings (effects, lighting)".to_owned())
}
