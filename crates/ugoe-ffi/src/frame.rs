//! 毎フレームの描画の状態をまとめて渡す (ADR 0031 の「毎フレームの描画の状態はまとめて渡す」。T0490)。
//!
//! `ugoe_puppet_update` の後に 1 回の呼び出しで、すべての描画可能要素の状態・描画順・頂点の位置を受け取る。
//! 読み込みのときに 1 回読む情報 (UV・インデックス・マスクの参照など) は描画要素ごとの関数 (`drawable.rs`) で読む。

use ugoe::runtime::{DrawableChanges, ModelInstance};

use crate::drawable::{UgoeDrawableState, blend_code};
use crate::error::{Error, UGOE_ERROR_BUFFER_TOO_SMALL, UgoeError, UgoeResult, guard};
use crate::puppet::{UgoePuppet, puppet_mut, write};

/// 描画可能要素の頂点の位置が、連続した配列 (`UgoeFrame::positions`) のどこにあるか。単位は float の数
/// (頂点 1 つが x と y の 2 つ)。読み込みと着せ替えの後は変わらない。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UgoePositionRange {
    /// 先頭 (その描画可能要素の最初の頂点の x の位置)。
    pub first: usize,
    /// float の数 (2 × 頂点の数)。
    pub count: usize,
}

/// 直前の `ugoe_puppet_update` で評価した、すべての描画可能要素の状態 (`ugoe_puppet_frame` が貸す)。
///
/// ポインタはすべて、次の `ugoe_puppet_update`・`ugoe_puppet_follow`・着せ替えの操作・`ugoe_puppet_destroy` まで有効。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeFrame {
    /// 描画可能要素の数 (`states`・`render_order`・`position_ranges` の要素の数)。
    pub drawable_count: u32,
    /// 描画可能要素ごとの状態 (番号順)。
    pub states: *const UgoeDrawableState,
    /// 奥から手前への描画順 (描画可能要素の番号の並び)。
    pub render_order: *const u32,
    /// 前の `ugoe_puppet_update` から描画順の並びが変わったか (作った直後と最初の `update` の後は true)。
    pub render_order_changed: bool,
    /// 描画可能要素ごとの、`positions` の中の頂点の位置の範囲 (番号順)。
    pub position_ranges: *const UgoePositionRange,
    /// すべての描画可能要素の頂点の位置 (キャンバス座標の x と y の組) を番号順につないだもの。
    pub positions: *const f32,
    /// `positions` の float の数。
    pub positions_len: usize,
}

impl Default for UgoeFrame {
    fn default() -> Self {
        Self {
            drawable_count: 0,
            states: std::ptr::null(),
            render_order: std::ptr::null(),
            render_order_changed: false,
            position_ranges: std::ptr::null(),
            positions: std::ptr::null(),
            positions_len: 0,
        }
    }
}

/// `ugoe_puppet_copy_frame` が写す先 (ホストのバッファ)。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeFrameBuffers {
    /// 描画可能要素ごとの状態を書く先 (`drawable_capacity` 個)。
    pub states: *mut UgoeDrawableState,
    /// 描画順を書く先 (`drawable_capacity` 個)。
    pub render_order: *mut u32,
    /// 頂点の位置の範囲を書く先 (`drawable_capacity` 個)。
    pub position_ranges: *mut UgoePositionRange,
    /// `states`・`render_order`・`position_ranges` のそれぞれに書ける要素の数。
    pub drawable_capacity: usize,
    /// 頂点の位置を書く先 (`positions_capacity` 個の float)。
    pub positions: *mut f32,
    /// `positions` に書ける float の数。
    pub positions_capacity: usize,
}

impl Default for UgoeFrameBuffers {
    fn default() -> Self {
        Self {
            states: std::ptr::null_mut(),
            render_order: std::ptr::null_mut(),
            position_ranges: std::ptr::null_mut(),
            drawable_capacity: 0,
            positions: std::ptr::null_mut(),
            positions_capacity: 0,
        }
    }
}

/// `ugoe_puppet_copy_frame` が写したものの大きさと、描画順が変わったか。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeFrameCopy {
    /// 描画可能要素の数 (`drawable_capacity` に要る数)。
    pub drawable_count: u32,
    /// 頂点の位置の float の数 (`positions_capacity` に要る数)。
    pub positions_len: usize,
    /// 前の `ugoe_puppet_update` から描画順の並びが変わったか (`UgoeFrame::render_order_changed`)。
    pub render_order_changed: bool,
}

