//! 同じ入力を PNG と KTX2 の両方としてデコードする (乗算済みアルファまで)。
//! 種は `fuzz/seeds/image` (`just fuzz image`)。

#![no_main]

use libfuzzer_sys::fuzz_target;
use ugoe::format::TextureFormat;
use ugoe::format::image::{ImageLimits, decode_image_premultiplied};

fuzz_target!(|data: &[u8]| {
    for format in [TextureFormat::Png, TextureFormat::Ktx2] {
        let _ = decode_image_premultiplied(format, data, ImageLimits::default());
    }
});
