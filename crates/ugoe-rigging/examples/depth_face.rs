//! 深度からの回転とミラーで振り向きの顔を作り、モデルの JSON を書き出す (目視確認用)。
//!
//! ```sh
//! cargo run -q -p ugoe-rigging --example depth_face -- /tmp/face.json
//! cargo run -q -p ugoe-cli -- render /tmp/face.json --param AngleX=30 -o /tmp/face-x30.png
//! ```
//!
//! 顔は 4×4 のワープで、制御点に球面に近い深度 (中央が手前に 48px) を持たせる。
//! 顔の中の輪郭・目・鼻・口はワープの子のメッシュで、`multiply_color` で色を付ける。
//! 右目の `AngleX` の格子は左目からミラーで作る (振り向いた先の側の目が細くなる)。

use ugoe_core::{Rgb, Vec2};
use ugoe_model::deformer::{WarpDeformer, WarpKeyform};
use ugoe_model::mesh::{BlendMode, MaskMode, MeshKeyform};
use ugoe_model::{
    Deformer, DeformerKind, DepthRotation, KeyformBinding, KeyformGrid, KeyformInference,
    KeyformOrigin, Mesh, Model, fixtures,
};
use ugoe_rigging::{
    InferScope, KeyformTarget, MirrorOptions, infer_depth_rotation, mirror_keyforms,
};

const KEYS: [f32; 5] = [-30.0, -15.0, 0.0, 15.0, 30.0];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: depth_face <output.json>")?;
    let mut model = fixtures::keyform_inference();
    model.meta.name = "depth_face".into();
    for p in &mut model.parameters {
        // AngleX は自分自身と対にする (ミラーで符号を反転したキーに対応させる)。
        if p.id == "AngleX".into() {
            p.mirror = Some(p.id.clone());
        }
    }
    model.deformers = vec![face_warp()];
    model.meshes = vec![
        mesh(
            "Skin",
            grid_quad(0.08, 0.05, 0.92, 0.97, 8),
            Rgb::new(0.96, 0.8, 0.68),
            100.0,
        ),
        mesh(
            "EyeL",
            grid_quad(0.24, 0.36, 0.42, 0.46, 1),
            Rgb::new(0.2, 0.3, 0.7),
            500.0,
        ),
        mesh(
            "EyeR",
            grid_quad(0.58, 0.36, 0.76, 0.46, 1),
            Rgb::new(0.2, 0.3, 0.7),
            500.0,
        ),
        mesh(
            "Nose",
            grid_quad(0.46, 0.5, 0.54, 0.64, 1),
            Rgb::new(0.85, 0.35, 0.3),
            510.0,
        ),
        mesh(
            "Mouth",
            grid_quad(0.36, 0.74, 0.64, 0.79, 2),
            Rgb::new(0.7, 0.15, 0.2),
            500.0,
        ),
    ];
    pair_eyes(&mut model);

    let face = KeyformTarget::Deformer("Face".into());
    let report = infer_depth_rotation(&mut model, &face, InferScope::AllDerived)?;
    eprintln!("face warp: {} derived cells", report.written.len());
    let report = mirror_keyforms(
        &mut model,
        &KeyformTarget::Mesh("EyeL".into()),
        &KeyformTarget::Mesh("EyeR".into()),
        &MirrorOptions::default(),
    )?;
    eprintln!("right eye: {} mirrored cells", report.written.len());

    std::fs::write(&path, serde_json::to_string_pretty(&model)?)?;
    Ok(())
}

fn binding(parameter: &str) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: KEYS.to_vec(),
    }
}

