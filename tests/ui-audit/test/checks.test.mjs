// UI の監査の計算と、まとめ・比べ方のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  AXE_STRUCTURE_OPTIONS,
  axeFindings,
  isNameRule,
  pageFindings,
  STRUCTURE_RULES,
} from "../lib/audit.mjs";
import { CHECKS, compare, countsTable, formatCompare, kindOf, summarize } from "../lib/report.mjs";
import { focusRegion, formatWalk, imageRing, walkProblems } from "../lib/walk.mjs";
import "../lib/page-checks.js";

const {
  parseColor,
  over,
  contrastRatio,
  requiredRatio,
  pixelContrast,
  focusChange,
  coveredControls,
  underScrollbar,
} = globalThis.__ugoeAudit;

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
      overflow: {
        document: "400 > 360",
        beyond: [],
        escaped: ['span.ugoe-visually-hidden "Warning:" escapes aside.panels "Scene"'],
      },
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
      ["overflow", 'span.ugoe-visually-hidden "Warning:"'],
      ["overlap", 'span.a "b"'],
      ["target-size", 'button.key "◆"'],
      ["error", 'p.studio__error "boom"'],
      ["aria", "html"],
    ],
  );
  // 失敗の出し方を撮る場面では、画面のエラーの文を数えない
  const expected = pageFindings(
    {
      contrast: { checked: 0, unknown: 0, failures: [], disabled: [] },
      overflow: { document: null, beyond: [] },
      overlap: [],
      small: [],
      motion: [],
      errors: ['p.studio__error "boom"'],
      lang: "en",
    },
    "en",
    { expectErrors: true },
  );
  assert.deepEqual(expected, []);
  assert.equal(found[0].detail, "3.20 < 4.5 (fg ...)");
});

test("開いたままのダイアログ・メニューは、開いて撮る場面のほかでは前の場面の閉じ忘れとして数える", () => {
  const empty = {
    contrast: { checked: 0, unknown: 0, failures: [], disabled: [] },
    overflow: { document: null, beyond: [] },
    overlap: [],
    small: [],
    motion: [],
    errors: [],
    lang: "en",
  };
  // T1655: 外での変更のダイアログ (-external-change) が次の場面に残った形
  const left = { ...empty, overlays: ['div.ugoe-dialog "The project changed outside"'] };
  assert.deepEqual(pageFindings(left, "en"), [
    {
      check: "error",
      target: 'div.ugoe-dialog "The project changed outside"',
      detail: "left open by an earlier scene",
    },
  ]);
  // ダイアログを開いて撮る場面 (`ctx.scene(name, { overlay: true })`) では数えない
  assert.deepEqual(pageFindings(left, "en", { expectOverlay: true }), []);
  assert.deepEqual(pageFindings({ ...empty, overlays: [] }, "en"), []);
});

/**
 * 押せる部品の重なりを判定する。`boxes` は {el: {name, parent?, overlay?}, rect}。上に描かれるのは後に並べた方 (流れの中の
 * 部品の描く順) で、`overlay` の付いた要素 (意図して重ねた印) を上に重ねた組は数えない。
 */
function covered(boxes) {
  const inside = (a, b) => {
    for (let n = b; n; n = n.parent) if (n === a) return true;
    return false;
  };
  return coveredControls(boxes, {
    related: (a, b) => inside(a, b) || inside(b, a),
    upperAt: (a, b) => (boxes.indexOf(a) < boxes.indexOf(b) ? b : a),
    intentional: (upper) => upper.overlay === true,
  }).map(({ upper, lower }) => `${lower.el.name} < ${upper.el.name}`);
}

const box = (el, left, top, right, bottom) => ({ el, rect: { left, top, right, bottom } });

test("押せる部品の重なり: 選択欄の右端 (矢印) が隣の数値の欄の下に隠れる形を見つける", () => {
  // T1650 の前のグルーの軸を足す行 (*-zoom200): 縮んだ枠から Select (最小 8em) がはみ出し、右の数値の欄の下に入る
  const select = { name: "select" };
  const number = { name: "number" };
  assert.deepEqual(covered([box(select, 0, 0, 128, 28), box(number, 100, 0, 180, 28)]), [
    "select < number",
  ]);
  // 直した後: 数値の欄は次の行へ送られ、重ならない
  assert.deepEqual(covered([box(select, 0, 0, 128, 28), box(number, 0, 36, 80, 64)]), []);
});

