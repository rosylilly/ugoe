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
import { formatList } from "@ugoe/i18n";
import { Button, Checkbox, Slider } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { errorText } from "../../errors";
import { fileDialogs } from "../../fileDialogs";
import { PNG_FILTER } from "../../filePaths";
import { t } from "../../i18n";
import { type ApplyReport, ipc, type Model, type NormalMapGroupView } from "../../ipc";
import { textureLabel } from "../../labels";
import { useBaseLock } from "../../ui/baseLock";
import { type BulgeForm, bulgeForm, bulgeLimits, bulgeSettings, bulgeUnit } from "./lighting";

interface Props {
  doc: EditorDocument;
  model: Model;
  /** 法線マップを付けるテクスチャの番号。 */
  texture: number;
}

let { doc, model, texture }: Props = $props();

const headingId = $props.id();
/** 本体のテクスチャなら BaseLock が無効にする (Slider のつまみには fieldset の disabled が効かない)。 */
const locked = useBaseLock();

const ref = $derived(model.textures[texture] ?? null);
const users = $derived(
  model.meshes.filter((m) => m.texture === texture).map((m) => m.name || m.id),
);
/** 同じ法線マップの画像を指すほかのテクスチャ。 */
const sharedWith = $derived(
  ref?.normal_map
    ? model.textures
        .filter((tex) => tex.name !== ref.name && tex.normal_map === ref.normal_map)
        .map((tex) => tex.name)
    : [],
);

/** ふくらみの設定 (長さは画面の単位)。 */
let form = $state<BulgeForm>(bulgeForm(null));
/** 選んだテクスチャの法線マップのまとまり (取れなければ null)。 */
let group = $state<NormalMapGroupView | null>(null);
let flipGreen = $state(false);
/** 編集を送っている間 (法線マップを作っている間を含む) はボタンを無効にする (docs/editor.md#パネルからの編集)。 */
const busy = $derived(doc.pending > 0);
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
const partners = $derived(group ? group.textures.filter((name) => name !== ref?.name) : []);

