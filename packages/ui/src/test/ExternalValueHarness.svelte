<!-- Slider と NumberField に、外から値を渡し直す (指揮役の値を表示するときのように)。`control.set` で値を変える。 -->
<script lang="ts">
import { untrack } from "svelte";
import NumberField from "../components/NumberField.svelte";
import Slider from "../components/Slider.svelte";

let {
  kind,
  initial,
  control,
  onchange,
  oncommit,
}: {
  kind: "slider" | "field";
  initial: number;
  control: { set?: (value: number) => void };
  onchange?: (value: number) => void;
  oncommit?: (value: number) => void;
} = $props();

let value = $state(untrack(() => initial));
untrack(() => control).set = (v) => {
  value = v;
};
</script>

{#if kind === "slider"}
  <Slider label="Breath" {value} min={0} max={1} step={0.1} defaultValue={0} {onchange} {oncommit} />
{:else}
  <NumberField label="Breath" {value} min={0} max={1} step={0.1} {onchange} />
{/if}
