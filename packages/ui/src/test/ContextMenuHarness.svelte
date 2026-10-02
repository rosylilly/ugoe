<script lang="ts">
import ContextMenu from "../components/ContextMenu.svelte";
import type { MenuEntry } from "../types";

let { entries }: { entries: MenuEntry[] } = $props();
</script>

<ContextMenu {entries} label="Layer actions"><div id="area">Layer</div></ContextMenu>
