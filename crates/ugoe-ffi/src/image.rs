//! テクスチャの画像 (PNG・KTX2) のデコード (T0466)。
//!
//! `ugoe_puppet_texture` / `ugoe_puppet_normal_map` はエンコードされたままのバイト列を渡し、デコードはホストが行う。
//! ゲームエンジンの標準のデコーダが読めない形式 (KTX2 の BCn など) のために、参照 (`ugoe render`)・wasm と同じ
//! `ugoe_format::image` のデコーダを出す。画像の扱いは `docs/spec/rendering.md#前提`、読む形式は
//! `docs/adr/0030-compressed-ktx2-decoding.md`。
//!
//! - バイト列と形式を受け取るだけの関数で、`UgoePuppet` の状態は変えない (読み込みの関数と意味は変わらない)。
//! - デコードの上限はホストが決める (`UgoeImageLimits`。NULL なら既定値)。
//! - 結果は不透明な `UgoeImage` で持ち、`ugoe_image_destroy` で解放する。

use ugoe::format::TextureFormat;
use ugoe_format::image::{
    DEFAULT_MAX_IMAGE_SIDE, ImageError, ImageLimits, Rgba8, decode_image,
    decode_image_premultiplied,
};

use crate::drawable::{UGOE_TEXTURE_KTX2, UGOE_TEXTURE_PNG};
use crate::error::{
    Error, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_ARGUMENT, UGOE_ERROR_LIMIT_EXCEEDED, UgoeError,
    UgoeResult, guard,
};
use crate::puppet::write;

/// 画像のデコードの上限 (Rust の `ugoe_format::image::ImageLimits`)。値は `ugoe_image_limits_default` から始めて変える。
///
/// 上限を足すときは、この構造体を変えずに別の構造体と関数を足す (crates/ugoe-ffi/README.md#abi-の版)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UgoeImageLimits {
    /// 画像の幅と高さそれぞれの上限 (画素)。既定は 16384。超える画像は画素を確保する前に失敗にする。
    pub max_side: u32,
}

/// 既定の画像のデコードの上限 (`limits` に NULL を渡したときの値)。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_image_limits_default() -> UgoeImageLimits {
    UgoeImageLimits {
        max_side: DEFAULT_MAX_IMAGE_SIDE,
    }
}

/// デコードした画像 (8bit RGBA)。`ugoe_image_decode` で作り、`ugoe_image_destroy` で解放する。
#[derive(Debug)]
pub struct UgoeImage {
    image: Rgba8,
}

fn image_error(error: ImageError) -> Error {
    /// 種類 (`ugoe_error_kind`): 一辺が上限を超える。補足の値は `width`・`height`・`max` (wasm の
    /// `image_limit_exceeded` と同じ値。wasm はさらに `image`・`texture` を持つ)。
    const IMAGE_LIMIT_EXCEEDED: &str = "image_limit_exceeded";
    /// 種類: 画像が壊れている (wasm の `invalid_texture` と同じ)。
    const INVALID_TEXTURE: &str = "invalid_texture";
    /// 種類: 読める KTX2 だが、デコードしない形式 (ETC2・ASTC・BC6H・Basis Universal など)。wasm では読み込みの警告
    /// (`texture_ktx2`・`normal_map_ktx2`) にして白で代用するもの。
    const UNSUPPORTED_TEXTURE: &str = "unsupported_texture";

    let message = error.to_string();
    match error {
        ImageError::TooLarge { width, height, max } => {
            Error::new(UGOE_ERROR_LIMIT_EXCEEDED, message).with_kind(
                IMAGE_LIMIT_EXCEEDED,
                vec![
                    ("width", width.to_string()),
                    ("height", height.to_string()),
                    ("max", max.to_string()),
                ],
            )
        }
        ImageError::UnsupportedKtx2(_) => {
            Error::new(UGOE_ERROR_FORMAT, message).with_kind(UNSUPPORTED_TEXTURE, Vec::new())
        }
        _ => Error::new(UGOE_ERROR_FORMAT, message).with_kind(INVALID_TEXTURE, Vec::new()),
    }
}

