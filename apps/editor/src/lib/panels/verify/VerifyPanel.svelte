<!--
  検証のパネル (docs/editor.md#はみ出しの検証)。右の列の「Verify」タブ。モデルは変えない。

  - 「Show overflows in viewport」: 今のパラメータ値で親のワープの格子の外にある点を、ビューポートに赤い印で
    重ねる (viewport/tools/overflow/OverflowOverlay.svelte)。
  - 「Verify all cells」: 親がワープのすべての要素について、キーフォームのすべてのセル (軸のキーの組み合わせ)
    ではみ出しを調べ、要素ごとに一覧にする。行を押すと、その要素を選び、ビューポートのパラメータを
    そのセルの値にする。一度調べた後は、モデルが変わるたびに調べ直す。
  - 「Physics」: 静止姿勢で衝突体に当たっている鎖 (physics/RestPoseWarnings.svelte、docs/editor.md#物理の警告)。
    行を押すと「Physics」のタブに移り、そのリグと衝突体を選ぶ。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { ipc, type Model, type OverflowFinding, type OverflowReport } from "../../ipc";
import type { Selection } from "../../selection.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import type { PhysicsView } from "../physics/physicsView.svelte";
import RestPoseWarnings from "../physics/RestPoseWarnings.svelte";
import {
  cellParameters,
  formatAmount,
  formatCell,
  formatCount,
  groupFindings,
  targetLabel,
  targetRef,
} from "./overflow";
import type { VerifyView } from "./verifyView.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
  view: VerifyView;
  physics: PhysicsView;
}

let { doc, selection, viewport, view, physics }: Props = $props();

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});

let report = $state.raw<OverflowReport | null>(null);
let error = $state<string | null>(null);
let running = $state(false);
/** 調べたときのドキュメントの版 (変わったら調べ直す)。 */
let checkedRevision = -1;

const groups = $derived(report ? groupFindings(report) : []);
const currentPoints = $derived(view.current.reduce((n, e) => n + e.points.length, 0));

async function verify(): Promise<void> {
  if (running) return;
  running = true;
  checkedRevision = doc.revision;
  try {
    report = await ipc.verifyOverflows();
    error = null;
  } catch (e) {
    report = null;
    error = String(e);
  } finally {
    running = false;
  }
}

// 一度調べた後は、モデルが変わるたびに調べ直す (軽い計算なので自動で行う)。
$effect(() => {
  const revision = doc.revision;
  if (report !== null && revision !== checkedRevision && !running) void verify();
});

/** 要素を選び、ビューポートのパラメータをセルの値にする。はみ出しの表示も出す。 */
function goTo(finding: OverflowFinding): void {
  const ref = targetRef(finding.target);
  if (ref) selection.set([ref]);
  if (viewport.previewingPhysics) viewport.stopPhysicsPreview();
  for (const [id, value] of cellParameters(finding.cell)) viewport.setParameter(id, value);
  view.showOverflows = true;
}

/** 「Physics」のタブに移り、リグと衝突体を選ぶ。 */
function goToPhysics(rig: string, collider: string): void {
  physics.rig = rig;
  physics.collider = collider;
  physics.active = true;
  view.active = false;
}
</script>

<section>
  <h3>Deformer overflow</h3>
  <p class="muted note">
    Points whose position in the parent warp's grid is outside the grid follow the warp by
    extrapolation, which bends less than the inside.
  </p>
  <label class="check">
    <input type="checkbox" bind:checked={view.showOverflows} />
    Show overflows in viewport
  </label>
  {#if view.showOverflows}
    {#if view.error}
      <p class="error">{view.error}</p>
    {:else if currentPoints === 0}
      <p class="muted">No points outside the parent warp at the current parameters.</p>
    {:else}
      <p class="warn">
        {currentPoints} {currentPoints === 1 ? "point" : "points"} in {view.current.length}
        {view.current.length === 1 ? "element" : "elements"} outside the parent warp now.
      </p>
    {/if}
  {/if}

  <div class="row">
    <button onclick={verify} disabled={running || !doc.model}>
      {running ? "Verifying…" : "Verify all cells"}
    </button>
    {#if report}
      <span class="muted">
        {report.elements} elements · {report.cells} cells
      </span>
    {/if}
  </div>
  {#if error}<p class="error">{error}</p>{/if}

  {#if report}
    {#if report.truncated.length > 0}
      <p class="muted note">
        Too many key combinations: only the axis ends were checked for
        {report.truncated.map((t) => targetLabel(t, model)).join(", ")}.
      </p>
    {/if}
    {#if groups.length === 0}
      <p class="ok">No overflow in any keyform cell.</p>
    {:else}
      <p class="warn">
        {groups.length} {groups.length === 1 ? "element overflows" : "elements overflow"} in
        {report.findings.length} {report.findings.length === 1 ? "cell" : "cells"}.
      </p>
      <ul class="groups">
        {#each groups as group (group.key)}
          <li>
            <div class="element">
              <strong>{targetLabel(group.target, model)}</strong>
              <span class="muted">in {targetLabel({ kind: "deformer", id: group.parent }, model)} · max {formatAmount(group.maxAmount)}</span>
            </div>
            <ul class="cells">
              {#each group.findings as finding, i (i)}
                <li>
                  <button
                    class="cell"
                    class:current={selection.primary?.id === group.target.id &&
                      cellParameters(finding.cell).every(([id, v]) => viewport.parameterValues[id] === v)}
                    onclick={() => goTo(finding)}
                    title="Select the element and set the parameters to this cell"
                  >
                    <span class="values">{formatCell(finding.cell, model)}</span>
                    <span class="amount">
                      {formatCount(group.kind, finding.count)} · {formatAmount(finding.max_amount)}
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<section>
  <h3>Physics</h3>
  <RestPoseWarnings {doc} onSelect={goToPhysics} showEmpty />
</section>

<style>
h3 {
  margin: 8px 0 4px;
  font-size: 13px;
}

p {
  margin: 4px 0;
}

.note {
  font-size: 11px;
}

.check {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 6px 0;
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 4px;
}

.error {
  color: #dc2626;
}

.warn {
  color: #dc2626;
}

.ok {
  color: #16a34a;
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

.groups > li {
  margin: 8px 0;
}

.element {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 8px;
  align-items: baseline;
  overflow-wrap: anywhere;
}

.cells {
  margin-top: 2px;
}

.cell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  margin: 2px 0;
  text-align: left;
  gap: 1px;
}

.cell.current {
  border-color: var(--accent);
}

.values {
  overflow-wrap: anywhere;
}

.amount {
  font-size: 11px;
  color: var(--muted);
}
</style>
