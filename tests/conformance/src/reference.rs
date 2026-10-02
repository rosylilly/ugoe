//! 参照の評価の結果を作る。
//!
//! バインディングと同じく `ugoe::Puppet` を作り (バンドルの物理を付け、モジュールは `set_modules` で付ける)、
//! パラメータを書き込んで `ModelInstance` を評価する。時間を進めるケースは `ugoe render --time` と同じ関数
//! (`ugoe_headless::animate::advance`) で進める。受け手として進めるケース (`Case::follow`) は `Puppet::follow` で進める。

use std::sync::Arc;

use ugoe::Puppet;
use ugoe_headless::animate;

use crate::frame::Frame;
use crate::plan::Case;
use crate::{Error, read};

/// `case` の参照の結果。
pub fn evaluate(case: &Case) -> Result<Frame, Error> {
    let puppet = puppet(case)?;
    Ok(Frame::from_instance(puppet.instance()))
}

/// 受け手として進めるケースの 1 フレーム分の入力 (`Puppet::follow` の引数)。
#[derive(Debug, Clone, PartialEq)]
pub struct FollowFrame {
    pub values: Vec<f32>,
    pub lip_sync: Option<ugoe::LipSyncInput>,
    pub part_opacities: Vec<ugoe::PartOpacityInput>,
}

/// 受け手として進めるケースの各フレームの入力: 物理を切った指揮役の `Puppet` を `case.steps` で進め、各 `update` の
/// 物理の段に入る直前の値・リップシンクの入力・モーションのパーツの不透明度のカーブを並べる
/// (ADR 0028 の 3.、`docs/spec/motion.md#受け手として進める`)。`case.motion` なら、指揮役は最初にそのモーションを再生する。
///
/// `effects` なら、指揮役は標準の目パチと呼吸 (`EyeBlink::standard()`・`Breath::standard()`) を使う (見本のモデルは目パチと呼吸の対象を
/// 持たないことがあるので、モデルの設定ではなく標準の設定で値を動かす)。`lip_sync` なら、指揮役はモデルのリップシンクの設定に
/// フレームごとに変わる音量 ([`lip_sync_level`]) を当てる。
pub fn conductor_frames(
    case: &Case,
    effects: bool,
    lip_sync: bool,
) -> Result<Vec<FollowFrame>, Error> {
    let mut frames = Vec::new();
    drive(case, effects, lip_sync, false, |conductor| {
        frames.push(FollowFrame {
            values: conductor.values_before_physics().to_vec(),
            lip_sync: conductor.lip_sync_applied(),
            part_opacities: conductor.part_opacities_applied().to_vec(),
        });
    })?;
    Ok(frames)
}

/// 指揮役が `i` 番目のフレームで当てるリップシンクの音量 (0〜1 を行き来する)。
pub fn lip_sync_level(i: usize) -> f32 {
    (i % 7) as f32 / 6.0
}

/// [`conductor_frames`] の指揮役を作って `case.steps` で進め、各 `update` の後に `each` を呼ぶ (物理は `physics` のとおり)。
fn drive(
    case: &Case,
    effects: bool,
    lip_sync: bool,
    physics: bool,
    mut each: impl FnMut(&Puppet),
) -> Result<Puppet, Error> {
    let mut conductor = loaded(case)?;
    if effects {
        let lip_sync = conductor
            .instance()
            .compiled()
            .model()
            .effects
            .lip_sync
            .clone();
        conductor.set_effects(ugoe::model::effects::Effects {
            eye_blink: ugoe::model::effects::EyeBlink::standard(),
            breath: ugoe::model::effects::Breath::standard(),
            lip_sync,
        });
    }
    conductor.set_toggles(ugoe::Toggles {
        physics,
        eye_blink: effects,
        breath: effects,
        ..ugoe::Toggles::default()
    });
    if let Some(name) = &case.motion {
        let motion = bundle_motion(case, name)?;
        conductor.start_motion(motion, ugoe::motion::Priority::Normal);
    }
    for (i, &dt) in case.steps.iter().enumerate() {
        if lip_sync {
            conductor.set_lip_sync_level(Some(lip_sync_level(i)));
        }
        conductor.update(dt);
        each(&conductor);
    }
    Ok(conductor)
}

