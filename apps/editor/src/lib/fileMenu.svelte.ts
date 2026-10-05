// ファイルメニューの流れ (開く・保存・書き出し・外での書き換え・本体の変化・本体の問題の直し方)。
// FileMenu.svelte は表示と呼び出しだけを持ち、流れはここに置く (fileMenu.test.ts で IPC を偽物にして確かめる)。
// docs/editor.md#外での書き換え、docs/editor.md#本体の変化、docs/editor.md#本体の問題の直し方。
//
// 操作は重ねない: 利用者の操作 (保存・上書き・開き直し・ダイアログで選ぶ操作など) は `busy` を立てて 1 つずつ行い、
// 実行中に来たものは捨てる (ボタンも押せない)。ウィンドウに戻ったとき・一定の間隔の確かめ (`checkExternal`) は、
// 操作の途中なら行わず、確かめの途中に来た確かめは同じ確かめを待つ。確かめの途中に来た利用者の操作は、確かめ
// (自動の開き直しまで) が終わるのを待ってから行う。
import { errorText } from "./errors";
import { exportSummary, moduleExportSummary } from "./exportSummary";
import {
  baseAction,
  externalAction,
  isExternalChangeError,
  loadAutoReload,
  saveAutoReload,
} from "./externalChange";
import { type FileDialogs, fileDialogs } from "./fileDialogs";
import {
  BUNDLE_FILTER,
  joinPath,
  MODULE_FILTER,
  PROJECT_FILTER,
  PSD_FILTER,
  parentDir,
  safeFileName,
  withExtension,
} from "./filePaths";
import { t } from "./i18n";
import {
  type ApplyReport,
  type Command,
  type ExternalChange,
  ipc,
  type ModuleProblem,
  type ModuleStatus,
  type ProjectStatus,
  type PsdImportWarning,
} from "./ipc";
import { baseIsProject, droppedCollidersNotice } from "./moduleProject";

/** ファイルダイアログでパスを選んで行う操作。 */
export type PathAction =
  | "newFromPsd"
  | "open"
  | "openSample"
  | "saveAs"
  | "export"
  | "exportModule"
  | "importPsd"
  | "locateBaseProject"
  | "locateBaseBundle";

/**
 * 成功した後のモデルの読み直し: しない、同じモデルの続き (取り込み)、別のモデルに替わった、
 * 同じプロジェクトを開き直した (外での書き換えの後)。
 */
type Reload = "none" | "same" | "replaced" | "reloaded";

/**
 * 本体の問題で開けなかった・読み直せなかったモジュールのプロジェクトと、その問題 (直し方を出す)。
 * `project` は付け替えて開き直すプロジェクトの場所 (未保存なら null)。`opening` は開けなかったこと
 * (読み直せなかっただけなら、開き直さずに reload_base で付け替えるので、保存していない編集は残る)。
 * `base` は選び直した本体 (それでも開けなかったとき。次の直し方でも使う)。
 */
export interface ModuleProblemState {
  project: string | null;
  problem: ModuleProblem;
  opening: boolean;
  base: string | null;
}

/** ファイルメニューの外へのつながり (画面とドキュメント)。 */
export interface FileMenuHost {
  /**
   * 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。`replaced` は別のモデルに替わったこと
   * (新規・PSD から新規・開く。パラメータのスライダーを既定値に戻す)。PSD の取り込みは `false`。
   * `reloaded` は同じプロジェクトを開き直したこと (外での書き換えの後。表示の位置を保つ)。
   */
  onchange: (replaced: boolean, reloaded: boolean) => void;
  /** モジュールの名前を変えた (作業用のモデルの ID の接頭辞が変わる。読み直す前に選択を付け替える)。 */
  onrenamemodule?: (from: string, to: string) => void;
  /** 適用の報告を返す IPC を呼んで読み直す (`EditorDocument.applyWith`。失敗は例外のまま返す)。 */
  applyWith: (
    action: () => Promise<ApplyReport>,
    options: { rethrow: true },
  ) => Promise<ApplyReport | null>;
  /** 保存・書き出しなどの結果を知らせる (Toast)。`long` は読むのに時間のかかる案内。 */
  inform: (text: string, long: boolean) => void;
  /** ファイルダイアログ (既定は `fileDialogs()`)。 */
  dialogs?: () => FileDialogs;
}

