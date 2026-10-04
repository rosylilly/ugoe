<!--
  表情とモーション (docs/studio.md#操作ウィンドウ のパネル、T0803): 表情の付け外し・モーションの再生 (優先度)・待機モーション・
  トラッキング中のモーションの優先。操作は指揮役に送り、様子は `animation` の通知で届く (操作ウィンドウはモデルを描かない)。
  規則はビューアの再生の層 (@ugoe/player、T0720〜T0722) と同じ。
-->
<script lang="ts">
import {
  describeMotion,
  groupTrackingParameters,
  MOTION_PRIORITIES,
  type MotionPriority,
  motionTrackingEffect,
} from "@ugoe/player";
import { Button, Checkbox, SegmentedControl, Select, Toggle } from "@ugoe/ui";
import { t } from "./lib/i18n";
import type { AnimationView, MotionSettings } from "./lib/inputs";
import { PendingEdit } from "./lib/pendingEdit.svelte";

interface Props {
  animation: AnimationView;
  settings: MotionSettings;
  disabled?: boolean;
  onplay: (name: string, priority: MotionPriority) => void;
  onstop: () => void;
  onexpression: (op: "set" | "add" | "remove" | "clear", name?: string) => void;
  /** 設定を送る (送り終えるまで、送った設定を画面に出し、次の変更の基にする)。 */
  onsettings: (settings: MotionSettings) => Promise<unknown>;
}

let {
  animation,
  settings,
  disabled = false,
  onplay,
  onstop,
  onexpression,
  onsettings,
}: Props = $props();

let priority = $state<MotionPriority>("normal");

/** 待機モーションの選択肢の「名前で選ぶ」の値 (モーションの名前とぶつからない)。 */
const AUTO = "\u0000auto";

const tracked = $derived(new Set(animation.trackingParameters));
const priorityOptions = $derived(
  MOTION_PRIORITIES.map((p) => ({ value: p.value, label: p.label })),
);
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

function toggleExpression(name: string) {
  onexpression(animation.activeExpressions.includes(name) ? "remove" : "set", name);
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

<div class="motion">
  {#if animation.expressions.length === 0 && animation.motions.length === 0}
    <p class="motion__hint">{t("motion.empty")}</p>
  {/if}

  {#if animation.expressions.length > 0}
    <div class="motion__group">
      <div class="motion__heading">
        <span class="motion__label">{t("motion.expressions")}</span>
        <Button
          small
          disabled={disabled || animation.activeExpressions.length === 0}
          onclick={() => onexpression("clear")}>{t("motion.clearExpressions")}</Button
        >
      </div>
      <ul class="motion__list">
        {#each animation.expressions as name (name)}
          {@const active = animation.activeExpressions.includes(name)}
          <li>
            <Button small pressed={active} {disabled} onclick={() => toggleExpression(name)}>{name}</Button>
            <Button
              small
              variant="ghost"
              disabled={disabled || active}
              onclick={() => onexpression("add", name)}>{t("motion.addExpression")}</Button
            >
          </li>
        {/each}
      </ul>
      <p class="motion__hint">{t("motion.expressionsHint")}</p>
    </div>
  {/if}

  {#if animation.motions.length > 0}
    <div class="motion__group">
      <div class="motion__heading">
        <span class="motion__label">{t("motion.motions")}</span>
        <Button small disabled={disabled || animation.current === null} onclick={onstop}
          >{t("motion.stop")}</Button
        >
      </div>
      <SegmentedControl
        label={t("motion.priority")}
        value={priority}
        options={priorityOptions}
        {disabled}
        onchange={(v) => (priority = v)}
      />
      <ul class="motion__list motion__list--rows">
        {#each animation.motions as m (m.name)}
          {@const effect = animation.tracking ? motionTrackingEffect(m, tracked) : null}
          <li>
            <Button
              small
              pressed={animation.current?.name === m.name}
              {disabled}
              onclick={() => onplay(m.name, priority)}>{m.name}</Button
            >
            <span class="motion__meta">
              {describeMotion(m)}
              {#if effect !== null && effect.kind !== "none" && !current.overTracking}
                · {t(effect.kind === "all" ? "motion.trackingAll" : "motion.trackingSome")}
              {/if}
            </span>
          </li>
        {/each}
      </ul>
      <p class="motion__hint" aria-live="polite">
        {animation.current === null
          ? t("motion.nothingPlaying")
          : t("motion.playing", {
              name: animation.current.name,
              priority:
                MOTION_PRIORITIES.find((p) => p.value === animation.current?.priority)?.label ?? "",
            })}
      </p>
    </div>

    <div class="motion__group">
      <Toggle
        label={t("motion.autoIdle")}
        checked={current.autoIdle}
        {disabled}
        onchange={(v) => change({ autoIdle: v })}
      />
      <Select
        label={t("motion.idleMotion")}
        value={current.idleMotion ?? AUTO}
        options={idleOptions}
        disabled={disabled || !current.autoIdle}
        onchange={(v) => change({ idleMotion: v === AUTO ? null : v })}
      />
      <p class="motion__hint">
        {animation.idleMotion === null
          ? t("motion.noIdle")
          : t("motion.idleIs", { name: animation.idleMotion })}
      </p>
    </div>
  {/if}

  <div class="motion__group">
    <Toggle
      label={t("motion.overTracking")}
      checked={current.overTracking}
      {disabled}
      onchange={(v) => change({ overTracking: v })}
    />
    <p class="motion__hint">{t("motion.overTrackingHint")}</p>
    {#if current.overTracking && groups.length > 0}
      <span class="motion__label">{t("motion.keptHeading")}</span>
      <div class="motion__checks">
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
  .motion {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-4);
  }

  .motion__group {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  .motion__heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
  }

  .motion__label {
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }

  .motion__list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .motion__list li {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }

  .motion__list--rows {
    flex-direction: column;
  }

  .motion__meta,
  .motion__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .motion__checks {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
  }
</style>
