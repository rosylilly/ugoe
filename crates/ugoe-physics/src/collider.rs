//! 衝突体 (円・カプセル) と粒子の区間の衝突。仕様: `docs/spec/physics.md#衝突体`。

use alloc::vec::Vec;
use ugoe_core::Vec2;
use ugoe_core::math;
use ugoe_model::physics::ColliderShape;
use ugoe_runtime::WorldTransform;

/// キャンバス座標に置いた衝突体の形状。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum WorldCollider {
    /// 円。
    Circle {
        /// 中心 (キャンバス座標)。
        center: Vec2,
        /// 半径 (ピクセル)。
        radius: f32,
    },
    /// 線分 `a`–`b` から `radius` 以内の領域。
    Capsule {
        /// 芯の線分の端 (キャンバス座標)。
        a: Vec2,
        /// 芯の線分のもう一方の端 (キャンバス座標)。
        b: Vec2,
        /// 半径 (ピクセル)。
        radius: f32,
    },
}

impl WorldCollider {
    /// 取り付け先の座標系で表した `shape` を、取り付け先のワールド変換 `world` で置く。
    ///
    /// 位置 (`center`・`a`・`b`) だけを写し、半径はキャンバスのピクセルのまま (取り付け先の
    /// 拡大・ワープの伸縮を受けない)。未知の形状は `None`。
    pub(crate) fn new(shape: &ColliderShape, world: &WorldTransform<'_>) -> Option<Self> {
        match *shape {
            ColliderShape::Circle { center, radius } => Some(Self::Circle {
                center: world.point(center),
                radius,
            }),
            ColliderShape::Capsule { a, b, radius } => Some(Self::Capsule {
                a: world.point(a),
                b: world.point(b),
                radius,
            }),
            ColliderShape::Unknown => None,
        }
    }

    /// 芯 (線分 `a`–`b`。円では `a == b`) と半径。
    fn core(&self) -> (Vec2, Vec2, f32) {
        match *self {
            Self::Circle { center, radius } => (center, center, radius),
            Self::Capsule { a, b, radius } => (a, b, radius),
        }
    }

    /// 粒子を `q` に置いたとき、区間 `parent`–`q` がこの衝突体に当たるか。
    ///
    /// `q` が半径を `margin` だけ広げた衝突体の内側にあるか、区間 (線分) が衝突体そのもの
    /// (`margin` なし) を横切れば当たる。`parent` が衝突体そのものの内側にあれば、区間の判定はしない。
    pub(crate) fn blocks(&self, parent: Vec2, q: Vec2, margin: f32) -> bool {
        self.blocks_from(self.distance_to(parent), parent, q, margin)
    }

    /// [`WorldCollider::blocks`] を、求めておいた `parent` から芯までの距離 `parent_distance`
    /// ([`WorldCollider::distance_to`]) で判定する。掃引の間は親が変わらないので、距離を判定のたびに求め直さない。
    fn blocks_from(&self, parent_distance: f32, parent: Vec2, q: Vec2, margin: f32) -> bool {
        let (a, b, radius) = self.core();
        if point_segment_distance(q, a, b) < radius + margin {
            return true;
        }
        parent_distance >= radius && segment_segment_distance(parent, q, a, b) < radius
    }

    /// 点 `p` から芯 (円の中心・カプセルの線分) までの距離。
    fn distance_to(&self, p: Vec2) -> f32 {
        let (a, b, _) = self.core();
        point_segment_distance(p, a, b)
    }

    /// 半径 (キャンバスのピクセル)。
    fn radius(&self) -> f32 {
        self.core().2
    }

