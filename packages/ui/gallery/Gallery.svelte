<!--
  部品の見本の画面。すべての部品を、URL の引数 (またはこの画面の上の設定) のテーマ・密度・言語で並べる。
-->
<script lang="ts">
import { formatList, setLocale } from "@ugoe/i18n";
import { onMount } from "svelte";
import {
  AlertDialog,
  Button,
  Checkbox,
  ColorPicker,
  ContextMenu,
  Dialog,
  DropZone,
  IconButton,
  Kbd,
  Menu,
  type MenuEntry,
  Notice,
  NumberField,
  SecretField,
  Section,
  SegmentedControl,
  Select,
  Slider,
  showToast,
  Tabs,
  TextArea,
  TextField,
  Toaster,
  Toggle,
  Toolbar,
  TooltipProvider,
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
// スクリーンショットで重ねて出す部品を撮るため、URL の `open` で 1 つを開いた状態にする
const initialOpen = pick(
  "open",
  ["none", "dialog", "alert", "menu", "popover", "toast", "tooltip"] as const,
  "none",
);
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
let layerName = $state("Hair front");
let layerId = $state("Layer3");
let filterText = $state("");
let fadeIn = $state<number | null>(null);
let authors = $state("Ann\nBob");
let fit = $state<string>("0");
let tab = $state<string>("0");
let mirrored = $state(true);
let renameOpen = $state(initialOpen === "dialog");
let forgetOpen = $state(initialOpen === "alert");
let fileMenuOpen = $state(initialOpen === "menu");
let colorOpen = $state(initialOpen === "popover");
let opacity = $state(1);
let autosave = $state(true);
let lightColor = $state("#ffcc88");
let rimColor = $state("#6d28d9");
let dropResult = $state("");
let noticeVisible = $state(true);

const fileEntries = $derived<MenuEntry[]>([
  { label: s.open, description: s.openDescription, shortcut: "Mod+O", onselect: () => {} },
  {
    label: s.save,
    shortcut: "Mod+S",
    onselect: () => showToast(s.toastSaved, { tone: "success" }),
  },
  { label: s.saveAs, shortcut: "Mod+Shift+S", onselect: () => {}, disabled: true },
  { kind: "separator" },
  {
    kind: "submenu",
    label: s.export,
    entries: [
      { label: s.exportPng, onselect: () => {} },
      { label: s.exportBundle, onselect: () => {} },
    ],
  },
  { kind: "checkbox", label: s.autosave, checked: autosave, onchange: (v) => (autosave = v) },
  { kind: "separator" },
  { label: s.close, shortcut: "Mod+W", onselect: () => {} },
]);
const itemEntries = $derived<MenuEntry[]>([
  { label: s.duplicate, shortcut: "Mod+D", onselect: () => {} },
  { label: s.renameLayer, onselect: () => (renameOpen = true) },
  { kind: "separator" },
  { label: s.deleteItem, shortcut: "Delete", danger: true, onselect: () => {} },
]);

onMount(() => {
  if (initialOpen === "toast") {
    showToast(strings[lang].toastSaved, { tone: "success", duration: 0 });
    showToast(strings[lang].toastWarning, { tone: "warning", duration: 0 });
  } else if (initialOpen === "tooltip") {
    document.getElementById("gallery-tooltip-target")?.focus();
  }
});

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
  "accent-soft",
  "danger-soft",
  "warning-soft",
  "success-soft",
  "hover",
  "active",
];
</script>

