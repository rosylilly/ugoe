<script lang="ts">
import IconButton from "../components/IconButton.svelte";
import Tooltip from "../components/Tooltip.svelte";
import TooltipProvider from "../components/TooltipProvider.svelte";
</script>

<TooltipProvider delay={0}>
  <Tooltip text="Fit the model to the view" id="plain" delay={0}>
    {#snippet children(attrs)}<button {...attrs}>Fit</button>{/snippet}
  </Tooltip>
  <IconButton id="undo" label="Undo" shortcut="Mod+Z"><svg viewBox="0 0 16 16"></svg></IconButton>
  <IconButton id="quiet" label="Quiet" tooltip={false}><svg viewBox="0 0 16 16"></svg></IconButton>
</TooltipProvider>
