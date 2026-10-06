// デザイントークン (tokens.css) の整理の検査 (docs/ui.md#デザイントークン)。
// ダークの 2 つの宣言がそろっていること・宣言したトークンがどこかで使われていること・使っているトークンが宣言されていることを見る。

import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

// happy-dom の下では import.meta.url が file: でないので、dirname を使う
const here = import.meta.dirname;
const root = join(here, "../../..");
const tokensPath = join(here, "tokens.css");
const tokens = readFileSync(tokensPath, "utf8");

/** packages/ と apps/ の画面のソース (Svelte・CSS・TS・HTML)。 */
function sources(): { path: string; text: string }[] {
  const out: { path: string; text: string }[] = [];
  const skip = new Set(["node_modules", "dist", "target", "src-tauri", "gen"]);
  function walk(dir: string) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (skip.has(entry.name) || entry.name.startsWith(".")) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (/\.(svelte|css|ts|html)$/.test(entry.name) && path !== tokensPath) {
        out.push({ path, text: readFileSync(path, "utf8") });
      }
    }
  }
  walk(join(root, "packages"));
  walk(join(root, "apps"));
  return out;
}

/** 宣言の `--name: value;` を名前から値への表にする。 */
function declarations(block: string): Map<string, string> {
  const map = new Map<string, string>();
  for (const m of block.matchAll(/(--ugoe-[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    map.set(m[1] as string, (m[2] as string).replace(/\s+/g, " ").trim());
  }
  return map;
}

/** `selector {` から対応する `}` までの中身。 */
function blockAfter(text: string, start: number): string {
  const open = text.indexOf("{", start);
  let depth = 0;
  for (let i = open; i < text.length; i++) {
    if (text[i] === "{") depth++;
    else if (text[i] === "}" && --depth === 0) return text.slice(open + 1, i);
  }
  throw new Error("閉じていないブロック");
}

describe("tokens.css", () => {
  it("declares the same dark values in the media query and in data-theme=dark", () => {
    const media = blockAfter(tokens, tokens.indexOf("@media (prefers-color-scheme: dark)"));
    const explicit = blockAfter(tokens, tokens.indexOf('[data-theme="dark"] {'));
    const a = declarations(media);
    const b = declarations(explicit);
    expect(a.size).toBeGreaterThan(10);
    expect(Object.fromEntries(a)).toEqual(Object.fromEntries(b));
  });

  it("declares every light color again for dark", () => {
    const light = declarations(blockAfter(tokens, tokens.indexOf(':root,\n[data-theme="light"]')));
    const dark = declarations(blockAfter(tokens, tokens.indexOf('[data-theme="dark"] {')));
    expect([...dark.keys()].sort()).toEqual([...light.keys()].sort());
  });

  const files = sources();
  const declared = new Set(declarations(tokens).keys());

  it("has no unused tokens", () => {
    const used = new Set<string>();
    for (const text of [tokens, ...files.map((f) => f.text)]) {
      for (const m of text.matchAll(/var\((--ugoe-[a-z0-9-]+)/g)) used.add(m[1] as string);
    }
    const unused = [...declared].filter((name) => !used.has(name));
    expect(unused).toEqual([]);
  });

  it("has no references to undeclared tokens", () => {
    const missing: string[] = [];
    for (const { path, text } of files) {
      // 部品が自分の中で宣言する変数 (`--ugoe-notice-color` など) は、その部品の中だけで使う
      const local = new Set(declarations(text).keys());
      for (const m of text.matchAll(/var\((--ugoe-[a-z0-9-]+)/g)) {
        const name = m[1] as string;
        // 名前を組み立てるもの (`var(--ugoe-color-${name})`) は数えない
        if (name.endsWith("-")) continue;
        if (!declared.has(name) && !local.has(name)) missing.push(`${path}: ${name}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("redraws only classes that the components still use in forced colors", () => {
    // 強制色の節 (docs/ui.md#強制色) が指す部品のクラスが、名前を変えて効かなくなっていないか
    const forced = blockAfter(tokens, tokens.indexOf("@media (forced-colors: active)"));
    const used = new Set(
      files.flatMap(({ text }) => [...text.matchAll(/\.(ugoe-[a-z0-9_-]+)/g)].map((m) => m[1])),
    );
    const names = [...new Set([...forced.matchAll(/\.(ugoe-[a-z0-9_-]+)/g)].map((m) => m[1]))];
    expect(names.length).toBeGreaterThan(5);
    expect(names.filter((name) => !used.has(name))).toEqual([]);
  });

  it("leaves the scrollbar gap in the scroll containers of the apps", () => {
    // WebKitGTK のスクロールバーは中身の上に重なる (T0970。docs/ui.md#レイアウト の「送り」)。送る枠の規則は、行末を
    // --ugoe-scrollbar-gap だけ空けるか、空けなくてよい理由 (中の段が空ける・padding が足りる) をスクロールバーの言葉で書く
    const apps = ["editor", "viewer", "studio"].map((app) => join(root, `apps/${app}/src`));
    const missing: string[] = [];
    let checked = 0;
    for (const { path, text } of files) {
      if (!apps.some((dir) => path.startsWith(dir))) continue;
      const css = path.endsWith(".svelte")
        ? [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]).join("\n")
        : path.endsWith(".css")
          ? text
          : "";
      for (const m of css.matchAll(/([^{}]*)\{([^{}]*)\}/g)) {
        const [rule, selector, body] = m as unknown as [string, string, string];
        if (!/overflow(-y)?\s*:\s*(auto|scroll)/.test(body)) continue;
        checked++;
        if (!rule.includes("--ugoe-scrollbar-gap") && !rule.includes("スクロールバー")) {
          missing.push(`${relative(root, path)}: ${selector.trim().split("\n").pop()}`);
        }
      }
    }
    expect(checked).toBeGreaterThan(5);
    expect(missing).toEqual([]);
  });
});