/** 前の編集の後に `action` で法線マップを作る・割り当てる。失敗はこの節に出す。 */
async function run(action: () => Promise<ApplyReport>): Promise<void> {
  error = null;
  try {
    await doc.applyWith(action, { rethrow: true });
  } catch (e) {
    error = errorText(e);
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
  const path = await fileDialogs().openFile({
    title: t("lighting.normalMap.assign.title"),
    filters: [PNG_FILTER],
  });
  if (path) void run(() => ipc.assignTextureNormalMap(name, path, flipGreen));
}

function remove(): void {
  if (!ref) return;
  void doc.apply({ op: "set_normal_map", texture: ref.name, normal_map: null });
}
</script>

{#if ref}
  <section aria-labelledby={headingId}>
    <h3 id={headingId}>{t("lighting.normalMap.title")}</h3>
    <p class="ed-hint">{t("lighting.normalMap.texture", { texture: textureLabel(ref.name, users) })}</p>
    <div class="images">
      <figure>
        {#if textureUrl}
          <img src={textureUrl} alt={t("lighting.normalMap.textureImage.alt", { name: ref.name })} />
        {:else}<div class="empty"></div>{/if}
        <figcaption>{t("lighting.normalMap.textureImage.caption")}</figcaption>
      </figure>
      <figure>
        {#if normalUrl}
          <img src={normalUrl} alt={t("lighting.normalMap.image.alt", { name: ref.normal_map ?? "" })} />
        {:else}
          <div class="empty ed-hint">{t("lighting.normalMap.image.none")}</div>
        {/if}
        <figcaption>{ref.normal_map ?? t("lighting.normalMap.image.empty")}</figcaption>
      </figure>
    </div>
    {#if sharedWith.length > 0}
      <p class="ed-hint note">
        {t("lighting.normalMap.sharedWith", { textures: formatList(sharedWith) })}
      </p>
    {/if}

    <h4>{t("lighting.normalMap.generate.title")}</h4>
    <p class="ed-hint note">{t("lighting.normalMap.generate.description")}</p>
    {#if group}
      <p class="ed-hint note" data-testid="normal-map-group">
        {#if group.group && ref && group.oriented.includes(ref.name)}
          {t("lighting.normalMap.group.oriented", {
            group: group.group,
            textures: formatList(partners.filter((p) => !group?.oriented.includes(p))),
          })}
        {:else if group.group && partners.length > 0}
          {t("lighting.normalMap.group.with", { group: group.group, textures: formatList(partners) })}
          {#if group.shape_only.length > 0}
            {t("lighting.normalMap.group.shapeOnly", { textures: formatList(group.shape_only) })}
          {/if}
          {#if group.oriented.length > 0}
            {t("lighting.normalMap.group.orientedOnly", { textures: formatList(group.oriented) })}
          {/if}
        {:else if group.group}
          {t("lighting.normalMap.group.alone", { group: group.group })}
        {:else}
          {t("lighting.normalMap.group.none")}
        {/if}
        {group.from_classification
          ? t("lighting.normalMap.group.fromClassification")
          : t("lighting.normalMap.group.fromLayerNames")}
      </p>
    {/if}
    <Slider
      label={t("lighting.normalMap.strength.label")}
      value={form.strength}
      min={0}
      max={2}
      step={0.05}
      decimals={2}
      disabled={locked()}
      onchange={(v) => (form.strength = v)}
    />
    <div class="row">
      <Checkbox label={t("lighting.normalMap.roundEdges.label")} bind:checked={form.roundEdges} />
    </div>
    {#if form.roundEdges}
      <Slider
        label={t("lighting.normalMap.edge.label")}
        {unit}
        value={form.edge}
        min={1}
        max={limits.edge}
        step={1}
        decimals={0}
        disabled={locked()}
        onchange={(v) => (form.edge = v)}
      />
    {/if}
    <Slider
      label={t("lighting.normalMap.smoothing.label")}
      {unit}
      value={form.smoothing}
      min={0}
      max={limits.smoothing}
      step={0.5}
      decimals={1}
      disabled={locked()}
      onchange={(v) => (form.smoothing = v)}
    />
    <div class="ed-buttons row">
      <Button disabled={busy} onclick={() => generate(false)}
        >{ref.normal_map ? t("lighting.normalMap.regenerate.label") : t("lighting.normalMap.generate.label")}</Button
      >
      {#if partners.length > 0}
        <Button disabled={busy} onclick={() => generate(true)}>{t("lighting.normalMap.generateGroup.label")}</Button>
      {/if}
      <Button disabled={busy || !group} onclick={() => useDefaults(group)}
        >{t("lighting.normalMap.defaults.label")}</Button
      >
    </div>

    <h4>{t("lighting.normalMap.imageSection.title")}</h4>
    <div class="row">
      <Checkbox label={t("lighting.normalMap.flipGreen.label")} bind:checked={flipGreen} />
    </div>
    <div class="ed-buttons row">
      <Button disabled={busy} onclick={() => void assign()}>{t("lighting.normalMap.assign.label")}</Button>
      <Button disabled={busy || !ref.normal_map} onclick={remove}>{t("lighting.normalMap.remove.label")}</Button>
    </div>
    {#if error}<p class="error ugoe-selectable">{error}</p>{/if}
  </section>
{/if}

<style>
.row {
  margin: var(--ugoe-space-2) 0;
}

.images {
  display: flex;
  gap: var(--ugoe-space-3);
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
  border: 1px solid var(--ugoe-color-border);
  /* 透明な部分が分かるように市松模様にする。 */
  background: repeating-conic-gradient(#8884 0% 25%, transparent 0% 50%) 50% / 16px 16px;
}

.empty {
  display: grid;
  place-items: center;
}

figcaption {
  font-size: var(--ugoe-font-size-xs);
  overflow-wrap: anywhere;
}

.note {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}

h4 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}
</style>
