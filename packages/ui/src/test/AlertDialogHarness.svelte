<script lang="ts">
import AlertDialog from "../components/AlertDialog.svelte";
import Button from "../components/Button.svelte";

let { onconfirm, oncancel }: { onconfirm: () => void; oncancel: () => void } = $props();
let open = $state(false);
</script>

<Button id="opener" variant="danger" onclick={() => (open = true)}>Forget</Button>
<AlertDialog
  bind:open
  title="Forget everything?"
  description="This cannot be undone."
  confirmLabel="Forget everything"
  {onconfirm}
  {oncancel}
/>
<output id="state">{open ? "open" : "closed"}</output>
