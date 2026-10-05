//! 自動メッシュ (`ugoe-mesher`) の設定と、テクスチャからのメッシュの作り直し。
//! 設計: `docs/editor.md#psd-の取り込み`、`docs/ai-integration.md#自動メッシュ`。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_mesher::{MeshError, MeshOptions};
use ugoe_model::Model;
use ugoe_model::mesh::Mesh;

use crate::document::EditError;
use crate::draw_clone;
use crate::texture::{ImageError, ImageLimits, Textures, decode_png};

/// [`ugoe_mesher::MeshOptions`] の JSON 表現。省略したフィールドは既定値になる。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(default)]
pub struct AutoMeshOptions {
    /// この値以上のアルファを中身とする (0..=255)。
    pub alpha_threshold: u8,
    /// 輪郭を外側に広げる距離 (px)。
    pub margin: f32,
    /// 輪郭上の頂点間隔の目安 (px)。
    pub edge_spacing: f32,
    /// 内部頂点の間隔の目安 (px)。0 なら内部点を置かない。
    pub interior_spacing: f32,
    /// 画素数がこれ未満の島を捨てる。
    pub min_island_area: u32,
}

impl Default for AutoMeshOptions {
    fn default() -> Self {
        MeshOptions::default().into()
    }
}

impl From<MeshOptions> for AutoMeshOptions {
    fn from(o: MeshOptions) -> Self {
        Self {
            alpha_threshold: o.alpha_threshold,
            margin: o.margin,
            edge_spacing: o.edge_spacing,
            interior_spacing: o.interior_spacing,
            min_island_area: o.min_island_area,
        }
    }
}

impl From<AutoMeshOptions> for MeshOptions {
    fn from(o: AutoMeshOptions) -> Self {
        Self {
            alpha_threshold: o.alpha_threshold,
            margin: o.margin,
            edge_spacing: o.edge_spacing,
            interior_spacing: o.interior_spacing,
            min_island_area: o.min_island_area,
        }
    }
}

impl AutoMeshOptions {
    /// `ugoe_mesher::generate` が受け付ける値か。不正ならその理由。
    ///
    /// 規則は [`MeshOptions::validate`] と同じ (`margin` は 0 以上 1024 以下、`interior_spacing` は
    /// 0 以上、`edge_spacing` は 0 より大きい有限の値)。
    pub fn check(&self) -> Result<(), &'static str> {
        MeshOptions::from(*self).validate().map_err(|e| match e {
            MeshError::InvalidOptions(reason) => reason,
            // `validate` はオプションの誤りしか返さない。
            _ => "invalid mesh options",
        })
    }
}

/// `RegenerateMesh`: テクスチャの画像からメッシュを作り直す。
///
/// 作り直せるのは、すべてのキーフォームの頂点位置が同じ (キーフォームが 1 つだけの場合を含む)
/// メッシュだけ。頂点が変わるとキーフォームごとの変形を引き継げないため。
/// 頂点位置は UV のアフィン写像 (取り込んだメッシュでは「テクスチャの左上の位置 + UV × 寸法」)
/// になっていなければならず、その写像で新しい頂点の位置を決める。
/// 頂点番号を参照するグルー、頂点ごとの深度を持つメッシュも拒否する。
pub(crate) fn regenerate_mesh(
    model: &mut Model,
    textures: &Textures,
    id: &ugoe_model::MeshId,
    options: &AutoMeshOptions,
) -> Result<(), EditError> {
    let reject = |reason: &str| EditError::CannotRegenerateMesh {
        id: id.to_string(),
        reason: reason.to_owned(),
    };
    if model
        .glues
        .iter()
        // 描画用の複製のグルーは、作り直した後に複製ごと合わせ直すので妨げない。
        .any(|g| (&g.mesh_a == id || &g.mesh_b == id) && !draw_clone::is_link(g))
    {
        return Err(reject("a glue refers to its vertices"));
    }
    let m = model
        .meshes
        .iter()
        .find(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    if m.depth.is_some() {
        return Err(reject("it has per-vertex depth"));
    }
    let Some(base) = m.keyforms.keyforms.first() else {
        return Err(reject("it has no keyforms"));
    };
    if m.keyforms
        .all_keyforms()
        .any(|k| k.positions != base.positions)
    {
        return Err(reject(
            "its keyforms have different vertex positions (re-meshing would discard them)",
        ));
    }
    let affine = fit_affine(&m.uvs, &base.positions)
        .ok_or_else(|| reject("its vertex positions are not an affine image of its UVs"))?;
    let texture = usize::try_from(m.texture)
        .ok()
        .and_then(|i| model.textures.get(i))
        .ok_or_else(|| reject("its texture index is out of range"))?;
    let png = textures
        .get(&texture.name)
        .ok_or_else(|| reject("the texture image is missing"))?;
    let image = decode_png(png, ImageLimits::default()).map_err(|error| match error {
        ImageError::TooLarge { width, height, max } => EditError::TextureTooLarge {
            texture: texture.name.clone(),
            width,
            height,
            max,
        },
        _ => reject("the texture image cannot be decoded"),
    })?;
    let (width, height, rgba) = (image.width, image.height, image.pixels);
    let generated =
        ugoe_mesher::generate(&rgba, width, height, &(*options).into()).map_err(|e| {
            EditError::Mesher {
                id: id.to_string(),
                error: e,
            }
        })?;
    let size = Vec2::new(width as f32, height as f32);
    let uvs: Vec<Vec2> = generated.vertices.iter().map(|v| *v / size).collect();
    let positions: Vec<Vec2> = uvs.iter().map(|uv| affine.apply(*uv)).collect();

    let m = find_mesh_mut(model, id)?;
    m.uvs = uvs;
    m.indices = generated.indices;
    for keyform in m.keyforms.all_keyforms_mut() {
        keyform.positions.clone_from(&positions);
    }
    Ok(())
}

fn find_mesh_mut<'a>(
    model: &'a mut Model,
    id: &ugoe_model::MeshId,
) -> Result<&'a mut Mesh, EditError> {
    model
        .meshes
        .iter_mut()
        .find(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })
}

