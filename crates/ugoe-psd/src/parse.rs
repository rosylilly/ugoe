//! PSD の最小限のパーサ。このモジュールの型はクレートの外に出さない。
//!
//! 形式は Adobe の "Adobe Photoshop File Formats Specification" に従う。
//! 読むのは、ヘッダ・レイヤーとマスクの情報・(レイヤーがない場合の) 合成画像だけ。
//! 16 ビットのチャンネルは、読んだところで 8 ビットに丸める。

use crate::PsdError;

/// PSD の画像の幅・高さの上限 (Photoshop の PSD 形式の上限)。
const MAX_DIMENSION: u32 = 30_000;
/// 1 レイヤーのチャンネル数の上限 (Photoshop の上限)。
const MAX_CHANNELS: u16 = 56;

pub(crate) struct Document {
    pub width: u32,
    pub height: u32,
    /// ファイル内の順 (下から上)。
    pub layers: Vec<RawLayer>,
    /// レイヤー情報がない PSD のときだけ使う合成画像 (非乗算 RGBA8)。
    pub composite: Option<Vec<u8>>,
}

pub(crate) struct Rect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

impl Rect {
    /// 幅と高さ。負の大きさや上限を超える大きさは壊れた入力として扱う。
    fn size(&self) -> Result<(u32, u32), PsdError> {
        let w = i64::from(self.right) - i64::from(self.left);
        let h = i64::from(self.bottom) - i64::from(self.top);
        let limit = i64::from(MAX_DIMENSION) * 2;
        if !(0..=limit).contains(&w) || !(0..=limit).contains(&h) {
            return Err(invalid(format!(
                "layer bounds out of range: {}x{} at ({}, {})",
                w, h, self.left, self.top
            )));
        }
        // 範囲は上で確かめてある。
        Ok((w as u32, h as u32))
    }
}

pub(crate) struct Channel {
    pub id: i16,
    pub len: usize,
    /// 圧縮を解いて 8 ビットにした画素 (チャンネル画像を読んだ後に入る)。
    pub pixels: Option<Vec<u8>>,
}

/// レイヤーマスク 1 つ (チャンネル -2 または -3 の置き場所と、外側の値)。
pub(crate) struct Mask {
    /// キャンバス上の矩形。
    pub rect: Rect,
    pub width: u32,
    pub height: u32,
    /// 矩形の外のマスクの値 (0 か 255)。
    pub default_color: u8,
    /// 濃度 (0..=255)。255 未満なら、マスクの隠す量をこの割合に弱める。
    pub density: u8,
}

/// セクション区切り (`lsct` / `lsdk`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    /// 開いた (1) / 閉じた (2) フォルダ。グループの本体で、子より後 (上) に現れる。
    Folder,
    /// グループの終わりの印 (3)。子より前 (下) に現れる。
    Divider,
}

pub(crate) struct RawLayer {
    pub rect: Rect,
    pub width: u32,
    pub height: u32,
    pub channels: Vec<Channel>,
    pub blend_key: [u8; 4],
    pub opacity: u8,
    pub clipping: bool,
    pub hidden: bool,
    pub name: String,
    pub id: Option<u32>,
    pub fill_opacity: Option<u8>,
    pub section: Option<Section>,
    /// `lsct` に書かれたブレンドモード (グループではこちらが本当の値)。
    pub section_blend_key: Option<[u8; 4]>,
    /// 有効なユーザーマスク (チャンネル -2)。
    pub mask: Option<Mask>,
    /// ベクトルマスクと併用したときの、ベクトルマスクと画素のマスクを合成した最終的なマスク
    /// (チャンネル -3)。あれば -2 の代わりに使う。
    pub real_mask: Option<Mask>,
    /// 読んだ追加情報のキー (未対応機能の検出に使う)。
    pub keys: Vec<[u8; 4]>,
    /// チャンネル画像の圧縮方式のうち、対応していないもの。
    pub unsupported_compression: Option<u16>,
}

fn invalid(msg: impl Into<String>) -> PsdError {
    PsdError::Invalid(msg.into())
}

fn truncated(what: &str) -> PsdError {
    invalid(format!("unexpected end of data while reading {what}"))
}

