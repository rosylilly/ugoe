<!--
  部品の見本の画面。すべての部品を、URL の引数 (またはこの画面の上の設定) のテーマ・密度・言語で並べる。
-->
<script lang="ts">
import { setLocale } from "@ugoe/i18n";
import {
  Button,
  Checkbox,
  IconButton,
  NumberField,
  Section,
  SegmentedControl,
  Select,
  Slider,
  Tabs,
  Toggle,
  Toolbar,
} from "../src/index";
import { type Lang, strings } from "./strings";

type Theme = "system" | "light" | "dark";
type Density = "compact" | "comfortable";

const params = new URLSearchParams(location.search);
const pick = <T extends string>(name: string, allowed: readonly T[], fallback: T): T => {
  const v = params.get(name);
  return allowed.includes(v as T) ? (v as T) : fallback;
};

let theme = $state<Theme>(pick("theme", ["system", "light", "dark"] as const, "system"));
let density = $state<Density>(pick("density", ["compact", "comfortable"] as const, "compact"));
let lang = $state<Lang>(pick("lang", ["en", "ja"] as const, "en"));
const s = $derived(strings[lang]);

$effect(() => {
  const root = document.documentElement;
  if (theme === "system") delete root.dataset.theme;
  else root.dataset.theme = theme;
  root.dataset.density = density;
  // 部品の既定の文言 (リセットの名前など) は @ugoe/i18n の言語に従う
  setLocale(lang);
  root.lang = lang;
  document.title = s.title;
});

// 部品の状態
let loop = $state(true);
let mirrorPreview = $state(false);
let autoBlink = $state(true);
let smoothing = $state(0.5);
let angle = $state(12.5);
let scale = $state(1);
let camera = $state<string>("1");
let fit = $state<string>("0");
let tab = $state<string>("0");
let mirrored = $state(true);

const cameraOptions = $derived(
  s.cameras.map((label, i) => ({ value: String(i), label, disabled: i === 3 })),
);
const fitOptions = $derived(s.fits.map((label, i) => ({ value: String(i), label })));
const tabOptions = $derived(s.tabNames.map((label, i) => ({ value: String(i), label })));

const swatches = [
  "bg",
  "surface",
  "sunken",
  "border",
  "control-border",
  "text",
  "text-muted",
  "accent",
  "danger",
  "warning",
  "success",
  "danger-soft",
  "warning-soft",
  "success-soft",
];
</script>

