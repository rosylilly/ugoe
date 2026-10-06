//! 指揮役 (conductor): パラメータの値を決める唯一の場所 (ADR 0028 の 3.、`docs/studio.md#全体の構成`)。
//!
//! ネイティブの [`Puppet`] を 1 つ持ち、**物理の段を切って** `update(dt)` する (描かない)。配るのは物理の段に入る直前の値
//! (`Puppet::values_before_physics`)・リップシンクの入力 (`Puppet::lip_sync_applied`)・モーションのパーツの不透明度のカーブ
//! (`Puppet::part_opacities_applied`) で、受け手はそれを基準値として書き、物理 → リップシンク → パーツの不透明度 (カーブとポーズ) の
//! 順に自分で進める (`docs/spec/motion.md#受け手として進める`、T0134・T0135)。
//! 時計とスレッドは持たない (刻みは `lib.rs` の刻みのスレッドが `dt` を測って [`Conductor::tick`] を呼ぶ)。
//!
//! モーション・表情・待機モーション・トラッキング・口パクの入力 (T0803) は [`Animation`] が持ち、[`Conductor::perform`] で触る。

use std::sync::Arc;

use serde::Serialize;
use ugoe::format::Bundle;
use ugoe::model::Parameter;
use ugoe::model::expression::Expression;
use ugoe::model::module::Module;
use ugoe::model::motion::{Motion, MotionEvent};
use ugoe::{DressUpError, Puppet, Toggles};

use crate::animation::{Animation, AnimationView};
use crate::stream::{Frame, Look, Scene};

/// 1 刻みで進める時間の上限 (秒)。止まっていた後に物理が大きく飛ばないように (ADR 0028 の 3.)。
pub const MAX_DT: f32 = 0.1;

/// モデルを読めない。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error(transparent)]
    Compile(#[from] ugoe::runtime::CompileError),
    #[error(transparent)]
    DressUp(#[from] DressUpError),
    /// モデルを読まずにシーンを替えようとした (モデルが無い)。
    #[error("no model is loaded")]
    NoModel,
}

/// 読んだモデル ([`assemble`] でパペットを組み立てる)。
#[derive(Debug)]
pub struct NewModel {
    /// 表示名 (ファイル名)。
    pub name: String,
    pub bundle: Bundle,
}

/// 組み立てたパペット ([`Conductor::present`] に渡す)。
///
/// 大きなモデルではモデルのコンパイルとモジュールの合成が重いので、舞台と指揮役のロックの外で組み立ててから差し込む
/// (`docs/studio.md#シーンの切り替えと舞台のロック`、T0837)。
#[derive(Debug)]
pub struct Assembled {
    name: String,
    puppet: Puppet,
    motions: Vec<Motion>,
    expressions: Vec<Expression>,
}

/// モデルからパペットを組み立て、`modules` を付ける (指揮役の状態は使わない。ロックの外で呼ぶ)。
///
/// 物理の設定は持たせたうえで段を切る (受け手が回す)。
pub fn assemble(model: NewModel, modules: &[Arc<Module>]) -> Result<Assembled, LoadError> {
    let bundle = model.bundle;
    let mut puppet = Puppet::new(bundle.model)?;
    if let Some(physics) = bundle.physics {
        puppet = puppet.with_physics(physics);
    }
    puppet.set_toggles(Toggles {
        physics: false,
        ..Toggles::default()
    });
    if !modules.is_empty() {
        puppet.set_modules(modules.to_vec())?;
    }
    Ok(Assembled {
        name: model.name,
        puppet,
        motions: bundle.motions,
        expressions: bundle.expressions,
    })
}

/// ポーズのグループ 1 つ (操作ウィンドウのポーズの欄。`@ugoe/player` の `PoseGroupInfo` と同じ形。T0842)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseGroupInfo {
    /// 要素のパーツの ID (グループの順)。モジュールのパーツは `<モジュール>/<パーツ>`。
    pub parts: Vec<String>,
    /// 着せ替えで付けたモジュールのグループなら、そのモジュールの名前 (先頭の要素の接頭辞)。本体のグループは `None`。
    pub module: Option<String>,
}

/// シーンの設定のポーズの要素 `wanted` を表示する要素にし、グループごとに表示する要素を返す (合成したポーズのグループの順。T0842)。
///
/// グループの要素が `wanted` に無ければ先頭の要素にする (同じグループの要素がいくつもあれば後のもの)。モデルに無い要素は捨てる。
/// 要素の無いグループは飛ばす。`immediately` ならフェードせずに表示する (モデルを読み直した・モジュールを付け直した直後。
/// 受け手も同じ時にフェードを飛ばす。T0844)。そうでなければ次の `update` からフェードで切り替わる。
fn show_pose(puppet: &mut Puppet, wanted: &[String], immediately: bool) -> Vec<String> {
    let shown: Vec<_> = puppet
        .pose()
        .pose()
        .groups
        .iter()
        .filter_map(|group| {
            wanted
                .iter()
                .rev()
                .find_map(|w| group.entries.iter().find(|e| e.part.as_str() == w))
                .or_else(|| group.entries.first())
                .map(|e| e.part.clone())
        })
        .collect();
    for part in &shown {
        if immediately {
            puppet.pose_mut().show_immediately(part);
        } else {
            puppet.pose_mut().show(part);
        }
    }
    shown.iter().map(|p| p.as_str().to_owned()).collect()
}

