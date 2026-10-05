// UI の監査の計算と、まとめ・比べ方のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { axeFindings, isNameRule, pageFindings } from "../lib/audit.mjs";
import { compare, countsTable, formatCompare, kindOf, summarize } from "../lib/report.mjs";
import { formatWalk, walkProblems } from "../lib/walk.mjs";
import "../lib/page-checks.js";

const { parseColor, over, contrastRatio, requiredRatio } = globalThis.__ugoeAudit;

const hex = (h) => ({
  r: Number.parseInt(h.slice(1, 3), 16),
  g: Number.parseInt(h.slice(3, 5), 16),
  b: Number.parseInt(h.slice(5, 7), 16),
  a: 1,
});

test("色の文字列を読む (rgb・rgba・CSS Color 4 の空白区切りと割合)", () => {
  assert.deepEqual(parseColor("rgb(1, 2, 3)"), { r: 1, g: 2, b: 3, a: 1 });
  assert.deepEqual(parseColor("rgba(1, 2, 3, 0.5)"), { r: 1, g: 2, b: 3, a: 0.5 });
  assert.deepEqual(parseColor("rgb(1 2 3 / 40%)"), { r: 1, g: 2, b: 3, a: 0.4 });
  assert.equal(parseColor("transparent"), null);
  assert.equal(parseColor("color(srgb 1 0 0)"), null);
});

test("コントラスト比は WCAG 2.x の式 (docs/ui.md の色の表と同じ値になる)", () => {
  assert.equal(contrastRatio(hex("#000000"), hex("#ffffff")).toFixed(1), "21.0");
  assert.equal(contrastRatio(hex("#ffffff"), hex("#ffffff")), 1);
  // 表の値: text-muted (ライト) は surface に 6.3、accent (ダーク) は bg に 6.5
  assert.equal(contrastRatio(hex("#5f5f68"), hex("#ffffff")).toFixed(1), "6.3");
  assert.equal(contrastRatio(hex("#a78bfa"), hex("#18181b")).toFixed(1), "6.5");
});

test("半透明の色は下の色に重ねて合成する", () => {
  const mixed = over({ r: 0, g: 0, b: 0, a: 0.5 }, { r: 255, g: 255, b: 255, a: 1 });
  assert.deepEqual(mixed, { r: 127.5, g: 127.5, b: 127.5, a: 1 });
  assert.deepEqual(over({ r: 9, g: 9, b: 9, a: 0 }, { r: 0, g: 0, b: 0, a: 0 }), {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
  });
});

test("大きい文字 (24px 以上、18.66px 以上の太字) は 3、ほかは 4.5", () => {
  assert.equal(requiredRatio(13, 400), 4.5);
  assert.equal(requiredRatio(24, 400), 3);
  assert.equal(requiredRatio(19, 700), 3);
  assert.equal(requiredRatio(19, 600), 4.5);
});

test("ページの検査の結果を検査ごとの見つけたものにする", () => {
  const found = pageFindings(
    {
      contrast: {
        checked: 3,
        unknown: 0,
        failures: ['span.hint "x" 3.20 < 4.5 (fg ...)'],
        disabled: [],
      },
      overflow: { document: "400 > 360", beyond: [] },
      overlap: ['span.a "b" overlaps span.c "d"'],
      small: ['button.key "◆" 10x10'],
      motion: [],
      errors: ['p.studio__error "boom"'],
      lang: "en",
    },
    "ja",
  );
  assert.deepEqual(
    found.map((f) => [f.check, f.target]),
    [
      ["contrast", 'span.hint "x"'],
      ["overflow", "document"],
      ["overlap", 'span.a "b"'],
      ["target-size", 'button.key "◆"'],
      ["error", 'p.studio__error "boom"'],
      ["aria", "html"],
    ],
  );
  assert.equal(found[0].detail, "3.20 < 4.5 (fg ...)");
});

