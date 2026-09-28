//! プロジェクト (`.ugoeproj/` ディレクトリ) の保存と読み込み、`.ugoe` バンドルへの書き出し。
//! 仕様: `docs/spec/file-formats.md#プロジェクト-ugoeproj`、設計: `docs/editor.md#プロジェクト`。
//!
//! - 出力は安定している: 同じ内容を保存すると、バイト単位で同じファイルになる。
//!   内容が変わらないファイルは書き直さない (更新日時も変えない)。
//! - 各ファイルは一時ファイルに書いてから名前を変えて置き換える。書き込み中に落ちても、
//!   既存のファイルが壊れた中身で残ることはない (複数ファイルをまたぐ原子性はない)。
//! - `project.json` は最後に書く。

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;
use ugoe_format::{Bundle, FormatError, TextureData, TextureFormat};
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{Canvas, Effects, Model, ModelMeta, Pose, SCHEMA_VERSION};

use crate::Document;
use crate::texture::{Textures, is_valid_texture_name, png_size};

/// `project.json` の `format_version` の現在の値。プロジェクトのディレクトリ構成を変えたら上げる。
/// モデルのスキーマのバージョン (`ugoe_model::SCHEMA_VERSION`) とは別。
pub const PROJECT_FORMAT_VERSION: u32 = 0;

/// プロジェクトのディレクトリの拡張子。
pub const PROJECT_EXTENSION: &str = "ugoeproj";

const PROJECT_JSON: &str = "project.json";
const MODEL_JSON: &str = "model.json";
const PHYSICS_JSON: &str = "physics.json";
const GITIGNORE: &str = ".gitignore";
const MOTIONS_DIR: &str = "motions";
const EXPRESSIONS_DIR: &str = "expressions";
const TEXTURES_DIR: &str = "textures";
const MOTION_SUFFIX: &str = ".motion.json";
const EXPRESSION_SUFFIX: &str = ".expression.json";
const TEXTURE_SUFFIX: &str = ".png";
/// 一時ファイルの接尾辞。書き込みに失敗して残っても、読み込みでは無視される。
const TEMP_SUFFIX: &str = ".ugoe-tmp";
const GITIGNORE_CONTENT: &str = ".ugoe-cache/\n";

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{}: {source}", path.display())]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("{}: {source}", path.display())]
    Format { path: PathBuf, source: FormatError },
    #[error("{}: not a ugoe project (project.json is missing)", path.display())]
    NotAProject { path: PathBuf },
    /// 保存先が空でなく、プロジェクトでもないディレクトリ (上書きして壊さないため拒否する)。
    #[error("{}: directory is not empty and is not a ugoe project", path.display())]
    NotEmpty { path: PathBuf },
    #[error("unsupported project format version {found} (supported up to {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    /// `kind` は `texture`・`motion`・`expression`。
    #[error("{kind} name `{name}` cannot be used as a file name")]
    InvalidName { kind: &'static str, name: String },
    /// 大文字と小文字を区別せずに比べて重なる名前 (ファイル名が衝突する)。
    #[error("{kind} name `{name}` is used more than once")]
    DuplicateName { kind: &'static str, name: String },
    #[error("texture `{name}` has no image data")]
    MissingTexture { name: String },
    #[error("{}: not a PNG image", path.display())]
    NotPng { path: PathBuf },
    #[error("bundle export failed: {0}")]
    Export(#[source] FormatError),
}

/// `project.json` の中身。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// プロジェクトの形式のバージョン ([`PROJECT_FORMAT_VERSION`])。
    pub format_version: u32,
    /// プロジェクトの表示名。
    pub name: String,
    /// エディタの設定 (自由な JSON)。空なら書き出さない。
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub editor: serde_json::Map<String, serde_json::Value>,
}

impl ProjectMeta {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: PROJECT_FORMAT_VERSION,
            name: name.into(),
            editor: serde_json::Map::new(),
        }
    }
}

