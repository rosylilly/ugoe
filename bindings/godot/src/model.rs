//! Godot に依存しない部分: バンドルの読み込み、マスクの段取り、フレームごとの幾何の準備。
//!
//! Godot の型を使わないので、エンジンなしの `cargo test` で確かめられる。
//! 描画の式は `docs/spec/rendering.md`。

use std::collections::HashMap;

use ugoe::Puppet;
use ugoe::core::Vec2;
use ugoe::format::{Bundle, FormatError, TextureData};
use ugoe::model::expression::Expression;
use ugoe::model::motion::Motion;
use ugoe::model::{MaskMode, Model};
use ugoe::runtime::{CompileError, CompiledModel};

/// 1 つの描画可能要素が読めるマスクの数の上限。
///
/// シェーダのサンプラを要素ごとに固定の数だけ用意するため。仕様 (`docs/spec/rendering.md#精度`) では
/// 上限がないが、このバインディングでは超えた分を無視する (読み込み時に警告を出す)。
pub const MAX_MASKS: usize = 8;

/// 読み込みの失敗。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read the bundle: {0}")]
    Format(#[from] FormatError),
    #[error("failed to compile the model: {0}")]
    Compile(#[from] CompileError),
}

/// 要素が読むマスク 1 つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskSlot {
    /// [`MaskPlan::sources`] の番号 (マスクのテクスチャの番号)。
    pub source: usize,
    pub invert: bool,
}

/// 描画可能要素の、評価で変わらない情報。
#[derive(Debug, Clone)]
pub struct DrawableInfo {
    pub id: String,
    /// テクスチャの番号 (`LoadedModel::textures`)。
    pub texture: usize,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u16>,
    pub culling: bool,
    /// 読むマスク (同じメッシュ・同じ `invert` の重複は除く。最大 [`MAX_MASKS`])。
    pub masks: Vec<MaskSlot>,
    pub mask_mode: MaskMode,
}

/// マスクの段取り: どのメッシュのカバレッジをテクスチャに描くか。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaskPlan {
    /// マスクとして参照されるメッシュの番号。入れ子のマスクが先に来る順 (依存するものが先)。
    pub sources: Vec<usize>,
    /// 上限 ([`MAX_MASKS`]) を超えて無視したマスクの参照がある要素の ID。
    pub truncated: Vec<String>,
}

/// 読み込んだモデル 1 体分。
#[derive(Debug)]
pub struct LoadedModel {
    pub puppet: Puppet,
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
    /// テクスチャの番号ごとの画像。バンドルに無い番号は `None` (白で代用する)。
    pub textures: Vec<Option<TextureData>>,
    pub drawables: Vec<DrawableInfo>,
    pub masks: MaskPlan,
}

impl LoadedModel {
    /// `.ugoe` バンドルのバイト列から読み込む。
    pub fn from_bundle_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        Self::from_bundle(ugoe::format::read_bundle(bytes)?)
    }

    /// 読み込んだバンドルから作る。物理・モーション・表情・テクスチャも使う。
    pub fn from_bundle(bundle: Bundle) -> Result<Self, LoadError> {
        let count = texture_count(&bundle.model);
        let Bundle {
            model,
            physics,
            motions,
            expressions,
            textures,
            ..
        } = bundle;
        let compiled = CompiledModel::compile(model)?;
        let mut puppet = Puppet::from_compiled(compiled);
        if let Some(physics) = physics {
            // `read_bundle` は物理の設定を検査済みで返す。
            puppet = puppet.with_physics(physics);
        }
        let mut textures: Vec<Option<TextureData>> = textures.into_iter().map(Some).collect();
        textures.resize(count, None);
        let (drawables, masks) = drawables(puppet.instance().compiled().model());
        Ok(Self {
            puppet,
            motions,
            expressions,
            textures,
            drawables,
            masks,
        })
    }

    pub fn model(&self) -> &Model {
        self.puppet.instance().compiled().model()
    }
}

/// 描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
fn texture_count(model: &Model) -> usize {
    model
        .meshes
        .iter()
        .map(|m| m.texture as usize + 1)
        .max()
        .unwrap_or(0)
        .max(model.textures.len())
}

