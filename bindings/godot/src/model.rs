//! Godot に依存しない部分: バンドルとモジュールの読み込み、マスクの段取りとアトラス、光源の変換、
//! フレームごとの幾何の準備。
//!
//! Godot の型を使わないので、エンジンなしの `cargo test` で確かめられる。
//! 描画の式は `docs/spec/rendering.md`。

use std::collections::HashMap;
use std::sync::Arc;

use ugoe::core::{Rgb, Vec2};
use ugoe::format::image::{ImageError, ImageLimits, Rgba8};
use ugoe::format::module_file::read_module_with;
use ugoe::format::{Bundle, FormatError, TextureData, TextureFormat};
use ugoe::lighting::{Light, Lighting, MAX_LIGHTS};
use ugoe::limits::{ReadLimitExceeded, ReadLimits};
use ugoe::model::expression::Expression;
use ugoe::model::module::Module;
use ugoe::model::motion::Motion;
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::ValidationError;
use ugoe::model::{MaskMode, Model};
use ugoe::runtime::{CompileError, CompiledModel};
use ugoe::{DressUpChange, DressUpError, FieldValue, LoadWarning, Puppet, TextureSource};

/// 画像の一辺の上限の既定値 (画素)。`ugoe_format::image::DEFAULT_MAX_IMAGE_SIDE` (wasm の `max_image_side` の既定値) と同じ。
pub const DEFAULT_MAX_IMAGE_SIDE: u32 = 16384;

/// 読み込みの上限 (`docs/spec/file-formats.md#読み込みの上限`)。上限はファイル形式の決まりではなく、組み込む側の方針。
///
/// 展開の上限 (`read`) は SDK・C ABI と同じ。画像の一辺の上限 (`max_image_side`) は wasm の `max_image_side` と同じ意味で、
/// 画像はデコードする前に (読み込みのときに) PNG と KTX2 の見出しの大きさで調べる ([`image_size`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadLimits {
    pub read: ReadLimits,
    pub max_image_side: u32,
}

impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            read: ReadLimits::default(),
            max_image_side: DEFAULT_MAX_IMAGE_SIDE,
        }
    }
}

/// 上限を調べる画像の種類 (エラーの補足の値 `image`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Texture,
    NormalMap,
}

impl ImageKind {
    /// 補足の値 `image` (`"texture"` | `"normal_map"`。wasm の `image_limit_exceeded` と同じ)。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::NormalMap => "normal_map",
        }
    }
}

/// 読み込みの失敗。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read the file: {0}")]
    Format(#[from] FormatError),
    #[error("failed to compile the model: {0}")]
    Compile(#[from] CompileError),
    /// 画像の一辺が上限 ([`LoadLimits::max_image_side`]) を超える。画像はデコードしていない。
    #[error(
        "the {} of texture {texture} is {width}x{height}, larger than the limit {max}x{max}",
        if *image == ImageKind::Texture { "image" } else { "normal map" }
    )]
    ImageTooLarge {
        image: ImageKind,
        /// テクスチャの番号 (本体なら `Model::textures`、モジュールなら `Module::textures` の添字)。
        texture: usize,
        width: u32,
        height: u32,
        max: u32,
    },
}

/// エラーの補足の値 ([`LoadError::info`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorField {
    Int(i64),
    Text(String),
}

impl LoadError {
    /// 画像の一辺の上限を超えたときの種類 (wasm の `image_limit_exceeded` と同じ)。
    pub const IMAGE_LIMIT_EXCEEDED: &'static str = "image_limit_exceeded";

    /// 機械が読む種類 (`code`) と補足の値。値は wasm の `LoadError::info` (`crates/ugoe-wasm/README.md`) と同じ:
    ///
    /// - `read_limit_exceeded`: `limit` (`"chunk_decompressed"` | `"total_decompressed"`)・`chunk` (タグ)・`max` (バイト数)。
    /// - `image_limit_exceeded`: `image` (`"texture"` | `"normal_map"`)・`texture`・`width`・`height`・`max` (画素)。
    /// - `unsupported_version`: `found`・`supported`。`unsupported_features`: `features` (`", "` でつないだ ID)。
    /// - `invalid_model`: `errors` (検査のエラーの数)・`error_codes` (その種類を `", "` でつないだもの。SDK の
    ///   [`ugoe::validation_fields`]。T0470)。`compile`: 評価できない構造。
    /// - `invalid_file`: ほかの読めないファイル (壊れている・ugoe のファイルでないなど)。
    pub fn info(&self) -> (&'static str, Vec<(&'static str, ErrorField)>) {
        use ErrorField::{Int, Text};
        let count = |n: usize| Int(i64::try_from(n).unwrap_or(i64::MAX));
        let invalid = |errors: &[ValidationError]| {
            let fields = ugoe::validation_fields(errors)
                .into_iter()
                .map(|(key, value)| match value {
                    FieldValue::Index(n) => (key, count(n)),
                    value => (key, Text(value.to_string())),
                })
                .collect();
            ("invalid_model", fields)
        };
        match self {
            Self::Format(e) => {
                if let Some(exceeded) = ReadLimitExceeded::from_error(e) {
                    return (
                        ReadLimitExceeded::CODE,
                        vec![
                            ("limit", Text(exceeded.limit.as_str().to_owned())),
                            ("chunk", Text(exceeded.chunk.to_string())),
                            ("max", Int(i64::try_from(exceeded.max).unwrap_or(i64::MAX))),
                        ],
                    );
                }
                if let Some(errors) = e.validation_errors() {
                    return invalid(errors);
                }
                match e {
                    &FormatError::UnsupportedVersion { found, supported }
                    | &FormatError::UnsupportedContainerVersion { found, supported } => (
                        "unsupported_version",
                        vec![
                            ("found", Int(found.into())),
                            ("supported", Int(supported.into())),
                        ],
                    ),
                    FormatError::UnsupportedFeatures(features) => (
                        "unsupported_features",
                        vec![("features", Text(features.join(", ")))],
                    ),
                    _ => (DressUpError::INVALID_FILE, Vec::new()),
                }
            }
            Self::Compile(CompileError::Invalid(errors)) => invalid(errors),
            Self::Compile(_) => ("compile", Vec::new()),
            &Self::ImageTooLarge {
                image,
                texture,
                width,
                height,
                max,
            } => (
                Self::IMAGE_LIMIT_EXCEEDED,
                vec![
                    ("image", Text(image.as_str().to_owned())),
                    ("texture", count(texture)),
                    ("width", Int(width.into())),
                    ("height", Int(height.into())),
                    ("max", Int(max.into())),
                ],
            ),
        }
    }
}

/// PNG の署名。
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// KTX2 の識別子 (`«KTX 20»\r\n\x1A\n`)。
const KTX2_IDENTIFIER: [u8; 12] = [
    0xab, b'K', b'T', b'X', b' ', b'2', b'0', 0xbb, 0x0d, 0x0a, 0x1a, 0x0a,
];

/// 画像の幅と高さを見出しから読む (デコードしない)。PNG は最初のチャンク `IHDR`、KTX2 は `pixelWidth`・`pixelHeight`。
/// 見出しが読めなければ `None` (上限は調べず、デコーダに任せる。読めなければ白で描く)。
pub fn image_size(format: TextureFormat, bytes: &[u8]) -> Option<(u32, u32)> {
    let read = |at: usize, big_endian: bool| -> Option<u32> {
        let b: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        Some(if big_endian {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        })
    };
    match format {
        TextureFormat::Png => {
            if !bytes.starts_with(&PNG_SIGNATURE) || bytes.get(12..16)? != b"IHDR" {
                return None;
            }
            Some((read(16, true)?, read(20, true)?))
        }
        TextureFormat::Ktx2 => {
            if !bytes.starts_with(&KTX2_IDENTIFIER) {
                return None;
            }
            Some((read(20, false)?, read(24, false)?))
        }
    }
}

