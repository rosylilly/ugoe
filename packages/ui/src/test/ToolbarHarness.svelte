<script lang="ts">
import Button from "../components/Button.svelte";
import IconButton from "../components/IconButton.svelte";
import Toolbar from "../components/Toolbar.svelte";

/** 最初は Undo と Mirror を無効にして描き、外のボタン (#enable) で押せるようにする。 */
let { disabledAtFirst = false }: { disabledAtFirst?: boolean } = $props();
// svelte-ignore state_referenced_locally
let disabled = $state(disabledAtFirst);
</script>

<Toolbar label="Tools">
  <Button {disabled}>Undo</Button>
  <IconButton label="Mirror" pressed={true} {disabled}><svg viewBox="0 0 16 16"></svg></IconButton>
  <Button {disabled}>Redo</Button>
</Toolbar>
<button id="enable" type="button" onclick={() => (disabled = false)}>enable</button>
