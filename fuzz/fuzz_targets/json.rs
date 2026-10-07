//! 同じ入力を JSON 形式のモデル・モジュール・物理・モーション・表情として読み、読めたものを `Puppet` で動かす
//! (モデルはそのまま、モジュールは着せ替えの本体に取り付け、物理・モーション・表情は `ugoe_model::fixtures` のモデルに当てる)。
//! 種は `spec/examples` の JSON (`just fuzz json`)。

#![no_main]

use libfuzzer_sys::fuzz_target;
use ugoe::format::{
    expression_from_json, model_from_json, module_from_json, motion_from_json, physics_from_json,
};
use ugoe::model::fixtures;
use ugoe_fuzz::{attach_and_drive, drive_model};

fuzz_target!(|data: &[u8]| {
    let Ok(json) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(model) = model_from_json(json) {
        drive_model(model, None, &[], &[]);
    }
    if let Ok(module) = module_from_json(json) {
        attach_and_drive(module);
    }
    if let Ok(physics) = physics_from_json(json) {
        drive_model(fixtures::physics_hair(), Some(physics), &[], &[]);
    }
    if let Ok(motion) = motion_from_json(json) {
        drive_model(fixtures::pose_and_effects(), None, &[motion], &[]);
    }
    if let Ok(expression) = expression_from_json(json) {
        drive_model(fixtures::pose_and_effects(), None, &[], &[expression]);
    }
});
