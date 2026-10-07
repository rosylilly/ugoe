//! チャンクのペイロードの MessagePack の読み書き (serde)。`no_std` + `alloc` で動く。
//! 仕様: `docs/spec/file-formats.md#ペイロードのエンコード`、判断の経緯: ADR 0008・ADR 0031。
//!
//! 書き出すバイト列と読み込みの振る舞いは、以前に使っていた `rmp-serde` 1.x の `to_vec_named` と
//! `Deserializer::from_read_ref` (既定の設定) と同じにしてある (`rmp-serde` は `std` を前提にするので置き換えた)。
//! 同じであることはテスト (`tests` と `assets/` のファイル) で `rmp-serde` と比べて確かめる。
//!
//! - 構造体はフィールド名付きのマップ。タプル・タプル構造体は配列。単位構造体は空の配列、`()` と `None` は nil。
//! - enum の単位のバリアントは名前の文字列。ほかのバリアントは名前をキーにした要素 1 つのマップ
//!   (`{名前: 値}`・`{名前: [..]}`・`{名前: {..}}`)。
//! - 整数は値が収まるいちばん短い形 (0 以上は符号なしの形)。`f32` は float 32、`f64` は float 64。
//! - 長さの分からない列とマップ (`#[serde(flatten)]` など) は、書き終えてから要素の数を前に置く。
//! - `is_human_readable` は `false`。`i128`・`u128` と MessagePack の拡張型を書くことには対応しない
//!   (モデルの型は使わない。拡張型は読むことはできる)。

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;

use serde::de::value::{
    BorrowedBytesDeserializer, I8Deserializer, SeqAccessDeserializer, SeqDeserializer,
};
use serde::de::{self, DeserializeSeed, Unexpected, Visitor};
use serde::ser::{self, Serialize};

/// MessagePack への書き出しの失敗。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MsgPackEncodeError {
    /// 文字列・バイト列・配列・マップの長さが MessagePack で書ける上限 (`u32::MAX`) を超える。
    #[error("length {0} is too long for MessagePack")]
    TooLong(usize),
    /// 値の `Serialize` の実装が失敗した (serde のメッセージ)。
    #[error("{0}")]
    Custom(String),
}

impl ser::Error for MsgPackEncodeError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

/// MessagePack の読み込みの失敗。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MsgPackDecodeError {
    /// 値の途中でバイト列が終わった。
    #[error("unexpected end of MessagePack data")]
    UnexpectedEof,
    /// 求める型と合わない形の値 (中身は値の先頭のバイト)。
    #[error("unexpected MessagePack marker 0x{0:02x}")]
    TypeMismatch(u8),
    /// 配列かマップの要素を、型が読み切らなかった (中身は読んだ要素の数)。
    #[error("MessagePack array or map has more elements than expected ({0} read)")]
    TrailingElements(u32),
    /// enum を表すマップの要素の数が 1 でない (中身は要素の数)。
    #[error("MessagePack enum map has {0} entries, expected 1")]
    EnumMapLength(u32),
    /// 文字列が UTF-8 でない。
    #[error("MessagePack string is not valid UTF-8: {0}")]
    Utf8(core::str::Utf8Error),
    /// 入れ子が上限より深い。
    #[error("MessagePack nesting is deeper than the limit")]
    DepthLimitExceeded,
    /// 値の `Deserialize` の実装が失敗した (serde のメッセージ。型や値が合わないなど)。
    #[error("{0}")]
    Custom(String),
}

impl de::Error for MsgPackDecodeError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

// ---- 書き出し ----

/// `value` を MessagePack (構造体はフィールド名付きのマップ) にする。
pub(crate) fn to_vec_named<T: Serialize + ?Sized>(
    value: &T,
) -> Result<Vec<u8>, MsgPackEncodeError> {
    let mut out = Vec::new();
    value.serialize(&mut Encoder { out: &mut out })?;
    Ok(out)
}

/// 長さを `u32` にする (MessagePack の長さは 32bit まで)。
fn len32(len: usize) -> Result<u32, MsgPackEncodeError> {
    u32::try_from(len).map_err(|_| MsgPackEncodeError::TooLong(len))
}

