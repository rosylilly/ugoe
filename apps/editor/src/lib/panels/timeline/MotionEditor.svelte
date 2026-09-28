<!--
  モーションの編集: モーションの選択と作成・削除・名前・長さ・ループ・フェード・イベント、再生・停止・シーク、
  パラメータごとの行とキー (ドープシート)、選んだ行のカーブ (CurveGraph)。
  キーは「今のスライダーの値」で打つ (行の ◆・「Key changed」)。操作はすべて Command (docs/editor.md#タイムライン)。
-->
<script lang="ts">
import { untrack } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import type { Command, CurveTarget, Motion, MotionEvent, Point } from "../../ipc";
import type { ViewportController } from "../../viewport/controller.svelte";
import CurveGraph from "./CurveGraph.svelte";
import {
  canMoveKey,
  curveKeys,
  FPS,
  isValidName,
  keyCommand,
  keyValuesCommand,
  newMotion,
  roundTime,
  snapToFrame,
  targetKey,
  tickStep,
  timeToX,
  tracks,
  uniqueName,
  xToTime,
} from "./timeline";
import type { TimelineState } from "./timeline.svelte";

interface Props {
  doc: EditorDocument;
  viewport: ViewportController;
  timeline: TimelineState;
}

let { doc, viewport, timeline }: Props = $props();

const motion = $derived(timeline.motion);
const rows = $derived(tracks(motion, doc.parameters));
const selectedRow = $derived(
  rows.find((r) => targetKey(r.target) === timeline.selectedTrack) ?? null,
);
/** 表示する時刻の範囲 (両端に少し余白を付けて、0 と最後のキーも押せるようにする)。 */
const span = $derived.by(() => {
  const d = motion?.duration ?? 1;
  const pad = d * 0.02;
  return { t0: -pad, t1: d + pad };
});
let laneWidth = $state(600);
const view = $derived({ ...span, width: laneWidth });
const ticks = $derived.by(() => {
  const d = motion?.duration ?? 0;
  const step = tickStep(d, laneWidth);
  const out: number[] = [];
  for (let i = 0; i * step <= d + 1e-6; i++) out.push(roundTime(i * step));
  return out;
});
let message = $state<string | null>(null);
let renaming = $state("");

/** 名前を変えている途中の新しい名前 (読み直しで選択が外れないように、先に覚えておく)。 */
let renamedTo: string | null = null;

// 最初のモーションを選ぶ。選んだモーションが消えたら (Undo・削除) 選び直す。名前を変えたら新しい名前を選ぶ。
$effect(() => {
  const motions = doc.motions;
  untrack(() => {
    if (renamedTo !== null && motions.some((m) => m.name === renamedTo)) {
      timeline.motionName = renamedTo;
      renamedTo = null;
    } else if (!motions.some((m) => m.name === timeline.motionName)) {
      timeline.select(motions[0]?.name ?? null);
    }
  });
});

// 編集 (と Undo) の後は、今の再生位置の姿を書き込み直す (シーク・再生・キーの操作をした後だけ)。
$effect(() => {
  const m = motion;
  untrack(() => {
    renaming = m?.name ?? "";
    if (m && timeline.engaged && !timeline.playing) timeline.applyPose();
  });
});

async function apply(command: Command): Promise<boolean> {
  message = null;
  await doc.apply(command);
  return !doc.error;
}

async function create() {
  const name = uniqueName(
    "Motion",
    doc.motions.map((m) => m.name),
  );
  if (await apply({ op: "add_motion", motion: newMotion(name) })) timeline.select(name);
}

async function remove() {
  if (!motion) return;
  await apply({ op: "remove_motion", name: motion.name });
}

async function rename() {
  if (!motion || renaming === motion.name) return;
  if (!isValidName(renaming)) {
    message = "Use letters, digits, _, - and . (not starting with .) in motion names.";
    renaming = motion.name;
    return;
  }
  const old = motion.name;
  renamedTo = renaming;
  if (!(await apply({ op: "rename_motion", name: old, new_name: renaming }))) {
    renamedTo = null;
    renaming = old;
  }
}

async function setProperty(field: "duration" | "fade_in" | "fade_out", e: Event) {
  if (!motion) return;
  const input = e.currentTarget as HTMLInputElement;
  const value = input.valueAsNumber;
  if (!Number.isFinite(value)) {
    input.value = String(motion[field]);
    return;
  }
  if (!(await apply({ op: "set_motion_properties", name: motion.name, [field]: value }))) {
    input.value = String(motion[field]);
  }
}

async function setLooping(e: Event) {
  if (!motion) return;
  const looping = (e.currentTarget as HTMLInputElement).checked;
  await apply({ op: "set_motion_properties", name: motion.name, looping });
}

// --- キー ---

function currentValues(): Record<string, number> {
  return viewport.parameterValues;
}

/** 行の ◆: その行のパラメータに、今のスライダーの値でキーを打つ (再生位置に)。 */
async function keyRow(target: CurveTarget) {
  if (!motion || target.kind !== "parameter") return;
  timeline.stop();
  const value = currentValues()[target.id];
  if (value === undefined) return;
  const t = timeline.time;
  if (await apply(keyCommand(motion.name, target, t, value))) timeline.selectKey(target, t);
}

/** スライダーを動かしたパラメータ (カーブの値・既定値と違うもの) すべてに、今の値でキーを打つ。 */
async function keyChanged() {
  if (!motion) return;
  timeline.stop();
  timeline.engaged = true;
  const command = keyValuesCommand(
    motion,
    doc.parameters,
    currentValues(),
    timeline.time,
    timeline.sample(timeline.time),
  );
  if (!command) {
    message = "No parameter differs from the motion at this time. Move a slider first.";
    return;
  }
  await apply(command);
}

async function removeSelectedKey() {
  const key = timeline.selectedKey;
  const row = rows.find((r) => targetKey(r.target) === key?.track);
  if (!motion || !key || !row) return;
  if (await apply({ op: "remove_motion_key", motion: motion.name, target: row.target, t: key.t })) {
    timeline.selectedKey = null;
  }
}

async function moveKey(target: CurveTarget, t: number, to: Point) {
  if (!motion) return;
  if (await apply({ op: "move_motion_key", motion: motion.name, target, t, to })) {
    timeline.selectKey(target, to.t);
  }
}

// --- ドープシートのドラッグ ---

let drag = $state<{ target: CurveTarget; t: number; v: number; x0: number; dt: number } | null>(
  null,
);

function onKeyDown(e: PointerEvent, target: CurveTarget, key: Point) {
  e.stopPropagation();
  timeline.stop();
  timeline.selectKey(target, key.t);
  timeline.seek(key.t);
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  drag = { target, t: key.t, v: key.v, x0: e.clientX, dt: 0 };
}

function onKeyMove(e: PointerEvent) {
  if (!drag) return;
  const t = xToTime(e.clientX - drag.x0, { t0: 0, t1: span.t1 - span.t0, width: laneWidth });
  drag.dt = t;
}

async function onKeyUp(e: PointerEvent) {
  const d = drag;
  drag = null;
  if (!d || Math.abs(e.clientX - d.x0) < 3) return;
  const to = snapToFrame(Math.max(0, d.t + d.dt));
  const curve = motion?.curves.find((c) => targetKey(c.target) === targetKey(d.target));
  if (!curve || to === d.t) return;
  if (!canMoveKey(curve, d.t, to)) {
    message = "A key cannot be moved past its neighbor.";
    return;
  }
  await moveKey(d.target, d.t, { t: to, v: d.v });
}

function keyX(target: CurveTarget, key: Point): number {
  const shifted =
    drag && targetKey(drag.target) === targetKey(target) && drag.t === key.t
      ? snapToFrame(Math.max(0, key.t + drag.dt))
      : key.t;
  return timeToX(shifted, view);
}

// --- シーク ---

let scrubbing = false;

function seekAt(e: PointerEvent) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const t = xToTime(e.clientX - rect.left, view);
  timeline.seek(e.shiftKey ? t : snapToFrame(t));
}