export class FileMenuFlow {
  status = $state<ProjectStatus | null>(null);
  /** モジュールのプロジェクトなら、本体とスロット (docs/editor.md#モジュールのプロジェクト)。 */
  moduleInfo = $state<ModuleStatus | null>(null);
  /** 新しいモジュールの画面を開いているか。 */
  newModuleOpen = $state(false);
  /** 本体の問題で開けなかった・読み直せなかったモジュールのプロジェクトと、その問題。 */
  problem = $state<ModuleProblemState | null>(null);
  /** モジュールの設定の画面を開いているか。 */
  settingsOpen = $state(false);
  /** 本体の変化 (確認のダイアログを開いている間だけ)。 */
  baseChange = $state<ExternalChange | null>(null);
  /** ダイアログを開いているか、その後の処理の途中か (利用者の操作を 1 つずつ行う)。 */
  busy = $state(false);
  /** 未保存の変更を破棄する操作 (確かめのダイアログで「Discard」を選んだら続ける)。 */
  pendingDiscard = $state<(() => void) | null>(null);
  /** 失敗の文 (知らせの帯に出す)。 */
  message = $state<string | null>(null);
  /** 最後の PSD の取り込み (または .ugoemod の書き出し) の警告。 */
  warnings = $state<PsdImportWarning[]>([]);
  /** 書き出しの設定の欄を開いているか。 */
  exportOpen = $state(false);
  /** 最後の書き出し (.ugoe か .ugoemod) の結果の要約の行 (テクスチャの枚数と状態の切り替えの変化・アトラス化)。 */
  exportLines = $state<string[] | null>(null);
  /** 最後にダイアログで選んだ場所 (次のダイアログの最初の場所)。 */
  lastDir = $state<string | null>(null);
  /** 外での書き換え (確認のダイアログを開いている間だけ)。 */
  external = $state<ExternalChange | null>(null);
  /** 保存しようとして外での書き換えが見つかったか (ダイアログで上書きを選べる)。 */
  externalOnSave = $state(false);
  autoReload = $state(loadAutoReload());

  /** 「後で」を選んだ外の状態の目印 (同じ変更を二度尋ねない。保存の前は尋ねる)。 */
  #dismissedToken: string | null = null;
  /** 「後で」を選んだ本体の状態の目印。 */
  #dismissedBaseToken: string | null = null;
  /** 途中の確かめ (ウィンドウに戻ったとき・一定の間隔)。 */
  #checking: Promise<void> | null = null;
  #host: FileMenuHost;

  constructor(host: FileMenuHost) {
    this.#host = host;
  }

  /** 状態 (名前・未保存の印) を取り直す。編集の後に親から呼ぶ。 */
  async refresh(): Promise<void> {
    try {
      this.status = await ipc.projectStatus();
      this.moduleInfo = await ipc.moduleStatus();
    } catch (e) {
      this.message = errorText(e);
    }
  }

  /** 書き出しの設定を残した後のプロジェクトの状態 (未保存の印)。 */
  setStatus(status: ProjectStatus): void {
    this.status = status;
  }

  // --- 操作を重ねない ---

  /**
   * 利用者の操作を行う。確かめの途中なら終わるのを待ち、別の操作の途中なら捨てる (`false`)。
   * `task` の例外はそのまま返す (ダイアログが自分のエラーとして出すもの)。
   */
  async #exclusive(task: () => Promise<void>): Promise<boolean> {
    while (this.#checking) await this.#checking;
    if (this.busy) return false;
    this.busy = true;
    try {
      await task();
    } finally {
      this.busy = false;
    }
    return true;
  }

