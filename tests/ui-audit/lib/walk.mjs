// Tab でフォーカスをたどり、止まった所の名前・フォーカスの印・画面の外かを記録する (tests/ui-audit/README.md)。
// 印は、止まりの周りをフォーカスの前と後で撮った画像の差で決める (T0045)。CSS の outline・box-shadow による判定は比べるために残す。
import { writeFileSync } from "node:fs";
import { decodePng } from "./png.mjs";
import "./page-checks.js";

/** 止まりの周りに含める幅 (CSS px)。outline の幅と outline-offset (2px と 2px ほど) が収まり、隣の部品をあまり含まない。 */
export const FOCUS_MARGIN = 6;

/** ページの中で今のフォーカスを調べる (page.evaluate に渡す)。 */
function focusInfo() {
  const el = document.activeElement;
  if (!el || el === document.body || el === document.documentElement) return { body: true };
  const hasRing = (n) => {
    const s = getComputedStyle(n);
    return (
      (s.outlineStyle !== "none" && Number.parseFloat(s.outlineWidth) > 0) ||
      (s.boxShadow !== "none" && s.boxShadow !== "")
    );
  };
  // 輪を外側の枠に出す部品 (NumberField の focus-within) があるので、2 つ上の祖先までを見る
  const ring =
    hasRing(el) || [el.parentElement, el.parentElement?.parentElement].some((n) => n && hasRing(n));
  const labelledby = el.getAttribute("aria-labelledby");
  const fromIds = labelledby
    ? labelledby
        .split(/\s+/)
        .map((id) => document.getElementById(id)?.textContent ?? "")
        .join(" ")
        .trim()
    : "";
  const name = (
    el.getAttribute("aria-label") ||
    fromIds ||
    el.labels?.[0]?.textContent ||
    (el.tagName === "INPUT" ? "" : el.textContent) ||
    el.getAttribute("title") ||
    el.getAttribute("placeholder") ||
    ""
  )
    .trim()
    .replace(/\s+/g, " ")
    .slice(0, 50);
  const r = el.getBoundingClientRect();
  // 前と後を撮る間、同じ要素を指すために覚えておく
  globalThis.__uiAuditFocused = el;
  const path = [];
  for (let n = el; n && n !== document.body && path.length < 6; n = n.parentElement) {
    path.unshift(n.tagName.toLowerCase());
  }
  return {
    tag: el.tagName.toLowerCase(),
    role: el.getAttribute("role"),
    name,
    ring,
    rect: { x: r.x, y: r.y, width: r.width, height: r.height },
    viewport: { width: innerWidth, height: innerHeight },
    offscreen: r.bottom < 0 || r.top > innerHeight || r.right < 0 || r.left > innerWidth,
    key: `${path.join(">")}|${el.id}|${Math.round(r.x)},${Math.round(r.y)}`,
  };
}