/// メッシュごとの描画の情報と、マスクの段取りを作る。
///
/// 検証済みのモデル (マスクの参照が循環しない) を前提にする。見つからない参照は無視する。
pub fn drawables(model: &Model) -> (Vec<DrawableInfo>, MaskPlan) {
    let index: HashMap<&str, usize> = model
        .meshes
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    // 各メッシュの参照 (メッシュの番号, invert)。重複を除き、上限で切る。
    let mut truncated = Vec::new();
    let refs: Vec<Vec<(usize, bool)>> = model
        .meshes
        .iter()
        .map(|mesh| {
            let mut out: Vec<(usize, bool)> = Vec::new();
            for r in &mesh.masks {
                let Some(&i) = index.get(r.mesh.as_str()) else {
                    continue;
                };
                if !out.contains(&(i, r.invert)) {
                    out.push((i, r.invert));
                }
            }
            if out.len() > MAX_MASKS {
                truncated.push(mesh.id.as_str().to_owned());
                out.truncate(MAX_MASKS);
            }
            out
        })
        .collect();

    // マスクとして使うメッシュを、入れ子のマスクが先に来る順に並べる (深さ優先の帰りがけ順)。
    let mut order = Vec::new();
    let mut seen = vec![false; model.meshes.len()];
    fn visit(i: usize, refs: &[Vec<(usize, bool)>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[i] {
            return;
        }
        seen[i] = true;
        for &(j, _) in &refs[i] {
            visit(j, refs, seen, order);
        }
        order.push(i);
    }
    for r in &refs {
        for &(j, _) in r {
            visit(j, &refs, &mut seen, &mut order);
        }
    }
    let source_of: HashMap<usize, usize> = order.iter().enumerate().map(|(s, &i)| (i, s)).collect();

    let drawables = model
        .meshes
        .iter()
        .zip(&refs)
        .map(|(mesh, refs)| DrawableInfo {
            id: mesh.id.as_str().to_owned(),
            texture: mesh.texture as usize,
            uvs: mesh.uvs.clone(),
            indices: mesh.indices.clone(),
            culling: mesh.culling,
            masks: refs
                .iter()
                .map(|&(i, invert)| MaskSlot {
                    source: source_of[&i],
                    invert,
                })
                .collect(),
            mask_mode: mesh.mask_mode.effective(),
        })
        .collect();
    (
        drawables,
        MaskPlan {
            sources: order,
            truncated,
        },
    )
}

/// 表向きの三角形だけの頂点番号 (`docs/spec/rendering.md#裏面カリング`)。
///
/// キャンバス座標 (Y 下向き) で符号付き面積が負の三角形を表とする。ビューには左右されない。
pub fn front_facing(positions: &[Vec2], indices: &[u16], out: &mut Vec<i32>) {
    out.clear();
    for tri in indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| positions[i as usize]);
        let cross = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if cross < 0.0 {
            out.extend(tri.iter().map(|&i| i32::from(i)));
        }
    }
}

/// 点の集合の外接矩形 `(min, max)`。空なら `None`。
pub fn bounds(positions: &[Vec2]) -> Option<(Vec2, Vec2)> {
    let first = *positions.first()?;
    Some(
        positions
            .iter()
            .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p))),
    )
}

/// マスクのテクスチャ 1 枚の大きさと、それが覆うキャンバス上の矩形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskArea {
    /// テクスチャの画素数。
    pub width: u32,
    pub height: u32,
    /// テクスチャの左上に対応するキャンバス座標。
    pub origin: Vec2,
    /// キャンバスの 1 単位あたりの画素数。
    pub scale: f32,
}

impl MaskArea {
    /// 1 辺の画素数の上限。
    pub const MAX_SIZE: u32 = 4096;
    /// 縁の余白 (画素)。クランプした読み出しで縁の値が伸びないように、外側を必ず 0 にする。
    const PADDING: u32 = 2;

