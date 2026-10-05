<!-- TextField に、外から値を渡し直す (確定した名前を呼び手が整えて返すときのように)。`control.set` で値を変える。 -->
<script lang="ts">
import { untrack } from "svelte";
import TextField from "../components/TextField.svelte";

let {
  initial,
  control,
  onchange,
}: {
  initial: string;
  control: { set?: (value: string) => void };
  onchange?: (value: string) => void;
} = $props();

let value = $state(untrack(() => initial));
untrack(() => control).set = (v) => {
  value = v;
};
</script>

<TextField label="Name" {value} {onchange} />