/// 境界検査付きのビッグエンディアン読み取り。
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn bytes(&mut self, n: usize, what: &str) -> Result<&'a [u8], PsdError> {
        if n > self.remaining() {
            return Err(truncated(what));
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn array<const N: usize>(&mut self, what: &str) -> Result<[u8; N], PsdError> {
        let mut out = [0; N];
        out.copy_from_slice(self.bytes(N, what)?);
        Ok(out)
    }

    fn u8(&mut self, what: &str) -> Result<u8, PsdError> {
        Ok(self.array::<1>(what)?[0])
    }

    fn u16(&mut self, what: &str) -> Result<u16, PsdError> {
        Ok(u16::from_be_bytes(self.array(what)?))
    }

    fn i16(&mut self, what: &str) -> Result<i16, PsdError> {
        Ok(i16::from_be_bytes(self.array(what)?))
    }

    fn u32(&mut self, what: &str) -> Result<u32, PsdError> {
        Ok(u32::from_be_bytes(self.array(what)?))
    }

    fn i32(&mut self, what: &str) -> Result<i32, PsdError> {
        Ok(i32::from_be_bytes(self.array(what)?))
    }

    fn rect(&mut self, what: &str) -> Result<Rect, PsdError> {
        Ok(Rect {
            top: self.i32(what)?,
            left: self.i32(what)?,
            bottom: self.i32(what)?,
            right: self.i32(what)?,
        })
    }

    fn rest(&mut self) -> &'a [u8] {
        let out = &self.data[self.pos..];
        self.pos = self.data.len();
        out
    }

    /// 32 ビットの長さが前に付いたブロックを切り出す。
    fn block(&mut self, what: &str) -> Result<Reader<'a>, PsdError> {
        let len = self.u32(what)? as usize;
        Ok(Reader::new(self.bytes(len, what)?))
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Document, PsdError> {
    let mut r = Reader::new(bytes);
    if r.bytes(4, "signature")? != b"8BPS" {
        return Err(invalid("not a psd file (bad signature)"));
    }
    match r.u16("version")? {
        1 => {}
        2 => {
            return Err(PsdError::Unsupported(
                "large document format (psb) is not supported".into(),
            ));
        }
        v => return Err(invalid(format!("unknown version {v}"))),
    }
    r.bytes(6, "header")?;
    let channels = r.u16("channel count")?;
    let height = r.u32("height")?;
    let width = r.u32("width")?;
    let depth = r.u16("depth")?;
    let mode = r.u16("color mode")?;
    if !(1..=MAX_CHANNELS).contains(&channels) {
        return Err(invalid(format!("invalid channel count {channels}")));
    }
    if !(1..=MAX_DIMENSION).contains(&width) || !(1..=MAX_DIMENSION).contains(&height) {
        return Err(invalid(format!("invalid image size {width}x{height}")));
    }
    if depth != 8 && depth != 16 {
        return Err(PsdError::Unsupported(format!(
            "{depth}-bit channels (only 8-bit and 16-bit are supported)"
        )));
    }
    if mode != 3 {
        return Err(PsdError::Unsupported(format!(
            "{} color mode (only rgb is supported)",
            color_mode_name(mode)
        )));
    }

    r.block("color mode data")?;
    r.block("image resources")?;
    let mut layer_and_mask = r.block("layer and mask information")?;
    let mut layers = if layer_and_mask.remaining() == 0 {
        Vec::new()
    } else {
        let mut info = layer_and_mask.block("layer info")?;
        parse_layer_info(&mut info, depth)?
    };
    if layers.is_empty() && layer_and_mask.remaining() > 0 {
        // 16 ビットの PSD では、レイヤー情報は空で、後ろの追加情報 `Lr16` に入る。
        layer_and_mask.block("global layer mask info")?;
        if let Some(mut info) = find_layer_info_block(&mut layer_and_mask, depth)? {
            layers = parse_layer_info(&mut info, depth)?;
        }
    }

    // レイヤーを持たない PSD は、合成画像を 1 枚のレイヤーとして扱う。
    let composite = if layers.is_empty() {
        Some(parse_composite(&mut r, width, height, channels, depth)?)
    } else {
        None
    };
    Ok(Document {
        width,
        height,
        layers,
        composite,
    })
}

fn color_mode_name(mode: u16) -> String {
    match mode {
        0 => "bitmap".into(),
        1 => "grayscale".into(),
        2 => "indexed".into(),
        4 => "cmyk".into(),
        7 => "multichannel".into(),
        8 => "duotone".into(),
        9 => "lab".into(),
        m => format!("unknown ({m})"),
    }
}

