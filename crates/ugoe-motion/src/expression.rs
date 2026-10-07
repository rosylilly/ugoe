//! 表情の再生 (フェードと重ね合わせ)。仕様: `docs/spec/motion.md#表情`。

use alloc::vec::Vec;
use ugoe_model::expression::{Expression, ExpressionOp};
use ugoe_runtime::ParameterValues;

use crate::player::{advance_clock, fade_weight};
use crate::resolve::ResolvedParams;

/// 表情 1 つ分の再生状態。フェードは [`MotionPlayer`](crate::MotionPlayer) と同じ規則
/// (`fade_in` で入り、[`stop`](Self::stop) から `fade_out` 秒で抜ける)。
#[derive(Debug)]
pub struct ExpressionPlayer {
    expression: Expression,
    /// 各項目の対象パラメータの添字 (適用時に解決し、パラメータの並びが変わったら解決し直す)。
    param_indices: ResolvedParams,
    /// 経過時間。長時間使い続けても `dt` を足せるように `f64` で持つ。
    elapsed: f64,
    /// 終了する経過時間。`None` なら止めるまで続く。
    end: Option<f64>,
}

impl ExpressionPlayer {
    /// `expression` を使い始める (`fade_in` でフェードインし、止めるまで続く)。
    pub fn new(expression: Expression) -> Self {
        Self {
            expression,
            param_indices: ResolvedParams::default(),
            elapsed: 0.0,
            end: None,
        }
    }

    /// 使っている表情。
    pub fn expression(&self) -> &Expression {
        &self.expression
    }

    /// 止めた後、フェードアウトし切ったか。
    pub fn is_finished(&self) -> bool {
        self.end.is_some_and(|end| self.elapsed >= end)
    }

    /// 今の時点から `fade_out` 秒かけてフェードアウトして終わる。既に止めていれば早いほうの終了時刻を保つ。
    pub fn stop(&mut self) {
        let end = self.elapsed + f64::from(self.expression.fade_out.max(0.0));
        self.end = Some(self.end.map_or(end, |e| e.min(end)));
    }

    /// フェードの重み。
    pub fn weight(&self) -> f32 {
        fade_weight(
            self.expression.fade_in,
            self.expression.fade_out,
            self.elapsed,
            self.end,
        )
    }

    /// `dt` 秒進める。`dt` が 0 以下か有限値でない (NaN・無限大) ときは何もしない。
    pub fn advance(&mut self, dt: f32) {
        if !self.is_finished() {
            advance_clock(&mut self.elapsed, self.end, dt);
        }
    }

    /// 現在のパラメータ値に表情を重ねる。
    pub fn apply(&mut self, parameters: &mut ParameterValues) {
        if self.is_finished() {
            return;
        }
        let w = self.weight();
        let indices = self.param_indices.get(
            parameters,
            self.expression
                .parameters
                .iter()
                .map(|p| Some(p.parameter.as_str())),
        );
        for (p, index) in self.expression.parameters.iter().zip(indices) {
            let Some(index) = *index else { continue };
            let v = parameters.get(index);
            let value = match p.op {
                ExpressionOp::Add => v + p.value * w,
                ExpressionOp::Multiply => v * (1.0 + (p.value - 1.0) * w),
                ExpressionOp::Overwrite => v + (p.value - v) * w,
                ExpressionOp::Unknown => continue,
            };
            parameters.set(index, value);
        }
    }
}

/// 同時に使う表情の管理。開始した順に重ねる。
#[derive(Debug, Default)]
pub struct ExpressionManager {
    players: Vec<ExpressionPlayer>,
}

impl ExpressionManager {
    /// 表情を使っていないマネージャを作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 表情を `expression` だけにする。使用中の表情はフェードアウトさせ、`expression` をフェードインさせる。
    pub fn set(&mut self, expression: Expression) {
        self.stop_all();
        self.add(expression);
    }

    /// 使用中の表情の上に `expression` を重ねる。
    pub fn add(&mut self, expression: Expression) {
        self.players.push(ExpressionPlayer::new(expression));
    }

    /// 名前が `name` の表情をフェードアウトさせる。
    pub fn stop(&mut self, name: &str) {
        for p in &mut self.players {
            if p.expression.name == name {
                p.stop();
            }
        }
    }

    /// すべての表情をフェードアウトさせる。
    pub fn stop_all(&mut self) {
        for p in &mut self.players {
            p.stop();
        }
    }

    /// 使用中の表情 (フェードアウト中のものを含む)。開始した順。
    pub fn players(&self) -> &[ExpressionPlayer] {
        &self.players
    }

    /// `dt` 秒進め、開始した順にパラメータへ重ねる。終わった表情は取り除く。
    pub fn update(&mut self, parameters: &mut ParameterValues, dt: f32) {
        for p in &mut self.players {
            p.advance(dt);
            p.apply(parameters);
        }
        self.players.retain(|p| !p.is_finished());
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::expression::ExpressionParameter;
    use ugoe_model::fixtures;
    use ugoe_runtime::ParamIndex;

    use super::*;
    use crate::easing;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= EPS
    }

