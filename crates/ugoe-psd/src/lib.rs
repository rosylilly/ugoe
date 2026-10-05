//! PSD 取り込み。仕様: `docs/ai-integration.md#psd-取り込み`。
//!
//! 出力は PSD 形式から独立した [`LayerTree`] で、エディタ・AI パイプラインはこれだけを見る。
//! PSD パーサは自前の小さな実装 (`parse` モジュール) で、このクレートの外には出さない。
//! 採用の理由は `README.md` を参照。

use std::fmt;

use ugoe_core::Vec2;

mod parse;

use parse::{RawLayer, Section};

/// 取り込みのエラー。
///
/// 壊れた入力 ([`PsdError::is_invalid`])・対応しない機能 ([`PsdError::is_unsupported`])・読み込みの上限の超過
/// ([`PsdError::LimitExceeded`]) に分かれ、どれも理由を種類と値で持つ (英語の文は `Display`。T0571)。
/// 変種を足しても公開 API の破壊的な変更にならないよう `#[non_exhaustive]` にしている
/// (外の `match` は `_` の腕が要る。T0558)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PsdError {
    // ---- 壊れた入力・途中で切れた入力 ----
    /// 先頭の印が `8BPS` でない (PSD のファイルでない)。
    #[error("invalid psd: not a psd file (bad signature)")]
    NotPsd,
    /// ヘッダの版が 1 (PSD) でも 2 (PSB) でもない。
    #[error("invalid psd: unknown version {version}")]
    UnknownVersion { version: u16 },
    /// `section` を読んでいる途中でデータが終わった。`section` は PSD の形式の区画の名前 (英語。`"layer info"` など)。
    #[error("invalid psd: unexpected end of data while reading {section}")]
    Truncated { section: &'static str },
    /// ヘッダのチャンネル数が 1..=56 の外。
    #[error("invalid psd: invalid channel count {channels}")]
    InvalidChannelCount { channels: u16 },
    /// ヘッダの幅・高さが 1..=30000 の外。
    #[error("invalid psd: invalid image size {width}x{height}")]
    InvalidImageSize { width: u32, height: u32 },
    /// レイヤー (またはマスク) の矩形の幅・高さが負か、上限 (60000) を超える。
    #[error("invalid psd: layer bounds out of range: {width}x{height} at ({left}, {top})")]
    LayerBoundsOutOfRange {
        width: i64,
        height: i64,
        left: i32,
        top: i32,
    },
    /// レイヤーのチャンネル数が上限 `max` を超える。
    #[error("invalid psd: too many channels ({channels})")]
    TooManyLayerChannels { channels: u16, max: u16 },
    /// レイヤーの記録のブレンドモードの印が `8BIM` でない。
    #[error("invalid psd: bad blend mode signature in layer record")]
    BadBlendModeSignature,
    /// ZIP で圧縮したチャンネルを展開できない。
    #[error("invalid psd: bad zip channel data")]
    BadZipData,
    /// ZIP で圧縮したチャンネルを展開した大きさ `len` がレイヤーの大きさ `expected` と合わない (バイト)。
    #[error("invalid psd: zip channel data decodes to {len} bytes, expected {expected}")]
    ZipLengthMismatch { len: u64, expected: u64 },
    /// PackBits の行が、行の幅を書ける最も短いバイト数より短い。
    #[error("invalid psd: rle row is too short for the row width")]
    RleRowTooShort,
    /// PackBits の行を展開した長さが行の幅と合わない。
    #[error("invalid psd: rle row decodes to the wrong length")]
    RleRowWrongLength,
    /// レイヤーのない PSD の合成画像のチャンネルが RGB の 3 つに足りない。
    #[error("invalid psd: rgb image needs 3 channels, found {channels}")]
    TooFewCompositeChannels { channels: u16 },
    /// グループの始まりに対応する終わりがない。
    #[error("invalid psd: group start without a matching group end")]
    UnbalancedGroups,
    // ---- 形式としては正しいが対応しない ----
    /// 大きなドキュメント形式 (PSB)。
    #[error("unsupported psd feature: large document format (psb) is not supported")]
    UnsupportedPsb,
    /// 8・16 ビット以外のチャンネル。`depth` はビット数。
    #[error("unsupported psd feature: {depth}-bit channels (only 8-bit and 16-bit are supported)")]
    UnsupportedDepth { depth: u16 },
    /// RGB 以外のカラーモード。`mode` は PSD のカラーモードの番号 (名前は [`color_mode_name`])。
    #[error("unsupported psd feature: {} color mode (only rgb is supported)", ColorModeName(*.mode))]
    UnsupportedColorMode { mode: u16 },
    /// 仕様にない合成画像の圧縮方式 (`compression` は PSD の番号)。
    #[error("unsupported psd feature: image data compression {compression}")]
    UnsupportedCompositeCompression { compression: u16 },
    /// グループの入れ子が `max` 段より深い。
    #[error("unsupported psd feature: groups nested deeper than {max} levels")]
    GroupsTooDeep { max: u32 },
    // ---- 読み込みの上限 ----
    /// チャンネルを展開した大きさが読み込みの上限 ([`ImportLimits`]) を超える。ファイルが壊れているとは限らない
    /// (上限を上げれば読めることがある)。展開する前に止めるので、`size` の平面は確保していない。
    #[error("{limit} of {size} bytes exceeds the import limit of {max} bytes")]
    LimitExceeded {
        /// 超えた上限の種類。
        limit: ImportLimit,
        /// 展開しようとした大きさ (バイト)。チャンネル 1 枚ならその平面の大きさ、合計ならそれまでの合計に
        /// その平面を足したもの (上限を超えたところで止めるので、PSD 全体の合計より小さいことがある)。
        size: u64,
        /// その上限の値 (バイト)。
        max: u64,
    },
}

