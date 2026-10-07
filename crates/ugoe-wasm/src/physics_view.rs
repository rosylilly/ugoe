//! エディタの物理の表示用の問い合わせ (T0629): デフォーマの写像と粒子の今の位置。GPU には触れない。
//!
//! 衝突体はデフォーマの局所座標で置かれるので、エディタはその写像 (と逆写像のためのヤコビ行列) で
//! キャンバスに置く (`docs/editor.md#物理の画面での操作`)。値は平らな `f32` の列で JS に渡す。

use ugoe::Puppet;
use ugoe::core::Vec2;
use ugoe::runtime::ModelInstance;

/// デフォーマ `id` の局所座標の点 `p` のキャンバス座標 `[x, y]` (ランタイムのワールド変換)。
/// デフォーマがなければ `None`。`instance` は評価済みであること。
pub fn deformer_point(instance: &ModelInstance, id: &str, p: Vec2) -> Option<Vec<f32>> {
    let index = instance.compiled().deformer_index(id)?;
    let w = instance.deformer_world_point(index, p)?;
    Some(vec![w.x, w.y])
}

/// デフォーマ `id` のワールド変換の、局所座標の点 `p` でのヤコビ行列 `[a, b, c, d]`
/// (列優先: 局所の x 方向の像 `(a, b)` と y 方向の像 `(c, d)`)。デフォーマがなければ `None`。
pub fn deformer_jacobian(instance: &ModelInstance, id: &str, p: Vec2) -> Option<Vec<f32>> {
    let index = instance.compiled().deformer_index(id)?;
    let j = instance.deformer_world_jacobian(index, p)?;
    Some(vec![j.x_axis.x, j.x_axis.y, j.y_axis.x, j.y_axis.y])
}

/// リグ `rig` の粒子の今の位置 `[x0, y0, x1, y1, ...]`。リグや物理が無ければ空。
pub fn particles(puppet: Option<&Puppet>, rig: u32) -> Vec<f32> {
    puppet
        .and_then(Puppet::physics)
        .and_then(|p| p.chain(usize::try_from(rig).ok()?))
        .map(|chain| {
            chain
                .particles()
                .iter()
                .flat_map(|p| [p.position.x, p.position.y])
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;
    use ugoe::model::standard;
    use ugoe::runtime::CompiledModel;

    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    #[test]
    fn deformer_point_and_jacobian_follow_the_world_transform() {
        let mut instance =
            ModelInstance::new(CompiledModel::compile(fixtures::rotation_chain()).unwrap());
        let index = instance.parameters().index_of("Angle").unwrap();
        instance.parameters_mut().set(index, 30.0);
        instance.update();
        let (s, c) = 30f32.to_radians().sin_cos();
        // Arm は (128,128) を中心に時計回りに 30 度。
        let p = deformer_point(&instance, "Arm", Vec2::new(10.0, 0.0)).unwrap();
        assert!(close(&p, &[128.0 + 10.0 * c, 128.0 + 10.0 * s]));
        let j = deformer_jacobian(&instance, "Arm", Vec2::ZERO).unwrap();
        assert!(close(&j, &[c, s, -s, c]));
        // ワープ (Cloth) の UV (0,0) は最初の制御点。
        let cloth = crate::gizmo::warp_points(&instance, "Cloth").unwrap();
        let corner = deformer_point(&instance, "Cloth", Vec2::ZERO).unwrap();
        assert!(close(&corner, &cloth[..2]));
        assert!(deformer_point(&instance, "Nope", Vec2::ZERO).is_none());
        assert!(deformer_jacobian(&instance, "Nope", Vec2::ZERO).is_none());
    }

    #[test]
    fn particles_follow_the_simulation() {
        let mut puppet = Puppet::new(fixtures::dynamic_hair()).unwrap();
        puppet.set_effects(Default::default());
        assert!(particles(Some(&puppet), 0).is_empty());
        puppet
            .set_physics(Some(fixtures::dynamic_hair_settings()))
            .unwrap();
        puppet.update(DT);
        // リグ Hair の根元は Hair1 の原点 (148,100)。粒子は 4 つ。
        let hair = particles(Some(&puppet), 0);
        assert_eq!(hair.len(), 8);
        assert!(close(&hair[..2], &[148.0, 100.0]));
        // 頭を動かすと根元がついて動く。
        let p = puppet.instance_mut().parameters_mut();
        let i = p.index_of(standard::ANGLE_X).unwrap();
        p.set(i, 20.0);
        puppet.update(DT);
        assert!(particles(Some(&puppet), 0)[0] > hair[0] + 1.0);
        assert!(particles(Some(&puppet), 9).is_empty());
        assert!(particles(None, 0).is_empty());
    }
}
