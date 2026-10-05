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
    /// - 見せ方 (`look`) は受け手に配るだけで、指揮役は使わない。
    ///
    /// モデルが無ければ ([`Conductor::clear`] の後など) `model` が要る。
    pub fn present(
        &mut self,
        model: Option<Assembled>,
        modules: Option<Vec<Arc<Module>>>,
        look: Look,
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
        let Some(puppet) = &self.puppet else {
            return Err(LoadError::NoModel);
        };
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
    pub fn clear(&mut self, look: Look) -> &Scene {
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

    /// 今のシーン。モデルを読んでいなければ `None`。
    pub fn scene(&self) -> Option<&Scene> {
        self.scene.as_ref()
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
        let Some(index) = model.parameters.iter().position(|p| p.id.as_str() == id) else {
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
            .position(|p| p.id.as_str() == "AngleX")
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
