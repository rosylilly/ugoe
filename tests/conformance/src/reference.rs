//! 参照の評価の結果を作る。
//!
//! バインディングと同じく `ugoe::Puppet` を作り (バンドルの物理を付け、モジュールは `set_modules` で付ける)、
//! パラメータを書き込んで `ModelInstance` を評価する。時間を進めるケースは `ugoe render --time` と同じ関数
//! (`ugoe_headless::animate::advance`) で進める。受け手として進めるケース (`Case::follow`) は `Puppet::follow` で進める。

use std::sync::Arc;

use ugoe::Puppet;
use ugoe::motion::Priority;
use ugoe_headless::animate;

use crate::frame::{Frame, MotionEventFrame, TextureFrame, image_digest, image_error_kind};
use crate::plan::Case;
use crate::{Error, read};

/// `case` の参照の結果。
pub fn evaluate(case: &Case) -> Result<Frame, Error> {
    let mut events = Vec::new();
    let puppet = run(case, &mut events)?;
    let mut frame = Frame::from_instance(puppet.instance());
    if !case.motions.is_empty() {
        frame.motion_events = Some(events);
    }
    frame.textures = textures(case, &puppet)?;
    Ok(frame)
}

/// テクスチャの番号 (`Puppet::texture_sources` の順) ごとのデコードの結果 (T0466)。テクスチャを持つバンドルのケースだけ
/// (PNG も KTX2 も。T0473。ほかは `None`)。描画するテクスチャは乗算済み、法線マップはアルファを掛けずにデコードする。
fn textures(case: &Case, puppet: &Puppet) -> Result<Option<Vec<TextureFrame>>, Error> {
    use ugoe::TextureSource;
    use ugoe_format::image::{ImageLimits, decode_image, decode_image_premultiplied};

    if !case.bundle {
        return Ok(None);
    }
    let bundle = ugoe_format::read_bundle(&read(&case.model)?).map_err(|source| Error::Format {
        context: format!("reading {}", case.model.display()),
        source,
    })?;
    let modules = case
        .modules
        .iter()
        .map(|path| {
            ugoe_format::read_module(&read(path)?).map_err(|source| Error::Format {
                context: format!("reading {}", path.display()),
                source,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let sources: Vec<_> = puppet
        .texture_sources()
        .into_iter()
        .map(|source| match source {
            TextureSource::Base(i) => {
                let i = i as usize;
                (bundle.textures.get(i), bundle.normal_map(i))
            }
            TextureSource::Module { module, index } => {
                let file = modules.iter().find(|f| f.module.name == module);
                let i = index as usize;
                (
                    file.and_then(|f| f.textures.get(i)),
                    file.and_then(|f| f.normal_map(i)),
                )
            }
        })
        .collect();
    if sources.iter().all(|(t, n)| t.is_none() && n.is_none()) {
        return Ok(None);
    }
    let digest = |t: Option<&ugoe_format::TextureData>, premultiply: bool| {
        t.map(|t| {
            let limits = ImageLimits::default();
            let image = if premultiply {
                decode_image_premultiplied(t.format, &t.bytes, limits)
            } else {
                decode_image(t.format, &t.bytes, limits)
            };
            match image {
                Ok(image) => image_digest(image.width, image.height, &image.pixels),
                Err(e) => image_error_kind(&e).to_owned(),
            }
        })
    };
    Ok(Some(
        sources
            .into_iter()
            .map(|(t, n)| TextureFrame {
                color: digest(t, true),
                normal_map: digest(n, false),
            })
            .collect(),
    ))
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

/// 指揮役の効果の設定 (`effects` のとき): 標準の目パチと呼吸 (`EyeBlink::standard()`・`Breath::standard()`) のうち
/// モデルにあるパラメータを対象にするものと、モデルのリップシンクの設定。`set_effects_json` はモデルにないパラメータを
/// 拒むので (T0461)、ここで外す (外さなくても、ランタイムはモデルにないパラメータを無視するので値は同じ)。
fn conductor_effects(conductor: &Puppet) -> ugoe::model::effects::Effects {
    use ugoe::model::effects::{Breath, Effects, EyeBlink};
    let model = conductor.instance().compiled().model();
    let known = |id: &ugoe::model::ParamId| model.parameters.iter().any(|p| &p.id == id);
    let mut eye_blink = EyeBlink::standard();
    eye_blink.parameters.retain(|id| known(id));
    let mut breath = Breath::standard();
    breath.parameters.retain(|p| known(&p.parameter));
    Effects {
        eye_blink,
        breath,
        lip_sync: model.effects.lip_sync.clone(),
    }
}

/// `conductor_effects` を、実行器が指揮役の `set_effects_json` に渡す JSON にしたもの (計画の `Conduct::effects`。T0454)。
pub fn conductor_effects_json(case: &Case) -> Result<String, Error> {
    let effects = conductor_effects(&loaded(case)?);
    serde_json::to_string(&effects).map_err(|e| invalid(case, e))
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
        let effects = conductor_effects(&conductor);
        conductor.set_effects(effects);
    }
    conductor.set_toggles(ugoe::Toggles {
        physics,
        eye_blink: effects,
        breath: effects,
        ..ugoe::Toggles::default()
    });
    if let Some(name) = &case.motion {
        let motion = bundle_motion(case, name)?;
        conductor.start_motion(motion, Priority::Normal);
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
    run(case, &mut Vec::new())
}

/// [`puppet`] と同じ。計画の `motions` を始め、発火したモーションのイベントを `events` に足す (T0462)。
fn run(case: &Case, events: &mut Vec<MotionEventFrame>) -> Result<Puppet, Error> {
    let mut puppet = loaded(case)?;
    if !case.steps.is_empty() {
        // 計画の列は `advance` と同じ (`plan::steps`)。バインディングと同じく、列のとおりに進める。
        puppet.set_toggles(toggles(case));
        if case.follow.is_empty() {
            for (i, &dt) in case.steps.iter().enumerate() {
                for m in case.motions.iter().filter(|m| m.step == i) {
                    let priority = match m.priority.as_str() {
                        "idle" => Priority::Idle,
                        "force" => Priority::Force,
                        _ => Priority::Normal,
                    };
                    let accepted = puppet.start_motion(bundle_motion(case, &m.name)?, priority);
                    if accepted != m.accepted {
                        return Err(invalid(
                            case,
                            format!("starting `{}` at step {i} gave {accepted}", m.name),
                        ));
                    }
                }
                puppet.update(dt);
                if !case.motions.is_empty() {
                    events.extend(
                        puppet
                            .motion_events()
                            .iter()
                            .zip(puppet.motion_event_sources())
                            .map(|(e, source)| MotionEventFrame {
                                step: i,
                                value: e.value.clone(),
                                time: e.t,
                                motion: source.name.clone(),
                                number: source.number,
                            }),
                    );
                }
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

    /// テクスチャを持つバンドルのケースの参照は、テクスチャごとのデコードの結果を書く (KTX2 は T0466、PNG は T0473)。
    /// BCn・16 ビットとパレットの PNG も読め、JSON のモデルのケースは書かない。
    #[test]
    fn bundle_cases_record_decoded_textures() {
        let plan = plan();
        let case = |name: &str| plan.cases.iter().find(|c| c.name == name).unwrap();
        let lit = evaluate(case("ktx2-bc-avatar-lit"))
            .unwrap()
            .textures
            .unwrap();
        assert!(!lit.is_empty());
        for t in &lit {
            let color = t.color.as_deref().unwrap();
            assert!(color.contains(':'), "{color}");
        }
        assert!(
            lit.iter()
                .any(|t| t.normal_map.as_deref().is_some_and(|n| n.contains(':')))
        );
        assert!(
            evaluate(case("ktx2-bc3-avatar"))
                .unwrap()
                .textures
                .is_some()
        );
        for name in ["masks-nested", "png-16bit", "png-palette", "avatar-default"] {
            let textures = evaluate(case(name)).unwrap().textures.unwrap();
            assert!(
                textures
                    .iter()
                    .all(|t| t.color.as_deref().is_some_and(|c| c.contains(':'))),
                "{name}: {textures:?}"
            );
        }
        // 16 ビットの PNG は上位の 8 ビットを使い、ガンマで値を変えないので、8 ビットの PNG のバンドルと同じ画素になる。
        assert_eq!(
            evaluate(case("png-16bit")).unwrap().textures,
            evaluate(case("masks-nested")).unwrap().textures
        );
        let json = plan.cases.iter().find(|c| !c.bundle).unwrap();
        assert_eq!(evaluate(json).unwrap().textures, None, "{}", json.name);
    }

    /// `motions` のケースの参照は、発火したイベントとその出どころを書く (T0462)。フェードアウト中の idle のイベントは
    /// 1 番目のもので、始まらなかった要求は番号を取らない (`spec/examples/motion-crossfade/timeline.json`)。
    #[test]
    fn motion_cases_record_the_motion_that_fired() {
        let plan = plan();
        let case = plan
            .cases
            .iter()
            .find(|c| c.name == "motion-events-crossfade")
            .unwrap();
        let events = evaluate(case).unwrap().motion_events.unwrap();
        let at = |step: usize| -> Vec<(&str, &str, u64)> {
            events
                .iter()
                .filter(|e| e.step == step)
                .map(|e| (e.value.as_str(), e.motion.as_str(), e.number))
                .collect()
        };
        assert_eq!(at(0), [("idle-start", "idle", 1)]);
        // wave (2 番目) に替えた後、フェードアウト中の idle のイベント
        assert_eq!(at(4), [("idle-start", "idle", 1)]);
        assert_eq!(at(5), [("idle-half", "idle", 1)]);
        assert_eq!(at(9), [("wave-end", "wave", 2)]);
        assert_eq!(at(10), [("idle-start", "idle", 3)]);
        assert!(events.iter().all(|e| e.motion != "wave" || e.number == 2));
        // ほかのケースは書かない
        let other = plan.cases.iter().find(|c| c.motions.is_empty()).unwrap();
        assert_eq!(evaluate(other).unwrap().motion_events, None);
    }

    /// 計画の `update` の列で進めた結果が、`ugoe render --time` (`animate::advance`) と同じになる。
    #[test]
    fn steps_match_advance() {
        let plan = plan();
        let case = plan
            .cases
            .iter()
            .find(|c| {
                !c.steps.is_empty()
                    && c.modules.is_empty()
                    && c.bundle
                    && c.follow.is_empty()
                    && c.motions.is_empty()
            })
            .unwrap();
        // テクスチャのデコードの結果は `ModelInstance` から作らないので比べない。
        let expected = Frame {
            textures: None,
            ..evaluate(case).unwrap()
        };
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

    /// 実行器の中で指揮役を進めるケース (T0454): 実行器と同じ手順 (計画の `Conduct` だけを読んで指揮役を作り、指揮役の口で読んだ値で
    /// 受け手を `follow` させる) で進めると、計画の `follow` の値で進めた参照と同じになる。
    #[test]
    fn conduct_cases_match_the_planned_follow_values() {
        let plan = plan();
        let cases: Vec<_> = plan.cases.iter().filter(|c| c.conduct.is_some()).collect();
        assert!(cases.len() >= 3);
        assert!(
            cases
                .iter()
                .any(|c| c.conduct.as_ref().unwrap().motion.is_some())
        );
        assert!(
            cases
                .iter()
                .any(|c| !c.conduct.as_ref().unwrap().lip_sync.is_empty())
        );
        assert!(
            cases
                .iter()
                .any(|c| c.conduct.as_ref().unwrap().effects.is_some())
        );
        for case in cases {
            let conduct = case.conduct.as_ref().unwrap();
            assert!(!case.follow.is_empty(), "{}", case.name);
            let mut conductor = loaded(case).unwrap();
            if let Some(json) = &conduct.effects {
                conductor.set_effects(serde_json::from_str(json).unwrap());
            }
            conductor.set_toggles(ugoe::Toggles {
                physics: false,
                eye_blink: conduct.effects.is_some(),
                breath: conduct.effects.is_some(),
                ..ugoe::Toggles::default()
            });
            if let Some(name) = &conduct.motion {
                let motion = bundle_motion(case, name).unwrap();
                assert!(conductor.start_motion(motion, ugoe::motion::Priority::Normal));
            }
            let mut follower = loaded(case).unwrap();
            follower.set_toggles(toggles(case));
            for (i, &dt) in case.steps.iter().enumerate() {
                if let Some(&level) = conduct.lip_sync.get(i) {
                    conductor.set_lip_sync_level(Some(level));
                }
                conductor.update(dt);
                follower
                    .follow(
                        conductor.values_before_physics(),
                        conductor.lip_sync_applied(),
                        conductor.part_opacities_applied(),
                        dt,
                    )
                    .unwrap();
            }
            let expected = Frame {
                textures: None,
                ..evaluate(case).unwrap()
            };
            assert_eq!(
                Frame::from_instance(follower.instance()),
                expected,
                "{}",
                case.name
            );
        }
    }
}