    /// 点 `p` が、半径を `margin` だけ広げた衝突体の内側にあれば、表面の最も近い点まで押し出す。
    ///
    /// 押し出す向きは、形状の芯 (円の中心・カプセルの線分) の最も近い点から `p` への向き。
    /// `p` が芯の上にあって向きが決まらなければ、円は上 (`-y`)、カプセルは線分の法線
    /// (線分の向きを反時計回りに 90 度回したもの) の向きに押し出す。
    pub(crate) fn push_out(&self, p: &mut Vec2, margin: f32) {
        let (core, radius, fallback) = match *self {
            Self::Circle { center, radius } => (center, radius, Vec2::NEG_Y),
            Self::Capsule { a, b, radius } => {
                let ab = b - a;
                let len2 = ab.length_squared();
                let t = if len2 > 0.0 {
                    ((*p - a).dot(ab) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let normal = Vec2::new(ab.y, -ab.x)
                    .try_normalize()
                    .unwrap_or(Vec2::NEG_Y);
                (a + ab * t, radius, normal)
            }
        };
        let reach = radius + margin;
        let d = *p - core;
        if d.length_squared() >= reach * reach {
            return;
        }
        let dir = d.try_normalize().unwrap_or(fallback);
        *p = core + dir * reach;
    }
}

/// 掃引で調べる角度の刻みの上限 (度)。
pub(crate) const SWEEP_MAX_STEP_DEGREES: f32 = 10.0;

/// 掃引の刻みを決めるときの、衝突体の半径の下限 (ピクセル)。半径が 0 に近い衝突体で刻みが細かくなりすぎないようにする。
const SWEEP_MIN_RADIUS: f32 = 0.5;

/// 掃引で半回転 (π) を調べる刻みの数の上限。区間が衝突体の半径に比べて極端に長くても、
/// 刻みをこれより細かくしない (調べる回数を抑える。`docs/spec/physics.md#衝突体`)。
const SWEEP_MAX_SAMPLES: u32 = 65_536;

/// 当たる向きと当たらない向きの境目を二分法で求める回数。
const SWEEP_BISECTIONS: u32 = 16;

/// [`constrain_segment`] の作業用のバッファ。
///
/// 掃引は親のまわりに区間を回しながら衝突体との当たりを何度も調べるが、親と衝突体は掃引の間変わらないので、
/// 区間の届く範囲にある衝突体とその親からの距離 (平方根を含む) を掃引の始めに 1 度だけ求めてここに入れる。
/// 衝突体の数はモデルで決まり上限が無いので、決まった大きさの配列 (スタック) ではなく、呼び出し側が持って使い回す
/// `Vec` にした (容量が足りた後は確保しない)。中身は呼び出しの間で引き継がない。
#[derive(Debug, Clone, Default)]
pub(crate) struct SweepBuffer {
    near: Vec<NearCollider>,
}

/// 区間の届く範囲にある衝突体と、親から芯までの距離。
#[derive(Debug, Clone, Copy)]
struct NearCollider {
    collider: WorldCollider,
    parent_distance: f32,
}

/// 粒子 i の区間 (親 `parent` から粒子 `to` まで) を、区間の長さを保ったまま衝突体の外に置く。
/// 仕様: `docs/spec/physics.md#衝突体`。
///
/// `from` はステップの始めの粒子の位置 (向きだけを使う)。区間を親のまわりに `from` の向きから `to` の向きへ
/// 近い側に回し、最初に衝突体に当たる手前で止める (1 ステップで衝突体をすり抜けない)。`from` の向きで既に
/// 当たっていれば、そこから最も近い当たらない向き (同じ角度なら時計回り) から回す。どの向きでも当たるなら、
/// 衝突体ごとに表面の最も近い点へ押し出す ([`WorldCollider::push_out`]。区間は伸びる)。
/// どの衝突体にも当たらなければ `to` を変えない。`buffer` は作業用 (呼び出しのたびに確保しないため)。
pub(crate) fn constrain_segment(
    colliders: &[WorldCollider],
    margin: f32,
    parent: Vec2,
    from: Vec2,
    to: &mut Vec2,
    buffer: &mut SweepBuffer,
) {
    let length = length(*to - parent);
    // 区間の届く範囲にある衝突体 (親からの距離が `length + radius + margin` 以上なら、どの向きでも当たらない)。
    buffer.near.clear();
    buffer.near.extend(colliders.iter().filter_map(|&collider| {
        let parent_distance = collider.distance_to(parent);
        if parent_distance >= length + collider.radius() + margin {
            None
        } else {
            Some(NearCollider {
                collider,
                parent_distance,
            })
        }
    }));
    let near = buffer.near.as_slice();
    if near.is_empty() {
        return;
    }
    let push_out_all = |p: &mut Vec2| near.iter().for_each(|n| n.collider.push_out(p, margin));
    let Some(target) = (*to - parent).try_normalize() else {
        push_out_all(to);
        return;
    };
    let base = (from - parent).try_normalize().unwrap_or(target);
    // 基準の向きから `angle` (ラジアン、時計回りが正) 回した粒子の位置。
    let at = |angle: f32| parent + Vec2::from_angle(angle).rotate(base) * length;
    let blocked = |angle: f32| {
        near.iter().any(|n| {
            n.collider
                .blocks_from(n.parent_distance, parent, at(angle), margin)
        })
    };
    let min_radius = near
        .iter()
        .map(|n| n.collider.radius())
        .fold(f32::INFINITY, f32::min)
        .max(SWEEP_MIN_RADIUS);
    let step = (min_radius / length)
        .min(SWEEP_MAX_STEP_DEGREES.to_radians())
        .max(core::f32::consts::PI / SWEEP_MAX_SAMPLES as f32);
    let end = math::atan2(base.perp_dot(target), base.dot(target));

    let start = if blocked(0.0) {
        match nearest_free(&blocked, step) {
            Some(start) => start,
            None => {
                push_out_all(to);
                return;
            }
        }
    } else {
        0.0
    };
    let span = wrap_radians(end - start);
    let samples = math::ceil(span.abs() / step).max(1.0) as u32;
    let mut free = start;
    for k in 1..=samples {
        let angle = start + span * (k as f32 / samples as f32);
        if blocked(angle) {
            *to = at(bisect(&blocked, free, angle));
            return;
        }
        free = angle;
    }
    // 最後の標本 (`to` の向き) まで当たらなければ、`to` はそのまま。
}

/// 基準の向き (角度 0、当たっている) から最も近い、当たらない向き。境目まで二分法で詰める。
fn nearest_free(blocked: &impl Fn(f32) -> bool, step: f32) -> Option<f32> {
    let half = math::ceil(core::f32::consts::PI / step) as u32;
    for k in 1..=half {
        for sign in [1.0, -1.0] {
            let angle = sign * (step * k as f32).min(core::f32::consts::PI);
            if !blocked(angle) {
                let inside = sign * step * (k - 1) as f32;
                return Some(bisect(blocked, angle, inside));
            }
        }
    }
    None
}

/// `free` (当たらない) と `hit` (当たる) の間の境目の、当たらない側の角度。
fn bisect(blocked: &impl Fn(f32) -> bool, mut free: f32, mut hit: f32) -> f32 {
    for _ in 0..SWEEP_BISECTIONS {
        let mid = 0.5 * (free + hit);
        if blocked(mid) {
            hit = mid;
        } else {
            free = mid;
        }
    }
    free
}

/// 角度 (ラジアン) を (-π, π] に畳む。
fn wrap_radians(angle: f32) -> f32 {
    use core::f32::consts::{PI, TAU};
    let a = math::rem_euclid(angle + PI, TAU) - PI;
    if a <= -PI { a + TAU } else { a }
}

/// 点 `p` と線分 `a`–`b` の距離。
fn point_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(p - (a + ab * t))
}

/// ベクトルの長さ。`Vec2::length` と同じ値 (どちらも `x * x + y * y` の正しく丸めた平方根) だが、`glam` の中の `libm` の
/// 平方根ではなく [`math::sqrt`] (`std` があれば CPU の命令) を使う。衝突の判定は 1 ステップに数百回呼ぶので、wasm で
/// `libm` の平方根 (ソフトウェア) が物理の段の時間の大半を占めていた (T0193)。
fn length(v: Vec2) -> f32 {
    math::sqrt(v.length_squared())
}

/// 線分 `p`–`q` と線分 `a`–`b` の距離。
fn segment_segment_distance(p: Vec2, q: Vec2, a: Vec2, b: Vec2) -> f32 {
    let d1 = (q - p).perp_dot(a - p);
    let d2 = (q - p).perp_dot(b - p);
    let d3 = (b - a).perp_dot(p - a);
    let d4 = (b - a).perp_dot(q - a);
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return 0.0;
    }
    point_segment_distance(p, a, b)
        .min(point_segment_distance(q, a, b))
        .min(point_segment_distance(a, p, q))
        .min(point_segment_distance(b, p, q))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 区間が衝突体の半径に比べて極端に長くても、調べる刻みの数に上限があるので掃引が終わる
    /// (始めの向きで当たっていないときと、当たっていて当たらない向きを探すとき。T0184)。
    #[test]
    fn extremely_long_segments_finish_the_sweep() {
        let c = WorldCollider::Circle {
            center: Vec2::new(0.0, 50.0),
            radius: 10.0,
        };
        for from in [Vec2::new(1.0e18, 0.0), Vec2::new(0.0, 1.0e18)] {
            let mut to = Vec2::new(-1.0e18, 1.0);
            constrain_segment(
                &[c],
                0.0,
                Vec2::ZERO,
                from,
                &mut to,
                &mut SweepBuffer::default(),
            );
            assert!(to.is_finite(), "{from}: {to}");
        }
    }

