//! `ugoe sample avatar --lit`: 見本のアバターに法線マップを付け、ライティングを有効にした版 (T0707)。
//!
//! 法線マップは T0506 の方法 (アルファの形のふくらみ、`ugoe_rigging::bulge_height`) で作る。ただしテクスチャごとに
//! 別々にふくらませると、顔の上の目・口が平らに浮いたり、首と肩の境目に段差が出たりする (T0506 のメモ)。そこで
//! 部位をまとまり ([`GROUPS`]) に分け、まとまりの形 (メンバーのアルファを基準の姿のキャンバスに並べた和) を 1 つの
//! ふくらみにして、その高さをメンバーのテクスチャに写してから法線にする。顔の上の目・口・鼻・眉は顔のふくらみを
//! そのまま受け、首と体は 1 つの形としてつながる。まとまりのふくらみはエディタの生成と同じ
//! `ugoe_rigging::group_bulge_normal_maps` (T0518) で作る。
//!
//! 法線は変化がなだらかなので、ファイルを小さくするためにテクスチャの半分の解像度で持つ ([`DOWNSCALE`]。
//! 法線マップはテクスチャと同じ UV の範囲に引き伸ばして使われる。`docs/spec/rendering.md#ライティング`)。
//! モデルのほかの部分 (リグ・物理・メタデータ) は [`super::build_avatar`] の出力のまま変えない。

use anyhow::{Context, anyhow, bail};
use ugoe_core::Vec2;
use ugoe_editor_core::texture::{decode_rgba_png, encode_rgba_png};
use ugoe_editor_core::{Command, Document, Project, PsdImportOptions};
use ugoe_rigging::{BulgeOptions, NormalMap, PlacedTexture, group_bulge_normal_maps};

/// 法線マップの解像度をテクスチャの何分の 1 にするか。
pub const DOWNSCALE: u32 = 2;

/// 法線マップの画像の名前に付ける接尾辞 (エディタの生成と同じ)。
const SUFFIX: &str = "_normal";

/// 1 つのふくらみにまとめる部位 (メッシュの名前) と、その設定。
struct Group {
    meshes: &'static [&'static str],
    /// 形にだけ加える部位 (法線マップはほかのまとまりで作る)。
    shape_only: &'static [&'static str],
    /// まとまりの形全体のふくらみ。
    bulge: BulgeOptions,
}

const fn bulge(strength: f32, radius: Option<f32>, smoothing: f32) -> BulgeOptions {
    BulgeOptions {
        strength,
        radius,
        smoothing,
        alpha_threshold: 128,
    }
}

