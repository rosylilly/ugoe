<!--
  Toolbar: ボタンの並び (docs/ui.md#部品の一覧)。土台は Bits UI の Toolbar
  (全体で 1 つのタブ止まり。中は矢印で移る)。中には Button・IconButton を置く。

  Bits UI 2.19.4 の roving は、描いた時点の先頭の押せる項目にだけ tabindex="0" を付け、項目の無効が後から変わっても
  付け直さない。そのため、描いた時点ですべて無効 (スタジオでホストを待つ間など) だと、押せるようになっても Tab で届かない。
  ここで「押せる項目のうち、ちょうど 1 つが tabindex="0"」を保つ (T0907)。
-->
<script lang="ts">
import { Toolbar } from "bits-ui";
import type { Snippet } from "svelte";
import { setInToolbar } from "../context";
import { keepOneTabStop } from "../roving";

interface Props {
  /** 読み上げの名前 (翻訳を通した文言)。 */
  label: string;
  orientation?: "horizontal" | "vertical";
  children: Snippet;
}

let { label, orientation = "horizontal", children }: Props = $props();

setInToolbar();

let root = $state<HTMLElement | null>(null);

$effect(() => {
  const el = root;
  if (!el) return;
  let last: HTMLElement | null = null;
  const fix = () => keepOneTabStop(el, last);
  const onFocusIn = (e: FocusEvent) => {
    const item = (e.target as HTMLElement | null)?.closest<HTMLElement>("[data-toolbar-item]");
    if (item && el.contains(item)) {
      last = item;
      fix();
    }
  };
  fix();
  const observer = new MutationObserver(fix);
  observer.observe(el, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["data-disabled", "disabled", "tabindex"],
  });
  el.addEventListener("focusin", onFocusIn);
  return () => {
    observer.disconnect();
    el.removeEventListener("focusin", onFocusIn);
  };
});
</script>

<Toolbar.Root bind:ref={root} class="ugoe-toolbar" {orientation} aria-label={label}>
  {@render children()}
</Toolbar.Root>

<style>
  :global {
    .ugoe-toolbar {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-1);
      padding: var(--ugoe-space-1);
    }

    .ugoe-toolbar[data-orientation="vertical"] {
      flex-direction: column;
    }
  }
</style>
