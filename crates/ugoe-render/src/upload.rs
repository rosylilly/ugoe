//! GPU バッファへの転送の省略 (`docs/spec/rendering.md` の「転送の省略」)。
//!
//! バッファに転送済みの内容を CPU 側に控えておき、変わった範囲だけを転送する。
//! 頂点位置は、控えを書いたときに見たインスタンスの `update` のちょうど次の出力なら
//! ランタイムの変更フラグ (`docs/spec/evaluation.md#変更フラグ`) を使い、そうでなければ控えと比べる。

use ugoe_core::Vec2;
use ugoe_runtime::{DrawableChanges, ModelInstance};

/// 頂点位置 1 つのバイト数。
const VERTEX_BYTES: u64 = std::mem::size_of::<[f32; 2]>() as u64;

/// 頂点位置バッファに転送済みの内容の控え。
///
/// バッファは描画可能要素の頂点位置を宣言順に詰めたもの (`Geometry`) で、控えも同じ並び。
#[derive(Debug, Default)]
pub(crate) struct PositionUploads {
    shadow: Vec<[f32; 2]>,
    /// 控えを最後に書いたインスタンスの `(id, update_count)`。`None` ならまだ何も転送していない。
    source: Option<(u64, u64)>,
    /// 直前の `upload` で転送したかどうか (要素ごと)。
    written: Vec<bool>,
}

/// 要素ごとに転送が要るかを決める方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// 控えが当てにならない (最初の転送)。すべて転送する。
    All,
    /// 控えを書いたときと同じ `update` の出力。何も転送しない。
    Unchanged,
    /// 控えを書いたときの次の `update` の出力。変更フラグの `POSITIONS` を使う。
    Flags,
    /// 別のインスタンス、または `update` を 2 回以上挟んだ。控えとビット列で比べる。
    Compare,
}

impl PositionUploads {
    /// バッファを作り直したときに呼ぶ。次の `upload` はすべてを転送する。
    pub fn reset(&mut self, vertex_count: usize) {
        self.shadow.clear();
        self.shadow.resize(vertex_count, [0.0; 2]);
        self.source = None;
        self.written.clear();
    }

    /// 直前の `upload` で要素 `i` の頂点位置を転送したか。
    pub fn written(&self, i: usize) -> bool {
        self.written.get(i).copied().unwrap_or(false)
    }

    /// `instance` の頂点位置のうち、バッファと違う要素だけを `write(バイトオフセット, データ)` で転送する。
    /// 隣り合う要素はまとめて 1 回で書く。転送したバイト数を返す。
    ///
    /// `instance` は `reset` に渡した頂点数のモデルのインスタンスであること。
    pub fn upload(&mut self, instance: &ModelInstance, mut write: impl FnMut(u64, &[u8])) -> u64 {
        let stamp = (instance.id(), instance.update_count());
        let mode = match self.source {
            None => Mode::All,
            Some(source) if source == stamp => Mode::Unchanged,
            Some((id, count)) if id == stamp.0 && count.checked_add(1) == Some(stamp.1) => {
                Mode::Flags
            }
            Some(_) => Mode::Compare,
        };
        self.source = Some(stamp);
        self.written.clear();

        let mut bytes = 0;
        // 転送を待っている連続した範囲 (頂点単位)。
        let mut run: Option<(usize, usize)> = None;
        let mut flush = |shadow: &[[f32; 2]], (start, end): (usize, usize)| {
            if end > start {
                write(
                    start as u64 * VERTEX_BYTES,
                    bytemuck::cast_slice(&shadow[start..end]),
                );
                bytes += (end - start) as u64 * VERTEX_BYTES;
            }
        };
        let mut start = 0;
        for (state, changes) in instance.drawables().iter().zip(instance.changes()) {
            let end = start + state.positions.len();
            let shadow = &mut self.shadow[start..end];
            let dirty = match mode {
                Mode::All => true,
                Mode::Unchanged => false,
                Mode::Flags => changes.contains(DrawableChanges::POSITIONS),
                Mode::Compare => !same_bits(shadow, &state.positions),
            };
            if dirty {
                for (s, p) in shadow.iter_mut().zip(&state.positions) {
                    *s = [p.x, p.y];
                }
                run = match run {
                    Some((s, e)) if e == start => Some((s, end)),
                    Some(r) => {
                        flush(&self.shadow, r);
                        Some((start, end))
                    }
                    None => Some((start, end)),
                };
            }
            self.written.push(dirty);
            start = end;
        }
        if let Some(r) = run {
            flush(&self.shadow, r);
        }
        bytes
    }
}

/// ランタイムの変更フラグと同じく、ビット列の一致で比べる。
fn same_bits(shadow: &[[f32; 2]], positions: &[Vec2]) -> bool {
    shadow.len() == positions.len()
        && shadow
            .iter()
            .zip(positions)
            .all(|(s, p)| s[0].to_bits() == p.x.to_bits() && s[1].to_bits() == p.y.to_bits())
}

/// 固定長のスロットを並べたバッファ (ユニフォーム) に転送済みの内容の控え。
#[derive(Debug, Default)]
pub(crate) struct SlotUploads {
    /// 空ならまだ何も転送していない。
    shadow: Vec<u8>,
}