/// プロジェクト 1 つ分の中身。
///
/// 編集中は、モデル・テクスチャ・物理・モーション・表情を [`Document`] が持つ (Undo の対象)。
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub meta: ProjectMeta,
    pub model: Model,
    /// テクスチャ名 (`Model::textures` の `name`) → PNG。
    pub textures: Textures,
    pub physics: Option<PhysicsSettings>,
    /// 読み込むとファイル名の順に並ぶ。名前 (`Motion::name`) がファイル名になる。
    pub motions: Vec<Motion>,
    /// 読み込むとファイル名の順に並ぶ。名前 (`Expression::name`) がファイル名になる。
    pub expressions: Vec<Expression>,
}

impl Project {
    /// モデルとテクスチャだけのプロジェクト。
    pub fn new(meta: ProjectMeta, model: Model, textures: Textures) -> Self {
        Self {
            meta,
            model,
            textures,
            physics: None,
            motions: Vec::new(),
            expressions: Vec::new(),
        }
    }

    /// 空のモデル (1024×1024 のキャンバス) を持つ新しいプロジェクト。
    pub fn empty(name: impl Into<String>) -> Self {
        let name = name.into();
        let model = Model {
            schema_version: SCHEMA_VERSION,
            meta: ModelMeta {
                name: name.clone(),
                canvas: Canvas {
                    width: 1024.0,
                    height: 1024.0,
                    origin: Vec2::new(512.0, 512.0),
                },
            },
            parameters: Vec::new(),
            parameter_groups: Vec::new(),
            parts: Vec::new(),
            deformers: Vec::new(),
            meshes: Vec::new(),
            glues: Vec::new(),
            textures: Vec::new(),
            hit_areas: Vec::new(),
            pose: Pose::default(),
            effects: Effects::default(),
            avatar: None,
        };
        Self::new(ProjectMeta::new(name), model, Textures::new())
    }

    /// 編集中のドキュメントの現在の状態から、保存・書き出し用のプロジェクトを作る。
    pub fn from_document(meta: ProjectMeta, document: &Document) -> Self {
        Self {
            meta,
            model: document.model().clone(),
            textures: document.textures().clone(),
            physics: document.physics().cloned(),
            motions: document.motions().to_vec(),
            expressions: document.expressions().to_vec(),
        }
    }

    /// モデル・テクスチャ・物理・モーション・表情から編集用のドキュメントを作る (保存済みの状態として)。
    pub fn document(&self) -> Document {
        Document::with_textures(self.model.clone(), self.textures.clone())
            .with_animations(self.motions.clone(), self.expressions.clone())
            .with_physics(self.physics.clone())
    }

    /// `dir` にプロジェクトを保存する。`dir` がなければ作る。
    ///
    /// `dir` が既存のプロジェクトなら上書きし、今の内容にないモーション・表情・テクスチャの
    /// ファイルと `physics.json` を消す。`sources/` と `.ugoe-cache/` には触れない。
    /// 空でなく、`project.json` もないディレクトリには保存しない ([`ProjectError::NotEmpty`])。
    /// ディスクに書く前にすべてのファイルの中身を作るので、名前の不正などのエラーでは何も書かない。
    pub fn save(&self, dir: &Path) -> Result<(), ProjectError> {
        let meta = to_json(&self.meta)?;
        let files = self.files()?;
        ensure_saveable(dir)?;
        fs::create_dir_all(dir).map_err(io_error(dir))?;

        for (relative, bytes) in &files {
            write_if_changed(&dir.join(relative), bytes)?;
        }
        let written: BTreeSet<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
        if self.physics.is_none() {
            remove_if_exists(&dir.join(PHYSICS_JSON))?;
        }
        remove_stale(dir, MOTIONS_DIR, MOTION_SUFFIX, &written)?;
        remove_stale(dir, EXPRESSIONS_DIR, EXPRESSION_SUFFIX, &written)?;
        remove_stale(dir, TEXTURES_DIR, TEXTURE_SUFFIX, &written)?;
        // project.json はプロジェクトとして完成したことの印になるので最後に書く。
        write_if_changed(&dir.join(PROJECT_JSON), &meta)
    }

