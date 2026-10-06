//! 操作 API のメソッド (`docs/studio-api.md#methods`)。操作は指揮役と舞台に入れる (ホットキーと同じ口。ADR 0028 の 3.)。
//!
//! `session.hello` と `events.*` は接続の状態を触るので `mod.rs` が行う。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::params;
use super::rpc::RpcError;
use super::store::Permission;
use crate::animation::{AnimationError, AnimationView};
use crate::hotkeys::{HotkeyAction, MotionPriority};
use crate::stage::{self, Stage};
use crate::{Monitor, Studio, lock};

/// パラメータの上書きの期限の既定 (秒。ADR 0028 の 6.)。
pub const DEFAULT_OVERRIDE_SECONDS: f32 = 1.0;
/// パラメータの上書きの期限の上限 (秒。クライアントが落ちても長く残らないように)。
pub const MAX_OVERRIDE_SECONDS: f32 = 10.0;
/// 1 回の `parameter.set` で書ける値の数の上限。
const MAX_VALUES: usize = 256;

/// メソッドに要る権限。知らないメソッドは `None`。
pub fn permission(method: &str) -> Option<Permission> {
    Some(match method {
        "studio.state" | "scene.list" | "expression.list" | "motion.list" | "parameter.list"
        | "dressup.list" | "pose.list" | "events.subscribe" | "events.unsubscribe" => {
            Permission::Read
        }
        "scene.activate" | "expression.set" | "expression.add" | "expression.remove"
        | "expression.toggle" | "expression.clear" | "motion.play" | "motion.stop"
        | "parameter.set" | "dressup.set" | "pose.set" | "tracking.pause"
        | "tracking.calibrate" | "lipsync.pause" => Permission::Control,
        _ => return None,
    })
}

/// メソッドを行う。
pub fn call(studio: &Studio, method: &str, p: &Map<String, Value>) -> Result<Value, RpcError> {
    match method {
        "studio.state" => Ok(state(studio)),
        "scene.list" => {
            let stage = lock(&studio.stage);
            Ok(json!({ "scenes": scene_list(&stage), "live": stage.live() }))
        }
        "scene.activate" => {
            #[derive(Deserialize)]
            struct P {
                scene: String,
            }
            let P { scene } = params(p)?;
            studio
                .switch_scene(|stage, conductor| stage.begin_activate(&scene, conductor).map(Some))
                .map_err(|e| RpcError::failed(&e))?;
            Ok(json!({}))
        }
        "expression.list" => {
            let view = lock(&studio.conductor).animation_view();
            Ok(json!({ "expressions": view.expressions, "active": view.active_expressions }))
        }
        "expression.set" | "expression.add" | "expression.remove" | "expression.toggle" => {
            #[derive(Deserialize)]
            struct P {
                expression: String,
            }
            let P { expression: name } = params(p)?;
            animate(studio, |animation, puppet| match method {
                "expression.set" => animation.set_expression(puppet, &name),
                "expression.add" => animation.add_expression(puppet, &name),
                "expression.toggle" => animation.toggle_expression(puppet, &name),
                _ => {
                    animation.remove_expression(puppet, &name);
                    Ok(())
                }
            })?;
            Ok(json!({}))
        }
        "expression.clear" => {
            animate(studio, |animation, puppet| {
                animation.clear_expressions(puppet);
                Ok(())
            })?;
            Ok(json!({}))
        }
        "motion.list" => {
            let view = lock(&studio.conductor).animation_view();
            let motions: Vec<Value> = view
                .motions
                .iter()
                .map(|m| json!({ "name": m.name, "duration": m.duration, "looping": m.looping }))
                .collect();
            Ok(json!({ "motions": motions, "current": view.current, "idle": view.idle_motion }))
        }
        "motion.play" => {
            #[derive(Deserialize)]
            struct P {
                motion: String,
                #[serde(default)]
                priority: MotionPriority,
            }
            let P { motion, priority } = params(p)?;
            let started = animate(studio, |animation, puppet| {
                animation.play(puppet, &motion, priority.into())
            })?;
            Ok(json!({ "started": started }))
        }
        "motion.stop" => {
            animate(studio, |animation, puppet| {
                animation.stop(puppet);
                Ok(())
            })?;
            Ok(json!({}))
        }
        "parameter.list" => Ok(parameter_list(studio)),
        "parameter.set" => parameter_set(studio, p),
        "dressup.list" => Ok(dressup_list(studio)),
        "dressup.set" => {
            #[derive(Deserialize)]
            struct P {
                slot: String,
                module: Option<String>,
            }
            let P { slot, module } = params(p)?;
            let module = module.map(PathBuf::from);
            if let Some(path) = &module {
                // 任意のパスは読ませない: プロファイルにあるモジュールだけ。
                if !wardrobe(&lock(&studio.stage)).contains(path) {
                    return Err(RpcError::invalid_params(
                        "`module` must be one of the modules listed by dressup.list",
                    ));
                }
            }
            if lock(&studio.stage).live().is_none() {
                return Err(RpcError::failed(&AnimationError::NoModel));
            }
            studio
                .switch_scene(|stage, conductor| {
                    stage.begin_dress_up(&slot, module.as_deref(), conductor)
                })
                .map_err(|e| RpcError::failed(&e))?;
            Ok(json!({}))
        }
        "pose.list" => Ok(pose_list(studio)),
        "pose.set" => {
            #[derive(Deserialize)]
            struct P {
                part: String,
            }
            let P { part } = params(p)?;
            if lock(&studio.stage).live().is_none() {
                return Err(RpcError::failed(&AnimationError::NoModel));
            }
            // 操作ウィンドウの `show_pose` と同じ (シーンに覚える)。
            studio
                .switch_scene(|stage, conductor| stage.begin_show_pose(&part, conductor))
                .map_err(|e| RpcError::failed(&e))?;
            Ok(json!({}))
        }
        "tracking.pause" | "lipsync.pause" => {
            #[derive(Deserialize)]
            struct P {
                #[serde(default)]
                paused: Option<bool>,
            }
            let P { paused } = params(p)?;
            let tracking = method == "tracking.pause";
            let view = {
                let mut conductor = lock(&studio.conductor);
                conductor.perform(|animation, puppet| {
                    if tracking {
                        let paused = paused.unwrap_or(!animation.tracking_paused());
                        animation.set_tracking_paused(puppet, paused);
                    } else {
                        let paused = paused.unwrap_or(!animation.lip_sync_paused());
                        animation.set_lip_sync_paused(puppet, paused);
                    }
                });
                conductor.animation_view()
            };
            Ok(tracking_state(&view))
        }
        "tracking.calibrate" => {
            // キャリブレーションは操作ウィンドウのトラッカーが行う (ホットキーと同じ知らせを送る)。
            let action = HotkeyAction::TrackingCalibrate;
            if let Some(body) = Studio::json(&Monitor::Hotkey {
                action: &action,
                error: None,
            }) {
                studio.monitors.send_all(&body);
            }
            Ok(json!({}))
        }
        _ => Err(RpcError::new(
            super::rpc::METHOD_NOT_FOUND,
            format!("unknown method `{method}`"),
        )),
    }
}