/// テクスチャや法線マップの画像 (PNG・KTX2) をデコードして、乗算していないアルファの RGBA8 にする。
///
/// Godot の読み込み (`Image.load_png_from_buffer`・`load_ktx_from_buffer`) は使わず、参照と同じデコーダ
/// (`ugoe::format::image::decode_image`) で読む。仕様では PNG のガンマや色のチャンクも KTX2 の
/// `_SRGB` や転送関数も見ずに保存された値のまま使い、PNG の 16 ビットは上位の 8 ビットに、KTX2 の BCn は CPU で展開する
/// (`docs/spec/rendering.md#前提`)。Godot の読み込みは PNG の `gAMA` や KTX2 の sRGB で値を変え、アルファの無い BC1 の
/// 3 色のモードの 4 つ目の色を透明にする (T0477・T0481)。
/// 一辺の上限は読み込みのときに見出しで調べてある ([`check_image_sizes`]) ので、ここでは制限しない。
pub fn decode_image(format: TextureFormat, bytes: &[u8]) -> Result<Rgba8, ImageError> {
    ugoe::format::image::decode_image(
        format,
        bytes,
        ImageLimits::default().with_max_side(u32::MAX),
    )
}

/// テクスチャ (`textures`) と法線マップ (`normal_maps`。どちらもテクスチャの番号の順) の一辺が `max` を超えないか調べる。
fn check_image_sizes<'a>(
    textures: impl IntoIterator<Item = Option<&'a TextureData>>,
    normal_maps: impl IntoIterator<Item = Option<&'a TextureData>>,
    max: u32,
) -> Result<(), LoadError> {
    let images = textures
        .into_iter()
        .enumerate()
        .map(|(i, t)| (ImageKind::Texture, i, t))
        .chain(
            normal_maps
                .into_iter()
                .enumerate()
                .map(|(i, t)| (ImageKind::NormalMap, i, t)),
        );
    for (image, texture, data) in images {
        let Some((width, height)) = data.and_then(|d| image_size(d.format, &d.bytes)) else {
            continue;
        };
        if width > max || height > max {
            return Err(LoadError::ImageTooLarge {
                image,
                texture,
                width,
                height,
                max,
            });
        }
    }
    Ok(())
}

/// 要素が読むマスク 1 つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskSlot {
    /// [`MaskPlan::sources`] の番号 (マスクのカバレッジの番号)。
    pub source: usize,
    pub invert: bool,
}

/// 描画可能要素の、評価で変わらない情報。
#[derive(Debug, Clone)]
pub struct DrawableInfo {
    pub id: String,
    /// テクスチャの番号 (`LoadedModel::texture`)。
    pub texture: usize,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u16>,
    pub culling: bool,
    /// 読むマスク (同じメッシュ・同じ `invert` の重複は除く)。数に上限はない。
    pub masks: Vec<MaskSlot>,
    pub mask_mode: MaskMode,
    /// ライティングを受けるか (`Model::lighting` と `Mesh::lit` の両方が true)。
    pub lit: bool,
}

/// マスクの段取り: どのメッシュのカバレッジを、どの段のアトラスに描くか。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaskPlan {
    /// マスクとして参照されるメッシュの番号。入れ子のマスクが先に来る順 (依存するものが先)。
    pub sources: Vec<usize>,
    /// `sources` ごとの段。自身がマスクを読まなければ 0、読むなら読むマスクの段の最大 + 1。
    /// 段 `k` のカバレッジは、段 `k − 1` までのカバレッジを収めたアトラスを読んで描く。
    pub levels: Vec<u32>,
}

impl MaskPlan {
    /// アトラスの段の数 (マスクが無ければ 0)。
    pub fn level_count(&self) -> usize {
        self.levels.iter().max().map_or(0, |&l| l as usize + 1)
    }
}

/// 着せ替えのモジュール (`.ugoemod`) 1 つ分: モジュールと、その画像。
#[derive(Debug)]
pub struct ModuleData {
    pub module: Arc<Module>,
    /// `module.textures` と同じ順。
    textures: Vec<TextureData>,
    /// `module.textures` と同じ順の法線マップ (無ければ `None`)。
    normal_maps: Vec<Option<TextureData>>,
    /// 読み込みの警告 (知らない任意の機能・未知の値。ADR 0013)。
    pub warnings: Vec<LoadWarning>,
}

impl ModuleData {
    /// `.ugoemod` のバイト列から、読み込みの上限 `limits` で読む。
    pub fn from_bytes(bytes: &[u8], limits: LoadLimits) -> Result<Self, LoadError> {
        let (file, report) = read_module_with(bytes, limits.read)?;
        let normal_maps: Vec<Option<TextureData>> = (0..file.module.textures.len())
            .map(|i| file.normal_map(i).cloned())
            .collect();
        check_image_sizes(
            file.textures.iter().map(Some),
            normal_maps.iter().map(Option::as_ref),
            limits.max_image_side,
        )?;
        Ok(Self {
            module: Arc::new(file.module),
            textures: file.textures,
            normal_maps,
            warnings: report.warnings.iter().map(LoadWarning::from).collect(),
        })
    }

    pub fn name(&self) -> &str {
        &self.module.name
    }
}

/// 読み込んだモデル 1 体分。
#[derive(Debug)]
pub struct LoadedModel {
    pub puppet: Puppet,
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
    /// 本体のテクスチャの番号ごとの画像。バンドルに無い番号は `None` (白で代用する)。
    base_textures: Vec<Option<TextureData>>,
    /// 本体のテクスチャの番号ごとの法線マップ。
    base_normal_maps: Vec<Option<TextureData>>,
    /// 取り付けているモジュール (`Puppet::modules` と同じ順)。
    modules: Vec<Arc<ModuleData>>,
    /// 今のモデルのテクスチャごとの出どころ (`Puppet::texture_sources`)。
    sources: Vec<TextureSource>,
    pub drawables: Vec<DrawableInfo>,
    pub masks: MaskPlan,
    /// 読み込みの警告 (知らない任意の機能・未知の値 (ADR 0013)・画像の無いテクスチャ)。
    pub warnings: Vec<LoadWarning>,
}

impl LoadedModel {
    /// `.ugoe` バンドルのバイト列から、読み込みの上限 `limits` で読み込む。
    pub fn from_bundle_bytes(bytes: &[u8], limits: LoadLimits) -> Result<Self, LoadError> {
        let (bundle, report) = ugoe::format::read_bundle_with(bytes, limits.read)?;
        let mut loaded = Self::from_bundle(bundle)?;
        check_image_sizes(
            loaded.base_textures.iter().map(Option::as_ref),
            loaded.base_normal_maps.iter().map(Option::as_ref),
            limits.max_image_side,
        )?;
        let read = report.warnings.iter().map(LoadWarning::from);
        loaded.warnings.splice(0..0, read);
        Ok(loaded)
    }

