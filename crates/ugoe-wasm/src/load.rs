//! モデルの読み込み (JSON / `.ugoe` バンドル) とテクスチャのデコード。GPU には触れない。
//!
//! テクスチャは Rust 側で PNG と KTX2 (圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7) をデコードし、乗算済みアルファの RGBA8 にする
//! (`docs/spec/rendering.md` の「前提」: 色の値はリニア化しない)。
//! ブラウザのデコーダは色空間の変換やアルファの扱いが実装ごとに異なるので使わない。

use serde::{Deserialize, Serialize};
use ugoe::format::{Bundle, TextureData};
use ugoe::limits::ReadLimits;
use ugoe::model::physics::PhysicsSettings;
use ugoe::render::RenderError;
use ugoe::{FieldValue, LoadErrorValue};
use ugoe_format::image::{ImageError, ImageLimits, decode_image, decode_image_premultiplied};

use crate::animation::Library;
use ugoe::model::{Model, Parameter, ParameterGroup};
use ugoe::runtime::ModelInstance;

/// 読み込みの失敗 (`load_json` / `load_bundle` と、`.ugoemod` の読み込み)。JS には `name` が `"LoadError"` の `Error` で、
/// `code` と補足の値は [`LoadError::info`] (`crates/ugoe-wasm/README.md#読み込みの失敗-t0434`)。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// ファイルを読めないか、モデルをコンパイルできない。種類と補足の値は SDK の [`ugoe::LoadError`] が決め、
    /// C ABI の `ugoe_puppet_create_from_*` と同じ (T0457)。
    #[error(transparent)]
    Model(#[from] ugoe::LoadError),
    #[error("decoding texture {index}: {message}")]
    Texture { index: usize, message: String },
    #[error("decoding the normal map of texture {index}: {message}")]
    NormalMap { index: usize, message: String },
    /// テクスチャの画像の一辺が上限 ([`LoadLimits::image`]) を超える。画素は確保していない。
    #[error("texture {index} is {width}x{height}, larger than the limit {max}x{max}")]
    TextureTooLarge {
        index: usize,
        width: u32,
        height: u32,
        max: u32,
    },
    /// 法線マップの一辺が上限を超える。
    #[error(
        "the normal map of texture {index} is {width}x{height}, larger than the limit {max}x{max}"
    )]
    NormalMapTooLarge {
        index: usize,
        width: u32,
        height: u32,
        max: u32,
    },
    /// レンダラがテクスチャを受け付けない (デバイスの扱える大きさを超えるなど)。何も変える前に検査する。
    #[error(transparent)]
    Render(#[from] RenderError),
    /// テクスチャのレンダラへの登録に失敗した。
    #[error("registering the textures failed: {0}")]
    Register(String),
    /// GPU のデバイスを失った (`WebPuppet::is_lost`)。新しい canvas で作り直す必要がある。
    #[error("the GPU device was lost")]
    DeviceLost,
}

impl From<ugoe::format::FormatError> for LoadError {
    fn from(e: ugoe::format::FormatError) -> Self {
        Self::Model(e.into())
    }
}

impl From<ugoe::runtime::CompileError> for LoadError {
    fn from(e: ugoe::runtime::CompileError) -> Self {
        Self::Model(e.into())
    }
}

impl LoadError {
    /// 読み込みの上限を超えたときの種類 (JS の `Error` の `code`)。展開の上限は SDK の値 (C ABI と同じ)。
    pub const IMAGE_LIMIT_EXCEEDED: &'static str = "image_limit_exceeded";

