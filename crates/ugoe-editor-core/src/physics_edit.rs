//! 物理の設定 (`PhysicsSettings`) の編集。規則は `docs/editor.md#物理の編集`、
//! データの意味は `docs/spec/physics.md`。
//!
//! 物理はモデルの外のデータだが、ドキュメントはモデルと一緒に持ち、同じ Undo の単位で変える。
//! 参照先の検査は `Document::apply` が適用の後に `validate_physics` で行う (新しいエラーが生じたら拒否)。
//! ここでは値の範囲 (有限・非負など、`validate_physics` が見ないもの) を検査する。

use std::collections::HashSet;

use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::physics::{
    ColliderShape, NormalizationRange, PhysicsBlend, PhysicsCollider, PhysicsInput,
    PhysicsNormalization, PhysicsOutput, PhysicsParticle, PhysicsRig, PhysicsRigId,
    PhysicsSettings, PhysicsSource, PhysicsTarget, VertexOwner,
};
use ugoe_model::{DeformerId, MeshId, Model};

use crate::command::{ChildRef, Command};
use crate::deformer_edit::{Rest, subtree_points};
use crate::document::EditError;
use crate::ids;

/// 物理の設定がないときに作る設定の重力・風・周波数 (`AddPhysicsRig` など)。
pub fn default_settings() -> PhysicsSettings {
    PhysicsSettings {
        gravity: Vec2::new(0.0, 1.0),
        wind: Vec2::ZERO,
        fps: 60.0,
        rigs: Vec::new(),
        colliders: Vec::new(),
    }
}

/// `CreatePhysicsRig` で作る粒子の係数 (`docs/spec/physics.md#係数の意味`)。
pub const DEFAULT_MOBILITY: f32 = 0.9;
pub const DEFAULT_DELAY: f32 = 0.9;
pub const DEFAULT_ACCELERATION: f32 = 1.5;

/// 鎖の先端が見つからない (子孫に点がない) ときの、最後の区間の長さのキャンバスの高さに対する割合。
const FALLBACK_TIP: f32 = 0.1;

fn invalid(reason: impl Into<String>) -> EditError {
    EditError::InvalidPhysics {
        reason: reason.into(),
    }
}

fn rig_not_found(id: &str) -> EditError {
    EditError::NotFound {
        kind: "physics rig",
        id: id.to_owned(),
    }
}

fn rig_mut<'a>(
    physics: &'a mut Option<PhysicsSettings>,
    id: &str,
) -> Result<&'a mut PhysicsRig, EditError> {
    physics
        .as_mut()
        .and_then(|p| p.rigs.iter_mut().find(|r| r.id.as_str() == id))
        .ok_or_else(|| rig_not_found(id))
}

