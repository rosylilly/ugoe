//! PSD の最小限のパーサ。このモジュールの型はクレートの外に出さない。
//!
//! 形式は Adobe の "Adobe Photoshop File Formats Specification" に従う。
//! 読むのは、ヘッダ・レイヤーとマスクの情報・(レイヤーがない場合の) 合成画像だけ。

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
    /// 圧縮を解いた画素 (チャンネル画像を読んだ後に入る)。
    pub pixels: Option<Vec<u8>>,
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
    /// 有効なユーザーマスクを持つ。
    pub has_mask: bool,
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
    if depth != 8 {
        return Err(PsdError::Unsupported(format!(
            "{depth}-bit channels (only 8-bit is supported)"
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
    let layers = if layer_and_mask.remaining() == 0 {
        Vec::new()
    } else {
        let mut info = layer_and_mask.block("layer info")?;
        parse_layer_info(&mut info)?
    };

    // レイヤーを持たない PSD は、合成画像を 1 枚のレイヤーとして扱う。
    let composite = if layers.is_empty() {
        Some(parse_composite(&mut r, width, height, channels)?)
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

fn parse_layer_info(r: &mut Reader<'_>) -> Result<Vec<RawLayer>, PsdError> {
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
        read_channel_images(r, layer)?;
    }
    Ok(layers)
}

fn parse_layer_record(r: &mut Reader<'_>) -> Result<RawLayer, PsdError> {
    let rect = Rect {
        top: r.i32("layer bounds")?,
        left: r.i32("layer bounds")?,
        bottom: r.i32("layer bounds")?,
        right: r.i32("layer bounds")?,
    };
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

    let mut mask = extra.block("layer mask data")?;
    let has_mask = parse_mask_enabled(&mut mask)?;
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
        name: String::from_utf8_lossy(name_bytes).into_owned(),
        id: None,
        fill_opacity: None,
        section: None,
        section_blend_key: None,
        has_mask,
        keys: Vec::new(),
        unsupported_compression: None,
    };
    parse_additional_info(&mut extra, &mut layer)?;
    Ok(layer)
}

/// ユーザーマスクがあり、無効化されていなければ true。
fn parse_mask_enabled(r: &mut Reader<'_>) -> Result<bool, PsdError> {
    if r.remaining() == 0 {
        return Ok(false);
    }
    let rect = Rect {
        top: r.i32("mask bounds")?,
        left: r.i32("mask bounds")?,
        bottom: r.i32("mask bounds")?,
        right: r.i32("mask bounds")?,
    };
    r.u8("mask default color")?;
    let flags = r.u8("mask flags")?;
    let (w, h) = rect.size()?;
    // bit 1: マスクを無効にしている。
    Ok(flags & 0x02 == 0 && w > 0 && h > 0)
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

fn read_channel_images(r: &mut Reader<'_>, layer: &mut RawLayer) -> Result<(), PsdError> {
    let (width, height) = (layer.width, layer.height);
    for channel in &mut layer.channels {
        let mut data = Reader::new(r.bytes(channel.len, "channel image data")?);
        if channel.len == 0 {
            continue;
        }
        let compression = data.u16("channel compression")?;
        // マスクのチャンネル (-2, -3) はマスクの矩形の大きさを持つので読まない。
        if channel.id < -1 {
            continue;
        }
        let pixels = match compression {
            0 => {
                let n = width as usize * height as usize;
                data.bytes(n, "raw channel data")?.to_vec()
            }
            1 => decode_rle(&mut data, width, height, 1)?,
            c => {
                layer.unsupported_compression = Some(c);
                continue;
            }
        };
        channel.pixels = Some(pixels);
    }
    Ok(())
}

/// PackBits で圧縮された `planes` 枚の平面を読む。先頭に全行のバイト数の表がある。
fn decode_rle(
    r: &mut Reader<'_>,
    width: u32,
    height: u32,
    planes: u16,
) -> Result<Vec<u8>, PsdError> {
    let rows = height as usize * usize::from(planes);
    let width = width as usize;
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
) -> Result<Vec<u8>, PsdError> {
    if channels < 3 {
        return Err(invalid(format!(
            "rgb image needs 3 channels, found {channels}"
        )));
    }
    let plane = width as usize * height as usize;
    let planes = match r.u16("image data compression")? {
        0 => r
            .bytes(plane.saturating_mul(usize::from(channels)), "image data")?
            .to_vec(),
        1 => decode_rle(r, width, height, channels)?,
        c => {
            return Err(PsdError::Unsupported(format!("image data compression {c}")));
        }
    };
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
    fn unpack_bits_rejects_wrong_lengths() {
        let mut out = Vec::new();
        assert!(unpack_bits(&[2, 1, 2], 3, &mut out).is_err());
        assert!(unpack_bits(&[0xfd, 9], 3, &mut out).is_err());
        assert!(unpack_bits(&[0, 1], 3, &mut out).is_err());
        assert!(unpack_bits(&[0xff], 2, &mut out).is_err());
    }
}