    /// 読み込んだバンドルから作る。物理・モーション・表情・テクスチャ・法線マップも使う。
    /// 画像の無いテクスチャ (白で描く) は警告にする (モデルの JSON から作ったときは画像が無いのが前提なので、呼ぶ側が捨てる)。
    pub fn from_bundle(bundle: Bundle) -> Result<Self, LoadError> {
        let count = texture_count(&bundle.model);
        let base_normal_maps = (0..count).map(|i| bundle.normal_map(i).cloned()).collect();
        let Bundle {
            model,
            physics,
            motions,
            expressions,
            textures,
            ..
        } = bundle;
        let compiled = CompiledModel::compile(model)?;
        let mut puppet = Puppet::from_compiled(compiled);
        if let Some(physics) = physics {
            // `read_bundle` は物理の設定を検査済みで返す。
            puppet = puppet.with_physics(physics);
        }
        let mut base_textures: Vec<Option<TextureData>> = textures.into_iter().map(Some).collect();
        base_textures.resize(count, None);
        let warnings = base_textures
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_none())
            .map(|(texture, _)| LoadWarning::TextureMissing { texture })
            .collect();
        let mut loaded = Self {
            puppet,
            motions,
            expressions,
            base_textures,
            base_normal_maps,
            modules: Vec::new(),
            sources: Vec::new(),
            drawables: Vec::new(),
            masks: MaskPlan::default(),
            warnings,
        };
        loaded.refresh();
        Ok(loaded)
    }

    pub fn model(&self) -> &Model {
        self.puppet.instance().compiled().model()
    }

    /// 描画に使うテクスチャの枚数。
    pub fn texture_count(&self) -> usize {
        texture_count(self.model())
    }

    /// 今のモデルの `index` 番のテクスチャの画像。無ければ `None` (白で代用する)。
    pub fn texture(&self, index: usize) -> Option<&TextureData> {
        match self.sources.get(index) {
            Some(TextureSource::Module { module, index }) => {
                self.module(module)?.textures.get(*index as usize)
            }
            Some(TextureSource::Base(i)) => self.base_textures.get(*i as usize)?.as_ref(),
            // メッシュが `model.textures` の外を参照している (本体だけのとき)
            None if self.modules.is_empty() => self.base_textures.get(index)?.as_ref(),
            None => None,
        }
    }

    /// 今のモデルの `index` 番のテクスチャの法線マップ。無ければ `None` (法線 `(0, 0, 1)`)。
    pub fn normal_map(&self, index: usize) -> Option<&TextureData> {
        match self.sources.get(index)? {
            TextureSource::Module { module, index } => self
                .module(module)?
                .normal_maps
                .get(*index as usize)?
                .as_ref(),
            TextureSource::Base(i) => self.base_normal_maps.get(*i as usize)?.as_ref(),
        }
    }

    /// 取り付けているモジュール (合成の順)。
    pub fn modules(&self) -> &[Arc<ModuleData>] {
        &self.modules
    }

    fn module(&self, name: &str) -> Option<&ModuleData> {
        self.modules
            .iter()
            .find(|m| m.name() == name)
            .map(Arc::as_ref)
    }

    /// 取り付けるモジュールの組を `modules` にする (`Puppet::set_modules`。空なら本体だけに戻す)。
    pub fn set_modules(
        &mut self,
        modules: Vec<Arc<ModuleData>>,
    ) -> Result<DressUpChange, DressUpError> {
        let arcs = modules.iter().map(|m| Arc::clone(&m.module)).collect();
        let change = self.puppet.set_modules(arcs)?;
        self.modules = modules;
        self.refresh();
        Ok(change)
    }

    /// 本体の物理の設定を差し替える (`Puppet::set_base_physics`。`None` で外す)。失敗したら何も変えない。
    ///
    /// 着せ替えのモジュールを付けていて、本体の物理の有無で合成したモデルが変わったときは、描画の情報を作り直して
    /// true を返す (呼ぶ側はレンダラを作り直す)。物理だけを差し替えたなら false。
    pub fn set_base_physics(
        &mut self,
        settings: Option<PhysicsSettings>,
    ) -> Result<bool, DressUpError> {
        let before = Arc::clone(self.puppet.instance().compiled());
        self.puppet.set_base_physics(settings)?;
        let swapped = !Arc::ptr_eq(&before, self.puppet.instance().compiled());
        if swapped {
            self.refresh();
        }
        Ok(swapped)
    }

    /// モデルが変わったあとに、テクスチャの出どころと描画の情報を作り直す。
    fn refresh(&mut self) {
        self.sources = self.puppet.texture_sources();
        let (drawables, masks) = drawables(self.model());
        self.drawables = drawables;
        self.masks = masks;
    }
}

/// 描画に使うテクスチャの枚数 (`model.textures` の数)。
///
/// メッシュの `texture` の番号からは数えない: 検証 (`TextureOutOfRange`) の前に呼ぶので、壊れた番号
/// (`u32::MAX` など) から巨大な数を確保しないため。検証を通ったモデルでは番号はこの数より小さい。
fn texture_count(model: &Model) -> usize {
    model.textures.len()
}

/// メッシュごとの描画の情報と、マスクの段取りを作る。
///
/// 検証済みのモデル (マスクの参照が循環しない) を前提にする。見つからない参照は無視する。
pub fn drawables(model: &Model) -> (Vec<DrawableInfo>, MaskPlan) {
    let index: HashMap<&str, usize> = model
        .meshes
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    // 各メッシュの参照 (メッシュの番号, invert)。重複を除く。
    let refs: Vec<Vec<(usize, bool)>> = model
        .meshes
        .iter()
        .map(|mesh| {
            let mut out: Vec<(usize, bool)> = Vec::new();
            for r in &mesh.masks {
                let Some(&i) = index.get(r.mesh.as_str()) else {
                    continue;
                };
                if !out.contains(&(i, r.invert)) {
                    out.push((i, r.invert));
                }
            }
            out
        })
        .collect();

    // マスクとして使うメッシュを、入れ子のマスクが先に来る順に並べる (深さ優先の帰りがけ順)。
    // 段は、読むマスクの段の最大 + 1 (読まなければ 0)。
    struct Walk<'a> {
        refs: &'a [Vec<(usize, bool)>],
        level: Vec<Option<u32>>,
        order: Vec<usize>,
    }
    impl Walk<'_> {
        fn visit(&mut self, i: usize) -> u32 {
            if let Some(level) = self.level[i] {
                return level;
            }
            // 循環は検証で弾かれている。念のため、訪問中は 0 とみなして止める。
            self.level[i] = Some(0);
            let mut level = 0;
            for k in 0..self.refs[i].len() {
                let j = self.refs[i][k].0;
                level = level.max(self.visit(j) + 1);
            }
            self.level[i] = Some(level);
            self.order.push(i);
            level
        }
    }
    let mut walk = Walk {
        refs: &refs,
        level: vec![None; model.meshes.len()],
        order: Vec::new(),
    };
    for r in &refs {
        for &(j, _) in r {
            walk.visit(j);
        }
    }
    let levels = walk
        .order
        .iter()
        .map(|&i| walk.level[i].unwrap_or(0))
        .collect();
    let order = walk.order;
    let source_of: HashMap<usize, usize> = order.iter().enumerate().map(|(s, &i)| (i, s)).collect();

    let drawables = model
        .meshes
        .iter()
        .zip(&refs)
        .map(|(mesh, refs)| DrawableInfo {
            id: mesh.id.as_str().to_owned(),
            texture: mesh.texture as usize,
            uvs: mesh.uvs.clone(),
            indices: mesh.indices.clone(),
            culling: mesh.culling,
            masks: refs
                .iter()
                .map(|&(i, invert)| MaskSlot {
                    source: source_of[&i],
                    invert,
                })
                .collect(),
            mask_mode: mesh.mask_mode.effective(),
            lit: model.lighting && mesh.lit,
        })
        .collect();
    (
        drawables,
        MaskPlan {
            sources: order,
            levels,
        },
    )
}

/// 表向きの三角形だけの頂点番号 (`docs/spec/rendering.md#裏面カリング`)。
///
/// キャンバス座標 (Y 下向き) で符号付き面積が負の三角形を表とする。ビューには左右されない。
pub fn front_facing(positions: &[Vec2], indices: &[u16], out: &mut Vec<i32>) {
    out.clear();
    for tri in indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| positions[i as usize]);
        let cross = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if cross < 0.0 {
            out.extend(tri.iter().map(|&i| i32::from(i)));
        }
    }
}

/// 点の集合の外接矩形 `(min, max)`。空なら `None`。
pub fn bounds(positions: &[Vec2]) -> Option<(Vec2, Vec2)> {
    let first = *positions.first()?;
    Some(
        positions
            .iter()
            .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p))),
    )
}

/// マスクのテクスチャ 1 枚の大きさと、それが覆うキャンバス上の矩形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskArea {
    /// テクスチャの画素数。
    pub width: u32,
    pub height: u32,
    /// テクスチャの左上に対応するキャンバス座標。
    pub origin: Vec2,
    /// キャンバスの 1 単位あたりの画素数。
    pub scale: f32,
}

impl MaskArea {
    /// 1 辺の画素数の上限。
    pub const MAX_SIZE: u32 = 4096;
    /// 縁の余白 (画素)。クランプした読み出しで縁の値が伸びないように、外側を必ず 0 にする。
    const PADDING: u32 = 2;

    /// 外接矩形 `(min, max)` を `scale` (キャンバスの 1 単位あたりの画素数) で覆う。
    ///
    /// `current` が十分に大きく、大きすぎない (面積の 4 倍以下) ならその大きさを使い回す
    /// (テクスチャを毎フレーム作り直さないため)。大きさが上限を超えるなら `scale` を下げる。
    /// 有限でない外接矩形 (頂点が NaN など) は大きさ 0 に、有限でない `scale` は 1 にする。
    pub fn fit(min: Vec2, max: Vec2, scale: f32, current: Option<(u32, u32)>) -> Self {
        let finite = |v: Vec2| v.x.is_finite() && v.y.is_finite();
        let (min, max) = if finite(min) && finite(max) {
            (min, max)
        } else {
            (Vec2::ZERO, Vec2::ZERO)
        };
        let size = (max - min).max(Vec2::ZERO);
        let pad = Self::PADDING as f32;
        let limit = (Self::MAX_SIZE - 2 * Self::PADDING) as f32;
        let mut scale = if scale.is_finite() {
            scale.max(1e-3)
        } else {
            1.0
        };
        let largest = size.x.max(size.y) * scale;
        if largest > limit {
            scale *= limit / largest;
        }
        // 画素の格子をキャンバスの `1 / scale` 単位の格子に揃える (倍率 1 で画面の画素と重なり、縁がぼけない)。
        let lo = (min * scale).floor();
        let hi = (max * scale).ceil().max(lo);
        let need = |len: f32| {
            (len as u32)
                .saturating_add(2 * Self::PADDING)
                .min(Self::MAX_SIZE)
        };
        let (need_w, need_h) = (need(hi.x - lo.x), need(hi.y - lo.y));
        let (width, height) = match current {
            Some((w, h))
                if w >= need_w
                    && h >= need_h
                    && u64::from(w) * u64::from(h) <= 4 * u64::from(need_w) * u64::from(need_h) =>
            {
                (w, h)
            }
            _ => (need_w, need_h),
        };
        Self {
            width,
            height,
            origin: (lo - Vec2::splat(pad)) / scale,
            scale,
        }
    }

