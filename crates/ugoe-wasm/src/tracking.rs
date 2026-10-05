//! フェイストラッキングの入力を [`Puppet`] に渡すための補助 (JS への公開は `web/tracking.rs`)。
//!
//! ブレンドシェイプの名前と値の列 (MediaPipe Face Landmarker の出力そのまま) と頭の回転から
//! [`TrackingInput`] を作る。パラメータへの変換は `ugoe_motion::tracking` (T0306) が行い、ここでは式を持たない。
//! 平滑化・キャリブレーション・回転行列からの角度の計算・鏡像の入れ替えはホスト (ビューア) が行う
//! (`docs/spec/avatar-profile.md#実装の範囲`)。

use ugoe::Puppet;
use ugoe::motion::{BodyInput, Rotation, TrackingInput, TrackingMap};

/// 名前 (ARKit / MediaPipe のカテゴリ名) と値の列、頭の回転 (度、プロファイルの向きの約束) と体の入力から
/// 入力を作る。
///
/// 知らない名前 (MediaPipe の `_neutral` など) は無視し、渡されなかったブレンドシェイプは 0 にする。
/// `names` と `scores` の長さが違えば、短い方に合わせる。体の入力の省略した軸は頭の回転 × 0.3 になる。
pub fn tracking_input(
    names: &[String],
    scores: &[f32],
    head: Rotation,
    body: BodyInput,
) -> TrackingInput {
    let mut input = TrackingInput {
        head,
        body,
        ..TrackingInput::default()
    };
    input.set_blend_shapes(names.iter().zip(scores.iter().copied()));
    input
}

/// トラッキングが書き込むパラメータの ID (モデルの定義順)。モデルの `avatar.tracking` の上書きを反映する。
pub fn tracking_targets(puppet: &Puppet) -> Vec<String> {
    let instance = puppet.instance();
    let model = instance.compiled().model();
    let parameters = instance.parameters();
    let map = TrackingMap::for_model(model, parameters);
    let mut indices: Vec<u32> = map.targets().map(|i| i.0).collect();
    indices.sort_unstable();
    indices
        .into_iter()
        .filter_map(|i| model.parameters.get(i as usize))
        .map(|p| p.id.as_str().to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use ugoe::model::effects::Effects;
    use ugoe::model::fixtures;

    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn input_takes_mediapipe_names_and_ignores_unknown_ones() {
        let head = Rotation {
            yaw: 10.0,
            pitch: -5.0,
            roll: 2.0,
        };
        let input = tracking_input(
            &names(&["_neutral", "eyeBlinkLeft", "jawOpen", "mouthClose"]),
            &[1.0, 0.75, 0.5],
            head,
            BodyInput::default(),
        );
        assert_eq!(input.head, head);
        assert_eq!(input.body, BodyInput::default());
        assert_eq!(input.blend_shape("eyeBlinkLeft"), Some(0.75));
        assert_eq!(input.blend_shape("jawOpen"), Some(0.5));
        // 値が足りない名前は無視する。
        assert_eq!(input.blend_shape("mouthClose"), Some(0.0));
    }

    #[test]
    fn puppet_maps_the_input_through_the_rust_mapping() {
        let mut puppet = Puppet::new(fixtures::avatar_profile()).unwrap();
        // 目パチと呼吸が重ならないように、エフェクトを止めて確かめる。
        puppet.set_effects(Effects::default());
        let input = tracking_input(
            &names(&["eyeBlinkLeft", "jawOpen"]),
            &[0.75, 0.5],
            Rotation {
                yaw: 15.0,
                ..Rotation::default()
            },
            BodyInput {
                roll: Some(-4.0),
                ..BodyInput::default()
            },
        );
        puppet.set_tracking(Some(input));
        puppet.update(1.0 / 60.0);
        let params = puppet.instance().parameters();
        let get = |id: &str| params.get(params.index_of(id).unwrap());
        assert!((get("EyeLOpen") - 0.25).abs() < 1e-6);
        assert!((get("EyeROpen") - 1.0).abs() < 1e-6);
        assert!((get("MouthOpenY") - 0.5).abs() < 1e-6);
        assert!((get("AngleX") - 15.0).abs() < 1e-6);
        // 体のヨーは省略したので頭 × 0.3、ロールは体の入力。
        assert!((get("BodyAngleX") - 4.5).abs() < 1e-5);
        assert!((get("BodyAngleZ") + 4.0).abs() < 1e-6);
    }

    #[test]
    fn targets_list_the_tracked_parameters_in_model_order() {
        let puppet = Puppet::new(fixtures::avatar_profile()).unwrap();
        let targets = tracking_targets(&puppet);
        for id in ["AngleX", "EyeLOpen", "MouthOpenY", "EyeBallX", "BodyAngleX"] {
            assert!(targets.iter().any(|t| t == id), "{id} is not tracked");
        }
        let model = fixtures::avatar_profile();
        let order: Vec<usize> = targets
            .iter()
            .map(|t| {
                model
                    .parameters
                    .iter()
                    .position(|p| p.id == t.as_str())
                    .unwrap()
            })
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]));
        // 呼吸はトラッキングしない。
        assert!(!targets.iter().any(|t| t == "Breath"));
    }
}
