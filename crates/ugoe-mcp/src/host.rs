//! ツールが扱う文書の持ち主 (ADR 0023)。
//!
//! ツール ([`crate::tools::Tools`]) は、プロジェクトの文書 (`Document`)・メタ情報・提案 (`Proposals`) を
//! 自分では持たず、持ち主 ([`DocumentHost`]) から借りる。持ち主は 2 通り:
//!
//! - [`FileHost`]: `.ugoeproj` を自分でディスクから開く (stdio・`--http` の `ugoe-mcp`。今までの振る舞い)。
//! - エディタ (T0642): 自分が開いている文書を貸す。適用・保存などは利用者の確認を待ってから許す
//!   ([`DocumentHost::authorize`]・[`Approvals`])。ほかのプロジェクトを開く・閉じるは断る。
//!
//! # スレッドと錠
//!
//! - ツールは HTTP の接続ごとのスレッド (または stdio の読み込みのスレッド) で同期に動く。Tauri の
//!   イベントループ (メインスレッド) からは呼ばれない。持ち主のメソッドはそのスレッドで呼ばれ、
//!   **待ってよい** ([`DocumentHost::authorize`] で利用者の答えを待つなど)。
//! - [`DocumentHost::with_project`] の `f` は、持ち主が錠を握ったまま呼ぶ。ツールは `f` の中で待たず、
//!   重い処理 (描画・PSD の画像) は複製を取ってから錠の外で行う。
//! - [`DocumentHost::authorize`]・[`DocumentHost::changed`] は、ツールがどの錠も握っていないときに呼ぶ。
//!   確認の間に、ほかの要求 (別のクライアントの読み取りなど) や利用者の編集は進む。確認の後にモデルが
//!   変わっていれば、適用は提案が古いというエラーになる (承認した差分と違う結果にしない)。
//! - エディタで非同期のコマンド (Tauri の `async` コマンド) から答えるときも、答えを渡す側
//!   ([`Approvals::answer`]) は待たないので、非同期のランタイムを止めない。
//!
//! # 取り消し
//!
//! クライアントが要求を取り消すと (`notifications/cancelled`、HTTP では答えを待たずに接続を閉じる)、
//! [`crate::Server`] はその要求を処理しているスレッドに取り消しの印を立てる。[`Approvals::request`] は
//! 待つ間にその印を見て、確認を拒否で終える (持ち主は何もしなくてよい。画面の確認は、ほかの終わり方と
//! 同じく `request` から戻ったときに閉じる)。印は要求を処理するスレッドに付く
//! ([`Approvals::request`] を要求のスレッドの外で呼ぶと、取り消しは届かない)。

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ugoe_editor_core::{Document, Project, ProjectMeta, Proposals, export_bundle};

use crate::root::Root;

/// 持ち主が断った・失敗した。ツールのエラー (`isError`) の文になる。
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// 持ち主が許さない (利用者が拒否した・エディタではできない操作)。
    #[error("{0}")]
    Denied(String),
    /// 利用者の答えを待つうちに時間が切れた。
    #[error("the user did not answer in time; ask them to check the editor and try again")]
    TimedOut,
    /// 読み書きの失敗など。
    #[error("{0}")]
    Failed(String),
}

impl From<String> for HostError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

/// 持ち主が貸す、開いているプロジェクト。
#[derive(Debug)]
pub struct Lent<'a> {
    pub meta: &'a ProjectMeta,
    pub document: &'a mut Document,
    /// 適用を待つ提案 (エディタでは AI タブの提案の一覧と同じもの)。
    pub proposals: &'a mut Proposals,
}

/// プロジェクトを開く中身 ([`DocumentHost::open`])。
#[derive(Debug)]
pub enum Opening {
    /// ディスクから読む (`open_project`)。
    Load,
    /// まだディスクに無い新しいプロジェクト (`open_project` の `create`・`import_psd`)。
    New {
        meta: ProjectMeta,
        document: Box<Document>,
    },
}

