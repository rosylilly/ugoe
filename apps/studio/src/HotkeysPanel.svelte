<!--
  ホットキー (docs/studio.md#ホットキー、T0803): 操作ごとのキーの割り当て。キーの欄を押してからキーを押すと割り当てる
  (押している間はホットキーの登録を外す)。割り当てはプロファイルに持ち、バックエンドがグローバルなショートカットとして登録する
  (アプリが前面に無くても効く)。登録できなかったもの (ぶつかり・OS が使っている) は、パネルの頭に数を、その行に印と理由を出す
  (ぶつかりは、相手の割り当てを操作の名前で示す。T0818)。削除した行は、パネルの頭の知らせの「元に戻す」で同じ位置に戻せる (T0845)。
-->
<script lang="ts">
import { MOTION_PRIORITIES, type MotionPriority } from "@ugoe/player";
import { Button, Icon, Kbd, Notice, SegmentedControl, Select } from "@ugoe/ui";
import { onDestroy, tick } from "svelte";
import { describeError } from "./lib/errors";
import {
  defaultAction,
  displayKeys,
  HOTKEY_KINDS,
  hasModifier,
  keysFromPress,
} from "./lib/hotkeys";
import { t } from "./lib/i18n";
import type { Hotkey, HotkeyAction, HotkeyKind, HotkeyProblem } from "./lib/inputs";
import { PendingEdit } from "./lib/pendingEdit.svelte";
import { fileName } from "./lib/profile";

interface Props {
  hotkeys: Hotkey[];
  /** 登録できなかった割り当ての位置と理由。 */
  problems: [number, HotkeyProblem][];
  wayland: boolean;
  /** シーンの ID と表示名。 */
  scenes: { id: string; label: string }[];
  expressions: string[];
  motions: string[];
  slots: string[];
  /** 配信しているモデルのポーズの要素のパーツ (T0842)。 */
  poses: string[];
  disabled?: boolean;
  /** 割り当てを送る (送り終えるまで、送った割り当てを画面に出し、次の変更の基にする)。 */
  onchange: (hotkeys: Hotkey[]) => Promise<unknown>;
  /** キーを押している間、ホットキーの登録を外す。 */
  onsuspend: (suspended: boolean) => void;
  /** モジュールのファイルを選ぶ (やめたら `null`)。 */
  onpickmodule: () => Promise<string | null>;
  /** モジュールが付くスロット (読めなければ例外)。 */
  moduleSlot: (path: string) => Promise<string>;
}

let {
  hotkeys,
  problems,
  wayland,
  scenes,
  expressions,
  motions,
  slots,
  poses,
  disabled = false,
  onchange,
  onsuspend,
  onpickmodule,
  moduleSlot,
}: Props = $props();

/** キーを待っている割り当ての位置。 */
let capturing = $state<number | null>(null);
let pickError = $state<string | null>(null);
/** 最後に削除した割り当てと、その位置 (知らせの「元に戻す」で戻す。T0845)。 */
let removed = $state<{ index: number; hotkey: Hotkey } | null>(null);
let root = $state<HTMLElement>();

const KIND_KEYS: Record<Exclude<HotkeyKind, "unknown">, string> = {
  scene: "hotkeys.kinds.scene",
  expression: "hotkeys.kinds.expression",
  clearExpressions: "hotkeys.kinds.clearExpressions",
  motion: "hotkeys.kinds.motion",
  stopMotions: "hotkeys.kinds.stopMotions",
  dressUp: "hotkeys.kinds.dressUp",
  pose: "hotkeys.kinds.pose",
  trackingPause: "hotkeys.kinds.trackingPause",
  trackingCalibrate: "hotkeys.kinds.trackingCalibrate",
  lipSyncPause: "hotkeys.kinds.lipSyncPause",
};

const kindOptions = $derived(
  HOTKEY_KINDS.map((kind) => ({
    value: kind,
    label: t(KIND_KEYS[kind as Exclude<HotkeyKind, "unknown">] as "hotkeys.kinds.scene"),
  })),
);
const priorityOptions = $derived(
  MOTION_PRIORITIES.map((p) => ({ value: p.value, label: p.label })),
);
const modeOptions = $derived([
  { value: "set" as const, label: t("hotkeys.expressionSet") },
  { value: "toggle" as const, label: t("hotkeys.expressionToggle") },
]);
const dressOptions = $derived([
  { value: "attach" as const, label: t("hotkeys.dressAttach") },
  { value: "remove" as const, label: t("hotkeys.dressRemove") },
]);
const problemAt = $derived(new Map(problems));

/** 送っている途中の割り当て (プロファイルの通知を待たずに、続けて変えた変更を積み重ねる)。 */
const edit = new PendingEdit<Hotkey[]>((next) => onchange(next));
/** 画面に出す割り当て。 */
const current = $derived(edit.current(hotkeys));

/** 選択肢 (今の値がモデルに無くても出す。別のモデルのシーン用の割り当てもあるため)。 */
function choices(list: readonly string[], current: string) {
  const all = current === "" || list.includes(current) ? list : [current, ...list];
  return all.map((value) => ({ value, label: value }));
}

function sceneChoices(current: string) {
  const known = scenes.some((s) => s.id === current);
  return [
    ...(known || current === "" ? [] : [{ value: current, label: t("hotkeys.missingScene") }]),
    ...scenes.map((s) => ({ value: s.id, label: s.label })),
  ];
}

function update(index: number, change: (hotkey: Hotkey) => void) {
  const next = structuredClone($state.snapshot(current)) as Hotkey[];
  const hotkey = next[index];
  if (hotkey === undefined) return;
  change(hotkey);
  void edit.submit(next);
}

function setAction(index: number, action: HotkeyAction) {
  update(index, (h) => (h.action = action));
}

function setKind(index: number, kind: HotkeyKind) {
  setAction(
    index,
    defaultAction(kind, { scenes: scenes.map((s) => s.id), expressions, motions, slots, poses }),
  );
}

function add() {
  const next = structuredClone($state.snapshot(current)) as Hotkey[];
  next.push({
    keys: "",
    action: defaultAction("scene", {
      scenes: scenes.map((s) => s.id),
      expressions,
      motions,
      slots,
      poses,
    }),
  });
  void edit.submit(next);
  startCapture(next.length - 1);
}

async function remove(index: number) {
  if (capturing !== null) stopCapture();
  const next = structuredClone($state.snapshot(current)) as Hotkey[];
  const [hotkey] = next.splice(index, 1);
  if (hotkey === undefined) return;
  removed = { index, hotkey };
  void edit.submit(next);
  // 押したボタンは行ごと消えるので、フォーカスを知らせの「元に戻す」へ移す (body へ落とさない)
  await tick();
  root?.querySelector<HTMLElement>(".hotkeys__undo")?.focus();
}

/** 削除した割り当てを、同じ位置 (後ろの行が減っていれば最後) に戻す。キーも操作も削除の前のまま。 */
async function undoRemove() {
  const last = removed;
  if (last === null) return;
  removed = null;
  const next = structuredClone($state.snapshot(current)) as Hotkey[];
  const index = Math.min(last.index, next.length);
  next.splice(index, 0, last.hotkey);
  void edit.submit(next);
  await tick();
  root?.querySelectorAll<HTMLElement>(".hotkeys__keys")[index]?.focus();
}

// --- キーの割り当て ---

function startCapture(index: number) {
  capturing = index;
  onsuspend(true);
}

function stopCapture() {
  capturing = null;
  onsuspend(false);
}

function onKeydown(event: KeyboardEvent) {
  const index = capturing;
  if (index === null) return;
  event.preventDefault();
  event.stopPropagation();
  const plain = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
  if (plain && event.code === "Escape") {
    stopCapture();
    return;
  }
  if (plain && (event.code === "Backspace" || event.code === "Delete")) {
    update(index, (h) => (h.keys = ""));
    stopCapture();
    return;
  }
  const keys = keysFromPress(event);
  if (keys === null) return;
  update(index, (h) => (h.keys = keys));
  stopCapture();
}

onDestroy(() => {
  if (capturing !== null) onsuspend(false);
});

// --- 着せ替え ---

async function pickModule(index: number) {
  pickError = null;
  const path = await onpickmodule();
  if (path === null) return;
  try {
    const slot = await moduleSlot(path);
    setAction(index, { kind: "dressUp", slot, module: path });
  } catch (e) {
    pickError = describeError(e);
  }
}

/** 割り当ての操作の名前 (「シーンの切り替え: 雑談」。ぶつかりの相手を示す)。 */
function describe(hotkey: Hotkey | undefined): string {
  if (hotkey === undefined) return "";
  const action = hotkey.action;
  if (action.kind === "unknown") return t("hotkeys.kinds.unknown");
  const kind = t(KIND_KEYS[action.kind] as "hotkeys.kinds.scene");
  let target: string | null = null;
  if (action.kind === "scene")
    target = scenes.find((s) => s.id === action.scene)?.label ?? t("hotkeys.missingScene");
  else if (action.kind === "expression") target = action.expression;
  else if (action.kind === "motion") target = action.motion;
  else if (action.kind === "pose") target = action.part;
  else if (action.kind === "dressUp")
    target = action.module === null ? action.slot : fileName(action.module);
  return target === null || target === "" ? kind : t("hotkeys.describe", { kind, target });
}

function problemText(problem: HotkeyProblem): string {
  switch (problem.code) {
    case "unassigned":
      return t("hotkeys.problems.unassigned");
    case "invalidKeys":
      return t("hotkeys.problems.invalidKeys");
    case "duplicate":
      return t("hotkeys.problems.duplicate", { action: describe(current[problem.other]) });
    case "registerFailed":
      return t("hotkeys.problems.registerFailed", { message: problem.message });
    case "unknownAction":
      return t("hotkeys.problems.unknownAction");
  }
}
</script>

<svelte:window onkeydowncapture={onKeydown} />

<div class="hotkeys" bind:this={root}>
  {#if removed !== null}
    <Notice ondismiss={() => (removed = null)}>
      {t("hotkeys.removed", { action: describe(removed.hotkey) })}
      {#snippet actions()}
        <Button small class="hotkeys__undo" {disabled} onclick={undoRemove}>{t("hotkeys.undo")}</Button>
      {/snippet}
    </Notice>
  {/if}
  {#if wayland}
    <Notice tone="warning">{t("hotkeys.wayland")}</Notice>
  {/if}
  {#if problems.length > 0}
    <Notice tone="warning">{t("hotkeys.problemsSummary", { count: problems.length })}</Notice>
  {/if}
  {#if pickError !== null}
    <Notice tone="danger" ondismiss={() => (pickError = null)}>{pickError}</Notice>
  {/if}
  {#if current.length === 0}
    <p class="hotkeys__hint">{t("hotkeys.empty")}</p>
  {/if}
  <ol class="hotkeys__list">
    {#each current as hotkey, index (index)}
      {@const problem = problemAt.get(index)}
      {@const action = hotkey.action}
      <li
        class="hotkeys__item"
        class:hotkeys__item--problem={problem !== undefined}
        aria-label={t("hotkeys.itemLabel", { n: index + 1 })}
      >
        <div class="hotkeys__row">
          <Button
            small
            class="hotkeys__keys"
            pressed={capturing === index}
            {disabled}
            aria-describedby={problem ? `hotkey-problem-${index}` : undefined}
            onclick={() => (capturing === index ? stopCapture() : startCapture(index))}
          >
            {#if capturing === index}
              {t("hotkeys.pressKeys")}
            {:else if hotkey.keys === ""}
              {t("hotkeys.noKeys")}
            {:else}
              <Kbd keys={displayKeys(hotkey.keys)} />
            {/if}
          </Button>
          <div class="hotkeys__kind">
            {#if action.kind === "unknown"}
              <span class="hotkeys__hint">{t("hotkeys.kinds.unknown")}</span>
            {:else}
              <Select
                aria-label={t("hotkeys.action")}
                value={action.kind}
                options={kindOptions}
                {disabled}
                onchange={(kind) => setKind(index, kind)}
              />
            {/if}
          </div>
          <Button small variant="ghost" {disabled} onclick={() => void remove(index)}>{t("hotkeys.remove")}</Button>
        </div>
        {#if action.kind === "scene"}
          <Select
            aria-label={t("hotkeys.scene")}
            value={action.scene}
            options={sceneChoices(action.scene)}
            {disabled}
            onchange={(scene) => setAction(index, { ...action, scene })}
          />
        {:else if action.kind === "expression"}
          <Select
            aria-label={t("hotkeys.expression")}
            value={action.expression}
            options={choices(expressions, action.expression)}
            placeholder={t("hotkeys.noExpressions")}
            {disabled}
            onchange={(expression) => setAction(index, { ...action, expression })}
          />
          <SegmentedControl
            aria-label={t("hotkeys.expressionMode")}
            value={action.mode}
            options={modeOptions}
            {disabled}
            onchange={(mode) => setAction(index, { ...action, mode })}
          />
        {:else if action.kind === "motion"}
          <Select
            aria-label={t("hotkeys.motion")}
            value={action.motion}
            options={choices(motions, action.motion)}
            placeholder={t("hotkeys.noMotions")}
            {disabled}
            onchange={(motion) => setAction(index, { ...action, motion })}
          />
          <SegmentedControl
            aria-label={t("hotkeys.priority")}
            value={action.priority}
            options={priorityOptions}
            {disabled}
            onchange={(priority: MotionPriority) => setAction(index, { ...action, priority })}
          />
        {:else if action.kind === "dressUp"}
          <SegmentedControl
            aria-label={t("hotkeys.dressMode")}
            value={action.module === null ? "remove" : "attach"}
            options={dressOptions}
            {disabled}
            onchange={(mode) =>
              mode === "remove"
                ? setAction(index, { ...action, module: null })
                : void pickModule(index)}
          />
          {#if action.module !== null}
            <div class="hotkeys__row">
              <span class="hotkeys__file" title={action.module}>{fileName(action.module)}</span>
              <Button small {disabled} onclick={() => pickModule(index)}>{t("hotkeys.chooseModule")}</Button>
            </div>
            <p class="hotkeys__hint">{t("hotkeys.slotOf", { slot: action.slot })}</p>
          {:else}
            <Select
              aria-label={t("hotkeys.slot")}
              value={action.slot}
              options={choices(slots, action.slot)}
              placeholder={t("hotkeys.noSlots")}
              {disabled}
              onchange={(slot) => setAction(index, { ...action, slot })}
            />
          {/if}
        {:else if action.kind === "pose"}
          <Select
            aria-label={t("hotkeys.pose")}
            value={action.part}
            options={choices(poses, action.part)}
            placeholder={t("hotkeys.noPoses")}
            {disabled}
            onchange={(part) => setAction(index, { ...action, part })}
          />
        {/if}
        {#if problem !== undefined}
          <p class="hotkeys__problem" id={`hotkey-problem-${index}`}>
            <Icon name="warning" />
            <span class="ugoe-selectable">{problemText(problem)}</span>
          </p>
        {:else if hotkey.keys !== "" && !hasModifier(hotkey.keys)}
          <p class="hotkeys__hint">{t("hotkeys.noModifier")}</p>
        {/if}
      </li>
    {/each}
  </ol>
  <div class="hotkeys__row">
    <Button {disabled} onclick={add}>{t("hotkeys.add")}</Button>
  </div>
  <p class="hotkeys__hint">{t("hotkeys.hint")}</p>
</div>

<style>
  .hotkeys {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .hotkeys__list {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .hotkeys__item {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    padding-bottom: var(--ugoe-space-3);
    border-bottom: 1px solid var(--ugoe-color-border);
  }

  .hotkeys__row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2);
  }

  .hotkeys__kind {
    flex: 1;
    min-width: 8rem;
  }

  .hotkeys__file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 14rem;
  }

  .hotkeys__hint,
  .hotkeys__problem {
    margin: 0;
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }

  /* 登録できない行: 色に加えて、左の線と印 (三角) で示す (色だけで伝えない) */
  .hotkeys__item--problem {
    padding-left: var(--ugoe-space-2);
    border-left: 3px solid var(--ugoe-color-warning);
  }

  .hotkeys__problem {
    display: flex;
    align-items: flex-start;
    gap: var(--ugoe-space-1);
    color: var(--ugoe-color-text);
  }

  .hotkeys__problem :global(.ugoe-icon) {
    flex: none;
    color: var(--ugoe-color-warning);
  }
</style>
