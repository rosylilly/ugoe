// プレビューの着せ替え (docs/editor.md#プレビューの着せ替え): `.ugoemod` を読み込み、ビューポートの WebPuppet に
// 付け外しする。モデルは変えない (コマンドを送らない。Undo の対象ではない)。
//
// - モジュールはプレビューの間だけ付ける。出るときに外し (`takeOff`)、入るときに付け直す。編集の画面には本体だけが出る。
// - 付け外しでパラメータの値・ポーズ・トラッキング・モーションは続きから動く (Puppet の規則。物理はリセットされる)。
// - エディタは編集のたびにモデルを読み直す。WebPuppet は付けていたモジュールを付け直し (T0123)、付けられないものは
//   外す。`loaded` で外れたもの (`detached_modules_json`。T0124) を `notices` で知らせ、付けたい組と比べる
//   (プレビューに入ったときなど、付け直しを WebPuppet に任せていないときは、1 つずつ試した DressUpError が理由)。
// - 読み込んだバイト列を持っておき、WebPuppet を作り直したとき (GPU の喪失など) に読み込み直す。
// - 一覧 (パスと付けるか) は project.json の editor.dress_up に残す (ipc.setPreviewModules)。ドロップしたファイルで
//   パスが分からないもの (ブラウザ) は残さない。
import { ipc } from "../ipc";
import type { WebPuppet } from "../viewport/wasm";
import {
  type DetachedModule,
  type DressUpProblem,
  describeProblem,
  fileStem,
  type LoadedModuleInfo,
  parseDetachedModules,
  parseModules,
  sameNames,
} from "./dressUp";

/** パネルの一覧の 1 行。 */
export interface DressUpEntry {
  /** モジュールの名前 (まだ読み込めていなければファイル名)。 */
  name: string;
  /** 取り付け先のスロット (読み込むまで分からない)。 */
  slot: string | null;
  /** `.ugoemod` の絶対パス。ドロップしたファイルでパスが分からなければ `null` (プロジェクトに残さない)。 */
  path: string | null;
  /** WebPuppet に読み込めたか。 */
  loaded: boolean;
  /** プレビューで付けるか (利用者の選択)。 */
  wanted: boolean;
  /** 今付いているか。 */
  attached: boolean;
  /** 読み込み・付け外しの最後の失敗。 */
  problem: DressUpProblem | null;
  /** 読み込めたが注意が要ること (代わりの画像を使ったテクスチャなど)。 */
  warnings: string[];
  requires: LoadedModuleInfo["requires"];
}

/** 読み込むファイル。`bytes` がなければ `path` から読む。 */
export interface ModuleSource {
  path: string | null;
  bytes?: Uint8Array;
  /** ファイル名 (パスの分からないドロップしたファイルの、エラーの見出し)。 */
  name?: string;
}

export interface DressUpHost {
  /** ビューポートの WebPuppet (プレビューの外でも)。 */
  puppet(): WebPuppet | null;
  /** プレビュー中か (付けるのはプレビュー中だけ)。 */
  active(): boolean;
  /** 付け外しの後 (スライダーの表示を写し直す)。 */
  changed(puppet: WebPuppet): void;
}

export class DressUp {
  entries = $state<DressUpEntry[]>([]);
  /** 自動で外れたことの知らせ (モデルを読み直した後など)。利用者が閉じるまで残す。 */
  notices = $state<string[]>([]);
  /** 読み込みの失敗 (一覧の行にならないもの)。 */
  error = $state<DressUpProblem | null>(null);
  /** ファイルを読んでいる間。 */
  busy = $state(false);

  /** 一覧をプロジェクトに残した後 (未保存の印が変わる) に呼ぶ。 */
  onsaved: (() => void) | null = null;

  #host: DressUpHost;
  /** 読み込んだモジュールのバイト列 (名前 → 中身)。WebPuppet を作り直したときに読み込み直す。 */
  #bytes = new Map<string, Uint8Array>();