/// 物理のコマンドを適用する。物理のコマンドでなければ何もしない。
pub(crate) fn apply(
    model: &Model,
    physics: &mut Option<PhysicsSettings>,
    command: Command,
) -> Result<(), EditError> {
    match command {
        Command::SetPhysicsWorld { gravity, wind, fps } => {
            check_vec("gravity", gravity)?;
            check_vec("wind", wind)?;
            if !(fps.is_finite() && fps > 0.0) {
                return Err(invalid(format!(
                    "fps must be positive and finite, got {fps}"
                )));
            }
            let settings = physics.get_or_insert_with(default_settings);
            (settings.gravity, settings.wind, settings.fps) = (gravity, wind, fps);
        }
        Command::AddPhysicsRig { rig } => add_rig(physics, rig)?,
        Command::CreatePhysicsRig {
            id,
            name,
            deformers,
        } => {
            let rig = chain_rig(model, physics.as_ref(), id, name, &deformers)?;
            add_rig(physics, rig)?;
        }
        Command::RemovePhysicsRig { id } => {
            let settings = physics.as_mut().ok_or_else(|| rig_not_found(id.as_str()))?;
            let index = settings
                .rigs
                .iter()
                .position(|r| r.id == id)
                .ok_or_else(|| rig_not_found(id.as_str()))?;
            settings.rigs.remove(index);
        }
        Command::RenamePhysicsRig { id, name } => rig_mut(physics, id.as_str())?.name = name,
        Command::SetPhysicsInputs { rig, inputs } => {
            inputs.iter().try_for_each(check_input)?;
            rig_mut(physics, rig.as_str())?.inputs = inputs;
        }
        Command::SetPhysicsOutputs { rig, outputs } => {
            outputs.iter().try_for_each(check_output)?;
            rig_mut(physics, rig.as_str())?.outputs = outputs;
        }
        Command::SetPhysicsParticles { rig, particles } => {
            check_particles(&particles)?;
            rig_mut(physics, rig.as_str())?.particles = particles;
        }
        Command::SetPhysicsNormalization { rig, normalization } => {
            check_normalization(&normalization)?;
            rig_mut(physics, rig.as_str())?.normalization = normalization;
        }
        Command::SetPhysicsRigColliders { rig, colliders } => {
            let mut seen = HashSet::new();
            if let Some(dup) = colliders.iter().find(|c| !seen.insert(c.as_str())) {
                return Err(invalid(format!("collider `{dup}` is listed twice")));
            }
            rig_mut(physics, rig.as_str())?.colliders = colliders;
        }
        Command::SetPhysicsFrame { rig, frame } => rig_mut(physics, rig.as_str())?.frame = frame,
        Command::AddPhysicsCollider { collider } => {
            let settings = physics.get_or_insert_with(default_settings);
            if settings.colliders.iter().any(|c| c.id == collider.id) {
                return Err(EditError::DuplicateId {
                    kind: "physics collider",
                    id: collider.id.to_string(),
                });
            }
            check_collider_id(collider.id.as_str())?;
            settings.colliders.push(collider);
        }
        Command::RemovePhysicsCollider { id } => {
            let not_found = || EditError::NotFound {
                kind: "physics collider",
                id: id.to_string(),
            };
            let settings = physics.as_mut().ok_or_else(not_found)?;
            let index = settings
                .colliders
                .iter()
                .position(|c| c.id == id)
                .ok_or_else(not_found)?;
            settings.colliders.remove(index);
            for rig in &mut settings.rigs {
                rig.colliders.retain(|c| c != &id);
            }
        }
        Command::SetPhysicsCollider { collider } => {
            let existing = physics
                .as_mut()
                .and_then(|p| p.colliders.iter_mut().find(|c| c.id == collider.id))
                .ok_or_else(|| EditError::NotFound {
                    kind: "physics collider",
                    id: collider.id.to_string(),
                })?;
            *existing = collider;
        }
        Command::ReorderPhysicsRig { id, before } => {
            reorder_rig(physics, id.as_str(), before.as_ref().map(|b| b.as_str()))?
        }
        // 物理のコマンドでない。
        _ => {}
    }
    Ok(())
}

fn add_rig(physics: &mut Option<PhysicsSettings>, rig: PhysicsRig) -> Result<(), EditError> {
    let settings = physics.get_or_insert_with(default_settings);
    if settings.rigs.iter().any(|r| r.id == rig.id) {
        return Err(EditError::DuplicateId {
            kind: "physics rig",
            id: rig.id.to_string(),
        });
    }
    if rig.id.as_str().is_empty() {
        return Err(invalid("physics rig id must not be empty"));
    }
    rig.inputs.iter().try_for_each(check_input)?;
    rig.outputs.iter().try_for_each(check_output)?;
    check_particles(&rig.particles)?;
    check_normalization(&rig.normalization)?;
    settings.rigs.push(rig);
    Ok(())
}

/// リグ `id` を `before` の直前 (`None` なら末尾) へ移す。
fn reorder_rig(
    physics: &mut Option<PhysicsSettings>,
    id: &str,
    before: Option<&str>,
) -> Result<(), EditError> {
    if before == Some(id) {
        return Err(EditError::SelfReference {
            kind: "physics rig",
            id: id.to_owned(),
            field: "before",
        });
    }
    let settings = physics.as_mut().ok_or_else(|| rig_not_found(id))?;
    let from = settings
        .rigs
        .iter()
        .position(|r| r.id.as_str() == id)
        .ok_or_else(|| rig_not_found(id))?;
    if let Some(before) = before
        && !settings.rigs.iter().any(|r| r.id.as_str() == before)
    {
        return Err(rig_not_found(before));
    }
    let moved = settings.rigs.remove(from);
    let to = before
        .and_then(|before| settings.rigs.iter().position(|r| r.id.as_str() == before))
        .unwrap_or(settings.rigs.len());
    settings.rigs.insert(to, moved);
    Ok(())
}