test("押せる部品の重なり: 枠の線を重ねた並び・入れ子・意図して重ねたものは数えない", () => {
  // つないだボタンの列 (枠の線を 1〜2px 重ねる)
  assert.deepEqual(
    covered([box({ name: "a" }, 0, 0, 40, 28), box({ name: "b" }, 38, 0, 80, 28)]),
    [],
  );
  // 行の中のボタン (ツリーの行の目のボタンなど)
  const row = { name: "row" };
  assert.deepEqual(
    covered([box(row, 0, 0, 200, 28), box({ name: "eye", parent: row }, 170, 2, 196, 26)]),
    [],
  );
  // ボタンに重ねたバッジ・ビューポートの上の道具 (重ねた側に印がある)
  const badge = { name: "badge", overlay: true };
  assert.deepEqual(covered([box({ name: "tab" }, 0, 0, 80, 28), box(badge, 70, -4, 90, 12)]), []);
  // 印のある部品の下に、印の無い部品が重なったのではなく、印の無い部品が上に来たら数える
  assert.deepEqual(covered([box(badge, 70, -4, 90, 12), box({ name: "tab" }, 0, 0, 80, 28)]), [
    "badge < tab",
  ]);
  // 上に描かれたのがどちらでもない (ダイアログの幕が両方を覆う) なら数えない
  const cover = coveredControls(
    [box({ name: "a" }, 0, 0, 50, 28), box({ name: "b" }, 20, 0, 70, 28)],
    {
      related: () => false,
      upperAt: () => null,
      intentional: () => false,
    },
  );
  assert.deepEqual(cover, []);
});

/**
 * 重ねて描くスクロールバーの下の部品を判定する。`frames` は {el, ...大きさ}、`items` は {el: {name, frame}, right}
 * (frame は部品を包む枠の el)。戻り値は `部品 < 枠 (空き)` の列。
 */
function underBar(frames, items, gap = 16) {
  return underScrollbar(frames, items, {
    gap,
    inside: (frame, el) => el.frames.includes(frame),
  }).map(({ item, frame, room }) => `${item.el.name} < ${frame.el.name} (${room})`);
}

/** 縦に 200px 送れる枠 (高さ 300・中身 500)。見える右端は 300。 */
const frame = (name, more = {}) => ({
  el: { name },
  overflowX: "hidden",
  overflowY: "auto",
  scrollWidth: 300,
  clientWidth: 300,
  scrollHeight: 500,
  clientHeight: 300,
  right: 300,
  ...more,
});

test("重ねて描くスクロールバー: 行末を空けない枠の中の文字・部品を見つける", () => {
  // T0970 の前のビューアのパラメータの一覧 (.list。行末の padding が 0): グループの数が枠の右端に掛かる
  const list = frame("list");
  const count = { name: "count", frames: [list.el] };
  const label = { name: "label", frames: [list.el] };
  assert.deepEqual(
    underBar(
      [list],
      [
        { el: count, right: 300 },
        { el: label, right: 120 },
      ],
    ),
    ["count < list (0)"],
  );
  // T0970 の前のスタジオの右の列 (.studio__panels。8px): 空きが 16px に足りない
  const panels = frame("panels");
  assert.deepEqual(
    underBar([panels], [{ el: { name: "button", frames: [panels.el] }, right: 292 }]),
    ["button < panels (8)"],
  );
});