/// レイヤーとマスクの情報の末尾の追加情報から、ビット深度に合うレイヤー情報
/// (8 ビットは `Layr`、16 ビットは `Lr16`) を探す。
fn find_layer_info_block<'a>(
    r: &mut Reader<'a>,
    depth: u16,
) -> Result<Option<Reader<'a>>, PsdError> {
    let wanted: &[u8; 4] = if depth == 16 { b"Lr16" } else { b"Layr" };
    while r.remaining() >= 12 {
        let sig = r.array::<4>("additional info signature")?;
        if &sig != b"8BIM" && &sig != b"8B64" {
            // 長さに含めない詰め物 (書き手によって 4 バイトに揃える) を 1 バイトずつ飛ばす。
            r.pos -= 3;
            continue;
        }
        let key = r.array::<4>("additional info key")?;
        let data = r.block("additional info")?;
        if &key == wanted {
            return Ok(Some(data));
        }
    }
    Ok(None)
}

fn parse_layer_info(r: &mut Reader<'_>, depth: u16) -> Result<Vec<RawLayer>, PsdError> {
    if r.remaining() == 0 {
        return Ok(Vec::new());
    }
    // 負の値は「最初のアルファチャンネルが合成結果の透明度」を表すだけで、数は絶対値。
    let count = r.i16("layer count")?.unsigned_abs();
    let mut layers = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        layers.push(parse_layer_record(r)?);
    }
    for layer in &mut layers {
        read_channel_images(r, layer, depth)?;
    }
    Ok(layers)
}

fn parse_layer_record(r: &mut Reader<'_>) -> Result<RawLayer, PsdError> {
    let rect = r.rect("layer bounds")?;
    let (width, height) = rect.size()?;
    let channel_count = r.u16("layer channel count")?;
    if channel_count > MAX_CHANNELS {
        return Err(invalid(format!("too many channels ({channel_count})")));
    }
    let mut channels = Vec::with_capacity(usize::from(channel_count));
    for _ in 0..channel_count {
        let id = r.i16("channel id")?;
        let len = r.u32("channel length")? as usize;
        channels.push(Channel {
            id,
            len,
            pixels: None,
        });
    }
    if r.bytes(4, "blend mode signature")? != b"8BIM" {
        return Err(invalid("bad blend mode signature in layer record"));
    }
    let blend_key = r.array("blend mode")?;
    let opacity = r.u8("opacity")?;
    let clipping = r.u8("clipping")? != 0;
    let flags = r.u8("layer flags")?;
    r.u8("filler")?;
    let mut extra = r.block("layer extra data")?;

    let mut mask_data = extra.block("layer mask data")?;
    let has_real_mask = channels.iter().any(|c| c.id == -3);
    let (mask, real_mask) = parse_masks(&mut mask_data, has_real_mask)?;
    extra.block("blending ranges")?;
    let name_len = usize::from(extra.u8("layer name")?);
    let name_bytes = extra.bytes(name_len, "layer name")?;
    // 長さのバイトを含めて 4 の倍数に揃えてある。
    let pad = (4 - (name_len + 1) % 4) % 4;
    extra.bytes(pad.min(extra.remaining()), "layer name padding")?;

    let mut layer = RawLayer {
        rect,
        width,
        height,
        channels,
        blend_key,
        opacity,
        clipping,
        hidden: flags & 0x02 != 0,
        name: decode_legacy_name(name_bytes),
        id: None,
        fill_opacity: None,
        section: None,
        section_blend_key: None,
        mask,
        real_mask,
        keys: Vec::new(),
        unsupported_compression: None,
    };
    parse_additional_info(&mut extra, &mut layer)?;
    Ok(layer)
}