/// 時間を進めるときの段 (目パチと呼吸は `effects` のとおり、ほかは動かす)。
fn toggles(case: &Case) -> ugoe::Toggles {
    ugoe::Toggles {
        eye_blink: case.effects,
        breath: case.effects,
        ..ugoe::Toggles::default()
    }
}

/// `case` のとおりに作って評価した `Puppet`。
pub fn puppet(case: &Case) -> Result<Puppet, Error> {
    let mut puppet = loaded(case)?;
    if !case.steps.is_empty() {
        // 計画の列は `advance` と同じ (`plan::steps`)。バインディングと同じく、列のとおりに進める。
        puppet.set_toggles(toggles(case));
        if case.follow.is_empty() {
            for &dt in &case.steps {
                puppet.update(dt);
            }
        } else {
            for (i, (&dt, values)) in case.steps.iter().zip(&case.follow).enumerate() {
                let lip_sync = case
                    .follow_lip_sync
                    .get(i)
                    .copied()
                    .flatten()
                    .map(|[level, strength]| ugoe::LipSyncInput { level, strength });
                let part_opacities: Vec<_> = case
                    .follow_part_opacities
                    .get(i)
                    .into_iter()
                    .flatten()
                    .map(|&(part, value, weight)| ugoe::PartOpacityInput {
                        part,
                        value,
                        weight,
                    })
                    .collect();
                puppet
                    .follow(values, lip_sync, &part_opacities, dt)
                    .map_err(|e| invalid(case, e))?;
            }
        }
    }
    Ok(puppet)
}

/// バンドル `case.model` のモーション `name`。
fn bundle_motion(case: &Case, name: &str) -> Result<ugoe::model::motion::Motion, Error> {
    let bundle = ugoe_format::read_bundle(&read(&case.model)?).map_err(|source| Error::Format {
        context: format!("reading {}", case.model.display()),
        source,
    })?;
    bundle
        .motions
        .into_iter()
        .find(|m| m.name == name)
        .ok_or_else(|| invalid(case, format!("the bundle has no motion `{name}`")))
}

