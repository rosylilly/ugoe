//! 制約付きドロネー三角形分割と、残す三角形の選別。

use spade::handles::{FixedFaceHandle, FixedVertexHandle, InnerTag};
use spade::{ConstrainedDelaunayTriangulation, Point2, PositionInTriangulation, Triangulation};
use ugoe_core::Vec2;

use crate::GeneratedMesh;
use crate::contour::P;
use crate::mask::Mask;

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

/// 輪郭 `rings` を制約辺として三角形分割し、偶奇規則で輪郭の内側にある三角形を残す。
///
/// `guard` は凸包を広げるための番兵で、残した三角形が使わなければ出力に現れない。
/// 最後に `opaque` の各画素の中心を含む三角形を必ず残す (制約辺を張れなかった場合や、
/// 簡略化で輪郭が画素の内側に入った場合の安全策)。
pub(crate) fn build(
    rings: &[Vec<P>],
    interior: &[P],
    guard: &[P; 4],
    opaque: &Mask,
    offset: P,
) -> GeneratedMesh {
    let mut cdt = Cdt::new();
    let insert = |cdt: &mut Cdt, p: P| -> Option<FixedVertexHandle> {
        cdt.insert(Point2::new(p[0], p[1])).ok()
    };
    for &p in guard {
        insert(&mut cdt, p);
    }
    let mut ring_handles = Vec::with_capacity(rings.len());
    for ring in rings {
        let handles: Vec<FixedVertexHandle> =
            ring.iter().filter_map(|&p| insert(&mut cdt, p)).collect();
        ring_handles.push(handles);
    }
    for &p in interior {
        insert(&mut cdt, p);
    }
    for handles in &ring_handles {
        for (i, &a) in handles.iter().enumerate() {
            let b = handles[(i + 1) % handles.len()];
            if a != b {
                // 他の制約辺と交わる辺は張らない (偶奇判定と最後の安全策で補う)
                cdt.try_add_constraint(a, b);
            }
        }
    }

    let parity = EvenOdd::new(rings);
    let mut keep = vec![false; cdt.num_all_faces()];
    for face in cdt.inner_faces() {
        let [a, b, c] = face.positions();
        let centroid = [(a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0];
        keep[face.fix().index()] = parity.contains(centroid);
    }

    force_cover(&cdt, opaque, offset, &mut keep);

    // 使われた頂点だけを、最初に使われた順に番号を振り直す
    const UNUSED: u32 = u32::MAX;
    let mut remap = vec![UNUSED; cdt.num_vertices()];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for face in cdt.inner_faces() {
        if !keep[face.fix().index()] {
            continue;
        }
        let [a, b, c] = face.vertices();
        // spade は数学座標 (Y 上向き) で反時計回り。Y 下向きの画面で反時計回りにするため反転する
        for v in [a, c, b] {
            let slot = &mut remap[v.fix().index()];
            if *slot == UNUSED {
                *slot = vertices.len() as u32;
                let p = v.position();
                vertices.push(Vec2::new(p.x as f32, p.y as f32));
            }
            indices.push(*slot as u16);
        }
    }
    GeneratedMesh { vertices, indices }
}

/// 中身の画素の中心を含む三角形を残す。
fn force_cover(cdt: &Cdt, opaque: &Mask, offset: P, keep: &mut [bool]) {
    let Some(first) = cdt.vertices().next() else {
        return;
    };
    let mut hint = first.fix();
    let mut faces: Vec<FixedFaceHandle<InnerTag>> = Vec::new();
    for y in 0..opaque.height {
        for x in 0..opaque.width {
            if !opaque.get(x, y) {
                continue;
            }
            let p = Point2::new(x as f64 + 0.5 + offset[0], y as f64 + 0.5 + offset[1]);
            faces.clear();
            match cdt.locate_with_hint(p, hint) {
                PositionInTriangulation::OnFace(f) => {
                    hint = cdt.face(f).vertices()[0].fix();
                    faces.push(f);
                }
                PositionInTriangulation::OnEdge(e) => {
                    let edge = cdt.directed_edge(e);
                    hint = edge.from().fix();
                    faces.extend(
                        [edge.face(), edge.rev().face()]
                            .iter()
                            .filter_map(|f| f.as_inner())
                            .map(|f| f.fix()),
                    );
                }
                PositionInTriangulation::OnVertex(v) => {
                    hint = v;
                    faces.extend(
                        cdt.vertex(v)
                            .out_edges()
                            .filter_map(|e| e.face().as_inner())
                            .map(|f| f.fix()),
                    );
                }
                // 番兵が凸包を広げているので起こらない
                PositionInTriangulation::OutsideOfConvexHull(_)
                | PositionInTriangulation::NoTriangulation => {}
            }
            // 点を含む三角形がどれも残っていなければ、番号の最も小さいものを残す
            if !faces.iter().any(|f| keep[f.index()])
                && let Some(f) = faces.iter().min_by_key(|f| f.index())
            {
                keep[f.index()] = true;
            }
        }
    }
}

/// 偶奇規則による点の内外判定。線分を y の整数の帯ごとに振り分けて速くする。
struct EvenOdd {
    y0: f64,
    rows: Vec<Vec<(P, P)>>,
}

impl EvenOdd {
    fn new(rings: &[Vec<P>]) -> Self {
        let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in rings.iter().flatten() {
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        if y0 > y1 {
            return Self {
                y0: 0.0,
                rows: Vec::new(),
            };
        }
        let y0 = y0.floor();
        let mut rows = vec![Vec::new(); (y1 - y0).floor() as usize + 1];
        for ring in rings {
            for (i, &a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                if a[1] == b[1] {
                    continue;
                }
                let lo = (a[1].min(b[1]) - y0).floor() as usize;
                let hi = (a[1].max(b[1]) - y0).floor() as usize;
                for row in &mut rows[lo..=hi] {
                    row.push((a, b));
                }
            }
        }
        Self { y0, rows }
    }

    fn contains(&self, p: P) -> bool {
        let row = (p[1] - self.y0).floor();
        if row < 0.0 || row as usize >= self.rows.len() {
            return false;
        }
        let mut inside = false;
        for &(a, b) in &self.rows[row as usize] {
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                if p[0] < x {
                    inside = !inside;
                }
            }
        }
        inside
    }
}
