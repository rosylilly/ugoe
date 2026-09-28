//! 振り子チェーンによる物理演算。仕様: `docs/spec/physics.md`。
//!
//! 入力パラメータ (顔の角度など) から根元の変位を作り、粒子チェーンを固定ステップで
//! シミュレーションし、セグメント角度を出力パラメータ (髪揺れなど) に書き戻す。

pub mod chain;

use ugoe_core::{Vec2, lerp};
use ugoe_model::physics::{NormalizationRange, PhysicsRig, PhysicsSettings, PhysicsSource};
use ugoe_runtime::{ParamIndex, ParameterValues};

pub use chain::{Chain, Particle, StepInput};

/// 1 回の `update` で実行するステップ数の上限 (`docs/spec/physics.md#時間の進め方`)。
pub const MAX_STEPS_PER_UPDATE: u32 = 8;

/// 係数の時間の基準となるフレームレート。`mobility` と `delay` は 1/30 秒あたりの値。
pub const BASE_FPS: f32 = 30.0;

/// `PhysicsSettings::fps` が正の有限値でないときに使うステップの周波数。
pub const DEFAULT_FPS: f32 = 60.0;

/// 物理演算の実行時状態。
#[derive(Debug)]
pub struct PhysicsSystem {
    settings: PhysicsSettings,
    rigs: Vec<RigState>,
    accumulator: f32,
    /// 最初の `update` で静止姿勢に置いたか。
    initialized: bool,
}

/// 物理リグ 1 つ分の実行時状態。
#[derive(Debug)]
struct RigState {
    chain: Chain,
    /// 最後に実行したステップの入力 (根元の姿勢)。
    pose: RootPose,
    /// 直前 2 ステップの出力値 (出力ごと。無効な出力は `None`)。
    previous: Vec<Option<f32>>,
    current: Vec<Option<f32>>,
}

/// 入力から求めた根元の姿勢。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct RootPose {
    translation: Vec2,
    /// 度、時計回りが正。
    angle: f32,
}

impl RootPose {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            angle: lerp(self.angle, other.angle, t),
        }
    }
}

impl PhysicsSystem {
    pub fn new(settings: PhysicsSettings) -> Self {
        let rigs = settings
            .rigs
            .iter()
            .map(|rig| RigState {
                chain: Chain::new(&rig.particles, Vec2::ZERO),
                pose: RootPose::default(),
                previous: vec![None; rig.outputs.len()],
                current: vec![None; rig.outputs.len()],
            })
            .collect();
        Self {
            settings,
            rigs,
            accumulator: 0.0,
            initialized: false,
        }
    }

    pub fn settings(&self) -> &PhysicsSettings {
        &self.settings
    }

    /// リグ `rig` (`PhysicsSettings::rigs` の添字) の粒子チェーン。
    pub fn chain(&self, rig: usize) -> Option<&Chain> {
        self.rigs.get(rig).map(|r| &r.chain)
    }

    /// 1 ステップの長さ (秒)。
    pub fn step_duration(&self) -> f32 {
        let fps = self.settings.fps;
        1.0 / if fps.is_finite() && fps > 0.0 {
            fps
        } else {
            DEFAULT_FPS
        }
    }

    /// `dt` 秒進め、出力パラメータを書き換える。
    ///
    /// 最初の呼び出し (と [`Self::reset`] の後) では、その時点の入力で粒子を静止姿勢に置く。
    pub fn update(&mut self, parameters: &mut ParameterValues, dt: f32) {
        let step = self.step_duration();
        let time_scale = step * BASE_FPS;
        let targets: Vec<RootPose> = self
            .settings
            .rigs
            .iter()
            .map(|rig| root_pose(rig, parameters))
            .collect();

        if !self.initialized {
            self.initialized = true;
            self.accumulator = 0.0;
            for ((state, rig), target) in
                self.rigs.iter_mut().zip(&self.settings.rigs).zip(&targets)
            {
                let gravity = rotate_deg(self.settings.gravity, target.angle);
                state.chain.settle(
                    &rig.particles,
                    target.translation,
                    gravity,
                    self.settings.wind,
                );
                state.pose = *target;
                state.current = outputs(rig, &state.chain, self.settings.gravity);
                state.previous.clone_from(&state.current);
            }
        }

        if dt.is_finite() && dt > 0.0 {
            self.accumulator += dt;
        }
        let whole = (self.accumulator / step).floor();
        self.accumulator -= whole * step;
        // ステップ数の上限を超えた分の時間は捨てる。
        let steps = (whole as u32).min(MAX_STEPS_PER_UPDATE);

        if steps > 0 {
            for ((state, rig), target) in
                self.rigs.iter_mut().zip(&self.settings.rigs).zip(&targets)
            {
                let start = state.pose;
                for k in 1..=steps {
                    // 入力はフレームの間で線形補間して、ステップごとに少しずつ動かす。
                    let pose = start.lerp(*target, k as f32 / steps as f32);
                    let input = StepInput {
                        root: pose.translation,
                        rotation_delta: pose.angle - state.pose.angle,
                        gravity: rotate_deg(self.settings.gravity, pose.angle),
                        wind: self.settings.wind,
                        time_scale,
                    };
                    state.chain.step(&rig.particles, &input, |_, _| {});
                    state.pose = pose;
                    std::mem::swap(&mut state.previous, &mut state.current);
                    state.current = outputs(rig, &state.chain, self.settings.gravity);
                }
            }
        }

        let alpha = (self.accumulator / step).clamp(0.0, 1.0);
        for (state, rig) in self.rigs.iter().zip(&self.settings.rigs) {
            write_outputs(rig, &state.previous, &state.current, alpha, parameters);
        }
    }

