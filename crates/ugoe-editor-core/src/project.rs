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
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;
use ugoe_format::{Bundle, FormatError, TextureData, TextureFormat};
use ugoe_model::expression::Expression;
use ugoe_model::module::ModuleRequirements;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, describe_errors};
use ugoe_model::{
    Canvas, Effects, Model, ModelMeta, Module, Pose, SCHEMA_VERSION, SlotId, TextureRef,
};

use crate::Document;
use crate::fingerprint::ProjectFingerprint;
use crate::message::Message;
use crate::module_project::{ColliderReference, ModuleContext, ModuleError, Workspace};
use crate::texture::{Textures, is_valid_texture_name, png_size};

mod files;
mod module;
mod paths;

pub(crate) use files::{io_error, list_files};
use files::{
    known, read_json, read_json_dir, remove_if_exists, remove_stale, to_json, with_newline,
    write_if_changed,
};
pub use module::{BaseCheck, load_base, load_base_checked, read_module_link};
use module::{module_meta_with, unchecked};
use paths::{absolute, normalize_path, relative_path};

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
    /// 比べる `.ugoemod` のバイト列を読めない ([`crate::compare_with_module_file`])。書き出しの失敗 ([`Self::Export`]) と
    /// 区別する。
    #[error("module file could not be read: {0}")]
    UnreadableModuleFile(#[source] FormatError),
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
    /// 本体のパスを確かめる関数 ([`BaseCheck`]) が本体を読むことを断った (MCP の `--root` の外など)。本体は読んでいない。
    #[error("base model {}: {reason}", path.display())]
    BaseRefused { path: PathBuf, reason: String },
    /// 本体がモジュールのプロジェクト (本体にはモデルのプロジェクトか `.ugoe` を選ぶ)。
    #[error("{}: the base must be a model project or a .ugoe bundle, not a module project", path.display())]
    BaseIsModule { path: PathBuf },
    /// 本体の `.ugoe` のテクスチャが PNG でない (エディタは PNG だけを扱う)。
    #[error("base texture `{name}` is not a PNG")]
    UnsupportedBaseTexture { name: String },
    /// 取り込む `.ugoe`・`.ugoemod` のテクスチャが PNG でない ([`crate::bundle_project`])。
    #[error("texture `{name}` is not a PNG")]
    UnsupportedTexture { name: String },
    /// モジュールの作業用のモデルを作れない・分けられない (スロットがない・合成できないなど)。
    /// [`ModuleError::NotModule`] は [`Self::NotAModule`] に変わる。
    #[error(transparent)]
    Module(ModuleError),
    /// モジュールのプロジェクトでない ([`export_module`])。
    #[error("this project is not a module project")]
    NotAModule,
    /// 書き出すモジュールが検証 (`validate_module`) を通らない。
    #[error("{}", describe_errors("module", .0))]
    InvalidModule(Vec<ValidationError>),
}

impl From<ModuleError> for ProjectError {
    fn from(error: ModuleError) -> Self {
        match error {
            ModuleError::NotModule => Self::NotAModule,
            other => Self::Module(other),
        }
    }
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
        let meta = to_json(Path::new(PROJECT_JSON), &self.meta_for(dir))?;
        let files = match &self.module {
            Some(context) => self.module_files(context)?,
            None => self.files()?,
        };
        ensure_saveable(dir)?;
        fs::create_dir_all(dir).map_err(io_error(dir))?;

