<!--
  ColorPicker: 光源などの色を選ぶ (docs/ui.md#部品の一覧)。自作。
  トリガーは色の見本と 16 進の値を出すボタンで、押すと Popover の中に、OS の色の選択 (<input type="color">)・
  16 進の欄・R・G・B の NumberField を出す。色は `#rrggbb` (小文字) で受け渡す。
-->
<script lang="ts">
import { hexToRgb, parseHexColor, type Rgb, rgbToHex } from "../color";
import { t } from "../i18n";
import type { ButtonBaseProps } from "../types";
import Button from "./Button.svelte";
import NumberField from "./NumberField.svelte";
import Popover from "./Popover.svelte";

interface Props {
  /** 色 (`#rrggbb`)。 */
  value?: string;
  /** 見えるラベル。`aria-labelledby` か `aria-label` を渡すときは省ける。 */
  label?: string;
  /** 見えるラベルを別の要素に置くときの、その要素の id。 */
  "aria-labelledby"?: string;
  /** 見えるラベルを置けないときの読み上げの名前。 */
  "aria-label"?: string;
  disabled?: boolean;
  /** 選ぶ枠 (Popover) が開いているか。 */
  open?: boolean;
  /** 色が変わるたび (OS の色の選択を動かしている間も)。 */
  onchange?: (value: string) => void;
}

let {
  value = $bindable("#000000"),
  label,
  "aria-labelledby": labelledby,
  "aria-label": ariaLabel,
  disabled = false,
  open = $bindable(false),
  onchange,
}: Props = $props();

const id = $props.id();
const color = $derived(parseHexColor(value) ?? "#000000");
const rgb = $derived(hexToRgb(color));
/** 16 進の欄の文字 (編集中は値に追従させない)。 */
let hexText = $state("");
let editingHex = $state(false);

$effect(() => {
  const shown = color;
  if (!editingHex) hexText = shown;
});

function set(next: string) {
  const normalized = parseHexColor(next);
  if (!normalized || normalized === color) return;
  value = normalized;
  onchange?.(normalized);
}

function setChannel(channel: keyof Rgb, v: number) {
  set(rgbToHex({ ...rgb, [channel]: v }));
}

function commitHex() {
  const parsed = parseHexColor(hexText);
  if (parsed) set(parsed);
  hexText = parsed ?? color;
}

const channels = $derived([
  { key: "r" as const, label: t("colorPicker.red.label") },
  { key: "g" as const, label: t("colorPicker.green.label") },
  { key: "b" as const, label: t("colorPicker.blue.label") },
]);
</script>

<span class="ugoe-color-picker" data-disabled={disabled ? "" : undefined}>
  {#if label}
    <span class="ugoe-color-picker__label" id={`${id}-label`}>{label}</span>
  {/if}
  <Popover label={label ?? ariaLabel ?? t("colorPicker.panel.aria")} {disabled} bind:open>
    {#snippet trigger(attrs)}
      <Button
        {...attrs as ButtonBaseProps}
        class="ugoe-color-picker__trigger"
        {disabled}
        aria-labelledby={labelledby ?? (label ? `${id}-label ${id}-value` : undefined)}
        aria-label={ariaLabel ? `${ariaLabel} ${color}` : undefined}
      >
        <span class="ugoe-color-picker__swatch" style:background={color}></span>
        <span class="ugoe-color-picker__value" id={`${id}-value`}>{color}</span>
      </Button>
    {/snippet}
    <div class="ugoe-color-picker__panel">
      <label class="ugoe-color-picker__native">
        <span>{t("colorPicker.system.label")}</span>
        <input type="color" value={color} oninput={(e) => set(e.currentTarget.value)} />
      </label>
      <label class="ugoe-color-picker__hex">
        <span>{t("colorPicker.hex.label")}</span>
        <input
          type="text"
          size="8"
          autocomplete="off"
          spellcheck="false"
          bind:value={hexText}
          onfocus={() => (editingHex = true)}
          onblur={() => {
            editingHex = false;
            commitHex();
          }}
          onkeydown={(e) => {
            if (e.key === "Enter") commitHex();
            else if (e.key === "Escape" && hexText !== color) {
              // 確定前の値に戻す。戻すものが無ければ Esc で Popover を閉じる
              e.preventDefault();
              e.stopPropagation();
              hexText = color;
            }
          }}
        />
      </label>
      <div class="ugoe-color-picker__channels">
        {#each channels as channel (channel.key)}
          <NumberField
            label={channel.label}
            value={rgb[channel.key]}
            min={0}
            max={255}
            step={1}
            size={3}
            onchange={(v) => setChannel(channel.key, v)}
          />
        {/each}
      </div>
    </div>
  </Popover>
</span>

<style>
  :global {
    .ugoe-color-picker {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-color-picker[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-button.ugoe-color-picker__trigger {
      padding-left: var(--ugoe-space-2);
    }

    .ugoe-color-picker__swatch {
      width: 1.15em;
      height: 1.15em;
      box-sizing: border-box;
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-sm);
    }

    .ugoe-color-picker__value {
      font-family: var(--ugoe-font-mono);
      font-variant-numeric: tabular-nums;
    }

    .ugoe-color-picker__panel {
      display: flex;
      flex-direction: column;
      gap: var(--ugoe-space-3);
    }

    .ugoe-color-picker__native,
    .ugoe-color-picker__hex {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--ugoe-space-3);
    }

    .ugoe-color-picker__native input {
      width: 64px;
      height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 2px;
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
      cursor: pointer;
    }

    .ugoe-color-picker__hex input {
      box-sizing: border-box;
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-2);
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
      color: inherit;
      font-family: var(--ugoe-font-mono);
      font-size: var(--ugoe-font-size-md);
    }

    .ugoe-color-picker__native input:focus-visible,
    .ugoe-color-picker__hex input:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-color-picker__channels {
      display: flex;
      flex-wrap: wrap;
      gap: var(--ugoe-space-3);
    }
  }
</style>
