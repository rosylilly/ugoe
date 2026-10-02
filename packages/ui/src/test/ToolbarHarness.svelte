<script lang="ts">
import Button from "../components/Button.svelte";
import IconButton from "../components/IconButton.svelte";
import Toolbar from "../components/Toolbar.svelte";
</script>

<Toolbar label="Tools">
  <Button>Undo</Button>
  <IconButton label="Mirror" pressed={true}><svg viewBox="0 0 16 16"></svg></IconButton>
  <Button>Redo</Button>
</Toolbar>