test("重ねて描くスクロールバー: 空けた枠・送らない枠・横に送る枠は数えない", () => {
  // 行末を --ugoe-scrollbar-gap (16px) 空けた枠 (端数の丸めの 0.5px は許す)
  const gapped = frame("gapped");
  assert.deepEqual(
    underBar(
      [gapped],
      [
        { el: { name: "a", frames: [gapped.el] }, right: 284 },
        { el: { name: "b", frames: [gapped.el] }, right: 284.4 },
      ],
    ),
    [],
  );
  // overflow-y: auto でも、中身が枠に収まっていて今は送れない枠 (スクロールバーが出ない)
  const still = frame("still", { scrollHeight: 300 });
  assert.deepEqual(underBar([still], [{ el: { name: "a", frames: [still.el] }, right: 300 }]), []);
  // 送らない枠 (overflow-y: hidden)
  const hidden = frame("hidden", { overflowY: "hidden" });
  assert.deepEqual(
    underBar([hidden], [{ el: { name: "a", frames: [hidden.el] }, right: 300 }]),
    [],
  );
  // 横に送る枠 (縦は送らない): 中身は右の端の先へ続く
  const wide = frame("wide", {
    overflowX: "auto",
    overflowY: "hidden",
    scrollWidth: 900,
    scrollHeight: 300,
  });
  assert.deepEqual(underBar([wide], [{ el: { name: "a", frames: [wide.el] }, right: 300 }]), []);
  // 縦と横の両方に今送れる枠 (タイムラインの表など)
  const both = frame("both", { overflowX: "auto", scrollWidth: 900 });
  assert.deepEqual(underBar([both], [{ el: { name: "a", frames: [both.el] }, right: 300 }]), []);
  // 枠の外の部品
  const other = frame("other");
  assert.deepEqual(underBar([other], [{ el: { name: "a", frames: [] }, right: 300 }]), []);
});

test("重ねて描くスクロールバー: 入れ子の枠では、部品ごとにいちばん内側の枠で 1 つ数える", () => {
  // 画面の全体の送り (狭い幅) の中の、行末を空けないパネルの送る枠
  const page = frame("page", { right: 360 });
  const panel = frame("panel", { right: 350 });
  const item = { name: "a", frames: [page.el, panel.el] };
  assert.deepEqual(underBar([page, panel], [{ el: item, right: 345 }]), ["a < panel (5)"]);
  // 内側の枠が空けていれば、外の枠の端から遠い部品は数えない
  assert.deepEqual(underBar([page, panel], [{ el: item, right: 330 }]), []);
});

