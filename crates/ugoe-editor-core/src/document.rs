use std::collections::BTreeSet;
use std::fmt::Display;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use ugoe_model::deformer::DeformerKind;
use ugoe_model::expression::Expression;
use ugoe_model::module::{ModuleRequirements, SlotId};
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, validate, validate_physics};
use ugoe_model::{
    Canvas, DeformerId, KeyformOrigin, MeshId, Model, Module, ParamId, PartId, TextureRef,
};
use ugoe_rigging::{InferError, InferScope, MirrorOptions};

use crate::command::KeyformTarget;
use crate::grid::{self, CellError};
use crate::inference::{MirrorReport, RederiveWarning};
use crate::module_project::{ModuleBase, ModuleContext, ModuleError};
use crate::motion_edit::{self, Animations};
use crate::texture::{Textures, is_valid_texture_name, png_size};
use crate::{Command, automesh, avatar, inference, keyform};
use crate::{
    deformer_edit, depth, draw_clone, mesh_edit, physics_edit, physics_template, slot_edit,
};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EditError {
    /// `kind` は要素の種類 (`parameter`、`part`、`deformer`、`mesh`)。
    #[error("{kind} `{id}` already exists")]
    DuplicateId { kind: &'static str, id: String },
    #[error("{kind} `{id}` not found")]
    NotFound { kind: &'static str, id: String },
    #[error("invalid range: min {min} <= default {default} <= max {max} must hold")]
    InvalidRange { min: f32, default: f32, max: f32 },
    /// 子が残っている要素は削除できない。`children` は `mesh \`Quad\`` のような表記。
    #[error("{kind} `{id}` still has children: {}", children.join(", "))]
    HasChildren {
        kind: &'static str,
        id: String,
        children: Vec<String>,
    },
    /// `field` が自分自身を指している (並べ替えの基準など)。
    #[error("{kind} `{id}`: {field} must not refer to itself")]
    SelfReference {
        kind: &'static str,
        id: String,
        field: &'static str,
    },
    #[error("deformer `{id}` is not a {expected} deformer")]
    DeformerKindMismatch { id: String, expected: &'static str },
    #[error("{kind} `{id}`: invalid keyform cell: {error}")]
    InvalidCell {
        kind: &'static str,
        id: String,
        error: CellError,
    },
    /// テクスチャ名がファイル名に使えない (`crate::texture::is_valid_texture_name`)。
    #[error("texture name `{name}` is invalid (use [A-Za-z0-9_.-], not starting with `.`)")]
    InvalidTextureName { name: String },
    /// テクスチャの画像が PNG でない (または IHDR が読めない)。
    #[error("texture `{name}`: image is not a PNG")]
    NotPng { name: String },
    /// キャンバスの大きさが 0 以下か有限でない。
    #[error("invalid canvas size {width}x{height}")]
    InvalidCanvas { width: f32, height: f32 },
    /// メッシュを作り直せない (`Command::RegenerateMesh` の条件を満たさない)。
    #[error("mesh `{id}` cannot be regenerated: {reason}")]
    CannotRegenerateMesh { id: String, reason: String },
    /// 自動メッシュの生成に失敗した (画像が透明、設定が不正など)。
    #[error("mesh `{id}`: automatic meshing failed: {message}")]
    Mesher { id: String, message: String },
    #[error("mesh `{id}`: vertex {index} out of range (vertices = {vertices})")]
    VertexOutOfRange {
        id: String,
        index: u16,
        vertices: usize,
    },
    /// 頂点の削除で、メッシュの頂点が 3 つ未満になる。
    #[error("mesh `{id}`: only {remaining} vertices would remain (at least 3 are needed)")]
    TooFewVertices { id: String, remaining: usize },
    /// メッシュの形の編集ができない (退化した三角形・同じ UV の頂点・評価できないモデルなど)。
    #[error("mesh `{id}`: {reason}")]
    MeshGeometry { id: String, reason: String },
    /// 描画順が有限の値でない。
    #[error("mesh `{id}`: draw order {draw_order} is not finite")]
    InvalidDrawOrder { id: String, draw_order: f32 },
    /// `SetKeyformCell` のキーフォームの種類が、対象の格子の種類 (`expected`) と違う。
    #[error("{kind} `{id}`: expected a {expected} keyform, got a {actual} keyform")]
    KeyformKindMismatch {
        kind: &'static str,
        id: String,
        expected: &'static str,
        actual: &'static str,
    },
    /// 適用するとモデルが検証 (`ugoe_model::validate`) に通らなくなる。新たに生じるエラーの一覧。
    #[error("command would make the model invalid: {}", join(.0))]
    Invalid(Vec<ValidationError>),
    /// キーフォームの推定 (ミラー・`derived` のセルの作り直しを含む) に失敗した。
    /// `target` は書き込み先の格子の要素 (``mesh `EyeL` `` のような表記)。
    #[error("{target}: keyform inference failed: {error}")]
    Inference { target: String, error: InferError },
    /// 格子の推定の方法が未知 (新しい版のファイル) なので、その設定では推定できない。
    #[error("{target}: keyform inference method is not supported by this version")]
    UnsupportedInference { target: String },
    /// 格子に設定できない補間の種類 (未知の値、パーツ・グルーの格子への曲線補間)。
    #[error("{target}: keyform interpolation `{interpolation:?}` is not supported for this grid")]
    UnsupportedInterpolation {
        target: String,
        interpolation: ugoe_model::KeyformInterpolation,
    },
    /// デフォーマの作成・付け替え・セルの編集ができない (子がない、循環する、座標を写せないなど)。
    #[error("deformer edit failed: {reason}")]
    DeformerEdit { reason: String },
    /// 深度の長さが要素の点 (頂点・制御点) の数と違う。
    #[error("{target}: depth has {actual} values but the element has {expected} points")]
    DepthLength {
        target: String,
        expected: usize,
        actual: usize,
    },
    /// 深度に有限でない値がある。
    #[error("{target}: depth value {value} is not finite")]
    NonFiniteDepth { target: String, value: f32 },
    /// 深度を設定する点の番号が範囲外。
    #[error("{target}: point {index} out of range (points = {points})")]
    DepthPointOutOfRange {
        target: String,
        index: u32,
        points: usize,
    },
    /// 左右の対 (`mirror`) を持てない要素 (パーツ・グルー)。
    #[error("{target}: only meshes and deformers have mirror pairs")]
    MirrorPairNotSupported { target: String },
    /// モーション・表情の名前がファイル名に使えない (`[A-Za-z0-9_.-]`、`.` 始まりは不可)。
    #[error("{kind} name `{name}` is invalid (use [A-Za-z0-9_.-], not starting with `.`)")]
    InvalidName { kind: &'static str, name: String },
    /// 編集するとモーションに問題 (長さ・フェード・キーの時刻の順・取っ手の範囲など) が生じる。
    #[error("motion `{motion}`: {reason}")]
    InvalidMotion { motion: String, reason: String },
    /// 編集すると表情に問題 (フェード・値・同じパラメータの重複など) が生じる。
    #[error("expression `{expression}`: {reason}")]
    InvalidExpression { expression: String, reason: String },
    /// 物理の設定の値が不正 (有限でない、範囲外、粒子がないなど。`docs/editor.md#物理の編集`)。
    #[error("invalid physics settings: {reason}")]
    InvalidPhysics { reason: String },
    /// 描画用の複製の形 (頂点・三角形) は元に従うので、直接は編集できない
    /// (`docs/editor.md#描画用の複製`)。
    #[error("mesh `{id}` is a draw clone of `{original}`; edit the shape of the original")]
    DrawCloneShape { id: String, original: String },
    /// 描画用の複製でも、複製を持つメッシュでもない。
    #[error("mesh `{id}` is neither a draw clone nor has draw clones")]
    NotDrawClone { id: String },
    /// アバタープロファイルの値が不正 (範囲外のレベル・有限でない数・未知の値など。
    /// `docs/editor.md#アバタープロファイルの編集`)。
    #[error("invalid avatar profile: {reason}")]
    InvalidAvatarProfile { reason: String },
    /// 法線マップの指定が不正 (`Command::SetNormalMap`)。
    #[error("texture `{texture}`: invalid normal map: {reason}")]
    InvalidNormalMap { texture: String, reason: String },
    /// スロットの指定が不正 (ID の形式・ほかのスロットとの描画順の範囲の重なり。
    /// `docs/editor.md#スロットの編集`)。
    #[error("slot `{id}`: {reason}")]
    InvalidSlot { id: String, reason: String },
    /// モジュールのプロジェクトで、読み取り専用の本体の要素を変えようとした
    /// (`element` は ``parameter `AngleX` `` のような表記。`docs/editor.md#モジュールのプロジェクト`)。
    #[error(
        "{element} belongs to the base model and can't be edited here. Edit it in the base project"
    )]
    ReadOnlyBase { element: String },
    /// モジュールのプロジェクトで、編集した結果がモジュールとして表せない
    /// (本体の要素への参照・接頭辞を外した ID の重なりなど)。
    #[error("{reason}")]
    ModuleShape { reason: String },
    /// モジュールのプロジェクトでないのに、モジュールの設定を変えようとした (`Command::SetModuleSettings`)。
    #[error("this is not a module project")]
    NotModule,
}

/// [`Document::apply_with_report`] の結果: コマンドは適用したが、利用者に知らせたいこと。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct ApplyReport {
    /// 自動の作り直しで失敗し、前の値のまま残したセルがある格子と、親を変えたことでミラーし直せなく
    /// なった左右の対。
    pub warnings: Vec<RederiveWarning>,
    /// 明示的に実行したミラー (`MirrorKeyforms`) ごとの、書いたセルと残したセルの数。
    pub mirrored: Vec<MirrorReport>,
}

fn join(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// 編集中のモデル・テクスチャ・物理の設定・モーション・表情と履歴。
///
/// 暫定実装としてスナップショット方式の Undo を使う (`docs/adr/0005-editor-command-model.md`)。
/// テクスチャの画像は `Arc` で共有するので、スナップショットでバイト列は複製されない。
#[derive(Debug)]
pub struct Document {
    current: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// 次に作る状態の番号。
    next_state: u64,
    /// 最後に保存した状態の番号 (未保存なら `None`)。
    saved: Option<u64>,
}

/// ある時点のドキュメントの状態。`state` は状態ごとに一意な番号 (変更の有無の判定に使う)。
#[derive(Debug, Clone)]
struct Snapshot {
    model: Model,
    textures: Textures,
    animations: Animations,
    /// 物理の設定 (`docs/editor.md#物理の編集`)。モデルと同じ Undo の単位で変える。
    physics: Option<PhysicsSettings>,
    /// モジュールのプロジェクトなら、本体とスロット・名前・条件 (`crate::module_project`)。
    /// このときモデルは本体と合成した作業用のモデルで、本体の部分は読み取り専用。
    /// 名前・スロット・条件は [`Command::SetModuleSettings`] で変わるので、Undo の単位に含める。
    module: Option<Arc<ModuleContext>>,
    state: u64,
}

impl Document {
    /// テクスチャの画像を持たないドキュメント。
    pub fn new(model: Model) -> Self {
        Self::with_textures(model, Textures::new())
    }

    /// モデルとテクスチャの画像 (`Model::textures` の名前 → PNG) からドキュメントを作る。
    /// 作った時点の状態は「保存済み」とみなす。
    pub fn with_textures(model: Model, textures: Textures) -> Self {
        Self {
            current: Snapshot {
                model,
                textures,
                animations: Animations::default(),
                physics: None,
                module: None,
                state: 0,
            },
            undo: Vec::new(),
            redo: Vec::new(),
            next_state: 1,
            saved: Some(0),
        }
    }

    /// 作ったばかりのドキュメントを、モジュールのプロジェクトのものにする (モデルは `context` の作業用のモデル)。
    pub fn with_module(mut self, context: Arc<ModuleContext>) -> Self {
        self.current.module = Some(context);
        self
    }

    /// モジュールのプロジェクトなら、その本体とスロット。
    pub fn module(&self) -> Option<&Arc<ModuleContext>> {
        self.current.module.as_ref()
    }

    /// モジュールのプロジェクトなら、今の状態をモジュールとその画像に分ける。
    pub fn split_module(&self) -> Option<Result<(Module, Textures), ModuleError>> {
        let context = self.current.module.as_ref()?;
        Some(context.split(
            &self.current.model,
            self.current.physics.as_ref(),
            &self.current.textures,
        ))
    }

    /// 作ったばかりのドキュメントに、モーションと表情を持たせる (保存済みの状態のまま)。
    pub fn with_animations(mut self, motions: Vec<Motion>, expressions: Vec<Expression>) -> Self {
        self.current.animations = Animations {
            motions,
            expressions,
        };
        self
    }

    pub fn model(&self) -> &Model {
        &self.current.model
    }

    /// モーション (保存・書き出しの順)。
    pub fn motions(&self) -> &[Motion] {
        &self.current.animations.motions
    }

    /// 表情 (保存・書き出しの順)。
    pub fn expressions(&self) -> &[Expression] {
        &self.current.animations.expressions
    }

    /// 物理の設定を持たせる (作った時点の状態として。保存済みのまま)。
    pub fn with_physics(mut self, physics: Option<PhysicsSettings>) -> Self {
        self.current.physics = physics;
        self
    }

    /// 物理の設定。
    pub fn physics(&self) -> Option<&PhysicsSettings> {
        self.current.physics.as_ref()
    }

    /// テクスチャ名 → PNG のバイト列。
    pub fn textures(&self) -> &Textures {
        &self.current.textures
    }

    /// 名前でテクスチャの PNG を引く。
    pub fn texture(&self, name: &str) -> Option<&[u8]> {
        self.current.textures.get(name).map(|png| &**png)
    }

    /// 最後に [`Document::mark_saved`] した (または作った) ときから状態が変わっているか。
    /// Undo で保存時の状態に戻れば `false` に戻る。
    pub fn is_modified(&self) -> bool {
        self.saved != Some(self.current.state)
    }

    /// 現在の状態を「保存済み」にする。
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.current.state);
    }

    /// 現在の状態の番号。状態ごとに一意で、Undo / Redo で戻れば同じ番号に戻る
    /// (提案が作られた状態のままかを確かめるのに使う。`crate::proposal`)。
    pub fn state_id(&self) -> u64 {
        self.current.state
    }

    /// 現在の状態だけを持つ複製 (履歴は持たず、未保存の扱い)。元を変えずにコマンドを試すときに使う。
    pub fn detached(&self) -> Document {
        Document {
            current: self.current.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            next_state: self.next_state,
            saved: None,
        }
    }

    /// `commands` を検証せずに順に適用し、それぞれが新しく作った要素の ID を返す (提案の依存の解析用。
    /// ドキュメントは変えない)。適用に失敗したコマンドとその後のものは空とする。
    pub(crate) fn created_ids(&self, commands: &[Command]) -> Vec<BTreeSet<String>> {
        let mut next = self.current.clone();
        let mut created = Vec::with_capacity(commands.len());
        let mut before = element_ids(&next);
        for command in commands {
            let ok = apply_to(
                &mut next.model,
                &mut next.textures,
                &mut next.animations,
                command.clone(),
                &mut Vec::new(),
                &mut Vec::new(),
                &mut next.physics,
            )
            .is_ok();
            if !ok {
                break;
            }
            let after = element_ids(&next);
            created.push(after.difference(&before).cloned().collect());
            before = after;
        }
        created.resize_with(commands.len(), BTreeSet::new);
        created
    }

    /// コマンドを適用する。失敗時はモデルを変更しない。
    ///
    /// 適用後のモデルを `ugoe_model::validate::validate` で検査し、適用前になかった
    /// 検証エラーが生じたら [`EditError::Invalid`] で拒否する。検証を通るモデルに対しては、
    /// 成功した適用の後も必ず検証を通る。`Batch` は全体を適用した後に 1 回だけ検査する。
    ///
    /// 検査の前に、コマンドが編集したキーフォーム格子の `derived` のセルをセルに記録した推定の方法で
    /// 作り直し、その格子をミラー元にする格子のミラーのセルも作り直す (同じ Undo 単位。
    /// `docs/editor.md#推定とミラー`)。作り直せなければ拒否する。ただしミラーのやり直しの失敗は
    /// 拒否せず、そのセルを前の値のまま残す (知らせるには [`Document::apply_with_report`] を使う)。
    pub fn apply(&mut self, command: Command) -> Result<(), EditError> {
        self.apply_with_report(command).map(drop)
    }

    /// [`Document::apply`] と同じく適用し、拒否しなかった問題 (ミラーのやり直しの失敗と、親を変えた
    /// ことでミラーし直せなくなった左右の対) を返す。
    pub fn apply_with_report(&mut self, command: Command) -> Result<ApplyReport, EditError> {
        if let Command::SetModuleSettings {
            name,
            slot,
            requires,
            keep_look,
        } = command
        {
            self.set_module_settings(name, &slot, requires, keep_look)?;
            return Ok(ApplyReport::default());
        }
        let mut next = self.current.clone();
        let mut touched = Vec::new();
        let mut mirrored = Vec::new();
        apply_to(
            &mut next.model,
            &mut next.textures,
            &mut next.animations,
            command,
            &mut touched,
            &mut mirrored,
            &mut next.physics,
        )?;
        if let Some(context) = &next.module {
            context.normalize(&mut next.model, &mut touched)?;
        }
        // 形・テクスチャ・親が変わった元のメッシュに、描画用の複製を追従させる。
        draw_clone::follow(&self.current.model, &mut next.model)?;
        let mut warnings = inference::rederive(&mut next.model, &touched)?;
        inference::check_reparented_pairs(&self.current.model, &next.model, &mut warnings);
        physics_edit::forget_removed(&self.current.model, &next.model, &mut next.physics);
        slot_edit::forget_removed_colliders(
            self.current.physics.as_ref(),
            next.physics.as_ref(),
            &mut next.model,
        );
        let before = validate(&self.current.model);
        let mut introduced: Vec<_> = validate(&next.model)
            .into_iter()
            .filter(|e| !before.contains(e))
            .collect();
        // 物理の参照先と値の検査 (`docs/spec/model.md#物理の検証`)。
        if let Some(physics) = &next.physics {
            let before = self
                .current
                .physics
                .as_ref()
                .map(|p| validate_physics(p, &self.current.model))
                .unwrap_or_default();
            introduced.extend(
                validate_physics(physics, &next.model)
                    .into_iter()
                    .filter(|e| !before.contains(e)),
            );
        }
        if !introduced.is_empty() {
            return Err(EditError::Invalid(introduced));
        }
        if let Some(context) = &next.module {
            // 本体が変わっていないこと、モジュールとして表せることを確かめる。
            if !next.animations.motions.is_empty() || !next.animations.expressions.is_empty() {
                return Err(EditError::ModuleShape {
                    reason: "a module has no motions or expressions".into(),
                });
            }
            context.split(&next.model, next.physics.as_ref(), &next.textures)?;
        }
        next.state = self.next_state;
        self.next_state += 1;
        self.undo.push(std::mem::replace(&mut self.current, next));
        self.redo.clear();
        Ok(ApplyReport { warnings, mirrored })
    }

    /// [`Command::SetModuleSettings`]: 作業用のモデルをモジュールに分け、新しい名前・スロット・条件で
    /// 本体と合成し直す (`docs/editor.md#モジュールの設定`)。モジュールの要素の座標は取り付け先の局所座標のまま
    /// なので、取り付け先の違うスロットに移すと見た目の位置が変わる。`keep_look` なら新しい取り付け先の局所座標に
    /// 写し直して見た目を保つ ([`ModuleContext::keep_look`])。変わらなければ何もしない。
    fn set_module_settings(
        &mut self,
        name: String,
        slot: &SlotId,
        requires: ModuleRequirements,
        keep_look: bool,
    ) -> Result<(), EditError> {
        let context = self.current.module.as_ref().ok_or(EditError::NotModule)?;
        if context.name() == name && &context.slot().id == slot && *context.requires() == requires {
            return Ok(());
        }
        let (module, images) = context.split(
            &self.current.model,
            self.current.physics.as_ref(),
            &self.current.textures,
        )?;
        let reconfigured = context.reconfigured(slot, name, requires)?;
        let mut workspace = reconfigured.workspace(&module, &images)?;
        if keep_look {
            let mut touched = Vec::new();
            reconfigured.keep_look(
                &mut workspace.model,
                context.slot().anchor.as_ref(),
                &mut touched,
            )?;
            inference::rederive(&mut workspace.model, &touched)?;
            let before = validate(&self.current.model);
            let introduced: Vec<_> = validate(&workspace.model)
                .into_iter()
                .filter(|e| !before.contains(e))
                .collect();
            if !introduced.is_empty() {
                return Err(EditError::Invalid(introduced));
            }
            reconfigured.split(
                &workspace.model,
                workspace.physics.as_ref(),
                &workspace.textures,
            )?;
        }
        let next = Snapshot {
            model: workspace.model,
            textures: workspace.textures,
            animations: self.current.animations.clone(),
            physics: workspace.physics,
            module: Some(Arc::new(reconfigured)),
            state: self.next_state,
        };
        self.next_state += 1;
        self.undo.push(std::mem::replace(&mut self.current, next));
        self.redo.clear();
        Ok(())
    }

    /// モジュールのプロジェクトの本体を `base` に替える (本体のファイルが外で変わったときの読み直し。
    /// `docs/editor.md#本体の変化`)。保存していないモジュールの編集は保つ。前の状態は古い本体のものなので
    /// Undo の履歴は消す。モジュールの中身は変わらないので、保存済みかどうかは保つ。
    /// 本体がスロットや条件を満たさなくなった・モジュールが合わなくなったならエラーで、何も変えない。
    pub fn rebase_module(&mut self, base: ModuleBase) -> Result<(), ModuleError> {
        self.rebase_module_on(base, None)
    }

    /// [`Document::rebase_module`] と同じく本体を `base` に替え、`slot` があればスロットも替える
    /// (本体を選び直した・本体からスロットがなくなったときに、開き直さずに直す。T0651)。
    /// 取り付け先の局所座標はそのまま使う。
    pub fn rebase_module_on(
        &mut self,
        base: ModuleBase,
        slot: Option<&SlotId>,
    ) -> Result<(), ModuleError> {
        let Some(context) = self.current.module.as_ref() else {
            return Err(ModuleError::Shape {
                reason: "this is not a module project".into(),
            });
        };
        let (module, images) = context.split(
            &self.current.model,
            self.current.physics.as_ref(),
            &self.current.textures,
        )?;
        let rebased = match slot {
            Some(slot) => ModuleContext::new(
                base,
                slot,
                context.name().to_owned(),
                context.requires().clone(),
            )?,
            None => context.rebased(base)?,
        };
        let workspace = rebased.workspace(&module, &images)?;
        // スロットが替われば `module.json` の中身も変わる。
        let saved = !self.is_modified() && rebased.slot().id == context.slot().id;
        self.current = Snapshot {
            model: workspace.model,
            textures: workspace.textures,
            animations: self.current.animations.clone(),
            physics: workspace.physics,
            module: Some(Arc::new(rebased)),
            state: self.next_state,
        };
        self.next_state += 1;
        self.undo.clear();
        self.redo.clear();
        self.saved = saved.then_some(self.current.state);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.current, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.current, next));
                true
            }
            None => false,
        }
    }
}