    /// 粒子を初期姿勢に戻す (モデル切替・テレポート時)。
    /// 次の `update` で、その時点の入力に対する静止姿勢に置き直す。
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.initialized = false;
        for (state, rig) in self.rigs.iter_mut().zip(&self.settings.rigs) {
            state.chain = Chain::new(&rig.particles, Vec2::ZERO);
            state.pose = RootPose::default();
            state.previous.fill(None);
            state.current.fill(None);
        }
    }
}

/// `v` を `degrees` 度 (時計回りが正) 回す。
fn rotate_deg(v: Vec2, degrees: f32) -> Vec2 {
    Vec2::from_angle(degrees.to_radians()).rotate(v)
}

/// 入力パラメータから根元の姿勢を求める (`docs/spec/physics.md#入力`)。
fn root_pose(rig: &PhysicsRig, parameters: &ParameterValues) -> RootPose {
    let mut pose = RootPose::default();
    for input in &rig.inputs {
        let Some(index) = parameters.index_of(input.parameter.as_str()) else {
            continue;
        };
        let range = match input.source {
            PhysicsSource::X | PhysicsSource::Y => rig.normalization.position,
            PhysicsSource::Angle => rig.normalization.angle,
            PhysicsSource::Unknown => continue,
        };
        let mut n = normalize(parameters, index, range);
        if input.reflect {
            n = -n;
        }
        let v = n * input.weight / 100.0;
        match input.source {
            PhysicsSource::X => pose.translation.x += v,
            PhysicsSource::Y => pose.translation.y += v,
            PhysicsSource::Angle => pose.angle += v,
            PhysicsSource::Unknown => {}
        }
    }
    pose
}

/// パラメータの値を、default の両側で別々に線形写像して `range` に写す。
fn normalize(parameters: &ParameterValues, index: ParamIndex, range: NormalizationRange) -> f32 {
    let value = parameters.get(index);
    let min = parameters.min(index);
    let max = parameters.max(index);
    let default = parameters.default_value(index);
    if value > default && max > default {
        let t = ((value - default) / (max - default)).min(1.0);
        lerp(range.default, range.max, t)
    } else if value < default && default > min {
        let t = ((default - value) / (default - min)).min(1.0);
        lerp(range.default, range.min, t)
    } else {
        range.default
    }
}

/// 現在の粒子から、出力ごとの値 (`scale` と `reflect` を適用済み) を求める
/// (`docs/spec/physics.md#出力`)。粒子の添字が不正な出力は `None`。
fn outputs(rig: &PhysicsRig, chain: &Chain, gravity: Vec2) -> Vec<Option<f32>> {
    let particles = chain.particles();
    rig.outputs
        .iter()
        .map(|output| {
            let i = output.particle as usize;
            if i == 0 || i >= particles.len() {
                return None;
            }
            let segment = particles[i].position - particles[i - 1].position;
            let source = match output.source {
                PhysicsSource::X => segment.x,
                PhysicsSource::Y => segment.y,
                PhysicsSource::Angle => {
                    let parent = if i >= 2 {
                        particles[i - 1].position - particles[i - 2].position
                    } else {
                        gravity
                    };
                    chain::signed_angle(parent, segment)
                }
                PhysicsSource::Unknown => return None,
            };
            let value = source * output.scale;
            Some(if output.reflect { -value } else { value })
        })
        .collect()
}

/// 直前 2 ステップの出力を `alpha` で補間し、パラメータに重み付きで書き込む。
fn write_outputs(
    rig: &PhysicsRig,
    previous: &[Option<f32>],
    current: &[Option<f32>],
    alpha: f32,
    parameters: &mut ParameterValues,
) {
    for ((output, previous), current) in rig.outputs.iter().zip(previous).zip(current) {
        let (Some(previous), Some(current)) = (*previous, *current) else {
            continue;
        };
        let Some(index) = parameters.index_of(output.parameter.as_str()) else {
            continue;
        };
        let value =
            lerp(previous, current, alpha).clamp(parameters.min(index), parameters.max(index));
        parameters.blend(index, value, (output.weight / 100.0).clamp(0.0, 1.0));
    }
}

#[cfg(test)]
mod tests;
