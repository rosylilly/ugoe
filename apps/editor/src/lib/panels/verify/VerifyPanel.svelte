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
import { Button, Toggle } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { errorText } from "../../errors";
import { t } from "../../i18n";
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
    error = errorText(e);
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
  <h3>{t("verify.overflow.title")}</h3>
  <p class="ed-hint">{t("verify.overflow.description")}</p>
  <div class="check">
    <Toggle bind:checked={view.showOverflows} label={t("verify.overflow.show")} />
  </div>
  {#if view.showOverflows}
    {#if view.error}
      <p class="error ugoe-selectable">{view.error}</p>
    {:else if currentPoints === 0}
      <p class="ed-hint">{t("verify.overflow.noneNow")}</p>
    {:else}
      <p class="warn">
        {t("verify.overflow.now", {
          points: t("verify.counts.points", { count: currentPoints }),
          elements: t("verify.counts.elements", { count: view.current.length }),
        })}
      </p>
    {/if}
  {/if}

  <div class="ed-buttons row">
    <Button onclick={verify} disabled={running || !doc.model}>
      {running ? t("verify.overflow.verify.running") : t("verify.overflow.verify.label")}
    </Button>
    {#if report}
      <span class="muted">
        {t("verify.overflow.checked", {
          elements: t("verify.counts.elements", { count: report.elements }),
          cells: t("verify.counts.cells", { count: report.cells }),
        })}
      </span>
    {/if}
  </div>
  {#if error}<p class="error ugoe-selectable">{error}</p>{/if}

  {#if report}
    {#if report.truncated.length > 0}
      <p class="ed-hint">
        {t("verify.overflow.truncated", {
          elements: report.truncated.map((target) => targetLabel(target, model)).join(", "),
        })}
      </p>
    {/if}
    {#if groups.length === 0}
      <p class="ok">{t("verify.overflow.none")}</p>
    {:else}
      <p class="warn">
        {t("verify.overflow.found", {
          count: groups.length,
          cells: t("verify.counts.cells", { count: report.findings.length }),
        })}
      </p>
      <ul class="groups">
        {#each groups as group (group.key)}
          <li>
            <div class="element">
              <strong>{targetLabel(group.target, model)}</strong>
              <span class="muted"
                >{t("verify.overflow.parent", {
                  parent: targetLabel({ kind: "deformer", id: group.parent }, model),
                  amount: formatAmount(group.maxAmount),
                })}</span
              >
            </div>
            <ul class="cells">
              {#each group.findings as finding, i (i)}
                <li>
                  <button
                    class="cell"
                    class:current={selection.primary?.id === group.target.id &&
                      cellParameters(finding.cell).every(([id, v]) => viewport.parameterValues[id] === v)}
                    onclick={() => goTo(finding)}
                    title={t("verify.overflow.cell")}
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
  <h3>{t("verify.physics")}</h3>
  <RestPoseWarnings {doc} onSelect={goToPhysics} showEmpty />
</section>

<style>
p {
  margin: var(--ugoe-space-2) 0;
}

.check {
  margin: var(--ugoe-space-2) 0;
}

.row {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}

.error {
  color: var(--ugoe-color-danger);
}

.warn {
  color: var(--ugoe-color-warning);
}

.ok {
  color: var(--ugoe-color-success);
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

.groups > li {
  margin: var(--ugoe-space-3) 0;
}

.element {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-1) var(--ugoe-space-3);
  align-items: baseline;
  overflow-wrap: anywhere;
}

.cells {
  margin-top: var(--ugoe-space-1);
}

.cell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  margin: var(--ugoe-space-1) 0;
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
  text-align: left;
  gap: 1px;
}

.cell.current {
  border-color: var(--ugoe-color-accent);
}

.cell:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.values {
  overflow-wrap: anywhere;
}

.amount {
  font-size: var(--ugoe-font-size-xs);
  color: var(--ugoe-color-text-muted);
}
</style>
