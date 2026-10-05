// Tab でフォーカスをたどり、止まった所の名前・フォーカスの印・画面の外かを記録する (tests/ui-audit/README.md)。

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
  const path = [];
  for (let n = el; n && n !== document.body && path.length < 6; n = n.parentElement) {
    path.unshift(n.tagName.toLowerCase());
  }
  return {
    tag: el.tagName.toLowerCase(),
    role: el.getAttribute("role"),
    name,
    ring,
    offscreen: r.bottom < 0 || r.top > innerHeight || r.right < 0 || r.left > innerWidth,
    key: `${path.join(">")}|${el.id}|${Math.round(r.x)},${Math.round(r.y)}`,
  };
}

/**
 * Tab を最大 `max` 回押してフォーカスをたどる。最初の止まりに戻ったら (一周したら) やめる。
 * 戻り値は止まりの列 ({tag, role, name, ring, offscreen, wrapped?} か {body: true})。
 */
export async function tabWalk(page, max = 80) {
  // 文書の頭から始める: body にフォーカスを置くと、次の Tab は最初のフォーカスできる要素へ進む
  // (blur だけでは、前にフォーカスしていた要素の位置から続く)
  await page.evaluate(() => {
    const body = document.body;
    const had = body.hasAttribute("tabindex");
    body.tabIndex = -1;
    body.focus();
    if (!had) body.removeAttribute("tabindex");
  });
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
      stops.push({ ...info, wrapped: true });
      break;
    }
    seen.add(info.key);
    stops.push(info);
  }
  return stops;
}

/** 止まりのうち問題のあるもの (印が無い・名前が無い・画面の外) を {target, detail} の列に。 */
export function walkProblems(stops) {
  const problems = [];
  stops.forEach((s, i) => {
    if (s.body || s.wrapped) return;
    const flags = [];
    if (!s.ring) flags.push("no focus ring");
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
      const flags = [
        s.ring ? "" : " NO-RING",
        s.name === "" ? " NO-NAME" : "",
        s.offscreen ? " OFFSCREEN" : "",
        s.wrapped ? " (wrapped)" : "",
      ].join("");
      return `${i + 1}. ${label(s)}${flags}`;
    })
    .join("\n");
}
