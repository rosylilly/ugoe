//! 描画可能要素 (メッシュ) の評価結果と、テクスチャの取り出し。
//! 出力の意味は `docs/spec/evaluation.md#8-出力`、描画の式は `docs/spec/rendering.md`。

use std::ffi::c_char;

use ugoe::format::TextureFormat;
use ugoe::model::{BlendMode, MaskMode};

use crate::error::{Error, UGOE_ERROR_BUFFER_TOO_SMALL, UgoeResult, guard};
use crate::puppet::{UgoePuppet, puppet_ref, write};

/// 通常の合成。
pub const UGOE_BLEND_NORMAL: u32 = 0;
/// 加算。
pub const UGOE_BLEND_ADDITIVE: u32 = 1;
/// 乗算。
pub const UGOE_BLEND_MULTIPLY: u32 = 2;

/// 通常マスクの和集合 `1 − Π(1 − c)`。
pub const UGOE_MASK_UNION: u32 = 0;
/// 通常マスクの共通部分 `Π c`。
pub const UGOE_MASK_INTERSECTION: u32 = 1;

/// テクスチャの画像がない (JSON から作ったとき、またはバンドルに無い番号)。白で代用する。
pub const UGOE_TEXTURE_NONE: u32 = 0;
/// PNG (乗算していないアルファ)。描画の前に乗算済みアルファにする (`docs/spec/rendering.md`)。
pub const UGOE_TEXTURE_PNG: u32 = 1;
/// KTX2。
pub const UGOE_TEXTURE_KTX2: u32 = 2;

/// `UgoeDrawableState::changes` のビット: 頂点位置。
pub const UGOE_CHANGED_POSITIONS: u8 = 1 << 0;
/// 不透明度。
pub const UGOE_CHANGED_OPACITY: u8 = 1 << 1;
/// 描画順の値 (並び全体の変化は `ugoe_puppet_render_order_changed`)。
pub const UGOE_CHANGED_DRAW_ORDER: u8 = 1 << 2;
/// 描くかどうか (`visible`)。
pub const UGOE_CHANGED_VISIBILITY: u8 = 1 << 3;
/// 乗算色。
pub const UGOE_CHANGED_MULTIPLY_COLOR: u8 = 1 << 4;
/// スクリーン色。
pub const UGOE_CHANGED_SCREEN_COLOR: u8 = 1 << 5;
/// マスクの効き具合。
pub const UGOE_CHANGED_MASK_STRENGTH: u8 = 1 << 6;

/// 描画可能要素の、評価で変わらない情報。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeDrawableInfo {
    /// ID (UTF-8、NUL 終端)。`UgoePuppet` を破棄するまで有効。
    pub id: *const c_char,
    /// 使うテクスチャの番号 (`ugoe_puppet_texture`)。
    pub texture: u32,
    /// 頂点の数 (位置は 2 × この数の float)。
    pub vertex_count: u32,
    /// 三角形の頂点番号の数 (3 の倍数)。
    pub index_count: u32,
    /// 裏向きの三角形を描かないか (`docs/spec/rendering.md` の表裏の規則)。
    pub culling: bool,
    /// マスク専用で、色としては描かないか。
    pub mask_only: bool,
    /// マスクの参照の数 (`ugoe_puppet_drawable_mask`)。
    pub mask_count: u32,
    /// 通常マスクの結合方法 (`UGOE_MASK_*`)。
    pub mask_mode: u32,
}

/// マスクの参照 1 つ。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeMask {
    /// マスクとして使う描画可能要素の番号。
    pub drawable: u32,
    /// true なら反転マスク (その要素の内側を隠す)。
    pub invert: bool,
}

