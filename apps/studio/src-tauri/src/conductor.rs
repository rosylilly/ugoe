//! 指揮役 (conductor): パラメータの値を決める唯一の場所 (ADR 0028 の 3.、`docs/studio.md#全体の構成`)。
//!
//! ネイティブの [`Puppet`] を 1 つ持ち、**物理の段を切って** `update(dt)` する (描かない)。配るのは物理の段に入る直前の値
//! (`Puppet::values_before_physics`) とリップシンクの入力 (`Puppet::lip_sync_applied`) で、受け手はそれを基準値として書き、
//! 物理 → リップシンク → ポーズの順に自分で進める (`docs/spec/motion.md#受け手として進める`、T0134)。
//! 時計とスレッドは持たない (刻みは `lib.rs` の刻みのスレッドが `dt` を測って [`Conductor::tick`] を呼ぶ)。

use ugoe::format::Bundle;
use ugoe::model::Parameter;
use ugoe::{Puppet, Toggles};

use crate::stream::{Frame, Scene};

/// 1 刻みで進める時間の上限 (秒)。止まっていた後に物理が大きく飛ばないように (ADR 0028 の 3.)。
pub const MAX_DT: f32 = 0.1;

/// モデルを読めない。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error(transparent)]
    Compile(#[from] ugoe::runtime::CompileError),
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
#[derive(Debug, Default)]
pub struct Conductor {
    puppet: Option<Puppet>,
    scene: Option<Scene>,
    /// 次に配るフレームの `seq`。
    next_seq: u64,
    revision: u32,
    /// 次のフレームに物理のリセットを載せるか。
    reset: bool,
}

impl Conductor {
    pub fn new() -> Self {
        Self::default()
    }

    /// モデルを読み、新しいシーンにする (`revision` が上がる)。パラメータは既定値から始める。
    ///
    /// 物理の設定は持たせたうえで段を切る (受け手が回す)。モーション・表情の一覧は T0803 で使う。
    pub fn load(&mut self, model_name: &str, bundle: Bundle) -> Result<&Scene, LoadError> {
        let mut puppet = Puppet::new(bundle.model)?;
        if let Some(physics) = bundle.physics {
            puppet = puppet.with_physics(physics);
        }
        puppet.set_toggles(Toggles {
            physics: false,
            ..Toggles::default()
        });
        self.revision += 1;
        let parameter_ids = puppet
            .instance()
            .compiled()
            .model()
            .parameters
            .iter()
            .map(|p| p.id.to_string())
            .collect();
        self.puppet = Some(puppet);
        // 受け手は新しいモデルを読み直すので、物理は新しく始まる (リセットを載せなくてよい)。
        self.reset = false;
        Ok(self.scene.insert(Scene {
            revision: self.revision,
            model_name: model_name.to_owned(),
            parameter_ids,
        }))
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

    /// すべてのパラメータを既定値に戻す。
    pub fn reset_parameters(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.instance_mut().parameters_mut().reset();
        }
    }

    /// リップシンクの音量 (0〜1。マイクの入力、T0803)。`None` でリップシンクを当てない。
    pub fn set_lip_sync_level(&mut self, level: Option<f32>) {
        if let Some(puppet) = &mut self.puppet {
            puppet.set_lip_sync_level(level);
        }
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
        puppet.update(dt);
        let frame = Frame {
            seq: self.next_seq,
            revision: self.revision,
            dt,
            reset: std::mem::take(&mut self.reset),
            values: puppet.values_before_physics().to_vec(),
            lip_sync: puppet.lip_sync_applied(),
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
}
