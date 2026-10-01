//! 描画結果の読み戻しと評価の結果の取り出しのうち、GPU とブラウザに触れない部分 (T0418)。
//! JS への公開は `src/web/readback.rs` (`read_pixels`・`drawable_states_json`)。

use serde::Serialize;
use ugoe::runtime::ModelInstance;

/// 要素 1 つの評価の結果 (`drawable_states_json` の要素。`docs/spec/evaluation.md#8-出力`)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawableState {
    pub opacity: f32,
    pub draw_order: f32,
    pub multiply_color: [f32; 3],
    pub screen_color: [f32; 3],
    pub mask_strength: f32,
    pub visible: bool,
}

/// 評価済みのインスタンスの要素ごとの結果 (モデルのメッシュの順)。
pub fn drawable_states(instance: &ModelInstance) -> Vec<DrawableState> {
    instance
        .drawables()
        .iter()
        .map(|d| DrawableState {
            opacity: d.opacity,
            draw_order: d.draw_order,
            multiply_color: [d.multiply_color.r, d.multiply_color.g, d.multiply_color.b],
            screen_color: [d.screen_color.r, d.screen_color.g, d.screen_color.b],
            mask_strength: d.mask_strength,
            visible: d.visible,
        })
        .collect()
}

/// 乗算済みアルファの画素 (`bgra` なら BGRA の順) を、ストレートアルファの RGBA にする
/// (`ugoe render` の PNG と同じ丸め。`ugoe_headless::image::unpremultiply`)。
pub fn to_straight_rgba(pixels: &mut [u8], bgra: bool) {
    for p in pixels.as_chunks_mut::<4>().0 {
        if bgra {
            p.swap(0, 2);
        }
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = (u32::from(*c) * 255 + a / 2)
                .checked_div(a)
                .map_or(0, |v| v.min(255) as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;
    use ugoe::runtime::CompiledModel;

    use super::*;

    #[test]
    fn straight_rgba_undoes_premultiplication() {
        let mut pixels = vec![100, 50, 0, 128, 10, 20, 30, 255, 7, 7, 7, 0];
        to_straight_rgba(&mut pixels, false);
        assert_eq!(pixels, [199, 100, 0, 128, 10, 20, 30, 255, 0, 0, 0, 0]);
        let mut bgra = vec![0, 50, 100, 128];
        to_straight_rgba(&mut bgra, true);
        assert_eq!(bgra, [199, 100, 0, 128]);
    }

    #[test]
    fn states_follow_the_meshes() {
        let mut inst =
            ModelInstance::new(CompiledModel::compile(fixtures::nested_masks()).unwrap());
        inst.update();
        let states = drawable_states(&inst);
        assert_eq!(states.len(), inst.drawables().len());
        let d = &inst.drawables()[1];
        assert_eq!(states[1].opacity, d.opacity);
        assert_eq!(states[1].draw_order, d.draw_order);
        assert_eq!(states[1].visible, d.visible);
        let json = serde_json::to_value(&states[0]).unwrap();
        assert!(json["multiply_color"].is_array());
        assert!(json["mask_strength"].is_number());
    }
}