/// 長さ `len` の見出し。`fix` は 1 バイトに収まる形 (先頭のバイトと、収まる長さの上限)、`m8`・`m16`・`m32` は
/// 長さを 8・16・32bit で後ろに書く形 (配列とマップには 8bit の形が無い)。
fn write_len(
    out: &mut Vec<u8>,
    len: u32,
    fix: Option<(u8, u32)>,
    m8: Option<u8>,
    m16: u8,
    m32: u8,
) {
    match fix {
        Some((fix, fix_max)) if len < fix_max => out.push(fix | len as u8),
        _ => match (m8, u8::try_from(len), u16::try_from(len)) {
            (Some(m8), Ok(len), _) => out.extend_from_slice(&[m8, len]),
            (_, _, Ok(len)) => {
                out.push(m16);
                out.extend_from_slice(&len.to_be_bytes());
            }
            _ => {
                out.push(m32);
                out.extend_from_slice(&len.to_be_bytes());
            }
        },
    }
}

fn write_str_len(out: &mut Vec<u8>, len: u32) {
    write_len(out, len, Some((0xa0, 32)), Some(0xd9), 0xda, 0xdb);
}

fn write_bin_len(out: &mut Vec<u8>, len: u32) {
    write_len(out, len, None, Some(0xc4), 0xc5, 0xc6);
}

fn write_array_len(out: &mut Vec<u8>, len: u32) {
    write_len(out, len, Some((0x90, 16)), None, 0xdc, 0xdd);
}

fn write_map_len(out: &mut Vec<u8>, len: u32) {
    write_len(out, len, Some((0x80, 16)), None, 0xde, 0xdf);
}

fn write_uint(out: &mut Vec<u8>, v: u64) {
    if v < 0x80 {
        out.push(v as u8);
    } else if v <= u64::from(u8::MAX) {
        out.extend_from_slice(&[0xcc, v as u8]);
    } else if v <= u64::from(u16::MAX) {
        out.push(0xcd);
        out.extend_from_slice(&(v as u16).to_be_bytes());
    } else if v <= u64::from(u32::MAX) {
        out.push(0xce);
        out.extend_from_slice(&(v as u32).to_be_bytes());
    } else {
        out.push(0xcf);
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn write_sint(out: &mut Vec<u8>, v: i64) {
    if v >= 0 {
        write_uint(out, v as u64);
    } else if v >= -32 {
        out.push(v as i8 as u8);
    } else if v >= i64::from(i8::MIN) {
        out.extend_from_slice(&[0xd0, v as i8 as u8]);
    } else if v >= i64::from(i16::MIN) {
        out.push(0xd1);
        out.extend_from_slice(&(v as i16).to_be_bytes());
    } else if v >= i64::from(i32::MIN) {
        out.push(0xd2);
        out.extend_from_slice(&(v as i32).to_be_bytes());
    } else {
        out.push(0xd3);
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn write_str(out: &mut Vec<u8>, v: &str) -> Result<(), MsgPackEncodeError> {
    write_str_len(out, len32(v.len())?);
    out.extend_from_slice(v.as_bytes());
    Ok(())
}

struct Encoder<'a> {
    out: &'a mut Vec<u8>,
}

/// 列・マップの書き出し。長さが分からなければ `buffered` に書いてから、数えた要素の数を前に置く。
struct Compound<'a, 'b> {
    enc: &'a mut Encoder<'b>,
    buffered: Option<(Vec<u8>, u32)>,
}

impl<'a, 'b> Compound<'a, 'b> {
    fn new(
        enc: &'a mut Encoder<'b>,
        len: Option<usize>,
        write_len: fn(&mut Vec<u8>, u32),
    ) -> Result<Self, MsgPackEncodeError> {
        let buffered = match len {
            Some(len) => {
                write_len(enc.out, len32(len)?);
                None
            }
            None => Some((Vec::new(), 0)),
        };
        Ok(Self { enc, buffered })
    }

    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), MsgPackEncodeError> {
        match &mut self.buffered {
            None => value.serialize(&mut *self.enc),
            Some((buf, count)) => {
                value.serialize(&mut Encoder { out: buf })?;
                *count = count.saturating_add(1);
                Ok(())
            }
        }
    }

    /// 長さが分からなかったときは、`count` (要素の数) から見出しを書いて中身を写す。
    fn finish(self, write_len: fn(&mut Vec<u8>, u32), per_entry: u32) {
        if let Some((buf, count)) = self.buffered {
            write_len(self.enc.out, count / per_entry);
            self.enc.out.extend_from_slice(&buf);
        }
    }
}

impl ser::SerializeSeq for Compound<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish(write_array_len, 1);
        Ok(())
    }
}

