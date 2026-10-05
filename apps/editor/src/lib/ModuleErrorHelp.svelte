<!--
  モジュールのプロジェクトで適用を拒否されたときの直し方の案内 (docs/editor.md#モジュールの適用のエラーの案内、T1609)。
  エラーの文の下に出す。本体を直せば済むエラー (本体の要素の編集・本体に物理が無い・スロットが公開していない・本体に無い) にだけ
  案内の文を出し、本体が .ugoeproj なら「Open base project」も出す。どのエラーに案内を付けるかは moduleProject.ts の moduleErrorHelp。
-->
<script lang="ts">
import { t } from "./i18n";
import { moduleErrorHelp } from "./moduleProject";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  /** 適用のエラー (バックエンドの `Message`)。 */
  error: unknown;
  /** 本体のパス (モジュールのプロジェクトでなければ null で、何も出さない)。 */
  base: string | null;
  busy?: boolean;
  onopenbase: () => void;
}

let { error, base, busy = false, onopenbase }: Props = $props();

const help = $derived(moduleErrorHelp(error, base));
</script>

{#if help}
  <div class="help">
    <p class="ed-hint">{help.hint}</p>
    {#if help.openBase}
      <TipButton disabled={busy} tooltip={t("module.errorHelp.openBase.tooltip")} onclick={onopenbase}>
        {t("module.errorHelp.openBase.label")}
      </TipButton>
    {/if}
  </div>
{/if}

<style>
  .help {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--ugoe-space-2);
  }
  p {
    margin: 0;
  }
</style>
