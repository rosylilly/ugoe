//! マスクの内容の決定とアトラスの割り当て (GPU を使わない部分)。
//! 意味論と方式は `docs/spec/rendering.md` の「マスク」。
//!
//! - **マスクの内容**: アトラスの 1 チャンネルに描く値。同じ値になる要素どうしで共有する。
//!   - `Product(ms)`: `Π_{i∈ms} (1 − c_i)`。通常マスクの和集合 (読むときに 1 から引く) と、
//!     反転マスクだけの要素 (そのまま読む) が同じ内容を共有する。
//!   - `Clip { pos, intersection, neg }`: 係数 `m = pos × (1 − neg)` そのもの。
//!     通常と反転の併用と、2 つ以上の通常マスクの共通部分に使う。
//! - アトラスは RGBA8。チャンネル R・G・B に内容を描き、A はマスクパスの作業領域に使う。
//! - 各内容に、画面上の外接矩形の画素数以上 (`downscale` で割った分) のセルを割り当てる。
//!   チャンネルごとに棚詰め (shelf packing) で詰め、入らなければアトラスを増やす。
//! - 入れ子のマスクは、マスクの依存の深さ (レベル) ごとに別のアトラスに描く
//!   (描いている最中のアトラスを読まないため)。
//! - アトラスは必要になればすぐ大きくし、要らなくなった状態が一定フレーム続いたら縮小・解放する
//!   ([`AtlasPool`])。

use std::collections::HashMap;

use ugoe_model::MaskMode;
use ugoe_runtime::CompiledModel;

/// マスクアトラスの設定 (`docs/spec/rendering.md` の「精度」)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskAtlasOptions {
    /// アトラス 1 枚の一辺の上限 (texel)。デバイスの `max_texture_dimension_2d` で切り詰める。
    /// 画面上の外接矩形がこれに収まらないマスクは、縮小して描く。
    pub max_size: u32,
    /// マスクを描く解像度を下げる割合。1 (既定) なら画面の画素と同じ解像度、
    /// 2 なら縦横 1/2 の解像度で描く (縁がぼやける代わりに、アトラスが小さく描画も軽くなる)。
    /// 0 は 1 とみなす。
    pub downscale: u32,
    /// アトラスを縮小・解放するまでのフレーム数。必要な一辺が今の半分以下のフレーム
    /// (または使わないアトラスがあるフレーム) がこれだけ続いたら、アトラスを作り直して小さくする。
    /// 大きさが揺れるたびに作り直さないためのヒステリシス。0 なら縮小しない。
    pub shrink_after_frames: u32,
}

impl Default for MaskAtlasOptions {
    fn default() -> Self {
        Self {
            max_size: 4096,
            downscale: 1,
            shrink_after_frames: 120,
        }
    }
}

/// アトラスの一辺の下限 (texel)。小さいアトラスを作り直し続けないようにする。
pub(crate) const MIN_ATLAS_SIZE: u32 = 256;

/// 内容を描くチャンネルの数 (R, G, B)。A は作業領域。
pub(crate) const CONTENT_CHANNELS: u32 = 3;

/// セル内の縁の幅 (texel)。バイリニア補間で隣のセルの値を拾わないよう、
/// 外接矩形はセルからこの幅だけ内側に写す (縁にもマスクは描かれる)。
pub(crate) const CELL_PADDING: u32 = 1;

/// メッシュ 1 つが自身のマスクとして読む内容。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MeshMask {
    /// [`MaskContent`] のインデックス。
    pub content: u32,
    /// 読んだ値を 1 から引くか (`Product` を通常マスクの和集合として読む場合)。
    pub complement: bool,
}

/// マスクパスで 1 つの内容を描く手順の 1 段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaskStep {
    /// 作業領域 (A) を 1 で埋める。
    FillScratch,
    /// マスク用メッシュ `mesh` を描き、書き込み先に `1 − c` を掛ける。
    /// `scratch` なら作業領域 (A) に、そうでなければ内容のチャンネルに掛ける。
    Draw { mesh: u32, scratch: bool },
    /// 内容のチャンネルに `1 − 作業領域` を掛ける。
    Resolve,
}

