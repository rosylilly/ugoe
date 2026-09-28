//! デフォーマのギズモ用の問い合わせ: 今のパラメータ値でのデフォーマのキャンバス座標の形。GPU には触れない。
//!
//! ランタイムはデフォーマのワールド変換を公開しないので、評価用のメッシュを足したモデルの複製を
//! 別のインスタンスで評価して求める (エディタ T0607。式は `docs/spec/evaluation.md` の 3.)。
//!
//! - ワープ: 制御点を頂点とし、ワープと同じキーフォーム格子を持つメッシュを **親の下** に置く。
//!   出力はキャンバス座標の制御点。
//! - 回転デフォーマ: `(0,0)`・`(1,0)`・`(0,1)` を頂点とするメッシュを **子** に置く。
//!   出力はワールド変換 (アフィン) の原点と軸。

use ugoe::core::{Rgb, Vec2};
use ugoe::model::deformer::DeformerKind;
use ugoe::model::mesh::MeshKeyform;
use ugoe::model::{BlendMode, KeyformGrid, MaskMode, Mesh, MeshId, Model, TextureRef};
use ugoe::runtime::{CompiledModel, ModelInstance, ParamIndex, ParameterValues};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Warp,
    Rotation,
}

#[derive(Debug)]
struct Entry {
    id: String,
    kind: Kind,
    /// 評価用のメッシュの、複製したモデルでの添字。
    probe: usize,
}

/// デフォーマのキャンバス座標の形を評価するための、モデルの複製とそのインスタンス。
#[derive(Debug)]
pub struct DeformerProbe {
    instance: ModelInstance,
    entries: Vec<Entry>,
}

impl DeformerProbe {
    /// `model` から作る。評価用のメッシュを足したモデルがコンパイルできなければ `None`。
    pub fn new(model: &Model) -> Option<Self> {
        let mut probed = model.clone();
        if probed.textures.is_empty() {
            probed.textures.push(TextureRef {
                name: "probe".into(),
                width: 1,
                height: 1,
            });
        }
        let mut entries = Vec::new();
        for d in &model.deformers {
            let (kind, parent, keyforms) = match &d.kind {
                DeformerKind::Warp(w) => (
                    Kind::Warp,
                    d.parent.clone(),
                    KeyformGrid::new(
                        w.keyforms.bindings.clone(),
                        w.keyforms
                            .keyforms
                            .iter()
                            .map(|k| keyform(k.points.clone()))
                            .collect(),
                    ),
                ),
                DeformerKind::Rotation(_) => (
                    Kind::Rotation,
                    Some(d.id.clone()),
                    KeyformGrid::constant(keyform(vec![Vec2::ZERO, Vec2::X, Vec2::Y])),
                ),
                DeformerKind::Unknown => continue,
            };
            let probe = probed.meshes.len();
            let n = keyforms.keyforms.first().map_or(0, |k| k.positions.len());
            probed.meshes.push(Mesh {
                id: MeshId::new(format!("__ugoe_gizmo_probe_{probe}")),
                name: String::new(),
                part: d.part.clone(),
                parent,
                texture: 0,
                uvs: vec![Vec2::ZERO; n],
                indices: Vec::new(),
                blend: BlendMode::Normal,
                culling: false,
                masks: Vec::new(),
                mask_mode: MaskMode::Union,
                mask_only: true,
                keyforms,
                mirror: None,
                mirror_axis: None,
                depth: None,
            });
            entries.push(Entry {
                id: d.id.to_string(),
                kind,
                probe,
            });
        }
        let compiled = CompiledModel::compile(probed).ok()?;
        Some(Self {
            instance: ModelInstance::new(compiled),
            entries,
        })
    }

