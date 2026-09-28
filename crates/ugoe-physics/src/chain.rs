//! 振り子チェーン (距離拘束でつないだ粒子の鎖) の 1 ステップ分の積分。
//! 仕様: `docs/spec/physics.md#振り子チェーン`。
//!
//! パラメータの入出力を知らない純粋なシミュレーションとして切り出してあり、
//! 動的物理 (T0305) の衝突体や重力の基準からも同じ積分を使う。

use ugoe_core::Vec2;
use ugoe_model::physics::PhysicsParticle;

/// 根元の回転に、子の粒子が引きずられる割合 (`docs/spec/physics.md#根元の回転`)。
pub const ROOT_ROTATION_FOLLOW: f32 = 0.2;

/// 粒子 1 つの実行時状態。座標は物理空間 (y 軸下向き)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub position: Vec2,
    /// 直前のステップの開始時の位置。
    pub last_position: Vec2,
    /// 速度 (物理空間の長さ / 基準フレーム)。
    pub velocity: Vec2,
}

impl Particle {
    fn at_rest(position: Vec2) -> Self {
        Self {
            position,
            last_position: position,
            velocity: Vec2::ZERO,
        }
    }
}

/// 1 ステップ分の外部入力。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepInput {
    /// 粒子 0 (根元) を置く位置。
    pub root: Vec2,
    /// このステップでの根元の回転の変化量 (度、時計回りが正)。
    pub rotation_delta: f32,
    /// 重力 (根元の回転を適用済み)。粒子ごとに `acceleration` 倍される。
    pub gravity: Vec2,
    /// 風。`acceleration` の影響を受けない。
    pub wind: Vec2,
    /// ステップ幅を基準フレーム (1/30 秒) 単位で表したもの。
    pub time_scale: f32,
}

/// 振り子チェーン 1 本の実行時状態。
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    particles: Vec<Particle>,
}