        for (relative, bytes) in &files {
            write_if_changed(&dir.join(relative), bytes)?;
        }
        // 種類を変えて上書きしたときに、前の種類のファイルを残さない。
        if self.module.is_some() {
            remove_if_exists(&dir.join(MODEL_JSON))?;
        } else {
            remove_if_exists(&dir.join(MODULE_JSON))?;
        }
        if self.physics.is_none() || self.module.is_some() {
            remove_if_exists(&dir.join(PHYSICS_JSON))?;
        }
        remove_stale(dir, MOTIONS_DIR, MOTION_SUFFIX, &files)?;
        remove_stale(dir, EXPRESSIONS_DIR, EXPRESSION_SUFFIX, &files)?;
        remove_stale(dir, TEXTURES_DIR, TEXTURE_SUFFIX, &files)?;
        // project.json はプロジェクトとして完成したことの印になるので最後に書く。
        write_if_changed(&dir.join(PROJECT_JSON), &meta)
    }

    /// `dir` からプロジェクトを読み込む。
    ///
    /// モデルは検証 (`ugoe_model::validate`) しない (不正なモデルもエディタで開いて直せるように)。
    /// `Model::textures` のすべてと、その `normal_map` のすべてに `textures/<name>.png` が必要。
    /// 参照されない PNG は無視する。
    pub fn load(dir: &Path) -> Result<Self, ProjectError> {
        Self::load_checking_base(dir, &unchecked)
    }

    /// [`Self::load`] と同じだが、モジュールのプロジェクトなら、本体を読む前に解決した本体のパスを `check` に渡し、
    /// 返したパスから読む (T0524)。`check` がエラーを返したら本体を読まずに [`ProjectError::BaseRefused`]。
    /// モデルのプロジェクトでは `check` を呼ばない。
    pub fn load_checking_base(dir: &Path, check: BaseCheck<'_>) -> Result<Self, ProjectError> {
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
            return Self::load_module(dir, meta, None, false, check).map(|(project, _)| project);
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
        // 書き出したバンドルを読み直すと拒まれる値は、プロジェクトの読み込みでも拒む (T1606)。
        let motions = read_json_dir(&dir.join(MOTIONS_DIR), MOTION_SUFFIX, check_motion)?;
        let expressions = read_json_dir(
            &dir.join(EXPRESSIONS_DIR),
            EXPRESSION_SUFFIX,
            check_expression,
        )?;

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
        Self::load_module_with(dir, Some(base), None)
    }

    /// モジュールのプロジェクト `dir` を、本体を `base` に、スロットを `slot` に替えて読む (`None` なら
    /// `project.json` のまま)。本体からスロットがなくなったとき・本体を選び直すときに使う。
    /// 替えたものは、次に保存したときに `project.json` に書く。
    pub fn load_module_with(
        dir: &Path,
        base: Option<&Path>,
        slot: Option<&SlotId>,
    ) -> Result<Self, ProjectError> {
        let meta = module_meta_with(dir, slot)?;
        Self::load_module(dir, meta, base, false, &unchecked).map(|(project, _)| project)
    }

    /// [`Self::load_module_with`] と同じだが、本体に無い (本体に物理が無い) スロットの公開する衝突体へのリグの
    /// 参照を外して読み、外した参照を返す (`ModuleContext::drop_unavailable_colliders`。T0661)。
    /// 外したことは次に保存したときに `module.json` に書く。利用者が外すことを確かめてから使う (T0655 の「黙って消さない」)。
    pub fn load_module_without_unavailable_colliders(
        dir: &Path,
        base: Option<&Path>,
        slot: Option<&SlotId>,
    ) -> Result<(Self, Vec<ColliderReference>), ProjectError> {
        let meta = module_meta_with(dir, slot)?;
        Self::load_module(dir, meta, base, true, &unchecked)
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
            let path = PathBuf::from(PHYSICS_JSON);
            known(&path, ugoe_model::unknown::in_physics(physics))?;
            let json = to_json(&path, physics)?;
            files.push((path, json));
        }

        check_names("motion", self.motions.iter().map(|m| m.name.as_str()))?;
        for motion in &self.motions {
            let path = Path::new(MOTIONS_DIR).join(format!("{}{MOTION_SUFFIX}", motion.name));
            known(&path, ugoe_model::unknown::in_motion(motion))?;
            let json = to_json(&path, motion)?;
            files.push((path, json));
        }
        check_names(
            "expression",
            self.expressions.iter().map(|e| e.name.as_str()),
        )?;
        for expression in &self.expressions {
            let path =
                Path::new(EXPRESSIONS_DIR).join(format!("{}{EXPRESSION_SUFFIX}", expression.name));
            known(&path, ugoe_model::unknown::in_expression(expression))?;
            let json = to_json(&path, expression)?;
            files.push((path, json));
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

/// モジュールのプロジェクトの本体だけを `.ugoe` バンドルのバイト列に書き出す (モデル・画像・物理。モーションと
/// 表情は本体から読まないので含めない)。エディタのプレビューが、本体に今の編集を書き出したモジュールを付けて
/// ほかのモジュールと比べるために使う (`docs/editor.md#プレビューの着せ替え`、T0651)。
pub fn export_base_bundle(project: &Project) -> Result<Vec<u8>, ProjectError> {
    let base = project
        .module
        .as_ref()
        .ok_or(ProjectError::NotAModule)?
        .base();
    let mut base_project = Project::new(
        ProjectMeta::new(base.model.meta.name.clone()),
        base.model.clone(),
        base.textures.clone(),
    );
    base_project.physics.clone_from(&base.physics);
    export_bundle(&base_project)
}

/// [`export_module`] の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleExport {
    /// `.ugoemod` のバイト列。
    pub bytes: Vec<u8>,
    /// 書き出しはしたが知らせたいこと (描画順が 0〜1000 の外にあり、スロットの範囲からはみ出すなど)。
    pub warnings: Vec<Message>,
    /// アトラス化したときの結果 ([`crate::export::export_module_with_settings`])。しなければ `None`。
    pub atlas: Option<crate::atlas::AtlasReport>,
}