/// レイヤーマスクの情報 (仕様の "Layer mask / adjustment layer data")。
/// 返すのは (チャンネル -2 のマスク, チャンネル -3 のマスク) で、無効にしてあるものは `None`。
///
/// 並びは、矩形・既定の色・フラグの後に、(チャンネル -3 があれば) -3 の フラグ・既定の色・矩形、
/// (フラグの bit 4 があれば) 濃度・ぼかしの値が続く。仕様書は濃度・ぼかしを先に書くが、
/// 実際のファイルはこの順 (psd-tools の読み方と、その issue #693 に従った)。
/// <https://github.com/psd-tools/psd-tools/blob/main/src/psd_tools/psd/layer_and_mask.py>
fn parse_masks(
    r: &mut Reader<'_>,
    has_real_mask: bool,
) -> Result<(Option<Mask>, Option<Mask>), PsdError> {
    let size = r.remaining();
    if size == 0 {
        return Ok((None, None));
    }
    let rect = r.rect("mask bounds")?;
    let default_color = r.u8("mask default color")?;
    let flags = r.u8("mask flags")?;
    let real = if has_real_mask && size >= 36 {
        let real_flags = r.u8("real mask flags")?;
        let real_color = r.u8("real mask default color")?;
        let real_rect = r.rect("real mask bounds")?;
        mask(real_rect, real_color, real_flags, 255)?
    } else {
        None
    };
    let mut density = 255;
    if flags & MASK_HAS_PARAMETERS != 0 && r.remaining() > 0 {
        // bit 0: ユーザーマスクの濃度 (1 バイト)、bit 1: ユーザーマスクのぼかし (8 バイトの double)。
        // bit 2・3 はベクトルマスクの濃度・ぼかし。ぼかしは読まない (掛けない)。
        if r.u8("mask parameters")? & 0x01 != 0 {
            density = r.u8("mask density")?;
        }
    }
    Ok((mask(rect, default_color, flags, density)?, real))
}

/// マスクのフラグの bit 1: マスクを無効にしている。
const MASK_DISABLED: u8 = 0x02;
/// マスクのフラグの bit 4: 濃度・ぼかしの値が続く。
const MASK_HAS_PARAMETERS: u8 = 0x10;

fn mask(rect: Rect, default_color: u8, flags: u8, density: u8) -> Result<Option<Mask>, PsdError> {
    let (width, height) = rect.size()?;
    // 矩形が空のマスクは、何もしない書き手の出力とみなして無視する。
    if flags & MASK_DISABLED != 0 || width == 0 || height == 0 {
        return Ok(None);
    }
    Ok(Some(Mask {
        rect,
        width,
        height,
        default_color,
        density,
    }))
}

/// `luni` を持たないレイヤーの名前 (パスカル文字列) を読む。
///
/// 仕様では OS の文字コードで、日本語の環境のツール (SAI など) は Shift-JIS (CP932) で書く。
/// UTF-8 として正しければ UTF-8 (ASCII もここに入る)、そうでなく Shift-JIS として正しければ
/// Shift-JIS、どちらでもなければ Windows-1252 (西欧の環境の既定) として読む。
fn decode_legacy_name(bytes: &[u8]) -> String {
    if let Ok(name) = std::str::from_utf8(bytes) {
        return name.to_owned();
    }
    if let Some(name) =
        encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(bytes)
    {
        return name.into_owned();
    }
    encoding_rs::WINDOWS_1252
        .decode_without_bom_handling(bytes)
        .0
        .into_owned()
}

fn parse_additional_info(r: &mut Reader<'_>, layer: &mut RawLayer) -> Result<(), PsdError> {
    while r.remaining() >= 12 {
        let sig = r.array::<4>("additional info signature")?;
        if &sig != b"8BIM" && &sig != b"8B64" {
            // 形式の違う末尾の詰め物などは無視する。
            break;
        }
        let key = r.array::<4>("additional info key")?;
        let mut data = r.block("additional info")?;
        match &key {
            b"luni" => {
                let n = data.u32("unicode name")? as usize;
                let raw = data.bytes(n.saturating_mul(2), "unicode name")?;
                let units = raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&c| u16::from_be_bytes(c))
                    .take_while(|&u| u != 0);
                layer.name = char::decode_utf16(units)
                    .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
                    .collect();
            }
            b"lyid" => layer.id = Some(data.u32("layer id")?),
            b"iOpa" => layer.fill_opacity = Some(data.u8("fill opacity")?),
            b"lsct" | b"lsdk" => {
                layer.section = match data.u32("section divider")? {
                    1 | 2 => Some(Section::Folder),
                    3 => Some(Section::Divider),
                    _ => None,
                };
                if data.remaining() >= 8 && data.bytes(4, "section blend signature")? == b"8BIM" {
                    layer.section_blend_key = Some(data.array("section blend mode")?);
                }
            }
            _ => {}
        }
        layer.keys.push(key);
    }
    Ok(())
}

