//! 評価済みの形に対する点の包含判定と、当たり判定 (`Model::hit_areas`) の問い合わせ
//! (`docs/spec/evaluation.md#9-当たり判定`)。
//!
//! どれも直前の [`ModelInstance::update`] の結果を読むだけで、再評価はしない。

use alloc::vec::Vec;
use ugoe_core::Vec2;

use crate::ModelInstance;

impl ModelInstance {
    /// メッシュ `mesh` の変形後の三角形のどれかが、キャンバス座標 `point` を含むか。
    ///
    /// 三角形の内外だけで判定する (辺の上を含む、向きは問わない)。可視性・不透明度・
    /// テクスチャの透明部分は見ない。範囲外の頂点を指す三角形は無視する。`mesh` が範囲外なら `None`。
    pub fn mesh_contains(&self, mesh: usize, point: Vec2) -> Option<bool> {
        let p = &self.drawables().get(mesh)?.positions;
        let hit = (self.compiled().model().meshes.get(mesh)?)
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .any(|t| {
                let [Some(a), Some(b), Some(c)] = t.map(|k| p.get(usize::from(k))) else {
                    return false;
                };
                in_triangle(point, *a, *b, *c)
            });
        Some(hit)
    }

    /// 当たり判定 `hit_area` (`Model::hit_areas` の添字) がキャンバス座標 `point` を含むか。
    /// 判定の規則は [`ModelInstance::mesh_contains`] と同じ。`hit_area` が範囲外なら `None`。
    pub fn hit_area_contains(&self, hit_area: usize, point: Vec2) -> Option<bool> {
        self.mesh_contains(self.compiled().hit_area_mesh(hit_area)?, point)
    }

    /// キャンバス座標 `point` を含む当たり判定 (`Model::hit_areas` の添字) を、手前から順に並べる。
    ///
    /// 順序はメッシュの描画順 ([`ModelInstance::render_order`]) の逆。同じメッシュを使う
    /// 当たり判定どうしは宣言順。可視性・不透明度は見ない (透明な判定用メッシュを使えるように)。
    pub fn hit_areas_at(&self, point: Vec2) -> Vec<usize> {
        let meshes = &self.compiled().hit_area_meshes;
        let mut hits = Vec::new();
        if meshes.is_empty() {
            return hits;
        }
        for &mesh in self.render_order().iter().rev() {
            let mut areas = (meshes.iter().enumerate())
                .filter(|&(_, &m)| m == mesh)
                .map(|(h, _)| h)
                .peekable();
            if areas.peek().is_some() && self.mesh_contains(mesh as usize, point) == Some(true) {
                hits.extend(areas);
            }
        }
        hits
    }
}

/// 点が三角形の内側 (辺の上を含む) にあるか。向きは問わない。
///
/// 3 辺に対する符号がすべて 0 以上か、すべて 0 以下なら内側 (NaN を含めば外側)。面積 0 の三角形では
/// 頂点を通る直線上のすべての点で符号が 0 になるので、外接矩形の中にあることも確かめて、
/// 線分 (または点) の上だけを内側とする。面積を持つ三角形の内側は必ず外接矩形の中なので結果を変えない。
fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let cross = |o: Vec2, u: Vec2, v: Vec2| (u - o).perp_dot(v - o);
    let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
    let same_side = (d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0) || (d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0);
    same_side && p.cmpge(a.min(b).min(c)).all() && p.cmple(a.max(b).max(c)).all()
}

#[cfg(test)]
mod tests {
    use ugoe_model::{HitArea, Model, fixtures};

    use super::*;
    use crate::CompiledModel;

