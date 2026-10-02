//! 受け手の口: 届いたパラメータの値を基準値として書き、物理より後の段だけを進める
//! (`docs/spec/motion.md#受け手として進める`、ADR 0028 の 3.)。
//!
//! スタジオの指揮役は [`Puppet::update`] の後に、物理の段に入る直前の値 ([`Puppet::values_before_physics`]) と
//! リップシンクの入力 ([`Puppet::lip_sync_applied`]) を配り、受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) は
//! それを [`Puppet::follow`] で順に適用する。段の順は `update` と同じなので、同じ列なら、どの受け手も物理を含めて
//! 1 つの `Puppet` と同じ形になる。

use ugoe_runtime::ParamIndex;

use crate::Puppet;

/// リップシンクの段の入力: 音量と、エフェクトグループ `LipSync` の強さ (`docs/spec/motion.md#リップシンク`)。
///
/// 対象の値 `v` は `lerp(v, level, strength)` になる。指揮役の [`Puppet::lip_sync_applied`] をそのまま受け手の
/// [`Puppet::follow`] に渡す。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LipSyncInput {
    /// 音量 (0〜1)。
    pub level: f32,
    /// エフェクトグループ `LipSync` の強さ (モーションのカーブ。無ければ 1)。
    pub strength: f32,
}

/// パラメータの値の並びを受け付けられない。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParameterValuesError {
    /// 値の数がパラメータの数と違う。
    #[error("expected {expected} parameter values, got {actual}")]
    Count { expected: usize, actual: usize },
    /// `index` 番目の値が NaN。
    #[error("parameter value {index} is NaN")]
    NaN { index: usize },
    /// リップシンクの音量か強さが NaN。
    #[error("the lip sync level or strength is NaN")]
    LipSyncNaN,
}

impl Puppet {
    /// パラメータの値 (モデルの定義順。`instance().parameters()` と同じ並び) を、すべて **基準値として** 書く。
    ///
    /// 値は値域にクランプする。ホストが `parameters_mut()` で書き換えた値は「前のフレームの出力とビット列で違うとき」
    /// だけ基準値になるが (`docs/spec/motion.md#フレーム内の適用順` の 1.)、この口は前のフレームの出力と同じ値でも
    /// 基準値にする。次の [`Puppet::update`] / [`Puppet::follow`] から、この値の上に各段が重なる。
    ///
    /// 数が違うか NaN を含むときは、何も変えずにエラーを返す。
    pub fn set_base_values(&mut self, values: &[f32]) -> Result<(), ParameterValuesError> {
        let params = self.instance.parameters_mut();
        if values.len() != params.len() {
            return Err(ParameterValuesError::Count {
                expected: params.len(),
                actual: values.len(),
            });
        }
        if let Some(index) = values.iter().position(|v| v.is_nan()) {
            return Err(ParameterValuesError::NaN { index });
        }
        for (i, &value) in values.iter().enumerate() {
            params.set(ParamIndex(i as u32), value);
        }
        self.base.copy_from_slice(params.as_slice());
        Ok(())
    }

    /// 直前の [`Puppet::update`] で、物理の段 (8.) に入る直前のパラメータ値 (定義順)。指揮役が受け手へ配る値。
    ///
    /// 物理の段を止めていても同じ位置の値を返す。`update` の前は作ったときの値。
    pub fn values_before_physics(&self) -> &[f32] {
        &self.before_physics
    }

    /// 直前の [`Puppet::update`] でリップシンクの段が当てた音量と強さ。段を止めていたか、音量が無ければ `None`。
    /// 指揮役が受け手へ配る (`values_before_physics` と組で使う)。
    pub fn lip_sync_applied(&self) -> Option<LipSyncInput> {
        self.lip_sync_applied
    }

