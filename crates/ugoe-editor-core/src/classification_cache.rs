//! PSD のレイヤーの分類の答えのキャッシュ (`.ugoe-cache/classification.json`)。
//! 仕様: `docs/spec/file-formats.md#分類のキャッシュ`、設計: `docs/editor.md#分類の答えのキャッシュ`。
//!
//! 分類の答え (名前の規則と AI の答えのラベル・左右・確からしさ・差分の組) を、`project.json` を変えずにプロジェクトの
//! 中の別のファイルに残す (T0567)。開き直しても分類し直さずに (AI を呼び直さずに) 済むように。
//!
//! - キャッシュなので Undo の対象にせず、`Document` にも持たない。読めない・形の違う・版の違うファイルは無いものとする。
//! - 分類した PSD のパスと識別 ([`PsdIdentity`]。大きさと中身のハッシュ) を添え、今の PSD と合わなければ使わない
//!   (合うかは使う側が PSD を読んで [`PsdIdentity::matches`] で確かめる)。
//! - このクレートは `ugoe-ai` に依存できないので、レイヤーの答えの型 (`ugoe_ai::classify::LayerLabel`) は型引数 `L` で
//!   受け取り、そのまま JSON にする。答えが PSD のレイヤーと合うか (数・パス・確からしさ) も使う側が確かめる。
//! - 書くときは `ugoe_fs::write_atomic` (中身が同じなら書かない)、読むときは通常のファイルだけを上限
//!   ([`MAX_CLASSIFICATION_CACHE_BYTES`]) までで読む (T1607・T0025)。エディタが書き、エディタと MCP サーバが読む。

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::project::{ProjectError, normalize_path, read_regular_file, to_json, write_if_changed};

/// キャッシュを置くディレクトリ (プロジェクトの `.gitignore` に書く。保存・読み込み・指紋では触れない)。
pub const CACHE_DIR: &str = ".ugoe-cache";
/// 分類の答えのキャッシュのファイル名 ([`CACHE_DIR`] の中)。
pub const CLASSIFICATION_CACHE_FILE: &str = "classification.json";
/// キャッシュの形の版。形を変えたら上げる (違う版のファイルは無いものとする)。
pub const CLASSIFICATION_CACHE_VERSION: u32 = 1;
/// キャッシュのファイルの大きさの上限 (バイト)。これより大きいファイルは読まない。
pub const MAX_CLASSIFICATION_CACHE_BYTES: u64 = 64 * 1024 * 1024;
/// レイヤーの数の上限。これより多いファイルは無いものとする。
pub const MAX_CACHED_LAYERS: usize = 65_536;

/// PSD の識別: 大きさと、中身の FNV-1a (64 ビット) のハッシュ。ファイルに残すので、Rust の版で変わりうる
/// `DefaultHasher` は使わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsdIdentity {
    /// バイト数。
    pub size: u64,
    /// 中身の FNV-1a (64 ビット)。
    pub hash: u64,
}

impl PsdIdentity {
    /// PSD のバイト列 `bytes` の識別。
    pub fn of(bytes: &[u8]) -> Self {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let hash = bytes
            .iter()
            .fold(OFFSET, |hash, b| (hash ^ u64::from(*b)).wrapping_mul(PRIME));
        Self {
            size: bytes.len() as u64,
            hash,
        }
    }

    /// `bytes` がこの識別の PSD か (大きさが違えばハッシュを求めない)。
    pub fn matches(&self, bytes: &[u8]) -> bool {
        self.size == bytes.len() as u64 && *self == Self::of(bytes)
    }
}

