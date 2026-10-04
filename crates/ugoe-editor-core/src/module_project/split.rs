//! 作業用のモデルをモジュールに組み立てる [`Splitter`] (`ModuleContext::split`)。

use std::collections::{BTreeMap, HashSet};

use ugoe_core::Id;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::keyform::KeyformGrid;
use ugoe_model::physics::{PhysicsSettings, PhysicsTarget, VertexOwner};
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{DeformerId, MeshId, Model, Module, PartId};

use super::{ModuleContext, ModuleError, shape, unmap_draw_order};
use crate::texture::Textures;

/// 作業用のモデルからモジュールを組み立てる。
pub(super) struct Splitter<'a> {
    context: &'a ModuleContext,
    /// 作業用の ID → モジュールの中の ID (種類ごと)。
    parts: BTreeMap<String, String>,
    deformers: BTreeMap<String, String>,
    meshes: BTreeMap<String, String>,
}

impl<'a> Splitter<'a> {
    pub(super) fn new(context: &'a ModuleContext, model: &Model) -> Result<Self, ModuleError> {
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

    pub(super) fn module(
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
            glue.id = glue_ids[glue.id.as_str()].as_str().into();
            glue.mesh_a = self.mesh(&glue.mesh_a, &what)?;
            glue.mesh_b = self.mesh(&glue.mesh_b, &what)?;
            self.grid(&mut glue.keyforms, &BTreeMap::new(), &what)?;
            module.glues.push(glue);
        }

        let own_textures = &model.textures[offset as usize..];
        let texture_names =
            context.renames("texture", own_textures.iter().map(|t| t.name.as_str()))?;
        // 法線マップの画像もテクスチャと同じ表 (`textures/`) に置くので、接頭辞を外した名前が重なれば
        // 片方の画像が失われる。共有する法線マップは 1 度だけ数える。
        let mut image_names: Vec<&str> = own_textures.iter().map(|t| t.name.as_str()).collect();
        for normal in own_textures.iter().filter_map(|t| t.normal_map.as_deref()) {
            if !image_names.contains(&normal) {
                image_names.push(normal);
            }
        }
        context.renames("image", image_names)?;
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
            rig.id = rig_ids[rig.id.as_str()].as_str().into();
            rig.frame = match &rig.frame {
                Some(f) => Some(self.deformer(f, &what)?),
                None => None,
            };
            // 本体の衝突体への参照はそのまま残す (スロットの `colliders` かは合成が検査する)。
            for collider in &mut rig.colliders {
                if let Some(own) = collider_ids.get(collider.as_str()) {
                    *collider = own.as_str().into();
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
            collider.id = collider_ids[collider.id.as_str()].as_str().into();
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