    /// 受け手として 1 フレーム進める: `values` を基準値として書き ([`Puppet::set_base_values`])、
    /// 物理 → リップシンク → ポーズ → 評価の順に `dt` 秒進める (`Puppet::update` の 8.〜11. と同じ順。
    /// `docs/spec/motion.md#受け手として進める`)。
    ///
    /// `values` は指揮役の [`Puppet::values_before_physics`]、`lip_sync` は [`Puppet::lip_sync_applied`]。
    /// リップシンクはこの `Puppet` のモデルの設定 (対象のパラメータ) で、渡した音量と強さを当てる
    /// (この `Puppet` の [`Puppet::set_lip_sync_level`] の音量は使わず、変えもしない)。
    ///
    /// モーション・トラッキング・表情・目パチ・呼吸の段は通さない (再生の状態も時刻も進まない)。
    /// それらは値を作った側 (指揮役) で済んでいるため。物理・リップシンク・ポーズは [`Puppet::set_toggles`] に従う。
    /// パーツの不透明度はポーズだけが書く (モーションのパーツの不透明度のカーブは使わない)。
    ///
    /// 同じ `values`・`lip_sync`・`dt` の列を与えた `Puppet` は、物理を含めて同じ形になる (決定的)。
    /// 数が違うか、値かリップシンクの入力が NaN を含むときは、何も変えずにエラーを返す (時間も進めない)。
    pub fn follow(
        &mut self,
        values: &[f32],
        lip_sync: Option<LipSyncInput>,
        dt: f32,
    ) -> Result<(), ParameterValuesError> {
        if lip_sync.is_some_and(|l| l.level.is_nan() || l.strength.is_nan()) {
            return Err(ParameterValuesError::LipSyncNaN);
        }
        self.set_base_values(values)?;
        self.before_physics
            .copy_from_slice(self.instance.parameters().as_slice());
        // 8. 物理 (動的物理を使う設定では、ここで 1 段目の評価を行う)
        if let Some(physics) = self.physics.as_mut().filter(|_| self.toggles.physics) {
            physics.update_instance(&mut self.instance, dt);
        }
        // 9. リップシンク (自分の音量は保ち、渡された音量で当てる)
        self.lip_sync_applied = None;
        if let Some(input) = lip_sync.filter(|_| self.toggles.lip_sync) {
            let own = self.lip_sync.level();
            self.lip_sync.set_level(Some(input.level));
            self.lip_sync
                .apply(self.instance.parameters_mut(), input.strength);
            self.lip_sync_applied = self
                .lip_sync
                .level()
                .map(|level| LipSyncInput { level, ..input });
            self.lip_sync.set_level(own);
        }
        self.output
            .copy_from_slice(self.instance.parameters().as_slice());
        // 10. パーツの不透明度 (ポーズだけ)
        self.apply_part_opacities(dt, false);
        // 11. 評価
        self.instance.update();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::effects::{Breath, Effects, LIP_SYNC_GROUP, LipSync};
    use ugoe_model::expression::{Expression, ExpressionOp, ExpressionParameter};
    use ugoe_model::motion::{Curve, CurveTarget, Motion, Point, Segment};
    use ugoe_model::{fixtures, standard};
    use ugoe_motion::{Priority, TrackingInput};

    use super::*;
    use crate::Toggles;

    const DT: f32 = 1.0 / 60.0;

    fn param(puppet: &Puppet, id: &str) -> f32 {
        let p = puppet.instance().parameters();
        p.get(p.index_of(id).unwrap())
    }

    fn values(puppet: &Puppet) -> Vec<f32> {
        puppet.instance().parameters().as_slice().to_vec()
    }

    /// 比べる状態: パラメータの値 (物理の出力を含む) と頂点の座標。
    fn state(puppet: &Puppet) -> Vec<f32> {
        let positions = puppet
            .instance()
            .drawables()
            .iter()
            .flat_map(|d| d.positions.iter().flat_map(|p| [p.x, p.y]));
        values(puppet).into_iter().chain(positions).collect()
    }

    /// 頭を振るモーション (物理の入力を動かす)。
    fn swing(id: &str) -> Motion {
        let p = |t: f32, v: f32| Point { t, v };
        Motion {
            name: "swing".into(),
            duration: 1.0,
            looping: true,
            fade_in: 0.0,
            fade_out: 0.0,
            curves: vec![Curve {
                target: CurveTarget::Parameter(id.into()),
                segments: vec![
                    Segment::Start { p: p(0.0, -30.0) },
                    Segment::Linear { to: p(0.5, 30.0) },
                    Segment::Linear { to: p(1.0, -30.0) },
                ],
                fade_in: None,
                fade_out: None,
            }],
            events: vec![],
        }
    }

    /// 指揮役 (物理を切る) の物理の前の値とリップシンクの入力を受け手が `follow` で適用すると、物理を有効にした
    /// 1 つの `Puppet` と同じ形になる (ADR 0028 の「10. 一致の検査」の 2 つ目)。静的な物理と動的物理の両方で、
    /// リップシンクの対象を物理の入力 (`AngleX`) にして確かめる (T0134)。
    #[test]
    fn conductor_and_follower_match_a_single_puppet_with_physics() {
        let cases = [
            (fixtures::physics_hair(), fixtures::physics_hair_settings()),
            (fixtures::dynamic_hair(), fixtures::dynamic_hair_settings()),
        ];
        let input = standard::ANGLE_X;
        for (model, settings) in cases {
            let make = |model: &ugoe_model::Model| {
                let mut puppet = Puppet::new(model.clone())
                    .unwrap()
                    .with_physics(settings.clone());
                // 呼吸を基準値の上に重ね、リップシンクは物理の入力に当てる (どちらも指揮役で入力を作る)。
                puppet.set_effects(Effects {
                    breath: Breath {
                        parameters: vec![ugoe_model::effects::BreathParameter {
                            parameter: input.into(),
                            offset: 0.0,
                            peak: 5.0,
                            cycle: 1.5,
                            weight: 1.0,
                        }],
                    },
                    lip_sync: LipSync {
                        parameters: vec![input.into()],
                    },
                    ..Effects::default()
                });
                puppet
            };
            let mut single = make(&model);
            let mut conductor = make(&model);
            conductor.set_toggles(Toggles {
                physics: false,
                ..Toggles::default()
            });
            // 受け手は物理とリップシンクの対象だけを使う (エフェクトは指揮役で済ませる)。
            let mut follower = make(&model);
            // T0133 の形 (リップシンクの後の値を基準値にする) の受け手。リップシンクが物理の入力に入ると合わない。
            let mut late = make(&model);
            for puppet in [&mut single, &mut conductor] {
                let mut motion = swing(input);
                // リップシンクの強さを半分にし、振りとリップシンクの両方が物理の入力に残るようにする。
                motion.curves.push(Curve {
                    target: CurveTarget::EffectGroup(LIP_SYNC_GROUP.into()),
                    segments: vec![Segment::Start {
                        p: Point { t: 0.0, v: 0.5 },
                    }],
                    fade_in: None,
                    fade_out: None,
                });
                assert!(puppet.start_motion(motion, Priority::Normal));
            }
            let mut late_differs = false;
            for frame in 0..180 {
                // 揺れの大きさが変わるよう、dt と音量を揺らす。
                let dt = if frame % 3 == 0 { DT * 2.0 } else { DT };
                let level = (frame % 7) as f32 / 6.0;
                for puppet in [&mut single, &mut conductor] {
                    puppet.set_lip_sync_level(Some(level));
                }
                single.update(dt);
                conductor.update(dt);
                let lip_sync = conductor.lip_sync_applied();
                assert_eq!(
                    lip_sync,
                    Some(LipSyncInput {
                        level,
                        strength: 0.5
                    })
                );
                follower
                    .follow(conductor.values_before_physics(), lip_sync, dt)
                    .unwrap();
                late.follow(&values(&conductor), None, dt).unwrap();
                let (a, b) = (state(&single), state(&follower));
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(&b) {
                    assert!((a - b).abs() <= 1e-3, "frame {frame}: {a} != {b}");
                }
                late_differs |= state(&single)
                    .iter()
                    .zip(state(&late))
                    .any(|(a, b)| (a - b).abs() > 1e-3);
            }
            assert!(late_differs);
            // 物理が実際に揺れている (受け手の形が物理なしの形と違う)。
            let mut still = make(&model);
            still.set_toggles(Toggles {
                physics: false,
                ..Toggles::default()
            });
            still
                .follow(
                    conductor.values_before_physics(),
                    conductor.lip_sync_applied(),
                    DT,
                )
                .unwrap();
            assert_ne!(state(&still), state(&follower));
        }
    }

    /// 受け手のリップシンクは渡した音量と強さで当て、自分の音量は使わず、変えもしない。段を止めれば当てない。
    #[test]
    fn follow_applies_the_given_lip_sync() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        puppet.set_lip_sync_level(Some(0.9));
        let sent = values(&puppet);
        let input = LipSyncInput {
            level: 0.6,
            strength: 0.5,
        };
        puppet.follow(&sent, Some(input), DT).unwrap();
        assert!((param(&puppet, standard::MOUTH_OPEN_Y) - 0.3).abs() < 1e-6);
        assert_eq!(puppet.lip_sync_applied(), Some(input));
        assert_eq!(puppet.lip_sync.level(), Some(0.9));
        // 前の値に重ならない (基準値は送った値)。
        puppet.follow(&sent, None, DT).unwrap();
        assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
        assert_eq!(puppet.lip_sync_applied(), None);
        puppet.set_toggles(Toggles {
            lip_sync: false,
            ..Toggles::default()
        });
        puppet.follow(&sent, Some(input), DT).unwrap();
        assert_eq!(param(&puppet, standard::MOUTH_OPEN_Y), 0.0);
        // NaN の入力は何も変えない。
        let nan = LipSyncInput {
            level: f32::NAN,
            strength: 1.0,
        };
        assert_eq!(
            puppet.follow(&sent, Some(nan), DT),
            Err(ParameterValuesError::LipSyncNaN)
        );
    }