/// `a` と `b` が同じモジュールの組か (`Arc` で見分ける)。
fn same_modules(a: &[Arc<Module>], b: &[Arc<Module>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Arc::ptr_eq(a, b))
}

/// `.ugoe` バンドルか JSON のモデルのバイト列を読む (先頭が `{` なら JSON)。
pub fn read_model(bytes: &[u8]) -> Result<Bundle, LoadError> {
    let json = bytes
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{');
    if json {
        let text = String::from_utf8_lossy(bytes);
        return Ok(Bundle::new(ugoe::format::model_from_json(&text)?));
    }
    Ok(ugoe::format::read_bundle(bytes)?)
}

/// 指揮役。
#[derive(Debug)]
pub struct Conductor {
    puppet: Option<Puppet>,
    /// モーション・表情・入力 (モデルを読み直しても入力と設定は保つ)。
    animation: Animation,
    scene: Option<Scene>,
    /// 次に配るフレームの `seq`。
    next_seq: u64,
    revision: u32,
    /// 次のフレームに物理のリセットを載せるか。
    reset: bool,
    /// 外部 API の値の上書き (T0805)。
    overrides: Vec<Override>,
    /// 直前に配った値 (上書きを含む)。
    sent: Vec<f32>,
    /// 刻みで発火したモーションのイベント (読むと空になる)。
    motion_events: Vec<FiredEvent>,
    /// 最後に知らせたポーズの表示中の要素と選択 ([`Conductor::take_pose_changed`]。T0848)。
    pose_noticed: (Vec<String>, Vec<String>),
}

/// 刻みで発火したモーションのイベントと、それを発火したモーション (外部 API の `motion.event`)。
#[derive(Debug, Clone, PartialEq)]
pub struct FiredEvent {
    /// イベントを発火したモーションの名前 (替えられてフェードアウトしているものや、この刻みで終わったものを含む)。
    pub motion: String,
    /// 発火したモーションが、刻みの後も今のモーションか。そうでなければ、終わったか替えられたモーション
    /// (その `motion.finished` は送り済みか、この刻みで送る)。
    pub current: bool,
    pub event: MotionEvent,
}

/// 外部 API の値の上書き (ADR 0028 の 6.。期限を過ぎたらやめる)。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Override {
    index: usize,
    value: f32,
    weight: f32,
    /// 残りの時間 (秒。刻みの `dt` で減らす)。
    remaining: f32,
}

impl Default for Conductor {
    fn default() -> Self {
        Self {
            puppet: None,
            animation: Animation::new(),
            scene: None,
            next_seq: 0,
            revision: 0,
            reset: false,
            overrides: Vec::new(),
            sent: Vec::new(),
            motion_events: Vec::new(),
            pose_noticed: (Vec::new(), Vec::new()),
        }
    }
}

impl Conductor {
    pub fn new() -> Self {
        Self::default()
    }

    /// モデルを読み、新しいシーンにする (`revision` が上がる)。パラメータは既定値から始める。モジュールは付けず、見せ方は既定。
    pub fn load(&mut self, model_name: &str, bundle: Bundle) -> Result<&Scene, LoadError> {
        let model = assemble(
            NewModel {
                name: model_name.to_owned(),
                bundle,
            },
            &[],
        )?;
        self.present(Some(model), Some(Vec::new()), Look::default())
    }

