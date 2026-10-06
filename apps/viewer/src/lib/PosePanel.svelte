<script lang="ts">
// ポーズのパネル (T0735)。ポーズのグループを持つモデルのときに App.svelte が出す。
// グループごとに表示する要素を選ぶ (Player.showPose → WebPuppet の show_pose。次のフレームからフェードで切り替わる)。
// グループは WebPuppet の合成したポーズ (pose_json) から読むので、着せ替えで付けたモジュールのグループも並ぶ
// (Player がモデルの読み直しと付け外しのたびに読み直す)。モーションが同じグループの要素を要求すれば、モーションが勝ち
// (docs/spec/motion.md#ポーズ)、欄も Player が読む表示中の要素 (poseShown) に追従する (T0736)。欄は @ugoe/player の PosePicker
// (スタジオと共有。T0948)。
import { type Player, PosePicker } from "@ugoe/player";
import { Section } from "@ugoe/ui";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let open = $state(true);
</script>

<div class="panel poses">
  <Section title={t("poses.title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status">{t("poses.status", { count: player.poseGroups.length })}</span>
    {/snippet}

    <p class="hint">{t("poses.hint")}</p>
    <div class="groups">
      <PosePicker groups={player.poseGroups} shown={player.poseShown} onchange={(part) => player.showPose(part)} />
    </div>
  </Section>
</div>

<style>
  /* 高さは中身のまま (右の列の上の段 (App.svelte) がまとめて送る。T0725)。 */
  .poses {
    font-size: var(--ugoe-font-size-sm);
  }
  .poses :global(.ugoe-select),
  .poses :global(.ugoe-segmented) {
    font-size: var(--ugoe-font-size-sm);
  }
  .status,
  .hint {
    color: var(--ugoe-color-text-muted);
    font-weight: var(--ugoe-font-weight-normal);
  }
  .status {
    font-size: var(--ugoe-font-size-xs);
  }
  .hint {
    margin: 0;
    font-size: var(--ugoe-font-size-xs);
  }
  .groups {
    margin-top: var(--ugoe-space-2);
  }
</style>