/// 状態の中の要素の ID (パラメータ・グループ・パーツ・デフォーマ・メッシュ・グルー・テクスチャ・当たり判定・スロット・
/// 物理のリグと衝突体・モーションと表情の名前)。
fn element_ids(snapshot: &Snapshot) -> BTreeSet<String> {
    let m = &snapshot.model;
    let mut ids = BTreeSet::new();
    ids.extend(m.parameters.iter().map(|p| p.id.to_string()));
    ids.extend(m.parameter_groups.iter().map(|g| g.id.clone()));
    ids.extend(m.parts.iter().map(|p| p.id.to_string()));
    ids.extend(m.deformers.iter().map(|d| d.id.to_string()));
    ids.extend(m.meshes.iter().map(|x| x.id.to_string()));
    ids.extend(m.glues.iter().map(|g| g.id.clone()));
    ids.extend(m.textures.iter().map(|t| t.name.clone()));
    ids.extend(m.hit_areas.iter().map(|h| h.id.clone()));
    ids.extend(m.slots.iter().map(|s| s.id.to_string()));
    if let Some(physics) = &snapshot.physics {
        ids.extend(physics.rigs.iter().map(|r| r.id.clone()));
        ids.extend(physics.colliders.iter().map(|c| c.id.clone()));
    }
    ids.extend(snapshot.animations.motions.iter().map(|x| x.name.clone()));
    ids.extend(
        snapshot
            .animations
            .expressions
            .iter()
            .map(|x| x.name.clone()),
    );
    ids
}