/// 部位のまとまり。`strength` はふくらみの高さと幅の比 (1 で断面が円)。顔と体のような大きな部位は弱く、
/// 髪の房・耳のような細い部位は強めにする。ならし (`smoothing`、キャンバスの画素) は輪郭で法線が
/// 真横を向く幅を抑える。
///
/// `ugoe_ai::normal::SurfaceGroup` の既定 (エディタの提案) はこの値から作ったもので、強さとならしは同じだが、
/// ここでは共有しない (T0132)。見本はコミットしたファイルなので、ugoe-ai の既定を調整するたびに見本が変わるのを避ける。
/// また、まとめ方も同じではない: 顔の縁の幅は、既定では形の幅に対する比 (`FACE_EDGE`) だが、ここでは画素で固定し、
/// 見本は意味ラベルを使わずメッシュの名前で選び、左右の耳・耳飾りを別々のまとまりにする。
const GROUPS: &[Group] = &[
    // 顔と、顔の上に描く部位。目・口などは顔と同じ面の向きを受ける (平らに浮かない)。
    Group {
        meshes: &[
            "face",
            "nose",
            "mouth-open",
            "mouth-close",
            "eyewhite-l",
            "eyewhite-r",
            "irides-l",
            "irides-r",
            "eyelash-l",
            "eyelash-r",
            "eyeclose-l",
            "eyeclose-r",
            "eyebrow-l",
            "eyebrow-r",
        ],
        shape_only: &[],
        bulge: bulge(0.35, Some(48.0), 6.0),
    },
    // 首・体・腕 (上着を含む)。腕は肩の上に重なるので、1 つの形にして首の付け根や肩に段差を作らない。
    // 顔を形に加えて首を形の上端にしない (首の上が上を向いて明るくなり、あごの下が顔より明るく見えるため)。
    Group {
        meshes: &["neck", "topwear", "handwear-l", "handwear-r"],
        shape_only: &["face"],
        bulge: bulge(0.3, None, 6.0),
    },
    Group {
        meshes: &["front hair"],
        shape_only: &[],
        bulge: bulge(0.5, None, 3.0),
    },
    Group {
        meshes: &["back hair"],
        shape_only: &[],
        bulge: bulge(0.5, None, 3.0),
    },
    Group {
        meshes: &["ears-l"],
        shape_only: &[],
        bulge: bulge(0.5, None, 2.0),
    },
    Group {
        meshes: &["ears-r"],
        shape_only: &[],
        bulge: bulge(0.5, None, 2.0),
    },
    Group {
        meshes: &["earwear-l"],
        shape_only: &[],
        bulge: bulge(0.6, None, 1.0),
    },
    Group {
        meshes: &["earwear-r"],
        shape_only: &[],
        bulge: bulge(0.6, None, 1.0),
    },
];

/// 見本のアバターのプロジェクト (`psd` から [`super::build_avatar`] で作ったもの) に、法線マップと
/// ライティングを付けたプロジェクトを返す。
///
/// すべてのテクスチャに法線マップを付け、`Model::lighting` を有効にする (メッシュの `lit` は既定の `true` のまま)。
/// 部位のキャンバスでの位置は、リグを組む前の取り込みの結果から読む (基準の姿では口などが変形しているため)。
pub fn light_avatar(project: &Project, psd: &[u8]) -> anyhow::Result<Project> {
    let mut document = project.document();
    let layers = Layer::all(&document, psd)?;
    let mut commands = Vec::new();
    let find = |names: &[&str]| {
        names
            .iter()
            .map(|name| {
                layers
                    .iter()
                    .find(|l| l.mesh == *name)
                    .ok_or_else(|| anyhow!("no mesh named `{name}`"))
            })
            .collect::<anyhow::Result<Vec<_>>>()
    };
    for group in GROUPS {
        let members = find(group.meshes)?;
        let shape = find(group.shape_only)?;
        let maps = group_normal_maps(&members, &shape, &group.bulge)?;
        for (layer, map) in members.iter().zip(maps) {
            let map = map.downscale(DOWNSCALE);
            commands.push(Command::SetNormalMap {
                texture: layer.texture.clone(),
                normal_map: Some(format!("{}{SUFFIX}", layer.texture)),
                png: Some(encode_rgba_png(map.width, map.height, &map.rgba)),
            });
        }
    }
    let covered = commands.len();
    if covered != layers.len() {
        bail!("{covered} of {} meshes are in a group", layers.len());
    }
    commands.push(Command::SetLighting { enabled: true });
    document.apply(Command::Batch {
        label: "Light the sample avatar".into(),
        commands,
    })?;
    Ok(Project::from_document(project.meta.clone(), &document))
}

/// メッシュ 1 つと、そのテクスチャの画素・基準の姿のキャンバスでの位置。
struct Layer {
    mesh: String,
    texture: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    /// テクスチャの画素 `(0, 0)` の中心のキャンバス座標。テクスチャの 1 画素はキャンバスの 1 画素
    /// (PSD の取り込みはレイヤーを切り抜いたまま置く)。
    origin: Vec2,
}

