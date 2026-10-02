<script lang="ts">
import Button from "../components/Button.svelte";
import Dialog from "../components/Dialog.svelte";
import NumberField from "../components/NumberField.svelte";

let { dismissible = true, onclose }: { dismissible?: boolean; onclose?: () => void } = $props();
let open = $state(false);
</script>

<Button id="opener" onclick={() => (open = true)}>Open</Button>
<Dialog bind:open title="Rename" description="Give the layer a new name." {dismissible} {onclose}>
  <NumberField label="Opacity" value={1} />
  {#snippet actions()}
    <Button id="cancel" onclick={() => (open = false)}>Cancel</Button>
    <Button id="ok" variant="primary" onclick={() => (open = false)}>OK</Button>
  {/snippet}
</Dialog>
<output id="state">{open ? "open" : "closed"}</output>
