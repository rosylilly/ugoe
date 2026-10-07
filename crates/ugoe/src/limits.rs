//! 読み込みの上限 (`docs/spec/file-formats.md#読み込みの上限`)。
//!
//! 上限はファイル形式の決まりではなく、読む側の方針。SDK では `ugoe::format::read_bundle_with` /
//! `ugoe::format::module_file::read_module_with` に [`ReadLimits`] を渡して指定する (既定値で読む `read_bundle` などはそのまま)。
//! 上限を超えたときの種類 (`code`) と補足の値はここで決め、wasm と C ABI が同じ値を返す (T0152)。

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ugoe_format::{ChunkTag, FormatError};

pub use ugoe_format::{ReadLimit, ReadLimits};

/// 読み込みの上限 ([`ReadLimits`]) を超えたので、読み込みをやめた (`FormatError::ReadLimitExceeded` の中身)。
///
/// ファイルが壊れているとは限らない (上限を上げれば読めることがある)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimitExceeded {
    /// 展開をやめたチャンクのタグ。
    pub chunk: ChunkTag,
    /// どの上限を超えたか。
    pub limit: ReadLimit,
    /// 超えた上限の値 (バイト)。
    pub max: u64,
}

impl ReadLimitExceeded {
    /// 機械が読む種類。wasm の読み込みのエラーの `code` と C ABI の `ugoe_error_kind` の値。変えない。
    pub const CODE: &'static str = "read_limit_exceeded";

    /// `error` が読み込みの上限を超えたものなら、その中身。
    pub fn from_error(error: &FormatError) -> Option<Self> {
        match *error {
            FormatError::ReadLimitExceeded { tag, limit, max } => Some(Self {
                chunk: tag,
                limit,
                max,
            }),
            _ => None,
        }
    }

    /// 補足の値 (キーと値の文字列)。キーは `limit` (`chunk_decompressed` | `total_decompressed`)・
    /// `chunk` (チャンクのタグ。`TEXR` など)・`max` (上限のバイト数、10 進)。
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        vec![
            ("limit", self.limit.as_str().to_owned()),
            ("chunk", self.chunk.to_string()),
            ("max", self.max.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use ugoe_format::{
        Bundle, TextureData, TextureFormat, WriteOptions, read_bundle, read_bundle_with,
        write_bundle_with,
    };
    use ugoe_model::fixtures;

    use super::*;

    /// SDK の利用者は `ugoe::format` の `*_with` で上限を指定し、超えたら種類と補足の値を得られる。
    #[test]
    fn small_limits_reject_compressed_bundles() {
        let mut bundle = Bundle::new(fixtures::single_quad());
        // 画像の中身は読まないので、PNG の署名だけのバイト列でよい。
        bundle.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        }];
        let bytes = write_bundle_with(&bundle, WriteOptions { compress: true }).unwrap();
        assert_eq!(read_bundle(&bytes).unwrap(), bundle);

        let limits = ReadLimits::default().with_max_total_decompressed(1);
        let error = read_bundle_with(&bytes, limits).unwrap_err();
        let exceeded = ReadLimitExceeded::from_error(&error).unwrap();
        assert_eq!(exceeded.limit, ReadLimit::TotalDecompressed);
        assert_eq!(exceeded.max, 1);
        assert_eq!(
            exceeded.fields(),
            [
                ("limit", "total_decompressed".to_owned()),
                ("chunk", exceeded.chunk.to_string()),
                ("max", "1".to_owned()),
            ]
        );
        assert!(ReadLimitExceeded::from_error(&FormatError::TooLarge).is_none());
    }

    /// モジュールも同じく `ugoe::format::module_file::read_module_with` で上限を指定する。
    #[test]
    fn small_limits_reject_compressed_modules() {
        use ugoe_format::module_file::{ModuleFile, read_module_with, write_module_with};

        let mut file = ModuleFile::new(fixtures::long_hair_module());
        file.textures = vec![TextureData {
            format: TextureFormat::Png,
            bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        }];
        let bytes = write_module_with(&file, WriteOptions { compress: true }).unwrap();
        assert!(read_module_with(&bytes, ReadLimits::default()).is_ok());
        let limits = ReadLimits::default().with_max_chunk_decompressed(1);
        let error = read_module_with(&bytes, limits).unwrap_err();
        let exceeded = ReadLimitExceeded::from_error(&error).unwrap();
        assert_eq!(exceeded.limit, ReadLimit::ChunkDecompressed);
        assert_eq!(
            exceeded.fields()[0],
            ("limit", "chunk_decompressed".to_owned())
        );
    }
}
