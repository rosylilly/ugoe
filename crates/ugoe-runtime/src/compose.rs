//! 着せ替え: 本体のモデルとモジュールを合成して 1 つのモデルを作る (`docs/spec/modules.md#合成`)。
//!
//! 合成したモデルは通常のモデルと同じようにコンパイル ([`crate::CompiledModel::compile`]) する。
//! 組み替えは合成とコンパイルのやり直しで行い、パラメータの値は
//! [`crate::ParameterValues::carry_over`] で引き継ぐ。

use std::collections::{HashMap, HashSet};

use ugoe_core::Id;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::keyform::KeyformGrid;
use ugoe_model::module::DrawOrderRange;
use ugoe_model::part::PartKeyform;
use ugoe_model::physics::{ColliderId, PhysicsSettings, PhysicsTarget, VertexOwner};
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{DeformerId, Model, Module, Part, PartId, Slot, standard};

/// 置き換えられるパーツを隠すために挟むパーツの ID の接頭辞 (`#replaced/<パーツの ID>`)。
/// モジュールの名前は `#` で始まれないので、モジュールの要素の ID とは衝突しない。
pub const REPLACED_PART_PREFIX: &str = "#replaced/";

/// 合成 ([`compose`]) の失敗。モジュールが本体に取り付けられない理由。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ComposeError {
    /// モジュールの名前が空か、`/` を含むか、`#` で始まる。中身はモジュールの名前。
    #[error("module name `{0}` must be non-empty, contain no `/` and not start with `#`")]
    InvalidModuleName(String),
    /// 同じ名前のモジュールを 2 つ以上取り付けようとした。中身はモジュールの名前。
    #[error("module name `{0}` is used more than once")]
    DuplicateModule(String),
    /// モジュールの取り付け先のスロットが本体に無い。
    #[error("module `{module}`: slot `{slot}` does not exist in the base model")]
    UnknownSlot {
        /// モジュールの名前。
        module: String,
        /// 取り付け先のスロットの ID。
        slot: String,
    },
    /// 1 つのスロットに 2 つのモジュールを取り付けようとした。
    #[error("slot `{slot}` already has module `{first}`, cannot also attach `{second}`")]
    SlotOccupied {
        /// スロットの ID。
        slot: String,
        /// 先にそのスロットに付いたモジュールの名前。
        first: String,
        /// 後から付けようとしたモジュールの名前。
        second: String,
    },
    /// 本体のアバタープロファイルのレベルが、モジュールの求めるレベルに届かない。
    #[error("module `{module}` requires avatar profile level {required}, base has {actual:?}")]
    AvatarLevel {
        /// モジュールの名前。
        module: String,
        /// モジュールが求めるレベル (`requires.avatar_level`)。
        required: u8,
        /// 本体のレベル。本体がアバタープロファイルを持たなければ `None`。
        actual: Option<u8>,
    },
    /// モジュールが求めるスロット (`requires.slots`) が本体に無い。
    #[error("module `{module}` requires slot `{slot}`, which the base model does not have")]
    MissingSlot {
        /// モジュールの名前。
        module: String,
        /// 無いスロットの ID。
        slot: String,
    },
    /// モジュールが求める、または使うパラメータが本体に無い。
    #[error("module `{module}` uses parameter `{parameter}`, which the base model does not have")]
    MissingParameter {
        /// モジュールの名前。
        module: String,
        /// 無いパラメータの ID。
        parameter: String,
    },
    /// 標準パラメータでも、スロットが公開するパラメータでもない。
    #[error("module `{module}`: parameter `{parameter}` is not exposed by slot `{slot}`")]
    ParameterNotExposed {
        /// モジュールの名前。
        module: String,
        /// モジュールの取り付け先のスロットの ID。
        slot: String,
        /// 使えないパラメータの ID。
        parameter: String,
    },
    /// マスクの参照先が、モジュールのメッシュでも、スロットの `mask_sources` でもない。
    #[error("module `{module}`: mesh `{mesh}` masks with `{mask}`, which is not available")]
    UnknownMask {
        /// モジュールの名前。
        module: String,
        /// マスクを使うメッシュの ID。
        mesh: String,
        /// 参照できないマスクのメッシュの ID。
        mask: String,
    },
    /// メッシュのテクスチャの番号が、モジュールの `textures` の範囲の外。
    #[error("module `{module}`: mesh `{mesh}` has texture {texture} out of range")]
    TextureOutOfRange {
        /// モジュールの名前。
        module: String,
        /// メッシュの ID。
        mesh: String,
        /// メッシュのテクスチャの番号 (モジュールの `textures` の中の番号)。
        texture: u32,
    },
    /// リグの衝突体が、モジュールの衝突体でも、スロットの `colliders` にある本体の衝突体でもない。
    #[error("module `{module}`: rig `{rig}` collides with `{collider}`, which is not available")]
    UnknownCollider {
        /// モジュールの名前。
        module: String,
        /// 物理のリグの ID。
        rig: String,
        /// 参照できない衝突体の ID。
        collider: String,
    },
    /// ポーズの追従先が、スロットの `pose_parts` にある本体のポーズの要素ではない。
    #[error("module `{module}`: pose entry `{part}` is not exposed by slot `{slot}`")]
    PoseNotExposed {
        /// モジュールの名前。
        module: String,
        /// モジュールの取り付け先のスロットの ID。
        slot: String,
        /// 追従先として指定した本体のパーツの ID。
        part: String,
    },
    /// 合成したモデルが検証を通らない (モジュール自体の不整合など)。
    #[error("{}", ugoe_model::validate::describe_errors("composed model", .0))]
    Invalid(Vec<ValidationError>),
}

