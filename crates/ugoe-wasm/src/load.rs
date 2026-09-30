//! モデルの読み込み (JSON / `.ugoe` バンドル) とテクスチャのデコード。GPU には触れない。
//!
//! テクスチャは Rust 側で PNG をデコードし、乗算済みアルファの RGBA8 にする
//! (`docs/spec/rendering.md` の「前提」: 色の値はリニア化しない)。
//! ブラウザのデコーダは色空間の変換やアルファの扱いが実装ごとに異なるので使わない。

use std::io::Cursor;

use serde::Serialize;
use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::physics::PhysicsSettings;

use crate::animation::Library;
use ugoe::model::{Model, Parameter, ParameterGroup};
use ugoe::runtime::ModelInstance;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Format(#[from] ugoe::format::FormatError),
    #[error("model does not compile: {0}")]
    Compile(#[from] ugoe::runtime::CompileError),
    #[error("decoding texture {index}: {message}")]
    Texture { index: usize, message: String },
    #[error("decoding the normal map of texture {index}: {message}")]
    NormalMap { index: usize, message: String },
}

/// 乗算済みアルファの RGBA8 画像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba8 {
    /// テクスチャが無いときに使う 1x1 の不透明な白 (`ugoe render` と同じ)。
    pub fn placeholder() -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        }
    }
}

/// 読み込んだモデル一式。
#[derive(Debug)]
pub struct LoadedModel {
    pub model: Model,
    /// バンドルの物理・モーション・表情。JSON のモデルでは `None` (今の設定を引き継ぐ)。
    pub animation: Option<BundleAnimation>,
    /// 描画に使うテクスチャ (添字 = テクスチャ番号)。足りない分はプレースホルダ。
    pub textures: Vec<Rgba8>,
    /// 法線マップ (添字 = テクスチャ番号。アルファを掛けない RGBA8)。`None` の番号は法線マップなし
    /// (`docs/spec/rendering.md#ライティング`)。JSON のモデルでは空。
    pub normal_maps: Vec<Option<Rgba8>>,
    /// 読み込めたが注意が要ること (プレースホルダで代用したテクスチャなど)。
    pub warnings: Vec<LoadWarning>,
}

/// 読み込めたが注意が要ること (T0125)。`Display` は読み込みの戻り値に入る英語の文 (T0125 の前から変えていない)。
/// 機械が読む形は [`LoadWarning::to_json`] (`load_warnings_json` の要素。`crates/ugoe-wasm/README.md#読み込みの警告-t0125`)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadWarning {
    /// テクスチャが KTX2 (未対応) なので、白のプレースホルダで代用した。
    #[error("texture {texture} is KTX2, which is not supported yet; using a white placeholder")]
    TextureKtx2 { texture: usize },
    /// テクスチャの画像が無いので、白のプレースホルダで代用した。
    #[error("no data for texture {texture}; using a white placeholder")]
    TextureMissing { texture: usize },
    /// 法線マップが KTX2 (未対応) なので、法線マップなしで描く。
    #[error(
        "the normal map of texture {texture} is KTX2, which is not supported yet; drawing without it"
    )]
    NormalMapKtx2 { texture: usize },
    /// 読み直す前の物理の設定が新しいモデルと合わないので外した。`reason` は検査の結果 (英語)。
    #[error("physics settings were removed: {reason}")]
    PhysicsRemoved { reason: String },
}

impl LoadWarning {
    /// 種類を表すコード (`load_warnings_json` の `code`)。
    pub fn code(&self) -> &'static str {
        match self {
            Self::TextureKtx2 { .. } => "texture_ktx2",
            Self::TextureMissing { .. } => "texture_missing",
            Self::NormalMapKtx2 { .. } => "normal_map_ktx2",
            Self::PhysicsRemoved { .. } => "physics_removed",
        }
    }

    /// JSON: `{"code", "warning", "texture"?, "reason"?}`。`warning` は文の警告と同じ文字列。
    pub fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({
            "code": self.code(),
            "warning": self.to_string(),
        });
        match self {
            Self::TextureKtx2 { texture }
            | Self::TextureMissing { texture }
            | Self::NormalMapKtx2 { texture } => json["texture"] = (*texture).into(),
            Self::PhysicsRemoved { reason } => json["reason"] = reason.as_str().into(),
        }
        json
    }
}

