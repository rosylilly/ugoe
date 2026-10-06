<!-- SecretField を呼び手と同じ形で使う (`bind:this` で `take`・`clear` を呼び、`bind:filled` を読む)。 -->
<script lang="ts">
import { untrack } from "svelte";
import SecretField from "../components/SecretField.svelte";

let {
  control,
}: {
  control: { field?: SecretField; filled?: () => boolean };
} = $props();

let field = $state<SecretField>();
let filled = $state(false);
$effect(() => {
  untrack(() => control).field = field;
});
untrack(() => control).filled = () => filled;
</script>

<SecretField bind:this={field} bind:filled label="API key" placeholder="Not stored" />