    /// シーンを替える (`revision` が上がる)。失敗したら何も変えない。
    ///
    /// - `model` が `Some` なら組み立てたパペット ([`assemble`]) に替える (パラメータは既定値から。受け手もモデルを読み直すので
    ///   物理は新しく始まる)。
    ///   `None` なら今のモデルのまま、パラメータの値・再生中のモーションなどを引き継ぐ (`docs/studio.md#シーンとプロファイル`)。
    /// - `modules` が `Some` なら付けるモジュールの組をそれにする (合成の順)。今のモデルのまま組が変わったときは、
    ///   次のフレームに物理のリセットを載せる (受け手どうしの物理を揃える)。`None` なら変えない。
    /// - 見せ方 (`look`) は受け手に配るだけで、指揮役は使わない。ただしポーズの要素 (`look.pose`) は指揮役のパペットにも表示させ、
    ///   モデルにある要素と、挙がっていないグループの先頭の要素で埋め直して配る (受け手のポーズを指揮役と揃える。T0842)。
    ///   モデルを読み直した・モジュールの組が変わったときはフェードせずに表示し、同じモデルと組のままならフェードで切り替える (T0844)。
    ///
    /// モデルが無ければ ([`Conductor::clear`] の後など) `model` が要る。
    pub fn present(
        &mut self,
        model: Option<Assembled>,
        modules: Option<Vec<Arc<Module>>>,
        mut look: Look,
    ) -> Result<&Scene, LoadError> {
        let revision = self.revision + 1;
        let previous = self.scene.as_ref();
        let (model_revision, model_name) = match &model {
            Some(m) => (revision, m.name.clone()),
            None => (
                previous.map_or(0, |s| s.model_revision),
                previous.map(|s| s.model_name.clone()).unwrap_or_default(),
            ),
        };
        let mut modules_revision = previous.map_or(revision, |s| s.modules_revision);
        let mut reset = false;
        match model {
            Some(model) => {
                let mut puppet = model.puppet;
                // 組み立てたときと違う組なら付け直す (ふつうは組み立てたときに付けてある)。
                if let Some(modules) = modules.filter(|m| !same_modules(puppet.modules(), m)) {
                    puppet.set_modules(modules)?;
                }
                self.animation
                    .load(&mut puppet, model.motions, model.expressions);
                // 受け手は新しいモデルを読み直すので、物理は新しく始まる (リセットを載せなくてよい)。
                self.puppet = Some(puppet);
                self.overrides.clear();
                self.sent.clear();
                modules_revision = revision;
            }
            None => {
                let Some(puppet) = &mut self.puppet else {
                    return Err(LoadError::NoModel);
                };
                if let Some(modules) = modules.filter(|m| !same_modules(puppet.modules(), m)) {
                    puppet.set_modules(modules)?;
                    modules_revision = revision;
                    reset = true;
                }
            }
        }
        let Some(puppet) = &mut self.puppet else {
            return Err(LoadError::NoModel);
        };
        // モデルを読み直したか、モジュールの組が変わったときは `modules_revision` がこのシーンの版になる。
        look.pose = show_pose(puppet, &look.pose, modules_revision == revision);
        self.revision = revision;
        self.reset = reset || (self.reset && model_revision != revision);
        let parameter_ids = puppet
            .instance()
            .compiled()
            .model()
            .parameters
            .iter()
            .map(|p| p.id.to_string())
            .collect();
        let modules = puppet.modules().iter().map(|m| m.name.clone()).collect();
        Ok(self.scene.insert(Scene {
            revision,
            model_revision,
            modules_revision,
            model_name,
            parameter_ids,
            modules,
            look,
        }))
    }

    /// モデルを外す (モデルの無いシーン)。`revision` が上がり、以後は [`Conductor::tick`] がフレームを作らない。
    pub fn clear(&mut self, mut look: Look) -> &Scene {
        look.pose.clear();
        self.revision += 1;
        self.puppet = None;
        self.animation.unload();
        self.reset = false;
        self.overrides.clear();
        self.sent.clear();
        self.scene.insert(Scene {
            revision: self.revision,
            model_revision: 0,
            modules_revision: self.revision,
            model_name: String::new(),
            parameter_ids: Vec::new(),
            modules: Vec::new(),
            look,
        })
    }

    /// モデル (と付けているモジュール) にライティングを受けるメッシュがあるか。モデルが無ければ `false`。
    pub fn uses_lighting(&self) -> bool {
        self.puppet.as_ref().is_some_and(Puppet::uses_lighting)
    }

    /// 付けているモジュール (合成の順)。
    pub fn modules(&self) -> &[Arc<Module>] {
        self.puppet.as_ref().map_or(&[], |p| p.modules())
    }

    /// 今のモデル (と付けているモジュール) のポーズのグループ (合成したポーズの順。要素の無いグループは飛ばす。T0842)。
    pub fn pose_groups(&self) -> Vec<PoseGroupInfo> {
        let Some(puppet) = &self.puppet else {
            return Vec::new();
        };
        puppet
            .pose()
            .pose()
            .groups
            .iter()
            .filter(|g| !g.entries.is_empty())
            .map(|g| {
                let parts: Vec<String> = g
                    .entries
                    .iter()
                    .map(|e| e.part.as_str().to_owned())
                    .collect();
                let module = parts
                    .first()
                    .and_then(|p| p.split_once('/'))
                    .map(|(m, _)| m.to_owned());
                PoseGroupInfo { parts, module }
            })
            .collect()
    }

    /// 今のシーン。モデルを読んでいなければ `None`。
    pub fn scene(&self) -> Option<&Scene> {
        self.scene.as_ref()
    }

    /// ポーズのグループごとに今表示している要素 (合成したポーズのグループの順。モデルが無ければ空。T0848)。
    ///
    /// シーンを出した直後はシーンの `look.pose` (選択) と同じで、モーションがパーツの不透明度のカーブで要素を切り替えると
    /// それに替わる (モーションが終わっても残る。`docs/studio.md#ポーズの要素`)。
    pub fn shown_pose(&self) -> Vec<String> {
        self.puppet.as_ref().map_or_else(Vec::new, |puppet| {
            puppet
                .pose()
                .visible_parts()
                .map(|p| p.as_str().to_owned())
                .collect()
        })
    }

