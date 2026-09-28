//! 衝突体 (円・カプセル) からの粒子の押し出し。仕様: `docs/spec/physics.md#衝突体`。

use ugoe_core::Vec2;
use ugoe_model::physics::ColliderShape;
use ugoe_runtime::WorldTransform;

/// キャンバス座標に置いた衝突体の形状。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldCollider {
    Circle {
        center: Vec2,
        radius: f32,
    },
    /// 線分 `a`–`b` から `radius` 以内の領域。
    Capsule {
        a: Vec2,
        b: Vec2,
        radius: f32,
    },
}

impl WorldCollider {
    /// 取り付け先の座標系で表した `shape` を、取り付け先のワールド変換 `world` で置く。
    ///
    /// 位置 (`center`・`a`・`b`) だけを写し、半径はキャンバスのピクセルのまま (取り付け先の
    /// 拡大・ワープの伸縮を受けない)。未知の形状は `None`。
    pub fn new(shape: &ColliderShape, world: &WorldTransform<'_>) -> Option<Self> {
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

    /// 点 `p` が、半径を `margin` だけ広げた衝突体の内側にあれば、表面の最も近い点まで押し出す。
    ///
    /// 押し出す向きは、形状の芯 (円の中心・カプセルの線分) の最も近い点から `p` への向き。
    /// `p` が芯の上にあって向きが決まらなければ、円は上 (`-y`)、カプセルは線分の法線
    /// (線分の向きを反時計回りに 90 度回したもの) の向きに押し出す。
    pub fn push_out(&self, p: &mut Vec2, margin: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
