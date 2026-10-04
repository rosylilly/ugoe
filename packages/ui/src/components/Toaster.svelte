<!--
  Toaster: Toast (すぐ消えてよい知らせ) を描く場所 (docs/ui.md#部品の一覧)。自作。アプリの根元に 1 つ置き、
  知らせは `showToast()` で出す (src/toast.svelte.ts)。
  - `popover="manual"` で最上位の層 (top layer) に出す。読み上げの領域を先に置いておくため、知らせが無くても
    開いておき (中身が無ければ見えず、クリックも通す)、知らせが来るたびに出し直して、後から開いた <dialog> より上にする。
  - 全体が `role="status"` の領域で、フォーカスを奪わない。ポインタかフォーカスが乗っている間は消えるまでの時間を止める。
  - 操作を含む知らせは Toast にしない (Notice にする)。閉じるボタンだけを持つ。
  - エラー (`danger`) の文は、写して報告に貼れるよう選択できる (`.ugoe-selectable`)。
-->
<script lang="ts">
import { t } from "../i18n";
import { dismissToast, pauseToasts, resumeToasts, toasts } from "../toast.svelte";
import Icon from "./Icon.svelte";
import IconButton from "./IconButton.svelte";

let region = $state<HTMLElement>();
const list = $derived(toasts());
let lastCount = -1;
// ポインタとフォーカスのどちらかが乗っている間は止める (片方が離れただけでは進めない)
let hovered = false;
let focused = false;

function update() {
  if (hovered || focused) pauseToasts();
  else resumeToasts();
}

$effect(() => {
  const count = list.length;
  const el = region;
  if (!el) return;
  // 最初と、知らせが増えたときに出し直して、いちばん上の層に置く (happy-dom などで popover が無ければ何もしない)
  if (count > lastCount && typeof el.showPopover === "function") {
    if (el.matches(":popover-open")) el.hidePopover();
    el.showPopover();
  }
  // 閉じるボタンで最後の知らせを消すと pointerleave・focusout が来ないので、ここで時間を進め直す
  if (count === 0) {
    hovered = false;
    focused = false;
    resumeToasts();
  }
  lastCount = count;
});
</script>

<div
  bind:this={region}
  class="ugoe-toaster"
  popover="manual"
  role="status"
  aria-label={t("toast.region.aria")}
  onpointerenter={() => {
    hovered = true;
    update();
  }}
  onpointerleave={() => {
    hovered = false;
    update();
  }}
  onfocusin={() => {
    focused = true;
    update();
  }}
  onfocusout={(e) => {
    if (region?.contains(e.relatedTarget as Node | null)) return;
    focused = false;
    update();
  }}
>
  {#each list as item (item.id)}
    <div class="ugoe-toast" data-tone={item.tone}>
      <span class="ugoe-toast__icon"><Icon name={item.tone} /></span>
      <p class="ugoe-toast__message" class:ugoe-selectable={item.tone === "danger"}>
        <span class="ugoe-visually-hidden">{t(`notice.tone.${item.tone}`)}</span>
        {item.message}
      </p>
      <IconButton label={t("common.close")} tooltip={false} small onclick={() => dismissToast(item.id)}>
        <Icon name="close" />
      </IconButton>
    </div>
  {/each}
</div>

<style>
  :global {
    .ugoe-toaster {
      position: fixed;
      inset: auto var(--ugoe-space-4) var(--ugoe-space-4) auto;
      z-index: var(--ugoe-z-toast);
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      gap: var(--ugoe-space-2);
      width: min(360px, calc(100vw - 2 * var(--ugoe-space-4)));
      margin: 0;
      padding: 0;
      overflow: visible;
      border: 0;
      background: transparent;
      color: var(--ugoe-color-text);
      /* 知らせの無いところはクリックを下へ通す */
      pointer-events: none;
    }

    .ugoe-toast {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      box-sizing: border-box;
      width: 100%;
      padding: var(--ugoe-space-2) var(--ugoe-space-2) var(--ugoe-space-2) var(--ugoe-space-3);
      border: 1px solid var(--ugoe-color-border);
      border-left: 4px solid var(--ugoe-toast-color);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
      pointer-events: auto;
      animation: ugoe-fade-in var(--ugoe-duration-normal) ease-out;
    }

    .ugoe-toast[data-tone="info"] {
      --ugoe-toast-color: var(--ugoe-color-accent);
    }

    .ugoe-toast[data-tone="success"] {
      --ugoe-toast-color: var(--ugoe-color-success);
    }

    .ugoe-toast[data-tone="warning"] {
      --ugoe-toast-color: var(--ugoe-color-warning);
    }

    .ugoe-toast[data-tone="danger"] {
      --ugoe-toast-color: var(--ugoe-color-danger);
    }

    .ugoe-toast__icon {
      display: inline-flex;
      color: var(--ugoe-toast-color);
    }

    .ugoe-toast__message {
      flex: 1;
      min-width: 0;
      margin: 0;
      overflow-wrap: anywhere;
    }
  }
</style>