impl PsdError {
    /// 機械が読む種類 (`"not_psd"` など。バリアントの名前の `snake_case`)。値は **変えない**。
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotPsd => "not_psd",
            Self::UnknownVersion { .. } => "unknown_version",
            Self::Truncated { .. } => "truncated",
            Self::InvalidChannelCount { .. } => "invalid_channel_count",
            Self::InvalidImageSize { .. } => "invalid_image_size",
            Self::LayerBoundsOutOfRange { .. } => "layer_bounds_out_of_range",
            Self::TooManyLayerChannels { .. } => "too_many_layer_channels",
            Self::BadBlendModeSignature => "bad_blend_mode_signature",
            Self::BadZipData => "bad_zip_data",
            Self::ZipLengthMismatch { .. } => "zip_length_mismatch",
            Self::RleRowTooShort => "rle_row_too_short",
            Self::RleRowWrongLength => "rle_row_wrong_length",
            Self::TooFewCompositeChannels { .. } => "too_few_composite_channels",
            Self::UnbalancedGroups => "unbalanced_groups",
            Self::UnsupportedPsb => "unsupported_psb",
            Self::UnsupportedDepth { .. } => "unsupported_depth",
            Self::UnsupportedColorMode { .. } => "unsupported_color_mode",
            Self::UnsupportedCompositeCompression { .. } => "unsupported_composite_compression",
            Self::GroupsTooDeep { .. } => "groups_too_deep",
            Self::LimitExceeded { .. } => "limit_exceeded",
        }
    }

    /// 壊れた入力・途中で切れた入力か。
    pub fn is_invalid(&self) -> bool {
        !self.is_unsupported() && !matches!(self, Self::LimitExceeded { .. })
    }

    /// 形式としては正しいが対応しない機能か (PSB・8/16 ビット以外・RGB 以外・深すぎるグループ・仕様にない合成画像の圧縮)。
    pub fn is_unsupported(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedPsb
                | Self::UnsupportedDepth { .. }
                | Self::UnsupportedColorMode { .. }
                | Self::UnsupportedCompositeCompression { .. }
                | Self::GroupsTooDeep { .. }
        )
    }
}

/// PSD のカラーモードの番号 `mode` の名前 (`"cmyk"` など。小文字の英語)。知らない番号は `None`。
pub fn color_mode_name(mode: u16) -> Option<&'static str> {
    Some(match mode {
        0 => "bitmap",
        1 => "grayscale",
        2 => "indexed",
        3 => "rgb",
        4 => "cmyk",
        7 => "multichannel",
        8 => "duotone",
        9 => "lab",
        _ => return None,
    })
}

