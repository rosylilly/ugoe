// 1 つの場面 (開いた画面の状態) の検査: ページの中の検査 (page-checks.js) と axe-core を流し、見つけたものを
// {check, target, detail} の列にする。check の名前と意味は tests/ui-audit/README.md#検査。
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

/** ページの中の検査のスクリプト (addInitScript で入れる)。 */
export const PAGE_CHECKS = readFileSync(new URL("./page-checks.js", import.meta.url), "utf8");

let axeSource = null;
/** axe-core の本体 (addScriptTag で入れる)。 */
export function axeScript() {
  axeSource ??= readFileSync(require.resolve("axe-core/axe.min.js"), "utf8");
  return axeSource;
}

/**
 * axe-core の規則のうち、読み上げの名前を確かめるもの (報告の「名前」)。ほかの違反は「aria」にまとめる。
 * コントラストと押せる大きさは自前の検査 (page-checks.js) で見るので、axe では切る (下の AXE_OPTIONS)。
 */
export function isNameRule(id) {
  return (
    id.endsWith("-name") ||
    ["label", "image-alt", "svg-img-alt", "role-img-alt", "frame-title"].includes(id)
  );
}

/** axe.run に渡す設定: WCAG 2.0〜2.2 の A・AA の規則 (best-practice は含めない)。 */
export const AXE_OPTIONS = {
  runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
  rules: { "color-contrast": { enabled: false }, "target-size": { enabled: false } },
  resultTypes: ["violations"],
};

/**
 * 画面の骨組み (ランドマークと見出しの階層) を見る axe-core の規則 (報告の「structure」)。axe では best-practice の札なので、
 * 上の WCAG の札の組とは別に、規則を名指しして流す (T0918)。
 */
export const STRUCTURE_RULES = [
  "landmark-one-main",
  "landmark-no-duplicate-main",
  "landmark-no-duplicate-banner",
  "landmark-no-duplicate-contentinfo",
  "landmark-main-is-top-level",
  "landmark-banner-is-top-level",
  "landmark-complementary-is-top-level",
  "landmark-contentinfo-is-top-level",
  "landmark-unique",
  "page-has-heading-one",
  "heading-order",
  "empty-heading",
  "region",
];

/**
 * 骨組みの規則で見ない所: 重ねて出すもの (メニュー・Popover・ツールチップ・ダイアログ・Toast)。
 * 2 つのランドマークの間の大きさを変える境 (`role="separator"`。どちらの領域にも属さず、`aria-controls` で動かす領域を指す) は、
 * それだけを包む要素も含めて `region` の違反から外す (`onlySeparators`)。
 */
export const STRUCTURE_EXCLUDE = [
  ["[data-bits-floating-content-wrapper]"],
  ["[popover]"],
  ["dialog"],
  ['[role="dialog"]'],
  ['[role="alertdialog"]'],
];

/** 骨組みの規則を流す axe.run の設定。 */
export const AXE_STRUCTURE_OPTIONS = {
  runOnly: { type: "rule", values: STRUCTURE_RULES },
  resultTypes: ["violations"],
};

/** axe の違反を {check, target, detail} の列に。 */
export function axeFindings(violations) {
  const out = [];
  for (const v of violations) {
    for (const node of v.nodes) {
      out.push({
        check: STRUCTURE_RULES.includes(v.id) ? "structure" : isNameRule(v.id) ? "name" : "aria",
        target: `${node.target.join(" ")}`,
        detail: `${v.id}: ${v.help}${node.html ? ` — ${node.html.replace(/\s+/g, " ").slice(0, 120)}` : ""}`,
      });
    }
  }
  return out;
}

/**
 * page-checks.js の結果を {check, target, detail} の列に。`lang` は期待する言語 (en・ja)。
 * `expectErrors` なら画面に出たエラーの文を数えない (失敗の出し方を撮る場面。スクリプトのエラーは run.mjs が別に数える)。
 * `expectOverlay` でなければ、開いているダイアログ・メニュー・Popover を前の場面の閉じ忘れとして数える (T0053)。
 */