    #[test]
    fn circle_pushes_points_to_its_surface() {
        let c = WorldCollider::Circle {
            center: Vec2::new(10.0, 10.0),
            radius: 5.0,
        };
        let mut p = Vec2::new(12.0, 10.0);
        c.push_out(&mut p, 1.0);
        assert!(p.abs_diff_eq(Vec2::new(16.0, 10.0), 1e-5));
        // 外側の点は動かさない。
        let mut q = Vec2::new(20.0, 10.0);
        c.push_out(&mut q, 1.0);
        assert_eq!(q, Vec2::new(20.0, 10.0));
        // 中心の点は上へ。
        let mut r = Vec2::new(10.0, 10.0);
        c.push_out(&mut r, 0.0);
        assert!(r.abs_diff_eq(Vec2::new(10.0, 5.0), 1e-5));
    }

    #[test]
    fn capsule_pushes_away_from_the_segment() {
        let c = WorldCollider::Capsule {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(10.0, 0.0),
            radius: 2.0,
        };
        let mut p = Vec2::new(4.0, 1.0);
        c.push_out(&mut p, 0.0);
        assert!(p.abs_diff_eq(Vec2::new(4.0, 2.0), 1e-5));
        // 端の外側は半円。
        let mut q = Vec2::new(11.0, 0.0);
        c.push_out(&mut q, 0.5);
        assert!(q.abs_diff_eq(Vec2::new(12.5, 0.0), 1e-5));
        // 線分の上の点は法線 (右向きの線分なら上) へ。
        let mut r = Vec2::new(5.0, 0.0);
        c.push_out(&mut r, 0.0);
        assert!(r.abs_diff_eq(Vec2::new(5.0, -2.0), 1e-5));
    }