  constructor(host: DressUpHost) {
    this.#host = host;
  }

  /** 開いているプロジェクトの一覧を読み直す (起動・新規・開くの後)。前のプロジェクトのモジュールは捨てる。 */
  async load(): Promise<void> {
    const puppet = this.#host.puppet();
    if (puppet) {
      for (const { name } of this.#infos(puppet)) {
        try {
          puppet.unload_module(name);
        } catch (e) {
          console.warn("ugoe:", e);
        }
      }
    }
    this.#bytes.clear();
    this.notices = [];
    this.error = null;
    try {
      const stored = await ipc.getPreviewModules();
      this.entries = stored.map((m) => ({
        name: fileStem(m.path),
        slot: null,
        path: m.path,
        loaded: false,
        wanted: m.attached,
        attached: false,
        problem: null,
        warnings: [],
        requires: {},
      }));
    } catch (e) {
      this.entries = [];
      this.error = describeProblem(e);
    }
  }

  /**
   * プレビューに入ったとき: パスのあるモジュールをファイルから読み直し (書き出し直した新しい版を使う)、
   * 付けたいものを付ける。
   */
  async refresh(): Promise<void> {
    const paths = this.entries.filter((e) => e.path !== null);
    if (paths.length === 0) return;
    this.busy = true;
    try {
      for (const entry of paths) {
        await this.#read(entry);
      }
    } finally {
      this.busy = false;
    }
    const puppet = this.#puppet();
    if (puppet) this.#reconcile(puppet);
  }

  /** ファイルを読み込んで付ける (同じスロットのモジュールは差し替わる)。一覧は残す。 */
  async add(sources: ModuleSource[]): Promise<void> {
    this.error = null;
    this.busy = true;
    try {
      for (const source of sources) {
        const name = await this.#load(source);
        if (name !== null) this.attach(name, false);
      }
    } finally {
      this.busy = false;
    }
    await this.#save();
  }

  /** モジュールを付ける。同じスロットに付いているモジュールは外れる。 */
  attach(name: string, save = true): void {
    const puppet = this.#puppet();
    const entry = this.#entry(name);
    if (!puppet || !entry) return;
    try {
      puppet.attach_module(name);
      entry.problem = null;
      this.#afterChange(puppet);
    } catch (e) {
      entry.problem = describeProblem(e);
      entry.wanted = false;
    }
    if (save) void this.#save();
  }

  /** モジュールを外す。 */
  detach(name: string): void {
    const puppet = this.#puppet();
    const entry = this.#entry(name);
    if (!puppet || !entry) return;
    try {
      if (entry.attached) puppet.detach_module(name);
      entry.problem = null;
      this.#afterChange(puppet);
    } catch (e) {
      entry.problem = describeProblem(e);
    }
    entry.wanted = false;
    void this.#save();
  }

  /** スロットに付けるモジュールを選ぶ。`null` でそのスロットのモジュールを外す。 */
  setSlot(slot: string, name: string | null): void {
    if (name !== null) {
      this.attach(name);
      return;
    }
    const current = this.entries.find((e) => e.slot === slot && e.attached);
    if (current) this.detach(current.name);
  }

  /** ファイルから読み直す (書き出し直したとき)。付いていれば新しい版に付け替わる。 */
  async reload(name: string): Promise<void> {
    const entry = this.#entry(name);
    if (!entry?.path) return;
    this.busy = true;
    try {
      await this.#read(entry);
    } finally {
      this.busy = false;
    }
    const puppet = this.#puppet();
    if (puppet) this.#reconcile(puppet);
  }

  /** 一覧から除く (付いていれば外す)。 */
  remove(name: string): void {
    const puppet = this.#host.puppet();
    const entry = this.#entry(name);
    if (!entry) return;
    if (puppet && entry.loaded) {
      this.#forget(puppet, name);
      this.#afterChange(puppet);
    }
    this.#bytes.delete(name);
    this.entries = this.entries.filter((e) => e !== entry);
    void this.#save();
  }