export function pageFindings(r, lang, { expectErrors = false, expectOverlay = false } = {}) {
  const out = [];
  const split = (item) => {
    // `tag.class "名前" 残り` の形 (page-checks.js の describe)
    const m = /^(\S+ "[^"]*")\s*(.*)$/.exec(item);
    return m ? { target: m[1], detail: m[2] } : { target: item, detail: "" };
  };
  for (const item of r.contrast.failures) out.push({ check: "contrast", ...split(item) });
  if (r.overflow.document)
    out.push({
      check: "overflow",
      target: "document",
      detail: `scrollWidth ${r.overflow.document}`,
    });
  for (const item of r.overflow.beyond) out.push({ check: "overflow", ...split(item) });
  for (const item of r.overflow.escaped ?? []) out.push({ check: "overflow", ...split(item) });
  for (const item of r.overlap) out.push({ check: "overlap", ...split(item) });
  for (const item of r.scrollbar ?? []) out.push({ check: "scrollbar", ...split(item) });
  for (const item of r.small) out.push({ check: "target-size", ...split(item) });
  for (const item of r.motion) out.push({ check: "motion", ...split(item) });
  for (const item of r.forced ?? []) out.push({ check: "forced", ...split(item) });
  for (const item of expectErrors ? [] : r.errors)
    out.push({ check: "error", target: split(item).target, detail: "shown on the screen" });
  for (const item of expectOverlay ? [] : (r.overlays ?? []))
    out.push({
      check: "error",
      target: split(item).target,
      detail: "left open by an earlier scene",
    });
  if (lang && !r.lang?.startsWith(lang)) {
    out.push({ check: "aria", target: "html", detail: `lang is "${r.lang}" (expected ${lang})` });
  }
  return out;
}

/** 場面を検査する。axe は `axe` が真のときだけ (重いので Tab などの補助の場面では切れる)。 */
export async function auditScene(
  page,
  { lang, axe = true, expectErrors = false, expectOverlay = false },
) {
  const r = await page.evaluate(() => globalThis.__ugoeAudit.pageAudit());
  let pixels = { checked: 0, unknown: 0, failures: [], disabled: [] };
  // 強制色では文字を透明にできない (透明も OS の文字の色に置き換わる) ので、画素では測らない (測れない数に入れる)
  const forced = await page.evaluate(() => matchMedia("(forced-colors: active)").matches);
  if (forced) pixels.unknown = r.contrast.pending;
  else if (r.contrast.pending > 0) {
    // canvas・画像の上の文字: 文字を消して撮り、その画素を地として測る
    await page.evaluate(() => globalThis.__ugoeAudit.hideText(true));
    const png = await page.screenshot();
    await page.evaluate(() => globalThis.__ugoeAudit.hideText(false));
    pixels = await page.evaluate(
      (b64) => globalThis.__ugoeAudit.pixelAudit(b64),
      png.toString("base64"),
    );
    r.contrast.failures.push(...pixels.failures);
    r.contrast.disabled.push(...pixels.disabled);
  }
  const findings = pageFindings(r, lang, { expectErrors, expectOverlay });
  if (axe) {
    const loaded = await page.evaluate(() => typeof globalThis.axe !== "undefined");
    if (!loaded) await page.addScriptTag({ content: axeScript() });
    const result = await page.evaluate(
      (options) => globalThis.axe.run(document, options),
      AXE_OPTIONS,
    );
    findings.push(...axeFindings(result.violations));
    // メニュー・Popover・ダイアログは開いた元の外 (body の直下) に描くので、骨組みの規則からは外す
    const structure = await page.evaluate(
      ({ exclude, options }) => globalThis.axe.run({ include: [["body"]], exclude }, options),
      { exclude: STRUCTURE_EXCLUDE, options: AXE_STRUCTURE_OPTIONS },
    );
    for (const v of structure.violations) {
      if (v.id !== "region") continue;
      const targets = v.nodes.map((n) => n.target[0]);
      const keep = await page.evaluate(
        (list) =>
          list.map((selector) => {
            const el = document.querySelector(selector);
            // 中身が境 (separator) だけの要素
            const onlySeparators =
              el !== null &&
              (el.getAttribute("role") === "separator" ||
                ((el.textContent ?? "").trim() === "" &&
                  el.querySelector('[role="separator"]') !== null &&
                  el.querySelector(
                    'button, a[href], input, select, textarea, [tabindex]:not([role="separator"])',
                  ) === null));
            return !onlySeparators;
          }),
        targets,
      );
      v.nodes = v.nodes.filter((_, i) => keep[i]);
    }
    findings.push(...axeFindings(structure.violations));
  }
  return {
    findings,
    stats: {
      textChecked: r.contrast.checked,
      textByPixels: pixels.checked,
      textUnmeasured: r.contrast.unknown + pixels.unknown,
      disabledLowContrast: r.contrast.disabled.length,
    },
  };
}