/// アトラスの 1 チャンネルに描く、マスクの内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaskContent {
    /// 描くマスク用メッシュ (重複なし、昇順)。外接矩形とテクスチャの検査に使う。
    pub members: Vec<u32>,
    pub steps: Vec<MaskStep>,
    /// マスク用メッシュのマスクの依存の深さの最大値。レベルの小さい内容から描く。
    pub level: u32,
}

impl MaskContent {
    /// `Draw` の段の数 (マスクパスで使うユニフォームのスロット数)。
    pub fn draw_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| matches!(s, MaskStep::Draw { .. }))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ContentKey {
    Product(Vec<u32>),
    Clip {
        pos: Vec<u32>,
        intersection: bool,
        neg: Vec<u32>,
    },
}

/// メッシュごとに読むマスクの内容と、内容の一覧を求める。
///
/// 内容の並びはモデルで初めて現れた順 (フレーム間で安定)。
pub(crate) fn build(compiled: &CompiledModel) -> (Vec<Option<MeshMask>>, Vec<MaskContent>) {
    let mesh_count = compiled.model().meshes.len();
    let mut contents: Vec<MaskContent> = Vec::new();
    let mut index_of: HashMap<ContentKey, u32> = HashMap::new();
    let per_mesh = (0..mesh_count)
        .map(|mesh| {
            let (key, complement) = content_key(compiled, mesh)?;
            let content = *index_of.entry(key).or_insert_with_key(|key| {
                contents.push(content_of(compiled, key));
                contents.len() as u32 - 1
            });
            Some(MeshMask {
                content,
                complement,
            })
        })
        .collect();
    (per_mesh, contents)
}

/// メッシュのマスクの内容のキーと、読むときに 1 から引くか。マスクがなければ `None`。
fn content_key(compiled: &CompiledModel, mesh: usize) -> Option<(ContentKey, bool)> {
    let refs = compiled.mask_refs(mesh);
    let set = |invert: bool| {
        let mut v: Vec<u32> = refs
            .iter()
            .filter(|r| r.invert == invert)
            .map(|r| r.mesh)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let (pos, neg) = (set(false), set(true));
    let intersection = compiled.mask_mode(mesh) == MaskMode::Intersection && pos.len() >= 2;
    Some(match (pos.is_empty(), neg.is_empty()) {
        (true, true) => return None,
        // 和集合 (通常マスク 1 つの共通部分も同じ): pos = 1 − Π(1 − c)。
        (false, true) if !intersection => (ContentKey::Product(pos), true),
        // 反転マスクだけ: m = 1 − neg = Π(1 − c)。
        (true, false) => (ContentKey::Product(neg), false),
        _ => (
            ContentKey::Clip {
                pos,
                intersection,
                neg,
            },
            false,
        ),
    })
}

fn content_of(compiled: &CompiledModel, key: &ContentKey) -> MaskContent {
    let draw = |mesh: u32, scratch: bool| MaskStep::Draw { mesh, scratch };
    let (mut members, steps) = match key {
        ContentKey::Product(ms) => (ms.clone(), ms.iter().map(|&m| draw(m, false)).collect()),
        ContentKey::Clip {
            pos,
            intersection,
            neg,
        } => {
            let mut steps = Vec::new();
            if *intersection {
                // 共通部分: 通常マスクごとに、作業領域に 1 − c を作ってチャンネルに c を掛ける。
                for &p in pos {
                    steps.extend([MaskStep::FillScratch, draw(p, true), MaskStep::Resolve]);
                }
            } else {
                // 和集合: 作業領域に Π(1 − c) を作ってチャンネルに 1 − Π(1 − c) を掛ける。
                steps.push(MaskStep::FillScratch);
                steps.extend(pos.iter().map(|&p| draw(p, true)));
                steps.push(MaskStep::Resolve);
            }
            // 反転マスク: チャンネルに直接 1 − c を掛ける。
            steps.extend(neg.iter().map(|&n| draw(n, false)));
            (pos.iter().chain(neg).copied().collect::<Vec<_>>(), steps)
        }
    };
    members.sort_unstable();
    members.dedup();
    let level = members
        .iter()
        .map(|&m| compiled.mask_depth(m as usize))
        .max()
        .unwrap_or(0);
    MaskContent {
        members,
        steps,
        level,
    }
}

/// 1 つの内容に割り当てたアトラス上の場所。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaskSlot {
    /// 何枚目のアトラスか。
    pub atlas: u32,
    /// 0..3 (R, G, B)。
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

/// 割り当てを求める内容 1 つ分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellRequest {
    pub level: u32,
    /// 縁を含むセルの大きさ (texel)。アトラスの一辺の上限以下であること。
    pub width: u32,
    pub height: u32,
}

