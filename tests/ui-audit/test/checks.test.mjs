// UI の監査の計算と、まとめ・比べ方のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { axeFindings, isNameRule, pageFindings } from "../lib/audit.mjs";
import { compare, countsTable, formatCompare, kindOf, summarize } from "../lib/report.mjs";
import { formatWalk, walkProblems } from "../lib/walk.mjs";
import "../lib/page-checks.js";

const { parseColor, over, contrastRatio, requiredRatio, pixelContrast } = globalThis.__ugoeAudit;

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

/** 1 色で塗った画像 ({data, width, height})。`paint(x, y)` が色を返せば、その画素だけ塗り替える。 */
function image(width, height, color, paint = () => null) {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const c = paint(x, y) ?? color;
      data.set([c.r, c.g, c.b, 255], (y * width + x) * 4);
    }
  }
  return { data, width, height };
}

test("canvas の上の文字は、文字を消して撮った画素を地として測る", () => {
  // T0725 の前のビューアの知らせ: ダークの text-muted (#a1a1aa) が、ビューポートの地 (モデルの背景色 #ffffff) の上にある
  const muted = {
    target: 'p.hint "No model"',
    fg: hex("#a1a1aa"),
    color: "rgb(161, 161, 170)",
    opacity: 1,
    size: 13,
    weight: 400,
    disabled: false,
    rects: [{ left: 2, top: 2, right: 8, bottom: 6 }],
  };
  const white = image(10, 10, hex("#ffffff"));
  const r = pixelContrast([muted], white, 1);
  assert.equal(r.checked, 1);
  assert.equal(r.failures.length, 1);
  assert.match(
    r.failures[0],
    /^p\.hint "No model" 2\.\d\d < 4\.5 \(fg rgb\(161, 161, 170\) on pixels such as rgb\(255, 255, 255\)/,
  );
  // 暗い地なら届く。DPR 2 では矩形を 2 倍にして画素を引く
  const dark = image(20, 20, hex("#18181b"), (x) => (x >= 16 ? hex("#ffffff") : null));
  assert.deepEqual(pixelContrast([muted], dark, 2).failures, []);
  // 無効の部品は分け、矩形が画像の外なら測れない
  const off = { ...muted, rects: [{ left: 40, top: 40, right: 50, bottom: 50 }] };
  const r2 = pixelContrast([{ ...muted, disabled: true }, off], white, 1);
  assert.equal(r2.disabled.length, 1);
  assert.equal(r2.unknown, 1);
});

test("地の画素の 1 割までは届かなくてよい (縁や模様)", () => {
  const text = {
    target: 'span "x"',
    fg: hex("#ffffff"),
    color: "rgb(255, 255, 255)",
    opacity: 1,
    size: 13,
    weight: 400,
    disabled: false,
    rects: [{ left: 0, top: 0, right: 10, bottom: 10 }],
  };
  // 10 列のうち 1 列が白い (地の 1 割)
  const edge = image(10, 10, hex("#000000"), (x) => (x === 0 ? hex("#ffffff") : null));
  assert.deepEqual(pixelContrast([text], edge, 1).failures, []);
  // 2 列 (2 割) なら届かない
  const wider = image(10, 10, hex("#000000"), (x) => (x <= 1 ? hex("#ffffff") : null));
  assert.equal(pixelContrast([text], wider, 1).failures.length, 1);
  // 半透明の文字 (opacity) は地に重ねてから比べる
  const faint = { ...text, opacity: 0.3 };
  assert.equal(pixelContrast([faint], image(10, 10, hex("#000000")), 1).failures.length, 1);
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