    /// 親 (0,0) から長さ 10 の区間。
    fn at_degrees(degrees: f32) -> Vec2 {
        Vec2::from_angle(degrees.to_radians()).rotate(Vec2::Y) * 10.0
    }

    #[test]
    fn segment_far_from_colliders_is_unchanged() {
        let c = WorldCollider::Circle {
            center: Vec2::new(100.0, 0.0),
            radius: 5.0,
        };
        let mut to = Vec2::new(3.0, 4.0) * 2.0;
        constrain_segment(
            &[c],
            1.0,
            Vec2::ZERO,
            Vec2::new(0.0, 10.0),
            &mut to,
            &mut SweepBuffer::default(),
        );
        assert_eq!(to, Vec2::new(6.0, 8.0));
    }

    #[test]
    fn sweep_stops_before_a_thin_collider_and_keeps_the_length() {
        // 下 (0,10) から右 (10,0) へ回す途中に、細い円 (半径 0.5) がある。1 ステップで飛び越えない。
        let c = WorldCollider::Circle {
            center: at_degrees(-45.0),
            radius: 0.5,
        };
        let mut to = Vec2::new(10.0, 0.0);
        constrain_segment(
            &[c],
            0.0,
            Vec2::ZERO,
            Vec2::new(0.0, 10.0),
            &mut to,
            &mut SweepBuffer::default(),
        );
        assert!((to.length() - 10.0).abs() < 1e-4, "{to}");
        let d = to.distance(at_degrees(-45.0));
        assert!((0.5..0.51).contains(&d), "{d}");
        // 回した側 (下の側) で止まる。
        assert!(to.y > at_degrees(-45.0).y, "{to}");
    }