impl ser::SerializeMap for Compound<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.element(key)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish(write_map_len, 2);
        Ok(())
    }
}

/// 長さの分かっている配列 (タプルなど) と、フィールド名付きのマップ (構造体)。
struct Fields<'a, 'b> {
    enc: &'a mut Encoder<'b>,
}

impl ser::SerializeTuple for Fields<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.enc)
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for Fields<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.enc)
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for Fields<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.enc)
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStruct for Fields<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        write_str(self.enc.out, key)?;
        value.serialize(&mut *self.enc)
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for Fields<'_, '_> {
    type Ok = ();
    type Error = MsgPackEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        write_str(self.enc.out, key)?;
        value.serialize(&mut *self.enc)
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, 'b> ser::Serializer for &'a mut Encoder<'b> {
    type Ok = ();
    type Error = MsgPackEncodeError;
    type SerializeSeq = Compound<'a, 'b>;
    type SerializeTuple = Fields<'a, 'b>;
    type SerializeTupleStruct = Fields<'a, 'b>;
    type SerializeTupleVariant = Fields<'a, 'b>;
    type SerializeMap = Compound<'a, 'b>;
    type SerializeStruct = Fields<'a, 'b>;
    type SerializeStructVariant = Fields<'a, 'b>;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bool(self, v: bool) -> Result<(), Self::Error> {
        self.out.push(if v { 0xc3 } else { 0xc2 });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<(), Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<(), Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<(), Self::Error> {
        write_sint(self.out, v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<(), Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<(), Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<(), Self::Error> {
        write_uint(self.out, v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<(), Self::Error> {
        self.out.push(0xca);
        self.out.extend_from_slice(&v.to_bits().to_be_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), Self::Error> {
        self.out.push(0xcb);
        self.out.extend_from_slice(&v.to_bits().to_be_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), Self::Error> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<(), Self::Error> {
        write_str(self.out, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), Self::Error> {
        write_bin_len(self.out, len32(v.len())?);
        self.out.extend_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<(), Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Self::Error> {
        self.out.push(0xc0);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> {
        write_array_len(self.out, 0);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<(), Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        write_map_len(self.out, 1);
        write_str(self.out, variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Compound::new(self, len, write_array_len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        write_array_len(self.out, len32(len)?);
        Ok(Fields { enc: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        write_map_len(self.out, 1);
        write_str(self.out, variant)?;
        self.serialize_tuple(len)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Compound::new(self, len, write_map_len)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        write_map_len(self.out, len32(len)?);
        Ok(Fields { enc: self })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        write_map_len(self.out, 1);
        write_str(self.out, variant)?;
        self.serialize_struct(name, len)
    }
}

// ---- 読み込み ----

/// MessagePack のバイト列から読む serde の `Deserializer`。値の後ろの余りのバイトは見ない。
///
/// 入れ子 (配列・マップ・拡張型) は `max_depth` 段より深いとエラーにする (`max_depth` 段目に入るところで失敗する。
/// スタックを使い切らないため)。
pub(crate) struct Decoder<'de> {
    input: &'de [u8],
    /// 先に読んだ値の先頭のバイト (`Option` の中身や enum の判別のために読み、まだ使っていないもの)。
    marker: Option<u8>,
    /// 残りの入れ子の段数。
    depth: usize,
}

impl<'de> Decoder<'de> {
    pub(crate) fn new(input: &'de [u8], max_depth: usize) -> Self {
        Self {
            input,
            marker: None,
            depth: max_depth,
        }
    }

    fn take(&mut self, len: usize) -> Result<&'de [u8], MsgPackDecodeError> {
        if len > self.input.len() {
            return Err(MsgPackDecodeError::UnexpectedEof);
        }
        let (head, rest) = self.input.split_at(len);
        self.input = rest;
        Ok(head)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], MsgPackDecodeError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn read_u8(&mut self) -> Result<u8, MsgPackDecodeError> {
        Ok(self.take_array::<1>()?[0])
    }

    fn read_u16(&mut self) -> Result<u16, MsgPackDecodeError> {
        Ok(u16::from_be_bytes(self.take_array()?))
    }

    fn read_u32(&mut self) -> Result<u32, MsgPackDecodeError> {
        Ok(u32::from_be_bytes(self.take_array()?))
    }

    fn read_u64(&mut self) -> Result<u64, MsgPackDecodeError> {
        Ok(u64::from_be_bytes(self.take_array()?))
    }

    fn take_marker(&mut self) -> Result<u8, MsgPackDecodeError> {
        match self.marker.take() {
            Some(marker) => Ok(marker),
            None => self.read_u8(),
        }
    }

    fn peek_marker(&mut self) -> Result<u8, MsgPackDecodeError> {
        let marker = self.take_marker()?;
        self.marker = Some(marker);
        Ok(marker)
    }

    /// 入れ子を 1 段深くして `f` を呼ぶ。
    fn nested<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, MsgPackDecodeError>,
    ) -> Result<T, MsgPackDecodeError> {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            return Err(MsgPackDecodeError::DepthLimitExceeded);
        }
        let result = f(self);
        self.depth += 1;
        result
    }

    /// 数・真偽値・nil を読む (`marker` は読んだ先頭のバイト)。ほかの形は [`MsgPackDecodeError::TypeMismatch`]。
    fn scalar<V: Visitor<'de>>(
        &mut self,
        marker: u8,
        visitor: V,
    ) -> Result<V::Value, MsgPackDecodeError> {
        match marker {
            0x00..=0x7f => visitor.visit_u8(marker),
            0xe0..=0xff => visitor.visit_i8(marker as i8),
            0xc0 => visitor.visit_unit(),
            0xc2 => visitor.visit_bool(false),
            0xc3 => visitor.visit_bool(true),
            0xcc => visitor.visit_u8(self.read_u8()?),
            0xcd => visitor.visit_u16(self.read_u16()?),
            0xce => visitor.visit_u32(self.read_u32()?),
            0xcf => visitor.visit_u64(self.read_u64()?),
            0xd0 => visitor.visit_i8(self.read_u8()? as i8),
            0xd1 => visitor.visit_i16(self.read_u16()? as i16),
            0xd2 => visitor.visit_i32(self.read_u32()? as i32),
            0xd3 => visitor.visit_i64(self.read_u64()? as i64),
            0xca => visitor.visit_f32(f32::from_bits(self.read_u32()?)),
            0xcb => visitor.visit_f64(f64::from_bits(self.read_u64()?)),
            _ => Err(MsgPackDecodeError::TypeMismatch(marker)),
        }
    }

    /// 長さの見出しを読む。`fix` は 1 バイトの形の範囲 (先頭, 下位ビットのマスク)、`markers` は 8・16・32bit の形。
    fn len_of(
        &mut self,
        marker: u8,
        fix: Option<(u8, u8)>,
        markers: [Option<u8>; 3],
    ) -> Result<Option<u32>, MsgPackDecodeError> {
        if let Some((base, mask)) = fix
            && marker & !mask == base
        {
            return Ok(Some(u32::from(marker & mask)));
        }
        Ok(match markers {
            [Some(m), _, _] if m == marker => Some(u32::from(self.read_u8()?)),
            [_, Some(m), _] if m == marker => Some(u32::from(self.read_u16()?)),
            [_, _, Some(m)] if m == marker => Some(self.read_u32()?),
            _ => None,
        })
    }

    /// 型を問わず次の値を読む。`allow_bytes` が偽なら、バイナリはバイトの列として渡す (列を求める型のため)。
    fn any<V: Visitor<'de>>(
        &mut self,
        visitor: V,
        allow_bytes: bool,
    ) -> Result<V::Value, MsgPackDecodeError> {
        let marker = self.take_marker()?;
        if let Some(len) = self.len_of(
            marker,
            Some((0xa0, 0x1f)),
            [Some(0xd9), Some(0xda), Some(0xdb)],
        )? {
            let bytes = self.take(len as usize)?;
            return match core::str::from_utf8(bytes) {
                Ok(s) => visitor.visit_borrowed_str(s),
                // UTF-8 でない文字列も、バイト列を受け取る型には渡す。
                Err(e) => visitor
                    .visit_borrowed_bytes::<MsgPackDecodeError>(bytes)
                    .map_err(|_| MsgPackDecodeError::Utf8(e)),
            };
        }
        if let Some(len) =
            self.len_of(marker, Some((0x90, 0x0f)), [None, Some(0xdc), Some(0xdd)])?
        {
            return self.nested(|de| {
                let mut access = Elements { de, left: len };
                let value = visitor.visit_seq(&mut access)?;
                match access.left {
                    0 => Ok(value),
                    left => Err(MsgPackDecodeError::TrailingElements(len - left)),
                }
            });
        }
        if let Some(len) =
            self.len_of(marker, Some((0x80, 0x0f)), [None, Some(0xde), Some(0xdf)])?
        {
            return self.nested(|de| {
                let mut access = Elements { de, left: len };
                let value = visitor.visit_map(&mut access)?;
                match access.left {
                    0 => Ok(value),
                    left => Err(MsgPackDecodeError::TrailingElements(len - left)),
                }
            });
        }
        if let Some(len) = self.len_of(marker, None, [Some(0xc4), Some(0xc5), Some(0xc6)])? {
            let bytes = self.take(len as usize)?;
            return if allow_bytes {
                visitor.visit_borrowed_bytes(bytes)
            } else {
                visitor.visit_seq(SeqDeserializer::new(bytes.iter().copied()))
            };
        }
        let ext_len = match marker {
            0xd4 => Some(1),
            0xd5 => Some(2),
            0xd6 => Some(4),
            0xd7 => Some(8),
            0xd8 => Some(16),
            _ => self.len_of(marker, None, [Some(0xc7), Some(0xc8), Some(0xc9)])?,
        };
        if let Some(len) = ext_len {
            // 拡張型は (型の番号 i8, 中身のバイト列) の 2 つの要素の列を中に持つ newtype として渡す。
            let tag = self.read_u8()? as i8;
            let data = self.take(len as usize)?;
            return self.nested(|_| {
                visitor.visit_newtype_struct(SeqAccessDeserializer::new(Extension {
                    tag: Some(tag),
                    data: Some(data),
                }))
            });
        }
        self.scalar(marker, visitor)
    }
}

/// 配列の要素とマップのキー・値 (マップは `left` をキーで数える)。
struct Elements<'a, 'de> {
    de: &'a mut Decoder<'de>,
    left: u32,
}

impl<'de> de::SeqAccess<'de> for Elements<'_, 'de> {
    type Error = MsgPackDecodeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        usize::try_from(self.left).ok()
    }
}

impl<'de> de::MapAccess<'de> for Elements<'_, 'de> {
    type Error = MsgPackDecodeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        usize::try_from(self.left).ok()
    }
}

/// 拡張型の中身 (型の番号と、中身のバイト列)。
struct Extension<'de> {
    tag: Option<i8>,
    data: Option<&'de [u8]>,
}

impl<'de> de::SeqAccess<'de> for Extension<'de> {
    type Error = MsgPackDecodeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if let Some(tag) = self.tag.take() {
            return seed.deserialize(I8Deserializer::new(tag)).map(Some);
        }
        match self.data.take() {
            Some(data) => seed
                .deserialize(BorrowedBytesDeserializer::new(data))
                .map(Some),
            None => Ok(None),
        }
    }
}

/// enum の値。`{名前: 中身}` のマップ (`map` が真) か、名前だけ (単位のバリアント)。
struct Variant<'a, 'de> {
    de: &'a mut Decoder<'de>,
    map: bool,
}

impl<'de> de::EnumAccess<'de> for Variant<'_, 'de> {
    type Error = MsgPackDecodeError;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self), Self::Error> {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for Variant<'_, 'de> {
    type Error = MsgPackDecodeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        if !self.map {
            return Ok(());
        }
        match self.de.take_marker()? {
            0xc0 => Ok(()),
            marker => Err(MsgPackDecodeError::TypeMismatch(marker)),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        if !self.map {
            return Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            ));
        }
        seed.deserialize(self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if !self.map {
            return Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            ));
        }
        self.de.any(visitor, false)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if !self.map {
            return Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            ));
        }
        self.de.any(visitor, false)
    }
}