{#snippet undoIcon()}
  <svg class="ugoe-icon" viewBox="0 0 16 16">
    <path d="M6 4L3 7l3 3M3 7h6a4 4 0 0 1 0 8H7" />
  </svg>
{/snippet}

{#snippet redoIcon()}
  <svg class="ugoe-icon" viewBox="0 0 16 16">
    <path d="M10 4l3 3-3 3M13 7H7a4 4 0 0 0 0 8h2" />
  </svg>
{/snippet}

{#snippet mirrorIcon()}
  <svg class="ugoe-icon" viewBox="0 0 16 16">
    <path d="M8 2v12M6 4L2 12h4zM10 4l4 8h-4z" />
  </svg>
{/snippet}

{#snippet moreIcon()}
  <svg class="ugoe-icon ugoe-icon--fill" viewBox="0 0 16 16">
    <circle cx="3.5" cy="8" r="1.25" /><circle cx="8" cy="8" r="1.25" /><circle cx="12.5" cy="8" r="1.25" />
  </svg>
{/snippet}

{#snippet trashIcon()}
  <svg class="ugoe-icon" viewBox="0 0 16 16">
    <path d="M3 4h10M6 4V2.5h4V4M4.5 4l.5 9.5h6l.5-9.5" />
  </svg>
{/snippet}

<TooltipProvider>
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
        <Button variant="primary" busy>{s.saving}</Button>
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
        <Select aria-label={s.camera} options={cameraOptions} placeholder={s.chooseCamera} />
        <Select
          label={s.frame}
          options={cameraOptions}
          value="2"
          invalid
          aria-describedby="gallery-frame-error"
        />
        <p id="gallery-frame-error" class="gallery__error">{s.frameError}</p>
        <SegmentedControl label={s.fit} options={fitOptions} bind:value={fit} />
        <!-- 幅が足りないと、名前も選択肢も字の途中で折らずに、選択肢ごと次の行へ送る (T0950) -->
        <div class="gallery__narrow">
          <SegmentedControl label={s.fitNarrow} options={fitOptions} bind:value={fit} />
        </div>
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
          tooltip={s.smoothingTooltip}
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
        <NumberField label={s.scale} value={12} step={0.1} invalid aria-describedby="gallery-scale-error" />
        <p id="gallery-scale-error" class="gallery__error">{s.scaleError}</p>
        <NumberField label={s.fadeIn} optional bind:value={fadeIn} emptyValue={0.5} min={0} step={0.1} unit={s.seconds} />
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.textFields}</h2>
      <div class="gallery__column">
        <TextField label={s.layerName} hint={s.layerNameHint} bind:value={layerName} />
        <TextField label={s.layerId} mono bind:value={layerId} error={s.layerIdError} />
        <TextField type="search" aria-label={s.filter} placeholder={s.filterPlaceholder} bind:value={filterText} />
        <TextField label={s.disabled} value={layerName} disabled />
        <TextArea label={s.authors} hint={s.authorsHint} rows={2} spellcheck={false} bind:value={authors} />
        <SecretField label={s.apiKey} placeholder={s.apiKeyPlaceholder} />
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.tabs}</h2>
      <Tabs tabs={tabOptions} aria-label={s.panels} bind:value={tab}>
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

    <section class="gallery__card">
      <h2>{s.overlays}</h2>
      <div class="gallery__row">
        <Button onclick={() => (renameOpen = true)}>{s.renameLayer}</Button>
        <Button variant="danger" onclick={() => (forgetOpen = true)}>{s.forgetAll}</Button>
      </div>
      <div class="gallery__row">
        <Button onclick={() => showToast(s.toastSaved, { tone: "success" })}>{s.showToast}</Button>
        <Button onclick={() => showToast(s.toastWarning, { tone: "warning" })}>
          {s.showWarningToast}
        </Button>
      </div>
      <Dialog bind:open={renameOpen} title={s.renameTitle} description={s.renameDescription}>
        <NumberField label={s.opacity} bind:value={opacity} min={0} max={1} step={0.05} />
        {#snippet actions()}
          <Button onclick={() => (renameOpen = false)}>{s.cancel}</Button>
          <Button variant="primary" onclick={() => (renameOpen = false)}>{s.rename}</Button>
        {/snippet}
      </Dialog>
      <AlertDialog
        bind:open={forgetOpen}
        title={s.forgetAllTitle}
        description={s.forgetAllDescription}
        confirmLabel={s.forgetAllConfirm}
        onconfirm={() => {}}
      />
    </section>

    <section class="gallery__card">
      <h2>{s.menus}</h2>
      <div class="gallery__row">
        <Menu label={s.file} entries={fileEntries} bind:open={fileMenuOpen} />
        <Menu label={s.moreActions} entries={itemEntries} icon={moreIcon} align="end" />
        <IconButton id="gallery-tooltip-target" label={s.undo} shortcut="Mod+Z">
          {@render undoIcon()}
        </IconButton>
      </div>
      <ContextMenu entries={itemEntries} label={s.moreActions} class="gallery__context">
        <p>{s.contextArea}</p>
      </ContextMenu>
    </section>

    <section class="gallery__card gallery__card--wide">
      <h2>{s.notices}</h2>
      <div class="gallery__column gallery__column--stretch">
        <Notice tone="info">{s.noticeInfo}</Notice>
        <Notice tone="success">{s.noticeSuccess}</Notice>
        {#if noticeVisible}
          <Notice tone="warning" title={s.noticeWarningTitle} ondismiss={() => (noticeVisible = false)}>
            {s.noticeWarning}
            {#snippet actions()}<Button small>{s.reload}</Button>{/snippet}
          </Notice>
        {/if}
        <Notice tone="danger" title={s.noticeDangerTitle}>{s.noticeDanger}</Notice>
        <!-- position を持たない送る枠の中の Notice (見せずに読ませる文字が枠の外を位置の基準にしないこと。main.ts で測る。T0954) -->
        <div class="gallery__scroller">
          <p class="gallery__spacer">{s.noticeScroller}</p>
          <Notice tone="info">{s.noticeInfo}</Notice>
        </div>
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.colors}</h2>
      <div class="gallery__column gallery__column--stretch">
        <ColorPicker label={s.lightColor} bind:value={lightColor} bind:open={colorOpen} />
        <ColorPicker label={s.rimColor} bind:value={rimColor} />
        <DropZone
          accept=".ugoe,.psd"
          multiple
          onfiles={(files) => (dropResult = s.dropped(formatList(files.map((f) => f.name))))}
          onreject={(files) => (dropResult = s.rejected(formatList(files.map((f) => f.name))))}
        >
          {s.dropHint}
        </DropZone>
        {#if dropResult}<p role="status">{dropResult}</p>{/if}
      </div>
    </section>

    <section class="gallery__card">
      <h2>{s.shortcuts}</h2>
      <dl class="gallery__shortcuts">
        {#each s.shortcutList as [keys, action] (keys)}
          <dt><Kbd {keys} /></dt>
          <dd>{action}</dd>
        {/each}
        <dt><Kbd keys="Mod+Shift+Z" platform="mac" /></dt>
        <dd>macOS</dd>
        <dt><Kbd keys="Mod+Shift+Z" platform="other" /></dt>
        <dd>Windows / Linux</dd>
      </dl>
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
<Toaster />
</TooltipProvider>

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

  .gallery__narrow {
    width: 7rem;
    padding: var(--ugoe-space-1);
    border: 1px dashed var(--ugoe-color-border);
  }

  .gallery__scroller {
    height: 6rem;
    overflow: auto;
    border: 1px dashed var(--ugoe-color-border);
  }

  .gallery__spacer {
    height: 12rem;
    margin: 0;
    padding: var(--ugoe-space-1);
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

  .gallery__column--stretch {
    align-items: stretch;
  }

  .gallery__column--stretch > :global(.ugoe-color-picker) {
    align-self: flex-start;
  }

  :global(.gallery__context) {
    display: block;
    margin-top: var(--ugoe-space-3);
    padding: var(--ugoe-space-4);
    border: 1px dashed var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-md);
    color: var(--ugoe-color-text-muted);
  }

  .gallery__shortcuts {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
    margin: 0;
  }

  .gallery__shortcuts dd {
    margin: 0;
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

  .gallery__error {
    margin: 0;
    color: var(--ugoe-color-danger);
    font-size: var(--ugoe-font-size-sm);
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
