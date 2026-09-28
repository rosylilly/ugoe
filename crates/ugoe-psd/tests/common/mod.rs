//! テスト用の最小限の PSD ライター。`assets/fixtures/layers.psd` もこれで作る。
//!
//! 取り込み側 (`ugoe_psd::import`) と独立に、形式の仕様どおりにバイト列を並べる。

#![allow(dead_code)]

/// 書き出すレイヤー 1 枚 (グループの始まり・終わりの印も 1 枚のレイヤーとして書く)。
#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    /// 非乗算 RGBA8。`None` なら画素を持たない (チャンネルの中身が空)。
    pub rgba: Option<Vec<u8>>,
    pub opacity: u8,
    pub clipping: bool,
    pub hidden: bool,
    pub blend: [u8; 4],
    /// 0 = 無圧縮、1 = PackBits。それ以外は中身を書かずに圧縮方式の番号だけを書く。
    pub compression: u16,
    pub id: Option<u32>,
    pub fill_opacity: Option<u8>,
    /// `lsct` の種類と、(あれば) ブレンドモード。
    pub section: Option<(u32, Option<[u8; 4]>)>,
    /// ユーザーマスク (矩形だけ。画素は書かない)。
    pub mask: bool,
    /// そのほかの追加情報 (キーと中身)。
    pub extra: Vec<([u8; 4], Vec<u8>)>,
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            rgba: None,
            opacity: 255,
            clipping: false,
            hidden: false,
            blend: *b"norm",
            compression: 1,
            id: None,
            fill_opacity: None,
            section: None,
            mask: false,
            extra: Vec::new(),
        }
    }

    pub fn raster(name: &str, left: i32, top: i32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
            rgba: Some(pattern(width, height, name.len() as u8)),
            ..Self::new(name)
        }
    }

    /// グループの終わりの印 (ファイル内では子より前 = 下に置く)。
    pub fn group_end() -> Self {
        Self {
            section: Some((3, None)),
            ..Self::new("</Layer group>")
        }
    }

    /// グループの本体 (ファイル内では子より後 = 上に置く)。
    pub fn group(name: &str) -> Self {
        Self {
            section: Some((1, Some(*b"pass"))),
            ..Self::new(name)
        }
    }
}

/// 位置と種から決まる、アルファの変化する模様。
pub fn pattern(width: u32, height: u32, seed: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let (x, y) = (x as u8, y as u8);
            out.extend_from_slice(&[
                x.wrapping_mul(7).wrapping_add(seed),
                y.wrapping_mul(11).wrapping_add(seed),
                // 行ごとに同じ値が続くので PackBits の繰り返しにもなる。
                seed.wrapping_mul(31),
                if x == 0 { 0 } else { 255 - y.wrapping_mul(3) },
            ]);
        }
    }
    out
}

struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    /// 32 ビットの長さを前に付けたブロック。長さは `pad` の倍数に揃える。
    fn block(&mut self, pad: usize, f: impl FnOnce(&mut Out)) {
        let mut inner = Out(Vec::new());
        f(&mut inner);
        while !inner.0.len().is_multiple_of(pad) {
            inner.0.push(0);
        }
        self.u32(inner.0.len() as u32);
        self.bytes(&inner.0);
    }
}

/// 1 行を PackBits で圧縮する。
pub fn pack_bits(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < row.len() {
        let mut run = 1;
        while i + run < row.len() && run < 128 && row[i + run] == row[i] {
            run += 1;
        }
        if run >= 3 {
            out.push((1 - run as i16) as u8);
            out.push(row[i]);
            i += run;
            continue;
        }
        let start = i;
        while i < row.len()
            && i - start < 128
            && !(i + 2 < row.len() && row[i] == row[i + 1] && row[i] == row[i + 2])
        {
            i += 1;
        }
        out.push((i - start - 1) as u8);
        out.extend_from_slice(&row[start..i]);
    }
    out
}

/// 1 チャンネル分のデータ (圧縮方式の 2 バイトを含む)。
fn channel_data(plane: &[u8], width: u32, height: u32, compression: u16) -> Vec<u8> {
    let mut out = Out(Vec::new());
    out.u16(compression);
    match compression {
        0 => out.bytes(plane),
        1 => {
            let rows: Vec<Vec<u8>> = plane
                .chunks(width.max(1) as usize)
                .take(height as usize)
                .map(pack_bits)
                .collect();
            for row in &rows {
                out.u16(row.len() as u16);
            }
            for row in &rows {
                out.bytes(row);
            }
        }
        // 対応していない方式は、それらしい中身だけを置く。
        _ => out.bytes(&[0x78, 0x9c, 0, 0]),
    }
    out.0
}

