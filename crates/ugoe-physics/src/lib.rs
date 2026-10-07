//! 振り子チェーンによる物理演算。仕様: `docs/spec/physics.md`。
//!
//! 入力パラメータ (顔の角度など) から根元の変位を作り、粒子チェーンを固定ステップで
//! シミュレーションし、セグメント角度を出力パラメータ (髪揺れなど) に書き戻す。
//!
//! 動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`) を使う設定では、
//! [`PhysicsSystem::update_instance`] がモデルの 1 段目の評価を読み、デフォーマ・頂点に出力する。

#![warn(missing_docs)]

pub mod chain;
pub mod collider;

use std::sync::Arc;
use ugoe_core::math;

use ugoe_core::{Vec2, lerp};
use ugoe_model::DeformerKind;
use ugoe_model::physics::{
    ColliderId, NormalizationRange, PhysicsBlend, PhysicsOutput, PhysicsRig, PhysicsRigId,
    PhysicsSettings, PhysicsSource, PhysicsTarget, VertexOwner,
};
use ugoe_runtime::{CompiledModel, ModelInstance, ParamIndex, ParameterLayout, ParameterValues};

pub use chain::{Chain, Particle, Segment, StepInput};
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
    /// 入力と出力のパラメータの添字 ([`RigState::input_params`]・[`RigState::output_params`]) を
    /// 解決したときの、パラメータの並びの手札。渡された `ParameterValues` の並びが違えば解決し直す。
    resolved_layout: Option<ParameterLayout>,
    /// 中継のための、ステップの中でリグの出力を書き込むパラメータの写し (中継があるときだけ)。
    /// フレームごとに作らず使い回す。
    relay_buffer: Option<Relay>,
    // 以下はフレームごとの作業用のバッファ (毎フレームの確保を避けるために使い回す)。
    frames: Vec<RigFrame>,
    world_colliders: Vec<Option<WorldCollider>>,
    targets: Vec<RootPose>,
    starts: Vec<RootPose>,
}

/// 衝突体を考えない静止姿勢で、衝突体に当たっている区間 ([`PhysicsSystem::rest_pose_collisions`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestPoseCollision {
    /// リグの ID。
    pub rig: PhysicsRigId,
    /// 当たっている区間の、子の粒子の添字 (区間は粒子 `particle - 1` → `particle`)。
    pub particle: usize,
    /// 衝突体の ID。
    pub collider: ColliderId,
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
    /// 入力ごとの、パラメータの添字 (見つからない入力は `None`)。
    input_params: Vec<Option<ParamIndex>>,
    /// 出力ごとの、出力先のパラメータの添字 (パラメータ以外の出力先・見つからないパラメータは `None`)。
    output_params: Vec<Option<ParamIndex>>,
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

    fn is_finite(&self) -> bool {
        self.translation.is_finite() && self.angle.is_finite() && self.frame_angle.is_finite()
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
#[derive(Debug, Clone, Default)]
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
    /// `colliders` のそれぞれの、`PhysicsSettings::colliders` の添字。
    collider_ids: Vec<usize>,
}

impl RigFrame {
    /// キャンバスに固定し、衝突体を持たない状態にする (バッファは使い回す)。
    fn set_canvas(&mut self, rig: &PhysicsRig, gravity: Vec2) {
        self.anchor = Vec2::ZERO;
        self.frame_angle = 0.0;
        self.references.clear();
        self.references.resize(rig.outputs.len(), gravity);
        self.flips.clear();
        self.flips.resize(rig.outputs.len(), false);
        self.colliders.clear();
        self.collider_ids.clear();
    }
}

