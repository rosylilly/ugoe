//! 動的物理の出力 (デフォーマ・頂点への加算量) と、2 段階の評価の状態。
//! 仕様: `docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`。

use alloc::vec;
use alloc::vec::Vec;
use ugoe_core::Vec2;
use ugoe_model::{DeformerKind, Model};

/// 動的物理がデフォーマと頂点に加える量。
///
/// [`crate::ModelInstance::dynamic_offsets_mut`] で得る。値は次の `update` で評価に加わり、
/// [`DynamicOffsets::clear`] するまで残る。
///
/// - 回転デフォーマの角度: キーフォームで補間した角度 (局所座標系、度) に足す。
/// - 回転デフォーマの原点・ワープの制御点・メッシュの頂点: ワールド変換で写した後の
///   キャンバス座標に足す (ピクセル)。
///
/// 有限でない加算量 (NaN か無限大。位置は成分のどれか) は、評価で 0 として扱う (書いた値は残るが足さない)。
#[derive(Debug, Clone)]
pub struct DynamicOffsets {
    /// デフォーマごとの角度の加算量 (回転デフォーマ以外は使わない)。
    angle: Vec<f32>,
    /// デフォーマごとの原点のずれ (キャンバス座標)。
    origin: Vec<Vec2>,
    /// ワープごとの制御点のずれ (キャンバス座標)。使うまで空。
    warp: Vec<Vec<Vec2>>,
    /// メッシュごとの頂点のずれ (キャンバス座標)。使うまで空。
    mesh: Vec<Vec<Vec2>>,
    /// ワープの制御点の数 (ワープ以外は 0)。
    warp_len: Vec<usize>,
    /// メッシュの頂点の数。
    mesh_len: Vec<usize>,
    /// 加算量を受けるデフォーマ (2 段目で評価し直す起点)。一度立てたら下ろさない。
    pub(crate) targets: Vec<bool>,
}

impl DynamicOffsets {
    pub(crate) fn new(model: &Model) -> Self {
        let n = model.deformers.len();
        Self {
            angle: vec![0.0; n],
            origin: vec![Vec2::ZERO; n],
            warp: vec![Vec::new(); n],
            mesh: vec![Vec::new(); model.meshes.len()],
            warp_len: model
                .deformers
                .iter()
                .map(|d| match &d.kind {
                    DeformerKind::Warp(w) => (usize::from(w.rows) + 1) * (usize::from(w.cols) + 1),
                    _ => 0,
                })
                .collect(),
            mesh_len: model.meshes.iter().map(|m| m.uvs.len()).collect(),
            targets: vec![false; n],
        }
    }

    /// すべての加算量を 0 に戻す (バッファと、評価し直すデフォーマの印は残す)。
    pub fn clear(&mut self) {
        self.angle.fill(0.0);
        self.origin.fill(Vec2::ZERO);
        for v in self.warp.iter_mut().chain(&mut self.mesh) {
            v.fill(Vec2::ZERO);
        }
    }

    /// デフォーマ `deformer` の角度に `degrees` を足す。範囲外なら何もしない。
    ///
    /// 回転デフォーマ以外では評価に使われない。
    pub fn add_angle(&mut self, deformer: usize, degrees: f32) {
        if let (Some(a), Some(target)) =
            (self.angle.get_mut(deformer), self.targets.get_mut(deformer))
        {
            *a += degrees;
            *target = true;
        }
    }

    /// デフォーマ `deformer` の原点のワールド位置を `offset` (キャンバス座標) だけずらす。
    /// 範囲外なら何もしない。回転デフォーマ以外では評価に使われない。
    pub fn add_origin(&mut self, deformer: usize, offset: Vec2) {
        if let (Some(o), Some(target)) = (
            self.origin.get_mut(deformer),
            self.targets.get_mut(deformer),
        ) {
            *o += offset;
            *target = true;
        }
    }

    /// ワープ `deformer` の制御点ごとのずれ (キャンバス座標、行優先で x が速い)。
    /// 最初の呼び出しでバッファを確保する。ワープでなければ `None`。
    pub fn warp_points_mut(&mut self, deformer: usize) -> Option<&mut [Vec2]> {
        let len = *self.warp_len.get(deformer)?;
        if len == 0 {
            return None;
        }
        let (Some(target), Some(v)) = (self.targets.get_mut(deformer), self.warp.get_mut(deformer))
        else {
            return None;
        };
        *target = true;
        if v.is_empty() {
            *v = vec![Vec2::ZERO; len];
        }
        Some(v)
    }