    /// 機械が読む種類と補足の値 (JS の `Error` の `code` とプロパティ)。値は組み込み先が照らし合わせるので変えない。
    /// ファイルとモデルの失敗 (`invalid_file`〜`read_limit_exceeded`) は SDK の [`ugoe::LoadError::code`]・`fields` で、
    /// C ABI の `ugoe_last_error_kind` と同じ値 (T0457)。
    ///
    /// - `invalid_file`: ファイルを読めない (ugoe のファイルでない・壊れている・JSON の誤りなど)。
    /// - `unsupported_version`: `found` (ファイルの版)・`supported` (読める版)。
    /// - `unsupported_features`: `features` (知らない必須の機能の ID を `", "` でつないだもの)。
    /// - `invalid_model`: `errors` (検査のエラーの数。モデル・物理・モーション・表情の検査)。
    /// - `compile`: 評価できない構造。
    /// - `read_limit_exceeded`: `limit` (`"chunk_decompressed"` | `"total_decompressed"`)・`chunk` (タグ)・`max` (バイト数)。
    /// - `image_limit_exceeded`: `image` (`"texture"` | `"normal_map"`)・`texture` (テクスチャの番号)・`width`・`height`・`max` (画素)。
    /// - `invalid_texture`: `image`・`texture`。画像 (PNG・KTX2) をデコードできない (壊れている。読まない形式の KTX2 は失敗でなく警告)。
    /// - `texture_too_large`: `image_limit_exceeded` と同じ補足の値。デバイスの扱える一辺 (`max`) を超える。
    /// - `texture`: テクスチャをレンダラに登録できない (着せ替えの `texture` と同じ)。
    /// - `device_lost`: GPU のデバイスを失った。
    ///
    /// 補足の値は、番号と大きさが数 ([`FieldValue::Index`] と同じ)、ほかが文字列 ([`FieldValue::Text`])。
    pub fn info(&self) -> ErrorInfo {
        use FieldValue::{Index, Text};
        let image = |code, image, index: usize, width: u32, height: u32, max: u32| ErrorInfo {
            code,
            fields: vec![
                ("image", field_json(Text(image))),
                ("texture", field_json(Index(index))),
                ("width", width.into()),
                ("height", height.into()),
                ("max", max.into()),
            ],
        };
        let decode = |image, index| ErrorInfo {
            code: "invalid_texture",
            fields: vec![
                ("image", field_json(Text(image))),
                ("texture", field_json(Index(index))),
            ],
        };
        let plain = |code| ErrorInfo {
            code,
            fields: Vec::new(),
        };
        match self {
            Self::Model(e) => model_error_info(e),
            &Self::Texture { index, .. } => decode("texture", index),
            &Self::NormalMap { index, .. } => decode("normal_map", index),
            &Self::TextureTooLarge {
                index,
                width,
                height,
                max,
            } => image(
                Self::IMAGE_LIMIT_EXCEEDED,
                "texture",
                index,
                width,
                height,
                max,
            ),
            &Self::NormalMapTooLarge {
                index,
                width,
                height,
                max,
            } => image(
                Self::IMAGE_LIMIT_EXCEEDED,
                "normal_map",
                index,
                width,
                height,
                max,
            ),
            &Self::Render(RenderError::TextureTooLarge {
                index,
                width,
                height,
                max,
            }) => image(
                "texture_too_large",
                "texture",
                index as usize,
                width,
                height,
                max,
            ),
            &Self::Render(RenderError::NormalMapTooLarge {
                index,
                width,
                height,
                max,
            }) => image(
                "texture_too_large",
                "normal_map",
                index as usize,
                width,
                height,
                max,
            ),
            Self::Render(_) | Self::Register(_) => plain("texture"),
            Self::DeviceLost => plain("device_lost"),
        }
    }

    /// 読み込みの上限 ([`LoadLimits`]) を超えた失敗か (`read_limit_exceeded`・`image_limit_exceeded`)。
    /// ファイルが壊れているとは限らない (上限を上げれば読めることがある)。
    pub fn exceeds_limit(&self) -> bool {
        matches!(
            self,
            Self::TextureTooLarge { .. } | Self::NormalMapTooLarge { .. }
        ) || matches!(self, Self::Model(e) if e.exceeds_limit())
    }
}

/// ファイルとモデルの失敗 ([`ugoe::LoadError`]) の種類と補足の値。SDK の値を JS の値にする:
/// 数 ([`LoadErrorValue::Number`]) は数、文は文字列 ([`LoadError::info`])。
fn model_error_info(error: &ugoe::LoadError) -> ErrorInfo {
    ErrorInfo {
        code: error.code(),
        fields: error
            .fields()
            .into_iter()
            .map(|(key, value)| {
                let value = match value {
                    // 上限のバイト数は wasm32 の `usize` に収まらないことがあるので、`u64` のまま数にする。
                    LoadErrorValue::Number(n) => n.into(),
                    LoadErrorValue::Text(s) => s.into(),
                };
                (key, value)
            })
            .collect(),
    }
}