    fn instance(model: Model) -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(model).unwrap())
    }

    fn hit_area(id: &str, mesh: &str) -> HitArea {
        HitArea {
            id: id.into(),
            mesh: mesh.into(),
        }
    }

    #[test]
    fn in_triangle_ignores_winding() {
        let (a, b, c) = (Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0));
        assert!(in_triangle(Vec2::new(2.0, 2.0), a, b, c));
        assert!(in_triangle(Vec2::new(2.0, 2.0), a, c, b));
        assert!(in_triangle(Vec2::ZERO, a, b, c));
        assert!(!in_triangle(Vec2::new(8.0, 8.0), a, b, c));
    }

    // evaluation.md 9.: 面積 0 の三角形が含むのは、その線分 (または点) の上だけ。
    // 潰れたメッシュ (拡大率 0 の回転デフォーマで隠したものなど) がキャンバス全体で当たってはいけない。
    #[test]
    fn degenerate_triangles_contain_only_their_points() {
        let p = Vec2::new(5.0, 5.0);
        assert!(in_triangle(p, p, p, p));
        assert!(!in_triangle(Vec2::new(100.0, 100.0), p, p, p));
        let (a, b, c) = (Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0));
        assert!(in_triangle(Vec2::new(15.0, 0.0), a, b, c));
        assert!(!in_triangle(Vec2::new(100.0, 0.0), a, b, c));
        assert!(!in_triangle(Vec2::new(-1.0, 0.0), a, b, c));
        assert!(!in_triangle(Vec2::new(5.0, 1.0), a, b, c));
        // 頂点が NaN の三角形は何も含まない。
        let nan = Vec2::new(f32::NAN, 0.0);
        assert!(!in_triangle(
            Vec2::new(1.0, 1.0),
            nan,
            b,
            Vec2::new(0.0, 10.0)
        ));
    }

    #[test]
    fn hit_area_follows_deformed_shape() {
        // single_quad の Quad は X=0 で [64, 192]²、X=1 で x が 64 だけ右へずれる。
        let mut model = fixtures::single_quad();
        model.hit_areas.push(hit_area("Body", "Quad"));
        let mut inst = instance(model);
        let (left, right) = (Vec2::new(100.0, 100.0), Vec2::new(220.0, 100.0));
        assert_eq!(inst.hit_areas_at(left), vec![0]);
        assert_eq!(inst.hit_areas_at(right), Vec::<usize>::new());

        let x = inst.parameters().index_of("X").unwrap();
        inst.parameters_mut().set(x, 1.0);
        // update するまでは前回の評価結果で判定する。
        assert_eq!(inst.hit_area_contains(0, left), Some(true));
        inst.update();
        assert_eq!(inst.hit_area_contains(0, left), Some(false));
        assert_eq!(inst.hit_areas_at(right), vec![0]);
    }

    #[test]
    fn hit_areas_ignore_opacity() {
        let mut model = fixtures::single_quad();
        model.hit_areas.push(hit_area("Body", "Quad"));
        model.parts[0].keyforms =
            ugoe_model::KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 0.0 });
        let inst = instance(model);
        assert!(!inst.drawables()[0].visible);
        assert_eq!(inst.hit_areas_at(Vec2::new(100.0, 100.0)), vec![0]);
    }

    #[test]
    fn hit_areas_are_front_to_back() {
        // Sclera (描画順 400) < Iris (500) < Pupil (600)。宣言順は Pupil, Iris, Sclera。
        let mut model = fixtures::nested_masks();
        model.hit_areas = vec![
            hit_area("Eye", "Sclera"),
            hit_area("Pupil", "Pupil"),
            hit_area("EyeAgain", "Sclera"),
        ];
        let inst = instance(model);
        assert_eq!(inst.compiled().hit_area_index("EyeAgain"), Some(2));
        assert_eq!(inst.compiled().hit_area_mesh(2), Some(2));
        // T0187: 範囲外の番号は `None`。
        assert_eq!(inst.compiled().hit_area_mesh(3), None);
        assert_eq!(inst.hit_area_contains(3, Vec2::new(128.0, 100.0)), None);
        assert_eq!(
            inst.mesh_contains(usize::MAX, Vec2::new(128.0, 100.0)),
            None
        );
        assert_eq!(inst.hit_areas_at(Vec2::new(128.0, 100.0)), vec![1, 0, 2]);
        assert_eq!(inst.hit_areas_at(Vec2::new(70.0, 100.0)), vec![0, 2]);
        assert_eq!(
            inst.hit_areas_at(Vec2::new(10.0, 10.0)),
            Vec::<usize>::new()
        );
    }
}
