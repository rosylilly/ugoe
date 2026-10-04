//! 与えられた点群の三角形分割 (エディタでのメッシュの作り直し用)。

use spade::handles::FixedVertexHandle;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};
use ugoe_core::Vec2;

use crate::MAX_VERTICES;

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

/// [`triangulate_points`] のエラー。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TriangulateError {
    /// 点の数が [`MAX_VERTICES`] を超える。
    #[error("{0} points exceed the limit of 65535")]
    TooManyPoints(usize),
    /// 座標が NaN か無限大の点の添字。
    #[error("point {0} is not finite")]
    NonFinite(usize),
    /// 2 つの点が同じ位置にある (三角形分割では 1 点にまとまってしまう)。
    #[error("points {0} and {1} are at the same position")]
    Duplicate(usize, usize),
    /// 制約辺が、存在しない点の添字を指している。
    #[error("constraint refers to point {0}, which does not exist")]
    ConstraintOutOfRange(u16),
}

/// 点群 `points` を三角形分割し、三角形のインデックス (`points` の添字) を返す。
///
/// - `constraints` が空なら、凸包全体のドロネー三角形分割。
/// - 空でなければ、それを制約辺とした制約付きドロネー三角形分割で、重心が制約辺の内側
///   (偶奇規則。閉じた輪郭の外周と穴を表す) にある三角形だけを残す。他の制約辺と交わる辺は
///   張らないが、内外の判定にはすべての制約辺を使う。
///
/// 三角形は画面 (Y 下向き) 上で反時計回り (`docs/spec/rendering.md#裏面カリング` の表)。
/// どの三角形にも使われない点がありうる (輪郭の外の点、すべてが一直線上にある場合など)。
/// 同じ入力からは常に同じ結果を返す。
pub fn triangulate_points(
    points: &[Vec2],
    constraints: &[[u16; 2]],
) -> Result<Vec<u16>, TriangulateError> {
    if points.len() > MAX_VERTICES {
        return Err(TriangulateError::TooManyPoints(points.len()));
    }
    if let Some(i) = points.iter().position(|p| !p.is_finite()) {
        return Err(TriangulateError::NonFinite(i));
    }
    if let Some(&[a, b]) = constraints
        .iter()
        .find(|c| c.iter().any(|&i| usize::from(i) >= points.len()))
    {
        let bad = if usize::from(a) >= points.len() { a } else { b };
        return Err(TriangulateError::ConstraintOutOfRange(bad));
    }

    let mut cdt = Cdt::new();
    // spade の頂点 → 入力の添字
    let mut owner: Vec<usize> = Vec::with_capacity(points.len());
    let mut handles: Vec<Option<FixedVertexHandle>> = Vec::with_capacity(points.len());
    // spade は絶対値が `MIN_ALLOWED_VALUE` (約 1.8e-43) 未満の 0 でない座標を拒むので、0 に寄せる
    // (当たるのは f32 の非正規化数だけ)。f32 の最大値は spade の上限より小さい。
    let coordinate = |v: f32| {
        let v = f64::from(v);
        if v.abs() < spade::MIN_ALLOWED_VALUE {
            0.0
        } else {
            v
        }
    };
    for (i, p) in points.iter().enumerate() {
        // 有限の値は検査済みで、極端に小さい値は 0 に寄せたので、挿入が失敗することはない。
        let handle = cdt
            .insert(Point2::new(coordinate(p.x), coordinate(p.y)))
            .ok();
        if let Some(h) = handle {
            if h.index() < owner.len() {
                return Err(TriangulateError::Duplicate(owner[h.index()], i));
            }
            owner.push(i);
        }
        handles.push(handle);
    }
    for &[a, b] in constraints {
        if let (Some(ha), Some(hb)) = (handles[usize::from(a)], handles[usize::from(b)])
            && ha != hb
        {
            cdt.try_add_constraint(ha, hb);
        }
    }

    let edges: Vec<[Vec2; 2]> = constraints
        .iter()
        .map(|&[a, b]| [points[usize::from(a)], points[usize::from(b)]])
        .collect();
    let mut indices = Vec::new();
    for face in cdt.inner_faces() {
        if !edges.is_empty() {
            let [a, b, c] = face.positions();
            let centroid = [(a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0];
            if !even_odd(&edges, centroid) {
                continue;
            }
        }
        let [a, b, c] = face.vertices();
        // spade は数学座標 (Y 上向き) で反時計回り。Y 下向きの画面で反時計回りにするため反転する
        for v in [a, c, b] {
            indices.push(owner[v.fix().index()] as u16);
        }
    }
    Ok(indices)
}

/// 偶奇規則で点 `p` が辺の集合の内側にあるか。
fn even_odd(edges: &[[Vec2; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    for [a, b] in edges {
        let (a, b) = (a.as_dvec2(), b.as_dvec2());
        if (a.y > p[1]) != (b.y > p[1]) {
            let x = a.x + (p[1] - a.y) * (b.x - a.x) / (b.y - a.y);
            if p[0] < x {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_area2(points: &[Vec2], t: &[u16]) -> f32 {
        let [a, b, c] = [0, 1, 2].map(|k| points[usize::from(t[k])]);
        (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)
    }

    /// 三角形の面積の合計。
    fn area(points: &[Vec2], indices: &[u16]) -> f32 {
        indices
            .chunks(3)
            .map(|t| signed_area2(points, t).abs() / 2.0)
            .sum()
    }

    #[test]
    fn delaunay_without_constraints_covers_the_convex_hull() {
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.5, 0.5),
        ];
        let indices = triangulate_points(&points, &[]).unwrap();
        assert_eq!(indices.len(), 12);
        assert!((area(&points, &indices) - 1.0).abs() < 1e-6);
        // Y 下向きの画面で反時計回り = 数学座標の符号では負
        for t in indices.chunks(3) {
            assert!(signed_area2(&points, t) < 0.0);
        }
    }

    #[test]
    fn constraints_keep_concavities_and_holes() {
        // L 字 (右上の 1/4 が欠けた正方形)
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.5, 0.0),
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 0.5),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let ring: Vec<[u16; 2]> = (0..6).map(|i| [i, (i + 1) % 6]).collect();
        let indices = triangulate_points(&points, &ring).unwrap();
        assert!((area(&points, &indices) - 0.75).abs() < 1e-6);
        // 制約がなければ凸包 (欠けも埋まる。1 - 0.5 * 0.5 / 2)
        let hull = triangulate_points(&points, &[]).unwrap();
        assert!((area(&points, &hull) - 0.875).abs() < 1e-6);

        // 穴: 外周 0..4、内周 4..8
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(0.0, 4.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(3.0, 3.0),
            Vec2::new(1.0, 3.0),
        ];
        let rings: Vec<[u16; 2]> = (0..4)
            .map(|i| [i, (i + 1) % 4])
            .chain((0..4).map(|i| [4 + i, 4 + (i + 1) % 4]))
            .collect();
        let indices = triangulate_points(&points, &rings).unwrap();
        assert!((area(&points, &indices) - 12.0).abs() < 1e-5);
    }

    #[test]
    fn unused_and_invalid_points() {
        // 輪郭の外の点は使われない
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(5.0, 5.0),
        ];
        let indices = triangulate_points(&points, &[[0, 1], [1, 2], [2, 0]]).unwrap();
        assert_eq!(indices.len(), 3);
        assert!(!indices.contains(&3));
        // 一直線上の点からは三角形ができない
        let line = [Vec2::ZERO, Vec2::X, Vec2::X * 2.0];
        assert_eq!(triangulate_points(&line, &[]).unwrap(), Vec::<u16>::new());

        let dup = [Vec2::ZERO, Vec2::X, Vec2::ZERO];
        assert_eq!(
            triangulate_points(&dup, &[]),
            Err(TriangulateError::Duplicate(0, 2))
        );
        assert_eq!(
            triangulate_points(&[Vec2::NAN], &[]),
            Err(TriangulateError::NonFinite(0))
        );
        assert_eq!(
            triangulate_points(&line, &[[0, 3]]),
            Err(TriangulateError::ConstraintOutOfRange(3))
        );
    }

    #[test]
    fn subnormal_coordinates_are_not_dropped() {
        // spade は 0 でない極端に小さい座標を受け付けない。0 とみなして三角形に使う。
        let points = [Vec2::new(1e-44, 0.0), Vec2::X, Vec2::Y];
        assert_eq!(triangulate_points(&points, &[]).unwrap().len(), 3);
        let dup = [Vec2::ZERO, Vec2::X, Vec2::new(0.0, -1e-44)];
        assert_eq!(
            triangulate_points(&dup, &[]),
            Err(TriangulateError::Duplicate(0, 2))
        );
    }
}