    /// メッシュ `mesh` の頂点ごとのずれ (キャンバス座標、グルーの前に足す)。
    /// 最初の呼び出しでバッファを確保する。範囲外なら `None`。
    pub fn mesh_positions_mut(&mut self, mesh: usize) -> Option<&mut [Vec2]> {
        let len = *self.mesh_len.get(mesh)?;
        let v = self.mesh.get_mut(mesh)?;
        if v.len() != len {
            *v = vec![Vec2::ZERO; len];
        }
        Some(v)
    }

    /// デフォーマ `deformer` の角度の加算量。
    pub fn angle(&self, deformer: usize) -> f32 {
        self.angle.get(deformer).copied().unwrap_or(0.0)
    }

    /// デフォーマ `deformer` の原点のずれ。
    pub fn origin(&self, deformer: usize) -> Vec2 {
        self.origin.get(deformer).copied().unwrap_or(Vec2::ZERO)
    }

    /// ワープ `deformer` の制御点のずれ。まだ確保していなければ空。
    pub fn warp_points(&self, deformer: usize) -> &[Vec2] {
        self.warp.get(deformer).map_or(&[], Vec::as_slice)
    }

    /// メッシュ `mesh` の頂点のずれ。まだ確保していなければ空。
    pub fn mesh_positions(&self, mesh: usize) -> &[Vec2] {
        self.mesh.get(mesh).map_or(&[], Vec::as_slice)
    }
}

/// 評価に足す角度の加算量。有限でなければ 0 (足さない)。
///
/// 加算量はホストも書けるので、NaN・無限大を頂点に入れないよう評価の側で止める
/// (`docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`)。
pub(crate) fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// 評価に足す位置の加算量。成分のどれかが有限でなければ 0 (足さない)。[`finite_or_zero`] と同じ理由。
pub(crate) fn finite_offset(offset: Vec2) -> Vec2 {
    if offset.is_finite() {
        offset
    } else {
        Vec2::ZERO
    }
}

/// 2 段階の評価の状態。
#[derive(Debug, Clone)]
pub(crate) struct Dynamic {
    pub(crate) offsets: DynamicOffsets,
    /// 1 段目で使ったパラメータ値 (2 段目で変わったものを見つける)。
    pub(crate) snapshot: Vec<f32>,
    /// 2 段目で評価し直すデフォーマ。
    pub(crate) dirty: Vec<bool>,
    /// 1 段目を評価して、2 段目を待っているか。
    pub(crate) pending: bool,
}

impl Dynamic {
    pub(crate) fn new(model: &Model) -> Self {
        Self {
            offsets: DynamicOffsets::new(model),
            snapshot: vec![0.0; model.parameters.len()],
            dirty: vec![false; model.deformers.len()],
            pending: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe_core::{Mat2, Vec2};
    use ugoe_model::fixtures;

    use crate::{CompiledModel, ModelInstance};

    fn instance() -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(fixtures::dynamic_hair()).unwrap())
    }

    fn deformer(instance: &ModelInstance, id: &str) -> usize {
        instance.compiled().deformer_index(id).unwrap()
    }

    fn mesh(instance: &ModelInstance, id: &str) -> usize {
        instance.compiled().mesh_index(id).unwrap()
    }

    fn set(instance: &mut ModelInstance, id: &str, value: f32) {
        let i = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(i, value);
    }

    /// Hair1 に 90 度・原点 (5,0)、Cloth の制御点 2 に (3,4)、Ribbon の頂点 2 に (1,2) を加える。
    fn apply_offsets(instance: &mut ModelInstance) {
        let (hair1, cloth, ribbon) = (
            deformer(instance, "Hair1"),
            deformer(instance, "Cloth"),
            mesh(instance, "Ribbon"),
        );
        let offsets = instance.dynamic_offsets_mut();
        offsets.clear();
        offsets.add_angle(hair1, 90.0);
        offsets.add_origin(hair1, Vec2::new(5.0, 0.0));
        offsets.warp_points_mut(cloth).unwrap()[2] += Vec2::new(3.0, 4.0);
        offsets.mesh_positions_mut(ribbon).unwrap()[2] += Vec2::new(1.0, 2.0);
    }