/// 指揮役の演技を触る (モデルが無ければ誤り)。
fn animate<T>(
    studio: &Studio,
    action: impl FnOnce(
        &mut crate::animation::Animation,
        &mut ugoe::Puppet,
    ) -> Result<T, AnimationError>,
) -> Result<T, RpcError> {
    lock(&studio.conductor)
        .perform(|animation, puppet| match puppet {
            Some(puppet) => action(animation, puppet),
            None => Err(AnimationError::NoModel),
        })
        .map_err(|e| RpcError::failed(&e))
}

/// トラッキングと口パクの様子 (`tracking.changed` の通知と `tracking.pause` の応答)。
pub fn tracking_state(view: &AnimationView) -> Value {
    json!({
        "tracking": view.tracking,
        "trackingPaused": view.tracking_paused,
        "lipSyncPaused": view.lip_sync_paused,
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// シーンの一覧 (`scene.list` と `scene.listChanged`)。モデルはファイル名だけを出す (パスは出さない)。
pub fn scene_list(stage: &Stage) -> Vec<Value> {
    stage
        .profile()
        .scenes
        .iter()
        .map(|s| json!({ "id": s.id, "name": s.name, "model": s.model.as_deref().map(file_name) }))
        .collect()
}

/// 出しているシーンに付けているモジュール (合成の順)。
pub fn live_modules(stage: &Stage) -> Vec<Value> {
    stage
        .live_modules()
        .map(|(path, module)| module_value(path, module))
        .collect()
}

fn module_value(path: &Path, module: &ugoe::model::module::Module) -> Value {
    json!({
        "id": path.to_string_lossy(),
        "name": module.name,
        "slot": module.slot.as_str(),
    })
}

/// 着せ替えに使えるモジュール: プロファイルのシーンが付けるもの・ホットキーの着せ替えに割り当てたもの (重なりなし、出てきた順)。
/// 外部 API は、ここにあるモジュールだけを付けられる (任意のパスを読ませない)。
pub fn wardrobe(stage: &Stage) -> Vec<PathBuf> {
    let profile = stage.profile();
    let from_hotkeys = profile.hotkeys.iter().filter_map(|h| match &h.action {
        HotkeyAction::DressUp {
            module: Some(path), ..
        } => Some(path),
        _ => None,
    });
    let mut paths: Vec<PathBuf> = Vec::new();
    for path in profile
        .scenes
        .iter()
        .flat_map(|s| &s.modules)
        .chain(from_hotkeys)
    {
        if !paths.contains(path) {
            paths.push(path.clone());
        }
    }
    paths
}

fn state(studio: &Studio) -> Value {
    let stage = lock(&studio.stage);
    let conductor = lock(&studio.conductor);
    let scenes = scene_list(&stage);
    let live = stage
        .live()
        .and_then(|id| scenes.iter().find(|s| s["id"] == id).cloned());
    let model = conductor
        .scene()
        .filter(|s| s.model_revision != 0)
        .map(|s| s.model_name.clone());
    let view = conductor.animation_view();
    json!({
        "scene": live,
        "model": model,
        "modules": live_modules(&stage),
        "expressions": view.active_expressions,
        "motion": view.current,
        "tracking": tracking_state(&view),
    })
}

fn parameter_list(studio: &Studio) -> Value {
    let conductor = lock(&studio.conductor);
    let values = conductor.sent_values();
    let parameters: Vec<Value> = conductor
        .parameters()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            json!({
                "id": p.id.as_str(),
                "name": p.name,
                "min": p.min,
                "max": p.max,
                "default": p.default,
                "value": values.get(i).copied().unwrap_or(p.default),
                "internal": p.internal,
            })
        })
        .collect();
    json!({ "parameters": parameters })
}

fn parameter_set(studio: &Studio, p: &Map<String, Value>) -> Result<Value, RpcError> {
    #[derive(Deserialize)]
    struct Item {
        id: String,
        value: f32,
        #[serde(default)]
        weight: Option<f32>,
    }
    #[derive(Deserialize)]
    struct P {
        values: Vec<Item>,
        #[serde(default)]
        duration: Option<f32>,
    }
    let P { values, duration } = params(p)?;
    if values.len() > MAX_VALUES {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_VALUES} values per call"
        )));
    }
    let duration = duration.unwrap_or(DEFAULT_OVERRIDE_SECONDS);
    if !(duration.is_finite() && duration > 0.0 && duration <= MAX_OVERRIDE_SECONDS) {
        return Err(RpcError::invalid_params(format!(
            "`duration` must be greater than 0 and at most {MAX_OVERRIDE_SECONDS} seconds"
        )));
    }
    // 先にすべてを確かめる (誤りがあれば、どの値も上書きしない)。
    if let Some(item) = values
        .iter()
        .find(|item| !item.value.is_finite() || !(0.0..=1.0).contains(&item.weight.unwrap_or(1.0)))
    {
        return Err(RpcError::invalid_params(format!(
            "`{}`: `value` must be a number and `weight` must be between 0 and 1",
            item.id
        )));
    }
    let mut conductor = lock(&studio.conductor);
    if conductor.scene().is_none_or(|s| s.model_revision == 0) {
        return Err(RpcError::failed(&AnimationError::NoModel));
    }
    let mut unknown = Vec::new();
    for item in values {
        let weight = item.weight.unwrap_or(1.0);
        if !conductor.override_parameter(&item.id, item.value, weight, duration) {
            unknown.push(item.id);
        }
    }
    Ok(json!({ "unknown": unknown }))
}