/// モデルを読み、モジュールを付け、パラメータを書いた `Puppet` (時間は進めない)。
fn loaded(case: &Case) -> Result<Puppet, Error> {
    let bytes = read(&case.model)?;
    let context = || format!("reading {}", case.model.display());
    let mut puppet = if case.bundle {
        let bundle = ugoe_format::read_bundle(&bytes).map_err(|source| Error::Format {
            context: context(),
            source,
        })?;
        let mut puppet = Puppet::new(bundle.model).map_err(|e| invalid(case, e))?;
        if let Some(physics) = bundle.physics {
            puppet = puppet.with_physics(physics);
        }
        puppet
    } else {
        let json = String::from_utf8(bytes)
            .map_err(|_| Error::Invalid(format!("{} is not UTF-8", case.model.display())))?;
        let model = ugoe_format::model_from_json(&json).map_err(|source| Error::Format {
            context: context(),
            source,
        })?;
        Puppet::new(model).map_err(|e| invalid(case, e))?
    };
    if !case.modules.is_empty() {
        let modules = case
            .modules
            .iter()
            .map(|path| {
                let file =
                    ugoe_format::read_module(&read(path)?).map_err(|source| Error::Format {
                        context: format!("reading {}", path.display()),
                        source,
                    })?;
                Ok(Arc::new(file.module))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        puppet.set_modules(modules).map_err(|e| invalid(case, e))?;
    }
    let params: Vec<(String, f32)> = case
        .parameters
        .iter()
        .map(|(id, v)| (id.clone(), *v))
        .collect();
    animate::apply_params(puppet.instance_mut(), &params).map_err(|source| Error::Headless {
        context: format!("case `{}`", case.name),
        source,
    })?;
    Ok(puppet)
}

fn invalid(case: &Case, e: impl std::fmt::Display) -> Error {
    Error::Invalid(format!("case `{}`: {e}", case.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{DEFAULT_FPS, Plan};

    fn plan() -> Plan {
        crate::tests::committed_plan()
    }

    #[test]
    fn every_case_evaluates() {
        for case in plan().cases {
            let frame = evaluate(&case).unwrap();
            assert!(!frame.meshes.is_empty(), "{}", case.name);
            assert_eq!(
                frame.render_order.len(),
                frame.meshes.len(),
                "{}",
                case.name
            );
        }
    }

    /// 計画の `update` の列で進めた結果が、`ugoe render --time` (`animate::advance`) と同じになる。
    #[test]
    fn steps_match_advance() {
        let plan = plan();
        let case = plan
            .cases
            .iter()
            .find(|c| {
                !c.steps.is_empty() && c.modules.is_empty() && c.bundle && c.follow.is_empty()
            })
            .unwrap();
        let expected = evaluate(case).unwrap();
        let mut stepped = case.clone();
        stepped.steps.clear();
        let mut puppet = puppet(&stepped).unwrap();
        let playback = animate::Playback {
            time: case.time.unwrap(),
            fps: DEFAULT_FPS,
            motion: None,
            expression: None,
            physics: None,
            effects: case.effects,
        };
        let updates = animate::advance(&mut puppet, playback).unwrap();
        assert_eq!(updates as usize, case.steps.len());
        assert_eq!(Frame::from_instance(puppet.instance()), expected);
    }

    /// 受け手として進めたケース (T0133・T0134) は、物理を有効にした 1 つの `Puppet` で同じ列を進めた結果と許容誤差内で同じ
    /// (ADR 0028 の「10. 一致の検査」の 2 つ目)。リップシンクの対象を物理の入力にするケース (`follow-lip-sync`) も含む。
    #[test]
    fn follow_cases_match_a_single_puppet_with_physics() {
        let plan = plan();
        let cases: Vec<_> = plan.cases.iter().filter(|c| !c.follow.is_empty()).collect();
        assert!(cases.len() >= 4);
        assert!(cases.iter().any(|c| !c.follow_part_opacities.is_empty()));
        // 指揮役の目パチと呼吸は `effects` で決まるが、計画には残らないので、どの `follow` のケースも `effects` を使う約束にする。
        for case in cases {
            assert_eq!(case.follow.len(), case.steps.len());
            let followed = evaluate(case).unwrap();
            // 物理を有効にした 1 つの `Puppet` (指揮役と同じ目パチ・呼吸・リップシンク) で同じ列を進める。
            let lip_sync = case.follow_lip_sync.iter().any(Option::is_some);
            let single = drive(case, true, lip_sync, true, |_| {}).unwrap();
            let expected = Frame::from_instance(single.instance());
            let errors = crate::compare::frames(&expected, &followed);
            assert!(errors.is_empty(), "{}: {errors:?}", case.name);
            // パーツの不透明度のカーブを渡さない実行器は合わない (T0135)。
            if !case.follow_part_opacities.is_empty() {
                let mut without = case.clone();
                without.follow_part_opacities.clear();
                let without = evaluate(&without).unwrap();
                assert!(
                    !crate::compare::frames(&without, &followed).is_empty(),
                    "{}",
                    case.name
                );
            }
            // リップシンクの入力を渡さない実行器は合わない (物理の入力が変わる)。
            if lip_sync {
                let mut without = case.clone();
                without.follow_lip_sync.clear();
                let without = evaluate(&without).unwrap();
                assert!(
                    !crate::compare::frames(&without, &followed).is_empty(),
                    "{}",
                    case.name
                );
            }
            // `follow` を呼ばずに `update` で進める実行器は合わない (目パチと呼吸は指揮役の値にしか無い)。
            let mut updated = case.clone();
            updated.follow.clear();
            let updated = evaluate(&updated).unwrap();
            assert!(
                !crate::compare::frames(&updated, &followed).is_empty(),
                "{}",
                case.name
            );
        }
    }
}