    /// `dir` からプロジェクトを読み込む。
    ///
    /// モデルは検証 (`ugoe_model::validate`) しない (不正なモデルもエディタで開いて直せるように)。
    /// `Model::textures` のすべてに `textures/<name>.png` が必要。参照されない PNG は無視する。
    pub fn load(dir: &Path) -> Result<Self, ProjectError> {
        let meta_path = dir.join(PROJECT_JSON);
        if !meta_path.is_file() {
            return Err(ProjectError::NotAProject {
                path: dir.to_path_buf(),
            });
        }
        let meta: ProjectMeta = read_json(&meta_path)?;
        if meta.format_version > PROJECT_FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion {
                found: meta.format_version,
                supported: PROJECT_FORMAT_VERSION,
            });
        }

        let model_path = dir.join(MODEL_JSON);
        let model_json = fs::read_to_string(&model_path).map_err(io_error(&model_path))?;
        let model =
            ugoe_format::model_from_json(&model_json).map_err(|source| ProjectError::Format {
                path: model_path.clone(),
                source,
            })?;

        let physics_path = dir.join(PHYSICS_JSON);
        let physics = if physics_path.exists() {
            Some(read_json(&physics_path)?)
        } else {
            None
        };
        let motions = read_json_dir(&dir.join(MOTIONS_DIR), MOTION_SUFFIX)?;
        let expressions = read_json_dir(&dir.join(EXPRESSIONS_DIR), EXPRESSION_SUFFIX)?;

        let mut textures = Textures::new();
        for texture in &model.textures {
            check_name("texture", &texture.name)?;
            let path = texture_path(dir, &texture.name);
            if !path.is_file() {
                return Err(ProjectError::MissingTexture {
                    name: texture.name.clone(),
                });
            }
            let png = fs::read(&path).map_err(io_error(&path))?;
            if png_size(&png).is_none() {
                return Err(ProjectError::NotPng { path });
            }
            textures.insert(texture.name.clone(), Arc::from(png));
        }

        Ok(Self {
            meta,
            model,
            textures,
            physics,
            motions,
            expressions,
        })
    }

    /// `project.json` 以外に保存するファイルの一覧 (プロジェクトからの相対パスと中身)。
    fn files(&self) -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectError> {
        let mut files = vec![(PathBuf::from(GITIGNORE), GITIGNORE_CONTENT.into())];
        let model =
            ugoe_format::model_to_json(&self.model).map_err(|source| ProjectError::Format {
                path: PathBuf::from(MODEL_JSON),
                source,
            })?;
        files.push((PathBuf::from(MODEL_JSON), with_newline(model)));
        if let Some(physics) = &self.physics {
            files.push((PathBuf::from(PHYSICS_JSON), to_json(physics)?));
        }

        check_names("motion", self.motions.iter().map(|m| m.name.as_str()))?;
        for motion in &self.motions {
            let path = Path::new(MOTIONS_DIR).join(format!("{}{MOTION_SUFFIX}", motion.name));
            files.push((path, to_json(motion)?));
        }
        check_names(
            "expression",
            self.expressions.iter().map(|e| e.name.as_str()),
        )?;
        for expression in &self.expressions {
            let path =
                Path::new(EXPRESSIONS_DIR).join(format!("{}{EXPRESSION_SUFFIX}", expression.name));
            files.push((path, to_json(expression)?));
        }
        check_names(
            "texture",
            self.model.textures.iter().map(|t| t.name.as_str()),
        )?;
        for texture in &self.model.textures {
            let png = self.texture_bytes(&texture.name)?;
            let path = Path::new(TEXTURES_DIR).join(format!("{}{TEXTURE_SUFFIX}", texture.name));
            files.push((path, png.to_vec()));
        }
        Ok(files)
    }

    fn texture_bytes(&self, name: &str) -> Result<&[u8], ProjectError> {
        self.textures
            .get(name)
            .map(|png| &**png)
            .ok_or_else(|| ProjectError::MissingTexture {
                name: name.to_owned(),
            })
    }
}