/// [`SlotUploads::upload`] の結果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SlotWrites {
    pub bytes: u64,
    /// 転送したスロットの番号の最大値。
    pub last_slot: Option<usize>,
}

impl SlotUploads {
    /// バッファを作り直したときに呼ぶ。次の `upload` はすべてを転送する。
    pub fn reset(&mut self) {
        self.shadow.clear();
    }

    /// `data` (`stride` バイトのスロットの列) のうち、バッファと違うスロットだけを転送する。
    /// 隣り合うスロットはまとめて 1 回で書く。
    pub fn upload(
        &mut self,
        data: &[u8],
        stride: usize,
        mut write: impl FnMut(u64, &[u8]),
    ) -> SlotWrites {
        let mut result = SlotWrites::default();
        if data.is_empty() || stride == 0 {
            return result;
        }
        if self.shadow.len() != data.len() {
            self.shadow.clear();
            self.shadow.extend_from_slice(data);
            write(0, data);
            result.bytes = data.len() as u64;
            result.last_slot = Some(data.len().div_ceil(stride) - 1);
            return result;
        }
        // 転送した範囲は控えと `data` で同じなので、`data` から書く。
        let mut flush = |(start, end): (usize, usize)| {
            let range = start * stride..(end * stride).min(data.len());
            write(range.start as u64, &data[range.clone()]);
            result.bytes += range.len() as u64;
            result.last_slot = Some(end - 1);
        };
        let mut run: Option<(usize, usize)> = None;
        for (slot, (shadow, new)) in self
            .shadow
            .chunks_mut(stride)
            .zip(data.chunks(stride))
            .enumerate()
        {
            if shadow == new {
                continue;
            }
            shadow.copy_from_slice(new);
            run = match run {
                Some((s, e)) if e == slot => Some((s, slot + 1)),
                Some(r) => {
                    flush(r);
                    Some((slot, slot + 1))
                }
                None => Some((slot, slot + 1)),
            };
        }
        if let Some(r) = run {
            flush(r);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;
    use ugoe_runtime::CompiledModel;

    use super::*;

    fn instance() -> ModelInstance {
        ModelInstance::new(CompiledModel::compile(fixtures::single_quad()).unwrap())
    }

    fn vertex_count(instance: &ModelInstance) -> usize {
        instance.drawables().iter().map(|d| d.positions.len()).sum()
    }

    /// 書き込みを記録して、転送したバイト数を返す。
    fn upload(uploads: &mut PositionUploads, instance: &ModelInstance) -> u64 {
        let mut writes = Vec::new();
        let bytes = uploads.upload(instance, |offset, data| writes.push((offset, data.len())));
        assert_eq!(writes.iter().map(|w| w.1 as u64).sum::<u64>(), bytes);
        bytes
    }

    #[test]
    fn positions_are_uploaded_once_until_they_change() {
        let mut a = instance();
        let full = vertex_count(&a) as u64 * VERTEX_BYTES;
        let mut uploads = PositionUploads::default();
        uploads.reset(vertex_count(&a));
        assert_eq!(upload(&mut uploads, &a), full, "first upload");
        assert_eq!(upload(&mut uploads, &a), 0, "same update");
        a.update();
        // 最初にホストが呼ぶ update はすべてを変更ありと報告する (フラグを信じる)。
        assert_eq!(upload(&mut uploads, &a), full);
        a.update();
        assert_eq!(upload(&mut uploads, &a), 0, "flags say nothing moved");
        a.update();
        a.update();
        assert_eq!(upload(&mut uploads, &a), 0, "compared: nothing moved");
        a.mark_all_changed();
        a.update();
        assert_eq!(upload(&mut uploads, &a), full, "mark_all_changed");
    }

    #[test]
    fn another_instance_is_compared_not_trusted() {
        let compiled = CompiledModel::compile(fixtures::single_quad()).unwrap();
        let mut a = ModelInstance::new(compiled.clone());
        let mut b = ModelInstance::new(compiled);
        let mut uploads = PositionUploads::default();
        uploads.reset(vertex_count(&a));
        a.update();
        b.update();
        b.update();
        upload(&mut uploads, &a);
        // b の変更フラグは「変化なし」だが、b はまだ転送していないので比べる (出力は同じ)。
        assert_eq!(upload(&mut uploads, &b), 0);
        assert!(!uploads.written(0));
    }

    #[test]
    fn slots_upload_only_changed_runs() {
        let mut uploads = SlotUploads::default();
        let mut data = vec![0u8; 4 * 5];
        let mut writes = Vec::new();
        let r = uploads.upload(&data, 4, |o, d| writes.push((o, d.len())));
        assert_eq!(r.bytes, 20);
        assert_eq!(r.last_slot, Some(4));
        writes.clear();
        let r = uploads.upload(&data, 4, |o, d| writes.push((o, d.len())));
        assert_eq!(r, SlotWrites::default());
        data[4] = 1;
        data[2 * 4 + 3] = 1;
        data[4 * 4] = 1;
        let r = uploads.upload(&data, 4, |o, d| writes.push((o, d.len())));
        assert_eq!(writes, vec![(4, 8), (16, 4)]);
        assert_eq!(r.bytes, 12);
        assert_eq!(r.last_slot, Some(4));
    }
}
