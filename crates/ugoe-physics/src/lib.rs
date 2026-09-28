//! 振り子チェーンによる物理演算。仕様: `docs/spec/physics.md`。
//!
//! 入力パラメータ (顔の角度など) から根元の変位を作り、粒子チェーンを固定ステップで
//! シミュレーションし、セグメント角度を出力パラメータ (髪揺れなど) に書き戻す。
//!
//! 動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`) を使う設定では、
//! [`PhysicsSystem::update_instance`] がモデルの 1 段目の評価を読み、デフォーマ・頂点に出力する。

pub mod chain;
pub mod collider;

use std::sync::Arc;

use ugoe_core::{Vec2, lerp};
use ugoe_model::DeformerKind;
use ugoe_model::physics::{
    NormalizationRange, PhysicsBlend, PhysicsOutput, PhysicsRig, PhysicsSettings, PhysicsSource,
    PhysicsTarget, VertexOwner,
};
use ugoe_runtime::{CompiledModel, ModelInstance, ParamIndex, ParameterValues};

pub use chain::{Chain, Particle, StepInput};
pub use collider::WorldCollider;

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
    /// 動的物理の機能 (パラメータ以外の出力先・衝突体・重力の基準) を使うリグがあるか。
    dynamic: bool,
    /// 前のリグの出力を入力に読むリグ (中継) があるか (`docs/spec/physics.md#リグの順序と中継`)。
    relay: bool,
    /// モデルの要素への参照を解決したもの (動的物理でだけ使う)。
    binding: Option<Binding>,
}

/// 物理リグ 1 つ分の実行時状態。
#[derive(Debug)]
struct RigState {
    chain: Chain,
    /// 最後に実行したステップの入力 (根元の姿勢)。中継の入力の分を含む。
    pose: RootPose,
    /// `pose` のうち、フレームの始めのパラメータから求めた分 (中継の入力を除く)。
    /// フレームの間の補間はこの姿勢で行う。
    external: RootPose,
    /// 入力ごとに、前のリグが出力するパラメータを読む (中継の入力) か。
    relay_inputs: Vec<bool>,
    /// 直前 2 ステップの出力値 (出力ごと。無効な出力は `None`)。スカラーの出力先では `x` だけを使う。
    previous: Vec<Option<Vec2>>,
    current: Vec<Option<Vec2>>,
}

/// 入力から求めた根元の姿勢。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct RootPose {
    translation: Vec2,
    /// 重力を回す角度 (度、時計回りが正)。入力の角度と基準のデフォーマの回転の和。
    angle: f32,
    /// 基準のデフォーマの回転 (度)。風はこの角度だけ回す。
    frame_angle: f32,
}

impl RootPose {
    /// 中継の入力から求めた平行移動と角度を足す (基準のデフォーマの回転は足さない)。
    fn with_relay(mut self, relay: RootPose) -> Self {
        self.translation += relay.translation;
        self.angle += relay.angle;
        self
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            angle: lerp(self.angle, other.angle, t),
            frame_angle: lerp(self.frame_angle, other.frame_angle, t),
        }
    }
}

/// 1 段目の評価から求めた、リグ 1 つ分のそのフレームの状態。
/// 動的物理を使わないときは [`RigFrame::canvas`] (キャンバスに固定、衝突体なし)。
#[derive(Debug, Clone)]
struct RigFrame {
    /// 根元の位置のずれ (`アンカーで写した粒子 0 の位置 - 粒子 0 の位置`)。
    anchor: Vec2,
    /// 基準のデフォーマの回転 (度)。
    frame_angle: f32,
    /// 出力ごとの、粒子 1 の角度を測る基準の向き。
    references: Vec<Vec2>,
    /// 出力ごとに、出力先の親の座標系が裏返っているか (角度の符号を反転する)。
    flips: Vec<bool>,
    colliders: Vec<WorldCollider>,
}

impl RigFrame {
    fn canvas(rig: &PhysicsRig, gravity: Vec2) -> Self {
        Self {
            anchor: Vec2::ZERO,
            frame_angle: 0.0,
            references: vec![gravity; rig.outputs.len()],
            flips: vec![false; rig.outputs.len()],
            colliders: Vec::new(),
        }
    }
}