    /// 途中から来た受け手に送るシーン: 今のシーンの `look.pose` を、表示している要素 ([`Conductor::shown_pose`]) にしたもの。
    ///
    /// モーションが切り替えた要素は受け手も前からのフレームで切り替えているので、後から来た受け手には選択ではなく
    /// 表示している要素を表示させる (指揮役と揃える。T0848)。
    pub fn joining_scene(&self) -> Option<Scene> {
        let mut scene = self.scene.clone()?;
        if self.puppet.is_some() {
            scene.look.pose = self.shown_pose();
        }
        Some(scene)
    }

    /// ポーズの表示中の要素か選択 (シーンの `look.pose`) が、前に読んだときから変わったか (読むと戻る。操作ウィンドウと
    /// 外部 API の `pose.changed` に知らせる。T0848)。変わっていなければ確保しない (刻みごとに呼ぶ)。
    pub fn take_pose_changed(&mut self) -> bool {
        let selected: &[String] = match (&self.puppet, &self.scene) {
            (Some(_), Some(scene)) => &scene.look.pose,
            _ => &[],
        };
        let shown_same = match &self.puppet {
            Some(puppet) => puppet.pose().visible_parts().map(|p| p.as_str()).eq(self
                .pose_noticed
                .0
                .iter()
                .map(String::as_str)),
            None => self.pose_noticed.0.is_empty(),
        };
        if shown_same && self.pose_noticed.1 == selected {
            return false;
        }
        self.pose_noticed = (self.shown_pose(), selected.to_vec());
        true
    }

    /// 今のモデルのパラメータの定義 (定義順)。
    pub fn parameters(&self) -> &[Parameter] {
        self.puppet
            .as_ref()
            .map(|p| p.instance().compiled().model().parameters.as_slice())
            .unwrap_or_default()
    }

    /// 今のパラメータの値 (定義順。直前の刻みの後の値。物理は切っているので物理の出力は入らない)。
    pub fn values(&self) -> &[f32] {
        self.puppet
            .as_ref()
            .map(|p| p.instance().parameters().as_slice())
            .unwrap_or_default()
    }

    /// パラメータを書き換える (ホストの書き換え。次の刻みから基準値になる。値域にクランプする)。
    /// ID が無ければ `false`。
    pub fn set_parameter(&mut self, id: &str, value: f32) -> bool {
        let Some(puppet) = &mut self.puppet else {
            return false;
        };
        let params = puppet.instance_mut().parameters_mut();
        let Some(index) = params.index_of(id) else {
            return false;
        };
        params.set(index, value);
        true
    }

    /// 外部 API の値の上書き (T0805): 物理の段に入る直前の値を、`duration` 秒の間 `value` へ `weight` (0〜1) の割合で寄せる。
    /// トラッキング・モーション・表情・目パチ・呼吸より後に書くので、外部 API の値が勝つ (リップシンクは受け手が後で重ねる)。
    /// 同じパラメータの上書きは置き換える。値域にクランプする。ID が無ければ `false`。
    pub fn override_parameter(&mut self, id: &str, value: f32, weight: f32, duration: f32) -> bool {
        let Some(puppet) = &self.puppet else {
            return false;
        };
        let model = puppet.instance().compiled().model();
        let Some(index) = model.parameters.iter().position(|p| p.id == id) else {
            return false;
        };
        let p = &model.parameters[index];
        let entry = Override {
            index,
            value: value.clamp(p.min, p.max),
            weight: weight.clamp(0.0, 1.0),
            remaining: duration,
        };
        match self.overrides.iter_mut().find(|o| o.index == index) {
            Some(o) => *o = entry,
            None => self.overrides.push(entry),
        }
        true
    }

    /// 直前の刻みで配った値 (定義順。外部 API の上書きを含む)。まだ配っていなければ空。
    pub fn sent_values(&self) -> &[f32] {
        &self.sent
    }

    /// 刻みで発火したモーションのイベント (読むと空になる)。
    pub fn take_motion_events(&mut self) -> Vec<FiredEvent> {
        std::mem::take(&mut self.motion_events)
    }

