//! 描画可能要素 (メッシュ) の評価結果と、テクスチャの取り出し。
//! 出力の意味は `docs/spec/evaluation.md#8-出力`、描画の式は `docs/spec/rendering.md`。

use core::ffi::c_char;

use ugoe::format::{TextureData, TextureFormat};
use ugoe::model::{BlendMode, MaskMode};

use crate::error::{Error, UgoeError, UgoeResult, at, guard};
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
/// 描画順の値 (並び全体の変化は `UgoeFrame::render_order_changed`)。
pub const UGOE_CHANGED_DRAW_ORDER: u8 = 1 << 2;
/// 描くかどうか (`visible`)。
pub const UGOE_CHANGED_VISIBILITY: u8 = 1 << 3;
/// 乗算色。
pub const UGOE_CHANGED_MULTIPLY_COLOR: u8 = 1 << 4;
/// スクリーン色。
pub const UGOE_CHANGED_SCREEN_COLOR: u8 = 1 << 5;
/// マスクの効き具合。
pub const UGOE_CHANGED_MASK_STRENGTH: u8 = 1 << 6;

// ビットは `ugoe::runtime::DrawableChanges` のものをそのまま渡す (`ugoe_puppet_frame`)。
const _: () = {
    use ugoe::runtime::DrawableChanges as C;
    assert!(UGOE_CHANGED_POSITIONS == C::POSITIONS.bits());
    assert!(UGOE_CHANGED_OPACITY == C::OPACITY.bits());
    assert!(UGOE_CHANGED_DRAW_ORDER == C::DRAW_ORDER.bits());
    assert!(UGOE_CHANGED_VISIBILITY == C::VISIBILITY.bits());
    assert!(UGOE_CHANGED_MULTIPLY_COLOR == C::MULTIPLY_COLOR.bits());
    assert!(UGOE_CHANGED_SCREEN_COLOR == C::SCREEN_COLOR.bits());
    assert!(UGOE_CHANGED_MASK_STRENGTH == C::MASK_STRENGTH.bits());
    assert!(C::ALL.bits() == (1 << 7) - 1);
};

// UV は `Vec2` の並びを float の並びとして貸す (`ugoe_puppet_drawable_uvs`)。
const _: () = {
    assert!(size_of::<ugoe::core::Vec2>() == 2 * size_of::<f32>());
    assert!(align_of::<ugoe::core::Vec2>() == align_of::<f32>());
};

/// 描画可能要素の、評価で変わらない情報。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UgoeDrawableInfo {
    /// ID (UTF-8、NUL 終端)。着せ替えの操作か `UgoePuppet` の破棄まで有効。
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

/// 描画可能要素の、直前の `ugoe_puppet_update` で評価した状態 (`ugoe_puppet_frame` で受け取る)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeDrawableState {
    pub opacity: f32,
    /// 描画順の値 (大きいほど手前)。並べた結果は `UgoeFrame::render_order`。
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
    /// ファイルの中身。`UGOE_TEXTURE_NONE` なら NULL。寿命は `ugoe_puppet_texture` を参照。
    pub data: *const u8,
    pub len: usize,
}

pub(crate) fn blend_code(blend: BlendMode) -> u32 {
    match blend.effective() {
        BlendMode::Additive => UGOE_BLEND_ADDITIVE,
        BlendMode::Multiply => UGOE_BLEND_MULTIPLY,
        _ => UGOE_BLEND_NORMAL,
    }
}

fn mask_mode_code(mode: MaskMode) -> u32 {
    match mode.effective() {
        MaskMode::Intersection => UGOE_MASK_INTERSECTION,
        _ => UGOE_MASK_UNION,
    }
}

impl UgoePuppet {
    pub(crate) fn drawable_index(&self, index: u32) -> Result<usize, Error> {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let compiled = p.puppet.instance().compiled();
        let mesh = at(&compiled.model().meshes, "drawable", i)?;
        let info = UgoeDrawableInfo {
            id: at(&p.drawable_ids, "drawable", i)?.as_ptr(),
            texture: mesh.texture,
            vertex_count: at(p.puppet.instance().drawables(), "drawable", i)?
                .positions
                .len() as u32,
            index_count: mesh.indices.len() as u32,
            culling: mesh.culling,
            mask_only: mesh.mask_only,
            mask_count: compiled.mask_refs(i).unwrap_or_default().len() as u32,
            mask_mode: mask_mode_code(compiled.mask_mode(i).unwrap_or_default()),
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let refs = (p.puppet.instance().compiled().mask_refs(i)).unwrap_or_default();
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

/// `index` 番目の描画可能要素のテクスチャ座標 (u と v の組を頂点の数だけ。0〜1、画像の左上が原点) を借りる。
/// ポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効。
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let uvs = &at(
            &p.puppet.instance().compiled().model().meshes,
            "drawable",
            i,
        )?
        .uvs;
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
/// ポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効。
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.drawable_index(index)?;
        let indices = &at(
            &p.puppet.instance().compiled().model().meshes,
            "drawable",
            i,
        )?
        .indices;
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", indices.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", indices.as_ptr()) }
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.texture_count() as u32) }
    })
}

/// 画像を C に渡す形にする。C ABI に値の無い形式 (`ugoe-format` に後で足した形式) は画像が無いものとして渡す。
fn texture_value(texture: Option<&TextureData>) -> UgoeTexture {
    let none = UgoeTexture {
        format: UGOE_TEXTURE_NONE,
        data: core::ptr::null(),
        len: 0,
    };
    let Some(t) = texture else {
        return none;
    };
    let format = match t.format {
        TextureFormat::Png => UGOE_TEXTURE_PNG,
        TextureFormat::Ktx2 => UGOE_TEXTURE_KTX2,
        _ => return none,
    };
    UgoeTexture {
        format,
        data: t.bytes.as_ptr(),
        len: t.bytes.len(),
    }
}

/// `index` 番目のテクスチャの画像 (エンコードされたまま)。デコードはホストが行う。
///
/// ポインタは、本体のテクスチャなら `UgoePuppet` を破棄するまで、モジュールのテクスチャならそのモジュールを
/// 外すか別の `UgoeModule` に差し替えるまで (`UgoePuppet` の破棄を含む) 有効。`UgoeModule` の破棄では無効にならない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_texture(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeTexture,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let (texture, _) = p
            .texture_data(index as usize)
            .ok_or_else(|| Error::out_of_range("texture", index as usize, p.texture_count()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", texture_value(texture)) }
    })
}

/// `index` 番目のテクスチャの法線マップ (エンコードされたまま)。無ければ `format` が `UGOE_TEXTURE_NONE`
/// (法線 `(0, 0, 1)` として扱う)。PNG は **アルファを掛けずに** デコードする
/// (`docs/spec/rendering.md#ホストでのライティング`)。ポインタの寿命は `ugoe_puppet_texture` と同じ。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_normal_map(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeTexture,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let (_, normal_map) = p
            .texture_data(index as usize)
            .ok_or_else(|| Error::out_of_range("texture", index as usize, p.texture_count()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", texture_value(normal_map)) }
    })
}
