//! `ugoe build <file>.model.json` / `<file>.module.json`: 手で書いたモデル・モジュールの JSON と、その隣の
//! `textures/<name>.png` から、バンドル・モジュールに書く材料を作る。
//!
//! 画像の置き場は `.ugoeproj` プロジェクトと同じ (`docs/spec/file-formats.md#プロジェクト-ugoeproj`)。
//! 見つからない画像は 1x1 の画像で埋める (テクスチャは不透明な白、法線マップは正面向き)。

use std::path::{Path, PathBuf};

use anyhow::Context;
use ugoe_editor_core::Textures;
use ugoe_editor_core::texture::{is_valid_texture_name, png_size, solid_png};
use ugoe_format::FormatError;
use ugoe_model::module::Module;
use ugoe_model::{Model, TextureRef};

/// 画像を探すディレクトリ (JSON の隣)。
const TEXTURES_DIR: &str = "textures";

/// 見つからないテクスチャの代わり (不透明な白。`ugoe render` が JSON のモデルに使うものと同じ)。
const PLACEHOLDER_TEXTURE: [u8; 4] = [255, 255, 255, 255];

/// 見つからない法線マップの代わり (`(0, 0, 1)` = 正面向き。`docs/spec/rendering.md#ライティング`)。
const PLACEHOLDER_NORMAL: [u8; 4] = [128, 128, 255, 255];

/// JSON から読んだもの。
pub enum JsonSource {
    Model(Box<Model>),
    Module(Box<Module>),
}

/// `json` をモデルかモジュールとして読む。最上位に `slot` があればモジュール、なければモデル
/// (バンドルとモジュールを中身で判別するのと同じく、拡張子は見ない)。
///
/// 読めた後の検証 (`validate` / `validate_module`) に失敗したら、その [`FormatError`] を返す
/// (呼び出し側がバンドルの検証エラーと同じ形で書く)。
pub fn parse(path: &Path, json: &str) -> anyhow::Result<Result<JsonSource, FormatError>> {
    let value: serde_json::Value =
        serde_json::from_str(json).with_context(|| format!("parsing {}", path.display()))?;
    if value.get("slot").is_some() {
        let module = ugoe_format::module_from_json(json)
            .with_context(|| format!("parsing module {}", path.display()))?;
        let errors = ugoe_model::validate::validate_module(&module);
        if !errors.is_empty() {
            return Ok(Err(FormatError::InvalidModule(errors)));
        }
        Ok(Ok(JsonSource::Module(Box::new(module))))
    } else {
        let model = ugoe_format::model_from_json(json)
            .with_context(|| format!("parsing model {}", path.display()))?;
        let errors = ugoe_model::validate::validate(&model);
        if !errors.is_empty() {
            return Ok(Err(FormatError::Invalid(errors)));
        }
        Ok(Ok(JsonSource::Model(Box::new(model))))
    }
}

/// `textures` のテクスチャと法線マップの画像を、`json` の隣の `textures/<name>.png` から名前で読む。
/// 見つからない画像は 1x1 の画像で埋め、標準エラーに `warning: ...` を書く。
pub fn images(json: &Path, textures: &[TextureRef]) -> anyhow::Result<Textures> {
    let dir = json.parent().unwrap_or(Path::new("")).join(TEXTURES_DIR);
    let mut images = Textures::new();
    let wanted = textures
        .iter()
        .map(|t| (t.name.as_str(), PLACEHOLDER_TEXTURE, "texture"))
        .chain(
            textures
                .iter()
                .filter_map(|t| t.normal_map.as_deref())
                .map(|name| (name, PLACEHOLDER_NORMAL, "normal map")),
        );
    for (name, placeholder, what) in wanted {
        if images.contains_key(name) {
            continue;
        }
        let png = match image_path(&dir, name) {
            Some(path) if path.exists() => read_png(&path)?,
            path => {
                let looked = path.map_or_else(
                    || "its name is not a file name".to_owned(),
                    |p| format!("no {}", p.display()),
                );
                eprintln!(
                    "warning: {what} `{name}` has no image ({looked}); using a 1x1 placeholder"
                );
                solid_png(1, 1, placeholder)
            }
        };
        images.insert(name.to_owned(), png.into());
    }
    Ok(images)
}

/// 画像のパス。名前がファイル名に使えなければ (プロジェクトと同じ規則) `None`。
fn image_path(dir: &Path, name: &str) -> Option<PathBuf> {
    is_valid_texture_name(name).then(|| dir.join(format!("{name}.png")))
}

/// 画像を読む。プロジェクトの `textures/` と同じく、通常のファイルだけを上限まで読み、PNG であることを確かめる。
fn read_png(path: &Path) -> anyhow::Result<Vec<u8>> {
    let max = ugoe_editor_core::project::ProjectReadLimits::default().max_image;
    let png = ugoe_fs::read_regular_file(path, max)
        .with_context(|| format!("reading {}", path.display()))?;
    if png_size(&png).is_none() {
        anyhow::bail!("{} is not a PNG", path.display());
    }
    Ok(png)
}
