//! スタジオ (書き手) と Windows のメディアソース (読み手。OS のサービス Frame Server の中で動く) の間の共有メモリの形。
//!
//! 共有メモリは読み手 (メディアソース) が作る (別のセッションから見える `Global\` の名前の共有メモリは、サービスでないと作れないため)。
//! スタジオは名前で開いて書く。形は版の番号付きで、スタジオとメディアソースの DLL は同じインストーラで入るので同じ版になる。
//!
//! | 位置 (u32 の語) | 書く側 | 中身 |
//! | --- | --- | --- |
//! | 0 | 読み手 | [`MAGIC`] |
//! | 1 | 読み手 | [`VERSION`] |
//! | 2 | 読み手 | 枠の数 ([`SLOTS`]) |
//! | 3 | 読み手 | 1 枠のバイト数 ([`SLOT_BYTES`]) |
//! | 4 | 読み手 | 渡したフレームの数 (受け取るアプリがあるかを書き手が知るため。1 枚渡すたびに 1 足す) |
//! | 8〜11 | 書き手 | 幅・高さ・画素の形 ([`FOURCC_NV12`])・フレームレート |
//! | 12 | 書き手 | 最新の枠の番号 + 1 (0 はまだ無い) |
//! | 13 | 書き手 | 書いたフレームの数 |
//! | 16 + i | 書き手 | 枠 i の版 (奇数は書いている途中。seqlock) |
//!
//! 枠 i の画素は `HEADER_BYTES + i * SLOT_BYTES` から。書き手は最新でない枠に書き、版を奇数 → 偶数にしてから最新の番号を替える。
//! 読み手は最新の枠を写し、写す前と後で版が同じ偶数なら使う (途中で書き換えられたら前の画を使い続ける)。
//!
//! 共有メモリへの読み書き ([`Region`]) だけが OS の境界で、ここの手順は safe な Rust で書いてテストする。

/// 先頭の印 (`UGVC`)。
pub const MAGIC: u32 = u32::from_le_bytes(*b"UGVC");
/// 形の版。形を変えたら上げる。
pub const VERSION: u32 = 1;
/// 枠の数 (書き手が 1 枚書く間に、読み手が別の 2 枚のどちらかを読める)。
pub const SLOTS: usize = 3;
/// 頭の大きさ (バイト)。
pub const HEADER_BYTES: usize = 4096;
/// 出せる最大の大きさ (1920×1080)。
pub const MAX_SIZE: (u32, u32) = (1920, 1080);
/// 1 枠のバイト数 (1920×1080 の NV12 を 4096 の倍数に切り上げたもの)。
pub const SLOT_BYTES: usize = (1920 * 1080 * 3 / 2usize).next_multiple_of(4096);
/// 共有メモリ全体の大きさ。
pub const REGION_BYTES: usize = HEADER_BYTES + SLOTS * SLOT_BYTES;
/// 画素の形 NV12 の FourCC。
pub const FOURCC_NV12: u32 = u32::from_le_bytes(*b"NV12");

// 1 枠に 1080p の NV12 が入り、枠の版が頭に収まる。
const _: () = assert!(SLOT_BYTES >= 1920 * 1080 * 3 / 2 && SLOT_BYTES.is_multiple_of(4096));
const _: () = assert!((W_SLOT_SEQ + SLOTS) * 4 <= HEADER_BYTES);

const W_MAGIC: usize = 0;
const W_VERSION: usize = 1;
const W_SLOTS: usize = 2;
const W_SLOT_BYTES: usize = 3;
const W_DELIVERED: usize = 4;
const W_WIDTH: usize = 8;
const W_HEIGHT: usize = 9;
const W_FORMAT: usize = 10;
const W_FPS: usize = 11;
const W_LATEST: usize = 12;
const W_WRITTEN: usize = 13;
const W_SLOT_SEQ: usize = 16;

/// 共有メモリ。語の読み書きは原子的に (書き手は Release、読み手は Acquire)、バイト列はまとめて写す。
pub trait Region {
    /// 大きさ (バイト)。
    fn byte_len(&self) -> usize;
    /// `word` 番目の u32 を読む (Acquire)。
    fn load(&self, word: usize) -> u32;
    /// `word` 番目の u32 に書く (Release)。
    fn store(&self, word: usize, value: u32);
    /// `offset` から `bytes` を写す。
    fn write(&self, offset: usize, bytes: &[u8]);
    /// `offset` から `out` に写す。
    fn read(&self, offset: usize, out: &mut [u8]);
}

