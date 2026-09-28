<!--
  パラメータ一覧。スライダーの値は表示の状態なので、ViewportController (wasm) を直接操作する (IPC を通らない)。
  値域などの定義を変える操作 (キーの追加など、T0602 以降) は EditorDocument.apply で Command を送る。
-->
<script lang="ts">
import type { ParameterDef } from "../document.svelte";
import type { ViewportController } from "../viewport/controller.svelte";

interface Props {
  parameters: ParameterDef[];
  viewport: ViewportController;
}

let { parameters, viewport }: Props = $props();

function onInput(id: string, e: Event) {
  viewport.setParameter(id, (e.currentTarget as HTMLInputElement).valueAsNumber);
}
</script>

<div class="head">
  <h2>Parameters</h2>
  <button onclick={() => viewport.resetParameters()}>Reset</button>
</div>
<ul>
  {#each parameters as p (p.id)}
    {@const value = viewport.parameterValues[p.id] ?? p.default}
    <li>
      <label>
        <span class="row">
          <span>{p.name}</span>
          <span class="muted">{value.toFixed(2)}</span>
        </span>
        <input
          type="range"
          min={p.min}
          max={p.max}
          step={(p.max - p.min) / 1000 || 0.001}
          {value}
          oninput={(e) => onInput(p.id, e)}
        />
      </label>
    </li>
  {/each}
</ul>

<style>
.head,
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
</style>
