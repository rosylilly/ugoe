//! プロジェクトのファイルの指紋。エディタの外 (`ugoe-mcp`・別のエディタ・手での編集) でプロジェクトが
//! 書き換えられたことに気づくために使う。設計: `docs/editor.md#外での書き換え`。
//!
//! - ファイルの監視 (OS の通知) は使わない。WSL 越しのパス (`\\wsl.localhost\...`) やネットワークのドライブでは
//!   通知が届かないことが多いため。代わりに、開いた (保存した) ときの指紋と、いまのディスクの指紋を比べる。
//! - 指紋は `Project::load` が読むファイル (`project.json`・`model.json`・`module.json`・`physics.json`・モーション・表情・
//!   `textures/*.png`) の、中身のハッシュと大きさ。更新日時だけが変わった (中身が同じ) ファイルは変更としない。
//! - モジュールのプロジェクトが参照する本体のファイルは含めない。モジュールのプロジェクトは本体を書かず、
//!   開き直せばそのときの本体と合成し直すので、本体が変わっても保存した内容は失われない (下敷きが古いだけ)。
//!   本体の変化には、本体の指紋 ([`ProjectFingerprint::read_base`]) を別に取って気づく (`docs/editor.md#本体の変化`)。
//! - PNG は大きいので、大きさと更新日時が前の指紋と同じなら前のハッシュを使い回す ([`ProjectFingerprint::reread`])。
//!   JSON は毎回読んでハッシュする (更新日時の刻みが粗いファイルシステムでも、編集を見逃さないため)。
//! - ハッシュは `std` の `DefaultHasher` で、同じプロセスの中で比べるだけ (ファイルには残さない)。

use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::project::{
    EXPRESSION_SUFFIX, EXPRESSIONS_DIR, MODEL_JSON, MODULE_JSON, MOTION_SUFFIX, MOTIONS_DIR,
    PHYSICS_JSON, PROJECT_JSON, ProjectError, TEXTURE_SUFFIX, TEXTURES_DIR, io_error, list_files,
};

/// プロジェクトのファイルの指紋 (プロジェクトからの相対パス → 中身のハッシュ)。
///
/// 比べる (`==`) ときは中身のハッシュと大きさだけを見る (更新日時は見ない)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFingerprint {
    files: BTreeMap<PathBuf, FileStamp>,
}

#[derive(Debug, Clone, Copy)]
struct FileStamp {
    len: u64,
    /// 取れないファイルシステムでは `None` (使い回さずに毎回ハッシュする)。
    modified: Option<SystemTime>,
    hash: u64,
}

impl PartialEq for FileStamp {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.hash == other.hash
    }
}

impl Eq for FileStamp {}

impl ProjectFingerprint {
    /// `dir` のプロジェクトの指紋を取る。`project.json` がなければ [`ProjectError::NotAProject`]。
    pub fn read(dir: &Path) -> Result<Self, ProjectError> {
        Self::default().reread(dir)
    }

    /// モジュールのプロジェクトの本体 (`.ugoeproj` のディレクトリか `.ugoe`) の指紋。`.ugoe` ならその
    /// ファイル 1 つ (ファイル名で持つ)。
    pub fn read_base(path: &Path) -> Result<Self, ProjectError> {
        Self::default().reread_base(path)
    }

    /// 本体の指紋を取り直す ([`ProjectFingerprint::reread`] と同じく、変わっていない大きなファイルは読まない)。
    pub fn reread_base(&self, path: &Path) -> Result<Self, ProjectError> {
        if path.is_dir() {
            return self.reread(path);
        }
        let name = PathBuf::from(path.file_name().unwrap_or(path.as_os_str()));
        let mut files = BTreeMap::new();
        if let Some(stamp) = stamp(path, self.files.get(&name))? {
            files.insert(name, stamp);
        }
        Ok(Self { files })
    }

    /// `dir` の指紋を取り直す。PNG は大きさと更新日時が `self` と同じなら、読まずに `self` のハッシュを使う。
    pub fn reread(&self, dir: &Path) -> Result<Self, ProjectError> {
        if !dir.join(PROJECT_JSON).is_file() {
            return Err(ProjectError::NotAProject {
                path: dir.to_path_buf(),
            });
        }
        let mut paths = vec![
            dir.join(PROJECT_JSON),
            dir.join(MODEL_JSON),
            dir.join(MODULE_JSON),
            dir.join(PHYSICS_JSON),
        ];
        paths.extend(list_files(&dir.join(MOTIONS_DIR), MOTION_SUFFIX)?);
        paths.extend(list_files(&dir.join(EXPRESSIONS_DIR), EXPRESSION_SUFFIX)?);
        let textures = list_files(&dir.join(TEXTURES_DIR), TEXTURE_SUFFIX)?;
        let first_texture = paths.len();
        paths.extend(textures);

        let mut files = BTreeMap::new();
        for (i, path) in paths.iter().enumerate() {
            let relative = path.strip_prefix(dir).unwrap_or(path).to_path_buf();
            let reuse = if i >= first_texture {
                self.files.get(&relative)
            } else {
                None
            };
            if let Some(stamp) = stamp(path, reuse)? {
                files.insert(relative, stamp);
            }
        }
        Ok(Self { files })
    }

    /// `self` (前の指紋) から `newer` までに、足された・消された・中身が変わったファイル
    /// (プロジェクトからの相対パス。区切りは `/`。名前の順)。
    pub fn changed_files(&self, newer: &Self) -> Vec<String> {
        let mut changed: Vec<&PathBuf> = self
            .files
            .iter()
            .filter(|(path, stamp)| newer.files.get(*path) != Some(stamp))
            .map(|(path, _)| path)
            .collect();
        changed.extend(newer.files.keys().filter(|p| !self.files.contains_key(*p)));
        let mut names: Vec<String> = changed.into_iter().map(|p| slash_path(p)).collect();
        names.sort();
        names
    }

    /// 指紋全体を 1 つにまとめた値 (同じ外の変更を二度知らせないための目印。プロセスの中でだけ意味がある)。
    pub fn digest(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (path, stamp) in &self.files {
            path.hash(&mut hasher);
            stamp.len.hash(&mut hasher);
            stamp.hash.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// ファイル 1 つの指紋。ファイルがなければ (読む間に消されたときも) `None`。
fn stamp(path: &Path, reuse: Option<&FileStamp>) -> Result<Option<FileStamp>, ProjectError> {
    let metadata = match fs::metadata(path) {
        Ok(m) if m.is_file() => m,
        Ok(_) => return Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(path)(e)),
    };
    let modified = metadata.modified().ok();
    if let Some(old) = reuse
        && old.modified.is_some()
        && old.modified == modified
        && old.len == metadata.len()
    {
        return Ok(Some(*old));
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(path)(e)),
    };
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(Some(FileStamp {
        len: bytes.len() as u64,
        modified,
        hash: hasher.finish(),
    }))
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
