//! `.ugoe` バンドル (チャンク形式のバイナリコンテナ) の読み書き。
//! 仕様: `docs/spec/file-formats.md` の「バンドル (`.ugoe`)」、判断の経緯: ADR 0008。
//!
//! - ヘッダ・チャンク表はリトルエンディアン。チャンクは 16 バイト境界に揃える。
//! - `MODL` `PHYS` `MOTN` `EXPR` のペイロードは MessagePack (フィールド名付き)。
//! - `TEXR` と `NRML` (法線マップ) は PNG / KTX2 のバイト列そのまま。`META` は JSON。
//! - `FEAT` は使っている機能の宣言 (MessagePack)。[`crate::features`] を参照。
//! - 着せ替えのモジュール (`.ugoemod`) も同じコンテナを使う ([`crate::module_file`])。
//! - 読み込みは入力を信用しない。範囲外・重なり・切り詰めはすべてエラーにし、パニックしない。

use std::borrow::Cow;
use std::fmt;
use std::io::Read;

use serde::Serialize;
use serde::de::DeserializeOwned;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{Model, TextureRef};

use crate::features::{self, FeatureDecl, FeatureTable};
use crate::{FormatError, MAGIC, check_schema_version};

/// コンテナ形式のバージョン (ヘッダの `version`)。スキーマのバージョンとは別。
pub const CONTAINER_VERSION: u32 = 0;

/// ヘッダのバイト数。
pub const HEADER_LEN: usize = 16;

/// チャンク表 1 項目のバイト数。
pub const CHUNK_ENTRY_LEN: usize = 24;

/// チャンクの開始位置の境界 (バイト)。
pub const CHUNK_ALIGN: usize = 16;

/// チャンクフラグ bit0: ペイロードが zstd で圧縮されている。
pub const CHUNK_FLAG_ZSTD: u32 = 1;

/// 圧縮チャンク 1 つの展開後の大きさで、仕様がどの読み込み器にも既定で読めることを求める下限 (256 MiB)。
///
/// [`ReadLimits::default`] の 1 チャンクの上限はこの値。書き出しは、展開後がこれを超えるチャンクを圧縮しない
/// (仕様は SHOULD NOT。`docs/spec/file-formats.md#読み込みの上限`)。
pub const MAX_DECOMPRESSED_CHUNK_LEN: usize = 256 * 1024 * 1024;

/// [`ReadLimits::default`] の、1 つのファイルで展開するバイトの合計の上限 (1 GiB)。
pub const DEFAULT_MAX_TOTAL_DECOMPRESSED: u64 = 1024 * 1024 * 1024;

/// 読み込みの資源の上限 (`docs/spec/file-formats.md#読み込みの上限`)。
///
/// 上限はファイル形式の決まりではなく、読む側の方針。外から来るファイルの展開の爆発
/// (数 KiB の zstd のフレームが数百 MiB に展開される) でメモリを使い切らないために持つ。
/// 超えそうになったら展開の途中でやめ、[`FormatError::ReadLimitExceeded`] を返す (先に確保しない)。
///
/// 数えるのは圧縮したチャンクを展開したバイトだけ。圧縮していないチャンクは入力のバイト列を指すだけで、
/// 大きさは入力の長さ (呼び出し側が分かる) で抑えられるので数えない。
/// 未知のタグのチャンクは展開しないので数えない。
///
/// 将来ほかの上限を足せるように `#[non_exhaustive]` にしている。値を変えるときは
/// `ReadLimits::default().with_max_total_decompressed(n)` のように既定値から始める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReadLimits {
    /// 圧縮したチャンク 1 つを展開した後の大きさの上限 (バイト)。既定は [`MAX_DECOMPRESSED_CHUNK_LEN`] (256 MiB)。
    pub max_chunk_decompressed: u64,
    /// 1 つのファイルで、圧縮したチャンクを展開したバイトの合計の上限。既定は
    /// [`DEFAULT_MAX_TOTAL_DECOMPRESSED`] (1 GiB)。
    pub max_total_decompressed: u64,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_chunk_decompressed: MAX_DECOMPRESSED_CHUNK_LEN as u64,
            max_total_decompressed: DEFAULT_MAX_TOTAL_DECOMPRESSED,
        }
    }
}

impl ReadLimits {
    /// 1 チャンクの展開後の上限を `bytes` にしたもの。
    pub const fn with_max_chunk_decompressed(self, bytes: u64) -> Self {
        Self {
            max_chunk_decompressed: bytes,
            ..self
        }
    }

    /// 展開したバイトの合計の上限を `bytes` にしたもの。
    pub const fn with_max_total_decompressed(self, bytes: u64) -> Self {
        Self {
            max_total_decompressed: bytes,
            ..self
        }
    }
}

/// どの読み込みの上限を超えたか ([`FormatError::ReadLimitExceeded`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadLimit {
    /// 1 チャンクの展開後の大きさ ([`ReadLimits::max_chunk_decompressed`])。
    ChunkDecompressed,
    /// ファイル全体で展開したバイトの合計 ([`ReadLimits::max_total_decompressed`])。
    TotalDecompressed,
}

impl ReadLimit {
    /// 安定した名前 (`chunk_decompressed` / `total_decompressed`)。エラーのコードの引数に使う。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChunkDecompressed => "chunk_decompressed",
            Self::TotalDecompressed => "total_decompressed",
        }
    }
}

impl fmt::Display for ReadLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ChunkDecompressed => "decompressed size of a chunk",
            Self::TotalDecompressed => "total decompressed size",
        })
    }
}

/// MessagePack の入れ子の深さの上限。モデルの型の入れ子より十分大きく、スタックを使い切らない値。
const MAX_DEPTH: usize = 128;

/// チャンクのタグ (4 バイトの ASCII)。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkTag(pub [u8; 4]);

impl ChunkTag {
    /// `Model`。1 つだけ。
    pub const MODL: Self = Self(*b"MODL");
    /// `PhysicsSettings`。0〜1 個。
    pub const PHYS: Self = Self(*b"PHYS");
    /// `Motion`。複数可。
    pub const MOTN: Self = Self(*b"MOTN");
    /// `Expression`。複数可。
    pub const EXPR: Self = Self(*b"EXPR");
    /// テクスチャ 1 枚。`Model.textures` と同じ順。
    pub const TEXR: Self = Self(*b"TEXR");
    /// 自由なメタデータ (JSON)。0〜1 個。
    pub const META: Self = Self(*b"META");
    /// 使っている機能の宣言。0〜1 個 (宣言がなければ書かない)。
    pub const FEAT: Self = Self(*b"FEAT");
    /// 法線マップ 1 枚。`Model.textures` のうち `normal_map` を持つものと同じ順。
    pub const NRML: Self = Self(*b"NRML");
    /// 着せ替えのモジュール (`Module`)。`.ugoemod` に 1 つだけ。`.ugoe` には置かない。
    pub const MODU: Self = Self(*b"MODU");
}

impl fmt::Display for ChunkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in &self.0 {
            if b.is_ascii_graphic() {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for ChunkTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkTag({self})")
    }
}

/// テクスチャ画像の形式。`TEXR` チャンクの先頭のシグネチャで判別する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Png,
    Ktx2,
}

pub(crate) const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
pub(crate) const KTX2_SIGNATURE: [u8; 12] = [
    0xab, b'K', b'T', b'X', b' ', b'2', b'0', 0xbb, 0x0d, 0x0a, 0x1a, 0x0a,
];

impl TextureFormat {
    /// バイト列の先頭のシグネチャから形式を判別する。
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&PNG_SIGNATURE) {
            Some(Self::Png)
        } else if bytes.starts_with(&KTX2_SIGNATURE) {
            Some(Self::Ktx2)
        } else {
            None
        }
    }
}

/// テクスチャ 1 枚分の画像ファイルのバイト列 (デコードはしない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureData {
    pub format: TextureFormat,
    /// PNG または KTX2 ファイルの中身そのもの。
    pub bytes: Vec<u8>,
}

/// `.ugoe` バンドル 1 つ分の中身。
#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    /// チャンク表の順に並ぶ。名前は `Motion::name`。
    pub motions: Vec<Motion>,
    /// チャンク表の順に並ぶ。名前は `Expression::name`。
    pub expressions: Vec<Expression>,
    /// `model.textures` と同じ数・同じ順。
    pub textures: Vec<TextureData>,
    /// 法線マップ (`NRML` チャンク)。`model.textures` のうち `normal_map` が `Some` のものと
    /// 同じ数・同じ順。テクスチャの番号からは [`Bundle::normal_map`] で引く。
    pub normal_maps: Vec<TextureData>,
    /// 作者やライセンスなどの自由なメタデータ。
    pub meta: Option<serde_json::Value>,
}

impl Bundle {
    /// モデルだけのバンドル (テクスチャなどは後から足す)。
    pub fn new(model: Model) -> Self {
        Self {
            model,
            physics: None,
            motions: Vec::new(),
            expressions: Vec::new(),
            textures: Vec::new(),
            normal_maps: Vec::new(),
            meta: None,
        }
    }

    /// `model.textures` の `texture` 番目のテクスチャの法線マップ。なければ `None`。
    pub fn normal_map(&self, texture: usize) -> Option<&TextureData> {
        normal_map_of(&self.model.textures, &self.normal_maps, texture)
    }
}

/// `textures` の `texture` 番目の法線マップを、`normal_map` を持つテクスチャの順に並ぶ `normal_maps` から引く。
pub(crate) fn normal_map_of<'a>(
    textures: &[TextureRef],
    normal_maps: &'a [TextureData],
    texture: usize,
) -> Option<&'a TextureData> {
    textures.get(texture)?.normal_map.as_ref()?;
    let index = textures[..texture]
        .iter()
        .filter(|t| t.normal_map.is_some())
        .count();
    normal_maps.get(index)
}

