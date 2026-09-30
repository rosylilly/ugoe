//! Claude Desktop などに入れる MCP Bundle (`.mcpb`): `ugoe-mcp --editor` (実行中のエディタへの中継) を包む。
//!
//! `.mcpb` は zip で、`manifest.json` と、中で動かすサーバ (ここでは `server/ugoe-mcp[.exe]`) を持つ。
//! Claude Desktop は `.mcpb` を開く (ダブルクリック・ウィンドウへのドロップ・「Settings > Extensions >
//! Advanced settings > Install Extension…」) とインストールの画面を出し、展開したディレクトリで
//! `mcp_config` のコマンドを stdio のサーバとして起動する。
//!
//! 仕様 (2026-09 に確かめた。版は `manifest_version` 0.3 = 公式の `mcpb` の CLI の既定):
//! - <https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md>・
//!   `schemas/mcpb-manifest-v0.3.schema.json`・`src/node/validate.ts`・`src/cli/pack.ts`
//! - <https://claude.com/docs/connectors/building/mcpb>
//!
//! 決めたこと:
//! - `server.type` は `binary`。1 つの `.mcpb` に 1 つの OS の実行ファイルだけを入れ、`compatibility.platforms`
//!   をその OS にする (エディタは自分の OS の `ugoe-mcp` を同梱しているので、それを包む)。
//! - `mcp_config.command` は `${__dirname}/server/ugoe-mcp` (Windows は `.exe` まで書く。仕様の
//!   `platform_overrides` の例と同じ)。`args` は `--editor` だけ (ポートとトークンは発見用のファイルから読むので、
//!   `user_config` は要らない)。`mcp_config` で使える変数は `${__dirname}`・`${/}`・`${pathSeparator}`・
//!   `${user_config.*}` だけ (公式の validate がほかを誤りにする)。
//! - `tools` は [`Tools::list`] (エディタのブリッジと同じ既定の設定) から作る。説明には英語の `title` を使う
//!   (インストールの画面に出るので)。
//! - zip の中の実行ファイルには Unix の権限 0755 を付ける (公式の `pack` も同じ。付けないと macOS・Linux で
//!   起動できない)。圧縮は deflate (`miniz_oxide`。ugoe-psd と同じ)。時刻は固定 (1980-01-01) にし、同じ入力から同じバイト列を作る。

use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::tools::{Options, Tools};

/// 書く `manifest_version`。
pub const MANIFEST_VERSION: &str = "0.3";
/// manifest の `name` (Claude Desktop がサーバを区別する名前)。
pub const NAME: &str = "ugoe-editor";
/// 利用者向けの操作マニュアル (Claude Desktop の節)。
pub const DOCUMENTATION: &str = "https://github.com/rosylilly/ugoe/blob/main/docs/manual/mcp.md";

/// `.mcpb` が動く OS (manifest の `compatibility.platforms` の値)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS (`darwin`)。
    Darwin,
    /// Windows (`win32`)。
    Win32,
    /// Linux (`linux`)。
    Linux,
}

impl Platform {
    /// この `ugoe-mcp` をビルドした OS。macOS・Windows・Linux のほかは `None`。
    pub fn current() -> Option<Self> {
        if cfg!(target_os = "macos") {
            Some(Self::Darwin)
        } else if cfg!(windows) {
            Some(Self::Win32)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }

    /// `compatibility.platforms` の値。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Darwin => "darwin",
            Self::Win32 => "win32",
            Self::Linux => "linux",
        }
    }

    /// 束の中の実行ファイルのパス (`entry_point`)。
    pub fn entry_point(self) -> &'static str {
        match self {
            Self::Win32 => "server/ugoe-mcp.exe",
            Self::Darwin | Self::Linux => "server/ugoe-mcp",
        }
    }
}

/// `.mcpb` を作れない。
#[derive(Debug, thiserror::Error)]
pub enum McpbError {
    #[error("failed to read {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to write {}: {source}", .path.display())]
    Write { path: PathBuf, source: io::Error },
    #[error("the icon must be a PNG file")]
    IconNotPng,
    #[error("{0} is too large for a .mcpb (4 GiB at most)")]
    TooLarge(String),
}

/// `.mcpb` の中身。
#[derive(Debug, Clone)]
pub struct Bundle {
    /// 包む `ugoe-mcp` の実行ファイル ([`Bundle::platform`] 向けのもの)。
    pub binary: PathBuf,
    pub platform: Platform,
    /// 束のアイコン (PNG。512×512 を勧める)。
    pub icon: Option<Vec<u8>>,
    /// 束の版 (semver)。既定は `ugoe-mcp` の版。
    pub version: String,
}

