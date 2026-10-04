//! 物理の設定の検査 (警告)。`docs/editor.md#物理の警告`。
//!
//! モデルだけを見る `validate_physics` (エラー) では分からない、実際に評価して初めて分かる問題を知らせる。
//! モデルも物理も変えない。`ugoe validate` とエディタの「Verify」のパネルが使う。

use serde::Serialize;
use ts_rs::TS;
use ugoe_model::Model;
use ugoe_model::physics::{ColliderId, PhysicsRigId, PhysicsSettings};
use ugoe_physics::PhysicsSystem;
use ugoe_runtime::{CompileError, CompiledModel, ModelInstance};

/// 衝突体を考えない静止姿勢で、リグの鎖が衝突体に当たっている (`docs/spec/physics.md#静止姿勢で衝突体に入っている場合`)。
///
/// 最初の `update` で、髪は真下に垂れずに衝突体の手前に止まり、衝突体に沿って置かれる。
/// 作者が意図しない姿勢になりやすいので警告にする (エラーではない)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct RestPoseCollision {
    /// リグの ID。
    pub rig: PhysicsRigId,
    /// 衝突体の ID。
    pub collider: ColliderId,
    /// 当たっている区間の、子の粒子の添字 (昇順。区間は粒子 `i - 1` → `i`)。
    pub particles: Vec<u32>,
}

/// パラメータの既定値での静止姿勢で、衝突体に当たっているリグの鎖を、リグと衝突体の組ごとに返す
/// (リグの並びの順、同じリグの中は最初に当たる粒子の順)。
///
/// 衝突体の位置はデフォーマの評価で決まるので、モデルをコンパイルして評価する
/// (`ugoe_physics::PhysicsSystem::rest_pose_collisions`)。動的物理を使わない設定では空。
///
/// # Errors
///
/// モデルをコンパイルできない (検証に通らない) とき。
pub fn rest_pose_collisions(
    model: &Model,
    physics: &PhysicsSettings,
) -> Result<Vec<RestPoseCollision>, CompileError> {
    let mut system = PhysicsSystem::new(physics.clone());
    if !system.is_dynamic() {
        return Ok(Vec::new());
    }
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone())?);
    let mut out: Vec<RestPoseCollision> = Vec::new();
    for found in system.rest_pose_collisions(&mut instance) {
        let particle = u32::try_from(found.particle).unwrap_or(u32::MAX);
        match out
            .iter_mut()
            .find(|c| c.rig == found.rig && c.collider == found.collider)
        {
            Some(c) => c.particles.push(particle),
            None => out.push(RestPoseCollision {
                rig: found.rig,
                collider: found.collider,
                particles: vec![particle],
            }),
        }
    }
    Ok(out)
}