/// 書き出しの設定。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WriteOptions {
    /// `TEXR` と `NRML` 以外のチャンクを zstd で圧縮する。画像は既に圧縮済みなので対象外。
    /// 展開後が [`MAX_DECOMPRESSED_CHUNK_LEN`] を超えるチャンクも圧縮しない (既定の上限の読み込み器で読めるように)。
    pub compress: bool,
}

/// バンドルを既定の設定 (無圧縮) でバイト列に書き出す。
pub fn write_bundle(bundle: &Bundle) -> Result<Vec<u8>, FormatError> {
    write_bundle_with(bundle, WriteOptions::default())
}

/// バンドルをバイト列に書き出す。
///
/// テクスチャと法線マップの数が `model.textures` と一致しない場合や、`TextureData::format` と
/// 中身のシグネチャが食い違う場合はエラーにする (読み込めないファイルを作らないため)。
/// 未知の値 (`Unknown`) を含む場合は [`FormatError::UnknownValues`] で拒否する
/// (元の値を失っているので、別の値として黙って書き出さないため)。
///
/// 使っている機能は [`features::detect`] で自動で宣言し、宣言があれば `FEAT` チャンクに書く。
pub fn write_bundle_with(bundle: &Bundle, options: WriteOptions) -> Result<Vec<u8>, FormatError> {
    write_with_features(bundle, options, &features::detect(bundle))
}

/// 機能の宣言 `declared` を指定して書き出す (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn write_with_features(
    bundle: &Bundle,
    options: WriteOptions,
    declared: &[FeatureDecl],
) -> Result<Vec<u8>, FormatError> {
    check_texture_count(&bundle.model.textures, bundle.textures.len())?;
    check_normal_map_count(&bundle.model.textures, bundle.normal_maps.len())?;
    let unknown = unknown_values(bundle);
    if !unknown.is_empty() {
        return Err(FormatError::UnknownValues(unknown));
    }

    let mut writer = ChunkWriter::new(options, declared)?;
    writer.push_encoded(ChunkTag::MODL, encode(&bundle.model)?);
    if let Some(physics) = &bundle.physics {
        writer.push_encoded(ChunkTag::PHYS, encode(physics)?);
    }
    for motion in &bundle.motions {
        writer.push_encoded(ChunkTag::MOTN, encode(motion)?);
    }
    for expression in &bundle.expressions {
        writer.push_encoded(ChunkTag::EXPR, encode(expression)?);
    }
    if let Some(meta) = &bundle.meta {
        writer.push_encoded(ChunkTag::META, serde_json::to_vec(meta)?);
    }
    writer.push_images(&bundle.textures, &bundle.normal_maps)?;
    writer.finish()
}

/// チャンクを集めてコンテナを組み立てる (バンドルとモジュールで共通)。
pub(crate) struct ChunkWriter<'a> {
    options: WriteOptions,
    /// これより大きいペイロードは圧縮しない ([`MAX_DECOMPRESSED_CHUNK_LEN`]。試験では小さくする)。
    compress_limit: usize,
    chunks: Vec<(ChunkTag, u32, Cow<'a, [u8]>)>,
}

impl<'a> ChunkWriter<'a> {
    /// 機能の宣言 `declared` があれば、最初に `FEAT` を置く。
    pub(crate) fn new(
        options: WriteOptions,
        declared: &[FeatureDecl],
    ) -> Result<Self, FormatError> {
        let mut writer = Self {
            options,
            compress_limit: MAX_DECOMPRESSED_CHUNK_LEN,
            chunks: Vec::new(),
        };
        if !declared.is_empty() {
            let table = FeatureTable {
                features: declared.to_vec(),
            };
            writer.push_encoded(ChunkTag::FEAT, encode(&table)?);
        }
        Ok(writer)
    }

    /// エンコード済みのペイロードを加える。設定に従って圧縮する。
    ///
    /// 展開後が [`MAX_DECOMPRESSED_CHUNK_LEN`] を超えるペイロードは、圧縮すると既定の上限の読み込み器で
    /// 拒否されるので、設定によらず圧縮せずに置く (`docs/spec/file-formats.md#読み込みの上限`)。
    pub(crate) fn push_encoded(&mut self, tag: ChunkTag, raw: Vec<u8>) {
        if self.options.compress && raw.len() <= self.compress_limit {
            let compressed = ruzstd::encoding::compress_to_vec(
                raw.as_slice(),
                ruzstd::encoding::CompressionLevel::Fastest,
            );
            self.chunks
                .push((tag, CHUNK_FLAG_ZSTD, Cow::Owned(compressed)));
        } else {
            self.chunks.push((tag, 0, Cow::Owned(raw)));
        }
    }

    /// テクスチャ (`TEXR`) と法線マップ (`NRML`) を、形式とシグネチャが合うか確かめて加える。圧縮しない。
    pub(crate) fn push_images(
        &mut self,
        textures: &'a [TextureData],
        normal_maps: &'a [TextureData],
    ) -> Result<(), FormatError> {
        for (index, texture) in textures.iter().enumerate() {
            if TextureFormat::detect(&texture.bytes) != Some(texture.format) {
                return Err(FormatError::TextureFormatMismatch { index });
            }
            self.chunks
                .push((ChunkTag::TEXR, 0, Cow::Borrowed(texture.bytes.as_slice())));
        }
        for (index, normal_map) in normal_maps.iter().enumerate() {
            if TextureFormat::detect(&normal_map.bytes) != Some(normal_map.format) {
                return Err(FormatError::NormalMapFormatMismatch { index });
            }
            self.chunks.push((
                ChunkTag::NRML,
                0,
                Cow::Borrowed(normal_map.bytes.as_slice()),
            ));
        }
        Ok(())
    }

    /// ヘッダ・チャンク表・チャンクの順に書き出す。
    pub(crate) fn finish(self) -> Result<Vec<u8>, FormatError> {
        let chunks = self.chunks;
        let chunk_count = u32::try_from(chunks.len()).map_err(|_| FormatError::TooLarge)?;
        let table_end = HEADER_LEN + CHUNK_ENTRY_LEN * chunks.len();

        // 先にチャンクの配置を決め、ヘッダ・チャンク表・本体の順に書く。
        let mut placements = Vec::with_capacity(chunks.len());
        let mut cursor = table_end;
        for (_, _, payload) in &chunks {
            let offset = align_up(cursor).ok_or(FormatError::TooLarge)?;
            placements.push(offset);
            cursor = offset
                .checked_add(payload.len())
                .ok_or(FormatError::TooLarge)?;
        }

        let mut out = Vec::with_capacity(cursor);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&chunk_count.to_le_bytes());
        for ((tag, flags, payload), &offset) in chunks.iter().zip(&placements) {
            out.extend_from_slice(&tag.0);
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&(offset as u64).to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        }
        for ((_, _, payload), &offset) in chunks.iter().zip(&placements) {
            out.resize(offset, 0);
            out.extend_from_slice(payload);
        }
        Ok(out)
    }
}

/// 読み込みの警告 ([`read_bundle_with_report`])。読み込みは成功しているが、表示が劣化しうる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadWarning {
    /// 知らない任意の機能。仕様の定めるフォールバックを適用して読んだ。
    UnknownOptionalFeature(String),
    /// 未知の値 (`Unknown` として読んだ enum の値)。その種類のフォールバックを適用する。
    /// `path` はチャンク (`MODL` や `MOTN#0`) とその中の場所。
    UnknownValue(String),
}

impl fmt::Display for ReadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOptionalFeature(id) => write!(f, "unknown optional feature {id}"),
            Self::UnknownValue(path) => write!(f, "unknown value at {path}"),
        }
    }
}

/// 読み込みの報告。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadReport {
    /// `FEAT` チャンクで宣言されていた機能 (知っているものも含む、宣言のまま)。
    pub features: Vec<FeatureDecl>,
    /// 警告 (知らない任意の機能と未知の値)。
    pub warnings: Vec<ReadWarning>,
}

/// バイト列からバンドルを既定の上限 ([`ReadLimits::default`]) で読み込む。警告は捨てる
/// ([`read_bundle_with_report`] を参照)。
pub fn read_bundle(bytes: &[u8]) -> Result<Bundle, FormatError> {
    read_bundle_with_report(bytes).map(|(bundle, _)| bundle)
}

/// バイト列からバンドルを既定の上限 ([`ReadLimits::default`]) で読み込み、警告を含む報告も返す。
///
/// コンテナの整合性 (マジック・版・範囲・境界・重なり・必須チャンク) を検査したうえで、
/// 機能の宣言 (`FEAT`) を調べ、モデルのスキーマバージョンを (どのペイロードもデコードする前に) 検査し、最後に
/// `ugoe_model::validate::validate` と (物理があれば) `validate_physics` を通す。
/// 検証に失敗したモデルは下流 (変形・描画) でパニックしうるため、ここで弾く。
///
/// - 知らない必須の機能があれば [`FormatError::UnsupportedFeatures`] で拒否する
///   (ペイロードのデコードより先に調べるので、知らない形のデータでもこのエラーになる)。
/// - `MODL` の `schema_version` が新しすぎれば [`FormatError::UnsupportedVersion`] で拒否する。
///   `schema_version` だけを先に読むので、形の変わった新しい版のモデルでもこのエラーになる。
/// - 知らない任意の機能と未知の値は、[`ReadWarning`] として報告して読み込みを続ける。
/// - 未知のタグのチャンクは読み飛ばす。
pub fn read_bundle_with_report(bytes: &[u8]) -> Result<(Bundle, ReadReport), FormatError> {
    read_bundle_with(bytes, ReadLimits::default())
}

/// 読み込みの上限 `limits` を指定してバンドルを読み込み、警告を含む報告も返す。
///
/// 検査は [`read_bundle_with_report`] と同じ。展開したバイトが上限を超えそうになったら、展開の途中でやめて
/// [`FormatError::ReadLimitExceeded`] を返す。
pub fn read_bundle_with(
    bytes: &[u8],
    limits: ReadLimits,
) -> Result<(Bundle, ReadReport), FormatError> {
    read_with_known(bytes, limits, |id| features::find(id).is_some())
}