/// 描画可能要素の、直前の `ugoe_puppet_update` で評価した状態。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeDrawableState {
    pub opacity: f32,
    /// 描画順の値 (大きいほど手前)。並べた結果は `ugoe_puppet_render_order`。
    pub draw_order: f32,
    /// 乗算色 (RGB)。
    pub multiply_color: [f32; 3],
    /// スクリーン色 (RGB)。
    pub screen_color: [f32; 3],
    /// 合成モード (`UGOE_BLEND_*`)。知らないモードは通常の合成に置き換え済み。
    pub blend_mode: u32,
    /// マスクの効き具合 (0〜1)。マスクを持たない要素では意味を持たない。
    pub mask_strength: f32,
    /// 色として描くか。不透明度 0 やマスク専用の要素は false。
    pub visible: bool,
    /// 前の `ugoe_puppet_update` から変わったもの (`UGOE_CHANGED_*` のビット和)。
    /// 作った直後と最初の `update` の後はすべてのビットが立つ。
    pub changes: u8,
}

/// 画像のエンコードされたままのバイト列。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeTexture {
    /// `UGOE_TEXTURE_*`。
    pub format: u32,
    /// ファイルの中身。`UGOE_TEXTURE_NONE` なら NULL。`UgoePuppet` を破棄するまで有効。
    pub data: *const u8,
    pub len: usize,
}

fn blend_code(blend: BlendMode) -> u32 {
    match blend.effective() {
        BlendMode::Additive => UGOE_BLEND_ADDITIVE,
        BlendMode::Multiply => UGOE_BLEND_MULTIPLY,
        BlendMode::Normal | BlendMode::Unknown => UGOE_BLEND_NORMAL,
    }
}

fn mask_mode_code(mode: MaskMode) -> u32 {
    match mode.effective() {
        MaskMode::Intersection => UGOE_MASK_INTERSECTION,
        MaskMode::Union | MaskMode::Unknown => UGOE_MASK_UNION,
    }
}

impl UgoePuppet {
    fn drawable_index(&self, index: u32) -> Result<usize, Error> {
        let len = self.drawable_ids.len();
        let i = index as usize;
        if i < len {
            Ok(i)
        } else {
            Err(Error::out_of_range("drawable", i, len))
        }
    }
}

/// 描画可能要素 (メッシュ) の数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.drawable_ids.len() as u32) }
    })
}

/// `index` 番目の描画可能要素の、評価で変わらない情報。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_info(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeDrawableInfo,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let compiled = p.puppet.instance().compiled();
        let mesh = &compiled.model().meshes[i];
        let info = UgoeDrawableInfo {
            id: p.drawable_ids[i].as_ptr(),
            texture: mesh.texture,
            vertex_count: p.puppet.instance().drawables()[i].positions.len() as u32,
            index_count: mesh.indices.len() as u32,
            culling: mesh.culling,
            mask_only: mesh.mask_only,
            mask_count: compiled.mask_refs(i).len() as u32,
            mask_mode: mask_mode_code(compiled.mask_mode(i)),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", info) }
    })
}

/// `index` 番目の描画可能要素の、`mask` 番目のマスクの参照。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_mask(
    puppet: *const UgoePuppet,
    index: u32,
    mask: u32,
    out: *mut UgoeMask,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let refs = p.puppet.instance().compiled().mask_refs(i);
        let r = refs
            .get(mask as usize)
            .ok_or_else(|| Error::out_of_range("mask", mask as usize, refs.len()))?;
        let value = UgoeMask {
            drawable: r.mesh,
            invert: r.invert,
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", value) }
    })
}

/// `index` 番目の描画可能要素の、直前の `ugoe_puppet_update` で評価した状態。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_state(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeDrawableState,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let instance = p.puppet.instance();
        let d = &instance.drawables()[i];
        let state = UgoeDrawableState {
            opacity: d.opacity,
            draw_order: d.draw_order,
            multiply_color: [d.multiply_color.r, d.multiply_color.g, d.multiply_color.b],
            screen_color: [d.screen_color.r, d.screen_color.g, d.screen_color.b],
            blend_mode: blend_code(d.blend),
            mask_strength: d.mask_strength,
            visible: d.visible,
            changes: instance.changes()[i].bits(),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", state) }
    })
}

