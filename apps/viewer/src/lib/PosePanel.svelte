<script lang="ts">
// ポーズのパネル (T0735)。ポーズのグループを持つモデルのときに App.svelte が出す。
// グループごとに表示する要素を選ぶ (Player.showPose → WebPuppet の show_pose。次のフレームからフェードで切り替わる)。
// グループは WebPuppet の合成したポーズ (pose_json) から読むので、着せ替えで付けたモジュールのグループも並ぶ
// (Player がモデルの読み直しと付け外しのたびに読み直す)。モーションが同じグループの要素を要求すれば、モーションが勝つ
// (docs/spec/motion.md#ポーズ)。要素は 2〜4 個なら SegmentedControl、5 個以上なら Select で選ぶ (docs/ui.md#部品の一覧)。
import { type Player, poseEntryLabel, poseGroupNumbers } from "@ugoe/player";
import { Section, SegmentedControl, Select } from "@ugoe/ui";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let open = $state(true);

let numbers = $derived(poseGroupNumbers(player.poseGroups));
</script>

<div class="panel poses">
  <Section title={t("poses.title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status">{t("poses.status", { count: player.poseGroups.length })}</span>
    {/snippet}

    <p class="hint">{t("poses.hint")}</p>
    <div class="groups">
      {#each player.poseGroups as group, i (`${i}:${group.parts.join(",")}`)}
        {@const n = numbers[i] ?? i + 1}
        {@const label =
          group.module === null ? t("poses.group", { n }) : t("poses.moduleGroup", { module: group.module, n })}
        {@const options = group.parts.map((part) => ({ value: part, label: poseEntryLabel(group, part) }))}
        <div class="group">
          {#if options.length <= 4}
            <SegmentedControl
              {label}
              value={player.poseShown[i] ?? ""}
              {options}
              onchange={(part) => player.showPose(part)}
            />
          {:else}
            <Select
              {label}
              value={player.poseShown[i] ?? ""}
              {options}
              onchange={(part) => player.showPose(part)}
            />
          {/if}
        </div>
      {/each}
    </div>
  </Section>
</div>

<style>
  /* 縦が足りないときは、見出しが見える高さまで縮んで中を送る (パラメータの一覧の高さを残す)。 */
  .poses {
    flex: 0 1 auto;
    min-height: calc(var(--ugoe-control-height) + var(--ugoe-space-3));
    max-height: 30%;
    overflow-y: auto;
    font-size: var(--ugoe-font-size-sm);
  }
  .poses :global(.ugoe-select),
  .poses :global(.ugoe-segmented) {
    font-size: var(--ugoe-font-size-sm);
  }
  /* 要素の名前が長くても横にはみ出さず、折り返す。 */
  .poses :global(.ugoe-segmented),
  .poses :global(.ugoe-segmented__group) {
    flex-wrap: wrap;
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
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-2);
  }
  .group {
    min-width: 0;
  }
</style>