    #[test]
    fn segment_does_not_cross_a_collider_near_the_parent() {
        // 粒子の通り道 (半径 10 の円) の内側にある円でも、区間 (線分) が当たれば止める。
        let c = WorldCollider::Circle {
            center: Vec2::new(3.0, 3.0),
            radius: 1.0,
        };
        let mut to = Vec2::new(10.0, 0.0);
        constrain_segment(
            &[c],
            0.0,
            Vec2::ZERO,
            Vec2::new(0.0, 10.0),
            &mut to,
            &mut SweepBuffer::default(),
        );
        assert!((to.length() - 10.0).abs() < 1e-4);
        assert!(!c.blocks(Vec2::ZERO, to, 0.0));
        assert!(to.y > to.x, "stopped on the near side: {to}");
    }

    #[test]
    fn blocked_start_turns_to_the_nearest_free_direction() {
        // 真下 (0,10) を横切る水平なカプセル (y = 8、x = -3..6)。左端の方が近いので左 (反時計回り) に回す。
        let c = WorldCollider::Capsule {
            a: Vec2::new(-3.0, 8.0),
            b: Vec2::new(6.0, 8.0),
            radius: 1.0,
        };
        let mut to = Vec2::new(0.0, 10.0);
        constrain_segment(
            &[c],
            0.5,
            Vec2::ZERO,
            Vec2::new(0.0, 10.0),
            &mut to,
            &mut SweepBuffer::default(),
        );
        assert!((to.length() - 10.0).abs() < 1e-4);
        assert!(!c.blocks(Vec2::ZERO, to, 0.5), "{to}");
        assert!(c.blocks(Vec2::ZERO, to + Vec2::new(0.05, 0.0), 0.5), "{to}");
        assert!(to.x < 0.0, "{to}");
    }

    #[test]
    fn parent_inside_everything_falls_back_to_push_out() {
        let c = WorldCollider::Circle {
            center: Vec2::ZERO,
            radius: 50.0,
        };
        let mut to = Vec2::new(0.0, 10.0);
        constrain_segment(
            &[c],
            0.0,
            Vec2::ZERO,
            Vec2::new(0.0, 10.0),
            &mut to,
            &mut SweepBuffer::default(),
        );
        assert!(to.abs_diff_eq(Vec2::new(0.0, 50.0), 1e-4), "{to}");
    }

    #[test]
    fn segment_distance_detects_crossing() {
        let (a, b) = (Vec2::new(-5.0, 5.0), Vec2::new(5.0, 5.0));
        assert_eq!(
            segment_segment_distance(Vec2::ZERO, Vec2::new(0.0, 10.0), a, b),
            0.0
        );
        assert!(
            (segment_segment_distance(Vec2::ZERO, Vec2::new(0.0, 3.0), a, b) - 2.0).abs() < 1e-6
        );
        assert!((wrap_radians(3.0 * core::f32::consts::PI) - core::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn shapes_follow_the_attachment_transform() {
        let world = WorldTransform::from_affine(ugoe_core::Affine2::from_scale_angle_translation(
            Vec2::splat(2.0),
            0.0,
            Vec2::new(100.0, 50.0),
        ));
        let shape = ColliderShape::Circle {
            center: Vec2::new(1.0, 2.0),
            radius: 3.0,
        };
        // 位置は写し、半径は変えない。
        assert_eq!(
            WorldCollider::new(&shape, &world),
            Some(WorldCollider::Circle {
                center: Vec2::new(102.0, 54.0),
                radius: 3.0,
            })
        );
        assert_eq!(WorldCollider::new(&ColliderShape::Unknown, &world), None);
    }
}