/// `index` 番目の描画可能要素の頂点位置 (キャンバス座標、x と y の組を頂点の数だけ) を、
/// ホストのバッファ `out` (`len` 個の float) にコピーする。`len` が 2 × 頂点数より小さければ
/// `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、何も書かない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は `len` 個の float を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_copy_positions(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut f32,
    len: usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let positions = &p.puppet.instance().drawables()[i].positions;
        let needed = positions.len() * 2;
        if len < needed {
            return Err(Error::new(
                UGOE_ERROR_BUFFER_TOO_SMALL,
                format!("`out` holds {len} floats but {needed} are needed"),
            ));
        }
        if needed == 0 {
            return Ok(());
        }
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: `out` は `len` (≥ needed) 個の float を書き込める (呼び出し側の約束)。
        let out = unsafe { std::slice::from_raw_parts_mut(out, needed) };
        for (dst, v) in out.as_chunks_mut::<2>().0.iter_mut().zip(positions) {
            *dst = [v.x, v.y];
        }
        Ok(())
    })
}

/// `index` 番目の描画可能要素の頂点位置を、内部のバッファのまま借りる。`*out` に先頭
/// (x と y の組が `*out_len` / 2 個)、`*out_len` に float の数を書く。
///
/// ポインタは、次の `ugoe_puppet_update` か `ugoe_puppet_destroy` まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_positions(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const f32,
    out_len: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let positions = &p.puppet.instance().drawables()[i].positions;
        // `glam::Vec2` は `#[repr(C)] { x: f32, y: f32 }` なので、float の並びとして読める。
        let ptr = positions.as_ptr().cast::<f32>();
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", positions.len() * 2)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", ptr) }
    })
}

/// `index` 番目の描画可能要素のテクスチャ座標 (u と v の組を頂点の数だけ。0〜1、画像の左上が原点) を借りる。
/// ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_uvs(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const f32,
    out_len: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let uvs = &p.puppet.instance().compiled().model().meshes[i].uvs;
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", uvs.len() * 2)? };
        // SAFETY: 呼び出し側の約束。`Vec2` の並びは float の並びとして読める。
        unsafe { write(out, "out", uvs.as_ptr().cast::<f32>()) }
    })
}

/// `index` 番目の描画可能要素の三角形の頂点番号 (3 つで 1 つの三角形) を借りる。
/// ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_drawable_indices(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const u16,
    out_len: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let indices = &p.puppet.instance().compiled().model().meshes[i].indices;
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", indices.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", indices.as_ptr()) }
    })
}

/// 奥から手前への描画順 (描画可能要素の番号の並び) を借りる。`*out_len` は描画可能要素の数。
/// ポインタは、次の `ugoe_puppet_update` か `ugoe_puppet_destroy` まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_render_order(
    puppet: *const UgoePuppet,
    out: *mut *const u32,
    out_len: *mut usize,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let order = p.puppet.instance().render_order();
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", order.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", order.as_ptr()) }
    })
}

/// 前の `ugoe_puppet_update` から描画順の並びが変わったか (作った直後と最初の `update` の後は true)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_render_order_changed(
    puppet: *const UgoePuppet,
    out: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let changed = p.puppet.instance().render_order_changed();
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", changed) }
    })
}

/// テクスチャの数 (描画可能要素の `texture` はこれより小さい)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_texture_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.textures.len() as u32) }
    })
}

/// `index` 番目のテクスチャの画像 (エンコードされたまま)。デコードはホストが行う。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_texture(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeTexture,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let texture = p
            .textures
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("texture", index as usize, p.textures.len()))?;
        let value = match texture {
            None => UgoeTexture {
                format: UGOE_TEXTURE_NONE,
                data: std::ptr::null(),
                len: 0,
            },
            Some(t) => UgoeTexture {
                format: match t.format {
                    TextureFormat::Png => UGOE_TEXTURE_PNG,
                    TextureFormat::Ktx2 => UGOE_TEXTURE_KTX2,
                },
                data: t.bytes.as_ptr(),
                len: t.bytes.len(),
            },
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", value) }
    })
}