    fn expression(name: &str, op: ExpressionOp, value: f32, fade: f32) -> Expression {
        Expression {
            name: name.into(),
            fade_in: fade,
            fade_out: fade,
            parameters: vec![ExpressionParameter {
                parameter: "X".into(),
                value,
                op,
            }],
        }
    }

    /// `X` (値域 -1〜1) を `x` にしたパラメータ。
    fn params(x: f32) -> ParameterValues {
        let mut p = ParameterValues::from_model(&fixtures::single_quad());
        p.set(ParamIndex(0), x);
        p
    }

    fn apply_at(op: ExpressionOp, value: f32, x: f32, elapsed: f32) -> f32 {
        let mut player = ExpressionPlayer::new(expression("e", op, value, 1.0));
        player.advance(elapsed);
        let mut p = params(x);
        player.apply(&mut p);
        p.get(ParamIndex(0))
    }

    /// `docs/spec/motion.md#表情` の op の式。
    #[test]
    fn ops_follow_spec() {
        let w = easing(0.5);
        assert!(close(
            apply_at(ExpressionOp::Add, 0.4, 0.2, 0.5),
            0.2 + 0.4 * w
        ));
        assert!(close(
            apply_at(ExpressionOp::Multiply, 0.5, 0.8, 0.5),
            0.8 * (1.0 + (0.5 - 1.0) * w)
        ));
        assert!(close(
            apply_at(ExpressionOp::Overwrite, -0.6, 0.2, 0.5),
            0.2 + (-0.6 - 0.2) * w
        ));
        assert!(close(apply_at(ExpressionOp::Unknown, 1.0, 0.2, 0.5), 0.2));
        // フェードインし切れば w = 1。
        assert!(close(
            apply_at(ExpressionOp::Overwrite, -0.6, 0.2, 2.0),
            -0.6
        ));
    }

    /// 長時間使い続けた後でも、止めてから `fade_out` 秒で抜ける (経過時間の精度が落ちて止まらない)。
    #[test]
    fn stop_finishes_after_a_long_run() {
        let mut player = ExpressionPlayer::new(expression("e", ExpressionOp::Add, 1.0, 0.5));
        // 約 12 日。f32 の経過時間では 1/60 秒を足しても変わらなくなる。
        player.advance(1_048_576.0);
        player.advance(f32::NAN);
        player.advance(f32::INFINITY);
        assert!(close(player.weight(), 1.0));
        player.stop();
        player.advance(0.25);
        assert!(close(player.weight(), easing(0.5)), "{}", player.weight());
        for _ in 0..16 {
            player.advance(1.0 / 60.0);
        }
        assert!(player.is_finished());
    }

    /// T0325: 並びの違う `ParameterValues` を渡すと添字を解決し直し、同じ ID のパラメータに書く。
    #[test]
    fn resolves_parameters_again_when_layout_changes() {
        let mut player = ExpressionPlayer::new(expression("e", ExpressionOp::Add, 0.5, 0.0));
        let mut p = params(0.0);
        player.apply(&mut p);
        assert!(close(p.get(ParamIndex(0)), 0.5));

        // 先頭に `Y` を足した並び (`X` は 1 番目)。
        let mut model = fixtures::single_quad();
        let mut y = model.parameters[0].clone();
        y.id = "Y".into();
        model.parameters.insert(0, y);
        let mut shifted = ParameterValues::from_model(&model);
        player.apply(&mut shifted);
        assert!(close(shifted.get(shifted.index_of("X").unwrap()), 0.5));
        assert_eq!(shifted.get(shifted.index_of("Y").unwrap()), 0.0);
    }

    /// 複数の表情は開始した順に適用する。
    #[test]
    fn expressions_apply_in_start_order() {
        let mut m = ExpressionManager::new();
        m.add(expression("add", ExpressionOp::Add, 0.5, 0.0));
        m.add(expression("mul", ExpressionOp::Multiply, 0.5, 0.0));
        let mut p = params(0.1);
        m.update(&mut p, 0.1);
        // (0.1 + 0.5) * 0.5
        assert!(close(p.get(ParamIndex(0)), 0.3));
    }

    #[test]
    fn set_fades_out_previous_expression() {
        let mut m = ExpressionManager::new();
        m.set(expression("a", ExpressionOp::Overwrite, -1.0, 1.0));
        m.update(&mut params(0.0), 2.0);
        m.set(expression("b", ExpressionOp::Overwrite, 1.0, 1.0));
        let mut p = params(0.0);
        m.update(&mut p, 0.25);
        let a = -easing(0.75);
        assert!(close(p.get(ParamIndex(0)), a + (1.0 - a) * easing(0.25)));
        m.update(&mut params(0.0), 1.0);
        assert_eq!(m.players().len(), 1);
        assert_eq!(m.players()[0].expression().name, "b");
        m.stop("b");
        m.update(&mut params(0.0), 1.0);
        assert!(m.players().is_empty());
    }
}
