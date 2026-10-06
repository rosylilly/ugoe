// 一覧 (Menu・ContextMenu・Select) が縦に送るときだけ、行末をスクロールバーの分空ける (docs/ui.md#レイアウト の「送り」。T0976)。
//
// WebKitGTK のスクロールバーは幅を取らずに中身の上に重ねて描くので、送る枠は行末を `--ugoe-scrollbar-gap` 空ける (T0970)。
// 一覧は低い窓でしか送らず、いつも空けるとすべての一覧が広がるので、送るときだけ `data-scrolling` を付けて CSS で空ける。
// 空けると中身は狭くなって高くなるだけなので、付けた印で送らなくなることはない (付け外しが行き来しない)。

import type { Attachment } from "svelte/attachments";

/** 縦に送るか (中身が枠より高いか)。端数の丸めで 1px 違うだけのときは送らないとみなす。 */
export function scrollsVertically(el: HTMLElement): boolean {
  return el.scrollHeight - el.clientHeight > 1;
}

/** 送るかどうかを `data-scrolling` に写す。枠と中の項目の大きさ・項目の増減が変わるたびに測り直す。 */
export const markScrolling: Attachment<HTMLElement> = (el) => {
  const update = () => el.toggleAttribute("data-scrolling", scrollsVertically(el));
  const resize = new ResizeObserver(update);
  const observeChildren = () => {
    resize.disconnect();
    resize.observe(el);
    for (const child of el.children) resize.observe(child);
    update();
  };
  const mutation = new MutationObserver(observeChildren);
  mutation.observe(el, { childList: true });
  observeChildren();
  return () => {
    resize.disconnect();
    mutation.disconnect();
  };
};
