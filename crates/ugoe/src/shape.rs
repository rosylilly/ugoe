//! 見た目の変化 ([`crate::Puppet::changed`]。`docs/spec/evaluation.md#見た目の変化`、T1402)。
//!
//! 最後に「変わった」と報告した評価の出力を控え、`update` のたびに今の出力と許容誤差付きで比べる。
//! 控えと比べるので、1 フレームでは許容誤差を下回る変化 (ゆっくりしたモーション) も、積もって超えれば報告する。

use alloc::vec::Vec;
use ugoe_runtime::{DrawableState, ModelInstance};

/// 頂点位置の許容誤差の、キャンバスの長い辺に対する割合。
///
/// 表示はキャンバスを画面に収める (ビューの倍率 1) ので、画面の長い辺が 4K (3840 画素) でもずれは 0.04 画素で、
/// 縁の画素の被覆率を 4% しか変えない。キャンバスの大きさが 2000 前後のとき f32 の座標の刻み (約 1.2e-4) の 100 倍あり、
/// 収まりかけた物理の端数の揺れを拾わない。
pub(crate) const POSITION_TOLERANCE: f32 = 1e-5;

/// 不透明度・乗算色・スクリーン色・マスクの効き具合の許容誤差 (絶対値)。
///
/// 8 ビットの出力の 1 段 (1/255 ≈ 0.0039) の 4 分の 1 で、下回る変化は出力を最大 1 段しか動かさない。
pub(crate) const VALUE_TOLERANCE: f32 = 1e-3;

/// 最後に「変わった」と報告した出力の控え。
#[derive(Debug)]
pub(crate) struct ShapeWatch {
    /// 直前の `update` で見た変化 ([`crate::Puppet::changed`])。
    changed: bool,
    /// 最後に見たインスタンスの `(id, update_count)`。まだ見ていなければ `None`。
    seen: Option<(u64, u64)>,
    /// 控えを取ったインスタンスの `id`。
    shown_id: Option<u64>,
    /// 最後に「変わった」と報告した出力。
    drawables: Vec<DrawableState>,
    render_order: Vec<u32>,
}

impl ShapeWatch {
    pub(crate) fn new() -> Self {
        Self {
            changed: true,
            seen: None,
            shown_id: None,
            drawables: Vec::new(),
            render_order: Vec::new(),
        }
    }

    pub(crate) fn changed(&self) -> bool {
        self.changed
    }

    /// 評価した直後の `instance` を見て、控えから見た目が変わったかを決める。変わったら控えを取り直す。
    pub(crate) fn observe(&mut self, instance: &ModelInstance) {
        let id = instance.id();
        let count = instance.update_count();
        // 直前に見たちょうど 1 つ後の出力なら、ランタイムの変更フラグ (ビット列の比較) が使える。
        // 控えと直前の出力の差は許容誤差の内なので、何も変わっていなければ控えとの差も許容誤差の内。
        let consecutive = self.seen == Some((id, count.wrapping_sub(1)));
        self.seen = Some((id, count));
        if self.shown_id != Some(id) {
            self.take(instance);
            return;
        }
        if consecutive
            && !instance.render_order_changed()
            && instance.changes().iter().all(|c| c.is_empty())
        {
            self.changed = false;
            return;
        }
        if self.differs(instance) {
            self.take(instance);
        } else {
            self.changed = false;
        }
    }

    /// 控えを今の出力にし、変わったとする。バッファは長さが同じなら使い回す。
    fn take(&mut self, instance: &ModelInstance) {
        self.changed = true;
        self.shown_id = Some(instance.id());
        copy_drawables(&mut self.drawables, instance.drawables());
        self.render_order.clear();
        self.render_order.extend_from_slice(instance.render_order());
    }