/// 共有メモリの形が合わない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SharedError {
    #[error("the shared memory is too small ({0} bytes)")]
    TooSmall(usize),
    #[error("the shared memory is not for the ugoe virtual camera")]
    Magic,
    #[error("the virtual camera driver is version {0}, but the studio needs version {VERSION}")]
    Version(u32),
    #[error("the frame is {width}x{height}, larger than the virtual camera allows")]
    TooLarge { width: u32, height: u32 },
    #[error("the frame has {actual} bytes but {expected} were expected")]
    Length { expected: usize, actual: usize },
}

/// 1 枚の NV12 のバイト数。
pub fn nv12_bytes(width: u32, height: u32) -> usize {
    width as usize * height as usize * 3 / 2
}

/// スタジオがメディアソースに設定 (大きさとフレームレート) を渡すときのバイト列 (版・幅・高さ・フレームレートの u32)。
/// Windows では仮想カメラの機器のプロパティに入れ、メディアソースが起動したときに読む。
pub fn encode_config(config: crate::Config) -> [u8; 16] {
    let mut out = [0; 16];
    for (i, v) in [VERSION, config.width, config.height, config.fps]
        .into_iter()
        .enumerate()
    {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// [`encode_config`] の逆。版が違う・選べない設定なら `None` (メディアソースは選べる設定をすべて出す)。
pub fn decode_config(bytes: &[u8]) -> Option<crate::Config> {
    let word = |i: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            bytes.get(i * 4..i * 4 + 4)?.try_into().ok()?,
        ))
    };
    if bytes.len() != 16 || word(0)? != VERSION {
        return None;
    }
    let config = crate::Config {
        width: word(1)?,
        height: word(2)?,
        fps: word(3)?,
    };
    config.validate().ok().map(|()| config)
}

/// Media Foundation の media type の `MF_MT_FRAME_SIZE` (幅 << 32 | 高さ) と `MF_MT_FRAME_RATE` (分子 << 32 | 分母) を
/// 設定にする。選べる組でなければ `None` (メディアソースは出した組のほかは受けない。巨大な大きさで確保しないため)。
/// フレームレートは四捨五入する (30000/1001 は 30)。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn decode_media_type(frame_size: u64, frame_rate: u64) -> Option<crate::Config> {
    let (num, den) = (
        u64::from((frame_rate >> 32) as u32),
        u64::from(frame_rate as u32),
    );
    if den == 0 {
        return None;
    }
    let config = crate::Config {
        width: (frame_size >> 32) as u32,
        height: frame_size as u32,
        fps: u32::try_from((num + den / 2) / den).ok()?,
    };
    config.validate().ok().map(|()| config)
}

/// 黒 (BT.709 の限定範囲: Y = 16、Cb = Cr = 128) の NV12 で `out` を埋める (まだ画が届いていないとき)。
pub fn fill_black_nv12(out: &mut [u8], width: u32, height: u32) {
    let luma = (width as usize * height as usize).min(out.len());
    let (y, uv) = out.split_at_mut(luma);
    y.fill(16);
    uv.fill(128);
}

/// NV12 の画を最も近い画素で拡大縮小する (受け取るアプリが選んだ大きさとスタジオの設定が違うとき)。
/// `src` は `sw`×`sh`、`dst` は `dw`×`dh` の NV12。大きさは偶数とする。どちらかの大きさが 0 なら何もしない。
pub fn scale_nv12_nearest(src: &[u8], (sw, sh): (u32, u32), dst: &mut [u8], (dw, dh): (u32, u32)) {
    let (sw, sh, dw, dh) = (sw as usize, sh as usize, dw as usize, dh as usize);
    if src.len() < sw * sh * 3 / 2 || dst.len() < dw * dh * 3 / 2 || [sw, sh, dw, dh].contains(&0) {
        return;
    }
    let (src_y, src_uv) = src.split_at(sw * sh);
    let (dst_y, dst_uv) = dst.split_at_mut(dw * dh);
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            dst_y[y * dw + x] = src_y[sy * sw + x * sw / dw];
        }
    }
    // 色差の面は幅 (バイト) が同じで高さが半分。Cb Cr の組ごとに写す。
    let (sw2, sh2, dw2, dh2) = (sw / 2, sh / 2, dw / 2, dh / 2);
    for y in 0..dh2 {
        let sy = y * sh2 / dh2;
        for x in 0..dw2 {
            let sx = x * sw2 / dw2;
            let s = sy * sw + sx * 2;
            let d = y * dw + x * 2;
            dst_uv[d..d + 2].copy_from_slice(&src_uv[s..s + 2]);
        }
    }
}

