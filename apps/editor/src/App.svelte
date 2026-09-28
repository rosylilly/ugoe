<script lang="ts">
import { type AppInfo, ipc } from "./lib/ipc";

interface ParameterView {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
}

let info = $state<AppInfo | null>(null);
let parameters = $state<ParameterView[]>([]);
let error = $state<string | null>(null);

async function refresh() {
  try {
    info = await ipc.appInfo();
    const model = JSON.parse(await ipc.getModelJson());
    parameters = model.parameters;
    error = null;
  } catch (e) {
    error = String(e);
  }
}

async function undo() {
  await ipc.undo();
  await refresh();
}

async function redo() {
  await ipc.redo();
  await refresh();
}

refresh();
</script>

<div class="shell">
  <header>
    <strong>ugoe</strong>
    <span class="muted">{info ? `v${info.version}` : "…"}</span>
    <span class="spacer"></span>
    <button onclick={undo}>Undo</button>
    <button onclick={redo}>Redo</button>
  </header>
  <aside class="left">
    <h2>Parts</h2>
    <p class="muted">TODO(T0602): パーツ / デフォーマツリー</p>
  </aside>
  <main class="viewport">
    <!-- TODO(T0402): ugoe-wasm による WebGPU / WebGL2 ビューポート -->
    <canvas></canvas>
  </main>
  <aside class="right">
    <h2>Parameters</h2>
    {#if error}<p class="error">{error}</p>{/if}
    <ul>
      {#each parameters as p (p.id)}
        <li>
          <label>
            {p.name}
            <input type="range" min={p.min} max={p.max} step="0.01" value={p.default} />
          </label>
        </li>
      {/each}
    </ul>
  </aside>
</div>