impl Layer {
    /// `document` のすべてのメッシュ (宣言順)。位置は `psd` を取り込み直したメッシュ (親がないので頂点はキャンバス座標)
    /// から読む。1 つのテクスチャを 1 つのメッシュだけが使うこと。
    fn all(document: &Document, psd: &[u8]) -> anyhow::Result<Vec<Self>> {
        let model = document.model();
        let mut imported = Project::empty(&model.meta.name).document();
        let import = ugoe_editor_core::import::import_psd(
            imported.model(),
            psd,
            &PsdImportOptions::new(&model.meta.name),
        )
        .context("importing the PSD")?;
        imported.apply(import.command)?;
        let mut layers = Vec::new();
        for mesh in &model.meshes {
            let rest = imported
                .model()
                .meshes
                .iter()
                .find(|m| m.name == mesh.name)
                .and_then(|m| m.keyforms.keyforms.first().map(|k| (m, k)))
                .with_context(|| format!("mesh `{}` is not in the PSD", mesh.name))?;
            let texture = model
                .textures
                .get(mesh.texture as usize)
                .with_context(|| format!("texture of mesh `{}`", mesh.name))?;
            if model
                .meshes
                .iter()
                .filter(|m| m.texture == mesh.texture)
                .count()
                != 1
            {
                bail!("texture `{}` is shared by several meshes", texture.name);
            }
            let png = document
                .textures()
                .get(&texture.name)
                .with_context(|| format!("image of texture `{}`", texture.name))?;
            let (width, height, rgba) = decode_rgba_png(png)
                .with_context(|| format!("decoding texture `{}`", texture.name))?;
            let size = Vec2::new(width as f32, height as f32);
            // UV (0〜1) は画素の角で測るので、画素の中心は 0.5 だけずれる。
            let origins: Vec<Vec2> = rest
                .0
                .uvs
                .iter()
                .zip(&rest.1.positions)
                .map(|(uv, p)| *p - *uv * size + Vec2::splat(0.5))
                .collect();
            let origin = origins[0];
            if origins.iter().any(|o| o.distance(origin) > 0.01) {
                bail!(
                    "mesh `{}` is not an unscaled copy of its texture at rest",
                    mesh.name
                );
            }
            layers.push(Self {
                mesh: mesh.name.clone(),
                texture: texture.name.clone(),
                width,
                height,
                rgba,
                origin,
            });
        }
        Ok(layers)
    }

    /// 画素ごとの、画素の中心のキャンバス座標 (行優先)。
    fn positions(&self) -> Vec<Option<Vec2>> {
        (0..self.height)
            .flat_map(|ty| {
                (0..self.width).map(move |tx| Some(self.origin + Vec2::new(tx as f32, ty as f32)))
            })
            .collect()
    }
}

/// まとまり `members` (と形にだけ加える `shape`) の形を 1 つのふくらみにし、`members` の法線マップ
/// (テクスチャと同じ解像度) を返す (`ugoe_rigging::group_bulge_normal_maps`、T0518)。
fn group_normal_maps(
    members: &[&Layer],
    shape: &[&Layer],
    options: &BulgeOptions,
) -> anyhow::Result<Vec<NormalMap>> {
    let member_positions: Vec<_> = members.iter().map(|l| l.positions()).collect();
    let shape_positions: Vec<_> = shape.iter().map(|l| l.positions()).collect();
    Ok(group_bulge_normal_maps(
        &placed(members, &member_positions),
        &placed(shape, &shape_positions),
        options,
    )?)
}

