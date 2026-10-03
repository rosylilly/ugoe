// W3C WebDriver の小さなクライアント (tests/e2e/README.md)。
//
// e2e で使う範囲だけを持つ: セッション・要素の検索と操作・スクリプトの実行・ウィンドウの切り替え・スクリーンショット。
// 相手は tauri-driver (Linux では WebKitWebDriver に中継する)。仕様: https://www.w3.org/TR/webdriver2/

/** 要素の参照の鍵 (W3C WebDriver の web element identifier)。 */
const ELEMENT = "element-6066-11e4-a52e-4f735466cecf";

export class WebDriverError extends Error {
  /**
   * @param {string} code WebDriver のエラーコード (`no such element` など)
   * @param {string} message
   */
  constructor(code, message) {
    super(`${code}: ${message}`);
    this.code = code;
  }
}

/** ドライバへの要求を送り、`value` を返す。`timeout` (ミリ秒) を過ぎたら `TimeoutError` で失敗する。 */
async function request(base, method, path, body, timeout = 60_000) {
  const res = await fetch(`${base}${path}`, {
    method,
    // 応答しない要求 (隠れたウィンドウのスクリーンショットなど) で筋書き全体が止まらないように
    signal: AbortSignal.timeout(timeout),
    headers: body === undefined ? {} : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    throw new WebDriverError("invalid response", `${method} ${path}: ${res.status} ${text}`);
  }
  const value = json?.value;
  // エラーは 4xx・5xx で返る (成功した値がたまたま `error` を持つこともあるので、状態のコードで見分ける)
  if (!res.ok) {
    throw new WebDriverError(value?.error ?? String(res.status), value?.message ?? text);
  }
  return value;
}

/** 条件が真になるまで待つ。`fn` が返した真の値を返す。 */
export async function waitFor(
  fn,
  { timeout = 10_000, interval = 100, message = "condition" } = {},
) {
  const end = Date.now() + timeout;
  let last;
  for (;;) {
    try {
      const value = await fn();
      if (value) return value;
      last = value;
    } catch (e) {
      last = e;
    }
    if (Date.now() > end) {
      const detail = last instanceof Error ? last.message : JSON.stringify(last);
      throw new Error(`timed out after ${timeout}ms waiting for ${message} (last: ${detail})`);
    }
    await new Promise((r) => setTimeout(r, interval));
  }
}

export class Element {
  /** @param {Session} session @param {string} id */
  constructor(session, id) {
    this.session = session;
    this.id = id;
  }

  #call(method, path, body) {
    return this.session.call(method, `/element/${this.id}${path}`, body);
  }

  /**
   * クリックする。先に要素を画面の中ほどへスクロールする (WebKitWebDriver は、スクロールする入れ物の中の見えていない要素を
   * 自分ではスクロールせず「element not interactable」にする)。
   */
  async click() {
    await this.session.execute("arguments[0].scrollIntoView({ block: 'center' })", this);
    return this.#call("POST", "/click", {});
  }

  /** 入力欄に文字を打つ (今の値の後ろに足す。消すときは先に `clear`)。 */
  type(text) {
    return this.#call("POST", "/value", { text });
  }

  clear() {
    return this.#call("POST", "/clear", {});
  }

  /**
   * 入力欄の中身を `text` に置き換えて Enter を押す。ページの中でフォーカスして全体を選んでから打つ (WebDriver の
   * Element Send Keys は、フォーカスのある要素では選択を保つので、打った文字が選択を置き換える)。
   * `clear` は先に change を起こし、その結果の再描画が打っている途中の文字を消すことがあるので使わない。
   */
  async replace(text) {
    await this.session.execute(
      "arguments[0].scrollIntoView({ block: 'center' }); arguments[0].focus(); arguments[0].select();",
      this,
    );
    await this.type(`${text}${Keys.Enter}`);
  }

  text() {
    return this.#call("GET", "/text");
  }

  attribute(name) {
    return this.#call("GET", `/attribute/${encodeURIComponent(name)}`);
  }

  property(name) {
    return this.#call("GET", `/property/${encodeURIComponent(name)}`);
  }

  displayed() {
    return this.#call("GET", "/displayed");
  }

  enabled() {
    return this.#call("GET", "/enabled");
  }

  /** この要素の中から探す。 */
  async find(css) {
    const value = await this.#call("POST", "/element", { using: "css selector", value: css });
    return new Element(this.session, value[ELEMENT]);
  }

  /** スクリプトの引数に渡す形。 */
  toJSON() {
    return { [ELEMENT]: this.id };
  }
}