/** 止まりの周りの撮る範囲 (CSS px。窓で切る)。大きさが無ければ null。 */
export function focusRegion(rect, viewport, margin = FOCUS_MARGIN) {
  const x0 = Math.max(0, Math.floor(rect.x - margin));
  const y0 = Math.max(0, Math.floor(rect.y - margin));
  const x1 = Math.min(viewport.width, Math.ceil(rect.x + rect.width + margin));
  const y1 = Math.min(viewport.height, Math.ceil(rect.y + rect.height + margin));
  if (x1 - x0 < 1 || y1 - y0 < 1) return null;
  return { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

/**
 * フォーカスの前と後に撮った PNG (Buffer) を Node の側で読んで比べる (page-checks.js の focusChange)。数は CSS px² に直す
 * (`scale` は devicePixelRatio)。ページの中で読むと、止まりごとに base64 で送る分も含めて 40ms ほどかかるため (T0054)。
 */
export function focusDiff(beforePng, afterPng, scale) {
  const r = globalThis.__ugoeAudit.focusChange(decodePng(beforePng), decodePng(afterPng));
  const area = scale * scale;
  return { changed: r.changed / area, strong: r.strong / area, maxRatio: r.maxRatio };
}

/**
 * 画像の差 (focusDiff。CSS px²) から印を決める。要るのは要素の幅と高さの和の面積 (1 CSS px の線で周りの半分を
 * なぞる量。2px の輪なら周りの 4 分の 1 で足りる): 前と後のコントラスト比が 3:1 以上の画素がそれだけあれば `ring`、
 * 3:1 に届かない変化だけがそれだけあれば `faint` (見えるが WCAG 1.4.11 の 3:1 に届かない)、どちらでもなければ `none`。
 * `rect` は窓の中に見える範囲の大きさ。
 */
export function imageRing(diff, rect) {
  const need = Math.max(1, rect.width + rect.height);
  if (diff.strong >= need) return "ring";
  if (diff.changed >= need) return "faint";
  return "none";
}

/**
 * たどる間に隠すもの: 欄のカーソル (点滅する)・文字の選択の地 (Tab で入った欄の文字は選ばれる)・ツールチップ (フォーカスで開くが、
 * 止まりの印ではない)。たどる間だけ置く (カーソルは screenshot の既定の `caret: "hide"` でも隠れるが、撮るたびに足し外す
 * 代わりにここでまとめて隠す)。
 */
const SHOT_STYLE =
  '* { caret-color: transparent !important; } ::selection { background: transparent !important; } [role="tooltip"] { visibility: hidden !important; }';

/**
 * フォーカスのある今 (後) と、外した後 (前) の止まりの周りを撮って比べる
 * (screenshot は新しく描いた画面を撮るので、フォーカスを外した後に描き終わるのを待たない)。
 * 撮った後はフォーカスを戻す (次の Tab が続きから進むように)。戻せなければ `lost`。
 */
async function measureFocus(page, info) {
  const clip = focusRegion(info.rect, info.viewport);
  if (!clip) return null;
  const after = await page.screenshot({ clip, caret: "initial" });
  await page.evaluate(() => globalThis.__uiAuditFocused?.blur());
  const before = await page.screenshot({ clip, caret: "initial" });
  const { lost, scale } = await page.evaluate(() => {
    const el = globalThis.__uiAuditFocused;
    el?.focus({ preventScroll: true });
    return { lost: document.activeElement !== el, scale: devicePixelRatio };
  });
  const diff = focusDiff(before, after, scale);
  const visible = {
    width: Math.max(
      0,
      Math.min(info.rect.x + info.rect.width, info.viewport.width) - Math.max(info.rect.x, 0),
    ),
    height: Math.max(
      0,
      Math.min(info.rect.y + info.rect.height, info.viewport.height) - Math.max(info.rect.y, 0),
    ),
  };
  return { ...diff, mark: imageRing(diff, visible), lost, shots: { before, after } };
}

/**
 * Tab を最大 `max` 回押してフォーカスをたどる。最初の止まりに戻ったら (一周したら) やめる。
 * 戻り値は止まりの列 ({tag, role, name, ring, image, offscreen, wrapped?} か {body: true})。`ring` は CSS の判定
 * (自分と 2 つ上の祖先の outline・box-shadow)、`image` は画像の差 (measureFocus。画面の外なら null)。
 * `shots` を渡すと、印が `ring` でない止まりと CSS の判定と食い違う止まりの前と後の画像を
 * `<shots>-<止まりの番号>-before.png`・`-after.png` に書く (人が確かめるため)。
 */
export async function tabWalk(page, max = 80, { shots = null } = {}) {
  // 文書の頭から始める: body にフォーカスを置くと、次の Tab は最初のフォーカスできる要素へ進む
  // (blur だけでは、前にフォーカスしていた要素の位置から続く)
  await page.evaluate(() => {
    const body = document.body;
    const had = body.hasAttribute("tabindex");
    body.tabIndex = -1;
    body.focus();
    if (!had) body.removeAttribute("tabindex");
  });
  await page.evaluate((css) => {
    const style = document.createElement("style");
    style.id = "ui-audit-focus-shot";
    style.textContent = css;
    document.head.append(style);
  }, SHOT_STYLE);
  const stops = [];
  const seen = new Set();
  for (let i = 0; i < max; i++) {
    await page.keyboard.press("Tab");
    await page.waitForTimeout(30);
    const info = await page.evaluate(focusInfo);
    if (info.body) {
      stops.push({ body: true });
      continue;
    }
    if (seen.has(info.key)) {
      stops.push({ ...stopOf(info, null), wrapped: true });
      break;
    }
    seen.add(info.key);
    const measured = info.offscreen ? null : await measureFocus(page, info);
    const { shots: pngs, ...image } = measured ?? {};
    const stop = stopOf(info, measured ? image : null);
    if (shots && measured && (image.mark !== "ring" || !stop.ring)) {
      writeFileSync(`${shots}-${stops.length + 1}-before.png`, pngs.before);
      writeFileSync(`${shots}-${stops.length + 1}-after.png`, pngs.after);
    }
    stops.push(stop);
  }
  await page.evaluate(() => document.getElementById("ui-audit-focus-shot")?.remove());
  return stops;
}

function stopOf(info, image) {
  const { rect: _rect, viewport: _viewport, ...rest } = info;
  return { ...rest, image };
}

/** 止まりの印: 画像で測れたら画像の判定、測れなければ (画面の外) CSS の判定。 */
export function markOf(s) {
  if (s.image) return s.image.mark;
  return s.ring ? "ring" : "none";
}

/** 止まりのうち問題のあるもの (印が無い・名前が無い・画面の外) を {target, detail} の列に。 */
export function walkProblems(stops) {
  const problems = [];
  stops.forEach((s, i) => {
    if (s.body || s.wrapped) return;
    const flags = [];
    const mark = markOf(s);
    if (mark === "none") flags.push("no focus ring");
    if (mark === "faint")
      flags.push(`faint focus ring (${s.image.maxRatio.toFixed(2)}:1 at most, needs 3:1)`);
    if (s.name === "") flags.push("no name");
    if (s.offscreen) flags.push("off screen");
    if (flags.length)
      problems.push({ target: label(s), detail: `${flags.join(", ")} (stop ${i + 1})` });
  });
  return problems;
}

function label(s) {
  return `${s.tag}${s.role ? `[${s.role}]` : ""} "${s.name}"`;
}

/** 止まりの列を 1 行ずつの文に。 */
export function formatWalk(stops) {
  return stops
    .map((s, i) => {
      if (s.body) return `${i + 1}. (body)`;
      const mark = markOf(s);
      const flags = [
        mark === "none" ? " NO-RING" : mark === "faint" ? " FAINT-RING" : "",
        // 画像と CSS の判定が食い違う止まり (CSS は印を見つけたが画像は変わらない・その逆)
        s.image && (mark === "ring") !== s.ring ? ` (css: ${s.ring ? "ring" : "none"})` : "",
        s.image?.lost ? " FOCUS-LOST" : "",
        s.name === "" ? " NO-NAME" : "",
        s.offscreen ? " OFFSCREEN" : "",
        s.wrapped ? " (wrapped)" : "",
      ].join("");
      return `${i + 1}. ${label(s)}${flags}`;
    })
    .join("\n");
}
