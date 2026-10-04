use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use ugoe_model::deformer::DeformerKind;
use ugoe_model::expression::Expression;
use ugoe_model::module::{ModuleRequirements, SlotId};
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{Canvas, KeyformOrigin, Model, Module};
use ugoe_rigging::{InferScope, MirrorOptions};

use crate::command::KeyformTarget;
use crate::grid;
use crate::inference::{MirrorReport, RederiveWarning};
use crate::module_project::{ModuleBase, ModuleContext, ModuleError};
use crate::motion_edit::{self, Animations};
use crate::texture::Textures;
use crate::{Command, automesh, avatar, cleanup, inference, keyform};
use crate::{
    deformer_edit, depth, draw_clone, glue_edit, mesh_edit, physics_edit, physics_template,
    slot_edit,
};

mod error;
mod history;
mod lookup;
mod pair;
mod remove;
mod reorder;
mod texture_edit;

pub use error::EditError;
#[cfg(test)]
pub(crate) use history::checks;
pub use history::{COALESCE_WINDOW, Coalesce, DEFAULT_UNDO_LIMIT};
use history::{History, Problems};
use lookup::{deformer, ensure_new, mesh, parameter, part};
use pair::{pair, set_mirror_pair};
use remove::{remove_deformer, remove_mesh, remove_parameter, remove_part};
use reorder::reorder_mesh;
use texture_edit::{add_texture, remove_texture, set_normal_map, set_texture_image};

/// [`Document::apply_with_report`] の結果: コマンドは適用したが、利用者に知らせたいこと。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct ApplyReport {
    /// 自動の作り直しで失敗し、前の値のまま残したセルがある格子と、親を変えたことでミラーし直せなく
    /// なった左右の対。
    pub warnings: Vec<RederiveWarning>,
    /// 明示的に実行したミラー (`MirrorKeyforms`) ごとの、書いたセルと残したセルの数。
    pub mirrored: Vec<MirrorReport>,
}

/// 編集中のモデル・テクスチャ・物理の設定・モーション・表情と履歴。
///
/// 暫定実装としてスナップショット方式の Undo を使う (`docs/adr/0005-editor-command-model.md`)。
/// テクスチャの画像は `Arc` で共有するので、スナップショットでバイト列は複製されない。
/// 履歴は Undo と Redo を合わせて [`Document::undo_limit`] 個までで、超えると古いものから捨てる
/// (`docs/editor.md#undo`)。
#[derive(Debug)]
pub struct Document {
    current: Snapshot,
    history: History<Snapshot>,
    /// 次に作る状態の番号。
    next_state: u64,
    /// 最後に保存した状態の番号 (未保存なら `None`)。
    saved: Option<u64>,
    /// 最後のまとめる適用 ([`Document::apply_coalescing`]) の印と、それで作った状態の番号。
    /// その状態のまま次のまとめる適用が来たら、Undo を積まずにまとめる。
    coalescing: Option<(Coalesce, u64)>,
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
    /// この状態の検証の結果 (初めて要ったときに求める)。状態を変えたら空に戻す ([`Snapshot::edited`])。
    problems: OnceLock<Problems>,
}

impl Snapshot {
    /// この状態の検証の結果。同じ状態では 1 度だけ検証する。
    fn problems(&self) -> &Problems {
        self.problems
            .get_or_init(|| Problems::of(&self.model, self.physics.as_ref()))
    }

