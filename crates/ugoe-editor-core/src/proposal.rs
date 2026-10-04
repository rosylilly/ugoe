//! 提案 (AI などが作った [`Command`] の列) の確かめ・プレビュー・採用 (ADR 0006、`docs/editor.md#ai-の提案`)。
//!
//! - 提案は、作ったときのドキュメントの状態 ([`Document::state_id`]) に対するもの。ドキュメントが変わったら
//!   プレビューも採用もせず、作り直してもらう (承認した差分と違う結果にならないように)。Undo で同じ状態に
//!   戻れば、また使える。
//! - 置く前に、ドキュメントの複製に `Batch` として適用し、検証を通ることを確かめる。
//! - 採用は `Batch` (ラベルは提案の題名) として適用し、1 回の Undo で戻る。採用した提案は一覧から隠すが、
//!   Undo で提案を作った状態に戻ると再び出る (選び直して採用し直せる。AI にもう一度尋ねずに済む)。
//! - **一部だけの採用:** コマンドの単位で外せる。外したコマンドを除いた列を同じく複製で確かめ、
//!   適用・検証に失敗したら採用しない (黙って直さない)。画面が依存を追えるよう、各コマンドが
//!   前のどのコマンドに頼るか ([`ProposalCommand::requires`]、規則は [`analyze`]) を返す。
//! - **注文を添えた作り直し:** 作り直した提案は前の提案を置き換えず、一覧の後ろに並べる (見比べてから選ぶ。
//!   前の提案は利用者が却下するまで残る)。作り直しの元・添えた注文・反映できなかった注文は
//!   [`ProposalOrigin`] として提案に記す ([`Proposals::set_origin`])。

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use crate::{ApplyReport, Command, Document, EditError};

/// 提案の 1 つのコマンドの要約 (画面の一覧に出す)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ProposalCommand {
    /// コマンドの種類 (`op`。例: `add_parameter`)。
    pub op: String,
    /// 対象の ID や名前 (例: `AngleX`、``deformer `WarpHead` ``)。分からなければ `null`。
    pub subject: Option<String>,
    /// このコマンドが頼る、前のコマンドの番号 (提案の中の 0 始まり。使う要素を作る・同じ要素を先に変える。
    /// 規則は [`analyze`])。これを外すと、このコマンドは適用できないか、結果が変わる。
    pub requires: Vec<usize>,
}

/// 画面に出す提案。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ProposalView {
    pub id: u32,
    pub title: String,
    /// 提案の理由 (複数行)。
    pub rationale: String,
    pub commands: Vec<ProposalCommand>,
    /// 提案の後にドキュメントが変わった (プレビュー・採用できない。Undo で戻れば使える)。
    pub stale: bool,
    /// 提案を作ったもの (エディタにつないだエージェントの名前など。[`Proposals::set_source`])。
    /// エディタの中で作った提案は `null`。
    pub source: Option<String>,
    /// この提案が作り直した元の提案の番号 ([`ProposalOrigin::revises`])。
    pub revises: Option<u32>,
    /// 提案を作るときに AI に添えた利用者の注文 ([`ProposalOrigin::instructions`])。
    pub instructions: Option<String>,
    /// 今の仕組みでは反映できなかった注文 (AI の答えの言葉のまま。[`ProposalOrigin::unapplied`])。
    pub unapplied: Vec<String>,
}