  dismissNotice(index: number): void {
    this.notices = this.notices.filter((_, i) => i !== index);
  }

  /** プレビューを出る前に、付けているモジュールをすべて外す (付けたい組は覚えておく)。 */
  takeOff(): void {
    const puppet = this.#host.puppet();
    if (!puppet?.has_model() || puppet.attached_modules().length === 0) return;
    try {
      puppet.set_modules([]);
    } catch (e) {
      console.warn("ugoe:", e);
    }
    for (const entry of this.entries) entry.attached = false;
  }

  /**
   * プレビュー中にモデルを読み込んだ直後 (入った時・読み直し・WebPuppet の作り直し)。持っているモジュールを
   * 読み込み直し、付けたい組に揃える。付けられなかったものは `notices` で知らせる。
   */
  loaded(puppet: WebPuppet): void {
    const known = new Set(this.#infos(puppet).map((m) => m.name));
    for (const [name, bytes] of this.#bytes) {
      if (known.has(name)) continue;
      try {
        puppet.load_module(bytes);
      } catch (e) {
        const entry = this.#entry(name);
        if (entry) entry.problem = describeProblem(e);
      }
    }
    // 読み直しで WebPuppet が外したもの (理由は付け直しを試した DressUpError の code)。
    // 一覧は直前の読み込みのものなので、その後で付け直したものは除く。
    const attached = puppet.attached_modules();
    for (const detached of this.#detached(puppet)) {
      if (!attached.includes(detached.module)) {
        this.#takenOff(detached.module, describeProblem(detached.error));
      }
    }
    this.#reconcile(puppet);
  }

  // --- 内部 ---

  #puppet(): WebPuppet | null {
    const puppet = this.#host.puppet();
    return this.#host.active() && puppet?.has_model() ? puppet : null;
  }