    /// 書き換えて次の状態にするための複製 (検証の結果は持たない)。
    fn edited(&self) -> Snapshot {
        let mut next = self.clone();
        next.problems = OnceLock::new();
        next
    }
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
                problems: OnceLock::new(),
            },
            history: History::new(DEFAULT_UNDO_LIMIT),
            next_state: 1,
            saved: Some(0),
            coalescing: None,
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
        self.current.problems = OnceLock::new();
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
        // 保存した状態に Undo で戻れるよう、まとめを切る。
        self.coalescing = None;
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
            history: History::new(self.history.limit()),
            next_state: self.next_state,
            saved: None,
            coalescing: None,
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
        self.apply_inner(command, false)
    }

    /// [`Document::apply_with_report`] と同じく適用するが、同じ印 (`coalesce.key`) のまとめる適用に続けて
    /// [`COALESCE_WINDOW`] の内に来たなら、Undo を積まずに前の適用とまとめる (数値の欄の矢印キーの押し続けなど。
    /// `docs/editor.md#undo`)。まとめた適用は 1 回の Undo で、まとまりの最初の適用の前に戻る。
    ///
    /// 前のまとめる適用の後に、ほかの適用・Undo・Redo・保存・本体の読み直しがあればまとめない (状態の番号で確かめる)。
    /// [`Command::SetModuleSettings`] はまとめない。拒否したときはまとまりを切らない。
    pub fn apply_coalescing(
        &mut self,
        command: Command,
        coalesce: Coalesce,
    ) -> Result<ApplyReport, EditError> {
        let merge =
            self.coalescing.as_ref().is_some_and(|(prev, state)| {
                *state == self.current.state && coalesce.continues(prev)
            }) && !matches!(command, Command::SetModuleSettings { .. });
        let report = self.apply_inner(command, merge)?;
        self.coalescing = Some((coalesce, self.current.state));
        Ok(report)
    }

    /// 適用の本体。`merge` なら前の状態を Undo に積まずに捨てる (直前の適用とまとめる)。
    fn apply_inner(&mut self, command: Command, merge: bool) -> Result<ApplyReport, EditError> {
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
        let mut next = self.current.edited();
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
        cleanup::forget_removed(
            &self.current.model,
            self.current.physics.as_ref(),
            &mut next.model,
            &mut next.physics,
        );
        if let Some(context) = &next.module {
            context.normalize(&mut next.model, &mut touched)?;
        }
        // 形・テクスチャ・親が変わった元のメッシュに、描画用の複製を追従させる。
        draw_clone::follow(&self.current.model, &mut next.model)?;
        let mut warnings = inference::rederive(&mut next.model, &touched)?;
        inference::check_reparented_pairs(&self.current.model, &next.model, &mut warnings);
        // 適用前の状態の検証の結果は、状態ごとに覚えたものを使う。
        let before = self.current.problems();
        let after = Problems::of(&next.model, next.physics.as_ref());
        let mut introduced: Vec<_> = after
            .model
            .iter()
            .filter(|e| !before.model.contains(e))
            .cloned()
            .collect();
        // 物理の参照先と値の検査 (`docs/spec/model.md#物理の検証`)。
        introduced.extend(
            after
                .physics
                .iter()
                .filter(|e| !before.physics.contains(e))
                .cloned(),
        );
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
        next.problems = OnceLock::from(after);
        next.state = self.next_state;
        self.next_state += 1;
        let prev = std::mem::replace(&mut self.current, next);
        if !merge {
            self.history.record(prev);
        }
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
        let mut problems = OnceLock::new();
        if keep_look {
            let mut touched = Vec::new();
            reconfigured.keep_look(
                &mut workspace.model,
                workspace.physics.as_mut(),
                context.slot().anchor.as_ref(),
                &mut touched,
            )?;
            inference::rederive(&mut workspace.model, &touched)?;
            let before = &self.current.problems().model;
            let after = Problems::of(&workspace.model, workspace.physics.as_ref());
            let introduced: Vec<_> = after
                .model
                .iter()
                .filter(|e| !before.contains(e))
                .cloned()
                .collect();
            if !introduced.is_empty() {
                return Err(EditError::Invalid(introduced));
            }
            reconfigured.split(
                &workspace.model,
                workspace.physics.as_ref(),
                &workspace.textures,
            )?;
            problems = OnceLock::from(after);
        }
        let next = Snapshot {
            model: workspace.model,
            textures: workspace.textures,
            animations: self.current.animations.clone(),
            physics: workspace.physics,
            module: Some(Arc::new(reconfigured)),
            state: self.next_state,
            problems,
        };
        self.next_state += 1;
        self.history
            .record(std::mem::replace(&mut self.current, next));
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
            return Err(ModuleError::NotModule);
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
            problems: OnceLock::new(),
        };
        self.next_state += 1;
        self.history.clear();
        self.coalescing = None;
        self.saved = saved.then_some(self.current.state);
        Ok(())
    }

    /// 1 つ前の状態に戻す。戻れなければ (履歴がない・上限を超えて捨てた) `false`。
    pub fn undo(&mut self) -> bool {
        self.coalescing = None;
        self.history.undo(&mut self.current)
    }

    /// Undo した状態を 1 つやり直す。やり直せなければ `false`。
    pub fn redo(&mut self) -> bool {
        self.coalescing = None;
        self.history.redo(&mut self.current)
    }

    /// [`Document::undo`] で戻れるか (履歴がない・上限を超えて捨てたときは `false`)。
    pub fn can_undo(&self) -> bool {
        self.undo_count() > 0
    }

    /// [`Document::redo`] で進めるか。
    pub fn can_redo(&self) -> bool {
        self.redo_count() > 0
    }

    /// 続けて Undo できる数。
    pub fn undo_count(&self) -> usize {
        self.history.undo_len()
    }

    /// 続けて Redo できる数。
    pub fn redo_count(&self) -> usize {
        self.history.redo_len()
    }

    /// 覚える履歴 (Undo と Redo を合わせた数) の上限。既定は [`DEFAULT_UNDO_LIMIT`]。
    pub fn undo_limit(&self) -> usize {
        self.history.limit()
    }

    /// 履歴の上限を変える。今の履歴が超えていれば、古い Undo から (尽きたら遠い Redo から) 捨てる。
    /// 0 なら履歴を持たない。保存済みの状態を捨てても [`Document::is_modified`] は正しいまま
    /// (戻れなくなるだけ)。
    pub fn set_undo_limit(&mut self, limit: usize) {
        self.history.set_limit(limit);
    }

    /// 上限を `limit` にしたドキュメント ([`Document::set_undo_limit`])。
    pub fn with_undo_limit(mut self, limit: usize) -> Self {
        self.set_undo_limit(limit);
        self
    }
}