/// 読み手が読んだフレームの情報。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// 書き手が書いたフレームの数 (書き手が止まったかを知るため)。
    pub written: u32,
}

/// 書き手 (スタジオ)。
#[derive(Debug)]
pub struct Writer<R: Region> {
    region: R,
}

impl<R: Region> Writer<R> {
    /// 読み手が作った共有メモリを確かめて書き手になる。
    pub fn new(region: R) -> Result<Self, SharedError> {
        if region.byte_len() < REGION_BYTES {
            return Err(SharedError::TooSmall(region.byte_len()));
        }
        if region.load(W_MAGIC) != MAGIC {
            return Err(SharedError::Magic);
        }
        let version = region.load(W_VERSION);
        if version != VERSION
            || region.load(W_SLOTS) as usize != SLOTS
            || region.load(W_SLOT_BYTES) as usize != SLOT_BYTES
        {
            return Err(SharedError::Version(version));
        }
        Ok(Self { region })
    }

    /// 読み手が渡したフレームの数。増えていれば受け取るアプリがある。
    pub fn delivered(&self) -> u32 {
        self.region.load(W_DELIVERED)
    }

    /// NV12 の 1 枚を書く。
    pub fn write(
        &mut self,
        width: u32,
        height: u32,
        fps: u32,
        nv12: &[u8],
    ) -> Result<(), SharedError> {
        if width > MAX_SIZE.0 || height > MAX_SIZE.1 {
            return Err(SharedError::TooLarge { width, height });
        }
        let expected = nv12_bytes(width, height);
        if nv12.len() != expected {
            return Err(SharedError::Length {
                expected,
                actual: nv12.len(),
            });
        }
        let r = &self.region;
        let latest = r.load(W_LATEST) as usize;
        // 最新 (latest - 1) の次の枠に書く。読み手が今読んでいるのは最新の枠なので、ふつうはぶつからない。
        let slot = latest % SLOTS;
        let seq = r.load(W_SLOT_SEQ + slot);
        // 奇数 (書いている途中) にする。前の書き手が途中で落ちて奇数のままなら、そのまま使う。
        let writing = seq | 1;
        r.store(W_SLOT_SEQ + slot, writing);
        r.write(HEADER_BYTES + slot * SLOT_BYTES, nv12);
        r.store(W_WIDTH, width);
        r.store(W_HEIGHT, height);
        r.store(W_FORMAT, FOURCC_NV12);
        r.store(W_FPS, fps);
        r.store(W_SLOT_SEQ + slot, writing.wrapping_add(1));
        r.store(W_LATEST, slot as u32 + 1);
        r.store(W_WRITTEN, r.load(W_WRITTEN).wrapping_add(1));
        Ok(())
    }
}

/// 読み手 (メディアソース)。
#[derive(Debug)]
pub struct Reader<R: Region> {
    region: R,
}

impl<R: Region> Reader<R> {
    /// 共有メモリの頭を書いて読み手になる。すでに頭があれば (前のメディアソースが作ったものを開き直した) 書き手の分は残す。
    pub fn new(region: R) -> Result<Self, SharedError> {
        if region.byte_len() < REGION_BYTES {
            return Err(SharedError::TooSmall(region.byte_len()));
        }
        if region.load(W_MAGIC) != MAGIC || region.load(W_VERSION) != VERSION {
            for word in 0..W_SLOT_SEQ + SLOTS {
                region.store(word, 0);
            }
            region.store(W_SLOTS, SLOTS as u32);
            region.store(W_SLOT_BYTES, SLOT_BYTES as u32);
            region.store(W_VERSION, VERSION);
            region.store(W_MAGIC, MAGIC);
        }
        Ok(Self { region })
    }

    /// 最新のフレームを `out` に写す。まだ無い・形が違う・大きさが 0 か奇数か大きすぎる・書き換えの途中だったときは `None` (`out` は変えないとは限らない)。
    pub fn read(&self, out: &mut Vec<u8>) -> Option<FrameInfo> {
        let r = &self.region;
        let latest = r.load(W_LATEST) as usize;
        if latest == 0 || latest > SLOTS || r.load(W_FORMAT) != FOURCC_NV12 {
            return None;
        }
        let slot = latest - 1;
        let before = r.load(W_SLOT_SEQ + slot);
        if before & 1 != 0 {
            return None;
        }
        let info = FrameInfo {
            width: r.load(W_WIDTH),
            height: r.load(W_HEIGHT),
            fps: r.load(W_FPS),
            written: r.load(W_WRITTEN),
        };
        // 書き手が書いた値でも確かめる (共有メモリは対話でログオンした利用者なら書ける)。NV12 は偶数の大きさ。
        let even = |v: u32| v > 0 && v.is_multiple_of(2);
        if !even(info.width)
            || !even(info.height)
            || info.width > MAX_SIZE.0
            || info.height > MAX_SIZE.1
        {
            return None;
        }
        out.resize(nv12_bytes(info.width, info.height), 0);
        r.read(HEADER_BYTES + slot * SLOT_BYTES, out);
        (r.load(W_SLOT_SEQ + slot) == before).then_some(info)
    }

