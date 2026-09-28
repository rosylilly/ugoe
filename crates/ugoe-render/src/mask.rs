//! マスクアトラスの割り当て (GPU を使わない部分)。方式は `docs/spec/rendering.md` の「マスク」。
//!
//! - アトラス 1 枚は `size × size` の RGBA8。4 つのチャンネルそれぞれを `g × g` のセルに等分し、
//!   マスク集合 1 つにセル 1 つ (矩形 + チャンネル) を割り当てる。1 枚に入るのは `4 g²` 集合。
//! - `g` はアトラスごとに、そのアトラスに入る集合がすべて収まる最小の値を選ぶ
//!   (集合が少ないほどセルが大きく、解像度が高い)。上限は `size / min_cell_size`。
//! - 上限で収まらない集合は次のアトラスに回す。

/// マスクアトラスの設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskAtlasOptions {
    /// アトラス 1 枚の一辺 (texel)。デバイスの `max_texture_dimension_2d` を超える値は切り詰める。
    pub size: u32,
    /// セルの一辺の下限 (texel)。1 チャンネルを分割する数の上限は `size / min_cell_size`。
    pub min_cell_size: u32,
}

impl Default for MaskAtlasOptions {
    fn default() -> Self {
        Self {
            size: 1024,
            min_cell_size: 256,
        }
    }
}

/// セル内の縁の幅 (texel)。バイリニア補間で隣のセルの値を拾わないよう、
/// 対象要素の外接矩形はセルからこの幅だけ内側に写す (縁にもマスクは描かれる)。
const CELL_PADDING: u32 = 1;

/// 1 つのマスク集合に割り当てたアトラス上の場所。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaskSlot {
    /// 何枚目のアトラスか。
    pub atlas: u32,
    /// 0..4 (R, G, B, A)。
    pub channel: u32,
    /// セル (texel、左上原点): x, y, width, height。マスクパスのシザー矩形にも使う。
    pub cell: [u32; 4],
}

impl MaskSlot {
    /// チャンネル選択の one-hot ベクトル。
    pub fn channel_mask(&self) -> [f32; 4] {
        let mut v = [0.0; 4];
        v[self.channel as usize] = 1.0;
        v
    }

    /// 画面 UV (0..1、左上原点) の外接矩形 `[x0, y0, x1, y1]` をセルの内側に写す変換。
    ///
    /// 戻り値 `[ox, oy, sx, sy]` で `atlas_uv = (ox, oy) + screen_uv * (sx, sy)`。
    pub fn transform(&self, atlas_size: u32, bbox: [f32; 4]) -> [f32; 4] {
        let [x, y, w, h] = self.cell;
        let pad = if w.min(h) > CELL_PADDING * 2 {
            CELL_PADDING
        } else {
            0
        };
        let size = atlas_size as f32;
        let inner = [
            (x + pad) as f32 / size,
            (y + pad) as f32 / size,
            (w - pad * 2) as f32 / size,
            (h - pad * 2) as f32 / size,
        ];
        let sx = inner[2] / (bbox[2] - bbox[0]);
        let sy = inner[3] / (bbox[3] - bbox[1]);
        [inner[0] - bbox[0] * sx, inner[1] - bbox[1] * sy, sx, sy]
    }
}

/// アトラスの一辺と、1 チャンネルの分割数の上限。
pub(crate) fn atlas_geometry(options: MaskAtlasOptions, max_texture_size: u32) -> (u32, u32) {
    let size = options.size.clamp(1, max_texture_size.max(1));
    let max_grid = (size / options.min_cell_size.max(1)).max(1);
    (size, max_grid)
}

/// `count` 個のマスク集合に場所を割り当てる。戻り値は集合の順に並ぶ。
pub(crate) fn allocate(count: usize, size: u32, max_grid: u32) -> Vec<MaskSlot> {
    let per_atlas = (4 * max_grid * max_grid) as usize;
    let mut slots = Vec::with_capacity(count);
    for start in (0..count).step_by(per_atlas) {
        let atlas = (start / per_atlas) as u32;
        let n = (count - start).min(per_atlas) as u32;
        // 4 g² >= n を満たす最小の g。
        let grid = (1..=max_grid).find(|g| 4 * g * g >= n).unwrap_or(max_grid);
        let cell_size = size / grid;
        for j in 0..n {
            let cell = j / 4;
            slots.push(MaskSlot {
                atlas,
                channel: j % 4,
                cell: [
                    (cell % grid) * cell_size,
                    (cell / grid) * cell_size,
                    cell_size,
                    cell_size,
                ],
            });
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_atlas_is_at_least_1024() {
        let (size, max_grid) = atlas_geometry(MaskAtlasOptions::default(), 8192);
        assert!(size >= 1024);
        assert_eq!(max_grid, 4);
        // デバイスの上限で切り詰める。
        assert_eq!(atlas_geometry(MaskAtlasOptions::default(), 512), (512, 2));
    }

    #[test]
    fn few_sets_use_whole_atlas_per_channel() {
        let slots = allocate(4, 1024, 4);
        assert_eq!(slots.len(), 4);
        for (i, s) in slots.iter().enumerate() {
            assert_eq!(s.atlas, 0);
            assert_eq!(s.channel, i as u32);
            assert_eq!(s.cell, [0, 0, 1024, 1024]);
        }
    }

    #[test]
    fn grid_grows_with_set_count() {
        let slots = allocate(5, 1024, 4);
        // 4 g² >= 5 → g = 2。
        assert!(slots.iter().all(|s| s.cell[2] == 512 && s.atlas == 0));
        assert_eq!(slots[4].channel, 0);
        assert_eq!(slots[4].cell, [512, 0, 512, 512]);
        // 同じチャンネル・同じセルを 2 つの集合が使わない。
        let slots = allocate(64, 1024, 4);
        for (i, a) in slots.iter().enumerate() {
            for b in &slots[i + 1..] {
                assert!((a.atlas, a.channel, a.cell) != (b.atlas, b.channel, b.cell));
            }
        }
    }

    #[test]
    fn overflow_allocates_more_atlases() {
        let slots = allocate(5, 32, 1);
        assert_eq!(slots.iter().filter(|s| s.atlas == 0).count(), 4);
        assert_eq!(slots[4].atlas, 1);
        assert_eq!(slots[4].channel, 0);
        assert_eq!(slots[4].cell, [0, 0, 32, 32]);
        // 2 枚目は入る集合の数で分割数を決め直す。
        let slots = allocate(16 + 1, 1024, 2);
        assert_eq!(slots[16].atlas, 1);
        assert_eq!(slots[16].cell, [0, 0, 1024, 1024]);
    }

    #[test]
    fn transform_maps_bbox_into_padded_cell() {
        let slot = MaskSlot {
            atlas: 0,
            channel: 2,
            cell: [512, 0, 512, 512],
        };
        assert_eq!(slot.channel_mask(), [0.0, 0.0, 1.0, 0.0]);
        let bbox = [0.25, 0.5, 0.75, 1.0];
        let [ox, oy, sx, sy] = slot.transform(1024, bbox);
        let map = |u: f32, v: f32| (ox + u * sx, oy + v * sy);
        let (x0, y0) = map(bbox[0], bbox[1]);
        let (x1, y1) = map(bbox[2], bbox[3]);
        let eps = 1e-6;
        assert!((x0 - 513.0 / 1024.0).abs() < eps);
        assert!((y0 - 1.0 / 1024.0).abs() < eps);
        assert!((x1 - 1023.0 / 1024.0).abs() < eps);
        assert!((y1 - 511.0 / 1024.0).abs() < eps);
    }
}