/// 画面上の外接矩形の大きさ (画素) から、縁を含むセルの大きさを求める。
///
/// 縁を除いた内側が `画素数 / downscale` 以上になるようにし、`max_size` で切り詰める。
pub(crate) fn cell_size(pixels: [f32; 2], downscale: u32, max_size: u32) -> [u32; 2] {
    let downscale = downscale.max(1) as f32;
    pixels.map(|p| {
        let inner = (p / downscale).ceil().max(1.0) as u32;
        (inner + CELL_PADDING * 2).min(max_size)
    })
}

/// アトラスの一辺の上限 (設定とデバイスの上限の小さい方)。
pub(crate) fn max_atlas_size(options: MaskAtlasOptions, device_max: u32) -> u32 {
    options.max_size.clamp(1, device_max.max(1))
}

/// セルを詰めるアトラスの一辺を決めて、割り当てる。戻り値は `(一辺, requests と同じ並びの場所)`。
///
/// 一辺は `current` (これより小さくはしない。0 ならこのフレームに要る最小の大きさを求める)・
/// [`MIN_ATLAS_SIZE`]・最大のセルを収める 2 の冪のうち最大のものから始め、レベルの数より多くのアトラスが要るなら
/// `max_size` まで倍にして詰め直す。
pub(crate) fn allocate(
    requests: &[CellRequest],
    current: u32,
    max_size: u32,
) -> (u32, Vec<MaskSlot>) {
    let largest = requests
        .iter()
        .map(|r| r.width.max(r.height))
        .max()
        .unwrap_or(1);
    let mut size = current
        .max(MIN_ATLAS_SIZE)
        .max(largest.next_power_of_two())
        .min(max_size);
    let mut levels: Vec<u32> = requests.iter().map(|r| r.level).collect();
    levels.sort_unstable();
    levels.dedup();
    loop {
        let slots = pack(requests, size);
        let atlases = slots.iter().map(|s| s.atlas + 1).max().unwrap_or(0) as usize;
        if atlases <= levels.len() || size >= max_size {
            return (size, slots);
        }
        size = size.saturating_mul(2).min(max_size);
    }
}