/// モデルの要素への参照を解決したもの。
#[derive(Debug)]
struct Binding {
    /// 解決に使ったモデル (別のモデルのインスタンスが渡されたら解決し直す)。
    compiled: Arc<CompiledModel>,
    rigs: Vec<RigBinding>,
    /// 衝突体ごとの取り付け先 (`Model::deformers` の添字)。取り付け先が見つからない衝突体は `Err`。
    colliders: Vec<Result<Option<usize>, ()>>,
}

#[derive(Debug)]
struct RigBinding {
    /// 根元を取り付けるデフォーマ (`docs/spec/physics.md#根元の取り付け先`)。
    anchor: Option<usize>,
    /// 重力と風の基準のデフォーマ。
    frame: Option<usize>,
    /// 衝突する衝突体 (`PhysicsSettings::colliders` の添字)。
    colliders: Vec<usize>,
    outputs: Vec<Target>,
}

/// 解決した出力先。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    /// 参照先が見つからない・種類が合わない・未知の出力先。
    Ignore,
    Parameter(ParamIndex),
    Angle(usize),
    Offset(usize),
    Warp(usize),
    Mesh(usize),
}

impl PhysicsSystem {
    pub fn new(settings: PhysicsSettings) -> Self {
        let rigs: Vec<RigState> = settings
            .rigs
            .iter()
            .enumerate()
            .map(|(k, rig)| RigState {
                chain: Chain::new(&rig.particles, Vec2::ZERO),
                pose: RootPose::default(),
                external: RootPose::default(),
                relay_inputs: relay_inputs(&settings.rigs[..k], rig),
                previous: vec![None; rig.outputs.len()],
                current: vec![None; rig.outputs.len()],
            })
            .collect();
        let dynamic = settings.rigs.iter().any(is_dynamic_rig);
        let relay = rigs.iter().any(|r| r.relay_inputs.contains(&true));
        Self {
            settings,
            rigs,
            accumulator: 0.0,
            initialized: false,
            dynamic,
            relay,
            binding: None,
        }
    }

    pub fn settings(&self) -> &PhysicsSettings {
        &self.settings
    }

