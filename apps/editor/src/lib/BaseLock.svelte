<!--
  モジュールのプロジェクトで、本体のもの (読み取り専用の下敷き) の欄をまとめて無効にし、「本体のプロジェクトで直す」と
  示す (docs/editor.md#本体のものの表示)。`locked` でなければ中身をそのまま出す。
  無効にするのは入力欄とボタン (fieldset の disabled)。見るだけの表示はそのまま読める。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import { BASE_EDIT_HINT } from "./moduleProject";

interface Props {
  locked: boolean;
  /** 無効にした理由と直す場所 (既定は本体のものであること)。 */
  hint?: string;
  children: Snippet;
}

let { locked, hint = BASE_EDIT_HINT, children }: Props = $props();
</script>

{#if locked}<p class="base-note" role="note">{hint}</p>{/if}
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
    margin: 4px 0;
    padding: 4px 8px;
    border-left: 3px solid var(--border);
    font-size: 12px;
    opacity: 0.85;
  }
</style>
