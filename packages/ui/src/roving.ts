// ツールバーのタブ止まりを 1 つに保つ (Toolbar.svelte の補い。T0907)。

/** 押せる項目のうち、ちょうど 1 つを tabindex="0" にし、ほかを -1 にする。
 * 選ぶのは、フォーカスを含む項目 → 最後にフォーカスした項目 → すでに 0 の項目 → 先頭の順。値が変わるときだけ書く。 */
export function keepOneTabStop(root: HTMLElement, last: HTMLElement | null): void {
  const items = [...root.querySelectorAll<HTMLElement>("[data-toolbar-item]")];
  const enabled = items.filter(
    (el) => !el.hasAttribute("data-disabled") && !el.hasAttribute("disabled"),
  );
  if (enabled.length === 0) return;
  const active = document.activeElement;
  const keep =
    enabled.find((el) => el === active || el.contains(active)) ??
    (last !== null && enabled.includes(last) ? last : undefined) ??
    enabled.find((el) => el.tabIndex === 0) ??
    enabled[0];
  for (const el of items) {
    const want = el === keep ? 0 : -1;
    if (el.tabIndex !== want || !el.hasAttribute("tabindex")) el.tabIndex = want;
  }
}
