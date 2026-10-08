//! 参照とバインディングの結果を比べる。
//!
//! - 評価の結果: 公開仕様の評価の適合の基準 (`spec/evaluation.md#conformance`。
//!   `docs/spec/evaluation.md` と `spec/examples/README.md#tolerances` と同じ値)。
//! - 画像: 描画の互換の基準 (`spec/rendering.md#conformance`、`docs/spec/rendering.md#互換の基準`)。
//!   すべてのチャンネルの差が 3/255 以下の画素が 99% 以上。

use crate::frame::{Frame, MeshFrame, MotionEventFrame, Rgb};

/// 頂点の位置の許容誤差 (キャンバスの px)。
pub const POSITION_TOLERANCE: f32 = 0.01;
/// 不透明度・乗算色・スクリーン色・マスクの効き具合の許容誤差。
pub const VALUE_TOLERANCE: f32 = 1e-4;
/// 描画順の値の許容誤差。描画の順番は、参照の値の差がこれ以下のメッシュの間では問わない。
pub const DRAW_ORDER_TOLERANCE: f32 = 1e-3;
/// `visible` を問わない不透明度 (参照の不透明度がこれ以下なら、どちらの値でもよい)。
pub const INVISIBLE_OPACITY: f32 = 1e-4;
/// 画素の各チャンネルの差の許容 (8 ビットの値)。
pub const PIXEL_TOLERANCE: u8 = 3;
/// 許容に収まる必要のある画素の割合。
pub const PIXEL_RATIO: f64 = 0.99;

/// 評価の結果を比べ、違いを人が読める文で返す (空なら一致)。
///
/// 比べるのは `actual` にある値だけ。`actual` のメッシュの集合は参照と同じでなければならない。
pub fn frames(reference: &Frame, actual: &Frame) -> Vec<String> {
    let mut errors = Vec::new();
    for id in reference.meshes.keys() {
        if !actual.meshes.contains_key(id) {
            errors.push(format!("mesh {id}: missing"));
        }
    }
    for (id, mesh) in &actual.meshes {
        match reference.meshes.get(id) {
            Some(expected) => mesh_errors(id, expected, mesh, &mut errors),
            None => errors.push(format!("mesh {id}: not in the reference")),
        }
    }
    render_order(reference, actual, &mut errors);
    motion_events(reference, actual, &mut errors);
    textures(reference, actual, &mut errors);
    changed(reference, actual, &mut errors);
    errors
}

/// 各 `update` の後に見た目が変わったか (T1404)。両方にあるときだけ比べ、並びが完全に一致すること
/// (書くのは `changed` を読めるバインディングだけ。`changed` は C ABI と Unity、`changed_after_load` は Godot。T1405)。
fn changed(reference: &Frame, actual: &Frame, errors: &mut Vec<String>) {
    changed_list(
        "changed",
        reference.changed.as_deref(),
        actual.changed.as_deref(),
        errors,
    );
    changed_list(
        "changed_after_load",
        reference.changed_after_load.as_deref(),
        actual.changed_after_load.as_deref(),
        errors,
    );
}

fn changed_list(
    key: &str,
    expected: Option<&[bool]>,
    changed: Option<&[bool]>,
    errors: &mut Vec<String>,
) {
    let (Some(expected), Some(changed)) = (expected, changed) else {
        return;
    };
    if expected != changed {
        let show = |list: &[bool]| {
            list.iter()
                .map(|&c| if c { '1' } else { '0' })
                .collect::<String>()
        };
        errors.push(format!(
            "{key}: {}, expected {}",
            show(changed),
            show(expected)
        ));
    }
}

/// テクスチャのデコードの結果 (T0466)。両方にあるときだけ比べ、番号ごとに完全に一致すること
/// (書くのは画像をデコードするバインディングだけ。今は Unity)。
fn textures(reference: &Frame, actual: &Frame, errors: &mut Vec<String>) {
    let (Some(expected), Some(textures)) = (&reference.textures, &actual.textures) else {
        return;
    };
    if expected.len() != textures.len() {
        errors.push(format!(
            "textures: {} textures, expected {}",
            textures.len(),
            expected.len()
        ));
        return;
    }
    for (i, (e, a)) in expected.iter().zip(textures).enumerate() {
        if e != a {
            errors.push(format!("texture {i}: {a:?}, expected {e:?}"));
        }
    }
}