    #[test]
    fn models_without_dynamic_physics_have_no_dynamic_state() {
        let mut instance =
            ModelInstance::new(CompiledModel::compile(fixtures::single_quad()).unwrap());
        instance.update();
        assert!(instance.dynamic_offsets().is_none());
    }

    #[test]
    fn offsets_move_deformers_and_vertices() {
        let mut instance = instance();
        let before = instance.drawables().to_vec();
        let (hair1, cloth, ribbon) = (
            deformer(&instance, "Hair1"),
            deformer(&instance, "Cloth"),
            mesh(&instance, "Ribbon"),
        );
        let cloth_before = instance.warp_world_points(cloth).unwrap().to_vec();
        apply_offsets(&mut instance);
        instance.update();

        // 角度は局所座標系で足し、原点のずれはワールド座標で足す。
        let frame = instance.rotation_world_frame(hair1).unwrap();
        assert!(frame.translation.abs_diff_eq(Vec2::new(153.0, 100.0), 1e-4));
        assert!(
            frame
                .matrix2
                .abs_diff_eq(Mat2::from_angle(90f32.to_radians()), 1e-6)
        );
        // 子孫の Hair3 も回る: 原点は Hair1 から (0,60) を 90 度回した (-60,0) の位置。
        let hair3 = instance
            .rotation_world_frame(deformer(&instance, "Hair3"))
            .unwrap();
        assert!(hair3.translation.abs_diff_eq(Vec2::new(93.0, 100.0), 1e-3));

        let points = instance.warp_world_points(cloth).unwrap();
        assert_eq!(points[2], cloth_before[2] + Vec2::new(3.0, 4.0));
        assert_eq!(points[0], cloth_before[0]);
        let r = &instance.drawables()[ribbon].positions;
        assert_eq!(r[2], before[ribbon].positions[2] + Vec2::new(1.0, 2.0));
        assert_eq!(r[0], before[ribbon].positions[0]);

        // clear すると元に戻る。
        instance.dynamic_offsets_mut().clear();
        instance.update();
        assert_eq!(instance.drawables(), &before[..]);
    }

