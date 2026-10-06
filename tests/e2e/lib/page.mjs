// ページの中で使う小さな道具 (画面の部品を名前で探す・値を読む)。`session.run` / `session.execute` から呼ぶ。
// ページで動くコードは関数で書き、関数のまま `session.execute` に渡す (`lib/webdriver.mjs`)。

/**
 * ページの中で `window.__e2e` を置く (何度呼んでもよい)。ページで動くので、このモジュールの変数は使えない。
 * 関数のまま `session.execute` に渡す (文字列にしないので、正規表現のエスケープが二重にならない。T0042)。
 */
function pageHelpers() {
  if (window.__e2e) return;
  const text = (el) => (el.textContent ?? "").replace(/\s+/g, " ").trim();
  window.__e2e = {
    /** @ugoe/ui の Slider のつまみ (ラベルが pattern に合うもの)。 */
    slider(pattern) {
      const re = new RegExp(pattern, "i");
      for (const root of document.querySelectorAll(".ugoe-slider")) {
        const label = root.querySelector(".ugoe-slider__label");
        if (label && re.test(text(label))) return root.querySelector("[role=slider]");
      }
      return null;
    },
    sliderValue(pattern) {
      const thumb = this.slider(pattern);
      return thumb ? Number(thumb.getAttribute("aria-valuenow")) : null;
    },
    /** @ugoe/ui の Toggle のスイッチ (ラベルが pattern に合うもの)。 */
    toggle(pattern) {
      const re = new RegExp(pattern, "i");
      for (const label of document.querySelectorAll(".ugoe-toggle__label")) {
        if (re.test(text(label))) return document.getElementById(label.htmlFor);
      }
      return null;
    },
    toggleChecked(pattern) {
      const sw = this.toggle(pattern);
      return sw
        ? sw.getAttribute("aria-checked") === "true" || sw.getAttribute("data-state") === "checked"
        : null;
    },
    /** selector に合い、文字が pattern に合う最初の要素。 */
    byText(selector, pattern) {
      const re = new RegExp(pattern, "i");
      for (const el of document.querySelectorAll(selector)) if (re.test(text(el))) return el;
      return null;
    },
    /** 役割 (role 属性) と名前 (aria-label、無ければ文字) が pattern に合う最初の要素 (アイコンだけのタブなど)。 */
    byRole(role, pattern) {
      const re = new RegExp(pattern, "i");
      for (const el of document.querySelectorAll(`[role="${role}"]`)) {
        if (re.test(el.getAttribute("aria-label") ?? text(el))) return el;
      }
      return null;
    },
    /**
     * 名前 (aria-label か aria-labelledby) が listName に合うリストの中で、文字が pattern に合う最初の項目。
     * クラス名に頼らず、役割と名前で探す (スタジオの「いまの状態」の印など)。
     */
    listItem(listName, pattern) {
      const nameRe = new RegExp(listName, "i");
      const re = new RegExp(pattern, "i");
      const accessibleName = (el) => {
        const ids = el.getAttribute("aria-labelledby");
        if (ids) {
          return ids
            .trim()
            .split(/\s+/)
            .map((id) => document.getElementById(id))
            .filter((el) => el !== null)
            .map(text)
            .join(" ");
        }
        return el.getAttribute("aria-label") ?? "";
      };
      for (const list of document.querySelectorAll("ul, ol, [role=list]")) {
        if (!nameRe.test(accessibleName(list))) continue;
        for (const item of list.querySelectorAll("li, [role=listitem]")) {
          if (re.test(text(item))) return item;
        }
      }
      return null;
    },
    text,
  };
}

/** ページに `window.__e2e` を置く (何度呼んでもよい)。 */
export async function installPageHelpers(session) {
  await session.execute(pageHelpers);
}

/**
 * ページの中で Tauri の invoke を数える仕組みを置き、`commands` を数える対象に足す。ページで動くので、このモジュールの
 * 変数は使えない (`commands` は引数で受け取る)。
 */
function watchInvokes(commands) {
  if (!window.__e2eInvoke) {
    const stats = { watch: new Set(), calls: {}, done: {}, errors: {}, latency: {} };
    window.__e2eInvoke = stats;
    const original = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const p = original(input, init);
      const url = String(input);
      const prefix = "ipc://localhost/";
      const cmd = url.startsWith(prefix)
        ? decodeURIComponent(url.slice(prefix.length).split("?")[0])
        : null;
      if (cmd === null || !stats.watch.has(cmd)) return p;
      stats.calls[cmd] = (stats.calls[cmd] ?? 0) + 1;
      const started = performance.now();
      p.then(
        (res) => {
          if (res.ok && res.headers.get("Tauri-Response") === "ok") {
            stats.done[cmd] = (stats.done[cmd] ?? 0) + 1;
            stats.latency[cmd] ??= [];
            stats.latency[cmd].push(performance.now() - started);
          } else {
            stats.errors[cmd] = (stats.errors[cmd] ?? 0) + 1;
          }
        },
        () => {
          stats.errors[cmd] = (stats.errors[cmd] ?? 0) + 1;
        },
      );
      return p;
    };
  }
  for (const cmd of commands) window.__e2eInvoke.watch.add(cmd);
}

/**
 * Tauri の invoke を数え、応答の時間を測る (`window.__e2eInvoke`)。`commands` に挙げたものだけ。
 * `__TAURI_INTERNALS__.invoke` は書き換えられない (writable: false) ので、IPC の要求 (Linux では `ipc://localhost/<コマンド>`
 * への fetch) を包む。T0812 の調査と同じ方法。
 */
export async function countInvokes(session, commands) {
  await session.execute(watchInvokes, commands);
}

/** `countInvokes` の結果 (Set は JSON にならないので除く)。 */
export function invokeStats(session) {
  return session.execute(() => {
    const { watch, ...rest } = window.__e2eInvoke ?? { watch: null };
    return rest;
  });
}

/** 小さい順に並べた値の p 分位 (0〜1)。 */
export function percentile(values, p) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
}
