//! 壊れた入力 (切り詰め・ビットの反転・書き換えた数・深い入れ子) を読んでも、読み込みと画像のデコードと、
//! 読めたモデルの評価がパニックしないこと (T0184、`docs/portability.md#パニックしない`、ADR 0031)。
//!
//! 元にするのは `assets/compat` のバンドルとモジュール、`ugoe_model::fixtures` の JSON。
//! `Puppet` (SDK) を通す検査は `crates/ugoe/tests/broken_input.rs`。

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use ugoe_format::image::{ImageLimits, decode_image_premultiplied};
use ugoe_format::module_file::read_module;
use ugoe_format::{
    TextureData, expression_from_json, model_from_json, model_to_json, module_from_json,
    motion_from_json, physics_from_json, physics_to_json, read_bundle,
};
use ugoe_model::{Model, fixtures};
use ugoe_physics::PhysicsSystem;
use ugoe_runtime::{CompiledModel, ModelInstance};

fn compat_files(extension: &str) -> Vec<(String, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/compat");
    let mut files = Vec::new();
    for version in ["dev", "future"] {
        let dir: PathBuf = root.join(version);
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == extension) {
                let name = format!("{version}/{}", path.file_name().unwrap().to_string_lossy());
                files.push((name, std::fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    assert!(!files.is_empty(), "no .{extension} in assets/compat");
    files
}

/// `f` がパニックしなければ `true`。
fn catch(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok()
}

/// 切り詰めた列 (すべての長さ) と、ビットを反転した列 (どのバイトも、位置で決めた 1 ビット)。
fn broken(bytes: &[u8]) -> impl Iterator<Item = (String, Vec<u8>)> + '_ {
    let truncated =
        (0..bytes.len()).map(|len| (format!("truncated to {len}"), bytes[..len].to_vec()));
    let flipped = (0..bytes.len()).map(|i| {
        let mut b = bytes.to_vec();
        b[i] ^= 1 << (i % 8);
        (format!("bit {} of byte {i} flipped", i % 8), b)
    });
    truncated.chain(flipped)
}

/// テクスチャと法線マップをデコードする (読めないのはかまわない)。
fn decode_all<'a>(textures: impl Iterator<Item = &'a TextureData>) {
    for t in textures {
        let _ = decode_image_premultiplied(t.format, &t.bytes, ImageLimits::default());
    }
}

/// 読めたモデルをコンパイルし、物理と合わせて評価する。
fn evaluate(model: Model, physics: Option<ugoe_model::physics::PhysicsSettings>) {
    let Ok(compiled) = CompiledModel::compile(model) else {
        return;
    };
    let mut instance = ModelInstance::new(compiled);
    let mut system = physics.map(PhysicsSystem::new);
    for dt in [1.0 / 60.0, 0.0, f32::MAX, f32::NAN] {
        if let Some(system) = &mut system {
            system.update_instance(&mut instance, dt);
        }
        instance.update();
    }
}

#[test]
fn broken_bundles_do_not_panic() {
    for (name, bytes) in compat_files("ugoe") {
        for (case, broken) in broken(&bytes) {
            let ok = catch(|| {
                let Ok(bundle) = read_bundle(&broken) else {
                    return;
                };
                decode_all(bundle.textures.iter().chain(&bundle.normal_maps));
                evaluate(bundle.model, bundle.physics);
            });
            assert!(ok, "{name}: {case}");
        }
    }
}

#[test]
fn broken_modules_do_not_panic() {
    for (name, bytes) in compat_files("ugoemod") {
        for (case, broken) in broken(&bytes) {
            let ok = catch(|| {
                if let Ok(file) = read_module(&broken) {
                    decode_all(file.textures.iter().chain(&file.normal_maps));
                }
            });
            assert!(ok, "{name}: {case}");
        }
    }
}

/// JSON の壊し方: 切り詰め (間引いた位置)・1 文字の置き換え (間引いた位置)・すべての数の書き換え。
fn broken_json(json: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let boundaries: Vec<usize> = (0..json.len())
        .filter(|&i| json.is_char_boundary(i))
        .collect();
    // 長い JSON でも数百か所に収まるように間引く (先頭と末尾の近くはすべて)。
    let step = (boundaries.len() / 256).max(1);
    let sampled = boundaries
        .iter()
        .enumerate()
        .filter(|&(k, _)| k < 64 || k + 64 >= boundaries.len() || k % step == 0)
        .map(|(_, &i)| i);
    for i in sampled {
        out.push((format!("truncated to {i}"), json[..i].to_owned()));
        let rest = &json[i..];
        let next = rest.chars().next().map_or(0, char::len_utf8);
        for c in ["\"", "{", "]", "0", "-", "e", "9", ",", "null"] {
            out.push((
                format!("byte {i} replaced with {c}"),
                format!("{}{c}{}", &json[..i], &rest[next..]),
            ));
        }
    }
    // 数をすべて巨大・極小・負・整数の範囲外のものにする (f32 で無限大になる値を含む)。
    for replacement in ["1e39", "-1e39", "1e-50", "-0", "4294967296", "-1", "1e999"] {
        out.push((
            format!("numbers replaced with {replacement}"),
            replace_numbers(json, replacement),
        ));
    }
    out
}

/// JSON の数をすべて `replacement` にする (文字列の中は変えない)。
fn replace_numbers(json: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = json.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
        } else if c == '-' || c.is_ascii_digit() {
            while chars
                .peek()
                .is_some_and(|n| n.is_ascii_digit() || matches!(n, '.' | 'e' | 'E' | '+' | '-'))
            {
                chars.next();
            }
            out.push_str(replacement);
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn broken_json_does_not_panic() {
    let models = [
        fixtures::pose_and_effects(),
        fixtures::glued_quads(),
        fixtures::blend_shapes(),
        fixtures::smooth_arc(),
        fixtures::dynamic_hair(),
    ];
    for model in models {
        let name = model.meta.name.clone();
        for (case, json) in broken_json(&model_to_json(&model).unwrap()) {
            let ok = catch(|| {
                if let Ok(model) = model_from_json(&json) {
                    evaluate(model, None);
                }
            });
            assert!(ok, "{name}: {case}");
        }
    }
    let physics = physics_to_json(&fixtures::dynamic_hair_settings()).unwrap();
    for (case, json) in broken_json(&physics) {
        let ok = catch(|| {
            if let Ok(settings) = physics_from_json(&json) {
                evaluate(fixtures::dynamic_hair(), Some(settings));
            }
        });
        assert!(ok, "physics: {case}");
    }
    // モーション・表情・モジュールは `assets/compat` のバンドルとモジュールから取る。
    let bundle = read_bundle(
        &compat_files("ugoe")
            .into_iter()
            .find(|(n, _)| n == "dev/pose-and-effects.ugoe")
            .unwrap()
            .1,
    )
    .unwrap();
    for motion in &bundle.motions {
        for (case, json) in broken_json(&serde_json::to_string(motion).unwrap()) {
            assert!(catch(|| drop(motion_from_json(&json))), "motion: {case}");
        }
    }
    for expression in &bundle.expressions {
        for (case, json) in broken_json(&serde_json::to_string(expression).unwrap()) {
            assert!(
                catch(|| drop(expression_from_json(&json))),
                "expression: {case}"
            );
        }
    }
    for (name, bytes) in compat_files("ugoemod")
        .into_iter()
        .filter(|(n, _)| n.starts_with("dev/"))
    {
        let module = read_module(&bytes).unwrap().module;
        for (case, json) in broken_json(&serde_json::to_string(&module).unwrap()) {
            assert!(catch(|| drop(module_from_json(&json))), "{name}: {case}");
        }
    }
}

/// 深い入れ子は再帰の上限で失敗する (スタックを使い切らない)。
#[test]
fn deeply_nested_json_is_rejected() {
    for open in ["[", "{\"a\":"] {
        let json = open.repeat(100_000);
        assert!(model_from_json(&json).is_err());
        assert!(physics_from_json(&json).is_err());
        assert!(motion_from_json(&json).is_err());
        assert!(expression_from_json(&json).is_err());
        assert!(module_from_json(&json).is_err());
    }
}

/// fuzz (`fuzz/`、`docs/testing.md#fuzz`) で見つかった入力を最小にしたもの。直した後も決まった入力として残す。
mod found_by_fuzzing {
    use super::catch;
    use ugoe_format::TextureFormat;
    use ugoe_format::image::{ImageError, ImageLimits, decode_image_premultiplied};

    /// パレットの PNG で、PLTE の長さが 3 の倍数でない (4 バイト)。`png` 0.18 はこれを受け入れ、
    /// 展開の途中でパニックしていた (T0188)。
    const PLTE_NOT_MULTIPLE_OF_3: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x03, 0x00, 0x00, 0x00, 0x28,
        0xcb, 0x34, 0xbb, 0x00, 0x00, 0x00, 0x04, 0x50, 0x4c, 0x54, 0x45, 0xff, 0x00, 0x00, 0x00,
        0x6a, 0xa6, 0xa8, 0x8b, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63,
        0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x48, 0xaf, 0xa4, 0x71, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn png_palette_of_a_length_not_a_multiple_of_3_is_an_error() {
        let mut result = None;
        let ok = catch(|| {
            result = Some(decode_image_premultiplied(
                TextureFormat::Png,
                PLTE_NOT_MULTIPLE_OF_3,
                ImageLimits::default(),
            ));
        });
        assert!(ok, "panicked");
        let err = result.unwrap().unwrap_err();
        assert!(matches!(err, ImageError::Decode(_)), "{err}");
    }
}