/// 本体 `base` にモジュール `modules` を取り付けたモデルを作る (`docs/spec/modules.md#合成`)。
///
/// 本体の要素の並びと ID は変えず、モジュールの要素を後ろに加える (本体のパーツ・デフォーマ・メッシュの
/// インデックスは合成の前後で同じ)。結果は検証 (`validate`) 済み。
///
/// # Errors
///
/// モジュールを取り付けられないとき (名前・スロット・条件・参照の誤り) と、合成したモデルが検証を
/// 通らないとき ([`ComposeError::Invalid`])。
pub fn compose(base: &Model, modules: &[&Module]) -> Result<Model, ComposeError> {
    let attachments = attach(base, modules)?;
    let mut model = base.clone();
    let mut replaced = Vec::new();
    for Attachment { module, slot } in attachments {
        let rename = Renamer::new(&module.name);
        let texture_offset = model.textures.len() as u32;

        for part in &module.parts {
            let mut part = part.clone();
            part.id = rename.id(&part.id);
            // 最上位のパーツはスロットの `part` の子にする。
            part.parent = match &part.parent {
                Some(parent) => Some(rename.id(parent)),
                None => slot.part.clone(),
            };
            rename.grid(&mut part.keyforms);
            model.parts.push(part);
        }
        for deformer in &module.deformers {
            let mut deformer = deformer.clone();
            deformer.id = rename.id(&deformer.id);
            deformer.part = rename.id(&deformer.part);
            deformer.parent = rename.parent(deformer.parent.as_ref(), slot);
            deformer.mirror = deformer.mirror.as_ref().map(|m| rename.id(m));
            match &mut deformer.kind {
                DeformerKind::Warp(warp) => rename.grid(&mut warp.keyforms),
                DeformerKind::Rotation(rotation) => rename.grid(&mut rotation.keyforms),
                DeformerKind::Unknown => {}
            }
            model.deformers.push(deformer);
        }
        let own_meshes: HashSet<&str> = module.meshes.iter().map(|m| m.id.as_str()).collect();
        for mesh in &module.meshes {
            if mesh.texture as usize >= module.textures.len() {
                return Err(ComposeError::TextureOutOfRange {
                    module: module.name.clone(),
                    mesh: mesh.id.to_string(),
                    texture: mesh.texture,
                });
            }
            let mut composed = mesh.clone();
            composed.id = rename.id(&mesh.id);
            composed.part = rename.id(&mesh.part);
            composed.parent = rename.parent(mesh.parent.as_ref(), slot);
            composed.texture = mesh.texture + texture_offset;
            composed.mirror = mesh.mirror.as_ref().map(|m| rename.id(m));
            for mask in &mut composed.masks {
                if own_meshes.contains(mask.mesh.as_str()) {
                    mask.mesh = rename.id(&mask.mesh);
                } else if !slot.mask_sources.contains(&mask.mesh) {
                    return Err(ComposeError::UnknownMask {
                        module: module.name.clone(),
                        mesh: mesh.id.to_string(),
                        mask: mask.mesh.to_string(),
                    });
                }
            }
            rename.grid(&mut composed.keyforms);
            for keyform in composed.keyforms.all_keyforms_mut() {
                keyform.draw_order = map_draw_order(slot.draw_order, keyform.draw_order);
            }
            model.meshes.push(composed);
        }
        for glue in &module.glues {
            let mut glue = glue.clone();
            glue.id = rename.id(&glue.id);
            glue.mesh_a = rename.id(&glue.mesh_a);
            glue.mesh_b = rename.id(&glue.mesh_b);
            rename.grid(&mut glue.keyforms);
            model.glues.push(glue);
        }
        for texture in &module.textures {
            let mut texture = texture.clone();
            texture.name = rename.string(&texture.name);
            model.textures.push(texture);
        }
        attach_pose(&mut model, module, slot, &rename)?;
        replaced.extend(slot.replaces.iter().cloned());
    }
    hide_replaced_parts(&mut model, replaced);

    let errors = validate(&model);
    if errors.is_empty() {
        Ok(model)
    } else {
        Err(ComposeError::Invalid(errors))
    }
}

/// [`compose`] と同じ組で、本体の物理 `physics` にモジュールの物理を加える。
///
/// リグと衝突体は本体のものの後ろに加わる (本体のリグが先に評価される)。重力・風・周波数は本体のもの。
/// 本体に物理がないときは [`compose_physics_without_base`] を使う。
pub fn compose_physics(
    base: &Model,
    physics: &PhysicsSettings,
    modules: &[&Module],
) -> Result<PhysicsSettings, ComposeError> {
    compose_physics_with(base, physics, modules, true)
}

