<!--
  シーンの編集 (docs/studio.md#操作ウィンドウ のパネル): 名前・モデル・着せ替えのモジュール (合成の順)・ライティング・
  位置と大きさと回転・背景 (透過・単色・クロマキー・画像)。変えるたびに `onchange` で新しい設定を渡す
  (配信しているシーンなら、バックエンドがすぐ出し直す)。
-->
<script lang="ts">
import { Button, ColorPicker, Notice, SegmentedControl, Slider } from "@ugoe/ui";
import LightingEditor from "./LightingEditor.svelte";
import { t } from "./lib/i18n";
import {
  type BackgroundKind,
  CHROMA_COLORS,
  DEFAULT_TRANSFORM,
  fileName,
  POSITION_LIMIT,
  ROTATION_RANGE,
  SCALE_RANGE,
  type SceneSettings,
  sceneLighting,
  type Transform,
  withBackgroundKind,
} from "./lib/profile";

interface Props {
  scene: SceneSettings;
  /** このシーンを配信しているか。 */
  live: boolean;
  /** 配信しているモデルがライティングを受けるか (配信していなければ `null`)。 */
  usesLighting: boolean | null;
  disabled?: boolean;
  onchange: (scene: SceneSettings) => void;
  onactivate: () => void;
  /** ファイルを選ぶ (やめたら `null`)。 */
  onpick: (kind: "model" | "module" | "image") => Promise<string | null>;
}

let { scene, live, usesLighting, disabled = false, onchange, onactivate, onpick }: Props = $props();

const nameId = $props.id();
const lighting = $derived(sceneLighting(scene));
const backgroundOptions = $derived([
  { value: "transparent" as BackgroundKind, label: t("editor.bgTransparent") },
  { value: "color" as BackgroundKind, label: t("editor.bgColor") },
  { value: "chroma" as BackgroundKind, label: t("editor.bgChroma") },
  { value: "image" as BackgroundKind, label: t("editor.bgImage") },
]);
const chromaOptions = $derived([
  { value: CHROMA_COLORS[0] as string, label: t("editor.chromaGreen") },
  { value: CHROMA_COLORS[1] as string, label: t("editor.chromaBlue") },
]);

/**
 * 写しを `change` で書き換えて渡す。何も変わらなければ渡さない (部品が、渡した値と同じ値で onchange を呼んでも
 * 送り返さない。送り合いが止まらなくならないように)。
 */
function edit(change: (next: SceneSettings) => void) {
  const current = $state.snapshot(scene) as SceneSettings;
  const next = structuredClone(current);
  change(next);
  if (JSON.stringify(next) !== JSON.stringify(current)) onchange(next);
}

function setTransform(key: keyof Transform, value: number) {
  edit((s) => (s.transform[key] = value));
}

async function chooseModel() {
  const path = await onpick("model");
  if (path === null || path === scene.model) return;
  edit((s) => {
    s.model = path;
    // 別のモデルのスロットには付かないことが多いので、モジュールは外す。
    s.modules = [];
  });
}

async function addModule() {
  const path = await onpick("module");
  if (path === null || scene.modules.includes(path)) return;
  edit((s) => s.modules.push(path));
}

function moveModule(index: number, to: number) {
  edit((s) => {
    const [moved] = s.modules.splice(index, 1);
    if (moved !== undefined) s.modules.splice(to, 0, moved);
  });
}

async function setBackgroundKind(kind: BackgroundKind) {
  if (kind !== "image") {
    edit((s) => (s.background = withBackgroundKind(s.background, kind)));
    return;
  }
  if (scene.background.kind === "image") return;
  await chooseImage();
}

async function chooseImage() {
  const path = await onpick("image");
  if (path !== null) edit((s) => (s.background = withBackgroundKind(s.background, "image", path)));
}
</script>

<div class="editor">
  {#if !live}
    <Notice tone="info">
      {t("editor.notLive")}
      {#snippet actions()}
        <Button small variant="primary" {disabled} onclick={onactivate}>{t("editor.activate")}</Button>
      {/snippet}
    </Notice>
  {/if}

  <div class="editor__field">
    <label for={nameId}>{t("editor.name")}</label>
    <input
      id={nameId}
      class="editor__input"
      type="text"
      value={scene.name}
      maxlength={100}
      {disabled}
      onchange={(e) => {
        const name = e.currentTarget.value.trim();
        edit((s) => (s.name = name));
      }}
    />
  </div>

  <div class="editor__field">
    <span class="editor__label">{t("editor.model")}</span>
    <div class="editor__file">
      <span class="editor__path">{scene.model === null ? t("editor.noModel") : fileName(scene.model)}</span>
      <Button small {disabled} onclick={chooseModel}>{t("editor.chooseModel")}</Button>
    </div>
    {#if scene.modules.length > 0}
      <p class="editor__hint">{t("editor.modelChangeHint")}</p>
    {/if}
  </div>

  <div class="editor__field">
    <span class="editor__label">{t("editor.modules")}</span>
    {#if scene.modules.length === 0}
      <p class="editor__hint">{t("editor.noModules")}</p>
    {:else}
      <ol class="editor__modules">
        {#each scene.modules as path, i (path)}
          {@const name = fileName(path)}
          <li>
            <span class="editor__path">{name}</span>
            <span class="editor__row-actions">
              <Button small disabled={disabled || i === 0} onclick={() => moveModule(i, i - 1)}>
                {t("scenes.moveUp")}
              </Button>
              <Button
                small
                disabled={disabled || i === scene.modules.length - 1}
                onclick={() => moveModule(i, i + 1)}
              >
                {t("scenes.moveDown")}
              </Button>
              <Button small {disabled} onclick={() => edit((s) => s.modules.splice(i, 1))}>
                {t("editor.moduleRemove")}
              </Button>
            </span>
          </li>
        {/each}
      </ol>
    {/if}
    <div>
      <Button small disabled={disabled || scene.model === null} onclick={addModule}>{t("editor.addModule")}</Button>
    </div>
  </div>

  <fieldset class="editor__group">
    <legend>{t("editor.placement")}</legend>
    <Slider
      label={t("editor.x")}
      value={scene.transform.x}
      min={-POSITION_LIMIT / 2}
      max={POSITION_LIMIT / 2}
      step={0.01}
      defaultValue={DEFAULT_TRANSFORM.x}
      {disabled}
      onchange={(v) => setTransform("x", v)}
    />
    <Slider
      label={t("editor.y")}
      value={scene.transform.y}
      min={-POSITION_LIMIT / 2}
      max={POSITION_LIMIT / 2}
      step={0.01}
      defaultValue={DEFAULT_TRANSFORM.y}
      {disabled}
      onchange={(v) => setTransform("y", v)}
    />
    <Slider
      label={t("editor.scale")}
      value={scene.transform.scale}
      min={SCALE_RANGE[0]}
      max={5}
      step={0.01}
      unit={t("editor.times")}
      defaultValue={DEFAULT_TRANSFORM.scale}
      {disabled}
      onchange={(v) => setTransform("scale", v)}
    />
    <Slider
      label={t("editor.rotation")}
      value={scene.transform.rotation}
      min={ROTATION_RANGE[0]}
      max={ROTATION_RANGE[1]}
      step={1}
      unit={t("editor.degrees")}
      defaultValue={DEFAULT_TRANSFORM.rotation}
      {disabled}
      onchange={(v) => setTransform("rotation", v)}
    />
    <div>
      <Button small {disabled} onclick={() => edit((s) => (s.transform = { ...DEFAULT_TRANSFORM }))}>
        {t("editor.resetPlacement")}
      </Button>
    </div>
  </fieldset>

  <fieldset class="editor__group">
    <legend>{t("editor.background")}</legend>
    <SegmentedControl
      aria-label={t("editor.backgroundKind")}
      value={scene.background.kind}
      options={backgroundOptions}
      {disabled}
      onchange={setBackgroundKind}
    />
    {#if scene.background.kind === "transparent"}
      <p class="editor__hint">{t("editor.transparentHint")}</p>
    {:else if scene.background.kind === "color"}
      <ColorPicker
        label={t("editor.color")}
        value={scene.background.color}
        {disabled}
        onchange={(color) => edit((s) => (s.background = { kind: "color", color }))}
      />
    {:else if scene.background.kind === "chroma"}
      {@const key = scene.background.color}
      <SegmentedControl
        label={t("editor.chromaColor")}
        value={chromaOptions.some((o) => o.value === key) ? key : ""}
        options={chromaOptions}
        {disabled}
        onchange={(color) => edit((s) => (s.background = { kind: "chroma", color }))}
      />
      <ColorPicker
        label={t("editor.color")}
        value={scene.background.color}
        {disabled}
        onchange={(color) => edit((s) => (s.background = { kind: "chroma", color }))}
      />
      <p class="editor__hint">{t("editor.chromaHint")}</p>
    {:else}
      <div class="editor__file">
        <span class="editor__path">{fileName(scene.background.path)}</span>
        <Button small {disabled} onclick={chooseImage}>{t("editor.chooseImage")}</Button>
      </div>
    {/if}
  </fieldset>

  <fieldset class="editor__group">
    <legend>{t("editor.lighting")}</legend>
    {#if usesLighting === false}
      <p class="editor__hint">{t("editor.noLighting")}</p>
    {/if}
    <LightingEditor value={lighting} {disabled} onchange={(l) => edit((s) => (s.lighting = l))} />
  </fieldset>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-4);
  }

  .editor__field {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  .editor__field > label,
  .editor__label {
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }

  .editor__input {
    height: var(--ugoe-control-height);
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-bg);
    color: var(--ugoe-color-text);
    font: inherit;
  }

  .editor__input:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 2px;
  }

  .editor__file,
  .editor__modules li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2);
  }

  .editor__path {
    flex: 1;
    min-width: 6rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor__modules {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    margin: 0;
    padding-inline-start: var(--ugoe-space-5);
  }

  .editor__row-actions {
    display: flex;
    gap: var(--ugoe-space-1);
  }

  .editor__group {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    margin: 0;
    padding: var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
  }

  .editor__group legend {
    padding: 0 var(--ugoe-space-1);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .editor__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