test("重ねて描くスクロールバーの下の部品は scrollbar として数え、報告の見出しがある", () => {
  const found = pageFindings(
    {
      contrast: { checked: 0, unknown: 0, failures: [], disabled: [] },
      overflow: { document: null, beyond: [] },
      overlap: [],
      scrollbar: ['span.count "3" is under the overlay scrollbar of div.list (0.0px < 16px)'],
      small: [],
      motion: [],
      errors: [],
      lang: "en",
    },
    "en",
  );
  assert.deepEqual(
    found.map((f) => [f.check, f.target, f.detail]),
    [["scrollbar", 'span.count "3"', "is under the overlay scrollbar of div.list (0.0px < 16px)"]],
  );
  assert.ok(Object.fromEntries(CHECKS).scrollbar);
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

test("ランドマークと見出しの規則は structure に分け、名指しで流す (axe では best-practice の札)", () => {
  assert.deepEqual(AXE_STRUCTURE_OPTIONS.runOnly, { type: "rule", values: STRUCTURE_RULES });
  for (const rule of ["landmark-one-main", "heading-order", "page-has-heading-one", "region"])
    assert.ok(STRUCTURE_RULES.includes(rule), rule);
  const found = axeFindings([
    { id: "heading-order", help: "order", nodes: [{ target: ["#h4"], html: "<h4>" }] },
    { id: "landmark-unique", help: "unique", nodes: [{ target: ["aside"], html: "<aside>" }] },
  ]);
  assert.deepEqual(
    found.map((f) => [f.check, f.target]),
    [
      ["structure", "#h4"],
      ["structure", "aside"],
    ],
  );
});

test("強制色で見分けられない状態は forced として数え、報告の見出しがある", () => {
  const found = pageFindings(
    {
      contrast: { checked: 0, unknown: 0, failures: [], disabled: [] },
      overflow: { document: null, beyond: [] },
      overlap: [],
      small: [],
      motion: [],
      forced: ['span.ugoe-toggle__thumb "" looks the same as its off state in forced colors'],
      errors: [],
      lang: "en",
    },
    "en",
  );
  assert.deepEqual(
    found.map((f) => [f.check, f.target, f.detail]),
    [["forced", 'span.ugoe-toggle__thumb ""', "looks the same as its off state in forced colors"]],
  );
  const titles = Object.fromEntries(CHECKS);
  assert.ok(titles.forced);
  assert.ok(titles.structure);
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

test("Tab の止まりのうち、印・名前が無いもの・画面の外のものを拾う (印は画像の判定、測れなければ CSS の判定)", () => {
  const ring = { changed: 200, strong: 200, maxRatio: 4.8, mark: "ring", lost: false };
  const stops = [
    { tag: "button", role: null, name: "Open", ring: true, image: ring, offscreen: false },
    { body: true },
    { tag: "div", role: "separator", name: "", ring: false, image: null, offscreen: true },
    // CSS は輪を見つけたが、画像は変わらない (常に影を持つ祖先の中など)
    {
      tag: "input",
      role: null,
      name: "Speed",
      ring: true,
      image: { changed: 4, strong: 0, maxRatio: 1.2, mark: "none", lost: false },
      offscreen: false,
    },
    // 地の色だけを変える印: CSS は見つけないが画像は変わる。3:1 に届かない
    {
      tag: "button",
      role: "tab",
      name: "Physics",
      ring: false,
      image: { changed: 900, strong: 0, maxRatio: 1.31, mark: "faint", lost: false },
      offscreen: false,
    },
    { tag: "a", role: null, name: "Docs", ring: false, image: ring, offscreen: false },
    {
      tag: "button",
      role: null,
      name: "Open",
      ring: true,
      image: null,
      offscreen: false,
      wrapped: true,
    },
  ];
  assert.deepEqual(walkProblems(stops), [
    { target: 'div[separator] ""', detail: "no focus ring, no name, off screen (stop 3)" },
    { target: 'input "Speed"', detail: "no focus ring (stop 4)" },
    {
      target: 'button[tab] "Physics"',
      detail: "faint focus ring (1.31:1 at most, needs 3:1) (stop 5)",
    },
  ]);
  assert.equal(
    formatWalk(stops),
    [
      '1. button "Open"',
      "2. (body)",
      '3. div[separator] "" NO-RING NO-NAME OFFSCREEN',
      '4. input "Speed" NO-RING (css: ring)',
      '5. button[tab] "Physics" FAINT-RING',
      '6. a "Docs" (css: none)',
      '7. button "Open" (wrapped)',
    ].join("\n"),
  );
});

test("フォーカスの前と後の画像の差: 変わった画素と、3:1 以上変わった画素を数える", () => {
  const white = hex("#ffffff");
  const before = image(10, 10, white);
  // 2px の濃い輪 (外周の 2 画素) と、中の薄い地の変化 (#f4f4f5)
  const after = image(10, 10, white, (x, y) => {
    if (x < 2 || y < 2 || x >= 8 || y >= 8) return hex("#2563eb");
    if (x < 4) return hex("#f4f4f5");
    return null;
  });
  const r = focusChange(before, after);
  assert.equal(r.strong, 64);
  // #f4f4f5 は白との差が 11 で、描き方の揺れとして数えない
  assert.equal(r.changed, 64);
  assert.equal(r.maxRatio.toFixed(2), "5.17");
  const tinted = image(10, 10, white, (x) => (x < 5 ? hex("#e4e4e7") : null));
  assert.deepEqual(focusChange(before, tinted), {
    changed: 50,
    strong: 0,
    maxRatio: contrastRatio(white, hex("#e4e4e7")),
  });
});

test("画像の差から印を決める: 幅と高さの和の面積が要る", () => {
  const rect = { width: 80, height: 24 };
  assert.equal(imageRing({ changed: 400, strong: 400 }, rect), "ring");
  assert.equal(imageRing({ changed: 400, strong: 60 }, rect), "faint");
  // カーソルの 1 本ほど
  assert.equal(imageRing({ changed: 20, strong: 20 }, rect), "none");
  // 止まりの周りは窓で切る
  assert.deepEqual(
    focusRegion({ x: 2, y: 100, width: 80, height: 24 }, { width: 1440, height: 900 }),
    { x: 0, y: 94, width: 88, height: 36 },
  );
  assert.equal(
    focusRegion({ x: 0, y: 950, width: 80, height: 24 }, { width: 1440, height: 900 }),
    null,
  );
});