/// `layers` を、画素の位置 `positions` (`layers` と同じ並び) とともにまとまりに置く。
fn placed<'a>(layers: &[&'a Layer], positions: &'a [Vec<Option<Vec2>>]) -> Vec<PlacedTexture<'a>> {
    layers
        .iter()
        .zip(positions)
        .map(|(l, positions)| PlacedTexture {
            width: l.width,
            height: l.height,
            rgba: &l.rgba,
            positions,
            // PSD の取り込みはレイヤーを切り抜いたまま置く (テクスチャの 1 画素はキャンバスの 1 画素)。
            canvas_per_texel: 1.0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ugoe_model::Model;

    use super::*;
    use crate::sample::{DEFAULT_PSD, build_avatar};

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn psd() -> Vec<u8> {
        std::fs::read(repo_path(DEFAULT_PSD)).unwrap()
    }

    /// 見本のアバターと、法線マップを付けた版。
    fn build() -> (Project, Project) {
        let psd = psd();
        let avatar = build_avatar(&psd).unwrap();
        let lit = light_avatar(&avatar, &psd).unwrap();
        (avatar, lit)
    }

    /// 法線マップを除いたモデル (ライティングも切る)。
    fn without_lighting(model: &Model) -> Model {
        let mut model = model.clone();
        model.lighting = false;
        for texture in &mut model.textures {
            texture.normal_map = None;
        }
        model
    }

    /// キャンバスの点 `p` での、メッシュ `mesh` の法線 (法線マップから読む)。
    fn normal_at(project: &Project, layers: &[Layer], mesh: &str, p: Vec2) -> [f32; 3] {
        let layer = layers.iter().find(|l| l.mesh == mesh).unwrap();
        let name = format!("{}{SUFFIX}", layer.texture);
        let (width, height, rgba) = decode_rgba_png(project.textures.get(&name).unwrap()).unwrap();
        let map = NormalMap {
            width,
            height,
            rgba,
        };
        let t = (p - layer.origin + Vec2::splat(0.5)) / DOWNSCALE as f32;
        map.normal(t.x as u32, t.y as u32).unwrap()
    }

    fn angle(a: [f32; 3], b: [f32; 3]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        dot.clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn lit_avatar_differs_from_the_avatar_only_in_lighting() {
        let (avatar, lit) = build();
        assert!(ugoe_model::validate::validate(&lit.model).is_empty());
        assert!(lit.model.lighting);
        assert!(lit.model.meshes.iter().all(|m| m.lit));
        assert!(lit.model.textures.iter().all(|t| t.normal_map.is_some()));
        assert_eq!(without_lighting(&lit.model), avatar.model);
        assert_eq!(lit.physics, avatar.physics);
        for texture in &avatar.model.textures {
            assert_eq!(
                lit.textures.get(&texture.name),
                avatar.textures.get(&texture.name)
            );
            let normal = format!("{}{SUFFIX}", texture.name);
            let (w, h, _) = decode_rgba_png(lit.textures.get(&normal).unwrap()).unwrap();
            assert_eq!(
                (w, h),
                (
                    texture.width.div_ceil(DOWNSCALE),
                    texture.height.div_ceil(DOWNSCALE)
                ),
                "{normal}"
            );
        }
    }

    /// 顔の上の目・鼻は顔と同じ向きを受け、首と体・肩の上の腕は境目で向きがつながる。
    /// 形の中ほどは正面を、輪郭の近くは外を向く。
    #[test]
    fn parts_in_a_group_share_the_surface() {
        let psd = psd();
        let (_, lit) = build();
        let layers = Layer::all(&lit.document(), &psd).unwrap();
        let center = |mesh: &str| {
            let l = layers.iter().find(|l| l.mesh == mesh).unwrap();
            l.origin + Vec2::new(l.width as f32, l.height as f32) * 0.5
        };
        // 目・鼻の中心で、目・鼻の法線と顔の法線がほぼ同じ。
        for mesh in [
            "eyewhite-l",
            "eyewhite-r",
            "nose",
            "irides-l",
            "mouth-close",
        ] {
            let p = center(mesh);
            let a = normal_at(&lit, &layers, mesh, p);
            let b = normal_at(&lit, &layers, "face", p);
            assert!(angle(a, b) < 3.0, "{mesh}: {a:?} vs face {b:?}");
        }
        // 首と体が重なる所・腕と体が重なる所 (肩) で、向きがほぼ同じ。
        for (a, b, p) in [
            ("neck", "topwear", Vec2::new(512.0, 400.0)),
            ("handwear-l", "topwear", Vec2::new(690.0, 520.0)),
            ("handwear-r", "topwear", Vec2::new(335.0, 520.0)),
        ] {
            let (na, nb) = (
                normal_at(&lit, &layers, a, p),
                normal_at(&lit, &layers, b, p),
            );
            assert!(angle(na, nb) < 3.0, "{a} {na:?} vs {b} {nb:?} at {p:?}");
        }
        // 顔の中ほどは正面を向き、左右の頬の輪郭の近くは外 (左・右) を向く。
        let face = layers.iter().find(|l| l.mesh == "face").unwrap();
        assert!(normal_at(&lit, &layers, "face", center("nose"))[2] > 0.97);
        // 鼻の高さの行で、顔の形の左右の端から 10 画素内側。
        let row = (center("nose").y - face.origin.y).round() as usize;
        let inside: Vec<usize> = (0..face.width as usize)
            .filter(|x| face.rgba[(row * face.width as usize + x) * 4 + 3] >= 128)
            .collect();
        let at = |x: usize| face.origin + Vec2::new(x as f32, row as f32);
        let left = normal_at(&lit, &layers, "face", at(inside[0] + 10));
        let right = normal_at(&lit, &layers, "face", at(inside[inside.len() - 1] - 10));
        assert!(left[0] < -0.2 && right[0] > 0.2, "{left:?} {right:?}");
        // 髪も (T0707 で直したマルチグリッドの確かめ) ふくらむ。
        for mesh in ["front hair", "back hair"] {
            let l = layers.iter().find(|l| l.mesh == mesh).unwrap();
            let name = format!("{}{SUFFIX}", l.texture);
            let (_, _, rgba) = decode_rgba_png(lit.textures.get(&name).unwrap()).unwrap();
            let tilted = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|c| c[2] < 230)
                .count();
            assert!(
                tilted * 10 > rgba.len() / 4,
                "{mesh}: {tilted} tilted texels"
            );
        }
    }

    /// コミットした `assets/samples/avatar-lit.ugoe` が、今のジェネレータの出力と同じこと。モデルは見本のアバター
    /// (`avatar.ugoe`) に法線マップの名前とライティングを足しただけで、法線マップは画素ごとに (丸めの誤差を除いて) 同じ。
    /// 違えば `just cli sample avatar --lit -o assets/samples/avatar-lit.ugoe` で作り直す。
    #[test]
    fn committed_lit_sample_is_up_to_date() {
        let read = |path: &str| {
            ugoe_format::read_bundle(&std::fs::read(repo_path(path)).unwrap()).unwrap()
        };
        let committed = read("assets/samples/avatar-lit.ugoe");
        let avatar = read("assets/samples/avatar.ugoe");
        assert_eq!(without_lighting(&committed.model), avatar.model);
        assert_eq!(committed.textures, avatar.textures);
        assert_eq!(committed.physics, avatar.physics);

        let (_, fresh) = build();
        assert_eq!(
            committed.model.textures, fresh.model.textures,
            "regenerate assets/samples/avatar-lit.ugoe"
        );
        for (index, texture) in fresh.model.textures.iter().enumerate() {
            let name = texture.normal_map.as_ref().unwrap();
            let a = decode_rgba_png(&committed.normal_map(index).unwrap().bytes).unwrap();
            let b = decode_rgba_png(fresh.textures.get(name).unwrap()).unwrap();
            assert_eq!((a.0, a.1), (b.0, b.1), "{name}");
            let diff = a.2.iter().zip(&b.2).map(|(x, y)| x.abs_diff(*y)).max();
            assert!(
                diff <= Some(2),
                "regenerate assets/samples/avatar-lit.ugoe ({name} differs by {diff:?})"
            );
        }
    }
}