    /// シェーダに渡す `(origin.x, origin.y, 幅, 高さ)` (キャンバス座標)。
    pub fn rect(&self) -> [f32; 4] {
        [
            self.origin.x,
            self.origin.y,
            self.width as f32 / self.scale,
            self.height as f32 / self.scale,
        ]
    }
}

/// マスクのアトラスの配置: 各カバレッジの矩形の左上の画素と、アトラスの大きさ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasLayout {
    pub width: u32,
    pub height: u32,
    pub positions: Vec<(u32, u32)>,
}

impl AtlasLayout {
    /// 大きさ `sizes` の矩形を、1 辺 `max` 画素以内のアトラスに重ならないように並べる (棚詰め)。
    /// 収まらなければ `None`。
    pub fn pack(sizes: &[(u32, u32)], max: u32) -> Option<Self> {
        if sizes.iter().any(|&(w, h)| w > max || h > max) {
            return None;
        }
        let area: u64 = sizes
            .iter()
            .map(|&(w, h)| u64::from(w) * u64::from(h))
            .sum();
        let widest = sizes.iter().map(|s| s.0).max().unwrap_or(1).max(1);
        // 正方形に近くなる幅から始め、高さが収まらなければ広げる。
        let mut width = widest.max((area as f64).sqrt().ceil() as u32).min(max);
        let mut order: Vec<usize> = (0..sizes.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(sizes[i].1));
        loop {
            if let Some(layout) = Self::shelves(sizes, &order, width, max) {
                return Some(layout);
            }
            if width == max {
                return None;
            }
            width = (width + width / 4 + 1).min(max);
        }
    }

    fn shelves(sizes: &[(u32, u32)], order: &[usize], width: u32, max: u32) -> Option<Self> {
        let mut positions = vec![(0, 0); sizes.len()];
        let (mut x, mut y, mut shelf, mut used) = (0u32, 0u32, 0u32, 1u32);
        for &i in order {
            let (w, h) = sizes[i];
            if x + w > width {
                y += shelf;
                x = 0;
                shelf = 0;
            }
            if y + h > max {
                return None;
            }
            positions[i] = (x, y);
            x += w;
            shelf = shelf.max(h);
            used = used.max(x);
        }
        Some(Self {
            width: used,
            height: (y + shelf).max(1),
            positions,
        })
    }
}

/// アトラスに収まらないときに下げるマスクの解像度の倍率と、それを戻す判断。
///
/// - 下げる: カバレッジの配置が 1 辺 [`MaskArea::MAX_SIZE`] 画素に収まらなければ、収まるまで [`Self::STEP`] 倍ずつ下げる。
/// - 戻す: 下げている間に (アニメーションで) マスクが小さくなり、1 段上げた解像度の配置が 1 辺
///   `MAX_SIZE` × [`Self::RESTORE_MARGIN`] 画素に収まるようになったら、1 段戻す。余裕を持たせるのは、
///   収まるかどうかの境目でマスクの大きさが揺れるたびに、上げ下げ (と並べ直し) を繰り返さないため。
///   1 段戻した直後に下げ直すのは、マスクの外接矩形がおよそ `1 / RESTORE_MARGIN` 倍に広がったときだけ。
/// - 画面上の大きさか `mask_scale` が変わったら 1 から測り直す (呼ぶ側が [`Self::default`] に戻す)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskShrink {
    /// マスクの解像度に掛ける倍率 (1 なら下げていない)。
    pub factor: f32,
}

impl Default for MaskShrink {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

impl MaskShrink {
    /// 1 段で解像度に掛ける倍率。
    pub const STEP: f32 = 0.75;
    /// 戻すときに、配置が収まらなければならないアトラスの 1 辺の割合。
    pub const RESTORE_MARGIN: f32 = 0.875;
    /// これより下げても収まらなければ諦める。
    const MIN: f32 = 1e-3;

    /// 外接矩形 `bounds` を、解像度 `scale × factor` で覆う (今の大きさは使い回さない)。
    pub fn areas(bounds: &[(Vec2, Vec2)], scale: f32, factor: f32) -> Vec<MaskArea> {
        bounds
            .iter()
            .map(|&(min, max)| MaskArea::fit(min, max, scale * factor, None))
            .collect()
    }

    /// 配置が収まるまで倍率を下げる (今の倍率で収まるなら変えない)。下げきっても収まらなければ `None`。
    pub fn shrink_to_fit(
        &mut self,
        bounds: &[(Vec2, Vec2)],
        scale: f32,
    ) -> Option<(Vec<MaskArea>, AtlasLayout)> {
        loop {
            let areas = Self::areas(bounds, scale, self.factor);
            let sizes: Vec<(u32, u32)> = areas.iter().map(|a| (a.width, a.height)).collect();
            if let Some(layout) = AtlasLayout::pack(&sizes, MaskArea::MAX_SIZE) {
                return Some((areas, layout));
            }
            if self.factor < Self::MIN {
                return None;
            }
            self.factor *= Self::STEP;
        }
    }

    /// 下げている倍率を 1 段戻せるなら戻し、その解像度での大きさと配置を返す。戻せなければ `None` (倍率は変えない)。
    pub fn try_restore(
        &mut self,
        bounds: &[(Vec2, Vec2)],
        scale: f32,
    ) -> Option<(Vec<MaskArea>, AtlasLayout)> {
        if self.factor >= 1.0 {
            return None;
        }
        let up = self.factor / Self::STEP;
        // 浮動小数点の誤差で 1 をわずかに下回ったままにしない。
        let up = if up > 0.999 { 1.0 } else { up };
        let areas = Self::areas(bounds, scale, up);
        let sizes: Vec<(u32, u32)> = areas.iter().map(|a| (a.width, a.height)).collect();
        let limit = (MaskArea::MAX_SIZE as f32 * Self::RESTORE_MARGIN) as u32;
        let layout = AtlasLayout::pack(&sizes, limit)?;
        self.factor = up;
        Some((areas, layout))
    }
}

/// マスクの表の 1 行の画素数 (表は RGBA の浮動小数点のテクスチャ)。
pub const MASK_TABLE_WIDTH: usize = 256;

/// シェーダがマスクを読むための表。1 画素が `vec4` 1 つ。
///
/// - 先頭にカバレッジ (`MaskPlan::sources`) ごとに 2 画素: キャンバス上の矩形 `(x, y, 幅, 高さ)` と、
///   アトラスでの矩形 `(u, v, 幅, 高さ)` (0〜1)。
/// - 続けて、要素ごとに読むマスクの列 (1 つが 1 画素: `(カバレッジの番号, 反転なら 1, 0, 0)`)。
///   要素はその列の先頭の番号 ([`MaskTable::first`]) と数をシェーダに渡す。
#[derive(Debug, Clone, PartialEq)]
pub struct MaskTable {
    texels: Vec<[f32; 4]>,
    /// 要素ごとの、読むマスクの列の先頭の画素の番号。
    pub first: Vec<i32>,
}

impl MaskTable {
    pub fn new(drawables: &[DrawableInfo], sources: usize) -> Self {
        let mut texels = vec![[0.0; 4]; 2 * sources];
        let first = drawables
            .iter()
            .map(|d| {
                let first = texels.len() as i32;
                texels.extend(
                    d.masks
                        .iter()
                        .map(|m| [m.source as f32, if m.invert { 1.0 } else { 0.0 }, 0.0, 0.0]),
                );
                first
            })
            .collect();
        Self { texels, first }
    }