impl Bundle {
    /// この OS 向けに `binary` を包む束 (アイコンなし)。
    pub fn new(binary: impl Into<PathBuf>, platform: Platform) -> Self {
        Self {
            binary: binary.into(),
            platform,
            icon: None,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    /// `manifest.json` の中身。
    pub fn manifest(&self) -> Value {
        let entry_point = self.platform.entry_point();
        let tools: Vec<Value> = Tools::new(Options::default())
            .list()
            .iter()
            .map(|tool| {
                let description = tool["title"].as_str().or(tool["description"].as_str());
                json!({ "name": tool["name"], "description": description })
            })
            .collect();
        let mut manifest = json!({
            "manifest_version": MANIFEST_VERSION,
            "name": NAME,
            "display_name": "ugoe editor",
            "version": self.version,
            "description": "Let Claude work on the model open in the running ugoe editor. Proposals appear in \
                            the editor's AI tab; applying, undoing and saving ask you in the editor first.",
            "long_description": "Connects Claude to the ugoe editor running on this computer through its \
                                 MCP bridge (ugoe-mcp --editor). Turn it on in the editor under Settings > \
                                 Connect agents and save the project first; Claude then reads the open \
                                 model, renders it, and proposes edits and rigs. Nothing changes until you \
                                 approve it in the editor. The bridge listens on 127.0.0.1 only.",
            "author": { "name": "ugoe contributors", "url": "https://github.com/rosylilly/ugoe" },
            "repository": { "type": "git", "url": "https://github.com/rosylilly/ugoe.git" },
            "homepage": "https://github.com/rosylilly/ugoe",
            "documentation": DOCUMENTATION,
            "support": "https://github.com/rosylilly/ugoe/issues",
            "server": {
                "type": "binary",
                "entry_point": entry_point,
                "mcp_config": {
                    "command": format!("${{__dirname}}/{entry_point}"),
                    "args": ["--editor"],
                    "env": {},
                },
            },
            "tools": tools,
            "tools_generated": false,
            "keywords": ["ugoe", "live2d", "rigging", "animation", "editor"],
            "license": "MIT",
            "compatibility": { "platforms": [self.platform.as_str()] },
        });
        if self.icon.is_some() {
            manifest["icon"] = json!("icon.png");
        }
        manifest
    }

    /// `.mcpb` (zip) のバイト列を作る。
    pub fn to_bytes(&self) -> Result<Vec<u8>, McpbError> {
        let binary = std::fs::read(&self.binary).map_err(|source| McpbError::Read {
            path: self.binary.clone(),
            source,
        })?;
        let mut manifest = serde_json::to_vec_pretty(&self.manifest()).unwrap_or_default();
        manifest.push(b'\n');
        let mut zip = ZipWriter::default();
        zip.add("manifest.json", &manifest, 0o644)?;
        if let Some(icon) = &self.icon {
            if !icon.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(McpbError::IconNotPng);
            }
            zip.add("icon.png", icon, 0o644)?;
        }
        zip.add(self.platform.entry_point(), &binary, 0o755)?;
        zip.finish()
    }

    /// `.mcpb` を `out` に書く (一時ファイルに書いてから名前を変える)。
    pub fn write(&self, out: &Path) -> Result<(), McpbError> {
        let bytes = self.to_bytes()?;
        let write_error = |source| McpbError::Write {
            path: out.to_path_buf(),
            source,
        };
        if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(write_error)?;
        }
        let mut tmp = out.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, &bytes).map_err(write_error)?;
        std::fs::rename(&tmp, out).map_err(|source| {
            let _ = std::fs::remove_file(&tmp);
            write_error(source)
        })
    }
}

/// zip の CRC-32 (IEEE 802.3。多項式 0xEDB88320 の反転形)。
fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 == 1 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    !data
        .iter()
        .fold(!0u32, |c, &b| TABLE[usize::from((c as u8) ^ b)] ^ (c >> 8))
}