/// ID で要素を探す。
fn find_mut<'a, T, K: PartialEq + Display>(
    items: &'a mut [T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<&'a mut T, EditError> {
    items
        .iter_mut()
        .find(|item| id_of(item) == id)
        .ok_or_else(|| EditError::NotFound {
            kind,
            id: id.to_string(),
        })
}

/// ID が未使用であることを確かめる。
fn ensure_new<T, K: PartialEq + Display>(
    items: &[T],
    kind: &'static str,
    id: &K,
    id_of: impl Fn(&T) -> &K,
) -> Result<(), EditError> {
    if items.iter().any(|item| id_of(item) == id) {
        return Err(EditError::DuplicateId {
            kind,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// 子の一覧が空でなければ [`EditError::HasChildren`] にする。
fn ensure_no_children(
    kind: &'static str,
    id: &impl Display,
    children: Vec<String>,
) -> Result<(), EditError> {
    if children.is_empty() {
        return Ok(());
    }
    Err(EditError::HasChildren {
        kind,
        id: id.to_string(),
        children,
    })
}

fn parameter<'a>(
    model: &'a mut Model,
    id: &ParamId,
) -> Result<&'a mut ugoe_model::Parameter, EditError> {
    find_mut(&mut model.parameters, "parameter", id, |p| &p.id)
}

fn part<'a>(model: &'a mut Model, id: &PartId) -> Result<&'a mut ugoe_model::Part, EditError> {
    find_mut(&mut model.parts, "part", id, |p| &p.id)
}