/// 持ち主に許しを求める操作 ([`DocumentHost::authorize`])。パスは解決した絶対パス。
#[derive(Debug, Clone, Copy)]
pub enum Action<'a> {
    /// 提案を適用する (`apply_proposal`)。`excluded` は外すコマンドの番号。
    Apply {
        project: &'a Path,
        proposal: u32,
        title: &'a str,
        excluded: &'a [usize],
    },
    /// 直前の適用を取り消す (`undo`)。
    Undo { project: &'a Path },
    /// 取り消した適用をやり直す (`redo`)。
    Redo { project: &'a Path },
    /// ディスクに保存する (`save_project`)。`bundle` は書き出す `.ugoe`。
    Save {
        project: &'a Path,
        bundle: Option<&'a Path>,
    },
    /// プロジェクトを開く・作る (`open_project`)。
    Open {
        project: &'a Path,
        create: bool,
        discard_changes: bool,
    },
    /// PSD から新しいプロジェクトを作る (`import_psd`)。
    Import {
        project: &'a Path,
        psd: &'a Path,
        discard_changes: bool,
    },
    /// プロジェクトを閉じる (`close_project`)。
    Close {
        project: &'a Path,
        discard_changes: bool,
    },
}

/// ツールが持ち主に知らせる出来事 ([`DocumentHost::changed`])。画面を描き直すなどに使う。
#[derive(Debug, Clone, Copy)]
pub enum Change<'a> {
    /// クライアントが握手をした (`initialize`)。
    ClientConnected {
        name: &'a str,
        version: Option<&'a str>,
    },
    /// 提案を置いた (`propose_edit`・`propose_rig`・`propose_module_rig`)。`client` は最後に握手したクライアントの名前
    /// (HTTP はセッションを持たないので、複数のクライアントが同時につながると区別できない)。
    Proposed {
        project: &'a Path,
        proposal: u32,
        client: Option<&'a str>,
    },
    /// 文書を変えた (適用・Undo・Redo)。
    Edited { project: &'a Path },
    /// 保存した。
    Saved { project: &'a Path },
    /// 開いた・作った。
    Opened { project: &'a Path },
    /// 閉じた。
    Closed { project: &'a Path },
}

/// ツールが扱う文書の持ち主。メソッドは HTTP の接続のスレッドなどから同時に呼ばれうる
/// (モジュールの説明の「スレッドと錠」)。
pub trait DocumentHost: Send + Sync {
    /// 開いているプロジェクト `project` (解決した絶対パス) を借りて `f` を 1 回呼ぶ。
    /// 開いていなければ、開く ([`FileHost`] はディスクから読む) か断る。
    fn with_project(&self, project: &Path, f: &mut dyn FnMut(Lent<'_>)) -> Result<(), HostError>;

    /// プロジェクトを開く。同じパスで開いているものは置き換える (未保存の変更があれば
    /// `discard_changes` が要る)。[`DocumentHost::authorize`] で許した後に呼ぶ。
    fn open(
        &self,
        project: &Path,
        opening: Opening,
        discard_changes: bool,
    ) -> Result<(), HostError>;

    /// プロジェクトを閉じる (未保存の変更があれば `discard_changes` が要る)。
    fn close(&self, project: &Path, discard_changes: bool) -> Result<(), HostError>;

    /// プロジェクトをディスクに保存し、`bundle` があれば `.ugoe` にも書き出す。保存した状態を
    /// 「保存済み」にする。[`DocumentHost::authorize`] で許した後に呼ぶ。
    ///
    /// `.ugoe` は一時ファイルに書いてから名前を変えて置き換え、途中で失敗しても前のファイルを壊さない。
    /// 書けないと保存の前に分かれば何も保存しない。保存の後に置き換えられなかったときは、保存済みのまま
    /// エラーを返し、文で「保存した」と伝える ([`FileHost`] の振る舞い)。
    fn save(&self, project: &Path, bundle: Option<&Path>) -> Result<(), HostError>;

    /// 操作を許すかを決める。待ってよい (利用者の確認を待つ)。既定はすべて許す。
    fn authorize(&self, action: &Action<'_>) -> Result<(), HostError> {
        let _ = action;
        Ok(())
    }

    /// 出来事を知らせる。待たないこと (画面へのイベントを送るだけにする)。既定は何もしない。
    fn changed(&self, change: &Change<'_>) {
        let _ = change;
    }

    /// サーバの `instructions` の後に足す文 (エディタにつながっていること、開いているプロジェクトなど)。
    fn instructions(&self) -> Option<String> {
        None
    }
}

/// `.ugoeproj` を自分でディスクから開く持ち主 (`ugoe-mcp` の stdio・`--http`)。
/// 開いているプロジェクトはパスごとに持ち、ツールがパスで指したときに開いていなければ読む。
///
/// 錠はプロジェクトごとの枠 ([`Slot`]) に持つ。表の錠は枠を探す・入れる・除く間だけ握り、読み込み・
/// 保存・書き出しはそのプロジェクトの枠の錠だけを握って行う (ほかのプロジェクトへの要求を待たせない)。
/// 同じプロジェクトへの要求は枠の錠で 1 つずつ進むので、同時に開いても文書は 1 つになる。
/// 錠の順は「枠 → 表」だけにする (表の錠を握ったまま枠の錠を待たない)。
#[derive(Debug, Default)]
pub struct FileHost {
    root: Option<Root>,
    projects: Mutex<BTreeMap<PathBuf, Arc<Slot>>>,
}

/// 1 つのプロジェクトの枠。
type Slot = Mutex<SlotState>;

#[derive(Debug)]
enum SlotState {
    /// 開いていない (作ったばかり・読み込みに失敗した・閉じた)。
    Closed,
    Open(Box<FileProject>),
    /// 表から除いた。この枠の錠を待っていた要求は、表から枠を取り直す。
    Removed,
}

#[derive(Debug)]
struct FileProject {
    meta: ProjectMeta,
    document: Document,
    proposals: Proposals,
}

impl FileProject {
    fn new(meta: ProjectMeta, document: Document) -> Self {
        Self {
            meta,
            document,
            proposals: Proposals::new(),
        }
    }
}

impl FileHost {
    /// `root` があれば、プロジェクトの中の root の外を指すシンボリックリンクを読み書きの前に拒否する
    /// (パスそのものは [`crate::tools::Tools`] が root で確かめる)。
    pub fn new(root: Option<Root>) -> Self {
        Self {
            root,
            projects: Mutex::default(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<PathBuf, Arc<Slot>>> {
        self.projects.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 表から `project` の枠を取る (無ければ入れる)。表の錠はこの間だけ握る。
    fn slot(&self, project: &Path) -> Arc<Slot> {
        Arc::clone(
            self.lock()
                .entry(project.to_path_buf())
                .or_insert_with(|| Arc::new(Mutex::new(SlotState::Closed))),
        )
    }

    /// `project` の枠の錠を握って `f` を呼ぶ。`f` の後に開いていなければ、枠を表から除く。
    fn with_slot<R>(&self, project: &Path, f: impl FnOnce(&mut SlotState) -> R) -> R {
        let mut slot = self.slot(project);
        let mut state = slot.lock().unwrap_or_else(PoisonError::into_inner);
        while matches!(*state, SlotState::Removed) {
            drop(state);
            slot = self.slot(project);
            state = slot.lock().unwrap_or_else(PoisonError::into_inner);
        }
        let out = f(&mut state);
        if matches!(*state, SlotState::Closed) {
            let mut projects = self.lock();
            if projects
                .get(project)
                .is_some_and(|current| Arc::ptr_eq(current, &slot))
            {
                projects.remove(project);
            }
            *state = SlotState::Removed;
        }
        out
    }

    /// 開いていなければディスクから読み、開いているプロジェクトを返す。
    fn opened<'a>(
        &self,
        project: &Path,
        state: &'a mut SlotState,
    ) -> Result<&'a mut FileProject, String> {
        if !matches!(state, SlotState::Open(_)) {
            *state = SlotState::Open(Box::new(self.load(project)?));
        }
        match state {
            SlotState::Open(open) => Ok(open),
            _ => Err(format!("project `{}` is not open", project.display())),
        }
    }

    /// `--root` があれば、プロジェクトのディレクトリの中の (読み書きする) シンボリックリンクが root の外を
    /// 指していないことを確かめる ([`Root::check_project_dir`])。
    fn check_project_dir(&self, dir: &Path) -> Result<(), String> {
        match &self.root {
            Some(root) => root.check_project_dir(dir),
            None => Ok(()),
        }
    }

    /// ディスクからプロジェクトを読む。モジュールなら本体も読む。
    fn load(&self, dir: &Path) -> Result<FileProject, String> {
        self.check_project_dir(dir)?;
        // モジュールのプロジェクトの本体も、読む前に root の下かを確かめる (T0524)。
        let project = match &self.root {
            Some(root) => Project::load_checking_base(dir, &|base| root.check_base(base)),
            None => Project::load(dir),
        }
        .map_err(|e| e.to_string())?;
        Ok(FileProject::new(project.meta.clone(), project.document()))
    }
}

fn unsaved(project: &Path) -> HostError {
    HostError::Failed(format!(
        "`{}` has unsaved changes; save it or pass discard_changes=true",
        project.display()
    ))
}

impl DocumentHost for FileHost {
    fn with_project(&self, project: &Path, f: &mut dyn FnMut(Lent<'_>)) -> Result<(), HostError> {
        self.with_slot(project, |state| {
            let open = self.opened(project, state)?;
            f(Lent {
                meta: &open.meta,
                document: &mut open.document,
                proposals: &mut open.proposals,
            });
            Ok(())
        })
    }

    fn open(
        &self,
        project: &Path,
        opening: Opening,
        discard_changes: bool,
    ) -> Result<(), HostError> {
        self.with_slot(project, |state| {
            if let SlotState::Open(open) = state
                && open.document.is_modified()
                && !discard_changes
            {
                return Err(unsaved(project));
            }
            let open = match opening {
                Opening::Load => self.load(project)?,
                Opening::New { meta, document } => FileProject::new(meta, *document),
            };
            *state = SlotState::Open(Box::new(open));
            Ok(())
        })
    }

    fn close(&self, project: &Path, discard_changes: bool) -> Result<(), HostError> {
        self.with_slot(project, |state| {
            let SlotState::Open(open) = state else {
                return Err(HostError::Failed(format!(
                    "`{}` is not open",
                    project.display()
                )));
            };
            if open.document.is_modified() && !discard_changes {
                return Err(unsaved(project));
            }
            *state = SlotState::Closed;
            Ok(())
        })
    }

    /// `bundle` があれば、保存の前に `.ugoe` のバイト列を作って一時ファイルに書き、保存の後に名前を変えて
    /// 置き換える。保存の前に失敗すれば何も変えない (プロジェクトも `.ugoe` も)。保存の後の名前の変更に
    /// 失敗したときだけ、プロジェクトは保存済みで `.ugoe` は前のままになり、そう伝える。
    fn save(&self, project: &Path, bundle: Option<&Path>) -> Result<(), HostError> {
        self.check_project_dir(project)?;
        self.with_slot(project, |state| {
            let open = self.opened(project, state)?;
            let data = Project::from_document(open.meta.clone(), &open.document);
            let staged = match bundle {
                Some(bundle) => {
                    let bytes = export_bundle(&data).map_err(|e| {
                        format!(
                            "failed to export `{}`: {e}; nothing was saved",
                            bundle.display()
                        )
                    })?;
                    let staged = StagedFile::write(bundle, &bytes).map_err(|e| {
                        format!(
                            "failed to write `{}`: {e}; nothing was saved",
                            bundle.display()
                        )
                    })?;
                    Some(staged)
                }
                None => None,
            };
            // 失敗すれば、書いておいた一時ファイルは捨てる (`StagedFile` の drop)。
            data.save(project).map_err(|e| e.to_string())?;
            open.document.mark_saved();
            if let Some(staged) = staged {
                let target = staged.target.clone();
                staged.commit().map_err(|e| {
                    format!(
                        "saved the project, but failed to write `{}`: {e}; the previous file is left as it was",
                        target.display()
                    )
                })?;
            }
            Ok(())
        })
    }
}

/// 置き換える前のファイル: 行き先と同じディレクトリの一時ファイルに書いておき、[`StagedFile::commit`] で
/// 名前を変えて置き換える (途中で失敗しても、行き先にある前のファイルを壊さない)。置き換えずに捨てると
/// 一時ファイルを消す。
struct StagedFile {
    temp: PathBuf,
    target: PathBuf,
    committed: bool,
}

impl StagedFile {
    fn write(target: &Path, bytes: &[u8]) -> std::io::Result<Self> {
        /// 同じプロセスの中で同じ行き先に同時に書いても、一時ファイルが重ならないように。
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = target
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let staged = Self {
            temp: target.with_file_name(temp_name),
            target: target.to_path_buf(),
            committed: false,
        };
        let mut file = std::fs::File::create(&staged.temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(staged)
    }

    fn commit(mut self) -> std::io::Result<()> {
        std::fs::rename(&self.temp, &self.target)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.temp);
        }
    }
}

/// 取り消された要求の確認を拒否するときの文 (クライアントはふつう読まない)。
const CANCELLED: &str = "the client cancelled the request";
/// 確認を待つ間に取り消しを確かめる間隔。
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// 処理中の要求の取り消しの印 (モジュールの説明の「取り消し」)。
pub(crate) struct Cancellation {
    cancelled: AtomicBool,
    /// クライアントが接続を閉じたか (HTTP の接続を覗く)。要求を処理するスレッドだけが呼ぶ。
    disconnected: Option<Box<dyn Fn() -> bool + Send + Sync>>,
}

impl Cancellation {
    pub(crate) fn new(disconnected: Option<Box<dyn Fn() -> bool + Send + Sync>>) -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            disconnected,
        }
    }

    /// 取り消す (どのスレッドから呼んでもよい)。
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// 取り消されたか (接続が閉じていれば取り消す)。
    pub(crate) fn is_cancelled(&self) -> bool {
        if self.cancelled.load(Ordering::SeqCst) {
            return true;
        }
        if self.disconnected.as_ref().is_some_and(|d| d()) {
            self.cancel();
            return true;
        }
        false
    }

    /// `f` の間、このスレッドの要求の印を `self` にする。
    pub(crate) fn scope<R>(self: &Arc<Self>, f: impl FnOnce() -> R) -> R {
        /// panic しても前の印に戻す。
        struct Restore(Option<Arc<Cancellation>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                let previous = self.0.take();
                CURRENT.with(|c| *c.borrow_mut() = previous);
            }
        }
        let previous = CURRENT.with(|c| c.borrow_mut().replace(Arc::clone(self)));
        let _restore = Restore(previous);
        f()
    }

    fn current() -> Option<Arc<Self>> {
        CURRENT.with(|c| c.borrow().clone())
    }
}

thread_local! {
    /// このスレッドが処理している要求の印 ([`Cancellation::scope`])。
    static CURRENT: RefCell<Option<Arc<Cancellation>>> = const { RefCell::new(None) };
}

/// 利用者の確認を待つ仕組み (エディタの持ち主が [`DocumentHost::authorize`] で使う)。
///
/// `authorize` のスレッドが [`Approvals::request`] で確認を出して待ち、画面 (Tauri のコマンド) が
/// [`Approvals::answer`] で答える。答えは待たずに渡すので、非同期のコマンドから呼んでよい。
#[derive(Default)]
pub struct Approvals {
    next: AtomicU64,
    waiting: Mutex<HashMap<u64, SyncSender<bool>>>,
}

impl fmt::Debug for Approvals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Approvals")
            .field("waiting", &self.lock().len())
            .finish()
    }
}

impl Approvals {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<u64, SyncSender<bool>>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 確認の番号を作って `ask` に渡し (`ask` は画面に確認を出すだけで待たない)、答えか時間切れまで待つ。
    /// 拒否は [`HostError::Denied`] (`denied` の文)、時間切れは [`HostError::TimedOut`]。
    ///
    /// クライアントが要求を取り消したら (モジュールの説明の「取り消し」)、答えを待たずに拒否で終える
    /// (待つ前に取り消されていれば `ask` を呼ばない)。
    pub fn request(
        &self,
        timeout: Duration,
        denied: &str,
        ask: impl FnOnce(u64),
    ) -> Result<(), HostError> {
        let cancellation = Cancellation::current();
        let cancelled = || cancellation.as_ref().is_some_and(|c| c.is_cancelled());
        if cancelled() {
            return Err(HostError::Denied(CANCELLED.to_owned()));
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let (sender, receiver) = mpsc::sync_channel(1);
        self.lock().insert(id, sender);
        ask(id);
        let deadline = Instant::now() + timeout;
        let answer = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = match &cancellation {
                Some(_) => remaining.min(CANCEL_POLL),
                None => remaining,
            };
            match receiver.recv_timeout(wait) {
                Err(RecvTimeoutError::Timeout) if wait < remaining => {
                    if cancelled() {
                        break None;
                    }
                }
                answer => break Some(answer),
            }
        };
        self.lock().remove(&id);
        match answer {
            Some(Ok(true)) => Ok(()),
            Some(Ok(false) | Err(RecvTimeoutError::Disconnected)) => {
                Err(HostError::Denied(denied.to_owned()))
            }
            Some(Err(RecvTimeoutError::Timeout)) => Err(HostError::TimedOut),
            None => Err(HostError::Denied(CANCELLED.to_owned())),
        }
    }

    /// 確認 `id` に答える。待っている確認が無ければ (時間切れの後など) `false`。
    pub fn answer(&self, id: u64, approved: bool) -> bool {
        match self.lock().remove(&id) {
            Some(sender) => sender.try_send(approved).is_ok(),
            None => false,
        }
    }

    /// 待っている確認の番号 (画面を開き直したときに出し直すため)。
    pub fn pending(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.lock().keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// 待っている確認をすべて拒否する (ブリッジを止めるとき)。
    pub fn deny_all(&self) {
        for (_, sender) in self.lock().drain() {
            let _ = sender.try_send(false);
        }
    }
}

/// 文書を貸そうとすると panic する持ち主 (持ち主やツールの不具合の代わり。テスト用)。
#[cfg(test)]
pub(crate) struct PanickingHost;

#[cfg(test)]
impl DocumentHost for PanickingHost {
    fn with_project(&self, _: &Path, _: &mut dyn FnMut(Lent<'_>)) -> Result<(), HostError> {
        panic!("the host is broken");
    }

    fn open(&self, _: &Path, _: Opening, _: bool) -> Result<(), HostError> {
        Ok(())
    }

    fn close(&self, _: &Path, _: bool) -> Result<(), HostError> {
        Ok(())
    }

    fn save(&self, _: &Path, _: Option<&Path>) -> Result<(), HostError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    use ugoe_editor_core::Command;

    /// テスト用の一時ディレクトリ。破棄時に消す。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("ugoe-mcp-host-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }

        /// 中の名前 (一時ファイルが残っていないかを見る)。
        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `dir` に単一の四角形のプロジェクトを保存する。
    fn save_quad(dir: &Path) {
        let mut textures = ugoe_editor_core::Textures::new();
        textures.insert(
            "white".to_owned(),
            ugoe_editor_core::texture::solid_png(1, 1, [255; 4]).into(),
        );
        let document = Document::with_textures(ugoe_model::fixtures::single_quad(), textures);
        Project::from_document(Project::empty("quad").meta, &document)
            .save(dir)
            .unwrap();
    }

    fn rename(name: &str) -> Command {
        Command::RenameMesh {
            id: "Quad".into(),
            name: name.into(),
        }
    }

    /// `project` を開いて (開いていなければ読む) 文書を変える。
    fn edit(host: &FileHost, project: &Path, name: &str) {
        host.with_project(project, &mut |lent| {
            lent.document.apply(rename(name)).unwrap();
        })
        .unwrap();
    }

    fn modified(host: &FileHost, project: &Path) -> bool {
        let mut modified = None;
        host.with_project(project, &mut |lent| {
            modified = Some(lent.document.is_modified());
        })
        .unwrap();
        modified.unwrap()
    }

    #[test]
    fn a_busy_project_does_not_block_other_projects() {
        let tmp = TempDir::new("busy");
        let (a, b) = (tmp.0.join("a.ugoeproj"), tmp.0.join("b.ugoeproj"));
        save_quad(&a);
        save_quad(&b);
        let host = Arc::new(FileHost::new(None));
        // 読み込み・保存・書き出しは、そのプロジェクトの枠の錠を握って行う。`with_project` の `f` も
        // 同じ錠の中で動くので、`f` で止めて「a を読み書きしている間」を作る。
        let (entered, entered_rx) = mpsc::channel();
        let (release, release_rx) = mpsc::channel::<()>();
        let busy = {
            let host = Arc::clone(&host);
            let a = a.clone();
            std::thread::spawn(move || {
                host.with_project(&a, &mut |_| {
                    entered.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
            })
        };
        entered_rx.recv().unwrap();

        // a を握っている間も、b は読み込み・編集・保存と書き出し・閉じるができる。
        let (done, done_rx) = mpsc::channel();
        let other = {
            let host = Arc::clone(&host);
            let bundle = tmp.0.join("b.ugoe");
            std::thread::spawn(move || {
                edit(&host, &b, "B");
                host.save(&b, Some(&bundle)).unwrap();
                assert!(!modified(&host, &b));
                host.close(&b, false).unwrap();
                done.send(()).unwrap();
            })
        };
        let finished = done_rx.recv_timeout(Duration::from_secs(30));
        release.send(()).unwrap();
        busy.join().unwrap().unwrap();
        other.join().unwrap();
        assert!(finished.is_ok(), "b waited for a");
        assert!(tmp.0.join("b.ugoe").is_file());
    }

    #[test]
    fn opening_the_same_project_at_once_makes_one_document() {
        const THREADS: usize = 8;
        let tmp = TempDir::new("same");
        let project = tmp.0.join("a.ugoeproj");
        save_quad(&project);
        let host = Arc::new(FileHost::new(None));
        let barrier = Arc::new(Barrier::new(THREADS));
        // どのスレッドもまだ開いていないプロジェクトを同時に指して、提案を 1 つ置く。文書が 1 つなら
        // 提案の番号は重ならない。
        let threads: Vec<_> = (0..THREADS)
            .map(|i| {
                let (host, barrier, project) =
                    (Arc::clone(&host), Arc::clone(&barrier), project.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    let mut id = None;
                    host.with_project(&project, &mut |lent| {
                        let view = lent
                            .proposals
                            .add(lent.document, format!("p{i}"), "", vec![rename("X")])
                            .unwrap();
                        id = Some(view.id);
                    })
                    .unwrap();
                    id.unwrap()
                })
            })
            .collect();
        let mut ids: Vec<u32> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..=THREADS as u32).collect::<Vec<_>>());
        assert_eq!(host.lock().len(), 1);

        // 開き直す・閉じるのと同時に指しても、表に枠が残らないか 1 つになる。
        let threads: Vec<_> = (0..THREADS)
            .map(|i| {
                let (host, project) = (Arc::clone(&host), project.clone());
                std::thread::spawn(move || match i % 3 {
                    0 => drop(host.open(&project, Opening::Load, true)),
                    1 => drop(host.close(&project, true)),
                    _ => edit(&host, &project, "Y"),
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(host.lock().len() <= 1);
        edit(&host, &project, "Z");
        host.close(&project, true).unwrap();
        assert!(host.lock().is_empty());
    }

    #[test]
    fn a_bundle_that_cannot_be_written_saves_nothing() {
        let tmp = TempDir::new("bundle-early");
        let project = tmp.0.join("a.ugoeproj");
        save_quad(&project);
        let before = std::fs::read(project.join("project.json")).unwrap();
        let host = FileHost::new(None);
        edit(&host, &project, "Renamed");
        // 書き出し先のディレクトリが無い: 保存の前に一時ファイルが書けない。
        let err = host
            .save(&project, Some(&tmp.0.join("missing/a.ugoe")))
            .unwrap_err();
        assert!(
            matches!(&err, HostError::Failed(m) if m.contains("nothing was saved")),
            "{err}"
        );
        assert!(modified(&host, &project));
        assert_eq!(std::fs::read(project.join("project.json")).unwrap(), before);
        assert_eq!(tmp.names(), ["a.ugoeproj"]);
    }

    #[test]
    fn a_bundle_is_replaced_whole_or_left_as_it_was() {
        let tmp = TempDir::new("bundle-replace");
        let project = tmp.0.join("a.ugoeproj");
        save_quad(&project);
        let host = FileHost::new(None);
        let bundle = tmp.0.join("a.ugoe");
        std::fs::write(&bundle, b"old").unwrap();
        edit(&host, &project, "First");
        host.save(&project, Some(&bundle)).unwrap();
        assert!(!modified(&host, &project));
        assert_ne!(std::fs::read(&bundle).unwrap(), b"old");
        assert_eq!(tmp.names(), ["a.ugoe", "a.ugoeproj"]);

        // 名前を変えて置き換えられない (行き先が空でないディレクトリ): プロジェクトは保存済みになり、
        // 行き先は前のまま、一時ファイルは残らない。
        let blocked = tmp.0.join("blocked.ugoe");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("keep"), b"keep").unwrap();
        edit(&host, &project, "Second");
        let err = host.save(&project, Some(&blocked)).unwrap_err();
        assert!(
            matches!(&err, HostError::Failed(m) if m.starts_with("saved the project, but failed to write")),
            "{err}"
        );
        assert!(!modified(&host, &project));
        assert_eq!(std::fs::read(blocked.join("keep")).unwrap(), b"keep");
        assert_eq!(tmp.names(), ["a.ugoe", "a.ugoeproj", "blocked.ugoe"]);
    }

    #[test]
    fn approvals_pass_answers_between_threads() {
        let approvals = Arc::new(Approvals::new());
        let asked = Arc::new(Mutex::new(Vec::new()));
        let ask = || {
            let approvals = Arc::clone(&approvals);
            let asked = Arc::clone(&asked);
            std::thread::spawn(move || {
                approvals.request(Duration::from_secs(10), "no", |id| {
                    asked.lock().unwrap().push(id);
                })
            })
        };
        let yes = ask();
        // 確認が出るまで待ってから答える。
        let first = loop {
            if let Some(&id) = asked.lock().unwrap().first() {
                break id;
            }
            std::thread::yield_now();
        };
        assert_eq!(approvals.pending(), vec![first]);
        assert!(approvals.answer(first, true));
        assert!(yes.join().unwrap().is_ok());
        assert!(!approvals.answer(first, true), "already answered");

        let no = ask();
        let second = loop {
            if let Some(&id) = asked.lock().unwrap().get(1) {
                break id;
            }
            std::thread::yield_now();
        };
        assert!(approvals.answer(second, false));
        assert!(matches!(no.join().unwrap(), Err(HostError::Denied(m)) if m == "no"));
        assert!(approvals.pending().is_empty());
    }

    #[test]
    fn cancelling_the_request_declines_its_approval() {
        let approvals = Arc::new(Approvals::new());
        let cancellation = Arc::new(Cancellation::new(None));
        let waiting = {
            let approvals = Arc::clone(&approvals);
            let cancellation = Arc::clone(&cancellation);
            std::thread::spawn(move || {
                cancellation.scope(|| approvals.request(Duration::from_secs(10), "no", |_| {}))
            })
        };
        while approvals.pending().is_empty() {
            std::thread::yield_now();
        }
        cancellation.cancel();
        assert!(matches!(waiting.join().unwrap(), Err(HostError::Denied(m)) if m == CANCELLED));
        assert!(approvals.pending().is_empty());

        // 待つ前に取り消されていれば、確認を出さない。接続が閉じていれば取り消す。
        let closed = Arc::new(Cancellation::new(Some(Box::new(|| true))));
        let mut asked = false;
        let result =
            closed.scope(|| approvals.request(Duration::from_secs(10), "no", |_| asked = true));
        assert!(matches!(result, Err(HostError::Denied(m)) if m == CANCELLED));
        assert!(!asked);
        // 印はスコープの外に残らない。
        assert!(Cancellation::current().is_none());
    }

    #[test]
    fn approvals_time_out_and_deny_all() {
        let approvals = Approvals::new();
        let result = approvals.request(Duration::from_millis(10), "no", |_| {});
        assert!(matches!(result, Err(HostError::TimedOut)));
        assert!(approvals.pending().is_empty());

        let approvals = Arc::new(Approvals::new());
        let waiting = {
            let approvals = Arc::clone(&approvals);
            std::thread::spawn(move || {
                approvals.request(Duration::from_secs(10), "stopped", |_| {})
            })
        };
        while approvals.pending().is_empty() {
            std::thread::yield_now();
        }
        approvals.deny_all();
        assert!(matches!(waiting.join().unwrap(), Err(HostError::Denied(m)) if m == "stopped"));
    }
}