/// 読み込みの警告の構造 (`load_warnings_json`)。戻り値の文の警告と同じ順: モデルの読み込みと物理の引き継ぎの
/// 警告 (`warnings`) の後に、付け直せずに外したモジュール (`detached`)。
pub fn warnings_json(
    warnings: &[LoadWarning],
    detached: &[crate::dress_up::Detached],
) -> Vec<serde_json::Value> {
    warnings
        .iter()
        .map(LoadWarning::to_json)
        .chain(detached.iter().map(crate::dress_up::Detached::to_json))
        .collect()
}

/// バンドルに入っていた物理・モーション・表情。
#[derive(Debug, Default)]
pub struct BundleAnimation {
    /// 読み込み時に `validate_physics` で検査済み (`ugoe_format::read_bundle`)。
    pub physics: Option<PhysicsSettings>,
    pub library: Library,
}

/// JSON のモデルを読む。テクスチャは持たないので、すべて白のプレースホルダにする。
pub fn from_json(json: &str) -> Result<LoadedModel, LoadError> {
    let model = ugoe::format::model_from_json(json)?;
    let textures = vec![Rgba8::placeholder(); texture_count(&model)];
    Ok(LoadedModel {
        model,
        animation: None,
        textures,
        normal_maps: Vec::new(),
        warnings: Vec::new(),
    })
}

/// `.ugoe` バンドルを読み、PNG のテクスチャと法線マップをデコードする。
pub fn from_bundle(bytes: &[u8]) -> Result<LoadedModel, LoadError> {
    let bundle = ugoe::format::read_bundle(bytes)?;
    let mut warnings = Vec::new();
    let normal_maps =
        decode_normal_maps(ugoe::lighting::bundle_normal_maps(&bundle), &mut warnings)?;
    let textures = decode_textures(
        texture_count(&bundle.model),
        |i| bundle.textures.get(i),
        &mut warnings,
    )?;
    let Bundle {
        model,
        physics,
        motions,
        expressions,
        ..
    } = bundle;
    Ok(LoadedModel {
        model,
        animation: Some(BundleAnimation {
            physics,
            library: Library {
                motions,
                expressions,
            },
        }),
        textures,
        normal_maps,
        warnings,
    })
}

/// テクスチャ `count` 枚をデコードする (`data(i)` が `i` 番目の画像)。PNG は乗算済みアルファにし、
/// KTX2 と画像の無い番号は白のプレースホルダで代用して警告する。
pub(crate) fn decode_textures<'a>(
    count: usize,
    data: impl Fn(usize) -> Option<&'a TextureData>,
    warnings: &mut Vec<LoadWarning>,
) -> Result<Vec<Rgba8>, LoadError> {
    (0..count)
        .map(|index| match data(index) {
            Some(TextureData {
                format: TextureFormat::Png,
                bytes,
            }) => decode_png_premultiplied(bytes)
                .map_err(|message| LoadError::Texture { index, message }),
            Some(TextureData {
                format: TextureFormat::Ktx2,
                ..
            }) => {
                // KTX2 のデコードは未対応 (`ugoe render` と同じ)。表示は止めずに白で代用する。
                warnings.push(LoadWarning::TextureKtx2 { texture: index });
                Ok(Rgba8::placeholder())
            }
            // 読み込み (`read_bundle` / `read_module`) がテクスチャの数を検証するので、通常は起きない。
            None => {
                warnings.push(LoadWarning::TextureMissing { texture: index });
                Ok(Rgba8::placeholder())
            }
        })
        .collect()
}