/// プロジェクトを `.ugoe` バンドルのバイト列に書き出す。
/// テクスチャは `Model::textures` の順に並べる。物理・モーション・表情も含める。
pub fn export_bundle(project: &Project) -> Result<Vec<u8>, ProjectError> {
    let textures = project
        .model
        .textures
        .iter()
        .map(|t| {
            Ok(TextureData {
                format: TextureFormat::Png,
                bytes: project.texture_bytes(&t.name)?.to_vec(),
            })
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    let bundle = Bundle {
        model: project.model.clone(),
        physics: project.physics.clone(),
        motions: project.motions.clone(),
        expressions: project.expressions.clone(),
        textures,
        meta: None,
    };
    ugoe_format::write_bundle(&bundle).map_err(ProjectError::Export)
}

/// 保存先が、存在しない・空・既存のプロジェクトのいずれかであること。
fn ensure_saveable(dir: &Path) -> Result<(), ProjectError> {
    if !dir.exists() || dir.join(PROJECT_JSON).is_file() {
        return Ok(());
    }
    let mut entries = fs::read_dir(dir).map_err(io_error(dir))?;
    if entries.next().is_some() {
        return Err(ProjectError::NotEmpty {
            path: dir.to_path_buf(),
        });
    }
    Ok(())
}

fn check_name(kind: &'static str, name: &str) -> Result<(), ProjectError> {
    if is_valid_texture_name(name) {
        Ok(())
    } else {
        Err(ProjectError::InvalidName {
            kind,
            name: name.to_owned(),
        })
    }
}

/// 名前がファイル名に使え、大文字と小文字を区別せずに一意であること。
fn check_names<'a>(
    kind: &'static str,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<(), ProjectError> {
    let mut seen = BTreeSet::new();
    for name in names {
        check_name(kind, name)?;
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(ProjectError::DuplicateName {
                kind,
                name: name.to_owned(),
            });
        }
    }
    Ok(())
}

fn texture_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(TEXTURES_DIR)
        .join(format!("{name}{TEXTURE_SUFFIX}"))
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> ProjectError + '_ {
    move |source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// 2 スペースで整形した JSON (末尾に改行)。フィールドは型定義の順になる。
fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, ProjectError> {
    serde_json::to_string_pretty(value)
        .map(with_newline)
        .map_err(|source| ProjectError::Json {
            path: PathBuf::new(),
            source,
        })
}

fn with_newline(mut json: String) -> Vec<u8> {
    json.push('\n');
    json.into_bytes()
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    let bytes = fs::read(path).map_err(io_error(path))?;
    serde_json::from_slice(&bytes).map_err(|source| ProjectError::Json {
        path: path.to_path_buf(),
        source,
    })
}

/// `dir` の中の `*<suffix>` を、ファイル名の順に読む。`dir` がなければ空。
fn read_json_dir<T: DeserializeOwned>(dir: &Path, suffix: &str) -> Result<Vec<T>, ProjectError> {
    list_files(dir, suffix)?
        .iter()
        .map(|path| read_json(path))
        .collect()
}

/// `dir` の中の、名前が `suffix` で終わるファイルをファイル名の順に返す。`dir` がなければ空。
fn list_files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, ProjectError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_error(dir))? {
        let entry = entry.map_err(io_error(dir))?;
        let path = entry.path();
        let matches = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(suffix) && !name.starts_with('.'));
        if matches && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// `dir/sub` の中の `*<suffix>` のうち、今回書いたもの (`written`) 以外を消す。
fn remove_stale(
    dir: &Path,
    sub: &str,
    suffix: &str,
    written: &BTreeSet<&Path>,
) -> Result<(), ProjectError> {
    for path in list_files(&dir.join(sub), suffix)? {
        let relative = path.strip_prefix(dir).unwrap_or(&path);
        if !written.contains(relative) {
            remove_if_exists(&path)?;
        }
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), ProjectError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(ProjectError::Io {
            path: path.to_path_buf(),
            source: e,
        }),
        _ => Ok(()),
    }
}

/// 中身が変わるときだけ、一時ファイルに書いてから名前を変えて置き換える。
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    if fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error(parent))?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(TEMP_SUFFIX);
    let temp = PathBuf::from(temp);
    let result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    result.map_err(|source| {
        let _ = fs::remove_file(&temp);
        ProjectError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}