/// 8 ビット RGB の PSD を書く。`layers` は下から上の順。`composite` は RGB8 の合成画像。
pub fn write_psd(width: u32, height: u32, layers: &[Layer], composite: &[u8]) -> Vec<u8> {
    let mut out = Out(Vec::new());
    out.bytes(b"8BPS");
    out.u16(1);
    out.bytes(&[0; 6]);
    out.u16(3);
    out.u32(height);
    out.u32(width);
    out.u16(8);
    out.u16(3);
    out.u32(0); // カラーモードデータ
    out.u32(0); // 画像リソース
    out.block(2, |lm| {
        if layers.is_empty() {
            return;
        }
        lm.block(2, |info| {
            info.i16(layers.len() as i16);
            let channels: Vec<Vec<(i16, Vec<u8>)>> = layers.iter().map(channels).collect();
            for (layer, channels) in layers.iter().zip(&channels) {
                layer_record(info, layer, channels);
            }
            for channels in &channels {
                for (_, data) in channels {
                    info.bytes(data);
                }
            }
        });
        lm.u32(0); // グローバルなレイヤーマスク情報
    });
    // 合成画像 (無圧縮)。
    out.u16(0);
    let plane = width as usize * height as usize;
    for c in 0..3 {
        for i in 0..plane {
            out.u8(composite[i * 3 + c]);
        }
    }
    out.0
}

fn channels(layer: &Layer) -> Vec<(i16, Vec<u8>)> {
    [-1i16, 0, 1, 2]
        .iter()
        .map(|&id| {
            let data = match &layer.rgba {
                Some(rgba) => {
                    let c = if id < 0 { 3 } else { id as usize };
                    let plane: Vec<u8> = rgba.as_chunks::<4>().0.iter().map(|p| p[c]).collect();
                    channel_data(&plane, layer.width, layer.height, layer.compression)
                }
                None => vec![0, 0],
            };
            (id, data)
        })
        .collect()
}

fn layer_record(out: &mut Out, layer: &Layer, channels: &[(i16, Vec<u8>)]) {
    out.i32(layer.top);
    out.i32(layer.left);
    out.i32(layer.top + layer.height as i32);
    out.i32(layer.left + layer.width as i32);
    out.u16(channels.len() as u16);
    for (id, data) in channels {
        out.i16(*id);
        out.u32(data.len() as u32);
    }
    out.bytes(b"8BIM");
    out.bytes(&layer.blend);
    out.u8(layer.opacity);
    out.u8(u8::from(layer.clipping));
    // bit 1: 非表示、bit 3: bit 4 が意味を持つ。
    out.u8(if layer.hidden { 0x0a } else { 0x08 });
    out.u8(0);
    out.block(1, |extra| {
        if layer.mask {
            extra.block(1, |m| {
                for v in [layer.top, layer.left, layer.top + 4, layer.left + 4] {
                    m.i32(v);
                }
                m.u8(0);
                m.u8(0);
                m.u16(0);
            });
        } else {
            extra.u32(0);
        }
        extra.u32(0); // ブレンド範囲
        // パスカル文字列の名前 (ASCII 以外は `?`)。長さのバイトを含めて 4 の倍数に揃える。
        let ascii: Vec<u8> = layer
            .name
            .chars()
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect();
        extra.u8(ascii.len() as u8);
        extra.bytes(&ascii);
        extra.bytes(&vec![0; (4 - (ascii.len() + 1) % 4) % 4]);
        let mut tagged = |key: &[u8; 4], f: &dyn Fn(&mut Out)| {
            extra.bytes(b"8BIM");
            extra.bytes(key);
            extra.block(4, f);
        };
        if let Some((kind, blend)) = layer.section {
            tagged(b"lsct", &|d| {
                d.u32(kind);
                if let Some(blend) = blend {
                    d.bytes(b"8BIM");
                    d.bytes(&blend);
                }
            });
        }
        let utf16: Vec<u16> = layer.name.encode_utf16().collect();
        tagged(b"luni", &|d| {
            d.u32(utf16.len() as u32);
            for u in &utf16 {
                d.u16(*u);
            }
        });
        if let Some(id) = layer.id {
            tagged(b"lyid", &|d| d.u32(id));
        }
        if let Some(fill) = layer.fill_opacity {
            tagged(b"iOpa", &|d| d.u8(fill));
        }
        for (key, data) in &layer.extra {
            tagged(key, &|d| d.bytes(data));
        }
    });
}
