<!--
  モジュールのプロジェクトで、本体のもの (読み取り専用の下敷き) の欄をまとめて無効にし、「本体のプロジェクトで直す」と
  示す (docs/editor.md#本体のものの表示)。`locked` でなければ中身をそのまま出す。
  無効にするのは入力欄とボタン (fieldset の disabled)。見るだけの表示はそのまま読める。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import { baseEditHint } from "./moduleProject";
import { provideBaseLock, useBaseLock } from "./ui/baseLock";

interface Props {
  locked: boolean;
  /** 無効にした理由と直す場所 (既定は本体のものであること)。 */
  hint?: string;
  children: Snippet;
}

let { locked, hint, children }: Props = $props();

// 外側の BaseLock が無効にしていれば、中も無効 (Slider などは `useBaseLock()` で読む)
const outer = useBaseLock();
provideBaseLock(() => locked || outer());
</script>

{#if locked}<p class="base-note" role="note">{hint ?? baseEditHint()}</p>{/if}
<fieldset class="base-lock" class:locked disabled={locked}>
  {@render children()}
</fieldset>

<style>
  .base-lock {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  .base-lock.locked {
    opacity: 0.65;
  }
  .base-note {
    margin: var(--ugoe-space-2) 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    border-left: 3px solid var(--ugoe-color-border);
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