fn deformer<'a>(
    model: &'a mut Model,
    id: &DeformerId,
) -> Result<&'a mut ugoe_model::Deformer, EditError> {
    find_mut(&mut model.deformers, "deformer", id, |d| &d.id)
}

fn mesh<'a>(model: &'a mut Model, id: &MeshId) -> Result<&'a mut ugoe_model::Mesh, EditError> {
    find_mut(&mut model.meshes, "mesh", id, |m| &m.id)
}

/// コマンドを適用する。
///
/// キーフォーム格子のセル (値や由来) を変えたコマンドは、その格子の要素を `touched` に積む。
/// 積んだ格子の `derived` のセルは、[`Document::apply`] が適用の後にまとめて作り直す
/// ([`inference::rederive`])。新しいコマンドもセルを変えたら積むこと。推定そのもの
/// (`InferKeyforms`・`MirrorKeyforms`) も積む: 作り直しはセルに記録した方法で行うので、推定の結果を
/// 別の方法で上書きせず、その格子をミラー元にする格子にも変化を伝える。
/// 明示的に実行したミラーの結果は `mirrored` に積む。
fn apply_to(
    model: &mut Model,
    textures: &mut Textures,
    anim: &mut Animations,
    command: Command,
    touched: &mut Vec<KeyformTarget>,
    mirrored: &mut Vec<MirrorReport>,
    physics: &mut Option<PhysicsSettings>,
) -> Result<(), EditError> {
    match command {
        Command::AddParameter { parameter } => {
            ensure_new(&model.parameters, "parameter", &parameter.id, |p| &p.id)?;
            model.parameters.push(parameter);
        }
        Command::RemoveParameter { id } => remove_parameter(model, &id)?,
        Command::SetParameterRange {
            id,
            min,
            default,
            max,
        } => {
            if !(min <= default && default <= max) {
                return Err(EditError::InvalidRange { min, default, max });
            }
            let p = parameter(model, &id)?;
            (p.min, p.default, p.max) = (min, default, max);
        }
        Command::Batch { commands, .. } => {
            for c in commands {
                apply_to(model, textures, anim, c, touched, mirrored, physics)?;
            }
        }
        Command::SetAvatarProfile { profile } => avatar::set_profile(model, profile)?,
        Command::UpdateAvatarProfile {
            level,
            meta,
            tracking,
            look_at,
            hit_areas,
            slots,
        } => avatar::update_profile(
            model,
            avatar::ProfileUpdate {
                level,
                meta,
                tracking,
                look_at,
                hit_areas,
                slots,
            },
        )?,
        Command::SetHitAreas { hit_areas } => avatar::set_hit_areas(model, hit_areas)?,
        Command::RenameParameter { id, name } => parameter(model, &id)?.name = name,
        Command::SetParameterGroup { id, group } => parameter(model, &id)?.group = group,
        Command::SetParameterMirror { id, mirror } => {
            pair(
                &mut model.parameters,
                "parameter",
                &id,
                mirror,
                |p| &p.id,
                |p| &mut p.mirror,
            )?;
        }
        Command::AddPart { part } => {
            ensure_new(&model.parts, "part", &part.id, |p| &p.id)?;
            model.parts.push(part);
        }
        Command::RemovePart { id } => {
            remove_part(model, &id)?;
            inference::forget_missing_mirror_sources(model);
        }
        Command::RenamePart { id, name } => part(model, &id)?.name = name,
        Command::SetPartParent { id, parent } => part(model, &id)?.parent = parent,
        Command::SetPartKeyforms { id, keyforms } => {
            grid::replace(&mut part(model, &id)?.keyforms, keyforms);
            touched.push(KeyformTarget::Part(id));
        }
        Command::AddDeformer { deformer } => {
            ensure_new(&model.deformers, "deformer", &deformer.id, |d| &d.id)?;
            model.deformers.push(deformer);
        }
        Command::RemoveDeformer { id } => {
            remove_deformer(model, &id)?;
            inference::forget_missing_mirror_sources(model);
        }
        Command::RenameDeformer { id, name } => deformer(model, &id)?.name = name,
        Command::SetDeformerParent { id, parent } => deformer(model, &id)?.parent = parent,
        Command::SetDeformerPart { id, part } => deformer(model, &id)?.part = part,
        Command::SetWarpKeyforms { id, keyforms } => {
            match &mut deformer(model, &id)?.kind {
                DeformerKind::Warp(warp) => grid::replace(&mut warp.keyforms, keyforms),
                DeformerKind::Rotation(_) | DeformerKind::Unknown => {
                    return Err(EditError::DeformerKindMismatch {
                        id: id.to_string(),
                        expected: "warp",
                    });
                }
            }
            touched.push(KeyformTarget::Deformer(id));
        }
        Command::SetRotationKeyforms { id, keyforms } => {
            match &mut deformer(model, &id)?.kind {
                DeformerKind::Rotation(rotation) => grid::replace(&mut rotation.keyforms, keyforms),
                DeformerKind::Warp(_) | DeformerKind::Unknown => {
                    return Err(EditError::DeformerKindMismatch {
                        id: id.to_string(),
                        expected: "rotation",
                    });
                }
            }
            touched.push(KeyformTarget::Deformer(id));
        }
        Command::AddMesh { mesh } => {
            ensure_new(&model.meshes, "mesh", &mesh.id, |m| &m.id)?;
            model.meshes.push(mesh);
        }
        Command::RemoveMesh { id } => {
            remove_mesh(model, &id)?;
            inference::forget_missing_mirror_sources(model);
        }
        Command::RenameMesh { id, name } => mesh(model, &id)?.name = name,
        Command::SetMeshProperties {
            id,
            part,
            texture,
            blend,
            culling,
            masks,
            mask_mode,
            mask_only,
            lit,
        } => {
            let m = mesh(model, &id)?;
            if let Some(part) = part {
                m.part = part;
            }
            if let Some(texture) = texture {
                m.texture = texture;
            }
            if let Some(blend) = blend {
                m.blend = blend;
            }
            if let Some(culling) = culling {
                m.culling = culling;
            }
            if let Some(masks) = masks {
                m.masks = masks;
            }
            if let Some(mask_mode) = mask_mode {
                m.mask_mode = mask_mode;
            }
            if let Some(mask_only) = mask_only {
                m.mask_only = mask_only;
            }
            if let Some(lit) = lit {
                m.lit = lit;
            }
        }
        Command::SetMeshParent { id, parent } => mesh(model, &id)?.parent = parent,
        Command::SetMeshKeyforms { id, keyforms } => {
            grid::replace(&mut mesh(model, &id)?.keyforms, keyforms);
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::SetMeshKeyform { id, cell, keyform } => {
            let shapes = keyform::blend_shape_parameters(model);
            let invalid = |error| EditError::InvalidCell {
                kind: "mesh",
                id: id.to_string(),
                error,
            };
            let m = mesh(model, &id)?;
            // ブレンドシェイプのパラメータを含む指定は、ブレンドシェイプ格子のセルを指す。
            let g = grid::cell_grid_mut(&mut m.keyforms, &cell, |p| shapes.contains(p))
                .map_err(invalid)?;
            let index = grid::cell_index(g, &cell).map_err(invalid)?;
            g.keyforms[index] = keyform;
            // 手で編集したセルは `authored` になる (`docs/spec/keyform-inference.md#セルの由来`)。
            grid::set_authored(g, index);
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::ReorderMesh { id, before } => reorder_mesh(model, &id, before.as_ref())?,
        Command::AddTexture { name, png } => add_texture(model, textures, name, png)?,
        Command::SetTextureImage { name, png } => set_texture_image(model, textures, name, png)?,
        Command::RemoveTexture { name } => remove_texture(model, textures, &name)?,
        Command::SetCanvas {
            width,
            height,
            origin,
        } => {
            if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
                return Err(EditError::InvalidCanvas { width, height });
            }
            model.meta.canvas = Canvas {
                width,
                height,
                origin,
            };
        }
        Command::RegenerateMesh { id, options } => {
            draw_clone::reject_shape_edit(model, &id)?;
            automesh::regenerate_mesh(model, textures, &id, &options)?;
        }
        Command::InferKeyforms {
            target,
            method,
            overwrite_derived,
        } => {
            inference::infer_keyforms(model, &target, method, overwrite_derived)?;
            touched.push(target);
        }
        Command::MirrorKeyforms {
            source,
            target,
            axis,
            tolerance,
            overwrite_authored,
        } => {
            let options = MirrorOptions {
                axis,
                tolerance,
                overwrite_authored,
                scope: InferScope::AllDerived,
            };
            mirrored.push(inference::mirror(model, &source, &target, options)?);
            touched.push(target);
        }
        Command::MarkKeyformAuthored { target, cells } => {
            inference::mark(model, &target, &cells, KeyformOrigin::Authored)?;
            touched.push(target);
        }
        Command::MarkKeyformDerived { target, cells } => {
            inference::mark(model, &target, &cells, KeyformOrigin::Derived)?;
            touched.push(target);
        }
        Command::SetKeyformInference { target, inference } => {
            inference::set_inference(model, &target, inference)?;
            touched.push(target);
        }
        // 対と対称軸はミラーのやり直しに使うので、対が変わった要素をミラー元にする格子を作り直す。
        Command::SetMirrorPair {
            target,
            mirror,
            axis,
        } => {
            touched.extend(set_mirror_pair(model, &target, mirror, axis)?);
        }
        // 頂点の編集はすべてのセルの位置を変えるので、`derived` のセルは作り直す。
        Command::MoveMeshVertices { id, moves } => {
            draw_clone::reject_shape_edit(model, &id)?;
            mesh_edit::move_vertices(model, &id, &moves)?;
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::AddMeshVertex { id, position } => {
            draw_clone::reject_shape_edit(model, &id)?;
            mesh_edit::add_vertex(model, &id, position)?;
            physics_edit::vertex_added(model, physics, &id);
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::RemoveMeshVertices { id, indices } => {
            draw_clone::reject_shape_edit(model, &id)?;
            mesh_edit::remove_vertices(model, &id, &indices)?;
            physics_edit::vertices_removed(physics, &id, &indices);
            touched.push(KeyformTarget::Mesh(id));
        }
        Command::RetriangulateMesh { id } => {
            draw_clone::reject_shape_edit(model, &id)?;
            mesh_edit::retriangulate(model, &id)?;
        }
        Command::AddKeyformKey {
            target,
            parameter,
            key,
        } => keyform::add_key(model, &target, &parameter, key)?,
        Command::RemoveKeyformKey {
            target,
            parameter,
            key,
        } => keyform::remove_key(model, &target, &parameter, key)?,
        Command::SetKeyformCell {
            target,
            cell,
            keyform,
        } => {
            keyform::set_cell(model, &target, &cell, keyform)?;
            touched.push(target);
        }
        // すべてのセルが同じ値になるので、推定で作り直しても変わらない (`touched` に積まない)。
        Command::SetMeshDrawOrder { id, draw_order } => {
            if !draw_order.is_finite() {
                return Err(EditError::InvalidDrawOrder {
                    id: id.to_string(),
                    draw_order,
                });
            }
            // ブレンドシェイプ格子のセルも揃えて、描画順の差分をなくす。
            for keyform in mesh(model, &id)?.keyforms.all_keyforms_mut() {
                keyform.draw_order = draw_order;
            }
        }
        Command::CreateWarpDeformer {
            id,
            name,
            children,
            parts,
            rows,
            cols,
            parent,
        } => {
            let new = deformer_edit::NewDeformer {
                id,
                name,
                parent,
                kind: deformer_edit::NewKind::Warp { rows, cols },
            };
            deformer_edit::create(model, new, &children, &parts, touched)?;
        }
        Command::CreateRotationDeformer {
            id,
            name,
            children,
            parts,
            origin,
            parent,
        } => {
            let new = deformer_edit::NewDeformer {
                id,
                name,
                parent,
                kind: deformer_edit::NewKind::Rotation { origin },
            };
            deformer_edit::create(model, new, &children, &parts, touched)?;
        }
        Command::ReparentPreservingLook {
            element,
            new_parent,
        } => deformer_edit::reparent(model, &element, new_parent.as_ref(), touched)?,
        // 深度は深度からの回転の入力なので、その格子の `derived` のセルを作り直す。
        Command::SetDepth { target, depth } => {
            touched.push(depth::set_depth(model, &target, depth)?);
        }
        Command::ApplyDepthPreset { target, preset } => {
            touched.push(depth::apply_preset(model, &target, &preset)?);
        }
        Command::MoveRotationOrigin { id, delta } => {
            deformer_edit::move_rotation_origin(model, &id, delta, touched)?;
        }
        Command::UnwrapDeformer { id } => {
            deformer_edit::unwrap(model, &id, touched)?;
            remove_deformer(model, &id)?;
        }
        Command::AddMotion { motion } => motion_edit::add_motion(anim, model, motion)?,
        Command::RemoveMotion { name } => motion_edit::remove_motion(anim, &name)?,
        Command::RenameMotion { name, new_name } => {
            motion_edit::rename_motion(anim, &name, new_name)?;
        }
        Command::SetMotionProperties {
            name,
            duration,
            looping,
            fade_in,
            fade_out,
        } => {
            let props = motion_edit::MotionProperties {
                duration,
                looping,
                fade_in,
                fade_out,
            };
            motion_edit::set_motion_properties(anim, &name, props)?;
        }
        Command::AddMotionCurve { motion, curve } => {
            motion_edit::add_curve(anim, model, &motion, curve)?;
        }
        Command::RemoveMotionCurve { motion, target } => {
            motion_edit::remove_curve(anim, &motion, &target)?;
        }
        Command::SetMotionCurveFade {
            motion,
            target,
            fade_in,
            fade_out,
        } => motion_edit::set_curve_fade(anim, &motion, &target, fade_in, fade_out)?,
        Command::SetMotionKey {
            motion,
            target,
            point,
        } => motion_edit::set_key(anim, model, &motion, &target, point)?,
        Command::MoveMotionKey {
            motion,
            target,
            t,
            to,
        } => motion_edit::move_key(anim, &motion, &target, t, to)?,
        Command::RemoveMotionKey { motion, target, t } => {
            motion_edit::remove_key(anim, &motion, &target, t)?;
        }
        Command::SetMotionSegment {
            motion,
            target,
            t,
            segment,
        } => motion_edit::set_segment(anim, &motion, &target, t, segment)?,
        Command::SetMotionEvents { motion, events } => {
            motion_edit::set_motion_events(anim, &motion, events)?;
        }
        Command::AddExpression { expression } => {
            motion_edit::add_expression(anim, model, expression)?;
        }
        Command::RemoveExpression { name } => motion_edit::remove_expression(anim, &name)?,
        Command::RenameExpression { name, new_name } => {
            motion_edit::rename_expression(anim, &name, new_name)?;
        }
        Command::SetExpressionProperties {
            name,
            fade_in,
            fade_out,
        } => motion_edit::set_expression_properties(anim, &name, fade_in, fade_out)?,
        Command::SetExpressionParameters { name, parameters } => {
            motion_edit::set_expression_parameters(anim, model, &name, parameters)?;
        }
        // 物理 (T0629)。
        command @ (Command::SetPhysicsWorld { .. }
        | Command::AddPhysicsRig { .. }
        | Command::CreatePhysicsRig { .. }
        | Command::RemovePhysicsRig { .. }
        | Command::RenamePhysicsRig { .. }
        | Command::SetPhysicsInputs { .. }
        | Command::SetPhysicsOutputs { .. }
        | Command::SetPhysicsParticles { .. }
        | Command::SetPhysicsNormalization { .. }
        | Command::SetPhysicsRigColliders { .. }
        | Command::SetPhysicsFrame { .. }
        | Command::AddPhysicsCollider { .. }
        | Command::RemovePhysicsCollider { .. }
        | Command::SetPhysicsCollider { .. }
        | Command::ReorderPhysicsRig { .. }) => physics_edit::apply(model, physics, command)?,
        // 補間の種類はセルの値を変えないので、推定で作り直さない。
        Command::SetKeyformInterpolation {
            target,
            blend_shape,
            interpolation,
        } => keyform::set_interpolation(model, &target, blend_shape.as_ref(), interpolation)?,
        Command::SetParameterInternal { id, internal } => {
            parameter(model, &id)?.internal = internal;
        }
        Command::CreateDrawClone { source, id, name } => {
            draw_clone::create(model, &source, id, name)?;
        }
        Command::SyncDrawClone { id } => draw_clone::sync(model, &id)?,
        Command::ApplyPhysicsTemplate { template, options } => {
            let commands = physics_template::commands(model, physics.as_ref(), template, &options)?;
            for c in commands {
                apply_to(model, textures, anim, c, touched, mirrored, physics)?;
            }
        }
        Command::SetNormalMap {
            texture,
            normal_map,
            png,
        } => set_normal_map(model, textures, &texture, normal_map, png)?,
        Command::SetLighting { enabled } => model.lighting = enabled,
        Command::SetEffects { effects } => model.effects = effects,
        Command::AddSlot { slot } => slot_edit::add(model, physics.as_ref(), slot)?,
        Command::SetSlot { id, slot } => slot_edit::set(model, physics.as_ref(), &id, slot)?,
        Command::RemoveSlot { id } => slot_edit::remove(model, &id)?,
        // 作業用のモデルを組み直すので、`Document::apply_with_report` が単独で受ける。
        Command::SetModuleSettings { .. } => {
            return Err(EditError::ModuleShape {
                reason: "module settings must be changed on their own, not in a batch".into(),
            });
        }
    }
    Ok(())
}

/// [`Command::SetMirrorPair`]。対を双方向に張り直し、`target` の対称軸を設定する。
/// `mirror`・`mirror_axis` が変わった要素を返す。
fn set_mirror_pair(
    model: &mut Model,
    target: &KeyformTarget,
    mirror: Option<String>,
    axis: Option<f32>,
) -> Result<Vec<KeyformTarget>, EditError> {
    match target {
        KeyformTarget::Mesh(id) => {
            let mut changed = pair(
                &mut model.meshes,
                "mesh",
                id,
                mirror.map(MeshId::new),
                |m| &m.id,
                |m| &mut m.mirror,
            )?;
            let item = mesh(model, id)?;
            set_axis(&mut item.mirror_axis, axis, id, &mut changed);
            Ok(changed.into_iter().map(KeyformTarget::Mesh).collect())
        }
        KeyformTarget::Deformer(id) => {
            let mut changed = pair(
                &mut model.deformers,
                "deformer",
                id,
                mirror.map(DeformerId::new),
                |d| &d.id,
                |d| &mut d.mirror,
            )?;
            let item = deformer(model, id)?;
            set_axis(&mut item.mirror_axis, axis, id, &mut changed);
            Ok(changed.into_iter().map(KeyformTarget::Deformer).collect())
        }
        KeyformTarget::Part(_) | KeyformTarget::Glue(_) => Err(EditError::MirrorPairNotSupported {
            target: target.to_string(),
        }),
    }
}

/// 対称軸を `axis` にし、変わったら `id` を `changed` に積む (積んでいなければ)。
/// 対称軸が有限の値であることは、適用後の検証 (`validate`) が確かめる。
fn set_axis<K: PartialEq + Clone>(
    field: &mut Option<f32>,
    axis: Option<f32>,
    id: &K,
    changed: &mut Vec<K>,
) {
    if *field != axis {
        *field = axis;
        if !changed.contains(id) {
            changed.push(id.clone());
        }
    }
}

/// `items` の中で `id` と `mirror` を双方向の対にする (`None` なら対を外す。`mirror` が `id` 自身なら
/// `id` だけを自分自身と対にする)。どちらかと対になっていた要素の対は外す。
/// `mirror` が変わった要素の ID を返す。メッシュ・デフォーマ・パラメータの対で共通。
fn pair<T, K: PartialEq + Clone + Display>(
    items: &mut [T],
    kind: &'static str,
    id: &K,
    mirror: Option<K>,
    id_of: impl Fn(&T) -> &K,
    mirror_of: impl Fn(&mut T) -> &mut Option<K>,
) -> Result<Vec<K>, EditError> {
    find_mut(items, kind, id, &id_of)?;
    if let Some(mirror) = &mirror {
        find_mut(items, kind, mirror, &id_of)?;
    }
    let partner = mirror.clone().filter(|m| m != id);
    let mut changed = Vec::new();
    for item in items.iter_mut() {
        let item_id = id_of(item).clone();
        let item_mirror = mirror_of(item);
        let before = item_mirror.clone();
        if &item_id == id {
            *item_mirror = mirror.clone();
        } else if partner.as_ref() == Some(&item_id) {
            *item_mirror = Some(id.clone());
        } else if item_mirror
            .as_ref()
            .is_some_and(|m| m == id || partner.as_ref() == Some(m))
        {
            // 張り直す 2 つのどちらかと対になっていた要素。
            *item_mirror = None;
        }
        if *item_mirror != before {
            changed.push(item_id);
        }
    }
    Ok(changed)
}

/// パラメータを削除し、参照を外す: キーフォーム格子の軸は既定値の断面に畳み、
/// 他のパラメータの `mirror` とエフェクトの対象からは取り除く。
fn remove_parameter(model: &mut Model, id: &ParamId) -> Result<(), EditError> {
    let index = model
        .parameters
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "parameter",
            id: id.to_string(),
        })?;
    let removed = model.parameters.remove(index);
    let default = removed.default;

    for part in &mut model.parts {
        grid::collapse_axis(&mut part.keyforms, id, default);
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(warp) => grid::collapse_axis(&mut warp.keyforms, id, default),
            DeformerKind::Rotation(rotation) => {
                grid::collapse_axis(&mut rotation.keyforms, id, default)
            }
            // 未知の種類はキーフォームを持たない。
            DeformerKind::Unknown => {}
        }
    }
    for mesh in &mut model.meshes {
        grid::collapse_axis(&mut mesh.keyforms, id, default);
    }
    for glue in &mut model.glues {
        grid::collapse_axis(&mut glue.keyforms, id, default);
    }

    for param in &mut model.parameters {
        if param.mirror.as_ref() == Some(id) {
            param.mirror = None;
        }
    }
    let effects = &mut model.effects;
    effects.eye_blink.parameters.retain(|p| p != id);
    effects.breath.parameters.retain(|b| &b.parameter != id);
    effects.lip_sync.parameters.retain(|p| p != id);
    for slot in &mut model.slots {
        slot.parameters.retain(|p| p != id);
    }
    Ok(())
}