fn check_collider_id(id: &str) -> Result<(), EditError> {
    if id.is_empty() {
        return Err(invalid("physics collider id must not be empty"));
    }
    Ok(())
}

fn check_finite(field: &str, value: f32) -> Result<(), EditError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(format!("{field} must be finite, got {value}")))
    }
}

fn check_vec(field: &str, value: Vec2) -> Result<(), EditError> {
    check_finite(field, value.x)?;
    check_finite(field, value.y)
}

fn check_input(input: &PhysicsInput) -> Result<(), EditError> {
    check_finite("input weight", input.weight)
}

fn check_output(output: &PhysicsOutput) -> Result<(), EditError> {
    check_finite("output scale", output.scale)?;
    check_finite("output weight", output.weight)?;
    if output.particle == 0 {
        return Err(invalid(
            "output particle must be 1 or more (particle 0 is the root)",
        ));
    }
    if output.blend == PhysicsBlend::Unknown {
        return Err(invalid("output blend must be overwrite or add"));
    }
    Ok(())
}

/// 粒子は 1 つ以上。値は有限で、`radius` と `collision_radius` は 0 以上、`mobility` は 0〜1
/// (1 を超えると揺れが発散する)、`delay` は 0 以上。
fn check_particles(particles: &[PhysicsParticle]) -> Result<(), EditError> {
    if particles.is_empty() {
        return Err(invalid("a physics rig needs at least one particle"));
    }
    for (i, p) in particles.iter().enumerate() {
        check_vec(&format!("particles[{i}].position"), p.position)?;
        for (name, value) in [
            ("mobility", p.mobility),
            ("delay", p.delay),
            ("acceleration", p.acceleration),
            ("radius", p.radius),
            ("collision_radius", p.collision_radius),
        ] {
            check_finite(&format!("particles[{i}].{name}"), value)?;
        }
        if !(0.0..=1.0).contains(&p.mobility) {
            return Err(invalid(format!(
                "particles[{i}].mobility must be in 0..=1, got {}",
                p.mobility
            )));
        }
        for (name, value) in [
            ("delay", p.delay),
            ("radius", p.radius),
            ("collision_radius", p.collision_radius),
        ] {
            if value < 0.0 {
                return Err(invalid(format!(
                    "particles[{i}].{name} must not be negative, got {value}"
                )));
            }
        }
    }
    Ok(())
}

fn check_normalization(normalization: &PhysicsNormalization) -> Result<(), EditError> {
    for (name, r) in [
        ("position", normalization.position),
        ("angle", normalization.angle),
    ] {
        let ok = [r.min, r.default, r.max].iter().all(|v| v.is_finite())
            && r.min <= r.default
            && r.default <= r.max;
        if !ok {
            return Err(invalid(format!(
                "normalization.{name} must be finite with min <= default <= max, got {} / {} / {}",
                r.min, r.default, r.max
            )));
        }
    }
    Ok(())
}