/// テクスチャの番号ごとの法線マップ (`None` は法線マップなし) を、アルファを掛けずにデコードする。
/// KTX2 は警告して使わない。
pub(crate) fn decode_normal_maps<'a>(
    data: impl IntoIterator<Item = Option<&'a TextureData>>,
    warnings: &mut Vec<LoadWarning>,
) -> Result<Vec<Option<Rgba8>>, LoadError> {
    data.into_iter()
        .enumerate()
        .map(|(index, data)| match data {
            None => Ok(None),
            Some(TextureData {
                format: TextureFormat::Png,
                bytes,
            }) => decode_png(bytes)
                .map(Some)
                .map_err(|message| LoadError::NormalMap { index, message }),
            Some(TextureData {
                format: TextureFormat::Ktx2,
                ..
            }) => {
                warnings.push(LoadWarning::NormalMapKtx2 { texture: index });
                Ok(None)
            }
        })
        .collect()
}

/// 描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
pub fn texture_count(model: &Model) -> usize {
    model
        .meshes
        .iter()
        .map(|m| m.texture as usize + 1)
        .max()
        .unwrap_or(0)
        .max(model.textures.len())
}

/// パラメータとパラメータグループの定義 (スライダーの一覧を作るため)。
#[derive(Debug, Serialize)]
struct ParameterList<'a> {
    parameters: &'a [Parameter],
    groups: &'a [ParameterGroup],
}

/// パラメータとグループの定義を JSON にする: `{ "parameters": [...], "groups": [...] }`。
///
/// 各要素はモデルの JSON 形式と同じ (`docs/spec/model.md`)。ビューアなどのホストが
/// スライダーの値域・既定値・グループを知るために使う。
pub fn parameters_json(model: &Model) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ParameterList {
        parameters: &model.parameters,
        groups: &model.parameter_groups,
    })
}

/// `from` のパラメータ値のうち、同じ ID が `to` にもあるものを `to` へ写す (値域でクランプされる)。
///
/// エディタでモデルを読み直したとき (編集・Undo のたび) に、スライダーで決めた表示の状態を保つため。
pub fn carry_over_parameters(from: &ModelInstance, to: &mut ModelInstance) {
    let values = from.parameters();
    let ids = from
        .compiled()
        .model()
        .parameters
        .iter()
        .map(|p| p.id.as_str());
    for (id, &value) in ids.zip(values.as_slice()) {
        let params = to.parameters_mut();
        if let Some(index) = params.index_of(id) {
            params.set(index, value);
        }
    }
    to.update();
}

/// PNG をデコードして、乗算済みアルファの RGBA8 にする。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。
pub fn decode_png_premultiplied(bytes: &[u8]) -> Result<Rgba8, String> {
    let mut image = decode_png(bytes)?;
    for p in image.pixels.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
    Ok(image)
}