/// 知っている機能の判定 `known` を指定して読む (試験用の機能 ID で仕組みを確かめるため)。
pub(crate) fn read_with_known(
    bytes: &[u8],
    limits: ReadLimits,
    known: impl Fn(&str) -> bool,
) -> Result<(Bundle, ReadReport), FormatError> {
    let mut payloads = Payloads::new(bytes, limits);
    let Container {
        entries,
        declared,
        mut warnings,
    } = read_container(&mut payloads, known)?;
    let mut body = check_schema_version_first(&entries, &mut payloads, ChunkTag::MODL)?;

    let mut model = None;
    let mut physics = None;
    let mut meta = None;
    let mut motions = Vec::new();
    let mut expressions = Vec::new();
    let mut textures = Vec::new();
    let mut normal_maps = Vec::new();

    for entry in &entries {
        match entry.tag {
            ChunkTag::MODL => {
                if model.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                let payload = match body.take() {
                    Some(payload) => payload,
                    None => payloads.get(entry)?,
                };
                let decoded: Model = decode(&payload)?;
                check_schema_version(decoded.schema_version)?;
                model = Some(decoded);
            }
            ChunkTag::PHYS => {
                if physics.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                physics = Some(decode(&payloads.get(entry)?)?);
            }
            ChunkTag::META => {
                if meta.is_some() {
                    return Err(FormatError::DuplicateChunk(entry.tag));
                }
                meta = Some(serde_json::from_slice(&payloads.get(entry)?)?);
            }
            ChunkTag::MOTN => motions.push(decode(&payloads.get(entry)?)?),
            ChunkTag::EXPR => expressions.push(decode(&payloads.get(entry)?)?),
            ChunkTag::TEXR => textures.push(read_texture(entry, &mut payloads, textures.len())?),
            ChunkTag::NRML => {
                normal_maps.push(read_normal_map(entry, &mut payloads, normal_maps.len())?);
            }
            // モジュールのファイルを取り違えたとき、黙って読み飛ばさずに知らせる。
            ChunkTag::MODU => return Err(FormatError::UnexpectedChunk(entry.tag)),
            // 機能の宣言は先に読んだ。前方互換のため、未知のタグは読み飛ばす。
            _ => {}
        }
    }

    let model = model.ok_or(FormatError::MissingChunk(ChunkTag::MODL))?;
    check_texture_count(&model.textures, textures.len())?;
    check_normal_map_count(&model.textures, normal_maps.len())?;
    let errors = ugoe_model::validate::validate(&model);
    if !errors.is_empty() {
        return Err(FormatError::Invalid(errors));
    }
    // 物理の参照先と値の検査 (`docs/spec/physics.md#検証`)。モデルと同じく、不正なものは弾く。
    if let Some(physics) = &physics {
        let errors = ugoe_model::validate::validate_physics(physics, &model);
        if !errors.is_empty() {
            return Err(FormatError::InvalidPhysics(errors));
        }
    }
    // モーションと表情の数値 (`docs/spec/model.md#モーションと表情の検証`)。最初に通らなかったものを返す。
    for motion in &motions {
        crate::check_motion(motion)?;
    }
    for expression in &expressions {
        crate::check_expression(expression)?;
    }

    let bundle = Bundle {
        model,
        physics,
        motions,
        expressions,
        textures,
        normal_maps,
        meta,
    };
    warnings.extend(
        unknown_values(&bundle)
            .into_iter()
            .map(ReadWarning::UnknownValue),
    );
    Ok((
        bundle,
        ReadReport {
            features: declared,
            warnings,
        },
    ))
}

/// 検査済みのコンテナ: チャンク表と機能の宣言 ([`read_container`])。
pub(crate) struct Container {
    pub(crate) entries: Vec<Entry>,
    /// `FEAT` の宣言のまま。
    pub(crate) declared: Vec<FeatureDecl>,
    /// 知らない任意の機能の警告。
    pub(crate) warnings: Vec<ReadWarning>,
}

/// コンテナのチャンク表を読み、機能の宣言 (`FEAT`) を調べる (バンドルとモジュールで共通)。
///
/// 機能の宣言は、ほかのチャンクをデコードする前に調べる。知らない必須の機能があれば
/// [`FormatError::UnsupportedFeatures`]。
pub(crate) fn read_container(
    payloads: &mut Payloads<'_>,
    known: impl Fn(&str) -> bool,
) -> Result<Container, FormatError> {
    let entries = read_table(payloads.bytes)?;
    let mut feat = entries.iter().filter(|e| e.tag == ChunkTag::FEAT);
    let declared = match (feat.next(), feat.next()) {
        (None, _) => Vec::new(),
        (Some(entry), None) => decode::<FeatureTable>(&payloads.get(entry)?)?.features,
        (Some(_), Some(entry)) => return Err(FormatError::DuplicateChunk(entry.tag)),
    };
    let warnings = features::check(&declared, known)
        .map_err(FormatError::UnsupportedFeatures)?
        .into_iter()
        .map(ReadWarning::UnknownOptionalFeature)
        .collect();
    Ok(Container {
        entries,
        declared,
        warnings,
    })
}

/// `TEXR` チャンクを読む。`index` はテクスチャの番号 (エラーの報告用)。
pub(crate) fn read_texture(
    entry: &Entry,
    payloads: &mut Payloads<'_>,
    index: usize,
) -> Result<TextureData, FormatError> {
    let bytes = payloads.get(entry)?.into_owned();
    let format =
        TextureFormat::detect(&bytes).ok_or(FormatError::UnknownTextureFormat { index })?;
    Ok(TextureData { format, bytes })
}

/// `NRML` チャンクを読む。`index` は `NRML` チャンクの順番 (エラーの報告用)。
pub(crate) fn read_normal_map(
    entry: &Entry,
    payloads: &mut Payloads<'_>,
    index: usize,
) -> Result<TextureData, FormatError> {
    let bytes = payloads.get(entry)?.into_owned();
    let format =
        TextureFormat::detect(&bytes).ok_or(FormatError::UnknownNormalMapFormat { index })?;
    Ok(TextureData { format, bytes })
}

/// バンドルの中の未知の値の場所 (チャンクの名前付き) をすべて返す。
fn unknown_values(bundle: &Bundle) -> Vec<String> {
    use ugoe_model::unknown;

    let mut found: Vec<String> = unknown::in_model(&bundle.model)
        .into_iter()
        .map(|path| format!("{}: {path}", ChunkTag::MODL))
        .collect();
    if let Some(physics) = &bundle.physics {
        let paths = unknown::in_physics(physics);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}: {p}", ChunkTag::PHYS)),
        );
    }
    for (index, motion) in bundle.motions.iter().enumerate() {
        let paths = unknown::in_motion(motion);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}#{index}: {p}", ChunkTag::MOTN)),
        );
    }
    for (index, expression) in bundle.expressions.iter().enumerate() {
        let paths = unknown::in_expression(expression);
        found.extend(
            paths
                .into_iter()
                .map(|p| format!("{}#{index}: {p}", ChunkTag::EXPR)),
        );
    }
    found
}

/// 検査済みのチャンク表の 1 項目。
#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) tag: ChunkTag,
    pub(crate) flags: u32,
    start: usize,
    end: usize,
}

/// 1 つのファイルのペイロードを、読み込みの上限 ([`ReadLimits`]) の中で取り出す。
/// 展開したバイトの合計を数える (バンドルとモジュールで共通)。
pub(crate) struct Payloads<'a> {
    bytes: &'a [u8],
    limits: ReadLimits,
    /// これまでに展開できたバイトの合計。失敗した展開は数えない。
    decompressed: u64,
}

impl<'a> Payloads<'a> {
    pub(crate) fn new(bytes: &'a [u8], limits: ReadLimits) -> Self {
        Self {
            bytes,
            limits,
            decompressed: 0,
        }
    }

    /// `entry` のペイロードを取り出す (圧縮されていれば展開する)。範囲は `read_table` で検査済み。
    ///
    /// 展開は、1 チャンクの上限と合計の残りの小さいほうまでしか読まない。それを超えるバイトが続けば、
    /// 展開しきらずに [`FormatError::ReadLimitExceeded`] を返す。
    pub(crate) fn get(&mut self, entry: &Entry) -> Result<Cow<'a, [u8]>, FormatError> {
        let raw = self
            .bytes
            .get(entry.start..entry.end)
            .ok_or(FormatError::ChunkOutOfBounds { tag: entry.tag })?;
        if entry.flags & CHUNK_FLAG_ZSTD == 0 {
            return Ok(Cow::Borrowed(raw));
        }
        let ReadLimits {
            max_chunk_decompressed,
            max_total_decompressed,
        } = self.limits;
        let remaining = max_total_decompressed.saturating_sub(self.decompressed);
        let (cap, limit, max) = if remaining < max_chunk_decompressed {
            (
                remaining,
                ReadLimit::TotalDecompressed,
                max_total_decompressed,
            )
        } else {
            (
                max_chunk_decompressed,
                ReadLimit::ChunkDecompressed,
                max_chunk_decompressed,
            )
        };
        let decoder = ruzstd::decoding::StreamingDecoder::new(raw)
            .map_err(|e| FormatError::Decompress(e.to_string()))?;
        let mut out = Vec::new();
        decoder
            .take(cap.saturating_add(1))
            .read_to_end(&mut out)
            .map_err(|e| FormatError::Decompress(e.to_string()))?;
        let len = out.len() as u64;
        if len > cap {
            return Err(FormatError::ReadLimitExceeded {
                tag: entry.tag,
                limit,
                max,
            });
        }
        self.decompressed += len;
        Ok(Cow::Owned(out))
    }
}

