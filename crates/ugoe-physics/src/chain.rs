//! 振り子チェーン (距離拘束でつないだ粒子の鎖) の 1 ステップ分の積分。
//! 仕様: `docs/spec/physics.md#振り子チェーン`。
//!
//! パラメータの入出力を知らない純粋なシミュレーションとして切り出してあり、
//! 動的物理 (T0305) の衝突体や重力の基準からも同じ積分を使う。

use alloc::vec::Vec;
use ugoe_core::Vec2;
use ugoe_core::math;
use ugoe_model::physics::PhysicsParticle;

/// 根元の回転に、子の粒子が引きずられる割合 (`docs/spec/physics.md#係数の意味`)。
pub const ROOT_ROTATION_FOLLOW: f32 = 0.2;

/// 粒子 1 つの実行時状態。座標は物理空間 (y 軸下向き)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// 位置。
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

/// 制約のフック ([`Chain::step`]・[`Chain::settle`]) に渡す、粒子 1 つ (i >= 1) の区間の情報。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// 粒子の添字。
    pub index: usize,
    /// 親 (粒子 i-1) の、補正した後の位置。
    pub parent: Vec2,
    /// 区間の始めの向きに置いた粒子の位置 (親から `radius` の距離)。ステップではステップの始めの区間の向き、
    /// 静止姿勢に置くときは初期位置 (`PhysicsParticle::position`) の区間の向き。
    pub from: Vec2,
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
///
/// [`Chain::settle`]・[`Chain::step`] に渡す粒子の定義 `defs` は、[`Chain::new`] に渡したものと同じ
/// (少なくとも同じ長さ) でなければならない (短いと添字が範囲外になる)。
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

    /// 粒子の実行時状態 (粒子 0 が根元)。
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// すべての粒子の位置・直前の位置・速度が有限値か (`docs/spec/physics.md#有限でない状態`)。
    pub fn is_finite(&self) -> bool {
        self.particles.iter().all(|p| {
            p.position.is_finite() && p.last_position.is_finite() && p.velocity.is_finite()
        })
    }

    /// 粒子を、各区間が力 (`gravity * acceleration + wind`) の向きに垂れた静止姿勢に置く。
    ///
    /// 読み込み直後やリセット後に、初期姿勢から揺れ始めないようにするために使う。
    /// `constrain` は [`Chain::step`] と同じく粒子ごと (i >= 1) に呼ばれ、置いた位置を補正できる
    /// (補正後の位置を次の粒子の親にする)。`Segment::from` は初期位置の区間の向き。
    pub fn settle(
        &mut self,
        defs: &[PhysicsParticle],
        root: Vec2,
        gravity: Vec2,
        wind: Vec2,
        mut constrain: impl FnMut(&Segment, &mut Vec2),
    ) {
        let Some(first) = defs.first() else {
            return;
        };
        let mut prev = root + first.position;
        // 粒子と定義は同じ数で作る ([`Chain::new`])。違えば短いほうに合わせる。
        let mut particles = self.particles.iter_mut().zip(defs);
        if let Some((p, _)) = particles.next() {
            *p = Particle::at_rest(prev);
        }
        for (i, (p, d)) in particles.enumerate().map(|(i, x)| (i + 1, x)) {
            let segment = Segment {
                index: i,
                parent: prev,
                from: prev + rest_direction(defs, i) * d.radius,
            };
            prev += rest_segment(defs, i, gravity, wind);
            constrain(&segment, &mut prev);
            *p = Particle::at_rest(prev);
        }
    }

    /// 1 ステップ進める。
    ///
    /// `constrain` は距離拘束の直後に粒子ごと (i >= 1) に呼ばれ、位置を補正できる
    /// (衝突体からの押し出しなど。補正後の位置から速度を求める)。`Segment::from` はステップの始めの
    /// 区間 (親と粒子のステップの始めの位置) の向き。
    pub fn step(
        &mut self,
        defs: &[PhysicsParticle],
        input: &StepInput,
        mut constrain: impl FnMut(&Segment, &mut Vec2),
    ) {
        let Some(first) = defs.first() else {
            return;
        };
        let Some(root) = self.particles.first_mut() else {
            return;
        };
        root.last_position = root.position;
        root.position = input.root + first.position;

        let rotation = Vec2::from_angle((input.rotation_delta * ROOT_ROTATION_FOLLOW).to_radians());
        // 粒子と定義は同じ数で作る ([`Chain::new`])。違えば短いほうに合わせる。
        for i in 1..self.particles.len().min(defs.len()) {
            let (Some(d), Some([parent, p])) = (defs.get(i), self.particles.get_mut(i - 1..=i))
            else {
                break;
            };
            let (parent, parent_last) = (parent.position, parent.last_position);
            let from_dir = (p.position - parent_last)
                .try_normalize()
                .unwrap_or_else(|| rest_direction(defs, i));
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
            let segment = Segment {
                index: i,
                parent,
                from: parent + from_dir * d.radius,
            };
            constrain(&segment, &mut position);

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

/// 静止姿勢での粒子 i の、根元 (粒子 0) からの相対位置 (衝突体を考えない)。
/// `i` が 0 か範囲外なら零ベクトル。動的物理の変位の基準に使う。
pub fn rest_offset(defs: &[PhysicsParticle], i: usize, gravity: Vec2, wind: Vec2) -> Vec2 {
    (1..=i.min(defs.len().saturating_sub(1)))
        .map(|k| rest_segment(defs, k, gravity, wind))
        .fold(Vec2::ZERO, |a, b| a + b)
}

/// 静止姿勢での区間 i (粒子 i-1 → i) のベクトル。力の向きが決まらなければ初期位置の向き。
fn rest_segment(defs: &[PhysicsParticle], i: usize, gravity: Vec2, wind: Vec2) -> Vec2 {
    let Some(d) = defs.get(i) else {
        return Vec2::ZERO;
    };
    let dir = (gravity * d.acceleration + wind)
        .try_normalize()
        .unwrap_or_else(|| rest_direction(defs, i));
    dir * d.radius
}

/// 1 ステップで残す速度の割合。`mobility` は基準フレームあたりの割合なので、
/// ステップ幅に応じて累乗する (ステップの周波数を変えても減衰の速さが変わらない)。
fn velocity_retention(mobility: f32, time_scale: f32) -> f32 {
    math::powf(mobility.max(0.0), time_scale)
}

/// 粒子 i の、初期位置での親からの向き。向きが決まらなければ (`i` が 0 か範囲外を含む) 真下。
fn rest_direction(defs: &[PhysicsParticle], i: usize) -> Vec2 {
    let (Some(parent), Some(d)) = (i.checked_sub(1).and_then(|j| defs.get(j)), defs.get(i)) else {
        return Vec2::Y;
    };
    (d.position - parent.position)
        .try_normalize()
        .unwrap_or(Vec2::Y)
}

/// `from` から `to` への符号付きの角度 (度、y 軸下向きの座標で時計回りが正)。
/// どちらかが零ベクトルなら 0。
pub fn signed_angle(from: Vec2, to: Vec2) -> f32 {
    if from == Vec2::ZERO || to == Vec2::ZERO {
        return 0.0;
    }
    math::atan2(from.perp_dot(to), from.dot(to)).to_degrees()
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
            collision_radius: 0.0,
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
            let root = Vec2::new(math::sin(k as f32 * 0.3) * 20.0, 0.0);
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
        chain.step(&defs, &input(Vec2::ZERO), |s, p| {
            assert_eq!(s.index, 1);
            assert_eq!(s.parent, Vec2::ZERO);
            assert_eq!(s.from, Vec2::new(0.0, 10.0));
            *p = Vec2::new(3.0, 4.0);
        });
        assert_eq!(chain.particles()[1].position, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn settle_hangs_along_force() {
        let defs = two_particles();
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.settle(
            &defs,
            Vec2::new(1.0, 2.0),
            Vec2::Y,
            Vec2::new(1.5, 0.0),
            |_, _| {},
        );
        let p = chain.particles();
        let dir = (p[1].position - p[0].position) / 10.0;
        // 力 = (0, 1.5) + (1.5, 0) なので 45 度。
        assert!((dir - Vec2::new(1.0, 1.0).normalize()).length() < 1e-4);
        assert_eq!(p[1].velocity, Vec2::ZERO);
        let rest = rest_offset(&defs, 1, Vec2::Y, Vec2::new(1.5, 0.0));
        assert!((p[1].position - p[0].position - rest).length() < 1e-4);
        assert_eq!(rest_offset(&defs, 0, Vec2::Y, Vec2::ZERO), Vec2::ZERO);
    }

    #[test]
    fn settle_applies_constrain_hook() {
        let defs = vec![
            particle(Vec2::ZERO, 0.0),
            particle(Vec2::new(0.0, 10.0), 10.0),
            particle(Vec2::new(0.0, 20.0), 10.0),
        ];
        let mut chain = Chain::new(&defs, Vec2::ZERO);
        chain.settle(&defs, Vec2::ZERO, Vec2::Y, Vec2::ZERO, |s, p| {
            if s.index == 1 {
                p.x += 5.0;
            }
        });
        // 補正した粒子 1 を親にして、粒子 2 を置く。
        let p = chain.particles();
        assert_eq!(p[1].position, Vec2::new(5.0, 10.0));
        assert_eq!(p[2].position, Vec2::new(5.0, 20.0));
    }
}