/// 4×4 の顔のワープ。基準のセルだけが authored で、残りは深度からの回転で作る。
fn face_warp() -> Deformer {
    let n = 4u16;
    let mut points = Vec::new();
    let mut depth = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            let (u, v) = (f32::from(x) / f32::from(n), f32::from(y) / f32::from(n));
            points.push(Vec2::new(48.0 + 160.0 * u, 40.0 + 180.0 * v));
            // 中央が手前に 48px 出た楕円体に近い深度。
            let r2 = (2.0 * u - 1.0).powi(2) + (2.0 * v - 1.0).powi(2);
            depth.push(48.0 * (1.0 - r2 / 2.0).max(0.0).sqrt());
        }
    }
    let base = WarpKeyform {
        points,
        opacity: 1.0,
    };
    let mut keyforms = KeyformGrid::new(vec![binding("AngleX"), binding("AngleY")], vec![base; 25]);
    keyforms.origins = vec![KeyformOrigin::Derived; 25];
    keyforms.origins[12] = KeyformOrigin::Authored;
    keyforms.inference = Some(KeyformInference::DepthRotation(DepthRotation {
        angle_scale: 1.0,
        pivot: Vec2::new(128.0, 130.0),
        pivot_depth: 0.0,
        yaw: Some("AngleX".into()),
        pitch: Some("AngleY".into()),
    }));
    Deformer {
        id: "Face".into(),
        name: "Face".into(),
        part: "Root".into(),
        parent: None,
        kind: DeformerKind::Warp(WarpDeformer {
            rows: n,
            cols: n,
            keyforms,
            depth: Some(depth),
        }),
        mirror: None,
        mirror_axis: None,
    }
}

/// UV 空間の矩形を `n`×`n` に分けた格子のメッシュの頂点と三角形。
fn grid_quad(x0: f32, y0: f32, x1: f32, y1: f32, n: u16) -> (Vec<Vec2>, Vec<u16>) {
    let mut positions = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            let (u, v) = (f32::from(x) / f32::from(n), f32::from(y) / f32::from(n));
            positions.push(Vec2::new(x0 + (x1 - x0) * u, y0 + (y1 - y0) * v));
        }
    }
    let mut indices = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let i = y * (n + 1) + x;
            indices.extend([i, i + 1, i + n + 2, i, i + n + 2, i + n + 1]);
        }
    }
    (positions, indices)
}

fn mesh(id: &str, (positions, indices): (Vec<Vec2>, Vec<u16>), color: Rgb, order: f32) -> Mesh {
    Mesh {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: Some("Face".into()),
        texture: 0,
        uvs: vec![Vec2::ZERO; positions.len()],
        indices,
        blend: BlendMode::Normal,
        culling: false,
        masks: vec![],
        mask_mode: MaskMode::Union,
        mask_only: false,
        keyforms: KeyformGrid::constant(MeshKeyform {
            positions,
            opacity: 1.0,
            draw_order: order,
            multiply_color: color,
            screen_color: Rgb::BLACK,
            mask_strength: 1.0,
        }),
        mirror: None,
        mirror_axis: None,
        depth: None,
    }
}

/// 目を対にし、左目に `AngleX` の格子を手で作る (右目はミラーで作る)。
fn pair_eyes(model: &mut Model) {
    for m in &mut model.meshes {
        let other = match m.id.as_str() {
            "EyeL" => "EyeR",
            "EyeR" => "EyeL",
            _ => continue,
        };
        m.mirror = Some(other.into());
        let base = m.keyforms.keyforms[0].clone();
        let keyforms = KEYS
            .iter()
            .map(|&k| {
                let mut kf = base.clone();
                // 左を向く (AngleX 負) と、奥へ回る左目 (画面の左) は少し細くなる。
                let squash = 1.0 - (-k / 30.0).max(0.0) * 0.3;
                let cx = kf.positions.iter().map(|p| p.x).sum::<f32>() / kf.positions.len() as f32;
                for p in &mut kf.positions {
                    p.x = cx + (p.x - cx) * squash;
                }
                kf
            })
            .collect();
        m.keyforms = KeyformGrid::new(vec![binding("AngleX")], keyforms);
        m.keyforms.origins = if m.id == "EyeL".into() {
            vec![KeyformOrigin::Authored; 5]
        } else {
            let mut o = vec![KeyformOrigin::Derived; 5];
            o[2] = KeyformOrigin::Authored;
            o
        };
    }
}