test("axe の違反は名前の規則と、ほかの ARIA に分ける", () => {
  assert.ok(isNameRule("button-name"));
  assert.ok(isNameRule("label"));
  assert.ok(!isNameRule("nested-interactive"));
  const found = axeFindings([
    {
      id: "button-name",
      help: "Buttons must have discernible text",
      nodes: [{ target: [".icon"], html: "<button class=icon></button>" }],
    },
    { id: "aria-required-children", help: "children", nodes: [{ target: [".tree"], html: "" }] },
  ]);
  assert.deepEqual(
    found.map((f) => [f.check, f.target]),
    [
      ["name", ".icon"],
      ["aria", ".tree"],
    ],
  );
});

test("要素の種類: クラスがあれば tag.class (祖先のクラスなら tag@.class)、無ければ全体", () => {
  assert.equal(kindOf('button.mesh-button "650 HairFront"'), "button.mesh-button");
  assert.equal(kindOf('input@.meshes "Lit: Face"'), "input@.meshes");
  assert.equal(kindOf('div[separator] "Resize"'), 'div[separator] "Resize"');
  assert.equal(kindOf('button "Settings"'), 'button "Settings"');
  assert.equal(kindOf(".tree"), ".tree");
});

test("同じ種類のものを場面と言語をまたいでまとめ、種類と延べを数える", () => {
  const s = summarize([
    {
      name: "en-light-wide",
      findings: [{ check: "target-size", target: 'button.key "A"', detail: "10x10" }],
    },
    {
      name: "ja-light-wide",
      findings: [
        { check: "target-size", target: 'button.key "あ"', detail: "10x10" },
        { check: "contrast", target: 'span "x"', detail: "3.0 < 4.5" },
      ],
    },
  ]);
  assert.equal(s.scenes, 2);
  assert.deepEqual(s.counts["target-size"], { unique: 1, total: 2 });
  assert.deepEqual(s.counts.contrast, { unique: 1, total: 1 });
  // 並びは検査の順 (コントラストが先)
  assert.equal(s.findings[0].check, "contrast");
  assert.deepEqual(s.findings[1].targets, ['button.key "A"', 'button.key "あ"']);
  assert.deepEqual(s.findings[1].scenes, ["en-light-wide", "ja-light-wide"]);
  const table = countsTable({ editor: s });
  assert.match(table, /\| 24px 未満の押せるもの \| 1 \(2\) \|/);
  assert.match(table, /\| 横のはみ出し \| 0 \|/);
});

test("前の結果と比べて、増えた・消えた・数が変わった種類を出す", () => {
  const before = summarize([
    {
      name: "a",
      findings: [
        { check: "contrast", target: 'span.old "x"', detail: "" },
        { check: "target-size", target: 'button.key "A"', detail: "" },
      ],
    },
  ]);
  const after = summarize([
    {
      name: "a",
      findings: [
        { check: "target-size", target: 'button.key "A"', detail: "" },
        { check: "target-size", target: 'button.key "B"', detail: "" },
        { check: "name", target: ".icon", detail: "button-name" },
      ],
    },
  ]);
  const diff = compare({ editor: before, viewer: before }, { editor: after });
  assert.deepEqual(Object.keys(diff), ["editor"]);
  assert.deepEqual(
    diff.editor.added.map((f) => f.kind),
    [".icon"],
  );
  assert.deepEqual(
    diff.editor.removed.map((f) => f.kind),
    ["span.old"],
  );
  assert.deepEqual(
    diff.editor.changed.map((c) => [c.finding.kind, c.before, c.finding.targets.length]),
    [["button.key", 1, 2]],
  );
  assert.match(formatCompare(diff), /^editor: \+1 -1 ~1/);
});

test("Tab の止まりのうち、印・名前が無いもの・画面の外のものを拾う", () => {
  const stops = [
    { tag: "button", role: null, name: "Open", ring: true, offscreen: false },
    { body: true },
    { tag: "div", role: "separator", name: "", ring: false, offscreen: true },
    { tag: "button", role: null, name: "Open", ring: true, offscreen: false, wrapped: true },
  ];
  assert.deepEqual(walkProblems(stops), [
    { target: 'div[separator] ""', detail: "no focus ring, no name, off screen (stop 3)" },
  ]);
  assert.equal(
    formatWalk(stops),
    [
      '1. button "Open"',
      "2. (body)",
      '3. div[separator] "" NO-RING NO-NAME OFFSCREEN',
      '4. button "Open" (wrapped)',
    ].join("\n"),
  );
});