    /// すべてのパラメータを既定値に戻す。
    pub fn reset_parameters(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.instance_mut().parameters_mut().reset();
        }
    }

    /// リップシンクの音量 (0〜1。マイクの入力、T0803)。`None` でリップシンクを当てない。
    pub fn set_lip_sync_level(&mut self, level: Option<f32>) {
        self.animation
            .set_lip_sync_level(self.puppet.as_mut(), level);
    }

    /// 演技 (モーション・表情・入力) を `action` で触る。`Puppet` はモデルが無ければ `None`。
    pub fn perform<T>(
        &mut self,
        action: impl FnOnce(&mut Animation, Option<&mut Puppet>) -> T,
    ) -> T {
        action(&mut self.animation, self.puppet.as_mut())
    }

    /// 演技の様子 (操作ウィンドウに出す)。
    pub fn animation_view(&self) -> AnimationView {
        self.animation.view(self.puppet.as_ref())
    }

    /// 演技の様子が変わったか (読むと戻る。刻みのスレッドが操作ウィンドウに知らせる)。
    pub fn take_animation_changed(&mut self) -> bool {
        self.animation.take_changed()
    }

    /// 受け手の物理を次のフレームでリセットさせる (着せ替えなど、モデルを読み直さずに形が変わるとき。T0802)。
    pub fn reset_physics(&mut self) {
        self.reset = true;
    }

    /// 1 刻み進め、配るフレームを返す。`dt` は 0〜[`MAX_DT`] に収める。モデルが無ければ何もせず `None`。
    pub fn tick(&mut self, dt: f32) -> Option<Frame> {
        let puppet = self.puppet.as_mut()?;
        let dt = if dt.is_finite() {
            dt.clamp(0.0, MAX_DT)
        } else {
            0.0
        };
        self.animation.before_update(puppet);
        puppet.update(dt);
        self.animation.after_update(puppet);
        // イベントは、発火したモーションの名前で知らせる (フェードアウト中のもの・この刻みで終わったものも、今のモーションの名前にしない)。
        let current = puppet.motions().current_motion_number();
        self.motion_events.extend(
            puppet
                .motion_event_sources()
                .iter()
                .zip(puppet.motion_events())
                .map(|(source, event)| FiredEvent {
                    motion: source.name.clone(),
                    current: Some(source.number) == current,
                    event: event.clone(),
                }),
        );
        let mut values = puppet.values_before_physics().to_vec();
        for o in &mut self.overrides {
            if let Some(v) = values.get_mut(o.index) {
                *v += (o.value - *v) * o.weight;
            }
            o.remaining -= dt;
        }
        self.overrides.retain(|o| o.remaining > 0.0);
        self.sent.clone_from(&values);
        let frame = Frame {
            seq: self.next_seq,
            revision: self.revision,
            dt,
            reset: std::mem::take(&mut self.reset),
            values,
            lip_sync: puppet.lip_sync_applied(),
            part_opacities: puppet.part_opacities_applied().to_vec(),
        };
        self.next_seq += 1;
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;

    use super::*;

    fn bundle() -> Bundle {
        let mut bundle = Bundle::new(fixtures::physics_hair());
        bundle.physics = Some(fixtures::physics_hair_settings());
        bundle
    }

    #[test]
    fn frames_count_up_across_scenes() {
        let mut conductor = Conductor::new();
        assert!(conductor.tick(0.016).is_none());
        let scene = conductor.load("hair.ugoe", bundle()).unwrap().clone();
        assert_eq!(scene.revision, 1);
        assert_eq!(scene.parameter_ids.len(), conductor.parameters().len());
        let a = conductor.tick(0.016).unwrap();
        let b = conductor.tick(0.016).unwrap();
        assert_eq!((a.seq, b.seq), (0, 1));
        assert_eq!(a.values.len(), scene.parameter_ids.len());
        conductor.load("hair.ugoe", bundle()).unwrap();
        let c = conductor.tick(0.016).unwrap();
        assert_eq!((c.seq, c.revision), (2, 2));
    }

    #[test]
    fn dt_is_clamped() {
        let mut conductor = Conductor::new();
        conductor.load("hair.ugoe", bundle()).unwrap();
        assert_eq!(conductor.tick(3.0).unwrap().dt, MAX_DT);
        assert_eq!(conductor.tick(-1.0).unwrap().dt, 0.0);
        assert_eq!(conductor.tick(f32::NAN).unwrap().dt, 0.0);
    }

    #[test]
    fn the_conductor_does_not_run_physics() {
        let mut conductor = Conductor::new();
        conductor.load("hair.ugoe", bundle()).unwrap();
        assert!(!conductor.puppet.as_ref().unwrap().toggles().physics);
    }

    #[test]
    fn reset_rides_on_the_next_frame_only() {
        let mut conductor = Conductor::new();
        conductor.load("hair.ugoe", bundle()).unwrap();
        conductor.reset_physics();
        assert!(conductor.tick(0.016).unwrap().reset);
        assert!(!conductor.tick(0.016).unwrap().reset);
    }

    #[test]
    fn host_writes_become_base_values() {
        let mut conductor = Conductor::new();
        conductor.load("hair.ugoe", bundle()).unwrap();
        assert!(conductor.set_parameter("AngleX", 12.0));
        assert!(!conductor.set_parameter("Nope", 1.0));
        let frame = conductor.tick(0.016).unwrap();
        let index = conductor
            .parameters()
            .iter()
            .position(|p| p.id == "AngleX")
            .unwrap();
        assert_eq!(frame.values[index], 12.0);
        conductor.reset_parameters();
        let frame = conductor.tick(0.016).unwrap();
        assert_eq!(frame.values[index], conductor.parameters()[index].default);
    }

    #[test]
    fn reads_bundles_and_json() {
        let mut source = bundle();
        // 書き出しはテクスチャの数と先頭のシグネチャだけを確かめる (中身はデコードしない)。
        source.textures = vec![
            ugoe::format::TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
            };
            source.model.textures.len()
        ];
        let bytes = ugoe::format::write_bundle(&source).unwrap();
        assert!(read_model(&bytes).unwrap().physics.is_some());
        let json = ugoe::format::model_to_json(&fixtures::single_quad()).unwrap();
        assert!(read_model(format!("  {json}").as_bytes()).is_ok());
        assert!(read_model(b"nope").is_err());
    }

    fn dress_up() -> Bundle {
        Bundle::new(fixtures::dress_up_base())
    }

    #[test]
    fn switching_within_a_model_keeps_values_and_versions_what_changed() {
        let mut conductor = Conductor::new();
        let hair = Arc::new(fixtures::long_hair_module());
        let model = assemble(
            NewModel {
                name: "base.ugoe".into(),
                bundle: dress_up(),
            },
            std::slice::from_ref(&hair),
        )
        .unwrap();
        let first = conductor
            .present(Some(model), Some(vec![Arc::clone(&hair)]), Look::default())
            .unwrap()
            .clone();
        assert_eq!(
            (first.revision, first.model_revision, first.modules_revision),
            (1, 1, 1)
        );
        assert_eq!(first.modules, std::slice::from_ref(&hair.name));
        // モデルを読み直したときは、物理のリセットを載せない (受け手も読み直す)。
        assert!(!conductor.tick(0.016).unwrap().reset);
        assert!(conductor.set_parameter("AngleX", 20.0));

        // 見せ方だけ: モデルとモジュールの版はそのまま、値も引き継ぐ。
        let look = Look {
            transform: crate::profile::Transform {
                x: 0.25,
                ..Default::default()
            },
            ..Look::default()
        };
        let second = conductor
            .present(None, Some(vec![Arc::clone(&hair)]), look.clone())
            .unwrap()
            .clone();
        assert_eq!(
            (
                second.revision,
                second.model_revision,
                second.modules_revision
            ),
            (2, 1, 1)
        );
        assert_eq!(second.look, look);
        let frame = conductor.tick(0.016).unwrap();
        assert!(!frame.reset);
        let index = second
            .parameter_ids
            .iter()
            .position(|id| id == "AngleX")
            .unwrap();
        assert_eq!(frame.values[index], 20.0);

        // 着せ替え: モジュールの版が上がり、次のフレームで受け手の物理をリセットする。
        let dress = Arc::new(fixtures::dress_module());
        let third = conductor
            .present(None, Some(vec![hair, Arc::clone(&dress)]), Look::default())
            .unwrap()
            .clone();
        assert_eq!(
            (third.revision, third.model_revision, third.modules_revision),
            (3, 1, 3)
        );
        assert_eq!(third.parameter_ids, first.parameter_ids);
        let frame = conductor.tick(0.016).unwrap();
        assert!(frame.reset);
        assert_eq!((frame.revision, frame.values[index]), (3, 20.0));
    }

    fn visible_parts(conductor: &Conductor) -> Vec<String> {
        let pose = conductor.puppet.as_ref().unwrap().pose();
        pose.visible_parts()
            .map(|p| p.as_str().to_owned())
            .collect()
    }

    /// シーンのポーズの要素を指揮役のパペットに表示させ、モデルにある要素と先頭の要素で埋め直して配る (T0842)。
    #[test]
    fn presenting_shows_the_scenes_pose() {
        let mut conductor = Conductor::new();
        let model = assemble(
            NewModel {
                name: "pose.ugoe".into(),
                bundle: Bundle::new(fixtures::pose_and_effects()),
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            conductor.pose_groups(),
            Vec::<PoseGroupInfo>::new(),
            "no model yet"
        );
        let look = Look {
            pose: vec!["Nope".into(), "ArmB".into()],
            ..Look::default()
        };
        let scene = conductor.present(Some(model), None, look).unwrap();
        // モデルに無い要素は捨てる。
        assert_eq!(scene.look.pose, ["ArmB"]);
        assert_eq!(visible_parts(&conductor), ["ArmB"]);
        assert_eq!(
            conductor.pose_groups(),
            [PoseGroupInfo {
                parts: vec!["ArmA".into(), "ArmB".into()],
                module: None
            }]
        );
        // 同じグループの要素がいくつもあれば後のもの。
        let look = Look {
            pose: vec!["ArmB".into(), "ArmA".into()],
            ..Look::default()
        };
        assert_eq!(
            conductor.present(None, None, look).unwrap().look.pose,
            ["ArmA"]
        );
        conductor
            .present(
                None,
                None,
                Look {
                    pose: vec!["ArmB".into()],
                    ..Look::default()
                },
            )
            .unwrap();
        // 挙がっていないグループは先頭の要素に戻す (シーンが姿を決める)。
        assert_eq!(
            conductor
                .present(None, None, Look::default())
                .unwrap()
                .look
                .pose,
            ["ArmA"]
        );
        assert_eq!(visible_parts(&conductor), ["ArmA"]);
        // モデルの無いシーンには要素が無い。
        let cleared = conductor.clear(Look {
            pose: vec!["ArmB".into()],
            ..Look::default()
        });
        assert!(cleared.look.pose.is_empty());
    }

    /// モデルを読み直したシーンは選んだ要素からフェードせずに始め、同じモデルのままの切り替えはフェードする (T0844)。
    #[test]
    fn a_reloaded_scene_shows_its_pose_without_a_fade() {
        let opacity = |conductor: &Conductor, id: &str| {
            let pose = conductor.puppet.as_ref().unwrap().pose();
            pose.opacities()
                .find(|(part, _)| *part == id)
                .map(|(_, o)| o)
                .unwrap()
        };
        let model = || {
            assemble(
                NewModel {
                    name: "pose.ugoe".into(),
                    bundle: Bundle::new(fixtures::pose_and_effects()),
                },
                &[],
            )
            .unwrap()
        };
        let look = |part: &str| Look {
            pose: vec![part.into()],
            ..Look::default()
        };
        let mut conductor = Conductor::new();
        conductor
            .present(Some(model()), Some(Vec::new()), look("ArmB"))
            .unwrap();
        assert_eq!(
            (opacity(&conductor, "ArmA"), opacity(&conductor, "ArmB")),
            (0.0, 1.0)
        );
        // 同じモデルのまま: 次の刻みからフェードで切り替わる。
        conductor.present(None, None, look("ArmA")).unwrap();
        assert_eq!(opacity(&conductor, "ArmB"), 1.0);
        conductor.tick(1.0 / 60.0).unwrap();
        let fading = opacity(&conductor, "ArmA");
        assert!(fading > 0.0 && fading < 1.0, "{fading}");
        // 読み直し: すぐに選んだ要素になる。
        conductor
            .present(Some(model()), Some(Vec::new()), look("ArmB"))
            .unwrap();
        assert_eq!(
            (opacity(&conductor, "ArmA"), opacity(&conductor, "ArmB")),
            (0.0, 1.0)
        );
    }

    /// 着せ替えの直後 (モジュールのポーズのグループが現れた・組が変わった) のシーンは、選んだ要素からフェードせずに始め、
    /// 同じ組のままの切り替えはフェードする (T0850)。
    #[test]
    fn a_dressed_scene_shows_the_modules_pose_without_a_fade() {
        let opacity = |conductor: &Conductor, id: &str| {
            let pose = conductor.puppet.as_ref().unwrap().pose();
            pose.opacities()
                .find(|(part, _)| *part == id)
                .map(|(_, o)| o)
                .unwrap()
        };
        let jacket = |conductor: &Conductor| {
            (
                opacity(conductor, "Jacket/Closed"),
                opacity(conductor, "Jacket/Open"),
            )
        };
        let look = |part: &str| Look {
            pose: vec![part.into()],
            ..Look::default()
        };
        let model = assemble(
            NewModel {
                name: "dress_up.ugoe".into(),
                bundle: dress_up(),
            },
            &[],
        )
        .unwrap();
        let mut conductor = Conductor::new();
        conductor
            .present(Some(model), None, Look::default())
            .unwrap();
        // モジュールを着せる: 選んだ要素 (Open) がすぐに出る。
        let scene = conductor
            .present(
                None,
                Some(vec![Arc::new(fixtures::jacket_module())]),
                look("Jacket/Open"),
            )
            .unwrap();
        assert_eq!(scene.modules_revision, scene.revision);
        assert_eq!(scene.look.pose, ["Jacket/Open"]);
        assert_eq!(jacket(&conductor), (0.0, 1.0));
        conductor.tick(1.0 / 60.0).unwrap();
        assert_eq!(jacket(&conductor), (0.0, 1.0));
        // 同じ組のまま: 次の刻みからフェードで切り替わる。
        conductor
            .present(None, None, look("Jacket/Closed"))
            .unwrap();
        conductor.tick(1.0 / 60.0).unwrap();
        let (fading, _) = jacket(&conductor);
        assert!(fading > 0.0 && fading < 1.0, "{fading}");
        // 組を替える (ほかのモジュールを足す): フェードの途中でも、すぐに選んだ要素になる。
        let modules = vec![
            Arc::new(fixtures::long_hair_module()),
            Arc::new(fixtures::jacket_module()),
        ];
        conductor
            .present(None, Some(modules), look("Jacket/Open"))
            .unwrap();
        assert_eq!(jacket(&conductor), (0.0, 1.0));
    }

    /// `ArmB` を表示させるモーション `wave` (0.5 秒。`pose_and_effects` の `ArmA` と `ArmB` のパーツの不透明度のカーブ)。
    fn wave() -> Motion {
        use ugoe::model::motion::{Curve, CurveTarget, Point, Segment};
        let curve = |part: &str, v: f32| Curve {
            target: CurveTarget::PartOpacity(part.into()),
            segments: vec![
                Segment::Start {
                    p: Point { t: 0.0, v },
                },
                Segment::Linear {
                    to: Point { t: 0.5, v },
                },
            ],
            fade_in: None,
            fade_out: None,
        };
        Motion {
            name: "wave".into(),
            duration: 0.5,
            looping: false,
            fade_in: 0.0,
            fade_out: 0.0,
            curves: vec![curve("ArmA", 0.0), curve("ArmB", 1.0)],
            events: vec![],
        }
    }

    /// 表示中の要素はモーションの切り替えに追従し、選択 (シーンの `look.pose`) は変わらない。変化は 1 度だけ知らせ、
    /// 途中から来た受け手に送るシーンは表示中の要素を持つ (T0848)。
    #[test]
    fn the_shown_pose_follows_motions_and_is_noticed_once() {
        let mut bundle = Bundle::new(fixtures::pose_and_effects());
        bundle.motions.push(wave());
        let mut conductor = Conductor::new();
        assert!(!conductor.take_pose_changed(), "nothing to tell yet");
        assert!(conductor.shown_pose().is_empty());
        assert!(conductor.joining_scene().is_none());
        conductor.load("pose", bundle).unwrap();
        assert_eq!(conductor.shown_pose(), ["ArmA"]);
        assert!(conductor.take_pose_changed());
        assert!(!conductor.take_pose_changed());
        let played = conductor.perform(|animation, puppet| {
            animation.play(puppet.unwrap(), "wave", ugoe::motion::Priority::Normal)
        });
        assert_eq!(played, Ok(true));
        let mut noticed = 0;
        for _ in 0..60 {
            conductor.tick(1.0 / 60.0).unwrap();
            noticed += usize::from(conductor.take_pose_changed());
        }
        // モーションが終わっても、モーションが表示させた要素が残る。
        assert_eq!(noticed, 1);
        assert_eq!(conductor.shown_pose(), ["ArmB"]);
        assert_eq!(conductor.scene().unwrap().look.pose, ["ArmA"]);
        assert_eq!(conductor.joining_scene().unwrap().look.pose, ["ArmB"]);
        // シーンを出し直すと選択に戻る (指揮役と受け手で揃う)。
        conductor.present(None, None, Look::default()).unwrap();
        assert_eq!(conductor.shown_pose(), ["ArmA"]);
        assert!(conductor.take_pose_changed());
        // 表示している要素を選んだ (選択だけが変わった) ときも知らせる。
        conductor
            .perform(|animation, puppet| {
                animation.play(puppet.unwrap(), "wave", ugoe::motion::Priority::Normal)
            })
            .unwrap();
        for _ in 0..60 {
            conductor.tick(1.0 / 60.0).unwrap();
        }
        assert!(conductor.take_pose_changed());
        let look = Look {
            pose: vec!["ArmB".into()],
            ..Look::default()
        };
        conductor.present(None, None, look).unwrap();
        assert_eq!(conductor.shown_pose(), ["ArmB"]);
        assert!(conductor.take_pose_changed());
        // モデルを外すと空になる。
        conductor.clear(Look::default());
        assert!(conductor.take_pose_changed());
        assert!(conductor.shown_pose().is_empty());
        assert!(conductor.joining_scene().unwrap().look.pose.is_empty());
    }

    #[test]
    fn a_failed_switch_changes_nothing() {
        let mut conductor = Conductor::new();
        assert!(matches!(
            conductor.present(None, None, Look::default()),
            Err(LoadError::NoModel)
        ));
        conductor.load("hair.ugoe", bundle()).unwrap();
        // 本体にスロットが無いのでモジュールを付けられない。
        let error = conductor.present(
            None,
            Some(vec![Arc::new(fixtures::long_hair_module())]),
            Look::default(),
        );
        assert!(matches!(error, Err(LoadError::DressUp(_))));
        let scene = conductor.scene().unwrap();
        assert_eq!((scene.revision, scene.modules.len()), (1, 0));
        assert!(conductor.modules().is_empty());
    }

    #[test]
    fn clearing_stops_the_frames() {
        let mut conductor = Conductor::new();
        conductor.load("hair.ugoe", bundle()).unwrap();
        let scene = conductor.clear(Look::default()).clone();
        assert_eq!((scene.revision, scene.model_revision), (2, 0));
        assert!(scene.parameter_ids.is_empty());
        assert!(conductor.tick(0.016).is_none());
        assert!(conductor.parameters().is_empty());
    }
}
