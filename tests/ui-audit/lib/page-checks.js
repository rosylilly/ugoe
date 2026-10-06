// UI の監査 (`just ui-audit`。tests/ui-audit/README.md) のうち、ページの中で動かす検査。
// import も export も持たない素のスクリプトにし、ページには addInitScript で、Node のテストには import で読む。
// どちらでも `globalThis.__ugoeAudit` に関数を置く。
//
// - コントラスト: 文字を直に持つ要素ごとに、文字の色と、祖先をたどって重ねた地の色 (半透明は合成) を比べる (WCAG 2.x の式)。
//   祖先に画像の地・グラデーションがある、canvas の上に半透明で重なっているもの (ビューポートの上の知らせなど) は、
//   文字を消して撮った画面の画素を地として測る (pixelAudit。T0952)。無効の部品は基準の外 (別に数える)。
// - はみ出し: 文書の横のスクロールと、画面の右端を越える要素 (スクロールする祖先の中は除く)。
// - 重なりと切れ: 別の要素の文字が透けて重なる・`overflow: hidden` で文字が切れる (省略記号を付けたものは除く)。
// - 押せる大きさ: 24×24 CSS px 未満のボタン・リンク・入力 (WCAG 2.5.8。docs/ui.md#そのほか)。
// - エラー: 画面に出ているエラーの文 (role="alert"・.error など)。
// - 動き: prefers-reduced-motion: reduce の下で、長さが 0 でない動き (CSS の animation・transition) が動いていないか。
(() => {
  /** `rgb(1 2 3 / 0.5)`・`rgba(1, 2, 3, 0.5)` を {r, g, b, a} に。読めなければ null。 */
  function parseColor(text) {
    const m = /rgba?\(([^)]+)\)/.exec(text);
    if (!m) return null;
    const p = m[1]
      .split(/[\s,/]+/)
      .filter(Boolean)
      .map((v) => (v.endsWith("%") ? Number.parseFloat(v) / 100 : Number(v)));
    if (p.length < 3 || p.some((v) => Number.isNaN(v))) return null;
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  }

  /** `top` を `under` の上に重ねた色 (source-over)。 */
  function over(top, under) {
    const a = top.a + under.a * (1 - top.a);
    if (a === 0) return { r: 0, g: 0, b: 0, a: 0 };
    const mix = (x, y) => (x * top.a + y * under.a * (1 - top.a)) / a;
    return { r: mix(top.r, under.r), g: mix(top.g, under.g), b: mix(top.b, under.b), a };
  }

  /** 相対輝度 (WCAG 2.x)。 */
  function luminance(c) {
    const f = (v) => {
      const s = v / 255;
      return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b);
  }

  /** コントラスト比 (1〜21)。 */
  function contrastRatio(a, b) {
    const x = luminance(a);
    const y = luminance(b);
    return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
  }

  /** 求める比: 大きい文字 (24px 以上、または 18.66px 以上の太字) は 3、ほかは 4.5。 */
  function requiredRatio(fontSizePx, fontWeight) {
    return fontSizePx >= 24 || (fontWeight >= 700 && fontSizePx >= 18.66) ? 3 : 4.5;
  }

  function visible(el) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const s = getComputedStyle(el);
    return s.visibility !== "hidden" && s.display !== "none" && Number(s.opacity) > 0.05;
  }

  /** 要素の短い説明 (タグ・クラス 2 つ・名前)。Svelte の `s-` のクラスは除く。 */
  function classes(el) {
    return typeof el.className === "string"
      ? el.className
          .split(/\s+/)
          .filter((c) => c && !c.startsWith("s-") && !c.startsWith("svelte-"))
          .slice(0, 2)
          .join(".")
      : "";
  }

  /**
   * 要素の短い説明: `tag.class "名前"`。クラスが無ければ、3 つ上までの祖先のクラスを `tag@.class` の形で付ける
   * (同じ部品の中の行を、報告でまとめられるように)。Svelte の `s-`・`svelte-` のクラスは除く。
   */
  function describe(el) {
    const name = (el.getAttribute("aria-label") ?? el.textContent ?? "")
      .trim()
      .replace(/\s+/g, " ")
      .slice(0, 40);
    let cls = classes(el);
    if (cls) cls = `.${cls}`;
    else {
      for (let n = el.parentElement, i = 0; n && i < 3; n = n.parentElement, i++) {
        const c = classes(n);
        if (c) {
          cls = `@.${c}`;
          break;
        }
      }
    }
    return `${el.tagName.toLowerCase()}${cls} "${name}"`;
  }

  /** 見えている canvas・video・img と、その矩形 (contrastAudit の始めに集める)。 */
  let media = [];

  /**
   * 地の色: 自分から祖先へたどり、不透明な地まで重ねる。測れなければ {unknown}。
   * 不透明な地の祖先の中に、文字に重なる canvas・video・img があれば、その上に描かれているとみなして測れないとする
   * (ビューポートの知らせのように、地の祖先と文字の間に描画が挟まる。T0952)。
   */
  function background(el) {
    const layers = [];
    let opacity = 1;
    let stop = null;
    for (let n = el; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      opacity *= Number(s.opacity);
      if (s.backgroundImage !== "none") {
        return { unknown: `background-image on ${describe(n)}` };
      }
      const c = parseColor(s.backgroundColor);
      if (c && c.a > 0) {
        layers.push(c);
        if (c.a >= 1) {
          stop = n;
          break;
        }
      }
    }
    if (stop !== null && media.length > 0) {
      const r = el.getBoundingClientRect();
      const hit = media.find(
        (m) =>
          stop.contains(m.el) &&
          !el.contains(m.el) &&
          m.rect.left < r.right &&
          r.left < m.rect.right &&
          m.rect.top < r.bottom &&
          r.top < m.rect.bottom,
      );
      if (hit) return { unknown: `over ${describe(hit.el)}` };
    }
    const opaque = layers.length > 0 && layers[layers.length - 1].a >= 1;
    if (!opaque && el.closest("[data-ui-audit-canvas], .viewport, .viewport-area, .stage, main")) {
      // 不透明な地が無いまま描画の上に重なっている (ビューポートの上の HUD など)
      if (document.querySelector("canvas")) return { unknown: "translucent over a canvas" };
    }
    let bg = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg);
    return { bg, opacity };
  }

  /** 要素が直に持つ文字の行ごとの矩形 (はみ出しを切り取る祖先と窓で切り取った、見える部分)。 */
  function textRects(el) {
    const clip = clipBox(el);
    const rects = [];
    for (const node of el.childNodes) {
      if (node.nodeType !== 3 || node.textContent.trim() === "") continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const r of range.getClientRects()) {
        const box = {
          left: Math.max(r.left, clip.left),
          top: Math.max(r.top, clip.top),
          right: Math.min(r.right, clip.right),
          bottom: Math.min(r.bottom, clip.bottom),
        };
        if (box.right - box.left > 1 && box.bottom - box.top > 1) rects.push(box);
      }
    }
    return rects;
  }

  /** 地を CSS から決められず、画素で測る文字 (contrastAudit が集め、pixelAudit が測る)。 */
  let pending = [];

  function contrastAudit() {
    pending = [];
    media = [...document.querySelectorAll("canvas, video, img")]
      .filter(visible)
      .map((m) => ({ el: m, rect: m.getBoundingClientRect() }));
    // モーダルのダイアログの後ろ (幕の下で操作できない) の文字は基準の外 (WCAG 1.4.3 の inactive)
    const modals = [
      ...document.querySelectorAll(
        "dialog:modal, [role=dialog][aria-modal=true], [role=alertdialog][aria-modal=true]",
      ),
    ].filter(visible);
    const behindModal = (el) => modals.length > 0 && !modals.some((m) => m.contains(el));
    const failures = [];
    const disabled = [];
    let checked = 0;
    let unknown = 0;
    for (const el of document.querySelectorAll("body *")) {
      if (["SCRIPT", "STYLE", "CANVAS", "OPTION", "TEMPLATE"].includes(el.tagName)) continue;
      if (el instanceof SVGElement) continue;
      const hasText = [...el.childNodes].some(
        (n) => n.nodeType === 3 && n.textContent.trim() !== "",
      );
      const isInput =
        el.tagName === "INPUT" &&
        !["checkbox", "radio", "range", "color", "hidden"].includes(el.type) &&
        el.value !== "";
      if (!hasText && !isInput) continue;
      if (!visible(el) || behindModal(el)) continue;
      const s = getComputedStyle(el);
      const fg = parseColor(s.color);
      if (!fg) continue;
      const b = background(el);
      if (b.unknown) {
        // 地を CSS から決められない: 文字の矩形を覚えておき、文字を消して撮った画素で測る (pixelAudit)
        const rects = isInput ? [el.getBoundingClientRect()] : textRects(el);
        if (rects.length === 0) {
          unknown++;
          continue;
        }
        let opacity = 1;
        for (let n = el; n; n = n.parentElement) opacity *= Number(getComputedStyle(n).opacity);
        pending.push({
          target: describe(el),
          fg,
          color: s.color,
          opacity,
          size: Number.parseFloat(s.fontSize),
          weight: Number(s.fontWeight),
          disabled: el.closest("[disabled], [aria-disabled=true], [data-disabled]") !== null,
          rects: rects.map((r) => ({ left: r.left, top: r.top, right: r.right, bottom: r.bottom })),
        });
        continue;
      }
      checked++;
      const color = over({ ...fg, a: fg.a * b.opacity }, b.bg);
      const ratio = contrastRatio(color, b.bg);
      const size = Number.parseFloat(s.fontSize);
      const need = requiredRatio(size, Number(s.fontWeight));
      if (ratio + 0.005 >= need) continue;
      const item = `${describe(el)} ${ratio.toFixed(2)} < ${need} (fg ${s.color} on rgb(${Math.round(b.bg.r)}, ${Math.round(b.bg.g)}, ${Math.round(b.bg.b)}), ${size}px)`;
      if (el.closest("[disabled], [aria-disabled=true], [data-disabled]")) disabled.push(item);
      else failures.push(item);
    }
    return { checked, unknown, pending: pending.length, failures, disabled };
  }

  /**
   * 画素で測るコントラスト。`items` は contrastAudit が集めた文字 (矩形は CSS px)、`image` は文字を消して撮った画面
   * ({data: RGBA の列, width, height})、`scale` は CSS px あたりの画素。矩形の中の地の画素ごとに、文字の色を重ねた色との比を求め、
   * 下から 1 割の比 (9 割の地の画素で届く比) を文字の比とする (縁や模様の数画素で落とさないため)。
   */
  function pixelContrast(items, image, scale) {
    const failures = [];
    const disabled = [];
    let checked = 0;
    let unknown = 0;
    for (const item of items) {
      const ratios = [];
      let darkest = null;
      for (const r of item.rects) {
        const x0 = Math.max(0, Math.floor(r.left * scale));
        const y0 = Math.max(0, Math.floor(r.top * scale));
        const x1 = Math.min(image.width, Math.ceil(r.right * scale));
        const y1 = Math.min(image.height, Math.ceil(r.bottom * scale));
        // 大きな矩形でも 1 つあたり 2500 ほどの画素に間引く
        const step = Math.max(1, Math.floor(Math.sqrt(((x1 - x0) * (y1 - y0)) / 2500)));
        for (let y = y0; y < y1; y += step) {
          for (let x = x0; x < x1; x += step) {
            const i = (y * image.width + x) * 4;
            const bg = { r: image.data[i], g: image.data[i + 1], b: image.data[i + 2], a: 1 };
            const color = over({ ...item.fg, a: item.fg.a * item.opacity }, bg);
            const ratio = contrastRatio(color, bg);
            ratios.push(ratio);
            if (darkest === null || ratio < darkest.ratio) darkest = { ratio, bg };
          }
        }
      }
      if (ratios.length === 0) {
        unknown++;
        continue;
      }
      checked++;
      ratios.sort((a, b) => a - b);
      const ratio = ratios[Math.floor(ratios.length * 0.1)];
      const need = requiredRatio(item.size, item.weight);
      if (ratio + 0.005 >= need) continue;
      const bg = darkest.bg;
      const line = `${item.target} ${ratio.toFixed(2)} < ${need} (fg ${item.color} on pixels such as rgb(${bg.r}, ${bg.g}, ${bg.b}), ${item.size}px)`;
      if (item.disabled) disabled.push(line);
      else failures.push(line);
    }
    return { checked, unknown, failures, disabled };
  }

  /** 画素で測る前に、画面のすべての文字を透明にする (`on` が偽なら戻す)。描き終わるまで待つ。 */
  async function hideText(on) {
    const id = "ui-audit-hide-text";
    document.getElementById(id)?.remove();
    if (on) {
      const style = document.createElement("style");
      style.id = id;
      style.textContent =
        "*, *::before, *::after, *::placeholder { color: transparent !important; -webkit-text-fill-color: transparent !important; text-shadow: none !important; caret-color: transparent !important; transition: none !important; }";
      document.head.append(style);
    }
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  }

  /** 文字を消して撮った画面 (PNG の base64) で、contrastAudit が集めた文字を測る。 */
  async function pixelAudit(png) {
    const bytes = Uint8Array.from(atob(png), (c) => c.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const g = canvas.getContext("2d");
    g.drawImage(bitmap, 0, 0);
    const image = g.getImageData(0, 0, bitmap.width, bitmap.height);
    return pixelContrast(pending, image, bitmap.width / innerWidth);
  }

  function overflowAudit() {
    const doc = document.scrollingElement ?? document.documentElement;
    const document_ =
      doc.scrollWidth > doc.clientWidth + 1 ? `${doc.scrollWidth} > ${doc.clientWidth}` : null;
    const beyond = [];
    for (const el of document.querySelectorAll("body *")) {
      if (!visible(el)) continue;
      const r = el.getBoundingClientRect();
      if (!(r.right > innerWidth + 1 && r.left < innerWidth)) continue;
      // 右端で切り取る (スクロールする・隠す) 祖先の中ならよい
      let clipped = false;
      for (let n = el.parentElement; n && n !== document.body; n = n.parentElement) {
        if (
          getComputedStyle(n).overflowX !== "visible" &&
          n.getBoundingClientRect().right <= innerWidth + 1
        ) {
          clipped = true;
          break;
        }
      }
      if (!clipped) beyond.push(`${describe(el)} right=${Math.round(r.right)}`);
    }
    return { document: document_, beyond };
  }

  /** 要素と祖先のうち、はみ出しを切り取る (overflow が visible でない) ものと窓で切り取った、見える範囲。 */
  function clipBox(el) {
    let left = 0;
    let top = 0;
    let right = innerWidth;
    let bottom = innerHeight;
    for (let n = el; n && n !== document.documentElement; n = n.parentElement) {
      const s = getComputedStyle(n);
      const r = n.getBoundingClientRect();
      if (s.overflowX !== "visible") {
        left = Math.max(left, r.left);
        right = Math.min(right, r.right);
      }
      if (s.overflowY !== "visible") {
        top = Math.max(top, r.top);
        bottom = Math.min(bottom, r.bottom);
      }
      if (s.position === "fixed") break;
    }
    return { left, top, right, bottom };
  }

  /** 文字を直に持つ見える要素と、その文字の行ごとの見える矩形 (切り取られた部分を除く)。 */
  function textBoxes() {
    const boxes = [];
    for (const el of document.querySelectorAll("body *")) {
      if (["SCRIPT", "STYLE", "CANVAS", "OPTION", "TEMPLATE"].includes(el.tagName)) continue;
      if (el instanceof SVGElement) continue;
      const nodes = [...el.childNodes].filter(
        (n) => n.nodeType === 3 && n.textContent.trim() !== "",
      );
      if (nodes.length === 0 || !visible(el)) continue;
      const rects = textRects(el);
      if (rects.length) boxes.push({ el, rects });
    }
    return boxes;
  }

  function commonAncestor(a, b) {
    for (let n = a; n; n = n.parentElement) if (n.contains(b)) return n;
    return null;
  }

  /**
   * 文字の重なりと切れ。
   * - 重なり: 別の要素の文字の矩形が 2px を超えて重なり、上に描かれた側から共通の祖先までに不透明な地が無い
   *   (下の文字が透けて見える)。上に不透明な地を敷いた重なり (ダイアログ・メニュー) は、隠しているので数えない。
   * - 切れ: `overflow: hidden` (か `clip`) で、`text-overflow: ellipsis` を付けずに中身が枠を越えている。
   */
  function overlapAudit() {
    const found = [];
    const boxes = textBoxes();
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) {
        const a = boxes[i];
        const b = boxes[j];
        if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
        let hit = null;
        for (const ra of a.rects) {
          for (const rb of b.rects) {
            const w = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left);
            const h = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
            if (w > 2 && h > 2) {
              hit = [
                (Math.max(ra.left, rb.left) + Math.min(ra.right, rb.right)) / 2,
                (Math.max(ra.top, rb.top) + Math.min(ra.bottom, rb.bottom)) / 2,
              ];
              break;
            }
          }
          if (hit) break;
        }
        if (!hit) continue;
        const top = document.elementFromPoint(hit[0], hit[1]);
        if (!top) continue;
        const [upper, lower] = a.el.contains(top)
          ? [a, b]
          : b.el.contains(top)
            ? [b, a]
            : [null, null];
        if (!upper) continue;
        const common = commonAncestor(a.el, b.el);
        let covered = false;
        for (let n = upper.el; n && n !== common; n = n.parentElement) {
          const s = getComputedStyle(n);
          const bg = parseColor(s.backgroundColor);
          if ((bg && bg.a >= 1) || s.backgroundImage !== "none") {
            covered = true;
            break;
          }
        }
        if (!covered) found.push(`${describe(upper.el)} overlaps ${describe(lower.el)}`);
      }
    }
    for (const { el } of boxes) {
      // 見せずに読ませる文字 (.ugoe-visually-hidden。1px の枠) は除く
      if (el.clientWidth <= 1 || el.clientHeight <= 1) continue;
      const s = getComputedStyle(el);
      const hidden = (o) => o === "hidden" || o === "clip";
      if (
        hidden(s.overflowX) &&
        s.textOverflow !== "ellipsis" &&
        el.scrollWidth > el.clientWidth + 1
      ) {
        found.push(
          `${describe(el)} is cut off horizontally (${el.scrollWidth} > ${el.clientWidth})`,
        );
      }
      if (
        hidden(s.overflowY) &&
        s.webkitLineClamp === "none" &&
        el.scrollHeight > el.clientHeight + 1
      ) {
        found.push(
          `${describe(el)} is cut off vertically (${el.scrollHeight} > ${el.clientHeight})`,
        );
      }
    }
    return found;
  }

  const TARGETS =
    "button, a[href], input:not([type=hidden]), select, textarea, [role=button], [role=tab], [role=radio], [role=switch], [role=checkbox], [role=menuitem], [role=menuitemcheckbox], [role=menuitemradio], [role=option], [role=treeitem]";

  function targetAudit() {
    const small = new Set();
    for (const el of document.querySelectorAll(TARGETS)) {
      if (!visible(el)) continue;
      // つまみの大きさは手で見る
      if (el.type === "range" || el.getAttribute("role") === "slider") continue;
      // 部品の押せる範囲: 数値の欄は枠の全体、チェックボックスとスイッチは見えるラベルまで (docs/ui.md#3-つのアプリの見直し-t0907)。
      // 標準のチェックボックス・ラジオは、包む <label> (押すと切り替わる) までを押せる範囲とする
      const label = el.type === "checkbox" || el.type === "radio" ? el.closest("label") : null;
      const area =
        el.closest(".ugoe-number-field__box, .ugoe-checkbox, .ugoe-toggle") ?? label ?? el;
      const r = area.getBoundingClientRect();
      if (r.width < 23.5 || r.height < 23.5)
        small.add(`${describe(el)} ${Math.round(r.width)}x${Math.round(r.height)}`);
    }
    return [...small];
  }

  function motionAudit() {
    if (typeof document.getAnimations !== "function") return [];
    const running = [];
    for (const a of document.getAnimations()) {
      if (a.playState !== "running") continue;
      const timing = a.effect?.getComputedTiming?.() ?? {};
      const duration = typeof timing.duration === "number" ? timing.duration : 0;
      if (duration <= 1) continue;
      const target = a.effect?.target;
      const name = a.animationName ?? a.transitionProperty ?? a.constructor.name;
      running.push(
        `${target ? describe(target) : "(no target)"} ${name} ${Math.round(duration)}ms${timing.iterations === Infinity ? " infinite" : ""}`,
      );
    }
    return running;
  }

  /** 画面に出ているエラーの文 (偽のホストの不足や、見本で出るはずのないエラーに気づくため)。 */
  function shownErrors() {
    const out = [];
    for (const el of document.querySelectorAll('[role=alert], .error, [class*="__error"]')) {
      if (!visible(el) || (el.textContent ?? "").trim() === "") continue;
      if (el.parentElement?.closest('[role=alert], .error, [class*="__error"]')) continue;
      out.push(describe(el));
    }
    return out;
  }

  /** ページの検査の一式。page.evaluate(() => __ugoeAudit.pageAudit()) で呼ぶ。 */
  function pageAudit() {
    return {
      contrast: contrastAudit(),
      overflow: overflowAudit(),
      overlap: overlapAudit(),
      small: targetAudit(),
      motion: motionAudit(),
      errors: shownErrors(),
      lang: document.documentElement.lang,
      theme: document.documentElement.dataset.theme ?? null,
    };
  }

  globalThis.__ugoeAudit = {
    parseColor,
    over,
    luminance,
    contrastRatio,
    requiredRatio,
    pixelContrast,
    pageAudit,
    hideText,
    pixelAudit,
  };
})();
