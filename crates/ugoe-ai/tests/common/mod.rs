//! 振り向きの提案のテストと例 (`examples/depth_turn.rs`) で共有する顔のモデル。
//!
//! キャンバスは 256×256。顔のワープ `Face` (4×4 セル、`(40,24)`〜`(216,232)`、軸なし) の子に、
//! 後ろ髪・顔・目・鼻・口・前髪のメッシュを置く。メッシュの座標はワープの UV 空間
//! (キャンバス座標 `(x, y)` は `((x - 40) / 176, (y - 24) / 208)`)。
//! 録画 `tests/cassettes/depth_face.json` の深度マップは、この配置に合わせて作ったもの。

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ugoe_ai::classify::Label;
use ugoe_ai::transport::Cassette;
use ugoe_core::{Rgb, Vec2};
use ugoe_model::deformer::{WarpDeformer, WarpKeyform};
use ugoe_model::mesh::{BlendMode, MaskMode, MeshKeyform};
use ugoe_model::{Deformer, DeformerKind, KeyformGrid, Mesh, MeshId, Model, fixtures};

/// ワープの外接矩形 (キャンバス座標)。
pub const WARP_MIN: Vec2 = Vec2::new(40.0, 24.0);
pub const WARP_SIZE: Vec2 = Vec2::new(176.0, 208.0);
/// 顔の楕円 (中心と半径、キャンバス座標)。
pub const FACE_CENTER: Vec2 = Vec2::new(128.0, 136.0);
pub const FACE_RADIUS: Vec2 = Vec2::new(72.0, 88.0);
/// 後ろ髪の楕円。
pub const HAIR_CENTER: Vec2 = Vec2::new(128.0, 124.0);
pub const HAIR_RADIUS: Vec2 = Vec2::new(100.0, 100.0);

pub fn face_model() -> Model {
    let mut model = fixtures::keyform_inference();
    model.meta.name = "depth_turn".into();
    model.meta.canvas.width = 256.0;
    model.meta.canvas.height = 256.0;
    model.deformers = vec![face_warp()];
    model.meshes = vec![
        mesh(
            "HairBack",
            ellipse(HAIR_CENTER, HAIR_RADIUS),
            Rgb::new(0.35, 0.22, 0.15),
            50.0,
        ),
        mesh(
            "Skin",
            ellipse(FACE_CENTER, FACE_RADIUS),
            Rgb::new(0.96, 0.8, 0.68),
            100.0,
        ),
        mesh(
            "EyeL",
            rect(84.0, 118.0, 116.0, 138.0, 1),
            Rgb::new(0.2, 0.3, 0.7),
            500.0,
        ),
        mesh(
            "EyeR",
            rect(140.0, 118.0, 172.0, 138.0, 1),
            Rgb::new(0.2, 0.3, 0.7),
            500.0,
        ),
        mesh(
            "Nose",
            rect(122.0, 140.0, 134.0, 168.0, 1),
            Rgb::new(0.85, 0.35, 0.3),
            510.0,
        ),
        mesh(
            "Mouth",
            rect(106.0, 186.0, 150.0, 196.0, 2),
            Rgb::new(0.7, 0.15, 0.2),
            500.0,
        ),
        mesh(
            "Bangs",
            rect(70.0, 44.0, 186.0, 96.0, 4),
            Rgb::new(0.45, 0.3, 0.2),
            600.0,
        ),
    ];
    model
}

/// メッシュの意味ラベル。
pub fn face_labels() -> BTreeMap<MeshId, Label> {
    [
        ("HairBack", Label::HairBack),
        ("Skin", Label::Face),
        ("EyeL", Label::Eye),
        ("EyeR", Label::Eye),
        ("Nose", Label::Nose),
        ("Mouth", Label::Mouth),
        ("Bangs", Label::HairFront),
    ]
    .into_iter()
    .map(|(m, l)| (MeshId::from(m), l))
    .collect()
}

/// PNG の署名だけを持つ小さなバイト列 (推定器は中身を見ない。録画の本文と長さが一致する)。
pub fn png() -> Vec<u8> {
    b"\x89PNG\r\n\x1a\nIHDR".to_vec()
}

pub fn cassette(name: &str) -> Cassette {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cassettes")
        .join(name);
    Cassette::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// 4×4 の顔のワープ (軸なし・深度なし)。
fn face_warp() -> Deformer {
    let n = 4u16;
    let mut points = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            let t = Vec2::new(f32::from(x), f32::from(y)) / f32::from(n);
            points.push(WARP_MIN + WARP_SIZE * t);
        }
    }
    Deformer {
        id: "Face".into(),
        name: "Face".into(),
        part: "Root".into(),
        parent: None,
        kind: DeformerKind::Warp(WarpDeformer {
            rows: n,
            cols: n,
            keyforms: KeyformGrid::constant(WarpKeyform {
                points,
                opacity: 1.0,
            }),
            depth: None,
        }),
        mirror: None,
        mirror_axis: None,
    }
}

fn to_uv(p: Vec2) -> Vec2 {
    (p - WARP_MIN) / WARP_SIZE
}

/// 楕円を中心からの扇で埋めたメッシュ (キャンバス座標で指定し、UV 空間に直す)。
fn ellipse(center: Vec2, radius: Vec2) -> (Vec<Vec2>, Vec<u16>) {
    const N: u16 = 32;
    let mut positions = vec![to_uv(center)];
    for i in 0..N {
        let a = std::f32::consts::TAU * f32::from(i) / f32::from(N);
        positions.push(to_uv(center + radius * Vec2::new(a.cos(), a.sin())));
    }
    let indices = (0..N).flat_map(|i| [0, 1 + i, 1 + (i + 1) % N]).collect();
    (positions, indices)
}

/// 矩形を `n`×`n` に分けた格子のメッシュ (キャンバス座標で指定)。
fn rect(x0: f32, y0: f32, x1: f32, y1: f32, n: u16) -> (Vec<Vec2>, Vec<u16>) {
    let mut positions = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            let t = Vec2::new(f32::from(x), f32::from(y)) / f32::from(n);
            positions.push(to_uv(Vec2::new(x0 + (x1 - x0) * t.x, y0 + (y1 - y0) * t.y)));
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
        lit: true,
    }
}