/// ヘッダとチャンク表を読み、各チャンクの範囲を検査する。
pub(crate) fn read_table(bytes: &[u8]) -> Result<Vec<Entry>, FormatError> {
    let header = bytes.get(..HEADER_LEN).ok_or(FormatError::Truncated)?;
    if header[0..4] != MAGIC {
        return Err(FormatError::BadMagic);
    }
    let version = read_u32(header, 4)?;
    if version != CONTAINER_VERSION {
        return Err(FormatError::UnsupportedContainerVersion {
            found: version,
            supported: CONTAINER_VERSION,
        });
    }
    let header_flags = read_u32(header, 8)?;
    if header_flags != 0 {
        return Err(FormatError::UnsupportedFlags(header_flags));
    }
    let chunk_count = read_u32(header, 12)? as usize;

    // 表が入力に収まることを先に確かめる。これで表の大きさ (= 確保量) は入力長で抑えられる。
    let table_end = chunk_count
        .checked_mul(CHUNK_ENTRY_LEN)
        .and_then(|len| len.checked_add(HEADER_LEN))
        .ok_or(FormatError::Truncated)?;
    let table = bytes
        .get(HEADER_LEN..table_end)
        .ok_or(FormatError::Truncated)?;

    let mut entries = Vec::with_capacity(chunk_count);
    for raw in table.as_chunks::<CHUNK_ENTRY_LEN>().0 {
        let tag = ChunkTag([raw[0], raw[1], raw[2], raw[3]]);
        let flags = read_u32(raw, 4)?;
        let offset = read_u64(raw, 8)?;
        let length = read_u64(raw, 16)?;
        if flags & !CHUNK_FLAG_ZSTD != 0 {
            return Err(FormatError::UnsupportedChunkFlags { tag, flags });
        }
        let start = usize::try_from(offset).map_err(|_| FormatError::ChunkOutOfBounds { tag })?;
        let end = usize::try_from(length)
            .ok()
            .and_then(|len| start.checked_add(len))
            .filter(|&end| end <= bytes.len())
            .ok_or(FormatError::ChunkOutOfBounds { tag })?;
        if start % CHUNK_ALIGN != 0 {
            return Err(FormatError::MisalignedChunk { tag, offset });
        }
        if start < table_end {
            return Err(FormatError::OverlappingChunks { tag });
        }
        entries.push(Entry {
            tag,
            flags,
            start,
            end,
        });
    }

    // チャンク同士の重なりを検査する (表の順序は保ったまま、位置順に並べた写しで調べる)。
    // 長さ 0 のチャンクはどのバイトも占めないので、何とも重ならない。
    let mut ranges: Vec<(usize, usize, ChunkTag)> = entries
        .iter()
        .filter(|e| e.start < e.end)
        .map(|e| (e.start, e.end, e.tag))
        .collect();
    ranges.sort_unstable_by_key(|&(start, end, _)| (start, end));
    for pair in ranges.windows(2) {
        if let [(_, prev_end, _), (start, _, tag)] = pair
            && start < prev_end
        {
            return Err(FormatError::OverlappingChunks { tag: *tag });
        }
    }

    Ok(entries)
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, FormatError> {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(FormatError::Truncated)
}

fn read_u64(bytes: &[u8], at: usize) -> Result<u64, FormatError> {
    bytes
        .get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(FormatError::Truncated)
}

pub(crate) fn align_up(value: usize) -> Option<usize> {
    value
        .checked_add(CHUNK_ALIGN - 1)
        .map(|v| v / CHUNK_ALIGN * CHUNK_ALIGN)
}

/// `TEXR` の数がテクスチャの参照 `textures` の数と一致するか。
pub(crate) fn check_texture_count(
    textures: &[TextureRef],
    actual: usize,
) -> Result<(), FormatError> {
    if textures.len() == actual {
        Ok(())
    } else {
        Err(FormatError::TextureCountMismatch {
            expected: textures.len(),
            actual,
        })
    }
}

/// `NRML` の数が `textures` のうち `normal_map` を持つものの数と一致するか。
pub(crate) fn check_normal_map_count(
    textures: &[TextureRef],
    actual: usize,
) -> Result<(), FormatError> {
    let expected = textures.iter().filter(|t| t.normal_map.is_some()).count();
    if expected == actual {
        Ok(())
    } else {
        Err(FormatError::NormalMapCountMismatch { expected, actual })
    }
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, FormatError> {
    Ok(rmp_serde::to_vec_named(value)?)
}

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, FormatError> {
    let mut de = rmp_serde::Deserializer::from_read_ref(bytes);
    de.set_max_depth(MAX_DEPTH);
    Ok(T::deserialize(&mut de)?)
}

/// 本体のチャンク `tag` (`MODL` / `MODU`) の `schema_version` を、どのペイロードもデコードする前に確かめる。
///
/// 新しすぎれば [`FormatError::UnsupportedVersion`] を返す。形の変わった新しい版のファイルが、
/// デコードのエラーでなくこのエラーになるようにするため (`docs/spec/file-formats.md#バージョニング`)。
/// 表で最初の `tag` のチャンクのペイロード (展開済み) を返すので、二度展開しないようにデコードではこれを使う。
/// 展開や先読みに失敗したときはここではエラーにせず、チャンクを順に読むところに任せる
/// (壊れた入力のエラーを今までと変えないため)。
pub(crate) fn check_schema_version_first<'a>(
    entries: &[Entry],
    payloads: &mut Payloads<'a>,
    tag: ChunkTag,
) -> Result<Option<Cow<'a, [u8]>>, FormatError> {
    let Some(entry) = entries.iter().find(|e| e.tag == tag) else {
        return Ok(None);
    };
    let Ok(payload) = payloads.get(entry) else {
        return Ok(None);
    };
    if let Some(version) = peek_schema_version(&payload) {
        check_schema_version(version)?;
    }
    Ok(Some(payload))
}

/// MessagePack のマップから `schema_version` の値だけを読む。
///
/// マップのキーを順に見て、`schema_version` 以外の値は型に当てはめずに読み飛ばす
/// (入れ子の深さの上限は [`decode`] と同じ。`rmp_serde` はマップを最後まで読まないとエラーにするので、
/// 見つけた後も最後まで読み飛ばす)。マップでない・キーが文字列でない・値が `u32` でない・
/// 見つからない・途中で壊れているときは `None`。
pub(crate) fn peek_schema_version(bytes: &[u8]) -> Option<u32> {
    use serde::de::{self, Deserializer as _, IgnoredAny, MapAccess, Visitor};

    /// マップのキー: `schema_version` かそれ以外か。
    enum Key {
        SchemaVersion,
        Other,
    }

    impl<'de> de::Deserialize<'de> for Key {
        fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct KeyVisitor;
            impl Visitor<'_> for KeyVisitor {
                type Value = Key;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("a field name")
                }
                fn visit_str<E: de::Error>(self, key: &str) -> Result<Key, E> {
                    Ok(if key == "schema_version" {
                        Key::SchemaVersion
                    } else {
                        Key::Other
                    })
                }
            }
            deserializer.deserialize_str(KeyVisitor)
        }
    }

    struct VersionVisitor;
    impl<'de> Visitor<'de> for VersionVisitor {
        type Value = Option<u32>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a map with schema_version")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Option<u32>, A::Error> {
            let mut version = None;
            while let Some(key) = map.next_key::<Key>()? {
                match key {
                    Key::SchemaVersion if version.is_none() => {
                        version = Some(map.next_value::<u32>()?);
                    }
                    _ => {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
            }
            Ok(version)
        }
    }

    let mut de = rmp_serde::Deserializer::from_read_ref(bytes);
    de.set_max_depth(MAX_DEPTH);
    de.deserialize_map(VersionVisitor).ok().flatten()
}

#[cfg(test)]
pub(crate) mod tests {
    use ugoe_core::Vec2;
    use ugoe_model::deformer::{Deformer, DeformerKind, RotationDeformer, RotationKeyform};
    use ugoe_model::expression::{ExpressionOp, ExpressionParameter};
    use ugoe_model::motion::{Curve, CurveTarget, MotionEvent, Point, Segment};
    use ugoe_model::physics::{
        NormalizationRange, PhysicsBlend, PhysicsInput, PhysicsNormalization, PhysicsOutput,
        PhysicsParticle, PhysicsRig, PhysicsSource, PhysicsTarget,
    };
    use ugoe_model::{DepthRotation, KeyformGrid, KeyformInference, KeyformOrigin, TextureRef};

    use super::*;