function onRulerDown(e: PointerEvent) {
  timeline.stop();
  scrubbing = true;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  seekAt(e);
}

function onRulerMove(e: PointerEvent) {
  if (scrubbing) seekAt(e);
}

function onTimeChange(e: Event) {
  const value = (e.currentTarget as HTMLInputElement).valueAsNumber;
  if (Number.isFinite(value)) timeline.seek(value);
}

function stepFrame(n: number) {
  timeline.stop();
  timeline.seek(snapToFrame(timeline.time + n / FPS));
}

function onPanelKey(e: KeyboardEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("input, select, textarea")) return;
  if (e.key === "Delete" || e.key === "Backspace") {
    if (timeline.selectedKey) {
      e.preventDefault();
      removeSelectedKey();
    }
  } else if (e.key === " ") {
    e.preventDefault();
    timeline.toggle();
  } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
    e.preventDefault();
    stepFrame(e.key === "ArrowLeft" ? -1 : 1);
  }
}

// --- イベント ---

async function setEvents(events: MotionEvent[]) {
  if (!motion) return;
  await apply({ op: "set_motion_events", motion: motion.name, events });
}

function addEvent() {
  if (!motion) return;
  setEvents([...motion.events, { t: timeline.time, value: "event" }]);
}

function editEvent(i: number, field: "t" | "value", e: Event) {
  if (!motion) return;
  const input = e.currentTarget as HTMLInputElement;
  const events = motion.events.map((ev) => ({ ...ev }));
  const event = events[i];
  if (!event) return;
  if (field === "t") {
    if (!Number.isFinite(input.valueAsNumber)) return;
    event.t = input.valueAsNumber;
  } else {
    event.value = input.value;
  }
  setEvents(events);
}

