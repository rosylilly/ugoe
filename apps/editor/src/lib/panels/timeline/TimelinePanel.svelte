<!--
  ビューポートの下のパネル: モーション (タイムライン) と表情のタブ。畳める (表示の状態、localStorage に覚える)。
  状態 (選択中のモーション・再生位置など) は TimelineState、編集は Command (docs/editor.md#タイムライン)。
-->
<script lang="ts">
import { onDestroy } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import ExpressionEditor from "./ExpressionEditor.svelte";
import MotionEditor from "./MotionEditor.svelte";
import { TimelineState } from "./timeline.svelte";

interface Props {
  doc: EditorDocument;
  viewport: ViewportController;
}

let { doc, viewport }: Props = $props();

// doc と viewport は App が作る 1 つだけのオブジェクトで、差し替わらない。
// svelte-ignore state_referenced_locally
const timeline = new TimelineState(doc, viewport);
onDestroy(() => timeline.dispose());

const STORAGE_KEY = "ugoe.timeline";
let tab = $state<"motions" | "expressions">("motions");
let open = $state(load());

function load(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== "closed";
  } catch {
    return true;
  }
}

function toggleOpen() {
  open = !open;
  try {
    localStorage.setItem(STORAGE_KEY, open ? "open" : "closed");
  } catch {
    // 覚えられなくても表示は変える。
  }
}

function switchTab(next: "motions" | "expressions") {
  tab = next;
  if (next === "motions") timeline.endPreview();
  else timeline.stop();
}
</script>

<section class="timeline" class:open aria-label="Timeline">
  <div class="tabs" role="tablist">
    <button
      class="toggle"
      onclick={toggleOpen}
      aria-expanded={open}
      title={open ? "Collapse the timeline" : "Expand the timeline"}>{open ? "▾" : "▸"}</button
    >
    <button role="tab" aria-selected={tab === "motions"} onclick={() => switchTab("motions")}
      >Motions <span class="count">{doc.motions.length}</span></button
    >
    <button
      role="tab"
      aria-selected={tab === "expressions"}
      onclick={() => switchTab("expressions")}
      >Expressions <span class="count">{doc.expressions.length}</span></button
    >
    {#if doc.error && open}<span class="error" title={doc.error}>{doc.error}</span>{/if}
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
    border-top: 1px solid var(--border);
    background: var(--panel);
    min-height: 0;
  }
  .timeline.open {
    height: clamp(280px, 42vh, 520px);
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-bottom: 1px solid var(--border);
    min-width: 0;
  }
  .tabs button[role="tab"] {
    border: none;
    background: none;
    color: var(--muted);
    padding: 4px 8px;
    border-bottom: 2px solid transparent;
  }
  .tabs button[aria-selected="true"] {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .toggle {
    border: none;
    background: none;
    color: var(--muted);
    width: 20px;
  }
  .count {
    color: var(--muted);
    font-size: 11px;
  }
  .error {
    margin-left: 8px;
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