/// 提案の出どころ: 作り直しの元と、利用者の注文 (`docs/editor.md#ai-への注文と作り直し`)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProposalOrigin {
    /// 作り直した元の提案の番号 (元の提案は置き換えず、そのまま残す)。
    pub revises: Option<u32>,
    /// AI に添えた利用者の注文。
    pub instructions: Option<String>,
    /// 反映できなかった注文。
    pub unapplied: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProposalError {
    #[error("proposal {0} not found")]
    NotFound(u32),
    #[error("the model changed after this proposal was made; undo the changes or propose again")]
    Stale,
    #[error("command {index} does not exist in the proposal")]
    InvalidIndex { index: usize },
    #[error("no commands are selected")]
    NothingSelected,
    #[error("the proposal cannot be applied: {0}")]
    Rejected(#[from] EditError),
}

#[derive(Debug, Clone)]
struct Pending {
    id: u32,
    title: String,
    rationale: String,
    commands: Vec<Command>,
    info: Vec<ProposalCommand>,
    /// 作ったときのドキュメントの状態。
    base: u64,
    /// 採用した (ドキュメントが `base` に戻るまで一覧に出さない)。
    adopted: bool,
    source: Option<String>,
    origin: ProposalOrigin,
}

/// 採用を待っている提案 (開いているプロジェクトごとに持つ)。
#[derive(Debug, Default)]
pub struct Proposals {
    items: Vec<Pending>,
    next_id: u32,
}

impl Proposals {
    pub fn new() -> Self {
        Self::default()
    }

    /// 提案を置く。`doc` の複製に全体を `Batch` として適用し、通らなければ置かない。
    pub fn add(
        &mut self,
        doc: &Document,
        title: impl Into<String>,
        rationale: impl Into<String>,
        commands: Vec<Command>,
    ) -> Result<ProposalView, ProposalError> {
        let title = title.into();
        if commands.is_empty() {
            return Err(ProposalError::NothingSelected);
        }
        doc.detached().apply(Command::Batch {
            label: title.clone(),
            commands: commands.clone(),
        })?;
        self.next_id += 1;
        let pending = Pending {
            id: self.next_id,
            title,
            rationale: rationale.into(),
            info: analyze(doc, &commands),
            commands,
            base: doc.state_id(),
            adopted: false,
            source: None,
            origin: ProposalOrigin::default(),
        };
        let view = view(&pending, doc);
        self.items.push(pending);
        Ok(view)
    }

    /// 一覧に出す提案 (古い順)。採用した提案は、ドキュメントが提案を作った状態に戻っている間だけ出す。
    pub fn views(&self, doc: &Document) -> Vec<ProposalView> {
        self.items
            .iter()
            .filter(|p| !p.adopted || p.base == doc.state_id())
            .map(|p| view(p, doc))
            .collect()
    }

    /// 置いてある提案がない (採用して隠れているものも数える)。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `excluded` の番号のコマンドを除いた提案を、`doc` の複製に適用する (`doc` は変えない)。
    pub fn preview(
        &self,
        doc: &Document,
        id: u32,
        excluded: &[usize],
    ) -> Result<(Document, ApplyReport), ProposalError> {
        let batch = self.batch(doc, id, excluded)?;
        let mut copy = doc.detached();
        let report = copy.apply_with_report(batch)?;
        Ok((copy, report))
    }

    /// `excluded` の番号のコマンドを除いた提案を `doc` に適用し (1 回の Undo で戻る)、提案を採用済みにする
    /// (一覧から隠す。Undo で戻れば再び出る)。失敗したら `doc` も提案もそのまま。
    pub fn adopt(
        &mut self,
        doc: &mut Document,
        id: u32,
        excluded: &[usize],
    ) -> Result<ApplyReport, ProposalError> {
        let batch = self.batch(doc, id, excluded)?;
        let report = doc.apply_with_report(batch)?;
        if let Some(p) = self.items.iter_mut().find(|p| p.id == id) {
            p.adopted = true;
        }
        Ok(report)
    }

    /// 提案を作ったものを記す (画面に出す。例: MCP でつないだエージェントの名前)。提案が無ければ `false`。
    pub fn set_source(&mut self, id: u32, source: impl Into<String>) -> bool {
        match self.items.iter_mut().find(|p| p.id == id) {
            Some(p) => {
                p.source = Some(source.into());
                true
            }
            None => false,
        }
    }

    /// 提案の出どころ (作り直しの元・注文・反映できなかった注文) を記す (画面に出す)。提案が無ければ `false`。
    pub fn set_origin(&mut self, id: u32, origin: ProposalOrigin) -> bool {
        match self.items.iter_mut().find(|p| p.id == id) {
            Some(p) => {
                p.origin = origin;
                true
            }
            None => false,
        }
    }

    /// 提案を捨てる。
    pub fn reject(&mut self, id: u32) -> Result<(), ProposalError> {
        let before = self.items.len();
        self.items.retain(|p| p.id != id);
        if self.items.len() == before {
            return Err(ProposalError::NotFound(id));
        }
        Ok(())
    }

    fn batch(&self, doc: &Document, id: u32, excluded: &[usize]) -> Result<Command, ProposalError> {
        let pending = self
            .items
            .iter()
            .find(|p| p.id == id)
            .ok_or(ProposalError::NotFound(id))?;
        if pending.base != doc.state_id() {
            return Err(ProposalError::Stale);
        }
        if let Some(&index) = excluded.iter().find(|&&i| i >= pending.commands.len()) {
            return Err(ProposalError::InvalidIndex { index });
        }
        let commands: Vec<Command> = pending
            .commands
            .iter()
            .enumerate()
            .filter(|(i, _)| !excluded.contains(i))
            .map(|(_, c)| c.clone())
            .collect();
        if commands.is_empty() {
            return Err(ProposalError::NothingSelected);
        }
        let label = if excluded.is_empty() {
            pending.title.clone()
        } else {
            format!(
                "{} ({} of {} commands)",
                pending.title,
                commands.len(),
                pending.commands.len()
            )
        };
        Ok(Command::Batch { label, commands })
    }
}

fn view(pending: &Pending, doc: &Document) -> ProposalView {
    ProposalView {
        id: pending.id,
        title: pending.title.clone(),
        rationale: pending.rationale.clone(),
        commands: pending.info.clone(),
        stale: pending.base != doc.state_id(),
        source: pending.source.clone(),
        revises: pending.origin.revises,
        instructions: pending.origin.instructions.clone(),
        unapplied: pending.origin.unapplied.clone(),
    }
}

/// コマンドごとの要約と依存。コマンド `i` は、前のコマンド `k` が作った要素か、`k` が主に変える要素
/// (`target_id`) を、JSON のどこか (文字列の値かオブジェクトのキー) で指すなら `k` に依存する。
///
/// - 作った要素: 外すと、その要素を使うコマンドは適用できない。
/// - 変えた要素: 同じ要素への編集は前の編集の結果に頼ることがある (推定の設定の後の推定、キーの追加の後の
///   セルの置き換えなど) ので、同じ要素への後の編集は前の編集に依存するとみなす。
///
/// 種類ごとの規則を持たずに済ませるための近似で、違う種類の要素が同じ ID なら余分に依存とみなすが、
/// 外しすぎる側に倒れるだけ (採用の前に複製で確かめるので、足りなくても誤った採用にはならない)。
pub fn analyze(doc: &Document, commands: &[Command]) -> Vec<ProposalCommand> {
    let created = doc.created_ids(commands);
    let jsons: Vec<Value> = commands
        .iter()
        .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
        .collect();
    let touched: Vec<BTreeSet<String>> = jsons
        .iter()
        .zip(&created)
        .map(|(json, created)| {
            let mut ids = created.clone();
            ids.extend(target_id(json));
            ids
        })
        .collect();
    jsons
        .iter()
        .enumerate()
        .map(|(i, json)| {
            let mut refs = BTreeSet::new();
            strings(json, &mut refs);
            let requires = touched[..i]
                .iter()
                .enumerate()
                .filter(|(_, ids)| ids.iter().any(|id| refs.contains(id.as_str())))
                .map(|(k, _)| k)
                .collect();
            ProposalCommand {
                op: json
                    .get("op")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
                subject: subject(json),
                requires,
            }
        })
        .collect()
}

/// コマンドが主に変える要素の ID (`id`、または `{kind, id}` の形の `target` / `element` の `id`)。
fn target_id(json: &Value) -> Option<String> {
    let id = |v: &Value| v.get("id").and_then(Value::as_str).map(str::to_owned);
    id(json).or_else(|| {
        ["target", "element"]
            .into_iter()
            .find_map(|key| json.get(key).and_then(id))
    })
}

/// JSON の文字列の値とオブジェクトのキーを集める。
fn strings<'a>(value: &'a Value, out: &mut BTreeSet<&'a str>) {
    match value {
        Value::String(s) => {
            out.insert(s);
        }
        Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        Value::Object(map) => {
            for (k, v) in map {
                out.insert(k);
                strings(v, out);
            }
        }
        _ => {}
    }
}

/// 一覧に出す対象: `id`、`{kind, id}` の形の `target` / `element`、要素そのもの (`parameter` など) の `id`、
/// テクスチャの名前 (`texture` が文字列のもの。`set_normal_map` など)、`name` の順に探す。
fn subject(json: &Value) -> Option<String> {
    let str_at = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
    if let Some(id) = str_at(json, "id") {
        return Some(id);
    }
    for key in ["target", "element"] {
        if let Some(v) = json.get(key)
            && let (Some(kind), Some(id)) = (str_at(v, "kind"), str_at(v, "id"))
        {
            return Some(format!("{kind} `{id}`"));
        }
    }
    for key in [
        "parameter",
        "part",
        "deformer",
        "mesh",
        "motion",
        "expression",
    ] {
        if let Some(id) = json
            .get(key)
            .and_then(|v| str_at(v, "id").or(str_at(v, "name")))
        {
            return Some(id);
        }
    }
    if let Some(texture) = str_at(json, "texture") {
        return Some(texture);
    }
    str_at(json, "name").filter(|n| !n.is_empty())
}
