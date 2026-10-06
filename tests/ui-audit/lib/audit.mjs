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

/** axe の違反を {check, target, detail} の列に。 */
export function axeFindings(violations) {
  const out = [];
  for (const v of violations) {
    for (const node of v.nodes) {
      out.push({
        check: isNameRule(v.id) ? "name" : "aria",
        target: `${node.target.join(" ")}`,
        detail: `${v.id}: ${v.help}${node.html ? ` — ${node.html.replace(/\s+/g, " ").slice(0, 120)}` : ""}`,
      });
    }
  }
  return out;
}

/** page-checks.js の結果を {check, target, detail} の列に。`lang` は期待する言語 (en・ja)。 */
export function pageFindings(r, lang) {
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
  for (const item of r.overlap) out.push({ check: "overlap", ...split(item) });
  for (const item of r.small) out.push({ check: "target-size", ...split(item) });
  for (const item of r.motion) out.push({ check: "motion", ...split(item) });
  for (const item of r.errors)
    out.push({ check: "error", target: split(item).target, detail: "shown on the screen" });
  if (lang && !r.lang?.startsWith(lang)) {
    out.push({ check: "aria", target: "html", detail: `lang is "${r.lang}" (expected ${lang})` });
  }
  return out;
}

/** 場面を検査する。axe は `axe` が真のときだけ (重いので Tab などの補助の場面では切れる)。 */
export async function auditScene(page, { lang, axe = true }) {
  const r = await page.evaluate(() => globalThis.__ugoeAudit.pageAudit());
  let pixels = { checked: 0, unknown: 0, failures: [], disabled: [] };
  if (r.contrast.pending > 0) {
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
  const findings = pageFindings(r, lang);
  if (axe) {
    const loaded = await page.evaluate(() => typeof globalThis.axe !== "undefined");
    if (!loaded) await page.addScriptTag({ content: axeScript() });
    const result = await page.evaluate(
      (options) => globalThis.axe.run(document, options),
      AXE_OPTIONS,
    );
    findings.push(...axeFindings(result.violations));
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