/// [`PsdError::UnsupportedColorMode`] の文のカラーモードの名前 (知らない番号は `unknown (番号)`)。
struct ColorModeName(u16);

impl fmt::Display for ColorModeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match color_mode_name(self.0) {
            Some(name) => f.write_str(name),
            None => write!(f, "unknown ({})", self.0),
        }
    }
}

/// どの読み込みの上限を超えたか ([`PsdError::LimitExceeded`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportLimit {
    /// チャンネル 1 枚の展開後の大きさ ([`ImportLimits::max_channel_decompressed`])。
    ChannelDecompressed,
    /// 1 つの PSD で展開するバイトの合計 ([`ImportLimits::max_total_decompressed`])。
    TotalDecompressed,
}

impl ImportLimit {
    /// 安定した名前 (`channel_decompressed` / `total_decompressed`)。エラーのコードの引数に使う。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChannelDecompressed => "channel_decompressed",
            Self::TotalDecompressed => "total_decompressed",
        }
    }
}

impl fmt::Display for ImportLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 取り込んだドキュメント。
///
/// `root` と各グループの `children` は **下から上** (描画順) に並ぶ。
/// 先頭が最も奥で、末尾が最も手前 (Photoshop のレイヤーパネルでは一番上)。
#[derive(Debug, Clone)]
pub struct LayerTree {
    pub width: u32,
    pub height: u32,
    pub root: Vec<LayerNode>,
    /// 取り込めなかった・無視した機能。ファイル内の順 (下から上) に並ぶ。
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone)]
pub enum LayerNode {
    Group {
        name: String,
        visible: bool,
        children: Vec<LayerNode>,
        /// PSD のレイヤー ID (`lyid`)。ドキュメント内で一意。
        id: Option<u32>,
        /// グループの不透明度 (0..=1)。
        opacity: f32,
        /// PSD のブレンドモード名 (正規化前)。通過は `"pass"`。
        blend_mode: String,
        /// 直下のレイヤー (兄弟) でクリッピングされている。
        clipping: bool,
    },
    Layer(Layer),
}

/// ラスタライズ済みレイヤー。画像はレイヤーの外接矩形で切り出した非乗算 RGBA8。
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    /// レイヤーの不透明度 (0..=1)。
    pub opacity: f32,
    /// キャンバス上の左上位置。キャンバスの外 (負の値など) にはみ出すこともある。
    pub offset: Vec2,
    /// 画像の大きさ。空のレイヤーでは 0 で、`rgba` も空になる。
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// PSD のブレンドモード名 (正規化前)。例: `"norm"` `"mul"` `"scrn"`。
    pub blend_mode: String,
    /// PSD のレイヤー ID (`lyid`)。ドキュメント内で一意。
    pub id: Option<u32>,
    /// 塗りの不透明度 (0..=1)。レイヤー効果を除いた画素にだけ掛かる。効果は取り込まないので、
    /// 実質的には `opacity` と掛け合わせて使う。
    pub fill_opacity: f32,
    /// 直下のレイヤー (兄弟) でクリッピングされている。連続するクリッピングレイヤーは、
    /// その下で最初に現れるクリッピングでないレイヤーを土台にする。
    pub clipping: bool,
}

/// 取り込みの警告。取り込みは続けるが、見た目が PSD と変わる可能性がある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportWarning {
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    pub path: Vec<String>,
    pub kind: WarningKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// 調整レイヤーまたは塗りつぶしレイヤー。画素を持たないので取り込まない。`key` は PSD のキー。
    AdjustmentLayer { key: String },
    /// 合成済みの画素を持たないスマートオブジェクト。取り込まない。
    SmartObjectWithoutPixels,
    /// レイヤー効果 (ドロップシャドウなど)。効果を除いた画素だけを取り込む。
    LayerEffectsIgnored,
    /// レイヤーマスクを掛けられなかった (グループのマスク、マスクの画素がないなど)。
    /// マスクを掛けていない画素を取り込む。
    LayerMaskIgnored,
    /// 対応していない圧縮方式のチャンネル。取り込まない。
    UnsupportedCompression { compression: u16 },
    /// 色チャンネルの画素がない。取り込まない。
    MissingPixels,
    /// レイヤー情報がなく、合成画像を 1 枚のレイヤーとして取り込んだ。
    FlattenedImage,
}