    /// 1 枚を受け取るアプリに渡したことを書き手に知らせる。
    pub fn mark_delivered(&self) {
        let r = &self.region;
        r.store(W_DELIVERED, r.load(W_DELIVERED).wrapping_add(1));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;

    /// テスト用のメモリ (プロセスの中の Vec)。
    #[derive(Debug)]
    pub(crate) struct Memory(pub Mutex<Vec<u8>>);

    impl Memory {
        pub(crate) fn new(len: usize) -> Self {
            Self(Mutex::new(vec![0; len]))
        }
    }

    impl Region for &Memory {
        fn byte_len(&self) -> usize {
            self.0.lock().unwrap().len()
        }
        fn load(&self, word: usize) -> u32 {
            let m = self.0.lock().unwrap();
            u32::from_le_bytes(m[word * 4..word * 4 + 4].try_into().unwrap())
        }
        fn store(&self, word: usize, value: u32) {
            self.0.lock().unwrap()[word * 4..word * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        fn write(&self, offset: usize, bytes: &[u8]) {
            self.0.lock().unwrap()[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        fn read(&self, offset: usize, out: &mut [u8]) {
            let m = self.0.lock().unwrap();
            out.copy_from_slice(&m[offset..offset + out.len()]);
        }
    }

    #[test]
    fn nv12_helpers() {
        let mut black = vec![0; nv12_bytes(4, 2)];
        fill_black_nv12(&mut black, 4, 2);
        assert_eq!(black, [16, 16, 16, 16, 16, 16, 16, 16, 128, 128, 128, 128]);

        // 4×2 → 8×4 (2 倍): 各画素が 2×2 に、色差の組が横に 2 つずつ並ぶ。
        let src = [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13];
        let mut dst = vec![0; nv12_bytes(8, 4)];
        scale_nv12_nearest(&src, (4, 2), &mut dst, (8, 4));
        assert_eq!(&dst[..8], &[1, 1, 2, 2, 3, 3, 4, 4]);
        assert_eq!(&dst[24..32], &[5, 5, 6, 6, 7, 7, 8, 8]);
        assert_eq!(&dst[32..40], &[10, 11, 10, 11, 12, 13, 12, 13]);
        assert_eq!(&dst[40..48], &dst[32..40]);
        // 縮小 (8×4 → 4×2) で元に戻る。
        let mut back = vec![0; nv12_bytes(4, 2)];
        scale_nv12_nearest(&dst, (8, 4), &mut back, (4, 2));
        assert_eq!(back, src);
    }

    #[test]
    fn config_round_trips() {
        let config = crate::Config {
            width: 1920,
            height: 1080,
            fps: 60,
        };
        assert_eq!(decode_config(&encode_config(config)), Some(config));
        assert_eq!(decode_config(&[0; 16]), None);
        assert_eq!(decode_config(&encode_config(config)[..12]), None);
        let mut odd = encode_config(config);
        odd[12] = 25;
        assert_eq!(decode_config(&odd), None);
    }

    #[test]
    fn only_offered_media_types_are_accepted() {
        let size = |w: u64, h: u64| (w << 32) | h;
        let rate = |n: u64, d: u64| (n << 32) | d;
        assert_eq!(
            decode_media_type(size(1920, 1080), rate(60, 1)),
            Some(crate::Config {
                width: 1920,
                height: 1080,
                fps: 60
            })
        );
        assert_eq!(
            decode_media_type(size(1280, 720), rate(30_000, 1001)).map(|c| c.fps),
            Some(30)
        );
        assert_eq!(decode_media_type(size(65_535, 65_535), rate(30, 1)), None);
        assert_eq!(decode_media_type(size(0, 0), rate(30, 1)), None);
        assert_eq!(decode_media_type(size(1280, 720), rate(30, 0)), None);
        assert_eq!(decode_media_type(size(1280, 720), rate(0, 1)), None);
        assert_eq!(
            decode_media_type(size(1280, 720), rate(u64::from(u32::MAX), 1)),
            None
        );
    }

    #[test]
    fn magic_reads_ugvc() {
        assert_eq!(&MAGIC.to_le_bytes(), b"UGVC");
    }

    #[test]
    fn the_writer_needs_a_reader_made_region() {
        let memory = Memory::new(REGION_BYTES);
        assert_eq!(Writer::new(&memory).unwrap_err(), SharedError::Magic);
        let small = Memory::new(16);
        assert_eq!(Writer::new(&small).unwrap_err(), SharedError::TooSmall(16));
        Reader::new(&memory).unwrap();
        (&memory).store(W_VERSION, VERSION + 1);
        assert_eq!(
            Writer::new(&memory).unwrap_err(),
            SharedError::Version(VERSION + 1)
        );
    }

    #[test]
    fn frames_pass_through_and_the_newest_wins() {
        let memory = Memory::new(REGION_BYTES);
        let reader = Reader::new(&memory).unwrap();
        let mut writer = Writer::new(&memory).unwrap();
        let mut out = Vec::new();
        assert_eq!(reader.read(&mut out), None);

        let frame = |v: u8| vec![v; nv12_bytes(8, 4)];
        for v in 1..=5 {
            writer.write(8, 4, 30, &frame(v)).unwrap();
        }
        let info = reader.read(&mut out).unwrap();
        assert_eq!(
            info,
            FrameInfo {
                width: 8,
                height: 4,
                fps: 30,
                written: 5
            }
        );
        assert_eq!(out, frame(5));

        assert_eq!(writer.delivered(), 0);
        reader.mark_delivered();
        assert_eq!(writer.delivered(), 1);

        assert_eq!(
            writer.write(8, 4, 30, &[0; 3]).unwrap_err(),
            SharedError::Length {
                expected: 48,
                actual: 3
            }
        );
        assert!(matches!(
            writer.write(3840, 2160, 30, &[]),
            Err(SharedError::TooLarge { .. })
        ));
    }

    #[test]
    fn a_torn_frame_is_not_used() {
        let memory = Memory::new(REGION_BYTES);
        let reader = Reader::new(&memory).unwrap();
        let mut writer = Writer::new(&memory).unwrap();
        writer.write(8, 4, 30, &[7; 48]).unwrap();
        // 書き手が最新の枠を書き換えている途中 (版が奇数)。
        let slot = (&memory).load(W_LATEST) as usize - 1;
        let seq = (&memory).load(W_SLOT_SEQ + slot);
        (&memory).store(W_SLOT_SEQ + slot, seq + 1);
        assert_eq!(reader.read(&mut Vec::new()), None);
        (&memory).store(W_SLOT_SEQ + slot, seq + 2);
        assert!(reader.read(&mut Vec::new()).is_some());
    }

    #[test]
    fn frames_of_odd_or_zero_size_are_not_read() {
        // 共有メモリは対話でログオンした利用者なら書けるので、頭の大きさは信じない (NV12 は偶数の大きさ)。
        let memory = Memory::new(REGION_BYTES);
        let reader = Reader::new(&memory).unwrap();
        let mut writer = Writer::new(&memory).unwrap();
        writer.write(8, 4, 30, &[7; 48]).unwrap();
        for (width, height) in [(0, 4), (8, 0), (0, 0), (7, 4), (8, 3)] {
            (&memory).store(W_WIDTH, width);
            (&memory).store(W_HEIGHT, height);
            assert_eq!(reader.read(&mut Vec::new()), None, "{width}x{height}");
        }
    }

    #[test]
    fn scaling_from_an_empty_frame_does_nothing() {
        let mut dst = vec![5; nv12_bytes(4, 2)];
        scale_nv12_nearest(&[], (0, 0), &mut dst, (4, 2));
        scale_nv12_nearest(&[], (4, 0), &mut dst, (4, 2));
        scale_nv12_nearest(&[], (0, 2), &mut dst, (4, 2));
        assert!(dst.iter().all(|&v| v == 5));
    }

    #[test]
    fn reopening_keeps_the_writers_frames() {
        let memory = Memory::new(REGION_BYTES);
        Reader::new(&memory).unwrap();
        Writer::new(&memory)
            .unwrap()
            .write(8, 4, 60, &[9; 48])
            .unwrap();
        let reader = Reader::new(&memory).unwrap();
        let mut out = Vec::new();
        assert_eq!(reader.read(&mut out).unwrap().fps, 60);
    }
}
