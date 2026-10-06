<!--
  ビューポートの下のパネル: モーション (タイムライン) と表情のタブ。畳める (表示の状態、localStorage に覚える)。
  開いているときの高さは CSS の変数 `--timeline-height` (App.svelte の区切りで変える。無ければ窓の高さに合わせる)。
  状態 (選択中のモーション・再生位置など) は TimelineState、編集は Command (docs/editor.md#タイムライン)。
-->
<script lang="ts">
import { Icon, IconButton } from "@ugoe/ui";
import { onDestroy, untrack } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import TabStrip, { type TabItem } from "../../ui/TabStrip.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import ExpressionEditor from "./ExpressionEditor.svelte";
import MotionEditor from "./MotionEditor.svelte";
import { loadPanelOpen, savePanelOpen } from "./timeline";
import { TimelineState } from "./timeline.svelte";

interface Props {
  doc: EditorDocument;
  viewport: ViewportController;
  /** 開いているか (App が高さの区切りを出すかに使う)。 */
  open?: boolean;
}

let { doc, viewport, open = $bindable(loadPanelOpen()) }: Props = $props();

// doc と viewport は App が作る 1 つだけのオブジェクトで、差し替わらない。
// svelte-ignore state_referenced_locally
const timeline = new TimelineState(doc, viewport);
onDestroy(() => timeline.dispose());

// プレビューモード (lib/preview/) に入ったら、再生と表情のプレビューを止める。表情のプレビューは
// 重ねる前の値 (編集中の姿) に戻すので、プレビューを出たときにもその姿に戻る。
$effect(() => {
  if (viewport.driven) {
    untrack(() => {
      timeline.stop();
      timeline.endPreview();
    });
  }
});

let tab = $state<"motions" | "expressions">("motions");

function toggleOpen() {
  open = !open;
  savePanelOpen(open);
}

type Tab = "motions" | "expressions";

const tabs = $derived<TabItem<Tab>[]>([
  { value: "motions", label: t("timeline.tabs.motions", { count: doc.motions.length }) },
  {
    value: "expressions",
    label: t("timeline.tabs.expressions", { count: doc.expressions.length }),
  },
]);

function switchTab(next: "motions" | "expressions") {
  tab = next;
  if (next === "motions") timeline.endPreview();
  else timeline.stop();
}
</script>

<section class="timeline" class:open aria-label={t("timeline.aria")}>
  <div class="tabs">
    <IconButton
      small
      label={open ? t("timeline.collapse") : t("timeline.expand")}
      aria-expanded={open}
      onclick={toggleOpen}><Icon name={open ? "chevronDown" : "chevronRight"} /></IconButton
    >
    <TabStrip {tabs} value={tab} aria-label={t("timeline.tabs.aria")} onselect={switchTab} />
    {#if doc.error && open}<span class="error ugoe-selectable" title={doc.error}>{doc.error}</span>{/if}
  </div>
  {#if open}
    <div class="body">
      {#if tab === "motions"}
        <MotionEditor {doc} {viewport} {timeline} />
      {:else}
        <ExpressionEditor {doc} {viewport} {timeline} />
      {/if}
    </div>
  {/if}
</section>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--ugoe-color-border);
    background: var(--ugoe-color-surface);
    min-height: 0;
  }
  .timeline.open {
    height: var(--timeline-height, clamp(280px, 42vh, 520px));
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    padding: 0 var(--ugoe-space-3);
    border-bottom: 1px solid var(--ugoe-color-border);
    min-width: 0;
  }
  /* 下の線は .tabs が引く (畳むボタンとエラーの下にも) */
  .tabs :global(.tab-strip) {
    border-bottom: 0;
  }
  .error {
    margin-left: var(--ugoe-space-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