export class Session {
  /** @param {string} base ドライバの URL @param {string} id セッション ID */
  constructor(base, id) {
    this.base = base;
    this.id = id;
    /** 1 つの要求を待つ時間 (ミリ秒)。ページが固まると、スクリプトの実行などが応答しなくなる。 */
    this.timeout = 30_000;
  }

  /**
   * 新しいセッションを作る (= アプリを起動する)。
   * @param {string} base ドライバの URL (`http://127.0.0.1:4444`)
   * @param {{ application: string, args?: string[] }} tauriOptions
   */
  static async create(base, tauriOptions) {
    const value = await request(base, "POST", "/session", {
      capabilities: { alwaysMatch: { browserName: "wry", "tauri:options": tauriOptions } },
    });
    return new Session(base, value.sessionId);
  }

  call(method, path, body) {
    return request(this.base, method, `/session/${this.id}${path}`, body, this.timeout);
  }

  /** セッションを閉じる (アプリが終わる)。 */
  async close() {
    await request(this.base, "DELETE", `/session/${this.id}`).catch(() => {});
  }

  /** ページの中で同期のスクリプトを実行する。`script` は関数の本体 (引数は `arguments`)。 */
  async execute(script, ...args) {
    return wrap(this, await this.call("POST", "/execute/sync", { script, args }));
  }

  /**
   * ページの中で非同期のスクリプトを実行する。`fn` は async 関数のソース (引数は args)。
   * 例外はメッセージ付きでテストに戻す。
   */
  async run(fn, ...args) {
    const script = `const done = arguments[arguments.length - 1];
      (${fn}).apply(null, Array.prototype.slice.call(arguments, 0, -1))
        .then((value) => done({ ok: true, value }), (e) => done({ ok: false, error: String(e?.stack ?? e) }));`;
    const result = await this.call("POST", "/execute/async", { script, args });
    if (!result?.ok) throw new Error(`script failed: ${result?.error}`);
    return wrap(this, result.value);
  }

  async find(css) {
    const value = await this.call("POST", "/element", { using: "css selector", value: css });
    return new Element(this, value[ELEMENT]);
  }

  async findAll(css) {
    const value = await this.call("POST", "/elements", { using: "css selector", value: css });
    return value.map((v) => new Element(this, v[ELEMENT]));
  }

  /** 要素が現れるまで待って返す。 */
  waitFind(css, timeout = 10_000) {
    return waitFor(() => this.find(css), { timeout, message: `element ${css}` });
  }

  /**
   * `css` に合う要素のうち、文字 (`textContent`。空白を詰める) が `text` を含む最初のものを待って返す (ボタンを名前で探す)。
   * WebDriver の Get Element Text は WebKitWebDriver では入れ子の要素の文字が欠けることがあるので、ページの中で探す。
   */
  waitFindText(css, text, timeout = 10_000) {
    return waitFor(
      () =>
        this.execute(
          `for (const el of document.querySelectorAll(arguments[0])) {
             if ((el.textContent ?? "").replace(/\\s+/g, " ").includes(arguments[1])) return el;
           }
           return null;`,
          css,
          text,
        ),
      { timeout, message: `${css} with text "${text}"` },
    );
  }

  title() {
    return this.call("GET", "/title");
  }

  url() {
    return this.call("GET", "/url");
  }

  windowHandles() {
    return this.call("GET", "/window/handles");
  }

  windowHandle() {
    return this.call("GET", "/window");
  }

  switchToWindow(handle) {
    return this.call("POST", "/window", { handle });
  }

  /** 今のウィンドウのスクリーンショット (PNG のバイト列)。 */
  async screenshot() {
    const base64 = await this.call("GET", "/screenshot");
    return Buffer.from(base64, "base64");
  }

  /** キーを押して離す (`Control` などの修飾キーは WebDriver のコード ``)。 */
  keys(...keys) {
    const actions = [];
    for (const k of keys) actions.push({ type: "keyDown", value: k });
    for (const k of [...keys].reverse()) actions.push({ type: "keyUp", value: k });
    return this.call("POST", "/actions", { actions: [{ type: "key", id: "keyboard", actions }] });
  }
}

/** スクリプトの戻り値の中の要素の参照を `Element` にする。 */
function wrap(session, value) {
  if (Array.isArray(value)) return value.map((v) => wrap(session, v));
  if (value && typeof value === "object") {
    if (ELEMENT in value) return new Element(session, value[ELEMENT]);
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, wrap(session, v)]));
  }
  return value;
}

/** WebDriver のキーコード (https://www.w3.org/TR/webdriver2/#keyboard-actions)。 */
export const Keys = {
  /** 押している修飾キーをすべて離す。 */
  Null: "\uE000",
  Enter: "\uE007",
  Escape: "\uE00C",
  Control: "\uE009",
  Shift: "\uE008",
  Alt: "\uE00A",
};