    /// 動的物理の機能 (パラメータ以外の出力先・衝突体・重力の基準) を使うリグがあるか。
    ///
    /// `false` なら、[`Self::update_instance`] は [`Self::update`] と同じで、評価は 1 回で済む。
    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }

    /// 前のリグが出力するパラメータを入力に読むリグ (中継) があるか
    /// (`docs/spec/physics.md#リグの順序と中継`)。
    pub fn has_relay(&self) -> bool {
        self.relay
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
    /// 動的物理の機能は使わない (根元はキャンバスに固定し、衝突体と重力の基準を無視し、
    /// パラメータ以外への出力を捨てる)。動的物理には [`Self::update_instance`] を使う。
    pub fn update(&mut self, parameters: &mut ParameterValues, dt: f32) {
        let gravity = self.settings.gravity;
        let frames: Vec<RigFrame> = self
            .settings
            .rigs
            .iter()
            .map(|rig| RigFrame::canvas(rig, gravity))
            .collect();
        let alpha = self.advance(&frames, parameters, dt);
        for (state, rig) in self.rigs.iter().zip(&self.settings.rigs) {
            write_parameters(rig, &state.previous, &state.current, alpha, parameters);
        }
    }

    /// モデルのインスタンスに対して `dt` 秒進める。`instance.update()` の前に呼ぶ。
    ///
    /// 動的物理を使わない設定なら [`Self::update`] と同じ (パラメータを書き換えるだけ)。
    /// 使う設定なら 2 段階の評価の 1 段目 ([`ModelInstance::update_first_stage`]) を行い、
    /// そのワールド変換で根元・衝突体・重力の基準を求めて進め、パラメータと動的物理の加算量
    /// ([`ModelInstance::dynamic_offsets_mut`]) を書き換える。続く `instance.update()` が 2 段目になる
    /// (`docs/spec/evaluation.md#動的物理を使う場合-2-段階の評価`)。
    pub fn update_instance(&mut self, instance: &mut ModelInstance, dt: f32) {
        if !self.dynamic {
            self.update(instance.parameters_mut(), dt);
            return;
        }
        let rebind = self
            .binding
            .as_ref()
            .is_none_or(|b| !Arc::ptr_eq(&b.compiled, instance.compiled()));
        if rebind {
            self.bind(instance);
        }
        instance.update_first_stage();
        let frames = self.rig_frames(instance);
        let alpha = self.advance(&frames, instance.parameters(), dt);
        self.write_instance(instance, &frames, alpha);
    }

    /// 動的物理の出力先・衝突体・基準のデフォーマを、`instance` のモデルの要素に解決する。
    ///
    /// [`Self::update_instance`] が必要なときに呼ぶので、呼ばなくてもよい。出力先の加算量のバッファを
    /// ここで確保するので、先に呼んでおくと最初のフレームでの確保を避けられる。
    /// 動的物理を使わない設定では何もしない。
    pub fn bind(&mut self, instance: &mut ModelInstance) {
        if !self.dynamic {
            return;
        }
        let compiled = Arc::clone(instance.compiled());
        let model = compiled.model();
        let deformer = |id: &str| compiled.deformer_index(id);
        let rotation = |id: &str| {
            deformer(id).filter(|&d| matches!(model.deformers[d].kind, DeformerKind::Rotation(_)))
        };
        let rigs = self
            .settings
            .rigs
            .iter()
            .map(|rig| {
                let outputs: Vec<Target> = rig
                    .outputs
                    .iter()
                    .map(|output| match &output.target {
                        PhysicsTarget::Parameter { id } => instance
                            .parameters()
                            .index_of(id.as_str())
                            .map_or(Target::Ignore, Target::Parameter),
                        PhysicsTarget::DeformerAngle { deformer } => {
                            rotation(deformer.as_str()).map_or(Target::Ignore, Target::Angle)
                        }
                        PhysicsTarget::DeformerOffset { deformer } => {
                            rotation(deformer.as_str()).map_or(Target::Ignore, Target::Offset)
                        }
                        PhysicsTarget::Vertices { owner, .. } => match owner {
                            VertexOwner::Warp(id) => deformer(id.as_str())
                                .filter(|&d| {
                                    matches!(model.deformers[d].kind, DeformerKind::Warp(_))
                                })
                                .map_or(Target::Ignore, Target::Warp),
                            VertexOwner::Mesh(id) => compiled
                                .mesh_index(id.as_str())
                                .map_or(Target::Ignore, Target::Mesh),
                        },
                        PhysicsTarget::Unknown => Target::Ignore,
                    })
                    .collect();
                let frame = rig.frame.as_ref().and_then(|f| deformer(f.as_str()));
                // 根元は最初の回転デフォーマの出力先、なければ基準のデフォーマに取り付ける。
                let anchor = outputs
                    .iter()
                    .find_map(|t| match *t {
                        Target::Angle(d) | Target::Offset(d) => Some(d),
                        _ => None,
                    })
                    .or(frame);
                let colliders = rig
                    .colliders
                    .iter()
                    .filter_map(|id| self.settings.colliders.iter().position(|c| &c.id == id))
                    .collect();
                RigBinding {
                    anchor,
                    frame,
                    colliders,
                    outputs,
                }
            })
            .collect();
        let colliders = self
            .settings
            .colliders
            .iter()
            .map(|c| match &c.attach {
                Some(id) => deformer(id.as_str()).map(Some).ok_or(()),
                None => Ok(None),
            })
            .collect();
        let binding = Binding {
            compiled: Arc::clone(&compiled),
            rigs,
            colliders,
        };
        // 加算量のバッファを確保し、評価し直すデフォーマの印を付けておく。
        let offsets = instance.dynamic_offsets_mut();
        for target in binding.rigs.iter().flat_map(|r| &r.outputs) {
            match *target {
                Target::Angle(d) => offsets.add_angle(d, 0.0),
                Target::Offset(d) => offsets.add_origin(d, Vec2::ZERO),
                Target::Warp(d) => {
                    offsets.warp_points_mut(d);
                }
                Target::Mesh(m) => {
                    offsets.mesh_positions_mut(m);
                }
                Target::Ignore | Target::Parameter(_) => {}
            }
        }
        self.binding = Some(binding);
    }

    /// 粒子を初期姿勢に戻す (モデル切替・テレポート時)。
    /// 次の `update` で、その時点の入力に対する静止姿勢に置き直す。
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.initialized = false;
        for (state, rig) in self.rigs.iter_mut().zip(&self.settings.rigs) {
            state.chain = Chain::new(&rig.particles, Vec2::ZERO);
            state.pose = RootPose::default();
            state.external = RootPose::default();
            state.previous.fill(None);
            state.current.fill(None);
        }
    }

    /// 1 段目の評価から、リグごとのそのフレームの状態を求める。
    fn rig_frames(&self, instance: &ModelInstance) -> Vec<RigFrame> {
        let gravity = self.settings.gravity;
        let Some(binding) = &self.binding else {
            return self
                .settings
                .rigs
                .iter()
                .map(|rig| RigFrame::canvas(rig, gravity))
                .collect();
        };
        let model = binding.compiled.model();
        let down = gravity.try_normalize().unwrap_or(Vec2::Y);
        let colliders: Vec<Option<WorldCollider>> = self
            .settings
            .colliders
            .iter()
            .zip(&binding.colliders)
            .map(|(c, attach)| {
                let attach = (*attach).ok()?;
                let world = attach.map_or(ugoe_runtime::WorldTransform::IDENTITY, |d| {
                    instance.deformer_world(d)
                });
                WorldCollider::new(&c.shape, &world)
            })
            .collect();
        self.settings
            .rigs
            .iter()
            .zip(&binding.rigs)
            .map(|(rig, rb)| {
                let p0 = rig.particles.first().map_or(Vec2::ZERO, |p| p.position);
                let anchor = rb
                    .anchor
                    .map_or(Vec2::ZERO, |a| instance.deformer_world(a).point(p0) - p0);
                let frame_angle = rb.frame.map_or(0.0, |f| {
                    // ワープは格子の中心での向き、回転デフォーマはどこでも同じ。
                    let at = match model.deformers[f].kind {
                        DeformerKind::Warp(_) => Vec2::splat(0.5),
                        _ => Vec2::ZERO,
                    };
                    chain::signed_angle(down, instance.deformer_world(f).jacobian(at) * down)
                });
                let (references, flips) = rb
                    .outputs
                    .iter()
                    .map(|target| match *target {
                        Target::Angle(d) => {
                            // 粒子 1 の角度は、出力先の親の座標系での重力の向きから測る。
                            let origin = instance
                                .rotation_local_keyform(d)
                                .map_or(Vec2::ZERO, |k| k.origin);
                            let j = instance.deformer_parent_world(d).jacobian(origin);
                            (j * gravity, j.determinant() < 0.0)
                        }
                        _ => (gravity, false),
                    })
                    .unzip();
                RigFrame {
                    anchor,
                    frame_angle,
                    references,
                    flips,
                    colliders: rb.colliders.iter().filter_map(|&c| colliders[c]).collect(),
                }
            })
            .collect()
    }

    /// 入力と `frames` から根元の姿勢を求めて、`dt` 秒分のステップを進める。
    /// 出力の補間に使う割合 (`accumulator / step`) を返す。
    ///
    /// 各ステップでは、リグを `PhysicsSettings::rigs` の順に進める。中継の入力は、前のリグが
    /// 同じステップで出した出力 (補間しない) を書き込んだパラメータから読む
    /// (`docs/spec/physics.md#リグの順序と中継`)。
    fn advance(&mut self, frames: &[RigFrame], parameters: &ParameterValues, dt: f32) -> f32 {
        let step = self.step_duration();
        let time_scale = step * BASE_FPS;
        let settings = &self.settings;
        let initialized = self.initialized;
        let targets: Vec<RootPose> = settings
            .rigs
            .iter()
            .zip(frames)
            .zip(&self.rigs)
            .map(|((rig, frame), state)| {
                let mut pose = root_pose(rig, parameters, |i| !state.relay_inputs[i]);
                pose.translation += frame.anchor;
                // 基準の回転は、前のステップから近い側に回ったとみなす (±180 度をまたいでも跳ばない)。
                let frame_angle = if initialized {
                    let prev = state.pose.frame_angle;
                    prev + wrap_degrees(frame.frame_angle - prev)
                } else {
                    frame.frame_angle
                };
                pose.angle += frame_angle;
                pose.frame_angle = frame_angle;
                pose
            })
            .collect();
        let step_input = |pose: RootPose, rotation_delta: f32| StepInput {
            root: pose.translation,
            rotation_delta,
            gravity: rotate_deg(settings.gravity, pose.angle),
            wind: rotate_deg(settings.wind, pose.frame_angle),
            time_scale,
        };
        // 中継があるときだけ、ステップの中の出力を書き込むパラメータの写しを持つ。
        let mut relay = self.relay.then(|| Relay::new(&settings.rigs, parameters));
        // 中継の入力を足した姿勢。中継の入力がないリグでは `pose` のまま (結果を変えない)。
        let with_relay =
            |pose: RootPose, rig: &PhysicsRig, state: &RigState, relay: &Option<Relay>| match relay
            {
                Some(r) if state.relay_inputs.contains(&true) => {
                    pose.with_relay(root_pose(rig, &r.values, |i| state.relay_inputs[i]))
                }
                _ => pose,
            };

        if !self.initialized {
            self.initialized = true;
            self.accumulator = 0.0;
            for (((state, rig), target), frame) in self
                .rigs
                .iter_mut()
                .zip(&settings.rigs)
                .zip(&targets)
                .zip(frames)
            {
                let pose = with_relay(*target, rig, state, &relay);
                let input = step_input(pose, 0.0);
                state.chain.settle(
                    &rig.particles,
                    pose.translation,
                    input.gravity,
                    input.wind,
                    |i, p| collide(&frame.colliders, rig, i, p),
                );
                state.pose = pose;
                state.external = *target;
                state.current = outputs(rig, &state.chain, &input, &frame.references);
                state.previous.clone_from(&state.current);
                if let Some(r) = &mut relay {
                    r.write(rig, &state.current);
                }
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
            let starts: Vec<RootPose> = self.rigs.iter().map(|s| s.external).collect();
            for k in 1..=steps {
                if let Some(r) = &mut relay {
                    r.restore();
                }
                for (((state, rig), (start, target)), frame) in self
                    .rigs
                    .iter_mut()
                    .zip(&settings.rigs)
                    .zip(starts.iter().zip(&targets))
                    .zip(frames)
                {
                    // 入力はフレームの間で線形補間して、ステップごとに少しずつ動かす。
                    let external = start.lerp(*target, k as f32 / steps as f32);
                    let pose = with_relay(external, rig, state, &relay);
                    let input = step_input(pose, pose.angle - state.pose.angle);
                    state.chain.step(&rig.particles, &input, |i, p| {
                        collide(&frame.colliders, rig, i, p);
                    });
                    state.pose = pose;
                    state.external = external;
                    std::mem::swap(&mut state.previous, &mut state.current);
                    state.current = outputs(rig, &state.chain, &input, &frame.references);
                    if let Some(r) = &mut relay {
                        r.write(rig, &state.current);
                    }
                }
            }
        }

        (self.accumulator / step).clamp(0.0, 1.0)
    }

    /// 出力をパラメータと動的物理の加算量に書き込む。
    fn write_instance(&self, instance: &mut ModelInstance, frames: &[RigFrame], alpha: f32) {
        let Some(binding) = &self.binding else {
            return;
        };
        instance.dynamic_offsets_mut().clear();
        for (((state, rig), rb), frame) in self
            .rigs
            .iter()
            .zip(&self.settings.rigs)
            .zip(&binding.rigs)
            .zip(frames)
        {
            for (o, output) in rig.outputs.iter().enumerate() {
                let (Some(previous), Some(current)) = (state.previous[o], state.current[o]) else {
                    continue;
                };
                let value = previous.lerp(current, alpha);
                let weight = (output.weight / 100.0).clamp(0.0, 1.0);
                match rb.outputs[o] {
                    Target::Ignore => {}
                    Target::Parameter(index) => {
                        blend_parameter(instance.parameters_mut(), index, output, value.x, weight);
                    }
                    Target::Angle(d) => {
                        let angle = if frame.flips[o] { -value.x } else { value.x };
                        instance.dynamic_offsets_mut().add_angle(d, angle * weight);
                    }
                    Target::Offset(d) => {
                        instance.dynamic_offsets_mut().add_origin(d, value * weight);
                    }
                    Target::Warp(d) => {
                        if let (Some(points), PhysicsTarget::Vertices { weights, .. }) = (
                            instance.dynamic_offsets_mut().warp_points_mut(d),
                            &output.target,
                        ) {
                            add_weighted(points, weights, value * weight);
                        }
                    }
                    Target::Mesh(m) => {
                        if let (Some(points), PhysicsTarget::Vertices { weights, .. }) = (
                            instance.dynamic_offsets_mut().mesh_positions_mut(m),
                            &output.target,
                        ) {
                            add_weighted(points, weights, value * weight);
                        }
                    }
                }
            }
        }
    }
}