/// 一辺 `size` のアトラスにセルを詰める。アトラスはレベルごとに分け、レベルの小さい順に並べる。
fn pack(requests: &[CellRequest], size: u32) -> Vec<MaskSlot> {
    let mut order: Vec<usize> = (0..requests.len()).collect();
    // 同じレベルの中では高いセルから詰める (棚の無駄が減る)。
    order.sort_by_key(|&i| (requests[i].level, std::cmp::Reverse(requests[i].height), i));
    let mut atlases: Vec<(u32, [Shelves; CONTENT_CHANNELS as usize])> = Vec::new();
    let mut slots = vec![
        MaskSlot {
            atlas: 0,
            channel: 0,
            cell: [0; 4],
        };
        requests.len()
    ];
    for i in order {
        let r = requests[i];
        let (w, h) = (r.width.min(size), r.height.min(size));
        let found = atlases
            .iter_mut()
            .enumerate()
            .filter(|(_, (level, _))| *level == r.level)
            .find_map(|(a, (_, planes))| {
                planes.iter_mut().enumerate().find_map(|(c, shelves)| {
                    shelves
                        .alloc(w, h, size)
                        .map(|[x, y]| (a as u32, c as u32, x, y))
                })
            });
        let (atlas, channel, x, y) = match found {
            Some(found) => found,
            None => {
                let mut planes: [Shelves; CONTENT_CHANNELS as usize] = Default::default();
                // 空のアトラスには必ず入る (w, h <= size)。
                let [x, y] = planes[0].alloc(w, h, size).unwrap_or([0, 0]);
                atlases.push((r.level, planes));
                (atlases.len() as u32 - 1, 0, x, y)
            }
        };
        slots[i] = MaskSlot {
            atlas,
            channel,
            cell: [x, y, w, h],
        };
    }
    slots
}

/// アトラスの一辺と枚数を、フレームをまたいで決める。
///
/// 大きくするのはすぐ行い、小さくするのは条件が `shrink_after_frames` フレーム続いてから行う
/// (ヒステリシス)。一辺は必要な大きさが今の半分以下のとき、枚数は今より少なくて済むときに縮める。
/// 縮めた後の値は、待っていた間に必要だった値の最大値。
#[derive(Debug, Default)]
pub(crate) struct AtlasPool {
    /// 今のアトラスの一辺 (0 はアトラスを持たない)。
    size: u32,
    /// 今持っているアトラスの枚数。
    count: u32,
    shrink_size: ShrinkTimer,
    shrink_count: ShrinkTimer,
}

impl AtlasPool {
    pub fn size(&self) -> u32 {
        self.size
    }

    /// 持っているアトラスの合計のバイト数 (RGBA8)。
    pub fn bytes(&self) -> u64 {
        u64::from(self.size) * u64::from(self.size) * 4 * u64::from(self.count)
    }

    /// このフレームでセルを詰めるアトラスの一辺を決める。
    ///
    /// `needed` はこのフレームだけで要る一辺 (`allocate(requests, 0, ..)` の結果、マスクがなければ 0)。
    pub fn frame_size(&mut self, needed: u32, shrink_after_frames: u32) -> u32 {
        if needed > self.size {
            self.shrink_size = ShrinkTimer::default();
            return needed;
        }
        let shrinkable = needed <= self.size / 2;
        self.shrink_size
            .step(shrinkable, needed, shrink_after_frames)
            .unwrap_or(self.size)
    }

    /// 詰めた結果 (一辺 `size`、使ったアトラス `used` 枚) を記録し、持っておく枚数を返す。
    ///
    /// 一辺が変わったらアトラスはすべて作り直すので、使った枚数だけを持つ。
    pub fn commit(&mut self, size: u32, used: u32, shrink_after_frames: u32) -> u32 {
        if size != self.size {
            self.size = size;
            self.count = 0;
            self.shrink_count = ShrinkTimer::default();
        }
        if used >= self.count {
            self.count = used;
            self.shrink_count = ShrinkTimer::default();
        } else if let Some(count) = self.shrink_count.step(true, used, shrink_after_frames) {
            self.count = count;
        }
        if self.count == 0 {
            // 1 枚も持たないなら一辺も 0 にする (次に要るときは必要な大きさから作る)。
            self.size = 0;
        }
        self.count
    }
}

/// 縮めてよいフレームが続いた数を数える。
#[derive(Debug, Default)]
struct ShrinkTimer {
    frames: u32,
    /// 数えている間に必要だった値の最大値。
    peak: u32,
}