impl fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.path.join(" / "))?;
        match &self.kind {
            WarningKind::AdjustmentLayer { key } => {
                write!(f, "adjustment or fill layer ({key}) skipped")
            }
            WarningKind::SmartObjectWithoutPixels => {
                write!(f, "smart object without rendered pixels skipped")
            }
            WarningKind::LayerEffectsIgnored => write!(f, "layer effects ignored"),
            WarningKind::LayerMaskIgnored => write!(f, "layer mask ignored"),
            WarningKind::UnsupportedCompression { compression } => {
                write!(
                    f,
                    "unsupported channel compression {compression}, layer skipped"
                )
            }
            WarningKind::MissingPixels => write!(f, "no color channel data, layer skipped"),
            WarningKind::FlattenedImage => {
                write!(f, "no layer data, imported the composite image")
            }
        }
    }
}

/// [`ImportLimits::default`] の、チャンネル 1 枚を展開した後の大きさの上限 (1 GiB)。
///
/// PSD の最大のキャンバス (30000 × 30000) を覆う 8 ビットのレイヤーの 1 チャンネル (約 858 MiB) が収まる。
pub const DEFAULT_MAX_CHANNEL_DECOMPRESSED: u64 = 1024 * 1024 * 1024;

/// [`ImportLimits::default`] の、1 つの PSD で展開するバイトの合計の上限 (4 GiB)。
pub const DEFAULT_MAX_TOTAL_DECOMPRESSED: u64 = 4 * 1024 * 1024 * 1024;

/// 読む PSD のファイルの大きさの上限 (2 GiB)。
///
/// PSD の形式は長さを 32 ビットで持つので、ファイルは 2 GB を超えない (それより大きな PSB は読まない)。
/// このクレートはファイルを読まない (バイト列を受け取る) ので、ファイルを読む側 (エディタ・`ugoe-mcp`・
/// `ugoe import-psd`) がこの値で読む大きさを抑える。装置や巨大なファイルを読んでメモリを使い果たさないように。
/// 利用者が変える設定は出さない (T0554)。
pub const MAX_PSD_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 取り込みの資源の上限。
///
/// 上限は PSD の形式の決まりではなく、読む側の方針。ZIP で圧縮したチャンネルは 1000 倍ほどに縮むので、
/// 数十 MiB の PSD が数百 GiB の画素に展開されうる (展開の爆発)。メモリを使い切らないよう、
/// チャンネルの画素を展開する前に、展開後の大きさ (矩形の画素数 × 標本のバイト数。16 ビットは 2 バイト) を
/// 上限と比べ、超えるなら [`PsdError::LimitExceeded`] を返す (先に確保しない)。
///
/// 数えるのは、色・アルファ・マスクのチャンネルと合成画像の平面を展開したバイトで、圧縮方式によらない
/// (無圧縮・PackBits は入力の大きさで抑えられるが、合計をメモリの目安にするため含める)。
/// 取り込んだ RGBA (1 画素 4 バイト) は、8 ビットの 4 チャンネルのレイヤーなら数えた大きさとほぼ同じになる。
///
/// 将来ほかの上限を足せるように `#[non_exhaustive]` にしている。値を変えるときは
/// `ImportLimits::default().with_max_total_decompressed(n)` のように既定値から始める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportLimits {
    /// チャンネル 1 枚を展開した後の大きさの上限 (バイト)。既定は
    /// [`DEFAULT_MAX_CHANNEL_DECOMPRESSED`] (1 GiB)。
    pub max_channel_decompressed: u64,
    /// 1 つの PSD で、チャンネルを展開したバイトの合計の上限。既定は
    /// [`DEFAULT_MAX_TOTAL_DECOMPRESSED`] (4 GiB)。
    pub max_total_decompressed: u64,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_channel_decompressed: DEFAULT_MAX_CHANNEL_DECOMPRESSED,
            max_total_decompressed: DEFAULT_MAX_TOTAL_DECOMPRESSED,
        }
    }
}