    /// 控えと今の出力が、許容誤差を超えて違うか。
    fn differs(&self, instance: &ModelInstance) -> bool {
        let drawables = instance.drawables();
        if self.drawables.len() != drawables.len() || self.render_order != instance.render_order() {
            return true;
        }
        let canvas = &instance.compiled().model().meta.canvas;
        let side = canvas.width.max(canvas.height);
        // キャンバスの大きさが正の有限でなければ (検証していないモデル)、許容誤差を 0 (ビット列の比較) にする。
        let position = if side.is_finite() && side > 0.0 {
            side * POSITION_TOLERANCE
        } else {
            0.0
        };
        self.drawables
            .iter()
            .zip(drawables)
            .any(|(shown, now)| drawable_differs(shown, now, position))
    }
}

/// 2 つの評価の結果が、許容誤差を超えて違うか (`blend` と `draw_order` は見ない。描画順は `render_order` で比べる)。
fn drawable_differs(shown: &DrawableState, now: &DrawableState, position: f32) -> bool {
    let value = |a: f32, b: f32| differs(a, b, VALUE_TOLERANCE);
    shown.visible != now.visible
        || value(shown.opacity, now.opacity)
        || value(shown.mask_strength, now.mask_strength)
        || value(shown.multiply_color.r, now.multiply_color.r)
        || value(shown.multiply_color.g, now.multiply_color.g)
        || value(shown.multiply_color.b, now.multiply_color.b)
        || value(shown.screen_color.r, now.screen_color.r)
        || value(shown.screen_color.g, now.screen_color.g)
        || value(shown.screen_color.b, now.screen_color.b)
        || shown.positions.len() != now.positions.len()
        || shown
            .positions
            .iter()
            .zip(&now.positions)
            .any(|(a, b)| differs(a.x, b.x, position) || differs(a.y, b.y, position))
}

/// `a` と `b` の差が `tolerance` を超えるか。同じビット列なら違わない (NaN が続いても変わらない)。
/// 片方だけ NaN か、無限大どうしでビット列が違えば違う。
fn differs(a: f32, b: f32, tolerance: f32) -> bool {
    use core::cmp::Ordering::{Equal, Less};
    // 差が NaN (片方だけ NaN) のときも違うとする。
    a.to_bits() != b.to_bits()
        && !matches!((a - b).abs().partial_cmp(&tolerance), Some(Less | Equal))
}

/// `src` を `dst` に写す。要素のバッファ (頂点位置) は長さが同じなら使い回す (毎フレームの確保をしない)。
fn copy_drawables(dst: &mut Vec<DrawableState>, src: &[DrawableState]) {
    dst.truncate(src.len());
    for (dst, src) in dst.iter_mut().zip(src) {
        // 派生した `Clone::clone_from` は作り直すので、頂点位置は手で写す。
        dst.positions.clear();
        dst.positions.extend_from_slice(&src.positions);
        dst.opacity = src.opacity;
        dst.draw_order = src.draw_order;
        dst.multiply_color = src.multiply_color;
        dst.screen_color = src.screen_color;
        dst.blend = src.blend;
        dst.mask_strength = src.mask_strength;
        dst.visible = src.visible;
    }
    if let Some(rest) = src.get(dst.len()..) {
        dst.extend_from_slice(rest);
    }
}

#[cfg(test)]
mod tests {
    use crate::Puppet;
    use ugoe_model::{fixtures, standard};

    const DT: f32 = 1.0 / 60.0;

    fn set(puppet: &mut Puppet, id: &str, value: f32) {
        let params = puppet.instance_mut().parameters_mut();
        let index = params.index_of(id).unwrap();
        params.set(index, value);
    }