    /// ワープと回転デフォーマの ID (モデルの定義順。未知の種類は含まない)。
    pub fn ids(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.id.clone()).collect()
    }

    /// パラメータ値 (同じモデルのインスタンスのもの) を写して評価し直す。
    pub fn update(&mut self, values: &ParameterValues) {
        let params = self.instance.parameters_mut();
        for (i, &v) in values.as_slice().iter().enumerate().take(params.len()) {
            params.set(ParamIndex(i as u32), v);
        }
        self.instance.update();
    }

    fn entry(&self, id: &str, kind: Kind) -> Option<&[Vec2]> {
        let e = self.entries.iter().find(|e| e.id == id && e.kind == kind)?;
        Some(&self.instance.drawables()[e.probe].positions)
    }

    /// ワープのキャンバス座標の制御点 `[x0, y0, x1, y1, ...]` (行優先、x が速い)。ワープでなければ `None`。
    pub fn warp_points(&self, id: &str) -> Option<Vec<f32>> {
        let p = self.entry(id, Kind::Warp)?;
        Some(p.iter().flat_map(|p| [p.x, p.y]).collect())
    }

    /// 回転デフォーマのワールド変換 `[ox, oy, ax, ay, bx, by]`: 原点 `o` と、局所座標の x 軸・y 軸の
    /// 単位ベクトルの像 `a`・`b` (`world(p) = o + a * p.x + b * p.y`)。回転デフォーマでなければ `None`。
    pub fn rotation_frame(&self, id: &str) -> Option<Vec<f32>> {
        let p = self.entry(id, Kind::Rotation)?;
        let (o, a, b) = (p[0], p[1] - p[0], p[2] - p[0]);
        Some(vec![o.x, o.y, a.x, a.y, b.x, b.y])
    }
}

fn keyform(positions: Vec<Vec2>) -> MeshKeyform {
    MeshKeyform {
        positions,
        opacity: 1.0,
        draw_order: 0.0,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
        mask_strength: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use ugoe::model::fixtures;

    use super::*;

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    #[test]
    fn warp_points_follow_parameters() {
        let model = fixtures::warp_grid();
        let mut probe = DeformerProbe::new(&model).unwrap();
        assert_eq!(probe.ids(), vec!["Warp"]);
        // X = 0 は 64..192 の正方形の 2×2 セル。
        let p = probe.warp_points("Warp").unwrap();
        assert_eq!(p.len(), 18);
        assert!(close(&p[..4], &[64.0, 64.0, 128.0, 64.0]));
        // X = 1 は上の行ほど右へずれるせん断。
        let mut instance = ModelInstance::new(CompiledModel::compile(model).unwrap());
        let x = instance.parameters().index_of("X").unwrap();
        instance.parameters_mut().set(x, 1.0);
        probe.update(instance.parameters());
        let p = probe.warp_points("Warp").unwrap();
        assert!(close(&p[..2], &[128.0, 64.0]));
        assert!(probe.rotation_frame("Warp").is_none());
    }

    #[test]
    fn rotation_frame_composes_parents() {
        let model = fixtures::rotation_chain();
        let mut probe = DeformerProbe::new(&model).unwrap();
        assert_eq!(probe.ids(), vec!["Arm", "Hand", "Cloth"]);
        // Angle = 0: Arm は (128,128) の恒等回転、Hand は Arm の (64,0) で拡大率 0.5。
        let hand = probe.rotation_frame("Hand").unwrap();
        assert!(close(&hand, &[192.0, 128.0, 0.5, 0.0, 0.0, 0.5]));
        // Angle = 30: Arm が時計回りに 30 度回る。
        let mut instance = ModelInstance::new(CompiledModel::compile(model).unwrap());
        let angle = instance.parameters().index_of("Angle").unwrap();
        instance.parameters_mut().set(angle, 30.0);
        probe.update(instance.parameters());
        let arm = probe.rotation_frame("Arm").unwrap();
        let (s, c) = 30f32.to_radians().sin_cos();
        assert!(close(&arm, &[128.0, 128.0, c, s, -s, c]));
        // Cloth (Arm の子のワープ) の制御点も回る。
        let cloth = probe.warp_points("Cloth").unwrap();
        assert!(close(&cloth[..2], &[128.0 - 20.0 * s, 128.0 + 20.0 * c]));
    }

    #[test]
    fn model_without_textures_still_probes() {
        let mut model = fixtures::rotation_under_warp();
        model.meshes.clear();
        model.textures.clear();
        let probe = DeformerProbe::new(&model).unwrap();
        assert!(probe.rotation_frame("Head").is_some());
    }
}
