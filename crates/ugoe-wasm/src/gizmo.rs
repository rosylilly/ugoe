//! デフォーマのギズモ用の問い合わせ: 今のパラメータ値でのデフォーマのキャンバス座標の形。GPU には触れない。
//!
//! ランタイムが評価したワールド変換 (`docs/spec/evaluation.md#デフォーマのワールド変換を読む`) を
//! JS に渡しやすい形 (平らな `f32` の列) にする。
//!
//! - ワープ: キャンバス座標の制御点。
//! - 回転デフォーマ: ワールド変換 (アフィン) の原点と軸。
//! - 回転デフォーマ: 今のパラメータ値で補間した局所の値 (親の座標系の原点・角度・拡大率)。

use ugoe::model::Model;
use ugoe::model::deformer::DeformerKind;
use ugoe::runtime::ModelInstance;

/// ワープと回転デフォーマの ID (モデルの定義順。未知の種類は含まない)。
pub fn ids(model: &Model) -> Vec<String> {
    model
        .deformers
        .iter()
        .filter(|d| !matches!(d.kind, DeformerKind::Unknown))
        .map(|d| d.id.to_string())
        .collect()
}

/// ワープ `id` のキャンバス座標の制御点 `[x0, y0, x1, y1, ...]` (行優先、x が速い)。
/// ワープでなければ `None`。`instance` は評価済みであること。
pub fn warp_points(instance: &ModelInstance, id: &str) -> Option<Vec<f32>> {
    let index = instance.compiled().deformer_index(id)?;
    let points = instance.warp_world_points(index)?;
    Some(points.iter().flat_map(|p| [p.x, p.y]).collect())
}

/// 回転デフォーマ `id` のワールド変換 `[ox, oy, ax, ay, bx, by]`: 原点 `o` と、局所座標の x 軸・y 軸の
/// 単位ベクトルの像 `a`・`b` (`world(p) = o + a * p.x + b * p.y`)。回転デフォーマでなければ `None`。
/// `instance` は評価済みであること。
pub fn rotation_frame(instance: &ModelInstance, id: &str) -> Option<Vec<f32>> {
    let index = instance.compiled().deformer_index(id)?;
    let frame = instance.rotation_world_frame(index)?;
    let (o, a, b) = (
        frame.translation,
        frame.matrix2.x_axis,
        frame.matrix2.y_axis,
    );
    Some(vec![o.x, o.y, a.x, a.y, b.x, b.y])
}

/// 回転デフォーマ `id` の今のパラメータ値で補間した局所の値 `[ox, oy, angle, scale]` (原点は親の座標系、角度は度、
/// 時計回り正)。キーの間でもキーフォームの補間 (とブレンドシェイプ) をした値で、親の変換は掛けない
/// (`ModelInstance::rotation_local_keyform`)。回転デフォーマでなければ `None`。
pub fn rotation_local(instance: &ModelInstance, id: &str) -> Option<Vec<f32>> {
    let index = instance.compiled().deformer_index(id)?;
    let k = instance.rotation_local_keyform(index)?;
    Some(vec![k.origin.x, k.origin.y, k.angle, k.scale])
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;
    use ugoe::runtime::CompiledModel;

    use super::*;

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    fn instance(model: Model) -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(model).unwrap())
    }

    fn set(instance: &mut ModelInstance, id: &str, value: f32) {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, value);
        instance.update();
    }

    #[test]
    fn warp_points_follow_parameters() {
        let model = fixtures::warp_grid();
        assert_eq!(ids(&model), vec!["Warp"]);
        let mut instance = instance(model);
        // X = 0 は 64..192 の正方形の 2×2 セル。
        let p = warp_points(&instance, "Warp").unwrap();
        assert_eq!(p.len(), 18);
        assert!(close(&p[..4], &[64.0, 64.0, 128.0, 64.0]));
        // X = 1 は上の行ほど右へずれるせん断。
        set(&mut instance, "X", 1.0);
        let p = warp_points(&instance, "Warp").unwrap();
        assert!(close(&p[..2], &[128.0, 64.0]));
        assert!(rotation_frame(&instance, "Warp").is_none());
    }

    #[test]
    fn rotation_frame_composes_parents() {
        let model = fixtures::rotation_chain();
        assert_eq!(ids(&model), vec!["Arm", "Hand", "Cloth"]);
        let mut instance = instance(model);
        // Angle = 0: Arm は (128,128) の恒等回転、Hand は Arm の (64,0) で拡大率 0.5。
        let hand = rotation_frame(&instance, "Hand").unwrap();
        assert!(close(&hand, &[192.0, 128.0, 0.5, 0.0, 0.0, 0.5]));
        // Angle = 30: Arm が時計回りに 30 度回る。
        set(&mut instance, "Angle", 30.0);
        let arm = rotation_frame(&instance, "Arm").unwrap();
        let (s, c) = 30f32.to_radians().sin_cos();
        assert!(close(&arm, &[128.0, 128.0, c, s, -s, c]));
        // Cloth (Arm の子のワープ) の制御点も回る。
        let cloth = warp_points(&instance, "Cloth").unwrap();
        assert!(close(&cloth[..2], &[128.0 - 20.0 * s, 128.0 + 20.0 * c]));
    }

    #[test]
    fn rotation_local_interpolates_between_keys_without_parents() {
        let mut instance = instance(fixtures::rotation_chain());
        // Angle = 0 は Arm のキー -30・30 のちょうど間: 角度は補間した 0、原点は親の座標系のまま。
        let arm = rotation_local(&instance, "Arm").unwrap();
        assert!(close(&arm, &[128.0, 128.0, 0.0, 1.0]));
        // Angle = 15 は 3/4 の位置で 15 度。
        set(&mut instance, "Angle", 15.0);
        let arm = rotation_local(&instance, "Arm").unwrap();
        assert!(close(&arm, &[128.0, 128.0, 15.0, 1.0]));
        // Hand は Arm の回転を掛けない局所の値 (rotation_frame はワールドで回っている)。
        let hand = rotation_local(&instance, "Hand").unwrap();
        assert!(close(&hand, &[64.0, 0.0, 0.0, 0.5]));
        let world = rotation_frame(&instance, "Hand").unwrap();
        assert!((world[0] - 192.0).abs() > 1.0);
        // ワープ・知らない ID は None。
        assert!(rotation_local(&instance, "Cloth").is_none());
        assert!(rotation_local(&instance, "Nope").is_none());
    }

    #[test]
    fn unknown_ids_are_none() {
        let instance = instance(fixtures::rotation_under_warp());
        assert!(rotation_frame(&instance, "Head").is_some());
        assert!(rotation_frame(&instance, "Nope").is_none());
        assert!(warp_points(&instance, "Nope").is_none());
    }
}