    // T0010: ホストが書いた有限でない加算量は足さない (0 と同じ)。有限の加算量はそのまま効く。
    #[test]
    fn non_finite_offsets_are_not_added() {
        let mut instance = instance();
        let before = instance.drawables().to_vec();
        let (hair1, cloth, ribbon) = (
            deformer(&instance, "Hair1"),
            deformer(&instance, "Cloth"),
            mesh(&instance, "Ribbon"),
        );
        let frame_before = instance.rotation_world_frame(hair1).unwrap();
        let cloth_before = instance.warp_world_points(cloth).unwrap().to_vec();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let offsets = instance.dynamic_offsets_mut();
            offsets.clear();
            offsets.add_angle(hair1, bad);
            offsets.add_origin(hair1, Vec2::new(0.0, bad));
            let points = offsets.warp_points_mut(cloth).unwrap();
            points[0] = Vec2::new(bad, 0.0);
            points[2] = Vec2::new(3.0, 4.0);
            offsets.mesh_positions_mut(ribbon).unwrap()[0] = Vec2::splat(bad);
            instance.update();

            assert_eq!(instance.rotation_world_frame(hair1).unwrap(), frame_before);
            let points = instance.warp_world_points(cloth).unwrap();
            assert_eq!(points[0], cloth_before[0]);
            assert_eq!(points[2], cloth_before[2] + Vec2::new(3.0, 4.0));
            assert_eq!(
                instance.drawables()[ribbon].positions[0],
                before[ribbon].positions[0]
            );
            for d in instance.drawables() {
                assert!(d.positions.iter().all(|p| p.is_finite()), "{bad}");
            }
        }
    }

    // 1 段目 → (パラメータと加算量の変更) → 2 段目は、同じ値での通常の update と一致する。
    #[test]
    fn second_stage_matches_full_update() {
        let mut staged = instance();
        let mut full = instance();

        set(&mut staged, "AngleZ", 10.0);
        staged.update_first_stage();
        // 1 段目は加算量を加えない。
        apply_offsets(&mut staged);
        let hair1 = deformer(&staged, "Hair1");
        let frame = staged.rotation_world_frame(hair1).unwrap();
        assert!(frame.translation.abs_diff_eq(
            Vec2::new(128.0, 100.0) + Mat2::from_angle(10f32.to_radians()) * Vec2::new(20.0, 0.0),
            1e-4
        ));
        let count = staged.update_count();
        // 1 段目の後に変えたパラメータも 2 段目で反映される。
        set(&mut staged, "AngleX", 12.0);
        staged.update();
        assert_eq!(staged.update_count(), count + 1);

        set(&mut full, "AngleZ", 10.0);
        set(&mut full, "AngleX", 12.0);
        apply_offsets(&mut full);
        full.update();

        assert_eq!(staged.drawables(), full.drawables());
        for d in 0..staged.compiled().model().deformers.len() {
            assert_eq!(staged.rotation_world_frame(d), full.rotation_world_frame(d));
            assert_eq!(staged.warp_world_points(d), full.warp_world_points(d));
        }
    }

    /// 親の座標系と補間した値から、回転デフォーマのワールド変換を作り直す。
    fn rebuilt_world(
        instance: &ModelInstance,
        d: usize,
        local: &ugoe_model::deformer::RotationKeyform,
    ) -> ugoe_core::Affine2 {
        instance
            .deformer_parent_world(d)
            .unwrap()
            .rotation_child(local)
    }

    // ブレンドシェイプの軸だけが 1 段目の後に変わっても、2 段目で評価し直す。
    #[test]
    fn second_stage_picks_up_blend_shape_parameters() {
        let compiled = CompiledModel::compile(fixtures::blend_shapes()).unwrap();
        let ranges: Vec<_> = compiled
            .model()
            .parameters
            .iter()
            .map(|p| (p.min, p.max))
            .collect();
        let mut staged = ModelInstance::new(compiled.clone());
        let mut full = ModelInstance::new(compiled);
        staged.update_first_stage();
        for (i, &(min, max)) in ranges.iter().enumerate() {
            let v = min + (max - min) * 0.7;
            staged.parameters_mut().set(crate::ParamIndex(i as u32), v);
            full.parameters_mut().set(crate::ParamIndex(i as u32), v);
        }
        staged.update();
        full.update();
        // 問い合わせの補間もブレンドシェイプの差分を含み、評価に使った値と同じになる。
        let jaw = full.compiled().deformer_index("Jaw").unwrap();
        let local = full.rotation_local_keyform(jaw).unwrap();
        let frame = rebuilt_world(&full, jaw, &local);
        assert_eq!(Some(frame), full.rotation_world_frame(jaw));
        assert_eq!(staged.drawables(), full.drawables());
        for d in 0..staged.compiled().model().deformers.len() {
            assert_eq!(staged.rotation_world_frame(d), full.rotation_world_frame(d));
            assert_eq!(staged.warp_world_points(d), full.warp_world_points(d));
        }
    }

    // 2 段目は、加算量を受けるデフォーマ・パラメータが変わったデフォーマとその子孫だけを評価し直す。
    #[test]
    fn second_stage_reevaluates_only_affected_deformers() {
        let mut instance = instance();
        let hair1 = deformer(&instance, "Hair1");
        instance.update_first_stage();
        instance.dynamic_offsets_mut().add_angle(hair1, 5.0);
        instance.update();
        let dirty: Vec<&str> = instance
            .compiled()
            .model()
            .deformers
            .iter()
            .zip(&instance.dynamic.as_ref().unwrap().dirty)
            .filter(|(_, dirty)| **dirty)
            .map(|(d, _)| d.id.as_str())
            .collect();
        assert_eq!(dirty, ["Hair1", "Hair2", "Hair3"]);

        // パラメータが変わると、そのデフォーマ (Head) とその子孫も評価し直す。
        instance.update_first_stage();
        set(&mut instance, "AngleZ", 3.0);
        instance.update();
        let dynamic = instance.dynamic.as_ref().unwrap();
        let head = deformer(&instance, "Head");
        assert!(dynamic.dirty[head]);
        assert!(!dynamic.dirty[deformer(&instance, "Body")]);
        assert!(!dynamic.dirty[deformer(&instance, "Cloth")]);
    }
}
