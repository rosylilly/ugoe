<!--
  右の列のタブのアイコン (T1632)。エディタでしか使わないので @ugoe/ui の Icon には置かない (docs/ui.md#アイコン)。
  16 の格子に線で描き、`.ugoe-icon` を付ける。絵の選び方と理由は docs/ui.md#アイコン の「右の列のタブ」。
-->
<script lang="ts" module>
export type PaneIconName =
  | "inspector"
  | "physics"
  | "glue"
  | "lighting"
  | "slots"
  | "poses"
  | "verify"
  | "ai";
</script>

<script lang="ts">
let { name }: { name: PaneIconName } = $props();
</script>

<svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
  {#if name === "inspector"}
    <path d="M2.5 4.5h3M8.5 4.5h5M2.5 8h7M12.5 8h1M2.5 11.5h1M6.5 11.5h7" />
    <circle cx="7" cy="4.5" r="1.5" /><circle cx="11" cy="8" r="1.5" /><circle cx="5" cy="11.5" r="1.5" />
  {:else if name === "physics"}
    <path d="M5.5 2h5M8 2v6.75" /><circle cx="8" cy="10.75" r="2" />
    <path d="M2.75 8.5a5.5 5.5 0 0 0 1.75 3.5M13.25 8.5a5.5 5.5 0 0 1-1.75 3.5" />
  {:else if name === "glue"}
    <path d="M6.5 9.5l3-3M7 4.5l1-1a2.5 2.5 0 0 1 3.5 3.5l-1 1M9 11.5l-1 1a2.5 2.5 0 0 1-3.5-3.5l1-1" />
  {:else if name === "lighting"}
    <circle cx="8" cy="8" r="2.75" />
    <path d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1" />
  {:else if name === "slots"}
    <path d="M6.5 4a1.5 1.5 0 1 1 1.5 1.5v1.25L2 11.5h12L8 6.75" />
  {:else if name === "poses"}
    <circle cx="8" cy="3" r="1.5" /><path d="M8 5v4.5M8 9.5l-2.5 4.5M8 9.5l2.5 4.5M3.5 4.5L8 6.5l4.5-3.5" />
  {:else if name === "verify"}
    <circle cx="7" cy="7" r="4.5" /><path d="M10.5 10.5l3 3M5 7l1.5 1.5 2.5-3" />
  {:else if name === "ai"}
    <path d="M7 2.5l1.2 3.8 3.8 1.2-3.8 1.2L7 12.5 5.8 8.7 2 7.5l3.8-1.2zM12.5 10.5v3M11 12h3" />
  {/if}
</svg>