impl ImportLimits {
    /// チャンネル 1 枚の展開後の上限を `bytes` にしたもの。
    pub const fn with_max_channel_decompressed(self, bytes: u64) -> Self {
        Self {
            max_channel_decompressed: bytes,
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

/// PSD を既定の上限 ([`ImportLimits::default`]) で取り込む。
///
/// 対応する色空間とビット深度。これ以外は対応しない機能のエラー ([`PsdError::is_unsupported`]) になる。
/// - 8 ビット・16 ビットの RGB (アルファの有無は問わない)。16 ビットは 8 ビットに丸める
///
/// レイヤーマスク (チャンネル -2 / -3) はレイヤーのアルファに掛ける。
/// グループの入れ子が 256 段より深いものは [`PsdError::GroupsTooDeep`] になる。
/// 展開した大きさが上限を超えるものは [`PsdError::LimitExceeded`] になる。
/// 壊れた入力や途中で切れた入力は壊れた入力のエラー ([`PsdError::is_invalid`]) を返し、パニックしない。
pub fn import(bytes: &[u8]) -> Result<LayerTree, PsdError> {
    import_with(bytes, ImportLimits::default())
}

/// PSD を、展開の上限 `limits` で取り込む。そのほかは [`import`] と同じ。
pub fn import_with(bytes: &[u8], limits: ImportLimits) -> Result<LayerTree, PsdError> {
    let doc = parse::parse(bytes, limits)?;
    if let Some(rgba) = doc.composite {
        let name = "Background".to_string();
        return Ok(LayerTree {
            width: doc.width,
            height: doc.height,
            root: vec![LayerNode::Layer(Layer {
                name: name.clone(),
                visible: true,
                opacity: 1.0,
                offset: Vec2::ZERO,
                width: doc.width,
                height: doc.height,
                rgba,
                blend_mode: "norm".into(),
                id: None,
                fill_opacity: 1.0,
                clipping: false,
            })],
            warnings: vec![ImportWarning {
                path: vec![name],
                kind: WarningKind::FlattenedImage,
            }],
        });
    }

    // 下から順に読み、グループの終わりの印で枠を積み、グループ本体で枠を閉じる。
    let mut stack = vec![Frame::default()];
    for raw in doc.layers {
        match raw.section {
            Some(Section::Divider) => {
                // 木を作る・複製する・捨てる処理は入れ子の深さだけ再帰するので、深さを抑える。
                if stack.len() > MAX_GROUP_DEPTH {
                    return Err(PsdError::GroupsTooDeep {
                        max: MAX_GROUP_DEPTH as u32,
                    });
                }
                stack.push(Frame::default());
            }
            Some(Section::Folder) => {
                // 対応する終わりの印がないフォルダは、空のグループとして扱う。
                let frame = if stack.len() > 1 {
                    stack.pop().unwrap_or_default()
                } else {
                    Frame::default()
                };
                let parent = stack.last_mut().ok_or_else(unbalanced)?;
                let name = raw.name.clone();
                parent
                    .warnings
                    .extend(frame.warnings.into_iter().map(|mut w| {
                        w.path.insert(0, name.clone());
                        w
                    }));
                // グループのマスクは合成したグループに掛かるもので、子のアルファに掛けても同じに
                // ならないため、警告だけにする。
                if raw.mask.is_some() || raw.real_mask.is_some() {
                    parent.warnings.push(ImportWarning {
                        path: vec![name.clone()],
                        kind: WarningKind::LayerMaskIgnored,
                    });
                }
                parent.nodes.push(group(raw, frame.nodes));
            }
            None => {
                let frame = stack.last_mut().ok_or_else(unbalanced)?;
                let name = raw.name.clone();
                let (layer, kinds) = layer(raw);
                frame
                    .warnings
                    .extend(kinds.into_iter().map(|kind| ImportWarning {
                        path: vec![name.clone()],
                        kind,
                    }));
                if let Some(layer) = layer {
                    frame.nodes.push(LayerNode::Layer(layer));
                }
            }
        }
    }
    if stack.len() != 1 {
        return Err(unbalanced());
    }
    let root = stack.pop().unwrap_or_default();
    Ok(LayerTree {
        width: doc.width,
        height: doc.height,
        root: root.nodes,
        warnings: root.warnings,
    })
}

/// グループの入れ子の深さの上限。これより深い PSD は [`PsdError::GroupsTooDeep`] にする。
const MAX_GROUP_DEPTH: usize = 256;

#[derive(Default)]
struct Frame {
    nodes: Vec<LayerNode>,
    warnings: Vec<ImportWarning>,
}

fn unbalanced() -> PsdError {
    PsdError::UnbalancedGroups
}

fn blend_name(key: [u8; 4]) -> String {
    String::from_utf8_lossy(&key).trim_end().to_string()
}

fn unit(v: u8) -> f32 {
    f32::from(v) / 255.0
}

fn group(raw: RawLayer, children: Vec<LayerNode>) -> LayerNode {
    LayerNode::Group {
        visible: !raw.hidden,
        id: raw.id,
        opacity: unit(raw.opacity),
        blend_mode: blend_name(raw.section_blend_key.unwrap_or(raw.blend_key)),
        clipping: raw.clipping,
        name: raw.name,
        children,
    }
}

/// 調整レイヤー・塗りつぶしレイヤーのキー。
const ADJUSTMENT_KEYS: &[&[u8; 4]] = &[
    b"SoCo", b"GdFl", b"PtFl", b"brit", b"levl", b"curv", b"expA", b"vibA", b"hue ", b"hue2",
    b"blnc", b"blwh", b"phfl", b"mixr", b"clrL", b"nvrt", b"post", b"thrs", b"grdm", b"selc",
];
/// スマートオブジェクトのキー。
const SMART_OBJECT_KEYS: &[&[u8; 4]] = &[b"SoLd", b"PlLd", b"SoLE"];
/// レイヤー効果のキー。
const EFFECT_KEYS: &[&[u8; 4]] = &[b"lrFX", b"lfx2", b"lfxs", b"lmfx"];

fn layer(mut raw: RawLayer) -> (Option<Layer>, Vec<WarningKind>) {
    let has = |keys: &[&[u8; 4]]| raw.keys.iter().find(|k| keys.contains(k)).copied();
    if let Some(key) = has(ADJUSTMENT_KEYS) {
        let key = blend_name(key);
        return (None, vec![WarningKind::AdjustmentLayer { key }]);
    }

    // 色やアルファのチャンネルが 1 つでも読めなければ、見た目が変わるので取り込まない。
    if let Some(compression) = raw.unsupported_compression {
        return (
            None,
            vec![WarningKind::UnsupportedCompression { compression }],
        );
    }
    let Some(rgba) = raw
        .rgba
        .take()
        .filter(|p| !(p.is_empty() && has(SMART_OBJECT_KEYS).is_some()))
    else {
        let kind = if has(SMART_OBJECT_KEYS).is_some() {
            WarningKind::SmartObjectWithoutPixels
        } else {
            WarningKind::MissingPixels
        };
        return (None, vec![kind]);
    };

    let mut warnings = Vec::new();
    if has(EFFECT_KEYS).is_some() {
        warnings.push(WarningKind::LayerEffectsIgnored);
    }
    if raw.mask_ignored {
        warnings.push(WarningKind::LayerMaskIgnored);
    }
    let layer = Layer {
        visible: !raw.hidden,
        opacity: unit(raw.opacity),
        offset: Vec2::new(raw.rect.left as f32, raw.rect.top as f32),
        width: raw.width,
        height: raw.height,
        rgba,
        blend_mode: blend_name(raw.blend_key),
        id: raw.id,
        fill_opacity: raw.fill_opacity.map_or(1.0, unit),
        clipping: raw.clipping,
        name: raw.name,
    };
    (Some(layer), warnings)
}