impl Chain {
    /// 粒子を初期位置 (`root + PhysicsParticle::position`) に静止させて作る。
    pub fn new(defs: &[PhysicsParticle], root: Vec2) -> Self {
        Self {
            particles: defs
                .iter()
                .map(|d| Particle::at_rest(root + d.position))
                .collect(),
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// 粒子を、各区間が力 (`gravity * acceleration + wind`) の向きに垂れた静止姿勢に置く。
    ///
    /// 読み込み直後やリセット後に、初期姿勢から揺れ始めないようにするために使う。
    pub fn settle(&mut self, defs: &[PhysicsParticle], root: Vec2, gravity: Vec2, wind: Vec2) {
        let Some(first) = defs.first() else {
            return;
        };
        let mut prev = root + first.position;
        self.particles[0] = Particle::at_rest(prev);
        for i in 1..self.particles.len() {
            let d = &defs[i];
            let fallback = rest_direction(defs, i);
            let dir = (gravity * d.acceleration + wind)
                .try_normalize()
                .unwrap_or(fallback);
            prev += dir * d.radius;
            self.particles[i] = Particle::at_rest(prev);
        }
    }

    /// 1 ステップ進める。
    ///
    /// `constrain` は距離拘束の直後に粒子ごと (i >= 1) に呼ばれ、位置を補正できる
    /// (衝突体からの押し出しなど。補正後の位置から速度を求める)。
    pub fn step(
        &mut self,
        defs: &[PhysicsParticle],
        input: &StepInput,
        mut constrain: impl FnMut(usize, &mut Vec2),
    ) {
        let Some(first) = defs.first() else {
            return;
        };
        let root = &mut self.particles[0];
        root.last_position = root.position;
        root.position = input.root + first.position;

        let rotation = Vec2::from_angle((input.rotation_delta * ROOT_ROTATION_FOLLOW).to_radians());
        for i in 1..self.particles.len() {
            let d = &defs[i];
            let parent = self.particles[i - 1].position;
            let p = &mut self.particles[i];
            let last = p.position;
            let h = input.time_scale * d.delay;

            // 根元の回転に引きずられる: 親のまわりに回す。
            let mut position = parent + rotation.rotate(p.position - parent);
            let force = input.gravity * d.acceleration + input.wind;
            position += p.velocity * h + force * (h * h);

            // 距離拘束: 親から radius の距離に戻す。
            let dir = (position - parent)
                .try_normalize()
                .unwrap_or_else(|| rest_direction(defs, i));
            position = parent + dir * d.radius;
            constrain(i, &mut position);

            p.velocity = if h > 0.0 {
                (position - last) / h * velocity_retention(d.mobility, input.time_scale)
            } else {
                Vec2::ZERO
            };
            p.last_position = last;
            p.position = position;
        }
    }
}

/// 1 ステップで残す速度の割合。`mobility` は基準フレームあたりの割合なので、
/// ステップ幅に応じて累乗する (ステップの周波数を変えても減衰の速さが変わらない)。
fn velocity_retention(mobility: f32, time_scale: f32) -> f32 {
    mobility.max(0.0).powf(time_scale)
}

/// 粒子 i の、初期位置での親からの向き。向きが決まらなければ真下。
fn rest_direction(defs: &[PhysicsParticle], i: usize) -> Vec2 {
    (defs[i].position - defs[i - 1].position)
        .try_normalize()
        .unwrap_or(Vec2::Y)
}

/// `from` から `to` への符号付きの角度 (度、y 軸下向きの座標で時計回りが正)。
/// どちらかが零ベクトルなら 0。
pub fn signed_angle(from: Vec2, to: Vec2) -> f32 {
    if from == Vec2::ZERO || to == Vec2::ZERO {
        return 0.0;
    }
    from.perp_dot(to).atan2(from.dot(to)).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn particle(position: Vec2, radius: f32) -> PhysicsParticle {
        PhysicsParticle {
            position,
            mobility: 0.95,
            delay: 0.9,
            acceleration: 1.5,
            radius,
        }
    }

    fn two_particles() -> Vec<PhysicsParticle> {
        vec![
            particle(Vec2::ZERO, 0.0),
            particle(Vec2::new(0.0, 10.0), 10.0),
        ]
    }

    fn input(root: Vec2) -> StepInput {
        StepInput {
            root,
            rotation_delta: 0.0,
            gravity: Vec2::Y,
            wind: Vec2::ZERO,
            time_scale: 0.5,
        }
    }

    #[test]
    fn signed_angle_is_clockwise_positive() {
        // y 軸下向きでは、下 (0,1) から左 (-1,0) への回転が時計回り。
        assert!((signed_angle(Vec2::Y, Vec2::NEG_X) - 90.0).abs() < 1e-4);
        assert!((signed_angle(Vec2::Y, Vec2::X) + 90.0).abs() < 1e-4);
        assert_eq!(signed_angle(Vec2::ZERO, Vec2::X), 0.0);
    }

    #[test]
    fn distance_constraint_keeps_radius() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        for k in 0..100 {
            let root = Vec2::new((k as f32 * 0.3).sin() * 20.0, 0.0);
            chain.step(&defs, &input(root), |_, _| {});
            let p = chain.particles();
            assert!(((p[1].position - p[0].position).length() - 10.0).abs() < 1e-4);
        }
    }

    #[test]
    fn hanging_chain_stays_at_rest() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        for _ in 0..30 {
            chain.step(&defs, &input(Vec2::ZERO), |_, _| {});
        }
        let p = chain.particles()[1];
        assert!((p.position - Vec2::new(0.0, 10.0)).length() < 1e-4);
        assert!(p.velocity.length() < 1e-4);
    }

    #[test]
    fn moving_root_drags_particle_behind() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.step(&defs, &input(Vec2::new(5.0, 0.0)), |_, _| {});
        // 根元が右に動くと、粒子は遅れて左に傾く。
        let p = chain.particles();
        assert!(p[1].position.x < p[0].position.x);
        assert!(p[1].position.x > 0.0);
    }

    #[test]
    fn swing_decays_with_mobility_below_one() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.step(&defs, &input(Vec2::new(5.0, 0.0)), |_, _| {});
        let first = (chain.particles()[1].position - chain.particles()[0].position)
            .x
            .abs();
        for _ in 0..600 {
            chain.step(&defs, &input(Vec2::new(5.0, 0.0)), |_, _| {});
        }
        let later = (chain.particles()[1].position - chain.particles()[0].position)
            .x
            .abs();
        assert!(later < first * 0.05, "first={first} later={later}");
    }

    #[test]
    fn constrain_hook_can_push_particles() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.step(&defs, &input(Vec2::ZERO), |i, p| {
            assert_eq!(i, 1);
            *p = Vec2::new(3.0, 4.0);
        });
        assert_eq!(chain.particles()[1].position, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn settle_hangs_along_force() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.settle(&defs, Vec2::new(1.0, 2.0), Vec2::Y, Vec2::new(1.5, 0.0));
        let p = chain.particles();
        let dir = (p[1].position - p[0].position) / 10.0;
        // 力 = (0, 1.5) + (1.5, 0) なので 45 度。
        assert!((dir - Vec2::new(1.0, 1.0).normalize()).length() < 1e-4);
        assert_eq!(p[1].velocity, Vec2::ZERO);
    }
}
