<script lang="ts">
import Button from "../components/Button.svelte";
import Popover from "../components/Popover.svelte";
import type { ButtonBaseProps } from "../types";

let { onclose }: { onclose?: () => void } = $props();
</script>

<Popover label="Light settings" {onclose}>
  {#snippet trigger(attrs)}<Button {...attrs as ButtonBaseProps} id="trigger">Light</Button>{/snippet}
  <button id="inside">Inside</button>
</Popover>
