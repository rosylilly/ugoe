//! 参照の評価の結果を作る。
//!
//! バインディングと同じく `ugoe::Puppet` を作り (バンドルの物理を付け、モジュールは `set_modules` で付ける)、
//! パラメータを書き込んで `ModelInstance` を評価する。時間を進めるケースは `ugoe render --time` と同じ関数
//! (`ugoe_headless::animate::advance`) で進める。

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

/// `case` のとおりに作って評価した `Puppet`。
pub fn puppet(case: &Case) -> Result<Puppet, Error> {
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
    if !case.steps.is_empty() {
        // 計画の列は `advance` と同じ (`plan::steps`)。バインディングと同じく、列のとおりに進める。
        puppet.set_toggles(ugoe::Toggles {
            eye_blink: case.effects,
            breath: case.effects,
            ..ugoe::Toggles::default()
        });
        for &dt in &case.steps {
            puppet.update(dt);
        }
    }
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
        let root = crate::repo_root();
        Plan::load(&root.join("tests/conformance/cases.json"), &root).unwrap()
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
            .find(|c| !c.steps.is_empty() && c.modules.is_empty() && c.bundle)
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
}