  /** 成功したら `true`。失敗はメッセージに出す。 */
  async #run(task: () => Promise<ProjectStatus | undefined>, reload: Reload): Promise<boolean> {
    try {
      const result = await task();
      if (result) this.status = result;
      this.message = null;
      this.moduleInfo = await ipc.moduleStatus();
      if (reload === "replaced") {
        // 別のプロジェクトの設定と結果を見せないよう、書き出しの欄を閉じる
        this.exportOpen = false;
        this.exportLines = null;
      }
      if (reload !== "none") this.#host.onchange(reload === "replaced", reload === "reloaded");
      return true;
    } catch (e) {
      this.message = errorText(e);
      return false;
    }
  }

  #inform(text: string, long = false): void {
    this.#host.inform(text, long);
  }

  // --- 未保存の変更を捨てる確かめ ---

  /**
   * 保存していない変更を捨てる操作: 変更があれば確かめてから `proceed` を呼ぶ。返す Promise は、すぐに続けたなら
   * その操作が終わったとき、確かめるならすぐに解決する。
   */
  async #withDiscard(proceed: () => unknown): Promise<void> {
    if (!this.status?.modified) {
      await proceed();
      return;
    }
    this.pendingDiscard = () => void proceed();
  }

  confirmDiscard(): void {
    const proceed = this.pendingDiscard;
    this.pendingDiscard = null;
    proceed?.();
  }

  cancelDiscard(): void {
    this.pendingDiscard = null;
  }

  // --- 新規 ---

  async newProject(): Promise<void> {
    if (this.busy) return;
    await this.#withDiscard(() =>
      this.#exclusive(async () => void (await this.#run(ipc.newProject, "replaced"))),
    );
  }

  async newModule(): Promise<void> {
    if (this.busy) return;
    await this.#withDiscard(() => {
      this.newModuleOpen = true;
    });
  }

  /** 新しいモジュールの画面の「Create」(失敗は例外のまま画面に返す)。 */
  async createModule(base: string, slot: string, name: string): Promise<void> {
    await this.#exclusive(async () => {
      this.status = await ipc.newModuleProject(base, slot, name);
      this.newModuleOpen = false;
      this.warnings = [];
      this.problem = null;
      await this.#run(async () => undefined, "replaced");
    });
  }

  // --- パスを選ぶ操作 ---

  /** ダイアログの最初の場所: 最後に選んだ場所、プロジェクトの隣、ドキュメント (なければホーム) の順。 */
  async #startDir(dialogs: FileDialogs): Promise<string | null> {
    return (
      this.lastDir ??
      (this.status?.path ? parentDir(this.status.path) : null) ??
      (await dialogs.defaultDirectory())
    );
  }

  /** 操作に合うダイアログを開き、選ばれたパスを返す。取り消したら `null`。 */
  async #choose(kind: PathAction): Promise<string | null> {
    const dialogs = (this.#host.dialogs ?? fileDialogs)();
    const dir = (await this.#startDir(dialogs)) ?? undefined;
    /** ファイル名だけを既定にするときの、場所を付けたパス。 */
    const suggest = (name: string) => (dir ? joinPath(dir, name) : name);
    const name = safeFileName(this.status?.name ?? "Untitled");
    switch (kind) {
      case "newFromPsd":
        return dialogs.openFile({
          title: t("fileMenu.dialog.newFromPsd"),
          defaultPath: dir,
          filters: [PSD_FILTER],
        });
      case "importPsd":
        return dialogs.openFile({
          title: t("fileMenu.dialog.importPsd"),
          defaultPath: dir,
          filters: [PSD_FILTER],
        });
      case "open":
        // .ugoeproj はディレクトリなので、フォルダーを選ぶ
        return dialogs.openDirectory({ title: t("fileMenu.dialog.open"), defaultPath: dir });
      case "openSample":
        return dialogs.openDirectory({ title: t("fileMenu.dialog.openSample"), defaultPath: dir });
      case "saveAs": {
        const path = await dialogs.saveFile({
          title: t("fileMenu.dialog.saveAs"),
          defaultPath: suggest(`${name}.ugoeproj`),
          filters: [PROJECT_FILTER],
        });
        return path && withExtension(path, "ugoeproj");
      }
      case "exportModule": {
        const path = await dialogs.saveFile({
          title: t("fileMenu.dialog.exportModule"),
          defaultPath: suggest(`${safeFileName(this.moduleInfo?.name ?? name)}.ugoemod`),
          filters: [MODULE_FILTER],
        });
        return path && withExtension(path, "ugoemod");
      }
      case "locateBaseProject":
        return dialogs.openDirectory({
          title: t("fileMenu.dialog.locateProject"),
          defaultPath: dir,
        });
      case "locateBaseBundle":
        return dialogs.openFile({
          title: t("fileMenu.dialog.locateBundle"),
          defaultPath: dir,
          filters: [BUNDLE_FILTER],
        });
      case "export": {
        const path = await dialogs.saveFile({
          title: t("fileMenu.dialog.export"),
          defaultPath: suggest(`${name}.ugoe`),
          filters: [BUNDLE_FILTER],
        });
        return path && withExtension(path, "ugoe");
      }
    }
  }

  /** 選んだパスで操作を実行する。成功したら `true`。 */
  async #perform(kind: PathAction, path: string): Promise<boolean> {
    switch (kind) {
      case "newFromPsd":
        return this.#run(async () => {
          this.warnings = [];
          this.warnings = await ipc.newProjectFromPsd(path);
          return ipc.projectStatus();
        }, "replaced");
      case "importPsd":
        return this.#run(async () => {
          this.warnings = [];
          this.warnings = await ipc.importPsd(path);
          return ipc.projectStatus();
        }, "same");
      case "open": {
        const ok = await this.#run(() => ipc.openProject(path), "replaced");
        this.problem = null;
        // モジュールのプロジェクトが本体の問題で開けなければ、直し方を出す
        if (!ok) {
          const found = await ipc.moduleOpenProblem(path).catch(() => null);
          if (found) {
            this.problem = { project: path, problem: found, opening: true, base: null };
            this.message = null;
          }
        }
        return ok;
      }
      case "openSample": {
        // 選んだフォルダーの中に新しいフォルダーを作って書き出し、本体を開く (docs/manual/dress-up.md#見本のプロジェクトを開く)
        const ok = await this.#run(() => ipc.openSampleProject(path), "replaced");
        if (ok) {
          this.problem = null;
          const folder = this.status?.path ? parentDir(this.status.path) : null;
          this.#inform(
            folder
              ? t("fileMenu.notice.sampleOpenedIn", { folder })
              : t("fileMenu.notice.sampleOpened"),
            true,
          );
        }
        return ok;
      }
      case "locateBaseProject":
      case "locateBaseBundle":
        // 開いているモジュールなら開き直さずに付け替える (保存していない編集を保つ)
        return this.problem?.opening
          ? this.#reopenModule(path, null)
          : this.#relocateBase(path, null);
      case "exportModule":
        return this.#run(async () => {
          const report = await ipc.exportModule(path);
          this.warnings = report.warnings.map((w) => ({
            path: [],
            code: w.code,
            args: w.args,
            message: w.message,
          }));
          this.exportLines = moduleExportSummary(report);
          this.#inform(t("fileMenu.notice.exported", { path }));
          return undefined;
        }, "none");
      case "saveAs":
        return this.#run(async () => {
          const result = await ipc.saveProjectAs(path);
          this.#inform(t("fileMenu.notice.saved", { path: result.path ?? path }));
          return result;
        }, "none");
      case "export":
        return this.#run(async () => {
          this.exportLines = exportSummary(await ipc.exportBundle(path));
          this.#inform(t("fileMenu.notice.exported", { path }));
          return undefined;
        }, "none");
    }
  }

  /** ダイアログでパスを選んで操作する。未保存の変更を捨てる操作は、変更があれば先に確かめる。 */
  async pick(kind: PathAction): Promise<void> {
    if (this.busy) return;
    if (kind === "open" || kind === "openSample" || kind === "newFromPsd") {
      await this.#withDiscard(() => this.#pickNow(kind));
    } else await this.#pickNow(kind);
  }

  async #pickNow(kind: PathAction): Promise<void> {
    await this.#exclusive(async () => {
      try {
        const path = await this.#choose(kind);
        this.message = null;
        if (path && (await this.#perform(kind, path))) {
          // 見本なら、次の「Open…」をモジュールのプロジェクトの並ぶフォルダーから始める
          const opened = kind === "openSample" && this.status?.path ? this.status.path : path;
          this.lastDir = parentDir(opened) ?? this.lastDir;
        }
      } catch (e) {
        this.message = errorText(e);
      }
    });
  }

  // --- 本体の問題の直し方 ---

  /**
   * 本体の問題で開けなかったモジュールのプロジェクトを、本体 `base` かスロット `slot` を付け替えて開き直す
   * (`base` が null なら、前に選び直した本体があればそれ)。それでも本体の問題で開けなければ、新しい問題を出す。
   */
  async #reopenModule(
    base: string | null,
    slot: string | null,
    dropColliders = false,
  ): Promise<boolean> {
    const project = this.problem?.project;
    if (!project) return false;
    const useBase = base ?? this.problem?.base ?? null;
    const dropped = dropColliders ? (this.problem?.problem.collider_references ?? []) : [];
    const ok = await this.#run(
      () => ipc.openModuleWithBase(project, useBase, slot, dropColliders),
      "replaced",
    );
    if (ok) {
      this.problem = null;
      this.#inform(
        dropColliders
          ? droppedCollidersNotice(dropped)
          : slot === null
            ? t("fileMenu.notice.openedWithBase")
            : t("fileMenu.notice.openedOnSlot", { slot }),
        true,
      );
    } else {
      const found = await ipc.moduleOpenProblem(project, useBase).catch(() => null);
      if (found) {
        this.problem = { project, problem: found, opening: true, base: useBase };
        this.message = null;
      }
    }
    return ok;
  }

  /**
   * 開いているモジュールのプロジェクトの本体 `base` かスロット `slot` を、開き直さずに付け替える (`base` が null なら、
   * 前に選び直した本体があればそれ)。保存していない編集は残る。付け替えられなければ、新しい問題を出す。
   */
  async #relocateBase(base: string | null, slot: string | null): Promise<boolean> {
    const useBase = base ?? this.problem?.base ?? null;
    this.baseChange = null;
    try {
      const result = await ipc.reloadBase(useBase, slot);
      this.status = result.status;
      if (result.problem) {
        this.problem = {
          project: result.status.path,
          problem: result.problem,
          opening: false,
          base: useBase,
        };
        this.message = null;
        return false;
      }
      this.problem = null;
      await this.#run(async () => undefined, "reloaded");
      this.#inform(
        slot === null ? t("fileMenu.notice.usingBase") : t("fileMenu.notice.movedToSlot", { slot }),
        true,
      );
      return true;
    } catch (e) {
      this.message = errorText(e);
      return false;
    }
  }

  /** 本体の問題の枠の「Use this slot」。 */
  async useSlot(slot: string): Promise<void> {
    await this.#exclusive(async () => {
      if (this.problem?.opening) await this.#reopenModule(null, slot);
      else await this.#relocateBase(null, slot);
    });
  }

  /** 本体の問題の枠の「Remove and open」: 確かめた衝突体の参照を外して開き直す (T0661)。 */
  async dropColliders(): Promise<void> {
    if (!this.problem?.opening) return;
    await this.#exclusive(async () => {
      if (this.problem?.opening) await this.#reopenModule(null, null, true);
    });
  }

  /** 本体の問題の枠の「Open base project」: 本体のプロジェクトを開いて直す (モジュールは後で開き直す)。 */
  async openBase(): Promise<void> {
    const base = this.problem?.problem.base;
    if (!base || this.busy) return;
    await this.#openBaseProject(base);
  }

  /**
   * 適用のエラーの案内の「Open base project」(T1609。docs/editor.md#モジュールの適用のエラーの案内): 開いている
   * モジュールの本体のプロジェクトを開く。保存していない変更があれば確かめる。設定の画面は先に閉じる。
   */
  async openModuleBase(): Promise<void> {
    const base = this.moduleInfo?.base;
    if (!base || !baseIsProject(base) || this.busy) return;
    this.settingsOpen = false;
    await this.#openBaseProject(base);
  }

  async #openBaseProject(base: string): Promise<void> {
    await this.#withDiscard(() =>
      this.#exclusive(async () => {
        if (await this.#run(() => ipc.openProject(base), "replaced")) {
          this.problem = null;
          this.#inform(t("fileMenu.notice.openedBase"), true);
        }
      }),
    );
  }

  dismissProblem(): void {
    this.problem = null;
  }

  dismissMessage(): void {
    this.message = null;
  }

  // --- モジュールの設定 ---

  openSettings(): void {
    this.settingsOpen = true;
  }

  closeSettings(): void {
    this.settingsOpen = false;
  }

  /**
   * モジュールの設定の画面の「Apply」(失敗は例外のまま画面に返す)。ほかの編集と同じく `EditorDocument.applyWith`
   * を通す (適用の報告の警告を出し、ドキュメントを読み直す)。名前を変えたら、読み直す前に選択を付け替える。
   */
  async applyModuleSettings(command: Command): Promise<void> {
    await this.#exclusive(async () => {
      const from = this.moduleInfo?.name;
      await this.#host.applyWith(
        async () => {
          const report = await ipc.applyCommand(command);
          if (command.op === "set_module_settings" && from && from !== command.name) {
            this.#host.onrenamemodule?.(from, command.name);
          }
          return report;
        },
        { rethrow: true },
      );
      this.settingsOpen = false;
      await this.#run(ipc.projectStatus, "reloaded");
    });
  }

  // --- 保存と外での書き換え ---

  async save(): Promise<void> {
    if (this.busy) return;
    if (!this.status?.path) return this.pick("saveAs");
    await this.#exclusive(() => this.#saveNow());
  }

  async #saveNow(): Promise<void> {
    // 保存の前に確かめ、外で変わっていれば選ばせる (バックエンドも保存の直前に確かめて断る)
    const change = await ipc.checkExternalChange().catch(() => null);
    if (change) {
      this.#askExternal(change, true);
      return;
    }
    /** バックエンドが外での書き換えを見つけて保存を断ったか。 */
    let refused = false;
    const saved = await this.#run(async () => {
      try {
        const result = await ipc.saveProject();
        this.#inform(t("fileMenu.notice.saved", { path: result.path ?? "" }));
        return result;
      } catch (e) {
        refused = isExternalChangeError(e);
        throw e;
      }
    }, "none");
    if (!saved && refused) {
      const again = await ipc.checkExternalChange().catch(() => null);
      if (again) {
        this.message = null;
        this.#askExternal(again, true);
      }
    }
  }

  #askExternal(change: ExternalChange, onSave: boolean): void {
    this.externalOnSave = onSave;
    this.external = change;
  }

  /**
   * ウィンドウに戻ったとき・一定の間隔で、外での書き換えと、モジュールのプロジェクトの本体の変化を確かめる。
   * 確かめの途中なら同じ確かめを返し、操作の途中なら確かめない (次の機会に確かめる)。
   */
  checkExternal(): Promise<void> {
    if (this.#checking) return this.#checking;
    if (this.busy) return Promise.resolve();
    const checking = this.#check().finally(() => {
      this.#checking = null;
    });
    this.#checking = checking;
    return checking;
  }

  /** 確かめるのをやめる場面: 操作の途中・確認のダイアログ・破棄の確かめを開いている。 */
  #interrupted(): boolean {
    return (
      this.busy ||
      this.external !== null ||
      this.baseChange !== null ||
      this.pendingDiscard !== null
    );
  }

  async #check(): Promise<void> {
    await this.#checkBase();
    if (this.#interrupted() || !this.status?.path) return;
    let change: ExternalChange | null;
    try {
      change = await ipc.checkExternalChange();
    } catch {
      return; // 書き込みの途中などで読めなければ、次の機会に確かめる
    }
    if (this.#interrupted()) return;
    const action = externalAction(change, {
      modified: this.status?.modified ?? false,
      autoReload: this.autoReload,
      dismissed: this.#dismissedToken,
    });
    // 確かめの途中は利用者の操作が待っているので、ここで busy を立てて開き直す
    if (action === "reload") await this.#busyWith(() => this.#reloadProjectNow(true));
    else if (action === "ask" && change) this.#askExternal(change, false);
  }

  /**
   * モジュールのプロジェクトの本体が変わったか確かめ、変わっていれば尋ねる (保存していなくても確かめる)。
   * 「Reload automatically」なら尋ねずに読み直す (保存していないモジュールの編集は残る)。
   */
  async #checkBase(): Promise<void> {
    if (this.#interrupted() || this.problem || !this.moduleInfo) return;
    const change = await ipc.checkBaseChange().catch(() => null);
    if (this.#interrupted()) return;
    const action = baseAction(change, {
      autoReload: this.autoReload,
      dismissed: this.#dismissedBaseToken,
    });
    if (action === "reload" && change) {
      this.baseChange = change;
      await this.#busyWith(() => this.#reloadBaseNow(true));
    } else if (action === "ask") this.baseChange = change;
  }

  async #busyWith(task: () => Promise<void>): Promise<void> {
    this.busy = true;
    try {
      await task();
    } finally {
      this.busy = false;
    }
  }

  /** 本体の変化の確認の「Reload the base」。 */
  async reloadBase(): Promise<void> {
    await this.#exclusive(() => this.#reloadBaseNow(false));
  }

  /** 本体を読み直す。読み直せなければ、問題と直し方を出す (モジュールはそのまま)。 */
  async #reloadBaseNow(auto: boolean): Promise<void> {
    const token = this.baseChange?.token ?? null;
    this.baseChange = null;
    this.#dismissedBaseToken = null;
    try {
      const result = await ipc.reloadBase();
      this.status = result.status;
      if (result.problem) {
        this.problem = {
          project: result.status.path,
          problem: result.problem,
          opening: false,
          base: null,
        };
        this.message = null;
        // 読み直せなかった同じ本体の状態は、もう尋ねない (直し方の枠で選ぶ)
        this.#dismissedBaseToken = token;
        return;
      }
      this.problem = null;
      await this.#run(async () => undefined, "reloaded");
      this.#inform(
        auto ? t("fileMenu.notice.baseReloadedAuto") : t("fileMenu.notice.baseReloaded"),
      );
    } catch (e) {
      this.message = errorText(e);
    }
  }

  dismissBase(): void {
    if (this.baseChange) this.#dismissedBaseToken = this.baseChange.token;
    this.baseChange = null;
  }

  /** 外での書き換えの確認の「Reload」。 */
  async reloadProject(): Promise<void> {
    await this.#exclusive(() => this.#reloadProjectNow(false));
  }

  async #reloadProjectNow(auto: boolean): Promise<void> {
    this.external = null;
    this.#dismissedToken = null;
    if (await this.#run(ipc.reloadProject, "reloaded")) {
      this.#inform(auto ? t("fileMenu.notice.reloadedAuto") : t("fileMenu.notice.reloaded"));
    }
  }

  /** 外での書き換えの確認の「Overwrite with my version」。 */
  async overwriteExternal(): Promise<void> {
    await this.#exclusive(async () => {
      this.external = null;
      this.#dismissedToken = null;
      await this.#run(async () => {
        const result = await ipc.saveProject(true);
        this.#inform(t("fileMenu.notice.saved", { path: result.path ?? "" }));
        return result;
      }, "none");
    });
  }

  saveAsExternal(): Promise<void> {
    if (this.external) this.#dismissedToken = this.external.token;
    this.external = null;
    return this.pick("saveAs");
  }

  dismissExternal(): void {
    if (this.external) this.#dismissedToken = this.external.token;
    this.external = null;
  }

  setAutoReload(enabled: boolean): void {
    this.autoReload = enabled;
    saveAutoReload(enabled);
  }
}
