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

/// 受け手として進めるケースの各フレームの値: 物理を切った指揮役の `Puppet` を `case.steps` で進め、
/// 各 `update` の後のパラメータの値を並べる (ADR 0028 の 3.、`docs/spec/motion.md#受け手として進める`)。
/// `effects` なら、指揮役は標準の目パチと呼吸 (`EyeBlink::standard()`・`Breath::standard()`) を使う (見本のモデルは目パチと呼吸の対象を
/// 持たないことがあるので、モデルの設定ではなく標準の設定で値を動かす)。
pub fn conductor_values(case: &Case, effects: bool) -> Result<Vec<Vec<f32>>, Error> {
    let mut conductor = conductor_with(case, effects, false)?;
    Ok(case
        .steps
        .iter()
        .map(|&dt| {
            conductor.update(dt);
            conductor.instance().parameters().as_slice().to_vec()
        })
        .collect())
}

/// [`conductor_values`] の指揮役 (物理は `physics` のとおり)。
fn conductor_with(case: &Case, effects: bool, physics: bool) -> Result<Puppet, Error> {
    let mut conductor = loaded(case)?;
    if effects {
        conductor.set_effects(ugoe::model::effects::Effects {
            eye_blink: ugoe::model::effects::EyeBlink::standard(),
            breath: ugoe::model::effects::Breath::standard(),
            ..Default::default()
        });
    }
    conductor.set_toggles(ugoe::Toggles {
        physics,
        eye_blink: effects,
        breath: effects,
        ..ugoe::Toggles::default()
    });
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
            for (&dt, values) in case.steps.iter().zip(&case.follow) {
                puppet.follow(values, dt).map_err(|e| invalid(case, e))?;
            }
        }
    }
    Ok(puppet)
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

    /// 受け手として進めたケース (T0133) は、物理を有効にした 1 つの `Puppet` で同じ列を進めた結果と許容誤差内で同じ
    /// (ADR 0028 の「10. 一致の検査」の 2 つ目)。見本は物理の入力にリップシンクの対象を使っていない。
    #[test]
    fn follow_cases_match_a_single_puppet_with_physics() {
        let plan = plan();
        let cases: Vec<_> = plan.cases.iter().filter(|c| !c.follow.is_empty()).collect();
        assert!(cases.len() >= 2);
        for case in cases {
            assert_eq!(case.follow.len(), case.steps.len());
            let followed = evaluate(case).unwrap();
            // 物理を有効にした 1 つの `Puppet` (指揮役と同じ目パチと呼吸) で同じ列を進める。
            let mut single = conductor_with(case, true, true).unwrap();
            for &dt in &case.steps {
                single.update(dt);
            }
            let expected = Frame::from_instance(single.instance());
            let errors = crate::compare::frames(&expected, &followed);
            assert!(errors.is_empty(), "{}: {errors:?}", case.name);
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