impl ShrinkTimer {
    /// 縮めてよいフレームが `after` フレーム続いたら、縮めた後の値を返す。
    fn step(&mut self, shrinkable: bool, needed: u32, after: u32) -> Option<u32> {
        if !shrinkable || after == 0 {
            *self = Self::default();
            return None;
        }
        self.frames += 1;
        self.peak = self.peak.max(needed);
        if self.frames < after {
            return None;
        }
        let peak = self.peak;
        *self = Self::default();
        Some(peak)
    }
}

/// 1 チャンネル分の棚詰め。
#[derive(Debug, Default)]
struct Shelves {
    /// 棚ごとの `[y, 高さ, 使った幅]`。
    shelves: Vec<[u32; 3]>,
    /// 次の棚を置く y。
    top: u32,
}

impl Shelves {
    fn alloc(&mut self, w: u32, h: u32, size: u32) -> Option<[u32; 2]> {
        for shelf in &mut self.shelves {
            let [y, height, used] = *shelf;
            if h <= height && used + w <= size {
                shelf[2] += w;
                return Some([used, y]);
            }
        }
        if w <= size && self.top + h <= size {
            let y = self.top;
            self.shelves.push([y, h, w]);
            self.top += h;
            return Some([0, y]);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe_model::{MaskRef, fixtures};

    fn compile(model: ugoe_model::Model) -> std::sync::Arc<CompiledModel> {
        CompiledModel::compile(model).expect("compile")
    }

    fn mesh_index(compiled: &CompiledModel, id: &str) -> u32 {
        compiled
            .model()
            .meshes
            .iter()
            .position(|m| m.id.as_str() == id)
            .expect("mesh") as u32
    }

    #[test]
    fn union_and_inverted_share_product_content() {
        let mut model = fixtures::mixed_masks();
        // Face を通常マスクにする要素と、反転マスクにする要素を足す。
        let brow = model.meshes[2].clone();
        let mut a = brow.clone();
        a.id = "A".into();
        a.masks = vec![MaskRef::normal("Face")];
        let mut b = brow;
        b.id = "B".into();
        b.masks = vec![MaskRef::inverted("Face"), MaskRef::inverted("Face")];
        model.meshes.extend([a, b]);
        let compiled = compile(model);
        let (per_mesh, contents) = build(&compiled);
        let face = mesh_index(&compiled, "Face");
        let (a, b) = (per_mesh[3].expect("A"), per_mesh[4].expect("B"));
        assert_eq!(a.content, b.content);
        assert!(a.complement && !b.complement);
        assert_eq!(
            contents[a.content as usize].steps,
            vec![MaskStep::Draw {
                mesh: face,
                scratch: false
            }]
        );
        // Brow は通常と反転の併用なので別の内容。
        let brow = per_mesh[2].expect("Brow");
        assert_ne!(brow.content, a.content);
        assert!(!brow.complement);
        let hair = mesh_index(&compiled, "Hair");
        assert_eq!(
            contents[brow.content as usize].steps,
            vec![
                MaskStep::FillScratch,
                MaskStep::Draw {
                    mesh: face,
                    scratch: true
                },
                MaskStep::Resolve,
                MaskStep::Draw {
                    mesh: hair,
                    scratch: false
                },
            ]
        );
        assert!(per_mesh[0].is_none() && per_mesh[1].is_none());
    }

    #[test]
    fn intersection_resolves_each_mask() {
        let compiled = compile(fixtures::intersection_masks());
        let (per_mesh, contents) = build(&compiled);
        let tongue = per_mesh[2].expect("Tongue");
        assert!(!tongue.complement);
        let steps = &contents[tongue.content as usize].steps;
        assert_eq!(steps.len(), 6);
        assert_eq!(contents[tongue.content as usize].draw_count(), 2);
        assert!(
            steps
                .iter()
                .all(|s| !matches!(s, MaskStep::Draw { scratch: false, .. }))
        );
    }

    #[test]
    fn single_mask_intersection_is_a_union() {
        let mut model = fixtures::intersection_masks();
        model.meshes[2].masks.truncate(1);
        let compiled = compile(model);
        let (per_mesh, contents) = build(&compiled);
        let tongue = per_mesh[2].expect("Tongue");
        assert!(tongue.complement);
        assert_eq!(contents[tongue.content as usize].steps.len(), 1);
    }

    #[test]
    fn unknown_mask_mode_is_a_union() {
        let mut union = fixtures::intersection_masks();
        union.meshes[2].mask_mode = MaskMode::Union;
        let mut unknown = union.clone();
        unknown.meshes[2].mask_mode = MaskMode::Unknown;
        let (union, unknown) = (compile(union), compile(unknown));
        let (expected, expected_contents) = build(&union);
        let (actual, actual_contents) = build(&unknown);
        let (expected, actual) = (expected[2].expect("Tongue"), actual[2].expect("Tongue"));
        assert_eq!(actual.complement, expected.complement);
        assert_eq!(
            actual_contents[actual.content as usize].steps,
            expected_contents[expected.content as usize].steps
        );
    }

    #[test]
    fn nested_levels_follow_mask_depth() {
        let compiled = compile(fixtures::nested_masks());
        let (per_mesh, contents) = build(&compiled);
        // Pupil (Iris で切り抜く) はレベル 1、Iris (Sclera で切り抜く) はレベル 0。
        let pupil = per_mesh[0].expect("Pupil");
        let iris = per_mesh[1].expect("Iris");
        assert_eq!(contents[pupil.content as usize].level, 1);
        assert_eq!(contents[iris.content as usize].level, 0);
        assert!(per_mesh[2].is_none());
    }

    #[test]
    fn cell_covers_on_screen_size() {
        assert_eq!(cell_size([100.0, 30.2], 1, 4096), [102, 33]);
        assert_eq!(cell_size([100.0, 30.2], 4, 4096), [27, 10]);
        assert_eq!(cell_size([5000.0, 0.0], 0, 2048), [2048, 3]);
        assert_eq!(max_atlas_size(MaskAtlasOptions::default(), 2048), 2048);
    }

    fn req(level: u32, width: u32, height: u32) -> CellRequest {
        CellRequest {
            level,
            width,
            height,
        }
    }

    fn overlaps(a: &MaskSlot, b: &MaskSlot) -> bool {
        let [ax, ay, aw, ah] = a.cell;
        let [bx, by, bw, bh] = b.cell;
        a.atlas == b.atlas
            && a.channel == b.channel
            && ax < bx + bw
            && bx < ax + aw
            && ay < by + bh
            && by < ay + ah
    }

    #[test]
    fn packs_cells_without_overlap() {
        let requests: Vec<_> = (0..40).map(|i| req(0, 30 + i * 7, 20 + i * 5)).collect();
        let (size, slots) = allocate(&requests, 0, 4096);
        // 1 枚に収まるまで大きくする。
        assert!(size.is_power_of_two() && size >= 512);
        assert!(slots.iter().all(|s| s.atlas == 0));
        for (i, a) in slots.iter().enumerate() {
            let r = requests[i];
            assert_eq!(a.cell[2..], [r.width, r.height]);
            assert!(a.cell[0] + a.cell[2] <= size && a.cell[1] + a.cell[3] <= size);
            for b in &slots[i + 1..] {
                assert!(!overlaps(a, b), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn atlas_grows_to_largest_cell_and_keeps_size() {
        let (size, _) = allocate(&[req(0, 600, 10)], 0, 4096);
        assert_eq!(size, 1024);
        // 前のフレームより小さくはしない。
        let (size, _) = allocate(&[req(0, 10, 10)], 1024, 4096);
        assert_eq!(size, 1024);
    }

    #[test]
    fn overflow_allocates_more_atlases_at_max_size() {
        // 上限 16 のアトラスには、10x16 のセルが 1 チャンネルに 1 つしか入らない。
        let requests = vec![req(0, 10, 16); 5];
        let (size, slots) = allocate(&requests, 0, 16);
        assert_eq!(size, 16);
        assert_eq!(slots.iter().filter(|s| s.atlas == 0).count(), 3);
        assert_eq!(slots.iter().filter(|s| s.atlas == 1).count(), 2);
    }

    #[test]
    fn levels_use_separate_atlases_in_order() {
        let requests = vec![
            req(1, 10, 10),
            req(0, 10, 10),
            req(2, 10, 10),
            req(0, 10, 10),
        ];
        let (_, slots) = allocate(&requests, 0, 4096);
        let atlas: Vec<u32> = slots.iter().map(|s| s.atlas).collect();
        assert_eq!(atlas, vec![1, 0, 2, 0]);
    }

    /// `needed` (一辺, 使う枚数) の列を 1 フレームずつ与え、各フレームの (一辺, 枚数) を返す。
    fn run(pool: &mut AtlasPool, frames: &[(u32, u32)], after: u32) -> Vec<(u32, u32)> {
        frames
            .iter()
            .map(|&(needed, used)| {
                let size = pool.frame_size(needed, after);
                let count = pool.commit(size, used, after);
                (size, count)
            })
            .collect()
    }

    #[test]
    fn pool_grows_immediately_and_shrinks_after_delay() {
        let mut pool = AtlasPool::default();
        assert_eq!(run(&mut pool, &[(2048, 1)], 3), vec![(2048, 1)]);
        assert_eq!(pool.bytes(), 2048 * 2048 * 4);
        // 必要な一辺が半分以下のフレームが 3 つ続いたら、その間の最大 (512) まで縮める。
        let out = run(&mut pool, &[(256, 1), (512, 1), (256, 1), (256, 1)], 3);
        assert_eq!(out, vec![(2048, 1), (2048, 1), (512, 1), (512, 1)]);
        assert_eq!(pool.bytes(), 512 * 512 * 4);
    }

    #[test]
    fn pool_does_not_thrash_within_hysteresis_band() {
        let mut pool = AtlasPool::default();
        run(&mut pool, &[(1024, 1)], 4);
        // 半分を超える大きさが混じるとタイマーが戻るので、何フレーム続いても作り直さない。
        let frames: Vec<_> = (0..100)
            .map(|i| if i % 2 == 0 { (512, 1) } else { (1024, 1) })
            .collect();
        assert!(run(&mut pool, &frames, 4).iter().all(|&s| s == (1024, 1)));
        // 大きさが揺れても、一度縮めたら揺れの範囲では作り直さない。
        let frames: Vec<_> = (0..100)
            .map(|i| if i % 2 == 0 { (256, 1) } else { (512, 1) })
            .collect();
        let out = run(&mut pool, &frames, 4);
        assert_eq!(out[3], (512, 1));
        assert!(out[3..].iter().all(|&s| s == (512, 1)));
    }

    #[test]
    fn pool_releases_unused_atlases() {
        let mut pool = AtlasPool::default();
        run(&mut pool, &[(4096, 3)], 2);
        // 溢れた分のアトラスは、使わないフレームが続いたら解放する。
        assert_eq!(
            run(&mut pool, &[(4096, 1), (4096, 2)], 2),
            vec![(4096, 3), (4096, 2)]
        );
        // マスクを使わないフレームが続いたらすべて解放する。
        assert_eq!(
            run(&mut pool, &[(0, 0), (0, 0)], 2),
            vec![(4096, 2), (0, 0)]
        );
        assert_eq!(pool.bytes(), 0);
        // 次に要るときは必要な大きさで作る。
        assert_eq!(run(&mut pool, &[(256, 1)], 2), vec![(256, 1)]);
    }

    #[test]
    fn pool_never_shrinks_when_disabled() {
        let mut pool = AtlasPool::default();
        run(&mut pool, &[(4096, 2)], 0);
        let frames = vec![(0, 0); 1000];
        assert!(run(&mut pool, &frames, 0).iter().all(|&s| s == (4096, 2)));
        assert_eq!(MaskAtlasOptions::default().shrink_after_frames, 120);
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