/// JS に渡すエラーの中身: 機械が読む `code` と、補足の値 (`module`・`slot`・`max` など)。
/// 文は元のエラーの `Display` (英語)。補足の値は文字列か数 (読み込みの警告と同じく、番号や大きさは数にする)。
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorInfo {
    pub code: &'static str,
    pub fields: Vec<(&'static str, serde_json::Value)>,
}

/// 読み込みの上限 (`load_bundle` / `load_module` の `limits_json`。`docs/spec/file-formats.md#読み込みの上限`)。
///
/// 上限はファイル形式の決まりではなく、組み込む側の方針。展開の上限 (`read`) は SDK と C ABI と同じ、
/// PNG の一辺の上限 (`image`) は wasm だけのもの (C ABI は画像をデコードしない)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadLimits {
    pub read: ReadLimits,
    pub image: ImageLimits,
}

/// `limits_json` の形。書かなかった上限は既定値。知らない項目は誤り (綴りの誤りで上限が効かないのを防ぐ)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsJson {
    max_chunk_decompressed: Option<u64>,
    max_total_decompressed: Option<u64>,
    max_image_side: Option<u32>,
}

impl LoadLimits {
    /// JSON `{"max_chunk_decompressed"?, "max_total_decompressed"?, "max_image_side"?}` (バイトと画素) を読む。
    /// `None` は既定値。
    pub fn from_json(json: Option<&str>) -> Result<Self, serde_json::Error> {
        let mut limits = Self::default();
        let Some(json) = json else {
            return Ok(limits);
        };
        let LimitsJson {
            max_chunk_decompressed,
            max_total_decompressed,
            max_image_side,
        } = serde_json::from_str(json)?;
        if let Some(bytes) = max_chunk_decompressed {
            limits.read = limits.read.with_max_chunk_decompressed(bytes);
        }
        if let Some(bytes) = max_total_decompressed {
            limits.read = limits.read.with_max_total_decompressed(bytes);
        }
        if let Some(pixels) = max_image_side {
            limits.image = limits.image.with_max_side(pixels);
        }
        Ok(limits)
    }
}

/// RGBA8 の画像 (`ugoe_format::image` と共有。テクスチャは乗算済み、法線マップはアルファを掛けない)。
pub use ugoe_format::image::Rgba8;