/// エンコードされた画像 (`ugoe_puppet_texture` / `ugoe_puppet_normal_map` のバイト列) を 8bit RGBA にデコードする。
///
/// 参照 (`ugoe render`)・wasm と同じデコーダで、同じ画素になる (`docs/spec/rendering.md#前提`)。PNG と、KTX2 の
/// 圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7 を読む。色はリニア化せず、KTX2 のミップマップは最初の段だけを使う。
/// 画素は左上から行の順 (上の行が先)。
///
/// - `format`: `UGOE_TEXTURE_PNG` か `UGOE_TEXTURE_KTX2` (`UgoeTexture::format`)。ほかは `UGOE_ERROR_INVALID_ARGUMENT`。
/// - `limits`: NULL なら既定値 (`ugoe_image_limits_default`)。
/// - `premultiply`: true なら乗算済みアルファにする (描画するテクスチャ。参照と同じ整数の丸め)。法線マップは false
///   (アルファを掛けない)。
///
/// 失敗したときの戻り値と種類 (`ugoe_error_kind`):
///
/// - `UGOE_ERROR_LIMIT_EXCEEDED`・`"image_limit_exceeded"`: 一辺が上限を超える (補足の値 `width`・`height`・`max`)。
/// - `UGOE_ERROR_FORMAT`・`"unsupported_texture"`: 読める KTX2 だが、デコードしない形式 (ETC2・ASTC など)。白で代用するとよい。
/// - `UGOE_ERROR_FORMAT`・`"invalid_texture"`: 画像が壊れている。
///
/// # Safety
///
/// `data` は `len` バイトを読める領域 (`len` が 0 なら NULL でよい)、`limits` は NULL か `UgoeImageLimits` を
/// 読める領域、`out` は `UgoeImage*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_image_decode(
    format: u32,
    data: *const u8,
    len: usize,
    limits: *const UgoeImageLimits,
    premultiply: bool,
    out: *mut *mut UgoeImage,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        if out.is_null() {
            return Err(Error::null("out"));
        }
        let format = match format {
            UGOE_TEXTURE_PNG => TextureFormat::Png,
            UGOE_TEXTURE_KTX2 => TextureFormat::Ktx2,
            other => {
                return Err(Error::new(
                    UGOE_ERROR_INVALID_ARGUMENT,
                    format!("unknown texture format {other}"),
                ));
            }
        };
        // SAFETY: 呼び出し側の約束。
        let (bytes, limits) = unsafe {
            (
                crate::puppet::bytes(data, len, "data")?,
                limits.as_ref().copied(),
            )
        };
        let limits = ImageLimits::default()
            .with_max_side(limits.map_or(DEFAULT_MAX_IMAGE_SIDE, |l| l.max_side));
        let image = if premultiply {
            decode_image_premultiplied(format, bytes, limits)
        } else {
            decode_image(format, bytes, limits)
        }
        .map_err(image_error)?;
        let boxed = Box::into_raw(Box::new(UgoeImage { image }));
        // SAFETY: `out` は NULL でないことを確かめた。呼び出し側の約束で書き込める。
        unsafe { out.write(boxed) };
        Ok(())
    })
}

/// `ugoe_image_decode` で作った画像を解放する。NULL なら何もしない。
///
/// # Safety
///
/// `image` は NULL か、`ugoe_image_decode` で作ってまだ解放していない `UgoeImage` であること。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_image_destroy(image: *mut UgoeImage) {
    if image.is_null() {
        return;
    }
    guard(std::ptr::null_mut(), || {
        // SAFETY: `ugoe_image_decode` が Box::into_raw で作ったポインタ。
        drop(unsafe { Box::from_raw(image) });
        Ok(())
    });
}

/// `UgoeImage` の共有参照を得る。
///
/// # Safety
///
/// `ptr` は NULL か、解放されていない `UgoeImage` を指すこと。
unsafe fn image_ref<'a>(ptr: *const UgoeImage) -> Result<&'a UgoeImage, Error> {
    // SAFETY: 呼び出し側の約束。
    unsafe { ptr.as_ref() }.ok_or_else(|| Error::null("image"))
}

/// 画像の幅と高さ (画素)。
///
/// # Safety
///
/// `image` は有効な `UgoeImage`、`out_width` と `out_height` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_image_size(
    image: *const UgoeImage,
    out_width: *mut u32,
    out_height: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let i = unsafe { image_ref(image)? };
        if out_height.is_null() {
            return Err(Error::null("out_height"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_width, "out_width", i.image.width)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_height, "out_height", i.image.height) }
    })
}