/// 回転デフォーマの鎖に沿ったリグを作る (`Command::CreatePhysicsRig`)。
///
/// 粒子 0 は最初のデフォーマの原点 (局所座標 `(0,0)`。根元はそのデフォーマに取り付く)。
/// 粒子 k (1 ≤ k < n) は k+1 番目のデフォーマの原点、粒子 n は最後のデフォーマの原点から重力の向きに、
/// 子孫の点の最も深い所まで進んだ点 (髪の房の先端)。位置は粒子 0 からの相対 (キャンバスのピクセル)。
fn chain_rig(
    model: &Model,
    physics: Option<&PhysicsSettings>,
    id: Option<PhysicsRigId>,
    name: String,
    deformers: &[DeformerId],
) -> Result<PhysicsRig, EditError> {
    let Some(first) = deformers.first() else {
        return Err(invalid("select at least one rotation deformer"));
    };
    let mut seen = HashSet::new();
    for d in deformers {
        if !seen.insert(d) {
            return Err(invalid(format!("deformer `{d}` is listed twice")));
        }
        let deformer =
            model
                .deformers
                .iter()
                .find(|x| &x.id == d)
                .ok_or_else(|| EditError::NotFound {
                    kind: "deformer",
                    id: d.to_string(),
                })?;
        if !matches!(deformer.kind, DeformerKind::Rotation(_)) {
            return Err(EditError::DeformerKindMismatch {
                id: d.to_string(),
                expected: "rotation",
            });
        }
    }
    let rest = Rest::new(model)?;
    let origin = |d: &DeformerId| rest.frame(Some(d)).map(Vec2::ZERO);
    let mut points: Vec<Vec2> = deformers.iter().map(origin).collect();
    let last = &deformers[deformers.len() - 1];
    let last_origin = points[points.len() - 1];
    // 先端は、子孫の点のうち重力の向きに最も遠い点の深さだけ、最後の原点から重力の向きに進んだ点。
    // 鎖が重力の向きに並ぶので、静止姿勢が初期位置と一致する (出力の角度は静止姿勢で 0)。
    let down = physics
        .map(|p| p.gravity)
        .and_then(Vec2::try_normalize)
        .unwrap_or(Vec2::Y);
    let mut subtree = Vec::new();
    subtree_points(
        model,
        &rest,
        &ChildRef::Deformer(last.clone()),
        &mut subtree,
    );
    let depth = subtree
        .iter()
        .map(|p| (*p - last_origin).dot(down))
        .fold(0.0f32, f32::max);
    let depth = if depth >= 1.0 {
        depth
    } else {
        model.meta.canvas.height.abs() * FALLBACK_TIP
    };
    let tip = last_origin + down * depth;
    points.push(tip);

    let root = points[0];
    let particles = points
        .iter()
        .enumerate()
        .map(|(i, &p)| PhysicsParticle {
            position: if i == 0 { Vec2::ZERO } else { p - root },
            mobility: if i == 0 { 1.0 } else { DEFAULT_MOBILITY },
            delay: if i == 0 { 1.0 } else { DEFAULT_DELAY },
            acceleration: DEFAULT_ACCELERATION,
            radius: if i == 0 {
                0.0
            } else {
                p.distance(points[i - 1])
            },
            collision_radius: 0.0,
        })
        .collect();
    let outputs = deformers
        .iter()
        .enumerate()
        .map(|(k, d)| PhysicsOutput {
            target: PhysicsTarget::DeformerAngle {
                deformer: d.clone(),
            },
            particle: u32::try_from(k + 1).unwrap_or(u32::MAX),
            source: PhysicsSource::Angle,
            scale: 1.0,
            weight: 100.0,
            reflect: false,
            blend: PhysicsBlend::Overwrite,
        })
        .collect();

    let first_name = model
        .deformers
        .iter()
        .find(|d| &d.id == first)
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let name = if name.is_empty() { first_name } else { name };
    let existing: Vec<&str> = physics
        .map(|p| p.rigs.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default();
    let id = match id {
        Some(id) => id,
        None if name.chars().any(|c| c.is_ascii_alphanumeric()) => {
            PhysicsRigId::new(ids::unique_id(&name, existing))
        }
        None => PhysicsRigId::new(ids::unique_id(&format!("Physics{first}"), existing)),
    };
    let range = NormalizationRange {
        min: -10.0,
        default: 0.0,
        max: 10.0,
    };
    Ok(PhysicsRig {
        id,
        name,
        inputs: Vec::new(),
        outputs,
        particles,
        normalization: PhysicsNormalization {
            position: range,
            angle: range,
        },
        colliders: Vec::new(),
        frame: None,
    })
}

/// メッシュ `id` に頂点を 1 つ追加した後、その頂点への出力の重みを 0 で足す (`vertices` の出力先)。
pub(crate) fn vertex_added(model: &Model, physics: &mut Option<PhysicsSettings>, id: &MeshId) {
    let Some(count) = model
        .meshes
        .iter()
        .find(|m| &m.id == id)
        .map(|m| m.uvs.len())
    else {
        return;
    };
    for weights in mesh_weights(physics, id) {
        if weights.len() + 1 == count {
            weights.push(0.0);
        }
    }
}

/// メッシュ `id` の頂点 `indices` を消した後、その頂点の重みを除く。
pub(crate) fn vertices_removed(
    physics: &mut Option<PhysicsSettings>,
    id: &MeshId,
    indices: &[u16],
) {
    let removed: HashSet<usize> = indices.iter().map(|&i| usize::from(i)).collect();
    for weights in mesh_weights(physics, id) {
        let kept = weights
            .iter()
            .enumerate()
            .filter(|(i, _)| !removed.contains(i))
            .map(|(_, &w)| w)
            .collect();
        *weights = kept;
    }
}

fn mesh_weights<'a>(
    physics: &'a mut Option<PhysicsSettings>,
    id: &'a MeshId,
) -> impl Iterator<Item = &'a mut Vec<f32>> + 'a {
    physics
        .iter_mut()
        .flat_map(|p| &mut p.rigs)
        .flat_map(|r| &mut r.outputs)
        .filter_map(move |o| match &mut o.target {
            PhysicsTarget::Vertices {
                owner: VertexOwner::Mesh(m),
                weights,
            } if m == id => Some(weights),
            _ => None,
        })
}

