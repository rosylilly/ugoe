<!--
  選んだメッシュのテクスチャの法線マップ (docs/editor.md#ライティングの画面)。
  - 「Generate」: アルファの形からふくらみの法線マップを作る (AI なし。ipc.generateTextureNormalMap)。強さと縁の丸みを選ぶ。
  - 「Assign PNG…」: PNG を選んで付ける (DirectX 形式なら「Flip green」)。
  - 「Remove」: set_normal_map で外す。
  どれも 1 回の Undo で戻る。テクスチャと法線マップの画像を並べて表示する。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { fileDialogs } from "../../fileDialogs";
import { type ApplyReport, ipc, type Model } from "../../ipc";
import { textureLabel } from "../../labels";

interface Props {
  doc: EditorDocument;
  model: Model;
  /** 法線マップを付けるテクスチャの番号。 */
  texture: number;
}

let { doc, model, texture }: Props = $props();

const PNG_FILTER = { name: "PNG image (.png)", extensions: ["png"] };

const ref = $derived(model.textures[texture] ?? null);
const users = $derived(
  model.meshes.filter((m) => m.texture === texture).map((m) => m.name || m.id),
);
/** 同じ法線マップの画像を指すほかのテクスチャ。 */
const sharedWith = $derived(
  ref?.normal_map
    ? model.textures
        .filter((t) => t.name !== ref.name && t.normal_map === ref.normal_map)
        .map((t) => t.name)
    : [],
);

let strength = $state(0.5);
let roundEdgesOnly = $state(false);
let edge = $state(12);
let flipGreen = $state(false);
let busy = $state(false);
let error = $state<string | null>(null);

// 画像の表示 (blob の URL)。モデルが変わるたびに取り直す。
let textureUrl = $state<string | null>(null);
let normalUrl = $state<string | null>(null);

async function imageUrl(name: string | null | undefined): Promise<string | null> {
  if (!name) return null;
  try {
    const bytes = await ipc.getTexturePng(name);
    return URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
  } catch {
    return null;
  }
}

$effect(() => {
  void doc.revision;
  const names = [ref?.name, ref?.normal_map];
  let cancelled = false;
  let urls: (string | null)[] = [];
  void Promise.all(names.map(imageUrl)).then((got) => {
    urls = got;
    if (cancelled) {
      for (const u of got) if (u) URL.revokeObjectURL(u);
      return;
    }
    [textureUrl, normalUrl] = got as [string | null, string | null];
  });
  return () => {
    cancelled = true;
    for (const u of urls) if (u) URL.revokeObjectURL(u);
  };
});

async function run(action: () => Promise<ApplyReport>): Promise<void> {
  if (busy) return;
  busy = true;
  error = null;
  try {
    if ((await doc.applyWith(action)) === null) error = doc.error;
  } finally {
    busy = false;
  }
}

function generate(): void {
  if (!ref) return;
  const name = ref.name;
  void run(() =>
    ipc.generateTextureNormalMap(name, {
      strength,
      edge: roundEdgesOnly ? edge : null,
      smoothing: 1,
    }),
  );
}

async function assign(): Promise<void> {
  if (!ref) return;
  const name = ref.name;
  const path = await fileDialogs().openFile({ title: "Assign normal map", filters: [PNG_FILTER] });
  if (path) void run(() => ipc.assignTextureNormalMap(name, path, flipGreen));
}

function remove(): void {
  if (!ref) return;
  void doc.apply({ op: "set_normal_map", texture: ref.name, normal_map: null });
}
</script>

{#if ref}
  <section aria-label="Normal map">
    <h3>Normal map</h3>
    <p class="muted">Texture: {textureLabel(ref.name, users)}</p>
    <div class="images">
      <figure>
        {#if textureUrl}<img src={textureUrl} alt="Texture {ref.name}" />{:else}<div class="empty"></div>{/if}
        <figcaption>Texture</figcaption>
      </figure>
      <figure>
        {#if normalUrl}
          <img src={normalUrl} alt="Normal map {ref.normal_map}" />
        {:else}
          <div class="empty muted">none</div>
        {/if}
        <figcaption>{ref.normal_map ?? "No normal map"}</figcaption>
      </figure>
    </div>
    {#if sharedWith.length > 0}
      <p class="muted note">
        Shared with {sharedWith.join(", ")}. Generating or assigning makes a separate image for this texture.
      </p>
    {/if}

    <h4>Generate from the shape</h4>
    <p class="muted note">
      Makes the texture's opaque shape bulge toward the viewer (no AI). Turns on lighting for the model if it is off.
    </p>
    <label class="row">
      <span>Strength</span>
      <span class="controls">
        <input type="range" min="0" max="2" step="0.05" bind:value={strength} aria-label="Strength" />
        <span class="value">{strength.toFixed(2)}</span>
      </span>
    </label>
    <label class="row">
      <span>Round edges only</span>
      <input type="checkbox" bind:checked={roundEdgesOnly} />
    </label>
    {#if roundEdgesOnly}
      <label class="row">
        <span>Edge width (px)</span>
        <span class="controls">
          <input type="range" min="1" max="64" step="1" bind:value={edge} aria-label="Edge width" />
          <span class="value">{edge}</span>
        </span>
      </label>
    {/if}
    <div class="row buttons">
      <button disabled={busy} onclick={generate}>{ref.normal_map ? "Regenerate" : "Generate"}</button>
    </div>

    <h4>Image</h4>
    <label class="row">
      <span>Flip green (DirectX)</span>
      <input type="checkbox" bind:checked={flipGreen} />
    </label>
    <div class="row buttons">
      <button disabled={busy} onclick={() => void assign()}>Assign PNG…</button>
      <button disabled={busy || !ref.normal_map} onclick={remove}>Remove normal map</button>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
  </section>
{/if}

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 4px 0;
}

.controls {
  display: flex;
  align-items: center;
  gap: 4px;
}

.controls input[type="range"] {
  width: 110px;
}

.value {
  width: 32px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.buttons {
  justify-content: flex-start;
  flex-wrap: wrap;
}

.images {
  display: flex;
  gap: 8px;
}

figure {
  flex: 1;
  min-width: 0;
  margin: 0;
}

img,
.empty {
  display: block;
  width: 100%;
  aspect-ratio: 1;
  object-fit: contain;
  border: 1px solid var(--border);
  /* 透明な部分が分かるように市松模様にする。 */
  background: repeating-conic-gradient(#8884 0% 25%, transparent 0% 50%) 50% / 16px 16px;
}

.empty {
  display: grid;
  place-items: center;
}

figcaption {
  font-size: 11px;
  overflow-wrap: anywhere;
}

.note {
  margin: 2px 0 6px;
  font-size: 11px;
}

h3,
h4 {
  margin: 8px 0 4px;
}
</style>