/// パーツを削除する。子パーツ・所属する要素があれば拒否し、ポーズからは取り除く。
fn remove_part(model: &mut Model, id: &PartId) -> Result<(), EditError> {
    let index = model
        .parts
        .iter()
        .position(|p| &p.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "part",
            id: id.to_string(),
        })?;
    let children = model
        .parts
        .iter()
        .filter(|p| p.parent.as_ref() == Some(id))
        .map(|p| format!("part `{}`", p.id))
        .chain(
            model
                .deformers
                .iter()
                .filter(|d| &d.part == id)
                .map(|d| format!("deformer `{}`", d.id)),
        )
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| &m.part == id)
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .chain(slot_edit::parented_to(model, id))
        .collect();
    ensure_no_children("part", id, children)?;
    model.parts.remove(index);
    for slot in &mut model.slots {
        slot.replaces.retain(|p| p != id);
        slot.pose_parts.retain(|p| p != id);
    }

    for group in &mut model.pose.groups {
        group.entries.retain(|e| &e.part != id);
        for entry in &mut group.entries {
            entry.links.retain(|l| l != id);
        }
    }
    model.pose.groups.retain(|g| !g.entries.is_empty());
    Ok(())
}

/// デフォーマを削除する。子デフォーマ・子メッシュがあれば拒否する。
fn remove_deformer(model: &mut Model, id: &DeformerId) -> Result<(), EditError> {
    let index = model
        .deformers
        .iter()
        .position(|d| &d.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "deformer",
            id: id.to_string(),
        })?;
    let children = model
        .deformers
        .iter()
        .filter(|d| d.parent.as_ref() == Some(id))
        .map(|d| format!("deformer `{}`", d.id))
        .chain(
            model
                .meshes
                .iter()
                .filter(|m| m.parent.as_ref() == Some(id))
                .map(|m| format!("mesh `{}`", m.id)),
        )
        .chain(slot_edit::anchored_to(model, id))
        .collect();
    ensure_no_children("deformer", id, children)?;
    model.deformers.remove(index);
    for deformer in &mut model.deformers {
        if deformer.mirror.as_ref() == Some(id) {
            deformer.mirror = None;
        }
    }
    Ok(())
}