/// 分類の答えのキャッシュの中身。`L` はレイヤー 1 枚の答え (`ugoe_ai::classify::LayerLabel`)。
#[derive(Debug, Clone, PartialEq)]
pub struct CachedClassification<L> {
    /// 分類した PSD の絶対パス。
    pub psd: PathBuf,
    /// 分類した PSD の識別。
    pub identity: PsdIdentity,
    /// AI で分類したか (偽なら名前の規則だけ)。
    pub by_ai: bool,
    /// AI で分類したときに添えた注文。
    pub instructions: Option<String>,
    /// レイヤーごとの答え (`ugoe_ai::classify::classify_by_name` と同じ並び)。
    pub layers: Vec<L>,
}

/// ファイルの形 (`Layers` は書くときは借りた列、読むときは `Vec`)。
#[derive(Serialize, Deserialize)]
struct Stored<Layers> {
    version: u32,
    psd: StoredPsd,
    by_ai: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    layers: Layers,
}

#[derive(Serialize, Deserialize)]
struct StoredPsd {
    /// プロジェクトの中なら、プロジェクトのディレクトリからの相対パス (`/` 区切り)。外なら絶対パス。
    path: String,
    size: u64,
    /// FNV-1a (64 ビット) の 16 桁の 16 進。
    hash: String,
}

/// プロジェクトのディレクトリ `dir` の、分類の答えのキャッシュのパス。
pub fn classification_cache_path(dir: &Path) -> PathBuf {
    dir.join(CACHE_DIR).join(CLASSIFICATION_CACHE_FILE)
}

/// プロジェクトのディレクトリ `dir` に、分類の答えのキャッシュを書く (中身が同じなら書かない)。
pub fn write_classification_cache<L: Serialize>(
    dir: &Path,
    cache: &CachedClassification<L>,
) -> Result<(), ProjectError> {
    let root = normalize_path(dir);
    let psd = normalize_path(&cache.psd);
    let path = match psd.strip_prefix(&root) {
        Ok(inside) => inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => psd.display().to_string(),
    };
    let stored = Stored {
        version: CLASSIFICATION_CACHE_VERSION,
        psd: StoredPsd {
            path,
            size: cache.identity.size,
            hash: format!("{:016x}", cache.identity.hash),
        },
        by_ai: cache.by_ai,
        instructions: cache.instructions.clone(),
        layers: cache.layers.as_slice(),
    };
    let file = classification_cache_path(dir);
    let bytes = to_json(&file, &stored)?;
    write_if_changed(&file, &bytes)
}

/// プロジェクトのディレクトリ `dir` の、分類の答えのキャッシュを読む。
///
/// ファイルがない・JSON として読めない・形や版が違う・レイヤーが多すぎるときは `Ok(None)` (キャッシュは無いものとする)。
/// 通常のファイルでない・上限より大きい・読めないときはエラー。PSD のパスは絶対パスにして返す。
pub fn read_classification_cache<L: DeserializeOwned>(
    dir: &Path,
) -> Result<Option<CachedClassification<L>>, ProjectError> {
    let file = classification_cache_path(dir);
    let bytes = match read_regular_file(&file, MAX_CLASSIFICATION_CACHE_BYTES) {
        Ok(bytes) => bytes,
        Err(ProjectError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let Ok(stored) = serde_json::from_slice::<Stored<Vec<L>>>(&bytes) else {
        return Ok(None);
    };
    if stored.version != CLASSIFICATION_CACHE_VERSION || stored.layers.len() > MAX_CACHED_LAYERS {
        return Ok(None);
    }
    let Ok(hash) = u64::from_str_radix(&stored.psd.hash, 16) else {
        return Ok(None);
    };
    if stored.psd.path.is_empty() {
        return Ok(None);
    }
    let psd = Path::new(&stored.psd.path);
    let psd = if psd.is_absolute() {
        normalize_path(psd)
    } else {
        normalize_path(&dir.join(psd))
    };
    Ok(Some(CachedClassification {
        psd,
        identity: PsdIdentity {
            size: stored.psd.size,
            hash,
        },
        by_ai: stored.by_ai,
        instructions: stored.instructions,
        layers: stored.layers,
    }))
}