fn read_channel_images(
    r: &mut Reader<'_>,
    layer: &mut RawLayer,
    depth: u16,
) -> Result<(), PsdError> {
    for channel in &mut layer.channels {
        let mut data = Reader::new(r.bytes(channel.len, "channel image data")?);
        if channel.len == 0 {
            continue;
        }
        let compression = data.u16("channel compression")?;
        // マスクのチャンネルは、マスクの矩形の大きさを持つ。
        let size = match channel.id {
            -2 => layer.mask.as_ref().map(|m| (m.width, m.height)),
            -3 => layer.real_mask.as_ref().map(|m| (m.width, m.height)),
            id if id < -3 => None,
            _ => Some((layer.width, layer.height)),
        };
        let Some((width, height)) = size else {
            continue;
        };
        if width == 0 || height == 0 {
            continue;
        }
        match decode_plane(&mut data, compression, width, height, depth)? {
            Some(pixels) => channel.pixels = Some(pixels),
            // マスクのチャンネルが読めないときは、マスクを掛けずに取り込む (警告は取り込み側)。
            None if channel.id < -1 => {}
            None => layer.unsupported_compression = Some(compression),
        }
    }
    Ok(())
}

/// 1 チャンネルの画素を読み、8 ビットにして返す。対応しない圧縮方式なら `None`。
fn decode_plane(
    r: &mut Reader<'_>,
    compression: u16,
    width: u32,
    height: u32,
    depth: u16,
) -> Result<Option<Vec<u8>>, PsdError> {
    let bytes_per_sample = usize::from(depth / 8);
    let row = width as usize * bytes_per_sample;
    let len = row.saturating_mul(height as usize);
    let mut raw = match compression {
        0 => r.bytes(len, "raw channel data")?.to_vec(),
        1 => decode_rle(r, row, height, 1)?,
        2 | 3 => inflate(r.rest(), len)?,
        _ => return Ok(None),
    };
    if compression == 3 {
        undo_prediction(&mut raw, row, depth);
    }
    Ok(Some(to_8bit(raw, depth)))
}

/// zlib ストリームを、ちょうど `len` バイトに展開する。
fn inflate(data: &[u8], len: usize) -> Result<Vec<u8>, PsdError> {
    let out = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, len)
        .map_err(|e| invalid(format!("bad zip channel data ({:?})", e.status)))?;
    if out.len() != len {
        return Err(invalid(format!(
            "zip channel data decodes to {} bytes, expected {len}",
            out.len()
        )));
    }
    Ok(out)
}

/// 予測付き ZIP の差分を戻す。各行で、左の標本との差 (8 ビットはバイト、16 ビットはビッグエンディアンの
/// 16 ビット整数) が書いてある。
fn undo_prediction(data: &mut [u8], row: usize, depth: u16) {
    if row == 0 {
        return;
    }
    for line in data.chunks_exact_mut(row) {
        if depth == 16 {
            let mut prev = 0u16;
            for sample in line.as_chunks_mut::<2>().0 {
                prev = prev.wrapping_add(u16::from_be_bytes(*sample));
                *sample = prev.to_be_bytes();
            }
        } else {
            let mut prev = 0u8;
            for v in line {
                prev = prev.wrapping_add(*v);
                *v = prev;
            }
        }
    }
}

/// ビッグエンディアンの標本を 8 ビットに丸める (16 ビットは 0..=65535 を 0..=255 に写す)。
fn to_8bit(data: Vec<u8>, depth: u16) -> Vec<u8> {
    if depth != 16 {
        return data;
    }
    data.as_chunks::<2>()
        .0
        .iter()
        .map(|&s| ((u32::from(u16::from_be_bytes(s)) * 255 + 32_767) / 65_535) as u8)
        .collect()
}

/// PackBits で圧縮された `planes` 枚の平面を読む。先頭に全行のバイト数の表がある。
/// `width` は 1 行のバイト数 (16 ビットでは画素数の 2 倍)。
fn decode_rle(
    r: &mut Reader<'_>,
    width: usize,
    height: u32,
    planes: u16,
) -> Result<Vec<u8>, PsdError> {
    let rows = height as usize * usize::from(planes);
    let mut counts = Vec::with_capacity(rows);
    for _ in 0..rows {
        counts.push(usize::from(r.u16("rle byte counts")?));
    }
    let total: usize = counts.iter().sum();
    if total > r.remaining() {
        return Err(truncated("rle data"));
    }
    // 2 バイトで最大 128 バイトに伸びる。これを超える大きさは嘘なので、確保する前に弾く。
    if rows.saturating_mul(width) > total.saturating_mul(64) {
        return Err(invalid("rle data is too short for the image size"));
    }
    let mut out = Vec::with_capacity(rows * width);
    for count in counts {
        let row = r.bytes(count, "rle row")?;
        unpack_bits(row, width, &mut out)?;
    }
    Ok(out)
}