/// リグが動的物理の機能を使うか。
fn is_dynamic_rig(rig: &PhysicsRig) -> bool {
    rig.frame.is_some()
        || !rig.colliders.is_empty()
        || rig.outputs.iter().any(|o| {
            !matches!(
                o.target,
                PhysicsTarget::Parameter { .. } | PhysicsTarget::Unknown
            )
        })
}

/// 入力ごとに、`earlier` (前のリグ) のどれかがパラメータとして出力するパラメータを読むか
/// (中継の入力、`docs/spec/physics.md#リグの順序と中継`)。
fn relay_inputs(earlier: &[PhysicsRig], rig: &PhysicsRig) -> Vec<bool> {
    rig.inputs
        .iter()
        .map(|input| {
            earlier
                .iter()
                .flat_map(|r| &r.outputs)
                .any(|o| o.parameter() == Some(&input.parameter))
        })
        .collect()
}

/// 中継のための、ステップの中でリグの出力を書き込むパラメータの写し。
struct Relay {
    values: ParameterValues,
    /// パラメータへの出力先と、フレームの始めの値 (ステップの始めに戻す)。
    bases: Vec<(ParamIndex, f32)>,
}

impl Relay {
    fn new(rigs: &[PhysicsRig], parameters: &ParameterValues) -> Self {
        let bases = rigs
            .iter()
            .flat_map(|r| &r.outputs)
            .filter_map(|o| parameters.index_of(o.parameter()?.as_str()))
            .map(|index| (index, parameters.get(index)))
            .collect();
        Self {
            values: parameters.clone(),
            bases,
        }
    }