    /// カバレッジ `source` の場所を書く。`slot` はアトラスでの左上の画素、`atlas` はアトラスの大きさ。
    pub fn set_source(
        &mut self,
        source: usize,
        area: &MaskArea,
        slot: (u32, u32),
        atlas: (u32, u32),
    ) {
        let (aw, ah) = (atlas.0.max(1) as f32, atlas.1.max(1) as f32);
        self.texels[2 * source] = area.rect();
        self.texels[2 * source + 1] = [
            slot.0 as f32 / aw,
            slot.1 as f32 / ah,
            area.width as f32 / aw,
            area.height as f32 / ah,
        ];
    }

    /// 表のテクスチャの大きさ (幅, 高さ)。
    pub fn size(&self) -> (usize, usize) {
        (
            MASK_TABLE_WIDTH,
            self.texels.len().div_ceil(MASK_TABLE_WIDTH).max(1),
        )
    }

    /// テクスチャに書くバイト列 (RGBA の 32 ビット浮動小数点、リトルエンディアン)。
    pub fn bytes(&self) -> Vec<u8> {
        let (w, h) = self.size();
        let mut out = Vec::with_capacity(w * h * 16);
        for texel in &self.texels {
            for v in texel {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out.resize(w * h * 16, 0);
        out
    }
}

/// シェーダに渡す光源 (`docs/spec/rendering.md#ライティング`)。参照レンダラと同じ形に直したもの。
#[derive(Debug, Clone, PartialEq)]
pub struct LightUniforms {
    pub ambient: [f32; 3],
    /// 平行光源: xyz = 光源へ向かう単位ベクトル, w = 0。点光源: xyz = 位置, w = 1。
    pub vectors: Vec<[f32; 4]>,
    /// rgb = 色, a = 点光源の `1 / range` (平行光源と減衰しない点光源は 0)。
    pub colors: Vec<[f32; 4]>,
}

impl LightUniforms {
    /// 使えない光源 (向きが 0・有限でない値・`range` が正でない・色が有限で負でない値でない) を除き、
    /// 先頭の [`MAX_LIGHTS`] 個を使う (C ABI の `ugoe-ffi` の検査と同じ条件)。
    pub fn new(lighting: &Lighting) -> Self {
        let a = lighting.ambient;
        let mut out = Self {
            ambient: [a.r, a.g, a.b],
            vectors: Vec::new(),
            colors: Vec::new(),
        };
        for light in &lighting.lights {
            if out.vectors.len() == MAX_LIGHTS {
                break;
            }
            let (vector, color, inv_range) = match *light {
                Light::Directional { direction, color } => {
                    let [x, y, z] = direction;
                    let length = (x * x + y * y + z * z).sqrt();
                    if !(length.is_finite() && length > 0.0) {
                        continue;
                    }
                    ([-x / length, -y / length, -z / length, 0.0], color, 0.0)
                }
                Light::Point {
                    position,
                    color,
                    range,
                } => {
                    if !(position.iter().all(|v| v.is_finite()) && range > 0.0) {
                        continue;
                    }
                    let [x, y, z] = position;
                    ([x, y, z, 1.0], color, 1.0 / range)
                }
            };
            if !valid_color(color) {
                continue;
            }
            out.vectors.push(vector);
            out.colors.push([color.r, color.g, color.b, inv_range]);
        }
        out
    }
}

/// 光の色 (環境光と光源) として使えるか: どのチャンネルも有限で負でない。
pub fn valid_color(c: Rgb) -> bool {
    [c.r, c.g, c.b].iter().all(|v| v.is_finite() && *v >= 0.0)
}

/// Godot の `DirectionalLight2D` の向きから、光が進む向き (キャンバス座標 + 奥行き、手前が正) を求める。
///
/// `down` はライトのノードの +Y の向き (光が画面の上を進む向き。パペットのノードの座標)、`height` は Godot の
/// `Light2D.height` (0〜1)。`height` の分だけ画面の奥 (−Z) へ傾ける: 0 で画面と平行、1 で正面から奥へ。
/// 向きが 0 か有限でなければ `None`。
pub fn directional_from_light2d(down: [f32; 2], height: f32) -> Option<[f32; 3]> {
    let length = down[0].hypot(down[1]);
    if !(length.is_finite() && length > 0.0) {
        return None;
    }
    let h = if height.is_finite() {
        height.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let v = [
        down[0] / length * (1.0 - h),
        down[1] / length * (1.0 - h),
        -h,
    ];
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    Some([v[0] / n, v[1] / n, v[2] / n])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe::model::{MaskRef, MeshId, fixtures};

    /// 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
    const FAKE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    fn bundle_bytes(model: Model) -> Vec<u8> {
        let mut bundle = Bundle::new(model.clone());
        bundle.textures = model
            .textures
            .iter()
            .map(|_| TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: FAKE_PNG.to_vec(),
            })
            .collect();
        ugoe::format::write_bundle(&bundle).expect("write")
    }

    #[test]
    fn loads_a_bundle_and_evaluates() {
        let mut loaded = LoadedModel::from_bundle_bytes(
            &bundle_bytes(fixtures::single_quad()),
            LoadLimits::default(),
        )
        .expect("load");
        assert_eq!(loaded.drawables.len(), 1);
        assert_eq!(loaded.texture_count(), loaded.model().textures.len().max(1));
        assert!(loaded.texture(0).is_some());
        loaded.puppet.update(1.0 / 60.0);
        let state = &loaded.puppet.instance().drawables()[0];
        assert_eq!(state.positions.len(), loaded.drawables[0].uvs.len());
    }

    #[test]
    fn reports_read_warnings() {
        // 知らない任意の機能を宣言したファイル (docs/testing.md#互換性テスト)。
        let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoe");
        let loaded = LoadedModel::from_bundle_bytes(bytes, LoadLimits::default()).expect("load");
        assert!(
            loaded
                .warnings
                .iter()
                .any(|w| w.code() == "unknown_optional_feature"),
            "{:?}",
            loaded.warnings
        );
        let bytes = include_bytes!("../../../assets/compat/future/optional-feature.ugoemod");
        let module = ModuleData::from_bytes(bytes, LoadLimits::default()).expect("module");
        assert!(!module.warnings.is_empty());
        // 警告の無いバンドル
        let loaded = LoadedModel::from_bundle_bytes(
            &bundle_bytes(fixtures::single_quad()),
            LoadLimits::default(),
        )
        .expect("load");
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn rejects_out_of_range_textures_before_allocating() {
        // 検証の前にメッシュのテクスチャの番号からテクスチャの数を決めると、約 43 億枚分を確保しようとする。
        let mut model = fixtures::single_quad();
        model.meshes[0].texture = u32::MAX;
        assert!(matches!(
            LoadedModel::from_bundle(Bundle::new(model)),
            Err(LoadError::Compile(_))
        ));
    }

    /// 幅 `width`・高さ `height` の PNG の見出し (署名と `IHDR`) だけのバイト列。画素は読まないので要らない。
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = FAKE_PNG.to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    /// PNG はガンマのチャンク (`gAMA` 1.0・`sRGB`) で値を変えず、16 ビットは上位の 8 ビットにする (参照と同じ。T0477)。
    #[test]
    fn png_ignores_gamma_and_takes_the_high_byte() {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        encoder.set_source_gamma(png::ScaledFloat::new(1.0));
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = encoder.write_header().expect("header");
        writer
            .write_image_data(&[
                0x12, 0xff, 0x80, 0x7f, 0xab, 0x01, 0xff, 0xff, //
                0x40, 0x00, 0xc0, 0x99, 0x01, 0xfe, 0x80, 0x80,
            ])
            .expect("data");
        writer.finish().expect("finish");
        let image = decode_image(TextureFormat::Png, &bytes).expect("decode");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(
            image.pixels,
            [0x12, 0x80, 0xab, 0xff, 0x40, 0xc0, 0x01, 0x80]
        );
        assert!(decode_image(TextureFormat::Png, &png_header(1, 1)).is_err());
    }

    #[test]
    fn reads_image_sizes_from_headers() {
        use ugoe::format::TextureFormat::{Ktx2, Png};
        assert_eq!(image_size(Png, &png_header(64, 32)), Some((64, 32)));
        assert_eq!(image_size(Png, &FAKE_PNG), None);
        assert_eq!(image_size(Png, b"not a png at all, no header"), None);
        let mut ktx2 = KTX2_IDENTIFIER.to_vec();
        ktx2.extend_from_slice(&[0; 8]); // vkFormat・typeSize
        ktx2.extend_from_slice(&300u32.to_le_bytes());
        ktx2.extend_from_slice(&200u32.to_le_bytes());
        assert_eq!(image_size(Ktx2, &ktx2), Some((300, 200)));
        assert_eq!(image_size(Ktx2, &ktx2[..22]), None);
        assert_eq!(image_size(Ktx2, &png_header(1, 1)), None);
    }

    #[test]
    fn rejects_images_larger_than_the_limit() {
        let model = fixtures::single_quad();
        let mut bundle = Bundle::new(model.clone());
        bundle.textures = vec![TextureData {
            format: ugoe::format::TextureFormat::Png,
            bytes: png_header(64, 32),
        }];
        let bytes = ugoe::format::write_bundle(&bundle).expect("write");
        let limits = |side| LoadLimits {
            max_image_side: side,
            ..LoadLimits::default()
        };
        assert!(LoadedModel::from_bundle_bytes(&bytes, limits(64)).is_ok());
        let error = LoadedModel::from_bundle_bytes(&bytes, limits(63)).expect_err("too large");
        assert!(matches!(
            error,
            LoadError::ImageTooLarge {
                image: ImageKind::Texture,
                texture: 0,
                width: 64,
                height: 32,
                max: 63,
            }
        ));
        let (code, fields) = error.info();
        assert_eq!(code, "image_limit_exceeded");
        assert_eq!(
            fields,
            [
                ("image", ErrorField::Text("texture".into())),
                ("texture", ErrorField::Int(0)),
                ("width", ErrorField::Int(64)),
                ("height", ErrorField::Int(32)),
                ("max", ErrorField::Int(63)),
            ]
        );
        // 見出しの読めない画像は調べない (Godot のデコーダに任せる)
        let loaded = LoadedModel::from_bundle_bytes(&bundle_bytes(model), limits(1));
        assert!(loaded.is_ok());
    }

    #[test]
    fn rejects_module_normal_maps_larger_than_the_limit() {
        use ugoe::format::module_file::{ModuleFile, write_module};
        let mut module = fixtures::long_hair_module();
        let texture = module.textures.len() - 1;
        module.textures[texture].normal_map = Some("long_hair_normal".into());
        let mut file = ModuleFile::new(module.clone());
        file.textures = module
            .textures
            .iter()
            .map(|_| TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: png_header(16, 16),
            })
            .collect();
        file.normal_maps = vec![TextureData {
            format: ugoe::format::TextureFormat::Png,
            bytes: png_header(16, 40),
        }];
        let bytes = write_module(&file).expect("write");
        let limits = LoadLimits {
            max_image_side: 32,
            ..LoadLimits::default()
        };
        let error = ModuleData::from_bytes(&bytes, limits).expect_err("too large");
        assert!(
            matches!(
                error,
                LoadError::ImageTooLarge {
                    image: ImageKind::NormalMap,
                    height: 40,
                    max: 32,
                    texture: t,
                    ..
                } if t == texture
            ),
            "{error:?}"
        );
        assert_eq!(error.info().0, "image_limit_exceeded");
        assert!(ModuleData::from_bytes(&bytes, LoadLimits::default()).is_ok());
    }

    #[test]
    fn rejects_bundles_and_modules_over_the_read_limits() {
        use ugoe::format::module_file::{ModuleFile, write_module_with};
        use ugoe::format::{WriteOptions, write_bundle_with};
        let mut bundle = Bundle::new(fixtures::single_quad());
        bundle.textures = vec![TextureData {
            format: ugoe::format::TextureFormat::Png,
            bytes: FAKE_PNG.to_vec(),
        }];
        let bytes = write_bundle_with(&bundle, WriteOptions { compress: true }).expect("write");
        let limits = LoadLimits {
            read: ReadLimits::default().with_max_total_decompressed(1),
            ..LoadLimits::default()
        };
        assert!(LoadedModel::from_bundle_bytes(&bytes, LoadLimits::default()).is_ok());
        let error = LoadedModel::from_bundle_bytes(&bytes, limits).expect_err("over the limit");
        let (code, fields) = error.info();
        assert_eq!(code, "read_limit_exceeded");
        assert_eq!(
            fields[0],
            ("limit", ErrorField::Text("total_decompressed".into()))
        );
        assert!(matches!(fields[1], ("chunk", ErrorField::Text(_))));
        assert_eq!(fields[2], ("max", ErrorField::Int(1)));

        let mut file = ModuleFile::new(fixtures::long_hair_module());
        file.textures = file
            .module
            .textures
            .iter()
            .map(|_| bundle.textures[0].clone())
            .collect();
        let bytes = write_module_with(&file, WriteOptions { compress: true }).expect("write");
        let limits = LoadLimits {
            read: ReadLimits::default().with_max_chunk_decompressed(1),
            ..LoadLimits::default()
        };
        let error = ModuleData::from_bytes(&bytes, limits).expect_err("over the limit");
        let (code, fields) = error.info();
        assert_eq!(code, "read_limit_exceeded");
        assert_eq!(
            fields[0],
            ("limit", ErrorField::Text("chunk_decompressed".into()))
        );
    }

    #[test]
    fn describes_other_load_errors() {
        let error = LoadedModel::from_bundle_bytes(b"not a bundle", LoadLimits::default())
            .expect_err("broken");
        assert_eq!(error.info(), ("invalid_file", Vec::new()));
        let mut model = fixtures::single_quad();
        model.meshes[0].texture = u32::MAX;
        let error = LoadedModel::from_bundle(Bundle::new(model)).expect_err("invalid");
        // 補足の値は SDK の `ugoe::LoadError::fields` と同じ `errors`・`error_codes` (T0470)。
        assert_eq!(
            error.info(),
            (
                "invalid_model",
                vec![
                    ("errors", ErrorField::Int(1)),
                    (
                        "error_codes",
                        ErrorField::Text("texture_out_of_range".into())
                    ),
                ]
            )
        );
    }

    #[test]
    fn rejects_broken_bytes() {
        assert!(matches!(
            LoadedModel::from_bundle_bytes(b"not a bundle", LoadLimits::default()),
            Err(LoadError::Format(_))
        ));
    }

    #[test]
    fn nested_masks_come_before_their_users() {
        let model = fixtures::nested_masks();
        let (drawables, plan) = drawables(&model);
        let pos = |id: &str| model.meshes.iter().position(|m| m.id == id).expect(id);
        // Pupil -> Iris -> Sclera。Sclera のカバレッジを先に描く。
        assert_eq!(plan.sources, vec![pos("Sclera"), pos("Iris")]);
        // Iris は Sclera を読むので 1 段目。
        assert_eq!(plan.levels, vec![0, 1]);
        assert_eq!(plan.level_count(), 2);
        let pupil = &drawables[pos("Pupil")];
        assert_eq!(
            pupil.masks,
            vec![MaskSlot {
                source: 1,
                invert: false
            }]
        );
        let iris = &drawables[pos("Iris")];
        assert_eq!(
            iris.masks,
            vec![MaskSlot {
                source: 0,
                invert: false
            }]
        );
        assert!(drawables[pos("Sclera")].masks.is_empty());
    }

    #[test]
    fn mixed_masks_keep_invert_and_mode() {
        let model = fixtures::mixed_masks();
        let (drawables, plan) = drawables(&model);
        let brow = drawables.iter().find(|d| d.id == "Brow").expect("Brow");
        assert_eq!(brow.masks.len(), 2);
        let invert: Vec<bool> = brow.masks.iter().map(|m| m.invert).collect();
        assert_eq!(invert, vec![false, true]);
        assert_eq!(plan.sources.len(), 2);
        assert_eq!(plan.level_count(), 1);

        let model = fixtures::intersection_masks();
        let (drawables, _) = super::drawables(&model);
        assert!(
            drawables
                .iter()
                .any(|d| d.mask_mode == MaskMode::Intersection)
        );
    }

    #[test]
    fn many_masks_are_all_kept() {
        let mask_ref = |m: &ugoe::model::Mesh| MaskRef {
            mesh: m.id.clone(),
            invert: false,
        };
        let mut model = fixtures::nested_masks();
        // 同じメッシュを同じ invert で 2 回参照しても 1 つと数える。
        let first = model.meshes[0].masks[0].clone();
        model.meshes[0].masks = vec![first.clone(), first];
        let (d, _) = drawables(&model);
        assert_eq!(d[0].masks.len(), 1);

        // 以前の上限 (8) を超える数の別のメッシュを参照させても、すべて読む。
        let template = model.meshes[2].clone();
        for i in 0..20 {
            let mut m = template.clone();
            m.id = MeshId::new(format!("Extra{i}"));
            model.meshes.push(m);
        }
        model.meshes[0].masks = model.meshes[1..].iter().map(mask_ref).collect();
        let (d, plan) = drawables(&model);
        assert_eq!(d[0].masks.len(), model.meshes.len() - 1);
        assert_eq!(plan.sources.len(), model.meshes.len() - 1);

        let table = MaskTable::new(&d, plan.sources.len());
        assert_eq!(table.first[0], 2 * plan.sources.len() as i32);
        let (w, h) = table.size();
        assert_eq!(table.bytes().len(), w * h * 16);
    }

    #[test]
    fn mask_table_maps_canvas_to_the_atlas() {
        let (d, plan) = drawables(&fixtures::nested_masks());
        let mut table = MaskTable::new(&d, plan.sources.len());
        let area = MaskArea::fit(Vec2::new(10.0, 20.0), Vec2::new(110.0, 70.0), 2.0, None);
        table.set_source(1, &area, (100, 50), (400, 200));
        assert_eq!(table.texels[2], area.rect());
        assert_eq!(table.texels[3], [0.25, 0.25, 204.0 / 400.0, 104.0 / 200.0]);
        // 要素ごとの列は表の後ろに続く (Pupil は Iris = カバレッジ 1 を読む)。
        let pupil = d.iter().position(|d| d.id == "Pupil").expect("Pupil");
        assert_eq!(
            table.texels[table.first[pupil] as usize],
            [1.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn atlas_packs_without_overlap() {
        let sizes = [(100, 40), (30, 80), (64, 64), (10, 10), (200, 20)];
        let layout = AtlasLayout::pack(&sizes, 4096).expect("fits");
        let rects: Vec<_> = sizes
            .iter()
            .zip(&layout.positions)
            .map(|(&(w, h), &(x, y))| (x, y, x + w, y + h))
            .collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a.2 <= layout.width && a.3 <= layout.height);
            for b in &rects[i + 1..] {
                assert!(
                    a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1,
                    "{a:?} {b:?}"
                );
            }
        }
        // 収まらなければ None。
        assert_eq!(AtlasLayout::pack(&[(100, 100); 5], 150), None);
        assert!(AtlasLayout::pack(&[(100, 100); 2], 200).is_some());
        assert_eq!(AtlasLayout::pack(&[(300, 10)], 200), None);
    }

    #[test]
    fn lights_follow_the_reference_renderer() {
        let lighting = Lighting {
            ambient: Rgb::new(0.2, 0.3, 0.4),
            lights: vec![
                Light::Directional {
                    direction: [0.0, 0.0, 0.0],
                    color: Rgb::WHITE,
                },
                Light::Directional {
                    direction: [3.0, 0.0, -4.0],
                    color: Rgb::new(1.0, 0.5, 0.0),
                },
                Light::Point {
                    position: [10.0, 20.0, 30.0],
                    color: Rgb::WHITE,
                    range: 4.0,
                },
                Light::Point {
                    position: [0.0, 0.0, 0.0],
                    color: Rgb::WHITE,
                    range: f32::NAN,
                },
            ],
        };
        let u = LightUniforms::new(&lighting);
        assert_eq!(u.ambient, [0.2, 0.3, 0.4]);
        assert_eq!(
            u.vectors,
            vec![[-0.6, -0.0, 0.8, 0.0], [10.0, 20.0, 30.0, 1.0]]
        );
        assert_eq!(u.colors, vec![[1.0, 0.5, 0.0, 0.0], [1.0, 1.0, 1.0, 0.25]]);

        let many = Lighting {
            ambient: Rgb::WHITE,
            lights: vec![
                Light::Point {
                    position: [0.0; 3],
                    color: Rgb::WHITE,
                    range: f32::INFINITY,
                };
                MAX_LIGHTS + 3
            ],
        };
        let u = LightUniforms::new(&many);
        assert_eq!(u.vectors.len(), MAX_LIGHTS);
        assert_eq!(u.colors[0][3], 0.0);
    }

    #[test]
    fn lit_meshes_need_the_model_switch() {
        let mut model = fixtures::single_quad();
        model.meshes[0].lit = true;
        assert!(!drawables(&model).0[0].lit);
        model.lighting = true;
        assert!(drawables(&model).0[0].lit);
    }

    #[test]
    fn modules_bring_their_textures() {
        use ugoe::format::module_file::{ModuleFile, write_module};

        let base = fixtures::dress_up_base();
        let mut loaded =
            LoadedModel::from_bundle_bytes(&bundle_bytes(base.clone()), LoadLimits::default())
                .expect("load");
        let base_count = loaded.texture_count();
        let module = fixtures::dress_module();
        let mut file = ModuleFile::new(module.clone());
        file.textures = module
            .textures
            .iter()
            .map(|_| TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: FAKE_PNG.to_vec(),
            })
            .collect();
        let data = Arc::new(
            ModuleData::from_bytes(&write_module(&file).expect("write"), LoadLimits::default())
                .expect("read"),
        );
        let name = data.name().to_owned();
        let change = loaded.set_modules(vec![Arc::clone(&data)]).expect("attach");
        assert_eq!(loaded.modules().len(), 1);
        assert_eq!(change.texture_count as usize, loaded.texture_count());
        assert!(loaded.texture_count() > base_count || module.textures.is_empty());
        if !module.textures.is_empty() {
            assert_eq!(
                loaded.texture(base_count).map(|t| t.bytes.clone()),
                Some(FAKE_PNG.to_vec())
            );
        }
        assert_eq!(loaded.drawables.len(), loaded.model().meshes.len());
        assert!(loaded.model().meshes.len() > base.meshes.len());

        assert_eq!(loaded.modules()[0].name(), name);
        let change = loaded.set_modules(Vec::new()).expect("detach");
        assert_eq!(change.texture_count as usize, base_count);
        assert!(loaded.modules().is_empty());
        assert_eq!(loaded.drawables.len(), base.meshes.len());
    }

    #[test]
    fn base_physics_is_replaced_and_validated() {
        let mut loaded = LoadedModel::from_bundle_bytes(
            &bundle_bytes(fixtures::physics_hair()),
            LoadLimits::default(),
        )
        .expect("load");
        assert!(loaded.puppet.physics().is_none());
        let swapped = loaded
            .set_base_physics(Some(fixtures::physics_hair_settings()))
            .expect("valid settings");
        assert!(!swapped);
        assert!(loaded.puppet.physics().is_some());
        assert!(loaded.puppet.base_physics().is_some());
        // モデルに合わない設定 (別のモデルのリグ) は拒み、今の設定を保つ。
        let mut loaded_quad = LoadedModel::from_bundle_bytes(
            &bundle_bytes(fixtures::single_quad()),
            LoadLimits::default(),
        )
        .expect("load");
        assert!(
            loaded_quad
                .set_base_physics(Some(fixtures::physics_hair_settings()))
                .is_err()
        );
        assert!(loaded_quad.puppet.physics().is_none());
        assert!(!loaded.set_base_physics(None).expect("remove"));
        assert!(loaded.puppet.physics().is_none());
        assert_eq!(loaded.drawables.len(), loaded.model().meshes.len());
    }

    #[test]
    fn front_facing_keeps_counter_clockwise_on_screen() {
        // キャンバス (Y 下向き) で画面上の反時計回り = 符号付き面積が負。
        let p = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 10.0),
            Vec2::new(10.0, 0.0),
        ];
        let mut out = Vec::new();
        front_facing(&p, &[0, 1, 2, 0, 2, 1], &mut out);
        assert_eq!(out, vec![0, 1, 2]);
    }

