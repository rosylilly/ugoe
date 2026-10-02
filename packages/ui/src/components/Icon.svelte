<!--
  部品の中で使う小さな印 (内部用。公開しない)。飾りなので aria-hidden にする。
-->
<script lang="ts">
type Name =
  | "close"
  | "info"
  | "success"
  | "warning"
  | "danger"
  | "check"
  | "chevronRight"
  | "chevronDown"
  | "upload";

let { name }: { name: Name } = $props();
</script>

<svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
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
  {/if}
</svg>

<style>
  :global {
    .ugoe-icon {
      flex: none;
      width: 1.15em;
      height: 1.15em;
      fill: none;
      stroke: currentColor;
      stroke-width: 1.5;
      stroke-linecap: round;
      stroke-linejoin: round;
    }
  }
</style>