/// 状態の中の要素の ID (パラメータ・グループ・パーツ・デフォーマ・メッシュ・グルー・テクスチャ・当たり判定・スロット・
/// 物理のリグと衝突体・モーションと表情の名前)。
fn element_ids(snapshot: &Snapshot) -> BTreeSet<String> {
    let m = &snapshot.model;
    let mut ids = BTreeSet::new();
    ids.extend(m.parameters.iter().map(|p| p.id.to_string()));
    ids.extend(m.parameter_groups.iter().map(|g| g.id.to_string()));
    ids.extend(m.parts.iter().map(|p| p.id.to_string()));
    ids.extend(m.deformers.iter().map(|d| d.id.to_string()));
    ids.extend(m.meshes.iter().map(|x| x.id.to_string()));
    ids.extend(m.glues.iter().map(|g| g.id.to_string()));
    ids.extend(m.textures.iter().map(|t| t.name.clone()));
    ids.extend(m.hit_areas.iter().map(|h| h.id.to_string()));
    ids.extend(m.slots.iter().map(|s| s.id.to_string()));
    if let Some(physics) = &snapshot.physics {
        ids.extend(physics.rigs.iter().map(|r| r.id.to_string()));
        ids.extend(physics.colliders.iter().map(|c| c.id.to_string()));
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
        Command::RemovePart { id } => remove_part(model, &id)?,
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
        Command::RemoveDeformer { id } => remove_deformer(model, &id)?,
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
        Command::RemoveMesh { id } => remove_mesh(model, &id)?,
        Command::RenameMesh { id, name } => mesh(model, &id)?.name = name,
        Command::SetMeshProperties {
            id,
            part,
            texture,
            texture_name,
            blend,
            culling,
            masks,
            mask_mode,
            mask_only,
            lit,
        } => {
            let texture = match (texture, texture_name) {
                (Some(_), Some(_)) => {
                    return Err(EditError::ConflictingTexture { id: id.to_string() });
                }
                (Some(index), None) => Some(index),
                (None, Some(name)) => Some(texture_edit::texture_index(model, &name)?),
                (None, None) => None,
            };
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
        Command::SetPose { pose } => model.pose = pose,
        Command::CreateGlue {
            id,
            mesh_a,
            mesh_b,
            distance,
        } => {
            glue_edit::create(model, id, mesh_a, mesh_b, distance)?;
        }
        Command::SetGlue { glue, distance } => glue_edit::set(model, glue, distance)?,
        Command::RemoveGlue { id } => glue_edit::remove(model, id.as_str())?,
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