    #[test]
    fn bounds_of_points() {
        assert_eq!(bounds(&[]), None);
        let b = bounds(&[
            Vec2::new(1.0, 5.0),
            Vec2::new(-2.0, 3.0),
            Vec2::new(4.0, 4.0),
        ]);
        assert_eq!(b, Some((Vec2::new(-2.0, 3.0), Vec2::new(4.0, 5.0))));
    }

    #[test]
    fn mask_area_covers_the_bounds_with_padding() {
        let a = MaskArea::fit(Vec2::new(10.0, 20.0), Vec2::new(110.0, 70.0), 2.0, None);
        assert_eq!((a.width, a.height), (204, 104));
        let [x, y, w, h] = a.rect();
        assert!((x - 9.0).abs() < 1e-5 && (y - 19.0).abs() < 1e-5);
        assert!((w - 102.0).abs() < 1e-5 && (h - 52.0).abs() < 1e-5);

        // 十分に大きい今のテクスチャは使い回す。大きすぎるものは作り直す。
        let b = MaskArea::fit(
            Vec2::new(10.0, 20.0),
            Vec2::new(100.0, 60.0),
            2.0,
            Some((204, 104)),
        );
        assert_eq!((b.width, b.height), (204, 104));
        let c = MaskArea::fit(Vec2::ZERO, Vec2::new(10.0, 10.0), 2.0, Some((204, 104)));
        assert_eq!((c.width, c.height), (24, 24));

        // 原点は画素の格子に揃える。
        let e = MaskArea::fit(Vec2::new(10.3, 20.7), Vec2::new(20.2, 30.1), 1.0, None);
        assert_eq!(e.origin, Vec2::new(8.0, 18.0));
        assert_eq!((e.width, e.height), (15, 15));

        // 上限を超えるなら解像度を下げる。
        let d = MaskArea::fit(Vec2::ZERO, Vec2::new(10000.0, 100.0), 1.0, None);
        assert_eq!(d.width, MaskArea::MAX_SIZE);
        assert!(d.scale < 1.0);
    }