/// メッシュを削除する。マスクの参照から外し、このメッシュを使うグルーと当たり判定を削除する
/// (その当たり判定を指すアバタープロファイルの標準の当たり判定も外す)。
fn remove_mesh(model: &mut Model, id: &MeshId) -> Result<(), EditError> {
    let index = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    model.meshes.remove(index);
    for mesh in &mut model.meshes {
        mesh.masks.retain(|m| &m.mesh != id);
    }
    model.glues.retain(|g| &g.mesh_a != id && &g.mesh_b != id);
    model.hit_areas.retain(|h| &h.mesh != id);
    avatar::forget_removed_hit_areas(model);
    for slot in &mut model.slots {
        slot.mask_sources.retain(|m| m != id);
    }
    for mesh in &mut model.meshes {
        if mesh.mirror.as_ref() == Some(id) {
            mesh.mirror = None;
        }
    }
    Ok(())
}

/// `meshes` の中でメッシュ `id` を `before` の直前 (`None` なら末尾) へ移す。
fn reorder_mesh(model: &mut Model, id: &MeshId, before: Option<&MeshId>) -> Result<(), EditError> {
    if before == Some(id) {
        return Err(EditError::SelfReference {
            kind: "mesh",
            id: id.to_string(),
            field: "before",
        });
    }
    let from = model
        .meshes
        .iter()
        .position(|m| &m.id == id)
        .ok_or_else(|| EditError::NotFound {
            kind: "mesh",
            id: id.to_string(),
        })?;
    if let Some(before) = before
        && !model.meshes.iter().any(|m| &m.id == before)
    {
        return Err(EditError::NotFound {
            kind: "mesh",
            id: before.to_string(),
        });
    }
    let moved = model.meshes.remove(from);
    let to = match before {
        Some(before) => model
            .meshes
            .iter()
            .position(|m| &m.id == before)
            .unwrap_or(model.meshes.len()),
        None => model.meshes.len(),
    };
    model.meshes.insert(to, moved);
    Ok(())
}

