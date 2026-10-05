<!--
  Icon: 線のアイコン (docs/ui.md#アイコン)。16 の格子で描き、飾りなので aria-hidden にする
  (意味は IconButton の label や並べた文字で伝える)。
  見た目は tokens.css の `.ugoe-icon` (大きさ・線の太さ)。再生・停止・キーの印は塗る。
  3 つのアプリで同じ意味の印を同じ形にするため、アプリはボタンの中の印に文字の記号 (×・▲・▶) を使わずこれを使う。
-->
<script lang="ts" module>
/** アイコンの名前。足すときは docs/ui.md の「アイコン」の一覧も直す。 */
export type IconName =
  | "close"
  | "info"
  | "success"
  | "warning"
  | "danger"
  | "check"
  | "chevronRight"
  | "chevronDown"
  | "upload"
  | "arrowUp"
  | "arrowDown"
  | "reload"
  | "play"
  | "stop"
  | "stepBack"
  | "stepForward"
  | "key"
  | "plus"
  | "minus"
  | "fit"
  | "settings";

const FILLED = new Set<IconName>(["play", "stop", "stepBack", "stepForward", "key"]);
</script>

<script lang="ts">
let { name }: { name: IconName } = $props();
</script>

<svg class={["ugoe-icon", FILLED.has(name) && "ugoe-icon--fill"]} viewBox="0 0 16 16" aria-hidden="true">
  {#if name === "close"}
    <path d="M4 4l8 8M12 4l-8 8" />
  {:else if name === "info"}
    <circle cx="8" cy="8" r="6.25" /><path d="M8 7.25v4M8 4.75v.01" />
  {:else if name === "success"}
    <circle cx="8" cy="8" r="6.25" /><path d="M5.25 8.25l2 2 3.5-4" />
  {:else if name === "warning"}
    <path d="M8 2.25l6.25 11H1.75z" /><path d="M8 6.5v3M8 11.5v.01" />
  {:else if name === "danger"}
    <path d="M5.5 1.75h5l3.75 3.75v5l-3.75 3.75h-5L1.75 10.5v-5z" /><path d="M8 4.75v4M8 11.25v.01" />
  {:else if name === "check"}
    <path d="M3.5 8.5l3 3 6-7" />
  {:else if name === "chevronRight"}
    <path d="M6 4l4 4-4 4" />
  {:else if name === "chevronDown"}
    <path d="M4 6l4 4 4-4" />
  {:else if name === "upload"}
    <path d="M8 10.5V2.75M4.75 6L8 2.75 11.25 6M2.75 10.5v2.75h10.5V10.5" />
  {:else if name === "arrowUp"}
    <path d="M8 13V3M4 7l4-4 4 4" />
  {:else if name === "arrowDown"}
    <path d="M8 3v10M4 9l4 4 4-4" />
  {:else if name === "reload"}
    <path d="M13 8a5 5 0 1 1-1.5-3.5M11.5 1.75v2.75h-2.75" />
  {:else if name === "play"}
    <path d="M5 3.5v9l7.5-4.5z" />
  {:else if name === "stop"}
    <rect x="4" y="4" width="8" height="8" rx="1" />
  {:else if name === "stepBack"}
    <path d="M12 3.5v9L5.5 8z" /><rect x="3" y="3.5" width="2" height="9" />
  {:else if name === "stepForward"}
    <path d="M4 3.5v9L10.5 8z" /><rect x="11" y="3.5" width="2" height="9" />
  {:else if name === "key"}
    <path d="M8 2.5L13.5 8 8 13.5 2.5 8z" />
  {:else if name === "plus"}
    <path d="M3.5 8h9M8 3.5v9" />
  {:else if name === "minus"}
    <path d="M3.5 8h9" />
  {:else if name === "fit"}
    <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />
  {:else if name === "settings"}
    <circle cx="8" cy="8" r="2.2" />
    <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
  {/if}
</svg>
