//! 評価済みの形に対する点の包含判定と、当たり判定 (`Model::hit_areas`) の問い合わせ
//! (`docs/spec/evaluation.md#9-当たり判定`)。
//!
//! どれも直前の [`ModelInstance::update`] の結果を読むだけで、再評価はしない。

use ugoe_core::Vec2;

use crate::ModelInstance;

impl ModelInstance {
    /// メッシュ `mesh` の変形後の三角形のどれかが、キャンバス座標 `point` を含むか。
    ///
    /// 三角形の内外だけで判定する (辺の上を含む、向きは問わない)。可視性・不透明度・
    /// テクスチャの透明部分は見ない。範囲外の頂点を指す三角形は無視する。
    ///
    /// # Panics
    ///
    /// `mesh` が範囲外のとき。
    pub fn mesh_contains(&self, mesh: usize, point: Vec2) -> bool {
        let p = &self.drawables()[mesh].positions;
        self.compiled().model().meshes[mesh]
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .any(|t| {
                let [Some(a), Some(b), Some(c)] = t.map(|k| p.get(usize::from(k))) else {
                    return false;
                };
                in_triangle(point, *a, *b, *c)
            })
    }

    /// 当たり判定 `hit_area` (`Model::hit_areas` の添字) がキャンバス座標 `point` を含むか。
    /// 判定の規則は [`ModelInstance::mesh_contains`] と同じ。
    ///
    /// # Panics
    ///
    /// `hit_area` が範囲外のとき。
    pub fn hit_area_contains(&self, hit_area: usize, point: Vec2) -> bool {
        self.mesh_contains(self.compiled().hit_area_mesh(hit_area), point)
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
            let mut areas = (0..meshes.len()).filter(|&h| meshes[h] == mesh).peekable();
            if areas.peek().is_some() && self.mesh_contains(mesh as usize, point) {
                hits.extend(areas);
            }
        }
        hits
    }
}

/// 点が三角形の内側 (辺の上を含む) にあるか。向きは問わない。
fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let cross = |o: Vec2, u: Vec2, v: Vec2| (u - o).perp_dot(v - o);
    let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
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
        assert!(inst.hit_area_contains(0, left));
        inst.update();
        assert!(!inst.hit_area_contains(0, left));
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
        assert_eq!(inst.compiled().hit_area_mesh(2), 2);
        assert_eq!(inst.hit_areas_at(Vec2::new(128.0, 100.0)), vec![1, 0, 2]);
        assert_eq!(inst.hit_areas_at(Vec2::new(70.0, 100.0)), vec![0, 2]);
        assert_eq!(
            inst.hit_areas_at(Vec2::new(10.0, 10.0)),
            Vec::<usize>::new()
        );
    }
}
