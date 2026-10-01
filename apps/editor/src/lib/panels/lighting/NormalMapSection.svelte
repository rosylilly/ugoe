<!--
  選んだメッシュのテクスチャの法線マップ (docs/editor.md#ライティングの画面)。
  - 「Generate」: 部位のまとまり (顔と目口鼻眉・首と体と腕・髪など。意味ラベルから決める) の形を 1 つのふくらみにして
    法線マップを作る (AI なし。ipc.generateTextureNormalMap)。強さ・縁の丸み・ならしはまとまりの既定から始まる (ipc.getNormalMapGroup)。
    縁の幅とならしはキャンバスの px で見せる (単独でふくらませるものはテクスチャの px。lighting.ts の bulgeForm)。
    「Generate for the group」はまとまりのすべてのテクスチャに作る。眼鏡はまとまり (顔) の形に加えず、向きだけを受ける
    (`oriented`)。すべてのテクスチャをまとめて提案するのは AI タブ (lib/ai/ProposalsPanel.svelte)。
  - 「Assign PNG…」: PNG を選んで付ける (DirectX 形式なら「Flip green」)。
  - 「Remove」: set_normal_map で外す。
  どれも 1 回の Undo で戻る。テクスチャと法線マップの画像を並べて表示する。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { fileDialogs } from "../../fileDialogs";
import { type ApplyReport, ipc, type Model, type NormalMapGroupView } from "../../ipc";
import { textureLabel } from "../../labels";
import { type BulgeForm, bulgeForm, bulgeLimits, bulgeSettings, bulgeUnit } from "./lighting";

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

/** ふくらみの設定 (長さは画面の単位)。 */
let form = $state<BulgeForm>(bulgeForm(null));
/** 選んだテクスチャの法線マップのまとまり (取れなければ null)。 */
let group = $state<NormalMapGroupView | null>(null);
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

/** 設定をまとまりの既定に戻す。 */
function useDefaults(view: NormalMapGroupView | null): void {
  form = bulgeForm(view);
}

// テクスチャを替えたら、まとまりと既定の設定を取り直す (ラベルはモデルの編集でも変わりうるので revision も見る)。
let groupFor: string | null = null;
$effect(() => {
  void doc.revision;
  const name = ref?.name ?? null;
  let cancelled = false;
  if (!name) {
    group = null;
    return;
  }
  void ipc
    .getNormalMapGroup(name)
    .catch(() => null)
    .then((view) => {
      if (cancelled) return;
      group = view;
      if (groupFor !== name) {
        groupFor = name;
        useDefaults(view);
      }
    });
  return () => {
    cancelled = true;
  };
});

/** 縁の幅とならしの欄の上限。 */
const limits = $derived(bulgeLimits(group));
/** 長さの単位 (まとまりはキャンバスの px)。 */
const unit = $derived(bulgeUnit(group));
/** いっしょにふくらませるほかのテクスチャ。 */
const partners = $derived(group ? group.textures.filter((t) => t !== ref?.name) : []);

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

function generate(wholeGroup: boolean): void {
  if (!ref) return;
  const name = ref.name;
  const settings = bulgeSettings(form, group);
  void run(() => ipc.generateTextureNormalMap(name, settings, wholeGroup));
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
      Makes the opaque shape bulge toward the viewer (no AI). Parts of the same group are shaped as one surface, so eyes
      follow the face and the neck joins the body; glasses take the face's direction without bulging. Turns on lighting
      for the model if it is off. To propose maps for every texture at once, use the AI tab.
    </p>
    {#if group}
      <p class="muted note" data-testid="normal-map-group">
        {#if group.group && ref && group.oriented.includes(ref.name)}
          Takes the direction of group “{group.group}” ({partners.filter((t) => !group?.oriented.includes(t)).join(", ")}) without bulging.
        {:else if group.group && partners.length > 0}
          Group “{group.group}” with {partners.join(", ")}{#if group.shape_only.length > 0}; also shaped by {group.shape_only.join(", ")}{/if}{#if group.oriented.length > 0}; {group.oriented.join(", ")} only take its direction{/if}.
        {:else if group.group}
          Group “{group.group}” (this texture only).
        {:else}
          No part label: shaped on its own.
        {/if}
        Labels from {group.from_classification ? "the PSD classification" : "the layer names"}.
      </p>
    {/if}
    <label class="row">
      <span>Strength</span>
      <span class="controls">
        <input type="range" min="0" max="2" step="0.05" bind:value={form.strength} aria-label="Strength" />
        <span class="value">{form.strength.toFixed(2)}</span>
      </span>
    </label>
    <label class="row">
      <span>Round edges only</span>
      <input type="checkbox" bind:checked={form.roundEdges} />
    </label>
    {#if form.roundEdges}
      <label class="row">
        <span>Edge width ({unit})</span>
        <span class="controls">
          <input type="range" min="1" max={limits.edge} step="1" bind:value={form.edge} aria-label="Edge width" />
          <span class="value">{Math.round(form.edge)}</span>
        </span>
      </label>
    {/if}
    <label class="row">
      <span>Smoothing ({unit})</span>
      <span class="controls">
        <input
          type="range"
          min="0"
          max={limits.smoothing}
          step="0.5"
          bind:value={form.smoothing}
          aria-label="Smoothing"
        />
        <span class="value">{form.smoothing.toFixed(1)}</span>
      </span>
    </label>
    <div class="row buttons">
      <button disabled={busy} onclick={() => generate(false)}>{ref.normal_map ? "Regenerate" : "Generate"}</button>
      {#if partners.length > 0}
        <button disabled={busy} onclick={() => generate(true)}>Generate for the group</button>
      {/if}
      <button disabled={busy || !group} onclick={() => useDefaults(group)}>Defaults</button>
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