/// テクスチャが無いときに使う 1x1 の不透明な白 (`ugoe render` と同じ)。
pub fn placeholder() -> Rgba8 {
    Rgba8 {
        width: 1,
        height: 1,
        pixels: vec![255; 4],
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

/// 読み込めたが注意が要ること。種類 (`code`) と補足の値は SDK の [`ugoe::LoadWarning`] が決める (T0412)。
/// `Display` は読み込みの戻り値に入る英語の文 (T0125 の前から変えていない)。
pub use ugoe::LoadWarning;

/// JSON: `{"code", "warning", "texture"?, "reason"?, "feature"?, "path"?}` (`load_warnings_json` の要素。
/// `crates/ugoe-wasm/README.md#読み込みの警告-t0125`)。`warning` は文の警告と同じ文字列。番号の補足の値は数にする。
pub fn warning_json(warning: &LoadWarning) -> serde_json::Value {
    let mut json = serde_json::json!({
        "code": warning.code(),
        "warning": warning.to_string(),
    });
    for (key, value) in warning.fields() {
        json[key] = field_json(value);
    }
    json
}

/// 補足の値 (警告と着せ替えの失敗の `fields`) の JSON。番号は数、文は文字列。
pub fn field_json(value: FieldValue<'_>) -> serde_json::Value {
    match value {
        FieldValue::Index(i) => i.into(),
        FieldValue::Text(s) => s.into(),
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
        .map(warning_json)
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
    let textures = vec![placeholder(); texture_count(&model)];
    Ok(LoadedModel {
        model,
        animation: None,
        textures,
        normal_maps: Vec::new(),
        warnings: Vec::new(),
    })
}

/// `.ugoe` バンドルを既定の上限で読み、PNG のテクスチャと法線マップをデコードする。
pub fn from_bundle(bytes: &[u8]) -> Result<LoadedModel, LoadError> {
    from_bundle_with(bytes, LoadLimits::default())
}

/// 読み込みの上限 `limits` を指定して `.ugoe` バンドルを読み、PNG のテクスチャと法線マップをデコードする。
pub fn from_bundle_with(bytes: &[u8], limits: LoadLimits) -> Result<LoadedModel, LoadError> {
    let (bundle, report) = ugoe::format::read_bundle_with(bytes, limits.read)?;
    // 読み込みの報告の警告を先に並べる (C ABI の `ugoe_puppet_load_warning_*` と同じ順。T0412)。
    let mut warnings: Vec<LoadWarning> = report.warnings.iter().map(LoadWarning::from).collect();
    let normal_maps = decode_normal_maps(
        ugoe::lighting::bundle_normal_maps(&bundle),
        limits.image,
        &mut warnings,
    )?;
    let textures = decode_textures(
        texture_count(&bundle.model),
        |i| bundle.textures.get(i),
        limits.image,
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

/// テクスチャ `count` 枚をデコードする (`data(i)` が `i` 番目の画像)。PNG と KTX2 は乗算済みアルファにし、
/// デコードしない形式の KTX2 (ETC2・ASTC・Basis Universal など) と画像の無い番号は白のプレースホルダで代用して警告する。
pub(crate) fn decode_textures<'a>(
    count: usize,
    data: impl Fn(usize) -> Option<&'a TextureData>,
    limits: ImageLimits,
    warnings: &mut Vec<LoadWarning>,
) -> Result<Vec<Rgba8>, LoadError> {
    (0..count)
        .map(|index| match data(index) {
            Some(TextureData { format, bytes }) => {
                match decode_image_premultiplied(*format, bytes, limits) {
                    Ok(image) => Ok(image),
                    // デコードしない形式の KTX2 (`ugoe render` はエラー)。表示は止めずに白で代用する。
                    Err(ImageError::UnsupportedKtx2(_)) => {
                        warnings.push(LoadWarning::TextureKtx2 { texture: index });
                        Ok(placeholder())
                    }
                    Err(ImageError::TooLarge { width, height, max }) => {
                        Err(LoadError::TextureTooLarge {
                            index,
                            width,
                            height,
                            max,
                        })
                    }
                    Err(e) => Err(LoadError::Texture {
                        index,
                        message: e.to_string(),
                    }),
                }
            }
            // 読み込み (`read_bundle` / `read_module`) がテクスチャの数を検証するので、通常は起きない。
            None => {
                warnings.push(LoadWarning::TextureMissing { texture: index });
                Ok(placeholder())
            }
        })
        .collect()
}

/// テクスチャの番号ごとの法線マップ (`None` は法線マップなし) を、アルファを掛けずにデコードする。
/// デコードしない形式の KTX2 は警告して使わない。
pub(crate) fn decode_normal_maps<'a>(
    data: impl IntoIterator<Item = Option<&'a TextureData>>,
    limits: ImageLimits,
    warnings: &mut Vec<LoadWarning>,
) -> Result<Vec<Option<Rgba8>>, LoadError> {
    data.into_iter()
        .enumerate()
        .map(|(index, data)| match data {
            None => Ok(None),
            Some(TextureData { format, bytes }) => match decode_image(*format, bytes, limits) {
                Ok(image) => Ok(Some(image)),
                Err(ImageError::UnsupportedKtx2(_)) => {
                    warnings.push(LoadWarning::NormalMapKtx2 { texture: index });
                    Ok(None)
                }
                Err(ImageError::TooLarge { width, height, max }) => {
                    Err(LoadError::NormalMapTooLarge {
                        index,
                        width,
                        height,
                        max,
                    })
                }
                Err(e) => Err(LoadError::NormalMap {
                    index,
                    message: e.to_string(),
                }),
            },
        })
        .collect()
}

/// 描画に使うテクスチャの枚数 (`model.textures` の数)。
///
/// メッシュが参照する番号は数えない。範囲外の番号はモデルの検証 (`Puppet::new`) が誤りにするが、
/// テクスチャはその前に用意するので、番号で数えると JSON の `"texture": 4294967295` のような値で巨大な確保になる。
pub fn texture_count(model: &Model) -> usize {
    model.textures.len()
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

/// ID のパラメータに `value` を書く (`WebPuppet::set_parameter`)。ID が無ければ `false`。
///
/// 値域にクランプする (無限大は端の値になる)。NaN は書かずに `true` を返す (`ParameterValues::set`、
/// `docs/spec/evaluation.md`。JS の入口で例外を増やさない。`README.md#有限でない値と-dt`)。
pub fn set_parameter(instance: &mut ModelInstance, id: &str, value: f32) -> bool {
    let params = instance.parameters_mut();
    let Some(index) = params.index_of(id) else {
        return false;
    };
    params.set(index, value);
    true
}

/// デコードする画像の一辺の上限 (`ugoe_format::image::DEFAULT_MAX_IMAGE_SIDE`。`ugoe_headless` の `MAX_SIZE` と同じ)。
pub use ugoe_format::image::DEFAULT_MAX_IMAGE_SIDE as MAX_IMAGE_SIZE;

/// PNG をデコードして、乗算済みアルファの RGBA8 にする (`ugoe_format::image::decode_png_premultiplied`)。
///
/// グレースケール・パレット・16bit は 8bit RGBA に展開する。一辺が `limits.max_side` (既定は [`MAX_IMAGE_SIZE`]) を
/// 超える画像は確保の前にエラー (デバイスの上限との比較 `check_texture_sizes` はデコードの後なので、ここで先に止める)。
pub fn decode_png_premultiplied(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    ugoe_format::image::decode_png_premultiplied(bytes, limits)
}

/// PNG をデコードして、アルファを掛けない RGBA8 にする (法線マップ用。`ugoe_format::image::decode_png`)。
///
/// 展開と上限は [`decode_png_premultiplied`] と同じ。
pub fn decode_png(bytes: &[u8], limits: ImageLimits) -> Result<Rgba8, ImageError> {
    ugoe_format::image::decode_png(bytes, limits)
}

#[cfg(test)]
mod tests {
    use ugoe::format::{TextureFormat, write_bundle};
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

    /// 圧縮していない RGBA8 (`vk_format` 37 = `_UNORM`、43 = `_SRGB`) か、ほかの `vk_format` の KTX2。
    /// レベルは 1 つで、データ形式の記述は付けない (デコードは見ない)。
    fn encode_ktx2(vk_format: u32, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut out = ugoe_format::image::KTX2_IDENTIFIER.to_vec();
        for v in [vk_format, 1, width, height, 0, 0, 1, 1, 0] {
            out.extend(v.to_le_bytes());
        }
        out.extend([0; 32]);
        for v in [80 + 24, data.len(), data.len()] {
            out.extend((v as u64).to_le_bytes());
        }
        out.extend(data);
        out
    }

    #[test]
    fn json_models_get_white_placeholders() {
        let json = ugoe::format::model_to_json(&fixtures::single_quad()).unwrap();
        let loaded = from_json(&json).unwrap();
        assert_eq!(loaded.textures, vec![placeholder()]);
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

    /// 圧縮していない KTX2 は値のまま読み、テクスチャは乗算し、法線マップは乗算しない (PNG と同じ)。
    #[test]
    fn bundle_ktx2_is_decoded_like_png() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        bundle.textures = vec![TextureData {
            format: TextureFormat::Ktx2,
            bytes: encode_ktx2(43, 2, 1, &[255, 0, 0, 255, 255, 255, 255, 128]),
        }];
        bundle.normal_maps = vec![TextureData {
            format: TextureFormat::Ktx2,
            bytes: encode_ktx2(37, 1, 1, &[128, 200, 255, 0]),
        }];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(
            loaded.textures,
            vec![Rgba8 {
                width: 2,
                height: 1,
                pixels: vec![255, 0, 0, 255, 128, 128, 128, 128],
            }]
        );
        assert_eq!(
            loaded.normal_maps,
            vec![Some(Rgba8 {
                width: 1,
                height: 1,
                pixels: vec![128, 200, 255, 0],
            })]
        );
    }

    /// ブロック圧縮 (BC3) の KTX2 は展開してから、テクスチャは乗算し、法線マップは乗算しない
    /// (`docs/adr/0030-compressed-ktx2-decoding.md`)。
    #[test]
    fn bundle_bc3_ktx2_is_decoded() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        // アルファ 128 (端点はどちらも 128) と、赤 (RGB565 の 0xf800。端点はどちらも同じ) の 4x4 の 1 ブロック。
        let block = [
            128, 128, 0, 0, 0, 0, 0, 0, 0x00, 0xf8, 0x00, 0xf8, 0, 0, 0, 0,
        ];
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: encode_ktx2(137, 4, 4, &block),
        };
        bundle.textures = vec![ktx2.clone()];
        bundle.normal_maps = vec![ktx2];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.textures[0].pixels, [128, 0, 0, 128].repeat(16));
        assert_eq!(
            loaded.normal_maps,
            vec![Some(Rgba8 {
                width: 4,
                height: 4,
                pixels: [255, 0, 0, 128].repeat(16),
            })]
        );
    }

    /// 壊れた KTX2 は読み込みの失敗。一辺の上限は PNG と同じく種類付きの失敗。
    #[test]
    fn broken_or_too_large_ktx2_is_an_error() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Ktx2,
            bytes: encode_ktx2(43, 2, 1, &[255; 4]),
        }];
        let err = from_bundle(&write_bundle(&bundle).unwrap()).unwrap_err();
        assert!(matches!(err, LoadError::Texture { index: 0, .. }), "{err}");
        assert!(err.to_string().contains("decoding KTX2"), "{err}");
        assert_eq!(err.info().code, "invalid_texture");
        bundle.textures[0].bytes = encode_ktx2(43, 3, 1, &[255; 12]);
        let limits = LoadLimits {
            image: ImageLimits::default().with_max_side(2),
            ..LoadLimits::default()
        };
        let err = from_bundle_with(&write_bundle(&bundle).unwrap(), limits).unwrap_err();
        assert!(
            matches!(
                err,
                LoadError::TextureTooLarge {
                    index: 0,
                    width: 3,
                    height: 1,
                    max: 2,
                }
            ),
            "{err}"
        );
        assert_eq!(err.info().code, LoadError::IMAGE_LIMIT_EXCEEDED);
    }

    /// デコードしない形式の KTX2 (ここでは `VK_FORMAT_ETC2_R8G8B8A8_UNORM_BLOCK`) は、白と法線マップなしで代用して警告する。
    #[test]
    fn unsupported_ktx2_images_are_replaced_with_structured_warnings() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        let ktx2 = TextureData {
            format: TextureFormat::Ktx2,
            bytes: encode_ktx2(151, 4, 4, &[0; 16]),
        };
        bundle.textures = vec![ktx2.clone()];
        bundle.normal_maps = vec![ktx2];
        let loaded = from_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(loaded.textures, vec![placeholder()]);
        assert_eq!(loaded.normal_maps, vec![None]);
        assert_eq!(
            loaded.warnings,
            [
                LoadWarning::NormalMapKtx2 { texture: 0 },
                LoadWarning::TextureKtx2 { texture: 0 },
            ]
        );
        // 構造には文の警告と同じ文字列を `warning` に入れる。
        assert_eq!(
            loaded.warnings[0].to_string(),
            "the normal map of texture 0 is KTX2 in a format that is not supported yet; drawing without it"
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
                    "warning": "texture 0 is KTX2 in a format that is not supported yet; using a white placeholder",
                    "texture": 0,
                }),
            ]
        );
    }

    /// 読み込みの報告の警告の構造 (T0412)。C ABI の `ugoe_puppet_load_warning_*` と同じ code・補足の値・文。
    fn read_warnings_json(report: &ugoe::format::ReadReport) -> Vec<serde_json::Value> {
        use ugoe::format::ReadWarning;
        report
            .warnings
            .iter()
            .map(|w| match w {
                ReadWarning::UnknownOptionalFeature(id) => serde_json::json!({
                    "code": "unknown_optional_feature",
                    "warning": w.to_string(),
                    "feature": id,
                }),
                ReadWarning::UnknownValue(path) => serde_json::json!({
                    "code": "unknown_value",
                    "warning": w.to_string(),
                    "path": path,
                }),
            })
            .collect()
    }

    #[test]
    fn unknown_features_and_values_are_reported_like_the_c_abi() {
        let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoe");
        let (_, report) = ugoe::format::read_bundle_with_report(bytes).unwrap();
        let loaded = from_bundle(bytes).unwrap();
        let expected = read_warnings_json(&report);
        assert!(!expected.is_empty());
        assert_eq!(warnings_json(&loaded.warnings, &[]), expected);
        let codes: Vec<_> = loaded.warnings.iter().map(LoadWarning::code).collect();
        assert!(codes.contains(&"unknown_optional_feature"), "{codes:?}");
        assert!(codes.contains(&"unknown_value"), "{codes:?}");

        let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoemod");
        let (_, report) = ugoe::format::module_file::read_module_with_report(bytes).unwrap();
        let loaded = crate::dress_up::from_module_file(bytes).unwrap();
        let details: Vec<_> = loaded.warnings.iter().map(warning_json).collect();
        assert!(!details.is_empty());
        assert_eq!(details, read_warnings_json(&report));
    }

    #[test]
    fn missing_textures_and_removed_physics_have_codes() {
        let mut warnings = Vec::new();
        let textures = decode_textures(2, |_| None, ImageLimits::default(), &mut warnings).unwrap();
        assert_eq!(textures, vec![placeholder(); 2]);
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

    /// テクスチャはモデルの検証の前に用意するので、メッシュの範囲外の番号で確保を大きくしない。
    #[test]
    fn out_of_range_mesh_textures_do_not_grow_the_textures() {
        let mut model = fixtures::single_quad();
        model.meshes[0].texture = u32::MAX;
        assert_eq!(texture_count(&model), model.textures.len());
        let loaded = from_json(&ugoe::format::model_to_json(&model).unwrap()).unwrap();
        assert_eq!(loaded.textures.len(), model.textures.len());
        // 範囲外の番号は読み込み (`Puppet::new`) で誤りになる。
        assert!(ugoe::Puppet::new(loaded.model).is_err());
    }

    /// IHDR が 65535x65535 で、画素のデータが zlib の見出しだけの小さな PNG。
    fn huge_png_header() -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 65535, 65535);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::IDAT, &[0x78, 0x9c]).unwrap();
        // 残りの画素は書かない (`finish` も呼ばない)。
        std::mem::forget(writer);
        out
    }

    /// 見出しの大きさを確かめてから確保する (外から来た PNG で約 17GB を確保しようとして落ちないように)。
    #[test]
    fn rejects_images_larger_than_the_limit_before_allocating() {
        let err = decode_png(&huge_png_header(), ImageLimits::default()).unwrap_err();
        assert!(err.to_string().contains("65535x65535"), "{err}");
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
    fn limits_json_overrides_only_the_given_limits() {
        assert_eq!(LoadLimits::from_json(None).unwrap(), LoadLimits::default());
        assert_eq!(
            LoadLimits::from_json(Some("{}")).unwrap(),
            LoadLimits::default()
        );
        let limits = LoadLimits::from_json(Some(
            r#"{"max_total_decompressed": 1024, "max_image_side": 64}"#,
        ))
        .unwrap();
        assert_eq!(
            limits.read,
            ReadLimits::default().with_max_total_decompressed(1024)
        );
        assert_eq!(limits.image, ImageLimits::default().with_max_side(64));
        // 綴りの誤り・負の値・小数は誤り (上限が黙って効かなくならないように)。
        for json in [
            r#"{"max_total": 1}"#,
            r#"{"max_image_side": -1}"#,
            r#"{"max_chunk_decompressed": 1.5}"#,
            "[]",
        ] {
            assert!(LoadLimits::from_json(Some(json)).is_err(), "{json}");
        }
    }

    /// 小さな展開の上限で読むと、SDK と C ABI と同じ種類と補足の値の失敗になる (T0152)。
    #[test]
    fn bundles_over_the_read_limits_are_rejected_with_a_code() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(1, 1, &[255; 4]),
        }];
        let bytes =
            ugoe::format::write_bundle_with(&bundle, ugoe::format::WriteOptions { compress: true })
                .unwrap();
        let limits = LoadLimits {
            read: ReadLimits::default().with_max_total_decompressed(1),
            ..LoadLimits::default()
        };
        let err = from_bundle_with(&bytes, limits).unwrap_err();
        let info = err.info();
        assert_eq!(info.code, "read_limit_exceeded");
        let field = |key: &str| {
            info.fields
                .iter()
                .find(|(k, _)| *k == key)
                .unwrap()
                .1
                .clone()
        };
        assert_eq!(field("limit"), "total_decompressed");
        assert_eq!(field("max"), 1);
        assert!(field("chunk").is_string());
        assert!(from_bundle_with(&bytes, LoadLimits::default()).is_ok());
        assert!(err.exceeds_limit());
    }

    /// PNG の一辺の上限を指定でき、超えるテクスチャと法線マップは画素を確保する前に種類付きの失敗になる。
    #[test]
    fn images_over_the_side_limit_are_rejected_with_a_code() {
        let mut model = fixtures::single_quad();
        model.textures[0].normal_map = Some("n".into());
        let mut bundle = Bundle::new(model);
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(1, 1, &[255; 4]),
        }];
        bundle.normal_maps = vec![TextureData {
            format: TextureFormat::Png,
            bytes: encode_png(2, 1, &[128, 128, 255, 255, 128, 128, 255, 255]),
        }];
        let limits = |side| LoadLimits {
            image: ImageLimits::default().with_max_side(side),
            ..LoadLimits::default()
        };
        let bytes = write_bundle(&bundle).unwrap();
        assert!(from_bundle_with(&bytes, limits(2)).is_ok());
        let err = from_bundle_with(&bytes, limits(1)).unwrap_err();
        assert!(matches!(
            err,
            LoadError::NormalMapTooLarge {
                index: 0,
                width: 2,
                height: 1,
                max: 1,
            }
        ));
        assert!(err.exceeds_limit());
        assert_eq!(
            err.info(),
            ErrorInfo {
                code: "image_limit_exceeded",
                fields: vec![
                    ("image", "normal_map".into()),
                    ("texture", 0.into()),
                    ("width", 2.into()),
                    ("height", 1.into()),
                    ("max", 1.into()),
                ],
            }
        );

        bundle.normal_maps[0].bytes = encode_png(1, 1, &[128, 128, 255, 255]);
        bundle.textures[0].bytes = encode_png(1, 3, &[255; 12]);
        let err = from_bundle_with(&write_bundle(&bundle).unwrap(), limits(2)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "texture 0 is 1x3, larger than the limit 2x2"
        );
        assert_eq!(err.info().fields[0], ("image", "texture".into()));
    }

    /// 読み込みのすべての失敗が `code` と補足の値を持つ (T0434)。表は README の「読み込みの失敗」。
    #[test]
    fn every_load_error_has_a_code() {
        use ugoe::format::FormatError;
        use ugoe::model::validate::ValidationError;
        use ugoe::runtime::CompileError;
        let validation = || {
            vec![ValidationError::InvalidValue {
                field: "a".into(),
                value: -1.0,
                expected: ">= 0",
            }]
        };
        let info =
            |code, fields: Vec<(&'static str, serde_json::Value)>| ErrorInfo { code, fields };
        let cases: Vec<(LoadError, ErrorInfo)> = vec![
            (
                from_bundle(b"not a bundle").unwrap_err(),
                info("invalid_file", vec![]),
            ),
            (from_json("{").unwrap_err(), info("invalid_file", vec![])),
            (
                FormatError::UnsupportedVersion {
                    found: 99,
                    supported: 3,
                }
                .into(),
                info(
                    "unsupported_version",
                    vec![("found", 99.into()), ("supported", 3.into())],
                ),
            ),
            (
                FormatError::UnsupportedFeatures(vec!["x.a".into(), "x.b".into()]).into(),
                info(
                    "unsupported_features",
                    vec![("features", "x.a, x.b".into())],
                ),
            ),
            (
                FormatError::InvalidPhysics(validation()).into(),
                info("invalid_model", vec![("errors", 1.into())]),
            ),
            (
                CompileError::Invalid(validation()).into(),
                info("invalid_model", vec![("errors", 1.into())]),
            ),
            (
                CompileError::Unsupported("zero-row warp").into(),
                info("compile", vec![]),
            ),
            (
                LoadError::NormalMap {
                    index: 2,
                    message: "broken".into(),
                },
                info(
                    "invalid_texture",
                    vec![("image", "normal_map".into()), ("texture", 2.into())],
                ),
            ),
            (
                RenderError::TextureTooLarge {
                    index: 1,
                    width: 5,
                    height: 6,
                    max: 4,
                }
                .into(),
                info(
                    "texture_too_large",
                    vec![
                        ("image", "texture".into()),
                        ("texture", 1.into()),
                        ("width", 5.into()),
                        ("height", 6.into()),
                        ("max", 4.into()),
                    ],
                ),
            ),
            (
                RenderError::MissingTexture(0).into(),
                info("texture", vec![]),
            ),
            (LoadError::Register("full".into()), info("texture", vec![])),
            (LoadError::DeviceLost, info("device_lost", vec![])),
        ];
        for (error, expected) in cases {
            assert_eq!(error.info(), expected, "{error}");
            assert!(!error.exceeds_limit(), "{error}");
        }
        // デコードできない PNG は、どの画像の何番かを持つ。
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0],
        }];
        let err = from_bundle(&write_bundle(&bundle).unwrap()).unwrap_err();
        assert_eq!(
            err.info(),
            info(
                "invalid_texture",
                vec![("image", "texture".into()), ("texture", 0.into())],
            )
        );
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