/// 物理の設定を持たない本体 `base` に付けたモジュールの物理を、`settings` (重力・風・周波数を決めた空の設定) に
/// 加える (`docs/spec/modules.md#合成`)。モデルだけで作った Puppet などで使う。
///
/// 本体に衝突体が無いので、スロットの `colliders` は何も指さない: リグの `colliders` のうちスロットが公開する本体の
/// 衝突体への参照は外し (リグはそれに当たらない)、[`compose`] で作ったモデル `composed` のスロットの `colliders` を
/// 空にする (合成した物理の検証で、スロットの衝突体を確かめない)。スロットが公開していない衝突体への参照は、
/// [`compose_physics`] と同じく [`ComposeError::UnknownCollider`]。
pub fn compose_physics_without_base(
    base: &Model,
    composed: &mut Model,
    settings: &PhysicsSettings,
    modules: &[&Module],
) -> Result<PhysicsSettings, ComposeError> {
    let physics = compose_physics_with(base, settings, modules, false)?;
    for slot in &mut composed.slots {
        slot.colliders.clear();
    }
    Ok(physics)
}

/// `base_colliders` が偽なら、本体に衝突体が無い ([`compose_physics_without_base`])。
fn compose_physics_with(
    base: &Model,
    physics: &PhysicsSettings,
    modules: &[&Module],
    base_colliders: bool,
) -> Result<PhysicsSettings, ComposeError> {
    let attachments = attach(base, modules)?;
    let mut physics = physics.clone();
    let base_ids: HashSet<ColliderId> = physics.colliders.iter().map(|c| c.id.clone()).collect();
    for Attachment { module, slot } in attachments {
        let rename = Renamer::new(&module.name);
        let own_colliders: HashSet<&str> = module
            .physics
            .colliders
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        for rig in &module.physics.rigs {
            let mut rig = rig.clone();
            let original = rig.id.clone();
            rig.id = rename.id(&rig.id);
            rig.frame = rig.frame.as_ref().map(|f| rename.id(f));
            let mut colliders = Vec::with_capacity(rig.colliders.len());
            for collider in &rig.colliders {
                if own_colliders.contains(collider.as_str()) {
                    colliders.push(rename.id(collider));
                } else if slot.colliders.contains(collider)
                    && (!base_colliders || base_ids.contains(collider))
                {
                    // 本体の衝突体はそのまま指す。本体に物理が無ければ指す先が無いので外す。
                    if base_colliders {
                        colliders.push(collider.clone());
                    }
                } else {
                    return Err(ComposeError::UnknownCollider {
                        module: module.name.clone(),
                        rig: original.to_string(),
                        collider: collider.to_string(),
                    });
                }
            }
            rig.colliders = colliders;
            for output in &mut rig.outputs {
                match &mut output.target {
                    PhysicsTarget::DeformerAngle { deformer }
                    | PhysicsTarget::DeformerOffset { deformer } => *deformer = rename.id(deformer),
                    PhysicsTarget::Vertices { owner, .. } => match owner {
                        VertexOwner::Mesh(mesh) => *mesh = rename.id(mesh),
                        VertexOwner::Warp(warp) => *warp = rename.id(warp),
                        VertexOwner::Unknown => {}
                    },
                    PhysicsTarget::Parameter { .. } | PhysicsTarget::Unknown => {}
                }
            }
            physics.rigs.push(rig);
        }
        for collider in &module.physics.colliders {
            let mut collider = collider.clone();
            collider.id = rename.id(&collider.id);
            collider.attach = rename.parent(collider.attach.as_ref(), slot);
            physics.colliders.push(collider);
        }
    }
    Ok(physics)
}

/// モジュールのポーズを加える。グループは本体のグループの後ろに加え、追従 (`links`) は
/// 本体のポーズの要素の `links` に加える。
fn attach_pose(
    model: &mut Model,
    module: &Module,
    slot: &Slot,
    rename: &Renamer<'_>,
) -> Result<(), ComposeError> {
    let rename_entry = |entry: &PoseEntry| PoseEntry {
        part: rename.id(&entry.part),
        links: entry.links.iter().map(|p| rename.id(p)).collect(),
    };
    for link in &module.pose.links {
        let entry = slot
            .pose_parts
            .contains(&link.part)
            .then(|| {
                model
                    .pose
                    .groups
                    .iter_mut()
                    .flat_map(|g| &mut g.entries)
                    .find(|e| e.part == link.part)
            })
            .flatten();
        let Some(entry) = entry else {
            return Err(ComposeError::PoseNotExposed {
                module: module.name.clone(),
                slot: slot.id.to_string(),
                part: link.part.to_string(),
            });
        };
        entry.links.extend(link.links.iter().map(|p| rename.id(p)));
    }
    model
        .pose
        .groups
        .extend(module.pose.groups.iter().map(|group| PoseGroup {
            entries: group.entries.iter().map(rename_entry).collect(),
        }));
    Ok(())
}

/// 描画順をスロットの範囲に写す: `min + (max − min) × order / 1000`。
fn map_draw_order(range: DrawOrderRange, order: f32) -> f32 {
    range.min + (range.max - range.min) * order / 1000.0
}

struct Attachment<'a> {
    module: &'a Module,
    slot: &'a Slot,
}

