//! `ugoe render --time/--motion/--expression`: ランタイム SDK の `Puppet` で時間を進めてから描く。
//!
//! 物理・モーション・表情・目パチ・呼吸・ポーズを `Puppet::update` と同じ順
//! (`docs/spec/motion.md#フレーム内の適用順`) で固定の刻みで進めるので、同じ引数からは常に同じ絵になる。
//! 手順は `docs/testing.md#モーションの途中の絵を確かめる`。

use std::path::Path;

use anyhow::{Context, bail};
use ugoe::motion::Priority;
use ugoe::{Puppet, Toggles};
use ugoe_model::Model;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;

/// バンドルに入っている物理・モーション・表情 (JSON とフィクスチャでは空)。
#[derive(Debug, Default)]
pub struct Animation {
    pub physics: Option<PhysicsSettings>,
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

/// 時間の進め方。
#[derive(Debug)]
pub struct Playback {
    /// 進める秒数。
    pub time: f32,
    /// 1 秒あたりの `update` の回数。
    pub fps: f32,
    pub motion: Option<Motion>,
    pub expression: Option<Expression>,
    pub physics: Option<PhysicsSettings>,
    /// 目パチと呼吸を動かすか。
    pub effects: bool,
}

/// `--time` を読む (0 以上の有限の秒数)。
pub fn parse_time(s: &str) -> Result<f32, String> {
    let t: f32 = s
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{s}`"))?;
    if !t.is_finite() || t < 0.0 {
        return Err(format!("time must be a finite number >= 0, got `{s}`"));
    }
    Ok(t)
}

/// `--fps` を読む (0 より大きい有限の値)。
pub fn parse_fps(s: &str) -> Result<f32, String> {
    let fps: f32 = s
        .trim()
        .parse()
        .map_err(|_| format!("invalid number `{s}`"))?;
    if !fps.is_finite() || fps <= 0.0 {
        return Err(format!("fps must be a finite number > 0, got `{s}`"));
    }
    Ok(fps)
}

/// ファイルのパスとして読むか (`.json` で終わるか、ファイルが存在する)。
fn is_path(arg: &str) -> bool {
    arg.ends_with(".json") || Path::new(arg).is_file()
}

fn read(path: &str) -> anyhow::Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {path}"))
}

/// `--motion`: バンドルのモーションの名前か、`.motion.json` のパス。
pub fn resolve_motion(arg: &str, motions: &[Motion]) -> anyhow::Result<Motion> {
    if let Some(m) = motions.iter().find(|m| m.name == arg) {
        return Ok(m.clone());
    }
    if is_path(arg) {
        return ugoe_format::motion_from_json(&read(arg)?)
            .with_context(|| format!("parsing motion {arg}"));
    }
    bail!(
        "no motion named `{arg}` (known: {})",
        names(motions.iter().map(|m| m.name.as_str()))
    )
}

/// `--expression`: バンドルの表情の名前か、`.expression.json` のパス。
pub fn resolve_expression(arg: &str, expressions: &[Expression]) -> anyhow::Result<Expression> {
    if let Some(e) = expressions.iter().find(|e| e.name == arg) {
        return Ok(e.clone());
    }
    if is_path(arg) {
        return ugoe_format::expression_from_json(&read(arg)?)
            .with_context(|| format!("parsing expression {arg}"));
    }
    bail!(
        "no expression named `{arg}` (known: {})",
        names(expressions.iter().map(|e| e.name.as_str()))
    )
}

/// `--physics`: 物理の設定の JSON のパス。
pub fn read_physics(path: &Path) -> anyhow::Result<PhysicsSettings> {
    let json =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    ugoe_format::physics_from_json(&json).with_context(|| format!("parsing {}", path.display()))
}

fn names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

/// `Puppet` を作り、`params` を書き込んで評価する (時間は進めない)。値域でクランプしたパラメータの警告文も返す。
///
/// 書き込んだ値は、この後の [`advance`] で基準値になる (モーションの対象は上書きされる)。
pub fn puppet(model: Model, params: &[(String, f32)]) -> anyhow::Result<(Puppet, Vec<String>)> {
    let mut puppet = Puppet::new(model).context("compiling model")?;
    let warnings = crate::render::apply_params(puppet.instance_mut(), params)?;
    Ok((puppet, warnings))
}

/// `playback` のとおりに `puppet` を進め、`update` の回数を返す。
///
/// `1 / fps` 秒の `update` を `floor(time * fps)` 回行い、余りがあればもう 1 回その長さで行う。
/// `time` が 0 なら `update(0)` を 1 回行う (物理は静止姿勢、モーションは 0 秒の値)。
pub fn advance(puppet: &mut Puppet, playback: Playback) -> anyhow::Result<u32> {
    if let Some(physics) = playback.physics {
        puppet.set_physics(Some(physics)).map_err(|errors| {
            let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
            anyhow::anyhow!("invalid physics settings: {}", errors.join("; "))
        })?;
    }
    puppet.set_toggles(Toggles {
        eye_blink: playback.effects,
        breath: playback.effects,
        ..Toggles::default()
    });
    if let Some(motion) = playback.motion {
        puppet.start_motion(motion, Priority::Force);
    }
    if let Some(expression) = playback.expression {
        puppet.set_expression(expression);
    }
    let step = 1.0 / playback.fps;
    // 丸め誤差で 1 回少なくならないように、わずかに切り上げてから切り捨てる。
    let whole = (playback.time * playback.fps + 1e-4).floor();
    let rest = (playback.time - whole * step).max(0.0);
    let mut updates = 0;
    for _ in 0..whole as u64 {
        puppet.update(step);
        updates += 1;
    }
    if rest > 1e-6 || updates == 0 {
        puppet.update(rest);
        updates += 1;
    }
    Ok(updates)
}

#[cfg(test)]
mod tests {
    use ugoe_model::motion::{Curve, CurveTarget, Point, Segment};
    use ugoe_model::{fixtures, standard};

    use super::*;

    fn playback(time: f32) -> Playback {
        Playback {
            time,
            fps: 60.0,
            motion: None,
            expression: None,
            physics: None,
            effects: false,
        }
    }

    fn play(
        model: Model,
        params: &[(String, f32)],
        playback: Playback,
    ) -> anyhow::Result<(Puppet, Vec<String>, u32)> {
        let (mut puppet, warnings) = puppet(model, params)?;
        let updates = advance(&mut puppet, playback)?;
        Ok((puppet, warnings, updates))
    }

    fn param(puppet: &Puppet, id: &str) -> f32 {
        let p = puppet.instance().parameters();
        p.get(p.index_of(id).unwrap())
    }

    /// `X` を 1 秒で 0 → 1 に動かすモーション。
    fn ramp() -> Motion {
        Motion {
            name: "Ramp".into(),
            duration: 1.0,
            looping: false,
            fade_in: 0.0,
            fade_out: 0.0,
            curves: vec![Curve {
                target: CurveTarget::Parameter("X".into()),
                segments: vec![
                    Segment::Start {
                        p: Point { t: 0.0, v: 0.0 },
                    },
                    Segment::Linear {
                        to: Point { t: 1.0, v: 1.0 },
                    },
                ],
                fade_in: None,
                fade_out: None,
            }],
            events: vec![],
        }
    }

    #[test]
    fn plays_a_motion_to_the_given_time() {
        for (time, expected, updates) in [(0.0, 0.0, 1), (0.25, 0.25, 15), (0.51, 0.51, 31)] {
            let (puppet, warnings, n) = play(
                fixtures::single_quad(),
                &[],
                Playback {
                    motion: Some(ramp()),
                    ..playback(time)
                },
            )
            .unwrap();
            assert!(warnings.is_empty());
            assert_eq!(n, updates, "{time}");
            assert!((param(&puppet, "X") - expected).abs() < 1e-4, "{time}");
        }
    }

    #[test]
    fn steps_physics_and_effects() {
        let model = fixtures::dynamic_hair();
        let (puppet, _, _) = play(
            model.clone(),
            &[(standard::ANGLE_X.into(), 30.0)],
            Playback {
                physics: Some(fixtures::dynamic_hair_settings()),
                ..playback(0.0)
            },
        )
        .unwrap();
        assert!(puppet.physics().unwrap().is_dynamic());
        assert_eq!(param(&puppet, standard::ANGLE_X), 30.0);

        // 呼吸は `--no-effects` で止まる。
        let breathing = |effects: bool| {
            let (puppet, _, _) = play(
                fixtures::pose_and_effects(),
                &[],
                Playback {
                    effects,
                    ..playback(1.0)
                },
            )
            .unwrap();
            param(&puppet, standard::BREATH)
        };
        assert!(breathing(true) > 0.0);
        assert_eq!(breathing(false), 0.0);
    }

    #[test]
    fn rejects_physics_that_does_not_match_the_model() {
        let err = play(
            fixtures::single_quad(),
            &[],
            Playback {
                physics: Some(fixtures::dynamic_hair_settings()),
                ..playback(0.0)
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("invalid physics"), "{err}");
    }

    #[test]
    fn resolves_motions_by_name() {
        let motions = [ramp()];
        assert_eq!(resolve_motion("Ramp", &motions).unwrap(), ramp());
        let err = resolve_motion("Idle", &motions).unwrap_err().to_string();
        assert!(err.contains("known: Ramp"), "{err}");
        let err = resolve_expression("Smile", &[]).unwrap_err().to_string();
        assert!(err.contains("known: none"), "{err}");
    }

    #[test]
    fn parses_time_and_fps() {
        assert_eq!(parse_time("1.5"), Ok(1.5));
        assert!(parse_time("-1").is_err());
        assert!(parse_time("inf").is_err());
        assert_eq!(parse_fps("30"), Ok(30.0));
        assert!(parse_fps("0").is_err());
    }
}