/// PNG をデコードして、アルファを掛けない RGBA8 にする (法線マップ用。アルファは使わない)。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。
pub fn decode_png(bytes: &[u8]) -> Result<Rgba8, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "PNG image is too large".to_owned())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let pixel_count = info.width as usize * info.height as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    match info.color_type {
        png::ColorType::Rgba => pixels = buf,
        png::ColorType::Rgb => {
            for p in buf.as_chunks::<3>().0 {
                pixels.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for p in buf.as_chunks::<2>().0 {
                pixels.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in &buf {
                pixels.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".to_owned()),
    }
    Ok(Rgba8 {
        width: info.width,
        height: info.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use ugoe::format::write_bundle;
    use ugoe::model::fixtures;
    use ugoe::runtime::CompiledModel;

    use super::*;

    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(rgba).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn json_models_get_white_placeholders() {
        let json = ugoe::format::model_to_json(&fixtures::single_quad()).unwrap();
        let loaded = from_json(&json).unwrap();
        assert_eq!(loaded.textures, vec![Rgba8::placeholder()]);
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn bundle_png_is_premultiplied() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(2, 1, &[255, 0, 0, 255, 255, 255, 255, 128]),
        }];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(loaded.textures.len(), 1);
        let t = &loaded.textures[0];
        assert_eq!((t.width, t.height), (2, 1));
        assert_eq!(t.pixels, vec![255, 0, 0, 255, 128, 128, 128, 128]);
    }

    #[test]
    fn bundle_normal_maps_are_not_premultiplied() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(1, 1, &[255; 4]),
        }];
        bundle.normal_maps = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(1, 1, &[128, 200, 255, 0]),
        }];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(
            loaded.normal_maps,
            vec![Some(Rgba8 {
                width: 1,
                height: 1,
                pixels: vec![128, 200, 255, 0],
            })]
        );
    }

    #[test]
    fn ktx2_images_are_replaced_with_structured_warnings() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: b"\xABKTX 20\xBB\r\n\x1A\n".to_vec(),
        };
        bundle.textures = vec![ktx2.clone()];
        bundle.normal_maps = vec![ktx2];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(loaded.textures, vec![Rgba8::placeholder()]);
        assert_eq!(loaded.normal_maps, vec![None]);
        assert_eq!(
            loaded.warnings,
            [
                LoadWarning::NormalMapKtx2 { texture: 0 },
                LoadWarning::TextureKtx2 { texture: 0 },
            ]
        );
        // 文の警告は T0125 の前と同じ文面。構造には同じ文字列を `warning` に入れる。
        assert_eq!(
            loaded.warnings[0].to_string(),
            "the normal map of texture 0 is KTX2, which is not supported yet; drawing without it"
        );
        assert_eq!(
            warnings_json(&loaded.warnings, &[]),
            [
                serde_json::json!({
                    "code": "normal_map_ktx2",
                    "warning": loaded.warnings[0].to_string(),
                    "texture": 0,
                }),
                serde_json::json!({
                    "code": "texture_ktx2",
                    "warning": "texture 0 is KTX2, which is not supported yet; using a white placeholder",
                    "texture": 0,
                }),
            ]
        );
    }

    #[test]
    fn missing_textures_and_removed_physics_have_codes() {
        let mut warnings = Vec::new();
        let textures = decode_textures(2, |_| None, &mut warnings).unwrap();
        assert_eq!(textures, vec![Rgba8::placeholder(); 2]);
        warnings.push(LoadWarning::PhysicsRemoved {
            reason: "bad rig".into(),
        });
        let json = warnings_json(&warnings, &[]);
        assert_eq!(json[1]["code"], "texture_missing");
        assert_eq!(json[1]["texture"], 1);
        assert_eq!(
            json[1]["warning"],
            "no data for texture 1; using a white placeholder"
        );
        assert_eq!(
            json[2],
            serde_json::json!({
                "code": "physics_removed",
                "warning": "physics settings were removed: bad rig",
                "reason": "bad rig",
            })
        );
    }

    #[test]
    fn broken_png_is_an_error() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0],
        }];
        let bytes = write_bundle(&bundle).unwrap();
        assert!(matches!(
            from_bundle(&bytes),
            Err(LoadError::Texture { index: 0, .. })
        ));
    }

    #[test]
    fn parameters_json_lists_ranges_and_groups() {
        let mut model = fixtures::single_quad();
        model.parameter_groups.push(ParameterGroup {
            id: "Face".into(),
            name: "Face".into(),
        });
        model.parameters[0].group = Some("Face".into());
        let json: serde_json::Value =
            serde_json::from_str(&parameters_json(&model).unwrap()).unwrap();
        let p = &json["parameters"][0];
        assert_eq!(p["id"], "X");
        assert_eq!(p["group"], "Face");
        assert_eq!(p["min"].as_f64(), Some(f64::from(model.parameters[0].min)));
        assert_eq!(p["max"].as_f64(), Some(f64::from(model.parameters[0].max)));
        assert_eq!(json["groups"][0]["name"], "Face");
    }

    #[test]
    fn carries_over_parameters_by_id() {
        let compiled = CompiledModel::compile(fixtures::single_quad()).unwrap();
        let mut old = ModelInstance::new(compiled);
        let x = old.parameters().index_of("X").unwrap();
        old.parameters_mut().set(x, 0.5);

        let mut model = fixtures::single_quad();
        model.parameters[0].max = 0.25;
        let mut new = ModelInstance::new(CompiledModel::compile(model).unwrap());
        carry_over_parameters(&old, &mut new);
        // 新しい値域でクランプされ、評価し直されている。
        assert_eq!(new.parameters().as_slice(), &[0.25]);
        assert_eq!(new.drawables()[0].positions[0].x, 64.0 + 16.0);
    }
}
