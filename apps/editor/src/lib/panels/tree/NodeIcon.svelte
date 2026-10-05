<!-- ツリーの節の種類を表す小さなアイコン。 -->
<script lang="ts">
import { t } from "../../i18n";
import type { NodeIcon } from "./build";

interface Props {
  icon: NodeIcon;
}

let { icon }: Props = $props();

const titles = $derived<Record<NodeIcon, string>>({
  part: t("tree.icon.part"),
  warp: t("tree.icon.warp"),
  rotation: t("tree.icon.rotation"),
  deformer: t("tree.icon.deformer"),
  mesh: t("tree.icon.mesh"),
  mask: t("tree.icon.mask"),
});
</script>

<svg class="icon {icon}" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
  <title>{titles[icon]}</title>
  {#if icon === "part"}
    <path d="M1.5 4.5v8h13v-7h-6l-1.5-1.5h-5.5z" />
  {:else if icon === "warp"}
    <path d="M2 3c4 1 8 1 12 0v10c-4 1-8 1-12 0zM6.7 3.7v9.6M11 3.6v9.8M2 8c4 1 8 1 12 0" />
  {:else if icon === "rotation"}
    <path d="M13 8a5 5 0 1 1-1.5-3.5M11.5 1.8v2.9h-2.9" />
    <circle cx="8" cy="8" r="1" />
  {:else if icon === "deformer"}
    <path d="M8 2l6 6-6 6-6-6z" />
  {:else if icon === "mesh"}
    <path d="M2 13l6-10 6 10zM8 3v10M5 8h6" />
  {:else}
    <path d="M2.5 2.5h11v11h-11z" stroke-dasharray="2 1.5" />
    <path d="M5 11l3-6 3 6z" />
  {/if}
</svg>

<style>
.icon {
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: var(--ugoe-icon-stroke);
  stroke-linejoin: round;
  stroke-linecap: round;
}
.part {
  color: var(--ugoe-color-hue-amber);
}
.warp,
.rotation,
.deformer {
  color: var(--ugoe-color-hue-cyan);
}
.mesh {
  color: var(--ugoe-color-accent);
}
.mask {
  color: var(--ugoe-color-hue-pink);
}
</style>