/// UV から位置へのアフィン写像 `p = a·u + b·v + c` (成分ごと)。
#[derive(Debug, Clone, Copy)]
struct Affine {
    x: [f64; 3],
    y: [f64; 3],
}

impl Affine {
    fn apply(&self, uv: Vec2) -> Vec2 {
        let (u, v) = (f64::from(uv.x), f64::from(uv.y));
        let eval = |c: [f64; 3]| (c[0] * u + c[1] * v + c[2]) as f32;
        Vec2::new(eval(self.x), eval(self.y))
    }
}

/// `positions ≈ A(uvs)` となるアフィン写像を最小二乗で求める。
///
/// UV が退化している (3 点以上が同一直線上にない) か、ずれが位置の広がりの 1e-4 倍を
/// 超えるなら `None`。
fn fit_affine(uvs: &[Vec2], positions: &[Vec2]) -> Option<Affine> {
    if uvs.len() != positions.len() || uvs.len() < 3 {
        return None;
    }
    // 正規方程式 M·c = r (M = Σ [u v 1]ᵀ[u v 1])。
    let mut m = [[0.0f64; 3]; 3];
    let mut rx = [0.0f64; 3];
    let mut ry = [0.0f64; 3];
    for (uv, p) in uvs.iter().zip(positions) {
        let row = [f64::from(uv.x), f64::from(uv.y), 1.0];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += row[i] * row[j];
            }
            rx[i] += row[i] * f64::from(p.x);
            ry[i] += row[i] * f64::from(p.y);
        }
    }
    let det = det3(&m);
    let scale = m.iter().flatten().fold(0.0f64, |a, &b| a.max(b.abs()));
    if !det.is_finite() || det.abs() <= 1e-12 * scale.powi(3) {
        return None;
    }
    let solve = |r: [f64; 3]| -> [f64; 3] {
        std::array::from_fn(|k| {
            let mut mk = m;
            for (row, value) in mk.iter_mut().zip(r) {
                row[k] = value;
            }
            det3(&mk) / det
        })
    };
    let affine = Affine {
        x: solve(rx),
        y: solve(ry),
    };

    let (min, max) = positions.iter().fold(
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let extent = (max - min).max_element();
    let tolerance = extent * 1e-4;
    let fits = extent.is_finite()
        && extent > 0.0
        && uvs
            .iter()
            .zip(positions)
            .all(|(uv, p)| (affine.apply(*uv) - *p).abs().max_element() <= tolerance);
    fits.then_some(affine)
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_follows_mesher_validation() {
        for margin in [0.0, 1024.0, 1024.5, -1.0, f32::NAN, f32::MAX] {
            let options = AutoMeshOptions {
                margin,
                ..AutoMeshOptions::default()
            };
            assert_eq!(
                options.check().is_ok(),
                MeshOptions::from(options).validate().is_ok(),
                "margin {margin}"
            );
        }
        let options = AutoMeshOptions {
            margin: 2048.0,
            ..AutoMeshOptions::default()
        };
        assert_eq!(options.check(), Err("margin must be between 0 and 1024"));
    }

    #[test]
    fn fits_offset_and_scale() {
        let uvs = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.25, 0.5),
        ];
        let positions: Vec<Vec2> = uvs
            .iter()
            .map(|uv| Vec2::new(10.0, -4.0) + *uv * Vec2::new(64.0, 32.0))
            .collect();
        let affine = fit_affine(&uvs, &positions).unwrap();
        let p = affine.apply(Vec2::new(0.5, 0.5));
        assert!((p - Vec2::new(42.0, 12.0)).length() < 1e-4, "{p}");
    }

    #[test]
    fn rejects_deformed_and_degenerate() {
        let uvs = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let bent = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(12.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(fit_affine(&uvs, &bent).is_none());
        let line = [Vec2::ZERO, Vec2::new(0.5, 0.5), Vec2::ONE];
        assert!(fit_affine(&line, &line).is_none());
    }
}
