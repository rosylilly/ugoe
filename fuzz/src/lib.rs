//! fuzz のターゲット (`fuzz_targets/`) が共有する、読めた入力を使い切る処理 (docs/testing.md#fuzz)。
//!
//! 決まった入力の検査 (`crates/ugoe-format/tests/broken_input.rs`・`crates/ugoe/tests/broken_input.rs`) と同じく、
//! 読めたものはデコードし、`Puppet` にして動かす。読めないのはかまわない (パニックしなければよい)。

use std::sync::{Arc, LazyLock};

use ugoe::Puppet;
use ugoe::core::Vec2;
use ugoe::format::image::{ImageLimits, decode_image_premultiplied};
use ugoe::format::{TextureData, read_bundle};
use ugoe::model::Model;
use ugoe::model::expression::Expression;
use ugoe::model::module::Module;
use ugoe::model::motion::Motion;
use ugoe::model::physics::PhysicsSettings;
use ugoe::motion::Priority;

/// 進める時間: 短い `dt`・止まった時間・大きな `dt`・有限でない `dt` (`docs/portability.md#ホストから来る数`)。
const DTS: [f32; 6] = [1.0 / 60.0, 1.0 / 60.0, 0.0, 0.5, f32::MAX, f32::NAN];

/// テクスチャと法線マップをデコードする (読めないのはかまわない)。
pub fn decode_all<'a>(textures: impl IntoIterator<Item = &'a TextureData>) {
    for t in textures {
        let _ = decode_image_premultiplied(t.format, &t.bytes, ImageLimits::default());
    }
}

/// モデルを `Puppet` にして、物理・モーション・表情を当てて動かす (コンパイルできないのはかまわない)。
pub fn drive_model(
    model: Model,
    physics: Option<PhysicsSettings>,
    motions: &[Motion],
    expressions: &[Expression],
) {
    let Ok(mut puppet) = Puppet::new(model) else {
        return;
    };
    if let Some(physics) = physics {
        puppet = puppet.with_physics(physics);
    }
    drive(&mut puppet, motions, expressions);
}

/// モーションと表情を当てて進め、当たり判定を問い合わせる。
pub fn drive(puppet: &mut Puppet, motions: &[Motion], expressions: &[Expression]) {
    for motion in motions {
        puppet.start_motion(motion.clone(), Priority::Force);
    }
    if let Some(expression) = expressions.first() {
        puppet.set_expression(expression.clone());
    }
    for dt in DTS {
        puppet.update(dt);
    }
    let _ = puppet.hit_areas_at(Vec2::new(0.0, 0.0));
}

/// 着せ替えの本体 (`assets/compat/dev/dress-up-base.ugoe`)。
static DRESS_UP_BASE: LazyLock<Option<Model>> = LazyLock::new(|| {
    read_bundle(include_bytes!("../../assets/compat/dev/dress-up-base.ugoe"))
        .ok()
        .map(|bundle| bundle.model)
});

/// 着せ替えのモジュールを本体 (`assets/compat/dev/dress-up-base.ugoe`) の `Puppet` に取り付けて動かす
/// (取り付けられないのはかまわない)。
pub fn attach_and_drive(module: Module) {
    let Some(base) = DRESS_UP_BASE.as_ref() else {
        return;
    };
    let Ok(mut puppet) = Puppet::new(base.clone()) else {
        return;
    };
    if puppet.attach_module(Arc::new(module)).is_ok() {
        drive(&mut puppet, &[], &[]);
    }
}