/// 発火したモーションのイベントと出どころ (T0462)。参照にあればバインディングも書くこと。値・名前・番号・`update` の番号は
/// 一致し、モーションの中の時刻は [`VALUE_TOLERANCE`] まで。
fn motion_events(reference: &Frame, actual: &Frame, errors: &mut Vec<String>) {
    let Some(expected) = &reference.motion_events else {
        return;
    };
    let Some(events) = &actual.motion_events else {
        errors.push("motion_events: missing".to_owned());
        return;
    };
    let same = |e: &MotionEventFrame, a: &MotionEventFrame| {
        e.step == a.step
            && e.value == a.value
            && e.motion == a.motion
            && e.number == a.number
            && !exceeds((e.time - a.time).abs(), VALUE_TOLERANCE)
    };
    if events.len() != expected.len() || !expected.iter().zip(events).all(|(e, a)| same(e, a)) {
        let show = |list: &[MotionEventFrame]| {
            list.iter()
                .map(|e| {
                    format!(
                        "{}:{}@{}({}#{})",
                        e.step, e.value, e.time, e.motion, e.number
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        errors.push(format!(
            "motion_events: [{}], expected [{}]",
            show(events),
            show(expected)
        ));
    }
}

fn mesh_errors(id: &str, expected: &MeshFrame, actual: &MeshFrame, errors: &mut Vec<String>) {
    if expected.positions.len() != actual.positions.len() {
        errors.push(format!(
            "mesh {id}: {} vertices, expected {}",
            actual.positions.len(),
            expected.positions.len()
        ));
    } else {
        let worst = expected
            .positions
            .iter()
            .zip(&actual.positions)
            .enumerate()
            .map(|(i, (e, a))| (i, (e[0] - a[0]).abs().max((e[1] - a[1]).abs())))
            .filter(|(_, d)| exceeds(*d, POSITION_TOLERANCE))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, d)) = worst {
            errors.push(format!(
                "mesh {id}: vertex {i} is {:?}, expected {:?} (off by {d})",
                actual.positions[i], expected.positions[i]
            ));
        }
    }
    let values = [
        ("opacity", expected.opacity, actual.opacity),
        (
            "mask_strength",
            expected.mask_strength,
            actual.mask_strength,
        ),
    ];
    for (name, e, a) in values {
        if let (Some(e), Some(a)) = (e, a)
            && exceeds((e - a).abs(), VALUE_TOLERANCE)
        {
            errors.push(format!("mesh {id}: {name} is {a}, expected {e}"));
        }
    }
    if let (Some(e), Some(a)) = (expected.draw_order, actual.draw_order)
        && exceeds((e - a).abs(), DRAW_ORDER_TOLERANCE)
    {
        errors.push(format!("mesh {id}: draw_order is {a}, expected {e}"));
    }
    let colors = [
        (
            "multiply_color",
            expected.multiply_color,
            actual.multiply_color,
        ),
        ("screen_color", expected.screen_color, actual.screen_color),
    ];
    for (name, e, a) in colors {
        if let (Some(e), Some(a)) = (e, a)
            && !color_matches(e, a)
        {
            errors.push(format!("mesh {id}: {name} is {a:?}, expected {e:?}"));
        }
    }
    if let (Some(e), Some(a)) = (expected.visible, actual.visible) {
        let either = expected.opacity.is_some_and(|o| o <= INVISIBLE_OPACITY);
        if e != a && !either {
            errors.push(format!("mesh {id}: visible is {a}, expected {e}"));
        }
    }
}

/// 差が許容誤差を超えるか (NaN も超えたとみなす)。
fn exceeds(diff: f32, tolerance: f32) -> bool {
    diff.is_nan() || diff > tolerance
}

fn color_matches(e: Rgb, a: Rgb) -> bool {
    [(e.r, a.r), (e.g, a.g), (e.b, a.b)]
        .iter()
        .all(|(e, a)| (e - a).abs() <= VALUE_TOLERANCE)
}

/// 描画の順番。参照で描画順の値の差が許容誤差以下のメッシュどうしは、どちらが先でもよい。
fn render_order(reference: &Frame, actual: &Frame, errors: &mut Vec<String>) {
    let mut sorted_ref = reference.render_order.clone();
    let mut sorted_actual = actual.render_order.clone();
    sorted_ref.sort();
    sorted_actual.sort();
    if sorted_ref != sorted_actual {
        errors.push(format!(
            "render_order has {:?}, expected the meshes {:?}",
            actual.render_order, reference.render_order
        ));
        return;
    }
    let position = |id: &str| reference.render_order.iter().position(|r| r == id);
    let order = |id: &str| reference.meshes.get(id).and_then(|m| m.draw_order);
    for (i, a) in actual.render_order.iter().enumerate() {
        for b in &actual.render_order[i + 1..] {
            // 実際は a が b より奥。参照で b が a より奥なら、値の差が許容誤差以下のときだけ許す。
            if position(b) < position(a) {
                let tie = match (order(a), order(b)) {
                    (Some(x), Some(y)) => (x - y).abs() <= DRAW_ORDER_TOLERANCE,
                    _ => false,
                };
                if !tie {
                    errors.push(format!(
                        "render_order draws {a} behind {b}, expected the reverse \
                         (actual {:?}, expected {:?})",
                        actual.render_order, reference.render_order
                    ));
                    return;
                }
            }
        }
    }
}

/// 画像の比べ合わせの結果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageComparison {
    /// 許容に収まった画素の割合。
    pub matching: f64,
    /// いちばん大きいチャンネルの差。
    pub worst: u8,
}

impl ImageComparison {
    pub fn passes(&self) -> bool {
        self.matching >= PIXEL_RATIO
    }
}

/// 同じ大きさの RGBA8 の画像を比べる。大きさが違えば `None`。
pub fn images(reference: &[u8], actual: &[u8]) -> Option<ImageComparison> {
    if reference.len() != actual.len() || !reference.len().is_multiple_of(4) {
        return None;
    }
    let total = reference.len() / 4;
    let mut bad = 0usize;
    let mut worst = 0u8;
    let (reference, _) = reference.as_chunks::<4>();
    let (actual, _) = actual.as_chunks::<4>();
    for (r, a) in reference.iter().zip(actual) {
        let diff = r
            .iter()
            .zip(a)
            .map(|(r, a)| r.abs_diff(*a))
            .max()
            .unwrap_or(0);
        worst = worst.max(diff);
        if diff > PIXEL_TOLERANCE {
            bad += 1;
        }
    }
    let matching = if total == 0 {
        1.0
    } else {
        1.0 - bad as f64 / total as f64
    };
    Some(ImageComparison { matching, worst })
}

/// 違う画素を赤、合う画素を参照の灰色の薄い版にした画像 (比べ合わせに落ちたときの手がかり)。
pub fn diff_image(reference: &[u8], actual: &[u8]) -> Vec<u8> {
    let (reference, _) = reference.as_chunks::<4>();
    let (actual, _) = actual.as_chunks::<4>();
    let mut out = Vec::with_capacity(reference.len() * 4);
    for (r, a) in reference.iter().zip(actual) {
        let diff = r
            .iter()
            .zip(a)
            .map(|(r, a)| r.abs_diff(*a))
            .max()
            .unwrap_or(0);
        if diff > PIXEL_TOLERANCE {
            out.extend_from_slice(&[255, 0, 0, 255]);
        } else {
            let luma = ((u32::from(r[0]) + u32::from(r[1]) + u32::from(r[2])) / 3) as u8;
            let faded = 192 + luma / 4;
            out.extend_from_slice(&[faded, faded, faded, 255]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::frame::TextureFrame;

    fn mesh(x: f32, draw_order: f32, opacity: f32) -> MeshFrame {
        MeshFrame {
            positions: vec![[x, 0.0], [x + 1.0, 1.0]],
            opacity: Some(opacity),
            draw_order: Some(draw_order),
            multiply_color: Some(Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            }),
            screen_color: None,
            mask_strength: Some(1.0),
            visible: Some(opacity > 0.0),
        }
    }

    fn frame(meshes: &[(&str, MeshFrame)], order: &[&str]) -> Frame {
        Frame {
            meshes: meshes
                .iter()
                .map(|(id, m)| ((*id).to_owned(), m.clone()))
                .collect::<BTreeMap<_, _>>(),
            render_order: order.iter().map(|s| (*s).to_owned()).collect(),
            motion_events: None,
            textures: None,
            changed: None,
            changed_after_load: None,
        }
    }

    fn event(step: usize, value: &str, time: f32, motion: &str, number: u64) -> MotionEventFrame {
        MotionEventFrame {
            step,
            value: value.to_owned(),
            time,
            motion: motion.to_owned(),
            number,
        }
    }

    #[test]
    fn motion_events_must_match_the_reference() {
        let mut reference = frame(&[("A", mesh(0.0, 500.0, 1.0))], &["A"]);
        reference.motion_events = Some(vec![
            event(0, "start", 0.0, "idle", 1),
            event(4, "half", 0.5, "idle", 1),
        ]);
        let mut actual = reference.clone();
        actual.motion_events.as_mut().unwrap()[1].time += 5e-5;
        assert!(frames(&reference, &actual).is_empty());
        // 出どころの番号が違う
        actual.motion_events.as_mut().unwrap()[1].number = 2;
        let errors = frames(&reference, &actual);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("4:half@0.50005(idle#2)"), "{errors:?}");
        // 書かなかった
        actual.motion_events = None;
        assert_eq!(frames(&reference, &actual), ["motion_events: missing"]);
        // 参照に無ければ比べない
        reference.motion_events = None;
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn textures_must_match_when_both_have_them() {
        let texture = |color: &str| TextureFrame {
            color: Some(color.to_owned()),
            normal_map: None,
        };
        let mut reference = frame(&[("A", mesh(0.0, 500.0, 1.0))], &["A"]);
        reference.textures = Some(vec![texture("4x4:00")]);
        let mut actual = reference.clone();
        assert!(frames(&reference, &actual).is_empty());
        actual.textures = Some(vec![texture("4x4:01")]);
        assert_eq!(frames(&reference, &actual).len(), 1);
        actual.textures = Some(Vec::new());
        assert_eq!(
            frames(&reference, &actual),
            ["textures: 0 textures, expected 1"]
        );
        // 書かないバインディング (C ABI の実行器・wasm・Godot) は比べない
        actual.textures = None;
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn changed_must_match_when_both_have_it() {
        let mut reference = frame(&[("A", mesh(0.0, 500.0, 1.0))], &["A"]);
        reference.changed = Some(vec![true, false, true]);
        let mut actual = reference.clone();
        assert!(frames(&reference, &actual).is_empty());
        actual.changed = Some(vec![true, true, true]);
        assert_eq!(frames(&reference, &actual), ["changed: 111, expected 101"]);
        // 書かないバインディング (wasm・Godot) は比べない
        actual.changed = None;
        assert!(frames(&reference, &actual).is_empty());
        // Godot は読み込みの直後に評価した列を書く (T1405)
        reference.changed_after_load = Some(vec![false, false, true]);
        actual.changed_after_load = Some(vec![true, false, true]);
        assert_eq!(
            frames(&reference, &actual),
            ["changed_after_load: 101, expected 001"]
        );
        actual.changed_after_load = reference.changed_after_load.clone();
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn frames_within_tolerance_match() {
        let reference = frame(
            &[("A", mesh(0.0, 500.0, 1.0)), ("B", mesh(5.0, 600.0, 1.0))],
            &["A", "B"],
        );
        let mut actual = reference.clone();
        actual.meshes.get_mut("A").unwrap().positions[1][0] += 0.009;
        actual.meshes.get_mut("B").unwrap().opacity = Some(1.0 - 5e-5);
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn frames_report_differences() {
        let reference = frame(
            &[("A", mesh(0.0, 500.0, 1.0)), ("B", mesh(5.0, 600.0, 1.0))],
            &["A", "B"],
        );
        let mut actual = reference.clone();
        actual.meshes.get_mut("A").unwrap().positions[1][1] += 0.02;
        actual.meshes.get_mut("B").unwrap().draw_order = Some(601.0);
        actual.render_order.reverse();
        let errors = frames(&reference, &actual);
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("vertex 1"));
        assert!(errors[1].contains("draw_order"));
        assert!(errors[2].contains("render_order"));
    }

    #[test]
    fn missing_values_are_not_compared() {
        let reference = frame(&[("A", mesh(0.0, 500.0, 1.0))], &["A"]);
        let mut actual = reference.clone();
        let a = actual.meshes.get_mut("A").unwrap();
        a.opacity = None;
        a.draw_order = None;
        a.multiply_color = None;
        a.mask_strength = None;
        a.visible = None;
        assert!(frames(&reference, &actual).is_empty());
        actual.meshes.clear();
        assert_eq!(frames(&reference, &actual), ["mesh A: missing"]);
    }

    #[test]
    fn ties_in_draw_order_may_swap() {
        let reference = frame(
            &[
                ("A", mesh(0.0, 500.0, 1.0)),
                ("B", mesh(5.0, 500.0005, 1.0)),
            ],
            &["A", "B"],
        );
        let mut actual = reference.clone();
        actual.render_order.reverse();
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn visible_is_free_for_transparent_meshes() {
        let reference = frame(&[("A", mesh(0.0, 500.0, 0.0))], &["A"]);
        let mut actual = reference.clone();
        actual.meshes.get_mut("A").unwrap().visible = Some(true);
        assert!(frames(&reference, &actual).is_empty());
    }

    #[test]
    fn images_count_pixels_within_tolerance() {
        let reference = vec![100u8; 4 * 200];
        let mut actual = reference.clone();
        actual[0] = 103; // 許容に収まる
        actual[4] = 110; // 1 画素だけ外れる
        let c = images(&reference, &actual).unwrap();
        assert_eq!(c.worst, 10);
        assert!((c.matching - 0.995).abs() < 1e-9);
        assert!(c.passes());
        actual[8] = 0;
        actual[12] = 0;
        assert!(!images(&reference, &actual).unwrap().passes());
        assert_eq!(images(&reference, &actual[4..]), None);
        let diff = diff_image(&reference, &actual);
        assert_eq!(&diff[4..8], &[255, 0, 0, 255]);
        assert_ne!(&diff[0..4], &[255, 0, 0, 255]);
    }
}