/// `pose.list`: 出しているモデル (と付けているモジュール) のポーズのグループと、グループごとに表示している要素 (T0843)。
fn pose_list(studio: &Studio) -> Value {
    let conductor = lock(&studio.conductor);
    let shown = conductor
        .scene()
        .map(|s| s.look.pose.as_slice())
        .unwrap_or_default();
    let groups: Vec<Value> = conductor
        .pose_groups()
        .into_iter()
        .map(|group| {
            let part = shown.iter().find(|p| group.parts.contains(p));
            json!({ "parts": group.parts, "module": group.module, "shown": part })
        })
        .collect();
    json!({ "groups": groups })
}

fn dressup_list(studio: &Studio) -> Value {
    dressup_list_with(studio, read_module_value)
}

/// `dressup.list` の中身。付けていないモジュールは `read` で読む (舞台と指揮役のロックを放してから。
/// 読む間にシーンの切り替えを待たせない。T0819)。読んだ概要はパス・更新時刻・大きさで覚え、変わっていなければ読み直さない
/// ([`ModuleSummaries`]。T0833)。
pub fn dressup_list_with(studio: &Studio, mut read: impl FnMut(&Path) -> Option<Value>) -> Value {
    let (slots, worn, closet) = {
        let stage = lock(&studio.stage);
        let slots = lock(&studio.conductor).animation_view().slots;
        let worn: Vec<(PathBuf, Value)> = stage
            .live_modules()
            .map(|(path, module)| (path.to_owned(), module_value(path, module)))
            .collect();
        (slots, worn, wardrobe(&stage))
    };
    let slot_list: Vec<Value> = slots
        .iter()
        .map(|slot| {
            let module = worn
                .iter()
                .find(|(_, value)| value["slot"] == slot.as_str())
                .map(|(path, _)| path.to_string_lossy().into_owned());
            json!({ "slot": slot, "module": module })
        })
        .collect();
    // 一覧に無くなったモジュールは忘れる。
    lock(&studio.module_summaries).retain(&closet);
    let mut modules = Vec::new();
    for path in closet {
        // 付けているものは読み直さない。読めないものは出さない。
        let found = worn
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, value)| value.clone());
        let value = match found {
            Some(value) => Some(value),
            None => summary(studio, &path, &mut read),
        };
        if let Some(value) = value
            && slots.iter().any(|s| value["slot"] == s.as_str())
        {
            modules.push(value);
        }
    }
    json!({ "slots": slot_list, "modules": modules })
}

