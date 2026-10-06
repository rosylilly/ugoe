<!--
  表情とモーション (docs/studio.md#操作ウィンドウ のパネル、T0803): 表情の付け外しとモーションの再生 (優先度)。
  操作は指揮役に送り、様子は `animation` の通知で届く (操作ウィンドウはモデルを描かない)。
  規則はビューアの再生の層 (@ugoe/player、T0720〜T0722) と同じ。待機モーションとトラッキング中の優先の設定は、配信中に触らないので
  MotionSettingsPanel.svelte (右の設定の列) に分けた (T0818)。
-->
<script lang="ts">
import {
  describeMotion,
  MOTION_PRIORITIES,
  type MotionPriority,
  motionTrackingEffect,
} from "@ugoe/player";
import { Button, SegmentedControl } from "@ugoe/ui";
import { t } from "./lib/i18n";
import type { AnimationView, MotionSettings } from "./lib/inputs";

interface Props {
  animation: AnimationView;
  /** モーションの設定 (トラッキング中の優先で、モーションの説明を変える)。 */
  settings: MotionSettings;
  disabled?: boolean;
  onplay: (name: string, priority: MotionPriority) => void;
  onstop: () => void;
  onexpression: (op: "set" | "add" | "remove" | "clear", name?: string) => void;
}

let { animation, settings, disabled = false, onplay, onstop, onexpression }: Props = $props();

let priority = $state<MotionPriority>("normal");

const tracked = $derived(new Set(animation.trackingParameters));
const priorityOptions = $derived(
  MOTION_PRIORITIES.map((p) => ({ value: p.value, label: p.label })),
);

function toggleExpression(name: string) {
  onexpression(animation.activeExpressions.includes(name) ? "remove" : "set", name);
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
              {#if effect !== null && effect.kind !== "none" && !settings.overTracking}
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
  {/if}
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
</style>