    #[test]
    fn mask_area_survives_non_finite_input() {
        // 無限大の解像度や NaN の外接矩形でも、あふれずに上限の中の大きさになる。
        for (min, max, scale) in [
            (Vec2::ZERO, Vec2::new(10.0, 10.0), f32::INFINITY),
            (Vec2::new(1.0, 1.0), Vec2::new(10.0, 10.0), f32::INFINITY),
            (Vec2::ZERO, Vec2::new(10.0, 10.0), f32::NAN),
            (Vec2::splat(f32::NAN), Vec2::new(10.0, 10.0), 1.0),
            (Vec2::ZERO, Vec2::splat(f32::INFINITY), 1.0),
        ] {
            let a = MaskArea::fit(min, max, scale, None);
            assert!(
                a.width <= MaskArea::MAX_SIZE && a.height <= MaskArea::MAX_SIZE,
                "{min:?} {max:?} {scale}: {a:?}"
            );
            assert!(a.scale.is_finite() && a.scale > 0.0, "{a:?}");
        }
    }

    #[test]
    fn lights_with_non_finite_colors_are_dropped() {
        let lighting = Lighting {
            ambient: Rgb::WHITE,
            lights: vec![
                Light::Directional {
                    direction: [0.0, 0.0, -1.0],
                    color: Rgb::new(f32::NAN, 1.0, 1.0),
                },
                Light::Point {
                    position: [0.0; 3],
                    color: Rgb::new(f32::INFINITY, 1.0, 1.0),
                    range: 1.0,
                },
                Light::Point {
                    position: [0.0; 3],
                    color: Rgb::new(-1.0, 1.0, 1.0),
                    range: 1.0,
                },
            ],
        };
        assert!(LightUniforms::new(&lighting).vectors.is_empty());
    }