  #entry(name: string): DressUpEntry | undefined {
    return this.entries.find((e) => e.name === name);
  }

  #infos(puppet: WebPuppet): LoadedModuleInfo[] {
    try {
      return parseModules(puppet.modules_json());
    } catch (e) {
      console.warn("ugoe:", e);
      return [];
    }
  }

  #detached(puppet: WebPuppet): DetachedModule[] {
    try {
      return parseDetachedModules(puppet.detached_modules_json());
    } catch (e) {
      console.warn("ugoe:", e);
      return [];
    }
  }

  /** 付けられなかったモジュールを付けたい組から外し、知らせる。 */
  #takenOff(name: string, problem: DressUpProblem): void {
    const entry = this.#entry(name);
    if (entry) {
      entry.wanted = false;
      entry.problem = problem;
    }
    this.notices = [...this.notices, `${name} was taken off: ${problem.summary}`];
  }

  /** `entry.path` から読み直す。読めなければ `entry.problem`。 */
  async #read(entry: DressUpEntry): Promise<void> {
    if (!entry.path) return;
    try {
      const bytes = await ipc.readModuleFile(entry.path);
      await this.#load({ path: entry.path, bytes }, entry);
    } catch (e) {
      entry.problem = describeProblem(e);
    }
  }

  /**
   * バイト列を WebPuppet に読み込み、一覧の行を作るか更新する。読み込めたらモジュールの名前を返す。
   * `entry` (ファイルから読み直す行) がなければ、同じ名前か同じパスの行を置き換える。
   */
  async #load(source: ModuleSource, entry?: DressUpEntry): Promise<string | null> {
    const puppet = this.#host.puppet();
    if (!puppet) return null;
    let bytes: Uint8Array;
    let name: string;
    try {
      bytes =
        source.bytes ?? (source.path ? await ipc.readModuleFile(source.path) : new Uint8Array());
      name = puppet.load_module(bytes);
    } catch (e) {
      const problem = describeProblem(e);
      if (entry) entry.problem = problem;
      else
        this.error = {
          ...problem,
          summary: `${fileStem(source.path ?? source.name ?? "File")}: ${problem.summary}`,
        };
      return null;
    }
    // 名前が変わった (別のモジュールを同じパスに書き出した) なら、前の名前のものを捨てる。
    if (entry?.loaded && entry.name !== name) this.#forget(puppet, entry.name);
    this.#bytes.set(name, bytes);
    const row: DressUpEntry = entry ?? {
      name,
      slot: null,
      path: source.path,
      loaded: true,
      wanted: true,
      attached: false,
      problem: null,
      warnings: [],
      requires: {},
    };
    row.name = name;
    row.loaded = true;
    // 付け外しの失敗 (付けたときに出たもの) は残し、ファイルの読み込みの失敗だけを消す。
    if (row.problem?.code === null || row.problem?.code === "invalid_file") row.problem = null;
    if (source.path !== null) row.path = source.path;
    if (!entry) {
      // 同じ名前か同じパスの行は、その場所で置き換える。
      const same = (e: DressUpEntry) =>
        e.name === name || (source.path !== null && e.path === source.path);
      const index = this.entries.findIndex(same);
      const replaced = this.entries[index];
      if (replaced && row.path === null) row.path = replaced.path;
      if (replaced?.loaded && replaced.name !== name) this.#forget(puppet, replaced.name);
      const others = this.entries.filter((e) => !same(e));
      if (index < 0) others.push(row);
      else others.splice(Math.min(index, others.length), 0, row);
      this.entries = others;
    }
    this.#update(puppet);
    return name;
  }

  /** 読み込んだモジュール `name` を WebPuppet と手元から捨てる。 */
  #forget(puppet: WebPuppet, name: string): void {
    this.#bytes.delete(name);
    try {
      puppet.unload_module(name);
    } catch (e) {
      console.warn("ugoe:", e);
    }
  }

  /** 付けたい組 (`wanted`) に揃える。付けられないものは外して知らせる。 */
  #reconcile(puppet: WebPuppet): void {
    const wanted = this.entries.filter((e) => e.wanted && e.loaded).map((e) => e.name);
    if (!sameNames(puppet.attached_modules(), wanted)) {
      try {
        puppet.set_modules(wanted);
      } catch {
        // 1 つずつ試し、付けられないものだけを外す (理由は DressUpError の code から)。
        const accepted: string[] = [];
        for (const name of wanted) {
          try {
            puppet.set_modules([...accepted, name]);
            accepted.push(name);
          } catch (e) {
            this.#takenOff(name, describeProblem(e));
          }
        }
        try {
          puppet.set_modules(accepted);
        } catch (e) {
          console.warn("ugoe:", e);
        }
      }
    }
    this.#afterChange(puppet);
  }

  #afterChange(puppet: WebPuppet): void {
    this.#update(puppet);
    // 同じスロットの差し替えで外れたものは、付けたい組からも外す。
    for (const entry of this.entries) {
      if (entry.loaded) entry.wanted = entry.attached;
    }
    this.#host.changed(puppet);
  }

  /** 一覧の行を WebPuppet の状態 (スロット・付いているか・警告) に合わせる。 */
  #update(puppet: WebPuppet): void {
    const infos = new Map(this.#infos(puppet).map((m) => [m.name, m]));
    for (const entry of this.entries) {
      const info = infos.get(entry.name);
      entry.loaded = info !== undefined;
      entry.attached = info?.attached ?? false;
      if (info) {
        entry.slot = info.slot;
        entry.warnings = info.warnings;
        entry.requires = info.requires;
      }
    }
  }

  /** 一覧をプロジェクトに残す (パスの分かるものだけ)。 */
  async #save(): Promise<void> {
    const modules = this.entries.flatMap((e) =>
      e.path === null ? [] : [{ path: e.path, attached: e.wanted }],
    );
    try {
      await ipc.setPreviewModules(modules);
      this.onsaved?.();
    } catch (e) {
      this.error = describeProblem(e);
    }
  }
}
