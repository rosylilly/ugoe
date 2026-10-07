//! `.ugoe` のバンドルを読み、画像をデコードし、読めたモデルを `Puppet` で動かす。
//! 種は `assets/compat` の `.ugoe` (`just fuzz bundle`)。

#![no_main]

use libfuzzer_sys::fuzz_target;
use ugoe::format::read_bundle;
use ugoe_fuzz::{decode_all, drive_model};

fuzz_target!(|data: &[u8]| {
    let Ok(bundle) = read_bundle(data) else {
        return;
    };
    decode_all(bundle.textures.iter().chain(&bundle.normal_maps));
    drive_model(
        bundle.model,
        bundle.physics,
        &bundle.motions,
        &bundle.expressions,
    );
});