/// 数・真偽値だけを読む `deserialize_*`。
macro_rules! scalar_methods {
    ($($method:ident)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
                let marker = self.take_marker()?;
                self.scalar(marker, visitor)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for &mut Decoder<'de> {
    type Error = MsgPackDecodeError;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.any(visitor, true)
    }

    scalar_methods! {
        deserialize_bool deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64
        deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64 deserialize_f32 deserialize_f64
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // nil でなければ先頭のバイトを戻して、中身として読み直す。
        match self.take_marker()? {
            0xc0 => visitor.visit_none(),
            marker => {
                self.marker = Some(marker);
                visitor.visit_some(self)
            }
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        // 単位構造体は空の配列で書く。nil も受け付ける。
        match self.take_marker()? {
            0xc0 | 0x90 => visitor.visit_unit(),
            marker => {
                self.marker = Some(marker);
                self.any(visitor, true)
            }
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.any(visitor, false)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.any(visitor, false)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.any(visitor, false)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.any(visitor, false)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let marker = self.peek_marker()?;
        match self.len_of(marker, Some((0x80, 0x0f)), [None, Some(0xde), Some(0xdf)])? {
            Some(1) => {
                self.marker = None;
                visitor.visit_enum(Variant {
                    de: self,
                    map: true,
                })
            }
            Some(len) => Err(MsgPackDecodeError::EnumMapLength(len)),
            // マップでなければ単位のバリアントの名前 (か番号)。先頭のバイトは名前を読むときに使う。
            None => visitor.visit_enum(Variant {
                de: self,
                map: false,
            }),
        }
    }

    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf unit map identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::*;

    fn rmp<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
        rmp_serde::to_vec_named(value).unwrap()
    }

    fn ours<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
        to_vec_named(value).unwrap()
    }

    /// `rmp-serde` (`from_read_ref`、上限 `depth`) で読んだ結果。
    fn rmp_decode<T: serde::de::DeserializeOwned>(bytes: &[u8], depth: usize) -> Result<T, String> {
        let mut de = rmp_serde::Deserializer::from_read_ref(bytes);
        de.set_max_depth(depth);
        T::deserialize(&mut de).map_err(|e| e.to_string())
    }

    fn our_decode<T: serde::de::DeserializeOwned>(bytes: &[u8], depth: usize) -> Result<T, String> {
        T::deserialize(&mut Decoder::new(bytes, depth)).map_err(|e| e.to_string())
    }

    /// 書き出しが rmp-serde と同じで、どちらで書いたものもどちらで読んでも同じ値になる。
    fn same<T>(value: &T)
    where
        T: Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
    {
        let bytes = rmp(value);
        assert_eq!(ours(value), bytes, "{value:?}");
        assert_eq!(
            our_decode::<T>(&bytes, 128).as_ref(),
            Ok(value),
            "{value:?}"
        );
    }

    #[test]
    fn integers_use_the_same_markers_as_rmp_serde() {
        let edges: [i64; 20] = [
            0,
            1,
            31,
            32,
            127,
            128,
            255,
            256,
            65535,
            65536,
            4_294_967_295,
            4_294_967_296,
            i64::MAX,
            -1,
            -32,
            -33,
            -128,
            -129,
            -32769,
            i64::MIN,
        ];
        for v in edges {
            same(&v);
            if let Ok(v) = i32::try_from(v) {
                same(&v);
            }
            if let Ok(v) = u64::try_from(v) {
                same(&v);
            }
            if let Ok(v) = u8::try_from(v) {
                same(&v);
            }
        }
        same(&u64::MAX);
    }

    #[test]
    fn floats_strings_and_containers_match_rmp_serde() {
        for v in [
            0.0f32,
            -0.0,
            1.5,
            f32::MAX,
            f32::MIN_POSITIVE,
            f32::INFINITY,
        ] {
            same(&v);
            same(&f64::from(v));
        }
        for len in [0usize, 1, 31, 32, 255, 256, 65535, 65536] {
            same(&"x".repeat(len));
            same(&vec![7u8; len]);
            same(&vec![0.25f32; len.min(70_000)]);
            let map: std::collections::BTreeMap<String, u8> = (0..len.min(70_000))
                .map(|i| (format!("{i:05}"), (i % 200) as u8))
                .collect();
            same(&map);
        }
        same(&'あ');
        same(&true);
        same(&(1u8, -2i16, "three".to_owned()));
        same(&Some(Some(3u32)));
        same(&None::<u32>);
        same(&serde_bytes_like());
    }

    fn serde_bytes_like() -> Vec<Vec<u8>> {
        vec![vec![], vec![1, 2, 3], vec![200; 300]]
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Unit;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Newtype(u16);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Pair(f32, String);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum External {
        A,
        B(u8),
        C(u8, String),
        D { x: f32, y: Option<i32> },
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Internal {
        Linear { at: f32 },
        Step,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Number(u32),
        Text(String),
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Inner {
        a: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        b: Option<String>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Outer {
        unit: Unit,
        newtype: Newtype,
        pair: Pair,
        external: Vec<External>,
        internal: Vec<Internal>,
        untagged: Vec<Untagged>,
        #[serde(flatten)]
        inner: Inner,
        nothing: (),
    }

    #[test]
    fn serde_shapes_match_rmp_serde() {
        same(&Outer {
            unit: Unit,
            newtype: Newtype(300),
            pair: Pair(0.5, "p".to_owned()),
            external: vec![
                External::A,
                External::B(3),
                External::C(4, "c".to_owned()),
                External::D {
                    x: 1.0,
                    y: Some(-5),
                },
            ],
            internal: vec![Internal::Linear { at: 2.0 }, Internal::Step],
            untagged: vec![Untagged::Number(9), Untagged::Text("t".to_owned())],
            inner: Inner {
                a: 1,
                b: Some("b".to_owned()),
            },
            nothing: (),
        });
        same(&Inner { a: 2, b: None });
    }

    /// 書き出しだけでは作られない形 (拡張型・バイナリ・`{名前: nil}` の単位のバリアント・nil の単位構造体など) も、
    /// 読み込みの結果 (値かエラーか) が rmp-serde と同じ。
    #[test]
    fn decoding_unusual_shapes_matches_rmp_serde() {
        let cases: Vec<Vec<u8>> = vec![
            vec![0x81, 0xa1, b'A', 0xc0],                   // {A: nil}
            vec![0x81, 0xa1, b'A', 0x01],                   // {A: 1}
            vec![0x82, 0xa1, b'A', 0xc0, 0xa1, b'B', 0x01], // 2 要素のマップ
            vec![0x00],                                     // 番号の単位のバリアント
            vec![0xa1, b'B'],                               // 中身の要る単位のバリアント
            vec![0xc0],
            vec![0x90],
            vec![0x91, 0x01],
            vec![0xd4, 0x05, 0xff],             // fixext1
            vec![0xc7, 0x02, 0x05, 0x01, 0x02], // ext8
            vec![0xc4, 0x02, 0x01, 0x02],       // bin8
            vec![0xa2, 0xff, 0xfe],             // UTF-8 でない文字列
            vec![0x92, 0x01],                   // 切り詰め
            vec![0xc1],                         // 予約
            vec![0xdc, 0x00, 0x01, 0xca, 0x3f, 0x80, 0x00, 0x00],
        ];
        for bytes in &cases {
            assert_eq!(
                our_decode::<Value>(bytes, 128).ok(),
                rmp_decode::<Value>(bytes, 128).ok(),
                "{bytes:02x?}"
            );
            assert_eq!(
                our_decode::<External>(bytes, 128).ok(),
                rmp_decode::<External>(bytes, 128).ok(),
                "{bytes:02x?}"
            );
            assert_eq!(
                our_decode::<Unit>(bytes, 128).ok(),
                rmp_decode::<Unit>(bytes, 128).ok(),
                "{bytes:02x?}"
            );
            assert_eq!(
                our_decode::<Vec<u8>>(bytes, 128).ok(),
                rmp_decode::<Vec<u8>>(bytes, 128).ok(),
                "{bytes:02x?}"
            );
            assert_eq!(
                our_decode::<Option<f32>>(bytes, 128).ok(),
                rmp_decode::<Option<f32>>(bytes, 128).ok(),
                "{bytes:02x?}"
            );
            assert_eq!(
                our_decode::<serde::de::IgnoredAny>(bytes, 128).is_ok(),
                rmp_decode::<serde::de::IgnoredAny>(bytes, 128).is_ok(),
                "{bytes:02x?}"
            );
        }
    }

    #[test]
    fn depth_limit_matches_rmp_serde() {
        for depth in [1usize, 2, 3, 127, 128, 129] {
            let mut bytes = vec![0x91; depth];
            bytes.push(0x01);
            for max in [1usize, 2, 3, 128] {
                assert_eq!(
                    our_decode::<serde::de::IgnoredAny>(&bytes, max).is_ok(),
                    rmp_decode::<serde::de::IgnoredAny>(&bytes, max).is_ok(),
                    "depth {depth}, max {max}"
                );
            }
        }
    }

    /// 壊したバイト列でも、読み込みの結果が rmp-serde と同じ (パニックもしない)。
    #[test]
    fn corrupted_payloads_decode_like_rmp_serde() {
        let original = rmp(&json!({
            "name": "x",
            "values": [1, -2, 3.5, null, true, {"nested": ["a", "b", {"deep": [1, 2, 3]}]}],
            "long": "y".repeat(40),
        }));
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..5000 {
            let mut bytes = original.clone();
            let at = next() as usize % bytes.len();
            match next() % 3 {
                0 => bytes[at] = next() as u8,
                1 => bytes.truncate(at),
                _ => bytes.insert(at, next() as u8),
            }
            assert_eq!(
                our_decode::<Value>(&bytes, 128).ok(),
                rmp_decode::<Value>(&bytes, 128).ok(),
                "{bytes:02x?}"
            );
        }
    }

    /// `T` として、rmp-serde と同じ値に読め、書き直したバイト列も rmp-serde と同じ。
    fn same_payload<T>(payload: &[u8], what: &str)
    where
        T: Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
    {
        let expected = rmp_decode::<T>(payload, 128);
        let actual = our_decode::<T>(payload, 128);
        assert_eq!(actual.as_ref().ok(), expected.as_ref().ok(), "{what}");
        // 未知の値 (`Unknown`) を持つ値はどちらも書き出せない。
        if let Ok(value) = expected {
            assert_eq!(
                to_vec_named(&value).ok(),
                rmp_serde::to_vec_named(&value).ok(),
                "{what}"
            );
        }
    }

    /// `assets/` のすべてのバンドルとモジュールの MessagePack のチャンクを、rmp-serde と同じに読み書きする
    /// (`assets/samples` と互換性テストの `assets/compat`)。
    #[test]
    fn asset_payloads_match_rmp_serde() {
        use ugoe_model::Model;
        use ugoe_model::expression::Expression;
        use ugoe_model::module::Module;
        use ugoe_model::motion::Motion;
        use ugoe_model::physics::PhysicsSettings;

        use crate::bundle::{ChunkTag, Payloads, ReadLimits, read_table};
        use crate::features::FeatureTable;

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut dirs = vec![root.join("samples"), root.join("compat")];
        let mut chunks = 0;
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if !matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("ugoe" | "ugoemod")
                ) {
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                let mut payloads = Payloads::new(&bytes, ReadLimits::default());
                for (i, entry) in read_table(&bytes).unwrap().iter().enumerate() {
                    let what = format!("{} #{i} {}", path.display(), entry.tag);
                    let msgpack = [
                        ChunkTag::MODL,
                        ChunkTag::MODU,
                        ChunkTag::PHYS,
                        ChunkTag::MOTN,
                        ChunkTag::EXPR,
                        ChunkTag::FEAT,
                    ];
                    if !msgpack.contains(&entry.tag) {
                        continue;
                    }
                    let payload = payloads.get(entry).unwrap();
                    match entry.tag {
                        ChunkTag::MODL => same_payload::<Model>(&payload, &what),
                        ChunkTag::MODU => same_payload::<Module>(&payload, &what),
                        ChunkTag::PHYS => same_payload::<PhysicsSettings>(&payload, &what),
                        ChunkTag::MOTN => same_payload::<Motion>(&payload, &what),
                        ChunkTag::EXPR => same_payload::<Expression>(&payload, &what),
                        ChunkTag::FEAT => same_payload::<FeatureTable>(&payload, &what),
                        _ => continue,
                    }
                    chunks += 1;
                }
            }
        }
        assert!(chunks > 30, "{chunks} chunks");
    }
}
