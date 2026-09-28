//! テスト用の PSD ライター。`assets/fixtures/*.psd` もこれで作る。
//!
//! 取り込み側 (`ugoe_psd::import`) と独立に、Adobe の "Adobe Photoshop File Formats Specification"
//! (<https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/>) どおりにバイト列を並べる。
//! 形式の規則に反する中身 (偽の圧縮データ・長さの合わない追加情報など) は書かない。
//! 取り込みのエラー処理を試す壊れた入力は、各テストがこの出力を書き換えて作る。
//!
//! ただし [`Layer::extra`] の中身はそのまま書くので、仕様どおりかは呼び出し側の責任になる
//! (仕様どおりの中身は [`levels_identity`] など。検出だけを試すテストは中身を省くことがある)。

#![allow(dead_code)]

pub mod compose;
pub mod fixtures;
pub mod shapes;

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
    /// チャンネルの圧縮方式。0 = 無圧縮、1 = PackBits、2 = ZIP、3 = 予測付き ZIP。
    /// ZIP は `miniz_oxide` で圧縮した zlib ストリームで書く。
    pub compression: u16,
    pub id: Option<u32>,
    pub fill_opacity: Option<u8>,
    /// `lsct` の種類と、(あれば) ブレンドモード。
    pub section: Option<(u32, Option<[u8; 4]>)>,
    /// ユーザーマスク。チャンネル -2 として書く。
    pub mask: Option<Mask>,
    /// ベクトルマスクと併用したときの最終的なマスク。チャンネル -3 として書く (`mask` も要る)。
    /// 見た目にはこちらだけが効く。
    pub real_mask: Option<Mask>,
    /// `luni` (Unicode の名前) を書く。false なら名前はパスカル文字列だけになる。
    pub unicode_name: bool,
    /// パスカル文字列の名前のバイト列。`None` なら `name` の ASCII 以外を `?` にしたもの。
    pub legacy_name: Option<Vec<u8>>,
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
            mask: None,
            real_mask: None,
            unicode_name: true,
            legacy_name: None,
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

    /// 画像を持つレイヤー。
    pub fn image(name: &str, left: i32, top: i32, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(rgba.len(), width as usize * height as usize * 4, "{name}");
        Self {
            left,
            top,
            width,
            height,
            rgba: Some(rgba),
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

    /// グループで実際に使われるブレンドモード (`lsct` にあればそちら)。
    pub fn group_blend(&self) -> [u8; 4] {
        match self.section {
            Some((_, Some(blend))) => blend,
            _ => self.blend,
        }
    }
}

/// レイヤーマスク (仕様の "Layer mask / adjustment layer data")。
#[derive(Clone)]
pub struct Mask {
    /// キャンバス上の矩形。
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    /// 矩形の中の値 (0 = 隠す、255 = 見せる)。
    pub pixels: Vec<u8>,
    /// 矩形の外の値 (0 か 255)。
    pub default_color: u8,
    /// マスクを無効にしている (フラグの bit 1)。
    pub disabled: bool,
    /// 濃度 (フラグの bit 4 と、マスクの値の bit 0)。-2 のマスクだけで使う。
    pub density: Option<u8>,
}

impl Mask {
    /// `(left, top)` から `width` x `height` を見せ、外を隠すマスク。
    pub fn window(left: i32, top: i32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
            pixels: vec![255; width as usize * height as usize],
            default_color: 0,
            disabled: false,
            density: None,
        }
    }

    /// キャンバス上の点のマスクの値。
    pub fn value(&self, x: i32, y: i32) -> u8 {
        let (mx, my) = (x - self.left, y - self.top);
        if mx < 0 || my < 0 || mx >= self.width as i32 || my >= self.height as i32 {
            self.default_color
        } else {
            self.pixels[my as usize * self.width as usize + mx as usize]
        }
    }

    /// 濃度を掛けた、キャンバス上の点のマスクの値 (0..=1)。
    pub fn coverage(&self, x: i32, y: i32) -> f32 {
        let hide = 1.0 - f32::from(self.value(x, y)) / 255.0;
        1.0 - hide * f32::from(self.density.unwrap_or(255)) / 255.0
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

/// 何も変えないレベル補正 (`levl`) の中身。
///
/// 仕様の "Levels file format": 版 (2) と 29 組のレコード (入力の下限・上限、出力の下限・上限、
/// ガンマ × 100 の 5 つの 16 ビット整数)。28・29 組目は予約で 0 にする。
pub fn levels_identity() -> Vec<u8> {
    let mut out = Out(Vec::new());
    out.u16(2);
    for i in 0..29 {
        let record: [u16; 5] = if i < 27 {
            [0, 255, 0, 255, 100]
        } else {
            [0; 5]
        };
        for v in record {
            out.u16(v);
        }
    }
    out.0
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

/// zlib ストリーム (RFC 1950 / 1951)。
pub fn zlib(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// 1 チャンネル分のデータ (圧縮方式の 2 バイトを含む)。`plane` は 8 ビットの画素で、
/// `depth` が 16 なら各値を 257 倍した (0..=65535 に広げた) ビッグエンディアンの 16 ビットで書く。
fn channel_data(plane: &[u8], width: u32, height: u32, compression: u16, depth: u16) -> Vec<u8> {
    let mut out = Out(Vec::new());
    out.u16(compression);
    if depth == 16 {
        let wide: Vec<u8> = plane
            .iter()
            .flat_map(|&v| (u16::from(v) * 257).to_be_bytes())
            .collect();
        match compression {
            // 行のバイト数が 2 倍になるだけで、PackBits はバイト単位。
            0..=2 => {
                let inner = channel_data(&wide, width * 2, height, compression, 8);
                out.bytes(&inner[2..]);
            }
            3 => {
                // 予測: 各行で、左の標本との差を 16 ビットで書く。
                let mut delta = Vec::with_capacity(wide.len());
                for row in wide.chunks(width.max(1) as usize * 2).take(height as usize) {
                    let mut prev = 0u16;
                    for s in row.as_chunks::<2>().0 {
                        let v = u16::from_be_bytes(*s);
                        delta.extend_from_slice(&v.wrapping_sub(prev).to_be_bytes());
                        prev = v;
                    }
                }
                out.bytes(&zlib(&delta));
            }
            c => panic!("compression {c} is not defined by the psd format"),
        }
        return out.0;
    }
    let rows = || plane.chunks(width.max(1) as usize).take(height as usize);
    match compression {
        0 => out.bytes(plane),
        1 => {
            let rows: Vec<Vec<u8>> = rows().map(pack_bits).collect();
            for row in &rows {
                out.u16(row.len() as u16);
            }
            for row in &rows {
                out.bytes(row);
            }
        }
        2 => out.bytes(&zlib(plane)),
        3 => {
            // 予測: 各行で、左の画素との差を書く。
            let mut delta = Vec::with_capacity(plane.len());
            for row in rows() {
                let mut prev = 0u8;
                for &v in row {
                    delta.push(v.wrapping_sub(prev));
                    prev = v;
                }
            }
            out.bytes(&zlib(&delta));
        }
        c => panic!("compression {c} is not defined by the psd format"),
    }
    out.0
}

/// 画像リソース 1 つ (シグネチャ・ID・空の名前・長さ・偶数に揃えた中身)。
fn image_resource(out: &mut Out, id: u16, data: &[u8]) {
    out.bytes(b"8BIM");
    out.u16(id);
    out.u16(0); // 空のパスカル文字列 (偶数の長さに揃えて 2 バイト)
    out.block(2, |d| d.bytes(data));
}

/// 8 ビット RGB の PSD を書く。`layers` は下から上の順。
///
/// 合成画像 (画像データのセクション) は [`compose::flatten`] でレイヤーから計算し、PackBits で書く。
pub fn write_psd(width: u32, height: u32, layers: &[Layer]) -> Vec<u8> {
    let composite = compose::flatten(width, height, layers);
    write(width, height, layers, &composite, 1, 8)
}

/// 16 ビット RGB の PSD を書く。Photoshop と同じく、ヘッダの直後のレイヤー情報は空にし、
/// レイヤーはレイヤーとマスクの情報の末尾の追加情報 `Lr16` に書く。
/// 画素 (8 ビットで渡す) と合成画像は 257 倍して 16 ビットにする。
pub fn write_psd16(width: u32, height: u32, layers: &[Layer]) -> Vec<u8> {
    let composite = compose::flatten(width, height, layers);
    write(width, height, layers, &composite, 1, 16)
}

/// 合成画像 (RGB8) を渡して書く。グループの対応が取れていないなど、合成できない (壊れた)
/// レイヤーの並びを試すテストだけで使う。
pub fn write_psd_unchecked(width: u32, height: u32, layers: &[Layer], rgb: &[u8]) -> Vec<u8> {
    write(width, height, layers, rgb, 1, 8)
}

/// レイヤーを持たない 8 ビット RGB の PSD を書く。`rgb` は RGB8 の画像。
pub fn write_flat_psd(width: u32, height: u32, rgb: &[u8], compression: u16) -> Vec<u8> {
    write(width, height, &[], rgb, compression, 8)
}

fn write(
    width: u32,
    height: u32,
    layers: &[Layer],
    rgb: &[u8],
    compression: u16,
    depth: u16,
) -> Vec<u8> {
    let plane = width as usize * height as usize;
    assert_eq!(rgb.len(), plane * 3);
    let mut out = Out(Vec::new());
    // ヘッダ: シグネチャ・版 1・予約 6 バイト・チャンネル数・高さ・幅・ビット深度・カラーモード (RGB)。
    out.bytes(b"8BPS");
    out.u16(1);
    out.bytes(&[0; 6]);
    out.u16(3);
    out.u32(height);
    out.u32(width);
    out.u16(depth);
    out.u16(3);
    out.u32(0); // カラーモードデータ (RGB では空)
    out.block(2, |res| {
        // ResolutionInfo (1005): 72 dpi。固定小数点 16.16 の解像度と単位 (1 = ピクセル/インチ、
        // 1 = インチ) を横・縦の順に書く。
        image_resource(res, 1005, &{
            let mut d = Out(Vec::new());
            for _ in 0..2 {
                d.u32(72 << 16);
                d.u16(1);
                d.u16(1);
            }
            d.0
        });
    });
    out.block(2, |lm| {
        if layers.is_empty() {
            return;
        }
        let layer_info = |info: &mut Out| {
            info.i16(layers.len() as i16);
            let channels: Vec<Vec<(i16, Vec<u8>)>> =
                layers.iter().map(|l| channels(l, depth)).collect();
            for (layer, channels) in layers.iter().zip(&channels) {
                layer_record(info, layer, channels);
            }
            for channels in &channels {
                for (_, data) in channels {
                    info.bytes(data);
                }
            }
        };
        if depth == 8 {
            lm.block(2, layer_info);
        } else {
            lm.u32(0);
        }
        // グローバルなレイヤーマスク情報: 重ねる色の色空間・色 4 つ・不透明度 100・種類
        // (128 = レイヤーごとの値を使う)・詰め物。
        // 16 ビットでは空にする (ImageMagick は空の直後に `Lr16` があるときだけレイヤーを読む)。
        lm.block(2, |g| {
            if depth != 8 {
                return;
            }
            g.u16(0);
            g.bytes(&[0; 8]);
            g.u16(100);
            g.u8(128);
        });
        if depth == 16 {
            // 追加情報: シグネチャ・キー・長さ (4 の倍数に揃える)・レイヤー情報 (長さを除いた本体)。
            lm.bytes(b"8BIM");
            lm.bytes(b"Lr16");
            lm.block(4, layer_info);
        }
    });
    // 画像データ: RRR GGG BBB の平面順。
    let planes: Vec<u8> = (0..3)
        .flat_map(|c| (0..plane).map(move |i| rgb[i * 3 + c]))
        .flat_map(|v| {
            if depth == 16 {
                (u16::from(v) * 257).to_be_bytes().to_vec()
            } else {
                vec![v]
            }
        })
        .collect();
    let row = width as usize * usize::from(depth / 8);
    out.u16(compression);
    match compression {
        0 => out.bytes(&planes),
        1 => {
            let rows: Vec<Vec<u8>> = planes.chunks(row).map(pack_bits).collect();
            for row in &rows {
                out.u16(row.len() as u16);
            }
            for row in &rows {
                out.bytes(row);
            }
        }
        c => panic!("composite compression {c} is not written by this writer"),
    }
    out.0
}

fn channels(layer: &Layer, depth: u16) -> Vec<(i16, Vec<u8>)> {
    let mut out: Vec<(i16, Vec<u8>)> = [-1i16, 0, 1, 2]
        .iter()
        .map(|&id| {
            let data = match &layer.rgba {
                Some(rgba) => {
                    let c = if id < 0 { 3 } else { id as usize };
                    let plane: Vec<u8> = rgba.as_chunks::<4>().0.iter().map(|p| p[c]).collect();
                    channel_data(&plane, layer.width, layer.height, layer.compression, depth)
                }
                // 矩形が空なので、中身は圧縮方式 (無圧縮) の 2 バイトだけ。
                None => vec![0, 0],
            };
            (id, data)
        })
        .collect();
    if let Some(mask) = &layer.mask {
        let data = channel_data(
            &mask.pixels,
            mask.width,
            mask.height,
            layer.compression,
            depth,
        );
        out.push((-2, data));
    }
    if let Some(real) = &layer.real_mask {
        assert!(
            layer.mask.is_some(),
            "{}: real mask needs a user mask",
            layer.name
        );
        let data = channel_data(
            &real.pixels,
            real.width,
            real.height,
            layer.compression,
            depth,
        );
        out.push((-3, data));
    }
    out
}

fn layer_record(out: &mut Out, layer: &Layer, channels: &[(i16, Vec<u8>)]) {
    if layer.rgba.is_none() {
        assert_eq!((layer.width, layer.height), (0, 0), "{}", layer.name);
    }
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
    // bit 1: 非表示、bit 3: bit 4 が意味を持つ、bit 4: 画素が見た目に関係しない (グループの印)。
    let mut flags = 0x08;
    if layer.hidden {
        flags |= 0x02;
    }
    if layer.section.is_some() {
        flags |= 0x10;
    }
    out.u8(flags);
    out.u8(0);
    out.block(1, |extra| {
        if let Some(mask) = &layer.mask {
            // 矩形 (上・左・下・右)・既定の色・フラグ。-3 があれば -3 のフラグ・既定の色・矩形、
            // 濃度があればマスクの値の種類 (bit 0 = 濃度) と濃度。何もなければ 2 バイトの詰め物で 20 バイト。
            // 仕様書は濃度を -3 より前に書くが、実際のファイル (psd-tools の読み方) はこの順。
            let flags = |m: &Mask| {
                let mut f = if m.disabled { 0x02 } else { 0 };
                if m.density.is_some() {
                    f |= 0x10;
                }
                f
            };
            let rect = |out: &mut Out, m: &Mask| {
                out.i32(m.top);
                out.i32(m.left);
                out.i32(m.top + m.height as i32);
                out.i32(m.left + m.width as i32);
            };
            extra.block(1, |m| {
                rect(m, mask);
                m.u8(mask.default_color);
                m.u8(flags(mask));
                if let Some(real) = &layer.real_mask {
                    m.u8(flags(real));
                    m.u8(real.default_color);
                    rect(m, real);
                }
                if let Some(density) = mask.density {
                    m.u8(0x01);
                    m.u8(density);
                }
                if m.0.len() == 18 {
                    m.u16(0);
                }
            });
        } else {
            extra.u32(0);
        }
        // ブレンド範囲: 合成のグレーと 4 チャンネル分の (元・先) の組。すべて 0..255 (効果なし)。
        extra.block(1, |b| {
            for _ in 0..5 * 2 {
                b.bytes(&[0, 0, 255, 255]);
            }
        });
        // パスカル文字列の名前 (既定では ASCII 以外を `?`)。長さのバイトを含めて 4 の倍数に揃える。
        let ascii: Vec<u8> = layer.legacy_name.clone().unwrap_or_else(|| {
            layer
                .name
                .chars()
                .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                .take(255)
                .collect()
        });
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
        // Unicode 文字列: UTF-16 の単位数・文字列・終端の 0。
        let utf16: Vec<u16> = layer.name.encode_utf16().collect();
        if layer.unicode_name {
            tagged(b"luni", &|d| {
                d.u32(utf16.len() as u32);
                for u in &utf16 {
                    d.u16(*u);
                }
                d.u16(0);
            });
        }
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