    /// 外接矩形 `(min, max)` を `scale` (キャンバスの 1 単位あたりの画素数) で覆う。
    ///
    /// `current` が十分に大きく、大きすぎない (面積の 4 倍以下) ならその大きさを使い回す
    /// (テクスチャを毎フレーム作り直さないため)。大きさが上限を超えるなら `scale` を下げる。
    pub fn fit(min: Vec2, max: Vec2, scale: f32, current: Option<(u32, u32)>) -> Self {
        let size = (max - min).max(Vec2::ZERO);
        let pad = Self::PADDING as f32;
        let limit = (Self::MAX_SIZE - 2 * Self::PADDING) as f32;
        let mut scale = scale.max(1e-3);
        let largest = size.x.max(size.y) * scale;
        if largest > limit {
            scale *= limit / largest;
        }
        let need = |len: f32| ((len * scale).ceil() as u32 + 2 * Self::PADDING).min(Self::MAX_SIZE);
        let (need_w, need_h) = (need(size.x), need(size.y));
        let (width, height) = match current {
            Some((w, h))
                if w >= need_w
                    && h >= need_h
                    && u64::from(w) * u64::from(h) <= 4 * u64::from(need_w) * u64::from(need_h) =>
            {
                (w, h)
            }
            _ => (need_w, need_h),
        };
        Self {
            width,
            height,
            origin: min - Vec2::splat(pad / scale),
            scale,
        }
    }

    /// シェーダに渡す `(origin.x, origin.y, 幅, 高さ)` (キャンバス座標)。
    pub fn rect(&self) -> [f32; 4] {
        [
            self.origin.x,
            self.origin.y,
            self.width as f32 / self.scale,
            self.height as f32 / self.scale,
        ]
    }
}