/// 頂点の位置を 1 つの連続した配列にしたものと、状態の並び。`ModelInstance` の出力から作り、
/// 同じ出力 (インスタンスの番号と `update_count` が同じ) なら作り直さない。
#[derive(Debug, Default)]
pub(crate) struct FrameCache {
    /// 写した出力 (`ModelInstance::id`・`ModelInstance::update_count`)。
    source: Option<(u64, u64)>,
    states: Vec<UgoeDrawableState>,
    ranges: Vec<UgoePositionRange>,
    positions: Vec<f32>,
}

impl FrameCache {
    /// `instance` の今の出力に合わせる。
    ///
    /// 前に写した出力のちょうど次の `update` なら、変更のフラグ (`ModelInstance::changes`) で頂点の位置が
    /// 変わった描画可能要素だけを写す (`docs/spec/evaluation.md#変更フラグ`)。それ以外 (最初・着せ替えで
    /// インスタンスが替わった・`update` を 2 回以上はさんだ) はすべてを写す。
    fn refresh(&mut self, instance: &ModelInstance) {
        let source = (instance.id(), instance.update_count());
        if self.source == Some(source) {
            return;
        }
        let drawables = instance.drawables();
        let changes = instance.changes();
        let next = self
            .source
            .is_some_and(|(id, count)| id == source.0 && count.wrapping_add(1) == source.1);
        if next && self.ranges.len() == drawables.len() {
            for ((d, change), range) in drawables.iter().zip(changes).zip(&self.ranges) {
                if !change.contains(DrawableChanges::POSITIONS) {
                    continue;
                }
                let dst = &mut self.positions[range.first..range.first + range.count];
                for (dst, v) in dst.as_chunks_mut::<2>().0.iter_mut().zip(&d.positions) {
                    *dst = [v.x, v.y];
                }
            }
        } else {
            self.ranges.clear();
            self.positions.clear();
            for d in drawables {
                let first = self.positions.len();
                self.positions
                    .extend(d.positions.iter().flat_map(|v| [v.x, v.y]));
                self.ranges.push(UgoePositionRange {
                    first,
                    count: d.positions.len() * 2,
                });
            }
        }
        self.states.clear();
        self.states.extend(
            drawables
                .iter()
                .zip(changes)
                .map(|(d, change)| UgoeDrawableState {
                    opacity: d.opacity,
                    draw_order: d.draw_order,
                    multiply_color: [d.multiply_color.r, d.multiply_color.g, d.multiply_color.b],
                    screen_color: [d.screen_color.r, d.screen_color.g, d.screen_color.b],
                    blend_mode: blend_code(d.blend),
                    mask_strength: d.mask_strength,
                    visible: d.visible,
                    changes: change.bits(),
                }),
        );
        self.source = Some(source);
    }
}

impl UgoePuppet {
    /// 今の出力に合わせた `FrameCache` と、出力を持つインスタンス。
    fn frame(&mut self) -> (&FrameCache, &ModelInstance) {
        let instance = self.puppet.instance();
        self.frame.refresh(instance);
        (&self.frame, instance)
    }
}

/// 直前の `ugoe_puppet_update` で評価した、すべての描画可能要素の状態・描画順・頂点の位置を、内部のバッファのまま借りる
/// (`*out` に書く)。ポインタの寿命は `UgoeFrame` を参照。
///
/// 頂点の位置は、変更のフラグ (`UgoeDrawableState::changes` の `UGOE_CHANGED_POSITIONS`) が立った描画可能要素だけを
/// ホストの頂点バッファへ写せばよい。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_frame(
    puppet: *mut UgoePuppet,
    out: *mut UgoeFrame,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        if out.is_null() {
            return Err(Error::null("out"));
        }
        let (cache, instance) = p.frame();
        let order = instance.render_order();
        let frame = UgoeFrame {
            drawable_count: cache.states.len() as u32,
            states: cache.states.as_ptr(),
            render_order: order.as_ptr(),
            render_order_changed: instance.render_order_changed(),
            position_ranges: cache.ranges.as_ptr(),
            positions: cache.positions.as_ptr(),
            positions_len: cache.positions.len(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", frame) }
    })
}