    pub(crate) fn png() -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(b"not really a png body");
        bytes
    }

    pub(crate) fn ktx2() -> Vec<u8> {
        let mut bytes = KTX2_SIGNATURE.to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        bytes
    }

    fn physics() -> PhysicsSettings {
        let range = NormalizationRange {
            min: -10.0,
            default: 0.0,
            max: 10.0,
        };
        PhysicsSettings {
            gravity: Vec2::new(0.0, -1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: vec![PhysicsRig {
                id: "Hair".into(),
                name: "Hair".into(),
                inputs: vec![PhysicsInput {
                    parameter: "X".into(),
                    source: PhysicsSource::X,
                    weight: 100.0,
                    reflect: false,
                }],
                outputs: vec![PhysicsOutput {
                    target: PhysicsTarget::Parameter { id: "X".into() },
                    particle: 1,
                    source: PhysicsSource::Angle,
                    scale: 1.5,
                    weight: 100.0,
                    reflect: true,
                    blend: PhysicsBlend::Overwrite,
                }],
                particles: vec![
                    PhysicsParticle {
                        position: Vec2::ZERO,
                        mobility: 1.0,
                        delay: 1.0,
                        acceleration: 1.0,
                        radius: 0.0,
                        collision_radius: 0.0,
                    },
                    PhysicsParticle {
                        position: Vec2::new(0.0, 10.0),
                        mobility: 0.9,
                        delay: 0.8,
                        acceleration: 1.5,
                        radius: 10.0,
                        collision_radius: 0.0,
                    },
                ],
                normalization: PhysicsNormalization {
                    position: range,
                    angle: range,
                },
                colliders: vec![],
                frame: None,
            }],
            colliders: vec![],
        }
    }

    fn motion() -> Motion {
        let p = |t, v| Point { t, v };
        Motion {
            name: "idle".into(),
            duration: 2.0,
            looping: true,
            fade_in: 0.5,
            fade_out: 0.25,
            curves: vec![
                Curve {
                    target: CurveTarget::Parameter("X".into()),
                    segments: vec![
                        Segment::Start { p: p(0.0, 0.0) },
                        Segment::Linear { to: p(0.5, 1.0) },
                        Segment::Bezier {
                            c1: p(0.7, 1.0),
                            c2: p(0.8, -1.0),
                            to: p(1.0, -1.0),
                        },
                        Segment::Stepped { to: p(1.5, 0.0) },
                        Segment::InverseStepped { to: p(2.0, 1.0) },
                    ],
                    fade_in: Some(0.1),
                    fade_out: None,
                },
                Curve {
                    target: CurveTarget::PartOpacity("Root".into()),
                    segments: vec![Segment::Start { p: p(0.0, 1.0) }],
                    fade_in: None,
                    fade_out: None,
                },
                Curve {
                    target: CurveTarget::EffectGroup("EyeBlink".into()),
                    segments: vec![Segment::Start { p: p(0.0, 1.0) }],
                    fade_in: None,
                    fade_out: Some(0.2),
                },
            ],
            events: vec![MotionEvent {
                t: 1.0,
                value: "wave".into(),
            }],
        }
    }

    fn expression() -> Expression {
        Expression {
            name: "smile".into(),
            fade_in: 0.3,
            fade_out: 0.0,
            parameters: vec![ExpressionParameter {
                parameter: "X".into(),
                value: 0.5,
                op: ExpressionOp::Multiply,
            }],
        }
    }

    /// モデル・物理・モーション・表情・テクスチャ (PNG と KTX2)・メタデータをすべて持つバンドル。
    fn full_bundle() -> Bundle {
        let mut model = ugoe_model::fixtures::single_quad();
        // 内部タグ付き enum (`DeformerKind`) の往復も確かめるため、デフォーマを 1 つ足す。
        model.deformers.push(Deformer {
            id: "Rot".into(),
            name: "Rot".into(),
            part: "Root".into(),
            parent: None,
            kind: DeformerKind::Rotation(RotationDeformer {
                keyforms: KeyformGrid::constant(RotationKeyform {
                    origin: Vec2::new(128.0, 128.0),
                    angle: 15.0,
                    scale: 1.0,
                    reflect_x: false,
                    reflect_y: true,
                    opacity: 1.0,
                }),
            }),
            mirror: Some("Rot".into()),
            mirror_axis: Some(0.0),
        });
        // オーサリング用のデータ (セルの由来・推定の設定・深度) も往復させる。
        let quad = &mut model.meshes[0];
        quad.keyforms.origins = vec![
            KeyformOrigin::Derived,
            KeyformOrigin::Authored,
            KeyformOrigin::Authored,
        ];
        quad.keyforms.inference = Some(KeyformInference::DepthRotation(DepthRotation {
            angle_scale: 0.5,
            pivot: Vec2::new(0.0, 32.0),
            pivot_depth: -16.0,
            yaw: Some("X".into()),
            pitch: None,
        }));
        quad.depth = Some(vec![0.0, 1.0, 2.0, 3.0]);
        // ライティング (基準の機能 `lighting.normal_map`): 2 枚目のテクスチャにだけ法線マップを添える。
        model.lighting = true;
        model.meshes[0].lit = false;
        model.textures.push(TextureRef {
            name: "compressed".into(),
            width: 4,
            height: 4,
            normal_map: Some("compressed_normal".into()),
        });
        Bundle {
            model,
            physics: Some(physics()),
            motions: vec![
                motion(),
                Motion {
                    name: "empty".into(),
                    duration: 0.0,
                    looping: false,
                    fade_in: 0.0,
                    fade_out: 0.0,
                    curves: vec![],
                    events: vec![],
                },
            ],
            expressions: vec![expression()],
            textures: vec![
                TextureData {
                    format: TextureFormat::Png,
                    bytes: png(),
                },
                TextureData {
                    format: TextureFormat::Ktx2,
                    bytes: ktx2(),
                },
            ],
            normal_maps: vec![TextureData {
                format: TextureFormat::Png,
                bytes: png(),
            }],
            meta: Some(serde_json::json!({ "author": "ugoe", "license": "CC0" })),
        }
    }

    fn model_only() -> Bundle {
        let mut bundle = Bundle::new(ugoe_model::fixtures::single_quad());
        bundle.textures.push(TextureData {
            format: TextureFormat::Png,
            bytes: png(),
        });
        bundle
    }

    /// 書き出しと同じ配置規則で、任意のチャンク列からコンテナを組み立てる (壊れた入力を作る用)。
    pub(crate) fn assemble(chunks: &[(ChunkTag, u32, Vec<u8>)]) -> Vec<u8> {
        let table_end = HEADER_LEN + CHUNK_ENTRY_LEN * chunks.len();
        let mut offsets = Vec::new();
        let mut cursor = table_end;
        for (_, _, payload) in chunks {
            let offset = align_up(cursor).unwrap();
            offsets.push(offset);
            cursor = offset + payload.len();
        }
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
        for ((tag, flags, payload), offset) in chunks.iter().zip(&offsets) {
            out.extend_from_slice(&tag.0);
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&(*offset as u64).to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        }
        for ((_, _, payload), offset) in chunks.iter().zip(&offsets) {
            out.resize(*offset, 0);
            out.extend_from_slice(payload);
        }
        out
    }

    fn model_chunk(model: &Model) -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::MODL, 0, encode(model).unwrap())
    }

    fn texture_chunk() -> (ChunkTag, u32, Vec<u8>) {
        (ChunkTag::TEXR, 0, png())
    }

    /// チャンク表 `index` 番目の項目の、`field` バイト目から始まる u64 を書き換える。
    pub(crate) fn patch_u64(bytes: &mut [u8], index: usize, field: usize, value: u64) {
        let at = HEADER_LEN + CHUNK_ENTRY_LEN * index + field;
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn roundtrip_full_bundle() {
        let bundle = full_bundle();
        let bytes = write_bundle(&bundle).unwrap();
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
    }

    #[test]
    fn roundtrip_compressed_bundle() {
        let bundle = full_bundle();
        let bytes = write_bundle_with(&bundle, WriteOptions { compress: true }).unwrap();
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
        // テクスチャと法線マップ以外のチャンクには圧縮フラグが立つ。
        let entries = read_table(&bytes).unwrap();
        for entry in entries {
            let expected = if entry.tag == ChunkTag::TEXR || entry.tag == ChunkTag::NRML {
                0
            } else {
                CHUNK_FLAG_ZSTD
            };
            assert_eq!(entry.flags, expected, "{}", entry.tag);
        }
    }

    #[test]
    fn roundtrip_model_only() {
        let bundle = model_only();
        assert_eq!(
            read_bundle(&write_bundle(&bundle).unwrap()).unwrap(),
            bundle
        );
    }

    /// アバタープロファイル (基準の機能 `avatar.profile`) は `MODL` に入り、機能として宣言しない。
    #[test]
    fn roundtrip_avatar_profile() {
        let mut bundle = model_only();
        bundle.model = ugoe_model::fixtures::avatar_profile();
        assert!(features::detect(&bundle).is_empty());
        for compress in [false, true] {
            let bytes = write_bundle_with(&bundle, WriteOptions { compress }).unwrap();
            let (read, report) = read_bundle_with_report(&bytes).unwrap();
            assert_eq!(read, bundle);
            assert!(report.features.is_empty());
        }
        let json = crate::model_to_json(&bundle.model).unwrap();
        assert_eq!(crate::model_from_json(&json).unwrap(), bundle.model);
    }

    fn limits(chunk: u64, total: u64) -> ReadLimits {
        ReadLimits::default()
            .with_max_chunk_decompressed(chunk)
            .with_max_total_decompressed(total)
    }

    fn zstd(raw: &[u8]) -> Vec<u8> {
        ruzstd::encoding::compress_to_vec(raw, ruzstd::encoding::CompressionLevel::Fastest)
    }

    /// 既定の上限は、仕様がどの読み込み器にも求める 1 チャンク 256 MiB と、合計 1 GiB。
    #[test]
    fn default_read_limits() {
        let limits = ReadLimits::default();
        assert_eq!(limits.max_chunk_decompressed, 256 * 1024 * 1024);
        assert_eq!(limits.max_total_decompressed, 1024 * 1024 * 1024);
    }

    /// 展開後の大きさが 1 チャンクの上限を超えるチャンクは、展開しきらずに拒否する。
    #[test]
    fn rejects_payloads_over_the_chunk_limit() {
        let bytes = assemble(&[(ChunkTag::META, CHUNK_FLAG_ZSTD, zstd(&[7u8; 64]))]);
        let entry = &read_table(&bytes).unwrap()[0];
        let mut payloads = Payloads::new(&bytes, limits(64, u64::MAX));
        assert_eq!(&*payloads.get(entry).unwrap(), [7u8; 64]);
        let mut payloads = Payloads::new(&bytes, limits(63, u64::MAX));
        assert!(matches!(
            payloads.get(entry),
            Err(FormatError::ReadLimitExceeded {
                tag: ChunkTag::META,
                limit: ReadLimit::ChunkDecompressed,
                max: 63,
            })
        ));
    }

    /// 合計の上限は、展開できたバイトを数えて、残りを超えるところで展開をやめる。失敗した展開は数えない。
    #[test]
    fn rejects_payloads_over_the_total_limit() {
        let bytes = assemble(&[
            (ChunkTag::META, CHUNK_FLAG_ZSTD, zstd(&[1u8; 40])),
            (ChunkTag::META, CHUNK_FLAG_ZSTD, zstd(&[2u8; 30])),
            (ChunkTag::META, CHUNK_FLAG_ZSTD, zstd(&[3u8; 20])),
            (ChunkTag::META, 0, vec![4u8; 1000]),
        ]);
        let entries = read_table(&bytes).unwrap();
        let mut payloads = Payloads::new(&bytes, limits(64, 64));
        assert_eq!(payloads.get(&entries[0]).unwrap().len(), 40);
        // 残りは 24 バイト。1 チャンクの上限 (64) より小さいので、合計の上限として報告する。
        assert!(matches!(
            payloads.get(&entries[1]),
            Err(FormatError::ReadLimitExceeded {
                limit: ReadLimit::TotalDecompressed,
                max: 64,
                ..
            })
        ));
        assert_eq!(payloads.get(&entries[2]).unwrap().len(), 20);
        // 圧縮していないチャンクは数えない。
        assert_eq!(payloads.get(&entries[3]).unwrap().len(), 1000);
        assert!(matches!(
            payloads.get(&entries[2]),
            Err(FormatError::ReadLimitExceeded {
                limit: ReadLimit::TotalDecompressed,
                ..
            })
        ));
    }

    /// 1 つずつは小さな上限に収まる圧縮チャンクを並べた小さなファイルが、合計の上限で拒否される
    /// (T0138 の展開の爆発。テクスチャの数の検査より前、展開の途中で止まる)。
    #[test]
    fn rejects_small_files_that_decompress_over_the_total_limit() {
        let mut image = png();
        image.resize(64 * 1024, 0);
        let texture = zstd(&image);
        let mut chunks = vec![model_chunk(&ugoe_model::fixtures::single_quad())];
        chunks.extend((0..32).map(|_| (ChunkTag::TEXR, CHUNK_FLAG_ZSTD, texture.clone())));
        let bytes = assemble(&chunks);
        assert!(bytes.len() < 64 * 1024, "{}", bytes.len());

        let result = read_bundle_with(&bytes, limits(64 * 1024, 16 * 64 * 1024));
        assert!(
            matches!(
                result,
                Err(FormatError::ReadLimitExceeded {
                    tag: ChunkTag::TEXR,
                    limit: ReadLimit::TotalDecompressed,
                    max: 1_048_576,
                })
            ),
            "{result:?}"
        );
        // 上限を上げれば先へ進み、テクスチャの数の検査で拒否される。
        let result = read_bundle_with(&bytes, limits(64 * 1024, 32 * 64 * 1024));
        assert!(
            matches!(result, Err(FormatError::TextureCountMismatch { .. })),
            "{result:?}"
        );
    }

    /// 上限は読み込みの全体 (機能の宣言・本体の先読み・各チャンク) に効き、展開したバイトだけを数える。
    #[test]
    fn read_limits_cover_the_whole_bundle() {
        let bundle = full_bundle();
        let compressed = write_bundle_with(&bundle, WriteOptions { compress: true }).unwrap();
        let total: u64 = read_table(&compressed)
            .unwrap()
            .iter()
            .filter(|e| e.flags & CHUNK_FLAG_ZSTD != 0)
            .map(|e| {
                Payloads::new(&compressed, ReadLimits::default())
                    .get(e)
                    .unwrap()
                    .len() as u64
            })
            .sum();
        let (read, _) = read_bundle_with(&compressed, limits(u64::MAX, total)).unwrap();
        assert_eq!(read, bundle);
        assert!(matches!(
            read_bundle_with(&compressed, limits(u64::MAX, total - 1)),
            Err(FormatError::ReadLimitExceeded {
                limit: ReadLimit::TotalDecompressed,
                ..
            })
        ));
        // 圧縮していないファイルは、上限が 0 でも読める。
        let plain = write_bundle(&bundle).unwrap();
        assert_eq!(read_bundle_with(&plain, limits(0, 0)).unwrap().0, bundle);
    }

    /// 展開後の上限を超えるペイロードは圧縮しない (読み込みで拒否されるファイルを作らないため)。
    #[test]
    fn oversized_payloads_are_not_compressed() {
        let mut writer = ChunkWriter::new(WriteOptions { compress: true }, &[]).unwrap();
        writer.compress_limit = 8;
        writer.push_encoded(ChunkTag::META, b"12345678".to_vec());
        writer.push_encoded(ChunkTag::META, b"123456789".to_vec());
        let flags: Vec<u32> = writer.chunks.iter().map(|&(_, flags, _)| flags).collect();
        assert_eq!(flags, [CHUNK_FLAG_ZSTD, 0]);
        assert_eq!(&*writer.chunks[1].2, b"123456789");
    }

    #[test]
    fn written_layout_follows_spec() {
        let bytes = write_bundle(&full_bundle()).unwrap();
        assert_eq!(&bytes[0..4], b"UGOE");
        assert_eq!(read_u32(&bytes, 4).unwrap(), CONTAINER_VERSION);
        assert_eq!(read_u32(&bytes, 8).unwrap(), 0);
        // MODL + PHYS + MOTN×2 + EXPR + META + TEXR×2 + NRML
        assert_eq!(read_u32(&bytes, 12).unwrap(), 9);
        let entries = read_table(&bytes).unwrap();
        assert_eq!(entries[0].tag, ChunkTag::MODL);
        assert_eq!(entries[8].tag, ChunkTag::NRML);
        for entry in &entries {
            assert_eq!(entry.start % CHUNK_ALIGN, 0);
        }
    }

    #[test]
    fn unknown_chunks_are_skipped() {
        let model = ugoe_model::fixtures::single_quad();
        let bytes = assemble(&[
            (ChunkTag(*b"XTRA"), 0, vec![0xff; 5]),
            model_chunk(&model),
            // 未知のタグのチャンクは展開もしない (圧縮フラグ付きの壊れた中身でも読み飛ばす)。
            (ChunkTag(*b"ZZZZ"), CHUNK_FLAG_ZSTD, b"garbage".to_vec()),
            texture_chunk(),
        ]);
        let bundle = read_bundle(&bytes).unwrap();
        assert_eq!(bundle, model_only());
    }

    fn assert_err(bytes: &[u8], check: impl Fn(&FormatError) -> bool) {
        match read_bundle(bytes) {
            Ok(_) => panic!("expected an error"),
            Err(e) => assert!(check(&e), "unexpected error: {e:?}"),
        }
    }

    #[test]
    fn rejects_empty_and_truncated_input() {
        assert_err(&[], |e| matches!(e, FormatError::Truncated));
        let bytes = write_bundle(&full_bundle()).unwrap();
        // 末尾を 1 バイトでも欠いたら読めない (最後のチャンクが範囲外になる)。
        for len in 0..bytes.len() {
            assert!(
                read_bundle(&bytes[..len]).is_err(),
                "prefix {len} was accepted"
            );
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[0] = b'X';
        assert_err(&bytes, |e| matches!(e, FormatError::BadMagic));
    }

    #[test]
    fn rejects_unknown_container_version_and_flags() {
        let good = write_bundle(&model_only()).unwrap();
        let mut bytes = good.clone();
        bytes[4] = 1;
        assert_err(&bytes, |e| {
            matches!(e, FormatError::UnsupportedContainerVersion { found: 1, .. })
        });
        let mut bytes = good;
        bytes[8] = 1;
        assert_err(&bytes, |e| matches!(e, FormatError::UnsupportedFlags(1)));
    }

    #[test]
    fn rejects_unknown_chunk_flags() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[HEADER_LEN + 4] = 0b10;
        assert_err(&bytes, |e| {
            matches!(e, FormatError::UnsupportedChunkFlags { flags: 2, .. })
        });
    }

    #[test]
    fn rejects_huge_chunk_count() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_err(&bytes, |e| matches!(e, FormatError::Truncated));
    }

    #[test]
    fn rejects_out_of_range_offsets_and_lengths() {
        let good = write_bundle(&model_only()).unwrap();
        for (field, value) in [(8, u64::MAX), (8, 1 << 40), (16, u64::MAX), (16, 1 << 40)] {
            let mut bytes = good.clone();
            patch_u64(&mut bytes, 0, field, value);
            assert_err(
                &bytes,
                |e| matches!(e, FormatError::ChunkOutOfBounds { tag } if *tag == ChunkTag::MODL),
            );
        }
    }

    #[test]
    fn rejects_misaligned_chunk() {
        let mut bytes = write_bundle(&model_only()).unwrap();
        let entries = read_table(&bytes).unwrap();
        patch_u64(&mut bytes, 1, 8, entries[1].start as u64 + 1);
        patch_u64(
            &mut bytes,
            1,
            16,
            (entries[1].end - entries[1].start - 1) as u64,
        );
        assert_err(&bytes, |e| matches!(e, FormatError::MisalignedChunk { .. }));
    }

    #[test]
    fn rejects_overlapping_chunks() {
        let good = write_bundle(&model_only()).unwrap();
        let entries = read_table(&good).unwrap();

        // 2 つのチャンクが同じ位置を指す。
        let mut bytes = good.clone();
        patch_u64(&mut bytes, 1, 8, entries[0].start as u64);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::OverlappingChunks { .. })
        });

        // チャンクがヘッダ・チャンク表に重なる。
        let mut bytes = good;
        patch_u64(&mut bytes, 0, 8, 0);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::OverlappingChunks { .. })
        });
    }

    /// 長さ 0 のチャンクは何とも重ならない。ほかのチャンクの中を指していても読める
    /// (`docs/spec/file-formats.md#読み込み時の検査`)。
    #[test]
    fn empty_chunks_overlap_nothing() {
        let model = ugoe_model::fixtures::single_quad();
        let good = assemble(&[
            model_chunk(&model),
            (ChunkTag(*b"XTRA"), 0, Vec::new()),
            texture_chunk(),
        ]);
        let entries = read_table(&good).unwrap();
        // 空のチャンクを、MODL のペイロードの中 (16 バイト境界) と、TEXR の先頭に置く。
        for offset in [entries[0].start + CHUNK_ALIGN, entries[2].start] {
            let mut bytes = good.clone();
            patch_u64(&mut bytes, 1, 8, offset as u64);
            assert_eq!(
                read_bundle(&bytes).unwrap(),
                model_only(),
                "offset {offset}"
            );
        }
        // 境界・チャンク表より後ろであること・ファイルの長さの検査は受ける。
        let mut bytes = good.clone();
        patch_u64(&mut bytes, 1, 8, 0);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::OverlappingChunks { .. })
        });
        let mut bytes = good.clone();
        patch_u64(&mut bytes, 1, 8, entries[0].start as u64 + 1);
        assert_err(&bytes, |e| matches!(e, FormatError::MisalignedChunk { .. }));
        let mut bytes = good;
        let past_end = align_up(bytes.len() + 1).unwrap() as u64;
        patch_u64(&mut bytes, 1, 8, past_end);
        assert_err(&bytes, |e| {
            matches!(e, FormatError::ChunkOutOfBounds { .. })
        });
    }

    #[test]
    fn rejects_missing_and_duplicate_chunks() {
        let model = ugoe_model::fixtures::single_quad();
        assert_err(&assemble(&[texture_chunk()]), |e| {
            matches!(e, FormatError::MissingChunk(ChunkTag::MODL))
        });
        assert_err(
            &assemble(&[model_chunk(&model), model_chunk(&model), texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::MODL)),
        );
        let meta = (ChunkTag::META, 0, b"{}".to_vec());
        assert_err(
            &assemble(&[model_chunk(&model), meta.clone(), meta, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::META)),
        );
        let phys = (ChunkTag::PHYS, 0, encode(&physics()).unwrap());
        assert_err(
            &assemble(&[model_chunk(&model), phys.clone(), phys, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::PHYS)),
        );
    }

    #[test]
    fn rejects_texture_problems() {
        let model = ugoe_model::fixtures::single_quad();
        assert_err(&assemble(&[model_chunk(&model)]), |e| {
            matches!(
                e,
                FormatError::TextureCountMismatch {
                    expected: 1,
                    actual: 0
                }
            )
        });
        assert_err(
            &assemble(&[model_chunk(&model), (ChunkTag::TEXR, 0, b"GIF89a".to_vec())]),
            |e| matches!(e, FormatError::UnknownTextureFormat { index: 0 }),
        );
    }

    /// 法線マップは `normal_map` を持つテクスチャの順に並び、テクスチャの番号から引ける。
    #[test]
    fn normal_maps_follow_textures_with_normal_map() {
        let mut bundle = full_bundle();
        assert_eq!(bundle.normal_map(0), None);
        assert_eq!(bundle.normal_map(1), Some(&bundle.normal_maps[0]));
        assert_eq!(bundle.normal_map(2), None);

        bundle.model.textures[0].normal_map = Some("white_normal".into());
        bundle.normal_maps.insert(
            0,
            TextureData {
                format: TextureFormat::Ktx2,
                bytes: ktx2(),
            },
        );
        let read = read_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert_eq!(read, bundle);
        assert_eq!(read.normal_map(0).unwrap().format, TextureFormat::Ktx2);
        assert_eq!(read.normal_map(1).unwrap().format, TextureFormat::Png);
    }

    /// ライティングは基準の機能なので宣言しない。既定値 (なし) のフィールドは書き出さない。
    #[test]
    fn lighting_is_baseline_and_defaults_are_omitted() {
        let bundle = full_bundle();
        assert!(features::detect(&bundle).is_empty());
        let (_, report) = read_bundle_with_report(&write_bundle(&bundle).unwrap()).unwrap();
        assert!(report.features.is_empty());

        let json = crate::model_to_json(&bundle.model).unwrap();
        assert_eq!(crate::model_from_json(&json).unwrap(), bundle.model);
        assert!(json.contains("\"lighting\": true"));
        assert!(json.contains("\"lit\": false"));
        assert!(json.contains("\"normal_map\": \"compressed_normal\""));

        let plain = crate::model_to_json(&ugoe_model::fixtures::single_quad()).unwrap();
        for field in ["lighting", "\"lit\"", "normal_map"] {
            assert!(!plain.contains(field), "{field}");
        }
        let model = crate::model_from_json(&plain).unwrap();
        assert!(!model.lighting);
        assert!(model.meshes[0].lit);
        assert_eq!(model.textures[0].normal_map, None);
    }

    #[test]
    fn rejects_normal_map_problems() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.textures[0].normal_map = Some("white_normal".into());
        assert_err(&assemble(&[model_chunk(&model), texture_chunk()]), |e| {
            matches!(
                e,
                FormatError::NormalMapCountMismatch {
                    expected: 1,
                    actual: 0
                }
            )
        });
        assert_err(
            &assemble(&[
                model_chunk(&model),
                texture_chunk(),
                (ChunkTag::NRML, 0, b"GIF89a".to_vec()),
            ]),
            |e| matches!(e, FormatError::UnknownNormalMapFormat { index: 0 }),
        );
        // 法線マップを持たないモデルに `NRML` がある。
        let plain = ugoe_model::fixtures::single_quad();
        assert_err(
            &assemble(&[
                model_chunk(&plain),
                texture_chunk(),
                (ChunkTag::NRML, 0, png()),
            ]),
            |e| {
                matches!(
                    e,
                    FormatError::NormalMapCountMismatch {
                        expected: 0,
                        actual: 1
                    }
                )
            },
        );
    }

    #[test]
    fn rejects_broken_payloads() {
        let model = ugoe_model::fixtures::single_quad();
        // MessagePack として壊れている。
        assert_err(
            &assemble(&[(ChunkTag::MODL, 0, vec![0xc1, 0x00]), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        // 途中で切れた MessagePack。
        let mut short = encode(&model).unwrap();
        short.truncate(short.len() / 2);
        assert_err(
            &assemble(&[(ChunkTag::MODL, 0, short), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
        // zstd として壊れている。
        assert_err(
            &assemble(&[
                (ChunkTag::MODL, CHUNK_FLAG_ZSTD, b"not zstd".to_vec()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::Decompress(_)),
        );
        // META が JSON でない。
        assert_err(
            &assemble(&[
                model_chunk(&model),
                (ChunkTag::META, 0, b"{".to_vec()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::Json(_)),
        );
    }

    #[test]
    fn rejects_newer_schema_version() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.schema_version = ugoe_model::SCHEMA_VERSION + 1;
        assert_err(&assemble(&[model_chunk(&model), texture_chunk()]), |e| {
            matches!(e, FormatError::UnsupportedVersion { .. })
        });
    }

    #[test]
    fn rejects_newer_schema_version_before_decoding() {
        use serde_json::json;
        // 新しい版で形が変わり、今の `Model` としてはデコードできないモデル。
        let newer = json!({
            "schema_version": ugoe_model::SCHEMA_VERSION + 1,
            "meta": "a newer layout",
            "nodes": [{ "id": "Root" }],
        });
        assert_err(
            &assemble(&[
                (ChunkTag::MODL, 0, encode(&newer).unwrap()),
                texture_chunk(),
            ]),
            |e| {
                matches!(
                    e,
                    FormatError::UnsupportedVersion { found, supported }
                        if *found == ugoe_model::SCHEMA_VERSION + 1
                            && *supported == ugoe_model::SCHEMA_VERSION
                )
            },
        );
        // 表で先にある別のチャンク (形の変わった物理) もデコードしない。
        assert_err(
            &assemble(&[
                (ChunkTag::PHYS, 0, encode(&json!({ "rigs": 1 })).unwrap()),
                (ChunkTag::MODL, 0, encode(&newer).unwrap()),
                texture_chunk(),
            ]),
            |e| matches!(e, FormatError::UnsupportedVersion { .. }),
        );
        // 同じ版なら今までどおりデコードのエラー。
        let same =
            json!({ "schema_version": ugoe_model::SCHEMA_VERSION, "meta": "a newer layout" });
        assert_err(
            &assemble(&[(ChunkTag::MODL, 0, encode(&same).unwrap()), texture_chunk()]),
            |e| matches!(e, FormatError::MsgPackDecode(_)),
        );
    }

    #[test]
    fn peeks_schema_version() {
        use serde_json::json;
        let peek = |value: serde_json::Value| peek_schema_version(&encode(&value).unwrap());
        let model = ugoe_model::fixtures::single_quad();
        assert_eq!(peek_schema_version(&encode(&model).unwrap()), Some(0));
        // 前にあるフィールドは中身を見ずに読み飛ばす。
        assert_eq!(
            peek(json!({ "a": { "b": [1, 2, { "c": null }] }, "schema_version": 7, "z": 1 })),
            Some(7)
        );
        // 読めないものは None (全体のデコードに任せる)。
        assert_eq!(peek(json!({ "meta": {} })), None);
        assert_eq!(peek(json!([0])), None);
        assert_eq!(peek(json!(0)), None);
        assert_eq!(peek(json!({ "schema_version": "1" })), None);
        assert_eq!(peek(json!({ "schema_version": -1 })), None);
        assert_eq!(
            peek(json!({ "schema_version": u64::from(u32::MAX) + 1 })),
            None
        );
        assert_eq!(peek_schema_version(&[]), None);
        assert_eq!(peek_schema_version(&[0xc1]), None);
        // キーが文字列でない。
        let mut int_key = vec![0x81, 0x00, 0x00];
        assert_eq!(peek_schema_version(&int_key), None);
        // 値の途中で切れている。
        int_key = encode(&json!({ "schema_version": 70000 })).unwrap();
        int_key.pop();
        assert_eq!(peek_schema_version(&int_key), None);
        // 深すぎる入れ子はスタックを使い切らずに None。
        let mut deep = vec![0x82, 0xa1, b'a'];
        deep.extend(std::iter::repeat_n(0x91, 100_000));
        deep.push(0xc0);
        deep.extend(encode(&json!("schema_version")).unwrap());
        deep.push(0x05);
        assert_eq!(peek_schema_version(&deep), None);
        // 上限より浅ければ読める。
        let mut shallow = vec![0x82, 0xa1, b'a'];
        shallow.extend(std::iter::repeat_n(0x91, 100));
        shallow.push(0xc0);
        shallow.extend(encode(&json!("schema_version")).unwrap());
        shallow.push(0x05);
        assert_eq!(peek_schema_version(&shallow), Some(5));
    }

    #[test]
    fn rejects_invalid_model() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.meshes[0].indices.push(0);
        assert_err(
            &assemble(&[model_chunk(&model), texture_chunk()]),
            |e| matches!(e, FormatError::Invalid(errors) if !errors.is_empty()),
        );
    }

    #[test]
    fn rejects_invalid_physics() {
        let mut bundle = model_only();
        let mut physics = physics();
        physics.rigs[0].inputs[0].parameter = "Missing".into();
        bundle.physics = Some(physics);
        assert_err(
            &write_bundle(&bundle).unwrap(),
            |e| matches!(e, FormatError::InvalidPhysics(errors) if !errors.is_empty()),
        );
    }

    #[test]
    fn rejects_non_finite_motion_and_expression() {
        // MessagePack は NaN と無限大を書ける。読み込みの検査で弾く。
        let mut bundle = model_only();
        let mut bad = motion();
        bad.name = "broken".into();
        bad.duration = f32::NAN;
        bundle.motions = vec![motion(), bad];
        assert_err(
            &write_bundle(&bundle).unwrap(),
            |e| matches!(e, FormatError::InvalidMotion { name, errors } if name == "broken" && errors.len() == 1),
        );

        let mut bundle = model_only();
        let mut bad = expression();
        bad.parameters[0].value = f32::INFINITY;
        bundle.expressions = vec![bad];
        assert_err(
            &write_bundle(&bundle).unwrap(),
            |e| matches!(e, FormatError::InvalidExpression { name, errors } if name == "smile" && errors.len() == 1),
        );
    }

    #[test]
    fn single_byte_corruption_never_panics() {
        let good = write_bundle_with(&full_bundle(), WriteOptions { compress: true }).unwrap();
        for i in 0..good.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut bytes = good.clone();
                bytes[i] ^= flip;
                let _ = read_bundle(&bytes);
            }
        }
    }

    fn test_features() -> Vec<FeatureDecl> {
        vec![
            FeatureDecl::optional("test.optional"),
            FeatureDecl::required("test.required"),
        ]
    }

    #[test]
    fn baseline_bundles_have_no_feat_chunk() {
        let bytes = write_bundle(&full_bundle()).unwrap();
        let entries = read_table(&bytes).unwrap();
        assert!(entries.iter().all(|e| e.tag != ChunkTag::FEAT));
        let (_, report) = read_bundle_with_report(&bytes).unwrap();
        assert_eq!(report, ReadReport::default());
    }

    #[test]
    fn roundtrip_feature_declarations() {
        // 試験用の機能 ID を知っているランタイムとして読む。
        let known = |id: &str| id.starts_with("test.");
        for compress in [false, true] {
            let bundle = full_bundle();
            let bytes =
                write_with_features(&bundle, WriteOptions { compress }, &test_features()).unwrap();
            assert_eq!(read_table(&bytes).unwrap()[0].tag, ChunkTag::FEAT);
            let (read, report) = read_with_known(&bytes, ReadLimits::default(), known).unwrap();
            assert_eq!(read, bundle);
            assert_eq!(report.features, test_features());
            assert!(report.warnings.is_empty());
        }
    }

    #[test]
    fn unknown_required_feature_is_rejected_with_ids() {
        let bytes =
            write_with_features(&model_only(), WriteOptions::default(), &test_features()).unwrap();
        assert_err(
            &bytes,
            |e| matches!(e, FormatError::UnsupportedFeatures(ids) if ids == &["test.required"]),
        );
        // 知らない形のモデル (新しい版) でも、デコードより先に機能の宣言で拒否する。
        let feat = FeatureTable {
            features: vec![
                FeatureDecl::required("test.a"),
                FeatureDecl::required("test.b"),
            ],
        };
        let bytes = assemble(&[
            (ChunkTag::MODL, 0, b"future payload".to_vec()),
            (ChunkTag::FEAT, 0, encode(&feat).unwrap()),
        ]);
        assert_err(
            &bytes,
            |e| matches!(e, FormatError::UnsupportedFeatures(ids) if ids == &["test.a", "test.b"]),
        );
    }

    #[test]
    fn unknown_optional_feature_loads_with_warnings() {
        let bundle = model_only();
        let declared = [
            FeatureDecl::optional("test.optional"),
            // 知っている機能 (基準の機能) の宣言は警告にしない。
            FeatureDecl::optional("mask.intersection"),
        ];
        let bytes = write_with_features(&bundle, WriteOptions::default(), &declared).unwrap();
        let (read, report) = read_bundle_with_report(&bytes).unwrap();
        assert_eq!(read, bundle);
        assert_eq!(report.features, declared);
        assert_eq!(
            report.warnings,
            [ReadWarning::UnknownOptionalFeature("test.optional".into())]
        );
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);
    }

    #[test]
    fn rejects_duplicate_feat_chunks() {
        let model = ugoe_model::fixtures::single_quad();
        let feat = (ChunkTag::FEAT, 0, encode(&FeatureTable::default()).unwrap());
        assert_err(
            &assemble(&[feat.clone(), model_chunk(&model), feat, texture_chunk()]),
            |e| matches!(e, FormatError::DuplicateChunk(ChunkTag::FEAT)),
        );
    }

    /// 新しい版で増えた値 (未知のタグとその中身) を含むバンドルを、MessagePack で組み立てる。
    fn future_bundle_bytes() -> Vec<u8> {
        use serde_json::json;

        let mut model = serde_json::to_value(ugoe_model::fixtures::single_quad()).unwrap();
        model["meshes"][0]["blend"] = json!("overlay");
        model["meshes"][0]["mask_mode"] = json!("xor");
        model["deformers"] = json!([{
            "id": "Bend",
            "name": "Bend",
            "part": "Root",
            "parent": null,
            "kind": { "type": "bezier", "handles": [[0.0, 1.0]], "extra": { "a": 1 } },
        }]);
        model["meshes"][0]["parent"] = json!("Bend");
        // 構造体の未知のフィールドも無視する。
        model["meshes"][0]["future_field"] = json!({ "nested": [1, 2, 3] });

        let mut physics = serde_json::to_value(physics()).unwrap();
        physics["rigs"][0]["outputs"][0]["source"] = json!("velocity");
        let mut motion = serde_json::to_value(motion()).unwrap();
        motion["curves"][1]["target"] = json!({ "kind": "camera", "id": { "zoom": 2 } });
        motion["curves"][0]["segments"][2] =
            json!({ "type": "hermite", "to": { "t": 1.0, "v": 0.0 } });
        let mut expression = serde_json::to_value(expression()).unwrap();
        expression["parameters"][0]["op"] = json!("screen");

        let msgpack = |value: &serde_json::Value| rmp_serde::to_vec_named(value).unwrap();
        assemble(&[
            (ChunkTag::MODL, 0, msgpack(&model)),
            (ChunkTag::PHYS, 0, msgpack(&physics)),
            (ChunkTag::MOTN, 0, msgpack(&motion)),
            (ChunkTag::EXPR, 0, msgpack(&expression)),
            texture_chunk(),
        ])
    }

    #[test]
    fn unknown_enum_tags_in_msgpack_load_with_warnings() {
        let (bundle, report) = read_bundle_with_report(&future_bundle_bytes()).unwrap();
        let mesh = &bundle.model.meshes[0];
        assert_eq!(mesh.blend, ugoe_model::BlendMode::Unknown);
        assert_eq!(mesh.mask_mode, ugoe_model::MaskMode::Unknown);
        assert_eq!(bundle.model.deformers[0].kind, DeformerKind::Unknown);
        assert_eq!(bundle.motions[0].curves[1].target, CurveTarget::Unknown);
        assert_eq!(bundle.motions[0].curves[0].segments[2], Segment::Unknown);
        let warnings: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
        assert_eq!(
            warnings,
            [
                "unknown value at MODL: deformers[Bend].kind",
                "unknown value at MODL: meshes[Quad].blend",
                "unknown value at MODL: meshes[Quad].mask_mode",
                "unknown value at PHYS: rigs[Hair].outputs[0].source",
                "unknown value at MOTN#0: curves[0].segments[2]",
                "unknown value at MOTN#0: curves[1].target",
                "unknown value at EXPR#0: parameters[0].op",
            ]
        );
    }

    #[test]
    fn unknown_values_are_not_written_back() {
        let (bundle, _) = read_bundle_with_report(&future_bundle_bytes()).unwrap();
        match write_bundle(&bundle) {
            Err(FormatError::UnknownValues(paths)) => assert_eq!(paths.len(), 7),
            other => panic!("unexpected result: {other:?}"),
        }
        assert!(matches!(
            crate::model_to_json(&bundle.model),
            Err(FormatError::UnknownValues(_))
        ));
    }

    #[test]
    fn write_rejects_inconsistent_textures() {
        let mut bundle = model_only();
        bundle.textures[0].format = TextureFormat::Ktx2;
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::TextureFormatMismatch { index: 0 })
        ));
        bundle.textures.clear();
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::TextureCountMismatch { .. })
        ));
    }

    #[test]
    fn write_rejects_inconsistent_normal_maps() {
        let mut bundle = full_bundle();
        bundle.normal_maps[0].format = TextureFormat::Ktx2;
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::NormalMapFormatMismatch { index: 0 })
        ));
        bundle.normal_maps.clear();
        assert!(matches!(
            write_bundle(&bundle),
            Err(FormatError::NormalMapCountMismatch {
                expected: 1,
                actual: 0
            })
        ));
    }
}