/// コマンドがモデルから消した要素 (パラメータ・デフォーマ・メッシュ) への物理の参照を外す。
///
/// - その要素を使う入力・出力は削除する。重力の基準 (`frame`) は外す (キャンバスになる)。
/// - 衝突体の取り付け先が消えたら、取り付けを外し、形状の位置を `before` の基準の姿での
///   キャンバス座標に写す (見た目の位置を保つ)。
///
/// 消えていない要素への参照 (読み込んだ時点で壊れていたものなど) は変えない。
pub(crate) fn forget_removed(before: &Model, after: &Model, physics: &mut Option<PhysicsSettings>) {
    let Some(settings) = physics.as_mut() else {
        return;
    };
    let removed = |old: Vec<&str>, new: Vec<&str>| -> HashSet<String> {
        let new: HashSet<&str> = new.into_iter().collect();
        old.into_iter()
            .filter(|id| !new.contains(id))
            .map(str::to_owned)
            .collect()
    };
    let params = removed(
        before.parameters.iter().map(|p| p.id.as_str()).collect(),
        after.parameters.iter().map(|p| p.id.as_str()).collect(),
    );
    let deformers = removed(
        before.deformers.iter().map(|d| d.id.as_str()).collect(),
        after.deformers.iter().map(|d| d.id.as_str()).collect(),
    );
    let meshes = removed(
        before.meshes.iter().map(|m| m.id.as_str()).collect(),
        after.meshes.iter().map(|m| m.id.as_str()).collect(),
    );
    if params.is_empty() && deformers.is_empty() && meshes.is_empty() {
        return;
    }
    for rig in &mut settings.rigs {
        rig.inputs
            .retain(|i| !params.contains(i.parameter.as_str()));
        rig.outputs.retain(|o| match &o.target {
            PhysicsTarget::Parameter { id } => !params.contains(id.as_str()),
            PhysicsTarget::DeformerAngle { deformer }
            | PhysicsTarget::DeformerOffset { deformer } => !deformers.contains(deformer.as_str()),
            PhysicsTarget::Vertices { owner, .. } => match owner {
                VertexOwner::Mesh(m) => !meshes.contains(m.as_str()),
                VertexOwner::Warp(d) => !deformers.contains(d.as_str()),
            },
            PhysicsTarget::Unknown => true,
        });
        if rig
            .frame
            .as_ref()
            .is_some_and(|f| deformers.contains(f.as_str()))
        {
            rig.frame = None;
        }
    }
    let detached: Vec<&mut PhysicsCollider> = settings
        .colliders
        .iter_mut()
        .filter(|c| {
            c.attach
                .as_ref()
                .is_some_and(|a| deformers.contains(a.as_str()))
        })
        .collect();
    if detached.is_empty() {
        return;
    }
    // 評価できないモデルなら、位置は局所座標のまま残す。
    let rest = Rest::new(before).ok();
    for collider in detached {
        if let Some(rest) = &rest {
            let frame = rest.frame(collider.attach.as_ref());
            collider.shape = match collider.shape {
                ColliderShape::Circle { center, radius } => ColliderShape::Circle {
                    center: frame.map(center),
                    radius,
                },
                ColliderShape::Capsule { a, b, radius } => ColliderShape::Capsule {
                    a: frame.map(a),
                    b: frame.map(b),
                    radius,
                },
                ColliderShape::Unknown => ColliderShape::Unknown,
            };
        }
        collider.attach = None;
    }
}
