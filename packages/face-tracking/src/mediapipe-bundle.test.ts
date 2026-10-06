// 入れている @mediapipe/tasks-vision の配布物が、利用状況を usage-logging.ts で断れる形で送るままかを確かめる (T0947)。
//
// usage-logging.ts は「送り先は https://odml.pa.googleapis.com で、送るたびにグローバルの fetch を呼ぶ」ことに頼っている。
// 版を上げて送り先や送り方 (sendBeacon・XMLHttpRequest・fetch を先に取っておく など) が変わると、包みを素通りして送られる。
// その変化をここで落とす。落ちたら配布物を読み直し、usage-logging.ts と README.md の「利用状況の送信」を直してから
// このテストの期待を合わせる (README.md の「MediaPipe の版を上げるとき」)。

import { readdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { isMediaPipeUsageLog, MEDIAPIPE_USAGE_LOG_ORIGIN } from "./usage-logging";

const packageDir = dirname(createRequire(import.meta.url).resolve("@mediapipe/tasks-vision"));
const { version } = JSON.parse(readFileSync(join(packageDir, "package.json"), "utf8")) as {
  version: string;
};
const read = (file: string) => readFileSync(join(packageDir, file), "utf8");

/** 利用状況を送る処理を持つ JS (ESM・CommonJS・旧来の 3 つ。中身は同じ)。 */
const BUNDLES = ["vision_bundle.mjs", "vision_bundle.cjs", "vision_bundle.js"];
/** wasm とその読み込みの JS (SIMD・SIMD 無し・module)。 */
const WASM_FILES = readdirSync(join(packageDir, "wasm")).filter((f) => /\.(js|wasm)$/.test(f));

const USAGE_LOG_URL = `${MEDIAPIPE_USAGE_LOG_ORIGIN}/v1/log`;
const USAGE_LOG_HOST = new URL(MEDIAPIPE_USAGE_LOG_ORIGIN).hostname;
/** fetch を通らずに送れる API。 */
const OTHER_SENDERS = ["sendBeacon", "XMLHttpRequest", "WebSocket", "EventSource"];

/** 識別子としての `fetch` の出現を、前後の数文字と一緒に返す。 */
function fetchTokens(source: string): { before: string; after: string }[] {
  return [...source.matchAll(/(?<![\w$])fetch(?![\w$])/g)].map((m) => ({
    before: source.slice(Math.max(0, m.index - 2), m.index),
    after: source.slice(m.index + "fetch".length, m.index + "fetch".length + 2),
  }));
}

describe(`@mediapipe/tasks-vision ${version}`, () => {
  it("has the wasm loaders and binaries", () => {
    expect(WASM_FILES.filter((f) => f.endsWith(".js"))).toHaveLength(3);
    expect(WASM_FILES.filter((f) => f.endsWith(".wasm"))).toHaveLength(3);
  });

  describe.each(BUNDLES)("%s", (file) => {
    const source = read(file);

    it("sends usage logs only to the origin usage-logging.ts refuses", () => {
      // 新しい送り先 (URL) が増えたら、それが何を送るかを読んで確かめる
      const urls = [...source.matchAll(/https?:\/\/[^\s"'`\\)]+/g)].map((m) => m[0]);
      expect(urls).toEqual([USAGE_LOG_URL]);
      expect(isMediaPipeUsageLog(USAGE_LOG_URL)).toBe(true);
      expect(source).toContain(`url:"${USAGE_LOG_URL}"`);
    });

    it("sends with the global fetch looked up at each call", () => {
      // 送る係は要求の url をそのまま `fetch(<要求>.url, ...)` に渡す
      expect(source).toMatch(/[^\w$.]fetch\([\w$]+\.url,/);
      for (const { before, after } of fetchTokens(source)) {
        // 直接の呼び出し (`fetch(`) か、文の中の語 (エラーの文の " fetch ")。
        // `x=fetch`・`fetch.bind(...)`・`f(fetch)` のように取っておくと、後から包んでも素通りする
        const isCall = after.startsWith("(") && !before.endsWith(".");
        const isProse = /[a-z] $/.test(before) && /^ [a-z]/.test(after);
        expect({ before, after, ok: isCall || isProse }).toMatchObject({ ok: true });
      }
    });

    it("has no other way to send", () => {
      for (const api of OTHER_SENDERS) expect(source).not.toContain(api);
    });
  });

  describe.each(WASM_FILES)("wasm/%s", (file) => {
    it("does not know the usage log endpoint", () => {
      // 送り先は JS の側にだけある (wasm からは API キーだけを取る)。wasm の側が送るようになったら包みでは止まらないかもしれない
      const contents = readFileSync(join(packageDir, "wasm", file)).toString("latin1");
      expect(contents).not.toContain(USAGE_LOG_HOST);
      if (file.endsWith(".js")) expect(contents).not.toContain("sendBeacon");
    });
  });
});
