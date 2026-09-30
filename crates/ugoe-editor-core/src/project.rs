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
use ugoe_model::module::ModuleRequirements;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::ValidationError;
use ugoe_model::{Canvas, Effects, Model, ModelMeta, Pose, SCHEMA_VERSION, SlotId, TextureRef};

use crate::Document;
use crate::fingerprint::ProjectFingerprint;
use crate::module_project::{ModuleBase, ModuleContext, ModuleError, Workspace};
use crate::texture::{Textures, is_valid_texture_name, png_size};

/// `project.json` の `format_version` の現在の値。プロジェクトのディレクトリ構成を変えたら上げる。
/// モデルのスキーマのバージョン (`ugoe_model::SCHEMA_VERSION`) とは別。
pub const PROJECT_FORMAT_VERSION: u32 = 0;

/// プロジェクトのディレクトリの拡張子。
pub const PROJECT_EXTENSION: &str = "ugoeproj";

pub(crate) const PROJECT_JSON: &str = "project.json";
pub(crate) const MODEL_JSON: &str = "model.json";
/// モジュールのプロジェクトで、`model.json` の代わりに置くモジュール (`ugoe_model::Module`)。
pub(crate) const MODULE_JSON: &str = "module.json";
pub(crate) const PHYSICS_JSON: &str = "physics.json";
const GITIGNORE: &str = ".gitignore";
pub(crate) const MOTIONS_DIR: &str = "motions";
pub(crate) const EXPRESSIONS_DIR: &str = "expressions";
pub(crate) const TEXTURES_DIR: &str = "textures";
pub(crate) const MOTION_SUFFIX: &str = ".motion.json";
pub(crate) const EXPRESSION_SUFFIX: &str = ".expression.json";
pub(crate) const TEXTURE_SUFFIX: &str = ".png";
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
    /// `kind` が `module` なのに `module` (本体とスロット) がない。
    #[error("project.json: a module project needs `module` (the base and the slot)")]
    MissingModuleLink,
    /// モジュールのプロジェクトが参照する本体が見つからない。
    #[error("base model not found: {}", path.display())]
    BaseNotFound { path: PathBuf },
    /// 本体を読めない。
    #[error("base model {}: {source}", path.display())]
    Base {
        path: PathBuf,
        source: Box<ProjectError>,
    },
    /// 本体がモジュールのプロジェクト (本体にはモデルのプロジェクトか `.ugoe` を選ぶ)。
    #[error("{}: the base must be a model project or a .ugoe bundle, not a module project", path.display())]
    BaseIsModule { path: PathBuf },
    /// 本体の `.ugoe` のテクスチャが PNG でない (エディタは PNG だけを扱う)。
    #[error("base texture `{name}` is not a PNG")]
    UnsupportedBaseTexture { name: String },
    /// モジュールの作業用のモデルを作れない・分けられない (スロットがない・合成できないなど)。
    #[error(transparent)]
    Module(#[from] ModuleError),
    /// 書き出すモジュールが検証 (`validate_module`) を通らない。
    /// モジュールのプロジェクトでない ([`export_module`])。
    #[error("this project is not a module project")]
    NotAModule,
    #[error("the module is invalid: {}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))]
    InvalidModule(Vec<ValidationError>),
}

/// `project.json` の中身。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// プロジェクトの形式のバージョン ([`PROJECT_FORMAT_VERSION`])。
    pub format_version: u32,
    /// プロジェクトの表示名。
    pub name: String,
    /// プロジェクトの種類。本体 (`model`、既定。書き出さない) か着せ替えのモジュール (`module`)。
    #[serde(default, skip_serializing_if = "ProjectKind::is_model")]
    pub kind: ProjectKind,
    /// モジュールのプロジェクトが参照する本体とスロット (`kind` が `module` のとき)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<ModuleLink>,
    /// エディタの設定 (自由な JSON)。空なら書き出さない。
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub editor: serde_json::Map<String, serde_json::Value>,
}