fn unpack_bits(mut src: &[u8], width: usize, out: &mut Vec<u8>) -> Result<(), PsdError> {
    let start = out.len();
    let overrun = || invalid("rle row decodes to the wrong length");
    while let Some((&header, rest)) = src.split_first() {
        src = rest;
        let written = out.len() - start;
        match header as i8 {
            // -128 は何もしない。
            -128 => {}
            n if n >= 0 => {
                let n = n as usize + 1;
                if n > src.len() || written + n > width {
                    return Err(overrun());
                }
                out.extend_from_slice(&src[..n]);
                src = &src[n..];
            }
            n => {
                let n = (1 - i16::from(n)) as usize;
                let (&value, rest) = src.split_first().ok_or_else(overrun)?;
                src = rest;
                if written + n > width {
                    return Err(overrun());
                }
                out.resize(out.len() + n, value);
            }
        }
    }
    if out.len() - start != width {
        return Err(overrun());
    }
    Ok(())
}

/// 合成画像を非乗算 RGBA8 で読む。
fn parse_composite(
    r: &mut Reader<'_>,
    width: u32,
    height: u32,
    channels: u16,
    depth: u16,
) -> Result<Vec<u8>, PsdError> {
    if channels < 3 {
        return Err(invalid(format!(
            "rgb image needs 3 channels, found {channels}"
        )));
    }
    let plane = width as usize * height as usize;
    let row = width as usize * usize::from(depth / 8);
    let planes = match r.u16("image data compression")? {
        0 => r
            .bytes(
                row.saturating_mul(height as usize)
                    .saturating_mul(usize::from(channels)),
                "image data",
            )?
            .to_vec(),
        1 => decode_rle(r, row, height, channels)?,
        c => {
            return Err(PsdError::Unsupported(format!("image data compression {c}")));
        }
    };
    let planes = to_8bit(planes, depth);
    let mut rgba = vec![255; plane * 4];
    for (c, src) in planes.chunks_exact(plane).take(4).enumerate() {
        for (px, &v) in rgba.as_chunks_mut::<4>().0.iter_mut().zip(src) {
            px[c] = v;
        }
    }
    // 4 番目のチャンネルは (あれば) 合成結果の透明度。
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_bits_handles_literal_and_repeat_runs() {
        let mut out = Vec::new();
        // リテラル 3 バイト、繰り返し 4 回、何もしない -128。
        unpack_bits(&[2, 1, 2, 3, 0xfd, 9, 0x80], 7, &mut out).unwrap();
        assert_eq!(out, [1, 2, 3, 9, 9, 9, 9]);
    }

    #[test]
    fn sixteen_bit_samples_round_to_eight_bit() {
        let samples: Vec<u8> = [0u16, 128, 129, 257, 32_767, 65_406, 65_535]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect();
        assert_eq!(to_8bit(samples, 16), [0, 0, 1, 1, 127, 254, 255]);
    }

    #[test]
    fn prediction_is_undone_per_row() {
        let mut bytes = vec![10, 1, 255, 5, 0, 2];
        undo_prediction(&mut bytes, 3, 8);
        assert_eq!(bytes, [10, 11, 10, 5, 5, 7]);
        // 16 ビット: 0x0100, +0xFFFF (= -1) → 0x00FF。
        let mut wide = vec![1, 0, 0xff, 0xff];
        undo_prediction(&mut wide, 4, 16);
        assert_eq!(wide, [1, 0, 0, 0xff]);
    }

    #[test]
    fn unpack_bits_rejects_wrong_lengths() {
        let mut out = Vec::new();
        assert!(unpack_bits(&[2, 1, 2], 3, &mut out).is_err());
        assert!(unpack_bits(&[0xfd, 9], 3, &mut out).is_err());
        assert!(unpack_bits(&[0, 1], 3, &mut out).is_err());
        assert!(unpack_bits(&[0xff], 2, &mut out).is_err());
    }
}