/// 乗算していないアルファの RGBA8 を乗算済みにする (`docs/spec/rendering.md#前提`)。
///
/// 色のチャンネルごとに `c' = (c × a + 127) / 255` (整数の割り算、切り捨て)。
pub fn premultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe::model::{MaskRef, MeshId, fixtures};

    /// 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
    const FAKE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    fn bundle_bytes(model: Model) -> Vec<u8> {
        let mut bundle = Bundle::new(model.clone());
        bundle.textures = model
            .textures
            .iter()
            .map(|_| TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: FAKE_PNG.to_vec(),
            })
            .collect();
        ugoe::format::write_bundle(&bundle).expect("write")
    }

    #[test]
    fn loads_a_bundle_and_evaluates() {
        let mut loaded =
            LoadedModel::from_bundle_bytes(&bundle_bytes(fixtures::single_quad())).expect("load");
        assert_eq!(loaded.drawables.len(), 1);
        assert_eq!(loaded.textures.len(), loaded.model().textures.len().max(1));
        loaded.puppet.update(1.0 / 60.0);
        let state = &loaded.puppet.instance().drawables()[0];
        assert_eq!(state.positions.len(), loaded.drawables[0].uvs.len());
    }

    #[test]
    fn rejects_broken_bytes() {
        assert!(matches!(
            LoadedModel::from_bundle_bytes(b"not a bundle"),
            Err(LoadError::Format(_))
        ));
    }

    #[test]
    fn nested_masks_come_before_their_users() {
        let model = fixtures::nested_masks();
        let (drawables, plan) = drawables(&model);
        let pos = |id: &str| {
            model
                .meshes
                .iter()
                .position(|m| m.id.as_str() == id)
                .expect(id)
        };
        // Pupil -> Iris -> Sclera。Sclera のカバレッジを先に描く。
        assert_eq!(plan.sources, vec![pos("Sclera"), pos("Iris")]);
        let pupil = &drawables[pos("Pupil")];
        assert_eq!(
            pupil.masks,
            vec![MaskSlot {
                source: 1,
                invert: false
            }]
        );
        let iris = &drawables[pos("Iris")];
        assert_eq!(
            iris.masks,
            vec![MaskSlot {
                source: 0,
                invert: false
            }]
        );
        assert!(drawables[pos("Sclera")].masks.is_empty());
        assert!(plan.truncated.is_empty());
    }

    #[test]
    fn mixed_masks_keep_invert_and_mode() {
        let model = fixtures::mixed_masks();
        let (drawables, plan) = drawables(&model);
        let brow = drawables.iter().find(|d| d.id == "Brow").expect("Brow");
        assert_eq!(brow.masks.len(), 2);
        let invert: Vec<bool> = brow.masks.iter().map(|m| m.invert).collect();
        assert_eq!(invert, vec![false, true]);
        assert_eq!(plan.sources.len(), 2);

        let model = fixtures::intersection_masks();
        let (drawables, _) = super::drawables(&model);
        assert!(
            drawables
                .iter()
                .any(|d| d.mask_mode == MaskMode::Intersection)
        );
    }

    #[test]
    fn masks_over_the_limit_are_truncated() {
        let mask_ref = |m: &ugoe::model::Mesh| MaskRef {
            mesh: m.id.clone(),
            invert: false,
        };
        let mut model = fixtures::nested_masks();
        // 同じメッシュを同じ invert で 2 回参照しても 1 つと数える。
        let first = model.meshes[0].masks[0].clone();
        model.meshes[0].masks = vec![first.clone(), first];
        let (d, plan) = drawables(&model);
        assert_eq!(d[0].masks.len(), 1);
        assert!(plan.truncated.is_empty());

        // 上限を超える数の別のメッシュを参照させる。
        let template = model.meshes[2].clone();
        for i in 0..MAX_MASKS {
            let mut m = template.clone();
            m.id = MeshId::new(format!("Extra{i}"));
            model.meshes.push(m);
        }
        model.meshes[0].masks = model.meshes[1..].iter().map(mask_ref).collect();
        let (d, plan) = drawables(&model);
        assert_eq!(d[0].masks.len(), MAX_MASKS);
        assert_eq!(plan.truncated, vec![model.meshes[0].id.as_str().to_owned()]);
    }

    #[test]
    fn front_facing_keeps_counter_clockwise_on_screen() {
        // キャンバス (Y 下向き) で画面上の反時計回り = 符号付き面積が負。
        let p = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 10.0),
            Vec2::new(10.0, 0.0),
        ];
        let mut out = Vec::new();
        front_facing(&p, &[0, 1, 2, 0, 2, 1], &mut out);
        assert_eq!(out, vec![0, 1, 2]);
    }

    #[test]
    fn bounds_of_points() {
        assert_eq!(bounds(&[]), None);
        let b = bounds(&[
            Vec2::new(1.0, 5.0),
            Vec2::new(-2.0, 3.0),
            Vec2::new(4.0, 4.0),
        ]);
        assert_eq!(b, Some((Vec2::new(-2.0, 3.0), Vec2::new(4.0, 5.0))));
    }

    #[test]
    fn mask_area_covers_the_bounds_with_padding() {
        let a = MaskArea::fit(Vec2::new(10.0, 20.0), Vec2::new(110.0, 70.0), 2.0, None);
        assert_eq!((a.width, a.height), (204, 104));
        let [x, y, w, h] = a.rect();
        assert!((x - 9.0).abs() < 1e-5 && (y - 19.0).abs() < 1e-5);
        assert!((w - 102.0).abs() < 1e-5 && (h - 52.0).abs() < 1e-5);

        // 十分に大きい今のテクスチャは使い回す。大きすぎるものは作り直す。
        let b = MaskArea::fit(
            Vec2::new(10.0, 20.0),
            Vec2::new(100.0, 60.0),
            2.0,
            Some((204, 104)),
        );
        assert_eq!((b.width, b.height), (204, 104));
        let c = MaskArea::fit(Vec2::ZERO, Vec2::new(10.0, 10.0), 2.0, Some((204, 104)));
        assert_eq!((c.width, c.height), (24, 24));

        // 上限を超えるなら解像度を下げる。
        let d = MaskArea::fit(Vec2::ZERO, Vec2::new(10000.0, 100.0), 1.0, None);
        assert_eq!(d.width, MaskArea::MAX_SIZE);
        assert!(d.scale < 1.0);
    }

    #[test]
    fn premultiply_matches_the_spec() {
        let mut px = [255, 128, 0, 128, 10, 20, 30, 0, 200, 100, 50, 255];
        premultiply(&mut px);
        assert_eq!(px, [128, 64, 0, 128, 0, 0, 0, 0, 200, 100, 50, 255]);
    }
}