/// モジュールごとに取り付け先のスロットを引き、名前・スロット・条件・パラメータを検査する。
fn attach<'a>(
    base: &'a Model,
    modules: &[&'a Module],
) -> Result<Vec<Attachment<'a>>, ComposeError> {
    let slots: HashMap<&str, &Slot> = base.slots.iter().map(|s| (s.id.as_str(), s)).collect();
    let params: HashSet<&str> = base.parameters.iter().map(|p| p.id.as_str()).collect();
    let mut names = HashSet::new();
    let mut occupied: HashMap<&str, &str> = HashMap::new();
    let mut attachments = Vec::with_capacity(modules.len());
    for &module in modules {
        let name = module.name.as_str();
        if !Module::is_valid_name(name) {
            return Err(ComposeError::InvalidModuleName(name.to_owned()));
        }
        if !names.insert(name) {
            return Err(ComposeError::DuplicateModule(name.to_owned()));
        }
        let Some(&slot) = slots.get(module.slot.as_str()) else {
            return Err(ComposeError::UnknownSlot {
                module: name.to_owned(),
                slot: module.slot.to_string(),
            });
        };
        if let Some(first) = occupied.insert(slot.id.as_str(), name) {
            return Err(ComposeError::SlotOccupied {
                slot: slot.id.to_string(),
                first: first.to_owned(),
                second: name.to_owned(),
            });
        }

        let requires = &module.requires;
        if let Some(required) = requires.avatar_level {
            let actual = base.avatar.as_ref().map(|a| a.level);
            if actual.is_none_or(|level| level < required) {
                return Err(ComposeError::AvatarLevel {
                    module: name.to_owned(),
                    required,
                    actual,
                });
            }
        }
        if let Some(missing) = requires
            .slots
            .iter()
            .find(|s| !slots.contains_key(s.as_str()))
        {
            return Err(ComposeError::MissingSlot {
                module: name.to_owned(),
                slot: missing.to_string(),
            });
        }
        let used = module.referenced_parameters();
        for parameter in requires.parameters.iter().chain(&used) {
            if !params.contains(parameter.as_str()) {
                return Err(ComposeError::MissingParameter {
                    module: name.to_owned(),
                    parameter: parameter.to_string(),
                });
            }
        }
        for parameter in &used {
            if standard::range(parameter.as_str()).is_none() && !slot.parameters.contains(parameter)
            {
                return Err(ComposeError::ParameterNotExposed {
                    module: name.to_owned(),
                    slot: slot.id.to_string(),
                    parameter: parameter.to_string(),
                });
            }
        }
        attachments.push(Attachment { module, slot });
    }
    Ok(attachments)
}

/// モジュールの中の ID を `<モジュールの名前>/<ID>` に付け替える。
struct Renamer<'a> {
    prefix: &'a str,
}

impl<'a> Renamer<'a> {
    fn new(prefix: &'a str) -> Self {
        Self { prefix }
    }

    fn string(&self, id: &str) -> String {
        format!("{}/{id}", self.prefix)
    }

    fn id<T>(&self, id: &Id<T>) -> Id<T> {
        Id::new(self.string(id.as_str()))
    }

    /// 親のデフォーマ。最上位 (`None`) はスロットの取り付け先の子にする。
    fn parent(&self, parent: Option<&DeformerId>, slot: &Slot) -> Option<DeformerId> {
        match parent {
            Some(parent) => Some(self.id(parent)),
            None => slot.anchor.clone(),
        }
    }

    /// 格子の中の要素の ID (ミラー元。ブレンドシェイプ格子のものも)。パラメータの ID は付け替えない。
    fn grid<T>(&self, grid: &mut KeyformGrid<T>) {
        if let Some(source) = &mut grid.mirror_source {
            source.id = self.string(&source.id);
        }
        for shape in &mut grid.blend_shapes {
            self.grid(shape);
        }
    }
}

/// 置き換えられるパーツを、不透明度 0 のパーツ (`#replaced/<ID>`) の子にして隠す。
///
/// パーツ自身の不透明度を 0 にするのではなく親を挟むので、ポーズやモーションがパーツの
/// 不透明度を上書きしても隠れたままになる。挟むパーツは末尾に加える。
fn hide_replaced_parts(model: &mut Model, replaced: Vec<PartId>) {
    let mut seen = HashSet::new();
    for id in replaced {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(part) = model.parts.iter_mut().find(|p| p.id == id) else {
            continue;
        };
        let hidden: PartId = Id::new(format!("{REPLACED_PART_PREFIX}{id}"));
        let parent = part.parent.replace(hidden.clone());
        let name = format!("{REPLACED_PART_PREFIX}{}", part.name);
        model.parts.push(Part {
            id: hidden,
            name,
            parent,
            keyforms: KeyformGrid::constant(PartKeyform { opacity: 0.0 }),
        });
    }
}

#[cfg(test)]
mod tests {
    use ugoe_core::Vec2;
    use ugoe_model::fixtures::{dress_module, dress_up_base, long_hair_module};
    use ugoe_model::validate::validate_physics;
    use ugoe_model::{AvatarProfile, MaskRef};

    use super::*;
    use crate::{CompiledModel, ModelInstance};

    fn composed() -> Model {
        compose(&dress_up_base(), &[&long_hair_module(), &dress_module()]).unwrap()
    }