    #[test]
    fn directional_light2d_tilts_into_the_screen() {
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
        // 高さ 0 は画面と平行 (ノードの +Y の向きに進む)
        assert!(close(
            directional_from_light2d([0.0, 2.0], 0.0).expect("light"),
            [0.0, 1.0, 0.0]
        ));
        // 高さ 1 は正面から奥へ
        assert!(close(
            directional_from_light2d([1.0, 0.0], 1.0).expect("light"),
            [0.0, 0.0, -1.0]
        ));
        let h = std::f32::consts::FRAC_1_SQRT_2;
        assert!(close(
            directional_from_light2d([1.0, 0.0], 0.5).expect("light"),
            [h, 0.0, -h]
        ));
        assert!(close(
            directional_from_light2d([0.0, 1.0], f32::NAN).expect("light"),
            [0.0, 1.0, 0.0]
        ));
        assert_eq!(directional_from_light2d([0.0, 0.0], 0.5), None);
    }

    #[test]
    fn mask_shrink_lowers_and_restores_with_a_margin() {
        let square = |side: f32| (Vec2::ZERO, Vec2::splat(side));
        // 1 辺 3000 のマスクが 2 つ: 倍率 1 (3004 × 2) でも 0.75 (2254 × 2) でも収まらず、0.5625 で収まる。
        let big = [square(3000.0), square(3000.0)];
        let mut shrink = MaskShrink::default();
        let (areas, layout) = shrink.shrink_to_fit(&big, 1.0).expect("fits");
        assert!((shrink.factor - 0.5625).abs() < 1e-6);
        assert!(layout.width <= MaskArea::MAX_SIZE && layout.height <= MaskArea::MAX_SIZE);
        assert_eq!(areas[0].width, 1692);
        // 収まっている間は戻さない (1 段上げると収まらない)。
        assert_eq!(shrink.try_restore(&big, 1.0), None);
        assert!((shrink.factor - 0.5625).abs() < 1e-6);

        // 1 段上げた配置が上限には収まるが、余裕 (7/8 = 3584) に収まらないうちは戻さない
        // (倍率 0.75 で 1 辺 1950 + 余白 4 = 1954 が 2 つで 3908)。
        let edge = [square(2600.0), square(2600.0)];
        assert_eq!(shrink.try_restore(&edge, 1.0), None);
        // 余裕に収まれば 1 段ずつ戻す。
        let small = [square(1500.0), square(1500.0)];
        let (areas, _) = shrink.try_restore(&small, 1.0).expect("restored");
        assert!((shrink.factor - 0.75).abs() < 1e-6);
        assert_eq!(areas[0].width, 1129);
        let (areas, _) = shrink.try_restore(&small, 1.0).expect("restored");
        assert_eq!(shrink.factor, 1.0);
        assert_eq!(areas[0].width, 1504);
        // 下げていなければ何もしない。
        assert_eq!(shrink.try_restore(&small, 1.0), None);

        // 大きく下げたときも、1 回に戻すのは 1 段だけ。
        let huge = [square(12000.0), square(12000.0)];
        let mut shrink = MaskShrink::default();
        shrink.shrink_to_fit(&huge, 1.0).expect("fits");
        let low = shrink.factor;
        assert!(low < 0.25);
        shrink.try_restore(&small, 1.0).expect("restored");
        assert!((shrink.factor - low / MaskShrink::STEP).abs() < 1e-6);
    }

    /// 見出しとレベル 0 だけの KTX2 (超圧縮なし、データ形式の記述なし)。
    fn ktx2(vk_format: u32, width: u32, height: u32, level: &[u8]) -> Vec<u8> {
        let mut bytes = KTX2_IDENTIFIER.to_vec();
        for v in [vk_format, 1, width, height, 0, 0, 1, 1, 0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.resize(80, 0);
        let len = level.len() as u64;
        for v in [104, len, len] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(level);
        bytes
    }

    /// KTX2 は参照のデコーダで読む (T0481): `_SRGB` の値は変えず、アルファの無い BC1 は 3 色のモードの 4 つ目の色も不透明。
    #[test]
    fn ktx2_keeps_stored_values() {
        // VK_FORMAT_R8G8B8A8_SRGB
        let texels = [0x12, 0x80, 0xab, 0xff, 0x40, 0xc0, 0x01, 0x80];
        let image = decode_image(TextureFormat::Ktx2, &ktx2(43, 2, 1, &texels)).expect("decode");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.pixels, texels);
        // 端点が color0 <= color1 で、すべての画素が 4 つ目の色 (黒) のブロック
        let block = [0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        let rgb = decode_image(TextureFormat::Ktx2, &ktx2(131, 4, 4, &block)).expect("BC1_RGB");
        assert!(
            rgb.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [0, 0, 0, 255])
        );
        let rgba = decode_image(TextureFormat::Ktx2, &ktx2(133, 4, 4, &block)).expect("BC1_RGBA");
        assert!(
            rgba.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [0, 0, 0, 0])
        );
        // 参照が読まない形式 (VK_FORMAT_UNDEFINED = Basis Universal など) はエラー
        assert!(decode_image(TextureFormat::Ktx2, &ktx2(0, 4, 4, &block)).is_err());
    }
}