    /// `update` は物理の段の直前の値を覚える (物理の出力やリップシンクの前の値)。
    #[test]
    fn update_records_the_values_before_physics() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        puppet.set_effects(Effects {
            lip_sync: LipSync::standard(),
            ..Effects::default()
        });
        puppet.set_lip_sync_level(Some(0.7));
        puppet.update(DT);
        let mouth = puppet
            .instance()
            .parameters()
            .index_of(standard::MOUTH_OPEN_Y)
            .unwrap();
        assert_eq!(puppet.values_before_physics()[mouth.0 as usize], 0.0);
        assert!((param(&puppet, standard::MOUTH_OPEN_Y) - 0.7).abs() < 1e-6);
        assert_eq!(
            puppet.lip_sync_applied(),
            Some(LipSyncInput {
                level: 0.7,
                strength: 1.0
            })
        );
    }

    /// 受け手のモーション・表情・トラッキング・目パチ・呼吸・リップシンクは値に触らない。
    #[test]
    fn follow_skips_the_puppets_own_layers() {
        let mut puppet = Puppet::new(fixtures::avatar_profile()).unwrap();
        puppet.set_effects(Effects {
            breath: Breath::standard(),
            ..Effects::default()
        });
        puppet.start_motion(swing(standard::ANGLE_X), Priority::Force);
        puppet.set_expression(Expression {
            name: "half".into(),
            fade_in: 0.0,
            fade_out: 0.0,
            parameters: vec![ExpressionParameter {
                parameter: standard::EYE_L_OPEN.into(),
                value: 0.5,
                op: ExpressionOp::Multiply,
            }],
        });
        let mut input = TrackingInput::default();
        input.head.pitch = 10.0;
        puppet.set_tracking(Some(input));
        puppet.set_lip_sync_level(Some(0.7));

        let mut sent = values(&puppet);
        let angle_x = puppet
            .instance()
            .parameters()
            .index_of(standard::ANGLE_X)
            .unwrap();
        sent[angle_x.0 as usize] = 12.0;
        for _ in 0..30 {
            puppet.follow(&sent, None, DT).unwrap();
            assert_eq!(values(&puppet), sent);
        }
        // 自分の層は止まったまま (モーションの時刻も進まない)。
        assert!(puppet.motions().is_playing());
    }

    /// 前のフレームの出力とビット列で同じ値でも基準値になる (ホストの書き換えの検出に頼らない。ADR 0028 の「結果」)。
    #[test]
    fn base_values_are_written_even_when_they_equal_the_last_output() {
        let breathing = || {
            let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
            puppet.set_effects(Effects {
                breath: Breath::standard(),
                ..Effects::default()
            });
            for _ in 0..20 {
                puppet.update(DT);
            }
            puppet
        };
        let no_breath = Toggles {
            breath: false,
            ..Toggles::default()
        };
        // 呼吸で動いた値 (前のフレームの出力) を、そのまま基準値にしたい。
        let mut explicit = breathing();
        let output = values(&explicit);
        let breathed = param(&explicit, standard::ANGLE_X);
        assert_ne!(breathed, 0.0);

        // ホストの書き換えとして同じ値を書いても検出されず、呼吸を止めると基準値 (呼吸の前の 0) に戻る。
        let mut host = breathing();
        let params = host.instance_mut().parameters_mut();
        for (i, &v) in output.iter().enumerate() {
            params.set(ParamIndex(i as u32), v);
        }
        host.set_toggles(no_breath);
        host.update(DT);
        assert_eq!(param(&host, standard::ANGLE_X), 0.0);

        // 明示的に書けば基準値になる。
        explicit.set_base_values(&output).unwrap();
        explicit.set_toggles(no_breath);
        explicit.update(DT);
        assert_eq!(param(&explicit, standard::ANGLE_X), breathed);
    }

    #[test]
    fn wrong_values_change_nothing() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).unwrap();
        let before = values(&puppet);
        let count = puppet.instance().update_count();
        assert_eq!(
            puppet.follow(&[0.0], None, DT),
            Err(ParameterValuesError::Count {
                expected: before.len(),
                actual: 1
            })
        );
        let mut nan = before.clone();
        nan[1] = f32::NAN;
        assert_eq!(
            puppet.set_base_values(&nan),
            Err(ParameterValuesError::NaN { index: 1 })
        );
        assert_eq!(values(&puppet), before);
        assert_eq!(puppet.instance().update_count(), count);
    }

    /// 値は値域にクランプする (`parameters_mut().set` と同じ)。
    #[test]
    fn base_values_are_clamped() {
        let mut puppet = Puppet::new(fixtures::single_quad()).unwrap();
        let n = puppet.instance().parameters().len();
        puppet.set_base_values(&vec![1e6; n]).unwrap();
        let params = puppet.instance().parameters();
        for i in 0..n {
            let i = ParamIndex(i as u32);
            assert_eq!(params.get(i), params.max(i));
        }
    }
}