/// 画像の画素 (`width * height * 4` バイト。R・G・B・A の順で、左上から行の順)。ポインタは `UgoeImage` を解放するまで有効。
///
/// # Safety
///
/// `image` は有効な `UgoeImage`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_image_pixels(
    image: *const UgoeImage,
    out: *mut *const u8,
    out_len: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let i = unsafe { image_ref(image)? };
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", i.image.pixels.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", i.image.pixels.as_ptr()) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawable::UGOE_TEXTURE_NONE;
    use crate::error::{TestError, UGOE_ERROR_NULL_POINTER, UGOE_OK};

    /// 1×2 の RGBA の PNG (上が半透明の赤、下が不透明の緑)。
    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer
                .write_image_data(&[255, 0, 0, 128, 0, 255, 0, 255])
                .expect("data");
        }
        bytes
    }

    fn decode(
        format: u32,
        bytes: &[u8],
        limits: Option<UgoeImageLimits>,
        premultiply: bool,
    ) -> (UgoeResult, *mut UgoeImage, TestError) {
        let error = TestError::new();
        let mut image = std::ptr::null_mut();
        let limits_ptr = limits
            .as_ref()
            .map_or(std::ptr::null(), |l| l as *const UgoeImageLimits);
        // SAFETY: 有効な領域を渡している。
        let code = unsafe {
            ugoe_image_decode(
                format,
                bytes.as_ptr(),
                bytes.len(),
                limits_ptr,
                premultiply,
                &mut image,
                error.0,
            )
        };
        (code, image, error)
    }

    fn pixels(image: *const UgoeImage) -> (u32, u32, Vec<u8>) {
        let (mut w, mut h) = (0, 0);
        let mut data = std::ptr::null();
        let mut len = 0;
        // SAFETY: 有効な `UgoeImage` と書き込める領域を渡している。
        unsafe {
            assert_eq!(
                ugoe_image_size(image, &mut w, &mut h, std::ptr::null_mut()),
                UGOE_OK
            );
            assert_eq!(
                ugoe_image_pixels(image, &mut data, &mut len, std::ptr::null_mut()),
                UGOE_OK
            );
            (w, h, std::slice::from_raw_parts(data, len).to_vec())
        }
    }

    #[test]
    fn decodes_png_straight_and_premultiplied() {
        let bytes = png();
        let (code, image, _) = decode(UGOE_TEXTURE_PNG, &bytes, None, false);
        assert_eq!(code, UGOE_OK);
        assert_eq!(pixels(image), (1, 2, vec![255, 0, 0, 128, 0, 255, 0, 255]));
        // SAFETY: 作った画像を 1 度だけ解放する。
        unsafe { ugoe_image_destroy(image) };

        let (code, image, _) = decode(UGOE_TEXTURE_PNG, &bytes, None, true);
        assert_eq!(code, UGOE_OK);
        let expected = ugoe_format::image::decode_png_premultiplied(&bytes, ImageLimits::default())
            .unwrap()
            .pixels;
        assert_eq!(pixels(image).2, expected);
        assert_eq!(expected[..4], [128, 0, 0, 128]);
        // SAFETY: 同上。
        unsafe { ugoe_image_destroy(image) };
    }

    #[test]
    fn decodes_bc1_ktx2_like_the_sdk() {
        // VK_FORMAT_BC1_RGB_UNORM_BLOCK (131) の 4×4 の 1 ブロック。color0 = 赤 (565 の 0xF800)、color1 = 青 (0x001F)。
        // 1 行目の 4 画素だけ番号 1 (青)、ほかは番号 0 (赤)。
        let mut bytes = ktx2_with_format(131, 4, 4, 8);
        let block = bytes.len() - 8;
        bytes[block..].copy_from_slice(&[0x00, 0xF8, 0x1F, 0x00, 0b0101_0101, 0, 0, 0]);
        let (code, image, _) = decode(UGOE_TEXTURE_KTX2, &bytes, None, true);
        assert_eq!(code, UGOE_OK);
        let (w, h, px) = pixels(image);
        assert_eq!((w, h), (4, 4));
        assert_eq!(px[..4], [0, 0, 255, 255], "上の行が先");
        assert_eq!(px[16..20], [255, 0, 0, 255]);
        let expected =
            ugoe_format::image::decode_ktx2_premultiplied(&bytes, ImageLimits::default())
                .unwrap()
                .pixels;
        assert_eq!(px, expected);
        // SAFETY: 作った画像を 1 度だけ解放する。
        unsafe { ugoe_image_destroy(image) };
    }

    #[test]
    fn limit_exceeded_has_kind_and_fields() {
        let (code, image, error) = decode(
            UGOE_TEXTURE_PNG,
            &png(),
            Some(UgoeImageLimits { max_side: 1 }),
            true,
        );
        assert_eq!(code, UGOE_ERROR_LIMIT_EXCEEDED);
        assert!(image.is_null());
        assert_eq!(error.kind().as_deref(), Some("image_limit_exceeded"));
        let field = |key: &str| error.field(&std::ffi::CString::new(key).unwrap()).unwrap();
        assert_eq!(field("width"), "1");
        assert_eq!(field("height"), "2");
        assert_eq!(field("max"), "1");
    }

    #[test]
    fn broken_and_unsupported_images_have_kinds() {
        let (code, image, error) = decode(UGOE_TEXTURE_PNG, b"not a png", None, true);
        assert_eq!(code, UGOE_ERROR_FORMAT);
        assert!(image.is_null());
        assert_eq!(error.kind().as_deref(), Some("invalid_texture"));

        let (code, _, error) = decode(UGOE_TEXTURE_KTX2, b"not a ktx2", None, true);
        assert_eq!(code, UGOE_ERROR_FORMAT);
        assert_eq!(error.kind().as_deref(), Some("invalid_texture"));

        // ETC2 (VK_FORMAT_ETC2_R8G8B8A8_UNORM_BLOCK = 151) の 4×4 の KTX2。読めるがデコードしない。
        let (code, _, error) = decode(
            UGOE_TEXTURE_KTX2,
            &ktx2_with_format(151, 4, 4, 16),
            None,
            true,
        );
        assert_eq!(code, UGOE_ERROR_FORMAT);
        assert_eq!(error.kind().as_deref(), Some("unsupported_texture"));
    }

    #[test]
    fn rejects_unknown_formats_and_null_pointers() {
        let bytes = png();
        let (code, _, _) = decode(UGOE_TEXTURE_NONE, &bytes, None, true);
        assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
        let (code, _, _) = decode(99, &bytes, None, true);
        assert_eq!(code, UGOE_ERROR_INVALID_ARGUMENT);
        // SAFETY: NULL を渡して失敗を確かめる。
        unsafe {
            let code = ugoe_image_decode(
                UGOE_TEXTURE_PNG,
                bytes.as_ptr(),
                bytes.len(),
                std::ptr::null(),
                true,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            assert_eq!(code, UGOE_ERROR_NULL_POINTER);
            let (mut w, mut h) = (0, 0);
            assert_eq!(
                ugoe_image_size(std::ptr::null(), &mut w, &mut h, std::ptr::null_mut()),
                UGOE_ERROR_NULL_POINTER
            );
            ugoe_image_destroy(std::ptr::null_mut());
        }
    }

    #[test]
    fn default_limits_match_the_sdk() {
        assert_eq!(
            ugoe_image_limits_default().max_side,
            ImageLimits::default().max_side
        );
    }

    /// 1 段だけの KTX2 (`vk_format`・`width`×`height`・中身は `level_len` バイトの 0)。
    fn ktx2_with_format(vk_format: u32, width: u32, height: u32, level_len: usize) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ugoe_format::image::KTX2_IDENTIFIER);
        let header = [vk_format, 1, width, height, 0, 0, 1, 1, 0];
        for v in header {
            out.extend_from_slice(&v.to_le_bytes());
        }
        // 索引: dfd・kvd (なし)、sgd (なし)
        let level_offset = 80 + 24;
        for v in [0u32, 0, 0, 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in [0u64, 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&(level_offset as u64).to_le_bytes());
        out.extend_from_slice(&(level_len as u64).to_le_bytes());
        out.extend_from_slice(&(level_len as u64).to_le_bytes());
        out.resize(level_offset + level_len, 0);
        out
    }
}