/// 最小の zip の書き手 (deflate・Zip64 なし・固定の時刻)。
#[derive(Default)]
struct ZipWriter {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

/// 1980-01-01 00:00 (MS-DOS の日付の最小)。
const DOS_DATE: u16 = (1 << 5) | 1;
const DOS_TIME: u16 = 0;
/// 作った OS (Unix。外部属性の上位 16 ビットを Unix の mode として読ませる) と版 2.0。
const VERSION_MADE_BY: u16 = (3 << 8) | 20;
const VERSION_NEEDED: u16 = 20;
const DEFLATE: u16 = 8;
/// 汎用のフラグ: 名前は UTF-8 (ビット 11)。データ記述子 (ビット 3) は使わない (大きさと CRC はヘッダに書く)。
const FLAGS: u16 = 1 << 11;
/// 通常のファイル (`S_IFREG`)。
const S_IFREG: u32 = 0o100_000;

impl ZipWriter {
    fn add(&mut self, name: &str, data: &[u8], mode: u32) -> Result<(), McpbError> {
        let too_large = || McpbError::TooLarge(name.to_owned());
        let crc = crc32(data);
        let compressed = miniz_oxide::deflate::compress_to_vec(data, 6);
        let offset = u32::try_from(self.out.len()).map_err(|_| too_large())?;
        let compressed_size = u32::try_from(compressed.len()).map_err(|_| too_large())?;
        let size = u32::try_from(data.len()).map_err(|_| too_large())?;
        let name = name.as_bytes();
        let name_len = u16::try_from(name.len()).map_err(|_| too_large())?;
        self.count = self.count.checked_add(1).ok_or_else(too_large)?;
        let sizes = |buf: &mut Vec<u8>| {
            put32(buf, crc);
            put32(buf, compressed_size);
            put32(buf, size);
            put16(buf, name_len);
            put16(buf, 0); // 拡張フィールド
        };
        // ローカルファイルヘッダ
        put32(&mut self.out, 0x0403_4b50);
        put16(&mut self.out, VERSION_NEEDED);
        put16(&mut self.out, FLAGS);
        put16(&mut self.out, DEFLATE);
        put16(&mut self.out, DOS_TIME);
        put16(&mut self.out, DOS_DATE);
        sizes(&mut self.out);
        self.out.extend_from_slice(name);
        self.out.extend_from_slice(&compressed);
        // 中央ディレクトリの項目
        let c = &mut self.central;
        put32(c, 0x0201_4b50);
        put16(c, VERSION_MADE_BY);
        put16(c, VERSION_NEEDED);
        put16(c, FLAGS);
        put16(c, DEFLATE);
        put16(c, DOS_TIME);
        put16(c, DOS_DATE);
        sizes(c);
        put16(c, 0); // コメント
        put16(c, 0); // ディスク番号
        put16(c, 0); // 内部属性
        put32(c, (S_IFREG | mode) << 16);
        put32(c, offset);
        c.extend_from_slice(name);
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<u8>, McpbError> {
        let too_large = || McpbError::TooLarge("the bundle".to_owned());
        let offset = u32::try_from(self.out.len()).map_err(|_| too_large())?;
        let size = u32::try_from(self.central.len()).map_err(|_| too_large())?;
        self.out.append(&mut self.central);
        put32(&mut self.out, 0x0605_4b50);
        put16(&mut self.out, 0);
        put16(&mut self.out, 0);
        put16(&mut self.out, self.count);
        put16(&mut self.out, self.count);
        put32(&mut self.out, size);
        put32(&mut self.out, offset);
        put16(&mut self.out, 0);
        Ok(self.out)
    }
}

fn put16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn put32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn manifest_runs_the_relay_for_one_platform() {
        let windows = Bundle::new("ugoe-mcp.exe", Platform::Win32).manifest();
        assert_eq!(windows["manifest_version"], MANIFEST_VERSION);
        assert_eq!(windows["server"]["type"], "binary");
        assert_eq!(windows["server"]["entry_point"], "server/ugoe-mcp.exe");
        assert_eq!(
            windows["server"]["mcp_config"]["command"],
            "${__dirname}/server/ugoe-mcp.exe"
        );
        assert_eq!(windows["server"]["mcp_config"]["args"], json!(["--editor"]));
        assert_eq!(windows["compatibility"]["platforms"], json!(["win32"]));
        assert!(windows.get("icon").is_none());

        let mac = Bundle::new("ugoe-mcp", Platform::Darwin).manifest();
        assert_eq!(mac["server"]["entry_point"], "server/ugoe-mcp");
        assert_eq!(mac["compatibility"]["platforms"], json!(["darwin"]));
    }

    #[test]
    fn manifest_lists_the_tools_the_editor_serves() {
        let manifest = Bundle::new("ugoe-mcp", Platform::Linux).manifest();
        let names: Vec<&str> = manifest["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        let expected: Vec<Value> = Tools::new(Options::default()).list();
        assert_eq!(names.len(), expected.len());
        for (name, tool) in names.iter().zip(&expected) {
            assert_eq!(*name, tool["name"]);
        }
        for name in [
            "propose_edit",
            "apply_proposal",
            "save_project",
            "render_png",
        ] {
            assert!(names.contains(&name), "{name} missing from {names:?}");
        }
        for tool in manifest["tools"].as_array().unwrap() {
            // 公式の schema: `tools` の項目は name と description だけ
            assert_eq!(tool.as_object().unwrap().len(), 2, "{tool}");
            assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
    }

    #[test]
    fn rejects_an_icon_that_is_not_png() {
        let dir = std::env::temp_dir().join(format!("ugoe-mcpb-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("ugoe-mcp");
        std::fs::write(&binary, b"binary").unwrap();
        let mut bundle = Bundle::new(&binary, Platform::Linux);
        bundle.icon = Some(b"GIF89a".to_vec());
        assert!(matches!(bundle.to_bytes(), Err(McpbError::IconNotPng)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