    /// 出力先のパラメータをフレームの始めの値に戻す。出力のブレンドは毎ステップ同じ値から始める。
    fn restore(&mut self) {
        for &(index, value) in &self.bases {
            self.values.set(index, value);
        }
    }

    /// リグのステップの出力 (補間しない) を書き込む。
    fn write(&mut self, rig: &PhysicsRig, current: &[Option<Vec2>]) {
        write_parameters(rig, current, current, 1.0, &mut self.values);
    }
}

/// 粒子 i を、リグの衝突体の外へ押し出す。
fn collide(colliders: &[WorldCollider], rig: &PhysicsRig, i: usize, p: &mut Vec2) {
    let margin = rig.particles[i].collision_radius.max(0.0);
    for c in colliders {
        c.push_out(p, margin);
    }
}

/// 頂点ごとの重みを掛けて足す。
fn add_weighted(points: &mut [Vec2], weights: &[f32], value: Vec2) {
    for (p, &w) in points.iter_mut().zip(weights) {
        *p += value * w;
    }
}

/// `v` を `degrees` 度 (時計回りが正) 回す。
fn rotate_deg(v: Vec2, degrees: f32) -> Vec2 {
    Vec2::from_angle(degrees.to_radians()).rotate(v)
}

/// 角度を (-180, 180] に収める。
fn wrap_degrees(degrees: f32) -> f32 {
    let r = degrees.rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

/// 入力パラメータから根元の姿勢を求める (`docs/spec/physics.md#入力`)。
/// `include(i)` が真の入力 (`rig.inputs` の添字) だけを使う。
fn root_pose(
    rig: &PhysicsRig,
    parameters: &ParameterValues,
    include: impl Fn(usize) -> bool,
) -> RootPose {
    let mut pose = RootPose::default();
    for (i, input) in rig.inputs.iter().enumerate() {
        if !include(i) {
            continue;
        }
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
///
/// スカラーの出力先 (パラメータ・回転デフォーマの角度) は `x` に値を入れる。
/// ベクトルの出力先 (原点のずれ・頂点) は、粒子の変位の `source` の成分だけを持つベクトル。
/// `references` は出力ごとの、粒子 1 の角度を測る基準の向き。
fn outputs(
    rig: &PhysicsRig,
    chain: &Chain,
    input: &StepInput,
    references: &[Vec2],
) -> Vec<Option<Vec2>> {
    let particles = chain.particles();
    rig.outputs
        .iter()
        .zip(references)
        .map(|(output, &reference)| {
            let i = output.particle as usize;
            if i == 0 || i >= particles.len() {
                return None;
            }
            let value = match output.target {
                PhysicsTarget::Unknown => return None,
                PhysicsTarget::DeformerOffset { .. } | PhysicsTarget::Vertices { .. } => {
                    // 変位 = 根元からの相対位置 - 静止姿勢での相対位置。
                    let rest = chain::rest_offset(&rig.particles, i, input.gravity, input.wind);
                    let d = particles[i].position - particles[0].position - rest;
                    match output.source {
                        PhysicsSource::X => Vec2::new(d.x, 0.0),
                        PhysicsSource::Y => Vec2::new(0.0, d.y),
                        PhysicsSource::Angle | PhysicsSource::Unknown => return None,
                    }
                }
                PhysicsTarget::Parameter { .. } | PhysicsTarget::DeformerAngle { .. } => {
                    let segment = particles[i].position - particles[i - 1].position;
                    let parent = if i >= 2 {
                        particles[i - 1].position - particles[i - 2].position
                    } else {
                        reference
                    };
                    let mut source = scalar_source(output.source, parent, segment)?;
                    if is_additive(output) {
                        // 加算の出力は、静止姿勢での値からのずれ (`docs/spec/physics.md#合成の方法`)。
                        let rest = |k: usize| {
                            chain::rest_offset(&rig.particles, k, input.gravity, input.wind)
                        };
                        let segment = rest(i) - rest(i - 1);
                        let parent = if i >= 2 {
                            rest(i - 1) - rest(i - 2)
                        } else {
                            reference
                        };
                        source -= scalar_source(output.source, parent, segment)?;
                    }
                    Vec2::new(source, 0.0)
                }
            };
            let value = value * output.scale;
            Some(if output.reflect { -value } else { value })
        })
        .collect()
}

/// 区間 `segment` (粒子 i-1 → i) と、その親の向き `parent` から、スカラーの出力の元の値を求める。
fn scalar_source(source: PhysicsSource, parent: Vec2, segment: Vec2) -> Option<f32> {
    match source {
        PhysicsSource::X => Some(segment.x),
        PhysicsSource::Y => Some(segment.y),
        PhysicsSource::Angle => Some(chain::signed_angle(parent, segment)),
        PhysicsSource::Unknown => None,
    }
}

/// パラメータへ加算する出力か。パラメータ以外の出力先は合成の方法を使わない (常に加算量)。
fn is_additive(output: &PhysicsOutput) -> bool {
    matches!(output.target, PhysicsTarget::Parameter { .. })
        && output.blend.effective() == PhysicsBlend::Add
}

/// 出力の値 `value` をパラメータに合成する (`docs/spec/physics.md#合成の方法`)。
///
/// `overwrite` は値域にクランプした `value` へ `weight` の割合で近づける。
/// `add` の `value` は静止時の値からのずれで、現在の値に `value * weight` を足してクランプする。
fn blend_parameter(
    parameters: &mut ParameterValues,
    index: ParamIndex,
    output: &PhysicsOutput,
    value: f32,
    weight: f32,
) {
    if is_additive(output) {
        // `set` が値域にクランプする。
        parameters.set(index, parameters.get(index) + value * weight);
    } else {
        let value = value.clamp(parameters.min(index), parameters.max(index));
        parameters.blend(index, value, weight);
    }
}

/// 直前 2 ステップの出力を `alpha` で補間し、パラメータに重み付きで書き込む。
/// パラメータ以外の出力先は無視する。
fn write_parameters(
    rig: &PhysicsRig,
    previous: &[Option<Vec2>],
    current: &[Option<Vec2>],
    alpha: f32,
    parameters: &mut ParameterValues,
) {
    for ((output, previous), current) in rig.outputs.iter().zip(previous).zip(current) {
        let (Some(previous), Some(current)) = (*previous, *current) else {
            continue;
        };
        let Some(index) = output
            .parameter()
            .and_then(|parameter| parameters.index_of(parameter.as_str()))
        else {
            continue;
        };
        let value = lerp(previous.x, current.x, alpha);
        blend_parameter(
            parameters,
            index,
            output,
            value,
            (output.weight / 100.0).clamp(0.0, 1.0),
        );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod dynamic_tests;

#[cfg(test)]
mod relay_tests;

#[cfg(test)]
mod additive_tests;
