//! 輪郭の抽出 (marching squares)・簡略化 (Douglas–Peucker)・再サンプリング。

use crate::mask::Mask;

/// 2 次元の点 (f64)。内部計算用。
pub(crate) type P = [f64; 2];

/// マスクの境界を閉じた折れ線の列として取り出す。
///
/// 格子点はセルの中心 (`(x + 0.5, y + 0.5)` に `offset` を足した位置) で、輪郭は中身と外の
/// 格子点を結ぶ辺の中点を通る。マスクの外周のセルは中身であってはならない (輪郭が閉じなくなる)。
/// 対角にだけ中身がある鞍点は、中身どうしをつなぐ (中身は 8 近傍連結)。
/// 輪郭は外周と穴を区別せずに返す (偶奇規則で内外を判定する)。
pub(crate) fn extract(mask: &Mask, offset: P) -> Vec<Vec<P>> {
    let (w, h) = (mask.width, mask.height);
    if w < 2 || h < 2 {
        return Vec::new();
    }
    // 辺の ID: (x, y) から右へ伸びる辺は 2 * (y * w + x)、下へ伸びる辺はそれ + 1。
    // next[a] = b は「辺 a の中点から辺 b の中点へ向かう線分」がある (中身が左手側) ことを表す。
    const NONE: u32 = u32::MAX;
    let mut next = vec![NONE; 2 * w * h];
    let hid = |x: usize, y: usize| (2 * (y * w + x)) as u32;
    let vid = |x: usize, y: usize| (2 * (y * w + x) + 1) as u32;
    // セル内の局所座標: 角 TL(0,0) TR(1,0) BR(1,1) BL(0,1)、辺 上/右/下/左 の中点
    const CORNER: [P; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    const MID: [P; 4] = [[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]];
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            let c = [
                mask.get(x, y),
                mask.get(x + 1, y),
                mask.get(x + 1, y + 1),
                mask.get(x, y + 1),
            ];
            let edge_id = [hid(x, y), vid(x + 1, y), hid(x, y + 1), vid(x, y)];
            // 辺 i は角 i と角 (i + 1) % 4 を結ぶ
            let mut link = |a: usize, b: usize, corner: usize| {
                // 参照する角が中身なら左手側 (外積が正) に来る向きにする
                let (p, q, r) = (MID[a], MID[b], CORNER[corner]);
                let cross = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
                let (a, b) = if (cross > 0.0) == c[corner] {
                    (a, b)
                } else {
                    (b, a)
                };
                next[edge_id[a] as usize] = edge_id[b];
            };
            match (c[0], c[1], c[2], c[3]) {
                // 鞍点: 外側の角をそれぞれ切り落とす
                (true, false, true, false) => {
                    link(0, 1, 1);
                    link(2, 3, 3);
                }
                (false, true, false, true) => {
                    link(3, 0, 0);
                    link(1, 2, 2);
                }
                _ => {
                    let mut crossing = (0..4).filter(|&i| c[i] != c[(i + 1) % 4]);
                    if let (Some(a), Some(b)) = (crossing.next(), crossing.next()) {
                        link(a, b, 0);
                    }
                }
            }
        }
    }
    let point = |id: u32| -> P {
        let i = (id / 2) as usize;
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        if id.is_multiple_of(2) {
            [x + 1.0 + offset[0], y + 0.5 + offset[1]]
        } else {
            [x + 0.5 + offset[0], y + 1.0 + offset[1]]
        }
    };
    let mut loops = Vec::new();
    for start in 0..next.len() {
        if next[start] == NONE {
            continue;
        }
        let mut ring = Vec::new();
        let mut cur = start;
        loop {
            let n = next[cur];
            if n == NONE {
                break;
            }
            next[cur] = NONE;
            ring.push(point(cur as u32));
            cur = n as usize;
        }
        loops.push(ring);
    }
    loops
}

/// 閉じた折れ線を Douglas–Peucker で簡略化する。点が 3 つ未満になったら `None`。
pub(crate) fn simplify(ring: &[P], tolerance: f64) -> Option<Vec<P>> {
    let n = ring.len();
    if n < 3 {
        return None;
    }
    // 始点から最も遠い点で 2 本の開いた折れ線に分ける
    let far = (1..n)
        .max_by(|&a, &b| dist2(ring[0], ring[a]).total_cmp(&dist2(ring[0], ring[b])))
        .unwrap_or(1);
    let at = |i: usize| ring[i % n];
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    let mut stack = vec![(0usize, far), (far, n)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (at(a), at(b));
        let mut best = (0.0, a);
        for i in a + 1..b {
            let d = segment_distance2(at(i), pa, pb);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tolerance * tolerance {
            keep[best.1] = true;
            stack.push((a, best.1));
            stack.push((best.1, b));
        }
    }
    let out: Vec<P> = (0..n).filter(|&i| keep[i]).map(|i| ring[i]).collect();
    (out.len() >= 3).then_some(out)
}

/// 閉じた折れ線の各辺を、長さが `spacing` 以下になるように等分する。角の点はそのまま残す。
/// 座標は f32 で表せる値に丸める (出力の頂点と三角形分割の入力を一致させるため)。
pub(crate) fn resample(ring: &[P], spacing: f64) -> Vec<P> {
    let mut out = Vec::new();
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        let len = dist2(a, b).sqrt();
        let k = (len / spacing).ceil().max(1.0) as usize;
        for j in 0..k {
            let t = j as f64 / k as f64;
            out.push(round_f32([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
            ]));
        }
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

pub(crate) fn round_f32(p: P) -> P {
    [f64::from(p[0] as f32), f64::from(p[1] as f32)]
}

fn dist2(a: P, b: P) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

fn segment_distance2(p: P, a: P, b: P) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist2(p, [a[0] + ab[0] * t, a[1] + ab[1] * t])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_from(rows: &[&str]) -> Mask {
        let mut mask = Mask::new(rows[0].len(), rows.len());
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                mask.cells[y * mask.width + x] = ch == '#';
            }
        }
        mask
    }

    fn signed_area(ring: &[P]) -> f64 {
        let n = ring.len();
        (0..n)
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn single_pixel_gives_a_diamond() {
        let loops = extract(&mask_from(&["...", ".#.", "..."]), [0.0, 0.0]);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
        assert!((signed_area(&loops[0]).abs() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn ring_gives_outer_and_hole_with_opposite_orientation() {
        let loops = extract(
            &mask_from(&[".....", ".###.", ".#.#.", ".###.", "....."]),
            [0.0, 0.0],
        );
        assert_eq!(loops.len(), 2);
        let (a, b) = (signed_area(&loops[0]), signed_area(&loops[1]));
        assert!(a * b < 0.0, "areas {a} {b}");
    }

    #[test]
    fn saddle_joins_diagonal_pixels() {
        let loops = extract(&mask_from(&["....", ".#..", "..#.", "...."]), [0.0, 0.0]);
        assert_eq!(loops.len(), 1);
    }

    #[test]
    fn simplify_keeps_corners_of_a_square() {
        let mut ring = Vec::new();
        for i in 0..10 {
            ring.push([i as f64, 0.0]);
        }
        for i in 0..10 {
            ring.push([10.0, i as f64]);
        }
        for i in 0..10 {
            ring.push([10.0 - i as f64, 10.0]);
        }
        for i in 0..10 {
            ring.push([0.0, 10.0 - i as f64]);
        }
        let s = simplify(&ring, 0.1).expect("square");
        assert_eq!(s.len(), 4);
        let r = resample(&s, 4.0);
        assert_eq!(r.len(), 12);
    }
}
