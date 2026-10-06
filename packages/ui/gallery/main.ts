// 部品の見本の画面。URL の引数でテーマ・密度・言語を決める (スクリーンショットの検査が使う):
// `?theme=light|dark&density=compact|comfortable&lang=en|ja`

import "../src/tokens.css";
import { mount, tick } from "svelte";
import Gallery from "./Gallery.svelte";

const root = document.documentElement;
const fail = (message: string) => {
  root.dataset.galleryError = message;
};
window.addEventListener("error", (e) => fail(String(e.message)));
window.addEventListener("unhandledrejection", (e) => fail(String(e.reason)));

const target = document.getElementById("app");
if (!target) throw new Error("#app が無い");
mount(Gallery, { target });

/** 文字が何行に分かれて描かれたか (行の上端の数)。 */
const lines = (el: Element) => {
  const range = document.createRange();
  range.selectNodeContents(el);
  return new Set([...range.getClientRects()].map((r) => Math.round(r.top))).size;
};

/** 描いた形の問題をスクリーンショットの検査に知らせる (DOM だけでは分からないもの)。 */
const checkLayout = () => {
  const problems: string[] = [];
  // SegmentedControl の名前と選択肢は字の途中で折らない (T0950)
  for (const el of document.querySelectorAll(".ugoe-segmented__label, .ugoe-segmented__item")) {
    if (lines(el) > 1) problems.push(`折れた: ${el.textContent?.trim()}`);
  }
  // 狭い所の SegmentedControl は、はみ出さずに選択肢ごと次の行へ送る
  const narrow = document.querySelector(".gallery__narrow");
  const group = narrow?.querySelector(".ugoe-segmented__group");
  if (narrow && group) {
    if (group.getBoundingClientRect().right > narrow.getBoundingClientRect().right) {
      problems.push("狭い所の SegmentedControl がはみ出した");
    }
    const tops = new Set(
      [...group.children].map((item) => Math.round(item.getBoundingClientRect().top)),
    );
    if (tops.size < 2) problems.push("狭い所の SegmentedControl が次の行へ送られていない");
  }
  if (problems.length > 0) root.dataset.galleryLayout = problems.join("; ");
};

// 描き終えたことをスクリーンショットの検査に知らせる
void tick()
  .then(() => document.fonts.ready)
  .then(() =>
    requestAnimationFrame(() => {
      checkLayout();
      root.dataset.galleryReady = "true";
    }),
  );