/// プロジェクトの種類 (`project.json` の `kind`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    /// 本体のモデル (`model.json`)。
    #[default]
    Model,
    /// 本体を参照する着せ替えのモジュール (`module.json`。ADR 0024)。
    Module,
}

impl ProjectKind {
    pub fn is_model(&self) -> bool {
        *self == Self::Model
    }
}

/// モジュールのプロジェクトが参照する本体とスロット (`project.json` の `module`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleLink {
    /// 本体 (`.ugoeproj` のディレクトリか `.ugoe`)。プロジェクトのディレクトリからの相対パス (`/` 区切り) か絶対パス。
    /// 保存するときは相対パスで書く (プロジェクトを本体ごと動かしても開けるように)。
    pub base: String,
    /// 取り付け先のスロット。
    pub slot: SlotId,
}

impl ProjectMeta {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: PROJECT_FORMAT_VERSION,
            name: name.into(),
            kind: ProjectKind::Model,
            module: None,
            editor: serde_json::Map::new(),
        }
    }

    /// このプロジェクトのデータ (画像・レイヤー名など) を外部の AI サービスに送ってよいか。
    ///
    /// `editor.ai.send_externally` が `false` のときだけ送らない (書いていなければ送ってよい。
    /// 送るたびに同意は求める)。`docs/ai-integration.md#セキュリティとプライバシー`。
    pub fn ai_send_externally(&self) -> bool {
        self.editor
            .get(AI_KEY)
            .and_then(|ai| ai.get(AI_SEND_EXTERNALLY_KEY))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
    }

    /// [`Self::ai_send_externally`] を設定する。`true` (既定) にすると項目を消し、
    /// `editor.ai` が空になればそれも消す (既定のままのプロジェクトの `project.json` を変えないため)。
    pub fn set_ai_send_externally(&mut self, allowed: bool) {
        let ai = self
            .editor
            .entry(AI_KEY)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if !ai.is_object() {
            *ai = serde_json::Value::Object(serde_json::Map::new());
        }
        if let serde_json::Value::Object(map) = ai {
            if allowed {
                map.remove(AI_SEND_EXTERNALLY_KEY);
            } else {
                map.insert(AI_SEND_EXTERNALLY_KEY.into(), false.into());
            }
            if map.is_empty() {
                self.editor.remove(AI_KEY);
            }
        }
    }
}

/// `editor` のうち AI の設定を置く項目。
const AI_KEY: &str = "ai";
const AI_SEND_EXTERNALLY_KEY: &str = "send_externally";