/// `ugoe_puppet_frame` と同じものを、ホストのバッファ `*buffers` に写す。大きさと、描画順が変わったかを `*out` に書く。
///
/// `drawable_capacity` が描画可能要素の数より、または `positions_capacity` が頂点の位置の float の数より小さければ
/// `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、`*out` (要る数) だけを書く (バッファには何も書かない)。
/// 大きさは読み込みと着せ替えの後は変わらないので、すべてのポインタを NULL・大きさを 0 にして 1 回呼べば要る数が分かる。
///
/// `changed_positions_only` が true なら、頂点の位置は、変更のフラグ (`UGOE_CHANGED_POSITIONS`) が立った描画可能要素の
/// 範囲だけを写す (ほかの範囲は書かない)。毎回の `ugoe_puppet_update` の後に同じバッファへ写すホストのためのもの
/// (作った直後と最初の `update` の後は、すべてのフラグが立つ)。状態・描画順・範囲は常にすべてを写す。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`buffers` は読める `UgoeFrameBuffers` を指し、その中の各ポインタは NULL か、
/// 書かれた数の要素を書き込める領域を指すこと。`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_copy_frame(
    puppet: *mut UgoePuppet,
    buffers: *const UgoeFrameBuffers,
    changed_positions_only: bool,
    out: *mut UgoeFrameCopy,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let b = unsafe { buffers.as_ref() }.ok_or_else(|| Error::null("buffers"))?;
        if out.is_null() {
            return Err(Error::null("out"));
        }
        let (cache, instance) = p.frame();
        let count = cache.states.len();
        let copy = UgoeFrameCopy {
            drawable_count: count as u32,
            positions_len: cache.positions.len(),
            render_order_changed: instance.render_order_changed(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", copy)? };
        if b.drawable_capacity < count {
            return Err(Error::new(
                UGOE_ERROR_BUFFER_TOO_SMALL,
                format!(
                    "`drawable_capacity` is {} but {count} are needed",
                    b.drawable_capacity
                ),
            ));
        }
        if b.positions_capacity < cache.positions.len() {
            return Err(Error::new(
                UGOE_ERROR_BUFFER_TOO_SMALL,
                format!(
                    "`positions_capacity` is {} but {} are needed",
                    b.positions_capacity,
                    cache.positions.len()
                ),
            ));
        }
        // SAFETY: 各ポインタは NULL か、`drawable_capacity` (≥ count) 個を書き込める (呼び出し側の約束)。
        let states = unsafe { slice_mut(b.states, count, "states")? };
        // SAFETY: 同上。
        let order = unsafe { slice_mut(b.render_order, count, "render_order")? };
        // SAFETY: 同上。
        let ranges = unsafe { slice_mut(b.position_ranges, count, "position_ranges")? };
        // SAFETY: `positions` は NULL か、`positions_capacity` (≥ 要る数) 個の float を書き込める (呼び出し側の約束)。
        let positions = unsafe { slice_mut(b.positions, cache.positions.len(), "positions")? };
        states.copy_from_slice(&cache.states);
        order.copy_from_slice(instance.render_order());
        ranges.copy_from_slice(&cache.ranges);
        if changed_positions_only {
            for (state, range) in cache.states.iter().zip(&cache.ranges) {
                if state.changes & DrawableChanges::POSITIONS.bits() != 0 {
                    let r = range.first..range.first + range.count;
                    positions[r.clone()].copy_from_slice(&cache.positions[r]);
                }
            }
        } else {
            positions.copy_from_slice(&cache.positions);
        }
        Ok(())
    })
}

/// ホストのバッファ `ptr` の先頭 `len` 個を、書き込める列にする。`len` が 0 なら NULL でもよい。
///
/// # Safety
///
/// `ptr` は NULL か、`len` 個の `T` を書き込める領域を指すこと。
unsafe fn slice_mut<'a, T>(ptr: *mut T, len: usize, name: &str) -> Result<&'a mut [T], Error> {
    if len == 0 {
        return Ok(&mut []);
    }
    if ptr.is_null() {
        return Err(Error::null(name));
    }
    // SAFETY: 呼び出し側の約束。
    Ok(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
}
