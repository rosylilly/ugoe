<!--
  モーションの設定 (docs/studio.md#操作ウィンドウ のパネル、T0803): 待機モーション・トラッキング中のモーションの優先と、
  トラッキングのままにするパラメータ。配信中には触らないので、表情とモーションの操作 (MotionPanel.svelte) と分けて右の設定の列に置く (T0818)。
  設定はプロファイルに持ち、送り終えるまで送った設定を画面に出す (lib/pendingEdit.svelte.ts)。
-->
<script lang="ts">
import { groupTrackingParameters } from "@ugoe/player";
import { Checkbox, Select, Toggle } from "@ugoe/ui";
import { t } from "./lib/i18n";
import type { AnimationView, MotionSettings } from "./lib/inputs";
import { PendingEdit } from "./lib/pendingEdit.svelte";

interface Props {
  animation: AnimationView;
  settings: MotionSettings;
  disabled?: boolean;
  /** 設定を送る (送り終えるまで、送った設定を画面に出し、次の変更の基にする)。 */
  onsettings: (settings: MotionSettings) => Promise<unknown>;
}

let { animation, settings, disabled = false, onsettings }: Props = $props();

/** 待機モーションの選択肢の「名前で選ぶ」の値 (モーションの名前とぶつからない)。 */
const AUTO = "\u0000auto";

const idleOptions = $derived([
  { value: AUTO, label: t("motion.idleAuto") },
  ...animation.motions.map((m) => ({ value: m.name, label: m.name })),
]);
const groups = $derived(groupTrackingParameters(animation.trackingParameters));

/** 送っている途中の設定 (プロファイルの通知を待たずに、続けて変えた変更を積み重ねる)。 */
const edit = new PendingEdit<MotionSettings>((next) => onsettings(next));
/** 画面に出す設定。 */
const current = $derived(edit.current(settings));

function change(patch: Partial<MotionSettings>) {
  void edit.submit({ ...$state.snapshot(current), ...patch } as MotionSettings);
}

/** まとまりのパラメータを、トラッキングのままにする・しない。 */
function keepGroup(ids: string[], keep: boolean) {
  const rest = current.trackingKept.filter((id) => !ids.includes(id));
  change({ trackingKept: keep ? [...rest, ...ids] : rest });
}

function groupState(ids: string[]): { checked: boolean; indeterminate: boolean } {
  const kept = ids.filter((id) => current.trackingKept.includes(id)).length;
  return { checked: kept === ids.length, indeterminate: kept > 0 && kept < ids.length };
}
</script>

<div class="motion-settings">
  <div class="motion-settings__group">
    <Toggle
      label={t("motion.autoIdle")}
      checked={current.autoIdle}
      {disabled}
      onchange={(v) => change({ autoIdle: v })}
    />
    {#if animation.motions.length > 0}
      <Select
        label={t("motion.idleMotion")}
        value={current.idleMotion ?? AUTO}
        options={idleOptions}
        disabled={disabled || !current.autoIdle}
        onchange={(v) => change({ idleMotion: v === AUTO ? null : v })}
      />
    {/if}
    <p class="motion-settings__hint">
      {animation.idleMotion === null
        ? t("motion.noIdle")
        : t("motion.idleIs", { name: animation.idleMotion })}
    </p>
  </div>

  <div class="motion-settings__group">
    <Toggle
      label={t("motion.overTracking")}
      checked={current.overTracking}
      {disabled}
      onchange={(v) => change({ overTracking: v })}
    />
    <p class="motion-settings__hint">{t("motion.overTrackingHint")}</p>
    {#if current.overTracking && groups.length > 0}
      <span class="motion-settings__label">{t("motion.keptHeading")}</span>
      <div class="motion-settings__checks">
        {#each groups as group (group.group)}
          {@const state = groupState(group.ids)}
          <Checkbox
            label={group.label}
            checked={state.checked}
            indeterminate={state.indeterminate}
            {disabled}
            onchange={(v) => keepGroup(group.ids, v)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .motion-settings {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-4);
  }

  .motion-settings__group {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  .motion-settings__label,
  .motion-settings__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .motion-settings__checks {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
  }
</style>