    fn mesh<'a>(model: &'a Model, id: &str) -> &'a ugoe_model::Mesh {
        model.meshes.iter().find(|m| m.id == id).unwrap()
    }

    fn part<'a>(model: &'a Model, id: &str) -> &'a Part {
        model.parts.iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn no_modules_leaves_the_base_unchanged() {
        assert_eq!(compose(&dress_up_base(), &[]).unwrap(), dress_up_base());
    }

    #[test]
    fn ids_are_renamed_and_attached_to_the_anchor() {
        let base = dress_up_base();
        let model = composed();
        // 本体の要素は同じ並びのまま先頭にある。
        for (a, b) in base.meshes.iter().zip(&model.meshes) {
            assert_eq!(a.id, b.id);
        }
        let ids: Vec<&str> = model.meshes.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "Face",
                "HairDefault",
                "BodyMesh",
                "LongHair/Back",
                "LongHair/Front",
                "Dress/Skirt"
            ]
        );
        // 本体と同じ ID のパーツ `Hair` も、付け替えで別のパーツになる。
        assert_eq!(part(&model, "LongHair/Hair").parent, None);
        let sway = model
            .deformers
            .iter()
            .find(|d| d.id == "LongHair/Sway")
            .unwrap();
        assert_eq!(sway.parent.as_ref().unwrap().as_str(), "Head");
        assert_eq!(sway.part.as_str(), "LongHair/Hair");
        let back = mesh(&model, "LongHair/Back");
        assert_eq!(back.parent.as_ref().unwrap().as_str(), "LongHair/Sway");
        assert_eq!(back.masks, vec![MaskRef::normal("Face")]);
        assert_eq!(
            mesh(&model, "LongHair/Front").masks,
            vec![MaskRef::normal("LongHair/Back")]
        );
        let skirt = mesh(&model, "Dress/Skirt");
        assert_eq!(skirt.parent.as_ref().unwrap().as_str(), "Torso");
        assert_eq!(skirt.part.as_str(), "Dress/Dress");
        // テクスチャは本体の後ろに加わり、メッシュの番号もずれる。
        let names: Vec<&str> = model.textures.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["white", "LongHair/long_hair", "Dress/dress"]);
        assert_eq!((back.texture, skirt.texture), (1, 2));
    }

    // modules.md 合成 2.: ミラー元の ID は、ブレンドシェイプ格子のものも付け替える。
    #[test]
    fn mirror_sources_are_renamed_in_blend_shape_grids() {
        let mut base = dress_up_base();
        base.parameters[3].kind = ugoe_model::parameter::ParameterKind::BlendShape;
        base.slots[1].parameters.push("Secret".into());
        let mut module = dress_module();
        let source = || {
            Some(ugoe_model::MirrorSource {
                id: "Skirt".into(),
                axis: None,
                tolerance: None,
            })
        };
        let skirt = &mut module.meshes[0].keyforms;
        let mut shape = KeyformGrid::new(
            vec![ugoe_model::KeyformBinding {
                parameter: "Secret".into(),
                keys: vec![0.0, 1.0],
            }],
            skirt.keyforms.clone(),
        );
        shape.mirror_source = source();
        skirt.mirror_source = source();
        skirt.blend_shapes.push(shape);

        let model = compose(&base, &[&module]).unwrap();
        let grid = &mesh(&model, "Dress/Skirt").keyforms;
        assert_eq!(grid.mirror_source.as_ref().unwrap().id, "Dress/Skirt");
        assert_eq!(
            grid.blend_shapes[0].mirror_source.as_ref().unwrap().id,
            "Dress/Skirt"
        );
    }

    #[test]
    fn draw_order_is_mapped_into_the_slot_range() {
        let model = composed();
        let order = |id| mesh(&model, id).keyforms.keyforms[0].draw_order;
        assert_eq!(order("LongHair/Back"), 550.0);
        assert_eq!(order("LongHair/Front"), 650.0);
        assert_eq!(order("Dress/Skirt"), 350.0);
        assert_eq!(order("HairDefault"), 600.0);
    }

    #[test]
    fn replaced_parts_stay_hidden_under_overrides() {
        let model = composed();
        let hair = part(&model, "Hair");
        assert_eq!(hair.parent.as_ref().unwrap().as_str(), "#replaced/Hair");
        let hidden = part(&model, "#replaced/Hair");
        assert_eq!(hidden.parent.as_ref().unwrap().as_str(), "Root");
        assert_eq!(model.parts.last().unwrap().id, hidden.id);

        let compiled = CompiledModel::compile(model).unwrap();
        let mut instance = ModelInstance::new(compiled.clone());
        // ポーズやモーションがパーツ自身の不透明度を上書きしても隠れたまま。
        let hair = compiled.part_index("Hair").unwrap();
        instance.set_part_opacity_override(hair, 1.0, 1.0);
        instance.update();
        let drawable = |id| &instance.drawables()[compiled.mesh_index(id).unwrap()];
        assert_eq!(drawable("HairDefault").opacity, 0.0);
        assert!(!drawable("HairDefault").visible);
        assert_eq!(drawable("LongHair/Front").opacity, 1.0);
        assert_eq!(drawable("Face").opacity, 1.0);
        // モジュールの最上位の座標は、取り付け先の局所座標系 (`Torso` の UV) で表される。
        let skirt = &drawable("Dress/Skirt").positions;
        assert!((skirt[0] - Vec2::new(64.0, 192.0)).length() < 1e-4);
        assert!((skirt[2] - Vec2::new(192.0, 256.0)).length() < 1e-4);
    }

    #[test]
    fn parameter_values_carry_over_when_recomposing() {
        let base = dress_up_base();
        let mut before = ModelInstance::new(CompiledModel::compile(base.clone()).unwrap());
        let set = |instance: &mut ModelInstance, id: &str, value: f32| {
            let index = instance.parameters().index_of(id).unwrap();
            instance.parameters_mut().set(index, value);
        };
        set(&mut before, "AngleX", 12.0);
        set(&mut before, "BodySway", 0.7);

        let model = compose(&base, &[&dress_module()]).unwrap();
        let mut after = ModelInstance::new(CompiledModel::compile(model).unwrap());
        after.parameters_mut().carry_over(before.parameters());
        let get = |instance: &ModelInstance, id: &str| {
            let params = instance.parameters();
            params.get(params.index_of(id).unwrap())
        };
        assert_eq!(get(&after, "AngleX"), 12.0);
        assert_eq!(get(&after, "BodySway"), 0.7);
        assert_eq!(get(&after, "HairFront"), 0.0);
        after.update();
        let skirt = after.compiled().mesh_index("Dress/Skirt").unwrap();
        // `BodySway = 0.7` で UV が 0.175 ずれる (Torso の幅 128 で 22.4px)。
        let x = after.drawables()[skirt].positions[0].x;
        assert!((x - (64.0 + 22.4)).abs() < 1e-3, "{x}");
    }

    #[test]
    fn carry_over_clamps_to_the_new_range() {
        let base = dress_up_base();
        let mut before = crate::ParameterValues::from_model(&base);
        before.set(before.index_of("BodySway").unwrap(), 1.0);
        let mut narrowed = base.clone();
        narrowed.parameters[2].max = 0.5;
        let mut after = crate::ParameterValues::from_model(&narrowed);
        after.carry_over(&before);
        assert_eq!(after.get(after.index_of("BodySway").unwrap()), 0.5);
    }

    #[test]
    fn physics_is_renamed_and_attached() {
        let base = dress_up_base();
        let empty = PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: Vec::new(),
            colliders: Vec::new(),
        };
        let modules = [&long_hair_module(), &dress_module()];
        let physics = compose_physics(&base, &empty, &modules).unwrap();
        let rig = &physics.rigs[0];
        assert_eq!(rig.id.as_str(), "LongHair/Sway");
        assert_eq!(rig.colliders, [ColliderId::new("LongHair/Collar")]);
        assert_eq!(
            rig.outputs[0].target,
            PhysicsTarget::DeformerAngle {
                deformer: "LongHair/Sway".into()
            }
        );
        assert_eq!(physics.colliders[0].id.as_str(), "LongHair/Collar");
        assert_eq!(
            physics.colliders[0].attach.as_ref().unwrap().as_str(),
            "Head"
        );
        let model = compose(&base, &modules).unwrap();
        assert_eq!(validate_physics(&physics, &model), vec![]);
    }

    /// 未知の対象 (`VertexOwner::Unknown`) への出力は付け替えずにそのまま残す (物理が無視する)。
    #[test]
    fn unknown_vertex_owners_are_kept_as_unknown() {
        let base = dress_up_base();
        let empty = PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: Vec::new(),
            colliders: Vec::new(),
        };
        let mut module = long_hair_module();
        let unknown = PhysicsTarget::Vertices {
            owner: VertexOwner::Unknown,
            weights: vec![1.0],
        };
        module.physics.rigs[0].outputs[0].target = unknown.clone();
        let physics = compose_physics(&base, &empty, &[&module]).unwrap();
        assert_eq!(physics.rigs[0].outputs[0].target, unknown);
        assert_eq!(
            ugoe_model::unknown::in_physics(&physics),
            ["rigs[LongHair/Sway].outputs[0].target.owner"]
        );
        let model = compose(&base, &[&module]).unwrap();
        assert_eq!(validate_physics(&physics, &model), vec![]);
    }

    #[test]
    fn top_level_parts_hang_under_the_slot_part() {
        let model = composed();
        // `Outfit` は `part: Body` なので、`Dress` の最上位のパーツは `Body` の子になる。
        assert_eq!(
            part(&model, "Dress/Dress")
                .parent
                .as_ref()
                .unwrap()
                .as_str(),
            "Body"
        );
        assert_eq!(part(&model, "LongHair/Hair").parent, None);

        // 本体のパーツを隠すと、モジュールのメッシュも隠れる。
        let compiled = CompiledModel::compile(model).unwrap();
        let mut instance = ModelInstance::new(compiled.clone());
        instance.set_part_opacity_override(compiled.part_index("Body").unwrap(), 0.0, 1.0);
        instance.update();
        let skirt = compiled.mesh_index("Dress/Skirt").unwrap();
        assert_eq!(instance.drawables()[skirt].opacity, 0.0);
    }

    /// 本体の腕の差分 `ArmA` / `ArmB` と、それに追従する袖・モジュールだけの帽子の差分。
    fn pose_base_and_module() -> (Model, Module) {
        let mut base = dress_up_base();
        let opaque = || KeyformGrid::constant(PartKeyform { opacity: 1.0 });
        for id in ["ArmA", "ArmB"] {
            base.parts.push(Part {
                id: id.into(),
                name: id.into(),
                parent: Some("Body".into()),
                keyforms: opaque(),
            });
        }
        base.pose.groups.push(PoseGroup {
            entries: vec![
                PoseEntry {
                    part: "ArmA".into(),
                    links: vec![],
                },
                PoseEntry {
                    part: "ArmB".into(),
                    links: vec![],
                },
            ],
        });
        base.slots[1].pose_parts = vec!["ArmA".into(), "ArmB".into()];
        let mut module = dress_module();
        for id in ["SleeveA", "SleeveB", "HatA", "HatB"] {
            module.parts.push(Part {
                id: id.into(),
                name: id.into(),
                parent: Some("Dress".into()),
                keyforms: opaque(),
            });
        }
        let entry = |part: &str, links: &[&str]| PoseEntry {
            part: part.into(),
            links: links.iter().map(|&l| l.into()).collect(),
        };
        module.pose.links = vec![entry("ArmA", &["SleeveA"]), entry("ArmB", &["SleeveB"])];
        module.pose.groups = vec![PoseGroup {
            entries: vec![entry("HatA", &[]), entry("HatB", &[])],
        }];
        (base, module)
    }

    #[test]
    fn module_parts_join_pose_groups() {
        let (base, module) = pose_base_and_module();
        let model = compose(&base, &[&module]).unwrap();
        let groups = &model.pose.groups;
        assert_eq!(groups.len(), 2);
        let links: Vec<Vec<&str>> = groups[0]
            .entries
            .iter()
            .map(|e| e.links.iter().map(|l| l.as_str()).collect())
            .collect();
        assert_eq!(links, [["Dress/SleeveA"], ["Dress/SleeveB"]]);
        let hats: Vec<&str> = groups[1].entries.iter().map(|e| e.part.as_str()).collect();
        assert_eq!(hats, ["Dress/HatA", "Dress/HatB"]);

        // スロットが公開していない要素には追従できない。
        let mut hidden = base.clone();
        hidden.slots[1].pose_parts.pop();
        assert_eq!(
            compose(&hidden, &[&module]).unwrap_err(),
            ComposeError::PoseNotExposed {
                module: "Dress".into(),
                slot: "Outfit".into(),
                part: "ArmB".into(),
            }
        );
        // モジュールにないパーツは、合成したモデルの検証で見つかる。
        let mut missing = module.clone();
        missing.pose.groups[0].entries[1].part = "NoHat".into();
        assert!(matches!(
            compose(&base, &[&missing]).unwrap_err(),
            ComposeError::Invalid(_)
        ));
    }

    fn physics_with_chest() -> PhysicsSettings {
        PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: Vec::new(),
            colliders: vec![ugoe_model::physics::PhysicsCollider {
                id: "Collar".into(),
                name: "Base collar".into(),
                attach: Some("Torso".into()),
                shape: ugoe_model::physics::ColliderShape::Circle {
                    center: Vec2::new(0.5, 0.0),
                    radius: 8.0,
                },
            }],
        }
    }

    #[test]
    fn module_rigs_collide_with_colliders_exposed_by_the_slot() {
        let mut base = dress_up_base();
        base.slots[0].colliders = vec!["Collar".into()];
        let physics = physics_with_chest();
        let mut hair = long_hair_module();
        // 本体の `Collar` と同じ ID の衝突体がモジュールにあれば、モジュールのものを指す。
        let composed = compose_physics(&base, &physics, &[&hair]).unwrap();
        assert_eq!(
            composed.rigs[0].colliders,
            [ColliderId::new("LongHair/Collar")]
        );

        // モジュールに無ければ、スロットが公開する本体の衝突体を指す (付け替えない)。
        hair.physics.colliders.clear();
        let composed = compose_physics(&base, &physics, &[&hair]).unwrap();
        assert_eq!(composed.rigs[0].colliders, [ColliderId::new("Collar")]);
        let model = compose(&base, &[&hair]).unwrap();
        assert_eq!(validate_physics(&composed, &model), vec![]);

        // 公開していなければエラー。
        base.slots[0].colliders.clear();
        assert_eq!(
            compose_physics(&base, &physics, &[&hair]).unwrap_err(),
            ComposeError::UnknownCollider {
                module: "LongHair".into(),
                rig: "Sway".into(),
                collider: "Collar".into(),
            }
        );
    }

    /// 本体に物理が無ければ、スロットが公開する衝突体は何も指さない: リグの参照は外れ、合成したモデルのスロットの
    /// `colliders` は空になり、合成した物理は検証を通る (T0655)。
    #[test]
    fn exposed_colliders_are_ignored_without_base_physics() {
        let mut base = dress_up_base();
        base.slots[0].colliders = vec!["Collar".into()];
        let empty = PhysicsSettings {
            colliders: Vec::new(),
            ..physics_with_chest()
        };
        let mut hair = long_hair_module();
        hair.physics.colliders.clear();
        let mut model = compose(&base, &[&hair]).unwrap();
        // 本体の物理があるつもりで合成すると、本体に無い衝突体を指すのでエラー。
        assert!(matches!(
            compose_physics(&base, &empty, &[&hair]).unwrap_err(),
            ComposeError::UnknownCollider { .. }
        ));
        assert!(!validate_physics(&empty, &model).is_empty());

        let physics = compose_physics_without_base(&base, &mut model, &empty, &[&hair]).unwrap();
        assert_eq!(physics.rigs[0].id.as_str(), "LongHair/Sway");
        assert!(physics.rigs[0].colliders.is_empty());
        assert!(model.slots.iter().all(|s| s.colliders.is_empty()));
        assert_eq!(validate_physics(&physics, &model), vec![]);

        // モジュールの衝突体は今までどおり付け替えて指す。
        let hair = long_hair_module();
        let mut model = compose(&base, &[&hair]).unwrap();
        let physics = compose_physics_without_base(&base, &mut model, &empty, &[&hair]).unwrap();
        assert_eq!(
            physics.rigs[0].colliders,
            [ColliderId::new("LongHair/Collar")]
        );

        // スロットが公開していない衝突体はエラーのまま。
        base.slots[0].colliders.clear();
        let mut hair = long_hair_module();
        hair.physics.colliders.clear();
        let mut model = compose(&base, &[&hair]).unwrap();
        assert!(matches!(
            compose_physics_without_base(&base, &mut model, &empty, &[&hair]).unwrap_err(),
            ComposeError::UnknownCollider { .. }
        ));
    }

    #[test]
    fn slot_and_module_errors() {
        let base = dress_up_base();
        let hair = long_hair_module();
        let err = |modules: &[&Module]| compose(&base, modules).unwrap_err();

        let mut other = long_hair_module();
        other.name = "OtherHair".into();
        assert_eq!(
            err(&[&hair, &other]),
            ComposeError::SlotOccupied {
                slot: "Hair".into(),
                first: "LongHair".into(),
                second: "OtherHair".into(),
            }
        );
        let mut same_name = dress_module();
        same_name.name = "LongHair".into();
        assert_eq!(
            err(&[&hair, &same_name]),
            ComposeError::DuplicateModule("LongHair".into())
        );
        let mut bad_name = dress_module();
        bad_name.name = "#x".into();
        assert_eq!(
            err(&[&bad_name]),
            ComposeError::InvalidModuleName("#x".into())
        );
        let mut hat = dress_module();
        hat.slot = "Hat".into();
        assert!(matches!(err(&[&hat]), ComposeError::UnknownSlot { .. }));
    }

    #[test]
    fn requirement_errors() {
        let base = dress_up_base();
        let err = |module: &Module| compose(&base, &[module]).unwrap_err();

        let mut module = dress_module();
        module.requires.avatar_level = Some(2);
        assert_eq!(
            err(&module),
            ComposeError::AvatarLevel {
                module: "Dress".into(),
                required: 2,
                actual: None,
            }
        );
        let mut leveled = base.clone();
        leveled.avatar = Some(AvatarProfile {
            level: 2,
            ..AvatarProfile::default()
        });
        assert!(compose(&leveled, &[&module]).is_ok());

        let mut module = dress_module();
        module.requires.slots.push("Shoes".into());
        assert_eq!(
            err(&module),
            ComposeError::MissingSlot {
                module: "Dress".into(),
                slot: "Shoes".into(),
            }
        );
        let mut module = dress_module();
        module.requires.parameters.push("EyeLOpen".into());
        assert_eq!(
            err(&module),
            ComposeError::MissingParameter {
                module: "Dress".into(),
                parameter: "EyeLOpen".into(),
            }
        );
    }

    #[test]
    fn parameters_mask_and_texture_errors() {
        let base = dress_up_base();
        let err = |module: &Module| compose(&base, &[module]).unwrap_err();

        // `Secret` は本体にあるが、`Outfit` は公開していない。
        let mut module = dress_module();
        module.meshes[0].keyforms.bindings[0].parameter = "Secret".into();
        assert_eq!(
            err(&module),
            ComposeError::ParameterNotExposed {
                module: "Dress".into(),
                slot: "Outfit".into(),
                parameter: "Secret".into(),
            }
        );
        // `BodySway` は `Hair` では公開されていない。
        let mut module = dress_module();
        module.slot = "Hair".into();
        assert!(matches!(
            err(&module),
            ComposeError::ParameterNotExposed { .. }
        ));
        // 本体のメッシュは `mask_sources` にあるものだけマスクにできる。
        let mut module = dress_module();
        module.meshes[0].masks.push(MaskRef::normal("Face"));
        assert_eq!(
            err(&module),
            ComposeError::UnknownMask {
                module: "Dress".into(),
                mesh: "Skirt".into(),
                mask: "Face".into(),
            }
        );
        let mut module = dress_module();
        module.meshes[0].texture = 1;
        assert!(matches!(
            err(&module),
            ComposeError::TextureOutOfRange { texture: 1, .. }
        ));
        // モジュール自体の不整合は、合成したモデルの検証で見つかる。
        let mut module = dress_module();
        module.meshes[0].part = "Missing".into();
        let ComposeError::Invalid(errors) = err(&module) else {
            panic!("not invalid");
        };
        assert_eq!(
            errors,
            vec![ValidationError::UnknownReference {
                kind: "mesh",
                id: "Dress/Skirt".into(),
                field: "part",
                target_kind: "part",
                target: "Dress/Missing".into(),
            }]
        );
    }
}
