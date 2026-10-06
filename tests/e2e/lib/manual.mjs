// マニュアルの画像 (docs/manual/images) を撮る道具 (`just e2e --manual`。tests/e2e/README.md#マニュアルの画像)。
// 筋書きは specs/<アプリ>.manual.mjs。撮った画像は `E2E_OUT/manual/<名前>.png` に置き、`--update-images` のときに
// run.mjs が色を減らして docs/manual/images へ写す。

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { out } from "./harness.mjs";
import { waitFor } from "./webdriver.mjs";

/**
 * ページの表示域 (`innerWidth` × `innerHeight`) を `width` × `height` にする。Xvfb にはウィンドウマネージャが無く枠も無いが、
 * GTK の窓とページの差 (あれば) を測って足す。
 */
export async function resizeTo(session, width, height) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const inner = await session.execute(() => [window.innerWidth, window.innerHeight]);
    if (inner[0] === width && inner[1] === height) return;
    const rect = await session.call("GET", "/window/rect");
    await session.setWindowRect({
      width: rect.width + width - inner[0],
      height: rect.height + height - inner[1],
    });
    await waitFor(
      async () => {
        const now = await session.execute(() => [window.innerWidth, window.innerHeight]);
        return (
          now[0] !== inner[0] || now[1] !== inner[1] || (now[0] === width && now[1] === height)
        );
      },
      { timeout: 5_000, message: `the window to become ${width}x${height}` },
    ).catch(() => {});
  }
  const inner = await session.execute(() => [window.innerWidth, window.innerHeight]);
  if (inner[0] !== width || inner[1] !== height) {
    throw new Error(`the page is ${inner[0]}x${inner[1]}, not ${width}x${height}`);
  }
  // 窓を小さくすると、ページの全体が少し送られたまま残ることがある (ヘッダーが上に欠ける)。頭に戻す
  await session.execute(() => {
    document.scrollingElement.scrollTop = 0;
  });
}

/**
 * 要素 (CSS のセレクタか `Element`) の画面の中の四角 (CSS px。devicePixelRatio は 1)。`pad` (数か、CSS と同じ順の
 * `[上, 右, 下, 左]`) だけ広げ、高さを `maxHeight` までにし、表示域で切る。セレクタに合うものが無ければ `null`。
 */
function clipRect(session, target, pad, maxHeight) {
  const [top, right, bottom, left] = typeof pad === "number" ? [pad, pad, pad, pad] : pad;
  return session.execute(
    (target, top, right, bottom, left, maxHeight) => {
      const el = typeof target === "string" ? document.querySelector(target) : target;
      if (!el) return null;
      const r = el.getBoundingClientRect();
      const x = Math.max(0, Math.floor(r.left - left));
      const y = Math.max(0, Math.floor(r.top - top));
      const width = Math.min(window.innerWidth, Math.ceil(r.right + right)) - x;
      const height = Math.min(window.innerHeight, Math.ceil(r.bottom + bottom), y + maxHeight) - y;
      return { x, y, width, height };
    },
    target,
    top,
    right,
    bottom,
    left,
    maxHeight ?? Number.MAX_SAFE_INTEGER,
  );
}

/**
 * マニュアルの画像を 1 枚撮る (`E2E_OUT/manual/<name>.png`)。`clip` を渡すと、その要素 (CSS のセレクタか `Element`) か
 * 四角 (`{ x, y, width, height }`) の範囲だけを残す (ウィンドウの全体を撮ってから ImageMagick で切る)。要素なら `pad` だけ広げ、
 * 高さを `maxHeight` までにする (長いパネルの頭だけを撮る)。動きの途中を撮らないように、撮る前に 2 フレーム待つ。
 * @param {{ clip?: string | import("./webdriver.mjs").Element | { x: number, y: number, width: number, height: number }, pad?: number | number[], maxHeight?: number }} options
 */
export async function manualShot(session, name, { clip, pad = 0, maxHeight } = {}) {
  await session.run(
    () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(null)))),
  );
  const dir = join(out(), "manual");
  mkdirSync(dir, { recursive: true });
  const path = join(dir, `${name}.png`);
  const png = await session.screenshot();
  if (clip === undefined) {
    writeFileSync(path, png);
    return path;
  }
  const rect =
    typeof clip === "object" && "width" in clip
      ? clip
      : await clipRect(session, clip, pad, maxHeight);
  if (!rect) throw new Error(`nothing to clip for ${name}`);
  const crop = spawnSync(
    "convert",
    [
      "png:-",
      "-crop",
      `${rect.width}x${rect.height}+${rect.x}+${rect.y}`,
      "+repage",
      `png:${path}`,
    ],
    { input: png },
  );
  if (crop.status !== 0) {
    throw new Error(`could not crop ${name} (ImageMagick): ${crop.stderr}`);
  }
  return path;
}

/** 画面の言語を `locale` にして読み込み直す (`key` はアプリの言語の設定の localStorage の名前)。 */
export async function useLocale(session, key, locale) {
  const current = await session.execute((key) => localStorage.getItem(key), key);
  if (current === locale) return;
  await session.execute((key, locale) => localStorage.setItem(key, locale), key, locale);
  await session.call("POST", "/refresh", {});
}
