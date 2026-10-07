//! 評価済みの描画可能要素に対する問い合わせ (外接矩形・ヒットテスト)。GPU には触れない。
//!
//! エディタのビューポートでの選択 (T0606) やギズモの配置に使う。座標はすべてキャンバス座標。

use ugoe::core::Vec2;
use ugoe::runtime::ModelInstance;

/// 要素ごとの外接矩形 `[min_x, min_y, max_x, max_y]` を `Model::meshes` の順に並べる。
/// 頂点を持たない要素は NaN で埋める。
pub fn drawable_bounds(instance: &ModelInstance) -> Vec<f32> {
    let mut out = Vec::with_capacity(instance.drawables().len() * 4);
    for d in instance.drawables() {
        let Some(first) = d.positions.first() else {
            out.extend_from_slice(&[f32::NAN; 4]);
            continue;
        };
        let (mut min, mut max) = (*first, *first);
        for p in d.positions.iter().skip(1) {
            min = min.min(*p);
            max = max.max(*p);
        }
        out.extend_from_slice(&[min.x, min.y, max.x, max.y]);
    }
    out
}

/// キャンバス座標 `point` にある、いちばん手前の表示中の要素 (`Model::meshes` の添字)。
///
/// 三角形の内外だけで判定し (`ModelInstance::mesh_contains`)、テクスチャの透明部分は考慮しない。
/// 非表示 (不透明度 0・マスク専用) の要素は除く。
pub fn hit_test(instance: &ModelInstance, point: [f32; 2]) -> Option<u32> {
    let mut hits = instance.render_order().iter().rev().copied();
    hits.find(|&i| contains(instance, i, point))
}

/// キャンバス座標 `point` にある表示中の要素のすべて (手前から奥へ)。条件は [`hit_test`] と同じ。
/// 重なった要素を順に選ぶ (Alt + クリック) のに使う。
pub fn hit_test_all(instance: &ModelInstance, point: [f32; 2]) -> Vec<u32> {
    instance
        .render_order()
        .iter()
        .rev()
        .copied()
        .filter(|&i| contains(instance, i, point))
        .collect()
}

/// 要素 `i` が表示中で、その三角形のどれかが `point` を含むか。
fn contains(instance: &ModelInstance, i: u32, point: [f32; 2]) -> bool {
    let Some(d) = instance.drawables().get(i as usize) else {
        return false;
    };
    if !d.visible || d.opacity <= 0.0 {
        return false;
    }
    instance.mesh_contains(i as usize, Vec2::from(point)) == Some(true)
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;
    use ugoe::runtime::CompiledModel;

    use super::*;

    fn instance() -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(fixtures::single_quad()).unwrap())
    }

    #[test]
    fn bounds_follow_parameters() {
        let mut inst = instance();
        assert_eq!(drawable_bounds(&inst), vec![64.0, 64.0, 192.0, 192.0]);
        let x = inst.parameters().index_of("X").unwrap();
        inst.parameters_mut().set(x, 1.0);
        inst.update();
        assert_eq!(drawable_bounds(&inst), vec![128.0, 64.0, 256.0, 192.0]);
    }

    #[test]
    fn hit_test_uses_triangles() {
        let inst = instance();
        assert_eq!(hit_test(&inst, [100.0, 100.0]), Some(0));
        assert_eq!(hit_test(&inst, [64.0, 64.0]), Some(0));
        assert_eq!(hit_test(&inst, [10.0, 100.0]), None);
    }

    #[test]
    fn hit_test_all_lists_overlaps_front_to_back() {
        // Sclera (描画順 400) < Iris (500) < Pupil (600)。宣言順は Pupil, Iris, Sclera。
        let inst = ModelInstance::new(CompiledModel::compile(fixtures::nested_masks()).unwrap());
        assert_eq!(hit_test_all(&inst, [128.0, 100.0]), vec![0, 1, 2]);
        assert_eq!(hit_test(&inst, [128.0, 100.0]), Some(0));
        assert_eq!(hit_test_all(&inst, [70.0, 100.0]), vec![2]);
        assert_eq!(hit_test_all(&inst, [10.0, 10.0]), Vec::<u32>::new());
    }
}
