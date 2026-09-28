//! テスト・サンプル用の小さなモデル。下流クレートのテストはここを共通の入力に使う。
//! 新しい機能をテストしたいときは、ここに関数を追加する (既存フィクスチャの挙動は変えない)。

use ugoe_core::{Rgb, Vec2};

use crate::keyform::{KeyformBinding, KeyformGrid};
use crate::mesh::{BlendMode, Mesh, MeshKeyform};
use crate::parameter::{Parameter, ParameterKind};
use crate::part::{Part, PartKeyform};
use crate::{Canvas, Model, ModelMeta, SCHEMA_VERSION, TextureRef};

/// 256×256 キャンバスに四角形メッシュ 1 枚。
/// パラメータ `X` (-1..1) で四角形が左右に 64px 平行移動する。
pub fn single_quad() -> Model {
    let quad = |dx: f32| MeshKeyform {
        positions: vec![
            Vec2::new(64.0 + dx, 64.0),
            Vec2::new(192.0 + dx, 64.0),
            Vec2::new(192.0 + dx, 192.0),
            Vec2::new(64.0 + dx, 192.0),
        ],
        opacity: 1.0,
        draw_order: 500.0,
        multiply_color: Rgb::WHITE,
        screen_color: Rgb::BLACK,
    };
    Model {
        schema_version: SCHEMA_VERSION,
        meta: ModelMeta {
            name: "single_quad".into(),
            canvas: Canvas {
                width: 256.0,
                height: 256.0,
                origin: Vec2::new(128.0, 128.0),
            },
        },
        parameters: vec![Parameter {
            id: "X".into(),
            name: "X".into(),
            min: -1.0,
            max: 1.0,
            default: 0.0,
            kind: ParameterKind::Normal,
            mirror: None,
            group: None,
        }],
        parameter_groups: vec![],
        parts: vec![Part {
            id: "Root".into(),
            name: "Root".into(),
            parent: None,
            keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
        }],
        deformers: vec![],
        meshes: vec![Mesh {
            id: "Quad".into(),
            name: "Quad".into(),
            part: "Root".into(),
            parent: None,
            texture: 0,
            uvs: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            blend: BlendMode::Normal,
            culling: false,
            masks: vec![],
            invert_mask: false,
            keyforms: KeyformGrid {
                bindings: vec![KeyformBinding {
                    parameter: "X".into(),
                    keys: vec![-1.0, 0.0, 1.0],
                }],
                keyforms: vec![quad(-64.0), quad(0.0), quad(64.0)],
            },
        }],
        glues: vec![],
        textures: vec![TextureRef {
            name: "white".into(),
            width: 1,
            height: 1,
        }],
        hit_areas: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quad_is_valid() {
        assert_eq!(crate::validate::validate(&single_quad()), vec![]);
    }

    #[test]
    fn single_quad_roundtrips_json() {
        let model = single_quad();
        let json = serde_json::to_string(&model).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
    }
}
