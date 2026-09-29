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
use ugoe_model::physics::{PhysicsSettings, PhysicsTarget, VertexOwner};
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{DeformerId, Model, Module, Part, PartId, Slot, standard};

/// 置き換えられるパーツを隠すために挟むパーツの ID の接頭辞 (`#replaced/<パーツの ID>`)。
/// モジュールの名前は `#` で始まれないので、モジュールの要素の ID とは衝突しない。
pub const REPLACED_PART_PREFIX: &str = "#replaced/";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ComposeError {
    #[error("module name `{0}` must be non-empty, contain no `/` and not start with `#`")]
    InvalidModuleName(String),
    #[error("module name `{0}` is used more than once")]
    DuplicateModule(String),
    #[error("module `{module}`: slot `{slot}` does not exist in the base model")]
    UnknownSlot { module: String, slot: String },
    #[error("slot `{slot}` already has module `{first}`, cannot also attach `{second}`")]
    SlotOccupied {
        slot: String,
        first: String,
        second: String,
    },
    #[error("module `{module}` requires avatar profile level {required}, base has {actual:?}")]
    AvatarLevel {
        module: String,
        required: u8,
        actual: Option<u8>,
    },
    #[error("module `{module}` requires slot `{slot}`, which the base model does not have")]
    MissingSlot { module: String, slot: String },
    #[error("module `{module}` uses parameter `{parameter}`, which the base model does not have")]
    MissingParameter { module: String, parameter: String },
    /// 標準パラメータでも、スロットが公開するパラメータでもない。
    #[error("module `{module}`: parameter `{parameter}` is not exposed by slot `{slot}`")]
    ParameterNotExposed {
        module: String,
        slot: String,
        parameter: String,
    },
    /// マスクの参照先が、モジュールのメッシュでも、スロットの `mask_sources` でもない。
    #[error("module `{module}`: mesh `{mesh}` masks with `{mask}`, which is not available")]
    UnknownMask {
        module: String,
        mesh: String,
        mask: String,
    },
    #[error("module `{module}`: mesh `{mesh}` has texture {texture} out of range")]
    TextureOutOfRange {
        module: String,
        mesh: String,
        texture: u32,
    },
    /// 合成したモデルが検証を通らない (モジュール自体の不整合など)。
    #[error("composed model failed validation: {0:?}")]
    Invalid(Vec<ValidationError>),
}

/// 本体 `base` にモジュール `modules` を取り付けたモデルを作る (`docs/spec/modules.md#合成`)。
///
/// 本体の要素の並びと ID は変えず、モジュールの要素を後ろに加える (本体のパーツ・デフォーマ・メッシュの
/// インデックスは合成の前後で同じ)。結果は検証 (`validate`) 済み。
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
            part.parent = part.parent.as_ref().map(|p| rename.id(p));
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
            glue.id = rename.string(&glue.id);
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
/// 本体に物理がないときは、重力などを決めた空の設定を渡す。
pub fn compose_physics(
    base: &Model,
    physics: &PhysicsSettings,
    modules: &[&Module],
) -> Result<PhysicsSettings, ComposeError> {
    let attachments = attach(base, modules)?;
    let mut physics = physics.clone();
    for Attachment { module, slot } in attachments {
        let rename = Renamer::new(&module.name);
        for rig in &module.physics.rigs {
            let mut rig = rig.clone();
            rig.id = rename.string(&rig.id);
            rig.frame = rig.frame.as_ref().map(|f| rename.id(f));
            for collider in &mut rig.colliders {
                *collider = rename.string(collider);
            }
            for output in &mut rig.outputs {
                match &mut output.target {
                    PhysicsTarget::DeformerAngle { deformer }
                    | PhysicsTarget::DeformerOffset { deformer } => *deformer = rename.id(deformer),
                    PhysicsTarget::Vertices { owner, .. } => match owner {
                        VertexOwner::Mesh(mesh) => *mesh = rename.id(mesh),
                        VertexOwner::Warp(warp) => *warp = rename.id(warp),
                    },
                    PhysicsTarget::Parameter { .. } | PhysicsTarget::Unknown => {}
                }
            }
            physics.rigs.push(rig);
        }
        for collider in &module.physics.colliders {
            let mut collider = collider.clone();
            collider.id = rename.string(&collider.id);
            collider.attach = rename.parent(collider.attach.as_ref(), slot);
            physics.colliders.push(collider);
        }
    }
    Ok(physics)
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

    /// 格子の中の要素の ID (ミラー元)。パラメータの ID は付け替えない。
    fn grid<T>(&self, grid: &mut KeyformGrid<T>) {
        if let Some(source) = &mut grid.mirror_source {
            source.id = self.string(&source.id);
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
        model.meshes.iter().find(|m| m.id.as_str() == id).unwrap()
    }

    fn part<'a>(model: &'a Model, id: &str) -> &'a Part {
        model.parts.iter().find(|p| p.id.as_str() == id).unwrap()
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
            .find(|d| d.id.as_str() == "LongHair/Sway")
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
        assert_eq!(rig.id, "LongHair/Sway");
        assert_eq!(rig.colliders, ["LongHair/Collar"]);
        assert_eq!(
            rig.outputs[0].target,
            PhysicsTarget::DeformerAngle {
                deformer: "LongHair/Sway".into()
            }
        );
        assert_eq!(physics.colliders[0].id, "LongHair/Collar");
        assert_eq!(
            physics.colliders[0].attach.as_ref().unwrap().as_str(),
            "Head"
        );
        let model = compose(&base, &modules).unwrap();
        assert_eq!(validate_physics(&physics, &model), vec![]);
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
