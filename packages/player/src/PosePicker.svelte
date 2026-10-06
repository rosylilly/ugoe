<!--
  ポーズの要素を選ぶ欄 (T0948)。ビューアのポーズのパネル (Player の `poseGroups`・`poseShown`・`showPose`。T0735) と、スタジオの
  操作ウィンドウのポーズのパネル (バックエンドの `scene` の通知と `show_pose`。T0842) で共有する。グループごとに、要素が 2〜4 個なら
  SegmentedControl、5 個以上なら Select で選ぶ (docs/ui.md#部品の一覧)。見出しは「グループ n」(モジュールのグループは「<モジュール> のグループ n」)、
  要素の表示名はモジュールの接頭辞を除いたもの。
  欄は `shown` をそのまま出すので、呼び手が表示中の要素に合わせれば (ビューアはモーションによる切り替えに追従する。T0736) 欄も追従する。
  `onchange` が Promise を返す間 (送っている途中) は、届いた `shown` より選んだ要素を先に出す。
-->
<script lang="ts">
import { SegmentedControl, Select } from "@ugoe/ui";
import { t } from "./i18n";
import { type PoseGroupInfo, poseEntryLabel, poseGroupNumbers } from "./pose";

interface Props {
  /** ポーズのグループ (本体と、着せ替えで付けたモジュールのもの)。 */
  groups: readonly PoseGroupInfo[];
  /** グループごとに表示している要素 (`groups` と同じ順)。 */
  shown: readonly string[];
  disabled?: boolean;
  /** 要素を選んだ。Promise を返せば、終わるまで選んだ要素を先に出す。 */
  onchange: (part: string) => unknown;
}

let { groups, shown, disabled = false, onchange }: Props = $props();

/** 送っている途中の要素 (グループの要素の並びを鍵にする。届いた `shown` より先に出す)。 */
let pending = $state<Record<string, string>>({});

const numbers = $derived(poseGroupNumbers(groups));

function keyOf(group: PoseGroupInfo): string {
  return group.parts.join("\n");
}

function choose(group: PoseGroupInfo, part: string) {
  const result = onchange(part);
  if (!(result instanceof Promise)) return;
  const key = keyOf(group);
  pending[key] = part;
  const settle = () => {
    if (pending[key] === part) delete pending[key];
  };
  result.then(settle, settle);
}
</script>

<div class="ugoe-pose-picker">
  {#each groups as group, i (`${i}:${keyOf(group)}`)}
    {@const n = numbers[i] ?? i + 1}
    {@const label =
      group.module === null ? t("pose.group", { n }) : t("pose.moduleGroup", { module: group.module, n })}
    {@const options = group.parts.map((part) => ({ value: part, label: poseEntryLabel(group, part) }))}
    {@const value = pending[keyOf(group)] ?? shown[i] ?? ""}
    <div class="ugoe-pose-picker__group">
      {#if options.length <= 4}
        <SegmentedControl {label} {value} {options} {disabled} onchange={(part) => choose(group, part)} />
      {:else}
        <Select {label} {value} {options} {disabled} onchange={(part) => choose(group, part)} />
      {/if}
    </div>
  {/each}
</div>

<style>
  .ugoe-pose-picker {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  /* 要素の名前が長くても横にはみ出さず、折り返す。 */
  .ugoe-pose-picker :global(.ugoe-segmented),
  .ugoe-pose-picker :global(.ugoe-segmented__group) {
    flex-wrap: wrap;
  }

  .ugoe-pose-picker__group {
    min-width: 0;
  }
</style>