function removeEvent(i: number) {
  if (!motion) return;
  setEvents(motion.events.filter((_, j) => j !== i));
}

function label(m: Motion): string {
  return `${m.name} (${m.duration}s${m.looping ? ", loop" : ""})`;
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="motion-editor" onkeydown={onPanelKey}>
  <div class="bar">
    <select
      aria-label="Motion"
      value={timeline.motionName ?? ""}
      onchange={(e) => timeline.select((e.currentTarget as HTMLSelectElement).value || null)}
    >
      {#if doc.motions.length === 0}<option value="">(no motions)</option>{/if}
      {#each doc.motions as m (m.name)}
        <option value={m.name}>{label(m)}</option>
      {/each}
    </select>
    <button onclick={create} title="Create a new motion">New motion</button>
    {#if motion}
      <button onclick={remove} title="Delete this motion">Delete</button>
      <span class="sep"></span>
      <label class="inline"
        >Name <input
          class="name"
          bind:value={renaming}
          onchange={rename}
          onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
        /></label
      >
      <label class="inline"
        >Duration <input
          type="number"
          min="0.01"
          step="0.1"
          value={motion.duration}
          onchange={(e) => setProperty("duration", e)}
        />s</label
      >
      <label class="inline"
        ><input type="checkbox" checked={motion.looping} onchange={setLooping} /> Loop</label
      >
      <label class="inline"
        >Fade in <input
          type="number"
          min="0"
          step="0.1"
          value={motion.fade_in}
          onchange={(e) => setProperty("fade_in", e)}
        />s</label
      >
      <label class="inline"
        >Fade out <input
          type="number"
          min="0"
          step="0.1"
          value={motion.fade_out}
          onchange={(e) => setProperty("fade_out", e)}
        />s</label
      >
      <details class="events">
        <summary>Events ({motion.events.length})</summary>
        <div class="popup">
          {#each motion.events as ev, i (i)}
            <div class="event">
              <input
                type="number"
                min="0"
                step="0.1"
                value={ev.t}
                aria-label="Event {i + 1} time"
                onchange={(e) => editEvent(i, "t", e)}
              />
              <input value={ev.value} aria-label="Event {i + 1} value" onchange={(e) => editEvent(i, "value", e)} />
              <button onclick={() => removeEvent(i)} aria-label="Remove event {i + 1}">×</button>
            </div>
          {/each}
          <button onclick={addEvent}>+ Event at {timeline.time}s</button>
        </div>
      </details>
    {/if}
  </div>

  {#if motion}
    <div class="bar transport">
      <button
        class="play"
        onclick={() => timeline.toggle()}
        title="Play / stop (Space)"
        aria-label={timeline.playing ? "Stop" : "Play"}>{timeline.playing ? "■ Stop" : "▶ Play"}</button
      >
      <label class="inline" title="Repeat the playback from the start (the motion's Loop also repeats)"
        ><input type="checkbox" bind:checked={timeline.repeat} /> Repeat</label
      >
      <button onclick={() => stepFrame(-1)} aria-label="Previous frame" title="Previous frame (←)">◀</button>
      <input
        class="time"
        type="number"
        min="0"
        max={motion.duration}
        step={1 / FPS}
        value={roundTime(timeline.time)}
        aria-label="Time"
        onchange={onTimeChange}
      />
      <button onclick={() => stepFrame(1)} aria-label="Next frame" title="Next frame (→)">▶</button>
      <span class="muted">/ {motion.duration}s</span>
      <span class="sep"></span>
      <button
        onclick={keyChanged}
        title="Key every parameter whose slider differs from the motion at this time"
        >◆ Key changed</button
      >
      <button
        onclick={removeSelectedKey}
        disabled={!timeline.selectedKey}
        title="Delete the selected key (Delete)">Delete key</button
      >
      {#if message}<span class="error">{message}</span>{/if}
    </div>

    <div class="sheet">
      <div class="grid">
        <div class="corner muted">{rows.filter((r) => r.curve).length} curves</div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="ruler"
          bind:clientWidth={laneWidth}
          onpointerdown={onRulerDown}
          onpointermove={onRulerMove}
          onpointerup={() => (scrubbing = false)}
          onpointercancel={() => (scrubbing = false)}
          title="Click or drag to seek (Shift: without snapping to frames)"
        >
          {#each ticks as t (t)}
            <span class="tick" style:left="{timeToX(t, view)}px">{t}</span>
          {/each}
          {#each motion.events as ev, i (i)}
            <span class="event-mark" style:left="{timeToX(ev.t, view)}px" title="Event: {ev.value}">▾</span>
          {/each}
          <span class="playhead" style:left="{timeToX(timeline.time, view)}px"></span>
        </div>
        {#each rows as row (targetKey(row.target))}
          {@const key = targetKey(row.target)}
          <div class="label" class:selected={timeline.selectedTrack === key} class:has-curve={row.curve}>
            <button class="track" onclick={() => (timeline.selectedTrack = key)} title={row.label}>{row.label}</button>
            {#if row.target.kind === "parameter" && row.parameter}
              <button
                class="key"
                onclick={() => keyRow(row.target)}
                title="Key {row.target.id} at {timeline.time}s with its slider value">◆</button
              >
            {/if}
          </div>
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            class="lane"
            class:selected={timeline.selectedTrack === key}
            onclick={() => (timeline.selectedTrack = key)}
          >
            <span class="playhead" style:left="{timeToX(timeline.time, view)}px"></span>
            {#if row.curve}
              {#each curveKeys(row.curve) as k (k.t)}
                {@const selected = timeline.selectedKey?.track === key && timeline.selectedKey.t === k.t}
                <button
                  class="diamond"
                  class:selected
                  style:left="{keyX(row.target, k)}px"
                  title="{row.label}: {k.v} at {k.t}s (drag to move)"
                  aria-label="Key {row.label} at {k.t}"
                  onpointerdown={(e) => onKeyDown(e, row.target, k)}
                  onpointermove={onKeyMove}
                  onpointerup={onKeyUp}
                  onpointercancel={() => (drag = null)}
                ></button>
              {/each}
            {/if}
          </div>
        {/each}
      </div>
    </div>

    {#if selectedRow}
      <CurveGraph {doc} {timeline} motion={motion} row={selectedRow} {view} onmessage={(m) => (message = m)} />
    {/if}
  {:else}
    <p class="muted empty">No motions yet. Press “New motion”, then key the sliders over time.</p>
  {/if}
</div>

<style>
  .motion-editor {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  input[type="number"] {
    width: 56px;
  }
  input.name {
    width: 110px;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--border);
  }
  .events {
    position: relative;
  }
  .events .popup {
    position: absolute;
    z-index: 10;
    top: 100%;
    right: 0;
    background: var(--panel);
    border: 1px solid var(--border);
    padding: 6px;
    display: grid;
    gap: 4px;
    min-width: 260px;
  }
  .event {
    display: flex;
    gap: 4px;
  }
  .sheet {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .grid {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
  }
  .corner,
  .ruler {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--panel);
    height: 20px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
  }
  .corner {
    padding: 2px 8px;
  }
  .ruler {
    position: sticky;
    cursor: ew-resize;
    touch-action: none;
  }
  .tick {
    position: absolute;
    top: 2px;
    transform: translateX(-50%);
    color: var(--muted);
    pointer-events: none;
  }
  .event-mark {
    position: absolute;
    bottom: -4px;
    transform: translateX(-50%);
    color: #d97706;
    pointer-events: none;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 20px;
    padding-left: 4px;
    min-width: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
  }
  .label .track {
    flex: 1;
    min-width: 0;
    text-align: left;
    border: none;
    background: none;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }
  .label.has-curve .track {
    color: var(--text);
  }
  .selected {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .key {
    border: none;
    background: none;
    color: var(--muted);
    font-size: 10px;
    padding: 0 4px;
  }
  .key:hover {
    color: var(--accent);
  }
  .lane {
    position: relative;
    height: 20px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
  }
  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #dc2626;
    pointer-events: none;
  }
  .diamond {
    position: absolute;
    top: 5px;
    width: 10px;
    height: 10px;
    margin-left: -5px;
    padding: 0;
    border: 1px solid var(--text);
    background: var(--panel);
    transform: rotate(45deg);
    cursor: grab;
    touch-action: none;
  }
  .diamond.selected {
    background: var(--accent);
    border-color: var(--accent);
  }
  .empty {
    padding: 12px;
  }
</style>
