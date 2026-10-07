//! `.ugoemod` (着せ替えのモジュール) を読み、画像をデコードし、`assets/compat` の着せ替えの本体に取り付けて動かす。
//! 種は `assets/compat` の `.ugoemod` (`just fuzz module`)。

#![no_main]

use libfuzzer_sys::fuzz_target;
use ugoe::format::module_file::read_module;
use ugoe_fuzz::{attach_and_drive, decode_all};

fuzz_target!(|data: &[u8]| {
    let Ok(file) = read_module(data) else {
        return;
    };
    decode_all(file.textures.iter().chain(&file.normal_maps));
    attach_and_drive(file.module);
});