/// PNG の寸法を読む。PNG でなければ [`EditError::NotPng`]。
fn png_dimensions(name: &str, png: &[u8]) -> Result<(u32, u32), EditError> {
    png_size(png).ok_or_else(|| EditError::NotPng {
        name: name.to_owned(),
    })
}

/// テクスチャを `Model::textures` の末尾に追加する。寸法は PNG から読む。
///
/// 名前は大文字と小文字を区別せずに重複を拒否する (大文字小文字を区別しないファイルシステムで
/// `textures/<name>.png` が衝突しないように)。法線マップの画像も同じ場所に置くので、その名前とも比べる。
fn add_texture(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    if !is_valid_texture_name(&name) {
        return Err(EditError::InvalidTextureName { name });
    }
    if model.textures.iter().any(|t| {
        t.name.eq_ignore_ascii_case(&name)
            || t.normal_map
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(&name))
    }) {
        return Err(EditError::DuplicateId {
            kind: "texture",
            id: name,
        });
    }
    let (width, height) = png_dimensions(&name, &png)?;
    model.textures.push(TextureRef {
        name: name.clone(),
        width,
        height,
        normal_map: None,
    });
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// テクスチャの画像を差し替える。`TextureRef` の寸法も PNG に合わせて更新する。
fn set_texture_image(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    let (width, height) = png_dimensions(&name, &png)?;
    let texture = find_mut(&mut model.textures, "texture", &name, |t| &t.name)?;
    (texture.width, texture.height) = (width, height);
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// テクスチャを削除する。使っているメッシュがあれば拒否する。
/// 後ろのテクスチャを指すメッシュの `texture` (インデックス) は 1 つ詰める。
fn remove_texture(model: &mut Model, textures: &mut Textures, name: &str) -> Result<(), EditError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == name)
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: name.to_owned(),
        })?;
    let children = model
        .meshes
        .iter()
        .filter(|m| usize::try_from(m.texture).is_ok_and(|t| t == index))
        .map(|m| format!("mesh `{}`", m.id))
        .collect();
    ensure_no_children("texture", &name, children)?;
    let removed = model.textures.remove(index);
    for mesh in &mut model.meshes {
        if usize::try_from(mesh.texture).is_ok_and(|t| t > index) {
            mesh.texture -= 1;
        }
    }
    textures.remove(name);
    if let Some(normal_map) = removed.normal_map {
        drop_unused_normal_map(model, textures, &normal_map);
    }
    Ok(())
}