    #[test]
    fn a_still_model_changes_only_on_the_first_update() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        assert!(puppet.changed());
        puppet.update(DT);
        assert!(puppet.changed());
        for _ in 0..10 {
            puppet.update(DT);
            assert!(!puppet.changed());
        }
    }

    #[test]
    fn a_parameter_change_is_reported_once() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        puppet.update(DT);
        set(&mut puppet, "X", 0.5);
        puppet.update(DT);
        assert!(puppet.changed());
        puppet.update(DT);
        assert!(!puppet.changed());
    }

    #[test]
    fn small_changes_are_ignored_until_they_add_up() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        puppet.update(DT);
        // `X` の 1e-5 は 0.00064 画素 (キャンバス 256 の許容誤差は 0.00256 画素)。
        let mut x = 0.0;
        let mut reported = 0;
        for _ in 0..10 {
            x += 1e-5;
            set(&mut puppet, "X", x);
            puppet.update(DT);
            if puppet.changed() {
                reported += 1;
                // 控えとの差が許容誤差を超えたときだけ報告する (5 回目で 0.0032 画素)。
                assert!(x > 3.9e-5);
            }
        }
        assert!(reported >= 1, "積もった変化を報告しない");
        assert!(reported <= 3, "許容誤差を下回る変化を報告した: {reported}");
    }

    #[test]
    fn the_puppet_redraws_while_physics_swings_and_stops_when_it_settles() {
        let mut puppet = Puppet::new(fixtures::dynamic_hair())
            .unwrap()
            .with_physics(fixtures::dynamic_hair_settings());
        puppet.update(DT);
        for _ in 0..600 {
            puppet.update(DT);
        }
        assert!(!puppet.changed(), "止まった物理で描き続ける");

        // 体を振って戻すと、揺れている間は変わり続ける。
        set(&mut puppet, standard::ANGLE_X, 30.0);
        puppet.update(DT);
        assert!(puppet.changed());
        set(&mut puppet, standard::ANGLE_X, 0.0);
        let mut swinging = 0;
        let mut last_change = 0;
        let mut evaluated_while_still = false;
        for frame in 1..=1200 {
            puppet.update(DT);
            if puppet.changed() {
                swinging += 1;
                last_change = frame;
            } else if instance_moved(&puppet) {
                // 描かない間も物理の時間は進み、出力は (許容誤差の内で) 動いている。
                evaluated_while_still = true;
            }
        }
        assert!(swinging >= 10, "揺れを描かない: {swinging}");
        assert!(last_change < 1200, "収まった物理で描き続ける");
        assert!(
            evaluated_while_still,
            "許容誤差の内の揺れが無いと、閾値を確かめられない"
        );

        // 止まった後でも、動き出したフレームですぐ変わる。
        set(&mut puppet, standard::ANGLE_X, -30.0);
        puppet.update(DT);
        assert!(puppet.changed());
    }

    fn instance_moved(puppet: &Puppet) -> bool {
        puppet.instance().changes().iter().any(|c| !c.is_empty())
    }

    #[test]
    fn host_evaluation_between_updates_is_compared_with_the_last_report() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        puppet.update(DT);
        // ホストが `update` の外で評価しても (update_count が飛ぶ)、控えと比べて報告する。
        set(&mut puppet, "X", 0.5);
        puppet.instance_mut().update();
        puppet.update(DT);
        assert!(puppet.changed());
        puppet.instance_mut().update();
        puppet.update(DT);
        assert!(!puppet.changed());
    }

    #[test]
    fn swapping_the_model_is_a_change() {
        let mut puppet = Puppet::new(fixtures::dress_up_base()).unwrap();
        puppet.update(DT);
        puppet.update(DT);
        assert!(!puppet.changed());
        puppet
            .attach_module(alloc::sync::Arc::new(fixtures::dress_module()))
            .unwrap();
        puppet.update(DT);
        assert!(puppet.changed());
    }

    #[test]
    fn follow_reports_changes_too() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        puppet.update(DT);
        let mut values = puppet.instance().parameters().as_slice().to_vec();
        puppet.follow(&values, None, &[], DT).unwrap();
        assert!(!puppet.changed());
        values[0] = 0.5;
        puppet.follow(&values, None, &[], DT).unwrap();
        assert!(puppet.changed());
    }

    #[test]
    fn nan_positions_do_not_report_every_frame() {
        assert!(!super::differs(f32::NAN, f32::NAN, 1.0));
        assert!(super::differs(f32::NAN, 0.0, 1.0));
        assert!(super::differs(f32::INFINITY, f32::NEG_INFINITY, 1.0));
        assert!(!super::differs(1.0, 1.0005, super::VALUE_TOLERANCE));
        assert!(super::differs(1.0, 1.002, super::VALUE_TOLERANCE));
    }
}