{#snippet undoIcon()}
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M6 4L3 7l3 3M3 7h6a4 4 0 0 1 0 8H7" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{/snippet}

{#snippet redoIcon()}
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M10 4l3 3-3 3M13 7H7a4 4 0 0 0 0 8h2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{/snippet}

{#snippet mirrorIcon()}
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M8 2v12M6 4L2 12h4zM10 4l4 8h-4z" stroke-linejoin="round" />
  </svg>
{/snippet}

{#snippet trashIcon()}
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M3 4h10M6 4V2.5h4V4M4.5 4l.5 9.5h6l.5-9.5" stroke-linejoin="round" />
  </svg>
{/snippet}

<main class="gallery">
  <header class="gallery__header">
    <h1>{s.title}</h1>
    <div class="gallery__settings" role="group" aria-label={s.settings}>
      <SegmentedControl
        label={s.theme}
        bind:value={theme}
        options={[
          { value: "system", label: s.themeSystem },
          { value: "light", label: s.themeLight },
          { value: "dark", label: s.themeDark },
        ]}
      />
      <SegmentedControl
        label={s.density}
        bind:value={density}
        options={[
          { value: "compact", label: s.compact },
          { value: "comfortable", label: s.comfortable },
        ]}
      />
      <SegmentedControl
        label={s.language}
        bind:value={lang}
        options={[
          { value: "en", label: "English" },
          { value: "ja", label: "日本語" },
        ]}
      />
    </div>
  </header>

  <div class="gallery__grid">
    <section class="gallery__card">
      <h2>{s.buttons}</h2>
      <div class="gallery__row">
        <Button variant="primary">{s.save}</Button>
        <Button>{s.cancel}</Button>
        <Button variant="danger">{s.forget}</Button>
        <Button variant="ghost">{s.resetView}</Button>
      </div>
      <div class="gallery__row">
        <Button small>{s.small}</Button>
        <Button disabled>{s.disabled}</Button>
        <Button pressed={mirrored} onclick={() => (mirrored = !mirrored)}>{s.mirror}</Button>
        <IconButton label={s.undo}>{@render undoIcon()}</IconButton>
        <IconButton label={s.deleteItem} variant="danger">{@render trashIcon()}</IconButton>
        <IconButton label={s.mirror} pressed={mirrored} onclick={() => (mirrored = !mirrored)}>
          {@render mirrorIcon()}
        </IconButton>
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.toolbar}</h2>
      <Toolbar label={s.tools}>
        <IconButton label={s.undo}>{@render undoIcon()}</IconButton>
        <IconButton label={s.redo} disabled>{@render redoIcon()}</IconButton>
        <IconButton label={s.mirror} pressed={mirrored} onclick={() => (mirrored = !mirrored)}>
          {@render mirrorIcon()}
        </IconButton>
        <Button>{s.resetView}</Button>
      </Toolbar>
    </section>

    <section class="gallery__card">
      <h2>{s.choices}</h2>
      <div class="gallery__column">
        <Checkbox label={s.loop} bind:checked={loop} />
        <Checkbox label={s.selectAll} indeterminate />
        <Checkbox label={s.disabled} disabled />
        <Toggle label={s.mirrorPreview} bind:checked={mirrorPreview} />
        <Toggle label={s.autoBlink} bind:checked={autoBlink} />
        <Toggle label={s.disabled} disabled />
        <Select label={s.camera} options={cameraOptions} bind:value={camera} />
        <Select ariaLabel={s.camera} options={cameraOptions} placeholder={s.chooseCamera} />
        <SegmentedControl label={s.fit} options={fitOptions} bind:value={fit} />
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.numbers}</h2>
      <div class="gallery__column">
        <Slider
          label={s.smoothing}
          bind:value={smoothing}
          min={0}
          max={1}
          step={0.05}
          defaultValue={0.5}
        />
        <Slider
          label={s.angle}
          bind:value={angle}
          min={-30}
          max={30}
          step={0.5}
          unit={s.degrees}
          defaultValue={0}
        />
        <Slider label={s.disabled} value={3} min={0} max={10} disabled />
        <NumberField label={s.scale} bind:value={scale} min={0.1} max={10} step={0.1} />
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.tabs}</h2>
      <Tabs tabs={tabOptions} ariaLabel={s.panels} bind:value={tab}>
        {#snippet panel(value)}
          <p>{s.panelText(s.tabNames[Number(value)] ?? "")}</p>
        {/snippet}
      </Tabs>
    </section>

    <section class="gallery__card">
      <h2>{s.sections}</h2>
      <Section title={s.lighting}>
        {#snippet actions()}
          <IconButton label={s.resetView} small>{@render undoIcon()}</IconButton>
        {/snippet}
        <p>{s.lightingText}</p>
      </Section>
      <Section title={s.closedSection} open={false}>
        <p>{s.hiddenText}</p>
      </Section>
    </section>

    <section class="gallery__card gallery__card--wide">
      <h2>{s.tokens}</h2>
      <ul class="gallery__swatches">
        {#each swatches as name (name)}
          <li>
            <span class="gallery__swatch" style:background={`var(--ugoe-color-${name})`}></span>
            <code>{name}</code>
          </li>
        {/each}
      </ul>
    </section>
  </div>
</main>

<style>
  /* 見本の画面だけの配置。部品の見た目は部品とトークンが決める */
  :global(body) {
    margin: 0;
    background: var(--ugoe-color-bg);
    color: var(--ugoe-color-text);
    font-family: var(--ugoe-font-sans);
    font-size: var(--ugoe-font-size-md);
    line-height: var(--ugoe-line-height);
  }

  .gallery {
    padding: var(--ugoe-space-5);
  }

  .gallery__header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-4);
    margin-bottom: var(--ugoe-space-5);
  }

  h1 {
    margin: 0;
    font-size: var(--ugoe-font-size-xl);
  }

  h2 {
    margin: 0 0 var(--ugoe-space-3);
    font-size: var(--ugoe-font-size-lg);
  }

  .gallery__settings {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-4);
  }

  .gallery__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 340px), 1fr));
    gap: var(--ugoe-space-4);
  }

  .gallery__card {
    padding: var(--ugoe-space-4);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-lg);
    background: var(--ugoe-color-surface);
  }

  .gallery__card--wide {
    grid-column: 1 / -1;
  }

  .gallery__row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2);
    margin-bottom: var(--ugoe-space-3);
  }

  .gallery__column {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--ugoe-space-3);
  }

  .gallery__column > :global(.ugoe-slider) {
    align-self: stretch;
  }

  .gallery__card p {
    margin: 0;
  }

  .gallery__swatches {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: var(--ugoe-space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .gallery__swatches li {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }

  .gallery__swatch {
    width: 24px;
    height: 24px;
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-sm);
  }

  code {
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