/// `frames` を、リグごとの [`RigFrame::set_canvas`] にする。
fn canvas_frames(settings: &PhysicsSettings, frames: &mut Vec<RigFrame>) {
    frames.resize_with(settings.rigs.len(), RigFrame::default);
    for (frame, rig) in frames.iter_mut().zip(&settings.rigs) {
        frame.set_canvas(rig, settings.gravity);
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
    /// 根元を取り付けるデフォーマ (`docs/spec/physics.md#物理空間と根元の取り付け先`)。
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
    /// 設定から作る。粒子は最初の [`Self::update`] (または [`Self::update_instance`]) で静止姿勢に置く。
    ///
    /// 設定の検査 (`ugoe_model::validate::validate_physics`) はしない。参照先が見つからない入力・出力・衝突体は実行時に無視する。
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
                input_params: Vec::new(),
                output_params: Vec::new(),
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
            resolved_layout: None,
            relay_buffer: None,
            frames: Vec::new(),
            world_colliders: Vec::new(),
            targets: Vec::new(),
            starts: Vec::new(),
        }
    }

    /// 作るときに渡した設定。
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
        let mut frames = std::mem::take(&mut self.frames);
        canvas_frames(&self.settings, &mut frames);
        let alpha = self.advance(&frames, parameters, dt);
        self.frames = frames;
        for (state, rig) in self.rigs.iter().zip(&self.settings.rigs) {
            write_parameters(
                rig,
                &state.output_params,
                &state.previous,
                &state.current,
                alpha,
                parameters,
            );
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
        let mut frames = std::mem::take(&mut self.frames);
        self.rig_frames(instance, &mut frames);
        let alpha = self.advance(&frames, instance.parameters(), dt);
        self.write_instance(instance, &frames, alpha);
        self.frames = frames;
    }

    /// 入力と出力のパラメータの添字を `parameters` の並びで解決する。前に解決したときと並びが
    /// 同じなら何もしない (名前の探索はパラメータの並びが変わったときだけ)。
    ///
    /// 並びの手札が同じなら定数時間で済む。手札が違っても ID の列が同じ (`from_model` で作り直したなど)
    /// なら、解決し直さずに手札だけ取り替える。
    fn resolve_parameters(&mut self, parameters: &ParameterValues) {
        if let Some(layout) = &self.resolved_layout {
            if parameters.has_layout(layout) {
                return;
            }
            if layout.ids() == parameters.ids() {
                self.resolved_layout = Some(parameters.layout());
                return;
            }
        }
        for (state, rig) in self.rigs.iter_mut().zip(&self.settings.rigs) {
            state.input_params.clear();
            state.input_params.extend(
                rig.inputs
                    .iter()
                    .map(|input| parameters.index_of(input.parameter.as_str())),
            );
            state.output_params.clear();
            state.output_params.extend(
                rig.outputs
                    .iter()
                    .map(|o| o.parameter().and_then(|p| parameters.index_of(p.as_str()))),
            );
        }
        self.relay_buffer = self.relay.then(|| Relay {
            values: parameters.clone(),
            bases: self
                .rigs
                .iter()
                .flat_map(|s| s.output_params.iter().flatten())
                .map(|&index| (index, parameters.get(index)))
                .collect(),
        });
        self.resolved_layout = Some(parameters.layout());
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
                            VertexOwner::Unknown => Target::Ignore,
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

    /// 衝突体を考えない静止姿勢で、衝突体に当たっている区間を調べる (`docs/spec/physics.md#衝突体`)。
    ///
    /// `instance` の今のパラメータ (ふつうは既定値) で 1 段目の評価を行い、各リグの鎖を、その入力と
    /// 重力・風に対する静止姿勢 (衝突体を考えない。中継の入力は最初の `update` と同じく、前のリグの
    /// 静止姿勢の出力を読む) に置いて、区間 (粒子 i-1 → i) が
    /// [`WorldCollider::blocks`] で衝突体に当たるものを返す。当たっている区間は、最初の `update` で
    /// 衝突体の外の最も近い向きへ回され、髪は衝突体に沿って置かれる。作者が意図しない姿勢になりやすいので、
    /// 取り込みやエディタで警告に使う。物理の状態 (粒子) は変えない。動的物理を使わない設定では空。
    pub fn rest_pose_collisions(&mut self, instance: &mut ModelInstance) -> Vec<RestPoseCollision> {
        if !self.dynamic {
            return Vec::new();
        }
        self.bind(instance);
        instance.update_first_stage();
        let mut frames = Vec::new();
        self.rig_frames(instance, &mut frames);
        let parameters = instance.parameters();
        self.resolve_parameters(parameters);
        let settings = &self.settings;
        // 最初の `update` と同じく、中継の入力は前のリグの静止姿勢の出力から読む。
        let mut relay = self.relay_buffer.as_mut();
        if let Some(r) = &mut relay {
            r.begin(parameters);
        }
        let mut found = Vec::new();
        let mut values = Vec::new();
        for ((rig, frame), state) in settings.rigs.iter().zip(&frames).zip(&self.rigs) {
            let mut pose = root_pose(rig, &state.input_params, parameters, |i| {
                !state.relay_inputs[i]
            });
            pose.translation += frame.anchor;
            pose.angle += frame.frame_angle;
            pose.frame_angle = frame.frame_angle;
            let pose = with_relay(pose, rig, state, relay.as_deref());
            let gravity = rotate_deg(settings.gravity, pose.angle);
            let wind = rotate_deg(settings.wind, pose.frame_angle);
            let mut chain = Chain::new(&rig.particles, Vec2::ZERO);
            chain.settle(&rig.particles, pose.translation, gravity, wind, |_, _| {});
            if let Some(r) = &mut relay {
                let input = StepInput {
                    root: pose.translation,
                    rotation_delta: 0.0,
                    gravity,
                    wind,
                    time_scale: 0.0,
                };
                values.clear();
                values.resize(rig.outputs.len(), None);
                outputs(rig, &chain, &input, &frame.references, &mut values);
                r.write(rig, &state.output_params, &values);
            }
            let particles = chain.particles();
            for i in 1..particles.len() {
                let margin = rig.particles[i].collision_radius.max(0.0);
                for (c, &id) in frame.colliders.iter().zip(&frame.collider_ids) {
                    if c.blocks(particles[i - 1].position, particles[i].position, margin) {
                        found.push(RestPoseCollision {
                            rig: rig.id.clone(),
                            particle: i,
                            collider: settings.colliders[id].id.clone(),
                        });
                    }
                }
            }
        }
        found
    }

    /// 1 段目の評価から、リグごとのそのフレームの状態を `frames` に求める (バッファは使い回す)。
    fn rig_frames(&mut self, instance: &ModelInstance, frames: &mut Vec<RigFrame>) {
        let gravity = self.settings.gravity;
        let Some(binding) = &self.binding else {
            canvas_frames(&self.settings, frames);
            return;
        };
        let model = binding.compiled.model();
        let down = gravity.try_normalize().unwrap_or(Vec2::Y);
        let colliders = &mut self.world_colliders;
        colliders.clear();
        colliders.extend(self.settings.colliders.iter().zip(&binding.colliders).map(
            |(c, attach)| {
                let attach = (*attach).ok()?;
                let world = attach.map_or(ugoe_runtime::WorldTransform::IDENTITY, |d| {
                    instance.deformer_world(d)
                });
                WorldCollider::new(&c.shape, &world)
            },
        ));
        frames.resize_with(self.settings.rigs.len(), RigFrame::default);
        for ((frame, rig), rb) in frames
            .iter_mut()
            .zip(&self.settings.rigs)
            .zip(&binding.rigs)
        {
            let p0 = rig.particles.first().map_or(Vec2::ZERO, |p| p.position);
            frame.anchor = rb
                .anchor
                .map_or(Vec2::ZERO, |a| instance.deformer_world(a).point(p0) - p0);
            frame.frame_angle = rb.frame.map_or(0.0, |f| {
                // ワープは格子の中心での向き、回転デフォーマはどこでも同じ。
                let at = match model.deformers[f].kind {
                    DeformerKind::Warp(_) => Vec2::splat(0.5),
                    _ => Vec2::ZERO,
                };
                chain::signed_angle(down, instance.deformer_world(f).jacobian(at) * down)
            });
            frame.references.clear();
            frame.flips.clear();
            for target in &rb.outputs {
                let (reference, flip) = match *target {
                    Target::Angle(d) => {
                        // 粒子 1 の角度は、出力先の親の座標系での重力の向きから測る。
                        let origin = instance
                            .rotation_local_keyform(d)
                            .map_or(Vec2::ZERO, |k| k.origin);
                        let j = instance.deformer_parent_world(d).jacobian(origin);
                        (j * gravity, j.determinant() < 0.0)
                    }
                    _ => (gravity, false),
                };
                frame.references.push(reference);
                frame.flips.push(flip);
            }
            frame.collider_ids.clear();
            frame.colliders.clear();
            for &c in &rb.colliders {
                if let Some(collider) = colliders[c] {
                    frame.collider_ids.push(c);
                    frame.colliders.push(collider);
                }
            }
        }
    }

    /// 入力と `frames` から根元の姿勢を求めて、`dt` 秒分のステップを進める。
    /// 出力の補間に使う割合 (`accumulator / step`) を返す。
    ///
    /// 各ステップでは、リグを `PhysicsSettings::rigs` の順に進める。中継の入力は、前のリグが
    /// 同じステップで出した出力 (補間しない) を書き込んだパラメータから読む
    /// (`docs/spec/physics.md#リグの順序と中継`)。
    fn advance(&mut self, frames: &[RigFrame], parameters: &ParameterValues, dt: f32) -> f32 {
        self.resolve_parameters(parameters);
        let step = self.step_duration();
        let time_scale = step * BASE_FPS;
        let settings = &self.settings;
        let initialized = self.initialized;
        let targets = &mut self.targets;
        targets.clear();
        targets.extend(settings.rigs.iter().zip(frames).zip(&self.rigs).map(
            |((rig, frame), state)| {
                let mut pose = root_pose(rig, &state.input_params, parameters, |i| {
                    !state.relay_inputs[i]
                });
                pose.translation += frame.anchor;
                // 基準の回転は、前のステップから近い側に回ったとみなす (±180 度をまたいでも跳ばない)。
                // 前の角度が有限でなければ (あふれた後)、今の角度から始め直す。
                let prev = state.pose.frame_angle;
                let frame_angle = if initialized && prev.is_finite() {
                    prev + wrap_degrees(frame.frame_angle - prev)
                } else {
                    frame.frame_angle
                };
                pose.angle += frame_angle;
                pose.frame_angle = frame_angle;
                pose
            },
        ));
        let targets = &self.targets;
        let step_input = |pose: RootPose, rotation_delta: f32| StepInput {
            root: pose.translation,
            rotation_delta,
            gravity: rotate_deg(settings.gravity, pose.angle),
            wind: rotate_deg(settings.wind, pose.frame_angle),
            time_scale,
        };
        // 中継があるときだけ、ステップの中の出力を書き込むパラメータの写しを持つ。
        let mut relay = self.relay_buffer.as_mut();
        if let Some(r) = &mut relay {
            r.begin(parameters);
        }

        if !self.initialized {
            self.initialized = true;
            self.accumulator = 0.0;
            for (((state, rig), target), frame) in self
                .rigs
                .iter_mut()
                .zip(&settings.rigs)
                .zip(targets)
                .zip(frames)
            {
                let pose = with_relay(*target, rig, state, relay.as_deref());
                let input = step_input(pose, 0.0);
                state.chain.settle(
                    &rig.particles,
                    pose.translation,
                    input.gravity,
                    input.wind,
                    |s, p| collide(&frame.colliders, rig, s, p),
                );
                state.pose = pose;
                state.external = *target;
                outputs(
                    rig,
                    &state.chain,
                    &input,
                    &frame.references,
                    &mut state.current,
                );
                state.previous.clone_from(&state.current);
                if let Some(r) = &mut relay {
                    r.write(rig, &state.output_params, &state.current);
                }
            }
        }

        if dt.is_finite() && dt > 0.0 {
            self.accumulator += dt;
        }
        let whole = math::floor(self.accumulator / step);
        if whole > MAX_STEPS_PER_UPDATE as f32 {
            // ステップ数の上限を超えた分の時間は捨てる。巨大な `dt` から `whole * step` を引くと
            // 桁落ちする (無限大にもなる) ので、余りは剰余で求めて [0, step) に収める。
            self.accumulator = math::rem_euclid(self.accumulator, step);
        } else {
            self.accumulator -= whole * step;
        }
        let steps = (whole as u32).min(MAX_STEPS_PER_UPDATE);

        if steps > 0 {
            let starts = &mut self.starts;
            starts.clear();
            // 前の姿勢が有限でなければ (根元があふれた後)、補間すると NaN になるので今の姿勢から始める。
            starts.extend(self.rigs.iter().zip(targets).map(|(s, t)| {
                if s.external.is_finite() {
                    s.external
                } else {
                    *t
                }
            }));
            for k in 1..=steps {
                if let Some(r) = &mut relay {
                    r.restore();
                }
                for (((state, rig), (start, target)), frame) in self
                    .rigs
                    .iter_mut()
                    .zip(&settings.rigs)
                    .zip(starts.iter().zip(targets))
                    .zip(frames)
                {
                    // 入力はフレームの間で線形補間して、ステップごとに少しずつ動かす。
                    let external = start.lerp(*target, k as f32 / steps as f32);
                    let pose = with_relay(external, rig, state, relay.as_deref());
                    let input = step_input(pose, pose.angle - state.pose.angle);
                    state.chain.step(&rig.particles, &input, |s, p| {
                        collide(&frame.colliders, rig, s, p);
                    });
                    if !state.chain.is_finite() {
                        // あふれた鎖は速度が NaN のまま戻らないので、このステップの入力の静止姿勢に
                        // 置き直す (`docs/spec/physics.md#有限でない状態`)。
                        state.chain.settle(
                            &rig.particles,
                            pose.translation,
                            input.gravity,
                            input.wind,
                            |s, p| collide(&frame.colliders, rig, s, p),
                        );
                    }
                    state.pose = pose;
                    state.external = external;
                    std::mem::swap(&mut state.previous, &mut state.current);
                    outputs(
                        rig,
                        &state.chain,
                        &input,
                        &frame.references,
                        &mut state.current,
                    );
                    if let Some(r) = &mut relay {
                        r.write(rig, &state.output_params, &state.current);
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
                // 有限の 2 つの値の補間も、差があふれると有限でなくなる。
                if !value.is_finite() {
                    continue;
                }
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
                PhysicsTarget::Parameter { .. }
                    | PhysicsTarget::Unknown
                    | PhysicsTarget::Vertices {
                        owner: VertexOwner::Unknown,
                        ..
                    }
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

/// 中継の入力を足した姿勢。中継の入力がないリグでは `pose` のまま (結果を変えない)。
fn with_relay(
    pose: RootPose,
    rig: &PhysicsRig,
    state: &RigState,
    relay: Option<&Relay>,
) -> RootPose {
    match relay {
        Some(r) if state.relay_inputs.contains(&true) => {
            pose.with_relay(root_pose(rig, &state.input_params, &r.values, |i| {
                state.relay_inputs[i]
            }))
        }
        _ => pose,
    }
}

/// 中継のための、ステップの中でリグの出力を書き込むパラメータの写し。
///
/// [`PhysicsSystem::resolve_parameters`] がパラメータの並びごとに作り、フレームの始めに
/// [`Relay::begin`] で今の値を写す。
#[derive(Debug)]
struct Relay {
    values: ParameterValues,
    /// パラメータへの出力先と、フレームの始めの値 (ステップの始めに戻す)。
    bases: Vec<(ParamIndex, f32)>,
}

impl Relay {
    /// フレームの始めのパラメータ (`values` と同じ並び) を写す。確保済みのバッファを使い回す。
    fn begin(&mut self, parameters: &ParameterValues) {
        self.values.clone_from(parameters);
        for (index, base) in &mut self.bases {
            *base = parameters.get(*index);
        }
    }

    /// 出力先のパラメータをフレームの始めの値に戻す。出力のブレンドは毎ステップ同じ値から始める。
    fn restore(&mut self) {
        for &(index, value) in &self.bases {
            self.values.set(index, value);
        }
    }

    /// リグのステップの出力 (補間しない) を書き込む。
    fn write(
        &mut self,
        rig: &PhysicsRig,
        indices: &[Option<ParamIndex>],
        current: &[Option<Vec2>],
    ) {
        write_parameters(rig, indices, current, current, 1.0, &mut self.values);
    }
}

/// 粒子の区間を、長さを保ってリグの衝突体の外に置く。
fn collide(colliders: &[WorldCollider], rig: &PhysicsRig, segment: &Segment, p: &mut Vec2) {
    if colliders.is_empty() {
        return;
    }
    let margin = rig.particles[segment.index].collision_radius.max(0.0);
    collider::constrain_segment(colliders, margin, segment.parent, segment.from, p);
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
    let r = math::rem_euclid(degrees, 360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

/// 入力パラメータから根元の姿勢を求める (`docs/spec/physics.md#入力`)。
/// `include(i)` が真の入力 (`rig.inputs` の添字) だけを使う。`indices` は入力ごとのパラメータの添字
/// ([`RigState::input_params`])。
fn root_pose(
    rig: &PhysicsRig,
    indices: &[Option<ParamIndex>],
    parameters: &ParameterValues,
    include: impl Fn(usize) -> bool,
) -> RootPose {
    let mut pose = RootPose::default();
    for ((i, input), &index) in rig.inputs.iter().enumerate().zip(indices) {
        if !include(i) {
            continue;
        }
        let Some(index) = index else {
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
/// `references` は出力ごとの、粒子 1 の角度を測る基準の向き。結果は `out` (出力ごと) に書く。
fn outputs(
    rig: &PhysicsRig,
    chain: &Chain,
    input: &StepInput,
    references: &[Vec2],
    out: &mut [Option<Vec2>],
) {
    let particles = chain.particles();
    let values = rig
        .outputs
        .iter()
        .zip(references)
        .map(|(output, &reference)| {
            let i = output.particle as usize;
            if i == 0 || i >= particles.len() {
                return None;
            }
            let value = match output.target {
                PhysicsTarget::Unknown
                | PhysicsTarget::Vertices {
                    owner: VertexOwner::Unknown,
                    ..
                } => return None,
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
            // 有限でない値は捨てる (`docs/spec/physics.md#有限でない状態`)。
            value
                .is_finite()
                .then_some(if output.reflect { -value } else { value })
        });
    for (slot, value) in out.iter_mut().zip(values) {
        *slot = value;
    }
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
/// パラメータ以外の出力先は無視する。`indices` は出力ごとのパラメータの添字 ([`RigState::output_params`])。
fn write_parameters(
    rig: &PhysicsRig,
    indices: &[Option<ParamIndex>],
    previous: &[Option<Vec2>],
    current: &[Option<Vec2>],
    alpha: f32,
    parameters: &mut ParameterValues,
) {
    for (((output, &index), previous), current) in
        rig.outputs.iter().zip(indices).zip(previous).zip(current)
    {
        let (Some(previous), Some(current)) = (*previous, *current) else {
            continue;
        };
        let Some(index) = index else {
            continue;
        };
        let value = lerp(previous.x, current.x, alpha);
        if !value.is_finite() {
            continue;
        }
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

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod non_finite_tests;