/// 付けていないモジュールの概要。覚えていて更新時刻と大きさが変わっていなければ読まない。
/// 覚えの表のロックは読む間は握らない。ファイルが無ければ忘れて `None`。
fn summary(
    studio: &Studio,
    path: &Path,
    read: &mut impl FnMut(&Path) -> Option<Value>,
) -> Option<Value> {
    let Some(stamp) = Stamp::of(path) else {
        lock(&studio.module_summaries).forget(path);
        return None;
    };
    if let Some(value) = lock(&studio.module_summaries).get(path, stamp) {
        return value;
    }
    // 読む前に調べた時刻と大きさで覚える (読む間に変わっても、次の呼び出しで読み直す)。
    let value = read(path);
    lock(&studio.module_summaries).insert(path, stamp, value.clone());
    value
}

/// `dressup.list` が覚えるモジュールの概要の数の上限 (概要は小さい。数で抑える)。
pub const MAX_MODULE_SUMMARIES: usize = 256;

/// ファイルの更新時刻と大きさ (変わったかを見る)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

impl Stamp {
    /// ファイルの時刻と大きさ。ファイルが無い・時刻を取れないなら `None` (覚えない)。
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok()?,
            len: metadata.len(),
        })
    }
}

/// `dressup.list` が読んだ付けていないモジュールの概要 (パス → 時刻・大きさ・概要。読めなかったものは `None` を覚える)。T0833。
/// 着せ替えに使えるモジュール (`wardrobe`) に無くなったものは忘れ、[`MAX_MODULE_SUMMARIES`] を超えては覚えない。
#[derive(Debug, Default)]
pub struct ModuleSummaries(HashMap<PathBuf, (Stamp, Option<Value>)>);

impl ModuleSummaries {
    fn get(&self, path: &Path, stamp: Stamp) -> Option<Option<Value>> {
        self.0
            .get(path)
            .filter(|(s, _)| *s == stamp)
            .map(|(_, value)| value.clone())
    }

    fn insert(&mut self, path: &Path, stamp: Stamp, value: Option<Value>) {
        if self.0.len() < MAX_MODULE_SUMMARIES || self.0.contains_key(path) {
            self.0.insert(path.to_owned(), (stamp, value));
        }
    }

    fn forget(&mut self, path: &Path) {
        self.0.remove(path);
    }

    fn retain(&mut self, closet: &[PathBuf]) {
        self.0.retain(|path, _| closet.contains(path));
    }

    /// 覚えている数。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 何も覚えていないか。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// モジュールのファイルを読み、`dressup.list` に出す値にする (読めなければ `None`)。
pub fn read_module_value(path: &Path) -> Option<Value> {
    let bytes = stage::read_asset(
        stage::Asset::Module,
        path,
        stage::MODULE_EXTENSIONS,
        stage::MAX_MODEL_BYTES,
    )
    .ok()?;
    let file = ugoe::format::read_module(&bytes).ok()?;
    Some(module_value(path, &file.module))
}
