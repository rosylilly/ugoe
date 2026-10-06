<script lang="ts">
import Button from "../components/Button.svelte";
import Dialog from "../components/Dialog.svelte";
import Icon from "../components/Icon.svelte";
import IconButton from "../components/IconButton.svelte";
import NumberField from "../components/NumberField.svelte";

let {
  dismissible = true,
  initialFocus,
  onclose,
}: { dismissible?: boolean; initialFocus?: string; onclose?: () => void } = $props();
let open = $state(false);
</script>

<Button id="opener" onclick={() => (open = true)}>Open</Button>
<Dialog bind:open title="Rename" description="Give the layer a new name." {dismissible} {initialFocus} {onclose}>
  <NumberField label="Opacity" value={1} />
  <IconButton id="help" label="Help"><Icon name="info" /></IconButton>
  {#snippet actions()}
    <Button id="cancel" onclick={() => (open = false)}>Cancel</Button>
    <Button id="ok" variant="primary" onclick={() => (open = false)}>OK</Button>
  {/snippet}
</Dialog>
<output id="state">{open ? "open" : "closed"}</output>
