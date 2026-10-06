<script lang="ts">
import Button from "../components/Button.svelte";

let { onsubmit }: { onsubmit: () => void } = $props();
</script>

<form
  id="rename-form"
  onsubmit={(e) => {
    e.preventDefault();
    onsubmit();
  }}
>
  <input name="name" value="Layer" />
</form>
<Button id="submit" type="submit" form="rename-form" variant="primary">Rename</Button>