/// プロジェクト 1 つ分の中身。
///
/// 編集中は、モデル・テクスチャ・物理・モーション・表情を [`Document`] が持つ (Undo の対象)。
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub meta: ProjectMeta,
    pub model: Model,
    /// テクスチャ名 (`Model::textures` の `name`) → PNG。
    /// 法線マップの画像も、`TextureRef::normal_map` の名前で同じ表に入れる。
    pub textures: Textures,
    pub physics: Option<PhysicsSettings>,
    /// 読み込むとファイル名の順に並ぶ。名前 (`Motion::name`) がファイル名になる。
    pub motions: Vec<Motion>,
    /// 読み込むとファイル名の順に並ぶ。名前 (`Expression::name`) がファイル名になる。
    pub expressions: Vec<Expression>,
    /// モジュールのプロジェクトなら、本体とスロット。このとき `model`・`textures`・`physics` は本体と合成した
    /// 作業用のもの (`crate::module_project`) で、保存するとモジュールの部分だけを書く。
    pub module: Option<Arc<ModuleContext>>,
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
            module: None,
        }
    }

    /// 本体 `base` (`.ugoeproj` のディレクトリか `.ugoe`) のスロット `slot` に付ける、空のモジュールの
    /// 新しいプロジェクト。モジュールの名前はプロジェクトの名前 `name`。
    pub fn new_module(name: &str, base: &Path, slot: &SlotId) -> Result<Self, ProjectError> {
        let base = load_base(&normalize_path(base))?;
        let context = ModuleContext::new(base, slot, name, ModuleRequirements::default())?;
        let workspace = context.workspace(&context.empty_module(), &Textures::new())?;
        let mut meta = ProjectMeta::new(name);
        meta.kind = ProjectKind::Module;
        meta.module = Some(ModuleLink {
            base: context.base().path.display().to_string(),
            slot: slot.clone(),
        });
        Ok(Self::from_workspace(meta, workspace, context))
    }

    fn from_workspace(meta: ProjectMeta, workspace: Workspace, context: ModuleContext) -> Self {
        Self {
            meta,
            model: workspace.model,
            textures: workspace.textures,
            physics: workspace.physics,
            motions: Vec::new(),
            expressions: Vec::new(),
            module: Some(Arc::new(context)),
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
            slots: Vec::new(),
            lighting: false,
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
            module: document.module().cloned(),
        }
    }

    /// モデル・テクスチャ・物理・モーション・表情から編集用のドキュメントを作る (保存済みの状態として)。
    pub fn document(&self) -> Document {
        let document = Document::with_textures(self.model.clone(), self.textures.clone())
            .with_animations(self.motions.clone(), self.expressions.clone())
            .with_physics(self.physics.clone());
        match &self.module {
            Some(context) => document.with_module(context.clone()),
            None => document,
        }
    }

    /// `dir` に保存するときの `project.json` (モジュールなら本体の参照を `dir` からの相対パスにする)。
    fn meta_for(&self, dir: &Path) -> ProjectMeta {
        let mut meta = self.meta.clone();
        if let Some(context) = &self.module {
            meta.kind = ProjectKind::Module;
            meta.module = Some(ModuleLink {
                base: relative_path(&normalize_path(&absolute(dir)), &context.base().path),
                slot: context.slot().id.clone(),
            });
        }
        meta
    }

    /// `dir` にプロジェクトを保存する。`dir` がなければ作る。
    ///
    /// `dir` が既存のプロジェクトなら上書きし、今の内容にないモーション・表情・テクスチャの
    /// ファイルと `physics.json` を消す。`sources/` と `.ugoe-cache/` には触れない。
    /// 空でなく、`project.json` もないディレクトリには保存しない ([`ProjectError::NotEmpty`])。
    /// ディスクに書く前にすべてのファイルの中身を作るので、名前の不正などのエラーでは何も書かない。
    pub fn save(&self, dir: &Path) -> Result<(), ProjectError> {
        let meta = to_json(&self.meta_for(dir))?;
        let files = match &self.module {
            Some(context) => self.module_files(context)?,
            None => self.files()?,
        };
        ensure_saveable(dir)?;
        fs::create_dir_all(dir).map_err(io_error(dir))?;

        for (relative, bytes) in &files {
            write_if_changed(&dir.join(relative), bytes)?;
        }
        let written: BTreeSet<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
        // 種類を変えて上書きしたときに、前の種類のファイルを残さない。
        if self.module.is_some() {
            remove_if_exists(&dir.join(MODEL_JSON))?;
        } else {
            remove_if_exists(&dir.join(MODULE_JSON))?;
        }
        if self.physics.is_none() || self.module.is_some() {
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
    /// `Model::textures` のすべてと、その `normal_map` のすべてに `textures/<name>.png` が必要。
    /// 参照されない PNG は無視する。
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
        if meta.kind == ProjectKind::Module {
            return Self::load_module(dir, meta, None);
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

        let textures = read_images(dir, &model.textures)?;

        Ok(Self {
            meta,
            model,
            textures,
            physics,
            motions,
            expressions,
            module: None,
        })
    }

    /// モジュールのプロジェクト `dir` を、本体を `base` に付け替えて読む (本体を動かした・見つからないとき)。
    /// 付け替えた参照は、次に保存したときに `project.json` に書く。
    pub fn load_with_base(dir: &Path, base: &Path) -> Result<Self, ProjectError> {
        let meta_path = dir.join(PROJECT_JSON);
        if !meta_path.is_file() {
            return Err(ProjectError::NotAProject {
                path: dir.to_path_buf(),
            });
        }
        let meta: ProjectMeta = read_json(&meta_path)?;
        if meta.kind != ProjectKind::Module {
            return Err(ProjectError::NotAModule);
        }
        Self::load_module(dir, meta, Some(base))
    }

    /// モジュールのプロジェクトを読む: `module.json` と画像を読み、本体を読んで作業用のモデルを作る。
    /// `base` があれば `project.json` の参照の代わりにそれを本体にする。
    fn load_module(
        dir: &Path,
        meta: ProjectMeta,
        base: Option<&Path>,
    ) -> Result<Self, ProjectError> {
        let link = meta.module.clone().ok_or(ProjectError::MissingModuleLink)?;
        let module_path = dir.join(MODULE_JSON);
        let json = fs::read_to_string(&module_path).map_err(io_error(&module_path))?;
        let module =
            ugoe_format::module_from_json(&json).map_err(|source| ProjectError::Format {
                path: module_path.clone(),
                source,
            })?;
        let images = read_images(dir, &module.textures)?;
        let base_path = match base {
            Some(base) => normalize_path(base),
            None => resolve_base(dir, &link.base),
        };
        let base = load_base(&base_path)?;
        let context = ModuleContext::new(
            base,
            &link.slot,
            module.name.clone(),
            module.requires.clone(),
        )?;
        let workspace = context.workspace(&module, &images)?;
        Ok(Self::from_workspace(meta, workspace, context))
    }

    /// モジュールのプロジェクトで保存するファイル (`module.json` と画像)。
    fn module_files(
        &self,
        context: &ModuleContext,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectError> {
        let (module, images) = context.split(&self.model, self.physics.as_ref(), &self.textures)?;
        let json = ugoe_format::module_to_json(&module).map_err(|source| ProjectError::Format {
            path: PathBuf::from(MODULE_JSON),
            source,
        })?;
        let mut files = vec![
            (PathBuf::from(GITIGNORE), GITIGNORE_CONTENT.into()),
            (PathBuf::from(MODULE_JSON), with_newline(json)),
        ];
        let names = image_names(&module.textures);
        check_names("texture", names.iter().copied())?;
        for name in names {
            let png = images
                .get(name)
                .ok_or_else(|| ProjectError::MissingTexture {
                    name: name.to_owned(),
                })?;
            let path = Path::new(TEXTURES_DIR).join(format!("{name}{TEXTURE_SUFFIX}"));
            files.push((path, png.to_vec()));
        }
        Ok(files)
    }

    /// [`Self::load`] と、読み込む直前のファイルの指紋 (外での書き換えに気づくため、`docs/editor.md#外での書き換え`)。
    ///
    /// 指紋は読み込みの前に取る。読み込みの間に外で書き換えられたら、次に比べたときに変更として見える
    /// (見逃すより、開き直しを促すほうを選ぶ)。
    pub fn load_tracked(dir: &Path) -> Result<(Self, ProjectFingerprint), ProjectError> {
        let fingerprint = ProjectFingerprint::read(dir)?;
        Ok((Self::load(dir)?, fingerprint))
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
        // 法線マップの画像もテクスチャと同じ `textures/` に置くので、合わせて大文字小文字の衝突を調べる。
        let images = image_names(&self.model.textures);
        check_names("texture", images.iter().copied())?;
        for name in images {
            let png = self.texture_bytes(name)?;
            let path = Path::new(TEXTURES_DIR).join(format!("{name}{TEXTURE_SUFFIX}"));
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
    // 法線マップの画像は、テクスチャと同じ `textures` から名前で引く。
    let normal_maps = project
        .model
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref())
        .map(|name| {
            Ok(TextureData {
                format: TextureFormat::Png,
                bytes: project.texture_bytes(name)?.to_vec(),
            })
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    let bundle = Bundle {
        model: project.model.clone(),
        physics: project.physics.clone(),
        motions: project.motions.clone(),
        expressions: project.expressions.clone(),
        textures,
        normal_maps,
        meta: None,
    };
    ugoe_format::write_bundle(&bundle).map_err(ProjectError::Export)
}

/// [`export_module`] の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleExport {
    /// `.ugoemod` のバイト列。
    pub bytes: Vec<u8>,
    /// 書き出しはしたが知らせたいこと (描画順が 0〜1000 の外にあり、スロットの範囲からはみ出すなど)。
    pub warnings: Vec<String>,
}

/// モジュールのプロジェクトを `.ugoemod` のバイト列に書き出す (`docs/editor.md#モジュールのプロジェクト`)。
///
/// 作業用のモデルからモジュールを取り出し、単体の検証 (`validate_module`) と、本体との合成
/// (`compose`・`compose_physics`) を確かめてから書く。テクスチャと法線マップを含める。
pub fn export_module(project: &Project) -> Result<ModuleExport, ProjectError> {
    let context = project.module.as_ref().ok_or(ProjectError::NotAModule)?;
    let (module, images) =
        context.split(&project.model, project.physics.as_ref(), &project.textures)?;
    let errors = ugoe_model::validate::validate_module(&module);
    if !errors.is_empty() {
        return Err(ProjectError::InvalidModule(errors));
    }
    let base = context.base();
    let model = ugoe_runtime::compose(&base.model, &[&module]).map_err(ModuleError::from)?;
    if base.physics.is_some() || !module.physics.is_empty() {
        let physics = base
            .physics
            .clone()
            .unwrap_or_else(crate::physics_edit::default_settings);
        let composed = ugoe_runtime::compose_physics(&base.model, &physics, &[&module])
            .map_err(ModuleError::from)?;
        let errors = ugoe_model::validate::validate_physics(&composed, &model);
        if !errors.is_empty() {
            return Err(ProjectError::InvalidModule(errors));
        }
    }

    let mut warnings = Vec::new();
    let range = 0.0..=crate::module_project::MODULE_DRAW_ORDER_RANGE;
    for mesh in &module.meshes {
        if mesh
            .keyforms
            .all_keyforms()
            .any(|k| !range.contains(&k.draw_order))
        {
            warnings.push(format!(
                "mesh `{}`: the draw order is outside the slot range, so it is drawn outside the slot",
                mesh.id
            ));
        }
    }

    let png = |name: &str| {
        images
            .get(name)
            .map(|png| TextureData {
                format: TextureFormat::Png,
                bytes: png.to_vec(),
            })
            .ok_or_else(|| ProjectError::MissingTexture {
                name: name.to_owned(),
            })
    };
    let textures = module
        .textures
        .iter()
        .map(|t| png(&t.name))
        .collect::<Result<Vec<_>, _>>()?;
    let normal_maps = module
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref())
        .map(png)
        .collect::<Result<Vec<_>, _>>()?;
    let file = ugoe_format::ModuleFile {
        module,
        textures,
        normal_maps,
        meta: None,
    };
    let bytes = ugoe_format::write_module(&file).map_err(ProjectError::Export)?;
    Ok(ModuleExport { bytes, warnings })
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

/// `textures/` に置く画像の名前。`textures` の順に、テクスチャの名前と、まだ出ていない
/// `normal_map` の名前を並べる (同じ名前は 1 度だけ)。
fn image_names(textures: &[TextureRef]) -> Vec<&str> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    let all = textures
        .iter()
        .map(|t| t.name.as_str())
        .chain(textures.iter().filter_map(|t| t.normal_map.as_deref()));
    for name in all {
        if seen.insert(name) {
            names.push(name);
        }
    }
    names
}

/// `textures` のすべての画像 (テクスチャと法線マップ) を `dir/textures/` から読む。
fn read_images(dir: &Path, textures: &[TextureRef]) -> Result<Textures, ProjectError> {
    let mut images = Textures::new();
    for name in image_names(textures) {
        check_name("texture", name)?;
        let path = texture_path(dir, name);
        if !path.is_file() {
            return Err(ProjectError::MissingTexture {
                name: name.to_owned(),
            });
        }
        let png = fs::read(&path).map_err(io_error(&path))?;
        if png_size(&png).is_none() {
            return Err(ProjectError::NotPng { path });
        }
        images.insert(name.to_owned(), Arc::from(png));
    }
    Ok(images)
}

/// モジュールのプロジェクトの本体を読む。`path` は `.ugoeproj` のディレクトリか `.ugoe`。
pub fn load_base(path: &Path) -> Result<ModuleBase, ProjectError> {
    let wrap = |source: ProjectError| ProjectError::Base {
        path: path.to_path_buf(),
        source: Box::new(source),
    };
    if !path.exists() {
        return Err(ProjectError::BaseNotFound {
            path: path.to_path_buf(),
        });
    }
    if path.is_dir() {
        let project = Project::load(path).map_err(wrap)?;
        if project.module.is_some() {
            return Err(ProjectError::BaseIsModule {
                path: path.to_path_buf(),
            });
        }
        return Ok(ModuleBase {
            path: path.to_path_buf(),
            model: project.model,
            physics: project.physics,
            textures: project.textures,
        });
    }
    let bytes = fs::read(path).map_err(io_error(path))?;
    let bundle = ugoe_format::read_bundle(&bytes).map_err(|source| {
        wrap(ProjectError::Format {
            path: path.to_path_buf(),
            source,
        })
    })?;
    let mut textures = Textures::new();
    let mut png = |name: &str, data: &TextureData| {
        if data.format != TextureFormat::Png {
            return Err(ProjectError::UnsupportedBaseTexture {
                name: name.to_owned(),
            });
        }
        textures.insert(name.to_owned(), Arc::from(data.bytes.as_slice()));
        Ok(())
    };
    for (texture, data) in bundle.model.textures.iter().zip(&bundle.textures) {
        png(&texture.name, data)?;
    }
    let normal_names = bundle
        .model
        .textures
        .iter()
        .filter_map(|t| t.normal_map.as_deref());
    for (name, data) in normal_names.zip(&bundle.normal_maps) {
        png(name, data)?;
    }
    Ok(ModuleBase {
        path: path.to_path_buf(),
        model: bundle.model,
        physics: bundle.physics,
        textures,
    })
}

/// `project.json` に書いた本体の参照を、プロジェクトのディレクトリ `dir` から解決する。
fn resolve_base(dir: &Path, base: &str) -> PathBuf {
    let base = Path::new(base);
    if base.is_absolute() {
        normalize_path(base)
    } else {
        normalize_path(&absolute(dir).join(base))
    }
}

/// 絶対パスにする (今の作業ディレクトリから)。取れなければそのまま。
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `.` と `..` を字面の上で取り除く (ファイルシステムは見ない)。
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in absolute(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// `from` (ディレクトリ) から `to` への相対パス (`/` 区切り)。根が違えば `to` の絶対パス。
fn relative_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_components)
        .take_while(|(a, b)| a == b)
        .count();
    if common == 0 {
        return to.display().to_string();
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); from.len() - common];
    parts.extend(
        to_components[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

fn texture_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(TEXTURES_DIR)
        .join(format!("{name}{TEXTURE_SUFFIX}"))
}

pub(crate) fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> ProjectError + '_ {
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
pub(crate) fn list_files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, ProjectError> {
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