/// [`Command::SetNormalMap`]。
fn set_normal_map(
    model: &mut Model,
    textures: &mut Textures,
    texture: &str,
    normal_map: Option<String>,
    png: Option<Vec<u8>>,
) -> Result<(), EditError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == texture)
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: texture.to_owned(),
        })?;
    let invalid = |reason: String| EditError::InvalidNormalMap {
        texture: texture.to_owned(),
        reason,
    };
    match (&normal_map, png) {
        (None, Some(_)) => {
            return Err(invalid("an image was given without a name".into()));
        }
        (None, None) => {}
        (Some(name), png) => {
            if !is_valid_texture_name(name) {
                return Err(EditError::InvalidTextureName { name: name.clone() });
            }
            // テクスチャの画像と同じ場所 (`textures/<name>.png`) に置くので、テクスチャの名前と重ならず、
            // ほかの法線マップの名前とは大文字小文字まで一致すること。
            if model
                .textures
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case(name))
            {
                return Err(EditError::DuplicateId {
                    kind: "texture",
                    id: name.clone(),
                });
            }
            let shared = model
                .textures
                .iter()
                .filter_map(|t| t.normal_map.as_deref())
                .find(|n| n.eq_ignore_ascii_case(name));
            if let Some(existing) = shared
                && existing != name
            {
                return Err(invalid(format!(
                    "`{name}` differs only in case from the normal map `{existing}`"
                )));
            }
            match png {
                Some(png) => {
                    png_dimensions(name, &png)?;
                    textures.insert(name.clone(), Arc::from(png));
                }
                None if shared.is_some() && textures.contains_key(name.as_str()) => {}
                None => {
                    return Err(EditError::NotFound {
                        kind: "normal map",
                        id: name.clone(),
                    });
                }
            }
        }
    }
    let previous = std::mem::replace(&mut model.textures[index].normal_map, normal_map);
    if let Some(previous) = previous {
        drop_unused_normal_map(model, textures, &previous);
    }
    Ok(())
}

/// どのテクスチャからも指されなくなった法線マップの画像を表から除く。
fn drop_unused_normal_map(model: &Model, textures: &mut Textures, name: &str) {
    let used = model
        .textures
        .iter()
        .any(|t| t.name == name || t.normal_map.as_deref() == Some(name));
    if !used {
        textures.remove(name);
    }
}
