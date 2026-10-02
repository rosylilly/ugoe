<!--
  シーンの光源の設定 (@ugoe/player の `LightingSettings`。意味は docs/spec/rendering.md#ライティング)。
  ビューアの光源のパネルより簡単にし、組 (既定の光源の組)・環境光・平行光源・点光源の主な値だけを出す。
-->
<script lang="ts">
import { LIMITS, type LightingSettings, PRESETS, sameLighting } from "@ugoe/player";
import { ColorPicker, Select, Slider, Toggle } from "@ugoe/ui";
import { t } from "./lib/i18n";

interface Props {
  value: LightingSettings;
  disabled?: boolean;
  onchange: (value: LightingSettings) => void;
}

let { value, disabled = false, onchange }: Props = $props();

type PresetId = "standard" | "sunset" | "lamp" | "custom";

const PRESET_NAMES = {
  standard: "lighting.presets.standard",
  sunset: "lighting.presets.sunset",
  lamp: "lighting.presets.lamp",
  custom: "lighting.presets.custom",
} as const;

const preset = $derived<PresetId>(
  (PRESETS.find((p) => sameLighting(p.settings, value))?.id as PresetId | undefined) ?? "custom",
);
const presetOptions = $derived(
  (Object.keys(PRESET_NAMES) as PresetId[]).map((id) => ({
    value: id,
    label: t(PRESET_NAMES[id]),
    disabled: id === "custom",
  })),
);

/**
 * 写しを `change` で書き換えて渡す。何も変わらなければ渡さない (部品が、渡した値と同じ値で onchange を呼んでも
 * 送り返さない。送り合いが止まらなくならないように)。
 */
function edit(change: (next: LightingSettings) => void) {
  const current = $state.snapshot(value) as LightingSettings;
  const next = structuredClone(current);
  change(next);
  if (JSON.stringify(next) !== JSON.stringify(current)) onchange(next);
}

function choosePreset(id: PresetId) {
  const found = PRESETS.find((p) => p.id === id);
  if (found !== undefined) onchange(structuredClone(found.settings));
}
</script>

<div class="lighting">
  <Toggle
    label={t("lighting.enabled")}
    checked={value.enabled}
    {disabled}
    onchange={(on) => edit((s) => (s.enabled = on))}
  />
  {#if value.enabled}
    <Select
      label={t("lighting.preset")}
      value={preset}
      options={presetOptions}
      {disabled}
      onchange={choosePreset}
    />

    <fieldset class="lighting__group">
      <legend>{t("lighting.ambient")}</legend>
      <ColorPicker
        label={t("lighting.color")}
        value={value.ambient.color}
        {disabled}
        onchange={(c) => edit((s) => (s.ambient.color = c))}
      />
      <Slider
        label={t("lighting.intensity")}
        value={value.ambient.intensity}
        min={LIMITS.intensity[0]}
        max={LIMITS.intensity[1]}
        step={0.01}
        {disabled}
        onchange={(v) => edit((s) => (s.ambient.intensity = v))}
      />
    </fieldset>

    <fieldset class="lighting__group">
      <legend>{t("lighting.directional")}</legend>
      <Toggle
        label={t("lighting.on")}
        checked={value.directional.enabled}
        {disabled}
        onchange={(on) => edit((s) => (s.directional.enabled = on))}
      />
      {#if value.directional.enabled}
        <ColorPicker
          label={t("lighting.color")}
          value={value.directional.color}
          {disabled}
          onchange={(c) => edit((s) => (s.directional.color = c))}
        />
        <Slider
          label={t("lighting.intensity")}
          value={value.directional.intensity}
          min={LIMITS.intensity[0]}
          max={LIMITS.intensity[1]}
          step={0.01}
          {disabled}
          onchange={(v) => edit((s) => (s.directional.intensity = v))}
        />
        <Slider
          label={t("lighting.azimuth")}
          value={value.directional.azimuth}
          min={LIMITS.azimuth[0]}
          max={LIMITS.azimuth[1]}
          step={1}
          unit={t("editor.degrees")}
          {disabled}
          onchange={(v) => edit((s) => (s.directional.azimuth = v))}
        />
        <Slider
          label={t("lighting.elevation")}
          value={value.directional.elevation}
          min={LIMITS.elevation[0]}
          max={LIMITS.elevation[1]}
          step={1}
          unit={t("editor.degrees")}
          {disabled}
          onchange={(v) => edit((s) => (s.directional.elevation = v))}
        />
      {/if}
    </fieldset>

    <fieldset class="lighting__group">
      <legend>{t("lighting.point")}</legend>
      <Toggle
        label={t("lighting.on")}
        checked={value.point.enabled}
        {disabled}
        onchange={(on) => edit((s) => (s.point.enabled = on))}
      />
      {#if value.point.enabled}
        <ColorPicker
          label={t("lighting.color")}
          value={value.point.color}
          {disabled}
          onchange={(c) => edit((s) => (s.point.color = c))}
        />
        <Slider
          label={t("lighting.intensity")}
          value={value.point.intensity}
          min={LIMITS.intensity[0]}
          max={LIMITS.intensity[1]}
          step={0.01}
          {disabled}
          onchange={(v) => edit((s) => (s.point.intensity = v))}
        />
        <Slider
          label={t("lighting.x")}
          value={value.point.x}
          min={LIMITS.x[0]}
          max={LIMITS.x[1]}
          step={0.01}
          {disabled}
          onchange={(v) => edit((s) => (s.point.x = v))}
        />
        <Slider
          label={t("lighting.y")}
          value={value.point.y}
          min={LIMITS.y[0]}
          max={LIMITS.y[1]}
          step={0.01}
          {disabled}
          onchange={(v) => edit((s) => (s.point.y = v))}
        />
        <Slider
          label={t("lighting.range")}
          value={value.point.range}
          min={LIMITS.range[0]}
          max={LIMITS.range[1]}
          step={0.01}
          {disabled}
          onchange={(v) => edit((s) => (s.point.range = v))}
        />
      {/if}
    </fieldset>
  {/if}
</div>

<style>
  .lighting {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .lighting__group {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    margin: 0;
    padding: var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
  }

  .lighting__group legend {
    padding: 0 var(--ugoe-space-1);
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