/// モジュールのプロジェクトを `.ugoemod` のバイト列に書き出す (`docs/editor.md#モジュールのプロジェクト`)。
///
/// 作業用のモデルからモジュールを取り出し、単体の検証 (`validate_module`) と、本体との合成
/// (`compose`・`compose_physics`) を確かめてから書く。テクスチャと法線マップを含める。
pub fn export_module(project: &Project) -> Result<ModuleExport, ProjectError> {
    export_module_with(project, None)
}

/// [`export_module`] と同じく書き出すが、モジュールの名前を `name` にする (ファイルには書かない。エディタの
/// プレビューが、今の編集を前に書き出した同じ名前の `.ugoemod` と並べて比べるために使う。
/// `docs/editor.md#プレビューの着せ替え`、T0653)。名前が使えなければ [`ModuleError::InvalidName`]。
pub fn export_module_as(project: &Project, name: &str) -> Result<ModuleExport, ProjectError> {
    if !Module::is_valid_name(name) {
        return Err(ModuleError::InvalidName(name.to_owned()).into());
    }
    export_module_with(project, Some(name))
}

fn export_module_with(project: &Project, name: Option<&str>) -> Result<ModuleExport, ProjectError> {
    let (module, images, warnings) = checked_module(project, name)?;
    let file = module_file(module, &images)?;
    let bytes = ugoe_format::write_module(&file).map_err(ProjectError::Export)?;
    Ok(ModuleExport {
        bytes,
        warnings,
        atlas: None,
    })
}

/// モジュールのプロジェクトからモジュールを取り出し、書き出しの検査 (単体の検証と本体との合成) を通す。
/// 画像 (テクスチャと法線マップの名前 → PNG) と、書き出しはできるが知らせたいことを一緒に返す。
/// `name` があればモジュールの名前をそれにする。
pub(crate) fn checked_module(
    project: &Project,
    name: Option<&str>,
) -> Result<(Module, Textures, Vec<Message>), ProjectError> {
    let context = project.module.as_ref().ok_or(ProjectError::NotAModule)?;
    let (mut module, images) =
        context.split(&project.model, project.physics.as_ref(), &project.textures)?;
    if let Some(name) = name {
        name.clone_into(&mut module.name);
    }
    let errors = ugoe_model::validate::validate_module(&module);
    if !errors.is_empty() {
        return Err(ProjectError::InvalidModule(errors));
    }
    let base = context.base();
    let model = ugoe_runtime::compose(&base.model, &[&module]).map_err(ModuleError::from)?;
    if base.physics.is_some() || !module.physics.is_empty() {
        let composed = context.compose_physics(&module)?;
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
            warnings.push(Message::new(
                "module.draw_order_outside_slot",
                crate::message_args! { "mesh" => mesh.id.to_string() },
                format!(
                    "mesh `{}`: the draw order is outside the slot range, so it is drawn outside the slot",
                    mesh.id
                ),
            ));
        }
    }
    Ok((module, images, warnings))
}

/// モジュールと画像 (名前 → PNG) を `.ugoemod` の中身にする。テクスチャと法線マップを名前で引く。
pub(crate) fn module_file(
    module: Module,
    images: &Textures,
) -> Result<ugoe_format::ModuleFile, ProjectError> {
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
    Ok(ugoe_format::ModuleFile {
        module,
        textures,
        normal_maps,
        meta: None,
    })
}

/// モーションの数値の検査 (`validate_motion`)。バンドルの読み込みと同じく [`FormatError::InvalidMotion`] で拒む
/// (`docs/spec/model.md#モーションと表情の検証`)。
fn check_motion(motion: &Motion) -> Result<(), FormatError> {
    let errors = ugoe_model::validate::validate_motion(motion);
    if errors.is_empty() {
        return Ok(());
    }
    Err(FormatError::InvalidMotion {
        name: motion.name.clone(),
        errors,
    })
}

/// 表情の数値の検査 (`validate_expression`)。通らなければ [`FormatError::InvalidExpression`]。
fn check_expression(expression: &Expression) -> Result<(), FormatError> {
    let errors = ugoe_model::validate::validate_expression(expression);
    if errors.is_empty() {
        return Ok(());
    }
    Err(FormatError::InvalidExpression {
        name: expression.name.clone(),
        errors,
    })
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

fn texture_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(TEXTURES_DIR)
        .join(format!("{name}{TEXTURE_SUFFIX}"))
}
